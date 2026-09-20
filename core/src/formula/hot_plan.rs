//! Formula-specific Architecture v3 plan lowering.
//!
//! Parsing/lowering remains in [`super::compute_ir`]. This adapter makes the
//! semantic/hot split explicit: a formula is first lowered to a logical DAG,
//! pure deterministic common subexpressions are interned once, then that DAG is
//! compiled into numeric kernel/input/parameter/buffer/state slots.

use super::ast::AstNode;
use super::compute_ir::FormulaComputePlan;
use crate::compute::{
    ComputeCapabilities, ComputeEffect, ComputeNode, ComputeNodeId, ComputePlan, ComputePlanError,
    LookbackRequirement,
};
use crate::execution_plan::{
    HotExecutionPlan, HotPlanError, InputSlot, ParameterArena, ParameterRange, ParameterValue,
};
use crate::registry::FunctionRegistry;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Fully compiled Formula Architecture v3 plan.
#[derive(Debug, Clone)]
pub struct FormulaHotPlan {
    semantic: FormulaComputePlan,
    hot: HotExecutionPlan,
    input_bindings: Vec<FormulaInputBinding>,
}

/// Compile-time binding from a formula variable to a numeric input slot.
///
/// Keeping this mapping in the hot plan avoids walking the semantic DAG and
/// resolving BTreeMap-backed slots on every repeated formula evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaInputBinding {
    name: String,
    slot: InputSlot,
}

impl FormulaInputBinding {
    /// Formula-context variable name, without the `VARIABLE:` operation prefix.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Numeric input slot consumed by the unified executor.
    pub const fn slot(&self) -> InputSlot {
        self.slot
    }
}

impl FormulaHotPlan {
    /// Compile with the canonical built-in function registry.
    pub fn compile(ast: &AstNode) -> Result<Self, FormulaHotPlanError> {
        let semantic = FormulaComputePlan::compile(ast)?;
        Self::finish(semantic, ast)
    }

    /// Compile with an explicit registry while keeping the same hot-plan ABI.
    pub fn compile_with_registry(
        ast: &AstNode,
        registry: &FunctionRegistry,
    ) -> Result<Self, FormulaHotPlanError> {
        let semantic = FormulaComputePlan::compile_with_registry(ast, registry)?;
        Self::finish(semantic, ast)
    }

    /// Shared tail of both compile entry points: literal binding, CSE, plumbing
    /// resolution, then numeric hot lowering.
    fn finish(semantic: FormulaComputePlan, ast: &AstNode) -> Result<Self, FormulaHotPlanError> {
        let (parameters, ranges) = bind_numeric_literals(ast, &semantic)?;
        let optimized = cse_plan(&semantic, &parameters, &ranges)?;
        let (numeric, root) = lower_formula_plumbing(&optimized, semantic.root())?;
        let numeric = prune_unreachable(&numeric, root)?;
        let hot =
            HotExecutionPlan::compile_with_parameters(&numeric, [root], parameters, ranges)?;
        let input_bindings = compile_input_bindings(&semantic, &hot);
        Ok(Self {
            semantic,
            hot,
            input_bindings,
        })
    }

    /// Logical DAG retained for diagnostics, optimizer passes and tooling.
    pub const fn semantic(&self) -> &FormulaComputePlan {
        &self.semantic
    }

    /// Numeric execution plan consumed by hot executors.
    pub const fn hot(&self) -> &HotExecutionPlan {
        &self.hot
    }

    /// Pre-resolved formula-variable bindings used by repeated evaluations.
    pub fn input_bindings(&self) -> &[FormulaInputBinding] {
        &self.input_bindings
    }
}

fn compile_input_bindings(
    semantic: &FormulaComputePlan,
    hot: &HotExecutionPlan,
) -> Vec<FormulaInputBinding> {
    let mut seen = BTreeSet::new();
    let mut bindings = Vec::new();
    for &node_id in semantic.plan().execution_order() {
        let Some(node) = semantic.plan().node(node_id) else {
            continue;
        };
        let Some(name) = node.operation.strip_prefix("VARIABLE:") else {
            continue;
        };
        if !seen.insert(name.to_string()) {
            continue;
        }
        let Some(slot) = hot
            .input_layout()
            .slot(node_id)
            .or_else(|| hot.input_layout().slot_for_operation(&node.operation))
        else {
            continue;
        };
        bindings.push(FormulaInputBinding {
            name: name.to_string(),
            slot,
        });
    }
    bindings
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

/// Resolve formula plumbing out of the numeric plan.
///
/// The semantic DAG keeps bookkeeping nodes so that optimizers and diagnostics
/// can see assignments, emitted outputs and statement structure. None of them
/// performs numeric work at run time:
///
/// * `ASSIGN:<name>` — the assigned value is already materialised in the
///   dependency's buffer, so the node is a pure alias.
/// * `OUTPUT:<name>` — same, the emitted value is its dependency.
/// * `VARIABLE:<name>` for a locally written name — reads back a value that was
///   just written, so it aliases the write.
/// * `STATEMENTS` — a block's value is its final statement.
/// * `COMPOUND:<name>:<op>` — rewrites to the equivalent `BINARY:<op>` node
///   with the current value and the right-hand side as operands.
///
/// Leaving these in the numeric plan had two costs. The runtime dispatcher was
/// forced to implement copy kernels it should never need, and — more seriously
/// — [`crate::execution_plan::InputLayout`] treats every `VARIABLE:` node as an
/// external input, so formula-local names such as `DIF` were advertised as
/// caller-supplied inputs. Removing them here fixes both at once and shrinks
/// the plan, which is why this runs after CSE (a cleaner graph means fewer
/// buffers and fewer hot instructions).
///
/// Only the root node id is remapped; every retained node keeps its original
/// id, so parameter ranges bound by [`bind_numeric_literals`] stay valid.
fn lower_formula_plumbing(
    optimized: &ComputePlan,
    root: ComputeNodeId,
) -> Result<(ComputePlan, ComputeNodeId), FormulaHotPlanError> {
    fn resolve(
        aliases: &BTreeMap<ComputeNodeId, ComputeNodeId>,
        mut id: ComputeNodeId,
    ) -> ComputeNodeId {
        // Aliases only ever point at an already-resolved id, so this walk is a
        // bounded chain and cannot loop.
        while let Some(&next) = aliases.get(&id) {
            if next == id {
                break;
            }
            id = next;
        }
        id
    }

    fn value_operand(
        rewritten: &[ComputeNodeId],
        operation: &str,
    ) -> Result<ComputeNodeId, FormulaHotPlanError> {
        rewritten.first().copied().ok_or_else(|| {
            FormulaHotPlanError::UnsupportedPlumbing {
                operation: operation.to_string(),
                reason: "node has no value operand".to_string(),
            }
        })
    }

    let mut aliases = BTreeMap::<ComputeNodeId, ComputeNodeId>::new();
    let mut local_writes = BTreeMap::<String, ComputeNodeId>::new();
    let mut nodes = Vec::with_capacity(optimized.len());
    let mut next_synthetic_id = optimized
        .execution_order()
        .iter()
        .map(|id| id.0)
        .max()
        .map_or(0, |max| max + 1);

    for &node_id in optimized.execution_order() {
        let source = optimized
            .node(node_id)
            .expect("optimized execution order only contains compiled nodes");
        let operation = source.operation.as_str();
        let rewritten: Vec<ComputeNodeId> = source
            .dependencies
            .iter()
            .map(|dependency| resolve(&aliases, *dependency))
            .collect();

        if let Some(name) = operation.strip_prefix("ASSIGN:") {
            let value = value_operand(&rewritten, operation)?;
            aliases.insert(node_id, value);
            local_writes.insert(name.to_string(), node_id);
            continue;
        }

        if operation.starts_with("OUTPUT:") {
            let value = value_operand(&rewritten, operation)?;
            aliases.insert(node_id, value);
            continue;
        }

        if let Some(name) = operation.strip_prefix("VARIABLE:") {
            if let Some(&write) = local_writes.get(name) {
                // A read of a formula-local variable is an alias for its most
                // recent write, which in turn aliases the value that produced it.
                aliases.insert(node_id, resolve(&aliases, write));
                continue;
            }
            // No preceding write: this is a genuine external input. Keep the
            // node so the hot plan can bind it to a caller-supplied slot.
            aliases.insert(node_id, node_id);
            nodes.push(ComputeNode::new(
                node_id,
                operation.to_string(),
                rewritten,
                source.capabilities.clone(),
            ));
            continue;
        }

        if operation == "STATEMENTS" {
            let value = rewritten.last().copied().ok_or_else(|| {
                FormulaHotPlanError::UnsupportedPlumbing {
                    operation: operation.to_string(),
                    reason: "statement block has no statements".to_string(),
                }
            })?;
            aliases.insert(node_id, value);
            continue;
        }

        if operation.starts_with("COMPOUND:") {
            let synthetic = lower_compound(
                node_id,
                operation,
                rewritten,
                next_synthetic_id,
                &mut aliases,
                &mut local_writes,
                &mut nodes,
            )?;
            next_synthetic_id = next_synthetic_id.max(synthetic.0 + 1);
            continue;
        }

        aliases.insert(node_id, node_id);
        nodes.push(ComputeNode::new(
            node_id,
            operation.to_string(),
            rewritten,
            source.capabilities.clone(),
        ));
    }

    let plan = ComputePlan::compile(nodes)?;
    Ok((plan, resolve(&aliases, root)))
}

/// Rewrite one `COMPOUND:<name>:<op>` node into the equivalent binary node.
///
/// `X += Y` is exactly `X = X + Y`, so the node is replaced by a synthetic
/// `BINARY:<op>` whose operands are already in `[current value, right-hand
/// side]` order. The synthetic id is allocated above every existing id, which
/// keeps the parameter ranges bound by [`bind_numeric_literals`] valid.
///
/// Returns the synthetic node id so the caller can keep allocating.
fn lower_compound(
    node_id: ComputeNodeId,
    operation: &str,
    rewritten: Vec<ComputeNodeId>,
    next_synthetic_id: usize,
    aliases: &mut BTreeMap<ComputeNodeId, ComputeNodeId>,
    local_writes: &mut BTreeMap<String, ComputeNodeId>,
    nodes: &mut Vec<ComputeNode>,
) -> Result<ComputeNodeId, FormulaHotPlanError> {
    let unsupported = |reason: String| FormulaHotPlanError::UnsupportedPlumbing {
        operation: operation.to_string(),
        reason,
    };

    let (name, assign_op) = operation
        .strip_prefix("COMPOUND:")
        .and_then(|rest| rest.rsplit_once(':'))
        .ok_or_else(|| unsupported("compound assignment is missing its operator".to_string()))?;
    let binary_op = match assign_op {
        "AddAssign" => "Add",
        "SubAssign" => "Sub",
        "MulAssign" => "Mul",
        "DivAssign" => "Div",
        other => {
            return Err(unsupported(format!(
                "unknown compound assignment operator `{other}`"
            )))
        }
    };
    if rewritten.len() != 2 {
        return Err(unsupported(format!(
            "expected 2 operands (current value, right-hand side), found {}",
            rewritten.len()
        )));
    }

    let synthetic = ComputeNodeId(next_synthetic_id);
    aliases.insert(node_id, synthetic);
    local_writes.insert(name.to_string(), synthetic);
    nodes.push(ComputeNode::new(
        synthetic,
        format!("BINARY:{binary_op}"),
        rewritten,
        ComputeCapabilities {
            deterministic: true,
            streaming: true,
            stateful: false,
            lookback: LookbackRequirement::None,
            effect: ComputeEffect::Pure,
        },
    ));
    Ok(synthetic)
}

/// Drop every node the retained root cannot reach.
///
/// The semantic plan keeps all syntax occurrences for diagnostics, and formula
/// plumbing resolution leaves the value subgraphs of dead assignments behind.
/// Neither contributes to the result, but both cost buffers, hot instructions
/// and — for nodes the numeric dispatcher has no kernel for, such as string
/// literals — would fail the whole plan.
///
/// This runs after [`lower_formula_plumbing`] on purpose: aliases must be
/// resolved first, otherwise a removed node would still look reachable through
/// a stale dependency edge.
fn prune_unreachable(
    plan: &ComputePlan,
    root: ComputeNodeId,
) -> Result<ComputePlan, FormulaHotPlanError> {
    let mut reachable = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(node_id) = pending.pop() {
        if !reachable.insert(node_id) {
            continue;
        }
        let node = plan
            .node(node_id)
            .ok_or(FormulaHotPlanError::UnsupportedPlumbing {
                operation: format!("{node_id:?}"),
                reason: "root references a node that is not in the numeric plan".to_string(),
            })?;
        pending.extend(node.dependencies.iter().copied());
    }

    let nodes = plan
        .execution_order()
        .iter()
        .filter(|node_id| reachable.contains(node_id))
        .map(|node_id| {
            let node = plan
                .node(*node_id)
                .expect("execution order only contains compiled nodes");
            ComputeNode::new(
                node.id,
                node.operation.clone(),
                node.dependencies.clone(),
                node.capabilities.clone(),
            )
        })
        .collect::<Vec<_>>();

    Ok(ComputePlan::compile(nodes)?)
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
    /// A formula plumbing node could not be resolved out of the numeric plan.
    UnsupportedPlumbing {
        /// Operation label that could not be lowered.
        operation: String,
        /// Why the node could not be lowered.
        reason: String,
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
            Self::UnsupportedPlumbing { operation, reason } => write!(
                f,
                "unsupported formula plumbing node `{operation}`: {reason}"
            ),
        }
    }
}

impl std::error::Error for FormulaHotPlanError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_plan::{InputSlot, ParameterSlot};
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

    #[test]
    fn input_bindings_are_precompiled_and_deduplicated() {
        let ast = parse_formula("EMA(CLOSE, 12) + ROC(CLOSE, 10)").unwrap();
        let compiled = FormulaHotPlan::compile(&ast).unwrap();

        assert_eq!(compiled.input_bindings().len(), 1);
        assert_eq!(compiled.input_bindings()[0].name(), "CLOSE");
        assert_eq!(compiled.input_bindings()[0].slot(), InputSlot(0));
    }
}
