//! Formula-specific Architecture v3 plan lowering.
//!
//! Parsing/lowering remains in [`super::compute_ir`]. This adapter makes the
//! semantic/hot split explicit: a formula is first lowered to a logical DAG,
//! pure deterministic common subexpressions are interned once, then that DAG is
//! compiled into numeric kernel/input/parameter/buffer/state slots.

use super::ast::{AstNode, OutputModifier};
use super::compute_ir::FormulaComputePlan;
use super::functions::get_builtin_functions;
use crate::buffer_arena::BufferSlot;
use crate::compute::{
    ComputeCapabilities, ComputeEffect, ComputeNode, ComputeNodeId, ComputePlan, ComputePlanError,
    DependencyShape, LookbackRequirement,
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
    hot: std::sync::Arc<HotExecutionPlan>,
    input_bindings: Vec<FormulaInputBinding>,
    outputs: Vec<FormulaOutputBinding>,
    variables: Vec<FormulaVariableBinding>,
    simple_obv_input_names: Option<(String, String)>,
    simple_obv_ma: Option<(usize, usize, usize, usize, usize)>,
    simple_ma_input_period: Option<(String, usize)>,
    simple_rsi_input_period: Option<(String, usize)>,
    simple_zlema_input_period: Option<(String, usize)>,
    simple_pine_mom: Option<(String, usize, usize, Vec<(usize, f64)>)>,
    simple_pine_cci: Option<(usize, usize, usize, usize, usize, usize, Vec<(usize, f64)>)>,
    simple_pine_vwma: Option<(usize, usize, usize, usize, Vec<(usize, f64)>)>,
    simple_pine_vwap: Option<(usize, usize, usize, usize, usize)>,
    simple_pine_wpr: Option<(usize, usize, usize, usize, usize, Vec<(usize, f64)>)>,
    simple_pine_roc: Option<(usize, usize, usize, Vec<(usize, f64)>)>,
    simple_pine_trix: Option<(usize, usize, usize, Vec<(usize, f64)>)>,
    simple_pine_rsi: Option<(usize, usize, usize, usize, Vec<(usize, f64)>)>,
    simple_atr_input_slots: Option<(usize, usize, usize, usize)>,
    simple_multi_ema_assignments: Option<(usize, Vec<(usize, usize)>)>,
    simple_coppock_input_periods: Option<(String, usize, usize, usize)>,
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

/// Compile-time binding from a formula variable to a retained buffer slot.
///
/// The tree path publishes every assignment into
/// [`FormulaContext::variables`](super::types::FormulaContext::variables) — a
/// plain `ASSIGN:`/`OUTPUT:`/`COMPOUND:` all write there — and the language
/// bindings read that map back to build their result dictionaries. A plan that
/// only retained `OUTPUT:` channels would therefore return a *smaller* result
/// set than the tree path for the same source, which is the kind of silent
/// cross-mode divergence this refactor exists to remove.
///
/// The type is shared with [`FormulaOutputBinding`] because a variable binding
/// is exactly the same pair: a name and the slot its series lands in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaVariableBinding {
    name: String,
    context_key: std::sync::Arc<str>,
    slot: BufferSlot,
    output_index: usize,
}

impl FormulaVariableBinding {
    /// Formula variable name, without the `ASSIGN:`/`OUTPUT:` operation prefix.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn context_key(&self) -> &std::sync::Arc<str> {
        &self.context_key
    }

    /// Buffer slot holding this variable's series after a run.
    pub const fn slot(&self) -> BufferSlot {
        self.slot
    }

    /// Index of this slot in the executor's retained output vector.
    pub(crate) const fn output_index(&self) -> usize {
        self.output_index
    }
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
        let builtins = get_builtin_functions();
        for &node_id in semantic.plan().execution_order() {
            let Some(node) = semantic.plan().node(node_id) else {
                continue;
            };
            let Some(name) = node.operation.strip_prefix("CALL:") else {
                continue;
            };
            let context_kernel = matches!(name, "BARSCOUNT" | "BARPOS" | "CAPITAL" | "DRAWNULL");
            if !context_kernel && !builtins.contains_key(name) {
                return Err(FormulaHotPlanError::Semantic(
                    ComputePlanError::UnknownFunction(name.to_string()),
                ));
            }
        }
        let (mut parameters, mut ranges) = bind_numeric_literals(&semantic)?;
        // A string literal cannot put its text in the arena — the arena is
        // numeric — so each `STRING_LITERAL` node carries its index into the
        // plan's ordered literal list instead. The index is what the tree path
        // evaluates the same node to, and the dispatcher adds the context's
        // pre-existing table length so a non-empty `string_table` stays
        // consistent across both paths.
        let string_ranges = bind_string_literals(&semantic, &mut parameters)?;
        ranges.extend(string_ranges);
        let optimized = cse_plan(&semantic, &parameters, &ranges)?;
        let optimized = fuse_scalar_arithmetic(&optimized, &mut ranges)?;
        let lowered = lower_formula_plumbing(&optimized, semantic.root())?;
        let numeric = prune_unreachable(&lowered.plan, &lowered.roots)?;
        let numeric = lower_scalar_number_outputs(&numeric, &lowered.roots)?;
        let hot = std::sync::Arc::new(HotExecutionPlan::compile_with_parameters(
            &numeric,
            lowered.roots,
            parameters,
            ranges,
        )?);
        let input_bindings = compile_input_bindings(&semantic, &hot);
        let outputs = compile_output_bindings(&lowered.named_outputs, &hot);
        let variables = compile_variable_bindings(&lowered.variable_writes, &hot);
        let simple_obv_input_names =
            compile_simple_obv_input_names(&semantic, &input_bindings, &hot);
        let simple_obv_ma =
            compile_simple_obv_ma(&semantic, &input_bindings, &outputs, &variables, &hot);
        let simple_ma_input_period = compile_simple_period_input_call(
            "MA",
            &semantic,
            &input_bindings,
            &outputs,
            &variables,
            &hot,
            false,
        );
        let simple_rsi_input_period = compile_simple_period_input_call(
            "RSI",
            &semantic,
            &input_bindings,
            &outputs,
            &variables,
            &hot,
            false,
        );
        let simple_zlema_input_period = compile_simple_period_input_call(
            "ZLEMA",
            &semantic,
            &input_bindings,
            &outputs,
            &variables,
            &hot,
            true,
        );
        let simple_pine_mom =
            compile_simple_pine_mom(&semantic, &input_bindings, &outputs, &variables, &hot);
        let simple_pine_cci =
            compile_simple_pine_cci(&semantic, &input_bindings, &outputs, &variables, &hot);
        let simple_pine_vwma =
            compile_simple_pine_vwma(&semantic, &input_bindings, &outputs, &variables, &hot);
        let simple_pine_vwap =
            compile_simple_pine_vwap(&semantic, &input_bindings, &outputs, &variables, &hot);
        let simple_pine_wpr =
            compile_simple_pine_wpr(&semantic, &input_bindings, &outputs, &variables, &hot);
        let simple_pine_roc = compile_simple_pine_momentum_plot(
            "ROC",
            "ROCVAL",
            &semantic,
            &input_bindings,
            &outputs,
            &variables,
            &hot,
        );
        let simple_pine_trix = compile_simple_pine_momentum_plot(
            "TRIX",
            "TRIXVAL",
            &semantic,
            &input_bindings,
            &outputs,
            &variables,
            &hot,
        );
        let simple_pine_rsi =
            compile_simple_pine_rsi(&semantic, &input_bindings, &outputs, &variables, &hot);
        let simple_atr_input_slots =
            compile_simple_atr_input_slots(&semantic, &input_bindings, &outputs, &variables, &hot);
        let simple_multi_ema_assignments = compile_simple_multi_ema_assignments(
            &semantic,
            &input_bindings,
            &outputs,
            &variables,
            &hot,
        );
        let simple_coppock_input_periods = compile_simple_coppock_input_periods(
            &semantic,
            &input_bindings,
            &outputs,
            &variables,
            &hot,
        );
        Ok(Self {
            semantic,
            hot,
            input_bindings,
            outputs,
            variables,
            simple_obv_input_names,
            simple_obv_ma,
            simple_ma_input_period,
            simple_rsi_input_period,
            simple_zlema_input_period,
            simple_pine_mom,
            simple_pine_cci,
            simple_pine_vwma,
            simple_pine_vwap,
            simple_pine_wpr,
            simple_pine_roc,
            simple_pine_trix,
            simple_pine_rsi,
            simple_atr_input_slots,
            simple_multi_ema_assignments,
            simple_coppock_input_periods,
        })
    }

    /// Logical DAG retained for diagnostics, optimizer passes and tooling.
    pub const fn semantic(&self) -> &FormulaComputePlan {
        &self.semantic
    }

    /// Numeric execution plan consumed by hot executors.
    pub fn hot(&self) -> &HotExecutionPlan {
        self.hot.as_ref()
    }

    /// Shared immutable hot plan for executors whose dispatcher is call-local.
    pub(crate) fn shared_hot(&self) -> std::sync::Arc<HotExecutionPlan> {
        std::sync::Arc::clone(&self.hot)
    }

    /// Pre-resolved formula-variable bindings used by repeated evaluations.
    pub fn input_bindings(&self) -> &[FormulaInputBinding] {
        &self.input_bindings
    }

    /// Input variable names for the root-only OBV fast path, resolved at
    /// compile time so repeated executions do not rescan the semantic DAG.
    pub(crate) fn simple_obv_input_names(&self) -> Option<(&str, &str)> {
        self.simple_obv_input_names
            .as_ref()
            .map(|(close, volume)| (close.as_str(), volume.as_str()))
    }

    /// Whether this formula's semantic graph invokes the named builtin.
    /// Used by Auto's measured cost policy for kernels whose surrounding plan
    /// overhead outweighed their long-series execution gain in the workload.
    pub(crate) fn contains_call(&self, name: &str) -> bool {
        let graph = self.semantic.plan();
        graph.execution_order().iter().any(|&id| {
            graph.node(id).is_some_and(|node| {
                node.operation
                    .strip_prefix("CALL:")
                    .is_some_and(|call| call.eq_ignore_ascii_case(name))
            })
        })
    }

    /// Exact `OBV_VAL:=OBV(CLOSE,VOL); MAOBV:=MA(OBV_VAL,period);` shape.
    pub(crate) const fn simple_obv_ma(&self) -> Option<(usize, usize, usize, usize, usize)> {
        self.simple_obv_ma
    }

    /// Root-only `MA(variable, integer-literal)` shape, resolved at compile
    /// time so FormulaEngine can bypass per-node dispatch for the common case.
    pub(crate) fn simple_ma_input_period(&self) -> Option<(&str, usize)> {
        self.simple_ma_input_period
            .as_ref()
            .map(|(name, period)| (name.as_str(), *period))
    }

    /// Root-only `RSI(variable, integer-literal)` shape for the primary API.
    pub(crate) fn simple_rsi_input_period(&self) -> Option<(&str, usize)> {
        self.simple_rsi_input_period
            .as_ref()
            .map(|(name, period)| (name.as_str(), *period))
    }

    /// Root-only ZLEMA call, optionally wrapped by one assignment to a retained
    /// variable. Used by the primary-series API without general node dispatch.
    pub(crate) fn simple_zlema_input_period(&self) -> Option<(&str, usize)> {
        self.simple_zlema_input_period
            .as_ref()
            .map(|(name, period)| (name.as_str(), *period))
    }

    /// The single MOM call and retained scalar channels in the Pine Momentum
    /// corpus shape. Its assignments and plot declarations are reconstructed
    /// by FormulaEngine without instantiating the general per-call executor.
    pub(crate) fn simple_pine_mom(&self) -> Option<(&str, usize, usize, &[(usize, f64)])> {
        self.simple_pine_mom
            .as_ref()
            .map(|(name, period, output_index, scalars)| {
                (name.as_str(), *period, *output_index, scalars.as_slice())
            })
    }

    /// Input/output slots for the narrow Pine CCI script shape with one
    /// HLC3 source, one CCI assignment, one plot and two scalar hline levels.
    pub(crate) fn simple_pine_cci(
        &self,
    ) -> Option<(usize, usize, usize, usize, usize, usize, &[(usize, f64)])> {
        self.simple_pine_cci.as_ref().map(
            |(high, low, close, period, cci_output, source_output, scalars)| {
                (
                    *high,
                    *low,
                    *close,
                    *period,
                    *cci_output,
                    *source_output,
                    scalars.as_slice(),
                )
            },
        )
    }

    /// Input/output slots for the single `ta.vwma(close, length)` Pine script.
    pub(crate) fn simple_pine_vwma(&self) -> Option<(usize, usize, usize, usize, &[(usize, f64)])> {
        self.simple_pine_vwma
            .as_ref()
            .map(|(close, volume, period, output, scalars)| {
                (*close, *volume, *period, *output, scalars.as_slice())
            })
    }

    pub(crate) fn simple_pine_vwap(&self) -> Option<(usize, usize, usize, usize, usize)> {
        self.simple_pine_vwap
    }

    /// Input/output slots for the checked-in Pine `ta.wpr(length)` script.
    pub(crate) fn simple_pine_wpr(
        &self,
    ) -> Option<(usize, usize, usize, usize, usize, &[(usize, f64)])> {
        self.simple_pine_wpr
            .as_ref()
            .map(|(high, low, close, period, output, scalars)| {
                (*high, *low, *close, *period, *output, scalars.as_slice())
            })
    }

    /// Input/output slots for the checked-in Pine `ta.roc(close, length)` script.
    pub(crate) fn simple_pine_roc(&self) -> Option<(usize, usize, usize, &[(usize, f64)])> {
        self.simple_pine_roc
            .as_ref()
            .map(|(close, period, output, scalars)| (*close, *period, *output, scalars.as_slice()))
    }

    pub(crate) fn simple_pine_trix(&self) -> Option<(usize, usize, usize, &[(usize, f64)])> {
        self.simple_pine_trix
            .as_ref()
            .map(|(close, period, output, scalars)| (*close, *period, *output, scalars.as_slice()))
    }

    pub(crate) fn simple_pine_rsi(&self) -> Option<(usize, usize, usize, usize, &[(usize, f64)])> {
        self.simple_pine_rsi
            .as_ref()
            .map(|(input, period, output, source, scalars)| {
                (*input, *period, *output, *source, scalars.as_slice())
            })
    }

    /// The input slots and period for an eligible root `ATR(HIGH, LOW, CLOSE, N)`.
    pub(crate) const fn simple_atr_input_slots(&self) -> Option<(usize, usize, usize, usize)> {
        self.simple_atr_input_slots
    }

    /// Three standalone EMA assignments over one input, with their periods
    /// and retained output indexes resolved at compile time.
    pub(crate) fn simple_multi_ema_assignments(&self) -> Option<(usize, &[(usize, usize)])> {
        self.simple_multi_ema_assignments
            .as_ref()
            .map(|(input, assignments)| (*input, assignments.as_slice()))
    }

    /// A sole Coppock call assigned to one formula variable, with its three
    /// periods fixed at compile time. This lets FormulaEngine reuse the
    /// canonical indicator kernel without constructing a per-call dispatcher.
    pub(crate) fn simple_coppock_input_periods(&self) -> Option<(&str, usize, usize, usize)> {
        self.simple_coppock_input_periods
            .as_ref()
            .map(|(name, wma_period, long_roc, short_roc)| {
                (name.as_str(), *wma_period, *long_roc, *short_roc)
            })
    }

    /// Named output channels in declaration order.
    ///
    /// The first value of an execution is always the formula's primary result
    /// (the value of its last value-producing statement); these bindings let a
    /// caller address every channel by name instead of only that one.
    ///
    /// The name is the spelling used **in the source**, not the canonical
    /// (upper-cased) form the internal `OUTPUT:` operation carries: the tree path
    /// keys `ctx.output_names` by the source spelling and the parser preserves
    /// case, so publishing the canonical form here would make the two modes
    /// disagree for a source such as `golden: CROSS(a, b)`. Callers that need to
    /// match a name case-insensitively — as
    /// [`crate::factor_graph::FactorGraphPlan::node_index`] does — must canonicalize
    /// both sides.
    pub fn outputs(&self) -> &[FormulaOutputBinding] {
        &self.outputs
    }

    /// Every variable this formula assigns, with the slot holding its series.
    ///
    /// This mirrors the entries the tree path leaves in
    /// [`FormulaContext::variables`](super::types::FormulaContext::variables):
    /// `ASSIGN:`, `OUTPUT:` and `COMPOUND:` writes all appear, in first-write
    /// order, with a later write to the same name replacing the earlier one.
    /// The language bindings read that map back after an evaluation, so a plan
    /// that did not expose these would return fewer results than the tree path
    /// for the same source.
    pub fn variables(&self) -> &[FormulaVariableBinding] {
        &self.variables
    }

    /// Chart styling for each declared output, keyed by the name as written.
    ///
    /// The FFI layer reads `ctx.output_modifiers` to describe a series' colour
    /// and line style, so the plan path publishes the same map. It is not part
    /// of the numeric plan, which is why it is read straight off the semantic
    /// plan instead of being a field here.
    pub fn output_modifiers(&self) -> &BTreeMap<String, OutputModifier> {
        self.semantic.output_modifiers()
    }
}

/// Identify the common two-step OBV → SMA assignment chain once at compile time.
fn compile_simple_obv_ma(
    semantic: &FormulaComputePlan,
    input_bindings: &[FormulaInputBinding],
    outputs: &[FormulaOutputBinding],
    variables: &[FormulaVariableBinding],
    hot: &HotExecutionPlan,
) -> Option<(usize, usize, usize, usize, usize)> {
    if input_bindings.len() != 2
        || !outputs.is_empty()
        || variables.len() != 2
        || hot.output_layout().len() != 2
        || !semantic.output_modifiers().is_empty()
        || !semantic.string_literals_ordered().is_empty()
    {
        return None;
    }
    let close_slot = input_bindings
        .iter()
        .find(|binding| binding.name().eq_ignore_ascii_case("CLOSE"))?
        .slot()
        .0;
    let volume_slot = input_bindings
        .iter()
        .find(|binding| binding.name().eq_ignore_ascii_case("VOL"))?
        .slot()
        .0;
    if close_slot == volume_slot {
        return None;
    }
    let graph = semantic.plan();
    let root = graph.node(semantic.root())?;
    if root.operation != "STATEMENTS" || root.dependencies.len() != 2 {
        return None;
    }

    let mut close_id = None;
    let mut volume_id = None;
    let mut obv_call = None;
    let mut obv_assignment = None;
    let mut obv_variable = None;
    let mut ma_call = None;
    let mut ma_assignment = None;
    let mut period_node = None;
    for &id in graph.execution_order() {
        let node = graph.node(id)?;
        match node.operation.as_str() {
            "VARIABLE:CLOSE" if close_id.is_none() => close_id = Some(id),
            "VARIABLE:VOL" if volume_id.is_none() => volume_id = Some(id),
            "CALL:OBV" if obv_call.is_none() && node.dependencies.len() == 2 => obv_call = Some(id),
            "ASSIGN:OBV_VAL" if obv_assignment.is_none() && !node.dependencies.is_empty() => {
                obv_assignment = Some((id, node.dependencies.clone()))
            }
            "VARIABLE:OBV_VAL" if obv_variable.is_none() && node.dependencies.len() == 1 => {
                obv_variable = Some((id, node.dependencies[0]))
            }
            "CALL:MA" if ma_call.is_none() && node.dependencies.len() == 2 => ma_call = Some(id),
            "ASSIGN:MAOBV" if ma_assignment.is_none() && !node.dependencies.is_empty() => {
                ma_assignment = Some((id, node.dependencies.clone()))
            }
            "NUMBER" if period_node.is_none() => period_node = Some(id),
            "STATEMENTS" => {}
            _ => return None,
        }
    }
    let close_id = close_id?;
    let volume_id = volume_id?;
    let obv_call_id = obv_call?;
    let (obv_assignment_id, obv_assignment_dependencies) = obv_assignment?;
    let (obv_variable_id, obv_variable_assignment) = obv_variable?;
    let ma_call_id = ma_call?;
    let (ma_assignment_id, ma_assignment_dependencies) = ma_assignment?;
    let period_node = period_node?;
    let period = semantic.number_literal(period_node)?;
    if !period.is_finite() || period < 1.0 || period > usize::MAX as f64 || period.fract() != 0.0 {
        return None;
    }
    let obv_node = graph.node(obv_call_id)?;
    let ma_node = graph.node(ma_call_id)?;
    if !obv_node.capabilities.effect.is_pure()
        || !ma_node.capabilities.effect.is_pure()
        || obv_node.dependencies.as_slice() != [close_id, volume_id]
        || obv_assignment_dependencies.first().copied() != Some(obv_call_id)
        || obv_variable_assignment != obv_assignment_id
        || ma_node.dependencies.as_slice() != [obv_variable_id, period_node]
        || ma_assignment_dependencies.first().copied() != Some(ma_call_id)
        || ma_assignment_dependencies.get(1).copied() != Some(obv_assignment_id)
        || !root.dependencies.contains(&obv_assignment_id)
        || !root.dependencies.contains(&ma_assignment_id)
    {
        return None;
    }
    let obv_index = variables
        .iter()
        .find(|binding| binding.name() == "OBV_VAL")?
        .output_index();
    let ma_index = variables
        .iter()
        .find(|binding| binding.name() == "MAOBV")?
        .output_index();
    if obv_index != 1 || ma_index != 0 {
        return None;
    }
    Some((
        close_slot,
        volume_slot,
        period as usize,
        obv_index,
        ma_index,
    ))
}

/// Identify the narrow root-only OBV case once, rather than walking the
/// semantic plan on every evaluation.
fn compile_simple_obv_input_names(
    semantic: &FormulaComputePlan,
    input_bindings: &[FormulaInputBinding],
    hot: &HotExecutionPlan,
) -> Option<(String, String)> {
    let graph = semantic.plan();
    if graph.execution_order().len() < 3
        || hot.output_layout().len() != 1
        || input_bindings.len() != 2
    {
        return None;
    }
    let first = input_bindings
        .iter()
        .find(|binding| binding.slot().0 == 0)?;
    let second = input_bindings
        .iter()
        .find(|binding| binding.slot().0 == 1)?;
    let mut obv_calls = 0usize;
    for &id in graph.execution_order() {
        let node = graph.node(id)?;
        match node.operation.as_str() {
            "CALL:OBV" => {
                obv_calls += 1;
                if node.dependencies.len() != 2
                    || !node.capabilities.effect.is_pure()
                    || graph.node(node.dependencies[0]).is_none_or(|input| {
                        input.operation.strip_prefix("VARIABLE:") != Some(first.name())
                    })
                    || graph.node(node.dependencies[1]).is_none_or(|input| {
                        input.operation.strip_prefix("VARIABLE:") != Some(second.name())
                    })
                {
                    return None;
                }
            }
            "STRING_LITERAL" => {
                if !node.dependencies.is_empty() {
                    return None;
                }
            }
            "STATEMENTS" => {}
            operation
                if operation.starts_with("ASSIGN:")
                    || operation.starts_with("OUTPUT:")
                    || operation.starts_with("VARIABLE:") => {}
            _ => return None,
        }
    }
    (obv_calls == 1).then(|| (first.name().to_owned(), second.name().to_owned()))
}

/// Identify a root single-input call with a positive integer literal period.
/// Optionally accepts one direct assignment to a single retained variable.
/// The engine can execute its known kernel without constructing a per-evaluation
/// dispatcher and runtime context.
fn compile_simple_period_input_call(
    function: &str,
    semantic: &FormulaComputePlan,
    input_bindings: &[FormulaInputBinding],
    outputs: &[FormulaOutputBinding],
    variables: &[FormulaVariableBinding],
    hot: &HotExecutionPlan,
    allow_single_assignment: bool,
) -> Option<(String, usize)> {
    if input_bindings.len() != 1
        || hot.output_layout().len() != 1
        || !outputs.is_empty()
        || (!variables.is_empty()
            && (!allow_single_assignment
                || variables.len() != 1
                || variables[0].output_index() != 0))
        || !semantic.output_modifiers().is_empty()
        || !semantic.string_literals_ordered().is_empty()
    {
        return None;
    }
    let input = &input_bindings[0];
    let plan = semantic.plan();
    let mut call_id = None;
    let mut assignment_id = None;
    for &id in plan.execution_order() {
        let node = plan.node(id)?;
        match node.operation.as_str() {
            operation
                if operation
                    .strip_prefix("CALL:")
                    .is_some_and(|name| name.eq_ignore_ascii_case(function))
                    && call_id.is_none()
                    && node.dependencies.len() == 2 =>
            {
                call_id = Some(id);
            }
            "NUMBER" => {
                let value = semantic.number_literal(id)?;
                if !value.is_finite()
                    || value < 1.0
                    || value > usize::MAX as f64
                    || (value.fract() != 0.0)
                {
                    return None;
                }
            }
            "STATEMENTS" => {}
            operation if allow_single_assignment && operation.starts_with("ASSIGN:") => {
                let variable = variables.first()?;
                if assignment_id.is_some()
                    || !operation
                        .strip_prefix("ASSIGN:")
                        .is_some_and(|name| name.eq_ignore_ascii_case(variable.name()))
                {
                    return None;
                }
                assignment_id = Some(id);
            }
            operation
                if operation
                    .strip_prefix("VARIABLE:")
                    .is_some_and(|name| name.eq_ignore_ascii_case(input.name())) => {}
            _ => return None,
        }
    }
    let call_id = call_id?;
    if allow_single_assignment && variables.len() != usize::from(assignment_id.is_some()) {
        return None;
    }
    let call = plan.node(call_id)?;
    let input_node = plan.node(call.dependencies[0])?;
    if !input_node
        .operation
        .strip_prefix("VARIABLE:")
        .is_some_and(|name| name.eq_ignore_ascii_case(input.name()))
    {
        return None;
    }
    let period_value = semantic.number_literal(call.dependencies[1])?;
    if !period_value.is_finite()
        || period_value < 1.0
        || period_value > usize::MAX as f64
        || period_value.fract() != 0.0
    {
        return None;
    }
    let root = plan.node(semantic.root())?;
    let expected_root = assignment_id.unwrap_or(call_id);
    if semantic.root() != expected_root
        && !(root.operation == "STATEMENTS"
            && root.dependencies.len() == 1
            && root.dependencies[0] == expected_root)
    {
        return None;
    }
    if let Some(assignment_id) = assignment_id {
        let assignment = plan.node(assignment_id)?;
        if assignment.dependencies.as_slice() != [call_id] {
            return None;
        }
    }
    if allow_single_assignment
        && (plan.len() != 4 + usize::from(root.operation == "STATEMENTS")
            || (variables.is_empty() && assignment_id.is_some()))
    {
        return None;
    }
    Some((input.name().to_owned(), period_value as usize))
}

/// Recognize the checked-in Pine Momentum shape: one `MOM(CLOSE, N)` call,
/// assigned to the plotted result, plus scalar `input(N)` and `hline(0)` slots.
/// All retained outputs are described so the primary-only engine can publish
/// the same variables without running the generic dispatcher.
fn compile_simple_pine_mom(
    semantic: &FormulaComputePlan,
    input_bindings: &[FormulaInputBinding],
    outputs: &[FormulaOutputBinding],
    variables: &[FormulaVariableBinding],
    hot: &HotExecutionPlan,
) -> Option<(String, usize, usize, Vec<(usize, f64)>)> {
    if input_bindings.len() != 1
        || hot.output_layout().len() != 3
        || outputs.len() != 2
        || variables.len() != 4
        || outputs[0].name() != "PLOT"
        || outputs[0].is_level_marker()
        || outputs[1].name() != "HLINE"
        || !outputs[1].is_level_marker()
    {
        return None;
    }

    let input = &input_bindings[0];
    if !input.name().eq_ignore_ascii_case("CLOSE") {
        return None;
    }
    for name in ["LENGTH", "MOMVAL", "PLOT", "HLINE"] {
        if !variables.iter().any(|binding| binding.name() == name) {
            return None;
        }
    }

    let graph = semantic.plan();
    let mut call_id = None;
    let mut period_id = None;
    let mut length_assignment = None;
    let mut mom_assignment = None;
    let mut plot_id = None;
    let mut hline_id = None;
    let mut zero_id = None;

    for &id in graph.execution_order() {
        let node = graph.node(id)?;
        match node.operation.as_str() {
            "CALL:MOM" if call_id.is_none() && node.dependencies.len() == 2 => {
                call_id = Some(id);
                period_id = Some(node.dependencies[1]);
            }
            "NUMBER" => {
                let value = semantic.number_literal(id)?;
                if value == 0.0 {
                    if zero_id.replace(id).is_some() {
                        return None;
                    }
                }
            }
            "ASSIGN:LENGTH" if length_assignment.is_none() && node.dependencies.len() == 1 => {
                length_assignment = Some((id, node.dependencies[0]));
            }
            "ASSIGN:MOMVAL" if mom_assignment.is_none() && node.dependencies.len() == 2 => {
                mom_assignment = Some((id, node.dependencies.clone()));
            }
            "OUTPUT:PLOT" if plot_id.is_none() => plot_id = Some(id),
            "OUTPUT:HLINE" if hline_id.is_none() => hline_id = Some(id),
            "STRING_LITERAL" | "STATEMENTS" | "VARIABLE:CLOSE" | "VARIABLE:MOMVAL" => {}
            _ => return None,
        }
    }

    let call_id = call_id?;
    let period_id = period_id?;
    let period = semantic.number_literal(period_id)?;
    if !period.is_finite() || period < 1.0 || period > usize::MAX as f64 || period.fract() != 0.0 {
        return None;
    }
    let call = graph.node(call_id)?;
    if !call.capabilities.effect.is_pure()
        || !graph
            .node(call.dependencies[0])?
            .operation
            .strip_prefix("VARIABLE:")
            .is_some_and(|name| name.eq_ignore_ascii_case(input.name()))
    {
        return None;
    }

    let (length_assignment_id, length_number_id) = length_assignment?;
    let (mom_assignment_id, mom_dependencies) = mom_assignment?;
    if semantic.number_literal(length_number_id)? != period
        || mom_dependencies.as_slice() != [call_id, length_assignment_id]
    {
        return None;
    }
    let mom_value_id = graph.execution_order().iter().copied().find(|&id| {
        graph.node(id).is_some_and(|node| {
            node.operation == "VARIABLE:MOMVAL"
                && node.dependencies.as_slice() == [mom_assignment_id]
        })
    })?;
    let plot_id = plot_id?;
    let plot = graph.node(plot_id)?;
    if plot.dependencies.first().copied()? != mom_value_id
        || !graph
            .node(mom_value_id)?
            .operation
            .strip_prefix("VARIABLE:")
            .is_some_and(|name| name == "MOMVAL")
    {
        return None;
    }
    let hline_id = hline_id?;
    let hline = graph.node(hline_id)?;
    let zero_id = zero_id?;
    if hline.dependencies.first().copied()? != zero_id || semantic.number_literal(zero_id)? != 0.0 {
        return None;
    }

    let layout = hot.output_layout().outputs();
    let call_index = layout.iter().position(|(node, _)| *node == call_id)?;
    if call_index != 0 {
        return None;
    }
    let mut scalar_outputs = Vec::with_capacity(layout.len() - 1);
    for (index, (node, _)) in layout.iter().enumerate() {
        if *node == call_id {
            continue;
        }
        let value = semantic.number_literal(*node)?;
        scalar_outputs.push((index, value));
    }
    if scalar_outputs.len() != layout.len() - 1 {
        return None;
    }

    Some((
        input.name().to_owned(),
        period as usize,
        call_index,
        scalar_outputs,
    ))
}

/// Recognize the checked-in Pine CCI script's single HLC3 → CCI pipeline.
/// FormulaEngine can compute the source and CCI directly while still
/// publishing the script's assignment and scalar variables.
fn compile_simple_pine_cci(
    semantic: &FormulaComputePlan,
    input_bindings: &[FormulaInputBinding],
    outputs: &[FormulaOutputBinding],
    variables: &[FormulaVariableBinding],
    hot: &HotExecutionPlan,
) -> Option<(usize, usize, usize, usize, usize, usize, Vec<(usize, f64)>)> {
    if input_bindings.len() != 3
        || hot.output_layout().len() != 4
        || outputs.len() != 3
        || variables.len() != 5
        || outputs[0].name() != "PLOT"
        || outputs[0].is_level_marker()
        || outputs[1].name() != "HLINE"
        || !outputs[1].is_level_marker()
        || outputs[2].name() != "HLINE"
        || !outputs[2].is_level_marker()
    {
        return None;
    }

    let input_slot = |name: &str| {
        input_bindings
            .iter()
            .find(|binding| binding.name().eq_ignore_ascii_case(name))
            .map(|binding| binding.slot().0)
    };
    let high_slot = input_slot("HIGH")?;
    let low_slot = input_slot("LOW")?;
    let close_slot = input_slot("CLOSE")?;
    if [high_slot, low_slot, close_slot]
        .into_iter()
        .collect::<BTreeSet<_>>()
        .len()
        != 3
    {
        return None;
    }
    for name in ["LENGTH", "SRC", "CCIVAL", "PLOT", "HLINE"] {
        if !variables.iter().any(|binding| binding.name() == name) {
            return None;
        }
    }

    let graph = semantic.plan();
    let mut cci_call = None;
    let mut source_assignment = None;
    let mut cci_assignment = None;
    let mut length_assignment = None;
    let mut additions = Vec::with_capacity(2);
    let mut division = None;
    for &id in graph.execution_order() {
        let node = graph.node(id)?;
        match node.operation.as_str() {
            "CALL:CCI" if cci_call.is_none() && node.dependencies.len() == 2 => {
                cci_call = Some(id);
            }
            "BINARY:Add" if additions.len() < 2 => additions.push(id),
            "BINARY:Div" if division.is_none() && node.dependencies.len() == 2 => {
                division = Some(id);
            }
            "ASSIGN:LENGTH" if length_assignment.is_none() && node.dependencies.len() == 1 => {
                length_assignment = Some((id, node.dependencies[0]));
            }
            "ASSIGN:SRC" if source_assignment.is_none() && !node.dependencies.is_empty() => {
                source_assignment = Some((id, node.dependencies[0]));
            }
            "ASSIGN:CCIVAL" if cci_assignment.is_none() && !node.dependencies.is_empty() => {
                cci_assignment = Some((id, node.dependencies[0]));
            }
            "VARIABLE:HIGH" | "VARIABLE:LOW" | "VARIABLE:CLOSE" | "VARIABLE:SRC"
            | "VARIABLE:CCIVAL" | "OUTPUT:PLOT" | "OUTPUT:HLINE" | "STRING_LITERAL" | "NUMBER"
            | "STATEMENTS" => {}
            _ => return None,
        }
    }

    let cci_call = cci_call?;
    let cci = graph.node(cci_call)?;
    if !cci.capabilities.effect.is_pure() {
        return None;
    }
    let period = semantic.number_literal(*cci.dependencies.get(1)?)?;
    if !period.is_finite() || period < 1.0 || period > usize::MAX as f64 || period.fract() != 0.0 {
        return None;
    }

    let (length_assignment_id, length_number_id) = length_assignment?;
    if semantic.number_literal(length_number_id)? != period {
        return None;
    }
    let (source_assignment_id, source_value_id) = source_assignment?;
    let (cci_assignment_id, cci_assignment_call) = cci_assignment?;
    if cci_assignment_call != cci_call {
        return None;
    }
    let division_id = division?;
    let source_assignment_node = graph.node(source_assignment_id)?;
    if source_value_id != division_id
        || source_assignment_id == cci_assignment_id
        || source_assignment_node.dependencies.as_slice() != [division_id, length_assignment_id]
    {
        return None;
    }
    let source_division = graph.node(division_id)?;
    if source_division.dependencies.first().copied()? != additions.get(1).copied()?
        || semantic.number_literal(*source_division.dependencies.get(1)?)? != 3.0
        || graph
            .node(source_assignment_id)?
            .dependencies
            .first()
            .copied()?
            != division_id
    {
        return None;
    }
    let sum_high_low = graph.node(*additions.first()?)?;
    let sum_hlc = graph.node(*additions.get(1)?)?;
    if sum_high_low.dependencies.len() != 2
        || graph
            .node(*sum_high_low.dependencies.first()?)?
            .operation
            .strip_prefix("VARIABLE:")?
            != "HIGH"
        || graph
            .node(*sum_high_low.dependencies.get(1)?)?
            .operation
            .strip_prefix("VARIABLE:")?
            != "LOW"
        || sum_hlc.dependencies.first().copied()? != additions.first().copied()?
        || graph
            .node(*sum_hlc.dependencies.get(1)?)?
            .operation
            .strip_prefix("VARIABLE:")?
            != "CLOSE"
    {
        return None;
    }
    if graph.node(*cci.dependencies.first()?)?.operation != "VARIABLE:SRC"
        || graph
            .node(*cci.dependencies.first()?)?
            .dependencies
            .as_slice()
            != [source_assignment_id]
    {
        return None;
    }
    let assignment = graph.node(cci_assignment_id)?;
    if assignment.dependencies.as_slice() != [cci_call, source_assignment_id] {
        return None;
    }

    let layout = hot.output_layout().outputs();
    let cci_output = layout.iter().position(|(node, _)| *node == cci_call)?;
    let source_output = layout.iter().position(|(node, _)| *node == division_id)?;
    if cci_output != 0 || cci_output == source_output {
        return None;
    }
    let mut scalar_outputs = Vec::with_capacity(layout.len() - 2);
    for (index, (node, _)) in layout.iter().enumerate() {
        if index == cci_output || index == source_output {
            continue;
        }
        scalar_outputs.push((index, semantic.number_literal(*node)?));
    }
    if scalar_outputs.len() != layout.len() - 2 {
        return None;
    }
    let output_index = |name: &str| {
        variables
            .iter()
            .find(|binding| binding.name() == name)
            .map(FormulaVariableBinding::output_index)
    };
    let length_output = output_index("LENGTH")?;
    let hline_output = output_index("HLINE")?;
    if output_index("CCIVAL")? != cci_output
        || output_index("PLOT")? != cci_output
        || output_index("SRC")? != source_output
        || !scalar_outputs
            .iter()
            .any(|(index, _)| *index == length_output)
        || !scalar_outputs
            .iter()
            .any(|(index, _)| *index == hline_output)
    {
        return None;
    }

    Some((
        high_slot,
        low_slot,
        close_slot,
        period as usize,
        cci_output,
        source_output,
        scalar_outputs,
    ))
}

fn compile_simple_pine_vwma(
    semantic: &FormulaComputePlan,
    input_bindings: &[FormulaInputBinding],
    outputs: &[FormulaOutputBinding],
    variables: &[FormulaVariableBinding],
    hot: &HotExecutionPlan,
) -> Option<(usize, usize, usize, usize, Vec<(usize, f64)>)> {
    if input_bindings.len() != 2
        || hot.output_layout().len() != 2
        || outputs.len() != 1
        || variables.len() != 3
        || outputs[0].name() != "PLOT"
        || outputs[0].is_level_marker()
    {
        return None;
    }
    let slot = |name: &str| {
        input_bindings
            .iter()
            .find(|binding| binding.name().eq_ignore_ascii_case(name))
            .map(|binding| binding.slot().0)
    };
    let close_slot = slot("CLOSE")?;
    let volume_slot = slot("VOL")?;
    if close_slot == volume_slot {
        return None;
    }
    for name in ["LENGTH", "VWMAVAL", "PLOT"] {
        if !variables.iter().any(|binding| binding.name() == name) {
            return None;
        }
    }

    let graph = semantic.plan();
    let mut call_id = None;
    let mut length_assignment = None;
    let mut value_assignment = None;
    let mut plot_id = None;
    for &id in graph.execution_order() {
        let node = graph.node(id)?;
        match node.operation.as_str() {
            "CALL:VWMA" if call_id.is_none() && node.dependencies.len() == 3 => {
                call_id = Some(id);
            }
            "ASSIGN:LENGTH" if length_assignment.is_none() && node.dependencies.len() == 1 => {
                length_assignment = Some((id, node.dependencies[0]));
            }
            "ASSIGN:VWMAVAL" if value_assignment.is_none() && !node.dependencies.is_empty() => {
                value_assignment = Some((id, node.dependencies[0]));
            }
            "OUTPUT:PLOT" if plot_id.is_none() => plot_id = Some(id),
            "STRING_LITERAL" | "NUMBER" | "STATEMENTS" | "VARIABLE:CLOSE" | "VARIABLE:VOL"
            | "VARIABLE:VWMAVAL" => {}
            _ => return None,
        }
    }
    let call_id = call_id?;
    let call = graph.node(call_id)?;
    if !call.capabilities.effect.is_pure()
        || graph.node(*call.dependencies.first()?)?.operation != "VARIABLE:CLOSE"
        || graph.node(*call.dependencies.get(1)?)?.operation != "VARIABLE:VOL"
    {
        return None;
    }
    let period = semantic.number_literal(*call.dependencies.get(2)?)?;
    if !period.is_finite() || period < 1.0 || period > usize::MAX as f64 || period.fract() != 0.0 {
        return None;
    }
    let (length_assignment_id, length_number) = length_assignment?;
    if semantic.number_literal(length_number)? != period {
        return None;
    }
    let (value_assignment_id, value_assignment_call) = value_assignment?;
    if value_assignment_call != call_id
        || graph.node(value_assignment_id)?.dependencies.as_slice()
            != [call_id, length_assignment_id]
    {
        return None;
    }
    let value_id = graph.execution_order().iter().copied().find(|&id| {
        graph.node(id).is_some_and(|node| {
            node.operation == "VARIABLE:VWMAVAL"
                && node.dependencies.as_slice() == [value_assignment_id]
        })
    })?;
    let plot_id = plot_id?;
    if graph.node(plot_id)?.dependencies.first().copied()? != value_id {
        return None;
    }
    let layout = hot.output_layout().outputs();
    let output_index = layout.iter().position(|(node, _)| *node == call_id)?;
    if output_index != 0
        || variables
            .iter()
            .find(|binding| binding.name() == "VWMAVAL")?
            .output_index()
            != output_index
        || variables
            .iter()
            .find(|binding| binding.name() == "PLOT")?
            .output_index()
            != output_index
    {
        return None;
    }
    let length_index = variables
        .iter()
        .find(|binding| binding.name() == "LENGTH")?
        .output_index();
    let mut scalar_outputs = Vec::with_capacity(layout.len() - 1);
    for (index, (node, _)) in layout.iter().enumerate() {
        if index != output_index {
            scalar_outputs.push((index, semantic.number_literal(*node)?));
        }
    }
    if !scalar_outputs
        .iter()
        .any(|(index, _)| *index == length_index)
    {
        return None;
    }
    Some((
        close_slot,
        volume_slot,
        period as usize,
        output_index,
        scalar_outputs,
    ))
}

/// Recognize the corpus's HLC3 → cumulative VWAP Pine assignment and plot.
/// The narrow shape lets primary-only Formula Plan runs fuse the source and
/// cumulative-volume loops while still publishing the same three variables.
fn compile_simple_pine_vwap(
    semantic: &FormulaComputePlan,
    input_bindings: &[FormulaInputBinding],
    outputs: &[FormulaOutputBinding],
    variables: &[FormulaVariableBinding],
    hot: &HotExecutionPlan,
) -> Option<(usize, usize, usize, usize, usize)> {
    if input_bindings.len() != 4
        || outputs.len() != 1
        || variables.len() != 3
        || outputs[0].name() != "PLOT"
        || outputs[0].is_level_marker()
    {
        return None;
    }
    let input_slot = |name: &str| {
        input_bindings
            .iter()
            .find(|binding| binding.name().eq_ignore_ascii_case(name))
            .map(|binding| binding.slot().0)
    };
    let high_slot = input_slot("HIGH")?;
    let low_slot = input_slot("LOW")?;
    let close_slot = input_slot("CLOSE")?;
    let volume_slot = input_slot("VOL")?;
    if [high_slot, low_slot, close_slot, volume_slot]
        .into_iter()
        .collect::<BTreeSet<_>>()
        .len()
        != 4
        || !["SRC", "VWAPVALUE", "PLOT"]
            .iter()
            .all(|name| variables.iter().any(|binding| binding.name() == *name))
    {
        return None;
    }

    let graph = semantic.plan();
    let (mut high_low_add, mut hlc_add, mut divide, mut source_assignment) =
        (None, None, None, None);
    let (mut call, mut value_assignment, mut plot) = (None, None, None);
    let mut divisor_number = None;
    for &id in graph.execution_order() {
        let node = graph.node(id)?;
        match node.operation.as_str() {
            "BINARY:Add" if node.dependencies.len() == 2 => {
                let left = graph.node(node.dependencies[0])?.operation.as_str();
                if left.starts_with("VARIABLE:") && high_low_add.is_none() {
                    high_low_add = Some(id);
                } else if hlc_add.is_none() {
                    hlc_add = Some(id);
                } else {
                    return None;
                }
            }
            "BINARY:Div" if divide.is_none() && node.dependencies.len() == 1 => {
                divide = Some(id);
            }
            "CALL:VWAP" if call.is_none() && node.dependencies.len() == 2 => call = Some(id),
            "ASSIGN:SRC" if source_assignment.is_none() && node.dependencies.len() == 1 => {
                source_assignment = Some(id);
            }
            "ASSIGN:VWAPVALUE" if value_assignment.is_none() && !node.dependencies.is_empty() => {
                value_assignment = Some(id);
            }
            "OUTPUT:PLOT" if plot.is_none() => plot = Some(id),
            "NUMBER" => {
                if divisor_number.is_some() || semantic.number_literal(id)? != 3.0 {
                    return None;
                }
                divisor_number = Some(id);
            }
            "STRING_LITERAL" | "STATEMENTS" | "VARIABLE:HIGH" | "VARIABLE:LOW"
            | "VARIABLE:CLOSE" | "VARIABLE:VOL" | "VARIABLE:SRC" | "VARIABLE:VWAPVALUE" => {}
            _ => return None,
        }
    }

    let high_low_add = high_low_add?;
    let hlc_add = hlc_add?;
    let divide = divide?;
    let source_assignment = source_assignment?;
    let call = call?;
    let value_assignment = value_assignment?;
    let plot = plot?;
    let hl_node = graph.node(high_low_add)?;
    if graph.node(hl_node.dependencies[0])?.operation != "VARIABLE:HIGH"
        || graph.node(hl_node.dependencies[1])?.operation != "VARIABLE:LOW"
    {
        return None;
    }
    let hlc_node = graph.node(hlc_add)?;
    if hlc_node.dependencies.first().copied() != Some(high_low_add)
        || graph.node(hlc_node.dependencies[1])?.operation != "VARIABLE:CLOSE"
    {
        return None;
    }
    let divide_node = graph.node(divide)?;
    if divisor_number.is_none()
        || divide_node.dependencies[0] != hlc_add
        || graph.node(source_assignment)?.dependencies.as_slice() != [divide]
    {
        return None;
    }
    let source_ref = graph.execution_order().iter().copied().find(|id| {
        graph.node(*id).is_some_and(|node| {
            node.operation == "VARIABLE:SRC" && node.dependencies.as_slice() == [source_assignment]
        })
    })?;
    let call_node = graph.node(call)?;
    if !call_node.capabilities.effect.is_pure()
        || call_node.dependencies[0] != source_ref
        || graph.node(call_node.dependencies[0])?.operation != "VARIABLE:SRC"
        || graph
            .node(call_node.dependencies[0])?
            .dependencies
            .as_slice()
            != [source_assignment]
        || graph.node(call_node.dependencies[1])?.operation != "VARIABLE:VOL"
        || graph.node(value_assignment)?.dependencies.as_slice() != [call, source_assignment]
        || graph.node(plot)?.dependencies.first().copied()
            != graph.execution_order().iter().copied().find(|id| {
                graph.node(*id).is_some_and(|node| {
                    node.operation == "VARIABLE:VWAPVALUE"
                        && node.dependencies.as_slice() == [value_assignment]
                })
            })
    {
        return None;
    }

    let layout = hot.output_layout().outputs();
    let value_index = layout.iter().position(|(node, _)| *node == call)?;
    let source_index = layout.iter().position(|(node, _)| *node == divide)?;
    if value_index != 0
        || variables
            .iter()
            .find(|binding| binding.name() == "VWAPVALUE")?
            .output_index()
            != value_index
        || variables
            .iter()
            .find(|binding| binding.name() == "PLOT")?
            .output_index()
            != value_index
        || variables
            .iter()
            .find(|binding| binding.name() == "SRC")?
            .output_index()
            != source_index
    {
        return None;
    }
    Some((high_slot, low_slot, close_slot, volume_slot, source_index))
}

fn compile_simple_pine_wpr(
    semantic: &FormulaComputePlan,
    input_bindings: &[FormulaInputBinding],
    outputs: &[FormulaOutputBinding],
    variables: &[FormulaVariableBinding],
    hot: &HotExecutionPlan,
) -> Option<(usize, usize, usize, usize, usize, Vec<(usize, f64)>)> {
    if input_bindings.len() != 3
        || hot.output_layout().len() != 4
        || outputs.len() != 3
        || variables.len() != 4
        || outputs[0].name() != "PLOT"
        || outputs[0].is_level_marker()
        || outputs[1].name() != "HLINE"
        || !outputs[1].is_level_marker()
        || outputs[2].name() != "HLINE"
        || !outputs[2].is_level_marker()
    {
        return None;
    }
    let input_slot = |name: &str| {
        input_bindings
            .iter()
            .find(|binding| binding.name().eq_ignore_ascii_case(name))
            .map(|binding| binding.slot().0)
    };
    let high_slot = input_slot("HIGH")?;
    let low_slot = input_slot("LOW")?;
    let close_slot = input_slot("CLOSE")?;
    if [high_slot, low_slot, close_slot]
        .into_iter()
        .collect::<BTreeSet<_>>()
        .len()
        != 3
        || !["LENGTH", "WR", "PLOT", "HLINE"]
            .into_iter()
            .all(|name| variables.iter().any(|binding| binding.name() == name))
    {
        return None;
    }

    let graph = semantic.plan();
    let mut call_id = None;
    let mut length_assignment = None;
    let mut value_assignment = None;
    let mut plot_id = None;
    let mut hline_ids = Vec::with_capacity(2);
    for &id in graph.execution_order() {
        let node = graph.node(id)?;
        match node.operation.as_str() {
            "CALL:WILLR" if call_id.is_none() && node.dependencies.len() == 4 => {
                call_id = Some(id);
            }
            "ASSIGN:LENGTH" if length_assignment.is_none() && node.dependencies.len() == 1 => {
                length_assignment = Some((id, node.dependencies[0]));
            }
            "ASSIGN:WR" if value_assignment.is_none() && node.dependencies.len() == 2 => {
                value_assignment = Some((id, node.dependencies.clone()));
            }
            "OUTPUT:PLOT" if plot_id.is_none() => plot_id = Some(id),
            "OUTPUT:HLINE" if hline_ids.len() < 2 => hline_ids.push(id),
            "STRING_LITERAL" | "NUMBER" | "STATEMENTS" | "VARIABLE:HIGH" | "VARIABLE:LOW"
            | "VARIABLE:CLOSE" | "VARIABLE:WR" => {}
            _ => return None,
        }
    }
    let call_id = call_id?;
    let call = graph.node(call_id)?;
    if !call.capabilities.effect.is_pure()
        || graph.node(*call.dependencies.first()?)?.operation != "VARIABLE:HIGH"
        || graph.node(*call.dependencies.get(1)?)?.operation != "VARIABLE:LOW"
        || graph.node(*call.dependencies.get(2)?)?.operation != "VARIABLE:CLOSE"
    {
        return None;
    }
    let period = semantic.number_literal(*call.dependencies.get(3)?)?;
    if !period.is_finite() || period < 1.0 || period > usize::MAX as f64 || period.fract() != 0.0 {
        return None;
    }
    let (length_assignment_id, length_number) = length_assignment?;
    if semantic.number_literal(length_number)? != period {
        return None;
    }
    let (value_assignment_id, value_dependencies) = value_assignment?;
    if value_dependencies.as_slice() != [call_id, length_assignment_id] {
        return None;
    }
    let value_id = graph.execution_order().iter().copied().find(|&id| {
        graph.node(id).is_some_and(|node| {
            node.operation == "VARIABLE:WR" && node.dependencies.as_slice() == [value_assignment_id]
        })
    })?;
    let plot_id = plot_id?;
    let plot = graph.node(plot_id)?;
    if plot.dependencies.first().copied()? != value_id {
        return None;
    }
    if hline_ids.len() != 2 {
        return None;
    }
    let mut prior_output = plot_id;
    for &id in &hline_ids {
        let hline = graph.node(id)?;
        if hline.dependencies.len() != 2
            || semantic.number_literal(hline.dependencies[0]).is_none()
            || hline.dependencies[1] != prior_output
        {
            return None;
        }
        prior_output = id;
    }

    let layout = hot.output_layout().outputs();
    let output_index = layout.iter().position(|(node, _)| *node == call_id)?;
    let wr_index = variables
        .iter()
        .find(|binding| binding.name() == "WR")?
        .output_index();
    let plot_index = variables
        .iter()
        .find(|binding| binding.name() == "PLOT")?
        .output_index();
    let length_index = variables
        .iter()
        .find(|binding| binding.name() == "LENGTH")?
        .output_index();
    if output_index != 0 || wr_index != output_index || plot_index != output_index {
        return None;
    }
    let mut scalar_outputs = Vec::with_capacity(layout.len() - 1);
    for (index, (node, _)) in layout.iter().enumerate() {
        if index != output_index {
            scalar_outputs.push((index, semantic.number_literal(*node)?));
        }
    }
    if scalar_outputs.len() != 3
        || !scalar_outputs
            .iter()
            .any(|(index, _)| *index == length_index)
    {
        return None;
    }
    Some((
        high_slot,
        low_slot,
        close_slot,
        period as usize,
        output_index,
        scalar_outputs,
    ))
}

fn compile_simple_pine_momentum_plot(
    function: &str,
    value_name: &str,
    semantic: &FormulaComputePlan,
    input_bindings: &[FormulaInputBinding],
    outputs: &[FormulaOutputBinding],
    variables: &[FormulaVariableBinding],
    hot: &HotExecutionPlan,
) -> Option<(usize, usize, usize, Vec<(usize, f64)>)> {
    if input_bindings.len() != 1
        || hot.output_layout().len() != 3
        || outputs.len() != 2
        || variables.len() != 4
        || outputs[0].name() != "PLOT"
        || outputs[0].is_level_marker()
        || outputs[1].name() != "HLINE"
        || !outputs[1].is_level_marker()
        || !input_bindings[0].name().eq_ignore_ascii_case("CLOSE")
        || !["LENGTH", value_name, "PLOT", "HLINE"]
            .into_iter()
            .all(|name| variables.iter().any(|binding| binding.name() == name))
    {
        return None;
    }

    let graph = semantic.plan();
    let mut call_id = None;
    let mut length_assignment = None;
    let mut value_assignment = None;
    let mut plot_id = None;
    let mut hline_id = None;
    let mut zero_id = None;
    for &id in graph.execution_order() {
        let node = graph.node(id)?;
        match node.operation.as_str() {
            operation
                if operation
                    .strip_prefix("CALL:")
                    .is_some_and(|name| name.eq_ignore_ascii_case(function))
                    && call_id.is_none()
                    && node.dependencies.len() == 2 =>
            {
                call_id = Some(id);
            }
            "ASSIGN:LENGTH" if length_assignment.is_none() && node.dependencies.len() == 1 => {
                length_assignment = Some((id, node.dependencies[0]));
            }
            operation
                if operation
                    .strip_prefix("ASSIGN:")
                    .is_some_and(|name| name.eq_ignore_ascii_case(value_name))
                    && value_assignment.is_none()
                    && node.dependencies.len() == 2 =>
            {
                value_assignment = Some((id, node.dependencies.clone()));
            }
            "OUTPUT:PLOT" if plot_id.is_none() => plot_id = Some(id),
            "OUTPUT:HLINE" if hline_id.is_none() => hline_id = Some(id),
            "NUMBER" => {
                if semantic.number_literal(id)? == 0.0 && zero_id.replace(id).is_some() {
                    return None;
                }
            }
            "STRING_LITERAL" | "STATEMENTS" | "VARIABLE:CLOSE" => {}
            operation
                if operation
                    .strip_prefix("VARIABLE:")
                    .is_some_and(|name| name.eq_ignore_ascii_case(value_name)) => {}
            _ => return None,
        }
    }

    let call_id = call_id?;
    let call = graph.node(call_id)?;
    if !call.capabilities.effect.is_pure()
        || graph.node(*call.dependencies.first()?)?.operation != "VARIABLE:CLOSE"
    {
        return None;
    }
    let period = semantic.number_literal(*call.dependencies.get(1)?)?;
    if !period.is_finite() || period < 1.0 || period > usize::MAX as f64 || period.fract() != 0.0 {
        return None;
    }
    let (length_assignment_id, length_number) = length_assignment?;
    if semantic.number_literal(length_number)? != period {
        return None;
    }
    let (value_assignment_id, value_dependencies) = value_assignment?;
    if value_dependencies.as_slice() != [call_id, length_assignment_id] {
        return None;
    }
    let value_id = graph.execution_order().iter().copied().find(|&id| {
        graph.node(id).is_some_and(|node| {
            node.operation
                .strip_prefix("VARIABLE:")
                .is_some_and(|name| name.eq_ignore_ascii_case(value_name))
                && node.dependencies.as_slice() == [value_assignment_id]
        })
    })?;
    let plot_id = plot_id?;
    if graph.node(plot_id)?.dependencies.first().copied()? != value_id {
        return None;
    }
    let hline = graph.node(hline_id?)?;
    let zero_id = zero_id?;
    if hline.dependencies.first().copied()? != zero_id
        || hline.dependencies.get(1).copied()? != plot_id
        || semantic.number_literal(zero_id)? != 0.0
    {
        return None;
    }

    let layout = hot.output_layout().outputs();
    let output_index = layout.iter().position(|(node, _)| *node == call_id)?;
    if output_index != 0
        || variables
            .iter()
            .find(|binding| binding.name().eq_ignore_ascii_case(value_name))?
            .output_index()
            != output_index
        || variables
            .iter()
            .find(|binding| binding.name() == "PLOT")?
            .output_index()
            != output_index
    {
        return None;
    }
    let length_index = variables
        .iter()
        .find(|binding| binding.name() == "LENGTH")?
        .output_index();
    let mut scalar_outputs = Vec::with_capacity(layout.len() - 1);
    for (index, (node, _)) in layout.iter().enumerate() {
        if index != output_index {
            scalar_outputs.push((index, semantic.number_literal(*node)?));
        }
    }
    if scalar_outputs.len() != 2
        || !scalar_outputs
            .iter()
            .any(|(index, _)| *index == length_index)
        || !scalar_outputs.iter().any(|(_, value)| *value == 0.0)
    {
        return None;
    }
    Some((
        input_bindings[0].slot().0,
        period as usize,
        output_index,
        scalar_outputs,
    ))
}

/// Recognize the corpus Pine RSI script's close → `ta.rsi` pipeline, two
/// scalar guide lines, and retained source/length variables.
fn compile_simple_pine_rsi(
    semantic: &FormulaComputePlan,
    input_bindings: &[FormulaInputBinding],
    outputs: &[FormulaOutputBinding],
    variables: &[FormulaVariableBinding],
    hot: &HotExecutionPlan,
) -> Option<(usize, usize, usize, usize, Vec<(usize, f64)>)> {
    if input_bindings.len() != 1
        || hot.output_layout().len() != 5
        || outputs.len() != 3
        || variables.len() != 5
        || !input_bindings[0].name().eq_ignore_ascii_case("CLOSE")
        || outputs[0].name() != "PLOT"
        || outputs[0].is_level_marker()
        || outputs[1].name() != "HLINE"
        || !outputs[1].is_level_marker()
        || outputs[2].name() != "HLINE"
        || !outputs[2].is_level_marker()
        || !["LENGTH", "SRC", "RSI", "PLOT", "HLINE"]
            .into_iter()
            .all(|name| variables.iter().any(|binding| binding.name() == name))
    {
        return None;
    }

    let graph = semantic.plan();
    let mut call_id = None;
    let mut length_assignment = None;
    let mut source_assignment = None;
    let mut rsi_assignment = None;
    let mut plot_id = None;
    let mut guide_lines = Vec::with_capacity(2);
    let mut close_id = None;
    for &id in graph.execution_order() {
        let node = graph.node(id)?;
        match node.operation.as_str() {
            "CALL:RSI" if call_id.is_none() && node.dependencies.len() == 2 => {
                call_id = Some(id);
            }
            "ASSIGN:LENGTH" if length_assignment.is_none() && node.dependencies.len() == 1 => {
                length_assignment = Some((id, node.dependencies[0]));
            }
            "ASSIGN:SRC" if source_assignment.is_none() && node.dependencies.len() == 1 => {
                source_assignment = Some((id, node.dependencies[0]));
            }
            "ASSIGN:RSI" if rsi_assignment.is_none() && node.dependencies.len() == 2 => {
                rsi_assignment = Some((id, node.dependencies.clone()));
            }
            "OUTPUT:PLOT" if plot_id.is_none() => plot_id = Some(id),
            "OUTPUT:HLINE" if guide_lines.len() < 2 => guide_lines.push(id),
            "VARIABLE:CLOSE" => close_id = Some(id),
            "STRING_LITERAL" | "NUMBER" | "STATEMENTS" | "VARIABLE:SRC" | "VARIABLE:RSI" => {}
            _ => return None,
        }
    }

    let call_id = call_id?;
    let call = graph.node(call_id)?;
    if !call.capabilities.effect.is_pure() {
        return None;
    }
    let period = semantic.number_literal(*call.dependencies.get(1)?)?;
    if !period.is_finite() || period < 1.0 || period > usize::MAX as f64 || period.fract() != 0.0 {
        return None;
    }
    let (length_assignment_id, length_literal) = length_assignment?;
    if semantic.number_literal(length_literal)? != period {
        return None;
    }
    let (source_assignment_id, source_value_id) = source_assignment?;
    let close_id = close_id?;
    if source_value_id != close_id
        || graph.node(source_assignment_id)?.dependencies.as_slice()
            != [close_id, length_assignment_id]
        || graph.node(*call.dependencies.first()?)?.operation != "VARIABLE:SRC"
        || graph
            .node(*call.dependencies.first()?)?
            .dependencies
            .as_slice()
            != [source_assignment_id]
    {
        return None;
    }
    let (rsi_assignment_id, rsi_dependencies) = rsi_assignment?;
    if rsi_dependencies.as_slice() != [call_id, source_assignment_id] {
        return None;
    }
    let rsi_value_id = graph.execution_order().iter().copied().find(|&id| {
        graph.node(id).is_some_and(|node| {
            node.operation == "VARIABLE:RSI" && node.dependencies.as_slice() == [rsi_assignment_id]
        })
    })?;
    let plot_id = plot_id?;
    if graph.node(plot_id)?.dependencies.first().copied()? != rsi_value_id || guide_lines.len() != 2
    {
        return None;
    }
    let first_guide_id = guide_lines[0];
    for (index, guide_id) in guide_lines.into_iter().enumerate() {
        let guide = graph.node(guide_id)?;
        if guide.dependencies.len() != 2
            || guide.dependencies[1] != if index == 0 { plot_id } else { first_guide_id }
            || !semantic.number_literal(guide.dependencies[0])?.is_finite()
        {
            return None;
        }
    }

    let layout = hot.output_layout().outputs();
    let output_index = layout.iter().position(|(node, _)| *node == call_id)?;
    let source_index = layout.iter().position(|(node, _)| *node == close_id)?;
    if output_index != 0
        || variables
            .iter()
            .find(|binding| binding.name() == "RSI")?
            .output_index()
            != output_index
        || variables
            .iter()
            .find(|binding| binding.name() == "PLOT")?
            .output_index()
            != output_index
        || variables
            .iter()
            .find(|binding| binding.name() == "SRC")?
            .output_index()
            != source_index
    {
        return None;
    }
    let length_index = variables
        .iter()
        .find(|binding| binding.name() == "LENGTH")?
        .output_index();
    let mut scalar_outputs = Vec::with_capacity(layout.len() - 2);
    for (index, (node, _)) in layout.iter().enumerate() {
        if index != output_index && index != source_index {
            scalar_outputs.push((index, semantic.number_literal(*node)?));
        }
    }
    if scalar_outputs.len() != 3
        || !scalar_outputs
            .iter()
            .any(|(index, _)| *index == length_index)
        || !scalar_outputs.iter().any(|(_, value)| *value == 70.0)
        || !scalar_outputs.iter().any(|(_, value)| *value == 30.0)
    {
        return None;
    }
    Some((
        input_bindings[0].slot().0,
        period as usize,
        output_index,
        source_index,
        scalar_outputs,
    ))
}

/// Identify a side-effect-free root `ATR(HIGH, LOW, CLOSE, positive integer)`
/// call so FormulaEngine can run its existing direct OHLC kernel without
/// constructing a dispatcher and runtime context for this single-node plan.
fn compile_simple_atr_input_slots(
    semantic: &FormulaComputePlan,
    input_bindings: &[FormulaInputBinding],
    outputs: &[FormulaOutputBinding],
    variables: &[FormulaVariableBinding],
    hot: &HotExecutionPlan,
) -> Option<(usize, usize, usize, usize)> {
    if input_bindings.len() != 3
        || hot.output_layout().len() != 1
        || !outputs.is_empty()
        || !variables.is_empty()
        || !semantic.output_modifiers().is_empty()
        || !semantic.string_literals_ordered().is_empty()
    {
        return None;
    }

    let mut high_slot = None;
    let mut low_slot = None;
    let mut close_slot = None;
    for binding in input_bindings {
        match crate::formula::types::classify_builtin_var(binding.name())? {
            crate::formula::types::BuiltinVar::High if high_slot.is_none() => {
                high_slot = Some(binding.slot().0);
            }
            crate::formula::types::BuiltinVar::Low if low_slot.is_none() => {
                low_slot = Some(binding.slot().0);
            }
            crate::formula::types::BuiltinVar::Close if close_slot.is_none() => {
                close_slot = Some(binding.slot().0);
            }
            _ => return None,
        }
    }
    let (high_slot, low_slot, close_slot) = (high_slot?, low_slot?, close_slot?);

    let graph = semantic.plan();
    let mut call_id = None;
    let mut number_count = 0;
    for &id in graph.execution_order() {
        let node = graph.node(id)?;
        match node.operation.as_str() {
            "CALL:ATR" if call_id.is_none() && node.dependencies.len() == 4 => {
                call_id = Some(id);
            }
            "NUMBER" => {
                let value = semantic.number_literal(id)?;
                if !value.is_finite()
                    || value < 1.0
                    || value > usize::MAX as f64
                    || value.fract() != 0.0
                {
                    return None;
                }
                number_count += 1;
            }
            "STATEMENTS" => {}
            operation if operation.starts_with("VARIABLE:") => {}
            _ => return None,
        }
    }

    let call_id = call_id?;
    if number_count != 1 {
        return None;
    }
    let call = graph.node(call_id)?;
    if !call.capabilities.effect.is_pure() {
        return None;
    }
    let expected = [
        crate::formula::types::BuiltinVar::High,
        crate::formula::types::BuiltinVar::Low,
        crate::formula::types::BuiltinVar::Close,
    ];
    for (dependency, expected_var) in call.dependencies[..3].iter().zip(expected) {
        let variable = graph.node(*dependency)?;
        let name = variable.operation.strip_prefix("VARIABLE:")?;
        if crate::formula::types::classify_builtin_var(name) != Some(expected_var) {
            return None;
        }
    }
    let period = semantic.number_literal(call.dependencies[3])?;
    if !period.is_finite() || period < 1.0 || period > usize::MAX as f64 || period.fract() != 0.0 {
        return None;
    }

    let root = graph.node(semantic.root())?;
    if semantic.root() != call_id
        && !(root.operation == "STATEMENTS" && root.dependencies.as_slice() == [call_id])
    {
        return None;
    }
    Some((high_slot, low_slot, close_slot, period as usize))
}

/// Identify exactly three standalone EMA assignments over one close series.
/// FormulaEngine can route this common multi-period shape through the existing
/// EMA sweep while retaining the original assignment/output indexes.
fn compile_simple_multi_ema_assignments(
    semantic: &FormulaComputePlan,
    input_bindings: &[FormulaInputBinding],
    outputs: &[FormulaOutputBinding],
    variables: &[FormulaVariableBinding],
    hot: &HotExecutionPlan,
) -> Option<(usize, Vec<(usize, usize)>)> {
    if input_bindings.len() != 1
        || !outputs.is_empty()
        || variables.len() != 3
        || hot.output_layout().len() != 3
        || !semantic.output_modifiers().is_empty()
        || !semantic.string_literals_ordered().is_empty()
    {
        return None;
    }
    let input = &input_bindings[0];
    if crate::formula::types::classify_builtin_var(input.name())
        != Some(crate::formula::types::BuiltinVar::Close)
    {
        return None;
    }

    let graph = semantic.plan();
    let root = graph.node(semantic.root())?;
    if root.operation != "STATEMENTS" || root.dependencies.len() != variables.len() {
        return None;
    }

    let mut assignments = BTreeSet::new();
    let mut calls = BTreeSet::new();
    let mut number_nodes = BTreeSet::new();
    let mut output_indexes = BTreeSet::new();
    let mut compiled = Vec::with_capacity(variables.len());
    let mut previous_assignment = None;
    for binding in variables {
        let operation = format!("ASSIGN:{}", binding.name());
        let mut matching = graph.execution_order().iter().copied().filter(|&id| {
            graph
                .node(id)
                .is_some_and(|node| node.operation.eq_ignore_ascii_case(&operation))
        });
        let assignment_id = matching.next()?;
        if matching.next().is_some() || !root.dependencies.contains(&assignment_id) {
            return None;
        }
        let assignment = graph.node(assignment_id)?;
        let expected_dependency_count = if previous_assignment.is_some() { 2 } else { 1 };
        if assignment.dependencies.len() != expected_dependency_count
            || assignment.dependencies.get(1).copied() != previous_assignment
        {
            return None;
        }
        let call_id = assignment.dependencies.first()?;
        let call = graph.node(*call_id)?;
        if call.operation != "CALL:EMA"
            || call.dependencies.len() != 2
            || !call.capabilities.effect.is_pure()
            || !calls.insert(*call_id)
        {
            return None;
        }
        let input_node = graph.node(call.dependencies[0])?;
        let input_name = input_node.operation.strip_prefix("VARIABLE:")?;
        if !input_name.eq_ignore_ascii_case(input.name()) {
            return None;
        }
        let period_node = call.dependencies[1];
        let period = semantic.number_literal(period_node)?;
        if !period.is_finite()
            || period < 1.0
            || period > usize::MAX as f64
            || period.fract() != 0.0
            || !number_nodes.insert(period_node)
            || !assignments.insert(assignment_id)
            || !output_indexes.insert(binding.output_index())
        {
            return None;
        }
        compiled.push((period as usize, binding.output_index()));
        previous_assignment = Some(assignment_id);
    }
    if output_indexes.len() != 3 || (0..3).any(|index| !output_indexes.contains(&index)) {
        return None;
    }

    // Keep this path limited to the exact assignments and literals already
    // checked above; extra expressions or side effects stay on the executor.
    for &id in graph.execution_order() {
        let node = graph.node(id)?;
        match node.operation.as_str() {
            "STATEMENTS" if id == semantic.root() => {}
            "VARIABLE:CLOSE" => {}
            "NUMBER" if number_nodes.contains(&id) => {}
            "CALL:EMA" if calls.contains(&id) => {}
            operation if operation.starts_with("ASSIGN:") && assignments.contains(&id) => {}
            _ => return None,
        }
    }
    Some((input.slot().0, compiled))
}

/// Identify the corpus's common `NAME:=COPPOCK(input,wma,long,short)` shape.
/// The only retained assignment must be the call result itself; any additional
/// arithmetic, calls, channels, or variables are left to the general plan.
fn compile_simple_coppock_input_periods(
    semantic: &FormulaComputePlan,
    input_bindings: &[FormulaInputBinding],
    outputs: &[FormulaOutputBinding],
    variables: &[FormulaVariableBinding],
    hot: &HotExecutionPlan,
) -> Option<(String, usize, usize, usize)> {
    if input_bindings.len() != 1
        || hot.output_layout().len() != 1
        || !outputs.is_empty()
        || variables.len() != 1
        || !semantic.output_modifiers().is_empty()
        || !semantic.string_literals_ordered().is_empty()
    {
        return None;
    }
    let input = &input_bindings[0];
    let graph = semantic.plan();
    let mut call_id = None;
    let mut assignment_id = None;
    let mut number_count = 0;
    for &id in graph.execution_order() {
        let node = graph.node(id)?;
        match node.operation.as_str() {
            "CALL:COPPOCK" if call_id.is_none() && node.dependencies.len() == 4 => {
                call_id = Some(id);
            }
            "NUMBER" => {
                let value = semantic.number_literal(id)?;
                if !value.is_finite()
                    || value < 1.0
                    || value > usize::MAX as f64
                    || value.fract() != 0.0
                {
                    return None;
                }
                number_count += 1;
            }
            operation if operation.starts_with("ASSIGN:") && assignment_id.is_none() => {
                assignment_id = Some(id);
            }
            "STATEMENTS" => {}
            operation
                if operation
                    .strip_prefix("VARIABLE:")
                    .is_some_and(|name| name.eq_ignore_ascii_case(input.name())) => {}
            _ => return None,
        }
    }

    let call_id = call_id?;
    let assignment_id = assignment_id?;
    let call = graph.node(call_id)?;
    if number_count != 3 {
        return None;
    }
    let periods = [
        semantic.number_literal(call.dependencies[1])?,
        semantic.number_literal(call.dependencies[2])?,
        semantic.number_literal(call.dependencies[3])?,
    ];
    if periods.iter().any(|value| {
        !value.is_finite() || *value < 1.0 || *value > usize::MAX as f64 || value.fract() != 0.0
    }) {
        return None;
    }
    if !call.capabilities.effect.is_pure()
        || !graph
            .node(call.dependencies[0])?
            .operation
            .strip_prefix("VARIABLE:")
            .is_some_and(|name| name.eq_ignore_ascii_case(input.name()))
    {
        return None;
    }
    let assignment = graph.node(assignment_id)?;
    if assignment.dependencies.as_slice() != [call_id]
        || variables[0].slot()
            != hot
                .output_layout()
                .outputs()
                .first()
                .map(|(_, slot)| *slot)?
    {
        return None;
    }
    let root = graph.node(semantic.root())?;
    if semantic.root() != assignment_id
        && !(root.operation == "STATEMENTS" && root.dependencies.as_slice() == [assignment_id])
    {
        return None;
    }
    Some((
        input.name().to_owned(),
        periods[0] as usize,
        periods[1] as usize,
        periods[2] as usize,
    ))
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

/// Pair each variable write with the buffer slot its series lands in.
///
/// A write whose node has no retained slot is dropped rather than guessed at:
/// the binding list is what the engine publishes into `ctx.variables`, and
/// publishing a value from the wrong slot would be worse than publishing none.
fn compile_variable_bindings(
    variable_writes: &[NamedOutput],
    hot: &HotExecutionPlan,
) -> Vec<FormulaVariableBinding> {
    let layout = hot.output_layout().outputs();
    variable_writes
        .iter()
        .filter_map(|write| {
            let slot = layout
                .iter()
                .find(|(node, _)| *node == write.node)
                .map(|(_, slot)| *slot)?;
            // Resolve the first output-vector position for this slot once at
            // compile time. Several named variables may intentionally share a
            // CSE slot, and runtime publication must resolve them to the same
            // first materialized value.
            let output_index = layout
                .iter()
                .position(|(_, candidate)| *candidate == slot)?;
            let name = write.name.clone();
            Some(FormulaVariableBinding {
                context_key: std::sync::Arc::from(name.as_str()),
                name,
                slot,
                output_index,
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
    bindings.sort_unstable_by_key(|binding| binding.slot.0);
    bindings
}

/// Return the canonical CSE spelling for built-in market-data aliases while
/// retaining the original operation on the plan node itself. `FormulaContext`
/// resolves each alias pair/triple to the same backing series, so formulas such
/// as `(H-L)/C` and `(HIGH-LOW)/CLOSE` have identical pure dependencies.
fn cse_operation(operation: &str) -> std::borrow::Cow<'_, str> {
    let Some(name) = operation.strip_prefix("VARIABLE:") else {
        return std::borrow::Cow::Borrowed(operation);
    };
    let canonical = match crate::formula::types::classify_builtin_var(name) {
        Some(crate::formula::types::BuiltinVar::Open) => Some("OPEN"),
        Some(crate::formula::types::BuiltinVar::High) => Some("HIGH"),
        Some(crate::formula::types::BuiltinVar::Low) => Some("LOW"),
        Some(crate::formula::types::BuiltinVar::Close) => Some("CLOSE"),
        Some(crate::formula::types::BuiltinVar::Volume) => Some("VOLUME"),
        Some(crate::formula::types::BuiltinVar::Amount) => Some("AMOUNT"),
        _ if name.eq_ignore_ascii_case("A") => Some("AMOUNT"),
        _ => None,
    };
    match canonical {
        Some(name) => std::borrow::Cow::Owned(format!("VARIABLE:{name}")),
        None => std::borrow::Cow::Borrowed(operation),
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
                cse_operation(&source.operation).into_owned(),
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

/// Replace common sequence/scalar arithmetic with a one-input scalar kernel.
///
/// Formula literals are represented as full-length `NUMBER` series in the
/// semantic DAG. Keeping them materialized makes expressions such as
/// `CLOSE * 2 + 1` allocate and traverse two unnecessary arrays. The hot plan
/// already has a scalar parameter arena, so move one literal onto its binary
/// consumer and remove that dependency from the runtime graph.
fn fuse_scalar_arithmetic(
    plan: &ComputePlan,
    ranges: &mut BTreeMap<ComputeNodeId, ParameterRange>,
) -> Result<ComputePlan, FormulaHotPlanError> {
    let mut nodes = Vec::with_capacity(plan.len());
    for &node_id in plan.execution_order() {
        let source = plan
            .node(node_id)
            .expect("execution order only contains compiled nodes");
        let mut operation = source.operation.clone();
        let mut dependencies = source.dependencies.clone();

        if dependencies.len() == 2 {
            let arithmetic = match source.operation.as_str() {
                "BINARY:Add" => Some("Add"),
                "BINARY:Sub" => Some("Sub"),
                "BINARY:Mul" => Some("Mul"),
                "BINARY:Div" => Some("Div"),
                _ => None,
            };
            if let Some(arithmetic) = arithmetic {
                let scalar_side = dependencies.iter().position(|dependency| {
                    plan.node(*dependency)
                        .is_some_and(|node| node.operation == "NUMBER")
                });
                if let Some(side) = scalar_side {
                    // Both operands being constants is better left to the
                    // normal binary path; this optimization targets one scalar
                    // beside an external or computed series.
                    let other = dependencies[1 - side];
                    let other_is_number = plan
                        .node(other)
                        .is_some_and(|node| node.operation == "NUMBER");
                    if !other_is_number {
                        let scalar_node = dependencies[side];
                        let scalar_operation = match (arithmetic, side) {
                            ("Sub", 0) => "Scalar:SubLeft",
                            ("Sub", _) => "Scalar:SubRight",
                            ("Div", 0) => "Scalar:DivLeft",
                            ("Div", _) => "Scalar:DivRight",
                            ("Add", _) => "Scalar:Add",
                            ("Mul", _) => "Scalar:Mul",
                            _ => unreachable!("only supported arithmetic reaches this arm"),
                        };
                        operation = scalar_operation.to_string();
                        dependencies.clear();
                        dependencies.push(other);
                        let scalar_range = *ranges
                            .get(&scalar_node)
                            .expect("NUMBER nodes have bound parameter ranges");
                        ranges.insert(node_id, scalar_range);
                    }
                }
            }
        }

        nodes.push(ComputeNode::new(
            node_id,
            operation,
            dependencies,
            source.capabilities.clone(),
        ));
    }

    ComputePlan::compile(nodes).map_err(|error| FormulaHotPlanError::Semantic(error))
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
    /// Every variable the tree path would publish in `ctx.variables`, in
    /// first-write order with later writes to the same name replacing earlier
    /// ones (the map semantics `ctx.variables` has).
    variable_writes: Vec<NamedOutput>,
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
        rewritten
            .first()
            .copied()
            .ok_or_else(|| FormulaHotPlanError::UnsupportedPlumbing {
                operation: operation.to_string(),
                reason: "node has no value operand".to_string(),
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
    let mut variable_writes = Vec::<NamedOutput>::new();
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
            // `ctx.variables` is part of the tree path's observable contract, so
            // the value has to stay readable here too. See `FormulaVariableBinding`.
            variable_writes.push(NamedOutput {
                name: published_name(&source.capabilities.effect)
                    .unwrap_or_else(|| name.to_string()),
                node: value,
                level_marker: false,
            });
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
            // The caller-visible spelling, not the canonical one the operation
            // string carries: `ctx.output_names` and `ctx.variables` are keyed by
            // the name as written, and the parser preserves case.
            let display =
                published_name(&source.capabilities.effect).unwrap_or_else(|| name.to_string());
            named_outputs.push(NamedOutput {
                name: display.clone(),
                node: value,
                level_marker: matches!(
                    source.capabilities.effect,
                    ComputeEffect::EmitLevelMarker(_)
                ),
            });
            // An `Output` is a variable write *as well as* a channel, so it
            // appears in both lists: `named_outputs` fixes the channel order a
            // caller sees, `variable_writes` mirrors what the tree path stores
            // in `ctx.variables`. Both point at the same node, so the two
            // bindings share one buffer slot rather than computing the value
            // twice.
            variable_writes.push(NamedOutput {
                name: display,
                node: value,
                level_marker: false,
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
                &mut variable_writes,
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

    // Collapse repeated writes to the same name *before* choosing roots, so a
    // superseded first write is not retained (and not published) at all.
    let variable_writes = dedupe_last_write_wins(variable_writes);

    // Retain the primary result first, then every named output, then every
    // variable the tree path would publish, then every drawing directive.
    // Named outputs keep their value subgraphs alive, retaining variables keeps
    // `ctx.variables` populated exactly as the tree path leaves it, and
    // retaining drawings stops chart side effects from being silently dropped
    // by dead-code elimination. An unsupported drawing still fails the plan
    // loudly instead of vanishing.
    //
    // A retained root is never recycled by the buffer arena (its last use is the
    // end of the run), so this does cost one buffer per variable. That is the
    // same memory the tree path already holds in `ctx.variables` for the same
    // source — parity, not a new cost.
    let primary = resolve(&aliases, root);
    let mut roots = vec![primary];
    for output in &named_outputs {
        if !roots.contains(&output.node) {
            roots.push(output.node);
        }
    }
    for variable in &variable_writes {
        if !roots.contains(&variable.node) {
            roots.push(variable.node);
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
        variable_writes,
    })
}

/// The spelling a caller sees for a write node.
///
/// `ASSIGN:`/`OUTPUT:` operation strings are canonicalised (upper-cased) so that
/// dependency resolution matches `VARIABLE:` reads, but the tree path keys
/// `ctx.variables` and `ctx.output_names` by the name *as written in the
/// source*, and the parser preserves case. The attached effect is the only place
/// that spelling survives lowering, so it is authoritative here.
fn published_name(effect: &ComputeEffect) -> Option<String> {
    match effect {
        ComputeEffect::WriteVariable(name)
        | ComputeEffect::EmitOutput(name)
        | ComputeEffect::EmitLevelMarker(name) => Some(name.clone()),
        _ => None,
    }
}

/// Keep only the final write to each variable name.
///
/// `ctx.variables` is a map, so `x = 1; x = 2` leaves one entry. Keeping both
/// here would retain a buffer for the dead first write and then publish the
/// wrong one, since the binding list is read in order.
fn dedupe_last_write_wins(writes: Vec<NamedOutput>) -> Vec<NamedOutput> {
    let mut seen = BTreeSet::new();
    let mut kept = Vec::with_capacity(writes.len());
    for write in writes.into_iter().rev() {
        if seen.insert(write.name.clone()) {
            kept.push(write);
        }
    }
    kept.reverse();
    kept
}

/// Rewrite one `COMPOUND:<name>:<op>` node into the equivalent binary node.
///
/// `X += Y` is exactly `X = X + Y`, so the node is replaced by a synthetic
/// `BINARY:<op>` whose operands are already in `[current value, right-hand
/// side]` order. The synthetic id is allocated above every existing id, which
/// keeps the parameter ranges bound by [`bind_numeric_literals`] valid.
///
/// The rewritten dependency list may carry one extra entry beyond the operands:
/// the sequencing edge every effect node gets. See the arity check below.
///
/// Returns the synthetic node id so the caller can keep allocating.
fn lower_compound(
    node_id: ComputeNodeId,
    operation: &str,
    rewritten: Vec<ComputeNodeId>,
    next_synthetic_id: usize,
    aliases: &mut BTreeMap<ComputeNodeId, ComputeNodeId>,
    local_writes: &mut BTreeMap<String, ComputeNodeId>,
    variable_writes: &mut Vec<NamedOutput>,
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
            )));
        }
    };
    // The node carries the two value operands, and — when it follows another
    // effect — the sequencing edge `add_effect` appends after them. Only the
    // first two are operands: forwarding the sequencing edge too would hand the
    // synthetic `BINARY:<op>` three inputs, which fails the kernel's arity check
    // at dispatch. `ASSIGN` and `OUTPUT` already read their value operand
    // positionally for the same reason.
    //
    // Dropping the edge loses no ordering. The write is registered in
    // `local_writes`, so every later read of the variable resolves to this node
    // and re-creates a real data dependency on it.
    if rewritten.len() < 2 {
        return Err(unsupported(format!(
            "expected at least 2 operands (current value, right-hand side), found {}",
            rewritten.len()
        )));
    }
    let operands = vec![rewritten[0], rewritten[1]];

    let synthetic = ComputeNodeId(next_synthetic_id);
    aliases.insert(node_id, synthetic);
    local_writes.insert(name.to_string(), synthetic);
    // The compound result is published to `ctx.variables` by the tree path too,
    // and this synthetic node is the only carrier of that value.
    variable_writes.push(NamedOutput {
        name: name.to_string(),
        node: synthetic,
        level_marker: false,
    });
    nodes.push(ComputeNode::new(
        synthetic,
        format!("BINARY:{binary_op}"),
        operands,
        ComputeCapabilities {
            deterministic: true,
            streaming: true,
            stateful: false,
            lookback: LookbackRequirement::None,
            effect: ComputeEffect::Pure,
            dependency: DependencyShape::FixedLookback(0),
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

/// Mark numeric literals that are consumed exclusively as scalar call
/// parameters. Their buffers still have the full logical length, but only the
/// first element is observable through the call sites below, so filling every
/// element is wasted work. Literals retained as outputs/variables, used by
/// element-wise operations, or shared with any non-scalar use keep the regular
/// full-series `NUMBER` kernel.
fn lower_scalar_number_outputs(
    plan: &ComputePlan,
    roots: &[ComputeNodeId],
) -> Result<ComputePlan, FormulaHotPlanError> {
    let mut scalar_only: BTreeSet<_> = plan
        .execution_order()
        .iter()
        .filter_map(|&id| {
            plan.node(id)
                .is_some_and(|node| node.operation == "NUMBER")
                .then_some(id)
        })
        .collect();

    for &root in roots {
        scalar_only.remove(&root);
    }
    for &id in plan.execution_order() {
        let Some(node) = plan.node(id) else {
            continue;
        };
        for (position, &dependency) in node.dependencies.iter().enumerate() {
            if scalar_only.contains(&dependency)
                && !is_scalar_call_parameter(&node.operation, position)
            {
                scalar_only.remove(&dependency);
            }
        }
    }
    if scalar_only.is_empty() {
        return Ok(plan.clone());
    }

    let nodes = plan.execution_order().iter().map(|&id| {
        let source = plan
            .node(id)
            .expect("execution order only contains compiled nodes");
        ComputeNode::new(
            source.id,
            if scalar_only.contains(&id) {
                "SCALAR:NUMBER"
            } else {
                source.operation.as_str()
            },
            source.dependencies.clone(),
            source.capabilities.clone(),
        )
    });
    Ok(ComputePlan::compile(nodes)?)
}

/// Whether a numeric call operand is read only through the dispatcher's scalar
/// or period accessor. Keep this conservative: a false negative costs a fill;
/// a false positive could change the values of a series operand.
fn is_scalar_call_parameter(operation: &str, position: usize) -> bool {
    match operation {
        "CALL:MA"
        | "CALL:EMA"
        | "CALL:WMA"
        | "CALL:HMA"
        | "CALL:RMA"
        | "CALL:MEDIAN"
        | "CALL:ROLLING_RANGE"
        | "CALL:KAMA"
        | "CALL:RSI"
        | "CALL:MOM"
        | "CALL:ROC"
        | "CALL:TRIX"
        | "CALL:TRIMA"
        | "CALL:STD"
        | "CALL:STDDEV"
        | "CALL:HHV"
        | "CALL:LLV"
        | "CALL:SUM"
        | "CALL:VAR"
        | "CALL:REF"
        | "CALL:ROCP"
        | "CALL:ROCR"
        | "CALL:ROCR100"
        | "CALL:RSQUARE"
        | "CALL:RESI"
        | "CALL:RANK_PCT"
        | "CALL:STDDEV_SAMPLE"
        | "CALL:LINEARREG_SLOPE"
        | "CALL:ZSCORE" => position == 1,
        "CALL:SMA" => position == 1 || position == 2,
        "CALL:QUANTILE" | "CALL:BOLLUP" | "CALL:BOLLMID" | "CALL:BOLLDN" => {
            position == 1 || position == 2
        }
        "CALL:ATR" | "CALL:NATR" | "CALL:CCI" => position == 3,
        "CALL:VWMA" | "CALL:CORREL" | "CALL:ICHIMOKU_TENKAN" | "CALL:ICHIMOKU_KIJUN" => {
            position == 2
        }
        "CALL:CMF" => position == 4,
        "CALL:CHOP" | "CALL:SUPERTREND" => position == 3 || position == 4,
        "CALL:FISHER" | "CALL:FISHER_SIGNAL" => position == 2,
        "CALL:TSI" => position == 1 || position == 2,
        "CALL:KDJ" | "CALL:KDJ_D" | "CALL:KDJ_J" => position == 3 || position == 4 || position == 5,
        "CALL:MACD" | "CALL:DEA" => position == 1 || position == 2 || position == 3,
        _ => false,
    }
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
/// Bind each `STRING_LITERAL` node's literal index into the parameter arena.
///
/// The plan's literal list is ordered by node id, which is the order the tree
/// path appends the same literals to `FormulaContext::string_table`. The stored
/// parameter is the *plan-local* index; the dispatcher offsets it by the
/// context's pre-existing table length, so both paths agree on the index even
/// when a caller pre-populated the table.
fn bind_string_literals(
    semantic: &FormulaComputePlan,
    arena: &mut ParameterArena,
) -> Result<BTreeMap<ComputeNodeId, ParameterRange>, FormulaHotPlanError> {
    let literals = semantic.string_literals_ordered();
    if literals.is_empty() {
        return Ok(BTreeMap::new());
    }

    let mut string_nodes = Vec::new();
    for raw_id in 0..semantic.plan().len() {
        let id = ComputeNodeId(raw_id);
        if semantic
            .plan()
            .node(id)
            .is_some_and(|node| node.operation == "STRING_LITERAL")
        {
            string_nodes.push(id);
        }
    }
    if string_nodes.len() != literals.len() {
        return Err(FormulaHotPlanError::LiteralBindingMismatch {
            ast_literals: literals.len(),
            number_nodes: string_nodes.len(),
        });
    }

    let mut ranges = BTreeMap::new();
    for (index, (node, text)) in literals.iter().enumerate() {
        debug_assert!(
            semantic.string_literal(*node).is_some_and(|t| t == *text),
            "string literal list must stay keyed by the node that carries it"
        );
        // `index` is a slot ordinal, never a magnitude: it is exact in f64 for
        // any table a formula could name.
        #[allow(clippy::cast_precision_loss)]
        let value = ParameterValue::from_f64(index as f64);
        let range = arena.extend([value]);
        ranges.insert(*node, range);
    }
    Ok(ranges)
}

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
        let value =
            semantic
                .number_literal(node)
                .ok_or(FormulaHotPlanError::LiteralBindingMismatch {
                    ast_literals: recorded,
                    number_nodes: recorded,
                })?;
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
    fn pine_trix_plot_shape_uses_the_direct_momentum_plan() {
        let pine =
            crate::formula::pine::parse_pine(include_str!("../../../tests/pine_corpus/trix.pine"))
                .unwrap();
        let ast = crate::formula::pine::map_pine_to_alphata(&pine).unwrap();
        let compiled = FormulaHotPlan::compile(&ast).unwrap();

        let (close_slot, period, output_index, scalar_outputs) = compiled
            .simple_pine_trix()
            .expect("TRIX plot shape is recognized");
        assert_eq!(close_slot, 0);
        assert_eq!(period, 15);
        assert_eq!(output_index, 0);
        assert_eq!(scalar_outputs.len(), 2);
        assert!(compiled.simple_pine_roc().is_none());
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

    #[test]
    fn simple_atr_metadata_accepts_builtin_aliases_and_rejects_wrappers() {
        let canonical = parse_formula("ATR(HIGH,LOW,CLOSE,14)").unwrap();
        assert!(FormulaHotPlan::compile(&canonical)
            .unwrap()
            .simple_atr_input_slots()
            .is_some());

        let ast = parse_formula("ATR(H,L,C,14)").unwrap();
        let compiled = FormulaHotPlan::compile(&ast).unwrap();
        let input_slot = |kind| {
            compiled
                .input_bindings()
                .iter()
                .find(|binding| {
                    crate::formula::types::classify_builtin_var(binding.name()) == Some(kind)
                })
                .unwrap()
                .slot()
                .0
        };
        assert_eq!(
            compiled.simple_atr_input_slots(),
            Some((
                input_slot(crate::formula::types::BuiltinVar::High),
                input_slot(crate::formula::types::BuiltinVar::Low),
                input_slot(crate::formula::types::BuiltinVar::Close),
                14,
            ))
        );

        let wrapped = parse_formula("ATR(HIGH,LOW,CLOSE,14)+1").unwrap();
        assert_eq!(
            FormulaHotPlan::compile(&wrapped)
                .unwrap()
                .simple_atr_input_slots(),
            None
        );
        let assigned = parse_formula("ATR14:=ATR(HIGH,LOW,CLOSE,14)").unwrap();
        assert_eq!(
            FormulaHotPlan::compile(&assigned)
                .unwrap()
                .simple_atr_input_slots(),
            None
        );
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
            compiled
                .outputs()
                .iter()
                .any(|output| !output.is_level_marker()),
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
        assert!(compiled
            .outputs()
            .iter()
            .all(|output| !output.is_level_marker()));

        // Both channels need distinct buffers.
        let slots: Vec<_> = compiled
            .outputs()
            .iter()
            .map(|output| output.slot())
            .collect();
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

        for binding in compiled.variables() {
            assert_eq!(
                roots[binding.output_index()].1,
                binding.slot(),
                "the precomputed publication index must resolve to its slot"
            );
            assert_eq!(
                binding.output_index(),
                roots
                    .iter()
                    .position(|(_, slot)| *slot == binding.slot())
                    .unwrap(),
                "shared slots must resolve to their first output position"
            );
        }
    }
}
