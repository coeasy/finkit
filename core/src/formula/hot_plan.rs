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
use crate::buffer_arena::BufferSlot;
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
    outputs: Vec<FormulaOutputBinding>,
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

/// Compile-time binding from a named formula output to a retained buffer slot.
///
/// A formula can emit several channels — a MACD script emits `DIF`, `DEA` and
/// `MACD`; a Pine script emits one channel per `plot`. Retaining every named
/// output lets a caller read them all instead of only the single primary
/// result, and keeps each channel's value subgraph alive through dead-code
/// elimination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaOutputBinding {
    name: String,
    slot: BufferSlot,
    level_marker: bool,
}

impl FormulaOutputBinding {
    /// Output channel name, without the `OUTPUT:` operation prefix.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Buffer slot holding this output's series after a run.
    pub const fn slot(&self) -> BufferSlot {
        self.slot
    }

    /// Whether this output is a level marker (Pine `hline`) rather than a data
    /// series. Level markers are still emitted, but are never chosen as a
    /// formula's primary result — see [`AstNode::produces_value`].
    pub const fn is_level_marker(&self) -> bool {
        self.level_marker
    }
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
        Self::finish(semantic)
    }

    /// Compile with an explicit registry while keeping the same hot-plan ABI.
    pub fn compile_with_registry(
        ast: &AstNode,
        registry: &FunctionRegistry,
    ) -> Result<Self, FormulaHotPlanError> {
        let semantic = FormulaComputePlan::compile_with_registry(ast, registry)?;
        Self::finish(semantic)
    }

    /// Shared tail of both compile entry points: literal binding, CSE, plumbing
    /// resolution, then numeric hot lowering.
    fn finish(semantic: FormulaComputePlan) -> Result<Self, FormulaHotPlanError> {
        let (parameters, ranges) = bind_numeric_literals(&semantic)?;
        let optimized = cse_plan(&semantic, &parameters, &ranges)?;
        let lowered = lower_formula_plumbing(&optimized, semantic.root())?;
        let numeric = prune_unreachable(&lowered.plan, &lowered.roots)?;
        let hot =
            HotExecutionPlan::compile_with_parameters(&numeric, lowered.roots, parameters, ranges)?;
        let input_bindings = compile_input_bindings(&semantic, &hot);
        let outputs = compile_output_bindings(&lowered.named_outputs, &hot);
        Ok(Self {
            semantic,
            hot,
            input_bindings,
            outputs,
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

    /// Named output channels in declaration order.
    ///
    /// The first value of an execution is always the formula's primary result
    /// (the value of its last value-producing statement); these bindings let a
    /// caller address every channel by name instead of only that one.
    pub fn outputs(&self) -> &[FormulaOutputBinding] {
        &self.outputs
    }
}

/// Pair each named output with the buffer slot its series lands in.
fn compile_output_bindings(
    named_outputs: &[NamedOutput],
    hot: &HotExecutionPlan,
) -> Vec<FormulaOutputBinding> {
    let layout = hot.output_layout().outputs();
    named_outputs
        .iter()
        .filter_map(|output| {
            let slot = layout
                .iter()
                .find(|(node, _)| *node == output.node)
                .map(|(_, slot)| *slot)?;
            Some(FormulaOutputBinding {
                name: output.name.clone(),
                slot,
                level_marker: output.level_marker,
            })
        })
        .collect()
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

/// One named output discovered while resolving formula plumbing.
#[derive(Debug, Clone, PartialEq, Eq)]
struct NamedOutput {
    name: String,
    /// Node carrying the output's value after plumbing resolution.
    node: ComputeNodeId,
    level_marker: bool,
}

/// Result of [`lower_formula_plumbing`].
struct LoweredPlumbing {
    /// Numeric plan with every plumbing node resolved away.
    plan: ComputePlan,
    /// Retained roots, frontend-requested order: the primary result first, then
    /// every named output, then every retained drawing directive. The primary
    /// result stays at index 0 so the first value of an execution is the
    /// formula's result.
    roots: Vec<ComputeNodeId>,
    /// Named outputs in declaration order.
    named_outputs: Vec<NamedOutput>,
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
/// * `STATEMENTS` — a block's value is its final *value-producing* statement,
///   so a trailing drawing directive or level marker cannot become the result.
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
/// Retained node ids are remapped only where a node was aliased away; every
/// kept node keeps its original id, so parameter ranges bound by
/// [`bind_numeric_literals`] stay valid.
fn lower_formula_plumbing(
    optimized: &ComputePlan,
    root: ComputeNodeId,
) -> Result<LoweredPlumbing, FormulaHotPlanError> {
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

    /// Mirrors [`AstNode::produces_value`] on the lowered numeric plan.
    ///
    /// Drawing directives and level markers are observable but carry no data
    /// series, so they can never be a statement block's result.
    fn value_producing(plan: &ComputePlan, id: ComputeNodeId) -> bool {
        plan.node(id).is_some_and(|node| {
            !matches!(
                node.capabilities.effect,
                ComputeEffect::Draw | ComputeEffect::EmitLevelMarker(_)
            )
        })
    }

    let mut aliases = BTreeMap::<ComputeNodeId, ComputeNodeId>::new();
    let mut local_writes = BTreeMap::<String, ComputeNodeId>::new();
    let mut named_outputs = Vec::<NamedOutput>::new();
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

        if let Some(name) = operation.strip_prefix("OUTPUT:") {
            let value = value_operand(&rewritten, operation)?;
            aliases.insert(node_id, value);
            // An `Output` is *also* a variable write. The tree path stores the
            // value in `ctx.variables[name]` (see the `AstNode::Output` arms in
            // `executor.rs`), which is what makes the ubiquitous
            // `DIF:...;DEA:EMA(DIF,9);MACD:(DIF-DEA)*2` idiom work. Omitting this
            // leaves the later read of `DIF` as a bare `VARIABLE:DIF` that misses
            // `local_writes`, so it is declared an external input and the plan
            // demands a caller-supplied series named `DIF`.
            //
            // The parser emits a single `AstNode::Output` for the single-colon
            // form, so this is the only place that can record the write.
            local_writes.insert(name.to_string(), node_id);
            named_outputs.push(NamedOutput {
                name: name.to_string(),
                node: value,
                level_marker: matches!(
                    source.capabilities.effect,
                    ComputeEffect::EmitLevelMarker(_)
                ),
            });
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
            // A block's result is its last *value-producing* statement, so a
            // trailing drawing directive or level marker cannot become the
            // whole result. The predicate reads the original dependency (the
            // statement node) rather than the resolved one: resolution
            // collapses e.g. `OUTPUT:HLINE` onto its constant operand and would
            // lose the fact that it was a marker.
            let value = source
                .dependencies
                .iter()
                .rev()
                .find(|dependency| value_producing(optimized, **dependency))
                .map(|dependency| resolve(&aliases, *dependency))
                .ok_or_else(|| FormulaHotPlanError::UnsupportedPlumbing {
                    operation: operation.to_string(),
                    reason: "statement block has no value-producing statement".to_string(),
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

    // Retain the primary result first, then every named output, then every
    // drawing directive. Named outputs keep their value subgraphs alive, and
    // retaining drawings stops chart side effects from being silently dropped
    // by dead-code elimination. An unsupported drawing still fails the plan
    // loudly instead of vanishing.
    let primary = resolve(&aliases, root);
    let mut roots = vec![primary];
    for output in &named_outputs {
        if !roots.contains(&output.node) {
            roots.push(output.node);
        }
    }
    for node in &nodes {
        if matches!(node.capabilities.effect, ComputeEffect::Draw) && !roots.contains(&node.id) {
            roots.push(node.id);
        }
    }

    let plan = ComputePlan::compile(nodes)?;
    Ok(LoweredPlumbing {
        plan,
        roots,
        named_outputs,
    })
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

/// Drop every node the retained roots cannot reach.
///
/// The semantic plan keeps all syntax occurrences for diagnostics, and formula
/// plumbing resolution leaves the value subgraphs of dead assignments behind.
/// Neither contributes to the result, but both cost buffers, hot instructions
/// and — for nodes the numeric dispatcher has no kernel for, such as string
/// literals — would fail the whole plan.
///
/// This runs after [`lower_formula_plumbing`] on purpose: aliases must be
/// resolved first, otherwise a removed node would still look reachable through
/// a stale dependency edge. Reachability starts from *every* retained root, not
/// just the primary result, so named outputs and drawing side effects survive.
fn prune_unreachable(
    plan: &ComputePlan,
    roots: &[ComputeNodeId],
) -> Result<ComputePlan, FormulaHotPlanError> {
    let mut reachable = BTreeSet::new();
    let mut pending = roots.to_vec();
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
/// Literals come from [`FormulaComputePlan::number_literal`], i.e. from the
/// lowerer that created each node, rather than from a second walk of the AST.
/// A mirrored walk cannot survive loop unrolling: it would have to reproduce
/// every lowering decision, including how many times a loop body was
/// duplicated, and any drift would bind the wrong constant to the wrong node
/// instead of failing.
fn bind_numeric_literals(
    semantic: &FormulaComputePlan,
) -> Result<(ParameterArena, BTreeMap<ComputeNodeId, ParameterRange>), FormulaHotPlanError> {
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

    let recorded = semantic.number_literals_len();
    if recorded != number_nodes.len() {
        return Err(FormulaHotPlanError::LiteralBindingMismatch {
            ast_literals: recorded,
            number_nodes: number_nodes.len(),
        });
    }

    let mut arena = ParameterArena::new();
    let mut ranges = BTreeMap::new();
    for node in number_nodes {
        let value = semantic.number_literal(node).ok_or(
            FormulaHotPlanError::LiteralBindingMismatch {
                ast_literals: recorded,
                number_nodes: recorded,
            },
        )?;
        let range = arena.extend([ParameterValue::from_f64(value)]);
        ranges.insert(node, range);
    }
    Ok((arena, ranges))
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
    fn a_named_output_is_readable_as_a_local_variable() {
        // The single-colon form parses to a bare `AstNode::Output`, so the
        // plumbing pass is the only place that can record it as a variable
        // write. Without that, the second statement's `DIF` read stays a bare
        // `VARIABLE:DIF` and the plan asks the caller for a series named `DIF`.
        let ast = parse_formula("DIF:EMA(CLOSE,12)-EMA(CLOSE,26);DEA:EMA(DIF,9)").unwrap();
        let compiled = FormulaHotPlan::compile(&ast).unwrap();

        let inputs: Vec<&str> = compiled
            .input_bindings()
            .iter()
            .map(|binding| binding.name())
            .collect();
        assert_eq!(inputs, vec!["CLOSE"], "only CLOSE is an external input");

        let outputs: Vec<&str> = compiled
            .outputs()
            .iter()
            .map(|binding| binding.name())
            .collect();
        assert_eq!(outputs, vec!["DIF", "DEA"]);
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

    /// A Pine script ending in `hline` must report its plotted series, not the
    /// marker constant.
    ///
    /// Before the level-marker tag, the retained root resolved to `hline(30)`,
    /// dead-code elimination reduced the plan to a single `NUMBER`, and the
    /// input layout came out empty — the executor then could not infer an
    /// execution length at all.
    #[test]
    fn trailing_level_markers_are_not_the_formula_result() {
        let pine = crate::formula::pine::parse_pine(
            "//@version=5\nindicator(\"RSI\")\nrsi = ta.rsi(close, 14)\nplot(rsi, \"RSI\")\nhline(70)\nhline(30)\n",
        )
        .expect("Pine script must parse");
        let ast = crate::formula::pine::map_pine_to_alphata(&pine).expect("Pine must map");
        let compiled = FormulaHotPlan::compile(&ast).expect("plan must compile");

        // The plotted series is the result, so `close` stays a real input.
        assert_eq!(compiled.input_bindings().len(), 1);
        assert_eq!(compiled.input_bindings()[0].name(), "CLOSE");

        // Both markers are still emitted as channels, tagged as level markers.
        let markers: Vec<_> = compiled
            .outputs()
            .iter()
            .filter(|output| output.is_level_marker())
            .collect();
        assert_eq!(markers.len(), 2, "both hline calls must be retained");
        assert!(
            compiled.outputs().iter().any(|output| !output.is_level_marker()),
            "the plotted series must be retained as a data channel"
        );
    }

    /// Named outputs are exposed with the buffer slot their series lands in.
    #[test]
    fn named_outputs_expose_buffer_slots() {
        let ast = parse_formula("MA5: MA(CLOSE, 5); MA10: MA(CLOSE, 10)").unwrap();
        let compiled = FormulaHotPlan::compile(&ast).unwrap();

        let names: Vec<_> = compiled
            .outputs()
            .iter()
            .map(|output| output.name().to_string())
            .collect();
        assert_eq!(names, vec!["MA5".to_string(), "MA10".to_string()]);
        assert!(compiled.outputs().iter().all(|output| !output.is_level_marker()));

        // Both channels need distinct buffers.
        let slots: Vec<_> = compiled.outputs().iter().map(|output| output.slot()).collect();
        assert_ne!(slots[0], slots[1]);

        // The retained-root order is not the declaration order: root 0 is the
        // *primary* result, which is the last value-producing statement — here
        // `MA10`. `outputs()` stays in declaration order so callers can address
        // channels by name.
        let roots = compiled.hot().output_layout().outputs();
        assert_eq!(
            roots[0].1, slots[1],
            "the primary result must be the last value-producing statement (MA10)"
        );
        assert_eq!(roots[1].1, slots[0], "MA5 must follow as a named channel");
    }
}
