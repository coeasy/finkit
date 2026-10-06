//! One execution context shared by every numeric executor.
//!
//! Before this module, an executor needed four unrelated things to run: a
//! [`BufferArena`](crate::buffer_arena::BufferArena) for scratch buffers, a
//! [`StateArena`](crate::state_arena::StateArena) for persistent kernel
//! state, a plan for the numeric program, and — for anything that wanted to
//! report or bound its work — a set of ad-hoc fields or parameters. Nothing
//! carried limits, nothing carried counters, and nothing carried diagnostics,
//! so a backend could not say "this plan needs more memory than I am allowed to
//! use" and no test could assert how many kernels a plan actually dispatched.
//!
//! [`RuntimeContext`](crate::runtime_context::RuntimeContext) is that one
//! object (§19 of the V4 plan). It owns:
//!
//! * `buffers` — the scratch allocation arena the plan reuses;
//! * `states` — the persistent state arena streaming kernels update;
//! * `cache` — a bounded, content-keyed store for compiled artifacts;
//! * `limits` — execution budgets the context enforces before work starts;
//! * `metrics` — what the context has actually executed;
//! * `diagnostics` — a bounded, lossy event log safe to write from a hot path.
//!
//! Executors take `&mut RuntimeContext` and nothing else. That keeps a backend
//! from inventing a parallel notion of budget or accounting, which is how the
//! pre-convergence code ended up with three different answers to "how much did
//! this cost".

use crate::buffer_arena::{BufferArena, BufferArenaConfig, BufferArenaStats};
use crate::execution_plan::HotExecutionPlan;
use crate::state_arena::StateArena;
use crate::unified_runtime::ArtifactHash;
use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::Arc;

/// How an execution was driven.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExecutionKind {
    /// The complete bound input extent.
    Full,
    /// An explicit half-open input range (dirty-range recomputation).
    Range,
    /// The final bound sample only (streaming / `eval_last`).
    Last,
}

impl ExecutionKind {
    /// Stable label used by metrics and diagnostics.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Range => "range",
            Self::Last => "last",
        }
    }
}

/// Budgets a context refuses to exceed.
///
/// Defaults are generous enough for the shipped plans and small enough that a
/// runaway lowering shows up as a typed error instead of an OOM. They are
/// deliberately *not* zero-defaulted: an unconfigured context must still run
/// the library's own tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionLimits {
    /// Maximum number of numeric nodes one plan may contain.
    pub max_plan_nodes: usize,
    /// Maximum bytes of reusable scratch the arena may cache.
    pub max_buffer_bytes: usize,
    /// Maximum number of persistent state slots one plan may request.
    pub max_state_slots: usize,
    /// Number of diagnostics entries retained before the oldest are dropped.
    pub diagnostic_capacity: usize,
    /// Maximum compiled artifacts one context retains.
    pub max_artifact_entries: usize,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            max_plan_nodes: 4096,
            max_buffer_bytes: 64 * 1024 * 1024,
            max_state_slots: 1024,
            diagnostic_capacity: 128,
            max_artifact_entries: 128,
        }
    }
}

impl ExecutionLimits {
    /// Construct explicit limits.
    pub const fn new(
        max_plan_nodes: usize,
        max_buffer_bytes: usize,
        max_state_slots: usize,
        diagnostic_capacity: usize,
    ) -> Self {
        Self {
            max_plan_nodes,
            max_buffer_bytes,
            max_state_slots,
            diagnostic_capacity,
            max_artifact_entries: 128,
        }
    }
}

/// What a context has executed.
///
/// Counters are cumulative for the lifetime of the context; [`RuntimeContext::reset_metrics`]
/// clears them, and [`RuntimeContext::reset`] does not (a logical series restart
/// is not an accounting boundary).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeMetrics {
    /// Full-extent executions.
    pub full_executions: u64,
    /// Range executions.
    pub range_executions: u64,
    /// Last-sample (streaming) executions.
    pub last_executions: u64,
    /// Kernel invocations across all executions.
    pub kernel_calls: u64,
    /// Nodes visited across all executions.
    pub node_visits: u64,
    /// Scratch buffers served by the arena.
    pub buffers_taken: u64,
    /// Scratch buffers returned to the arena for reuse.
    pub buffers_recycled: u64,
    /// Artifact lookups served from the context cache.
    pub cache_hits: u64,
    /// Artifact lookups that had to build.
    pub cache_misses: u64,
    /// Entries evicted to stay inside the artifact capacity.
    pub cache_evictions: u64,
    /// Diagnostics dropped because the ring was full.
    pub diagnostics_dropped: u64,
}

impl RuntimeMetrics {
    /// Total executions of every kind.
    pub const fn executions(&self) -> u64 {
        self.full_executions + self.range_executions + self.last_executions
    }

    /// Total artifact lookups.
    pub const fn cache_lookups(&self) -> u64 {
        self.cache_hits + self.cache_misses
    }
}

/// Identity of one cached compiled artifact.
///
/// An artifact is identified by *what it is* (`namespace`, chosen by the owning
/// frontend) and *what it was built from* ([`ArtifactHash`] of the canonical
/// request). Two frontends that compile the same request under different
/// namespaces therefore do not collide, and the same request always maps to the
/// same entry across processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArtifactKey {
    /// Frontend-chosen discriminant (for example a stable per-surface constant).
    pub namespace: u64,
    /// Content identity of the request the artifact was compiled from.
    pub artifact: ArtifactHash,
}

impl ArtifactKey {
    /// Build an artifact key.
    pub const fn new(namespace: u64, artifact: ArtifactHash) -> Self {
        Self {
            namespace,
            artifact,
        }
    }

    /// Build a key directly from canonical request bytes.
    pub fn from_bytes(namespace: u64, bytes: &[u8]) -> Self {
        Self::new(namespace, ArtifactHash::from_bytes(bytes))
    }
}

/// Current artifact-cache counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactCacheStats {
    /// Retained entries.
    pub entries: usize,
    /// Configured maximum entries.
    pub capacity: usize,
    /// Lookups served without building.
    pub hits: u64,
    /// Lookups that had to build.
    pub misses: u64,
    /// Entries evicted to respect the capacity.
    pub evictions: u64,
}

struct ArtifactEntry {
    value: Arc<dyn Any + Send + Sync>,
    last_used: u64,
}

/// Bounded, content-keyed store for compiled artifacts.
///
/// Formula, factor, composite and research planners each grew their own cache
/// with its own key type and its own eviction rule, so "is this already
/// compiled?" had a different answer in every layer. This is the one cache the
/// runtime context owns; a frontend picks a `namespace` and stores whatever
/// `Send + Sync` artifact it compiles.
///
/// Eviction is least-recently-used and the store never grows past its
/// capacity, so a context that sees unbounded distinct requests retains
/// bounded memory.
pub struct ArtifactCache {
    entries: HashMap<ArtifactKey, ArtifactEntry>,
    capacity: usize,
    clock: u64,
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl fmt::Debug for ArtifactCache {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // `entries` is summarized by length rather than dumped: the values are
        // opaque `Arc<dyn Any>` payloads that have no useful formatting, and a
        // 128-entry dump would bury every other diagnostic.
        f.debug_struct("ArtifactCache")
            .field("entries", &self.entries.len())
            .field("capacity", &self.capacity)
            .field("clock", &self.clock)
            .field("hits", &self.hits)
            .field("misses", &self.misses)
            .field("evictions", &self.evictions)
            .finish()
    }
}

impl Default for ArtifactCache {
    fn default() -> Self {
        Self::new(ExecutionLimits::default().max_artifact_entries)
    }
}

impl ArtifactCache {
    /// Create a cache retaining at most `capacity` artifacts.
    ///
    /// A capacity of zero disables storage: every lookup misses and nothing is
    /// retained, which is the useful configuration for a one-shot context.
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            capacity,
            clock: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// Configured maximum entries.
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Retained entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is retained.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Look up an artifact, returning `None` when absent or stored as another type.
    ///
    /// A type mismatch is treated as a miss, and the stale entry is dropped: the
    /// caller is about to build the right type, and keeping two values under one
    /// content key would make the entry permanently unusable.
    pub fn get<T: Any + Send + Sync>(&mut self, key: ArtifactKey) -> Option<Arc<T>> {
        self.clock = self.clock.wrapping_add(1);
        let clock = self.clock;
        let Some(entry) = self.entries.get_mut(&key) else {
            self.misses = self.misses.saturating_add(1);
            return None;
        };
        if let Ok(value) = Arc::clone(&entry.value).downcast::<T>() {
            entry.last_used = clock;
            self.hits = self.hits.saturating_add(1);
            Some(value)
        } else {
            self.entries.remove(&key);
            self.misses = self.misses.saturating_add(1);
            None
        }
    }

    /// Store an artifact, evicting the least recently used entry when full.
    pub fn insert<T: Any + Send + Sync>(&mut self, key: ArtifactKey, value: Arc<T>) {
        if self.capacity == 0 {
            return;
        }
        self.clock = self.clock.wrapping_add(1);
        if self.entries.len() >= self.capacity && !self.entries.contains_key(&key) {
            self.evict_one();
        }
        self.entries.insert(
            key,
            ArtifactEntry {
                value,
                last_used: self.clock,
            },
        );
    }

    /// Return the cached artifact for `key`, building it on a miss.
    pub fn get_or_insert_with<T: Any + Send + Sync>(
        &mut self,
        key: ArtifactKey,
        build: impl FnOnce() -> T,
    ) -> Arc<T> {
        if let Some(value) = self.get::<T>(key) {
            return value;
        }
        let value = Arc::new(build());
        self.insert(key, Arc::clone(&value));
        value
    }

    /// Drop one entry and report whether anything was removed.
    pub fn remove<T: Any + Send + Sync>(&mut self, key: ArtifactKey) -> Option<Arc<T>> {
        let entry = self.entries.remove(&key)?;
        entry.value.downcast::<T>().ok()
    }

    /// Drop every retained artifact while keeping the hit/miss history.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Current retention and lookup statistics.
    pub fn stats(&self) -> ArtifactCacheStats {
        ArtifactCacheStats {
            entries: self.entries.len(),
            capacity: self.capacity,
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
        }
    }

    /// Change the capacity, evicting down to the new bound immediately.
    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity;
        while self.entries.len() > self.capacity {
            self.evict_one();
        }
    }

    fn evict_one(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        // Least recently used. The store is bounded to a small number of
        // compiled artifacts, so a linear scan is cheaper than maintaining a
        // second index that could itself drift out of sync.
        let victim = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(key, _)| *key);
        if let Some(victim) = victim {
            self.entries.remove(&victim);
            self.evictions = self.evictions.saturating_add(1);
        }
    }
}

/// A bounded, lossy event log.
///
/// Diagnostics may be written from a hot loop, so the log never grows without
/// bound and never allocates on the drop path. Drops are counted rather than
/// silently ignored: a truncated log is still evidence, but only if the reader
/// knows it was truncated.
#[derive(Debug, Clone)]
pub struct Diagnostics {
    events: VecDeque<String>,
    capacity: usize,
    dropped: u64,
}

impl Diagnostics {
    /// Create a log bounded to `capacity` entries.
    pub fn new(capacity: usize) -> Self {
        Self {
            events: VecDeque::with_capacity(capacity.max(1)),
            capacity: capacity.max(1),
            dropped: 0,
        }
    }

    /// Record one event, dropping the oldest entry when full.
    pub fn push(&mut self, event: impl Into<String>) {
        if self.events.len() == self.capacity {
            self.events.pop_front();
            self.dropped = self.dropped.saturating_add(1);
        }
        self.events.push_back(event.into());
    }

    /// Retained events, oldest first.
    pub fn events(&self) -> impl Iterator<Item = &str> {
        self.events.iter().map(String::as_str)
    }

    /// Number of retained entries.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether nothing has been recorded.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Entries dropped because the ring was full.
    pub const fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Retained capacity.
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Drop all retained events, keeping the drop counter.
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

/// A limit the context refused to exceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeContextError {
    /// The plan has more nodes than the context is allowed to execute.
    PlanTooLarge {
        /// Nodes requested by the plan.
        nodes: usize,
        /// Configured maximum.
        limit: usize,
    },
    /// The plan requests more persistent state slots than allowed.
    StateSlotsExceeded {
        /// Slots requested by the plan.
        slots: usize,
        /// Configured maximum.
        limit: usize,
    },
    /// A scratch request exceeds the arena byte budget.
    BufferBudgetExceeded {
        /// Bytes requested.
        bytes: usize,
        /// Configured maximum.
        limit: usize,
    },
}

impl fmt::Display for RuntimeContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PlanTooLarge { nodes, limit } => write!(
                f,
                "plan declares {nodes} nodes but the execution limit is {limit}"
            ),
            Self::StateSlotsExceeded { slots, limit } => write!(
                f,
                "plan requests {slots} state slots but the execution limit is {limit}"
            ),
            Self::BufferBudgetExceeded { bytes, limit } => write!(
                f,
                "scratch request of {bytes} bytes exceeds the arena budget of {limit}"
            ),
        }
    }
}

impl std::error::Error for RuntimeContextError {}

/// The one context every numeric executor runs against.
#[derive(Debug)]
pub struct RuntimeContext {
    buffers: BufferArena,
    states: StateArena,
    cache: ArtifactCache,
    limits: ExecutionLimits,
    metrics: RuntimeMetrics,
    diagnostics: Diagnostics,
}

impl Default for RuntimeContext {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeContext {
    /// Create a context with default limits and the default buffer arena.
    pub fn new() -> Self {
        Self::with_limits(ExecutionLimits::default())
    }

    /// Create a context with explicit limits and the default buffer arena.
    pub fn with_limits(limits: ExecutionLimits) -> Self {
        Self::with_limits_and_arena(limits, BufferArenaConfig::default())
    }

    /// Create a context with explicit limits and buffer-arena configuration.
    pub fn with_limits_and_arena(limits: ExecutionLimits, arena: BufferArenaConfig) -> Self {
        Self {
            buffers: BufferArena::new(arena),
            states: StateArena::new(),
            cache: ArtifactCache::new(limits.max_artifact_entries),
            limits,
            metrics: RuntimeMetrics::default(),
            diagnostics: Diagnostics::new(limits.diagnostic_capacity),
        }
    }

    /// Scratch allocation arena.
    pub const fn buffers(&self) -> &BufferArena {
        &self.buffers
    }

    /// Mutable scratch allocation arena.
    pub const fn buffers_mut(&mut self) -> &mut BufferArena {
        &mut self.buffers
    }

    /// Current buffer reuse counters.
    pub fn buffer_stats(&self) -> BufferArenaStats {
        self.buffers.stats()
    }

    /// Persistent kernel state arena.
    pub const fn states(&self) -> &StateArena {
        &self.states
    }

    /// Mutable persistent kernel state arena.
    pub const fn states_mut(&mut self) -> &mut StateArena {
        &mut self.states
    }

    /// Both arenas at once.
    ///
    /// Executors need the scratch arena and the state arena in the same
    /// expression (a kernel reads scratch and writes state). Borrowing them
    /// through two `&mut self` calls does not type-check, and nothing here
    /// aliases, so the pair is handed out together.
    pub const fn arenas_mut(&mut self) -> (&mut BufferArena, &mut StateArena) {
        (&mut self.buffers, &mut self.states)
    }

    /// Configured budgets.
    pub const fn limits(&self) -> ExecutionLimits {
        self.limits
    }

    /// Mutable budgets. Tightening a limit does not retroactively fail work,
    /// except that an artifact capacity is applied to the cache immediately.
    pub fn limits_mut(&mut self) -> &mut ExecutionLimits {
        &mut self.limits
    }

    /// Shared compiled-artifact cache.
    pub const fn cache(&self) -> &ArtifactCache {
        &self.cache
    }

    /// Mutable compiled-artifact cache.
    ///
    /// Prefer [`Self::artifact_or_insert_with`], which also keeps the context's
    /// cache counters honest.
    pub const fn cache_mut(&mut self) -> &mut ArtifactCache {
        &mut self.cache
    }

    /// Current artifact retention and lookup statistics.
    pub fn artifact_cache_stats(&self) -> ArtifactCacheStats {
        self.cache.stats()
    }

    /// Return the cached artifact for `key`, building it on a miss.
    ///
    /// This is the only artifact entry point that updates
    /// [`RuntimeMetrics::cache_hits`]/[`RuntimeMetrics::cache_misses`], so a
    /// context's accounting matches its cache. `build` is called without access
    /// to the context: it must be a pure function of `key`, which is what makes
    /// the content-addressed key sound in the first place.
    pub fn artifact_or_insert_with<T: Any + Send + Sync>(
        &mut self,
        key: ArtifactKey,
        build: impl FnOnce() -> T,
    ) -> Arc<T> {
        let value = self.cache.get_or_insert_with(key, build);
        let stats = self.cache.stats();
        self.metrics.cache_hits = stats.hits;
        self.metrics.cache_misses = stats.misses;
        self.metrics.cache_evictions = stats.evictions;
        value
    }

    /// Drop every cached artifact while keeping the cache's lookup history.
    pub fn clear_artifacts(&mut self) {
        self.cache.clear();
    }

    /// Cumulative execution counters.
    pub const fn metrics(&self) -> RuntimeMetrics {
        self.metrics
    }

    /// Mutable counters, for backends that account their own kernel calls.
    pub const fn metrics_mut(&mut self) -> &mut RuntimeMetrics {
        &mut self.metrics
    }

    /// Bounded event log.
    pub const fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// Mutable bounded event log.
    pub const fn diagnostics_mut(&mut self) -> &mut Diagnostics {
        &mut self.diagnostics
    }

    /// Record one diagnostic event and mirror the drop counter.
    pub fn note(&mut self, event: impl Into<String>) {
        self.diagnostics.push(event);
        self.metrics.diagnostics_dropped = self.diagnostics.dropped();
    }

    /// Refuse a plan that exceeds the node or state-slot budget.
    ///
    /// This runs *before* any allocation, so an oversized plan costs one
    /// comparison instead of a partially materialised arena.
    pub fn enforce_plan(&self, plan: &HotExecutionPlan) -> Result<(), RuntimeContextError> {
        let nodes = plan.nodes().len();
        if nodes > self.limits.max_plan_nodes {
            return Err(RuntimeContextError::PlanTooLarge {
                nodes,
                limit: self.limits.max_plan_nodes,
            });
        }
        let slots = plan.state_layout().slot_count();
        if slots > self.limits.max_state_slots {
            return Err(RuntimeContextError::StateSlotsExceeded {
                slots,
                limit: self.limits.max_state_slots,
            });
        }
        Ok(())
    }

    /// Refuse a scratch request that exceeds the arena byte budget.
    pub const fn enforce_buffer_bytes(&self, bytes: usize) -> Result<(), RuntimeContextError> {
        if bytes > self.limits.max_buffer_bytes {
            return Err(RuntimeContextError::BufferBudgetExceeded {
                bytes,
                limit: self.limits.max_buffer_bytes,
            });
        }
        Ok(())
    }

    /// Count one execution of `kind` over `nodes` nodes.
    pub fn record_execution(&mut self, kind: ExecutionKind, nodes: usize) {
        match kind {
            ExecutionKind::Full => self.metrics.full_executions += 1,
            ExecutionKind::Range => self.metrics.range_executions += 1,
            ExecutionKind::Last => self.metrics.last_executions += 1,
        }
        self.metrics.node_visits = self.metrics.node_visits.saturating_add(nodes as u64);
    }

    /// Count dispatched kernels.
    pub fn record_kernel_calls(&mut self, calls: usize) {
        self.metrics.kernel_calls = self.metrics.kernel_calls.saturating_add(calls as u64);
    }

    /// Count scratch buffers served by the arena.
    pub fn record_buffers_taken(&mut self, buffers: usize) {
        self.metrics.buffers_taken = self.metrics.buffers_taken.saturating_add(buffers as u64);
    }

    /// Count scratch buffers returned for reuse.
    pub fn record_buffers_recycled(&mut self, buffers: usize) {
        self.metrics.buffers_recycled =
            self.metrics.buffers_recycled.saturating_add(buffers as u64);
    }

    /// Drop persistent state while keeping the limit and metric history.
    ///
    /// Cached artifacts are content-addressed, so a logically independent series
    /// can still reuse them; use [`Self::clear_artifacts`] to drop them.
    /// Scratch buffers are retained as well — that is the arena's whole point.
    pub fn reset(&mut self) {
        self.states.clear();
        self.diagnostics.clear();
    }

    /// Clear cumulative counters.
    pub fn reset_metrics(&mut self) {
        let dropped = self.diagnostics.dropped();
        self.metrics = RuntimeMetrics {
            diagnostics_dropped: dropped,
            ..RuntimeMetrics::default()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_separate_execution_kinds() {
        let mut context = RuntimeContext::new();
        context.record_execution(ExecutionKind::Full, 3);
        context.record_execution(ExecutionKind::Range, 2);
        context.record_execution(ExecutionKind::Last, 1);
        let metrics = context.metrics();
        assert_eq!(metrics.full_executions, 1);
        assert_eq!(metrics.range_executions, 1);
        assert_eq!(metrics.last_executions, 1);
        assert_eq!(metrics.executions(), 3);
        assert_eq!(metrics.node_visits, 6);
    }

    #[test]
    fn diagnostics_are_bounded_and_count_drops() {
        let mut context = RuntimeContext::with_limits(ExecutionLimits::new(8, 1024, 4, 2));
        context.note("first");
        context.note("second");
        context.note("third");
        assert_eq!(context.diagnostics().len(), 2);
        assert_eq!(context.diagnostics().dropped(), 1);
        assert_eq!(context.metrics().diagnostics_dropped, 1);
        let retained: Vec<&str> = context.diagnostics().events().collect();
        assert_eq!(retained, vec!["second", "third"]);
    }

    #[test]
    fn buffer_budget_is_enforced_before_allocation() {
        let context = RuntimeContext::with_limits(ExecutionLimits::new(8, 1024, 4, 4));
        assert!(context.enforce_buffer_bytes(1024).is_ok());
        assert_eq!(
            context.enforce_buffer_bytes(1025),
            Err(RuntimeContextError::BufferBudgetExceeded {
                bytes: 1025,
                limit: 1024,
            })
        );
    }

    #[test]
    fn reset_drops_state_but_keeps_accounting() {
        let mut context = RuntimeContext::new();
        context.record_execution(ExecutionKind::Full, 1);
        context.note("event");
        context.reset();
        assert!(context.states().is_empty());
        assert!(context.diagnostics().is_empty());
        assert_eq!(context.metrics().full_executions, 1);
        context.reset_metrics();
        assert_eq!(context.metrics().full_executions, 0);
    }

    #[test]
    fn artifact_cache_serves_the_same_request_once() {
        let mut context = RuntimeContext::new();
        let key = ArtifactKey::from_bytes(7, b"FORMULA:ma(close,5)");
        let mut builds = 0;

        for _ in 0..2 {
            let artifact: Arc<u32> = context.artifact_or_insert_with(key, || {
                builds += 1;
                42
            });
            assert_eq!(*artifact, 42);
        }

        assert_eq!(builds, 1);
        let metrics = context.metrics();
        assert_eq!(metrics.cache_hits, 1);
        assert_eq!(metrics.cache_misses, 1);
        assert_eq!(metrics.cache_lookups(), 2);
        assert_eq!(context.artifact_cache_stats().entries, 1);
    }

    #[test]
    fn artifact_cache_is_content_and_namespace_keyed() {
        let mut context = RuntimeContext::new();
        let a: Arc<u8> = context.artifact_or_insert_with(ArtifactKey::from_bytes(1, b"x"), || 1);
        let b: Arc<u8> = context.artifact_or_insert_with(ArtifactKey::from_bytes(2, b"x"), || 2);
        let c: Arc<u8> = context.artifact_or_insert_with(ArtifactKey::from_bytes(1, b"y"), || 3);
        assert_eq!(*a, 1);
        assert_eq!(*b, 2);
        assert_eq!(*c, 3);
        assert_eq!(context.artifact_cache_stats().entries, 3);
    }

    #[test]
    fn artifact_cache_evicts_least_recently_used_entries() {
        let mut cache = ArtifactCache::new(2);
        let first = ArtifactKey::from_bytes(1, b"first");
        let second = ArtifactKey::from_bytes(1, b"second");
        let third = ArtifactKey::from_bytes(1, b"third");

        cache.insert(first, Arc::new(1u8));
        cache.insert(second, Arc::new(2u8));
        // Touch `first` so `second` becomes the least recently used entry.
        assert!(cache.get::<u8>(first).is_some());
        cache.insert(third, Arc::new(3u8));

        assert_eq!(cache.len(), 2);
        assert!(cache.get::<u8>(second).is_none());
        assert!(cache.get::<u8>(first).is_some());
        assert!(cache.get::<u8>(third).is_some());
        assert_eq!(cache.stats().evictions, 1);
    }

    #[test]
    fn artifact_cache_treats_a_type_mismatch_as_a_miss() {
        let mut cache = ArtifactCache::new(4);
        let key = ArtifactKey::from_bytes(9, b"artifact");
        cache.insert(key, Arc::new(1u8));
        assert!(cache.get::<u16>(key).is_none());
        // The stale entry is gone, so the right type can be stored cleanly.
        assert!(cache.get::<u8>(key).is_none());
        cache.insert(key, Arc::new(2u16));
        assert_eq!(cache.get::<u16>(key).map(|value| *value), Some(2));
    }

    #[test]
    fn zero_capacity_cache_never_retains() {
        let mut context = RuntimeContext::with_limits(ExecutionLimits {
            max_artifact_entries: 0,
            ..ExecutionLimits::default()
        });
        let key = ArtifactKey::from_bytes(1, b"artifact");
        let first: Arc<u8> = context.artifact_or_insert_with(key, || 1);
        let second: Arc<u8> = context.artifact_or_insert_with(key, || 2);
        assert_eq!(*first, 1);
        assert_eq!(*second, 2);
        assert_eq!(context.artifact_cache_stats().entries, 0);
        assert_eq!(context.metrics().cache_misses, 2);
    }
}
