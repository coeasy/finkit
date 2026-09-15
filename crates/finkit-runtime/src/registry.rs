//! Runtime factor registry foundation.

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FactorDescriptor {
    pub name: String,
    pub params: HashMap<String, String>,
}

#[derive(Clone, Debug, Default)]
pub struct FactorRegistry {
    factors: Vec<FactorDescriptor>,
}

impl FactorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, name: impl Into<String>) {
        self.factors.push(FactorDescriptor {
            name: name.into(),
            params: HashMap::new(),
        });
    }

    pub fn register_with_params(
        &mut self,
        name: impl Into<String>,
        params: HashMap<String, String>,
    ) {
        self.factors.push(FactorDescriptor {
            name: name.into(),
            params,
        });
    }

    pub fn contains(&self, name: &str) -> bool {
        self.factors.iter().any(|item| item.name == name)
    }

    pub fn len(&self) -> usize {
        self.factors.len()
    }
}
