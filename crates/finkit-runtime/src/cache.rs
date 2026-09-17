//! Runtime cache primitives.

use std::collections::HashMap;
use crate::{FactorCacheKey, FactorOutput};

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

    pub fn contains(&self, key: &FactorCacheKey) -> bool {
        self.entries.contains_key(key)
    }

    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
