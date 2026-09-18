//! Declarative composite-indicator graphs.
//!
//! This layer complements the low-level indicator functions and the factor
//! registry.  A graph is made from named series, built-in indicators, simple
//! vector operators, and user-registered functions.  Definitions are
//! evaluated with dependency memoization and cycle detection, so a request
//! can reuse an intermediate series without recomputing it.

use crate::factors::{zscore, BorrowedFactorContext, FactorContext, FactorError, FactorResult};
use crate::indicators::{self, momentum, volatility};
use crate::math::{moving_avg, statistics};
use crate::unified_runtime::{
    DirtyRange, RuntimeExecution, RuntimeExecutionMode, RuntimeExecutionTrace,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// A user-defined vector function used by [`CompositeExpr::Call`].
pub type CompositeFn = Arc<dyn Fn(&[&[f64]], &[f64]) -> FactorResult<Vec<f64>> + Send + Sync>;

/// An expression in a composite-indicator graph.
#[derive(Debug, Clone)]
pub enum CompositeExpr {
    /// Read one raw or previously defined named series.
    Series(String),
    /// A scalar broadcast to the current row count.
    Constant(f64),
    /// Invoke a registered indicator/function.
    Call {
        /// Registered function name, case-insensitive for built-ins.
        name: String,
        /// Input expressions passed to the function.
        inputs: Vec<Self>,
        /// Numeric parameters such as lookback windows.
        params: Vec<f64>,
    },
    /// A named graph definition.
    Ref(String),
    /// Element-wise vector operation.
    Op {
        /// Operation to apply.
        op: CompositeOp,
        /// One or more operands.
        inputs: Vec<Self>,
    },
}

impl CompositeExpr {
    /// Shorthand for a named series.
    pub fn series(name: impl Into<String>) -> Self {
        Self::Series(name.into())
    }

    /// Shorthand for a named graph reference.
    pub fn reference(name: impl Into<String>) -> Self {
        Self::Ref(name.into())
    }

    /// Shorthand for a function call.
    pub fn call(name: impl Into<String>, inputs: Vec<Self>, params: Vec<f64>) -> Self {
        Self::Call {
            name: name.into(),
            inputs,
            params,
        }
    }

    /// Shorthand for a weighted composite blend. Empty weights mean equal
    /// weights; otherwise one weight is required for each input.
    pub fn weighted_average(inputs: Vec<Self>, weights: Vec<f64>) -> Self {
        Self::call("weighted_average", inputs, weights)
    }
}

/// Element-wise operations available in a composite graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CompositeOp {
    /// Sum all operands.
    Add,
    /// Subtract subsequent operands from the first.
    Sub,
    /// Multiply all operands.
    Mul,
    /// Divide the first operand by subsequent operands.
    Div,
    /// Element-wise minimum.
    Min,
    /// Element-wise maximum.
    Max,
    /// Weighted average with equal weights for the operands. Use
    /// [`CompositeExpr::weighted_average`] when explicit weights are needed.
    WeightedAverage,
}

/// Explicit state kernel used by a Composite call in a stateful plan.
///
/// The batch Composite function remains a vector callback, while this
/// declaration selects the independent row-state implementation. Keeping the
/// capability in the registry avoids making the stateful executor infer
/// semantics from arbitrary function names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StatefulCompositeSpec {
    Sma,
    Wma,
    Ema,
    Rsi,
    Atr,
    Macd,
    BollMid,
    BollUpper,
    BollLower,
    RollingStd,
    RollingMin,
    RollingMax,
    Return,
    Vwma,
    Volatility,
    Threshold,
    Between,
    Clip,
    Abs,
    Neg,
    Sign,
    WeightedAverage,
    CrossUp,
    CrossDown,
}

/// One named output in a composite graph.
#[derive(Debug, Clone)]
pub struct CompositeDefinition {
    /// Stable output name.
    pub name: String,
    /// Expression producing the output.
    pub expression: CompositeExpr,
}

impl CompositeDefinition {
    /// Create a named definition.
    pub fn new(name: impl Into<String>, expression: CompositeExpr) -> Self {
        Self {
            name: name.into(),
            expression,
        }
    }
}

/// Reusable, validated composite graph plan.
///
/// The plan owns the graph definition and requested outputs so callers can
/// execute the same graph repeatedly without rebuilding the definition map on
/// every batch, range, or streaming request.
#[derive(Debug, Clone)]
pub struct CompiledCompositePlan {
    pub(crate) definitions: Vec<CompositeDefinition>,
    pub(crate) outputs: Vec<String>,
    pub(crate) required_raw_inputs: Vec<String>,
    range_lookback: Option<usize>,
    pub(crate) signature: u64,
    pub(crate) stateful_specs: BTreeMap<String, StatefulCompositeSpec>,
}

impl CompiledCompositePlan {
    /// Stable graph identity used by result caches and provenance.
    #[must_use]
    pub const fn signature(&self) -> u64 {
        self.signature
    }

    /// Requested output names in deterministic order.
    #[must_use]
    pub fn outputs(&self) -> &[String] {
        &self.outputs
    }

    /// Raw input names required by the selected graph outputs.
    #[must_use]
    pub fn required_raw_inputs(&self) -> &[String] {
        &self.required_raw_inputs
    }

    /// Whether the selected graph has a finite, proven dirty-range contract.
    ///
    /// Recursive stateful indicators such as EMA, RSI, ATR and MACD return
    /// `None`; a range executor must not guess a finite history for them.
    #[must_use]
    pub const fn supports_range_incremental(&self) -> bool {
        self.range_lookback.is_some()
    }

    /// Historical rows required before an affected output interval.
    #[must_use]
    pub const fn range_lookback(&self) -> Option<usize> {
        self.range_lookback
    }

    /// Create a bounded-window append-only stream for this plan.
    ///
    /// Recursive, whole-series, and custom-function plans are rejected. They
    /// need either a dedicated stateful kernel or full execution because a
    /// finite replay window cannot prove their numerical correctness.
    pub fn stream(&self, engine: CompositeEngine) -> FactorResult<CompositeStream> {
        if self.required_raw_inputs.is_empty() {
            return Err(FactorError::InvalidParameter(
                "composite bounded streaming requires at least one raw input series".to_string(),
            ));
        }
        let _ = self.range_lookback.ok_or_else(|| {
            FactorError::InvalidParameter(
                "composite plan is not range-safe: recursive, whole-series, or custom functions require full execution"
                    .to_string(),
            )
        })?;
        Ok(CompositeStream {
            plan: self.clone(),
            engine,
            row_count: 0,
            inputs: BTreeMap::new(),
            output: BTreeMap::new(),
        })
    }

    /// Create an O(1)-per-row stateful stream for supported built-in calls.
    ///
    /// Unlike [`Self::stream`], this path does not require a finite replay
    /// lookback. Unsupported custom or whole-series functions fail explicitly
    /// during compilation instead of silently changing their semantics.
    #[cfg(feature = "indicators-all")]
    pub fn stateful_stream(
        &self,
    ) -> FactorResult<crate::stateful_composite::StatefulCompositeStream> {
        crate::stateful_composite::StatefulCompositeStream::from_plan(self)
    }
}

const COMPILED_PLAN_CACHE_CAPACITY: usize = 64;

#[derive(Clone)]
struct CompiledPlanCacheEntry {
    plan: CompiledCompositePlan,
    last_used: u64,
}

/// Dependency-aware composite-indicator evaluator.
#[derive(Clone)]
pub struct CompositeEngine {
    functions: BTreeMap<String, CompositeFn>,
    builtin_functions: BTreeSet<String>,
    stateful_specs: BTreeMap<String, StatefulCompositeSpec>,
    /// Compiled graph plans are reused across cache revisions and scopes.
    /// Keeping them separate from result snapshots prevents repeated cached
    /// evaluations from rebuilding dependency maps and cycle checks.
    compiled_plans: BTreeMap<u64, CompiledPlanCacheEntry>,
    compiled_plan_cache_hits: u64,
    compiled_plan_cache_misses: u64,
    compiled_plan_cache_clock: u64,
    cache: BTreeMap<CompositeCacheKey, CompositeCacheEntry>,
    cache_capacity: usize,
    cache_clock: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CompositeCacheKey {
    scope: String,
    data_revision: u64,
    graph_signature: u64,
}

#[derive(Debug, Clone)]
struct CompositeCacheEntry {
    values: BTreeMap<String, Vec<f64>>,
    last_used: u64,
}

/// A value produced while walking a composite expression.
///
/// Raw context series are borrowed for the duration of one expression. Only
/// operators, indicator calls, constants and named graph results allocate an
/// owned vector. This is the important distinction between the public
/// `evaluate` convenience API and `evaluate_borrowed`, which is used by
/// high-frequency chart/strategy paths.
enum CompositeValue<'a> {
    Borrowed(&'a [f64]),
    Owned(Vec<f64>),
}

impl<'a> CompositeValue<'a> {
    fn as_slice(&self) -> &[f64] {
        match self {
            Self::Borrowed(values) => values,
            Self::Owned(values) => values,
        }
    }

    fn into_owned(self) -> Vec<f64> {
        match self {
            Self::Borrowed(values) => values.to_vec(),
            Self::Owned(values) => values,
        }
    }
}

impl CompositeEngine {
    /// Create an engine with the standard built-in functions registered.
    pub fn new() -> Self {
        let mut engine = Self::default();
        engine.register_builtins();
        engine.builtin_functions = builtin_function_names();
        engine.stateful_specs = builtin_stateful_specs();
        engine
    }

    /// Register or replace a custom vector function.
    pub fn register(&mut self, name: impl Into<String>, function: CompositeFn) -> FactorResult<()> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(FactorError::InvalidParameter(
                "composite function name must not be empty".to_string(),
            ));
        }
        let normalized = name.to_ascii_lowercase();
        self.builtin_functions.remove(&normalized);
        self.stateful_specs.remove(&normalized);
        self.functions.insert(normalized, function);
        self.compiled_plans.clear();
        self.compiled_plan_cache_hits = 0;
        self.compiled_plan_cache_misses = 0;
        self.compiled_plan_cache_clock = 0;
        self.cache.clear();
        self.cache_clock = 0;
        Ok(())
    }

    /// Register the explicit row-state kernel for an existing Composite
    /// function. The function's batch implementation remains authoritative for
    /// full recomputation and must be verified against the state kernel.
    pub fn register_stateful_spec(
        &mut self,
        name: impl Into<String>,
        spec: StatefulCompositeSpec,
    ) -> FactorResult<()> {
        let normalized = name.into().to_ascii_lowercase();
        if !self.functions.contains_key(&normalized) {
            return Err(FactorError::UnknownFactor(normalized));
        }
        self.stateful_specs.insert(normalized, spec);
        self.compiled_plans.clear();
        self.compiled_plan_cache_hits = 0;
        self.compiled_plan_cache_misses = 0;
        self.compiled_plan_cache_clock = 0;
        self.cache.clear();
        self.cache_clock = 0;
        Ok(())
    }

    /// Return the registered state kernel for a function name.
    #[must_use]
    pub fn stateful_spec(&self, name: &str) -> Option<StatefulCompositeSpec> {
        self.stateful_specs.get(&name.to_ascii_lowercase()).copied()
    }

    /// Set the maximum number of cached graph snapshots retained by this engine.
    ///
    /// A zero value is treated as one entry so the cache remains deterministic
    /// without requiring a special case in the evaluation path.
    pub fn with_cache_capacity(mut self, capacity: usize) -> Self {
        self.cache_capacity = capacity.max(1);
        self.cache.clear();
        self.cache_clock = 0;
        self
    }

    /// Change the cache limit after construction and invalidate old snapshots.
    pub fn set_cache_capacity(&mut self, capacity: usize) {
        self.cache_capacity = capacity.max(1);
        self.cache.clear();
        self.cache_clock = 0;
    }

    /// Remove all cached graph snapshots.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.cache_clock = 0;
    }

    /// Return the number of compiled graph plans currently retained.
    #[must_use]
    pub fn compiled_plan_count(&self) -> usize {
        self.compiled_plans.len()
    }

    /// Compile and validate a reusable composite graph plan.
    pub fn compile(
        &self,
        definitions: &[CompositeDefinition],
        outputs: &[&str],
    ) -> FactorResult<CompiledCompositePlan> {
        let mut names = BTreeMap::new();
        for definition in definitions {
            if definition.name.trim().is_empty() {
                return Err(FactorError::InvalidParameter(
                    "composite definition name must not be empty".to_string(),
                ));
            }
            if names.insert(definition.name.as_str(), definition).is_some() {
                return Err(FactorError::InvalidParameter(format!(
                    "duplicate composite definition: {}",
                    definition.name
                )));
            }
        }
        for output in outputs {
            if !names.contains_key(output) {
                return Err(FactorError::UnknownFactor((*output).to_string()));
            }
        }
        let mut visiting = Vec::new();
        for output in outputs {
            validate_named_graph(output, &names, &mut visiting)?;
        }
        let mut raw_inputs = BTreeSet::new();
        let mut lookback_memo = BTreeMap::new();
        let mut lookback_visiting = Vec::new();
        for output in outputs {
            collect_raw_inputs(output, &names, &mut raw_inputs, &mut Vec::new())?;
            let output_lookback = expression_lookback_for_named(
                output,
                &names,
                &self.builtin_functions,
                &mut lookback_memo,
                &mut lookback_visiting,
            )?;
            if output_lookback.is_none() {
                lookback_memo.insert((*output).to_string(), None);
            }
        }
        Ok(CompiledCompositePlan {
            definitions: definitions.to_vec(),
            outputs: outputs.iter().map(|name| (*name).to_string()).collect(),
            required_raw_inputs: raw_inputs.into_iter().collect(),
            range_lookback: outputs
                .iter()
                .filter_map(|output| lookback_memo.get(*output).copied().flatten())
                .max()
                .filter(|_| {
                    outputs
                        .iter()
                        .all(|output| lookback_memo.get(*output).is_some_and(Option::is_some))
                }),
            signature: graph_signature(definitions, outputs),
            stateful_specs: self.stateful_specs.clone(),
        })
    }

    /// Compile a graph through the engine-owned bounded plan cache.
    ///
    /// This is the single cache owner for Composite plans. Higher-level
    /// façades should call this method instead of keeping a second cache.
    pub fn compile_cached(
        &mut self,
        definitions: &[CompositeDefinition],
        outputs: &[&str],
    ) -> FactorResult<CompiledCompositePlan> {
        let signature = graph_signature(definitions, outputs);
        let tick = self.next_compiled_plan_tick();
        if let Some(entry) = self.compiled_plans.get_mut(&signature) {
            self.compiled_plan_cache_hits = self.compiled_plan_cache_hits.saturating_add(1);
            entry.last_used = tick;
            return Ok(entry.plan.clone());
        }

        self.compiled_plan_cache_misses = self.compiled_plan_cache_misses.saturating_add(1);
        let plan = self.compile(definitions, outputs)?;
        if self.compiled_plans.len() >= COMPILED_PLAN_CACHE_CAPACITY {
            if let Some(oldest) = self
                .compiled_plans
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(signature, _)| *signature)
            {
                self.compiled_plans.remove(&oldest);
            }
        }
        self.compiled_plans.insert(
            signature,
            CompiledPlanCacheEntry {
                plan: plan.clone(),
                last_used: tick,
            },
        );
        Ok(plan)
    }

    /// Evaluate and cache a graph snapshot for an explicit data revision.
    ///
    /// The caller owns revision management. Reusing a revision for changed
    /// numeric data is invalid; advancing it gives deterministic invalidation
    /// across repeated chart/strategy requests.
    pub fn evaluate_cached(
        &mut self,
        definitions: &[CompositeDefinition],
        outputs: &[&str],
        context: &BorrowedFactorContext<'_>,
        data_revision: u64,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        self.evaluate_cached_scoped("", definitions, outputs, context, data_revision)
    }

    /// Evaluate and cache a graph under an explicit symbol/timeframe scope.
    ///
    /// The scope is part of cache identity; callers must not reuse one scope
    /// for different instruments or timeframes while keeping a revision.
    pub fn evaluate_cached_scoped(
        &mut self,
        scope: &str,
        definitions: &[CompositeDefinition],
        outputs: &[&str],
        context: &BorrowedFactorContext<'_>,
        data_revision: u64,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        let plan = self.compile_cached(definitions, outputs)?;
        self.evaluate_cached_scoped_compiled(scope, &plan, context, data_revision)
    }

    fn next_compiled_plan_tick(&mut self) -> u64 {
        self.compiled_plan_cache_clock = self.compiled_plan_cache_clock.wrapping_add(1);
        self.compiled_plan_cache_clock
    }

    /// Evaluate and cache a previously compiled graph plan.
    pub fn evaluate_cached_scoped_compiled(
        &mut self,
        scope: &str,
        plan: &CompiledCompositePlan,
        context: &BorrowedFactorContext<'_>,
        data_revision: u64,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        let key = CompositeCacheKey {
            scope: scope.to_string(),
            data_revision,
            graph_signature: plan.signature,
        };
        if let Some(result) = self.get_cached_result(&key) {
            return Ok(result);
        }
        let result = self.evaluate_compiled(plan, context)?;
        if self.cache.len() >= self.cache_capacity.max(1) {
            if let Some(oldest) = self
                .cache
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| key.clone())
            {
                self.cache.remove(&oldest);
            }
        }
        let last_used = self.next_cache_tick();
        self.cache.insert(
            key,
            CompositeCacheEntry {
                values: result.clone(),
                last_used,
            },
        );
        Ok(result)
    }

    fn get_cached_result(&mut self, key: &CompositeCacheKey) -> Option<BTreeMap<String, Vec<f64>>> {
        let last_used = self.next_cache_tick();
        let entry = self.cache.get_mut(key)?;
        entry.last_used = last_used;
        Some(entry.values.clone())
    }

    fn next_cache_tick(&mut self) -> u64 {
        self.cache_clock = self.cache_clock.wrapping_add(1);
        self.cache_clock
    }

    /// Evaluate selected outputs from an owned context.
    pub fn evaluate(
        &self,
        definitions: &[CompositeDefinition],
        outputs: &[&str],
        context: &FactorContext,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        let borrowed = context.as_borrowed();
        self.evaluate_borrowed(definitions, outputs, &borrowed)
    }

    /// Evaluate selected outputs without copying raw input series.
    pub fn evaluate_borrowed(
        &self,
        definitions: &[CompositeDefinition],
        outputs: &[&str],
        context: &BorrowedFactorContext<'_>,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        let plan = self.compile(definitions, outputs)?;
        self.evaluate_compiled(&plan, context)
    }

    /// Evaluate a previously compiled graph without rebuilding its definition map.
    pub fn evaluate_compiled(
        &self,
        plan: &CompiledCompositePlan,
        context: &BorrowedFactorContext<'_>,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        let definitions = plan
            .definitions
            .iter()
            .map(|definition| (definition.name.as_str(), definition))
            .collect::<BTreeMap<_, _>>();
        let mut cache = BTreeMap::new();
        let mut visiting = Vec::new();
        for output in &plan.outputs {
            self.eval_named(output, &definitions, context, &mut cache, &mut visiting)?;
        }
        Ok(plan
            .outputs
            .iter()
            .filter_map(|name| cache.get(name).map(|values| (name.clone(), values.clone())))
            .collect())
    }

    /// Recompute only the affected rows of a finite-lookback graph.
    ///
    /// The plan is compiled with an explicit capability proof. Graphs that
    /// contain recursive or whole-series functions return an error instead of
    /// silently producing an incorrect partial result.
    pub fn execute_range_borrowed(
        &self,
        plan: &CompiledCompositePlan,
        context: &BorrowedFactorContext<'_>,
        previous: &BTreeMap<String, Vec<f64>>,
        dirty: DirtyRange,
    ) -> FactorResult<RuntimeExecution<BTreeMap<String, Vec<f64>>>> {
        let mut output = previous.clone();
        let trace = self.execute_range_into_borrowed(plan, context, &mut output, dirty)?;
        Ok(RuntimeExecution { output, trace })
    }

    /// In-place form of [`Self::execute_range_borrowed`].
    pub fn execute_range_into_borrowed(
        &self,
        plan: &CompiledCompositePlan,
        context: &BorrowedFactorContext<'_>,
        output: &mut BTreeMap<String, Vec<f64>>,
        dirty: DirtyRange,
    ) -> FactorResult<RuntimeExecutionTrace> {
        let rows = context.len();
        if !dirty.is_within(rows) {
            return Err(FactorError::InvalidParameter(format!(
                "dirty range {}..{} exceeds composite input rows {rows}",
                dirty.start, dirty.end
            )));
        }
        let lookback = plan.range_lookback.ok_or_else(|| {
            FactorError::InvalidParameter(
                "composite plan is not range-safe: recursive, whole-series, or custom functions require full execution"
                    .to_string(),
            )
        })?;
        for name in &plan.outputs {
            let values = output.get(name).ok_or_else(|| {
                FactorError::InvalidParameter(format!(
                    "dirty-range execution requires retained output for {name}"
                ))
            })?;
            if values.len() != rows {
                return Err(FactorError::LengthMismatch {
                    name: name.clone(),
                    expected: rows,
                    actual: values.len(),
                });
            }
        }
        if dirty.is_empty() {
            return Ok(RuntimeExecutionTrace {
                mode: RuntimeExecutionMode::Range {
                    input_dirty: dirty,
                    affected: dirty,
                    recompute: dirty,
                },
                rows,
                executed_nodes: 0,
                recomputed_rows: 0,
            });
        }

        let affected = dirty.propagate_forward(lookback, rows);
        let recompute = affected.with_lookback(lookback);
        let mut sliced = BorrowedFactorContext::new();
        for input in &plan.required_raw_inputs {
            let values = context
                .get(input)
                .ok_or_else(|| FactorError::MissingInput(input.clone()))?;
            sliced.insert(input.clone(), &values[recompute.as_range()])?;
        }
        let partial = self.evaluate_compiled(plan, &sliced)?;
        let offset = affected.start - recompute.start;
        let source_end = offset + affected.len();
        for name in &plan.outputs {
            let source = partial.get(name).ok_or_else(|| {
                FactorError::InvalidParameter(format!(
                    "composite runtime did not materialize planned output {name}"
                ))
            })?;
            if source_end > source.len() {
                return Err(FactorError::LengthMismatch {
                    name: name.clone(),
                    expected: source_end,
                    actual: source.len(),
                });
            }
            let target = output.get_mut(name).ok_or_else(|| {
                FactorError::InvalidParameter(format!(
                    "dirty-range execution requires retained output for {name}"
                ))
            })?;
            target[affected.as_range()].copy_from_slice(&source[offset..source_end]);
        }

        Ok(RuntimeExecutionTrace {
            mode: RuntimeExecutionMode::Range {
                input_dirty: dirty,
                affected,
                recompute,
            },
            rows,
            executed_nodes: plan.definitions.len(),
            recomputed_rows: recompute.len(),
        })
    }

    /// Register the standard functions used by daily indicator composition.
    fn register_builtins(&mut self) {
        let _ = self.register(
            "sma",
            Arc::new(|inputs, params| {
                unary_input(inputs, "sma")
                    .and_then(|input| Ok(rolling_sma(input, period(params, 14)?)))
            }),
        );
        let _ = self.register(
            "ema",
            Arc::new(|inputs, params| {
                unary_input(inputs, "ema")
                    .and_then(|input| Ok(rolling_ema(input, period(params, 14)?)))
            }),
        );
        let _ = self.register(
            "wma",
            Arc::new(|inputs, params| {
                unary_input(inputs, "wma")
                    .and_then(|input| Ok(rolling_wma(input, period(params, 14)?)))
            }),
        );
        let _ = self.register(
            "rsi",
            Arc::new(|inputs, params| {
                unary_input(inputs, "rsi").and_then(|input| {
                    indicators::momentum::rsi(input, period(params, 14)?)
                        .map(|value| value.into_raw_vec())
                        .map_err(ta_error)
                })
            }),
        );
        let _ = self.register(
            "atr",
            Arc::new(|inputs, params| {
                if inputs.len() != 3 {
                    return Err(FactorError::InvalidParameter(
                        "atr requires high, low and close".to_string(),
                    ));
                }
                volatility::atr(inputs[0], inputs[1], inputs[2], period(params, 14)?)
                    .map(|value| value.into_raw_vec())
                    .map_err(ta_error)
            }),
        );
        let _ = self.register(
            "return",
            Arc::new(|inputs, params| {
                unary_input(inputs, "return")
                    .and_then(|input| crate::factors::time_series_return(input, period(params, 1)?))
            }),
        );
        let _ = self.register(
            "zscore",
            Arc::new(|inputs, _| unary_input(inputs, "zscore").map(zscore)),
        );
        let _ = self.register(
            "vwma",
            Arc::new(|inputs, params| {
                if inputs.len() != 2 {
                    return Err(FactorError::InvalidParameter(
                        "vwma requires input and volume".to_string(),
                    ));
                }
                moving_avg::vwma(inputs[0], inputs[1], period(params, 14)?)
                    .map(|value| value.into_raw_vec())
                    .map_err(ta_error)
            }),
        );
        let _ = self.register(
            "macd",
            Arc::new(|inputs, params| {
                let input = unary_input(inputs, "macd")?;
                let fast = period_at(params, 0, 12)?;
                let slow = period_at(params, 1, 26)?;
                let signal = period_at(params, 2, 9)?;
                momentum::macd(input, fast, slow, signal)
                    .map(|value| value.hist.into_raw_vec())
                    .map_err(ta_error)
            }),
        );
        let _ = self.register(
            "boll_mid",
            Arc::new(|inputs, params| {
                let input = unary_input(inputs, "boll_mid")?;
                indicators::overlap::bbands(input, period(params, 20)?, 2.0, 2.0)
                    .map(|value| value.middle.into_raw_vec())
                    .map_err(ta_error)
            }),
        );
        let _ = self.register(
            "boll_upper",
            Arc::new(|inputs, params| {
                let input = unary_input(inputs, "boll_upper")?;
                let deviation = params.get(1).copied().unwrap_or(2.0);
                indicators::overlap::bbands(input, period(params, 20)?, deviation, deviation)
                    .map(|value| value.upper.into_raw_vec())
                    .map_err(ta_error)
            }),
        );
        let _ = self.register(
            "boll_lower",
            Arc::new(|inputs, params| {
                let input = unary_input(inputs, "boll_lower")?;
                let deviation = params.get(1).copied().unwrap_or(2.0);
                indicators::overlap::bbands(input, period(params, 20)?, deviation, deviation)
                    .map(|value| value.lower.into_raw_vec())
                    .map_err(ta_error)
            }),
        );
        let _ = self.register(
            "threshold",
            Arc::new(|inputs, params| {
                let input = unary_input(inputs, "threshold")?;
                let level = params.first().copied().unwrap_or(0.0);
                if !level.is_finite() {
                    return Err(FactorError::InvalidParameter(
                        "threshold must be finite".to_string(),
                    ));
                }
                Ok(input
                    .iter()
                    .map(|value| {
                        if value.is_finite() {
                            (*value >= level) as u8 as f64
                        } else {
                            f64::NAN
                        }
                    })
                    .collect())
            }),
        );
        let _ = self.register(
            "between",
            Arc::new(|inputs, params| {
                let input = unary_input(inputs, "between")?;
                let lower = params.first().copied().unwrap_or(0.0);
                let upper = params.get(1).copied().unwrap_or(1.0);
                if !lower.is_finite() || !upper.is_finite() || lower > upper {
                    return Err(FactorError::InvalidParameter(
                        "between bounds must be finite and ordered".to_string(),
                    ));
                }
                Ok(input
                    .iter()
                    .map(|value| {
                        if value.is_finite() {
                            (*value >= lower && *value <= upper) as u8 as f64
                        } else {
                            f64::NAN
                        }
                    })
                    .collect())
            }),
        );
        let _ = self.register(
            "clip",
            Arc::new(|inputs, params| {
                let input = unary_input(inputs, "clip")?;
                let lower = params.first().copied().unwrap_or(-f64::MAX);
                let upper = params.get(1).copied().unwrap_or(f64::MAX);
                if !lower.is_finite() || !upper.is_finite() || lower > upper {
                    return Err(FactorError::InvalidParameter(
                        "clip bounds must be finite and ordered".to_string(),
                    ));
                }
                Ok(input
                    .iter()
                    .map(|value| {
                        value
                            .is_finite()
                            .then(|| value.clamp(lower, upper))
                            .unwrap_or(f64::NAN)
                    })
                    .collect())
            }),
        );
        for (name, operation) in [("abs", 0_u8), ("neg", 1_u8), ("sign", 2_u8)] {
            let _ = self.register(
                name,
                Arc::new(move |inputs, _| {
                    let input = unary_input(inputs, name)?;
                    Ok(input
                        .iter()
                        .map(|value| {
                            if !value.is_finite() {
                                f64::NAN
                            } else {
                                match operation {
                                    0 => value.abs(),
                                    1 => -*value,
                                    _ => value.signum(),
                                }
                            }
                        })
                        .collect())
                }),
            );
        }
        for (name, operation) in [
            ("rolling_std", 0_u8),
            ("rolling_min", 1_u8),
            ("rolling_max", 2_u8),
        ] {
            let _ = self.register(
                name,
                Arc::new(move |inputs, params| {
                    let input = unary_input(inputs, name)?;
                    let window = period(params, 20)?;
                    match operation {
                        0 => statistics::rolling_std_dev(input, window),
                        1 => statistics::rolling_min(input, window),
                        _ => statistics::rolling_max(input, window),
                    }
                    .map(|values| values.into_raw_vec())
                    .map_err(ta_error)
                }),
            );
        }
        let _ = self.register(
            "volatility",
            Arc::new(|inputs, params| {
                unary_input(inputs, "volatility").and_then(|input| {
                    crate::factors::rolling_volatility(input, period(params, 20)?)
                })
            }),
        );
        let _ = self.register("cross_up", Arc::new(|inputs, _| cross(inputs, true)));
        let _ = self.register("cross_down", Arc::new(|inputs, _| cross(inputs, false)));
        let _ = self.register(
            "weighted_average",
            Arc::new(|inputs, params| weighted_average(inputs, params)),
        );
    }

    fn eval_named(
        &self,
        name: &str,
        definitions: &BTreeMap<&str, &CompositeDefinition>,
        context: &BorrowedFactorContext<'_>,
        cache: &mut BTreeMap<String, Vec<f64>>,
        visiting: &mut Vec<String>,
    ) -> FactorResult<()> {
        if cache.contains_key(name) {
            return Ok(());
        }
        if let Some(position) = visiting.iter().position(|current| current == name) {
            let mut cycle = visiting[position..].to_vec();
            cycle.push(name.to_string());
            return Err(FactorError::DependencyCycle(cycle));
        }
        let definition = definitions
            .get(name)
            .ok_or_else(|| FactorError::UnknownFactor(name.to_string()))?;
        visiting.push(name.to_string());
        let values = self
            .eval_expr(
                &definition.expression,
                definitions,
                context,
                cache,
                visiting,
            )?
            .into_owned();
        if values.len() != context.len() {
            return Err(FactorError::LengthMismatch {
                name: name.to_string(),
                expected: context.len(),
                actual: values.len(),
            });
        }
        cache.insert(name.to_string(), values);
        visiting.pop();
        Ok(())
    }

    fn eval_expr<'a>(
        &self,
        expression: &CompositeExpr,
        definitions: &BTreeMap<&str, &CompositeDefinition>,
        context: &BorrowedFactorContext<'a>,
        cache: &mut BTreeMap<String, Vec<f64>>,
        visiting: &mut Vec<String>,
    ) -> FactorResult<CompositeValue<'a>> {
        match expression {
            CompositeExpr::Series(name) => context
                .get(name)
                .map(CompositeValue::Borrowed)
                .ok_or_else(|| FactorError::MissingInput(name.clone())),
            CompositeExpr::Constant(value) => {
                Ok(CompositeValue::Owned(vec![*value; context.len()]))
            }
            CompositeExpr::Ref(name) => {
                self.eval_named(name, definitions, context, cache, visiting)?;
                cache
                    .get(name)
                    .cloned()
                    .map(CompositeValue::Owned)
                    .ok_or_else(|| FactorError::UnknownFactor(name.clone()))
            }
            CompositeExpr::Call {
                name,
                inputs,
                params,
            } => {
                let values: Vec<CompositeValue<'_>> = inputs
                    .iter()
                    .map(|input| self.eval_expr(input, definitions, context, cache, visiting))
                    .collect::<FactorResult<_>>()?;
                let views: Vec<&[f64]> = values.iter().map(CompositeValue::as_slice).collect();
                let function = self
                    .functions
                    .get(&name.to_ascii_lowercase())
                    .ok_or_else(|| FactorError::UnknownFactor(name.clone()))?;
                let result = function(&views, params)?;
                if result.len() != context.len() {
                    return Err(FactorError::LengthMismatch {
                        name: name.clone(),
                        expected: context.len(),
                        actual: result.len(),
                    });
                }
                Ok(CompositeValue::Owned(result))
            }
            CompositeExpr::Op { op, inputs } => {
                let values: Vec<CompositeValue<'_>> = inputs
                    .iter()
                    .map(|input| self.eval_expr(input, definitions, context, cache, visiting))
                    .collect::<FactorResult<_>>()?;
                apply_op(*op, &values, &[])
            }
        }
    }
}

/// Persistable checkpoint for [`CompositeStream`].
#[derive(Debug, Clone, PartialEq)]
pub struct CompositeStreamCheckpoint {
    signature: u64,
    row_count: usize,
    inputs: BTreeMap<String, Vec<f64>>,
    output: BTreeMap<String, Vec<f64>>,
}

impl CompositeStreamCheckpoint {
    /// Stable graph signature for the checkpointed plan.
    #[must_use]
    pub const fn signature(&self) -> u64 {
        self.signature
    }

    /// Number of rows represented by this checkpoint.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.row_count
    }

    /// Retained raw input window.
    #[must_use]
    pub fn inputs(&self) -> &BTreeMap<String, Vec<f64>> {
        &self.inputs
    }

    /// Retained materialized output window.
    #[must_use]
    pub fn outputs(&self) -> &BTreeMap<String, Vec<f64>> {
        &self.output
    }

    /// Construct a checkpoint payload for a compiled graph.
    #[must_use]
    pub fn from_parts(
        signature: u64,
        row_count: usize,
        inputs: BTreeMap<String, Vec<f64>>,
        output: BTreeMap<String, Vec<f64>>,
    ) -> Self {
        Self {
            signature,
            row_count,
            inputs,
            output,
        }
    }
}

/// Append-only bounded-window composite executor with checkpoint/restore.
#[derive(Clone)]
pub struct CompositeStream {
    plan: CompiledCompositePlan,
    engine: CompositeEngine,
    row_count: usize,
    inputs: BTreeMap<String, Vec<f64>>,
    output: BTreeMap<String, Vec<f64>>,
}

impl CompositeStream {
    /// Compiled plan used by this stream.
    #[must_use]
    pub fn plan(&self) -> &CompiledCompositePlan {
        &self.plan
    }

    /// Number of rows accepted by the stream.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.row_count
    }

    /// Return the retained output series for all requested graph outputs.
    #[must_use]
    pub fn outputs(&self) -> &BTreeMap<String, Vec<f64>> {
        &self.output
    }

    /// Append one aligned row and return the newly computed graph outputs.
    pub fn push_row(
        &mut self,
        values: &BTreeMap<String, f64>,
    ) -> FactorResult<BTreeMap<String, f64>> {
        let ordered = self
            .plan
            .required_raw_inputs
            .iter()
            .map(|input| {
                values
                    .get(input)
                    .copied()
                    .ok_or_else(|| FactorError::MissingInput(input.clone()))
            })
            .collect::<FactorResult<Vec<_>>>()?;
        self.push_values(&ordered)
    }

    /// Append one row in the plan's sorted raw-input order.
    ///
    /// This avoids constructing a string-keyed map in hot loops. The order is
    /// exactly [`CompiledCompositePlan::required_raw_inputs`].
    pub fn push_values(&mut self, values: &[f64]) -> FactorResult<BTreeMap<String, f64>> {
        if values.len() != self.plan.required_raw_inputs.len() {
            return Err(FactorError::LengthMismatch {
                name: "composite_stream_row".to_string(),
                expected: self.plan.required_raw_inputs.len(),
                actual: values.len(),
            });
        }

        let row = self.inputs.values().next().map_or(0, Vec::len);
        for (input, value) in self
            .plan
            .required_raw_inputs
            .iter()
            .zip(values.iter().copied())
        {
            self.inputs.entry(input.clone()).or_default().push(value);
        }
        for output in &self.plan.outputs {
            self.output
                .entry(output.clone())
                .or_default()
                .push(f64::NAN);
        }

        let result = (|| {
            let context = composite_borrowed_context(&self.inputs)?;
            self.engine.execute_range_into_borrowed(
                &self.plan,
                &context,
                &mut self.output,
                DirtyRange::new(row, row + 1),
            )?;
            Ok(self
                .plan
                .outputs
                .iter()
                .map(|name| {
                    (
                        name.clone(),
                        self.output
                            .get(name)
                            .and_then(|series| series.last().copied())
                            .unwrap_or(f64::NAN),
                    )
                })
                .collect())
        })();

        if result.is_ok() {
            self.row_count = self.row_count.saturating_add(1);
            let capacity = self.plan.range_lookback().unwrap_or(0).saturating_add(1);
            if self
                .inputs
                .values()
                .next()
                .is_some_and(|series| series.len() > capacity)
            {
                for input in &self.plan.required_raw_inputs {
                    if let Some(series) = self.inputs.get_mut(input) {
                        series.remove(0);
                    }
                }
                for output in &self.plan.outputs {
                    if let Some(series) = self.output.get_mut(output) {
                        series.remove(0);
                    }
                }
            }
        } else {
            for input in &self.plan.required_raw_inputs {
                if let Some(series) = self.inputs.get_mut(input) {
                    series.pop();
                }
            }
            for output in &self.plan.outputs {
                if let Some(series) = self.output.get_mut(output) {
                    series.pop();
                }
            }
        }
        result
    }

    /// Append an aligned batch and execute one range evaluation for the batch.
    ///
    /// This is the preferred ingestion path for adapters and data feeds. The
    /// input map may contain extra columns, but every raw input required by the
    /// plan must be present and all required columns must have equal lengths.
    pub fn push_batch(
        &mut self,
        values: &BTreeMap<String, Vec<f64>>,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        let mut emitted = BTreeMap::new();
        self.push_batch_into(values, &mut emitted)?;
        Ok(emitted)
    }

    /// Append an aligned batch and write graph outputs into reusable buffers.
    ///
    /// The caller owns `emitted` and may keep it across calls. Existing vector
    /// capacity is reused, so steady-state ingestion does not allocate a new
    /// result vector for every batch. On validation or execution failure the
    /// output buffers are left unchanged.
    pub fn push_batch_into(
        &mut self,
        values: &BTreeMap<String, Vec<f64>>,
        emitted: &mut BTreeMap<String, Vec<f64>>,
    ) -> FactorResult<()> {
        let required = &self.plan.required_raw_inputs;
        let rows = required
            .first()
            .and_then(|name| values.get(name))
            .ok_or_else(|| {
                FactorError::MissingInput(
                    required
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "composite_stream_input".to_string()),
                )
            })?
            .len();
        for input in required {
            let series = values
                .get(input)
                .ok_or_else(|| FactorError::MissingInput(input.clone()))?;
            if series.len() != rows {
                return Err(FactorError::LengthMismatch {
                    name: input.clone(),
                    expected: rows,
                    actual: series.len(),
                });
            }
        }
        if rows == 0 {
            for name in &self.plan.outputs {
                emitted.entry(name.clone()).or_default().clear();
            }
            return Ok(());
        }

        let previous_rows = self.inputs.values().next().map_or(0, Vec::len);
        for input in required {
            self.inputs
                .entry(input.clone())
                .or_default()
                .extend_from_slice(&values[input]);
        }
        let total_rows = previous_rows + rows;
        for output in &self.plan.outputs {
            self.output
                .entry(output.clone())
                .or_default()
                .resize(total_rows, f64::NAN);
        }

        let result = (|| {
            let context = composite_borrowed_context(&self.inputs)?;
            self.engine.execute_range_into_borrowed(
                &self.plan,
                &context,
                &mut self.output,
                DirtyRange::new(previous_rows, total_rows),
            )?;
            Ok(())
        })();

        if result.is_ok() {
            for name in &self.plan.outputs {
                let target = emitted.entry(name.clone()).or_default();
                target.clear();
                if let Some(series) = self.output.get(name) {
                    target.extend_from_slice(&series[previous_rows..total_rows]);
                }
            }
            self.row_count = self.row_count.saturating_add(rows);
            let capacity = self.plan.range_lookback().unwrap_or(0).saturating_add(1);
            let retained = self.inputs.values().next().map_or(0, Vec::len);
            let excess = retained.saturating_sub(capacity);
            if excess > 0 {
                for input in required {
                    if let Some(series) = self.inputs.get_mut(input) {
                        series.drain(..excess);
                    }
                }
                for output in &self.plan.outputs {
                    if let Some(series) = self.output.get_mut(output) {
                        series.drain(..excess);
                    }
                }
            }
        } else {
            for input in required {
                if let Some(series) = self.inputs.get_mut(input) {
                    series.truncate(previous_rows);
                }
            }
            for output in &self.plan.outputs {
                if let Some(series) = self.output.get_mut(output) {
                    series.truncate(previous_rows);
                }
            }
        }
        result
    }

    /// Capture a portable checkpoint at the current append boundary.
    #[must_use]
    pub fn checkpoint(&self) -> CompositeStreamCheckpoint {
        CompositeStreamCheckpoint {
            signature: self.plan.signature,
            row_count: self.row_count,
            inputs: self.inputs.clone(),
            output: self.output.clone(),
        }
    }

    /// Restore a checkpoint produced by the same compiled graph.
    pub fn restore(&mut self, checkpoint: &CompositeStreamCheckpoint) -> FactorResult<()> {
        if checkpoint.signature != self.plan.signature {
            return Err(FactorError::InvalidParameter(
                "composite stream checkpoint belongs to a different graph".to_string(),
            ));
        }
        validate_composite_stream_checkpoint(
            &self.plan,
            checkpoint.row_count,
            &checkpoint.inputs,
            &checkpoint.output,
        )?;
        self.row_count = checkpoint.row_count;
        self.inputs = checkpoint.inputs.clone();
        self.output = checkpoint.output.clone();
        Ok(())
    }
}

fn composite_borrowed_context(
    inputs: &BTreeMap<String, Vec<f64>>,
) -> FactorResult<BorrowedFactorContext<'_>> {
    let mut context = BorrowedFactorContext::new();
    for (name, values) in inputs {
        context.insert(name.clone(), values.as_slice())?;
    }
    Ok(context)
}

fn validate_composite_stream_checkpoint(
    plan: &CompiledCompositePlan,
    row_count: usize,
    inputs: &BTreeMap<String, Vec<f64>>,
    output: &BTreeMap<String, Vec<f64>>,
) -> FactorResult<()> {
    let rows = inputs.values().next().map_or(0, Vec::len);
    if rows > row_count {
        return Err(FactorError::InvalidParameter(
            "composite stream checkpoint buffer exceeds its logical row count".to_string(),
        ));
    }
    for input in &plan.required_raw_inputs {
        let values = inputs
            .get(input)
            .ok_or_else(|| FactorError::MissingInput(input.clone()))?;
        if values.len() != rows {
            return Err(FactorError::LengthMismatch {
                name: input.clone(),
                expected: rows,
                actual: values.len(),
            });
        }
    }
    for name in &plan.outputs {
        let values = output.get(name).ok_or_else(|| {
            FactorError::InvalidParameter(format!(
                "composite stream checkpoint is missing output {name}"
            ))
        })?;
        if values.len() != rows {
            return Err(FactorError::LengthMismatch {
                name: name.clone(),
                expected: rows,
                actual: values.len(),
            });
        }
    }
    Ok(())
}

impl Default for CompositeEngine {
    fn default() -> Self {
        Self {
            functions: BTreeMap::new(),
            builtin_functions: BTreeSet::new(),
            stateful_specs: BTreeMap::new(),
            compiled_plans: BTreeMap::new(),
            compiled_plan_cache_hits: 0,
            compiled_plan_cache_misses: 0,
            compiled_plan_cache_clock: 0,
            cache: BTreeMap::new(),
            cache_capacity: 64,
            cache_clock: 0,
        }
    }
}

/// Calculate the stable graph signature used by compiled plans and caches.
pub fn graph_signature(definitions: &[CompositeDefinition], outputs: &[&str]) -> u64 {
    let signature = format!("{definitions:?}|{outputs:?}");
    let mut hash = 0xcbf29ce484222325u64;
    for byte in signature.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn validate_named_graph<'a>(
    name: &str,
    definitions: &BTreeMap<&'a str, &'a CompositeDefinition>,
    visiting: &mut Vec<String>,
) -> FactorResult<()> {
    if let Some(position) = visiting.iter().position(|current| current == name) {
        let mut cycle = visiting[position..].to_vec();
        cycle.push(name.to_string());
        return Err(FactorError::DependencyCycle(cycle));
    }
    let definition = definitions
        .get(name)
        .ok_or_else(|| FactorError::UnknownFactor(name.to_string()))?;
    visiting.push(name.to_string());
    validate_expression_refs(&definition.expression, definitions, visiting)?;
    visiting.pop();
    Ok(())
}

fn validate_expression_refs<'a>(
    expression: &CompositeExpr,
    definitions: &BTreeMap<&'a str, &'a CompositeDefinition>,
    visiting: &mut Vec<String>,
) -> FactorResult<()> {
    match expression {
        CompositeExpr::Ref(name) => validate_named_graph(name, definitions, visiting),
        CompositeExpr::Call { inputs, .. } | CompositeExpr::Op { inputs, .. } => {
            for input in inputs {
                validate_expression_refs(input, definitions, visiting)?;
            }
            Ok(())
        }
        CompositeExpr::Series(_) | CompositeExpr::Constant(_) => Ok(()),
    }
}

fn collect_raw_inputs<'a>(
    name: &str,
    definitions: &BTreeMap<&'a str, &'a CompositeDefinition>,
    raw_inputs: &mut BTreeSet<String>,
    visiting: &mut Vec<String>,
) -> FactorResult<()> {
    if visiting.iter().any(|current| current == name) {
        return Ok(());
    }
    let definition = definitions
        .get(name)
        .ok_or_else(|| FactorError::UnknownFactor(name.to_string()))?;
    visiting.push(name.to_string());
    collect_expression_raw_inputs(&definition.expression, definitions, raw_inputs, visiting)?;
    visiting.pop();
    Ok(())
}

fn collect_expression_raw_inputs<'a>(
    expression: &CompositeExpr,
    definitions: &BTreeMap<&'a str, &'a CompositeDefinition>,
    raw_inputs: &mut BTreeSet<String>,
    visiting: &mut Vec<String>,
) -> FactorResult<()> {
    match expression {
        CompositeExpr::Series(name) => {
            raw_inputs.insert(name.clone());
            Ok(())
        }
        CompositeExpr::Ref(name) => collect_raw_inputs(name, definitions, raw_inputs, visiting),
        CompositeExpr::Call { inputs, .. } | CompositeExpr::Op { inputs, .. } => {
            inputs.iter().try_for_each(|input| {
                collect_expression_raw_inputs(input, definitions, raw_inputs, visiting)
            })
        }
        CompositeExpr::Constant(_) => Ok(()),
    }
}

fn expression_lookback_for_named<'a>(
    name: &str,
    definitions: &BTreeMap<&'a str, &'a CompositeDefinition>,
    builtin_functions: &BTreeSet<String>,
    memo: &mut BTreeMap<String, Option<usize>>,
    visiting: &mut Vec<String>,
) -> FactorResult<Option<usize>> {
    if let Some(value) = memo.get(name) {
        return Ok(*value);
    }
    if let Some(position) = visiting.iter().position(|current| current == name) {
        let mut cycle = visiting[position..].to_vec();
        cycle.push(name.to_string());
        return Err(FactorError::DependencyCycle(cycle));
    }
    let definition = definitions
        .get(name)
        .ok_or_else(|| FactorError::UnknownFactor(name.to_string()))?;
    visiting.push(name.to_string());
    let value = expression_lookback(
        &definition.expression,
        definitions,
        builtin_functions,
        memo,
        visiting,
    )?;
    visiting.pop();
    memo.insert(name.to_string(), value);
    Ok(value)
}

fn expression_lookback<'a>(
    expression: &CompositeExpr,
    definitions: &BTreeMap<&'a str, &'a CompositeDefinition>,
    builtin_functions: &BTreeSet<String>,
    memo: &mut BTreeMap<String, Option<usize>>,
    visiting: &mut Vec<String>,
) -> FactorResult<Option<usize>> {
    match expression {
        CompositeExpr::Series(_) | CompositeExpr::Constant(_) => Ok(Some(0)),
        CompositeExpr::Ref(name) => {
            expression_lookback_for_named(name, definitions, builtin_functions, memo, visiting)
        }
        CompositeExpr::Op { inputs, .. } => combine_lookbacks(inputs.iter().map(|input| {
            expression_lookback(input, definitions, builtin_functions, memo, visiting)
        })),
        CompositeExpr::Call {
            name,
            inputs,
            params,
        } => {
            let input_lookback = combine_lookbacks(inputs.iter().map(|input| {
                expression_lookback(input, definitions, builtin_functions, memo, visiting)
            }))?;
            let normalized = name.to_ascii_lowercase();
            if !builtin_functions.contains(&normalized) {
                return Ok(None);
            }
            let own_lookback = match normalized.as_str() {
                "sma" | "wma" | "vwma" => period(params, 14)?.saturating_sub(1),
                "boll_mid" | "boll_upper" | "boll_lower" | "rolling_std" | "rolling_min"
                | "rolling_max" => period(params, 20)?.saturating_sub(1),
                "return" => period(params, 1)?,
                "volatility" => period(params, 20)?,
                "cross_up" | "cross_down" => 1,
                "threshold" | "between" | "clip" | "abs" | "neg" | "sign" | "weighted_average" => 0,
                // These functions carry recursive state. A changed input can
                // affect every subsequent row, so range execution must fall
                // back to a full pass until a checkpointed state API exists.
                "ema" | "rsi" | "atr" | "macd" | "zscore" => return Ok(None),
                _ => return Ok(None),
            };
            Ok(input_lookback.map(|value| value.saturating_add(own_lookback)))
        }
    }
}

fn combine_lookbacks<I>(values: I) -> FactorResult<Option<usize>>
where
    I: IntoIterator<Item = FactorResult<Option<usize>>>,
{
    let mut maximum = 0usize;
    for value in values {
        match value? {
            Some(value) => maximum = maximum.max(value),
            None => return Ok(None),
        }
    }
    Ok(Some(maximum))
}

fn builtin_function_names() -> BTreeSet<String> {
    [
        "sma",
        "ema",
        "wma",
        "rsi",
        "atr",
        "return",
        "zscore",
        "vwma",
        "macd",
        "boll_mid",
        "boll_upper",
        "boll_lower",
        "threshold",
        "between",
        "clip",
        "abs",
        "neg",
        "sign",
        "rolling_std",
        "rolling_min",
        "rolling_max",
        "volatility",
        "cross_up",
        "cross_down",
        "weighted_average",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn builtin_stateful_specs() -> BTreeMap<String, StatefulCompositeSpec> {
    [
        ("sma", StatefulCompositeSpec::Sma),
        ("wma", StatefulCompositeSpec::Wma),
        ("ema", StatefulCompositeSpec::Ema),
        ("rsi", StatefulCompositeSpec::Rsi),
        ("atr", StatefulCompositeSpec::Atr),
        ("macd", StatefulCompositeSpec::Macd),
        ("boll_mid", StatefulCompositeSpec::BollMid),
        ("boll_upper", StatefulCompositeSpec::BollUpper),
        ("boll_lower", StatefulCompositeSpec::BollLower),
        ("rolling_std", StatefulCompositeSpec::RollingStd),
        ("rolling_min", StatefulCompositeSpec::RollingMin),
        ("rolling_max", StatefulCompositeSpec::RollingMax),
        ("return", StatefulCompositeSpec::Return),
        ("vwma", StatefulCompositeSpec::Vwma),
        ("volatility", StatefulCompositeSpec::Volatility),
        ("threshold", StatefulCompositeSpec::Threshold),
        ("between", StatefulCompositeSpec::Between),
        ("clip", StatefulCompositeSpec::Clip),
        ("abs", StatefulCompositeSpec::Abs),
        ("neg", StatefulCompositeSpec::Neg),
        ("sign", StatefulCompositeSpec::Sign),
        ("weighted_average", StatefulCompositeSpec::WeightedAverage),
        ("cross_up", StatefulCompositeSpec::CrossUp),
        ("cross_down", StatefulCompositeSpec::CrossDown),
    ]
    .into_iter()
    .map(|(name, spec)| (name.to_string(), spec))
    .collect()
}

fn period(params: &[f64], default: usize) -> FactorResult<usize> {
    period_at(params, 0, default)
}

fn ta_error(error: crate::error::TaError) -> FactorError {
    FactorError::Compute(error.to_string())
}

fn rolling_sma(values: &[f64], period: usize) -> Vec<f64> {
    let mut output = vec![f64::NAN; values.len()];
    if period == 0 {
        return output;
    }
    for index in period.saturating_sub(1)..values.len() {
        let window = &values[index + 1 - period..=index];
        if window.iter().all(|value| value.is_finite()) {
            output[index] = window.iter().sum::<f64>() / period as f64;
        }
    }
    output
}

fn rolling_ema(values: &[f64], period: usize) -> Vec<f64> {
    let mut output = vec![f64::NAN; values.len()];
    if period == 0 {
        return output;
    }
    let alpha = 2.0 / (period as f64 + 1.0);
    let mut previous = f64::NAN;
    for index in 0..values.len() {
        if !values[index].is_finite() {
            previous = f64::NAN;
            continue;
        }
        if previous.is_finite() {
            previous += alpha * (values[index] - previous);
            output[index] = previous;
        } else if index + 1 >= period
            && values[index + 1 - period..=index]
                .iter()
                .all(|value| value.is_finite())
        {
            previous = values[index + 1 - period..=index].iter().sum::<f64>() / period as f64;
            output[index] = previous;
        }
    }
    output
}

fn rolling_wma(values: &[f64], period: usize) -> Vec<f64> {
    let mut output = vec![f64::NAN; values.len()];
    if period == 0 {
        return output;
    }
    let denominator = (period * (period + 1) / 2) as f64;
    for index in period.saturating_sub(1)..values.len() {
        let window = &values[index + 1 - period..=index];
        if window.iter().all(|value| value.is_finite()) {
            output[index] = window
                .iter()
                .enumerate()
                .map(|(offset, value)| *value * (offset + 1) as f64)
                .sum::<f64>()
                / denominator;
        }
    }
    output
}

fn period_at(params: &[f64], index: usize, default: usize) -> FactorResult<usize> {
    let value = params.get(index).copied().unwrap_or(default as f64);
    if !value.is_finite() || value < 1.0 || value.fract() != 0.0 {
        return Err(FactorError::InvalidParameter(
            "period must be a positive integer".to_string(),
        ));
    }
    Ok(value as usize)
}

fn unary_input<'a>(inputs: &'a [&[f64]], name: &str) -> FactorResult<&'a [f64]> {
    if inputs.len() != 1 {
        return Err(FactorError::InvalidParameter(format!(
            "{name} requires one input"
        )));
    }
    Ok(inputs[0])
}

fn cross(inputs: &[&[f64]], upward: bool) -> FactorResult<Vec<f64>> {
    if inputs.len() != 2 || inputs[0].len() != inputs[1].len() {
        return Err(FactorError::InvalidParameter(
            "cross requires two aligned inputs".to_string(),
        ));
    }
    let mut output = vec![0.0; inputs[0].len()];
    for index in 1..output.len() {
        let (previous_a, previous_b) = (inputs[0][index - 1], inputs[1][index - 1]);
        let (current_a, current_b) = (inputs[0][index], inputs[1][index]);
        if [previous_a, previous_b, current_a, current_b]
            .iter()
            .all(|value| value.is_finite())
        {
            output[index] = if upward {
                (previous_a <= previous_b && current_a > current_b) as u8 as f64
            } else {
                (previous_a >= previous_b && current_a < current_b) as u8 as f64
            };
        }
    }
    Ok(output)
}

fn weighted_average(inputs: &[&[f64]], weights: &[f64]) -> FactorResult<Vec<f64>> {
    if inputs.is_empty() {
        return Err(FactorError::InvalidParameter(
            "weighted_average needs inputs".to_string(),
        ));
    }
    let len = inputs[0].len();
    if inputs.iter().any(|input| input.len() != len) {
        return Err(FactorError::InvalidParameter(
            "weighted_average inputs must be aligned".to_string(),
        ));
    }
    if !weights.is_empty() && weights.len() != inputs.len() {
        return Err(FactorError::InvalidParameter(
            "weights must match input count".to_string(),
        ));
    }
    let weights = if weights.is_empty() {
        vec![1.0; inputs.len()]
    } else {
        weights.to_vec()
    };
    if weights.iter().any(|weight| !weight.is_finite()) {
        return Err(FactorError::InvalidParameter(
            "weights must be finite".to_string(),
        ));
    }
    let total: f64 = weights.iter().map(|value| value.abs()).sum();
    let mut output = vec![f64::NAN; len];
    if total == 0.0 {
        return Ok(output);
    }
    for index in 0..len {
        if inputs.iter().any(|input| !input[index].is_finite()) {
            continue;
        }
        output[index] = inputs
            .iter()
            .zip(weights.iter())
            .map(|(input, weight)| input[index] * weight)
            .sum::<f64>()
            / total;
    }
    Ok(output)
}

fn apply_op(
    op: CompositeOp,
    values: &[CompositeValue<'_>],
    weights: &[f64],
) -> FactorResult<CompositeValue<'static>> {
    if values.is_empty() {
        return Err(FactorError::InvalidParameter(
            "composite operation needs inputs".to_string(),
        ));
    }
    let len = values[0].as_slice().len();
    if values.iter().any(|value| value.as_slice().len() != len) {
        return Err(FactorError::InvalidParameter(
            "composite inputs must be aligned".to_string(),
        ));
    }
    let mut output = vec![f64::NAN; len];
    for index in 0..len {
        let row: Vec<f64> = values.iter().map(|value| value.as_slice()[index]).collect();
        if row.iter().any(|value| !value.is_finite()) {
            continue;
        }
        output[index] = match op {
            CompositeOp::Add => row.iter().sum(),
            CompositeOp::Sub => row[1..].iter().fold(row[0], |value, next| value - next),
            CompositeOp::Mul => row.iter().product(),
            CompositeOp::Div => row[1..]
                .iter()
                .try_fold(row[0], |value, next| {
                    if *next == 0.0 {
                        None
                    } else {
                        Some(value / next)
                    }
                })
                .unwrap_or(f64::NAN),
            CompositeOp::Min => row.iter().copied().fold(f64::INFINITY, f64::min),
            CompositeOp::Max => row.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            CompositeOp::WeightedAverage => {
                let weights = if weights.is_empty() {
                    vec![1.0; row.len()]
                } else {
                    weights.to_vec()
                };
                if weights.len() != row.len() {
                    return Err(FactorError::InvalidParameter(
                        "weights must match input count".to_string(),
                    ));
                }
                let total: f64 = weights.iter().map(|value| value.abs()).sum();
                if total == 0.0 {
                    f64::NAN
                } else {
                    row.iter()
                        .zip(weights.iter())
                        .map(|(value, weight)| value * weight)
                        .sum::<f64>()
                        / total
                }
            }
        };
    }
    Ok(CompositeValue::Owned(output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    #[test]
    fn evaluates_shared_named_dependencies_once_and_composes_them() {
        let close: Vec<f64> = (1..=40).map(|value| value as f64).collect();
        let context = FactorContext::new()
            .with_series("close", close)
            .expect("valid context");
        let definitions = vec![
            CompositeDefinition::new(
                "fast",
                CompositeExpr::call("ema", vec![CompositeExpr::series("close")], vec![5.0]),
            ),
            CompositeDefinition::new(
                "slow",
                CompositeExpr::call("ema", vec![CompositeExpr::series("close")], vec![10.0]),
            ),
            CompositeDefinition::new(
                "spread",
                CompositeExpr::Op {
                    op: CompositeOp::Sub,
                    inputs: vec![
                        CompositeExpr::reference("fast"),
                        CompositeExpr::reference("slow"),
                    ],
                },
            ),
            CompositeDefinition::new(
                "signal",
                CompositeExpr::call("sma", vec![CompositeExpr::reference("spread")], vec![3.0]),
            ),
        ];
        let result = CompositeEngine::new()
            .evaluate(&definitions, &["signal"], &context)
            .expect("graph evaluates");
        assert_eq!(result["signal"].len(), 40);
        assert!(result["signal"][39].is_finite());
    }

    #[test]
    fn detects_definition_cycles() {
        let context = FactorContext::new()
            .with_series("close", vec![1.0, 2.0])
            .expect("context");
        let definitions = vec![
            CompositeDefinition::new("a", CompositeExpr::reference("b")),
            CompositeDefinition::new("b", CompositeExpr::reference("a")),
        ];
        let error = CompositeEngine::new()
            .evaluate(&definitions, &["a"], &context)
            .expect_err("cycle must fail");
        assert!(matches!(error, FactorError::DependencyCycle(_)));
    }

    #[test]
    fn cached_evaluation_reuses_revision_and_invalidates_on_revision_change() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let mut engine = CompositeEngine::new();
        engine
            .register(
                "identity",
                Arc::new(move |inputs, _| {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(inputs[0].to_vec())
                }),
            )
            .expect("register custom function");
        let context = FactorContext::new()
            .with_series("close", vec![1.0, 2.0, 3.0])
            .expect("context");
        let borrowed = context.as_borrowed();
        let definitions = vec![CompositeDefinition::new(
            "value",
            CompositeExpr::call("identity", vec![CompositeExpr::series("close")], vec![]),
        )];
        engine
            .evaluate_cached(&definitions, &["value"], &borrowed, 7)
            .expect("first evaluation");
        assert_eq!(engine.compiled_plans.len(), 1);
        assert_eq!(engine.compiled_plan_cache_hits, 0);
        assert_eq!(engine.compiled_plan_cache_misses, 1);
        engine
            .evaluate_cached(&definitions, &["value"], &borrowed, 7)
            .expect("cached evaluation");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(engine.compiled_plans.len(), 1);
        assert_eq!(engine.compiled_plan_cache_hits, 1);
        assert_eq!(engine.compiled_plan_cache_misses, 1);
        engine
            .evaluate_cached(&definitions, &["value"], &borrowed, 8)
            .expect("new revision evaluation");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(engine.compiled_plans.len(), 1);
        assert_eq!(engine.compiled_plan_cache_hits, 2);
        assert_eq!(engine.compiled_plan_cache_misses, 1);
    }

    #[test]
    fn compiled_plan_cache_is_bounded() {
        let mut engine = CompositeEngine::new();
        for index in 0..=COMPILED_PLAN_CACHE_CAPACITY {
            let name = format!("value_{index}");
            let definitions = [CompositeDefinition::new(
                name.clone(),
                CompositeExpr::Constant(index as f64),
            )];
            let outputs = [name.as_str()];
            engine
                .compile_cached(&definitions, &outputs)
                .expect("compile cached graph");
        }

        assert_eq!(engine.compiled_plan_count(), COMPILED_PLAN_CACHE_CAPACITY);
        assert_eq!(engine.compiled_plan_cache_hits, 0);
        assert_eq!(
            engine.compiled_plan_cache_misses,
            (COMPILED_PLAN_CACHE_CAPACITY + 1) as u64
        );
    }

    #[test]
    fn cached_evaluation_isolated_by_scope_at_the_same_revision() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let mut engine = CompositeEngine::new();
        engine
            .register(
                "identity",
                Arc::new(move |inputs, _| {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(inputs[0].to_vec())
                }),
            )
            .expect("register custom function");
        let first_context = FactorContext::new()
            .with_series("close", vec![1.0, 2.0])
            .expect("first context");
        let second_context = FactorContext::new()
            .with_series("close", vec![10.0, 20.0])
            .expect("second context");
        let first = first_context.as_borrowed();
        let second = second_context.as_borrowed();
        let definitions = [CompositeDefinition::new(
            "value",
            CompositeExpr::call("identity", vec![CompositeExpr::series("close")], vec![]),
        )];

        let first_result = engine
            .evaluate_cached_scoped("AAA@1d", &definitions, &["value"], &first, 1)
            .unwrap();
        let second_result = engine
            .evaluate_cached_scoped("BBB@1d", &definitions, &["value"], &second, 1)
            .unwrap();
        let first_cached = engine
            .evaluate_cached_scoped("AAA@1d", &definitions, &["value"], &first, 1)
            .unwrap();

        assert_eq!(first_result["value"], vec![1.0, 2.0]);
        assert_eq!(second_result["value"], vec![10.0, 20.0]);
        assert_eq!(first_cached["value"], vec![1.0, 2.0]);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn borrowed_evaluation_passes_raw_series_without_copying() {
        let context = FactorContext::new()
            .with_series("close", vec![1.0, 2.0, 3.0])
            .expect("context");
        let borrowed = context.as_borrowed();
        let original_pointer = borrowed.get("close").expect("close series").as_ptr() as usize;
        let same_storage = Arc::new(AtomicBool::new(false));
        let observed = same_storage.clone();
        let mut engine = CompositeEngine::new();
        engine
            .register(
                "identity",
                Arc::new(move |inputs, _| {
                    observed.store(
                        inputs[0].as_ptr() as usize == original_pointer,
                        Ordering::SeqCst,
                    );
                    Ok(inputs[0].to_vec())
                }),
            )
            .expect("register identity");
        let definitions = vec![CompositeDefinition::new(
            "value",
            CompositeExpr::call("identity", vec![CompositeExpr::series("close")], vec![]),
        )];
        engine
            .evaluate_borrowed(&definitions, &["value"], &borrowed)
            .expect("borrowed graph evaluates");
        assert!(same_storage.load(Ordering::SeqCst));
    }

    #[test]
    fn weighted_average_uses_declared_parameters() {
        let context = FactorContext::new()
            .with_series("a", vec![10.0, 20.0])
            .expect("first series")
            .with_series("b", vec![20.0, 40.0])
            .expect("second series");
        let definitions = vec![CompositeDefinition::new(
            "value",
            CompositeExpr::call(
                "weighted_average",
                vec![CompositeExpr::series("a"), CompositeExpr::series("b")],
                vec![1.0, 3.0],
            ),
        )];
        let result = CompositeEngine::new()
            .evaluate(&definitions, &["value"], &context)
            .expect("weighted graph evaluates");
        assert_eq!(result["value"], vec![17.5, 35.0]);
    }

    #[test]
    fn threshold_and_rolling_builtins_support_signal_composition() {
        let context = FactorContext::new()
            .with_series("close", vec![1.0, 2.0, 3.0, 2.0, 4.0])
            .expect("context");
        let definitions = vec![
            CompositeDefinition::new(
                "above",
                CompositeExpr::call("threshold", vec![CompositeExpr::series("close")], vec![3.0]),
            ),
            CompositeDefinition::new(
                "range",
                CompositeExpr::call(
                    "between",
                    vec![CompositeExpr::series("close")],
                    vec![2.0, 3.0],
                ),
            ),
            CompositeDefinition::new(
                "signal",
                CompositeExpr::call(
                    "cross_up",
                    vec![CompositeExpr::series("close"), CompositeExpr::Constant(2.5)],
                    vec![],
                ),
            ),
            CompositeDefinition::new(
                "vol",
                CompositeExpr::call(
                    "rolling_std",
                    vec![CompositeExpr::series("close")],
                    vec![3.0],
                ),
            ),
        ];
        let result = CompositeEngine::new()
            .evaluate(&definitions, &["above", "range", "signal", "vol"], &context)
            .expect("threshold graph evaluates");
        assert_eq!(result["above"], vec![0.0, 0.0, 1.0, 0.0, 1.0]);
        assert_eq!(result["range"], vec![0.0, 1.0, 1.0, 1.0, 0.0]);
        assert_eq!(result["signal"][2], 1.0);
        assert!(result["vol"][2].is_finite());
    }

    #[test]
    fn finite_lookback_composite_supports_dirty_range_execution() {
        let definitions = vec![
            CompositeDefinition::new(
                "sma3",
                CompositeExpr::call("sma", vec![CompositeExpr::series("close")], vec![3.0]),
            ),
            CompositeDefinition::new(
                "signal",
                CompositeExpr::call(
                    "threshold",
                    vec![CompositeExpr::reference("sma3")],
                    vec![5.0],
                ),
            ),
        ];
        let engine = CompositeEngine::new();
        let plan = engine.compile(&definitions, &["signal"]).unwrap();
        assert_eq!(plan.required_raw_inputs(), &["close".to_string()]);
        assert_eq!(plan.range_lookback(), Some(2));

        let original = (1..=10).map(f64::from).collect::<Vec<_>>();
        let original_context = FactorContext::new().with_series("close", original).unwrap();
        let original_borrowed = original_context.as_borrowed();
        let previous = engine.evaluate_compiled(&plan, &original_borrowed).unwrap();

        let revised = [1.0, 2.0, 3.0, 4.0, 5.0, 20.0, 7.0, 8.0, 9.0, 10.0];
        let revised_context = FactorContext::new()
            .with_series("close", revised.to_vec())
            .unwrap();
        let revised_borrowed = revised_context.as_borrowed();
        let expected = engine.evaluate_compiled(&plan, &revised_borrowed).unwrap();
        let ranged = engine
            .execute_range_borrowed(&plan, &revised_borrowed, &previous, DirtyRange::new(5, 6))
            .unwrap();

        let actual = &ranged.output["signal"];
        let expected = &expected["signal"];
        assert!(actual.iter().zip(expected).all(|(left, right)| {
            (left.is_nan() && right.is_nan()) || (left - right).abs() < 1e-12
        }));
        assert_eq!(
            ranged.trace.mode,
            RuntimeExecutionMode::Range {
                input_dirty: DirtyRange::new(5, 6),
                affected: DirtyRange::new(5, 8),
                recompute: DirtyRange::new(3, 8),
            }
        );
        assert_eq!(ranged.trace.recomputed_rows, 5);
    }

    #[test]
    fn recursive_composite_functions_require_full_execution() {
        let definitions = vec![CompositeDefinition::new(
            "ema",
            CompositeExpr::call("ema", vec![CompositeExpr::series("close")], vec![3.0]),
        )];
        let plan = CompositeEngine::new()
            .compile(&definitions, &["ema"])
            .unwrap();
        assert!(!plan.supports_range_incremental());
        assert_eq!(plan.range_lookback(), None);
    }

    #[test]
    fn finite_composite_stream_matches_batch_and_restores_checkpoint() {
        let definitions = vec![CompositeDefinition::new(
            "sma3",
            CompositeExpr::call("sma", vec![CompositeExpr::series("close")], vec![3.0]),
        )];
        let engine = CompositeEngine::new();
        let plan = engine.compile(&definitions, &["sma3"]).unwrap();
        let mut stream = plan.stream(engine.clone()).unwrap();
        let close = [10.0, 11.0, 12.0, 15.0, 14.0];
        let mut values = Vec::new();
        for value in close {
            let mut row = BTreeMap::new();
            row.insert("close".to_string(), value);
            values.push(stream.push_row(&row).unwrap()["sma3"]);
        }

        let context = FactorContext::new()
            .with_series("close", close.to_vec())
            .unwrap();
        let batch = engine
            .evaluate_compiled(&plan, &context.as_borrowed())
            .unwrap();
        assert!(values.iter().zip(&batch["sma3"]).all(|(left, right)| {
            (left.is_nan() && right.is_nan()) || (left - right).abs() < 1e-12
        }));

        let mut batch_stream = plan.stream(engine.clone()).unwrap();
        let batch_inputs = BTreeMap::from([(String::from("close"), close.to_vec())]);
        let batch_values = batch_stream.push_batch(&batch_inputs).unwrap();
        assert!(batch_values["sma3"]
            .iter()
            .zip(&batch["sma3"])
            .all(|(left, right)| {
                (left.is_nan() && right.is_nan()) || (left - right).abs() < 1e-12
            }));

        let mut reusable_stream = plan.stream(engine.clone()).unwrap();
        let mut emitted = BTreeMap::new();
        reusable_stream
            .push_batch_into(&batch_inputs, &mut emitted)
            .unwrap();
        let first_capacity = emitted["sma3"].capacity();
        let first_pointer = emitted["sma3"].as_ptr();
        let next_inputs = BTreeMap::from([(String::from("close"), vec![16.0, 17.0])]);
        reusable_stream
            .push_batch_into(&next_inputs, &mut emitted)
            .unwrap();
        assert_eq!(emitted["sma3"].len(), 2);
        assert_eq!(emitted["sma3"].capacity(), first_capacity);
        assert_eq!(emitted["sma3"].as_ptr(), first_pointer);

        let checkpoint = stream.checkpoint();
        let mut next = BTreeMap::new();
        next.insert("close".to_string(), 16.0);
        let expected = stream.push_row(&next).unwrap();
        stream.restore(&checkpoint).unwrap();
        assert_eq!(stream.push_row(&next).unwrap(), expected);
        assert_eq!(stream.rows(), 6);
    }

    #[test]
    fn recursive_composite_plan_rejects_bounded_streaming() {
        let definitions = vec![CompositeDefinition::new(
            "ema",
            CompositeExpr::call("ema", vec![CompositeExpr::series("close")], vec![3.0]),
        )];
        let plan = CompositeEngine::new()
            .compile(&definitions, &["ema"])
            .unwrap();
        assert!(matches!(
            plan.stream(CompositeEngine::new()),
            Err(FactorError::InvalidParameter(_))
        ));
    }

    #[test]
    fn rejects_duplicate_or_empty_definition_names() {
        let context = FactorContext::new()
            .with_series("close", vec![1.0, 2.0])
            .expect("context");
        let duplicate = vec![
            CompositeDefinition::new("value", CompositeExpr::series("close")),
            CompositeDefinition::new("value", CompositeExpr::series("close")),
        ];
        assert!(matches!(
            CompositeEngine::new().evaluate(&duplicate, &["value"], &context),
            Err(FactorError::InvalidParameter(message)) if message.contains("duplicate")
        ));
        let empty = vec![CompositeDefinition::new(
            " ",
            CompositeExpr::series("close"),
        )];
        assert!(matches!(
            CompositeEngine::new().evaluate(&empty, &[" "][..], &context),
            Err(FactorError::InvalidParameter(message)) if message.contains("must not be empty")
        ));
    }
}
