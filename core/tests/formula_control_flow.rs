use finkit::formula::{AstNode, FormulaContext, FormulaError, FormulaExecutor};
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
fn while_limit_is_consistent_across_executor_paths() {
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
