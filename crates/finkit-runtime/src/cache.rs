//! Runtime cache primitives.

use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct FactorCache {
    entries: HashMap<String, String>,
}

impl FactorCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.entries.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.entries.get(key)
    }
}
