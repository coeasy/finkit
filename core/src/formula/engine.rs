use crate::formula::analysis::{analyze_formula, FormulaAnalysis, FormulaSeriesMetadata};
use crate::formula::ast::AstNode;
use crate::formula::bytecode::{compile_to_bytecode, Bytecode, BytecodeVM};
use crate::formula::compiler::{CompiledFormula, FormulaCache};
use crate::formula::compute_ir::FormulaComputePlan;
use crate::formula::custom::FormulaRegistry;
use crate::formula::debugger::FormulaDebugger;
use crate::formula::executor::FormulaExecutor;
use crate::formula::hot_plan::FormulaHotPlan;
use crate::formula::jit::JitCompiler;
#[cfg(feature = "formula-jit")]
use crate::formula::jit::OptimizedBytecode;
use crate::formula::optimizer::{DependencyAnalyzer, FormulaOptimizer};
use crate::formula::params::{apply_params, parse_params, validate_params, ParamDef, ParamValues};
use crate::formula::parser::parse_formula;
use crate::formula::pine::{map_pine_to_alphata_with_security, parse_pine, PineSecurityResolver};
use crate::formula::templates::{FormulaTemplate, FormulaTemplates};
use crate::formula::types::*;
use crate::formula::unified_dispatch::unified_formula_executor_with_host_and_context;
use crate::formula::{normalize_formula_source, parse_formula_with_dialect, FormulaDialect};
use crate::runtime_context::RuntimeContext;
use crate::streaming::indicators::{StreamingRsi, StreamingSma};
use crate::streaming::StreamingIndicator;
use ndarray::Array1;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;

/// Identity of one variant of a source-keyed plan entry. The source is the
/// outer cache key and is borrowed on lookups, avoiding a per-hit source copy.
/// Dialect and parameters remain load-bearing because identical text can parse
/// differently by dialect and parameter values are bound into the compiled plan.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FormulaPlanVariantKey {
    dialect: FormulaDialect,
    params: String,
}

impl FormulaPlanVariantKey {
    fn new(dialect: FormulaDialect, params: &ParamValues) -> Self {
        Self {
            dialect,
            params: if params.is_empty() {
                String::new()
            } else {
                parameter_fingerprint(params)
            },
        }
    }
}

#[derive(Debug)]
struct FormulaPlanSourceEntry {
    /// No-parameter plans are overwhelmingly common and need only one array
    /// lookup after the borrowed source lookup.
    defaults: [Option<Arc<FormulaHotPlan>>; 5],
    parameterized: HashMap<FormulaPlanVariantKey, Arc<FormulaHotPlan>>,
}

impl FormulaPlanSourceEntry {
    fn new() -> Self {
        Self {
            defaults: std::array::from_fn(|_| None),
            parameterized: HashMap::new(),
        }
    }

    fn get(&self, variant: &FormulaPlanVariantKey) -> Option<&Arc<FormulaHotPlan>> {
        if variant.params.is_empty() {
            self.defaults
                .get(dialect_cache_index(variant.dialect))?
                .as_ref()
        } else {
            self.parameterized.get(variant)
        }
    }

    fn insert(&mut self, variant: FormulaPlanVariantKey, plan: Arc<FormulaHotPlan>) {
        if variant.params.is_empty() {
            self.defaults[dialect_cache_index(variant.dialect)] = Some(plan);
        } else {
            self.parameterized.insert(variant, plan);
        }
    }

    fn remove(&mut self, variant: &FormulaPlanVariantKey) {
        if variant.params.is_empty() {
            self.defaults[dialect_cache_index(variant.dialect)] = None;
        } else {
            self.parameterized.remove(variant);
        }
    }

    fn is_empty(&self) -> bool {
        self.defaults.iter().all(Option::is_none) && self.parameterized.is_empty()
    }
}

const fn dialect_cache_index(dialect: FormulaDialect) -> usize {
    match dialect {
        FormulaDialect::AlphaTA => 0,
        FormulaDialect::TongDaXin => 1,
        FormulaDialect::TongHuaShun => 2,
        FormulaDialect::EastMoney => 3,
        FormulaDialect::Pine => 4,
    }
}

/// Order-independent fingerprint of a parameter set.
///
/// [`ParamValues`] is a `HashMap`, whose iteration order is unspecified, so the
/// fingerprint sorts by name: two callers that pass the same parameters in a
/// different order must share one cached plan.
///
/// Values are rendered with `{:?}` rather than `{}` because a cache key must not
/// collide. Rust's float `Debug` prints the shortest representation that
/// round-trips, so it keeps `0.0` and `-0.0` distinct — they really do behave
/// differently — while folding every NaN into one key, which is right because
/// they do not.
fn parameter_fingerprint(params: &ParamValues) -> String {
    let mut entries: Vec<(&str, f64)> = params
        .iter()
        .map(|(name, &value)| (name.as_str(), value))
        .collect();
    // Names are unique by construction (`HashMap` keys), so this is a total
    // order and the result is deterministic.
    entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
    entries
        .into_iter()
        .map(|(name, value)| format!("{name}={value:?}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// Per-engine capacity for every compile-artifact cache (semantic plans, hot
/// plans, bytecode). All three are pure accelerators — a miss only means
/// recompiling — so the bound is a memory guarantee, not a semantic one.
/// Real workloads reuse a handful of distinct formulas (a screening pass
/// re-evaluates *one* formula across thousands of symbols), so 1024 sits far
/// above any reuse window while capping a long-lived multi-tenant engine's
/// growth. V5 R2-2 previously flagged the unbounded caches as a memory risk.
const ENGINE_CACHE_CAPACITY: usize = 1024;

/// Borrow already-normalized source on the hot path; only transport variants
/// with a BOM or carriage returns need a rewritten allocation.
#[inline]
fn normalized_source(source: &str, dialect: FormulaDialect) -> Cow<'_, str> {
    if source.starts_with('\u{feff}') || source.contains('\r') {
        Cow::Owned(normalize_formula_source(source, dialect))
    } else {
        Cow::Borrowed(source)
    }
}

/// A string-keyed `HashMap` that evicts in insertion order at a fixed
/// capacity.
///
/// The engine's source-keyed caches (`semantic_plan_cache`,
/// `bytecode_cache`) used to be plain `HashMap`s — unbounded growth for a
/// long-lived engine fed many distinct formulas. FIFO (not LRU) keeps the
/// structure trivial and the eviction deterministic; the caches are pure
/// accelerators, so evicting a still-hot entry only costs a recompile.
struct BoundedFifoMap<V> {
    map: HashMap<String, V>,
    order: VecDeque<String>,
    capacity: usize,
}

impl<V> BoundedFifoMap<V> {
    fn new(capacity: usize) -> Self {
        debug_assert!(capacity > 0, "a zero-capacity cache can never hold");
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            capacity: capacity.max(1),
        }
    }

    fn get(&self, key: &str) -> Option<&V> {
        self.map.get(key)
    }

    fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        self.map.get_mut(key)
    }

    fn insert(&mut self, key: String, value: V) {
        if !self.map.contains_key(&key) {
            while self.order.len() >= self.capacity {
                if let Some(oldest) = self.order.pop_front() {
                    self.map.remove(&oldest);
                }
            }
            self.order.push_back(key.clone());
        }
        self.map.insert(key, value);
    }

    fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }
}

/// Statistics for the compiled-plan cache.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FormulaPlanCacheStats {
    /// Lookups that returned an already-compiled plan.
    pub hits: u64,
    /// Lookups that had to compile.
    pub misses: u64,
}

/// Compiled-plan cache.
///
/// Bounded with FIFO eviction at [`ENGINE_CACHE_CAPACITY`] distinct plans —
/// matching the engine's other accelerators (`semantic_plan_cache`,
/// `bytecode_cache`, all sharing the same capacity and policy). The capacity
/// sits orders of magnitude above the handful of distinct formulas any real
/// workload (including a screening pass over thousands of symbols) reuses, so
/// a bound never evicts a plan that is about to be reused, while a
/// long-lived multi-tenant engine can no longer grow without limit — the
/// previous unbounded design was a documented memory risk (V5 R2-2). Eviction
/// is always safe: every cache here is a pure accelerator, a miss only means
/// recompiling. [`FormulaEngine::clear_plan_cache`] drops everything if a
/// caller wants that explicitly.
///
/// Both counters live here, next to the lookup that decides them, so a call site
/// cannot bump the wrong one.
///
/// Entries are handed out as `Arc` handles rather than clones. A screening pass
/// evaluates the same formula against thousands of symbols, and the previous
/// `plan.clone()` on every hit deep-copied the whole compiled plan. Source text
/// is now the outer key so hot lookups can borrow `&str`; empty-parameter hits
/// also avoid creating a fingerprint.
#[derive(Debug)]
struct FormulaPlanCache {
    entries: HashMap<String, FormulaPlanSourceEntry>,
    /// Insertion order for FIFO eviction, mirroring each source/variant pair.
    order: VecDeque<(String, FormulaPlanVariantKey)>,
    capacity: usize,
    len: usize,
    hits: u64,
    misses: u64,
}

impl FormulaPlanCache {
    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            capacity: ENGINE_CACHE_CAPACITY,
            len: 0,
            hits: 0,
            misses: 0,
        }
    }

    fn get(
        &mut self,
        source: &str,
        variant: &FormulaPlanVariantKey,
    ) -> Option<Arc<FormulaHotPlan>> {
        if let Some(plan) = self
            .entries
            .get(source)
            .and_then(|entry| entry.get(variant))
        {
            self.hits = self.hits.saturating_add(1);
            Some(Arc::clone(plan))
        } else {
            self.misses = self.misses.saturating_add(1);
            None
        }
    }

    fn insert(&mut self, source: &str, variant: FormulaPlanVariantKey, plan: Arc<FormulaHotPlan>) {
        let exists = self
            .entries
            .get(source)
            .and_then(|entry| entry.get(&variant))
            .is_some();
        if !exists {
            while self.len >= self.capacity {
                if let Some((old_source, old_variant)) = self.order.pop_front() {
                    let remove_source = if let Some(entry) = self.entries.get_mut(&old_source) {
                        entry.remove(&old_variant);
                        entry.is_empty()
                    } else {
                        false
                    };
                    if remove_source {
                        self.entries.remove(&old_source);
                    }
                    self.len = self.len.saturating_sub(1);
                }
            }
            self.order.push_back((source.to_owned(), variant.clone()));
            self.len += 1;
        }
        self.entries
            .entry(source.to_owned())
            .or_insert_with(FormulaPlanSourceEntry::new)
            .insert(variant, plan);
    }

    fn stats(&self) -> FormulaPlanCacheStats {
        FormulaPlanCacheStats {
            hits: self.hits,
            misses: self.misses,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn contains_key(&self, source: &str, variant: &FormulaPlanVariantKey) -> bool {
        self.entries
            .get(source)
            .and_then(|entry| entry.get(variant))
            .is_some()
    }

    /// Drop every plan but keep the counters.
    ///
    /// Registering a custom component can change what an existing source
    /// resolves to, so the plans must go; the counters still describe this
    /// engine's behaviour, so they stay.
    fn invalidate(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.len = 0;
    }

    /// Drop every plan and reset the counters.
    fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.len = 0;
        self.hits = 0;
        self.misses = 0;
    }
}

#[derive(Clone)]
enum AutoPlanChoice {
    Tree,
    Plan(Arc<FormulaHotPlan>),
}

struct AutoPlanSelections {
    dialects: [Option<AutoPlanChoice>; 5],
}

impl AutoPlanSelections {
    fn new() -> Self {
        Self {
            dialects: std::array::from_fn(|_| None),
        }
    }
}

/// One compiled-plan evaluation: the primary series plus every named channel.
///
/// The primary is always the value of the formula's last value-producing
/// statement (see [`FormulaHotPlan::outputs`]); `channels` are the `OUTPUT:`
/// declarations, so a MACD script reports `DIF`/`DEA`/`MACD` rather than only
/// its last line.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FormulaPlanOutput {
    primary: Vec<f64>,
    channels: Vec<(String, Vec<f64>)>,
    variables: Vec<(String, Vec<f64>)>,
}

impl FormulaPlanOutput {
    /// The formula's primary result series.
    #[must_use]
    pub fn primary(&self) -> &[f64] {
        &self.primary
    }

    /// Named output channels in declaration order.
    ///
    /// These are the `OUTPUT:`/`plot` declarations only. Assigned variables are
    /// in [`Self::variables`], mirroring the tree path's split between
    /// `ctx.output_names` and `ctx.variables`.
    #[must_use]
    pub fn channels(&self) -> &[(String, Vec<f64>)] {
        &self.channels
    }

    /// One named channel's series, if the formula declares it.
    #[must_use]
    pub fn channel(&self, name: &str) -> Option<&[f64]> {
        self.channels
            .iter()
            .find(|(channel, _)| channel == name)
            .map(|(_, values)| values.as_slice())
    }

    /// Every variable the formula assigns, in first-write order.
    ///
    /// The same set the tree path publishes into `ctx.variables`, including
    /// `OUTPUT:` names — an output is a variable write as well as a channel.
    #[must_use]
    pub fn variables(&self) -> &[(String, Vec<f64>)] {
        &self.variables
    }

    /// One assigned variable's series, if the formula writes it.
    #[must_use]
    pub fn variable(&self, name: &str) -> Option<&[f64]> {
        self.variables
            .iter()
            .find(|(variable, _)| variable == name)
            .map(|(_, values)| values.as_slice())
    }

    /// Consume this output and return its primary series.
    #[must_use]
    pub fn into_primary(self) -> Vec<f64> {
        self.primary
    }
}

#[derive(Debug, Clone)]
struct StreamingEmaState {
    period: usize,
    len: usize,
    seed_sum: f64,
    value: f64,
    valid: bool,
    previous_input: f64,
}

enum StreamingFormulaIndicator {
    Sma(StreamingSma),
    Rsi(StreamingRsi),
    Atr(StreamingSmaAtr),
}

/// Formula ATR follows the canonical batch Wilder seed: the first bar is used
/// only to establish the previous close, then TR[1..=period] seeds the RMA.
/// Keeping this state local avoids rebuilding the complete formula range while
/// preserving the public ATR warm-up and recurrence exactly.
struct StreamingSmaAtr {
    period: usize,
    atr_value: f64,
    tr_sum: f64,
    previous_close: f64,
    count: usize,
}

impl StreamingSmaAtr {
    fn new(period: usize) -> Self {
        Self {
            period,
            atr_value: f64::NAN,
            tr_sum: 0.0,
            previous_close: f64::NAN,
            count: 0,
        }
    }

    fn next(&mut self, high: f64, low: f64, close: f64) -> Option<f64> {
        self.count += 1;
        if self.count == 1 {
            self.previous_close = close;
            return None;
        }

        let true_range = (high - low)
            .max((high - self.previous_close).abs())
            .max((low - self.previous_close).abs());
        self.previous_close = close;

        if self.count <= self.period {
            self.tr_sum += true_range;
            None
        } else if self.count == self.period + 1 {
            self.tr_sum += true_range;
            self.atr_value = self.tr_sum / self.period as f64;
            Some(self.atr_value)
        } else {
            self.atr_value += (true_range - self.atr_value) / self.period as f64;
            Some(self.atr_value)
        }
    }
}

impl StreamingFormulaIndicator {
    fn next(&mut self, input: [f64; 3]) -> Option<f64> {
        match self {
            Self::Sma(indicator) => indicator.next(input[0]),
            Self::Rsi(indicator) => indicator.next(input[0]),
            Self::Atr(indicator) => indicator.next(input[0], input[1], input[2]),
        }
    }
}

struct StreamingFormulaState {
    len: usize,
    last_input: [f64; 3],
    indicator: StreamingFormulaIndicator,
}

/// 公式引擎主入口
/// Which execution path the `eval*` entry points take.
///
/// The two paths are kept numerically equal by the differential gates
/// (`formula_plan_differential.rs`, `formula_differential_tests.rs`), but they
/// are not interchangeable in every respect, and the differences are why this
/// is an explicit choice rather than an internal detail:
///
/// | | [`Self::Tree`] | [`Self::Plan`] |
/// |---|---|---|
/// | Coverage | everything | only what lowers to a compute plan |
/// | Failure | non-evaluable nodes error at runtime | **unsupported formulas fail at compile time** |
/// | `ctx.variables`, `ctx.output_names`, `ctx.output_modifiers` | written back | written back |
/// | Warm-up / streaming state | full | single-shot batch |
///
/// `Auto` selects the compiled plan for sufficiently long series when the
/// formula can be planned, and otherwise uses the tree-walking interpreter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormulaExecutionMode {
    /// Tree-walking reference interpreter. Explicit selection always uses it.
    Tree,
    /// Compiled compute plan driven by `UnifiedExecutor`.
    Plan,
    /// Select a backend using the input length and plan coverage.
    #[default]
    Auto,
}

impl FormulaExecutionMode {
    /// Return the stable lowercase spelling used by language bindings.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tree => "tree",
            Self::Plan => "plan",
            Self::Auto => "auto",
        }
    }

    /// Parse a backend name accepted by language-neutral APIs.
    pub fn from_name(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("tree") {
            Some(Self::Tree)
        } else if value.eq_ignore_ascii_case("plan") {
            Some(Self::Plan)
        } else if value.eq_ignore_ascii_case("auto") {
            Some(Self::Auto)
        } else {
            None
        }
    }
}

/// 公式引擎主入口
///
/// # Which entry points [`FormulaExecutionMode`] governs
///
/// The switch is a **backend contract**, so the split below is exhaustive and
/// machine-checked by `core/tests/formula_execution_mode.rs`:
///
/// | Family | Entries | Under [`FormulaExecutionMode::Plan`] |
/// |---|---|---|
/// | Governed, plan-capable | `eval`, `eval_with_dialect`, `eval_with_params`, `eval_multi`, `eval_multi_with_dialect` (non-Pine), `eval_incremental`, `eval_simd`, `eval_batch`, `eval_batch_shared` | run on the plan backend |
/// | Governed, tree-only | `eval_ast`, `eval_lazy`, `eval_parallel`, `eval_optimized`, `eval_with_debug`, `eval_template`, `eval_with_validation`, `eval_with_defaults`, `eval_zero_copy`, `eval_zero_copy_cached`, `eval_zero_alloc`, `eval_multi_with_dialect(Pine)`, `eval_multi_with_pine_security` | [`FormulaError::BackendUnsupported`] |
/// Auto applies to `eval`, `eval_with_dialect`, and `eval_with_params`, plus
/// wrappers that delegate to them: cold formulas use Tree, and repeated calls
/// select Plan at 5000 domestic rows or 2400 Pine rows when compilation
/// succeeds. Other entry points with separate
/// implementations use Tree under Auto. Explicit Tree and Plan retain the
/// contracts in the table.
///
/// | Backend-explicit (never governed) | `eval_plan*`, `compile_plan*`, `eval_jit`, `compile_jit`, `execute_jit`, `compile_bytecode`, `execute_bytecode`, and every entry taking a `&CompiledFormula` (`execute`, `eval_into`, `eval_range`, `eval_range_zero_copy_inputs`, `eval_last`, `eval_zero_copy_inputs`, `execute_zero_copy_cached`) | runs the backend its argument type declares |
///
/// Two rules follow from that table:
///
/// 1. A *source-level* entry point never silently runs a backend the caller did
///    not select. A tree-only one fails with
///    [`FormulaError::BackendUnsupported`] naming itself and the selected
///    backend.
/// 2. An entry point whose argument is already a tree artifact
///    (`CompiledFormula`, `AstNode`, `Bytecode`) is backend-explicit: building
///    that artifact *is* the backend choice. Its plan counterpart is
///    `compile_plan`/`eval_plan`.
///
/// # See also
///
/// [`FormulaEngine::execution_mode`] to query the selection,
/// [`FormulaEngine::set_execution_mode`] to change it, and
/// `docs/formula-runtime-contract.md` for the cross-language view.
pub struct FormulaEngine {
    /// Execution path used by the `eval*` entry points.
    execution_mode: FormulaExecutionMode,
    executor: FormulaExecutor,
    cache: FormulaCache,
    /// Parsed Pine ASTs keyed by normalized source so Tree mode does not
    /// tokenize and lower the same script on every symbol evaluation.
    pine_ast_cache: RefCell<BoundedFifoMap<Arc<AstNode>>>,
    /// Measured Auto backend decision per source and dialect. Holds the compiled
    /// plan directly so warmed calls avoid route rescans and duplicate lookup.
    auto_plan_cache: RefCell<BoundedFifoMap<AutoPlanSelections>>,
    /// Semantic Compute IR plans keyed by the exact formula source.
    semantic_plan_cache: RefCell<BoundedFifoMap<FormulaComputePlan>>,
    /// Compiled hot plans keyed by source + dialect + parameter fingerprint.
    ///
    /// Distinct from `semantic_plan_cache`, which is keyed by source alone and
    /// holds the *semantic* DAG: this one holds the *numeric* plan the unified
    /// executor runs, and it is dialect- and parameter-sensitive.
    plan_cache: RefCell<FormulaPlanCache>,
    /// Reusable numeric scratch arenas for repeated Plan-mode evaluations.
    /// The executor itself remains per-call because its dispatcher borrows the
    /// current formula context; only the context-owned arenas survive a call.
    plan_runtime_context: RefCell<RuntimeContext>,
    templates: FormulaTemplates,
    jit_compiler: RefCell<JitCompiler>,
    /// Persistent bytecode cache and VM scratch buffers.
    bytecode_cache: RefCell<BoundedFifoMap<Bytecode>>,
    bytecode_vm: RefCell<BytecodeVM>,
    /// Stateful fast paths for append/eval_last.  A failed continuity check
    /// simply falls back to the exact range evaluator.
    streaming_ema: RefCell<HashMap<String, StreamingEmaState>>,
    /// O(1) append paths for common direct formula indicators whose existing
    /// streaming implementations have exactly the same warm-up contract.
    streaming_common: RefCell<HashMap<String, StreamingFormulaState>>,
    /// User-defined expression components expanded before semantic planning.
    custom_formulas: FormulaRegistry,
}

impl Default for FormulaEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Fast path for the multi-series `ATR(HIGH, LOW, CLOSE, N)` shape.
///
/// The single-input fast path cannot express ATR: its period is the *last*
/// argument, not the second, and it reads three series. ATR is worth the
/// special case because it measured as the widest engine-vs-native gap.
///
/// Returns `false` on any shape or data condition this path does not own, so
/// the caller falls through to the general executor unchanged.
fn try_atr_fast_path(ctx: &FormulaContext, args: &[AstNode], output: &mut [f64]) -> bool {
    let (high_name, low_name, close_name) = match (&args[0], &args[1], &args[2]) {
        (AstNode::Variable(h), AstNode::Variable(l), AstNode::Variable(c)) => (h, l, c),
        _ => return false,
    };
    let (Some(high), Some(low), Some(close)) = (
        ctx.get_data(high_name),
        ctx.get_data(low_name),
        ctx.get_data(close_name),
    ) else {
        return false;
    };
    let AstNode::Number(period_value) = &args[3] else {
        return false;
    };
    if !period_value.is_finite() || *period_value < 1.0 {
        return false;
    }
    let period = *period_value as usize;
    if period == 0 || high.len() != output.len() {
        return false;
    }
    // Same contract as the single-input path: a non-finite input belongs to
    // the general executor, which owns the all-NaN conversion.
    if high.iter().any(|v| !v.is_finite())
        || low.len() != output.len()
        || close.len() != output.len()
        || low.iter().any(|v| !v.is_finite())
        || close.iter().any(|v| !v.is_finite())
    {
        return false;
    }
    crate::indicators::volatility::atr_into(high, low, close, period, output).is_ok()
}

/// The plan-side ATR shortcut scans all three inputs to preserve the
/// non-finite fallback contract. Keep that preflight on short histories, where
/// avoiding executor setup wins; long histories use the fused dispatcher path.
const PLAN_SIMPLE_ATR_FASTPATH_MAX_ROWS: usize = 1024;

impl FormulaEngine {
    pub fn new() -> Self {
        Self {
            execution_mode: FormulaExecutionMode::default(),
            executor: FormulaExecutor::new(),
            cache: FormulaCache::new(100),
            pine_ast_cache: RefCell::new(BoundedFifoMap::new(ENGINE_CACHE_CAPACITY)),
            auto_plan_cache: RefCell::new(BoundedFifoMap::new(ENGINE_CACHE_CAPACITY)),
            semantic_plan_cache: RefCell::new(BoundedFifoMap::new(ENGINE_CACHE_CAPACITY)),
            plan_cache: RefCell::new(FormulaPlanCache::new()),
            plan_runtime_context: RefCell::new(RuntimeContext::new()),
            templates: FormulaTemplates::new(),
            jit_compiler: RefCell::new(JitCompiler::new()),
            bytecode_cache: RefCell::new(BoundedFifoMap::new(ENGINE_CACHE_CAPACITY)),
            bytecode_vm: RefCell::new(BytecodeVM::new()),
            streaming_ema: RefCell::new(HashMap::new()),
            streaming_common: RefCell::new(HashMap::new()),
            custom_formulas: FormulaRegistry::new(),
        }
    }

    /// Build an engine that runs formulas on the given execution path.
    #[must_use]
    pub fn with_execution_mode(mut self, mode: FormulaExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }

    /// Switch the execution path used by the `eval*` entry points.
    pub fn set_execution_mode(&mut self, mode: FormulaExecutionMode) {
        self.execution_mode = mode;
    }

    /// Execution path the `eval*` entry points currently use.
    pub const fn execution_mode(&self) -> FormulaExecutionMode {
        self.execution_mode
    }

    /// Whether the caller selected the compiled-plan backend.
    fn plan_selected(&self) -> bool {
        self.execution_mode == FormulaExecutionMode::Plan
    }

    /// The workload has separate long-history sizes: 5K rows for domestic
    /// formulas and 2.4K rows for Pine formulas.
    const AUTO_PLAN_MIN_ROWS: usize = 5000;
    const AUTO_PINE_PLAN_MIN_ROWS: usize = 2400;

    fn remember_auto_choice(&self, source: &str, dialect: FormulaDialect, choice: AutoPlanChoice) {
        let mut cache = self.auto_plan_cache.borrow_mut();
        if cache.get(source).is_none() {
            cache.insert(source.to_owned(), AutoPlanSelections::new());
        }
        if let Some(entry) = cache.get_mut(source) {
            entry.dialects[dialect_cache_index(dialect)] = Some(choice);
        }
    }

    fn auto_tree_source_hint(source: &str, dialect: FormulaDialect) -> bool {
        let source = source.to_ascii_lowercase();
        if dialect == FormulaDialect::Pine {
            // These measured long-history corpus cases have a faster cached
            // Tree path than Plan. Decide before compiling a Plan so Auto does
            // not pay the compilation cost for a route it will reject anyway.
            let donchian_from_primitives =
                source.contains("ta.highest") && source.contains("ta.lowest");
            donchian_from_primitives
                || [
                    "ta.obv",
                    "ta.rsi",
                    "ta.sma",
                    "ta.mom",
                    "ta.roc",
                    "ta.atr",
                    "ta.ema",
                    "ta.vwap",
                    "ta.trix",
                    "ta.sar",
                    "ta.wpr",
                    "ta.vwma",
                    "ta.donchian",
                ]
                .iter()
                .any(|name| source.contains(name))
        } else {
            [
                "zlema",
                "coppock",
                "ao(",
                "fractal(",
                "aroon",
                "kc_mid(",
                "kc_lower(",
                "refdate(",
                "vortex(",
                "trix:=",
                "nvi",
                "pvi",
            ]
            .iter()
            .any(|name| source.contains(name))
        }
    }

    fn auto_plan_for(
        &self,
        source: &str,
        dialect: FormulaDialect,
        ctx: &FormulaContext,
        params: &ParamValues,
    ) -> Option<Arc<FormulaHotPlan>> {
        let min_rows = if dialect == FormulaDialect::Pine {
            Self::AUTO_PINE_PLAN_MIN_ROWS
        } else {
            Self::AUTO_PLAN_MIN_ROWS
        };
        if ctx.data_len < min_rows {
            return None;
        }
        let dialect_index = dialect_cache_index(dialect);
        if params.is_empty() {
            let cached = self
                .auto_plan_cache
                .borrow()
                .get(source)
                .and_then(|entry| entry.dialects[dialect_index].clone());
            if let Some(choice) = cached {
                return match choice {
                    AutoPlanChoice::Tree => None,
                    AutoPlanChoice::Plan(plan) => Some(plan),
                };
            }
            if Self::auto_tree_source_hint(source, dialect) {
                self.remember_auto_choice(source, dialect, AutoPlanChoice::Tree);
                return None;
            }
        }
        // Plan construction costs more than it saves on a one-shot formula.
        // Repeated calls have a warm Tree cache and can amortize planning.
        let tree_is_warm = match dialect {
            FormulaDialect::Pine => self.pine_ast_cache.borrow().get(source).is_some(),
            _ => self.cache.contains(source),
        };
        if !tree_is_warm {
            return None;
        }
        let plan = self.plan_for(source, dialect, params).ok()?;
        // These workload shapes were measured slower on Plan at the long
        // history size. Keep Auto on Tree until their Plan paths catch up.
        let pine_tree_winners = dialect == FormulaDialect::Pine
            && (plan.simple_obv_input_names().is_some() || plan.contains_call("RSI"));
        let use_plan =
            !pine_tree_winners && !plan.contains_call("ZLEMA") && !plan.contains_call("COPPOCK");
        if params.is_empty() {
            let choice = if use_plan {
                AutoPlanChoice::Plan(Arc::clone(&plan))
            } else {
                AutoPlanChoice::Tree
            };
            self.remember_auto_choice(source, dialect, choice);
        }
        use_plan.then_some(plan)
    }

    #[cfg(test)]
    fn auto_uses_plan(
        &self,
        source: &str,
        dialect: FormulaDialect,
        ctx: &FormulaContext,
        params: &ParamValues,
    ) -> bool {
        self.auto_plan_for(source, dialect, ctx, params).is_some()
    }

    /// Refuse a tree-only entry point while the caller selected `Plan`.
    ///
    /// [`FormulaExecutionMode`] is a backend contract, not a hint. An entry
    /// point that only the tree-walker can serve must report that instead of
    /// quietly running a backend the caller did not choose — see the enum
    /// documentation for the exact split between governed and backend-explicit
    /// entry points.
    fn require_tree_backend(&self, entry: &'static str) -> Result<(), FormulaError> {
        if self.plan_selected() {
            return Err(FormulaError::BackendUnsupported {
                backend: "plan".to_string(),
                entry: entry.to_string(),
            });
        }
        Ok(())
    }

    pub fn with_cache_size(cache_size: usize) -> Self {
        Self {
            execution_mode: FormulaExecutionMode::default(),
            executor: FormulaExecutor::new(),
            cache: FormulaCache::new(cache_size),
            pine_ast_cache: RefCell::new(BoundedFifoMap::new(ENGINE_CACHE_CAPACITY)),
            auto_plan_cache: RefCell::new(BoundedFifoMap::new(ENGINE_CACHE_CAPACITY)),
            semantic_plan_cache: RefCell::new(BoundedFifoMap::new(ENGINE_CACHE_CAPACITY)),
            plan_cache: RefCell::new(FormulaPlanCache::new()),
            plan_runtime_context: RefCell::new(RuntimeContext::new()),
            templates: FormulaTemplates::new(),
            jit_compiler: RefCell::new(JitCompiler::new()),
            bytecode_cache: RefCell::new(BoundedFifoMap::new(ENGINE_CACHE_CAPACITY)),
            bytecode_vm: RefCell::new(BytecodeVM::new()),
            streaming_ema: RefCell::new(HashMap::new()),
            streaming_common: RefCell::new(HashMap::new()),
            custom_formulas: FormulaRegistry::new(),
        }
    }

    /// 编译公式字符串
    pub fn compile(&mut self, source: &str) -> Result<CompiledFormula, FormulaError> {
        // The public entry point hands back an owned formula, so it pays one
        // deep clone to separate the caller's copy from the cached one. The
        // engine's own evaluation paths go through `compile_shared` instead and
        // move a handle, which is what keeps that clone off the hot path.
        self.compile_shared(source)
            .map(|formula| formula.as_ref().clone())
    }

    /// Compile `source` into a shared handle, reusing the cached AST.
    ///
    /// Auto uses the tree-walker for cold or unsupported formulas, so this is
    /// the per-call compile step for those callers: a cache hit costs one
    /// refcount bump instead of a deep copy of the whole `AstNode`.
    fn compile_shared(&mut self, source: &str) -> Result<Arc<CompiledFormula>, FormulaError> {
        if let Some(formula) = self.cache.get_shared(source) {
            return Ok(formula);
        }

        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        let ast = self
            .custom_formulas
            .expand(&ast)
            .map_err(FormulaError::InvalidOperation)?;
        // Semantic analysis is deliberately performed before AST optimization.
        // This locks dependencies/effects against the source program so later
        // optimization and incremental execution cannot accidentally erase an
        // assignment, output, drawing command, or control-flow barrier.
        let semantic_plan = FormulaComputePlan::compile(&ast).map_err(|error| {
            FormulaError::InvalidOperation(format!("formula compute planning failed: {error}"))
        })?;
        // Compile the optimized AST once so repeated evaluations share the
        // same CSE and constant-folding decisions while preserving assignment
        // side effects exposed through FormulaContext::variables.
        let ast = FormulaOptimizer::optimize_for_execution(&ast);
        let formula = Arc::new(CompiledFormula {
            ast,
            source: source.to_string(),
        });

        self.semantic_plan_cache
            .borrow_mut()
            .insert(source.to_string(), semantic_plan);
        // Store the handle we are about to return rather than cloning the
        // formula a second time just to fill the cache.
        self.cache.insert_shared(source, Arc::clone(&formula));

        Ok(formula)
    }

    /// Compile `source` into an executable hot plan, cached by
    /// `source + dialect + parameter fingerprint`.
    ///
    /// This is the compile half of P0-2 step 2: it makes the compiled plan
    /// reachable from the engine so the execution path can be switched onto it.
    /// The AST pipeline deliberately mirrors [`Self::eval_with_dialect`] (source
    /// normalisation, then a dialect parse) followed by [`Self::compile`]'s
    /// custom-component expansion, so the plan describes exactly what the tree
    /// path would have evaluated.
    ///
    /// # Errors
    ///
    /// A formula the plan path cannot compile returns [`FormulaError`]; this
    /// **never** silently falls back to the tree-walker. A caller that wants the
    /// tree path must ask for it explicitly, which is what makes
    /// `formula_execution_mode` a real switch rather than a hint.
    ///
    /// Failure is reported as [`FormulaError::InvalidOperation`] rather than as a
    /// new variant because `ffi/c-binding` matches `FormulaError` exhaustively to
    /// produce its error-code ABI; adding a variant would silently change that
    /// contract for every language binding. The message is prefixed so it stays
    /// distinguishable in logs.
    pub fn compile_plan(
        &self,
        source: &str,
        dialect: FormulaDialect,
        params: &ParamValues,
    ) -> Result<FormulaHotPlan, FormulaError> {
        // This public entry point hands back an owned plan, so it pays one deep
        // clone to separate the caller's copy from the cached one. The internal
        // execution path does not go through here — it uses `plan_for` and moves
        // a shared handle, which is what keeps the clone off the hot path.
        self.plan_for(source, dialect, params)
            .map(|plan| plan.as_ref().clone())
    }

    /// Resolve `source` to a compiled plan, reusing the cached one when possible.
    ///
    /// Returns a shared handle so a cache hit costs one refcount bump instead of
    /// a deep copy (see [`FormulaPlanCache`]).
    fn plan_for(
        &self,
        source: &str,
        dialect: FormulaDialect,
        params: &ParamValues,
    ) -> Result<Arc<FormulaHotPlan>, FormulaError> {
        let variant = FormulaPlanVariantKey::new(dialect, params);
        // Bind the lookup to a local so the `RefCell` borrow provably ends here,
        // rather than relying on `if let` scrutinee temporary lifetimes.
        let cached = self.plan_cache.borrow_mut().get(source, &variant);
        if let Some(plan) = cached {
            return Ok(plan);
        }

        // A miss stores the very handle it returns, so the miss path is also
        // clone-free — it used to compile the plan, clone it into the cache and
        // hand back the original.
        let plan = Arc::new(self.build_plan(source, dialect, params)?);
        self.plan_cache
            .borrow_mut()
            .insert(source, variant, Arc::clone(&plan));
        Ok(plan)
    }

    /// [`Self::compile_plan`] with the default dialect and no parameters.
    ///
    /// # Errors
    ///
    /// As [`Self::compile_plan`].
    pub fn compile_plan_default(&self, source: &str) -> Result<FormulaHotPlan, FormulaError> {
        self.compile_plan(source, FormulaDialect::default(), &ParamValues::new())
    }

    /// Build a plan without consulting or populating the cache.
    fn build_plan(
        &self,
        source: &str,
        dialect: FormulaDialect,
        params: &ParamValues,
    ) -> Result<FormulaHotPlan, FormulaError> {
        let normalized = normalized_source(source, dialect);
        let ast =
            parse_formula_with_dialect(&normalized, dialect).map_err(FormulaError::ParseError)?;
        let ast = self
            .custom_formulas
            .expand(&ast)
            .map_err(FormulaError::InvalidOperation)?;
        // Parameters are substituted *before* planning, not after: the plan
        // binds numeric literals into its parameter arena, so the substituted
        // values are precisely what make two parameterisations different plans.
        let ast = apply_params(&ast, params);
        // The plan path has no evaluation context to fall back on, so the
        // context-implicit price arguments of `resolve_hl*` short forms are
        // expanded during lowering rather than here — see
        // `FormulaComputePlan::compile_with_registry`. Doing it in the lowerer
        // keeps the compiled-plan differential gate (which calls
        // `FormulaHotPlan::compile` directly, without an engine) on the same
        // code path as production.
        FormulaHotPlan::compile(&ast).map_err(|error| {
            FormulaError::InvalidOperation(format!("formula plan compilation failed: {error}"))
        })
    }

    /// Compiled-plan cache statistics.
    pub fn plan_cache_stats(&self) -> FormulaPlanCacheStats {
        self.plan_cache.borrow().stats()
    }

    /// Number of compiled plans currently cached.
    pub fn plan_cache_size(&self) -> usize {
        self.plan_cache.borrow().len()
    }

    /// Drop every cached plan and reset the cache statistics.
    pub fn clear_plan_cache(&mut self) {
        self.plan_cache.borrow_mut().clear();
        self.auto_plan_cache.borrow_mut().clear();
    }

    /// Evaluate `source` through the compiled plan path, returning its primary series.
    ///
    /// This is the execution half of P0-2 step 3: it drives the same
    /// `UnifiedExecutor` the differential gate validates, using the cached plan
    /// from [`Self::compile_plan`].
    ///
    /// The tree path is untouched — this is an *additional* entry point, so
    /// nothing switches over until `formula_execution_mode` lands in step 4.
    ///
    /// # Errors
    ///
    /// Returns [`FormulaError`] if the formula cannot be parsed or planned (see
    /// [`Self::compile_plan`]), if the context does not supply every input series
    /// the plan declares, or if execution fails — most often because the plan
    /// contains an operator with no numeric kernel yet.
    pub fn eval_plan(
        &self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        Ok(Array1::from_vec(
            self.eval_plan_channels_impl(
                source,
                FormulaDialect::default(),
                &ParamValues::new(),
                ctx,
                false,
            )?
            .into_primary(),
        ))
    }

    /// [`Self::eval_plan`] for an explicit dialect.
    ///
    /// # Errors
    ///
    /// As [`Self::eval_plan`].
    pub fn eval_plan_with_dialect(
        &self,
        source: &str,
        dialect: FormulaDialect,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        Ok(Array1::from_vec(
            self.eval_plan_channels_impl(source, dialect, &ParamValues::new(), ctx, false)?
                .into_primary(),
        ))
    }

    /// Evaluate `source` through the compiled plan path and return every channel.
    ///
    /// This takes `&mut FormulaContext` because the plan path reproduces the
    /// tree path's side effects: assignments and outputs are published into
    /// `ctx.variables`, and declared channels into `ctx.output_names`. The
    /// language bindings read both back to build their result dictionaries, so a
    /// shared borrow would make the two modes observably different for the same
    /// source.
    ///
    /// # Errors
    ///
    /// As [`Self::eval_plan`].
    pub fn eval_plan_channels(
        &self,
        source: &str,
        dialect: FormulaDialect,
        params: &ParamValues,
        ctx: &mut FormulaContext,
    ) -> Result<FormulaPlanOutput, FormulaError> {
        self.eval_plan_channels_impl(source, dialect, params, ctx, true)
    }

    /// Execute the general hot plan while preserving the engine's reusable
    /// scratch buffers and dispatcher context.
    fn execute_formula_plan(
        &self,
        plan: &FormulaHotPlan,
        ctx: &FormulaContext,
        inputs: &[&[f64]],
        length: usize,
        string_base: usize,
    ) -> Result<Vec<Vec<f64>>, FormulaError> {
        let mut runtime_context = std::mem::take(&mut *self.plan_runtime_context.borrow_mut());
        // Formula evaluations are independent batches. Keep reusable scratch
        // buffers and the content-addressed cache, but clear streaming state.
        runtime_context.reset();
        let host = HostContext {
            chip: ctx.chip_data.as_ref(),
            period_type: ctx.period_type,
            money_flow: ctx.money_flow_data.as_ref(),
            ohlc: Some(HostOhlc {
                high: ctx.high.as_slice(),
                low: ctx.low.as_slice(),
                close: ctx.close.as_slice(),
            }),
            capital: ctx.capital,
            string_table: Some(ctx.string_table.as_slice()),
            em_data: ctx.em_data.as_ref(),
            block_data: ctx.block_data.as_ref(),
        };
        let mut executor =
            unified_formula_executor_with_host_and_context(plan, host, runtime_context);
        executor.dispatcher_mut().set_string_base(string_base);
        let result = executor.execute_range(inputs, 0..length);
        *self.plan_runtime_context.borrow_mut() = executor.into_context();
        result.map(|output| output.values).map_err(|error| {
            FormulaError::RuntimeError(format!("formula plan execution failed: {error}"))
        })
    }

    /// Execute a single retained result directly into the public primary-series
    /// return path. Avoids constructing the executor's one-element output Vec.
    fn execute_formula_plan_primary(
        &self,
        plan: &FormulaHotPlan,
        ctx: &FormulaContext,
        inputs: &[&[f64]],
        string_base: usize,
    ) -> Result<Vec<f64>, FormulaError> {
        let mut runtime_context = std::mem::take(&mut *self.plan_runtime_context.borrow_mut());
        runtime_context.reset();
        let host = HostContext {
            chip: ctx.chip_data.as_ref(),
            period_type: ctx.period_type,
            money_flow: ctx.money_flow_data.as_ref(),
            ohlc: Some(HostOhlc {
                high: ctx.high.as_slice(),
                low: ctx.low.as_slice(),
                close: ctx.close.as_slice(),
            }),
            capital: ctx.capital,
            string_table: Some(ctx.string_table.as_slice()),
            em_data: ctx.em_data.as_ref(),
            block_data: ctx.block_data.as_ref(),
        };
        let mut executor =
            unified_formula_executor_with_host_and_context(plan, host, runtime_context);
        executor.dispatcher_mut().set_string_base(string_base);
        let result = executor.execute_primary(inputs);
        *self.plan_runtime_context.borrow_mut() = executor.into_context();
        result.map_err(|error| {
            FormulaError::RuntimeError(format!("formula plan execution failed: {error}"))
        })
    }

    /// Shared execution path. Primary-only callers still publish assignments to
    /// the context, but avoid constructing and cloning the detailed result maps
    /// that `eval_plan_channels` returns to its caller.
    fn eval_plan_channels_impl(
        &self,
        source: &str,
        dialect: FormulaDialect,
        params: &ParamValues,
        ctx: &mut FormulaContext,
        include_details: bool,
    ) -> Result<FormulaPlanOutput, FormulaError> {
        // Shared handle, not an owned plan: the executor only borrows it, so
        // nothing here needs the deep copy `compile_plan` hands to its callers.
        let plan = self.plan_for(source, dialect, params)?;
        self.eval_plan_channels_with_plan(plan, ctx, include_details)
    }

    fn eval_plan_channels_with_plan(
        &self,
        plan: Arc<FormulaHotPlan>,
        ctx: &mut FormulaContext,
        include_details: bool,
    ) -> Result<FormulaPlanOutput, FormulaError> {
        ctx.validate_alignment()?;

        // The plan path must honour the same execution sandbox as the tree path.
        // It used to ignore `ctx.sandbox` outright, so every configured budget
        // became unenforced the moment a caller switched modes -- a silent
        // security regression rather than a numeric one.
        //
        // Two limits map over directly: the nesting limit is checked against the
        // AST depth measured while lowering (lowering is where this path spends
        // stack), and the memory budget is charged the same single output series
        // the tree path charges. The timeout is coarser here -- checked on entry
        // and again after execution, not per node -- because the executor runs
        // the plan as one call.
        let sandbox_unlimited = ctx.sandbox.timeout_ms.is_none()
            && ctx.sandbox.max_recursion_depth.is_none()
            && ctx.sandbox.max_memory_bytes.is_none();
        crate::formula::sandbox::sandbox_reset(&ctx.sandbox_state);
        let _sandbox_guard = if sandbox_unlimited {
            None
        } else {
            crate::formula::sandbox::sandbox_check_depth(
                &ctx.sandbox,
                plan.semantic().max_ast_depth(),
            )?;
            Some(crate::formula::sandbox::sandbox_enter(
                &ctx.sandbox,
                &ctx.sandbox_state,
            )?)
        };

        // String literals are the one side effect that cannot ride in a numeric
        // buffer: the tree path appends each literal to `ctx.string_table` at
        // evaluation time and yields the new slot's index, and functions that
        // take a name argument (`GET_BLOCK_NAME` and the block/field family)
        // read that table back by index during the same evaluation.
        //
        // The plan carries the texts instead of the numbers, so they are
        // appended *before* the input series are borrowed: the read-backs then
        // find the same text at the same index, and the offset handed to the
        // dispatcher keeps the emitted indices absolute when a caller
        // pre-populated the table.
        let string_base = ctx.string_table.len();
        for (_, literal) in plan.semantic().string_literals_ordered() {
            ctx.string_table.push(literal.to_string());
        }

        // Bind every numeric input slot the plan declared. A slot that no binding
        // fills is a hard error: substituting another series would convert an
        // input-layout bug into a silent numeric mismatch.
        // Most formulas bind only one to four series. Keep those borrowed
        // slices inline so a warm evaluation does not allocate a temporary
        // input-reference vector; retain the general heap path for unusual
        // high-arity plans.
        const INLINE_INPUTS: usize = 8;
        let input_count = plan.input_bindings().len();
        let mut inline_inputs: [&[f64]; INLINE_INPUTS] = [&[]; INLINE_INPUTS];
        let mut heap_inputs = Vec::with_capacity(if input_count > INLINE_INPUTS {
            input_count
        } else {
            0
        });
        let mut bound_inputs = 0;
        for binding in plan.input_bindings() {
            let slot = binding.slot().0;
            if slot != bound_inputs {
                return Err(FormulaError::RuntimeError(format!(
                    "formula plan input slot {} is not contiguous after {} bindings",
                    slot, bound_inputs
                )));
            }
            let values = ctx.get_data(binding.name()).ok_or_else(|| {
                FormulaError::RuntimeError(format!(
                    "formula plan requires input series `{}`, which the context does not provide",
                    binding.name()
                ))
            })?;
            if input_count <= INLINE_INPUTS {
                inline_inputs[slot] = values;
            } else {
                heap_inputs.push(values);
            }
            bound_inputs += 1;
        }
        let inputs: &[&[f64]] = if input_count <= INLINE_INPUTS {
            &inline_inputs[..input_count]
        } else {
            &heap_inputs
        };
        if inputs.len() != plan.hot().input_layout().len() {
            return Err(FormulaError::RuntimeError(format!(
                "formula plan bound {} input slots but its hot layout declares {}",
                inputs.len(),
                plan.hot().input_layout().len()
            )));
        }

        // A formula with no bound inputs is a constant expression (`10 + 20`),
        // and it still has a length: the context's series length. `execute`
        // derives the length from `inputs.first()`, which does not exist here, so
        // the range is supplied explicitly. When inputs do exist they define the
        // length, exactly as before.
        let length = inputs.first().map_or(ctx.data_len, |input| input.len());

        // A root MA with one retained result has no context side effects (the
        // plan compiler excludes assignments, named outputs, modifiers and
        // string literals). Return its directly computed series here so the
        // common primary-only API does not wrap it in `Vec<Vec<_>>`, move it
        // through the general publication machinery, or construct host data
        // for a dispatcher that will not be used.
        if !include_details {
            if let Some((close_name, volume_name)) = plan.simple_obv_input_names() {
                if let (Some(close), Some(volume)) =
                    (ctx.get_data(close_name), ctx.get_data(volume_name))
                {
                    if close.len() == length && volume.len() == length {
                        let mut primary = crate::utils::uninit_output(length);
                        if crate::math::volume_kernels::obv_into(close, volume, &mut primary)
                            .is_ok()
                        {
                            for binding in plan.variables() {
                                if binding.output_index() != 0 {
                                    return Err(FormulaError::RuntimeError(format!(
                                        "formula variable `{}` has no retained OBV value",
                                        binding.name()
                                    )));
                                }
                                if let Some(existing) =
                                    ctx.variables.get_mut(binding.context_key().as_ref())
                                {
                                    if existing.len() == length {
                                        if let Some(slice) = existing.as_slice_mut() {
                                            slice.copy_from_slice(&primary);
                                        } else {
                                            ctx.variables.insert(
                                                binding.context_key().clone(),
                                                Array1::from_vec(primary.clone()),
                                            );
                                        }
                                    } else {
                                        ctx.variables.insert(
                                            binding.context_key().clone(),
                                            Array1::from_vec(primary.clone()),
                                        );
                                    }
                                } else {
                                    ctx.variables.insert(
                                        binding.context_key().clone(),
                                        Array1::from_vec(primary.clone()),
                                    );
                                }
                            }
                            for binding in plan.outputs() {
                                if !ctx
                                    .output_names
                                    .iter()
                                    .any(|existing| existing == binding.name())
                                {
                                    ctx.output_names.push(binding.name().to_string());
                                }
                            }
                            for (name, modifier) in plan.output_modifiers() {
                                if ctx.output_modifiers.get(name) != Some(modifier) {
                                    ctx.output_modifiers.insert(name.clone(), modifier.clone());
                                }
                            }
                            if !sandbox_unlimited {
                                crate::formula::sandbox::sandbox_check_elapsed(
                                    &ctx.sandbox,
                                    &ctx.sandbox_state,
                                )?;
                                crate::formula::sandbox::sandbox_track_bytes(
                                    &ctx.sandbox,
                                    &ctx.sandbox_state,
                                    ctx.data_len * 8,
                                )?;
                            }
                            return Ok(FormulaPlanOutput {
                                primary,
                                channels: Vec::new(),
                                variables: Vec::new(),
                            });
                        }
                    }
                }
            }
            if let Some((high_slot, low_slot, close_slot, volume_slot, source_index)) =
                plan.simple_pine_vwap()
            {
                if let (Some(high), Some(low), Some(close), Some(volume)) = (
                    inputs.get(high_slot),
                    inputs.get(low_slot),
                    inputs.get(close_slot),
                    inputs.get(volume_slot),
                ) {
                    if high.len() == length
                        && low.len() == length
                        && close.len() == length
                        && volume.len() == length
                    {
                        let mut source = crate::utils::uninit_output(length);
                        let mut primary = crate::utils::uninit_output(length);
                        let (mut cumulative_price_volume, mut cumulative_volume) = (0.0, 0.0);
                        for i in 0..length {
                            let price = (high[i] + low[i] + close[i]) / 3.0;
                            source[i] = price;
                            cumulative_price_volume += price * volume[i];
                            cumulative_volume += volume[i];
                            primary[i] = if cumulative_volume.abs() > crate::utils::NUMERIC_EPSILON
                            {
                                cumulative_price_volume / cumulative_volume
                            } else {
                                f64::NAN
                            };
                        }
                        for binding in plan.variables() {
                            let index = binding.output_index();
                            let values = if index == source_index {
                                &source
                            } else if index == 0 {
                                &primary
                            } else {
                                return Err(FormulaError::RuntimeError(format!(
                                    "formula variable `{}` has no retained VWAP value",
                                    binding.name()
                                )));
                            };
                            let key = binding.context_key().clone();
                            let wrote_existing = ctx
                                .variables
                                .get_mut(key.as_ref())
                                .filter(|existing| existing.len() == length)
                                .and_then(|existing| existing.as_slice_mut())
                                .map(|existing| existing.copy_from_slice(values))
                                .is_some();
                            if !wrote_existing {
                                ctx.variables.insert(key, Array1::from_vec(values.clone()));
                            }
                        }
                        for binding in plan.outputs() {
                            if !ctx
                                .output_names
                                .iter()
                                .any(|existing| existing == binding.name())
                            {
                                ctx.output_names.push(binding.name().to_string());
                            }
                        }
                        for (name, modifier) in plan.output_modifiers() {
                            if ctx.output_modifiers.get(name) != Some(modifier) {
                                ctx.output_modifiers.insert(name.clone(), modifier.clone());
                            }
                        }
                        if !sandbox_unlimited {
                            crate::formula::sandbox::sandbox_check_elapsed(
                                &ctx.sandbox,
                                &ctx.sandbox_state,
                            )?;
                            crate::formula::sandbox::sandbox_track_bytes(
                                &ctx.sandbox,
                                &ctx.sandbox_state,
                                ctx.data_len * 8,
                            )?;
                        }
                        return Ok(FormulaPlanOutput {
                            primary,
                            channels: Vec::new(),
                            variables: Vec::new(),
                        });
                    }
                }
            }
            if let Some((
                high_slot,
                low_slot,
                close_slot,
                period,
                cci_index,
                source_index,
                scalars,
            )) = plan.simple_pine_cci()
            {
                if let (Some(high), Some(low), Some(close)) = (
                    inputs.get(high_slot),
                    inputs.get(low_slot),
                    inputs.get(close_slot),
                ) {
                    if high.len() == length && low.len() == length && close.len() == length {
                        let mut source = crate::utils::uninit_output(length);
                        for i in 0..length {
                            source[i] = (high[i] + low[i] + close[i]) / 3.0;
                        }
                        let mut primary = crate::utils::uninit_output(length);
                        if crate::indicators::momentum::cci_source_into(
                            &source,
                            period,
                            &mut primary,
                        )
                        .is_ok()
                        {
                            for binding in plan.variables() {
                                let index = binding.output_index();
                                let name = binding.context_key().clone();
                                let series = if index == cci_index {
                                    if let Some(existing) = ctx.variables.get_mut(name.as_ref()) {
                                        if existing.len() == length {
                                            if let Some(slice) = existing.as_slice_mut() {
                                                slice.copy_from_slice(&primary);
                                            } else {
                                                ctx.variables.insert(
                                                    name.clone(),
                                                    Array1::from_vec(primary.clone()),
                                                );
                                            }
                                        } else {
                                            ctx.variables.insert(
                                                name.clone(),
                                                Array1::from_vec(primary.clone()),
                                            );
                                        }
                                    } else {
                                        ctx.variables.insert(
                                            name.clone(),
                                            Array1::from_vec(primary.clone()),
                                        );
                                    }
                                    None
                                } else if index == source_index {
                                    Some(source.clone())
                                } else if let Some((_, scalar)) =
                                    scalars.iter().find(|(slot, _)| *slot == index)
                                {
                                    if let Some(existing) = ctx.variables.get_mut(name.as_ref()) {
                                        if existing.len() == length {
                                            if let Some(slice) = existing.as_slice_mut() {
                                                slice.fill(*scalar);
                                            } else {
                                                ctx.variables.insert(
                                                    name.clone(),
                                                    Array1::from_elem(length, *scalar),
                                                );
                                            }
                                        } else {
                                            ctx.variables.insert(
                                                name.clone(),
                                                Array1::from_elem(length, *scalar),
                                            );
                                        }
                                    } else {
                                        ctx.variables.insert(
                                            name.clone(),
                                            Array1::from_elem(length, *scalar),
                                        );
                                    }
                                    None
                                } else {
                                    return Err(FormulaError::RuntimeError(format!(
                                        "formula variable `{}` has no retained CCI value",
                                        binding.name()
                                    )));
                                };
                                if let Some(series) = series {
                                    if let Some(existing) = ctx.variables.get_mut(name.as_ref()) {
                                        if existing.len() == length {
                                            if let Some(slice) = existing.as_slice_mut() {
                                                slice.copy_from_slice(&series);
                                            } else {
                                                ctx.variables
                                                    .insert(name.clone(), Array1::from_vec(series));
                                            }
                                        } else {
                                            ctx.variables
                                                .insert(name.clone(), Array1::from_vec(series));
                                        }
                                    } else {
                                        ctx.variables
                                            .insert(name.clone(), Array1::from_vec(series));
                                    }
                                }
                            }
                            for binding in plan.outputs() {
                                if !ctx
                                    .output_names
                                    .iter()
                                    .any(|existing| existing == binding.name())
                                {
                                    ctx.output_names.push(binding.name().to_string());
                                }
                            }
                            for (name, modifier) in plan.output_modifiers() {
                                if ctx.output_modifiers.get(name) != Some(modifier) {
                                    ctx.output_modifiers.insert(name.clone(), modifier.clone());
                                }
                            }
                            if !sandbox_unlimited {
                                crate::formula::sandbox::sandbox_check_elapsed(
                                    &ctx.sandbox,
                                    &ctx.sandbox_state,
                                )?;
                                crate::formula::sandbox::sandbox_track_bytes(
                                    &ctx.sandbox,
                                    &ctx.sandbox_state,
                                    ctx.data_len * 8,
                                )?;
                            }
                            return Ok(FormulaPlanOutput {
                                primary,
                                channels: Vec::new(),
                                variables: Vec::new(),
                            });
                        }
                    }
                }
            }
            if let Some((input, period, value_index, scalar_outputs)) = plan
                .simple_pine_mom()
                .and_then(|(_, period, value_index, scalar_outputs)| {
                    inputs
                        .first()
                        .map(|input| (*input, period, value_index, scalar_outputs))
                })
            {
                let mut primary = crate::utils::uninit_output(length);
                if input.len() == length
                    && crate::indicators::momentum::mom_into(input, period, &mut primary).is_ok()
                {
                    for binding in plan.variables() {
                        let index = binding.output_index();
                        if index == value_index {
                            if let Some(existing) =
                                ctx.variables.get_mut(binding.context_key().as_ref())
                            {
                                if existing.len() == length {
                                    if let Some(slice) = existing.as_slice_mut() {
                                        slice.copy_from_slice(&primary);
                                    } else {
                                        ctx.variables.insert(
                                            binding.context_key().clone(),
                                            Array1::from_vec(primary.clone()),
                                        );
                                    }
                                } else {
                                    ctx.variables.insert(
                                        binding.context_key().clone(),
                                        Array1::from_vec(primary.clone()),
                                    );
                                }
                            } else {
                                ctx.variables.insert(
                                    binding.context_key().clone(),
                                    Array1::from_vec(primary.clone()),
                                );
                            }
                        } else if let Some((_, scalar)) =
                            scalar_outputs.iter().find(|(slot, _)| *slot == index)
                        {
                            if let Some(existing) =
                                ctx.variables.get_mut(binding.context_key().as_ref())
                            {
                                if existing.len() == length {
                                    if let Some(slice) = existing.as_slice_mut() {
                                        slice.fill(*scalar);
                                    } else {
                                        ctx.variables.insert(
                                            binding.context_key().clone(),
                                            Array1::from_elem(length, *scalar),
                                        );
                                    }
                                } else {
                                    ctx.variables.insert(
                                        binding.context_key().clone(),
                                        Array1::from_elem(length, *scalar),
                                    );
                                }
                            } else {
                                ctx.variables.insert(
                                    binding.context_key().clone(),
                                    Array1::from_elem(length, *scalar),
                                );
                            }
                        } else {
                            return Err(FormulaError::RuntimeError(format!(
                                "formula variable `{}` has no retained Momentum value",
                                binding.name()
                            )));
                        }
                    }
                    for binding in plan.outputs() {
                        if !ctx
                            .output_names
                            .iter()
                            .any(|existing| existing == binding.name())
                        {
                            ctx.output_names.push(binding.name().to_string());
                        }
                    }
                    for (name, modifier) in plan.output_modifiers() {
                        if ctx.output_modifiers.get(name) != Some(modifier) {
                            ctx.output_modifiers.insert(name.clone(), modifier.clone());
                        }
                    }
                    if !sandbox_unlimited {
                        crate::formula::sandbox::sandbox_check_elapsed(
                            &ctx.sandbox,
                            &ctx.sandbox_state,
                        )?;
                        crate::formula::sandbox::sandbox_track_bytes(
                            &ctx.sandbox,
                            &ctx.sandbox_state,
                            ctx.data_len * 8,
                        )?;
                    }
                    return Ok(FormulaPlanOutput {
                        primary,
                        channels: Vec::new(),
                        variables: Vec::new(),
                    });
                }
            }
            if let Some((close, volume)) = plan
                .simple_obv_input_names()
                .and_then(|(close, volume)| Some((ctx.get_data(close)?, ctx.get_data(volume)?)))
            {
                if close.len() == length && volume.len() == length {
                    let mut primary = crate::utils::uninit_output(length);
                    if crate::math::volume_kernels::obv_into(close, volume, &mut primary).is_ok() {
                        for binding in plan.variables() {
                            if binding.output_index() != 0 {
                                return Err(FormulaError::RuntimeError(format!(
                                    "formula variable `{}` has no retained OBV value",
                                    binding.name()
                                )));
                            }
                            if let Some(existing) =
                                ctx.variables.get_mut(binding.context_key().as_ref())
                            {
                                if existing.len() == length {
                                    if let Some(slice) = existing.as_slice_mut() {
                                        slice.copy_from_slice(&primary);
                                    } else {
                                        ctx.variables.insert(
                                            binding.context_key().clone(),
                                            Array1::from_vec(primary.clone()),
                                        );
                                    }
                                } else {
                                    ctx.variables.insert(
                                        binding.context_key().clone(),
                                        Array1::from_vec(primary.clone()),
                                    );
                                }
                            } else {
                                ctx.variables.insert(
                                    binding.context_key().clone(),
                                    Array1::from_vec(primary.clone()),
                                );
                            }
                        }
                        for binding in plan.outputs() {
                            if !ctx
                                .output_names
                                .iter()
                                .any(|existing| existing == binding.name())
                            {
                                ctx.output_names.push(binding.name().to_string());
                            }
                        }
                        for (name, modifier) in plan.output_modifiers() {
                            if ctx.output_modifiers.get(name) != Some(modifier) {
                                ctx.output_modifiers.insert(name.clone(), modifier.clone());
                            }
                        }
                        if !sandbox_unlimited {
                            crate::formula::sandbox::sandbox_check_elapsed(
                                &ctx.sandbox,
                                &ctx.sandbox_state,
                            )?;
                            crate::formula::sandbox::sandbox_track_bytes(
                                &ctx.sandbox,
                                &ctx.sandbox_state,
                                ctx.data_len * 8,
                            )?;
                        }
                        return Ok(FormulaPlanOutput {
                            primary,
                            channels: Vec::new(),
                            variables: Vec::new(),
                        });
                    }
                }
            }
            if let Some((input, period)) = plan
                .simple_ma_input_period()
                .and_then(|(_, period)| inputs.first().map(|values| (*values, period)))
            {
                let mut primary = crate::utils::uninit_output(length);
                if crate::math::moving_avg::sma_into(input, period, &mut primary).is_err() {
                    primary.fill(f64::NAN);
                }
                if !sandbox_unlimited {
                    crate::formula::sandbox::sandbox_check_elapsed(
                        &ctx.sandbox,
                        &ctx.sandbox_state,
                    )?;
                    crate::formula::sandbox::sandbox_track_bytes(
                        &ctx.sandbox,
                        &ctx.sandbox_state,
                        ctx.data_len * 8,
                    )?;
                }
                return Ok(FormulaPlanOutput {
                    primary,
                    channels: Vec::new(),
                    variables: Vec::new(),
                });
            }
            if let Some((input, period)) = plan
                .simple_rsi_input_period()
                .and_then(|(_, period)| inputs.first().map(|values| (*values, period)))
                .filter(|(input, _)| input.iter().all(|value| value.is_finite()))
            {
                let mut primary = crate::utils::uninit_output(length);
                crate::math::simd_kernels::rsi_simd_into(input, period, &mut primary);
                if !sandbox_unlimited {
                    crate::formula::sandbox::sandbox_check_elapsed(
                        &ctx.sandbox,
                        &ctx.sandbox_state,
                    )?;
                    crate::formula::sandbox::sandbox_track_bytes(
                        &ctx.sandbox,
                        &ctx.sandbox_state,
                        ctx.data_len * 8,
                    )?;
                }
                return Ok(FormulaPlanOutput {
                    primary,
                    channels: Vec::new(),
                    variables: Vec::new(),
                });
            }
            if let Some((_, period)) = plan.simple_zlema_input_period() {
                let input = inputs.first().copied().unwrap_or_default();
                if input.len() == length {
                    let mut primary = crate::utils::uninit_output(length);
                    if crate::math::moving_avg::zlema_into(input, period, &mut primary).is_err() {
                        primary.fill(f64::NAN);
                    }
                    for binding in plan.variables() {
                        if binding.output_index() != 0 {
                            return Err(FormulaError::RuntimeError(format!(
                                "formula variable `{}` has no retained ZLEMA value",
                                binding.name()
                            )));
                        }
                        let key = binding.context_key().clone();
                        let wrote_existing = ctx
                            .variables
                            .get_mut(key.as_ref())
                            .filter(|existing| existing.len() == length)
                            .and_then(|existing| existing.as_slice_mut())
                            .map(|existing| {
                                existing.copy_from_slice(&primary);
                            })
                            .is_some();
                        if !wrote_existing {
                            ctx.variables.insert(key, Array1::from_vec(primary.clone()));
                        }
                    }
                    if !sandbox_unlimited {
                        crate::formula::sandbox::sandbox_check_elapsed(
                            &ctx.sandbox,
                            &ctx.sandbox_state,
                        )?;
                        crate::formula::sandbox::sandbox_track_bytes(
                            &ctx.sandbox,
                            &ctx.sandbox_state,
                            ctx.data_len * 8,
                        )?;
                    }
                    return Ok(FormulaPlanOutput {
                        primary,
                        channels: Vec::new(),
                        variables: Vec::new(),
                    });
                }
            }
        }

        // Host data cannot travel in a numeric slot, so it is handed to the
        // dispatcher separately: the chip distribution, chart period, money-flow
        // series and implicit OHLC for `WINNER`/`COST`/`PERIODTYPE`/`TR` and the
        // money-flow family, plus the string table and the `EM_REF`/`BLOCK*`
        // data the literal-consuming kernels resolve against. Without it those
        // kernels would return NaN (or fail) on the plan path while the tree
        // path returned real values -- a silent cross-path divergence.
        //
        // Borrowed rather than cloned, and scoped to this block: the executor is
        // the only holder of the borrow, and it is dropped with `output`, so the
        // write-back below can still take `&mut ctx`.
        let simple_ma = plan
            .simple_ma_input_period()
            .and_then(|(_, period)| inputs.first().map(|values| (*values, period)));
        let simple_atr = plan
            .simple_atr_input_slots()
            .and_then(|(high, low, close, period)| {
                Some((
                    *inputs.get(high)?,
                    *inputs.get(low)?,
                    *inputs.get(close)?,
                    period,
                ))
            });
        let simple_coppock =
            plan.simple_coppock_input_periods()
                .and_then(|(_, wma_period, long_roc, short_roc)| {
                    inputs
                        .first()
                        .filter(|input| input.len() == ctx.data_len)
                        .map(|input| (*input, wma_period, long_roc, short_roc))
                });
        let simple_obv_inputs = plan
            .simple_obv_input_names()
            .and_then(|(close, volume)| Some((ctx.get_data(close)?, ctx.get_data(volume)?)));
        let primary_only = if !include_details
            && !inputs.is_empty()
            && plan.hot().output_layout().len() == 1
            // These shapes have direct kernels below. Let them reach those
            // kernels instead of dispatching the general executor first.
            && simple_atr.is_none()
            && simple_coppock.is_none()
        {
            Some(self.execute_formula_plan_primary(&plan, ctx, &inputs, string_base)?)
        } else {
            None
        };
        let output_values = if primary_only.is_some() {
            Vec::new()
        } else if let Some((close_slot, volume_slot, period, obv_index, ma_index)) =
            plan.simple_obv_ma().filter(|_| !include_details)
        {
            let close = inputs.get(close_slot).copied().unwrap_or_default();
            let volume = inputs.get(volume_slot).copied().unwrap_or_default();
            if close.len() == length && volume.len() == length {
                let mut obv = crate::utils::uninit_output(length);
                if crate::math::volume_kernels::obv_into(close, volume, &mut obv).is_ok() {
                    let mut ma = crate::utils::uninit_output(length);
                    if crate::math::moving_avg::sma_into(&obv, period, &mut ma).is_ok() {
                        let mut values = vec![Vec::new(); plan.hot().output_layout().len()];
                        values[obv_index] = obv;
                        values[ma_index] = ma;
                        values
                    } else {
                        self.execute_formula_plan(&plan, ctx, &inputs, length, string_base)?
                    }
                } else {
                    self.execute_formula_plan(&plan, ctx, &inputs, length, string_base)?
                }
            } else {
                self.execute_formula_plan(&plan, ctx, &inputs, length, string_base)?
            }
        } else if let Some((close_slot, period, output_index, scalar_outputs)) =
            plan.simple_pine_roc().filter(|_| !include_details)
        {
            let close = inputs.get(close_slot).copied().unwrap_or_default();
            let mut output = crate::utils::uninit_output(length);
            if close.len() == length
                && crate::indicators::momentum::roc_into(close, period, &mut output).is_ok()
            {
                let mut values = vec![Vec::new(); plan.hot().output_layout().len()];
                values[output_index] = output;
                for &(index, scalar) in scalar_outputs {
                    values[index] = vec![scalar; length];
                }
                values
            } else {
                self.execute_formula_plan(&plan, ctx, &inputs, length, string_base)?
            }
        } else if let Some((close_slot, period, output_index, scalar_outputs)) =
            plan.simple_pine_trix().filter(|_| !include_details)
        {
            let close = inputs.get(close_slot).copied().unwrap_or_default();
            let mut output = crate::utils::uninit_output(length);
            if close.len() == length
                && crate::indicators::momentum::trix_into(close, period, &mut output).is_ok()
            {
                let mut values = vec![Vec::new(); plan.hot().output_layout().len()];
                values[output_index] = output;
                for &(index, scalar) in scalar_outputs {
                    values[index] = vec![scalar; length];
                }
                values
            } else {
                self.execute_formula_plan(&plan, ctx, &inputs, length, string_base)?
            }
        } else if let Some((
            high_slot,
            low_slot,
            close_slot,
            period,
            output_index,
            scalar_outputs,
        )) = plan.simple_pine_wpr().filter(|_| !include_details)
        {
            let high = inputs.get(high_slot).copied().unwrap_or_default();
            let low = inputs.get(low_slot).copied().unwrap_or_default();
            let close = inputs.get(close_slot).copied().unwrap_or_default();
            let mut output = crate::utils::uninit_output(length);
            if high.len() == length
                && low.len() == length
                && close.len() == length
                && crate::indicators::momentum::willr_into(high, low, close, period, &mut output)
                    .is_ok()
            {
                let mut values = vec![Vec::new(); plan.hot().output_layout().len()];
                values[output_index] = output;
                for &(index, scalar) in scalar_outputs {
                    values[index] = vec![scalar; length];
                }
                values
            } else {
                self.execute_formula_plan(&plan, ctx, &inputs, length, string_base)?
            }
        } else if let Some((close_slot, volume_slot, period, output_index, scalar_outputs)) =
            plan.simple_pine_vwma().filter(|_| !include_details)
        {
            let close = inputs.get(close_slot).copied().unwrap_or_default();
            let volume = inputs.get(volume_slot).copied().unwrap_or_default();
            let mut output = crate::utils::uninit_output(length);
            if close.len() == length
                && volume.len() == length
                && crate::math::moving_avg::vwma_into(close, volume, period, &mut output).is_ok()
            {
                let mut values = vec![Vec::new(); plan.hot().output_layout().len()];
                values[output_index] = output;
                for &(index, scalar) in scalar_outputs {
                    values[index] = vec![scalar; length];
                }
                values
            } else {
                self.execute_formula_plan(&plan, ctx, &inputs, length, string_base)?
            }
        } else if let Some((input_slot, period, output_index, source_index, scalar_outputs)) =
            plan.simple_pine_rsi().filter(|_| !include_details)
        {
            let input = inputs.get(input_slot).copied().unwrap_or_default();
            let mut output = crate::utils::uninit_output(length);
            if input.len() == length {
                crate::math::simd_kernels::rsi_simd_into(input, period, &mut output);
                let mut values = vec![Vec::new(); plan.hot().output_layout().len()];
                values[output_index] = output;
                values[source_index] = input.to_vec();
                for &(index, scalar) in scalar_outputs {
                    values[index] = vec![scalar; length];
                }
                values
            } else {
                self.execute_formula_plan(&plan, ctx, &inputs, length, string_base)?
            }
        } else if !include_details && plan.simple_multi_ema_assignments().is_some() {
            if let Some((input_slot, assignments)) = plan.simple_multi_ema_assignments() {
                let input = inputs.get(input_slot).copied().unwrap_or_default();
                let periods = [assignments[0].0, assignments[1].0, assignments[2].0];
                if input.len() == length
                    && length >= periods.into_iter().max().unwrap_or(usize::MAX)
                    && input.iter().all(|value| value.is_finite())
                {
                    match crate::indicators::sweep::ema_sweep(input, &periods) {
                        Ok(mut swept) => {
                            let mut values = vec![Vec::new(); plan.hot().output_layout().len()];
                            for (sweep_index, &(_, output_index)) in assignments.iter().enumerate()
                            {
                                values[output_index] = std::mem::take(&mut swept[sweep_index]);
                            }
                            values
                        }
                        Err(_) => {
                            self.execute_formula_plan(&plan, ctx, &inputs, length, string_base)?
                        }
                    }
                } else {
                    self.execute_formula_plan(&plan, ctx, &inputs, length, string_base)?
                }
            } else {
                self.execute_formula_plan(&plan, ctx, &inputs, length, string_base)?
            }
        } else if let Some((input, period)) = simple_ma {
            let mut output = crate::utils::uninit_output(length);
            if crate::math::moving_avg::sma_into(input, period, &mut output).is_err() {
                output.fill(f64::NAN);
            }
            vec![output]
        } else if let Some((high, low, close, period)) = simple_atr {
            if length <= PLAN_SIMPLE_ATR_FASTPATH_MAX_ROWS
                && period < length
                && high.len() == length
                && low.len() == length
                && close.len() == length
                && high.iter().all(|value| value.is_finite())
                && low.iter().all(|value| value.is_finite())
                && close.iter().all(|value| value.is_finite())
            {
                let mut output = crate::utils::uninit_output(length);
                if crate::indicators::volatility::atr_into(high, low, close, period, &mut output)
                    .is_ok()
                {
                    vec![output]
                } else {
                    self.execute_formula_plan(&plan, ctx, &inputs, length, string_base)?
                }
            } else {
                self.execute_formula_plan(&plan, ctx, &inputs, length, string_base)?
            }
        } else if let Some((input, wma_period, long_roc, short_roc)) = simple_coppock {
            let start = crate::formula::functions::warmup_offset(input);
            if start >= input.len() {
                vec![vec![f64::NAN; length]]
            } else {
                match crate::indicators::momentum_ext::coppock(
                    &input[start..],
                    wma_period,
                    long_roc,
                    short_roc,
                ) {
                    Ok(output) => {
                        vec![crate::formula::functions::shift_back(output, start, length)
                            .into_raw_vec()]
                    }
                    Err(_) => vec![vec![f64::NAN; length]],
                }
            }
        } else if let Some((close, volume)) = simple_obv_inputs {
            if close.len() == length && volume.len() == length {
                let mut primary = crate::utils::uninit_output(length);
                if crate::math::volume_kernels::obv_into(close, volume, &mut primary).is_ok() {
                    vec![primary]
                } else {
                    self.execute_formula_plan(&plan, ctx, &inputs, length, string_base)?
                }
            } else {
                self.execute_formula_plan(&plan, ctx, &inputs, length, string_base)?
            }
        } else {
            self.execute_formula_plan(&plan, ctx, &inputs, length, string_base)?
        };

        // Re-check the wall-clock budget: entering the sandbox only proves the
        // limit held before execution started.
        if !sandbox_unlimited {
            crate::formula::sandbox::sandbox_check_elapsed(&ctx.sandbox, &ctx.sandbox_state)?;
            crate::formula::sandbox::sandbox_track_bytes(
                &ctx.sandbox,
                &ctx.sandbox_state,
                ctx.data_len * 8,
            )?;
        }

        if !include_details {
            let mut values = output_values;
            let primary = if let Some(primary) = primary_only {
                primary
            } else {
                values.first_mut().map(std::mem::take).ok_or_else(|| {
                    FormulaError::RuntimeError("formula plan produced no output series".to_string())
                })?
            };
            // Most formulas publish only a handful of variables. Avoid a
            // per-evaluation HashMap allocation for those common plans; keep
            // the general hash-backed path for scripts with many bindings.
            const INLINE_PUBLISHED_SLOTS: usize = 8;
            let variables = plan.variables();
            let mut inline_published: [Option<(usize, std::sync::Arc<str>)>;
                INLINE_PUBLISHED_SLOTS] = std::array::from_fn(|_| None);
            let mut inline_published_len = 0;
            let mut heap_published = std::collections::HashMap::<usize, std::sync::Arc<str>>::new();
            for binding in variables {
                let index = binding.output_index();
                let name = binding.context_key().clone();
                let series = if index == 0 {
                    // The primary output must also be returned to the caller.
                    // Reuse the context's existing array when possible instead
                    // of cloning the primary Vec and allocating a new Array1 on
                    // every warm evaluation.
                    if let Some(existing) = ctx.variables.get_mut(name.as_ref()) {
                        if existing.len() == primary.len() {
                            if let Some(slice) = existing.as_slice_mut() {
                                slice.copy_from_slice(&primary);
                            } else {
                                ctx.variables
                                    .insert(name.clone(), Array1::from_vec(primary.clone()));
                            }
                        } else {
                            ctx.variables
                                .insert(name.clone(), Array1::from_vec(primary.clone()));
                        }
                    } else {
                        ctx.variables
                            .insert(name.clone(), Array1::from_vec(primary.clone()));
                    }
                    None
                } else if let Some(previous_name) = if variables.len() <= INLINE_PUBLISHED_SLOTS {
                    inline_published[..inline_published_len]
                        .iter()
                        .flatten()
                        .find_map(|(slot, previous_name)| (*slot == index).then_some(previous_name))
                } else {
                    heap_published.get(&index)
                } {
                    Some(
                        ctx.variables
                            .get(previous_name)
                            .ok_or_else(|| {
                                FormulaError::RuntimeError(format!(
                                    "formula variable `{}` lost its shared plan slot",
                                    binding.name()
                                ))
                            })?
                            .to_owned()
                            .into_raw_vec(),
                    )
                } else {
                    let Some(slot_value) = values.get_mut(index) else {
                        return Err(FormulaError::RuntimeError(format!(
                            "formula variable `{}` has no retained plan value",
                            binding.name()
                        )));
                    };
                    Some(std::mem::take(slot_value))
                };
                if let Some(series) = series {
                    if let Some(existing) = ctx.variables.get_mut(name.as_ref()) {
                        if existing.len() == series.len() {
                            if let Some(slice) = existing.as_slice_mut() {
                                slice.copy_from_slice(&series);
                            } else {
                                ctx.variables.insert(name.clone(), Array1::from_vec(series));
                            }
                        } else {
                            ctx.variables.insert(name.clone(), Array1::from_vec(series));
                        }
                    } else {
                        ctx.variables.insert(name.clone(), Array1::from_vec(series));
                    }
                }
                if variables.len() <= INLINE_PUBLISHED_SLOTS {
                    if !inline_published[..inline_published_len]
                        .iter()
                        .flatten()
                        .any(|(slot, _)| *slot == index)
                    {
                        inline_published[inline_published_len] = Some((index, name));
                        inline_published_len += 1;
                    }
                } else {
                    heap_published.entry(index).or_insert(name);
                }
            }
            for binding in plan.outputs() {
                if !ctx
                    .output_names
                    .iter()
                    .any(|existing| existing == binding.name())
                {
                    ctx.output_names.push(binding.name().to_string());
                }
            }
            for (name, modifier) in plan.output_modifiers() {
                if ctx.output_modifiers.get(name) != Some(modifier) {
                    ctx.output_modifiers.insert(name.clone(), modifier.clone());
                }
            }
            return Ok(FormulaPlanOutput {
                primary,
                channels: Vec::new(),
                variables: Vec::new(),
            });
        }

        // `ExecutionOutput::values` is ordered by retained output, so resolve each
        // named channel through the output layout instead of assuming the binding
        // order matches it.
        //
        // The primary and the *last* named output are usually the same retained
        // buffer — `DIF:...;DEA:...;MACD:...` reports `MACD` as the result — so
        // neither may simply be `take`n first: taking the channel would leave the
        // primary empty. `values` is therefore consumed through `Option`s.
        //
        // Several *names* can also share one slot: `value := ...` and
        // `plot(value, title="VALUE")` resolve to the same node, so the variable
        // binding and the channel binding point at the same buffer. The first
        // reader takes the buffer and later readers of that slot get a copy of
        // that same series — *not* of the primary one, which is what an
        // unconditional fallback to `primary` handed back whenever the shared
        // value was not itself the primary result.
        let layout = plan.hot().output_layout().outputs();
        let mut values: Vec<Option<Vec<f64>>> = output_values.into_iter().map(Some).collect();
        let primary = values.first_mut().and_then(Option::take).ok_or_else(|| {
            FormulaError::RuntimeError("formula plan produced no output series".to_string())
        })?;
        let mut already_read: std::collections::HashMap<usize, Vec<f64>> =
            std::collections::HashMap::new();
        let mut read_slot = |slot: crate::buffer_arena::BufferSlot| -> Option<Vec<f64>> {
            let index = layout
                .iter()
                .position(|(_, candidate)| *candidate == slot)?;
            // Index 0 is the primary result, which was taken above.
            if index == 0 {
                return Some(primary.clone());
            }
            if let Some(existing) = already_read.get(&index) {
                return Some(existing.clone());
            }
            let series = values[index].take()?;
            already_read.insert(index, series.clone());
            Some(series)
        };

        let channels: Vec<(String, Vec<f64>)> = plan
            .outputs()
            .iter()
            .filter_map(|binding| {
                read_slot(binding.slot()).map(|series| (binding.name().to_string(), series))
            })
            .collect();
        let variables: Vec<(String, Vec<f64>)> = plan
            .variables()
            .iter()
            .filter_map(|binding| {
                read_slot(binding.slot()).map(|series| (binding.name().to_string(), series))
            })
            .collect();

        // Publish the same side effects the tree path leaves behind, so a caller
        // reading `ctx.variables`/`ctx.output_names` after an evaluation cannot
        // tell which mode ran it. The language bindings build their result
        // dictionaries from exactly these two fields; without this, switching to
        // the plan path silently dropped every named result.
        for (name, series) in &variables {
            ctx.variables.insert(
                std::sync::Arc::<str>::from(name.as_str()),
                Array1::from_vec(series.clone()),
            );
        }
        for (name, _) in &channels {
            if !ctx.output_names.iter().any(|existing| existing == name) {
                ctx.output_names.push(name.clone());
            }
        }
        // Styling travels beside the numeric plan rather than inside it, so it is
        // copied here rather than derived from a channel's series.
        for (name, modifier) in plan.output_modifiers() {
            if ctx.output_modifiers.get(name) != Some(modifier) {
                ctx.output_modifiers.insert(name.clone(), modifier.clone());
            }
        }

        Ok(FormulaPlanOutput {
            primary,
            channels,
            variables,
        })
    }

    /// Register a reusable, parameterized expression component.
    ///
    /// Components are expanded before analysis and execution, so a formula
    /// such as `SIGNAL(CLOSE)` can be composed from built-ins without adding a
    /// new runtime function. Registration invalidates compiled plans because
    /// an existing source may now resolve a newly registered component.
    pub fn register_custom_formula(
        &mut self,
        name: &str,
        parameters: &[&str],
        source: &str,
    ) -> Result<(), FormulaError> {
        self.custom_formulas
            .register(name, parameters, source)
            .map_err(FormulaError::InvalidOperation)?;
        self.invalidate_formula_caches();
        Ok(())
    }

    /// Alias for [`Self::register_custom_formula`] for registry-oriented APIs.
    pub fn register_formula(
        &mut self,
        name: &str,
        parameters: &[&str],
        source: &str,
    ) -> Result<(), FormulaError> {
        self.register_custom_formula(name, parameters, source)
    }

    /// Remove one custom component and invalidate compiled plans if removed.
    pub fn unregister_custom_formula(&mut self, name: &str) -> Result<bool, FormulaError> {
        let removed = self
            .custom_formulas
            .unregister(name)
            .map_err(FormulaError::InvalidOperation)?;
        if removed {
            self.invalidate_formula_caches();
        }
        Ok(removed)
    }

    /// Remove all custom components and invalidate compiled plans.
    pub fn clear_custom_formulas(&mut self) {
        self.custom_formulas.clear();
        self.invalidate_formula_caches();
    }

    /// Return registered custom component names in deterministic order.
    pub fn custom_formula_names(&self) -> Vec<String> {
        self.custom_formulas.names()
    }

    /// Inspect the registry used by this engine.
    pub fn custom_formula_registry(&self) -> &FormulaRegistry {
        &self.custom_formulas
    }

    fn invalidate_formula_caches(&mut self) {
        self.cache.clear();
        self.pine_ast_cache.borrow_mut().clear();
        self.auto_plan_cache.borrow_mut().clear();
        self.semantic_plan_cache.borrow_mut().clear();
        // `invalidate`, not `clear`: the plans are stale because a custom
        // component changed, but the counters still describe this engine.
        self.plan_cache.borrow_mut().invalidate();
        self.bytecode_cache.borrow_mut().clear();
        self.streaming_ema.borrow_mut().clear();
        self.streaming_common.borrow_mut().clear();
    }

    /// 执行已编译的公式
    pub fn execute(
        &self,
        formula: &CompiledFormula,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        // Fast-path the common single-indicator formulas before entering the
        // general AST executor. This avoids materialising input argument
        // arrays and lets the SIMD _into kernels write directly into one
        // result buffer. Complex formulas keep the existing semantics.
        let sandbox_unlimited = ctx.sandbox.timeout_ms.is_none()
            && ctx.sandbox.max_recursion_depth.is_none()
            && ctx.sandbox.max_memory_bytes.is_none();
        if sandbox_unlimited {
            if let Some(result) = self.try_execute_simple_formula(&formula.ast, ctx) {
                return Ok(result);
            }
        }
        self.executor.execute(&formula.ast, ctx)
    }

    /// Execute a simple built-in formula directly into a caller-owned buffer.
    fn try_execute_simple_formula_into(
        &self,
        ast: &AstNode,
        ctx: &FormulaContext,
        output: &mut Array1<f64>,
    ) -> bool {
        let (name, args) = match ast {
            AstNode::FunctionCall { name, args } if args.len() >= 2 => (name.as_str(), args),
            _ => return false,
        };

        // Multi-series form: `ATR(HIGH, LOW, CLOSE, N)` carries its period
        // *last*, so it cannot share the single-input shape below. It is the
        // one multi-input kernel worth specialising because it is the widest
        // gap in the engine-vs-native measurements.
        if name.eq_ignore_ascii_case("atr") && args.len() >= 4 {
            return try_atr_fast_path(
                ctx,
                args,
                output.as_slice_mut().expect("Array1 is contiguous"),
            );
        }

        let input = match &args[0] {
            AstNode::Variable(name) => ctx.get_data(name),
            _ => None,
        };
        let Some(input) = input else {
            return false;
        };
        if input.len() != output.len() {
            return false;
        }

        let period_value = match &args[1] {
            AstNode::Number(value) if value.is_finite() && *value > 0.0 => *value,
            _ => return false,
        };
        let period = period_value as usize;
        if period == 0 {
            return false;
        }

        // The name check must precede the non-finite guard, and both must fall
        // through rather than answer from here:
        //
        // * an unsupported name (`SUM`, `HHV`, ...) was previously answered with
        //   all-NaN purely because its input was non-finite, silently shadowing
        //   the general executor;
        // * a supported name whose input is not fully finite must reach the
        //   general executor, which owns the "invalid market data becomes
        //   all-NaN" contract *and* the math layer's warm-up rule -- a
        //   **leading** NaN run is an upstream indicator's warm-up prefix and is
        //   computed through, while **interior** non-finite values are rejected
        //   there and become all-NaN anyway.
        let upper = name.to_ascii_uppercase();
        if !matches!(
            upper.as_str(),
            "MA" | "BOLLMID" | "EMA" | "RSI" | "WMA" | "DEMA" | "TEMA" | "TRIMA" | "KAMA"
        ) || input.iter().any(|value| !value.is_finite())
        {
            return false;
        }

        let out = output.as_slice_mut().expect("Array1 is contiguous");
        // Every kernel below is the one the general executor would reach --
        // `fn_wma` calls `moving_avg::wma`, which shares `wma_kernel_into`
        // with `wma_into`, and so on -- so the specialised path is bit-identical
        // to the fallback, not merely close. `DEMA`/`TEMA` additionally apply
        // `warmup_offset` + `shift_back`; that pair is a no-op exactly when the
        // input is fully finite, which is the guard this path already requires.
        //
        // EMA deliberately uses `moving_avg::ema_into` rather than
        // `simd_kernels::ema_simd_into`: measured in-process at 10k bars, the
        // "SIMD" EMA is 44.4 us against 6.2 us for `ema_fast_into` and ~20 us
        // for the scalar recurrence, because EMA is a serial dependency chain
        // that the block-prefix AVX2 path does not actually shorten.
        match upper.as_str() {
            "MA" | "BOLLMID" => {
                crate::math::simd_kernels::sma_simd_into(input, period, out);
                true
            }
            "EMA" => crate::math::moving_avg::ema_into(input, period, out).is_ok(),
            "RSI" => {
                crate::math::simd_kernels::rsi_simd_into(input, period, out);
                true
            }
            "WMA" => crate::math::moving_avg::wma_into(input, period, out).is_ok(),
            "DEMA" => crate::math::moving_avg::dema_into(input, period, out).is_ok(),
            "TEMA" => crate::math::moving_avg::tema_into(input, period, out).is_ok(),
            "TRIMA" => crate::math::moving_avg::trima_into(input, period, out).is_ok(),
            // `kama_into` is the same `dispatch_kama_kernel` the allocating
            // `kama` wraps, and it writes every slot (proved by
            // `fastpath_full_writer_contract`), so the result lands in the caller's
            // buffer directly -- 2/30 smoothing defaults as `fn_kama` uses.
            "KAMA" => crate::math::moving_avg::kama_into(input, period, 2, 30, out).is_ok(),
            _ => false,
        }
    }

    /// Execute a simple built-in formula through the native SIMD kernel.
    ///
    /// This is deliberately conservative: only a single function call with a
    /// literal period is specialised, so formulas with assignments, nested
    /// expressions, or dynamic periods still use the general executor.
    fn try_execute_simple_formula(
        &self,
        ast: &AstNode,
        ctx: &FormulaContext,
    ) -> Option<Array1<f64>> {
        let (name, args) = match ast {
            AstNode::FunctionCall { name, args } if args.len() >= 2 => (name.as_str(), args),
            _ => return None,
        };

        // Multi-series form: `ATR(HIGH, LOW, CLOSE, N)` carries its period
        // *last*, so the single-input shape below cannot express it.
        if name.eq_ignore_ascii_case("atr") && args.len() >= 4 {
            let len = match &args[0] {
                AstNode::Variable(name) => ctx.get_data(name).map(<[f64]>::len),
                _ => None,
            };
            let Some(len) = len else {
                return None;
            };
            // `atr_into` is a full writer (see `fastpath_full_writer_contract`), so the
            // buffer needs no NaN pre-fill.
            let mut output = Array1::from_vec(crate::utils::uninit_output(len));
            return if try_atr_fast_path(
                ctx,
                args,
                output.as_slice_mut().expect("Array1 is contiguous"),
            ) {
                Some(output)
            } else {
                None
            };
        }

        let input = match &args[0] {
            AstNode::Variable(name) => ctx.get_data(name),
            _ => None,
        }?;

        let period_value = match &args[1] {
            AstNode::Number(value) if value.is_finite() && *value > 0.0 => *value,
            _ => return None,
        };
        let period = period_value as usize;
        if period == 0 {
            return None;
        }

        // See `try_execute_simple_formula_into`: the name check must precede the
        // non-finite guard, and neither an unsupported name nor a non-finite
        // input may be answered from here. The general executor owns both the
        // "invalid market data becomes all-NaN" contract and the math layer's
        // warm-up rule, under which a leading NaN run is computed through.
        let upper = name.to_ascii_uppercase();
        if !matches!(
            upper.as_str(),
            "MA" | "BOLLMID" | "EMA" | "RSI" | "WMA" | "DEMA" | "TEMA" | "TRIMA" | "KAMA"
        ) || input.iter().any(|value| !value.is_finite())
        {
            return None;
        }

        // See `try_execute_simple_formula_into` for why every kernel below is
        // bit-identical to the general path, and why EMA uses the scalar
        // recurrence instead of `simd_kernels::ema_simd_into`.
        // Every kernel in the `match` below is a full writer (proved by
        // `fastpath_full_writer_contract`), so the result buffer is allocated
        // uninitialised rather than paying a full-length NaN store pass.
        let mut output = Array1::from_vec(crate::utils::uninit_output(input.len()));
        let out = output.as_slice_mut().expect("Array1 is contiguous");
        match upper.as_str() {
            "MA" | "BOLLMID" => {
                crate::math::simd_kernels::sma_simd_into(input, period, out);
            }
            "EMA" => {
                if crate::math::moving_avg::ema_into(input, period, out).is_err() {
                    return None;
                }
            }
            "RSI" => {
                crate::math::simd_kernels::rsi_simd_into(input, period, out);
            }
            "WMA" => {
                if crate::math::moving_avg::wma_into(input, period, out).is_err() {
                    return None;
                }
            }
            "DEMA" => {
                if crate::math::moving_avg::dema_into(input, period, out).is_err() {
                    return None;
                }
            }
            "TEMA" => {
                if crate::math::moving_avg::tema_into(input, period, out).is_err() {
                    return None;
                }
            }
            "TRIMA" => {
                if crate::math::moving_avg::trima_into(input, period, out).is_err() {
                    return None;
                }
            }
            "KAMA" => {
                if crate::math::moving_avg::kama_into(input, period, 2, 30, out).is_err() {
                    return None;
                }
            }
            _ => return None,
        }

        Some(output)
    }

    /// Execute a compiled formula into a caller-provided output buffer.
    ///
    /// Reuse the same buffer and engine across calls to avoid allocating the
    /// final result on every evaluation. The formula's intermediate buffers
    /// are also recycled by the executor.
    pub fn eval_into(
        &self,
        formula: &CompiledFormula,
        ctx: &mut FormulaContext,
        output: &mut Array1<f64>,
    ) -> Result<(), FormulaError> {
        if output.len() != ctx.data_len {
            return Err(FormulaError::InvalidParameter(format!(
                "output length {} != ctx.data_len {}",
                output.len(),
                ctx.data_len
            )));
        }
        let sandbox_unlimited = ctx.sandbox.timeout_ms.is_none()
            && ctx.sandbox.max_recursion_depth.is_none()
            && ctx.sandbox.max_memory_bytes.is_none();
        if !sandbox_unlimited {
            let result = self.executor.execute(&formula.ast, ctx)?;
            output.assign(&result);
            return Ok(());
        }
        if self.try_execute_simple_formula_into(&formula.ast, ctx, output) {
            return Ok(());
        }
        self.executor.eval_into(&formula.ast, ctx, output)
    }

    /// Evaluate only the requested half-open output range.
    ///
    /// For formulas with a finite dependency lookback, the executor receives
    /// only the dependency window. Recursive/unknown functions conservatively
    /// use the full prefix to preserve exact historical semantics.
    pub fn eval_range(
        &self,
        formula: &CompiledFormula,
        ctx: &FormulaContext,
        start: usize,
        end: usize,
    ) -> Result<Array1<f64>, FormulaError> {
        if start > end || end > ctx.data_len {
            return Err(FormulaError::InvalidParameter(format!(
                "invalid eval_range [{start}, {end}) for data_len {}",
                ctx.data_len
            )));
        }
        if start == end {
            return Ok(Array1::zeros(0));
        }
        let cached_effects = {
            let cache = self.semantic_plan_cache.borrow();
            cache
                .get(&formula.source)
                .map(|plan| plan.plan().has_observable_effects())
        };
        let has_observable_effects = match cached_effects {
            Some(value) => value,
            None => {
                // CompiledFormula is public and can originate from another
                // FormulaCompiler, so rebuild semantic metadata defensively
                // when this engine did not create the object itself.
                let plan = FormulaComputePlan::compile(&formula.ast).map_err(|error| {
                    FormulaError::InvalidOperation(format!(
                        "formula compute planning failed: {error}"
                    ))
                })?;
                let value = plan.plan().has_observable_effects();
                self.semantic_plan_cache
                    .borrow_mut()
                    .insert(formula.source.clone(), plan);
                value
            }
        };
        let window_start = if has_observable_effects {
            // Effectful formulas may depend on assignments, outputs, drawing,
            // or control flow before the requested range. Preserve the full
            // prefix until a dedicated control-flow-aware incremental backend
            // can prove a smaller window is safe.
            0
        } else {
            FormulaOptimizer::required_lookback(&formula.ast)
                .map(|lookback| start.saturating_sub(lookback))
                .unwrap_or(0)
        };
        // The OHLCV arrays are borrowed for this synchronous call.  The old
        // owned window copied five full slices on every chart refresh; only
        // optional ndarray-backed metadata still needs a defensive copy.
        let mut window = ctx.borrowed_window(window_start, end)?;
        let result = self.execute(formula, &mut window)?;
        let local_start = start - window_start;
        Ok(result
            .slice(ndarray::s![local_start..(local_start + end - start)])
            .to_owned())
    }

    /// Evaluate a half-open range directly from borrowed contiguous OHLCV
    /// slices.  This is the public high-throughput range API used by bindings;
    /// it avoids copying the complete history before dependency trimming.
    pub fn eval_range_zero_copy_inputs(
        &self,
        formula: &CompiledFormula,
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        volume: &[f64],
        start: usize,
        end: usize,
        amount: Option<&[f64]>,
    ) -> Result<Array1<f64>, FormulaError> {
        if close.is_empty()
            || [open, high, low, close, volume]
                .iter()
                .any(|values| values.len() != close.len())
            || amount.is_some_and(|values| values.len() != close.len())
            || start > end
            || end > close.len()
        {
            return Err(FormulaError::InvalidParameter(
                "zero-copy range inputs must be non-empty, aligned, and satisfy 0 <= start <= end <= len"
                    .to_string(),
            ));
        }
        let context = FormulaContext::from_borrowed_ohlcv(
            open,
            high,
            low,
            close,
            volume,
            amount.map(|values| Array1::from_vec(values.to_vec())),
        );
        self.eval_range(formula, &context, start, end)
    }

    /// Evaluate the last bar only.
    pub fn eval_last(
        &self,
        formula: &CompiledFormula,
        ctx: &FormulaContext,
    ) -> Result<f64, FormulaError> {
        if ctx.data_len == 0 {
            return Err(FormulaError::InvalidParameter(
                "cannot eval_last an empty context".to_string(),
            ));
        }
        if let Some(value) = self.try_eval_last_streaming_ema(formula, ctx) {
            return Ok(value);
        }
        if let Some(value) = self.try_eval_last_streaming_common(formula, ctx) {
            return Ok(value);
        }
        let result = self.eval_range(formula, ctx, ctx.data_len - 1, ctx.data_len)?;
        Ok(result[0])
    }

    /// O(1) EMA append path for a direct built-in formula.  This path is
    /// deliberately conservative: it is used only for a literal period and
    /// a direct OHLCV/variable input, and continuity is checked by length and
    /// the previous sample.  Any mismatch uses the exact batch evaluator.
    fn try_eval_last_streaming_ema(
        &self,
        formula: &CompiledFormula,
        ctx: &FormulaContext,
    ) -> Option<f64> {
        let (input_name, period) = match &formula.ast {
            AstNode::FunctionCall { name, args }
                if name.eq_ignore_ascii_case("EMA") && args.len() >= 2 =>
            {
                let AstNode::Variable(input_name) = &args[0] else {
                    return None;
                };
                let AstNode::Number(period) = args[1] else {
                    return None;
                };
                if !period.is_finite() || period < 1.0 || period.fract() != 0.0 {
                    return None;
                }
                (input_name.clone(), period as usize)
            }
            _ => return None,
        };
        let input = ctx.get_data(&input_name)?;
        if input.len() != ctx.data_len || input.is_empty() {
            self.streaming_ema.borrow_mut().remove(&formula.source);
            return None;
        }

        let mut states = self.streaming_ema.borrow_mut();
        let state_was_existing = states.contains_key(&formula.source);
        if !state_was_existing && input.iter().any(|v| !v.is_finite()) {
            return None;
        }

        // A cached state is only reusable for a genuine one-bar append.  A
        // same-length call means a *different* context — the scan loop shape,
        // where one `CompiledFormula` is evaluated against symbol after symbol
        // of equal length — or a caller mutation of the history.  Dropping the
        // state and returning `None` used to hand those calls to `eval_range`,
        // which trims the window to `required_lookback` and therefore cannot
        // rebuild a Wilder-style recursion: every second symbol came back
        // `NaN`.  Reseed from the supplied series instead, so `eval_last`
        // always returns the value the series actually implies.
        let can_append = states.get(&formula.source).is_some_and(|state| {
            state.period == period
                && state.len != 0
                && state.len + 1 == ctx.data_len
                && state.previous_input == input[state.len - 1]
        });
        if !can_append {
            states.remove(&formula.source);
        }
        let state = states.entry(formula.source.clone()).or_insert_with(|| {
            let seed_sum = input.iter().sum::<f64>();
            if input.len() >= period {
                let initial_sum = input[input.len() - period..].iter().sum::<f64>();
                let value = if input.len() == period {
                    initial_sum / period as f64
                } else {
                    // A state created after a full evaluation must match the
                    // last batch output; calculate that one time only.
                    crate::math::moving_avg::ema(input, period)
                        .ok()
                        .and_then(|v| v.last().copied())
                        .unwrap_or(f64::NAN)
                };
                StreamingEmaState {
                    period,
                    len: input.len(),
                    seed_sum: initial_sum,
                    value,
                    valid: value.is_finite(),
                    previous_input: *input.last().unwrap(),
                }
            } else {
                StreamingEmaState {
                    period,
                    len: input.len(),
                    seed_sum,
                    value: f64::NAN,
                    valid: false,
                    previous_input: *input.last().unwrap(),
                }
            }
        });

        if !can_append {
            // Freshly seeded or reseeded from the current series: the seed
            // already holds the value for the last bar.
            return Some(state.value);
        }

        let current = input[state.len];
        if !current.is_finite() {
            states.remove(&formula.source);
            return None;
        }
        state.len = ctx.data_len;
        state.previous_input = current;
        if !state.valid {
            state.seed_sum += current;
            if state.len >= period {
                state.value = state.seed_sum / period as f64;
                state.valid = true;
            } else {
                state.value = f64::NAN;
            }
        } else {
            let alpha = 2.0 / (period as f64 + 1.0);
            state.value = (current - state.value) * alpha + state.value;
        }
        Some(state.value)
    }

    /// O(1) append path for direct MA/RSI/ATR formula calls.  The first call
    /// seeds the indicator from the supplied history; subsequent calls are
    /// accepted only for a genuine one-bar append with an unchanged previous
    /// input.  Any mutation or discontinuity falls back to exact evaluation.
    fn try_eval_last_streaming_common(
        &self,
        formula: &CompiledFormula,
        ctx: &FormulaContext,
    ) -> Option<f64> {
        let (kind, input_names, period) = match &formula.ast {
            AstNode::FunctionCall { name, args } if args.len() >= 2 => {
                let upper = name.to_ascii_uppercase();
                let period_index = match upper.as_str() {
                    "MA" | "RSI" => 1,
                    "ATR" => 3,
                    _ => return None,
                };
                let Some(AstNode::Number(period)) = args.get(period_index) else {
                    return None;
                };
                if !period.is_finite() || *period < 1.0 || period.fract() != 0.0 {
                    return None;
                }
                let required_inputs = if upper == "ATR" { 3 } else { 1 };
                let names = args
                    .get(..required_inputs)?
                    .iter()
                    .map(|arg| match arg {
                        AstNode::Variable(name) => Some(name.clone()),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()?;
                (upper, names, *period as usize)
            }
            _ => return None,
        };
        if ctx.data_len == 0
            || input_names.iter().any(|name| {
                ctx.get_data(name)
                    .is_none_or(|values| values.len() != ctx.data_len)
            })
        {
            self.streaming_common.borrow_mut().remove(&formula.source);
            return None;
        }

        let values = |index: usize| -> Option<[f64; 3]> {
            let mut input = [0.0; 3];
            for (slot, name) in input_names.iter().enumerate() {
                let series = ctx.get_data(name)?;
                input[slot] = *series.get(index)?;
            }
            if input[..input_names.len()]
                .iter()
                .any(|value| !value.is_finite())
            {
                None
            } else {
                Some(input)
            }
        };

        let mut states = self.streaming_common.borrow_mut();

        // Only a genuine one-bar append over the cached history is O(1).  A
        // same-length call — one compiled formula evaluated against symbol
        // after symbol of equal length, which is exactly what a market scan
        // does — must reseed from the supplied series.  Dropping the state and
        // falling through to `eval_range` used to return `NaN` for every
        // second symbol, because that path trims the window to
        // `required_lookback` and cannot rebuild a recursive indicator.
        let can_append = states.get(&formula.source).is_some_and(|state| {
            state.len != 0
                && state.len + 1 == ctx.data_len
                && values(state.len - 1) == Some(state.last_input)
        });

        if !can_append {
            states.remove(&formula.source);
            let mut indicator = match kind.as_str() {
                "MA" => StreamingFormulaIndicator::Sma(StreamingSma::new(period)),
                "RSI" => StreamingFormulaIndicator::Rsi(StreamingRsi::new(period)),
                "ATR" => StreamingFormulaIndicator::Atr(StreamingSmaAtr::new(period)),
                _ => return None,
            };
            let mut last = [f64::NAN; 3];
            let mut value = None;
            for index in 0..ctx.data_len {
                let input = values(index)?;
                last = input;
                value = indicator.next(input);
            }
            let value = value.unwrap_or(f64::NAN);
            states.insert(
                formula.source.clone(),
                StreamingFormulaState {
                    len: ctx.data_len,
                    last_input: last,
                    indicator,
                },
            );
            return Some(value);
        }

        let state = states.get_mut(&formula.source)?;
        let current = values(state.len)?;
        let value = state.indicator.next(current);
        state.len = ctx.data_len;
        state.last_input = current;
        Some(value.unwrap_or(f64::NAN))
    }

    /// Analyze a formula without executing it.
    pub fn analyze(&mut self, source: &str) -> Result<FormulaAnalysis, FormulaError> {
        let formula = self.compile_shared(source)?;
        Ok(analyze_formula(&formula.ast))
    }

    /// Analyze an already parsed/compiled AST without executing it.
    pub fn analyze_ast(&self, ast: &AstNode) -> FormulaAnalysis {
        analyze_formula(ast)
    }

    /// Return the stable result-shape and warm-up contract for a formula.
    pub fn metadata(
        &mut self,
        source: &str,
        data_len: usize,
    ) -> Result<FormulaSeriesMetadata, FormulaError> {
        let analysis = self.analyze(source)?;
        Ok(analysis.result_metadata(data_len))
    }

    /// Return metadata for an already compiled formula.
    pub fn metadata_for_formula(
        &self,
        formula: &CompiledFormula,
        data_len: usize,
    ) -> FormulaSeriesMetadata {
        analyze_formula(&formula.ast).result_metadata(data_len)
    }

    /// Evaluate common NumPy-backed formulas directly from borrowed slices.
    ///
    /// The fast path never materialises an Array1 for the five OHLCV inputs.
    /// Complex formulas fall back to the regular executor, where intermediate
    /// arrays are required by the formula function ABI.
    pub fn eval_zero_copy_inputs(
        &self,
        formula: &CompiledFormula,
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        volume: &[f64],
        amount: Option<&[f64]>,
    ) -> Result<Array1<f64>, FormulaError> {
        if close.is_empty()
            || [open, high, low, close, volume]
                .iter()
                .any(|values| values.len() != close.len())
            || amount.is_some_and(|values| values.len() != close.len())
        {
            return Err(FormulaError::InvalidParameter(
                "zero-copy formula inputs must be non-empty and have equal lengths".to_string(),
            ));
        }

        if let Some(result) =
            self.try_execute_simple_formula_slices(&formula.ast, open, high, low, close, volume)
        {
            return Ok(result);
        }

        let mut context = FormulaContext::from_borrowed_ohlcv(
            open,
            high,
            low,
            close,
            volume,
            amount.map(|values| Array1::from_vec(values.to_vec())),
        );
        self.execute(formula, &mut context)
    }

    fn try_execute_simple_formula_slices(
        &self,
        ast: &AstNode,
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        volume: &[f64],
    ) -> Option<Array1<f64>> {
        let (name, args) = match ast {
            AstNode::FunctionCall { name, args } if args.len() >= 2 => (name.as_str(), args),
            _ => return None,
        };
        let input = match &args[0] {
            AstNode::Variable(name)
                if name.eq_ignore_ascii_case("C") || name.eq_ignore_ascii_case("CLOSE") =>
            {
                close
            }
            AstNode::Variable(name)
                if name.eq_ignore_ascii_case("O") || name.eq_ignore_ascii_case("OPEN") =>
            {
                open
            }
            AstNode::Variable(name)
                if name.eq_ignore_ascii_case("H") || name.eq_ignore_ascii_case("HIGH") =>
            {
                high
            }
            AstNode::Variable(name)
                if name.eq_ignore_ascii_case("L") || name.eq_ignore_ascii_case("LOW") =>
            {
                low
            }
            AstNode::Variable(name)
                if name.eq_ignore_ascii_case("V")
                    || name.eq_ignore_ascii_case("VOL")
                    || name.eq_ignore_ascii_case("VOLUME") =>
            {
                volume
            }
            _ => return None,
        };
        if input.is_empty()
            || [open, high, low, close, volume]
                .iter()
                .any(|values| values.len() != input.len())
        {
            return None;
        }
        let period_value = match &args[1] {
            AstNode::Number(value) if value.is_finite() && *value > 0.0 => *value,
            _ => return None,
        };
        let period = period_value as usize;
        if period == 0 || (period as f64 - period_value).abs() > f64::EPSILON {
            return None;
        }
        // Same rule as the two `try_execute_simple_formula*` helpers above: the
        // name check comes first and a non-finite input falls through, so the
        // general executor applies the math layer's warm-up rule.
        let upper = name.to_ascii_uppercase();
        if !matches!(
            upper.as_str(),
            "MA" | "BOLLMID" | "EMA" | "RSI" | "WMA" | "DEMA" | "TEMA" | "TRIMA" | "KAMA"
        ) || input.iter().any(|value| !value.is_finite())
        {
            return None;
        }
        // Full writers (proved by `fastpath_full_writer_contract`): no NaN pre-fill.
        let mut output = Array1::from_vec(crate::utils::uninit_output(input.len()));
        let out = output.as_slice_mut().expect("Array1 is contiguous");
        match upper.as_str() {
            "MA" | "BOLLMID" => crate::math::simd_kernels::sma_simd_into(input, period, out),
            // `ema_into`, not `simd_kernels::ema_simd_into`: the latter differs
            // from the scalar recurrence by 16/10000 bits, which would make this
            // entry point disagree with `eval`. Same reason as
            // `try_execute_simple_formula_into`.
            "EMA" => {
                if crate::math::moving_avg::ema_into(input, period, out).is_err() {
                    return None;
                }
            }
            "RSI" => crate::math::simd_kernels::rsi_simd_into(input, period, out),
            "WMA" => {
                if crate::math::moving_avg::wma_into(input, period, out).is_err() {
                    return None;
                }
            }
            "DEMA" => {
                if crate::math::moving_avg::dema_into(input, period, out).is_err() {
                    return None;
                }
            }
            "TEMA" => {
                if crate::math::moving_avg::tema_into(input, period, out).is_err() {
                    return None;
                }
            }
            "TRIMA" => {
                if crate::math::moving_avg::trima_into(input, period, out).is_err() {
                    return None;
                }
            }
            "KAMA" => {
                if crate::math::moving_avg::kama_into(input, period, 2, 30, out).is_err() {
                    return None;
                }
            }
            _ => return None,
        }
        Some(output)
    }

    /// 便捷方法：编译并执行
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "debug", skip_all, fields(source_len = source.len())))]
    pub fn eval(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        if self.execution_mode == FormulaExecutionMode::Plan {
            return self.eval_plan(source, ctx);
        }
        if self.execution_mode == FormulaExecutionMode::Auto {
            if let Some(plan) =
                self.auto_plan_for(source, FormulaDialect::default(), ctx, &ParamValues::new())
            {
                return Ok(Array1::from_vec(
                    self.eval_plan_channels_with_plan(plan, ctx, false)?
                        .into_primary(),
                ));
            }
            return self.eval_tree(source, ctx);
        }
        self.eval_tree(source, ctx)
    }

    fn eval_tree(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let formula = self.compile_shared(source)?;
        self.execute(&formula, ctx)
    }

    /// Compile and evaluate a formula through the selected dialect contract.
    ///
    /// Domestic dialects share the canonical AlphaTA AST/runtime, while Pine
    /// is parsed and lowered by its dedicated subset mapper. Source
    /// normalization is performed once here so native, panel, and FFI callers
    /// cannot diverge on BOM or line-ending handling.
    ///
    /// Under [`FormulaExecutionMode::Plan`] both branches route through the
    /// compiled plan path, which does its own dialect-aware parse.
    pub fn eval_with_dialect(
        &mut self,
        source: &str,
        dialect: FormulaDialect,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let normalized = normalized_source(source, dialect);
        if self.execution_mode == FormulaExecutionMode::Plan {
            return self.eval_plan_with_dialect(&normalized, dialect, ctx);
        }
        if self.execution_mode == FormulaExecutionMode::Auto {
            if let Some(plan) = self.auto_plan_for(&normalized, dialect, ctx, &ParamValues::new()) {
                return Ok(Array1::from_vec(
                    self.eval_plan_channels_with_plan(plan, ctx, false)?
                        .into_primary(),
                ));
            }
        }
        match dialect {
            FormulaDialect::AlphaTA
            | FormulaDialect::TongDaXin
            | FormulaDialect::TongHuaShun
            | FormulaDialect::EastMoney => {
                if self.execution_mode == FormulaExecutionMode::Auto {
                    self.eval_tree(&normalized, ctx)
                } else {
                    self.eval(&normalized, ctx)
                }
            }
            FormulaDialect::Pine => {
                let ast = self.pine_ast(&normalized)?;
                self.eval_ast(ast.as_ref(), ctx)
            }
        }
    }

    /// Return the shared parsed Pine AST for normalized source, compiling it
    /// once per engine instead of reparsing on each Tree-mode evaluation.
    fn pine_ast(&self, normalized: &str) -> Result<Arc<AstNode>, FormulaError> {
        if let Some(ast) = self.pine_ast_cache.borrow().get(normalized).cloned() {
            return Ok(ast);
        }
        let ast = Arc::new(
            parse_formula_with_dialect(normalized, FormulaDialect::Pine)
                .map_err(FormulaError::ParseError)?,
        );
        self.pine_ast_cache
            .borrow_mut()
            .insert(normalized.to_string(), Arc::clone(&ast));
        Ok(ast)
    }

    /// Evaluate a pre-built AST directly (no string parsing).
    ///
    /// This is the integration point for alternative dialects such as Pine
    /// Script v5: callers parse with [`parse_formula_with_dialect`] (which
    /// maps Pine → AlphaTA `AstNode`) and hand the resulting node here, so
    /// Pine indicators reuse the full AlphaTA execution pipeline (bytecode,
    /// JIT, SIMD, partial-eval).
    ///
    /// Tree-only: an `AstNode` is a tree-walker artifact, so this reports
    /// [`FormulaError::BackendUnsupported`] under
    /// [`FormulaExecutionMode::Plan`] instead of silently walking the tree.
    pub fn eval_ast(
        &self,
        ast: &AstNode,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        self.require_tree_backend("eval_ast")?;
        ctx.validate_alignment()?;
        self.executor.execute(ast, ctx)
    }

    /// Partial-evaluation entry point (R-2).
    ///
    /// Attempts to evaluate the formula; on per-element runtime failures
    /// (e.g. divide-by-zero, log-of-negative, undefined variables in array
    /// positions) the result array is filled with `f64::NAN` at those
    /// indices and a vector of human-readable error messages is returned
    /// alongside the result. Callers can then decide whether to retry,
    /// patch the input, or surface the errors upstream.
    ///
    /// # Returns
    /// A `(result, errors)` tuple. `errors` is empty on a fully successful
    /// evaluation. The `result` is always a fully-shaped `Array1<f64>` —
    /// either the actual computed values, or NaN where evaluation failed.
    pub fn eval_partial(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> (Array1<f64>, Vec<String>) {
        match self.eval(source, ctx) {
            Ok(v) => (v, Vec::new()),
            Err(e) => {
                #[cfg(feature = "tracing")]
                crate::warn!(source, error = %e, "formula partial-eval failed");
                #[cfg(feature = "metrics")]
                crate::metrics::formula_error("partial");
                let err_msg = format!("{e}");
                let n = ctx.data_len;
                (Array1::from_elem(n, f64::NAN), vec![err_msg])
            }
        }
    }

    /// 多输出执行：返回所有 Output 变量及最终值
    pub fn eval_multi(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<MultiOutput, FormulaError> {
        let vars_before: std::collections::HashSet<Arc<str>> =
            ctx.variables.keys().cloned().collect();
        let final_value = self.eval(source, ctx)?;
        let mut multi = MultiOutput::new(final_value);
        for (name, value) in &ctx.variables {
            if !vars_before.contains(name) {
                multi.outputs.insert(name.to_string(), value.clone());
            }
        }
        Ok(multi)
    }

    /// Evaluate a formula with dialect-aware parsing and named outputs.
    ///
    /// This is the canonical multi-output entry point used by the operation
    /// engine and language-neutral bindings.
    pub fn eval_multi_with_dialect(
        &mut self,
        source: &str,
        dialect: FormulaDialect,
        ctx: &mut FormulaContext,
    ) -> Result<MultiOutput, FormulaError> {
        let normalized = normalized_source(source, dialect);
        match dialect {
            FormulaDialect::AlphaTA
            | FormulaDialect::TongDaXin
            | FormulaDialect::TongHuaShun
            | FormulaDialect::EastMoney => self.eval_multi(&normalized, ctx),
            FormulaDialect::Pine => {
                self.require_tree_backend("eval_multi_with_dialect(Pine)")?;
                let vars_before: std::collections::HashSet<Arc<str>> =
                    ctx.variables.keys().cloned().collect();
                let ast = self.pine_ast(&normalized)?;
                let final_value = self.eval_ast(&ast, ctx)?;
                let mut multi = MultiOutput::new(final_value);
                for (name, value) in &ctx.variables {
                    if !vars_before.contains(name) {
                        multi.outputs.insert(name.to_string(), value.clone());
                    }
                }
                Ok(multi)
            }
        }
    }

    /// Evaluate Pine with an explicit host/provider-backed
    /// `request.security` resolver. The resolver must return an already
    /// timestamp-aligned series variable; this method does not resample or
    /// infer higher-timeframe values.
    pub fn eval_multi_with_pine_security(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
        resolver: &dyn PineSecurityResolver,
    ) -> Result<MultiOutput, FormulaError> {
        self.require_tree_backend("eval_multi_with_pine_security")?;
        let normalized = normalized_source(source, FormulaDialect::Pine);
        let pine = parse_pine(&normalized)
            .map_err(|error| FormulaError::ParseError(format!("Pine parse error: {error}")))?;
        let ast = map_pine_to_alphata_with_security(&pine, Some(resolver))
            .map_err(|error| FormulaError::ParseError(format!("Pine map error: {error}")))?;
        let vars_before: std::collections::HashSet<Arc<str>> =
            ctx.variables.keys().cloned().collect();
        let final_value = self.eval_ast(&ast, ctx)?;
        let mut multi = MultiOutput::new(final_value);
        for (name, value) in &ctx.variables {
            if !vars_before.contains(name) {
                multi.outputs.insert(name.to_string(), value.clone());
            }
        }
        Ok(multi)
    }

    /// 惰性求值：通过依赖分析只计算最终输出所需的变量
    ///
    /// Tree-only: the pruning is an AST rewrite. Under
    /// [`FormulaExecutionMode::Plan`] this reports
    /// [`FormulaError::BackendUnsupported`] rather than silently walking the
    /// tree; the plan backend already drops unreachable operations itself.
    pub fn eval_lazy(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        self.require_tree_backend("eval_lazy")?;
        let formula = self.compile_shared(source)?;
        let pruned = DependencyAnalyzer::analyze_and_prune(&formula.ast);
        self.executor.execute(&pruned, ctx)
    }

    /// 增量计算：追加新 bar 后重算公式，利用编译缓存加速
    /// 返回完整结果（包含所有 bars）。
    /// 如果 ctx 已有之前的 variables，会先清除再重算。
    pub fn eval_incremental(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        ctx.variables.clear();
        ctx.output_names.clear();
        ctx.output_modifiers.clear();
        self.eval(source, ctx)
    }

    /// 并行计算：分析 AST 中的独立子表达式，并行求值无依赖的分支。
    /// 当 rayon feature 未启用时，退化为串行求值（结果一致）。
    ///
    /// Tree-only: parallelism is scheduled over the AST. Under
    /// [`FormulaExecutionMode::Plan`] this reports
    /// [`FormulaError::BackendUnsupported`] instead of silently walking the
    /// tree.
    pub fn eval_parallel(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        self.require_tree_backend("eval_parallel")?;
        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        self.execute_parallel(&ast, ctx)
    }

    fn execute_parallel(
        &self,
        ast: &crate::formula::ast::AstNode,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        use crate::formula::ast::AstNode;
        use crate::formula::result_statement_index;

        match ast {
            AstNode::Statements(stmts) => {
                // This path overwrites `last_result` once per dependency group,
                // which cannot express "the last *value-producing* statement".
                // When trailing drawing directives or level markers would change
                // the result, fall back to the serial executor — the same
                // no-rayon behaviour this path already documents.
                if result_statement_index(stmts) != stmts.len().checked_sub(1) {
                    return self.executor.execute(ast, ctx);
                }

                // Group statements into independent batches based on dependencies
                let groups = DependencyAnalyzer::group_independent_stmts(stmts);

                let mut last_result = Array1::zeros(ctx.data_len);
                for group in groups {
                    if group.len() == 1 {
                        last_result = self.executor.execute(&group[0], ctx)?;
                    } else {
                        #[cfg(feature = "rayon")]
                        {
                            use rayon::prelude::*;
                            let local_ctxs: Vec<_> =
                                (0..group.len()).map(|_| ctx.clone()).collect();
                            let results: Vec<_> = group
                                .par_iter()
                                .zip(local_ctxs.into_par_iter())
                                .map(|(stmt, mut local_ctx)| {
                                    let local_exec = FormulaExecutor::new();
                                    let result = local_exec.execute(stmt, &mut local_ctx);
                                    (
                                        result,
                                        local_ctx.variables,
                                        local_ctx.output_names,
                                        local_ctx.output_modifiers,
                                        local_ctx.draw_commands.into_inner(),
                                    )
                                })
                                .collect();
                            for (r, vars, output_names, mods, draws) in results {
                                last_result = r?;
                                ctx.variables.extend(vars);
                                for output in output_names {
                                    if !ctx.output_names.iter().any(|existing| existing == &output)
                                    {
                                        ctx.output_names.push(output);
                                    }
                                }
                                ctx.output_modifiers.extend(mods);
                                ctx.draw_commands
                                    .borrow_mut()
                                    .commands
                                    .extend(draws.commands);
                            }
                        }
                        #[cfg(not(feature = "rayon"))]
                        {
                            for stmt in &group {
                                last_result = self.executor.execute(stmt, ctx)?;
                            }
                        }
                    }
                }
                Ok(last_result)
            }
            other => self.executor.execute(other, ctx),
        }
    }

    /// 带参数执行
    pub fn eval_with_params(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
        params: &ParamValues,
    ) -> Result<Array1<f64>, FormulaError> {
        if self.execution_mode == FormulaExecutionMode::Plan {
            return Ok(Array1::from_vec(
                self.eval_plan_channels_impl(
                    source,
                    FormulaDialect::default(),
                    params,
                    ctx,
                    false,
                )?
                .into_primary(),
            ));
        }
        if self.execution_mode == FormulaExecutionMode::Auto {
            if let Some(plan) = self.auto_plan_for(source, FormulaDialect::default(), ctx, params) {
                return Ok(Array1::from_vec(
                    self.eval_plan_channels_with_plan(plan, ctx, false)?
                        .into_primary(),
                ));
            }
        }
        let formula = self.compile_shared(source)?;
        let ast_with_params = apply_params(&formula.ast, params);
        self.executor.execute(&ast_with_params, ctx)
    }

    /// 获取公式参数定义
    pub fn get_param_defs(&self, formula: &CompiledFormula) -> Result<Vec<ParamDef>, FormulaError> {
        parse_params(&formula.ast)
    }

    /// 验证参数并执行
    ///
    /// Tree-only: parameter rewriting happens on the AST before dispatch. Under
    /// [`FormulaExecutionMode::Plan`] this reports
    /// [`FormulaError::BackendUnsupported`]; use
    /// [`Self::eval_with_params`], which the plan backend does implement.
    pub fn eval_with_validation(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
        params: &ParamValues,
    ) -> Result<Array1<f64>, FormulaError> {
        self.require_tree_backend("eval_with_validation")?;
        let formula = self.compile_shared(source)?;
        let param_defs = parse_params(&formula.ast)?;
        validate_params(&param_defs, params)?;
        let ast_with_params = apply_params(&formula.ast, params);
        self.executor.execute(&ast_with_params, ctx)
    }

    /// 使用默认参数执行
    ///
    /// Tree-only: parameter rewriting happens on the AST before dispatch. Under
    /// [`FormulaExecutionMode::Plan`] this reports
    /// [`FormulaError::BackendUnsupported`]; use
    /// [`Self::eval_with_params`] with the declared defaults instead.
    pub fn eval_with_defaults(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        self.require_tree_backend("eval_with_defaults")?;
        let formula = self.compile_shared(source)?;
        let param_defs = parse_params(&formula.ast)?;
        let defaults: ParamValues = param_defs
            .iter()
            .map(|p| (p.name.clone(), p.default))
            .collect();
        let ast_with_params = apply_params(&formula.ast, &defaults);
        self.executor.execute(&ast_with_params, ctx)
    }

    /// Batch evaluation: compute multiple formulas in a single pass.
    /// Shares the same context across all formulas, reducing data traversal overhead.
    ///
    /// The fused single-graph path is a tree-walker optimization (AST fusion
    /// plus the expression optimizer), so it is skipped while the plan backend
    /// is selected: each formula is evaluated through its own plan instead of
    /// silently fusing ASTs behind the caller's back.
    pub fn eval_batch(
        &mut self,
        formulas: &[&str],
        ctx: &mut FormulaContext,
    ) -> Result<Vec<Array1<f64>>, FormulaError> {
        let analyses: Vec<_> = formulas
            .iter()
            .map(|source| self.analyze(source))
            .collect::<Result<_, _>>()?;
        let all_effect_free = analyses
            .iter()
            .all(|analysis| !analysis.has_observable_effects);
        if all_effect_free && !self.plan_selected() {
            return self.eval_batch_shared_compiled(formulas, ctx);
        }
        let mut results: Vec<Option<Array1<f64>>> = vec![None; formulas.len()];
        // A duplicated formula may only reuse an earlier result while nothing in
        // the batch has mutated the context. One effectful formula can change a
        // variable that a later duplicate reads, so a batch containing any
        // observable effect is evaluated strictly in order and nothing is
        // cached — previously every result was cached, which silently skipped
        // the effects of a repeated formula.
        let mut completed: HashMap<String, Array1<f64>> = HashMap::new();
        for (index, &source) in formulas.iter().enumerate() {
            if all_effect_free {
                if let Some(cached) = completed.get(source) {
                    results[index] = Some(cached.clone());
                    continue;
                }
            }
            let result = self.eval(source, ctx)?;
            if all_effect_free {
                completed.insert(source.to_string(), result.clone());
            }
            results[index] = Some(result);
        }
        Ok(results
            .into_iter()
            .map(|result| result.expect("every batch formula is evaluated"))
            .collect())
    }

    /// Evaluate independent formulas as one statement graph.
    ///
    /// Pure formulas are wrapped as named outputs and executed in one pooled
    /// pass.  This gives the optimizer a single graph in which common
    /// subexpressions such as `EMA(CLOSE, 20)` can be reused.  Formulas with
    /// assignments, drawing or other observable effects deliberately retain
    /// sequential semantics and use the regular batch path.
    pub fn eval_batch_shared(
        &mut self,
        formulas: &[&str],
        ctx: &mut FormulaContext,
    ) -> Result<Vec<Array1<f64>>, FormulaError> {
        self.eval_batch(formulas, ctx)
    }

    fn eval_batch_shared_compiled(
        &mut self,
        formulas: &[&str],
        ctx: &mut FormulaContext,
    ) -> Result<Vec<Array1<f64>>, FormulaError> {
        if formulas.is_empty() {
            return Ok(Vec::new());
        }
        let mut statements = Vec::with_capacity(formulas.len());
        let mut names = Vec::with_capacity(formulas.len());
        for (index, source) in formulas.iter().enumerate() {
            // This one genuinely needs ownership: the AST is moved into the
            // synthetic `Output` node below, so it cannot stay shared.
            let formula = self.compile(source)?;
            let analysis = analyze_formula(&formula.ast);
            if analysis.has_observable_effects {
                return formulas
                    .iter()
                    .map(|source| self.eval(source, ctx))
                    .collect();
            }
            let name = format!("__FINKIT_BATCH_{index}");
            names.push(name.clone());
            statements.push(AstNode::Output {
                name,
                expr: Box::new(formula.ast),
                modifier: None,
            });
        }
        let combined = FormulaOptimizer::optimize_for_execution(&AstNode::Statements(statements));
        self.executor.execute(&combined, ctx)?;
        names
            .iter()
            .map(|name| {
                ctx.variables.get(name.as_str()).cloned().ok_or_else(|| {
                    FormulaError::RuntimeError(format!("shared batch output `{name}` missing"))
                })
            })
            .collect()
    }

    /// 缓存相关方法
    ///
    /// Whether `source` is already compiled, **in a cache the active mode
    /// uses**.
    ///
    /// The engine keeps two compiled-formula caches: the AST cache the tree path
    /// uses and the plan cache the plan path uses. Reporting only the AST cache
    /// made these statistics describe a cache the engine was not consulting, so
    /// every formula looked uncached in plan mode while its plan sat in the plan
    /// cache. Which cache is live follows [`Self::execution_mode`].
    ///
    /// The plan cache is keyed by source *and* dialect *and* parameter
    /// fingerprint, but this method only receives a source, so it queries the
    /// default dialect with no parameters — the same key [`Self::eval`] uses.
    /// [`Self::plan_cache_stats`] covers the rest.
    pub fn cache_hit(&self, source: &str) -> bool {
        match self.execution_mode {
            FormulaExecutionMode::Tree => self.cache.contains(source),
            FormulaExecutionMode::Plan => self.plan_cache.borrow().contains_key(
                source,
                &FormulaPlanVariantKey::new(FormulaDialect::default(), &ParamValues::new()),
            ),
            FormulaExecutionMode::Auto => {
                self.cache.contains(source)
                    || self.plan_cache.borrow().contains_key(
                        source,
                        &FormulaPlanVariantKey::new(FormulaDialect::default(), &ParamValues::new()),
                    )
            }
        }
    }

    /// Number of compiled formulas currently held, in the active mode's cache.
    ///
    /// See [`Self::cache_hit`] for why this follows the execution mode.
    pub fn cache_size(&self) -> usize {
        match self.execution_mode {
            FormulaExecutionMode::Tree => self.cache.len(),
            FormulaExecutionMode::Plan => self.plan_cache.borrow().len(),
            FormulaExecutionMode::Auto => self.cache.len() + self.plan_cache.borrow().len(),
        }
    }

    /// Drop every compiled formula and every streaming scratch series.
    ///
    /// Clears the AST, plan, and JIT hot-plan caches regardless of the active
    /// mode. Clearing only some would let the next evaluation skip
    /// recompilation right after the caller asked for a clean slate — the whole
    /// point of calling this after registering a custom component.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.pine_ast_cache.borrow_mut().clear();
        self.streaming_ema.borrow_mut().clear();
        self.streaming_common.borrow_mut().clear();
        self.clear_plan_cache();
        self.jit_compiler.borrow_mut().reset_hot_counts();
    }

    pub fn compile_bytecode(&mut self, source: &str) -> Result<Bytecode, FormulaError> {
        if let Some(bytecode) = self.bytecode_cache.borrow().get(source).cloned() {
            return Ok(bytecode);
        }
        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        // Execution-safe pass set: `optimize` also runs statement-level DCE,
        // which would erase assignments visible in `FormulaContext::variables`
        // and shift `FormulaContext::string_table` indices. See
        // `FormulaOptimizer::optimize_for_execution`.
        let ast = FormulaOptimizer::optimize_for_execution(&ast);
        let bytecode = compile_to_bytecode(&ast, source).map_err(FormulaError::RuntimeError)?;
        self.bytecode_cache
            .borrow_mut()
            .insert(source.to_string(), bytecode.clone());
        Ok(bytecode)
    }

    /// Execute bytecode with the VM owned by this engine.
    ///
    /// Keeping the VM alive lets its stack, variable map, and hash tables
    /// retain capacity between calls instead of reallocating every time.
    pub fn execute_bytecode(
        &self,
        bytecode: &Bytecode,
        ctx: &FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let mut vm = self.bytecode_vm.borrow_mut();
        let exec_result = vm.execute(bytecode, ctx)?;
        Ok(exec_result.final_value)
    }

    /// Tree-only: the optimization is an AST rewrite, so under
    /// [`FormulaExecutionMode::Plan`] this reports
    /// [`FormulaError::BackendUnsupported`] rather than silently walking the
    /// tree.
    pub fn eval_optimized(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        self.require_tree_backend("eval_optimized")?;
        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        // The same pass set `eval` uses. It is still an AST rewrite
        // (constant folding, algebraic simplification, strength reduction, CSE)
        // -- it just does not include statement-level DCE, which is not valid for
        // a compiled execution contract.
        let optimized = FormulaOptimizer::optimize_for_execution(&ast);
        self.executor.execute(&optimized, ctx)
    }

    /// Tree-only: tracing instruments the tree-walker, so under
    /// [`FormulaExecutionMode::Plan`] this reports
    /// [`FormulaError::BackendUnsupported`] rather than silently walking the
    /// tree.
    pub fn eval_with_debug(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<(Array1<f64>, FormulaDebugger), FormulaError> {
        self.require_tree_backend("eval_with_debug")?;
        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        let mut debugger = FormulaDebugger::new();
        debugger.enable_trace();
        let result = debugger.run_with_debug(&ast, ctx, &self.executor)?;
        Ok((result, debugger))
    }

    pub fn get_template(&self, name: &str) -> Option<&FormulaTemplate> {
        self.templates.get(name)
    }

    pub fn search_templates(&self, keyword: &str) -> Vec<&FormulaTemplate> {
        self.templates.search(keyword)
    }

    pub fn eval_template(
        &mut self,
        name: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        self.require_tree_backend("eval_template")?;
        let tmpl = self
            .templates
            .get(name)
            .ok_or_else(|| FormulaError::RuntimeError(format!("Template not found: {}", name)))?;
        let source = tmpl.source.clone();
        let param_defaults: Vec<(String, f64)> = tmpl
            .parameters
            .iter()
            .map(|(n, _min, _max, default)| (n.clone(), *default))
            .collect();
        let formula = self.compile_shared(&source)?;
        let defaults: ParamValues = param_defaults.into_iter().collect();
        let ast_with_params = apply_params(&formula.ast, &defaults);
        self.executor.execute(&ast_with_params, ctx)
    }

    /// Evaluate through the **frozen** experimental bytecode path.
    ///
    /// See `formula::jit` for the freeze rules: opt-in only, never on the
    /// default path, compared against the tree path by the differential gate.
    /// Prefer [`Self::eval`] or the `plan` path for anything new.
    #[cfg(feature = "formula-jit")]
    pub fn eval_jit(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        // `compile_shared` already returns an execution-optimized AST. Re-running
        // the optimizer here applied a *second*, different pass set -- including
        // statement-level DCE -- on top of it.
        let formula = self.compile_shared(source)?;
        let bytecode =
            compile_to_bytecode(&formula.ast, source).map_err(FormulaError::RuntimeError)?;
        let mut jit = self.jit_compiler.borrow_mut();
        let optimized = jit.compile_cached(bytecode);
        jit.execute(&optimized, ctx).map(|r| r.final_value)
    }

    /// **Frozen** compatibility alias — this is `eval`, verbatim.
    ///
    /// There is no SIMD formula-evaluation path: the body delegates to
    /// [`Self::eval`]. It exists because four language bindings export
    /// `formula_eval_simd`, and removing it would be a cross-language breaking
    /// change. SIMD is real at the *kernel* level (`math::simd_kernels`, reached
    /// from the normal path) — not here.
    ///
    /// Freeze rule: keep it an exact alias. If it ever needs different results
    /// from `eval`, that is a new feature and belongs in a new entry point.
    #[cfg(feature = "formula-simd")]
    pub fn eval_simd(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        self.eval(source, ctx)
    }

    /// Execute an already compiled formula through the persistent pooled path.
    pub fn execute_zero_copy_cached(
        &self,
        formula: &CompiledFormula,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        self.executor.execute_zero_copy_cached(&formula.ast, ctx)
    }

    pub fn eval_zero_copy(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        self.require_tree_backend("eval_zero_copy")?;
        let formula = self.compile_shared(source)?;
        self.executor.execute_zero_copy(&formula.ast, ctx)
    }

    /// 使用 VarNameCache 的零拷贝执行路径，避免重复创建 `Arc<str>`
    pub fn eval_zero_copy_cached(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        self.require_tree_backend("eval_zero_copy_cached")?;
        let formula = self.compile_shared(source)?;
        self.executor.execute_zero_copy_cached(&formula.ast, ctx)
    }

    pub fn eval_zero_alloc(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        self.require_tree_backend("eval_zero_alloc")?;
        let formula = self.compile_shared(source)?;
        let val = self.executor.execute_val(&formula.ast, ctx)?;
        Ok(val.to_array(ctx.data_len))
    }

    /// Compile into the **frozen** experimental bytecode representation.
    ///
    /// See [`Self::eval_jit`] and `formula::jit`; opt-in only, no new callers.
    #[cfg(feature = "formula-jit")]
    pub fn compile_jit(&mut self, source: &str) -> Result<OptimizedBytecode, FormulaError> {
        // Same reasoning as `eval_jit`: the AST that comes back from
        // `compile_shared` is already optimized for execution.
        let formula = self.compile_shared(source)?;
        let bytecode =
            compile_to_bytecode(&formula.ast, source).map_err(FormulaError::RuntimeError)?;
        let mut jit = self.jit_compiler.borrow_mut();
        Ok(jit.compile_cached(bytecode))
    }

    /// Execute an already-compiled **frozen** bytecode program.
    ///
    /// See [`Self::eval_jit`] and `formula::jit`; opt-in only, no new callers.
    #[cfg(feature = "formula-jit")]
    pub fn execute_jit(
        &self,
        optimized: &OptimizedBytecode,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let jit = self.jit_compiler.borrow();
        jit.execute(optimized, ctx).map(|r| r.final_value)
    }
}

/// 公式执行结果
pub struct FormulaResult {
    /// 输出变量及其值
    pub outputs: HashMap<String, Array1<f64>>,
    /// 最后一个表达式的值
    pub final_value: Array1<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx(len: usize) -> FormulaContext {
        let open = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.1).collect());
        let high = Array1::from_vec((0..len).map(|i| 11.0 + i as f64 * 0.2).collect());
        let low = Array1::from_vec((0..len).map(|i| 9.0 + i as f64 * 0.1).collect());
        let close = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.15).collect());
        let volume = Array1::from_vec((0..len).map(|i| 1000.0 + i as f64 * 10.0).collect());
        FormulaContext::new(open, high, low, close, volume, None)
    }

    #[test]
    fn simple_atr_plan_fastpath_matches_tree_and_falls_back_for_nan() {
        for (period, insert_nan) in [(14, false), (14, true), (200, false)] {
            let mut plan_context = make_ctx(128);
            let mut tree_context = make_ctx(128);
            if insert_nan {
                plan_context.high[47] = f64::NAN;
                tree_context.high[47] = f64::NAN;
            }
            let formula = format!("ATR(HIGH,LOW,CLOSE,{period})");

            let mut plan = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
            let mut tree = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Tree);
            let plan_output = plan
                .eval_with_dialect(&formula, FormulaDialect::TongDaXin, &mut plan_context)
                .unwrap();
            let tree_output = tree
                .eval_with_dialect(&formula, FormulaDialect::TongDaXin, &mut tree_context)
                .unwrap();

            assert_eq!(plan_output.len(), tree_output.len());
            for (plan_value, tree_value) in plan_output.iter().zip(tree_output.iter()) {
                if plan_value.is_nan() && tree_value.is_nan() {
                    continue;
                }
                assert_eq!(plan_value.to_bits(), tree_value.to_bits());
            }
        }
    }

    #[test]
    fn simple_ma_plan_fastpath_matches_tree() {
        let formula = "MA(CLOSE, 20)";
        let mut plan_context = make_ctx(250);
        let mut tree_context = make_ctx(250);
        let mut plan = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
        let mut tree = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Tree);

        let plan_output = plan
            .eval_with_dialect(formula, FormulaDialect::TongDaXin, &mut plan_context)
            .unwrap();
        let tree_output = tree
            .eval_with_dialect(formula, FormulaDialect::TongDaXin, &mut tree_context)
            .unwrap();

        assert_eq!(plan_output.len(), tree_output.len());
        for (plan_value, tree_value) in plan_output.iter().zip(tree_output.iter()) {
            assert!(
                (plan_value.is_nan() && tree_value.is_nan())
                    || plan_value.to_bits() == tree_value.to_bits(),
                "MA mismatch: {plan_value} != {tree_value}"
            );
        }
    }

    #[test]
    fn aroon_plan_single_leg_kernels_match_tree_for_deque_periods() {
        let formula = "AU:=AROON(HIGH,LOW,14);AD:=AROON_DOWN(HIGH,LOW,14);";
        let mut plan_context = make_ctx(5000);
        let mut tree_context = make_ctx(5000);
        let mut plan = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
        let mut tree = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Tree);
        let plan_result = plan
            .eval_with_dialect(formula, FormulaDialect::TongDaXin, &mut plan_context)
            .unwrap();
        let tree_result = tree
            .eval_with_dialect(formula, FormulaDialect::TongDaXin, &mut tree_context)
            .unwrap();

        assert_eq!(plan_result.len(), tree_result.len());
        for (plan_value, tree_value) in plan_result.iter().zip(tree_result.iter()) {
            assert!(
                (plan_value.is_nan() && tree_value.is_nan())
                    || plan_value.to_bits() == tree_value.to_bits(),
                "Aroon mismatch: {plan_value} != {tree_value}"
            );
        }
        for name in ["AU", "AD"] {
            let plan_values = plan_context.variables.get(name).unwrap();
            let tree_values = tree_context.variables.get(name).unwrap();
            for (plan_value, tree_value) in plan_values.iter().zip(tree_values.iter()) {
                assert!(
                    (plan_value.is_nan() && tree_value.is_nan())
                        || plan_value.to_bits() == tree_value.to_bits(),
                    "Aroon {name} mismatch: {plan_value} != {tree_value}"
                );
            }
        }
    }

    #[test]
    fn auto_mode_routes_long_formulas_by_measured_backend_cost() {
        assert_eq!(FormulaExecutionMode::default(), FormulaExecutionMode::Auto);
        assert_eq!(FormulaExecutionMode::Auto.as_str(), "auto");
        assert_eq!(
            FormulaExecutionMode::from_name("AUTO"),
            Some(FormulaExecutionMode::Auto)
        );

        let formula = "MA(CLOSE, 20)";
        let mut short_context = make_ctx(250);
        let mut short_auto = FormulaEngine::new();
        let short_result = short_auto
            .eval_with_dialect(formula, FormulaDialect::TongDaXin, &mut short_context)
            .unwrap();
        assert_eq!(short_result.len(), 250);
        assert_eq!(short_auto.plan_cache_size(), 0);
        assert_eq!(short_auto.cache_size(), 1);

        let mut long_auto_context = make_ctx(5000);
        let mut tree_context = make_ctx(5000);
        let mut long_auto = FormulaEngine::new();
        let mut tree = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Tree);
        let first_auto_result = long_auto
            .eval_with_dialect(formula, FormulaDialect::TongDaXin, &mut long_auto_context)
            .unwrap();
        assert_eq!(long_auto.plan_cache_size(), 0);
        assert_eq!(first_auto_result.len(), 5000);
        let auto_result = long_auto
            .eval_with_dialect(formula, FormulaDialect::TongDaXin, &mut long_auto_context)
            .unwrap();
        let tree_result = tree
            .eval_with_dialect(formula, FormulaDialect::TongDaXin, &mut tree_context)
            .unwrap();

        assert_eq!(long_auto.plan_cache_size(), 1);
        assert_eq!(auto_result.len(), tree_result.len());
        for (auto_value, tree_value) in auto_result.iter().zip(tree_result.iter()) {
            assert!(
                (auto_value.is_nan() && tree_value.is_nan())
                    || auto_value.to_bits() == tree_value.to_bits(),
                "Auto mismatch: {auto_value} != {tree_value}"
            );
        }
        long_auto.clear_cache();
        assert_eq!(long_auto.plan_cache_size(), 0);
        long_auto
            .eval_with_dialect(formula, FormulaDialect::TongDaXin, &mut long_auto_context)
            .unwrap();
        assert_eq!(long_auto.plan_cache_size(), 0);

        let pine_source = include_str!("../../../tests/pine_corpus/trix.pine");
        let mut pine_context = make_ctx(2400);
        let mut pine_auto = FormulaEngine::new();
        pine_auto
            .eval_with_dialect(pine_source, FormulaDialect::Pine, &mut pine_context)
            .unwrap();
        assert_eq!(pine_auto.plan_cache_size(), 0);
        pine_auto
            .eval_with_dialect(pine_source, FormulaDialect::Pine, &mut pine_context)
            .unwrap();
        assert_eq!(pine_auto.plan_cache_size(), 0);
        assert!(!pine_auto.auto_uses_plan(
            pine_source,
            FormulaDialect::Pine,
            &pine_context,
            &ParamValues::new(),
        ));

        let pine_obv = include_str!("../../../tests/pine_corpus/obv.pine");
        let mut obv_context = make_ctx(2400);
        let mut obv_auto = FormulaEngine::new();
        obv_auto
            .eval_with_dialect(pine_obv, FormulaDialect::Pine, &mut obv_context)
            .unwrap();
        let normalized_obv = normalized_source(pine_obv, FormulaDialect::Pine);
        assert!(!obv_auto.auto_uses_plan(
            &normalized_obv,
            FormulaDialect::Pine,
            &obv_context,
            &ParamValues::new(),
        ));

        for formula in [
            "ZLEMA20:=ZLEMA(CLOSE,20);",
            "CPK:=COPPOCK(CLOSE,10,14,11);",
            "AU:=AROON(HIGH,LOW,14);AD:=AROON_DOWN(HIGH,LOW,14);",
            "KCM:=KC_MID(HIGH,LOW,CLOSE,20,10,2);KCU:=KC(HIGH,LOW,CLOSE,20,10,2);KCL:=KC_LOWER(HIGH,LOW,CLOSE,20,10,2);",
            "周期:=PERIODTYPE();昨收:=REFDATE(CLOSE,1);今开对比:=(CLOSE-REFDATE(CLOSE,1))/REFDATE(CLOSE,1)*100;",
            "VIP:=VORTEX(HIGH,LOW,CLOSE,14);VIM:=VORTEX_MINUS(HIGH,LOW,CLOSE,14);",
            "TR:=EMA(EMA(EMA(CLOSE,12),12),12);TRIX:=(TR-REF(TR,1))/REF(TR,1)*100;TRMA:=MA(TRIX,9);",
            "NV:=NVI(CLOSE,VOLUME);PV2:=PVI(CLOSE,VOLUME);EF:=EFI(CLOSE,VOLUME,13);",
        ] {
            let mut context = make_ctx(5000);
            let mut auto = FormulaEngine::new();
            auto.eval_with_dialect(formula, FormulaDialect::TongDaXin, &mut context)
                .unwrap();
            assert!(!auto.auto_uses_plan(
                formula,
                FormulaDialect::TongDaXin,
                &context,
                &ParamValues::new(),
            ));
        }

        let pine_rsi = include_str!("../../../tests/pine_corpus/rsi.pine");
        let mut rsi_context = make_ctx(2400);
        let mut rsi_auto = FormulaEngine::new();
        rsi_auto
            .eval_with_dialect(pine_rsi, FormulaDialect::Pine, &mut rsi_context)
            .unwrap();
        let normalized_rsi = normalized_source(pine_rsi, FormulaDialect::Pine);
        assert!(!rsi_auto.auto_uses_plan(
            &normalized_rsi,
            FormulaDialect::Pine,
            &rsi_context,
            &ParamValues::new(),
        ));

        let pine_donchian = include_str!("../../../tests/pine_corpus/donchian_channels.pine");
        let mut donchian_context = make_ctx(2400);
        let mut donchian_auto = FormulaEngine::new();
        donchian_auto
            .eval_with_dialect(pine_donchian, FormulaDialect::Pine, &mut donchian_context)
            .unwrap();
        let normalized_donchian = normalized_source(pine_donchian, FormulaDialect::Pine);
        assert!(!donchian_auto.auto_uses_plan(
            &normalized_donchian,
            FormulaDialect::Pine,
            &donchian_context,
            &ParamValues::new(),
        ));
    }

    #[test]
    fn pine_trix_plan_fastpath_matches_tree_and_publishes_context() {
        let source = include_str!("../../../tests/pine_corpus/trix.pine");
        let mut plan_context = make_ctx(250);
        let mut tree_context = make_ctx(250);
        let mut plan = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
        let mut tree = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Tree);

        let plan_output = plan
            .eval_with_dialect(source, FormulaDialect::Pine, &mut plan_context)
            .unwrap();
        let tree_output = tree
            .eval_with_dialect(source, FormulaDialect::Pine, &mut tree_context)
            .unwrap();

        assert_eq!(plan_output.len(), tree_output.len());
        for (plan_value, tree_value) in plan_output.iter().zip(tree_output.iter()) {
            assert!(
                (plan_value.is_nan() && tree_value.is_nan())
                    || plan_value.to_bits() == tree_value.to_bits(),
                "TRIX mismatch: {plan_value} != {tree_value}"
            );
        }
        assert_eq!(plan_context.output_names, tree_context.output_names);
        assert_eq!(plan_context.variables.len(), tree_context.variables.len());
        for (name, plan_values) in &plan_context.variables {
            let tree_values = tree_context
                .variables
                .get(name)
                .unwrap_or_else(|| panic!("Tree context did not publish `{name}`"));
            assert_eq!(plan_values.len(), tree_values.len(), "variable `{name}`");
            for (plan_value, tree_value) in plan_values.iter().zip(tree_values.iter()) {
                assert!(
                    (plan_value.is_nan() && tree_value.is_nan())
                        || plan_value.to_bits() == tree_value.to_bits(),
                    "published variable `{name}` differs: {plan_value} != {tree_value}"
                );
            }
        }
    }

    #[test]
    fn test_engine_eval_errors_return_err_not_panic() {
        // T3 regression guard: user-reachable formula errors (syntax, unknown
        // function, arity mismatch) must surface as `Err(FormulaError)` through
        // `eval` — never as a panic that an FFI `catch_unwind` guard (A3) would
        // silently swallow into a null result with the error lost.
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let cases = [
            "SMA(CLOSE",         // unterminated syntax error
            "1 +",               // incomplete expression
            ")",                 // stray token
            "FOOBAR(CLOSE, 20)", // unknown function
            "MA(CLOSE)",         // too few arguments
        ];
        for src in cases {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                engine.eval(src, &mut ctx)
            }));
            match res {
                Ok(r) => assert!(
                    r.is_err(),
                    "bad formula '{}' should return Err, got Ok",
                    src
                ),
                Err(_) => panic!(
                    "T3 regression: bad formula '{}' PANICKED instead of returning Err",
                    src
                ),
            }
        }
    }

    #[test]
    fn test_simple_formula_dispatch_matches_general_executor() {
        for source in [
            "MA(CLOSE, 20)",
            "EMA(CLOSE, 12)",
            "RSI(CLOSE, 14)",
            "BOLLMID(CLOSE, 20)",
        ] {
            let mut fast_engine = FormulaEngine::new();
            let formula = fast_engine.compile(source).unwrap();
            let mut fast_ctx = make_ctx(128);
            let fast = fast_engine.execute(&formula, &mut fast_ctx).unwrap();

            let ast = parse_formula(source).unwrap();
            let executor = FormulaExecutor::new();
            let mut reference_ctx = make_ctx(128);
            let reference = executor.execute(&ast, &mut reference_ctx).unwrap();

            assert_eq!(fast.len(), reference.len(), "{source}");
            for (actual, expected) in fast.iter().zip(reference.iter()) {
                assert!(
                    (actual.is_nan() && expected.is_nan()) || (actual - expected).abs() < 1e-12,
                    "{source}: {actual} != {expected}"
                );
            }
        }
    }

    #[test]
    fn test_simple_formula_dispatch_writes_reusable_output() {
        let mut engine = FormulaEngine::new();
        let formula = engine.compile("RSI(CLOSE, 14)").unwrap();
        let mut ctx = make_ctx(64);
        let mut output = Array1::zeros(64);
        engine.eval_into(&formula, &mut ctx, &mut output).unwrap();

        let reference = FormulaExecutor::new();
        let ast = parse_formula("RSI(CLOSE, 14)").unwrap();
        let mut reference_ctx = make_ctx(64);
        let expected = reference.execute(&ast, &mut reference_ctx).unwrap();

        for (actual, expected) in output.iter().zip(expected.iter()) {
            assert!(
                (actual.is_nan() && expected.is_nan()) || (actual - expected).abs() < 1e-12,
                "{actual} != {expected}"
            );
        }
    }

    #[test]
    fn test_simple_formula_dispatch_falls_back_for_nested_expression() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(32);
        let result = engine.eval("MA(CLOSE, 20) + 1", &mut ctx).unwrap();
        assert!(result.iter().skip(19).all(|value| value.is_finite()));
    }

    #[test]
    fn test_engine_new() {
        let engine = FormulaEngine::new();
        assert_eq!(engine.cache_size(), 0);
    }

    #[test]
    fn test_engine_with_cache_size() {
        let engine = FormulaEngine::with_cache_size(50);
        assert_eq!(engine.cache_size(), 0);
    }

    #[test]
    fn test_engine_compile_and_execute() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let formula = engine.compile("10 + 20").unwrap();
        let result = engine.execute(&formula, &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 30.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_engine_eval() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval("CLOSE + OPEN", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            assert!((result[i] - (close_val + open_val)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_engine_compile_caching() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);

        engine.eval("MA(CLOSE, 5)", &mut ctx).unwrap();
        assert!(engine.cache_hit("MA(CLOSE, 5)"));

        engine.eval("MA(CLOSE, 5)", &mut ctx).unwrap();
        assert_eq!(engine.cache_size(), 1);
    }

    #[test]
    fn test_engine_eval_with_params() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let mut params = ParamValues::new();
        params.insert("N".to_string(), 10.0);

        let result = engine
            .eval_with_params("MA(CLOSE, N)", &mut ctx, &params)
            .unwrap();
        assert_eq!(result.len(), 30);
    }

    #[test]
    fn test_engine_get_param_defs() {
        let mut engine = FormulaEngine::new();
        let source = "PARAMS: N(1, 100, 20); MA5: MA(CLOSE, N)";
        let formula = engine.compile(source).unwrap();
        let param_defs = engine.get_param_defs(&formula).unwrap();
        assert_eq!(param_defs.len(), 1);
        assert_eq!(param_defs[0].name, "N");
        assert_eq!(param_defs[0].min, 1.0);
        assert_eq!(param_defs[0].max, 100.0);
        assert_eq!(param_defs[0].default, 20.0);
    }

    #[test]
    fn test_engine_eval_with_validation_valid() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let mut params = ParamValues::new();
        params.insert("N".to_string(), 50.0);

        let source = "PARAMS: N(1, 100, 20); MA5: MA(CLOSE, N)";
        let result = engine.eval_with_validation(source, &mut ctx, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_engine_eval_with_validation_invalid() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let mut params = ParamValues::new();
        params.insert("N".to_string(), 150.0);

        let source = "PARAMS: N(1, 100, 20); MA5: MA(CLOSE, N)";
        let result = engine.eval_with_validation(source, &mut ctx, &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_engine_eval_with_defaults() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);

        let source = "PARAMS: N(1, 100, 20); MA5: MA(CLOSE, N)";
        let result = engine.eval_with_defaults(source, &mut ctx).unwrap();
        assert_eq!(result.len(), 30);
    }

    #[test]
    fn test_engine_clear_cache() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        engine.eval("CLOSE + 1", &mut ctx).unwrap();
        assert_eq!(engine.cache_size(), 1);
        engine.clear_cache();
        assert_eq!(engine.cache_size(), 0);
    }

    #[test]
    fn test_engine_eval_batch() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let formulas = vec!["MA(CLOSE, 5)", "MA(CLOSE, 10)", "CLOSE + OPEN"];
        let results = engine.eval_batch(&formulas, &mut ctx).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].len(), 30);
        assert_eq!(results[1].len(), 30);
        assert_eq!(results[2].len(), 30);
    }

    #[test]
    fn test_engine_eval_batch_shared_matches_individual_formulas() {
        let formulas = [
            "EMA(CLOSE, 5)",
            "EMA(CLOSE, 5) + MA(CLOSE, 3)",
            "RSI(CLOSE, 5)",
        ];
        let mut shared_engine = FormulaEngine::new();
        let mut shared_ctx = make_ctx(40);
        let shared = shared_engine
            .eval_batch_shared(&formulas, &mut shared_ctx)
            .unwrap();
        let mut individual_engine = FormulaEngine::new();
        let mut individual_ctx = make_ctx(40);
        let individual = formulas
            .iter()
            .map(|source| individual_engine.eval(source, &mut individual_ctx).unwrap())
            .collect::<Vec<_>>();
        for (actual, expected) in shared.iter().zip(individual.iter()) {
            assert!(actual
                .iter()
                .zip(expected.iter())
                .all(|(a, b)| { (a - b).abs() < 1e-12 || (a.is_nan() && b.is_nan()) }));
        }
    }

    #[test]
    fn test_engine_invalid_formula() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval("CLOSE +", &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_compile_bytecode_simple() {
        let mut engine = FormulaEngine::new();
        let bytecode = engine.compile_bytecode("10 + 20").unwrap();
        assert!(!bytecode.instructions.is_empty());
    }

    #[test]
    fn test_compile_bytecode_with_function() {
        let mut engine = FormulaEngine::new();
        let bytecode = engine.compile_bytecode("MA(CLOSE, 5)").unwrap();
        assert!(bytecode.instructions.len() >= 2);
    }

    #[test]
    fn test_execute_bytecode_constant() {
        let mut engine = FormulaEngine::new();
        let bytecode = engine.compile_bytecode("42").unwrap();
        let ctx = make_ctx(5);
        let result = engine.execute_bytecode(&bytecode, &ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 42.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_bytecode_expression() {
        let mut engine = FormulaEngine::new();
        let bytecode = engine.compile_bytecode("CLOSE + OPEN").unwrap();
        let ctx = make_ctx(5);
        let result = engine.execute_bytecode(&bytecode, &ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            assert!((result[i] - (close_val + open_val)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_optimized_constant_folding() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_optimized("1 + 2 + 3", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 6.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_optimized_with_variables() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let result = engine.eval_optimized("MA(CLOSE, 5)", &mut ctx).unwrap();
        assert_eq!(result.len(), 30);
    }

    #[test]
    fn test_eval_with_debug_basic() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let (result, debugger) = engine.eval_with_debug("CLOSE + 1", &mut ctx).unwrap();
        assert_eq!(result.len(), 5);
        assert!(!debugger.get_events().is_empty());
    }

    #[test]
    fn test_eval_with_debug_complex() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let (result, debugger) = engine
            .eval_with_debug("MA5 := MA(CLOSE, 5); MA5 > 10", &mut ctx)
            .unwrap();
        assert_eq!(result.len(), 30);
        let events = debugger.get_events();
        assert!(events.len() > 2);
    }

    #[test]
    fn test_get_template_exists() {
        let engine = FormulaEngine::new();
        let tmpl = engine.get_template("ma_cross");
        assert!(tmpl.is_some());
        assert_eq!(tmpl.unwrap().name, "均线金叉死叉");
    }

    #[test]
    fn test_get_template_not_exists() {
        let engine = FormulaEngine::new();
        assert!(engine.get_template("nonexistent_template").is_none());
    }

    #[test]
    fn test_search_templates_by_keyword() {
        let engine = FormulaEngine::new();
        let results = engine.search_templates("MACD");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_search_templates_empty() {
        let engine = FormulaEngine::new();
        let results = engine.search_templates("zzzzznotfound");
        assert!(results.is_empty());
    }

    #[test]
    fn test_eval_template_existing() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let result = engine.eval_template("kdj_overbought", &mut ctx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 30);
    }

    #[test]
    fn test_eval_template_nonexistent() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_template("nonexistent", &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_template_ma_cross() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(50);
        let result = engine.eval_template("ma_cross", &mut ctx).unwrap();
        assert_eq!(result.len(), 50);
    }

    #[test]
    fn test_bytecode_roundtrip_compile_execute() {
        let mut engine = FormulaEngine::new();
        let ctx = make_ctx(30);

        let bytecode = engine.compile_bytecode("CLOSE > OPEN").unwrap();
        let result = engine.execute_bytecode(&bytecode, &ctx).unwrap();

        for i in 0..30 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            let expected = if close_val > open_val { 1.0 } else { 0.0 };
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_optimized_matches_eval() {
        let mut engine = FormulaEngine::new();
        let mut ctx1 = make_ctx(30);
        let mut ctx2 = make_ctx(30);

        let result1 = engine.eval("MA(CLOSE, 10)", &mut ctx1).unwrap();
        let result2 = engine.eval_optimized("MA(CLOSE, 10)", &mut ctx2).unwrap();

        for i in 0..30 {
            if result1[i].is_nan() {
                assert!(result2[i].is_nan());
            } else {
                assert!((result1[i] - result2[i]).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_compile_bytecode_complex_formula() {
        let mut engine = FormulaEngine::new();
        let source = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); MA5 > MA10";
        let bytecode = engine.compile_bytecode(source).unwrap();
        assert!(bytecode.instructions.len() > 10);
    }

    #[test]
    fn test_execute_bytecode_after_serialize() {
        let mut engine = FormulaEngine::new();
        let bytecode = engine.compile_bytecode("CLOSE * 2").unwrap();
        let data = bytecode.serialize();
        let restored = Bytecode::deserialize(&data).expect("Deserialize failed");

        let ctx = make_ctx(5);
        let result = engine.execute_bytecode(&restored, &ctx).unwrap();
        for i in 0..5 {
            let expected = (10.0 + i as f64 * 0.15) * 2.0;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_jit_constant() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_jit("10 + 20", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 30.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_jit_with_variables() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_jit("CLOSE + OPEN", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            assert!((result[i] - (close_val + open_val)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_jit_with_function() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let result = engine.eval_jit("MA(CLOSE, 5)", &mut ctx).unwrap();
        assert_eq!(result.len(), 30);
    }

    #[test]
    fn test_eval_jit_invalid_formula() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_jit("CLOSE +", &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_jit_matches_eval() {
        let mut engine = FormulaEngine::new();
        let mut ctx1 = make_ctx(30);
        let mut ctx2 = make_ctx(30);
        let result1 = engine.eval("MA(CLOSE, 10)", &mut ctx1).unwrap();
        let result2 = engine.eval_jit("MA(CLOSE, 10)", &mut ctx2).unwrap();
        for i in 0..30 {
            if result1[i].is_nan() {
                assert!(result2[i].is_nan());
            } else {
                assert!((result1[i] - result2[i]).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_eval_simd_constant() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_simd("10 + 20", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 30.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_simd_with_variables() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_simd("CLOSE * 2", &mut ctx).unwrap();
        for i in 0..5 {
            let expected = (10.0 + i as f64 * 0.15) * 2.0;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_simd_matches_eval() {
        let mut engine = FormulaEngine::new();
        let mut ctx1 = make_ctx(30);
        let mut ctx2 = make_ctx(30);
        let result1 = engine.eval("MA(CLOSE, 5)", &mut ctx1).unwrap();
        let result2 = engine.eval_simd("MA(CLOSE, 5)", &mut ctx2).unwrap();
        for i in 0..30 {
            if result1[i].is_nan() {
                assert!(result2[i].is_nan());
            } else {
                assert!((result1[i] - result2[i]).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_eval_zero_copy_constant() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_zero_copy("42", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 42.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_zero_copy_with_variables() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_zero_copy("CLOSE + OPEN", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            assert!((result[i] - (close_val + open_val)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_zero_copy_matches_eval() {
        let mut engine = FormulaEngine::new();
        let mut ctx1 = make_ctx(30);
        let mut ctx2 = make_ctx(30);
        let result1 = engine.eval("MA(CLOSE, 5)", &mut ctx1).unwrap();
        let result2 = engine.eval_zero_copy("MA(CLOSE, 5)", &mut ctx2).unwrap();
        for i in 0..30 {
            if result1[i].is_nan() {
                assert!(result2[i].is_nan());
            } else {
                assert!((result1[i] - result2[i]).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_compile_jit_simple() {
        let mut engine = FormulaEngine::new();
        let optimized = engine.compile_jit("10 + 20").unwrap();
        assert!(optimized.buffer_size() >= 2);
        assert!(!optimized.is_hot());
        assert_eq!(optimized.source(), "10 + 20");
    }

    #[test]
    fn test_compile_jit_with_function() {
        let mut engine = FormulaEngine::new();
        let optimized = engine.compile_jit("MA(CLOSE, 5)").unwrap();
        assert!(optimized.cached_call_count() > 0);
    }

    #[test]
    fn test_compile_jit_invalid_formula() {
        let mut engine = FormulaEngine::new();
        let result = engine.compile_jit("CLOSE +");
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_jit_constant() {
        let mut engine = FormulaEngine::new();
        let optimized = engine.compile_jit("42").unwrap();
        let mut ctx = make_ctx(5);
        let result = engine.execute_jit(&optimized, &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 42.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_jit_expression() {
        let mut engine = FormulaEngine::new();
        let optimized = engine.compile_jit("CLOSE + OPEN").unwrap();
        let mut ctx = make_ctx(5);
        let result = engine.execute_jit(&optimized, &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            assert!((result[i] - (close_val + open_val)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_compile_jit_then_execute_jit() {
        let mut engine = FormulaEngine::new();
        let optimized = engine.compile_jit("MA(CLOSE, 5)").unwrap();
        let mut ctx = make_ctx(30);
        let result = engine.execute_jit(&optimized, &mut ctx).unwrap();
        assert_eq!(result.len(), 30);
    }

    #[test]
    fn test_compile_jit_reuse_multiple_executions() {
        let mut engine = FormulaEngine::new();
        let optimized = engine.compile_jit("CLOSE * 2").unwrap();

        let mut ctx1 = make_ctx(5);
        let result1 = engine.execute_jit(&optimized, &mut ctx1).unwrap();

        let mut ctx2 = make_ctx(10);
        let result2 = engine.execute_jit(&optimized, &mut ctx2).unwrap();

        for i in 0..5 {
            let expected = (10.0 + i as f64 * 0.15) * 2.0;
            assert!((result1[i] - expected).abs() < 1e-10);
        }
        for i in 0..10 {
            let expected = (10.0 + i as f64 * 0.15) * 2.0;
            assert!((result2[i] - expected).abs() < 1e-10);
        }
    }
    #[test]
    fn test_eval_range_and_last_match_full_evaluation() {
        let mut engine = FormulaEngine::new();
        let formula = engine.compile("MA(CLOSE, 5)").unwrap();
        let mut full_ctx = make_ctx(40);
        let full = engine.execute(&formula, &mut full_ctx).unwrap();

        let range_ctx = make_ctx(40);
        let range = engine.eval_range(&formula, &range_ctx, 10, 25).unwrap();
        assert_eq!(range.len(), 15);
        for (offset, value) in range.iter().enumerate() {
            assert!((*value - full[10 + offset]).abs() < 1e-12);
        }

        let last_ctx = make_ctx(40);
        let last = engine.eval_last(&formula, &last_ctx).unwrap();
        assert!((last - full[39]).abs() < 1e-12);
    }

    #[test]
    fn test_streaming_ema_eval_last_matches_append_full_recompute() {
        let mut engine = FormulaEngine::new();
        let formula = engine.compile("EMA(CLOSE, 5)").unwrap();
        let mut ctx = make_ctx(20);
        let first = engine.eval_last(&formula, &ctx).unwrap();
        let expected_first = crate::math::moving_avg::ema(&ctx.close, 5).unwrap();
        assert!((first - expected_first[19]).abs() < 1e-12);

        ctx.append_bar(12.0, 13.0, 11.0, 13.25, 2000.0);
        let streamed = engine.eval_last(&formula, &ctx).unwrap();
        let expected = crate::math::moving_avg::ema(&ctx.close, 5).unwrap();
        assert!((streamed - expected[20]).abs() < 1e-12);
        assert!((engine.eval_last(&formula, &ctx).unwrap() - streamed).abs() < 1e-12);
    }

    #[test]
    fn test_streaming_common_formula_paths_match_batch() {
        for source in ["MA(CLOSE, 5)", "RSI(CLOSE, 5)", "ATR(HIGH, LOW, CLOSE, 5)"] {
            let mut engine = FormulaEngine::new();
            let formula = engine.compile(source).unwrap();
            let mut ctx = make_ctx(20);
            let _first = engine.eval_last(&formula, &ctx).unwrap();

            ctx.append_bar(12.0, 13.0, 11.0, 13.25, 2000.0);
            let streamed = engine.eval_last(&formula, &ctx).unwrap();

            let mut expected_ctx = make_ctx(21);
            expected_ctx.close[20] = 13.25;
            expected_ctx.high[20] = 13.0;
            expected_ctx.low[20] = 11.0;
            expected_ctx.open[20] = 12.0;
            expected_ctx.volume[20] = 2000.0;
            let expected = FormulaEngine::new()
                .eval(source, &mut expected_ctx)
                .unwrap()[20];
            if expected.is_nan() {
                assert!(streamed.is_nan(), "{source} streamed value should be NaN");
            } else {
                assert!(
                    (streamed - expected).abs() < 1e-10,
                    "{source}: streamed={streamed} expected={expected}"
                );
            }
        }
    }

    #[test]
    fn test_formula_analysis_is_available_from_engine() {
        let mut engine = FormulaEngine::new();
        let report = engine.analyze("MA5:=MA(CLOSE,5); MA5 + OPEN").unwrap();
        assert_eq!(report.required_lookback, Some(4));
        assert!(report.input_variables.contains(&"CLOSE".to_string()));
        assert!(report.input_variables.contains(&"OPEN".to_string()));
        assert!(report.assigned_variables.contains(&"MA5".to_string()));
    }

    #[test]
    fn test_formula_metadata_is_stable_for_bindings() {
        let mut engine = FormulaEngine::new();
        let metadata = engine.metadata("MA5:=MA(CLOSE,5); MA5", 20).unwrap();
        assert_eq!(metadata.schema_version, "finkit.formula-series.v1");
        assert_eq!(metadata.length, 20);
        assert_eq!(metadata.output_names, vec!["MA5", "__result__"]);
        assert_eq!(metadata.valid_start, Some(4));
        assert_eq!(metadata.null_policy, "nan");
    }

    #[test]
    fn test_borrowed_slice_fast_path_matches_owned_context() {
        let ctx = make_ctx(32);
        let formula = FormulaEngine::new();
        let compiled = {
            let mut compiler = FormulaEngine::new();
            compiler.compile("MA(CLOSE, 5)").unwrap()
        };
        let borrowed = formula
            .eval_zero_copy_inputs(
                &compiled,
                &ctx.open,
                &ctx.high,
                &ctx.low,
                &ctx.close,
                &ctx.volume,
                None,
            )
            .unwrap();
        let mut owned_ctx = make_ctx(32);
        let mut engine = FormulaEngine::new();
        let owned_formula = engine.compile("MA(CLOSE, 5)").unwrap();
        let owned = engine.execute(&owned_formula, &mut owned_ctx).unwrap();
        assert_eq!(borrowed.len(), owned.len());
        for (a, b) in borrowed.iter().zip(owned.iter()) {
            assert!((a - b).abs() < 1e-12 || (a.is_nan() && b.is_nan()));
        }
    }

    #[test]
    fn custom_components_are_compiled_and_cached_as_canonical_formulas() {
        let mut engine = FormulaEngine::new();
        engine
            .register_custom_formula("ZMA", &["X", "N"], "MA(X, N) + EMA(X, N)")
            .unwrap();
        let mut ctx = make_ctx(32);
        let composed = engine.eval("zma(CLOSE, 5)", &mut ctx).unwrap();

        let mut baseline_ctx = make_ctx(32);
        let baseline = FormulaEngine::new()
            .eval("MA(CLOSE, 5) + EMA(CLOSE, 5)", &mut baseline_ctx)
            .unwrap();
        for (actual, expected) in composed.iter().zip(baseline.iter()) {
            assert!((actual - expected).abs() < 1e-12 || (actual.is_nan() && expected.is_nan()));
        }
        assert_eq!(engine.custom_formula_names(), vec!["ZMA".to_string()]);
        assert!(engine.unregister_custom_formula("zma").unwrap());
    }

    #[test]
    fn test_borrowed_slice_path_supports_complex_formula() {
        let ctx = make_ctx(32);
        let mut compiler = FormulaEngine::new();
        let compiled = compiler.compile("MA(CLOSE, 5) + 1").unwrap();
        let borrowed = FormulaEngine::new()
            .eval_zero_copy_inputs(
                &compiled,
                &ctx.open,
                &ctx.high,
                &ctx.low,
                &ctx.close,
                &ctx.volume,
                None,
            )
            .unwrap();

        let mut owned_ctx = make_ctx(32);
        let mut engine = FormulaEngine::new();
        let owned_formula = engine.compile("MA(CLOSE, 5) + 1").unwrap();
        let owned = engine.execute(&owned_formula, &mut owned_ctx).unwrap();
        for (a, b) in borrowed.iter().zip(owned.iter()) {
            assert!((a - b).abs() < 1e-12 || (a.is_nan() && b.is_nan()));
        }
    }

    #[test]
    fn test_borrowed_range_matches_full_history() {
        let mut compiler = FormulaEngine::new();
        let compiled = compiler.compile("MA(CLOSE, 5) + EMA(CLOSE, 3)").unwrap();
        let ctx = make_ctx(64);
        let full = compiler.execute(&compiled, &mut ctx.clone()).unwrap();
        let range = compiler
            .eval_range_zero_copy_inputs(
                &compiled,
                &ctx.open,
                &ctx.high,
                &ctx.low,
                &ctx.close,
                &ctx.volume,
                17,
                41,
                None,
            )
            .unwrap();
        assert_eq!(range.len(), 24);
        for (offset, actual) in range.iter().enumerate() {
            let expected = full[17 + offset];
            assert!((actual - expected).abs() < 1e-12 || (actual.is_nan() && expected.is_nan()));
        }
    }
}

#[cfg(test)]
mod pr14_compute_ir_production_tests {
    use super::*;

    #[test]
    fn compile_populates_semantic_compute_plan_before_execution() {
        let mut engine = FormulaEngine::new();
        let compiled = engine.compile("X:=MA(CLOSE,5);X").unwrap();
        let cache = engine.semantic_plan_cache.borrow();
        let plan = cache.get(&compiled.source).expect("semantic plan cached");
        assert!(!plan.plan().is_empty());
        assert!(plan.plan().has_observable_effects());
    }

    #[test]
    fn pure_formula_plan_remains_incremental_candidate() {
        let mut engine = FormulaEngine::new();
        let compiled = engine.compile("MA(CLOSE,5)").unwrap();
        let cache = engine.semantic_plan_cache.borrow();
        let plan = cache.get(&compiled.source).expect("semantic plan cached");
        assert!(!plan.plan().is_empty());
        assert!(!plan.plan().has_observable_effects());
    }
}

#[cfg(test)]
mod plan_cache_tests {
    use super::*;
    use crate::execution_plan::{ParameterSlot, ParameterValue};

    fn params(pairs: &[(&str, f64)]) -> ParamValues {
        pairs
            .iter()
            .map(|(name, value)| ((*name).to_string(), *value))
            .collect()
    }

    /// Every scalar the plan bound, as `f64`, in slot order.
    ///
    /// Integer parameters are stored as `Usize`, so decode both representations
    /// rather than assuming every literal was bound as raw bits.
    fn bound_parameters(plan: &FormulaHotPlan) -> Vec<f64> {
        let arena = plan.hot().parameter_arena();
        (0..arena.len())
            .filter_map(|slot| arena.get(ParameterSlot(slot)))
            .map(|value| match value {
                ParameterValue::F64Bits(bits) => f64::from_bits(bits),
                ParameterValue::Usize(value) => value as f64,
            })
            .collect()
    }

    #[test]
    fn a_repeated_compile_returns_the_cached_plan() {
        let engine = FormulaEngine::new();

        engine
            .compile_plan_default("MA(CLOSE,5)")
            .expect("plan compiles");
        engine
            .compile_plan_default("MA(CLOSE,5)")
            .expect("plan compiles");

        assert_eq!(engine.plan_cache_size(), 1);
        assert_eq!(
            engine.plan_cache_stats(),
            FormulaPlanCacheStats { hits: 1, misses: 1 }
        );
    }

    #[test]
    fn parameter_values_are_part_of_the_plan_identity() {
        let engine = FormulaEngine::new();

        let short = engine
            .compile_plan(
                "MA(CLOSE,N)",
                FormulaDialect::AlphaTA,
                &params(&[("N", 14.0)]),
            )
            .expect("plan compiles");
        let long = engine
            .compile_plan(
                "MA(CLOSE,N)",
                FormulaDialect::AlphaTA,
                &params(&[("N", 20.0)]),
            )
            .expect("plan compiles");

        // Two entries, not one reused plan: the parameter is part of the key.
        assert_eq!(engine.plan_cache_size(), 2);
        assert_eq!(
            engine.plan_cache_stats(),
            FormulaPlanCacheStats { hits: 0, misses: 2 }
        );
        // And the plans really differ, rather than being two keys over one plan.
        assert_eq!(bound_parameters(&short), vec![14.0]);
        assert_eq!(bound_parameters(&long), vec![20.0]);
    }

    #[test]
    fn parameter_insertion_order_does_not_split_the_cache() {
        let engine = FormulaEngine::new();
        let source = "MA(CLOSE,A)+MA(CLOSE,B)";

        engine
            .compile_plan(
                source,
                FormulaDialect::AlphaTA,
                &params(&[("A", 3.0), ("B", 5.0)]),
            )
            .expect("plan compiles");
        engine
            .compile_plan(
                source,
                FormulaDialect::AlphaTA,
                &params(&[("B", 5.0), ("A", 3.0)]),
            )
            .expect("plan compiles");

        // `ParamValues` is a `HashMap`, whose iteration order is unspecified, so
        // without a sorted fingerprint these two identical requests would miss
        // twice and compile the same plan twice.
        assert_eq!(engine.plan_cache_size(), 1);
        assert_eq!(
            engine.plan_cache_stats(),
            FormulaPlanCacheStats { hits: 1, misses: 1 }
        );
    }

    #[test]
    fn the_dialect_is_part_of_the_plan_identity() {
        let engine = FormulaEngine::new();

        engine
            .compile_plan_default("MA(CLOSE,5)")
            .expect("AlphaTA plan compiles");
        engine
            .compile_plan(
                "MA(CLOSE,5)",
                FormulaDialect::TongDaXin,
                &ParamValues::new(),
            )
            .expect("TongDaXin plan compiles");

        // AlphaTA and TongDaXin currently share `parse_formula`, so this costs a
        // duplicate plan rather than preventing a wrong reuse. That is the side
        // to err on: Pine genuinely parses the same text to a different AST.
        assert_eq!(engine.plan_cache_size(), 2);
    }

    #[test]
    fn a_pine_source_compiles_through_the_pine_parser() {
        let engine = FormulaEngine::new();

        let plan = engine
            .compile_plan(
                "ta.sma(close, 5)",
                FormulaDialect::Pine,
                &ParamValues::new(),
            )
            .expect("Pine plan compiles");

        assert!(plan.hot().buffer_layout().slot_count() > 0);
    }

    #[test]
    fn an_unplannable_formula_reports_an_error_instead_of_falling_back() {
        let engine = FormulaEngine::new();

        // A statement block whose statements are all drawing directives has no
        // value-producing statement, so the plan layer cannot name a result and
        // must refuse the plan. The tree path would hand back a placeholder
        // buffer; the plan path says so instead of quietly returning something
        // else. (A single `STICKLINE` is not enough — it lowers to a numeric
        // node and only fails later, at execution.)
        let error = engine
            .compile_plan_default(
                "STICKLINE(CLOSE>OPEN,CLOSE,OPEN,3,TRUE);STICKLINE(CLOSE>OPEN,OPEN,CLOSE,3,FALSE)",
            )
            .expect_err("a draw-only block cannot be planned");

        let message = error.to_string();
        assert!(
            message.contains("formula plan compilation failed"),
            "unexpected error: {message}"
        );
        // A failed compile must not be cached, or a later call would report a
        // stale success.
        assert_eq!(engine.plan_cache_size(), 0);
    }

    #[test]
    fn a_parse_failure_is_reported_rather_than_silently_ignored() {
        let engine = FormulaEngine::new();

        assert!(engine.compile_plan_default("MA(CLOSE,").is_err());
        assert_eq!(engine.plan_cache_size(), 0);
    }

    #[test]
    fn registering_a_custom_formula_drops_plans_but_keeps_the_counters() {
        let mut engine = FormulaEngine::new();
        engine
            .compile_plan_default("MA(CLOSE,5)")
            .expect("plan compiles");
        engine.remember_auto_choice(
            "MA(CLOSE,5)",
            FormulaDialect::default(),
            AutoPlanChoice::Tree,
        );
        assert_eq!(engine.plan_cache_size(), 1);
        assert_eq!(engine.auto_plan_cache.borrow().map.len(), 1);

        engine
            .register_custom_formula("SIGNAL", &["X"], "MA(X,5)")
            .expect("custom formula registers");

        // A newly registered component can change what an existing source
        // resolves to, so the plans must go; the counters still describe this
        // engine's behaviour and are kept.
        assert_eq!(engine.plan_cache_size(), 0);
        assert_eq!(engine.auto_plan_cache.borrow().map.len(), 0);
        assert_eq!(
            engine.plan_cache_stats(),
            FormulaPlanCacheStats { hits: 0, misses: 1 }
        );
    }

    #[test]
    fn clear_plan_cache_drops_entries_and_resets_the_counters() {
        let mut engine = FormulaEngine::new();
        engine
            .compile_plan_default("MA(CLOSE,5)")
            .expect("plan compiles");
        engine.clear_plan_cache();

        assert_eq!(engine.plan_cache_size(), 0);
        assert_eq!(
            engine.plan_cache_stats(),
            FormulaPlanCacheStats { hits: 0, misses: 0 }
        );
    }

    /// V5 Batch 3 gate: the compile-artifact caches are bounded. A long-lived
    /// engine fed more distinct formulas than `ENGINE_CACHE_CAPACITY` must
    /// evict (FIFO) instead of growing without limit — the previous unbounded
    /// design was a documented memory risk (R2-2). Every cache here is a pure
    /// accelerator, so eviction costs a recompile, never a wrong number.
    #[test]
    fn hot_plan_cache_stays_bounded_and_still_recompiles_evicted_entries() {
        let mut engine = FormulaEngine::new();
        let total = ENGINE_CACHE_CAPACITY + 16;
        for i in 0..total {
            let source = format!("MA(CLOSE,2) + {}", i);
            engine
                .compile_plan_default(&source)
                .unwrap_or_else(|error| panic!("plan `{source}` failed to compile: {error}"));
        }
        let cache = engine.plan_cache.borrow();
        assert!(
            cache.len() <= ENGINE_CACHE_CAPACITY,
            "hot-plan cache grew to {} entries, bound is {ENGINE_CACHE_CAPACITY}",
            cache.len()
        );
        assert_eq!(
            cache.stats().misses as usize,
            total,
            "every distinct source had to compile at least once"
        );
        // The oldest entries were evicted, the newest are still resident.
        let newest = format!("MA(CLOSE,2) + {}", total - 1);
        assert!(
            cache.contains_key(
                &newest,
                &FormulaPlanVariantKey::new(FormulaDialect::default(), &ParamValues::new()),
            ),
            "most recently inserted plan must still be cached"
        );
        drop(cache);

        // Re-evaluating an evicted entry is a pure accelerator miss: the plan
        // recompiles and the result is still correct.
        let evicted_source = "MA(CLOSE,2) + 0";
        let ctx_len = 32;
        let open = Array1::from_vec(vec![1.0; ctx_len]);
        let high = Array1::from_vec(vec![2.0; ctx_len]);
        let low = Array1::from_vec(vec![0.5; ctx_len]);
        let close = Array1::from_vec(vec![1.5; ctx_len]);
        let volume = Array1::from_vec(vec![10.0; ctx_len]);
        let mut ctx = FormulaContext::new(open, high, low, close, volume, None);
        let result = engine.eval(evicted_source, &mut ctx).expect("recompiles");
        assert_eq!(result.len(), ctx_len);
    }

    #[test]
    fn semantic_and_bytecode_caches_stay_bounded() {
        let mut engine = FormulaEngine::new();
        let total = ENGINE_CACHE_CAPACITY + 16;
        for i in 0..total {
            let source = format!("EMA(CLOSE,2) * {}", i);
            engine
                .compile(&source)
                .unwrap_or_else(|error| panic!("compile `{source}` failed: {error}"));
        }
        assert!(
            engine.semantic_plan_cache.borrow().map.len() <= ENGINE_CACHE_CAPACITY,
            "semantic plan cache exceeded its bound"
        );
        // Bytecode is only produced by `compile_bytecode`, on demand.
        for i in 0..total {
            let source = format!("EMA(CLOSE,2) * {}", i);
            engine
                .compile_bytecode(&source)
                .unwrap_or_else(|error| panic!("bytecode `{source}` failed: {error}"));
        }
        assert!(
            engine.bytecode_cache.borrow().map.len() <= ENGINE_CACHE_CAPACITY,
            "bytecode cache exceeded its bound"
        );
    }

    #[test]
    fn bounded_fifo_map_evicts_in_insertion_order() {
        // Strict FIFO: re-inserting an existing key updates its value but does
        // not refresh its eviction position (documented policy — the capacity
        // sits far above any reuse window, so refresh behaviour is moot).
        let mut map = BoundedFifoMap::new(2);
        map.insert("a".to_string(), 1);
        map.insert("b".to_string(), 2);
        map.insert("a".to_string(), 10);
        assert_eq!(map.get("a"), Some(&10));
        map.insert("c".to_string(), 3);
        // `a` (oldest insert) was evicted; `b` survived, `c` entered.
        assert_eq!(map.get("a"), None);
        assert_eq!(map.get("b"), Some(&2));
        assert_eq!(map.get("c"), Some(&3));
        map.clear();
        assert_eq!(map.get("b"), None);
        assert_eq!(map.get("c"), None);
    }
}

#[cfg(test)]
mod plan_execution_tests {
    use super::*;

    /// Deterministic OHLCV with a trend plus a wave, long enough for the longest
    /// warm-up used below.
    #[allow(clippy::cast_precision_loss)] // bar counts here are far below 2^53
    fn ctx(len: usize) -> FormulaContext {
        let series = |base: f64, step: f64, wave: f64| {
            Array1::from_vec(
                (0..len)
                    .map(|index| {
                        let i = index as f64;
                        base + step * i + wave * (i * 0.7).sin()
                    })
                    .collect(),
            )
        };
        FormulaContext::new(
            series(10.0, 0.10, 0.30),
            series(10.5, 0.10, 0.40),
            series(9.5, 0.10, 0.20),
            series(10.0, 0.10, 0.35),
            series(1000.0, 10.0, 50.0),
            None,
        )
    }

    /// Compare two series element-wise, treating two NaNs as equal.
    ///
    /// Warm-up bars are legitimately NaN — `EMA(CLOSE, 26)` has no value for its
    /// first 25 samples — so a plain equality assertion on the vectors would fail
    /// on a correct result.
    fn assert_same_series(plan: &[f64], tree: &[f64], tolerance: f64) {
        assert_eq!(plan.len(), tree.len(), "series lengths differ");
        for (index, (left, right)) in plan.iter().zip(tree.iter()).enumerate() {
            assert!(
                (left - right).abs() <= tolerance || (left.is_nan() && right.is_nan()),
                "index {index}: plan {left} vs tree {right}"
            );
        }
    }

    /// Evaluate `source` on both paths and assert they agree element-wise.
    fn assert_plan_matches_tree(source: &str, len: usize) {
        let mut context = ctx(len);
        let plan = FormulaEngine::new()
            .eval_plan(source, &mut context)
            .unwrap_or_else(|error| panic!("plan path failed for `{source}`: {error}"));

        let mut tree_context = context.clone();
        let tree = FormulaEngine::new()
            .eval(source, &mut tree_context)
            .unwrap_or_else(|error| panic!("tree path failed for `{source}`: {error}"));

        assert_same_series(&plan.to_vec(), &tree.to_vec(), 1e-9);
    }

    #[test]
    fn a_stateless_plan_matches_the_tree_path() {
        assert_plan_matches_tree("CLOSE * 2 + 1", 60);
        assert_plan_matches_tree("MA(CLOSE, 5)", 60);
        assert_plan_matches_tree("HHV(HIGH, 10) - LLV(LOW, 10)", 60);
    }

    #[test]
    fn a_stateful_plan_matches_the_tree_path() {
        assert_plan_matches_tree("EMA(CLOSE, 12)", 80);
        assert_plan_matches_tree("RSI(CLOSE, 14)", 80);
        assert_plan_matches_tree("EMA(CLOSE, 12) + ROC(CLOSE, 10)", 80);
    }

    #[test]
    fn plan_and_tree_agree_on_warm_up_nans() {
        let mut context = ctx(40);
        let plan = FormulaEngine::new()
            .eval_plan("EMA(CLOSE, 12)", &mut context)
            .expect("plan evaluates");
        let mut tree_context = context.clone();
        let tree = FormulaEngine::new()
            .eval("EMA(CLOSE, 12)", &mut tree_context)
            .expect("tree evaluates");

        let plan = plan.to_vec();
        let tree = tree.to_vec();
        // The first 11 samples have no EMA yet, on both paths.
        assert!(
            plan[..11].iter().all(|value| value.is_nan()),
            "plan warm-up should be NaN: {:?}",
            &plan[..11]
        );
        assert!(tree[..11].iter().all(|value| value.is_nan()));
        assert!(plan[11].is_finite() && tree[11].is_finite());
        assert_same_series(&plan, &tree, 1e-9);
    }

    #[test]
    fn a_multi_channel_formula_exposes_every_named_output() {
        let engine = FormulaEngine::new();
        let mut context = ctx(80);

        let output = engine
            .eval_plan_channels(
                "DIF:EMA(CLOSE,12)-EMA(CLOSE,26);DEA:EMA(DIF,9);MACD:(DIF-DEA)*2",
                FormulaDialect::AlphaTA,
                &ParamValues::new(),
                &mut context,
            )
            .expect("multi-channel plan evaluates");

        // Every `OUTPUT:` declaration is addressable by name, not just the last
        // line — that is what multi-root retention in the plan layer buys.
        assert_eq!(output.channels().len(), 3);
        for name in ["DIF", "DEA", "MACD"] {
            let channel = output
                .channel(name)
                .unwrap_or_else(|| panic!("channel `{name}` missing from {:?}", output.channels()));
            assert_eq!(channel.len(), 80);
        }
        assert_eq!(output.primary().len(), 80);

        // `DIF` is the first channel and must carry real data: the plan lowered
        // `DIF:...` into a local write (see the `OUTPUT:` arm of
        // `lower_formula_plumbing`), so the later `EMA(DIF, 9)` reads it as a
        // variable rather than demanding a caller-supplied series named `DIF`.
        let dif = output.channel("DIF").unwrap();
        assert!(
            dif.iter().any(|value| value.is_finite()),
            "the DIF channel should carry computed values, not warm-up NaNs only"
        );

        // The primary is the value of the last value-producing statement, which
        // for this formula is `MACD` — so the two must be the *same* series. This
        // is the case that used to hand back an empty primary, because resolving
        // the channels consumed the very buffer the primary also points at.
        //
        // Compared NaN-aware, not with `assert_eq!`: `DEA` is `EMA(DIF, 9)` and
        // `DIF` is NaN for its first 25 samples, so the EMA seed averages a NaN
        // window and `DEA`/`MACD` are legitimately all-NaN here. Slice equality
        // would fail on two identical vectors because `NaN != NaN`.
        assert_same_series(output.primary(), output.channel("MACD").unwrap(), 0.0);
        assert!(output.channel("NOPE").is_none());
    }

    #[test]
    fn a_multi_channel_formula_carries_distinct_finite_series() {
        // A channel that is *not* the primary must still be its own series: this
        // formula's last statement is `C`, but `A` and `B` are retained roots in
        // their own right. Using rolling means (rather than EMAs) keeps the
        // values finite from index 9, so the comparison is numeric rather than a
        // warm-up-NaN artefact.
        let engine = FormulaEngine::new();
        let mut context = ctx(60);

        let output = engine
            .eval_plan_channels(
                "A:MA(CLOSE,5);B:MA(CLOSE,10);C:A-B",
                FormulaDialect::AlphaTA,
                &ParamValues::new(),
                &mut context,
            )
            .expect("multi-channel plan evaluates");

        let a = output.channel("A").expect("channel A").to_vec();
        let b = output.channel("B").expect("channel B").to_vec();
        let c = output.channel("C").expect("channel C").to_vec();

        assert!(c.iter().any(|value| value.is_finite()));
        assert_eq!(c[9], a[9] - b[9], "C should be A - B at index 9");
        assert!(
            a.iter().zip(b.iter()).any(|(left, right)| left != right),
            "A and B should be different series"
        );

        // `C` is the last statement, so it is both the primary and a channel.
        assert_same_series(output.primary(), &c, 0.0);

        // And the whole channel set must agree with the reference tree path.
        let mut tree_context = context.clone();
        let tree = FormulaEngine::new()
            .eval("A:MA(CLOSE,5);B:MA(CLOSE,10);C:A-B", &mut tree_context)
            .expect("tree evaluates");
        assert_same_series(output.primary(), &tree.to_vec(), 1e-9);
    }

    #[test]
    fn a_missing_input_series_is_reported_rather_than_substituted() {
        let engine = FormulaEngine::new();
        let mut context = ctx(40);

        // `MYVAR` is neither OHLCV nor a context variable. The plan path must
        // refuse, because silently substituting another series would hide an
        // input-layout bug behind a numeric mismatch.
        let error = engine
            .eval_plan("MA(MYVAR, 5)", &mut context)
            .expect_err("an unbound input must fail");

        let message = error.to_string();
        assert!(
            message.contains("MYVAR"),
            "error should name the missing series: {message}"
        );
    }

    #[test]
    fn a_repeated_evaluation_reuses_the_cached_plan() {
        let engine = FormulaEngine::new();
        let mut context = ctx(40);

        engine
            .eval_plan("MA(CLOSE,5)", &mut context)
            .expect("plan evaluates");
        engine
            .eval_plan("MA(CLOSE,5)", &mut context)
            .expect("plan evaluates");

        assert_eq!(engine.plan_cache_size(), 1);
        assert_eq!(
            engine.plan_cache_stats(),
            FormulaPlanCacheStats { hits: 1, misses: 1 }
        );
    }

    #[test]
    fn an_unsupported_operator_fails_loudly_instead_of_falling_back() {
        let engine = FormulaEngine::new();
        let mut context = ctx(40);

        // `FILTER` is a documented kernel gap. The plan path must report it
        // rather than quietly re-running the tree path, otherwise a caller
        // could never tell which engine produced a number.
        //
        // `IF` used to stand in here and had to be swapped out once it gained
        // a kernel. If this fails because the function below now works, pick
        // another entry from the plan-kernel backlog -- the point of the test
        // is that *some* gap still fails loudly, so do not delete the check.
        let error = engine
            .eval_plan("FILTER(CLOSE>OPEN, 5)", &mut context)
            .expect_err("a kernel-less operator must fail");

        let message = error.to_string();
        assert!(
            message.contains("formula plan"),
            "error should identify the plan path: {message}"
        );
    }
}
