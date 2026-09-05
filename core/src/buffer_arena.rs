//! Bounded reusable `f64` buffer arena for compute backends.
//!
//! Formula, factor and batch planners can share this allocation policy without
//! coupling their numerical kernels. Buffers are keyed by logical length and
//! retained only within configured count/byte limits.
//!
//! Architecture v3 also needs *in-plan* reuse, not only reuse across calls.
//! [`PlanBufferLayout`] performs dependency lifetime analysis once at compile
//! time and maps logical compute-node values onto compact numeric [`BufferSlot`]
//! ids. Hot executors can therefore address scratch storage only by slot and do
//! not need string-keyed buffer maps or per-evaluation lifetime bookkeeping.

use crate::compute::{ComputeNodeId, ComputePlan};
use std::collections::{BTreeMap, BTreeSet};
use std::mem::size_of;

/// Compact physical scratch-buffer slot used by a precompiled execution plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BufferSlot(pub usize);

/// Compile-time mapping from logical compute-node values to physical scratch
/// buffers.
///
/// A slot is reused only after the previous value's final dependent has run.
/// Values passed in `retained` are pinned through the end of execution so the
/// caller can safely observe final outputs after the last kernel finishes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanBufferLayout {
    slots: BTreeMap<ComputeNodeId, BufferSlot>,
    last_use: BTreeMap<ComputeNodeId, usize>,
    slot_count: usize,
}

impl PlanBufferLayout {
    /// Compile physical scratch slots from a validated dependency plan.
    ///
    /// `retained` should contain every logical output that remains observable
    /// after plan execution. Retained values are never recycled inside the
    /// current execution, while intermediate values are released immediately
    /// after their final dependent.
    pub fn compile(plan: &ComputePlan, retained: impl IntoIterator<Item = ComputeNodeId>) -> Self {
        let order = plan.execution_order();
        let end = order.len();
        let positions: BTreeMap<ComputeNodeId, usize> = order
            .iter()
            .copied()
            .enumerate()
            .map(|(position, node)| (node, position))
            .collect();

        // A value must remain alive through the kernel that produced it at a
        // minimum. Every dependency edge extends that lifetime to the consumer.
        let mut last_use = positions.clone();
        for (position, &node_id) in order.iter().enumerate() {
            let node = plan
                .node(node_id)
                .expect("execution order only contains compiled nodes");
            for dependency in &node.dependencies {
                let entry = last_use
                    .get_mut(dependency)
                    .expect("compiled dependencies always have positions");
                *entry = (*entry).max(position);
            }
        }
        for node in retained {
            if let Some(last) = last_use.get_mut(&node) {
                *last = end;
            }
        }

        let mut slots = BTreeMap::new();
        let mut free = BTreeSet::new();
        let mut active: Vec<(ComputeNodeId, BufferSlot)> = Vec::new();
        let mut slot_count = 0usize;

        for (position, &node_id) in order.iter().enumerate() {
            // Only values whose last consumer ran in an earlier step can be
            // reused here. Values consumed by the current kernel remain live
            // until that kernel completes, so input/output aliasing is never
            // introduced implicitly.
            active.retain(|(active_node, slot)| {
                let still_live = last_use
                    .get(active_node)
                    .is_some_and(|last| *last >= position);
                if !still_live {
                    free.insert(slot.0);
                }
                still_live
            });

            let slot = if let Some(index) = free.pop_first() {
                BufferSlot(index)
            } else {
                let slot = BufferSlot(slot_count);
                slot_count += 1;
                slot
            };
            slots.insert(node_id, slot);
            active.push((node_id, slot));
        }

        Self {
            slots,
            last_use,
            slot_count,
        }
    }

    /// Physical scratch slot for a logical compute node.
    pub fn slot(&self, node: ComputeNodeId) -> Option<BufferSlot> {
        self.slots.get(&node).copied()
    }

    /// Final execution-order position that needs this logical value.
    pub fn last_use(&self, node: ComputeNodeId) -> Option<usize> {
        self.last_use.get(&node).copied()
    }

    /// Number of physical buffers required by this plan after lifetime reuse.
    pub const fn slot_count(&self) -> usize {
        self.slot_count
    }

    /// Checkout exactly the physical buffers required by this layout.
    ///
    /// Kernels must obey their full-write contract before observing a buffer
    /// returned by this method; cached allocations intentionally are not
    /// cleared on checkout.
    pub fn take_buffers(&self, arena: &mut BufferArena, len: usize) -> Vec<Vec<f64>> {
        (0..self.slot_count)
            .map(|_| arena.take_overwrite(len))
            .collect()
    }

    /// Return a set of plan scratch buffers to the reusable arena.
    pub fn recycle_buffers(&self, arena: &mut BufferArena, buffers: Vec<Vec<f64>>) {
        debug_assert_eq!(buffers.len(), self.slot_count);
        for buffer in buffers {
            arena.recycle(buffer);
        }
    }
}

/// Retention limits for [`BufferArena`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferArenaConfig {
    /// Maximum cached buffers for one logical length.
    pub max_cached_per_len: usize,
    /// Maximum retained allocation capacity across the entire arena.
    pub max_cached_bytes: usize,
}

impl Default for BufferArenaConfig {
    fn default() -> Self {
        Self {
            max_cached_per_len: 8,
            max_cached_bytes: 64 * 1024 * 1024,
        }
    }
}

/// Current arena allocation/reuse counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferArenaStats {
    /// Number of successful buffer reuses.
    pub cache_hits: u64,
    /// Number of new allocations caused by a cache miss.
    pub cache_misses: u64,
    /// Number of buffers currently retained.
    pub cached_buffers: usize,
    /// Total retained allocation capacity in bytes.
    pub cached_bytes: usize,
}

/// Bounded reusable pool of caller-owned `Vec<f64>` scratch buffers.
#[derive(Debug)]
pub struct BufferArena {
    config: BufferArenaConfig,
    free: BTreeMap<usize, Vec<Vec<f64>>>,
    cached_bytes: usize,
    cache_hits: u64,
    cache_misses: u64,
}

impl Default for BufferArena {
    fn default() -> Self {
        Self::new(BufferArenaConfig::default())
    }
}

impl BufferArena {
    /// Create an arena with explicit retention limits.
    pub fn new(config: BufferArenaConfig) -> Self {
        Self {
            config,
            free: BTreeMap::new(),
            cached_bytes: 0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    /// Return the configured retention limits.
    pub const fn config(&self) -> BufferArenaConfig {
        self.config
    }

    fn pop_cached(&mut self, len: usize) -> Option<Vec<f64>> {
        let mut remove_bucket = false;
        let cached = self.free.get_mut(&len).and_then(|bucket| {
            let buffer = bucket.pop();
            remove_bucket = bucket.is_empty();
            buffer
        });
        if remove_bucket {
            self.free.remove(&len);
        }
        if let Some(buffer) = &cached {
            self.cached_bytes = self
                .cached_bytes
                .saturating_sub(allocation_bytes(buffer.capacity()));
            self.cache_hits = self.cache_hits.saturating_add(1);
        }
        cached
    }

    /// Checkout a zero-filled buffer with exactly `len` logical values.
    pub fn take(&mut self, len: usize) -> Vec<f64> {
        self.take_filled(len, 0.0)
    }

    /// Checkout a buffer and initialize every value to `fill`.
    pub fn take_filled(&mut self, len: usize, fill: f64) -> Vec<f64> {
        let mut buffer = if let Some(buffer) = self.pop_cached(len) {
            buffer
        } else {
            self.cache_misses = self.cache_misses.saturating_add(1);
            vec![fill; len]
        };

        if buffer.len() != len {
            buffer.resize(len, fill);
        }
        buffer.fill(fill);
        buffer
    }

    /// Checkout a buffer for a kernel that guarantees it will overwrite every
    /// element before any value is observed.
    ///
    /// On a cache hit the existing initialized allocation is returned as-is, so
    /// checkout performs no O(n) clear/fill pass. On a cold miss Rust still needs
    /// initialized `f64` storage; the unavoidable first allocation is zero-filled.
    /// Callers must only use this API for kernels with a proven full-write contract.
    pub fn take_overwrite(&mut self, len: usize) -> Vec<f64> {
        if let Some(buffer) = self.pop_cached(len) {
            return buffer;
        }
        self.cache_misses = self.cache_misses.saturating_add(1);
        vec![0.0; len]
    }

    /// Prepare an existing caller-owned buffer for overwrite without round-tripping
    /// it through the free-list.
    ///
    /// When `buffer.capacity() >= len`, the same allocation is retained and only its
    /// logical length/content are reset. If the allocation is too small, the old
    /// buffer is recycled and a right-sized arena buffer is checked out.
    pub fn overwrite(&mut self, buffer: &mut Vec<f64>, len: usize, fill: f64) {
        if buffer.capacity() >= len {
            buffer.resize(len, fill);
            buffer.fill(fill);
            self.cache_hits = self.cache_hits.saturating_add(1);
            return;
        }

        let old = std::mem::take(buffer);
        self.recycle(old);
        *buffer = self.take_filled(len, fill);
    }

    /// Return a buffer to the arena when retention limits permit it.
    ///
    /// Buffers with zero logical length, oversized allocations, or full size
    /// buckets are simply dropped. This bounds idle memory retention even when
    /// callers process a wide range of series lengths.
    pub fn recycle(&mut self, buffer: Vec<f64>) {
        let len = buffer.len();
        if len == 0 || self.config.max_cached_per_len == 0 {
            return;
        }

        let bytes = allocation_bytes(buffer.capacity());
        if bytes > self.config.max_cached_bytes
            || self.cached_bytes.saturating_add(bytes) > self.config.max_cached_bytes
        {
            return;
        }

        let bucket = self.free.entry(len).or_default();
        if bucket.len() >= self.config.max_cached_per_len {
            return;
        }

        self.cached_bytes = self.cached_bytes.saturating_add(bytes);
        bucket.push(buffer);
    }

    /// Execute a closure with a temporary initialized buffer and recycle it on
    /// normal return.
    pub fn with_buffer<R>(
        &mut self,
        len: usize,
        fill: f64,
        use_buffer: impl FnOnce(&mut [f64]) -> R,
    ) -> R {
        let mut buffer = self.take_filled(len, fill);
        let result = use_buffer(&mut buffer);
        self.recycle(buffer);
        result
    }

    /// Drop all retained buffers while preserving hit/miss counters.
    pub fn clear(&mut self) {
        self.free.clear();
        self.cached_bytes = 0;
    }

    /// Return current reuse and retention statistics.
    pub fn stats(&self) -> BufferArenaStats {
        BufferArenaStats {
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
            cached_buffers: self.free.values().map(Vec::len).sum(),
            cached_bytes: self.cached_bytes,
        }
    }
}

const fn allocation_bytes(capacity: usize) -> usize {
    capacity.saturating_mul(size_of::<f64>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compute::{ComputeCapabilities, ComputeEffect, ComputeNode, LookbackRequirement};

    fn pure_capabilities() -> ComputeCapabilities {
        ComputeCapabilities {
            deterministic: true,
            streaming: true,
            stateful: false,
            lookback: LookbackRequirement::None,
            effect: ComputeEffect::Pure,
        }
    }

    #[test]
    fn plan_layout_reuses_dead_intermediate_slots_without_aliasing_live_inputs() {
        let plan = ComputePlan::compile([
            ComputeNode::new(ComputeNodeId(0), "INPUT", vec![], pure_capabilities()),
            ComputeNode::new(
                ComputeNodeId(1),
                "EMA",
                vec![ComputeNodeId(0)],
                pure_capabilities(),
            ),
            ComputeNode::new(
                ComputeNodeId(2),
                "ROC",
                vec![ComputeNodeId(1)],
                pure_capabilities(),
            ),
            ComputeNode::new(
                ComputeNodeId(3),
                "ADD",
                vec![ComputeNodeId(2)],
                pure_capabilities(),
            ),
        ])
        .unwrap();

        let layout = PlanBufferLayout::compile(&plan, [ComputeNodeId(3)]);
        assert_eq!(layout.slot_count(), 2);
        assert_eq!(layout.slot(ComputeNodeId(0)), Some(BufferSlot(0)));
        assert_eq!(layout.slot(ComputeNodeId(1)), Some(BufferSlot(1)));
        assert_eq!(layout.slot(ComputeNodeId(2)), Some(BufferSlot(0)));
        assert_eq!(layout.slot(ComputeNodeId(3)), Some(BufferSlot(1)));
        assert_eq!(layout.last_use(ComputeNodeId(3)), Some(plan.len()));
    }

    #[test]
    fn retained_branch_outputs_stay_live_until_plan_end() {
        let plan = ComputePlan::compile([
            ComputeNode::new(ComputeNodeId(0), "INPUT", vec![], pure_capabilities()),
            ComputeNode::new(
                ComputeNodeId(1),
                "EMA",
                vec![ComputeNodeId(0)],
                pure_capabilities(),
            ),
            ComputeNode::new(
                ComputeNodeId(2),
                "RSI",
                vec![ComputeNodeId(0)],
                pure_capabilities(),
            ),
        ])
        .unwrap();

        let layout = PlanBufferLayout::compile(&plan, [ComputeNodeId(1), ComputeNodeId(2)]);
        assert_eq!(layout.slot_count(), 3);
        assert_ne!(layout.slot(ComputeNodeId(1)), layout.slot(ComputeNodeId(2)));
    }

    #[test]
    fn plan_buffers_round_trip_through_arena() {
        let plan = ComputePlan::compile([
            ComputeNode::new(ComputeNodeId(0), "INPUT", vec![], pure_capabilities()),
            ComputeNode::new(
                ComputeNodeId(1),
                "EMA",
                vec![ComputeNodeId(0)],
                pure_capabilities(),
            ),
        ])
        .unwrap();
        let layout = PlanBufferLayout::compile(&plan, [ComputeNodeId(1)]);
        let mut arena = BufferArena::default();

        let first = layout.take_buffers(&mut arena, 64);
        let pointers: Vec<_> = first.iter().map(|buffer| buffer.as_ptr()).collect();
        layout.recycle_buffers(&mut arena, first);
        let second = layout.take_buffers(&mut arena, 64);
        assert!(second
            .iter()
            .all(|buffer| pointers.contains(&buffer.as_ptr())));
    }

    #[test]
    fn recycled_buffer_is_reused_and_reinitialized() {
        let mut arena = BufferArena::default();
        let mut first = arena.take_filled(16, f64::NAN);
        let pointer = first.as_ptr();
        first[0] = 42.0;
        arena.recycle(first);

        let second = arena.take(16);
        assert_eq!(second.as_ptr(), pointer);
        assert!(second.iter().all(|value| *value == 0.0));
        assert_eq!(arena.stats().cache_hits, 1);
        assert_eq!(arena.stats().cache_misses, 1);
    }

    #[test]
    fn take_overwrite_reuses_dirty_buffer_without_clearing_it() {
        let mut arena = BufferArena::default();
        let mut first = arena.take(8);
        first.fill(42.0);
        let pointer = first.as_ptr();
        arena.recycle(first);

        let second = arena.take_overwrite(8);
        assert_eq!(second.as_ptr(), pointer);
        assert!(second.iter().all(|value| *value == 42.0));
        assert_eq!(arena.stats().cache_hits, 1);
        assert_eq!(arena.stats().cache_misses, 1);
    }

    #[test]
    fn overwrite_keeps_existing_allocation_when_capacity_is_sufficient() {
        let mut arena = BufferArena::default();
        let mut buffer = Vec::with_capacity(32);
        buffer.extend([1.0, 2.0, 3.0, 4.0]);
        let pointer = buffer.as_ptr();

        arena.overwrite(&mut buffer, 24, f64::NAN);

        assert_eq!(buffer.as_ptr(), pointer);
        assert_eq!(buffer.len(), 24);
        assert!(buffer.iter().all(|value| value.is_nan()));
        assert_eq!(arena.stats().cache_hits, 1);
        assert_eq!(arena.stats().cache_misses, 0);
    }

    #[test]
    fn overwrite_replaces_too_small_buffer_through_arena() {
        let mut arena = BufferArena::default();
        let mut buffer = vec![1.0; 4];
        arena.overwrite(&mut buffer, 32, 7.0);

        assert_eq!(buffer.len(), 32);
        assert!(buffer.iter().all(|value| *value == 7.0));
        assert_eq!(arena.stats().cache_misses, 1);
    }

    #[test]
    fn arena_respects_per_length_and_byte_limits() {
        let mut arena = BufferArena::new(BufferArenaConfig {
            max_cached_per_len: 1,
            max_cached_bytes: 128,
        });
        arena.recycle(vec![1.0; 8]);
        arena.recycle(vec![2.0; 8]);
        arena.recycle(vec![3.0; 32]);

        let stats = arena.stats();
        assert_eq!(stats.cached_buffers, 1);
        assert!(stats.cached_bytes <= 128);
    }

    #[test]
    fn with_buffer_recycles_on_normal_return() {
        let mut arena = BufferArena::default();
        let sum = arena.with_buffer(4, 2.0, |buffer| buffer.iter().sum::<f64>());
        assert_eq!(sum, 8.0);
        assert_eq!(arena.stats().cached_buffers, 1);
    }
}
