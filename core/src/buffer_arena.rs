//! Bounded reusable `f64` buffer arena for compute backends.
//!
//! Formula, factor and batch planners can share this allocation policy without
//! coupling their numerical kernels. Buffers are keyed by logical length and
//! retained only within configured count/byte limits.

use std::collections::BTreeMap;
use std::mem::size_of;

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
