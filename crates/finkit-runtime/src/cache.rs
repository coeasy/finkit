//! Runtime cache primitives.

use crate::{FactorCacheKey, FactorOutput};
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
}

#[derive(Clone, Debug, Default)]
pub struct FactorCache {
    entries: HashMap<FactorCacheKey, FactorOutput>,
    stats: CacheStats,
}

impl FactorCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            stats: CacheStats::default(),
        }
    }

    pub fn insert(&mut self, key: FactorCacheKey, value: FactorOutput) {
        self.entries.insert(key, value);
    }

    pub fn get(&mut self, key: &FactorCacheKey) -> Option<&FactorOutput> {
        if self.entries.contains_key(key) {
            self.stats.hits += 1;
        } else {
            self.stats.misses += 1;
        }
        self.entries.get(key)
    }

    /// Return a cached output or compute and insert it exactly once for this
    /// mutable cache instance.
    pub fn get_or_compute<E, F>(
        &mut self,
        key: FactorCacheKey,
        compute: F,
    ) -> Result<FactorOutput, E>
    where
        F: FnOnce() -> Result<FactorOutput, E>,
    {
        if let Some(value) = self.entries.get(&key) {
            self.stats.hits += 1;
            return Ok(value.clone());
        }

        self.stats.misses += 1;
        let value = compute()?;
        self.entries.insert(key, value.clone());
        Ok(value)
    }

    pub fn contains(&self, key: &FactorCacheKey) -> bool {
        self.entries.contains_key(key)
    }

    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finkit_array::FloatArray;
    use finkit_series::QuantSeries;

    fn output() -> FactorOutput {
        FactorOutput::new(
            "SMA",
            QuantSeries::new("TEST", vec![1], FloatArray::new(vec![2.0])),
        )
    }

    #[test]
    fn get_or_compute_caches_result_and_updates_stats() {
        let mut cache = FactorCache::new();
        let key = FactorCacheKey::new("TEST", "SMA", "period=2");
        let first = cache
            .get_or_compute(key.clone(), || Ok::<_, ()>(output()))
            .unwrap();
        let second = cache
            .get_or_compute(key, || -> Result<FactorOutput, ()> {
                panic!("cache should not compute twice")
            })
            .unwrap();

        assert_eq!(first.series.values(), second.series.values());
        assert_eq!(cache.stats().misses, 1);
        assert_eq!(cache.stats().hits, 1);
        assert_eq!(cache.len(), 1);
    }
}
