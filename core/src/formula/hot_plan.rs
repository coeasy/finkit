//! Formula-specific Architecture v3 plan lowering.
//!
//! Parsing/lowering remains in [`super::compute_ir`]. This adapter makes the
//! semantic/hot split explicit: a formula is first lowered to a logical DAG,
//! pure deterministic common subexpressions are interned once, then that DAG is
//! compiled into numeric kernel/input/parameter/buffer/state slots.

use super::ast::AstNode;
use super::compute_ir::FormulaComputePlan;
use crate::compute::{ComputeNode, ComputeNodeId, ComputePlan, ComputePlanError};
use crate::execution_plan::{
    HotExecutionPlan, HotPlanError, ParameterArena, ParameterRange, ParameterValue,
};
use crate::registry::FunctionRegistry;
use std::collections::BTreeMap;
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
        let (parameters, ranges) = bind_numeric_literals(ast, &semantic)?;
        let optimized = cse_plan(&semantic, &parameters, &ranges)?;
        let hot = HotExecutionPlan::compile_with_parameters(
            &optimized,
            [semantic.root()],
            parameters,
            ranges,
        )?;
        Ok(Self { semantic, hot })
    }

    /// Compile with an explicit registry while keeping the same hot-plan ABI.
    pub fn compile_with_registry(
        ast: &AstNode,
        registry: &FunctionRegistry,
    ) -> Result<Self, FormulaHotPlanError> {
        let semantic = FormulaComputePlan::compile_with_registry(ast, registry)?;
        let (parameters, ranges) = bind_numeric_literals(ast, &semantic)?;
        let optimized = cse_plan(&semantic, &parameters, &ranges)?;
        let hot = HotExecutionPlan::compile_with_parameters(
            &optimized,
            [semantic.root()],
            parameters,
            ranges,
        )?;
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

/// Compile-time common-subexpression elimination for the numeric hot plan.
///
/// The semantic plan deliberately keeps every syntax occurrence for diagnostics.
/// At the hot boundary we can safely intern only deterministic, pure, stateless
/// nodes. Keys include canonicalized dependency ids and exact scalar parameter
/// bits, so e.g. EMA(CLOSE, 12) can never alias EMA(CLOSE, 26). Observable/stateful
/// nodes are never interned. The semantic root is retained under its original id
/// so public output layout and diagnostics keep a stable anchor.
fn cse_plan(
    semantic: &FormulaComputePlan,
    parameters: &ParameterArena,
    ranges: &BTreeMap<ComputeNodeId, ParameterRange>,
) -> Result<ComputePlan, ComputePlanError> {
    type CseKey = (String, Vec<ComputeNodeId>, Vec<ParameterValue>);

    let root = semantic.root();
    let mut aliases = BTreeMap::<ComputeNodeId, ComputeNodeId>::new();
    let mut interned = BTreeMap::<CseKey, ComputeNodeId>::new();
    let mut nodes = Vec::with_capacity(semantic.plan().len());

    for &node_id in semantic.plan().execution_order() {
        let source = semantic
            .plan()
            .node(node_id)
            .expect("semantic execution order only contains compiled nodes");
        let dependencies: Vec<_> = source
            .dependencies
            .iter()
            .map(|dependency| aliases.get(dependency).copied().unwrap_or(*dependency))
            .collect();

        let parameter_values = ranges
            .get(&node_id)
            .and_then(|range| parameters.range(*range))
            .unwrap_or(&[])
            .to_vec();
        let eligible = node_id != root
            && source.capabilities.deterministic
            && !source.capabilities.stateful
            && source.capabilities.effect.is_pure();

        if eligible {
            let key = (
                source.operation.clone(),
                dependencies.clone(),
                parameter_values,
            );
            if let Some(&canonical) = interned.get(&key) {
                aliases.insert(node_id, canonical);
                continue;
            }
            interned.insert(key, node_id);
        }

        aliases.insert(node_id, node_id);
        nodes.push(ComputeNode::new(
            node_id,
            source.operation.clone(),
            dependencies,
            source.capabilities.clone(),
        ));
    }

    ComputePlan::compile(nodes)
}

/// Bind exact numeric literals to NUMBER nodes without carrying literal strings
/// or floating-point equality into the hot loop.
///
/// `FormulaLowerer` allocates node ids monotonically while recursively visiting
/// the AST. This visitor mirrors only the child traversal performed by that
/// lowerer. NUMBER node ids are then paired with literals in creation order and
/// encoded as exact IEEE-754 bits in the immutable [`ParameterArena`].
fn bind_numeric_literals(
    ast: &AstNode,
    semantic: &FormulaComputePlan,
) -> Result<(ParameterArena, BTreeMap<ComputeNodeId, ParameterRange>), FormulaHotPlanError> {
    let mut literals = Vec::new();
    collect_lowered_numeric_literals(ast, &mut literals);

    let mut number_nodes = Vec::new();
    for raw_id in 0..semantic.plan().len() {
        let id = ComputeNodeId(raw_id);
        if semantic
            .plan()
            .node(id)
            .is_some_and(|node| node.operation == "NUMBER")
        {
            number_nodes.push(id);
        }
    }

    if literals.len() != number_nodes.len() {
        return Err(FormulaHotPlanError::LiteralBindingMismatch {
            ast_literals: literals.len(),
            number_nodes: number_nodes.len(),
        });
    }

    let mut arena = ParameterArena::new();
    let mut ranges = BTreeMap::new();
    for (node, value) in number_nodes.into_iter().zip(literals) {
        let range = arena.extend([ParameterValue::from_f64(value)]);
        ranges.insert(node, range);
    }
    Ok((arena, ranges))
}

/// Mirror FormulaLowerer child traversal exactly. Loop bodies are intentionally
/// excluded because `compute_ir` currently treats loop bodies as opaque control
/// flow and does not lower them into the acyclic compute plan.
fn collect_lowered_numeric_literals(ast: &AstNode, out: &mut Vec<f64>) {
    match ast {
        AstNode::Number(value) => out.push(*value),
        AstNode::StringLit(_) | AstNode::Variable(_) | AstNode::ParamDecl { .. } => {}
        AstNode::BinaryOp { left, right, .. } => {
            collect_lowered_numeric_literals(left, out);
            collect_lowered_numeric_literals(right, out);
        }
        AstNode::UnaryOp { expr, .. }
        | AstNode::Assignment { expr, .. }
        | AstNode::CompoundAssignment { expr, .. }
        | AstNode::Output { expr, .. } => collect_lowered_numeric_literals(expr, out),
        AstNode::FunctionCall { args, .. }
        | AstNode::Statements(args)
        | AstNode::DrawGeneric { args, .. } => {
            for arg in args {
                collect_lowered_numeric_literals(arg, out);
            }
        }
        AstNode::IndexAccess { array, index } => {
            collect_lowered_numeric_literals(array, out);
            collect_lowered_numeric_literals(index, out);
        }
        AstNode::DrawText { cond, price, .. } => {
            collect_lowered_numeric_literals(cond, out);
            collect_lowered_numeric_literals(price, out);
        }
        AstNode::DrawIcon {
            cond, price, icon, ..
        } => {
            collect_lowered_numeric_literals(cond, out);
            collect_lowered_numeric_literals(price, out);
            collect_lowered_numeric_literals(icon, out);
        }
        AstNode::StickLine {
            cond,
            price1,
            price2,
            width,
            ..
        } => {
            collect_lowered_numeric_literals(cond, out);
            collect_lowered_numeric_literals(price1, out);
            collect_lowered_numeric_literals(price2, out);
            collect_lowered_numeric_literals(width, out);
        }
        AstNode::IfThenElse {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_lowered_numeric_literals(cond, out);
            collect_lowered_numeric_literals(then_branch, out);
            collect_lowered_numeric_literals(else_branch, out);
        }
        AstNode::ForLoop { start, end, .. } => {
            collect_lowered_numeric_literals(start, out);
            collect_lowered_numeric_literals(end, out);
        }
        AstNode::WhileLoop { cond, .. } => collect_lowered_numeric_literals(cond, out),
    }
}

/// Errors produced while compiling a Formula Architecture v3 plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormulaHotPlanError {
    /// Semantic DAG validation/lowering failed.
    Semantic(ComputePlanError),
    /// Numeric hot-plan lowering failed.
    Hot(HotPlanError),
    /// Formula AST literals and semantic NUMBER nodes diverged.
    LiteralBindingMismatch {
        /// Number of literals observed while mirroring semantic lowering.
        ast_literals: usize,
        /// Number of NUMBER nodes present in the semantic plan.
        number_nodes: usize,
    },
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
            Self::LiteralBindingMismatch {
                ast_literals,
                number_nodes,
            } => write!(
                f,
                "formula literal binding mismatch: {ast_literals} AST literals vs {number_nodes} NUMBER nodes"
            ),
        }
    }
}

impl std::error::Error for FormulaHotPlanError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_plan::ParameterSlot;
    use crate::formula::parse_formula;

    #[test]
    fn formula_compiles_through_semantic_and_numeric_plan_layers() {
        let ast = parse_formula("EMA(CLOSE, 12) + ROC(CLOSE, 10)").unwrap();
        let compiled = FormulaHotPlan::compile(&ast).unwrap();

        assert!(!compiled.semantic().plan().is_empty());
        assert!(compiled.hot().nodes().len() <= compiled.semantic().plan().len());
        assert!(compiled.hot().buffer_layout().slot_count() > 0);
        assert_eq!(compiled.hot().parameter_arena().len(), 2);
    }

    #[test]
    fn duplicate_pure_subexpressions_are_interned_before_hot_lowering() {
        let ast = parse_formula("EMA(CLOSE,12) + EMA(CLOSE,12)").unwrap();
        let compiled = FormulaHotPlan::compile(&ast).unwrap();
        let semantic_ema = compiled
            .semantic()
            .plan()
            .execution_order()
            .iter()
            .filter(|&&id| {
                compiled
                    .semantic()
                    .plan()
                    .node(id)
                    .is_some_and(|node| node.operation == "CALL:EMA")
            })
            .count();
        let hot_ema = compiled
            .hot()
            .nodes()
            .iter()
            .filter(|node| node.kernel == crate::execution_plan::KernelId::from_static("CALL:EMA"))
            .count();

        assert_eq!(semantic_ema, 2);
        assert_eq!(hot_ema, 1);
        assert!(compiled.hot().nodes().len() < compiled.semantic().plan().len());
    }

    #[test]
    fn cse_key_keeps_different_literal_parameters_distinct() {
        let ast = parse_formula("EMA(CLOSE,12) + EMA(CLOSE,26)").unwrap();
        let compiled = FormulaHotPlan::compile(&ast).unwrap();
        let hot_ema = compiled
            .hot()
            .nodes()
            .iter()
            .filter(|node| node.kernel == crate::execution_plan::KernelId::from_static("CALL:EMA"))
            .count();
        assert_eq!(hot_ema, 2);
    }

    #[test]
    fn formula_root_is_retained_through_end_of_hot_execution() {
        let ast = parse_formula("EMA(CLOSE, 12)").unwrap();
        let compiled = FormulaHotPlan::compile(&ast).unwrap();
        let root = compiled.semantic().root();

        assert_eq!(
            compiled.hot().buffer_layout().last_use(root),
            Some(compiled.hot().nodes().len())
        );
    }

    #[test]
    fn formula_literals_keep_exact_ieee_bits_in_parameter_arena() {
        let ast = parse_formula("CLOSE * 0.125 + REF(CLOSE, 10)").unwrap();
        let compiled = FormulaHotPlan::compile(&ast).unwrap();
        let arena = compiled.hot().parameter_arena();

        assert_eq!(arena.len(), 2);
        assert_eq!(
            arena
                .get(ParameterSlot(0))
                .unwrap()
                .as_f64()
                .unwrap()
                .to_bits(),
            0.125f64.to_bits()
        );
        assert_eq!(
            arena
                .get(ParameterSlot(1))
                .unwrap()
                .as_f64()
                .unwrap()
                .to_bits(),
            10.0f64.to_bits()
        );

        let number_nodes: Vec<_> = compiled
            .hot()
            .nodes()
            .iter()
            .filter(|node| !node.parameters.is_empty())
            .collect();
        assert_eq!(number_nodes.len(), 2);
        assert_eq!(number_nodes[0].parameters.len, 1);
        assert_eq!(number_nodes[1].parameters.len, 1);
    }
}
