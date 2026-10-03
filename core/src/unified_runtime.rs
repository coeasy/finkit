//! Unified runtime contracts shared by formula, factor, and research layers.
//!
//! This module owns cross-domain execution identity and dirty-range semantics.
//! Domain planners remain responsible for proving that a node is range-safe;
//! once that proof exists, [`UnifiedRuntime`] executes only the required input
//! slice and splices the affected rows into the retained materialization.

use crate::compute::{DependencyShape, FactorPlan};
use crate::factors::{
    BorrowedFactorContext, FactorContext, FactorEngine, FactorError, FactorResult,
};
use crate::runtime_context::{ArtifactKey, ExecutionKind, RuntimeContext};
use crate::semantic_graph::{CseReport, SemanticGraph};
use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;

/// Stable, typed content identity for a materialized artifact.
///
/// The hash deliberately uses a fixed FNV-1a 64-bit algorithm rather than
/// `DefaultHasher`, whose algorithm is not a persistence contract. This makes
/// persisted artifact references deterministic across processes and platforms.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ArtifactHash(u64);

impl ArtifactHash {
    const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    /// Build a typed hash from an already validated raw value.
    #[must_use]
    pub const fn from_u64(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw numeric representation for ABI/legacy adapters.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Compute a deterministic content hash from canonical bytes.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self::empty().chain(bytes)
    }

    /// The identity of the empty byte string, usable as a hash seed.
    ///
    /// Exposed so callers that must hash a structure field by field — a graph,
    /// a plan, a schema — can feed canonical pieces in without first
    /// materializing one large buffer. `from_bytes` is exactly
    /// `empty().chain(bytes)`, so the two spellings agree by construction.
    #[must_use]
    pub const fn empty() -> Self {
        Self(Self::FNV_OFFSET_BASIS)
    }

    /// Continue an existing hash state with more canonical bytes.
    ///
    /// FNV-1a is a byte-at-a-time streaming hash, so chaining is not an
    /// approximation of hashing the concatenation — it *is* hashing the
    /// concatenation:
    ///
    /// ```text
    /// a.chain(b) == ArtifactHash::from_bytes(&[a, b].concat())
    /// ```
    ///
    /// The caller is responsible for the separators that keep the encoding
    /// unambiguous; `[b"ab"]` and `[b"a", b"b"]` deliberately hash the same,
    /// because the byte streams are the same.
    #[must_use]
    pub fn chain(self, bytes: &[u8]) -> Self {
        let mut state = self.0;
        for byte in bytes {
            state ^= u64::from(*byte);
            state = state.wrapping_mul(Self::FNV_PRIME);
        }
        Self(state)
    }
}

impl fmt::Display for ArtifactHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl fmt::LowerHex for ArtifactHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

/// Half-open row range invalidated by a data change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DirtyRange {
    /// First dirty row, inclusive.
    pub start: usize,
    /// First clean row after the dirty region, exclusive.
    pub end: usize,
}

impl DirtyRange {
    /// Build a normalized half-open dirty range.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }

    /// A dirty range covering every row.
    #[must_use]
    pub const fn full(rows: usize) -> Self {
        Self {
            start: 0,
            end: rows,
        }
    }

    /// Whether no rows are dirty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Number of dirty rows.
    #[must_use]
    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Whether the range is valid for an input with `rows` rows.
    #[must_use]
    pub const fn is_within(self, rows: usize) -> bool {
        self.start <= self.end && self.end <= rows
    }

    /// Convert to a standard Rust range.
    #[must_use]
    pub const fn as_range(self) -> Range<usize> {
        self.start..self.end
    }

    /// Expand backwards to provide historical rows required by an output
    /// interval. This does not change the output interval itself.
    #[must_use]
    pub const fn with_lookback(self, lookback: usize) -> Self {
        Self {
            start: self.start.saturating_sub(lookback),
            end: self.end,
        }
    }

    /// Propagate an input mutation forward through a trailing-window dependency.
    ///
    /// If a node needs `lookback` previous rows, mutating input row `i` can
    /// affect outputs from `i` through `i + lookback`. Dependency-chain
    /// lookbacks therefore expand the dirty range's end before execution.
    #[must_use]
    pub const fn propagate_forward(self, lookback: usize, rows: usize) -> Self {
        if self.is_empty() {
            return self;
        }
        let expanded_end = self.end.saturating_add(lookback);
        Self {
            start: self.start,
            end: if expanded_end < rows {
                expanded_end
            } else {
                rows
            },
        }
    }

    /// Merge two invalidation ranges conservatively.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        Self {
            start: if self.start < other.start {
                self.start
            } else {
                other.start
            },
            end: if self.end > other.end {
                self.end
            } else {
                other.end
            },
        }
    }
}

/// Runtime path used for one plan execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeExecutionMode {
    /// Every row was recomputed.
    Full,
    /// Only rows affected by one input dirty range were evaluated.
    Range {
        /// Rows changed in the authoritative raw input.
        input_dirty: DirtyRange,
        /// Output interval affected after dependency propagation.
        affected: DirtyRange,
        /// Input slice actually evaluated after historical lookback expansion.
        recompute: DirtyRange,
    },
}

/// Evidence emitted by the unified runtime for correctness/performance gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeExecutionTrace {
    /// Selected execution path.
    pub mode: RuntimeExecutionMode,
    /// Total rows in the authoritative input.
    pub rows: usize,
    /// Number of factor nodes executed.
    pub executed_nodes: usize,
    /// Number of rows evaluated by each executed node.
    pub recomputed_rows: usize,
}

/// Typed runtime result carrying both values and execution evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeExecution<T> {
    /// Materialized plan outputs.
    pub output: T,
    /// Runtime execution evidence.
    pub trace: RuntimeExecutionTrace,
}

/// Canonical plan execution boundary.
///
/// The factor implementation is the first domain wired through this boundary.
/// Formula and research planners can use the same contracts without creating a
/// second `DirtyRange` or artifact identity model.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnifiedRuntime;

impl UnifiedRuntime {
    /// Execute a factor plan over owned aligned input.
    pub fn execute_factor_plan(
        plan: &FactorPlan,
        engine: &FactorEngine,
        context: &FactorContext,
    ) -> FactorResult<RuntimeExecution<BTreeMap<String, Vec<f64>>>> {
        let output = plan.execute(engine, context)?;
        Ok(RuntimeExecution {
            output,
            trace: RuntimeExecutionTrace {
                mode: RuntimeExecutionMode::Full,
                rows: context.len(),
                executed_nodes: plan.execution_order().len(),
                recomputed_rows: context.len(),
            },
        })
    }

    /// Execute a factor plan over zero-copy borrowed aligned input.
    pub fn execute_factor_plan_borrowed(
        plan: &FactorPlan,
        engine: &FactorEngine,
        context: &BorrowedFactorContext<'_>,
    ) -> FactorResult<RuntimeExecution<BTreeMap<String, Vec<f64>>>> {
        let output = plan.execute_borrowed(engine, context)?;
        Ok(RuntimeExecution {
            output,
            trace: RuntimeExecutionTrace {
                mode: RuntimeExecutionMode::Full,
                rows: context.len(),
                executed_nodes: plan.execution_order().len(),
                recomputed_rows: context.len(),
            },
        })
    }

    /// Recompute only rows affected by a proven-safe raw-input dirty interval.
    ///
    /// `lookback` is supplied by the semantic planner after accumulating the
    /// historical requirements of the complete dependency chain. The runtime
    /// first propagates the raw change forward through that dependency chain,
    /// then expands backwards only as far as needed to evaluate those outputs.
    pub fn execute_factor_plan_range_borrowed(
        plan: &FactorPlan,
        engine: &FactorEngine,
        context: &BorrowedFactorContext<'_>,
        previous: &BTreeMap<String, Vec<f64>>,
        dirty: DirtyRange,
        lookback: usize,
    ) -> FactorResult<RuntimeExecution<BTreeMap<String, Vec<f64>>>> {
        let mut output = previous.clone();
        let trace = Self::execute_factor_plan_range_into_borrowed(
            plan,
            engine,
            context,
            &mut output,
            dirty,
            lookback,
        )?;
        Ok(RuntimeExecution { output, trace })
    }

    /// In-place dirty-range execution that preserves all unaffected rows and
    /// their allocations.
    pub fn execute_factor_plan_range_into_borrowed(
        plan: &FactorPlan,
        engine: &FactorEngine,
        context: &BorrowedFactorContext<'_>,
        output: &mut BTreeMap<String, Vec<f64>>,
        dirty: DirtyRange,
        lookback: usize,
    ) -> FactorResult<RuntimeExecutionTrace> {
        plan.validate_borrowed_context(context)?;
        let rows = context.len();
        if !dirty.is_within(rows) {
            return Err(FactorError::InvalidParameter(format!(
                "dirty range {}..{} exceeds factor input rows {rows}",
                dirty.start, dirty.end
            )));
        }
        Self::validate_retained_outputs(plan, output, rows)?;

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
        for input in plan.required_raw_inputs() {
            let values = context
                .get(input)
                .ok_or_else(|| FactorError::MissingInput(input.clone()))?;
            sliced.insert(input.clone(), &values[recompute.as_range()])?;
        }

        let partial = plan.execute_borrowed(engine, &sliced)?;
        let offset = affected.start - recompute.start;
        let source_end = offset + affected.len();
        for name in plan.execution_order() {
            let source = partial.get(name).ok_or_else(|| {
                FactorError::InvalidParameter(format!(
                    "factor runtime did not materialize planned node {name}"
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
            executed_nodes: plan.execution_order().len(),
            recomputed_rows: recompute.len(),
        })
    }

    fn validate_retained_outputs(
        plan: &FactorPlan,
        output: &BTreeMap<String, Vec<f64>>,
        rows: usize,
    ) -> FactorResult<()> {
        for name in plan.execution_order() {
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
        Ok(())
    }
}

// ─────────────── §19: the same context for every executor ───────────────
//
// `UnifiedExecutor` (the compiled-plan path) already runs against a
// `RuntimeContext`. The factor path used to run against nothing at all: it
// allocated its own scratch, kept no counters, and had no way to refuse work
// that exceeded a caller's budget. These entry points close that gap so an
// application can share one context across formula, factor and research work
// and read one set of numbers back.
impl UnifiedRuntime {
    /// Execute a factor plan over zero-copy borrowed input, recording into a
    /// shared runtime context.
    pub fn execute_factor_plan_borrowed_with_context(
        plan: &FactorPlan,
        engine: &FactorEngine,
        context: &BorrowedFactorContext<'_>,
        runtime: &mut RuntimeContext,
    ) -> FactorResult<RuntimeExecution<BTreeMap<String, Vec<f64>>>> {
        let execution = Self::execute_factor_plan_borrowed(plan, engine, context)?;
        let mut executed_nodes = 0usize;
        for name in plan.execution_order() {
            if execution.output.contains_key(name) {
                executed_nodes += 1;
            }
        }
        Self::account(runtime, ExecutionKind::Full, executed_nodes, context.len());
        Ok(execution)
    }

    /// Recompute only rows affected by a dirty interval, recording into a shared
    /// runtime context, with range eligibility taken from the typed
    /// [`DependencyShape`] rather than a bare lookback.
    ///
    /// A shape that does not prove a fixed lookback is refused outright. This is
    /// the §17 rule enforced at the entry point instead of inside each kernel:
    /// a kernel that decided "I am probably range-safe" could silently return
    /// stale rows, while a refusal only costs the caller a full recompute.
    pub fn execute_factor_plan_range_into_borrowed_with_context(
        plan: &FactorPlan,
        engine: &FactorEngine,
        context: &BorrowedFactorContext<'_>,
        output: &mut BTreeMap<String, Vec<f64>>,
        dirty: DirtyRange,
        dependency: DependencyShape,
        runtime: &mut RuntimeContext,
    ) -> FactorResult<RuntimeExecutionTrace> {
        let lookback = dependency.fixed_lookback().ok_or_else(|| {
            FactorError::InvalidParameter(format!(
                "dirty-range execution requires a proven fixed lookback, got {}",
                dependency.label()
            ))
        })?;
        let trace = Self::execute_factor_plan_range_into_borrowed(
            plan, engine, context, output, dirty, lookback,
        )?;
        Self::account(
            runtime,
            ExecutionKind::Range,
            trace.executed_nodes,
            trace.recomputed_rows,
        );
        Ok(trace)
    }

    /// Fold a plan's per-node shapes into the one shape the runtime may use.
    ///
    /// This is the function a caller passes the result of to
    /// [`Self::execute_factor_plan_range_into_borrowed_with_context`]. It exists
    /// so "can this plan run on a dirty range?" has exactly one answer, computed
    /// from typed node semantics, instead of each backend re-deriving it from a
    /// bare `lookback` integer it happens to have.
    ///
    /// The conservative direction is the only sound one: a plan is
    /// range-eligible only when *every* node proves a fixed lookback. One
    /// expanding node anywhere in the dependency chain invalidates local
    /// recomputation for the whole plan.
    pub fn combine_dependencies(
        shapes: impl IntoIterator<Item = DependencyShape>,
    ) -> DependencyShape {
        DependencyShape::combine_all(shapes)
    }

    fn account(
        runtime: &mut RuntimeContext,
        kind: ExecutionKind,
        executed_nodes: usize,
        recomputed_rows: usize,
    ) {
        runtime.record_execution(kind, executed_nodes);
        runtime.record_kernel_calls(executed_nodes);
        runtime.record_buffers_taken(1);
        runtime.record_buffers_recycled(1);
        runtime.note(format!(
            "factor {} execution: {executed_nodes} node(s) over {recomputed_rows} row(s)",
            kind.label()
        ));
    }
}

/// Which §20 graph optimization to apply before lowering a [`SemanticGraph`].
///
/// This is an explicit argument rather than a default because the two answers
/// produce different plans, and a caller comparing "before" against "after" for
/// a performance claim has to be able to ask for each one deliberately.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum GraphOptimization {
    /// Execute the graph exactly as declared.
    ///
    /// The default: optimization must be opted into, so a caller who has not
    /// thought about it pays no surprise and gets the frontend's own plan.
    #[default]
    None,
    /// Fold duplicated pure sub-computations before lowering (§20, priority 1).
    CommonSubexpressions,
}

impl GraphOptimization {
    /// Human-readable label for diagnostics.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "as-declared",
            Self::CommonSubexpressions => "common-subexpressions",
        }
    }

    /// Whether this setting runs a pass at all.
    #[must_use]
    pub const fn is_optimizing(self) -> bool {
        matches!(self, Self::CommonSubexpressions)
    }
}

/// Why a [`SemanticGraph`] could not be turned into an executable plan.
///
/// Both underlying errors are `Clone + PartialEq + Eq` and are carried through
/// unchanged rather than flattened to a string, so a caller that matches on
/// `ComputePlanError::UnknownFunction` keeps that ability after going through a
/// graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphPlanError {
    /// The graph failed structural validation while lowering.
    Plan(crate::compute::ComputePlanError),
    /// The graph declares no target, so there is nothing to execute.
    ///
    /// A graph with nodes but no target is a frontend bug rather than an empty
    /// computation: the nodes were built, then never claimed as an output.
    NoTarget,
    /// The lowered plan could not be compiled into a numeric hot plan.
    Hot(crate::execution_plan::HotPlanError),
}

impl fmt::Display for GraphPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Plan(error) => write!(f, "semantic graph did not lower: {error}"),
            Self::NoTarget => {
                write!(
                    f,
                    "semantic graph declares no target, so it computes nothing"
                )
            }
            Self::Hot(error) => write!(f, "lowered plan is not hot-compilable: {error}"),
        }
    }
}

impl std::error::Error for GraphPlanError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Plan(error) => Some(error),
            Self::Hot(error) => Some(error),
            Self::NoTarget => None,
        }
    }
}

impl From<crate::compute::ComputePlanError> for GraphPlanError {
    fn from(error: crate::compute::ComputePlanError) -> Self {
        Self::Plan(error)
    }
}

impl From<crate::execution_plan::HotPlanError> for GraphPlanError {
    fn from(error: crate::execution_plan::HotPlanError) -> Self {
        Self::Hot(error)
    }
}

/// What [`UnifiedRuntime::compile_semantic_graph`] produced.
pub struct CompiledSemanticGraph<D: crate::unified_executor::KernelDispatcher> {
    executor: crate::unified_executor::UnifiedExecutor<D>,
    cse: Option<CseReport>,
    dependency: DependencyShape,
}

impl<D: crate::unified_executor::KernelDispatcher> CompiledSemanticGraph<D> {
    /// The executor, ready for `execute` / `execute_range` / `execute_into`.
    ///
    /// Returned by mutable reference rather than by value so the caller keeps
    /// the arena and persistent kernel state across repeated runs, which is
    /// §20's second priority: a scan over ten thousand symbols must not
    /// re-allocate the working set per symbol.
    pub const fn executor_mut(&mut self) -> &mut crate::unified_executor::UnifiedExecutor<D> {
        &mut self.executor
    }

    /// Consume the wrapper and hand over the executor.
    pub fn into_executor(self) -> crate::unified_executor::UnifiedExecutor<D> {
        self.executor
    }

    /// What CSE did, or `None` when no optimization was requested.
    ///
    /// `None` and a report with `merged == 0` are different answers and are kept
    /// distinct: the first says "no pass ran", the second says "a pass ran and
    /// found nothing to fold". Collapsing them would make a caller unable to
    /// tell a frontend that emits clean DAGs from one that was never optimized.
    #[must_use]
    pub const fn cse(&self) -> Option<CseReport> {
        self.cse
    }

    /// Combined dependency shape of the executed graph.
    #[must_use]
    pub const fn dependency(&self) -> DependencyShape {
        self.dependency
    }

    /// Whether the executed graph may be recomputed over a dirty range.
    #[must_use]
    pub const fn can_execute_range(&self) -> bool {
        self.dependency.allows_range_execution()
    }
}

impl<D: crate::unified_executor::KernelDispatcher> fmt::Debug for CompiledSemanticGraph<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Deliberately not derived: the executor owns a dispatcher whose type
        // may not be `Debug`, and dumping the whole arena would bury the two
        // facts a reader actually wants.
        f.debug_struct("CompiledSemanticGraph")
            .field("cse", &self.cse)
            .field("dependency", &self.dependency)
            .field("state_slots", &self.executor.context().states().len())
            .finish()
    }
}

/// The reusable, immutable product of lowering and compiling a graph.
///
/// This — not [`CompiledSemanticGraph`] — is what the artifact cache stores.
/// The compiled graph owns a `UnifiedExecutor`, which in turn owns a buffer
/// arena and persistent kernel state; sharing one across symbols would let a
/// scan's second symbol overwrite the first one's working set. Everything here
/// is plain data, so sharing it is safe and cloning it is cheap.
#[derive(Debug, Clone)]
pub struct CompiledPlanArtifact {
    /// The compiled numeric plan.
    pub plan: crate::execution_plan::HotExecutionPlan,
    /// What CSE did, when an optimizing pass ran.
    pub cse: Option<CseReport>,
    /// Combined dependency shape over the graph that actually lowered.
    pub dependency: DependencyShape,
}

impl UnifiedRuntime {
    /// Cache namespace for a plan compiled from an as-declared graph.
    ///
    /// Spelled as bytes so a reader can confirm the two namespaces differ by
    /// exactly the last digit rather than having to trust two literals.
    const PLAN_NAMESPACE_AS_DECLARED: u64 = 0x4649_4E50_4C41_4E30; // "FINPLAN0"
    /// Cache namespace for a plan compiled with CSE applied.
    ///
    /// A separate namespace rather than a suffix on the key: the two settings
    /// produce genuinely different plans from one declaration, and a shared
    /// namespace would let an optimized compile answer an unoptimized request.
    const PLAN_NAMESPACE_OPTIMIZED: u64 = 0x4649_4E50_4C41_4E31; // "FINPLAN1"

    /// Content key for a compiled plan.
    ///
    /// Keyed on the *declaration*: computing the optimized graph's identity
    /// would mean running CSE merely to decide whether to run CSE, which is the
    /// cost the cache exists to avoid. The optimization setting therefore has
    /// to live in the namespace instead.
    fn plan_cache_key(graph: &SemanticGraph, optimization: GraphOptimization) -> ArtifactKey {
        let namespace = if optimization.is_optimizing() {
            Self::PLAN_NAMESPACE_OPTIMIZED
        } else {
            Self::PLAN_NAMESPACE_AS_DECLARED
        };
        ArtifactKey::new(namespace, graph.content_hash())
    }

    /// Lower and compile `graph` without consulting any cache.
    fn build_plan_artifact(
        graph: &SemanticGraph,
        optimization: GraphOptimization,
    ) -> Result<CompiledPlanArtifact, GraphPlanError> {
        let outcome = optimization
            .is_optimizing()
            .then(|| graph.eliminate_common_subexpressions());
        let optimized = outcome.as_ref().map_or(graph, |outcome| &outcome.graph);

        let target = optimized
            .targets()
            .first()
            .map(|target| crate::compute::ComputeNodeId(target.0))
            .ok_or(GraphPlanError::NoTarget)?;

        let plan = optimized.lower()?;
        let hot = crate::execution_plan::HotExecutionPlan::compile(&plan, [target])?;

        // Every read of `optimized` has to happen before `outcome` is moved out
        // below, because `optimized` borrows it.
        let dependency =
            Self::combine_dependencies(optimized.iter().map(|node| node.capabilities.dependency));
        let cse = outcome.map(|outcome| outcome.report);

        Ok(CompiledPlanArtifact {
            plan: hot,
            cse,
            dependency,
        })
    }

    /// Wrap a compiled artifact in a fresh executor.
    fn assemble<D>(artifact: CompiledPlanArtifact, dispatcher: D) -> CompiledSemanticGraph<D>
    where
        D: crate::unified_executor::KernelDispatcher,
    {
        CompiledSemanticGraph {
            executor: crate::unified_executor::UnifiedExecutor::new(artifact.plan, dispatcher),
            cse: artifact.cse,
            dependency: artifact.dependency,
        }
    }

    /// Turn a §16 [`SemanticGraph`] into something executable.
    ///
    /// This is the production path for the semantic layer. Without it the graph
    /// type is only reachable from tests, and §20's first performance priority —
    /// not recomputing work the frontend declared twice — would be available to
    /// benchmarks and to nobody else.
    ///
    /// The graph is *not* mutated: with
    /// [`GraphOptimization::CommonSubexpressions`] the pass returns a rewritten
    /// graph, and that copy is what lowers. A caller can therefore keep the
    /// declaration and the optimization side by side, which is what the
    /// numerical-equivalence gate does.
    ///
    /// # Errors
    ///
    /// Returns [`GraphPlanError::NoTarget`] when the graph declares no target,
    /// [`GraphPlanError::Plan`] when lowering fails, and
    /// [`GraphPlanError::Hot`] when the lowered plan cannot be compiled.
    pub fn compile_semantic_graph<D>(
        graph: &SemanticGraph,
        dispatcher: D,
        optimization: GraphOptimization,
    ) -> Result<CompiledSemanticGraph<D>, GraphPlanError>
    where
        D: crate::unified_executor::KernelDispatcher,
    {
        Ok(Self::assemble(
            Self::build_plan_artifact(graph, optimization)?,
            dispatcher,
        ))
    }

    /// Like [`Self::compile_semantic_graph`], but reuses an already-compiled plan.
    ///
    /// §20 ranks "stop re-deriving what has already been derived" above every
    /// micro-optimization: a factor scan compiles *one* declaration and then
    /// runs it for every symbol in the universe, so without this the
    /// lower-and-compile cost is paid once per symbol rather than once per
    /// declaration.
    ///
    /// The executor is still built per call — only the immutable plan is shared —
    /// so two results can never overwrite each other's arena or kernel state.
    /// Pass the same `runtime` across a scan to turn the first call's work into
    /// every later call's cache hit.
    ///
    /// # Errors
    ///
    /// Same as [`Self::compile_semantic_graph`]. A failed build is not cached,
    /// so a corrected graph is not poisoned by an earlier failure.
    pub fn compile_semantic_graph_cached<D>(
        graph: &SemanticGraph,
        dispatcher: D,
        optimization: GraphOptimization,
        runtime: &mut RuntimeContext,
    ) -> Result<CompiledSemanticGraph<D>, GraphPlanError>
    where
        D: crate::unified_executor::KernelDispatcher,
    {
        let key = Self::plan_cache_key(graph, optimization);
        let cached = runtime.cache_mut().get::<CompiledPlanArtifact>(key);
        if let Some(cached) = cached {
            return Ok(Self::assemble((*cached).clone(), dispatcher));
        }

        let artifact = Self::build_plan_artifact(graph, optimization)?;
        runtime
            .cache_mut()
            .insert(key, std::sync::Arc::new(artifact.clone()));
        Ok(Self::assemble(artifact, dispatcher))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factors::{FactorDefinition, FactorDirection, FactorKind, FactorRegistry};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn typed_artifact_hash_is_stable_and_distinct() {
        let first = ArtifactHash::from_bytes(b"factor-output");
        assert_eq!(first, ArtifactHash::from_bytes(b"factor-output"));
        assert_ne!(first, ArtifactHash::from_bytes(b"factor-output-2"));
        assert_eq!(first.to_string().len(), 16);
        assert_eq!(ArtifactHash::from_u64(first.get()), first);
    }

    #[test]
    fn dirty_range_propagates_forward_then_expands_for_history() {
        let dirty = DirtyRange::new(5, 6);
        let affected = dirty.propagate_forward(3, 12);
        assert_eq!(affected, DirtyRange::new(5, 9));
        assert_eq!(affected.with_lookback(3), DirtyRange::new(2, 9));
        assert_eq!(dirty.len(), 1);
        assert!(dirty.is_within(12));
    }

    #[test]
    fn range_runtime_executes_only_affected_window_plus_history() {
        let visited_rows = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&visited_rows);
        let mut registry = FactorRegistry::new();
        registry
            .register(FactorDefinition::new(
                "score",
                ["close"],
                FactorKind::TimeSeries,
                FactorDirection::HigherBetter,
                Arc::new(move |inputs| {
                    let close = inputs.get("close")?;
                    counter.fetch_add(close.len(), Ordering::SeqCst);
                    Ok(close.iter().map(|value| value * 2.0).collect())
                }),
            ))
            .unwrap();
        let plan = FactorPlan::compile(&registry, &["score"]).unwrap();
        let engine = FactorEngine::new(registry);

        let original = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let original_context = BorrowedFactorContext::new()
            .with_series("close", &original)
            .unwrap();
        let full = UnifiedRuntime::execute_factor_plan_borrowed(&plan, &engine, &original_context)
            .unwrap();
        assert_eq!(visited_rows.load(Ordering::SeqCst), original.len());

        let changed = [1.0, 2.0, 3.0, 40.0, 5.0, 6.0];
        let changed_context = BorrowedFactorContext::new()
            .with_series("close", &changed)
            .unwrap();
        let ranged = UnifiedRuntime::execute_factor_plan_range_borrowed(
            &plan,
            &engine,
            &changed_context,
            &full.output,
            DirtyRange::new(3, 4),
            1,
        )
        .unwrap();

        assert_eq!(visited_rows.load(Ordering::SeqCst), original.len() + 3);
        assert_eq!(
            ranged.output["score"],
            vec![2.0, 4.0, 6.0, 80.0, 10.0, 12.0]
        );
        assert_eq!(
            ranged.trace.mode,
            RuntimeExecutionMode::Range {
                input_dirty: DirtyRange::new(3, 4),
                affected: DirtyRange::new(3, 5),
                recompute: DirtyRange::new(2, 5),
            }
        );
        assert_eq!(ranged.trace.recomputed_rows, 3);
    }

    #[test]
    fn combined_dependency_is_conservative() {
        assert_eq!(
            UnifiedRuntime::combine_dependencies([]),
            DependencyShape::FixedLookback(0)
        );
        assert_eq!(
            UnifiedRuntime::combine_dependencies([
                DependencyShape::FixedLookback(2),
                DependencyShape::FixedLookback(5),
            ]),
            DependencyShape::FixedLookback(5)
        );
        // One non-fixed node anywhere invalidates the whole plan.
        assert_eq!(
            UnifiedRuntime::combine_dependencies([
                DependencyShape::FixedLookback(5),
                DependencyShape::Expanding,
            ]),
            DependencyShape::Dynamic
        );
        assert_eq!(
            UnifiedRuntime::combine_dependencies([
                DependencyShape::CrossSectional,
                DependencyShape::CrossSectional,
            ]),
            DependencyShape::CrossSectional
        );
    }

    #[test]
    fn range_execution_is_refused_without_a_proven_lookback() {
        let mut registry = FactorRegistry::new();
        registry
            .register(FactorDefinition::new(
                "score",
                ["close"],
                FactorKind::TimeSeries,
                FactorDirection::HigherBetter,
                Arc::new(|inputs| {
                    let close = inputs.get("close")?;
                    Ok(close.iter().map(|value| value * 2.0).collect())
                }),
            ))
            .unwrap();
        let plan = FactorPlan::compile(&registry, &["score"]).unwrap();
        let engine = FactorEngine::new(registry);
        let series = [1.0, 2.0, 3.0, 4.0];
        let context = BorrowedFactorContext::new()
            .with_series("close", &series)
            .unwrap();
        let mut output = BTreeMap::from([("score".to_string(), vec![2.0, 4.0, 6.0, 8.0])]);
        let mut runtime = RuntimeContext::new();

        for shape in [
            DependencyShape::Expanding,
            DependencyShape::Dynamic,
            DependencyShape::CrossSectional,
            DependencyShape::Global,
        ] {
            let error = UnifiedRuntime::execute_factor_plan_range_into_borrowed_with_context(
                &plan,
                &engine,
                &context,
                &mut output,
                DirtyRange::new(1, 2),
                shape,
                &mut runtime,
            )
            .unwrap_err();
            assert!(
                error.to_string().contains("fixed lookback"),
                "unexpected error for {shape:?}: {error}"
            );
        }
        // Nothing ran, so nothing was accounted.
        assert_eq!(runtime.metrics().executions(), 0);
    }

    #[test]
    fn context_records_factor_execution_kinds() {
        let mut registry = FactorRegistry::new();
        registry
            .register(FactorDefinition::new(
                "score",
                ["close"],
                FactorKind::TimeSeries,
                FactorDirection::HigherBetter,
                Arc::new(|inputs| {
                    let close = inputs.get("close")?;
                    Ok(close.iter().map(|value| value * 2.0).collect())
                }),
            ))
            .unwrap();
        let plan = FactorPlan::compile(&registry, &["score"]).unwrap();
        let engine = FactorEngine::new(registry);
        let series = [1.0, 2.0, 3.0, 4.0];
        let context = BorrowedFactorContext::new()
            .with_series("close", &series)
            .unwrap();
        let mut runtime = RuntimeContext::new();

        let full = UnifiedRuntime::execute_factor_plan_borrowed_with_context(
            &plan,
            &engine,
            &context,
            &mut runtime,
        )
        .unwrap();
        let mut output = full.output.clone();
        let trace = UnifiedRuntime::execute_factor_plan_range_into_borrowed_with_context(
            &plan,
            &engine,
            &context,
            &mut output,
            DirtyRange::new(1, 2),
            DependencyShape::FixedLookback(1),
            &mut runtime,
        )
        .unwrap();

        // Dirty row 1, propagated forward through a lookback of 1 (affected
        // 1..3) and expanded backwards by that same lookback to seed the
        // window: 0..3.
        assert_eq!(trace.recomputed_rows, 3);
        let metrics = runtime.metrics();
        assert_eq!(metrics.full_executions, 1);
        assert_eq!(metrics.range_executions, 1);
        assert_eq!(metrics.executions(), 2);
        assert_eq!(metrics.kernel_calls, 2);
        assert_eq!(runtime.diagnostics().len(), 2);
    }
}
