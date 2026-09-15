//! Runtime cache primitives.

use std::collections::HashMap;
use crate::FactorCacheKey;

#[derive(Clone, Debug, Default)]
pub struct FactorCache {
    entries: HashMap<FactorCacheKey, String>,
}

impl FactorCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: FactorCacheKey, value: impl Into<String>) {
        self.entries.insert(key, value.into());
    }

    pub fn get(&self, key: &FactorCacheKey) -> Option<&String> {
        self.entries.get(key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
