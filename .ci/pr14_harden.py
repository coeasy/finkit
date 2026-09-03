from pathlib import Path
import sys


def replace_exact(path: str, old: str, new: str, expected: int = 1) -> None:
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    count = text.count(old)
    if count != expected:
        raise SystemExit(
            f"{path}: expected {expected} occurrences, found {count}: {old[:120]!r}"
        )
    file.write_text(text.replace(old, new), encoding="utf-8")


def round1() -> None:
    compute = Path("core/src/compute.rs")
    text = compute.read_text(encoding="utf-8")
    old_import = (
        "use crate::factors::{FactorContext, FactorEngine, FactorError, FactorRegistry, FactorResult};"
    )
    new_import = """use crate::factors::{
    BorrowedFactorContext, FactorContext, FactorEngine, FactorError, FactorRegistry, FactorResult,
};"""
    if text.count(old_import) != 1:
        raise SystemExit("compute.rs: Factor imports do not match expected baseline")
    text = text.replace(old_import, new_import, 1)

    old_execution = """    /// Validate raw inputs before numerical execution starts.
    pub fn validate_context(&self, context: &FactorContext) -> FactorResult<()> {
        for input in &self.required_raw_inputs {
            if context.get(input).is_none() {
                return Err(FactorError::MissingInput(input.clone()));
            }
        }
        Ok(())
    }

    /// Execute the plan through the current factor engine implementation.
    ///
    /// This preserves the existing memoization and result semantics. A future
    /// optimized backend can consume [`Self::execution_order`] directly
    /// without changing this public planning contract.
    pub fn execute(
        &self,
        engine: &FactorEngine,
        context: &FactorContext,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        self.validate_context(context)?;
        self.validate_engine(engine)?;
        self.execute_precompiled(engine, context)
    }

    fn validate_engine(&self, engine: &FactorEngine) -> FactorResult<()> {
"""
    new_execution = """    /// Validate owned raw inputs before numerical execution starts.
    pub fn validate_context(&self, context: &FactorContext) -> FactorResult<()> {
        self.validate_raw_inputs(|name| context.get(name).is_some())
    }

    /// Validate borrowed raw inputs before numerical execution starts.
    pub fn validate_borrowed_context(
        &self,
        context: &BorrowedFactorContext<'_>,
    ) -> FactorResult<()> {
        self.validate_raw_inputs(|name| context.get(name).is_some())
    }

    fn validate_raw_inputs(&self, mut contains: impl FnMut(&str) -> bool) -> FactorResult<()> {
        for input in &self.required_raw_inputs {
            if !contains(input) {
                return Err(FactorError::MissingInput(input.clone()));
            }
        }
        Ok(())
    }

    /// Execute the plan through the precompiled topological order.
    pub fn execute(
        &self,
        engine: &FactorEngine,
        context: &FactorContext,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        self.execute_precompiled(engine, context)
    }

    /// Execute this plan in precompiled order over owned input.
    pub fn execute_precompiled(
        &self,
        engine: &FactorEngine,
        context: &FactorContext,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        self.validate_context(context)?;
        self.validate_engine(engine)?;
        engine.evaluate_precompiled(self.execution_order(), self.required_raw_inputs(), context)
    }

    /// Execute this plan in precompiled order over zero-copy borrowed input.
    pub fn execute_borrowed(
        &self,
        engine: &FactorEngine,
        context: &BorrowedFactorContext<'_>,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        self.validate_borrowed_context(context)?;
        self.validate_engine(engine)?;
        engine.evaluate_precompiled_borrowed(
            self.execution_order(),
            self.required_raw_inputs(),
            context,
        )
    }

    fn validate_engine(&self, engine: &FactorEngine) -> FactorResult<()> {
"""
    if text.count(old_execution) != 1:
        raise SystemExit("compute.rs: FactorPlan execution baseline not found exactly once")
    text = text.replace(old_execution, new_execution, 1)

    test_marker = """    #[test]
    fn compute_input_enforces_runtime_nan_policy() {
"""
    regression = """    #[test]
    fn factor_plan_direct_paths_reject_stale_registry_dependencies() {
        fn identity(name: &'static str, dependency: &'static str) -> FactorDefinition {
            FactorDefinition::new(
                name,
                [dependency],
                FactorKind::TimeSeries,
                FactorDirection::HigherBetter,
                Arc::new(move |inputs| Ok(inputs.get(dependency)?.to_vec())),
            )
        }

        let mut original = FactorRegistry::new();
        original.register(identity("target", "close")).unwrap();
        original.register(identity("other", "volume")).unwrap();
        let plan = FactorPlan::compile(&original, &["target", "other"]).unwrap();
        assert_eq!(
            plan.required_raw_inputs()
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["close", "volume"]
        );

        let mut changed = FactorRegistry::new();
        changed.register(identity("target", "volume")).unwrap();
        changed.register(identity("other", "volume")).unwrap();
        let engine = FactorEngine::new(changed);

        let owned = FactorContext::new()
            .with_series("close", vec![1.0, 2.0])
            .unwrap()
            .with_series("volume", vec![10.0, 20.0])
            .unwrap();
        let owned_error = plan.execute_precompiled(&engine, &owned).unwrap_err();
        assert!(matches!(
            owned_error,
            FactorError::InvalidParameter(message)
                if message.contains("factor plan is stale") && message.contains("target")
        ));

        let close = [1.0, 2.0];
        let volume = [10.0, 20.0];
        let borrowed = BorrowedFactorContext::new()
            .with_series("close", &close)
            .unwrap()
            .with_series("volume", &volume)
            .unwrap();
        let borrowed_error = plan.execute_borrowed(&engine, &borrowed).unwrap_err();
        assert!(matches!(
            borrowed_error,
            FactorError::InvalidParameter(message)
                if message.contains("factor plan is stale") && message.contains("target")
        ));
    }

"""
    if text.count(test_marker) != 1:
        raise SystemExit("compute.rs: regression-test insertion point not found")
    text = text.replace(test_marker, regression + test_marker, 1)
    compute.write_text(text, encoding="utf-8")

    factors = Path("core/src/factors.rs")
    text = factors.read_text(encoding="utf-8")
    split_start = text.find("\nimpl crate::compute::FactorPlan {")
    split_end_marker = "\n}\n\n/// Simple return over `period` bars with NaN warm-up values."
    split_end = text.find(split_end_marker, split_start)
    if split_start < 0 or split_end < 0:
        raise SystemExit("factors.rs: split FactorPlan impl block not found")
    text = (
        text[:split_start]
        + "\n\n/// Simple return over `period` bars with NaN warm-up values."
        + text[split_end + len(split_end_marker) :]
    )

    raw_marker = """    fn evaluate_precompiled_raw(
        &self,
        execution_order: &[String],
        required_raw_inputs: &[String],
        context: &dyn RawFactorContext,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
"""
    wrappers = """    pub(crate) fn evaluate_precompiled(
        &self,
        execution_order: &[String],
        required_raw_inputs: &[String],
        context: &FactorContext,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        self.evaluate_precompiled_raw(execution_order, required_raw_inputs, context)
    }

    pub(crate) fn evaluate_precompiled_borrowed(
        &self,
        execution_order: &[String],
        required_raw_inputs: &[String],
        context: &BorrowedFactorContext<'_>,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        self.evaluate_precompiled_raw(execution_order, required_raw_inputs, context)
    }

"""
    if text.count(raw_marker) != 1:
        raise SystemExit("factors.rs: evaluate_precompiled_raw marker not found")
    text = text.replace(raw_marker, wrappers + raw_marker, 1)
    factors.write_text(text, encoding="utf-8")

    replace_exact(
        "ffi/python-binding/src/lib.rs",
        """        "beta" => IndicatorResult::Error(
            "Beta requires benchmark data (use individual function)".to_string(),
        ),
        "correl" | "correlation" => IndicatorResult::Error(
            "Correlation requires second series data (use individual function)".to_string(),
        ),
""",
        "",
    )

    replace_exact(
        ".github/workflows/ci.yml",
        "actions/checkout@v4",
        "actions/checkout@v7",
        7,
    )
    replace_exact(
        ".github/workflows/docs-check.yml",
        "actions/checkout@v4",
        "actions/checkout@v7",
    )
    replace_exact(
        ".github/workflows/python-wheels.yml",
        "actions/checkout@v4",
        "actions/checkout@v7",
        4,
    )
    replace_exact(
        ".github/workflows/python-wheels.yml",
        "actions/upload-artifact@v4",
        "actions/upload-artifact@v7",
    )
    replace_exact(
        ".github/workflows/python-wheels.yml",
        "actions/download-artifact@v4",
        "actions/download-artifact@v8",
        3,
    )
    replace_exact(
        ".github/workflows/python-wheels.yml",
        "PyO3/maturin-action@86b9d133d34bc1b40018696f782949dac11bd380 # v1.49.4",
        "PyO3/maturin-action@e83996d129638aa358a18fbd1dfb82f0b0fb5d3b # v1.51.0",
    )


def round2() -> None:
    path = Path("core/src/formula/compute_ir.rs")
    text = path.read_text(encoding="utf-8")

    old = """                let capabilities = self.function_capabilities(name);
                if capabilities.effect.is_pure() {
                    self.add_node(
                        format!("CALL:{}", canonical_name(name)),
                        dependencies,
                        capabilities,
                    )
                } else {
                    self.add_effect(
                        format!("CALL:{}", canonical_name(name)),
                        dependencies,
                        capabilities,
                    )
                }
"""
    new = """                let (operation_name, capabilities) = self.function_metadata(name);
                if capabilities.effect.is_pure() {
                    self.add_node(format!("CALL:{operation_name}"), dependencies, capabilities)
                } else {
                    self.add_effect(format!("CALL:{operation_name}"), dependencies, capabilities)
                }
"""
    if text.count(old) != 1:
        raise SystemExit("compute_ir.rs: FunctionCall lowering block not found")
    text = text.replace(old, new, 1)

    old = """    fn function_capabilities(&self, name: &str) -> ComputeCapabilities {
        self.registry.get(name).map_or_else(
            || ComputeCapabilities {
                // Unknown/custom formula functions are deliberately conservative.
                // Once registered in the SSOT they regain precise capabilities.
                deterministic: false,
                streaming: false,
                stateful: true,
                lookback: LookbackRequirement::Dynamic,
                effect: ComputeEffect::Stateful,
            },
            ComputeCapabilities::from_function_spec,
        )
    }
"""
    new = """    fn function_metadata(&self, name: &str) -> (String, ComputeCapabilities) {
        self.registry.get(name).map_or_else(
            || {
                (
                    canonical_name(name),
                    ComputeCapabilities {
                        // Unknown/custom formula functions are deliberately conservative.
                        // Once registered in the SSOT they regain precise capabilities.
                        deterministic: false,
                        streaming: false,
                        stateful: true,
                        lookback: LookbackRequirement::Dynamic,
                        effect: ComputeEffect::Stateful,
                    },
                )
            },
            |spec| {
                (
                    canonical_name(spec.name),
                    ComputeCapabilities::from_function_spec(spec),
                )
            },
        )
    }
"""
    if text.count(old) != 1:
        raise SystemExit("compute_ir.rs: function_capabilities baseline not found")
    text = text.replace(old, new, 1)

    old_expect = '.find(|&id| plan.node(id).unwrap().operation == "CALL:MA")'
    if text.count(old_expect) != 1:
        raise SystemExit("compute_ir.rs: CALL:MA test expectation not found")
    text = text.replace(
        old_expect,
        '.find(|&id| plan.node(id).unwrap().operation == "CALL:SMA")',
        1,
    )

    marker = """    #[test]
    fn unknown_function_is_conservative_until_registered() {
"""
    regression = """    #[test]
    fn aliases_lower_to_the_same_canonical_operation() {
        let ma = FormulaComputePlan::compile(&parse_formula("MA(CLOSE,5)").unwrap()).unwrap();
        let sma = FormulaComputePlan::compile(&parse_formula("SMA(CLOSE,5)").unwrap()).unwrap();

        let ma_node = ma.plan().node(ma.root()).unwrap();
        let sma_node = sma.plan().node(sma.root()).unwrap();
        assert_eq!(ma_node.operation, "CALL:SMA");
        assert_eq!(sma_node.operation, "CALL:SMA");
        assert_eq!(ma_node.capabilities, sma_node.capabilities);
    }

"""
    if text.count(marker) != 1:
        raise SystemExit("compute_ir.rs: unknown-function test marker not found")
    text = text.replace(marker, regression + marker, 1)
    path.write_text(text, encoding="utf-8")


def round3() -> None:
    path = Path("core/src/formula/compute_ir.rs")
    text = path.read_text(encoding="utf-8")

    replace = (
        "    last_write: BTreeMap<String, ComputeNodeId>,\n"
        "    last_effect: Option<ComputeNodeId>,\n"
    )
    if text.count(replace) != 1:
        raise SystemExit("compute_ir.rs: lowerer fields baseline not found")
    text = text.replace(
        replace,
        replace + "    last_control_flow: Option<ComputeNodeId>,\n",
        1,
    )

    replace = "            last_write: BTreeMap::new(),\n            last_effect: None,\n"
    if text.count(replace) != 1:
        raise SystemExit("compute_ir.rs: lowerer init baseline not found")
    text = text.replace(
        replace,
        replace + "            last_control_flow: None,\n",
        1,
    )

    old = """                self.last_write.insert(canonical_name(var), id);
                id
            }
            AstNode::WhileLoop { cond, .. } => {
                let cond = self.lower(cond);
                self.add_effect("WHILE_LOOP", vec![cond], opaque_control_flow_capabilities())
            }
"""
    new = """                self.last_write.insert(canonical_name(var), id);
                self.last_control_flow = Some(id);
                id
            }
            AstNode::WhileLoop { cond, .. } => {
                let cond = self.lower(cond);
                let id = self.add_effect(
                    "WHILE_LOOP",
                    vec![cond],
                    opaque_control_flow_capabilities(),
                );
                self.last_control_flow = Some(id);
                id
            }
"""
    if text.count(old) != 1:
        raise SystemExit("compute_ir.rs: opaque loop baseline not found")
    text = text.replace(old, new, 1)

    old = """    fn lower_variable(&mut self, name: &str) -> ComputeNodeId {
        let key = canonical_name(name);
        let dependencies = self.last_write.get(&key).copied().into_iter().collect();
        self.add_pure(format!("VARIABLE:{key}"), dependencies)
    }
"""
    new = """    fn lower_variable(&mut self, name: &str) -> ComputeNodeId {
        let key = canonical_name(name);
        let mut dependencies = Vec::with_capacity(2);
        if let Some(write) = self.last_write.get(&key).copied() {
            dependencies.push(write);
        }
        if let Some(barrier) = self.last_control_flow {
            if !dependencies.contains(&barrier) {
                dependencies.push(barrier);
            }
        }
        self.add_pure(format!("VARIABLE:{key}"), dependencies)
    }
"""
    if text.count(old) != 1:
        raise SystemExit("compute_ir.rs: lower_variable baseline not found")
    text = text.replace(old, new, 1)

    marker = """    #[test]
    fn drawing_commands_are_effectful_and_ordered() {
"""
    regression = """    #[test]
    fn reads_after_opaque_control_flow_depend_on_the_control_barrier() {
        let ast = AstNode::Statements(vec![
            AstNode::Assignment {
                name: "X".to_string(),
                expr: Box::new(AstNode::Number(0.0)),
            },
            AstNode::WhileLoop {
                cond: Box::new(AstNode::Number(0.0)),
                body: vec![AstNode::Assignment {
                    name: "X".to_string(),
                    expr: Box::new(AstNode::Number(1.0)),
                }],
            },
            AstNode::Variable("X".to_string()),
        ]);
        let formula_plan = FormulaComputePlan::compile(&ast).unwrap();
        let plan = formula_plan.plan();
        let barrier = plan
            .execution_order()
            .iter()
            .copied()
            .find(|&id| plan.node(id).unwrap().operation == "WHILE_LOOP")
            .unwrap();
        let read = plan
            .execution_order()
            .iter()
            .copied()
            .filter(|&id| plan.node(id).unwrap().operation == "VARIABLE:X")
            .last()
            .unwrap();

        assert!(plan.node(read).unwrap().dependencies.contains(&barrier));
    }

"""
    if text.count(marker) != 1:
        raise SystemExit("compute_ir.rs: drawing-test marker not found")
    text = text.replace(marker, regression + marker, 1)
    path.write_text(text, encoding="utf-8")

    path = Path("core/src/formula/executor.rs")
    text = path.read_text(encoding="utf-8")
    alias = (
        "type FormulaFn = fn(&FormulaContext, &[Array1<f64>]) -> "
        "Result<Array1<f64>, FormulaError>;\n"
    )
    helper = alias + """
const MAX_LOOP_ITERATIONS: usize = 10_000;

fn loop_iteration_limit_error(kind: &str) -> FormulaError {
    FormulaError::RuntimeError(format!(
        "{kind} loop exceeded maximum iterations ({MAX_LOOP_ITERATIONS})"
    ))
}
"""
    if text.count(alias) != 1:
        raise SystemExit("executor.rs: FormulaFn alias marker not found")
    text = text.replace(alias, helper, 1)

    if text.count("                let max_iterations = 10000i64;\n") != 3:
        raise SystemExit("executor.rs: expected three FOR max-iteration declarations")
    text = text.replace("                let max_iterations = 10000i64;\n", "")

    old_for_guard = """                    if count as i64 >= max_iterations {
                        return Err(FormulaError::RuntimeError(format!(
                            "FOR loop exceeded maximum iterations ({})",
                            max_iterations
                        )));
                    }
"""
    new_for_guard = """                    if count >= MAX_LOOP_ITERATIONS {
                        return Err(loop_iteration_limit_error("FOR"));
                    }
"""
    if text.count(old_for_guard) != 3:
        raise SystemExit("executor.rs: expected three FOR limit guards")
    text = text.replace(old_for_guard, new_for_guard)

    old = """            AstNode::WhileLoop { cond, body } => {
                let mut result = FormulaValue::Scalar(0.0);
                let max_iterations = 10000usize;
                for _ in 0..max_iterations {
                    let cond_val = self.execute_val(cond, ctx)?;
                    let cond_arr = cond_val.to_array(ctx.data_len);
                    if !cond_arr.iter().any(|&v| v > 0.0) {
                        break;
                    }
                    for stmt in body {
                        result = self.execute_val(stmt, ctx)?;
                    }
                }
                Ok(result)
            }
"""
    new = """            AstNode::WhileLoop { cond, body } => {
                let mut result = FormulaValue::Scalar(0.0);
                let mut iterations = 0usize;
                loop {
                    let cond_val = self.execute_val(cond, ctx)?;
                    let cond_arr = cond_val.to_array(ctx.data_len);
                    if !cond_arr.iter().any(|&v| v > 0.0) {
                        break;
                    }
                    if iterations >= MAX_LOOP_ITERATIONS {
                        return Err(loop_iteration_limit_error("WHILE"));
                    }
                    for stmt in body {
                        result = self.execute_val(stmt, ctx)?;
                    }
                    iterations += 1;
                }
                Ok(result)
            }
"""
    if text.count(old) != 1:
        raise SystemExit("executor.rs: standard WHILE baseline not found")
    text = text.replace(old, new, 1)

    old = """            AstNode::WhileLoop { cond, body } => {
                let mut result = pool.get_buffer(ctx.data_len);
                let max_iterations = 10000usize;
                for _ in 0..max_iterations {
                    let cond_val = self.execute_with_pool_cached(cond, ctx, pool, name_cache)?;
                    let should_break = !cond_val.iter().any(|&v| v > 0.0);
                    pool.return_buffer(cond_val);
                    if should_break {
                        break;
                    }
                    for stmt in body {
                        let new_result =
                            self.execute_with_pool_cached(stmt, ctx, pool, name_cache)?;
                        pool.return_buffer(result);
                        result = new_result;
                    }
                }
                Ok(result)
            }
"""
    new = """            AstNode::WhileLoop { cond, body } => {
                let mut result = pool.get_buffer(ctx.data_len);
                let mut iterations = 0usize;
                loop {
                    let cond_val = self.execute_with_pool_cached(cond, ctx, pool, name_cache)?;
                    let should_break = !cond_val.iter().any(|&v| v > 0.0);
                    pool.return_buffer(cond_val);
                    if should_break {
                        break;
                    }
                    if iterations >= MAX_LOOP_ITERATIONS {
                        pool.return_buffer(result);
                        return Err(loop_iteration_limit_error("WHILE"));
                    }
                    for stmt in body {
                        let new_result =
                            self.execute_with_pool_cached(stmt, ctx, pool, name_cache)?;
                        pool.return_buffer(result);
                        result = new_result;
                    }
                    iterations += 1;
                }
                Ok(result)
            }
"""
    if text.count(old) != 1:
        raise SystemExit("executor.rs: cached WHILE baseline not found")
    text = text.replace(old, new, 1)

    old = """            AstNode::WhileLoop { cond, body } => {
                let mut result = pool.get_buffer(ctx.data_len);
                let max_iterations = 10000usize;
                for _ in 0..max_iterations {
                    let cond_val = self.execute_with_pool(cond, ctx, pool)?;
                    let should_break = !cond_val.iter().any(|&v| v > 0.0);
                    pool.return_buffer(cond_val);
                    if should_break {
                        break;
                    }
                    for stmt in body {
                        let new_result = self.execute_with_pool(stmt, ctx, pool)?;
                        pool.return_buffer(result);
                        result = new_result;
                    }
                }
                Ok(result)
            }
"""
    new = """            AstNode::WhileLoop { cond, body } => {
                let mut result = pool.get_buffer(ctx.data_len);
                let mut iterations = 0usize;
                loop {
                    let cond_val = self.execute_with_pool(cond, ctx, pool)?;
                    let should_break = !cond_val.iter().any(|&v| v > 0.0);
                    pool.return_buffer(cond_val);
                    if should_break {
                        break;
                    }
                    if iterations >= MAX_LOOP_ITERATIONS {
                        pool.return_buffer(result);
                        return Err(loop_iteration_limit_error("WHILE"));
                    }
                    for stmt in body {
                        let new_result = self.execute_with_pool(stmt, ctx, pool)?;
                        pool.return_buffer(result);
                        result = new_result;
                    }
                    iterations += 1;
                }
                Ok(result)
            }
"""
    if text.count(old) != 1:
        raise SystemExit("executor.rs: pooled WHILE baseline not found")
    text = text.replace(old, new, 1)
    path.write_text(text, encoding="utf-8")

    test = Path("core/tests/formula_control_flow.rs")
    if test.exists():
        raise SystemExit("core/tests/formula_control_flow.rs already exists")
    test.write_text(
        """use finkit::formula::{AstNode, FormulaContext, FormulaError, FormulaExecutor};
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
""",
        encoding="utf-8",
    )


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] not in {"round1", "round2", "round3"}:
        raise SystemExit("usage: pr14_harden.py <round1|round2|round3>")
    {"round1": round1, "round2": round2, "round3": round3}[sys.argv[1]]()


if __name__ == "__main__":
    main()
