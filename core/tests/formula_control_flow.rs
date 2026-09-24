use finkit::formula::{
    compile_to_bytecode, parse_formula, AstNode, BytecodeVM, FormulaContext, FormulaError,
    FormulaExecutor, FormulaHotPlan, JitCompiler,
};
use ndarray::Array1;

fn context() -> FormulaContext {
    let values = Array1::from_vec(vec![1.0]);
    FormulaContext::new(
        values.clone(),
        values.clone(),
        values.clone(),
        values.clone(),
        values,
        None,
    )
}

fn infinite_while() -> AstNode {
    AstNode::WhileLoop {
        cond: Box::new(AstNode::Number(1.0)),
        body: Vec::new(),
    }
}

fn assert_while_limit(error: FormulaError) {
    assert!(matches!(
        error,
        FormulaError::RuntimeError(message)
            if message == "WHILE loop exceeded maximum iterations (10000)"
    ));
}

#[test]
fn unknown_formula_function_is_rejected_during_plan_compilation() {
    let ast = parse_formula("NOT_REGISTERED(CLOSE)").unwrap();
    let error = FormulaHotPlan::compile(&ast).unwrap_err();
    assert!(error
        .to_string()
        .contains("unknown formula function: NOT_REGISTERED"));
}

#[test]
fn malformed_context_is_rejected_before_formula_indexing() {
    let mut ctx = FormulaContext::new(
        Array1::from_vec(vec![1.0, 2.0]),
        Array1::from_vec(vec![1.0]),
        Array1::from_vec(vec![1.0, 2.0]),
        Array1::from_vec(vec![1.0, 2.0]),
        Array1::from_vec(vec![1.0, 2.0]),
        None,
    );
    let error = FormulaExecutor::new()
        .execute(&AstNode::Number(1.0), &mut ctx)
        .unwrap_err();
    assert!(matches!(
        error,
        FormulaError::InvalidParameter(message)
            if message.contains("OHLCV series length mismatch")
    ));
}

#[test]
fn while_limit_is_consistent_across_executor_paths() {
    let bytecode = compile_to_bytecode(&infinite_while(), "while 1 {};").unwrap();
    let mut bytecode_vm = BytecodeVM::new();
    let bytecode_error = match bytecode_vm.execute(&bytecode, &context()) {
        Ok(_) => panic!("bytecode VM accepted an unbounded loop"),
        Err(error) => error,
    };
    assert!(matches!(
        bytecode_error,
        FormulaError::RuntimeError(message)
            if message == "loop iteration limit exceeded in bytecode VM"
    ));

    let mut jit = JitCompiler::new();
    let optimized = jit.compile(bytecode);
    let mut jit_context = context();
    let jit_error = match jit.execute(&optimized, &mut jit_context) {
        Ok(_) => panic!("JIT VM accepted an unbounded loop"),
        Err(error) => error,
    };
    assert!(matches!(
        jit_error,
        FormulaError::RuntimeError(message)
            if message == "loop iteration limit exceeded in JIT VM"
    ));

    let executor = FormulaExecutor::new();
    let ast = infinite_while();

    let mut standard = context();
    assert_while_limit(executor.execute(&ast, &mut standard).unwrap_err());

    let mut cached = context();
    assert_while_limit(
        executor
            .execute_zero_copy_cached(&ast, &mut cached)
            .unwrap_err(),
    );

    let mut pooled = context();
    assert_while_limit(executor.execute_zero_copy(&ast, &mut pooled).unwrap_err());
}
