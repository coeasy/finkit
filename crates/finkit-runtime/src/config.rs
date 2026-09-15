//! Factor execution configuration.

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FactorConfig {
    pub name: String,
    pub params: HashMap<String, String>,
}

impl FactorConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: HashMap::new(),
        }
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }
}
