//! Runtime cache primitives.

use std::collections::HashMap;
use crate::{FactorCacheKey, FactorOutput};

#[derive(Clone, Debug, Default)]
pub struct FactorCache {
    entries: HashMap<FactorCacheKey, FactorOutput>,
}

impl FactorCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: FactorCacheKey, value: FactorOutput) {
        self.entries.insert(key, value);
    }

    pub fn get(&self, key: &FactorCacheKey) -> Option<&FactorOutput> {
        self.entries.get(key)
    }

    pub fn contains(&self, key: &FactorCacheKey) -> bool {
        self.entries.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
