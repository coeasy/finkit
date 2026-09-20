//! Canonical operation contracts shared by indicators, formulas, factors,
//! composites, and drawing adapters.
//!
//! The registry is the stable discovery and planning layer. The unified façade
//! below routes the currently executable Formula, Factor, and Composite paths
//! through one request/result/error contract; each new operation still needs a
//! verified dispatcher and golden vectors before it is marked fully complete.

use crate::composite::{CompositeDefinition, CompositeEngine};
use crate::data_contract::{
    CrossSectionView, DataContractError, FrameKey, FundamentalSeries, MarketPanel,
    TemporalAlignment, TemporalSeries,
};
use crate::factor_system::{CompiledFactorPlan, FactorCatalog};
use crate::factors::{
    BorrowedFactorContext, FactorDefinition, FactorEngine, FactorKind, FactorRegistry,
};
use crate::formula::{
    AstNode, DrawResult, FormulaContext, FormulaDialect, FormulaEngine, FormulaError,
    FormulaStatefulStream, PineSecurityResolver,
};
use crate::registry::{
    builtin_function_registry, FunctionCategory, FunctionSpec, InputKind, LookbackSpec, ParamSpec,
};
use crate::unified_runtime::{DirtyRange, RuntimeExecutionTrace};
use ndarray::Array1;
use std::collections::BTreeMap;
use std::fmt;

/// Reserved name used for the primary value returned by a formula.
pub const PRIMARY_OUTPUT_NAME: &str = "__PRIMARY__";

const COMPILED_FACTOR_PLAN_CACHE_CAPACITY: usize = 64;

/// Top-level kind of a public operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationKind {
    /// TA-Lib-style or native technical indicator.
    Indicator,
    /// A callable formula-language primitive.
    FormulaFunction,
    /// A reusable one-output or multi-output factor transform.
    Factor,
    /// A composition of operations and/or factors.
    Composite,
    /// A rendering instruction or derived chart series.
    Draw,
}

/// Shape of an operation's value result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValueShape {
    /// One value per row.
    Series,
    /// Multiple aligned series per row, such as MACD or Bollinger Bands.
    MultiSeries,
    /// One value per symbol and timestamp.
    CrossSection,
    /// Discrete events aligned to rows.
    Event,
    /// A non-row result, such as a report or summary.
    Report,
}

/// Explicit execution and semantic capabilities.
///
/// A capability is only set when the underlying implementation has a public
/// contract for it. In particular, `parallel`, `multi_symbol`, and
/// `multi_timeframe` are not inferred from a batch function accepting slices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationCapabilities {
    /// Supports finite batch evaluation.
    pub batch: bool,
    /// Supports incremental/stateful evaluation.
    pub streaming: bool,
    /// Supports a cross-sectional execution context.
    pub cross_sectional: bool,
    /// Accepts multiple symbols as one logical request.
    pub multi_symbol: bool,
    /// Accepts multiple explicit timeframes without implicit resampling.
    pub multi_timeframe: bool,
    /// Can be partitioned by the planner without changing results.
    pub parallel: bool,
    /// Repeated evaluation is deterministic for identical inputs/configuration.
    pub deterministic: bool,
    /// Does not read future rows.
    pub causal: bool,
    /// May intentionally read future rows.
    pub lookahead: bool,
    /// May revise prior values after new input arrives.
    pub repaint: bool,
    /// Requires mutable state between evaluations.
    pub stateful: bool,
    /// Produces or mutates a drawing scene.
    pub drawable: bool,
}

impl OperationCapabilities {
    /// Conservative capabilities for a regular causal indicator.
    pub const fn indicator(streaming: bool, deterministic: bool) -> Self {
        Self {
            batch: true,
            streaming,
            cross_sectional: false,
            multi_symbol: false,
            multi_timeframe: false,
            parallel: false,
            deterministic,
            causal: true,
            lookahead: false,
            repaint: false,
            stateful: streaming,
            drawable: false,
        }
    }

    /// Capabilities for a formula primitive invoked inside a formula plan.
    pub const fn formula_function(streaming: bool, deterministic: bool) -> Self {
        Self {
            batch: true,
            streaming,
            cross_sectional: false,
            multi_symbol: false,
            multi_timeframe: false,
            parallel: false,
            deterministic,
            causal: true,
            lookahead: false,
            repaint: false,
            stateful: streaming,
            drawable: false,
        }
    }

    /// Conservative capabilities for a user or built-in factor definition.
    pub const fn factor(kind: FactorKind) -> Self {
        Self {
            batch: true,
            streaming: false,
            cross_sectional: matches!(kind, FactorKind::CrossSectional),
            multi_symbol: matches!(kind, FactorKind::CrossSectional),
            multi_timeframe: false,
            parallel: false,
            deterministic: true,
            causal: true,
            lookahead: false,
            repaint: false,
            stateful: false,
            drawable: false,
        }
    }
}

/// Stable identifier derived from the canonical normalized operation name.
///
/// The identifier is not a cryptographic hash. The registry checks collisions
/// at registration time, and the canonical name remains the human-readable
/// source of truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(pub u32);

impl OperationId {
    /// Build the stable id for a canonical operation name.
    pub fn from_name(name: &str) -> Self {
        let normalized = normalize_name(name);
        let mut hash = 0x811c_9dc5_u32;
        for byte in normalized.bytes() {
            hash ^= u32::from(byte);
            hash = hash.wrapping_mul(0x0100_0193);
        }
        Self(hash)
    }
}

/// Canonical metadata consumed by planners, bindings, docs, and adapters.
#[derive(Debug, Clone, PartialEq)]
pub struct OperationSpec {
    /// Canonical uppercase name.
    pub name: String,
    /// Accepted aliases, normalized at registration time.
    pub aliases: Vec<String>,
    /// Stable operation kind.
    pub kind: OperationKind,
    /// Result shape.
    pub value_shape: ValueShape,
    /// Legacy discovery category where one exists.
    pub category: Option<FunctionCategory>,
    /// Input contract inherited from the function registry when available.
    pub input: Option<InputKind>,
    /// Parameter declarations.
    pub params: Vec<ParamSpec>,
    /// Number of aligned output series.
    pub outputs: usize,
    /// Stable names for the aligned output series, in result order.
    pub output_names: Vec<String>,
    /// Warm-up/lookback behavior.
    pub lookback: LookbackSpec,
    /// Execution and semantic capabilities.
    pub capabilities: OperationCapabilities,
    /// Version of this metadata contract.
    pub schema_version: u16,
}

impl OperationSpec {
    /// Return the stable numeric identity used by planners and bindings.
    pub fn id(&self) -> OperationId {
        OperationId::from_name(&self.name)
    }

    /// Project an existing public function description into the canonical
    /// operation contract without claiming capabilities it did not declare.
    pub fn from_function(spec: &FunctionSpec) -> Self {
        let kind = match spec.category {
            FunctionCategory::Formula => OperationKind::FormulaFunction,
            FunctionCategory::Factor => OperationKind::Factor,
            _ => OperationKind::Indicator,
        };
        let value_shape = if spec.outputs > 1 {
            ValueShape::MultiSeries
        } else {
            ValueShape::Series
        };
        Self {
            name: normalize_name(spec.name),
            aliases: spec
                .aliases
                .iter()
                .map(|alias| normalize_name(alias))
                .collect(),
            kind,
            value_shape,
            category: Some(spec.category),
            input: Some(spec.input),
            params: spec.params.to_vec(),
            outputs: spec.outputs,
            output_names: output_names_for(&spec.name, spec.outputs),
            lookback: spec.lookback,
            capabilities: match kind {
                OperationKind::Indicator => {
                    OperationCapabilities::indicator(spec.streaming, spec.deterministic)
                }
                OperationKind::FormulaFunction => {
                    OperationCapabilities::formula_function(spec.streaming, spec.deterministic)
                }
                OperationKind::Factor => OperationCapabilities::factor(FactorKind::TimeSeries),
                OperationKind::Composite | OperationKind::Draw => {
                    OperationCapabilities::indicator(false, spec.deterministic)
                }
            },
            schema_version: 1,
        }
    }

    /// Project a registered Factor into the canonical operation catalog.
    pub fn from_factor(factor: &FactorDefinition) -> Self {
        Self {
            name: normalize_name(&factor.name),
            aliases: Vec::new(),
            kind: OperationKind::Factor,
            value_shape: match factor.kind {
                FactorKind::TimeSeries => ValueShape::Series,
                FactorKind::CrossSectional => ValueShape::CrossSection,
            },
            category: Some(FunctionCategory::Factor),
            input: Some(InputKind::Dynamic),
            params: Vec::new(),
            outputs: 1,
            output_names: vec![normalize_name(&factor.name)],
            lookback: LookbackSpec::Dynamic,
            capabilities: OperationCapabilities::factor(factor.kind),
            schema_version: 1,
        }
    }
}

/// Errors raised while registering a canonical operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationRegistryError {
    /// The canonical name or an alias is empty.
    EmptyName,
    /// The operation name is already registered.
    DuplicateName(String),
    /// An alias conflicts with an operation name or another alias.
    DuplicateAlias(String),
    /// Two canonical names produced the same stable id.
    IdCollision {
        id: OperationId,
        existing: String,
        incoming: String,
    },
}

impl fmt::Display for OperationRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("operation name must not be empty"),
            Self::DuplicateName(name) => write!(formatter, "operation already registered: {name}"),
            Self::DuplicateAlias(alias) => {
                write!(formatter, "operation alias already registered: {alias}")
            }
            Self::IdCollision {
                id,
                existing,
                incoming,
            } => write!(
                formatter,
                "operation id collision {}: {existing} vs {incoming}",
                id.0
            ),
        }
    }
}

impl std::error::Error for OperationRegistryError {}

/// Deterministic registry for the unified public operation catalog.
#[derive(Debug, Clone, Default)]
pub struct OperationRegistry {
    specs: BTreeMap<String, OperationSpec>,
    aliases: BTreeMap<String, String>,
    ids: BTreeMap<OperationId, String>,
}

impl OperationRegistry {
    /// Create an empty operation registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an operation atomically.
    pub fn register(&mut self, mut spec: OperationSpec) -> Result<(), OperationRegistryError> {
        let canonical = normalize_name(&spec.name);
        if canonical.is_empty() {
            return Err(OperationRegistryError::EmptyName);
        }
        if self.specs.contains_key(&canonical) || self.aliases.contains_key(&canonical) {
            return Err(OperationRegistryError::DuplicateName(canonical));
        }

        let id = OperationId::from_name(&canonical);
        if let Some(existing) = self.ids.get(&id) {
            return Err(OperationRegistryError::IdCollision {
                id,
                existing: existing.clone(),
                incoming: canonical,
            });
        }

        let mut normalized_aliases = BTreeMap::new();
        for alias in &spec.aliases {
            let normalized = normalize_name(alias);
            if normalized.is_empty()
                || normalized == canonical
                || self.specs.contains_key(&normalized)
                || self.aliases.contains_key(&normalized)
                || normalized_aliases.insert(normalized.clone(), ()).is_some()
            {
                return Err(OperationRegistryError::DuplicateAlias(normalized));
            }
        }

        spec.name = canonical.clone();
        spec.aliases = normalized_aliases.keys().cloned().collect();
        self.ids.insert(id, canonical.clone());
        for alias in spec.aliases.iter() {
            self.aliases.insert(alias.clone(), canonical.clone());
        }
        self.specs.insert(canonical, spec);
        Ok(())
    }

    /// Register all entries from the existing function metadata registry.
    pub fn from_function_registry(
        functions: &crate::registry::FunctionRegistry,
    ) -> Result<Self, OperationRegistryError> {
        let mut registry = Self::new();
        for function in functions.iter() {
            registry.register(OperationSpec::from_function(function))?;
        }
        Ok(registry)
    }

    /// Register all factors from a caller-owned factor registry.
    pub fn register_factor_registry(
        &mut self,
        factors: &FactorRegistry,
    ) -> Result<(), OperationRegistryError> {
        for factor in factors.iter() {
            self.register(OperationSpec::from_factor(factor))?;
        }
        Ok(())
    }

    /// Resolve a canonical name or alias case-insensitively.
    pub fn get(&self, name: &str) -> Option<&OperationSpec> {
        let normalized = normalize_name(name);
        self.specs.get(&normalized).or_else(|| {
            self.aliases
                .get(&normalized)
                .and_then(|canonical| self.specs.get(canonical))
        })
    }

    /// Resolve a stable id.
    pub fn get_by_id(&self, id: OperationId) -> Option<&OperationSpec> {
        self.ids.get(&id).and_then(|name| self.specs.get(name))
    }

    /// Iterate in stable canonical-name order.
    pub fn iter(&self) -> impl Iterator<Item = &OperationSpec> {
        self.specs.values()
    }

    /// Number of canonical operations.
    pub fn len(&self) -> usize {
        self.specs.len()
    }

    /// Whether no operations are registered.
    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }
}

/// Build the canonical catalog from all built-in public functions.
pub fn builtin_operation_registry() -> OperationRegistry {
    OperationRegistry::from_function_registry(&builtin_function_registry())
        .expect("built-in operation names and aliases are unique")
}

/// A request routed through the unified Formula/Factor/Composite façade.
///
/// The request borrows caller-owned contexts. No raw market data is copied at
/// dispatch time; ownership is transferred only in the returned result.
pub enum OperationRequest<'a> {
    /// Invoke one registered indicator directly without parsing a source string.
    Indicator {
        /// Canonical name or registered alias.
        name: &'a str,
        /// Named input series resolved by the formula context.
        inputs: &'a [&'a str],
        /// Numeric parameters in the operation's declared order.
        params: &'a [f64],
        /// Mutable input context.
        context: &'a mut FormulaContext,
    },
    /// Compile and execute a formula against a mutable formula context.
    Formula {
        /// Formula source in the selected dialect's canonical syntax.
        source: &'a str,
        /// Parser/semantic dialect used for the source.
        dialect: FormulaDialect,
        /// Mutable context, because formulas may assign variables or emit draw commands.
        context: &'a mut FormulaContext,
    },
    /// Execute a Pine Formula with an explicit host-owned `request.security`
    /// resolver.
    ///
    /// The resolver owns symbol/timeframe alignment and data-leakage policy;
    /// parsing, mapping, evaluation, and result normalization remain in the
    /// unified Core Runtime.
    FormulaWithPineSecurity {
        /// Pine source in the supported Pine subset.
        source: &'a str,
        /// Mutable input context.
        context: &'a mut FormulaContext,
        /// Host-owned provider resolver.
        resolver: &'a dyn PineSecurityResolver,
    },
    /// Evaluate one registered factor from a borrowed named-series context.
    Factor {
        /// Registered factor name.
        name: &'a str,
        /// Borrowed factor input context.
        context: &'a BorrowedFactorContext<'a>,
        /// Optional caller-owned revision. When present, the result is
        /// eligible for the bounded operation cache.
        data_revision: Option<u64>,
        /// Optional symbol/timeframe or caller-defined cache namespace.
        cache_scope: Option<&'a str>,
    },
    /// Evaluate selected composite outputs from a borrowed named-series context.
    Composite {
        /// Named composite definitions.
        definitions: &'a [CompositeDefinition],
        /// Requested output names.
        outputs: &'a [&'a str],
        /// Borrowed composite input context.
        context: &'a BorrowedFactorContext<'a>,
        /// Optional caller-owned revision used by the composite cache.
        data_revision: Option<u64>,
        /// Optional symbol/timeframe or caller-defined cache namespace.
        cache_scope: Option<&'a str>,
    },
    /// Evaluate one cross-sectional factor at every timestamp row.
    CrossSectionalFactor {
        /// Registered factor name.
        name: &'a str,
        /// Named cross-sectional input views with identical dimensions.
        inputs: &'a [(&'a str, &'a CrossSectionView<'a>)],
    },
}

/// Unified named-series result returned by Formula, Factor, and Composite execution.
#[derive(Debug, Clone)]
pub struct OperationResult {
    /// Named aligned output series. Keys are deterministic and operation-defined.
    pub values: BTreeMap<String, Vec<f64>>,
    /// Shape of the returned values.
    pub shape: ValueShape,
    /// Name of the primary output, if the operation has one.
    pub primary: Option<String>,
    /// Formula drawing commands, when the execution emitted any.
    pub draw: Option<DrawResult>,
}

/// Results keyed by the explicit symbol/timeframe frame that produced them.
#[derive(Debug, Clone)]
pub struct PanelOperationResult {
    /// Deterministically ordered results keyed by `FrameKey`.
    pub values: BTreeMap<crate::data_contract::FrameKey, OperationResult>,
}

/// Stable result-cache identity for a panel operation.
///
/// The numeric buffers are intentionally not hashed. The caller owns the
/// monotonic `data_revision` contract and must advance it whenever any input
/// that can affect the result changes. This keeps hot-path caching O(1) with
/// respect to data size while making symbol/timeframe isolation explicit.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OperationCacheKey {
    /// High-level operation family, such as `formula.panel`.
    pub operation: String,
    /// Canonical request signature, excluding the frame and revision.
    pub request: String,
    /// Formula dialect, when the operation has one.
    pub dialect: Option<String>,
    /// Explicit symbol/timeframe frame identity.
    pub frame: FrameKey,
    /// Caller-owned input revision.
    pub data_revision: u64,
}

impl OperationCacheKey {
    /// Build a cache key for a panel formula request.
    pub fn panel_formula(
        source: &str,
        dialect: FormulaDialect,
        frame: &FrameKey,
        data_revision: u64,
    ) -> Self {
        Self {
            operation: "formula.panel".to_string(),
            request: source.to_string(),
            dialect: Some(dialect.as_str().to_string()),
            frame: frame.clone(),
            data_revision,
        }
    }

    /// Build a cache key for a revision-scoped Factor result.
    ///
    /// The caller must advance `data_revision` whenever any raw input that can
    /// affect the factor changes. The logical frame is kept separate from
    /// panel-formula keys by the operation family and has a deterministic
    /// default namespace for single-series callers.
    pub fn factor(name: &str, cache_scope: Option<&str>, data_revision: u64) -> Self {
        Self {
            operation: "factor".to_string(),
            request: name.to_string(),
            dialect: None,
            frame: FrameKey {
                symbol: cache_scope.unwrap_or("__default__").to_string(),
                timeframe: "factor".to_string(),
            },
            data_revision,
        }
    }

    /// Build a revision-scoped cache key for a multi-target Factor batch.
    pub fn factor_batch(names: &[String], cache_scope: Option<&str>, data_revision: u64) -> Self {
        Self {
            operation: "factor.batch".to_string(),
            request: names.join("\u{1f}"),
            dialect: None,
            frame: FrameKey {
                symbol: cache_scope.unwrap_or("__default__").to_string(),
                timeframe: "factor".to_string(),
            },
            data_revision,
        }
    }
}

/// Observable counters for the unified operation result cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationCacheStats {
    /// Number of cache hits since construction or the last clear.
    pub hits: u64,
    /// Number of cache misses since construction or the last clear.
    pub misses: u64,
    /// Number of retained result entries.
    pub entries: usize,
    /// Maximum number of retained result entries.
    pub capacity: usize,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    result: OperationResult,
    last_used: u64,
}

/// Bounded, revision-aware result cache keyed by [`OperationCacheKey`].
///
/// This is *the* result cache of the operation façade — it is an extraction of
/// the entry table, capacity, LRU clock and counters that used to live as five
/// separate fields of [`UnifiedOperationEngine`]. Keeping them in one type is
/// the point: they are only correct together, and while they were spread over
/// the engine every call site had to remember to bump the right counter on
/// exactly one branch. [`Self::get`] now owns that, so a call site cannot get
/// it wrong.
///
/// Cache isolation is the caller's contract, not a property of the key's
/// contents. Entries are never invalidated on read, so a caller must advance
/// [`OperationCacheKey::data_revision`] whenever any input that can affect the
/// result changes. Reusing a revision for changed data serves a stale result —
/// this is a correctness requirement, not a performance tuning knob.
#[derive(Debug, Clone)]
pub struct OperationResultCache {
    entries: BTreeMap<OperationCacheKey, CacheEntry>,
    capacity: usize,
    hits: u64,
    misses: u64,
    clock: u64,
}

impl OperationResultCache {
    /// Capacity of a freshly constructed façade.
    pub const DEFAULT_CAPACITY: usize = 64;

    /// Create an empty cache.
    ///
    /// `capacity` is clamped to at least one entry, so the eviction path needs
    /// no special case for a zero-sized cache.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            capacity: capacity.max(1),
            hits: 0,
            misses: 0,
            clock: 0,
        }
    }

    /// Look up `key`, marking it most-recently-used and counting a hit or miss.
    ///
    /// A lookup produces exactly one of the two counters, and this is the only
    /// place either is incremented.
    pub fn get(&mut self, key: &OperationCacheKey) -> Option<OperationResult> {
        let tick = self.next_tick();
        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_used = tick;
            self.hits = self.hits.saturating_add(1);
            Some(entry.result.clone())
        } else {
            self.misses = self.misses.saturating_add(1);
            None
        }
    }

    /// Retain `result` under `key` as the most-recently-used entry.
    ///
    /// Evicts the least-recently-used entry when the cache is already at
    /// capacity and `key` is new. Replacing an existing key never evicts, so a
    /// refresh cannot displace an unrelated entry.
    pub fn insert(&mut self, key: OperationCacheKey, result: OperationResult) {
        let last_used = self.next_tick();
        if self.entries.len() >= self.capacity && !self.entries.contains_key(&key) {
            if let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| key.clone())
            {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(key, CacheEntry { result, last_used });
    }

    /// Hit/miss counters and current occupancy.
    #[must_use]
    pub fn stats(&self) -> OperationCacheStats {
        OperationCacheStats {
            hits: self.hits,
            misses: self.misses,
            entries: self.entries.len(),
            capacity: self.capacity,
        }
    }

    /// Remove every entry and reset the counters.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.hits = 0;
        self.misses = 0;
        self.clock = 0;
    }

    /// Change the capacity and invalidate retained entries.
    ///
    /// Counters are lifetime statistics and are deliberately *not* reset here;
    /// use [`Self::clear`] when the counters should restart too.
    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity.max(1);
        self.entries.clear();
    }

    /// Number of retained entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no entries are retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Maximum number of retained entries.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Advance the monotonic LRU clock, wrapping rather than overflowing.
    fn next_tick(&mut self) -> u64 {
        self.clock = self.clock.wrapping_add(1);
        self.clock
    }
}

#[derive(Clone)]
struct CompiledFactorPlanCacheEntry {
    plan: CompiledFactorPlan,
    last_used: u64,
}

impl PanelOperationResult {
    /// Number of symbol/timeframe frames evaluated.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the panel produced no frame results.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Return one frame result without copying it.
    pub fn get(&self, key: &crate::data_contract::FrameKey) -> Option<&OperationResult> {
        self.values.get(key)
    }
}

impl OperationResult {
    /// Return the primary output without copying it.
    pub fn primary_values(&self) -> Option<&[f64]> {
        self.primary
            .as_deref()
            .and_then(|name| self.values.get(name).map(Vec::as_slice))
    }

    /// Return a named output without copying it.
    pub fn get(&self, name: &str) -> Option<&[f64]> {
        self.values.get(name).map(Vec::as_slice)
    }

    /// Number of named output series.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the result contains no output series.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Errors surfaced by the unified operation façade.
#[derive(Debug, Clone, PartialEq)]
pub enum OperationExecutionError {
    /// The requested operation is absent from the canonical catalog.
    UnknownOperation(String),
    /// The request shape or numeric arguments are invalid before execution.
    InvalidRequest(String),
    /// Formula parsing, planning, or evaluation failed.
    Formula(FormulaError),
    /// Factor registration, dependency resolution, or evaluation failed.
    Factor(crate::factors::FactorError),
    /// Composite graph validation or evaluation failed.
    Composite(crate::factors::FactorError),
    /// Market-panel data violated the canonical dimension contract.
    DataContract(DataContractError),
}

impl fmt::Display for OperationExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOperation(name) => write!(formatter, "unknown operation: {name}"),
            Self::InvalidRequest(message) => {
                write!(formatter, "invalid operation request: {message}")
            }
            Self::Formula(error) => write!(formatter, "formula operation failed: {error}"),
            Self::Factor(error) => write!(formatter, "factor operation failed: {error}"),
            Self::Composite(error) => write!(formatter, "composite operation failed: {error}"),
            Self::DataContract(error) => {
                write!(formatter, "operation data contract failed: {error}")
            }
        }
    }
}

impl std::error::Error for OperationExecutionError {}

impl From<FormulaError> for OperationExecutionError {
    fn from(value: FormulaError) -> Self {
        Self::Formula(value)
    }
}

impl From<DataContractError> for OperationExecutionError {
    fn from(value: DataContractError) -> Self {
        Self::DataContract(value)
    }
}

/// Runtime façade that routes the three currently executable high-level
/// operation families through one request/result/error contract.
pub struct UnifiedOperationEngine {
    catalog: OperationRegistry,
    formula: FormulaEngine,
    factor: FactorEngine,
    composite: CompositeEngine,
    factor_catalog: FactorCatalog,
    factor_plans: BTreeMap<String, CompiledFactorPlanCacheEntry>,
    factor_plan_cache_hits: u64,
    factor_plan_cache_misses: u64,
    factor_plan_cache_clock: u64,
    operation_cache: OperationResultCache,
}

impl UnifiedOperationEngine {
    /// Create an engine with the built-in operation catalog and caller factors.
    pub fn new(factors: FactorRegistry) -> Self {
        Self::try_new(factors).expect("factor names must not collide with built-in operations")
    }

    /// Create an engine and return configuration errors instead of panicking.
    pub fn try_new(factors: FactorRegistry) -> Result<Self, OperationRegistryError> {
        let mut catalog = builtin_operation_registry();
        catalog.register_factor_registry(&factors)?;
        let factor_catalog = FactorCatalog::from_registry(factors.clone());
        Ok(Self {
            catalog,
            formula: FormulaEngine::new(),
            factor: FactorEngine::new(factors),
            composite: CompositeEngine::new(),
            factor_catalog,
            factor_plans: BTreeMap::new(),
            factor_plan_cache_hits: 0,
            factor_plan_cache_misses: 0,
            factor_plan_cache_clock: 0,
            operation_cache: OperationResultCache::new(OperationResultCache::DEFAULT_CAPACITY),
        })
    }

    /// Create an engine with an explicit unified result-cache capacity.
    pub fn with_cache_capacity(factors: FactorRegistry, capacity: usize) -> Self {
        let mut engine = Self::new(factors);
        engine.set_cache_capacity(capacity);
        engine
    }

    /// Change the unified result-cache capacity and invalidate old entries.
    pub fn set_cache_capacity(&mut self, capacity: usize) {
        self.operation_cache.set_capacity(capacity);
    }

    /// Remove all unified operation results and reset cache counters.
    pub fn clear_cache(&mut self) {
        self.operation_cache.clear();
    }

    /// Return unified operation result-cache counters.
    pub fn cache_stats(&self) -> OperationCacheStats {
        self.operation_cache.stats()
    }

    /// Return Composite result-cache counters owned by the canonical façade.
    ///
    /// Composite keeps its graph-result cache separate from the generic
    /// operation cache so compiled plans and snapshots can use their own
    /// bounded lifetimes. Exposing the counters here keeps observability at
    /// the same typed Runtime boundary used for execution.
    #[must_use]
    pub fn composite_cache_stats(&self) -> crate::composite::CompositeCacheStats {
        self.composite.cache_stats()
    }

    /// Read the canonical metadata catalog used by this engine.
    pub fn catalog(&self) -> &OperationRegistry {
        &self.catalog
    }

    /// Read the canonical Factor catalog used for planning and metadata.
    pub fn factor_catalog(&self) -> &FactorCatalog {
        &self.factor_catalog
    }

    /// Access the factor engine for registration and precompiled workflows.
    pub fn factor_engine(&self) -> &FactorEngine {
        &self.factor
    }

    /// Access the composite engine for custom function registration and cache control.
    pub fn composite_engine(&self) -> &CompositeEngine {
        &self.composite
    }

    /// Mutably access the composite engine for custom function registration and cache control.
    pub fn composite_engine_mut(&mut self) -> &mut CompositeEngine {
        &mut self.composite
    }

    /// Prepare a Composite streaming plan from the canonical Runtime engine.
    ///
    /// The stream object owns an engine snapshot because it may outlive the
    /// short mutable borrow used by the dispatcher. The snapshot is cloned
    /// from the registered Runtime engine, so custom functions and stateful
    /// specifications are preserved instead of silently falling back to a
    /// fresh built-in-only `CompositeEngine` in a binding.
    pub fn prepare_composite_stream(
        &mut self,
        definitions: &[CompositeDefinition],
        outputs: &[&str],
    ) -> Result<(crate::composite::CompiledCompositePlan, CompositeEngine), OperationExecutionError>
    {
        let plan = self
            .composite
            .compile_cached(definitions, outputs)
            .map_err(OperationExecutionError::Composite)?;
        Ok((plan, self.composite.clone()))
    }

    /// Prepare a Factor streaming plan from the canonical Runtime engine.
    ///
    /// Targets are resolved, de-duplicated, and canonically ordered before
    /// entering the shared compiled-plan cache. The stream owns a cloned
    /// FactorEngine snapshot for the same reason as Composite streams: the
    /// stream can outlive the dispatch borrow without losing registered
    /// factors.
    pub fn prepare_factor_stream(
        &mut self,
        targets: &[&str],
    ) -> Result<(CompiledFactorPlan, FactorEngine), OperationExecutionError> {
        let plan = self.prepare_factor_plan(targets)?;
        Ok((plan, self.factor.clone()))
    }

    /// Prepare a canonical Factor plan from the Runtime-owned catalog and
    /// compiled-plan cache without creating a stream object.
    pub fn prepare_factor_plan(
        &mut self,
        targets: &[&str],
    ) -> Result<CompiledFactorPlan, OperationExecutionError> {
        if targets.is_empty() {
            return Err(OperationExecutionError::InvalidRequest(
                "factor targets must not be empty".to_string(),
            ));
        }
        let mut canonical_targets = targets
            .iter()
            .map(|target| {
                self.factor_catalog
                    .resolve_name(target)
                    .map(str::to_owned)
                    .ok_or_else(|| OperationExecutionError::UnknownOperation((*target).to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if canonical_targets
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != canonical_targets.len()
        {
            return Err(OperationExecutionError::InvalidRequest(
                "factor targets must be unique".to_string(),
            ));
        }
        canonical_targets.sort_unstable();
        self.compiled_factor_plan_targets(&canonical_targets)
    }

    /// Compile a stateful Formula stream through the canonical Runtime
    /// boundary. The returned stream owns its incremental state and portable
    /// checkpoint image; the Runtime remains responsible for dialect selection
    /// and admission of the stream contract.
    pub fn prepare_formula_stream(
        &mut self,
        source: &str,
        dialect: FormulaDialect,
    ) -> Result<FormulaStatefulStream, OperationExecutionError> {
        FormulaStatefulStream::from_source(source, dialect).map_err(OperationExecutionError::Factor)
    }

    /// Execute a Formula, Factor, or Composite request using one result contract.
    pub fn execute<'a>(
        &mut self,
        request: OperationRequest<'a>,
    ) -> Result<OperationResult, OperationExecutionError> {
        match request {
            OperationRequest::Indicator {
                name,
                inputs,
                params,
                context,
            } => self.execute_indicator(name, inputs, params, context),
            OperationRequest::Formula {
                source,
                dialect,
                context,
            } => {
                let (values, draw) = self.execute_formula(source, dialect, context)?;
                let shape = if values.len() > 1 {
                    ValueShape::MultiSeries
                } else {
                    ValueShape::Series
                };
                Ok(OperationResult {
                    values,
                    shape,
                    primary: Some(PRIMARY_OUTPUT_NAME.to_string()),
                    draw,
                })
            }
            OperationRequest::FormulaWithPineSecurity {
                source,
                context,
                resolver,
            } => self.execute_formula_with_pine_security(source, context, resolver),
            OperationRequest::Factor {
                name,
                context,
                data_revision,
                cache_scope,
            } => {
                let canonical = self
                    .factor_catalog
                    .resolve_name(name)
                    .map(str::to_owned)
                    .ok_or_else(|| OperationExecutionError::UnknownOperation(name.to_string()))?;
                if let Some(revision) = data_revision {
                    let key = OperationCacheKey::factor(&canonical, cache_scope, revision);
                    self.cached_or_compute(key, |engine| {
                        engine.execute_factor_uncached(&canonical, context)
                    })
                } else {
                    self.execute_factor_uncached(&canonical, context)
                }
            }
            OperationRequest::Composite {
                definitions,
                outputs,
                context,
                data_revision,
                cache_scope,
            } => {
                let plan = self
                    .composite
                    .compile_cached(definitions, outputs)
                    .map_err(OperationExecutionError::Composite)?;
                let values = match data_revision {
                    Some(revision) => self.composite.evaluate_cached_scoped_compiled(
                        cache_scope.unwrap_or(""),
                        &plan,
                        context,
                        revision,
                    ),
                    None => self.composite.evaluate_compiled(&plan, context),
                }
                .map_err(OperationExecutionError::Composite)?;
                let primary = (outputs.len() == 1).then(|| outputs[0].to_string());
                Ok(OperationResult {
                    values,
                    shape: if outputs.len() > 1 {
                        ValueShape::MultiSeries
                    } else {
                        ValueShape::Series
                    },
                    primary,
                    draw: None,
                })
            }
            OperationRequest::CrossSectionalFactor { name, inputs } => {
                let result = self
                    .factor
                    .evaluate_cross_sectional(name, inputs)
                    .map_err(OperationExecutionError::Factor)?;
                let values = std::iter::once((name.to_string(), result.values)).collect();
                Ok(OperationResult {
                    values,
                    shape: ValueShape::CrossSection,
                    primary: Some(name.to_string()),
                    draw: None,
                })
            }
        }
    }

    /// Execute one or more Factor targets through one shared dependency plan.
    ///
    /// A single target preserves the ordinary Factor cache identity. Multiple
    /// targets are compiled and evaluated as one DAG, so common dependencies
    /// are executed once and the complete result is cached as one batch.
    pub fn execute_factor_targets(
        &mut self,
        names: &[&str],
        context: &BorrowedFactorContext<'_>,
        data_revision: Option<u64>,
        cache_scope: Option<&str>,
    ) -> Result<OperationResult, OperationExecutionError> {
        if names.is_empty() {
            return Err(OperationExecutionError::InvalidRequest(
                "factor targets must not be empty".to_string(),
            ));
        }
        let canonical_targets = names
            .iter()
            .map(|name| {
                self.factor_catalog
                    .resolve_name(name)
                    .map(str::to_owned)
                    .ok_or_else(|| OperationExecutionError::UnknownOperation((*name).to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if canonical_targets
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != canonical_targets.len()
        {
            return Err(OperationExecutionError::InvalidRequest(
                "factor targets must be unique".to_string(),
            ));
        }
        let mut canonical_targets = canonical_targets;
        canonical_targets.sort_unstable();
        if canonical_targets.len() == 1 {
            return self.execute(OperationRequest::Factor {
                name: &canonical_targets[0],
                context,
                data_revision,
                cache_scope,
            });
        }

        match data_revision {
            Some(revision) => {
                let key = OperationCacheKey::factor_batch(&canonical_targets, cache_scope, revision);
                self.cached_or_compute(key, |engine| {
                    engine.execute_factor_targets_uncached(&canonical_targets, context)
                })
            }
            None => self.execute_factor_targets_uncached(&canonical_targets, context),
        }
    }

    /// Evaluate several canonical Factor targets as one shared dependency plan.
    ///
    /// Split out of [`Self::execute_factor_targets`] so the cached and uncached
    /// paths run the same code and the result cache has a single place to wrap.
    fn execute_factor_targets_uncached(
        &mut self,
        canonical_targets: &[String],
        context: &BorrowedFactorContext<'_>,
    ) -> Result<OperationResult, OperationExecutionError> {
        let plan = self.compiled_factor_plan_targets(canonical_targets)?;
        let output = plan
            .execute_borrowed(&self.factor, context)
            .map_err(OperationExecutionError::Factor)?;
        let values = canonical_targets
            .iter()
            .map(|name| {
                output
                    .get(name)
                    .cloned()
                    .map(|series| (name.clone(), series))
                    .ok_or_else(|| {
                        OperationExecutionError::InvalidRequest(format!(
                            "compiled factor plan did not produce {name}"
                        ))
                    })
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        Ok(OperationResult {
            values,
            shape: ValueShape::MultiSeries,
            primary: None,
            draw: None,
        })
    }

    /// Execute a range-safe Factor plan through the canonical Runtime façade.
    ///
    /// This is the incremental counterpart to [`Self::execute`]. It keeps
    /// plan compilation, dependency validation, and dirty-range semantics in
    /// the same typed engine instead of making FFI adapters call the domain
    /// executor directly.
    pub fn execute_factor_range(
        &mut self,
        name: &str,
        context: &BorrowedFactorContext<'_>,
        previous: &BTreeMap<String, Vec<f64>>,
        dirty: DirtyRange,
    ) -> Result<(OperationResult, RuntimeExecutionTrace), OperationExecutionError> {
        self.execute_factor_range_targets(&[name], context, previous, dirty)
    }

    /// Execute several range-safe Factor targets as one shared dependency plan.
    ///
    /// Multi-target requests are compiled together so shared dependencies are
    /// evaluated once, matching the full-batch Factor contract.
    pub fn execute_factor_range_targets(
        &mut self,
        names: &[&str],
        context: &BorrowedFactorContext<'_>,
        previous: &BTreeMap<String, Vec<f64>>,
        dirty: DirtyRange,
    ) -> Result<(OperationResult, RuntimeExecutionTrace), OperationExecutionError> {
        if names.is_empty() {
            return Err(OperationExecutionError::InvalidRequest(
                "factor range targets must not be empty".to_string(),
            ));
        }
        let canonical_targets = names
            .iter()
            .map(|name| {
                self.factor_catalog
                    .resolve_name(name)
                    .map(str::to_owned)
                    .ok_or_else(|| OperationExecutionError::UnknownOperation((*name).to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let target_refs = canonical_targets
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let plan = if canonical_targets.len() == 1 {
            self.compiled_factor_plan(&canonical_targets[0])?
        } else {
            self.factor_catalog
                .compile(&target_refs)
                .map_err(OperationExecutionError::Factor)?
        };
        let runtime = plan
            .execute_range_borrowed(&self.factor, context, previous, dirty)
            .map_err(OperationExecutionError::Factor)?;
        let mut named = BTreeMap::new();
        for canonical in &canonical_targets {
            let values = runtime.output.get(canonical).cloned().ok_or_else(|| {
                OperationExecutionError::InvalidRequest(format!(
                    "compiled factor plan did not produce {canonical}"
                ))
            })?;
            named.insert(canonical.clone(), values);
        }
        let primary = (canonical_targets.len() == 1).then(|| canonical_targets[0].clone());
        Ok((
            OperationResult {
                values: named,
                shape: if canonical_targets.len() > 1 {
                    ValueShape::MultiSeries
                } else {
                    ValueShape::Series
                },
                primary,
                draw: None,
            },
            runtime.trace,
        ))
    }

    /// Execute a range-safe Composite plan through the canonical Runtime façade.
    ///
    /// Composite result snapshots are intentionally not inserted into the
    /// full-result cache: the caller supplies the retained previous material-
    /// ization, and the range executor returns a new trace describing exactly
    /// which rows were recomputed.
    pub fn execute_composite_range(
        &mut self,
        definitions: &[CompositeDefinition],
        outputs: &[&str],
        context: &BorrowedFactorContext<'_>,
        previous: &BTreeMap<String, Vec<f64>>,
        dirty: DirtyRange,
    ) -> Result<(OperationResult, RuntimeExecutionTrace), OperationExecutionError> {
        let plan = self
            .composite
            .compile_cached(definitions, outputs)
            .map_err(OperationExecutionError::Composite)?;
        let runtime = self
            .composite
            .execute_range_borrowed(&plan, context, previous, dirty)
            .map_err(OperationExecutionError::Composite)?;
        Ok((
            OperationResult {
                values: runtime.output,
                shape: if outputs.len() > 1 {
                    ValueShape::MultiSeries
                } else {
                    ValueShape::Series
                },
                primary: (outputs.len() == 1).then(|| outputs[0].to_string()),
                draw: None,
            },
            runtime.trace,
        ))
    }

    fn execute_indicator(
        &mut self,
        name: &str,
        inputs: &[&str],
        params: &[f64],
        context: &mut FormulaContext,
    ) -> Result<OperationResult, OperationExecutionError> {
        let spec = self
            .catalog
            .get(name)
            .ok_or_else(|| OperationExecutionError::UnknownOperation(name.to_string()))?;
        if !matches!(
            spec.kind,
            OperationKind::Indicator | OperationKind::FormulaFunction
        ) {
            return Err(OperationExecutionError::InvalidRequest(format!(
                "operation {name} is not directly invokable as an indicator"
            )));
        }
        if inputs.iter().any(|input| input.trim().is_empty()) {
            return Err(OperationExecutionError::InvalidRequest(
                "indicator input names must not be empty".to_string(),
            ));
        }
        if params.iter().any(|value| !value.is_finite()) {
            return Err(OperationExecutionError::InvalidRequest(
                "indicator parameters must be finite".to_string(),
            ));
        }

        if spec.outputs > 1 {
            return execute_multi_output_indicator(spec.name.as_str(), inputs, params, context);
        }

        let mut args = Vec::with_capacity(inputs.len() + params.len());
        args.extend(
            inputs
                .iter()
                .map(|input| AstNode::Variable((*input).to_string())),
        );
        args.extend(params.iter().copied().map(AstNode::Number));
        let ast = AstNode::FunctionCall {
            name: spec.name.clone(),
            args,
        };
        let result = self.formula.eval_ast(&ast, context)?;
        let mut values = BTreeMap::new();
        values.insert(PRIMARY_OUTPUT_NAME.to_string(), result.to_vec());
        Ok(OperationResult {
            values,
            shape: if spec.outputs > 1 {
                ValueShape::MultiSeries
            } else {
                ValueShape::Series
            },
            primary: Some(PRIMARY_OUTPUT_NAME.to_string()),
            draw: None,
        })
    }

    /// Execute one formula independently for every explicit symbol/timeframe frame.
    ///
    /// This method intentionally returns a panel result rather than forcing
    /// multi-dimensional data into the single-operation result map.
    pub fn execute_panel_formula(
        &mut self,
        source: &str,
        dialect: FormulaDialect,
        panel: &MarketPanel<'_>,
    ) -> Result<PanelOperationResult, OperationExecutionError> {
        let mut values = BTreeMap::new();
        for (key, frame) in panel.iter() {
            values.insert(
                key.clone(),
                self.execute_formula_on_frame(source, dialect, frame)?,
            );
        }
        Ok(PanelOperationResult { values })
    }

    /// Execute a panel formula with revision-aware result caching.
    ///
    /// Every frame is cached independently, so adding or changing one symbol
    /// does not invalidate unrelated symbol/timeframe results. Reusing a
    /// revision for changed input data is a caller error by contract.
    pub fn execute_panel_formula_cached(
        &mut self,
        source: &str,
        dialect: FormulaDialect,
        panel: &MarketPanel<'_>,
        data_revision: u64,
    ) -> Result<PanelOperationResult, OperationExecutionError> {
        let mut values = BTreeMap::new();
        for (key, frame) in panel.iter() {
            let cache_key = OperationCacheKey::panel_formula(source, dialect, key, data_revision);
            let result = self.cached_or_compute(cache_key, |engine| {
                engine.execute_formula_on_frame(source, dialect, frame)
            })?;
            values.insert(key.clone(), result);
        }
        Ok(PanelOperationResult { values })
    }

    /// Execute one registered indicator independently for every panel frame.
    ///
    /// A panel is a collection of independent symbol/timeframe series. This
    /// method preserves that boundary and never concatenates bars from unlike
    /// frames before dispatching to the indicator kernel.
    pub fn execute_panel_indicator(
        &mut self,
        name: &str,
        inputs: &[&str],
        params: &[f64],
        panel: &MarketPanel<'_>,
    ) -> Result<PanelOperationResult, OperationExecutionError> {
        let mut values = BTreeMap::new();
        for (key, frame) in panel.iter() {
            frame
                .validate()
                .map_err(|error| DataContractError::InvalidMarketFrame(error))?;
            let amount = frame.amount.map(|series| Array1::from_vec(series.to_vec()));
            let mut context = FormulaContext::from_borrowed_ohlcv(
                frame.open,
                frame.high,
                frame.low,
                frame.close,
                frame.volume,
                amount,
            );
            context.datetime = frame
                .timestamp
                .map(|timestamps| Array1::from_vec(timestamps.to_vec()));
            values.insert(
                key.clone(),
                self.execute_indicator(name, inputs, params, &mut context)?,
            );
        }
        Ok(PanelOperationResult { values })
    }

    /// Execute a formula on one frame with point-in-time fundamental inputs.
    ///
    /// Each fundamental field is expanded to the frame's timestamps using its
    /// publication-time `as_of` rule. No future revision can enter an earlier
    /// market bar, and callers remain responsible for selecting the correct
    /// symbol before invoking this method.
    pub fn execute_formula_with_fundamentals(
        &mut self,
        source: &str,
        dialect: FormulaDialect,
        frame: &crate::runtime::MarketFrame<'_>,
        fundamentals: &[FundamentalSeries<'_>],
    ) -> Result<OperationResult, OperationExecutionError> {
        frame
            .validate()
            .map_err(|error| DataContractError::InvalidMarketFrame(error))?;
        let timestamps = frame.timestamp.ok_or_else(|| {
            OperationExecutionError::InvalidRequest(
                "point-in-time fundamentals require frame timestamps".to_string(),
            )
        })?;
        let amount = frame.amount.map(|series| Array1::from_vec(series.to_vec()));
        let mut context = FormulaContext::from_borrowed_ohlcv(
            frame.open,
            frame.high,
            frame.low,
            frame.close,
            frame.volume,
            amount,
        );
        context.datetime = Some(Array1::from_vec(timestamps.to_vec()));
        for fundamental in fundamentals {
            let fundamental = FundamentalSeries::new(
                fundamental.name,
                fundamental.timestamps,
                fundamental.values,
            )
            .map_err(OperationExecutionError::DataContract)?;
            let values = timestamps
                .iter()
                .map(|&timestamp| fundamental.as_of(timestamp).unwrap_or(f64::NAN))
                .collect::<Vec<_>>();
            context.variables.insert(
                std::sync::Arc::from(normalize_name(fundamental.name)),
                Array1::from_vec(values),
            );
        }

        let (values, draw) = self.execute_formula(source, dialect, &mut context)?;
        let shape = if values.len() > 1 {
            ValueShape::MultiSeries
        } else {
            ValueShape::Series
        };
        Ok(OperationResult {
            values,
            shape,
            primary: Some(PRIMARY_OUTPUT_NAME.to_string()),
            draw,
        })
    }

    /// Execute a formula with explicitly aligned inputs from other timelines.
    ///
    /// Each input declares its own timestamps and alignment policy. The
    /// `AsOfClosed` policy is suitable for a higher-timeframe series and
    /// prevents an unfinished source bar from being visible to an earlier
    /// target row. This method performs alignment only; it never resamples or
    /// mutates the source frames.
    pub fn execute_formula_with_temporal_inputs(
        &mut self,
        source: &str,
        dialect: FormulaDialect,
        frame: &crate::runtime::MarketFrame<'_>,
        inputs: &[(&str, &[i64], &[f64], TemporalAlignment)],
    ) -> Result<OperationResult, OperationExecutionError> {
        frame
            .validate()
            .map_err(|error| DataContractError::InvalidMarketFrame(error))?;
        let timestamps = frame.timestamp.ok_or_else(|| {
            OperationExecutionError::InvalidRequest(
                "temporal formula inputs require frame timestamps".to_string(),
            )
        })?;
        let amount = frame.amount.map(|series| Array1::from_vec(series.to_vec()));
        let mut context = FormulaContext::from_borrowed_ohlcv(
            frame.open,
            frame.high,
            frame.low,
            frame.close,
            frame.volume,
            amount,
        );
        context.datetime = Some(Array1::from_vec(timestamps.to_vec()));
        for &(name, input_timestamps, values, policy) in inputs {
            let input = TemporalSeries::new(name, input_timestamps, values)
                .map_err(OperationExecutionError::DataContract)?;
            let aligned = input
                .align_to(timestamps, policy)
                .map_err(OperationExecutionError::DataContract)?;
            context.variables.insert(
                std::sync::Arc::from(normalize_name(name)),
                Array1::from_vec(aligned),
            );
        }

        let (values, draw) = self.execute_formula(source, dialect, &mut context)?;
        let shape = if values.len() > 1 {
            ValueShape::MultiSeries
        } else {
            ValueShape::Series
        };
        Ok(OperationResult {
            values,
            shape,
            primary: Some(PRIMARY_OUTPUT_NAME.to_string()),
            draw,
        })
    }

    /// Execute a Pine formula with an explicit host-owned `request.security`
    /// resolver.
    ///
    /// The resolver is deliberately supplied by the host because symbol and
    /// timeframe alignment are data-contract concerns. Keeping the actual
    /// Pine parse/map/evaluate step here ensures temporal Pine execution uses
    /// the same Formula engine and result envelope as every other formula
    /// request instead of creating a second engine in a binding.
    pub fn execute_formula_with_pine_security(
        &mut self,
        source: &str,
        context: &mut FormulaContext,
        resolver: &dyn PineSecurityResolver,
    ) -> Result<OperationResult, OperationExecutionError> {
        let result = self
            .formula
            .eval_multi_with_pine_security(source, context, resolver)?;
        let mut values = result
            .outputs
            .into_iter()
            .map(|(name, value)| (name, value.to_vec()))
            .collect::<BTreeMap<_, _>>();
        values.insert(PRIMARY_OUTPUT_NAME.to_string(), result.final_value.to_vec());
        let draw = {
            let draw = context.draw_commands.borrow();
            (!draw.commands.is_empty()).then(|| draw.clone())
        };
        let shape = if values.len() > 1 {
            ValueShape::MultiSeries
        } else {
            ValueShape::Series
        };
        Ok(OperationResult {
            values,
            shape,
            primary: Some(PRIMARY_OUTPUT_NAME.to_string()),
            draw,
        })
    }

    fn execute_formula_on_frame(
        &mut self,
        source: &str,
        dialect: FormulaDialect,
        frame: &crate::runtime::MarketFrame<'_>,
    ) -> Result<OperationResult, OperationExecutionError> {
        frame
            .validate()
            .map_err(|error| DataContractError::InvalidMarketFrame(error))?;
        let amount = frame.amount.map(|series| Array1::from_vec(series.to_vec()));
        let mut context = FormulaContext::from_borrowed_ohlcv(
            frame.open,
            frame.high,
            frame.low,
            frame.close,
            frame.volume,
            amount,
        );
        context.datetime = frame
            .timestamp
            .map(|timestamps| Array1::from_vec(timestamps.to_vec()));
        let (values, draw) = self.execute_formula(source, dialect, &mut context)?;
        let shape = if values.len() > 1 {
            ValueShape::MultiSeries
        } else {
            ValueShape::Series
        };
        Ok(OperationResult {
            values,
            shape,
            primary: Some(PRIMARY_OUTPUT_NAME.to_string()),
            draw,
        })
    }

    /// Return the cached result for `key`, or compute, retain and return it.
    ///
    /// This is the `get_or_compute` abstraction kept from the retired
    /// `crates/finkit-runtime` cache, and it is the only place a result enters
    /// [`OperationResultCache`] on a miss. Both the hit and miss counters live
    /// inside the cache's `get`, so no call site can bump one on the wrong
    /// branch.
    ///
    /// The compute closure receives `&mut self` rather than being self-contained
    /// so that a miss can still compile and cache a factor plan. That ordering is
    /// deliberate: compiling the plan *before* the lookup would make every
    /// result-cache hit pay for a plan lookup and would move the
    /// `factor_plan_cache_hits` / `factor_plan_cache_misses` counters.
    fn cached_or_compute<F>(
        &mut self,
        key: OperationCacheKey,
        compute: F,
    ) -> Result<OperationResult, OperationExecutionError>
    where
        F: FnOnce(&mut Self) -> Result<OperationResult, OperationExecutionError>,
    {
        if let Some(result) = self.operation_cache.get(&key) {
            return Ok(result);
        }
        let result = compute(self)?;
        self.operation_cache.insert(key, result.clone());
        Ok(result)
    }

    fn execute_factor_uncached(
        &mut self,
        canonical: &str,
        context: &BorrowedFactorContext<'_>,
    ) -> Result<OperationResult, OperationExecutionError> {
        let plan = self.compiled_factor_plan(canonical)?;
        let result = plan
            .execute_borrowed(&self.factor, context)
            .map_err(OperationExecutionError::Factor)?
            .remove(canonical)
            .ok_or_else(|| {
                OperationExecutionError::InvalidRequest(format!(
                    "compiled factor plan did not produce {canonical}"
                ))
            })?;
        let mut values = BTreeMap::new();
        values.insert(canonical.to_string(), result);
        Ok(OperationResult {
            values,
            shape: ValueShape::Series,
            primary: Some(canonical.to_string()),
            draw: None,
        })
    }

    fn compiled_factor_plan(
        &mut self,
        canonical: &str,
    ) -> Result<CompiledFactorPlan, OperationExecutionError> {
        self.compiled_factor_plan_targets(&[canonical.to_string()])
    }

    fn compiled_factor_plan_targets(
        &mut self,
        canonical_targets: &[String],
    ) -> Result<CompiledFactorPlan, OperationExecutionError> {
        let cache_name = if canonical_targets.len() == 1 {
            canonical_targets[0].clone()
        } else {
            format!("batch:{}", canonical_targets.join("\u{1f}"))
        };
        let tick = self.next_factor_plan_tick();
        if let Some(entry) = self.factor_plans.get_mut(&cache_name) {
            self.factor_plan_cache_hits = self.factor_plan_cache_hits.saturating_add(1);
            entry.last_used = tick;
            return Ok(entry.plan.clone());
        }

        self.factor_plan_cache_misses = self.factor_plan_cache_misses.saturating_add(1);
        let plan = self
            .factor_catalog
            .compile(
                &canonical_targets
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            )
            .map_err(OperationExecutionError::Factor)?;
        if self.factor_plans.len() >= COMPILED_FACTOR_PLAN_CACHE_CAPACITY {
            if let Some(oldest) = self
                .factor_plans
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(name, _)| name.clone())
            {
                self.factor_plans.remove(&oldest);
            }
        }
        self.factor_plans.insert(
            cache_name,
            CompiledFactorPlanCacheEntry {
                plan: plan.clone(),
                last_used: tick,
            },
        );
        Ok(plan)
    }

    fn next_factor_plan_tick(&mut self) -> u64 {
        self.factor_plan_cache_clock = self.factor_plan_cache_clock.wrapping_add(1);
        self.factor_plan_cache_clock
    }

    fn execute_formula(
        &mut self,
        source: &str,
        dialect: FormulaDialect,
        context: &mut FormulaContext,
    ) -> Result<(BTreeMap<String, Vec<f64>>, Option<DrawResult>), FormulaError> {
        let result = self
            .formula
            .eval_multi_with_dialect(source, dialect, context)?;
        let mut values = result
            .outputs
            .into_iter()
            .map(|(name, value)| (name, value.to_vec()))
            .collect::<BTreeMap<_, _>>();
        values.insert(PRIMARY_OUTPUT_NAME.to_string(), result.final_value.to_vec());
        let draw = {
            let draw = context.draw_commands.borrow();
            (!draw.commands.is_empty()).then(|| draw.clone())
        };
        Ok((values, draw))
    }
}

fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_uppercase()
}

fn output_names_for(name: &str, outputs: usize) -> Vec<String> {
    match normalize_name(name).as_str() {
        "MACD" | "MACDEXT" | "MACDFIX" => vec![
            "MACD".to_string(),
            "MACD_SIGNAL".to_string(),
            "MACD_HIST".to_string(),
        ],
        "BBANDS" => vec![
            "UPPERBAND".to_string(),
            "MIDDLEBAND".to_string(),
            "LOWERBAND".to_string(),
        ],
        "ACCBANDS" => vec![
            "UPPERBAND".to_string(),
            "MIDDLEBAND".to_string(),
            "LOWERBAND".to_string(),
        ],
        "MAMA" => vec!["MAMA".to_string(), "FAMA".to_string()],
        "HT_PHASOR" => vec!["INPHASE".to_string(), "QUADRATURE".to_string()],
        "HT_SINE" => vec!["SINE".to_string(), "LEADSINE".to_string()],
        "MINMAX" => vec!["MIN".to_string(), "MAX".to_string()],
        "MINMAXINDEX" => vec!["MININDEX".to_string(), "MAXINDEX".to_string()],
        _ if outputs == 1 => vec![normalize_name(name)],
        _ => (0..outputs)
            .map(|index| format!("{}_{}", normalize_name(name), index + 1))
            .collect(),
    }
}

fn execute_multi_output_indicator(
    name: &str,
    inputs: &[&str],
    params: &[f64],
    context: &FormulaContext,
) -> Result<OperationResult, OperationExecutionError> {
    let result = match name {
        "MACDEXT" => {
            require_input_count(name, inputs, 1)?;
            let close = resolve_indicator_input(name, context, inputs[0])?;
            let fast = parameter_usize(name, params, 0, 12)?;
            let fast_type = talib_ma_type(name, params, 1, 0)?;
            let slow = parameter_usize(name, params, 2, 26)?;
            let slow_type = talib_ma_type(name, params, 3, 0)?;
            let signal = parameter_usize(name, params, 4, 9)?;
            let signal_type = talib_ma_type(name, params, 5, 0)?;
            let output = crate::indicators::momentum::macdext(
                close,
                fast,
                fast_type,
                slow,
                slow_type,
                signal,
                signal_type,
            )
            .map_err(|error| indicator_execution_error(name, error))?;
            MultiIndicatorOutput {
                values: vec![
                    ("MACD", output.macd.to_vec()),
                    ("MACD_SIGNAL", output.signal.to_vec()),
                    ("MACD_HIST", output.hist.to_vec()),
                ],
                primary: "MACD",
            }
        }
        "MACDFIX" => {
            require_input_count(name, inputs, 1)?;
            let close = resolve_indicator_input(name, context, inputs[0])?;
            let signal = parameter_usize(name, params, 0, 9)?;
            let output = crate::indicators::momentum::macdfix_with_signal(close, signal)
                .map_err(|error| indicator_execution_error(name, error))?;
            MultiIndicatorOutput {
                values: vec![
                    ("MACD", output.macd.to_vec()),
                    ("MACD_SIGNAL", output.signal.to_vec()),
                    ("MACD_HIST", output.hist.to_vec()),
                ],
                primary: "MACD",
            }
        }
        "MACD" => {
            require_input_count(name, inputs, 1)?;
            let close = resolve_indicator_input(name, context, inputs[0])?;
            let fast = parameter_usize(name, params, 0, 12)?;
            let slow = parameter_usize(name, params, 1, 26)?;
            let signal = parameter_usize(name, params, 2, 9)?;
            let output = crate::indicators::momentum::macd(close, fast, slow, signal)
                .map_err(|error| indicator_execution_error(name, error))?;
            MultiIndicatorOutput {
                values: vec![
                    ("MACD", output.macd.to_vec()),
                    ("MACD_SIGNAL", output.signal.to_vec()),
                    ("MACD_HIST", output.hist.to_vec()),
                ],
                primary: "MACD",
            }
        }
        "BBANDS" => {
            require_input_count(name, inputs, 1)?;
            let close = resolve_indicator_input(name, context, inputs[0])?;
            let period = parameter_usize(name, params, 0, 20)?;
            let deviation = parameter_f64(name, params, 1, 2.0)?;
            let output = crate::indicators::overlap::bbands(close, period, deviation, deviation)
                .map_err(|error| indicator_execution_error(name, error))?;
            MultiIndicatorOutput {
                values: vec![
                    ("UPPERBAND", output.upper.to_vec()),
                    ("MIDDLEBAND", output.middle.to_vec()),
                    ("LOWERBAND", output.lower.to_vec()),
                ],
                primary: "MIDDLEBAND",
            }
        }
        "ACCBANDS" => {
            require_input_count(name, inputs, 3)?;
            let high = resolve_indicator_input(name, context, inputs[0])?;
            let low = resolve_indicator_input(name, context, inputs[1])?;
            let close = resolve_indicator_input(name, context, inputs[2])?;
            let period = parameter_usize(name, params, 0, 20)?;
            let output = crate::indicators::overlap::accbands(high, low, close, period)
                .map_err(|error| indicator_execution_error(name, error))?;
            MultiIndicatorOutput {
                values: vec![
                    ("UPPERBAND", output.upper.to_vec()),
                    ("MIDDLEBAND", output.middle.to_vec()),
                    ("LOWERBAND", output.lower.to_vec()),
                ],
                primary: "MIDDLEBAND",
            }
        }
        "MAMA" => {
            require_input_count(name, inputs, 1)?;
            let close = resolve_indicator_input(name, context, inputs[0])?;
            let fast_limit = parameter_f64(name, params, 0, 0.5)?;
            let slow_limit = parameter_f64(name, params, 1, 0.05)?;
            let output = crate::indicators::overlap::mama(close, fast_limit, slow_limit)
                .map_err(|error| indicator_execution_error(name, error))?;
            MultiIndicatorOutput {
                values: vec![
                    ("MAMA", output.mama.to_vec()),
                    ("FAMA", output.fama.to_vec()),
                ],
                primary: "MAMA",
            }
        }
        "HT_PHASOR" => {
            require_input_count(name, inputs, 1)?;
            let close = resolve_indicator_input(name, context, inputs[0])?;
            let output = crate::indicators::cycle::ht_phasor(close)
                .map_err(|error| indicator_execution_error(name, error))?;
            MultiIndicatorOutput {
                values: vec![
                    ("INPHASE", output.0.to_vec()),
                    ("QUADRATURE", output.1.to_vec()),
                ],
                primary: "INPHASE",
            }
        }
        "HT_SINE" => {
            require_input_count(name, inputs, 1)?;
            let close = resolve_indicator_input(name, context, inputs[0])?;
            let output = crate::indicators::cycle::ht_sine(close)
                .map_err(|error| indicator_execution_error(name, error))?;
            MultiIndicatorOutput {
                values: vec![("SINE", output.0.to_vec()), ("LEADSINE", output.1.to_vec())],
                primary: "SINE",
            }
        }
        "MINMAX" => {
            require_input_count(name, inputs, 1)?;
            let input = resolve_indicator_input(name, context, inputs[0])?;
            let period = parameter_usize(name, params, 0, 30)?;
            let (minimum, maximum) = crate::indicators::math_operators::minmax(input, period)
                .map_err(|error| indicator_execution_error(name, error))?;
            MultiIndicatorOutput {
                values: vec![("MIN", minimum.to_vec()), ("MAX", maximum.to_vec())],
                primary: "MIN",
            }
        }
        "MINMAXINDEX" => {
            require_input_count(name, inputs, 1)?;
            let input = resolve_indicator_input(name, context, inputs[0])?;
            let period = parameter_usize(name, params, 0, 30)?;
            let (minimum, maximum) = crate::indicators::math_operators::minmaxindex(input, period)
                .map_err(|error| indicator_execution_error(name, error))?;
            MultiIndicatorOutput {
                values: vec![
                    (
                        "MININDEX",
                        minimum.iter().map(|value| *value as f64).collect(),
                    ),
                    (
                        "MAXINDEX",
                        maximum.iter().map(|value| *value as f64).collect(),
                    ),
                ],
                primary: "MININDEX",
            }
        }
        _ => {
            return Err(OperationExecutionError::InvalidRequest(format!(
                "multi-output operation {name} has no unified dispatcher yet"
            )))
        }
    };

    let values = result
        .values
        .into_iter()
        .map(|(name, values)| (name.to_string(), values))
        .collect::<BTreeMap<_, _>>();
    Ok(OperationResult {
        values,
        shape: ValueShape::MultiSeries,
        primary: Some(result.primary.to_string()),
        draw: None,
    })
}

struct MultiIndicatorOutput {
    values: Vec<(&'static str, Vec<f64>)>,
    primary: &'static str,
}

fn require_input_count(
    name: &str,
    inputs: &[&str],
    expected: usize,
) -> Result<(), OperationExecutionError> {
    if inputs.len() != expected {
        return Err(OperationExecutionError::InvalidRequest(format!(
            "{name} expects {expected} input series, got {}",
            inputs.len()
        )));
    }
    Ok(())
}

fn resolve_indicator_input<'a>(
    name: &str,
    context: &'a FormulaContext,
    input: &str,
) -> Result<&'a [f64], OperationExecutionError> {
    context.get_data(input).ok_or_else(|| {
        OperationExecutionError::InvalidRequest(format!(
            "{name} input series is not present in the formula context: {input}"
        ))
    })
}

fn parameter_usize(
    name: &str,
    params: &[f64],
    index: usize,
    default: usize,
) -> Result<usize, OperationExecutionError> {
    let value = params.get(index).copied().unwrap_or(default as f64);
    if !value.is_finite() || value < 1.0 || value.fract() != 0.0 {
        return Err(OperationExecutionError::InvalidRequest(format!(
            "{name} parameter {index} must be a positive integer"
        )));
    }
    Ok(value as usize)
}

fn parameter_f64(
    name: &str,
    params: &[f64],
    index: usize,
    default: f64,
) -> Result<f64, OperationExecutionError> {
    let value = params.get(index).copied().unwrap_or(default);
    if !value.is_finite() || value < 0.0 {
        return Err(OperationExecutionError::InvalidRequest(format!(
            "{name} parameter {index} must be finite and non-negative"
        )));
    }
    Ok(value)
}

fn talib_ma_type(
    name: &str,
    params: &[f64],
    index: usize,
    default: usize,
) -> Result<crate::indicators::overlap::MaType, OperationExecutionError> {
    let value = params.get(index).copied().unwrap_or(default as f64);
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value > 8.0 {
        return Err(OperationExecutionError::InvalidRequest(format!(
            "{name} MA type parameter {index} must be an integer in 0..8"
        )));
    }
    Ok(match value as usize {
        0 => crate::indicators::overlap::MaType::Sma,
        1 => crate::indicators::overlap::MaType::Ema,
        2 => crate::indicators::overlap::MaType::Wma,
        3 => crate::indicators::overlap::MaType::Dema,
        4 => crate::indicators::overlap::MaType::Tema,
        5 => crate::indicators::overlap::MaType::Trima,
        6 => crate::indicators::overlap::MaType::Kama,
        7 => crate::indicators::overlap::MaType::Mama,
        8 => crate::indicators::overlap::MaType::T3,
        _ => unreachable!(),
    })
}

fn indicator_execution_error(name: &str, error: impl fmt::Display) -> OperationExecutionError {
    OperationExecutionError::Formula(FormulaError::RuntimeError(format!(
        "{name} indicator execution failed: {error}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composite::{CompositeExpr, CompositeOp};
    use crate::data_contract::FrameKey;
    use crate::factors::{builtin_factor_registry, FactorDefinition, FactorDirection, FactorKind};
    use crate::runtime::MarketFrame;
    use crate::unified_runtime::RuntimeExecutionMode;
    use ndarray::Array1;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Minimal single-series result for cache-level tests.
    fn cached_result(value: f64) -> OperationResult {
        let mut values = BTreeMap::new();
        values.insert(PRIMARY_OUTPUT_NAME.to_string(), vec![value]);
        OperationResult {
            values,
            shape: ValueShape::Series,
            primary: Some(PRIMARY_OUTPUT_NAME.to_string()),
            draw: None,
        }
    }

    #[test]
    fn result_cache_counts_exactly_one_hit_or_miss_per_lookup() {
        let mut cache = OperationResultCache::new(4);
        let key = OperationCacheKey::factor("SMA", Some("AAA"), 1);

        assert!(cache.get(&key).is_none());
        assert_eq!(cache.stats().misses, 1);
        assert_eq!(cache.stats().hits, 0);

        cache.insert(key.clone(), cached_result(1.0));
        assert!(cache.get(&key).is_some());
        assert_eq!(cache.stats().misses, 1);
        assert_eq!(cache.stats().hits, 1);

        // A miss must not be recorded as a hit, and vice versa.
        assert!(cache.get(&OperationCacheKey::factor("SMA", Some("BBB"), 1)).is_none());
        assert_eq!(cache.stats().misses, 2);
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn result_cache_isolates_entries_by_data_revision() {
        // The revision is the caller's input-identity contract: the same factor
        // over changed data must not be served from the previous revision's
        // entry, so it has to be a distinct cache key.
        let mut cache = OperationResultCache::new(4);
        let first = OperationCacheKey::factor("SMA", Some("AAA"), 1);
        cache.insert(first.clone(), cached_result(1.0));

        let second = OperationCacheKey::factor("SMA", Some("AAA"), 2);
        assert!(
            cache.get(&second).is_none(),
            "a new data_revision must miss even for an unchanged factor and scope"
        );
        assert_eq!(cache.stats().misses, 1);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn result_cache_evicts_the_least_recently_used_entry() {
        let mut cache = OperationResultCache::new(2);
        let key_a = OperationCacheKey::factor("SMA", Some("AAA"), 1);
        let key_b = OperationCacheKey::factor("SMA", Some("BBB"), 1);
        let key_c = OperationCacheKey::factor("SMA", Some("CCC"), 1);

        cache.insert(key_a.clone(), cached_result(1.0));
        cache.insert(key_b.clone(), cached_result(2.0));
        // Touch `a` so `b` becomes the least-recently-used entry.
        assert!(cache.get(&key_a).is_some());

        cache.insert(key_c.clone(), cached_result(3.0));
        assert_eq!(cache.len(), 2);
        assert!(cache.get(&key_a).is_some(), "recently used entry must survive");
        assert!(cache.get(&key_b).is_none(), "least recently used entry must be evicted");
        assert!(cache.get(&key_c).is_some());
    }

    #[test]
    fn result_cache_replacing_a_key_does_not_evict_another_entry() {
        let mut cache = OperationResultCache::new(2);
        let key_a = OperationCacheKey::factor("SMA", Some("AAA"), 1);
        let key_b = OperationCacheKey::factor("SMA", Some("BBB"), 1);

        cache.insert(key_a.clone(), cached_result(1.0));
        cache.insert(key_b.clone(), cached_result(2.0));
        // A refresh at capacity must replace in place, not evict a sibling.
        cache.insert(key_a.clone(), cached_result(9.0));

        assert_eq!(cache.len(), 2);
        assert!(cache.get(&key_b).is_some());
        // The replacement is visible under the original key, and `b` survived.
        assert_eq!(
            cache.get(&key_a).unwrap().primary_values().unwrap(),
            &[9.0]
        );
    }

    #[test]
    fn result_cache_capacity_and_clear_scope_their_effects_correctly() {
        let mut cache = OperationResultCache::new(4);
        let key = OperationCacheKey::factor("SMA", Some("AAA"), 1);
        cache.insert(key.clone(), cached_result(1.0));
        let _ = cache.get(&key);

        // Changing capacity invalidates entries but keeps lifetime counters.
        cache.set_capacity(2);
        assert_eq!(cache.capacity(), 2);
        assert!(cache.is_empty());
        assert_eq!(cache.stats().hits, 1);
        assert_eq!(cache.stats().misses, 0);

        // Clearing resets the counters too.
        cache.insert(key.clone(), cached_result(1.0));
        let _ = cache.get(&key);
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.stats().hits, 0);
        assert_eq!(cache.stats().misses, 0);

        // A zero capacity is clamped so eviction needs no special case.
        assert_eq!(OperationResultCache::new(0).capacity(), 1);
    }

    #[test]
    fn builtins_project_without_losing_function_contracts() {
        let functions = builtin_function_registry();
        let operations = OperationRegistry::from_function_registry(&functions).unwrap();

        assert_eq!(operations.len(), functions.len());
        let ema = operations.get(" ema ").unwrap();
        assert_eq!(ema.kind, OperationKind::Indicator);
        assert_eq!(ema.value_shape, ValueShape::Series);
        assert!(ema.capabilities.batch);
        assert!(ema.capabilities.streaming);
        assert!(ema.capabilities.causal);
        assert!(!ema.capabilities.lookahead);
    }

    #[test]
    fn ids_are_case_insensitive_and_resolvable() {
        let operations = builtin_operation_registry();
        let ema = operations.get("EMA").unwrap();
        let id = OperationId::from_name("ema");

        assert_eq!(operations.get_by_id(id), Some(ema));
        assert_eq!(id, OperationId::from_name(" EMA "));
    }

    #[test]
    fn registered_factor_is_projected_with_cross_sectional_capabilities() {
        let mut factors = FactorRegistry::new();
        factors
            .register(FactorDefinition::new(
                "cs_rank",
                ["close"],
                FactorKind::CrossSectional,
                FactorDirection::HigherBetter,
                Arc::new(|inputs| Ok(inputs.get("close")?.to_vec())),
            ))
            .unwrap();

        let mut operations = builtin_operation_registry();
        operations.register_factor_registry(&factors).unwrap();
        let spec = operations.get("CS_RANK").unwrap();
        assert_eq!(spec.kind, OperationKind::Factor);
        assert_eq!(spec.value_shape, ValueShape::CrossSection);
        assert!(spec.capabilities.cross_sectional);
        assert!(spec.capabilities.multi_symbol);
        assert!(!spec.capabilities.streaming);
    }

    #[test]
    fn try_new_rejects_factor_name_collisions_before_engine_creation() {
        let mut factors = FactorRegistry::new();
        factors
            .register(FactorDefinition::new(
                "EMA",
                ["close"],
                FactorKind::TimeSeries,
                FactorDirection::Neutral,
                Arc::new(|inputs| Ok(inputs.get("close")?.to_vec())),
            ))
            .unwrap();
        assert!(matches!(
            UnifiedOperationEngine::try_new(factors),
            Err(OperationRegistryError::DuplicateName(name)) if name == "EMA"
        ));
    }

    #[test]
    fn registration_rejects_alias_conflicts_atomically() {
        let mut operations = OperationRegistry::new();
        let first = OperationSpec {
            name: "FIRST".to_string(),
            aliases: vec!["one".to_string()],
            kind: OperationKind::Indicator,
            value_shape: ValueShape::Series,
            category: None,
            input: None,
            params: Vec::new(),
            outputs: 1,
            output_names: vec!["FIRST".to_string()],
            lookback: LookbackSpec::None,
            capabilities: OperationCapabilities::indicator(false, true),
            schema_version: 1,
        };
        operations.register(first).unwrap();

        let conflicting = OperationSpec {
            name: "SECOND".to_string(),
            aliases: vec!["ONE".to_string(), "two".to_string()],
            kind: OperationKind::Composite,
            value_shape: ValueShape::MultiSeries,
            category: None,
            input: None,
            params: Vec::new(),
            outputs: 2,
            output_names: vec!["SECOND_1".to_string(), "SECOND_2".to_string()],
            lookback: LookbackSpec::Dynamic,
            capabilities: OperationCapabilities::indicator(false, true),
            schema_version: 1,
        };

        assert!(matches!(
            operations.register(conflicting),
            Err(OperationRegistryError::DuplicateAlias(_))
        ));
        assert_eq!(operations.len(), 1);
        assert!(operations.get("SECOND").is_none());
        assert!(operations.get("TWO").is_none());
    }

    fn formula_context() -> FormulaContext {
        let values = |start| Array1::from_vec((0..6).map(|index| start + index as f64).collect());
        FormulaContext::new(
            values(1.0),
            values(2.0),
            values(0.0),
            values(10.0),
            values(100.0),
            None,
        )
    }

    fn assert_series_equal(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len());
        for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (actual.is_nan() && expected.is_nan()) || (actual - expected).abs() <= 1e-12,
                "series mismatch at {index}: {actual} vs {expected}"
            );
        }
    }

    #[test]
    fn unified_engine_executes_formula_and_preserves_primary_result() {
        let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
        let mut context = formula_context();
        let result = engine
            .execute(OperationRequest::Formula {
                source: "SMA(CLOSE, 3)",
                dialect: FormulaDialect::AlphaTA,
                context: &mut context,
            })
            .unwrap();

        assert_eq!(result.primary.as_deref(), Some(PRIMARY_OUTPUT_NAME));
        assert_eq!(result.primary_values().unwrap().len(), 6);
        assert!(result.draw.is_none());
    }

    #[test]
    fn unified_engine_normalizes_source_through_the_selected_domestic_dialect() {
        let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
        let mut context = formula_context();
        let result = engine
            .execute(OperationRequest::Formula {
                source: "\u{feff}X:=CLOSE;\r\nX",
                dialect: FormulaDialect::TongDaXin,
                context: &mut context,
            })
            .unwrap();

        assert_eq!(
            result.primary_values().unwrap(),
            &[10.0, 11.0, 12.0, 13.0, 14.0, 15.0]
        );
        assert_eq!(result.primary.as_deref(), Some(PRIMARY_OUTPUT_NAME));
    }

    #[test]
    fn unified_engine_dispatches_registered_indicator_without_source_parsing() {
        let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
        let mut context = formula_context();
        let inputs = ["CLOSE"];
        let params = [2.0];
        let result = engine
            .execute(OperationRequest::Indicator {
                name: "ema",
                inputs: &inputs,
                params: &params,
                context: &mut context,
            })
            .unwrap();

        assert_eq!(result.shape, ValueShape::Series);
        assert_eq!(result.primary_values().unwrap().len(), 6);
        assert!(result
            .primary_values()
            .unwrap()
            .iter()
            .any(|value| value.is_finite()));
        let direct = result.primary_values().unwrap().to_vec();
        let mut formula_context = formula_context();
        let formula = engine
            .execute(OperationRequest::Formula {
                source: "EMA(CLOSE, 2)",
                dialect: FormulaDialect::AlphaTA,
                context: &mut formula_context,
            })
            .unwrap();
        for (actual, expected) in direct.iter().zip(formula.primary_values().unwrap()) {
            assert!(
                (actual.is_nan() && expected.is_nan()) || (actual - expected).abs() <= 1e-12,
                "direct and formula dispatch differ: {actual} vs {expected}"
            );
        }
    }

    #[test]
    fn unified_engine_preserves_macd_named_multi_outputs() {
        let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
        let mut context = formula_context();
        let inputs = ["CLOSE"];
        let params = [2.0, 3.0, 2.0];
        let result = engine
            .execute(OperationRequest::Indicator {
                name: "MACD",
                inputs: &inputs,
                params: &params,
                context: &mut context,
            })
            .unwrap();

        assert_eq!(result.shape, ValueShape::MultiSeries);
        assert_eq!(result.primary.as_deref(), Some("MACD"));
        assert_eq!(result.len(), 3);
        assert!(result.get("MACD").is_some());
        assert!(result.get("MACD_SIGNAL").is_some());
        assert!(result.get("MACD_HIST").is_some());
        let expected = crate::indicators::momentum::macd(&context.close, 2, 3, 2)
            .unwrap()
            .macd;
        assert_series_equal(
            result.primary_values().unwrap(),
            expected.as_slice().unwrap(),
        );
    }

    #[test]
    fn unified_engine_preserves_bbands_named_multi_outputs() {
        let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
        let mut context = formula_context();
        let inputs = ["CLOSE"];
        let params = [3.0, 2.0];
        let result = engine
            .execute(OperationRequest::Indicator {
                name: "BBANDS",
                inputs: &inputs,
                params: &params,
                context: &mut context,
            })
            .unwrap();

        assert_eq!(result.shape, ValueShape::MultiSeries);
        assert_eq!(result.primary.as_deref(), Some("MIDDLEBAND"));
        assert_eq!(result.len(), 3);
        let expected = crate::indicators::overlap::bbands(&context.close, 3, 2.0, 2.0).unwrap();
        assert_series_equal(
            result.get("UPPERBAND").unwrap(),
            expected.upper.as_slice().unwrap(),
        );
        assert_series_equal(
            result.get("MIDDLEBAND").unwrap(),
            expected.middle.as_slice().unwrap(),
        );
        assert_series_equal(
            result.get("LOWERBAND").unwrap(),
            expected.lower.as_slice().unwrap(),
        );
    }

    #[test]
    fn unified_engine_routes_pine_subset_to_the_same_result_contract() {
        let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
        let mut context = formula_context();
        let result = engine
            .execute(OperationRequest::Formula {
                source: "//@version=5\nindicator(\"M\")\nr = ta.sma(close, 3)\nplot(r)\n",
                dialect: FormulaDialect::Pine,
                context: &mut context,
            })
            .unwrap();

        assert!(result.primary_values().is_some());
        assert!(result.get("PLOT").is_some());
        assert!(result.draw.is_none());
    }

    #[test]
    fn unified_engine_routes_pine_security_through_the_same_formula_engine() {
        struct TestSecurityResolver;

        impl crate::formula::PineSecurityResolver for TestSecurityResolver {
            fn resolve_security(
                &self,
                args: &[(Option<String>, crate::formula::PineAstNode)],
            ) -> Result<crate::formula::AstNode, crate::formula::PineMapperError> {
                assert_eq!(args.len(), 3);
                Ok(crate::formula::AstNode::Variable("__HTF".to_string()))
            }
        }

        let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
        let mut context = formula_context();
        context.variables.insert(
            Arc::from("__HTF"),
            Array1::from_vec(vec![20.0, 20.0, 30.0, 30.0, 40.0, 40.0]),
        );
        let result = engine
            .execute(OperationRequest::FormulaWithPineSecurity {
                source: "//@version=5\nindicator(\"HTF\")\nhtf = request.security(\"AAA\", \"D\", close)\nhtf",
                context: &mut context,
                resolver: &TestSecurityResolver,
            })
            .unwrap();

        assert_eq!(result.shape, ValueShape::MultiSeries);
        assert_eq!(
            result.primary_values().unwrap(),
            &[20.0, 20.0, 30.0, 30.0, 40.0, 40.0]
        );
    }

    #[test]
    fn panel_formula_keeps_symbol_and_timeframe_results_separate() {
        let open_a = [1.0, 2.0, 3.0];
        let close_a = [2.0, 3.0, 4.0];
        let open_b = [10.0, 12.0, 14.0];
        let close_b = [11.0, 13.0, 15.0];
        let timestamps = [100, 200, 300];
        let frame_a = MarketFrame::new(&open_a, &close_a, &open_a, &close_a, &open_a)
            .unwrap()
            .with_timestamp(&timestamps)
            .unwrap();
        let frame_b = MarketFrame::new(&open_b, &close_b, &open_b, &close_b, &open_b)
            .unwrap()
            .with_timestamp(&timestamps)
            .unwrap();
        let mut panel = MarketPanel::new();
        let key_a = FrameKey::new("AAA", "1d").unwrap();
        let key_b = FrameKey::new("BBB", "5m").unwrap();
        panel.insert(key_a.clone(), frame_a).unwrap();
        panel.insert(key_b.clone(), frame_b).unwrap();

        let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
        let result = engine
            .execute_panel_formula("SMA(CLOSE, 2)", FormulaDialect::AlphaTA, &panel)
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result.get(&key_a).unwrap().shape, ValueShape::Series);
        assert_eq!(result.get(&key_b).unwrap().shape, ValueShape::Series);
        let values_a = result.get(&key_a).unwrap().primary_values().unwrap();
        let values_b = result.get(&key_b).unwrap().primary_values().unwrap();
        assert_eq!(values_a, &[2.0, 2.5, 3.25]);
        assert_eq!(values_b, &[11.0, 12.0, 13.5]);

        let inputs = ["CLOSE"];
        let params = [2.0];
        let indicator = engine
            .execute_panel_indicator("EMA", &inputs, &params, &panel)
            .unwrap();
        assert_eq!(indicator.len(), 2);
        let indicator_a = indicator.get(&key_a).unwrap().primary_values().unwrap();
        let indicator_b = indicator.get(&key_b).unwrap().primary_values().unwrap();
        assert!(indicator_a[0].is_nan());
        assert!(indicator_b[0].is_nan());
        assert_eq!(&indicator_a[1..], &[2.5, 3.5]);
        assert_eq!(&indicator_b[1..], &[12.0, 14.0]);
    }

    #[test]
    fn cached_panel_formula_isolated_by_frame_and_revision() {
        let values_a = [1.0, 2.0, 3.0];
        let values_b = [10.0, 20.0, 30.0];
        let timestamps = [100, 200, 300];
        let frame_a = MarketFrame::new(&values_a, &values_a, &values_a, &values_a, &values_a)
            .unwrap()
            .with_timestamp(&timestamps)
            .unwrap();
        let frame_b = MarketFrame::new(&values_b, &values_b, &values_b, &values_b, &values_b)
            .unwrap()
            .with_timestamp(&timestamps)
            .unwrap();
        let key_a = FrameKey::new("AAA", "1d").unwrap();
        let key_b = FrameKey::new("BBB", "1d").unwrap();
        let mut panel = MarketPanel::new();
        panel.insert(key_a.clone(), frame_a).unwrap();
        panel.insert(key_b.clone(), frame_b).unwrap();

        let mut engine = UnifiedOperationEngine::with_cache_capacity(FactorRegistry::new(), 8);
        let first = engine
            .execute_panel_formula_cached("CLOSE + 1", FormulaDialect::AlphaTA, &panel, 7)
            .unwrap();
        assert_eq!(first.get(&key_a).unwrap().primary_values().unwrap()[0], 2.0);
        assert_eq!(
            first.get(&key_b).unwrap().primary_values().unwrap()[0],
            11.0
        );
        assert_eq!(engine.cache_stats().misses, 2);
        assert_eq!(engine.cache_stats().hits, 0);

        let second = engine
            .execute_panel_formula_cached("CLOSE + 1", FormulaDialect::AlphaTA, &panel, 7)
            .unwrap();
        assert_eq!(
            second.get(&key_a).unwrap().primary_values().unwrap()[2],
            4.0
        );
        assert_eq!(
            second.get(&key_b).unwrap().primary_values().unwrap()[2],
            31.0
        );
        assert_eq!(engine.cache_stats().hits, 2);
        assert_eq!(engine.cache_stats().entries, 2);

        engine
            .execute_panel_formula_cached("CLOSE + 1", FormulaDialect::AlphaTA, &panel, 8)
            .unwrap();
        assert_eq!(engine.cache_stats().misses, 4);
        assert_eq!(engine.cache_stats().entries, 4);
    }

    #[test]
    fn cached_panel_formula_uses_recently_used_eviction_order() {
        let values = [1.0, 2.0, 3.0];
        let timestamps = [100, 200, 300];
        let frame = MarketFrame::new(&values, &values, &values, &values, &values)
            .unwrap()
            .with_timestamp(&timestamps)
            .unwrap();
        let key = FrameKey::new("AAA", "1d").unwrap();
        let mut panel = MarketPanel::new();
        panel.insert(key, frame).unwrap();

        let mut engine = UnifiedOperationEngine::with_cache_capacity(FactorRegistry::new(), 2);
        engine
            .execute_panel_formula_cached("CLOSE + 1", FormulaDialect::AlphaTA, &panel, 1)
            .unwrap();
        engine
            .execute_panel_formula_cached("CLOSE + 2", FormulaDialect::AlphaTA, &panel, 1)
            .unwrap();

        // Touch the first entry so the second entry becomes the LRU entry.
        engine
            .execute_panel_formula_cached("CLOSE + 1", FormulaDialect::AlphaTA, &panel, 1)
            .unwrap();
        engine
            .execute_panel_formula_cached("CLOSE + 3", FormulaDialect::AlphaTA, &panel, 1)
            .unwrap();

        let misses_before = engine.cache_stats().misses;
        engine
            .execute_panel_formula_cached("CLOSE + 1", FormulaDialect::AlphaTA, &panel, 1)
            .unwrap();
        assert_eq!(
            engine.cache_stats().misses,
            misses_before,
            "recently-used cache entry should survive eviction"
        );

        engine
            .execute_panel_formula_cached("CLOSE + 2", FormulaDialect::AlphaTA, &panel, 1)
            .unwrap();
        assert_eq!(engine.cache_stats().misses, misses_before + 1);
    }

    #[test]
    fn formula_expands_fundamentals_using_point_in_time_as_of() {
        let open = [1.0, 2.0, 3.0];
        let timestamps = [100, 200, 300];
        let frame = MarketFrame::new(&open, &open, &open, &open, &open)
            .unwrap()
            .with_timestamp(&timestamps)
            .unwrap();
        let fundamental_timestamps = [150, 280];
        let fundamental_values = [10.0, 20.0];
        let fundamental =
            FundamentalSeries::new("earnings_ttm", &fundamental_timestamps, &fundamental_values)
                .unwrap();

        let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
        let result = engine
            .execute_formula_with_fundamentals(
                "EARNINGS_TTM",
                FormulaDialect::AlphaTA,
                &frame,
                &[fundamental],
            )
            .unwrap();
        let actual = result.primary_values().unwrap();
        assert!(actual[0].is_nan());
        assert_eq!(&actual[1..], &[10.0, 20.0]);
    }

    #[test]
    fn formula_fundamentals_require_frame_timestamps() {
        let values = [1.0, 2.0];
        let frame = MarketFrame::new(&values, &values, &values, &values, &values).unwrap();
        let fundamental = FundamentalSeries::new("book_value", &[10], &[3.0]).unwrap();
        let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
        assert!(matches!(
            engine.execute_formula_with_fundamentals(
                "BOOK_VALUE",
                FormulaDialect::AlphaTA,
                &frame,
                &[fundamental],
            ),
            Err(OperationExecutionError::InvalidRequest(message))
                if message.contains("timestamps")
        ));
    }

    #[test]
    fn temporal_formula_inputs_use_only_closed_source_values() {
        let values = [10.0, 11.0, 12.0];
        let target_timestamps = [10, 15, 20];
        let frame = MarketFrame::new(&values, &values, &values, &values, &values)
            .unwrap()
            .with_timestamp(&target_timestamps)
            .unwrap();
        let source_timestamps = [10, 20];
        let source_values = [100.0, 200.0];
        let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
        let result = engine
            .execute_formula_with_temporal_inputs(
                "HIGHER + CLOSE",
                FormulaDialect::AlphaTA,
                &frame,
                &[(
                    "higher",
                    &source_timestamps,
                    &source_values,
                    TemporalAlignment::AsOfClosed,
                )],
            )
            .unwrap();

        assert_eq!(result.primary_values().unwrap(), &[110.0, 111.0, 212.0]);
    }

    #[test]
    fn temporal_formula_inputs_require_target_timestamps() {
        let values = [1.0, 2.0];
        let frame = MarketFrame::new(&values, &values, &values, &values, &values).unwrap();
        let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
        assert!(matches!(
            engine.execute_formula_with_temporal_inputs(
                "HIGHER",
                FormulaDialect::AlphaTA,
                &frame,
                &[("higher", &[10], &[3.0], TemporalAlignment::Exact)],
            ),
            Err(OperationExecutionError::InvalidRequest(message))
                if message.contains("timestamps")
        ));
    }

    #[test]
    fn unified_engine_routes_factor_and_composite_through_existing_engines() {
        let mut factors = FactorRegistry::new();
        factors
            .register(FactorDefinition::new(
                "DOUBLE_CLOSE",
                ["close"],
                FactorKind::TimeSeries,
                FactorDirection::Neutral,
                Arc::new(|inputs| {
                    Ok(inputs
                        .get("close")?
                        .iter()
                        .map(|value| value * 2.0)
                        .collect())
                }),
            ))
            .unwrap();
        factors
            .register(FactorDefinition::new(
                "ROW_SHIFT",
                ["score"],
                FactorKind::CrossSectional,
                FactorDirection::HigherBetter,
                Arc::new(|inputs| {
                    Ok(inputs
                        .get("score")?
                        .iter()
                        .map(|value| value + 1.0)
                        .collect())
                }),
            ))
            .unwrap();
        let mut engine = UnifiedOperationEngine::new(factors);
        let close = [1.0, 2.0, 3.0];
        let context = BorrowedFactorContext::new()
            .with_series("close", &close)
            .unwrap();

        let factor = engine
            .execute(OperationRequest::Factor {
                name: "DOUBLE_CLOSE",
                context: &context,
                data_revision: None,
                cache_scope: None,
            })
            .unwrap();
        assert_eq!(factor.primary_values().unwrap(), &[2.0, 4.0, 6.0]);
        assert_eq!(engine.factor_plan_cache_hits, 0);
        assert_eq!(engine.factor_plan_cache_misses, 1);
        let factor_again = engine
            .execute(OperationRequest::Factor {
                name: "DOUBLE_CLOSE",
                context: &context,
                data_revision: None,
                cache_scope: None,
            })
            .unwrap();
        assert_eq!(factor_again.primary_values().unwrap(), &[2.0, 4.0, 6.0]);
        assert_eq!(engine.factor_plans.len(), 1);
        assert_eq!(engine.factor_plan_cache_hits, 1);
        assert_eq!(engine.factor_plan_cache_misses, 1);

        let cached = engine
            .execute(OperationRequest::Factor {
                name: "DOUBLE_CLOSE",
                context: &context,
                data_revision: Some(1),
                cache_scope: Some("AAA@1d"),
            })
            .unwrap();
        assert_eq!(cached.primary_values().unwrap(), &[2.0, 4.0, 6.0]);
        assert_eq!(engine.cache_stats().misses, 1);
        let cached_again = engine
            .execute(OperationRequest::Factor {
                name: "DOUBLE_CLOSE",
                context: &context,
                data_revision: Some(1),
                cache_scope: Some("AAA@1d"),
            })
            .unwrap();
        assert_eq!(cached_again.primary_values().unwrap(), &[2.0, 4.0, 6.0]);
        assert_eq!(engine.cache_stats().hits, 1);
        engine
            .execute(OperationRequest::Factor {
                name: "DOUBLE_CLOSE",
                context: &context,
                data_revision: Some(1),
                cache_scope: Some("BBB@1d"),
            })
            .unwrap();
        engine
            .execute(OperationRequest::Factor {
                name: "DOUBLE_CLOSE",
                context: &context,
                data_revision: Some(2),
                cache_scope: Some("AAA@1d"),
            })
            .unwrap();
        assert_eq!(engine.cache_stats().misses, 3);

        let timestamps = [10, 20];
        let symbols = ["AAA", "BBB", "CCC"];
        let scores = [1.0, 3.0, 2.0, 5.0, 4.0, 6.0];
        let view = CrossSectionView::new(&timestamps, &symbols, &scores).unwrap();
        let cross_inputs = [("score", &view)];
        let cross = engine
            .execute(OperationRequest::CrossSectionalFactor {
                name: "ROW_SHIFT",
                inputs: &cross_inputs,
            })
            .unwrap();
        assert_eq!(cross.shape, ValueShape::CrossSection);
        assert_eq!(
            cross.primary_values().unwrap(),
            &[2.0, 4.0, 3.0, 6.0, 5.0, 7.0]
        );

        let definitions = [CompositeDefinition::new(
            "SUM_CLOSE",
            CompositeExpr::Op {
                op: CompositeOp::Add,
                inputs: vec![CompositeExpr::series("close"), CompositeExpr::Constant(1.0)],
            },
        )];
        let outputs = ["SUM_CLOSE"];
        let composite = engine
            .execute(OperationRequest::Composite {
                definitions: &definitions,
                outputs: &outputs,
                context: &context,
                data_revision: Some(1),
                cache_scope: Some("AAA@1d"),
            })
            .unwrap();
        assert_eq!(composite.primary_values().unwrap(), &[2.0, 3.0, 4.0]);
        let composite_again = engine
            .execute(OperationRequest::Composite {
                definitions: &definitions,
                outputs: &outputs,
                context: &context,
                data_revision: Some(1),
                cache_scope: Some("AAA@1d"),
            })
            .unwrap();
        assert_eq!(composite_again.primary_values().unwrap(), &[2.0, 3.0, 4.0]);
        assert_eq!(engine.composite_engine().compiled_plan_count(), 1);
        assert_eq!(engine.composite_cache_stats().hits, 1);
        assert_eq!(engine.composite_cache_stats().misses, 1);
        assert_eq!(engine.composite_cache_stats().entries, 1);
    }

    #[test]
    fn unified_engine_prepares_composite_stream_from_registered_runtime() {
        let definitions = [CompositeDefinition::new(
            "sum",
            CompositeExpr::Op {
                op: CompositeOp::Add,
                inputs: vec![CompositeExpr::series("close"), CompositeExpr::Constant(1.0)],
            },
        )];
        let outputs = ["sum"];
        let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
        let (plan, stream_engine) = engine
            .prepare_composite_stream(&definitions, &outputs)
            .unwrap();

        assert_eq!(engine.composite_engine().compiled_plan_count(), 1);
        assert_eq!(plan.required_raw_inputs(), &["close".to_string()]);
        let mut stream = plan.stream(stream_engine).unwrap();
        let emitted = stream
            .push_batch(&BTreeMap::from([(
                "close".to_string(),
                vec![1.0, 2.0, 3.0],
            )]))
            .unwrap();
        assert_eq!(stream.rows(), 3);
        assert_eq!(emitted["sum"], vec![2.0, 3.0, 4.0]);
    }

    #[test]
    fn unified_engine_prepares_factor_stream_from_registered_runtime() {
        let mut engine = UnifiedOperationEngine::new(builtin_factor_registry());
        let (plan, stream_engine) = engine.prepare_factor_stream(&["momentum_5"]).unwrap();

        assert_eq!(engine.factor_plan_cache_misses, 1);
        assert_eq!(plan.targets(), &["momentum_5".to_string()]);
        let mut stream = plan.stream(stream_engine).unwrap();
        let emitted = stream
            .push_batch(&BTreeMap::from([(
                "close".to_string(),
                vec![10.0, 11.0, 12.0, 13.0, 14.0, 20.0],
            )]))
            .unwrap();
        assert_eq!(stream.rows(), 6);
        assert!(emitted["momentum_5"][0..5]
            .iter()
            .all(|value| value.is_nan()));
        assert_eq!(emitted["momentum_5"][5], 1.0);
    }

    #[test]
    fn unified_engine_routes_factor_and_composite_range_execution() {
        let close = [10.0, 11.0, 12.0, 13.0, 14.0, 20.0, 16.0];
        let context = BorrowedFactorContext::new()
            .with_series("close", &close)
            .unwrap();

        let mut engine = UnifiedOperationEngine::new(builtin_factor_registry());
        let previous_factor = BTreeMap::from([("momentum_5".to_string(), vec![0.0; close.len()])]);
        let (factor, factor_trace) = engine
            .execute_factor_range(
                "momentum_5",
                &context,
                &previous_factor,
                DirtyRange::new(5, 6),
            )
            .unwrap();
        assert_eq!(factor.primary_values().unwrap()[5], 1.0);
        assert_eq!(
            factor_trace.mode,
            RuntimeExecutionMode::Range {
                input_dirty: DirtyRange::new(5, 6),
                affected: DirtyRange::new(5, 7),
                recompute: DirtyRange::new(0, 7),
            }
        );

        let definitions = [CompositeDefinition::new(
            "sum",
            CompositeExpr::Op {
                op: CompositeOp::Add,
                inputs: vec![CompositeExpr::series("close"), CompositeExpr::Constant(1.0)],
            },
        )];
        let outputs = ["sum"];
        let previous_composite = BTreeMap::from([("sum".to_string(), vec![0.0; close.len()])]);
        let (composite, composite_trace) = engine
            .execute_composite_range(
                &definitions,
                &outputs,
                &context,
                &previous_composite,
                DirtyRange::new(5, 6),
            )
            .unwrap();
        assert_eq!(composite.primary_values().unwrap()[5], 21.0);
        assert_eq!(
            composite_trace.mode,
            RuntimeExecutionMode::Range {
                input_dirty: DirtyRange::new(5, 6),
                affected: DirtyRange::new(5, 6),
                recompute: DirtyRange::new(5, 6),
            }
        );
    }

    #[test]
    fn multi_target_factor_execution_shares_dependencies_and_batch_cache() {
        let base_calls = Arc::new(AtomicUsize::new(0));
        let base_calls_for_factor = base_calls.clone();
        let mut factors = FactorRegistry::new();
        factors
            .register(FactorDefinition::new(
                "BASE",
                ["close"],
                FactorKind::TimeSeries,
                FactorDirection::Neutral,
                Arc::new(move |inputs| {
                    base_calls_for_factor.fetch_add(1, Ordering::SeqCst);
                    Ok(inputs
                        .get("close")?
                        .iter()
                        .map(|value| value * 2.0)
                        .collect())
                }),
            ))
            .unwrap();
        factors
            .register(FactorDefinition::new(
                "LEFT",
                ["BASE"],
                FactorKind::TimeSeries,
                FactorDirection::Neutral,
                Arc::new(|inputs| {
                    Ok(inputs
                        .get("BASE")?
                        .iter()
                        .map(|value| value + 1.0)
                        .collect())
                }),
            ))
            .unwrap();
        factors
            .register(FactorDefinition::new(
                "RIGHT",
                ["BASE"],
                FactorKind::TimeSeries,
                FactorDirection::Neutral,
                Arc::new(|inputs| {
                    Ok(inputs
                        .get("BASE")?
                        .iter()
                        .map(|value| value - 1.0)
                        .collect())
                }),
            ))
            .unwrap();

        let close = [1.0, 2.0, 3.0];
        let context = BorrowedFactorContext::new()
            .with_series("close", &close)
            .unwrap();
        let mut engine = UnifiedOperationEngine::with_cache_capacity(factors, 8);
        let targets = ["LEFT", "RIGHT"];
        let first = engine
            .execute_factor_targets(&targets, &context, Some(7), Some("AAA@1d"))
            .unwrap();
        assert_eq!(first.shape, ValueShape::MultiSeries);
        assert_eq!(first.primary, None);
        assert_eq!(first.get("LEFT").unwrap(), &[3.0, 5.0, 7.0]);
        assert_eq!(first.get("RIGHT").unwrap(), &[1.0, 3.0, 5.0]);
        assert_eq!(base_calls.load(Ordering::SeqCst), 1);

        let reversed_targets = ["RIGHT", "LEFT"];
        let second = engine
            .execute_factor_targets(&reversed_targets, &context, Some(7), Some("AAA@1d"))
            .unwrap();
        assert_eq!(second.values, first.values);
        assert_eq!(base_calls.load(Ordering::SeqCst), 1);
        assert_eq!(engine.cache_stats().misses, 1);
        assert_eq!(engine.cache_stats().hits, 1);
        assert_eq!(engine.factor_plan_cache_hits, 0);
        assert_eq!(engine.factor_plan_cache_misses, 1);
    }
}
