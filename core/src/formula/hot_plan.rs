//! Formula-specific Architecture v3 plan lowering.
//!
//! Parsing/lowering remains in [`super::compute_ir`]. This adapter makes the
//! semantic/hot split explicit: a formula is first lowered to a logical DAG,
//! then that DAG is compiled once into numeric kernel/buffer/state slots.

use super::ast::AstNode;
use super::compute_ir::FormulaComputePlan;
use crate::compute::ComputePlanError;
use crate::execution_plan::{HotExecutionPlan, HotPlanError};
use crate::registry::FunctionRegistry;
use std::fmt;

/// Fully compiled Formula Architecture v3 plan.
#[derive(Debug, Clone)]
pub struct FormulaHotPlan {
    semantic: FormulaComputePlan,
    hot: HotExecutionPlan,
}

impl FormulaHotPlan {
    /// Compile with the canonical built-in function registry.
    pub fn compile(ast: &AstNode) -> Result<Self, FormulaHotPlanError> {
        let semantic = FormulaComputePlan::compile(ast)?;
        let hot = HotExecutionPlan::compile(semantic.plan(), [semantic.root()])?;
        Ok(Self { semantic, hot })
    }

    /// Compile with an explicit registry while keeping the same hot-plan ABI.
    pub fn compile_with_registry(
        ast: &AstNode,
        registry: &FunctionRegistry,
    ) -> Result<Self, FormulaHotPlanError> {
        let semantic = FormulaComputePlan::compile_with_registry(ast, registry)?;
        let hot = HotExecutionPlan::compile(semantic.plan(), [semantic.root()])?;
        Ok(Self { semantic, hot })
    }

    /// Logical DAG retained for diagnostics, optimizer passes and tooling.
    pub const fn semantic(&self) -> &FormulaComputePlan {
        &self.semantic
    }

    /// Numeric execution plan consumed by hot executors.
    pub const fn hot(&self) -> &HotExecutionPlan {
        &self.hot
    }
}

/// Errors produced while compiling a Formula Architecture v3 plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormulaHotPlanError {
    /// Semantic DAG validation/lowering failed.
    Semantic(ComputePlanError),
    /// Numeric hot-plan lowering failed.
    Hot(HotPlanError),
}

impl From<ComputePlanError> for FormulaHotPlanError {
    fn from(value: ComputePlanError) -> Self {
        Self::Semantic(value)
    }
}

impl From<HotPlanError> for FormulaHotPlanError {
    fn from(value: HotPlanError) -> Self {
        Self::Hot(value)
    }
}

impl fmt::Display for FormulaHotPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Semantic(error) => write!(f, "semantic formula plan error: {error}"),
            Self::Hot(error) => write!(f, "hot formula plan error: {error}"),
        }
    }
}

impl std::error::Error for FormulaHotPlanError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::parse_formula;

    #[test]
    fn formula_compiles_through_semantic_and_numeric_plan_layers() {
        let ast = parse_formula("EMA(CLOSE, 12) + ROC(CLOSE, 10)").unwrap();
        let compiled = FormulaHotPlan::compile(&ast).unwrap();

        assert!(!compiled.semantic().plan().is_empty());
        assert_eq!(
            compiled.hot().nodes().len(),
            compiled.semantic().plan().len()
        );
        assert!(compiled.hot().buffer_layout().slot_count() > 0);
    }

    #[test]
    fn formula_root_is_retained_through_end_of_hot_execution() {
        let ast = parse_formula("EMA(CLOSE, 12)").unwrap();
        let compiled = FormulaHotPlan::compile(&ast).unwrap();
        let root = compiled.semantic().root();

        assert_eq!(
            compiled.hot().buffer_layout().last_use(root),
            Some(compiled.semantic().plan().len())
        );
    }
}
