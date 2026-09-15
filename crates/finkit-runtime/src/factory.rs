//! Factor creation abstraction.

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FactorFactoryRequest {
    pub name: String,
    pub params: Vec<(String, String)>,
}

impl FactorFactoryRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: Vec::new(),
        }
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.push((key.into(), value.into()));
        self
    }

    pub fn params_map(&self) -> HashMap<String, String> {
        self.params.iter().cloned().collect()
    }
}

pub trait FactorFactory {
    fn name(&self) -> &str;

    fn create(&self, params: &HashMap<String, String>) -> String;
}

#[derive(Clone, Debug)]
pub struct SimpleFactorFactory {
    name: String,
}

impl SimpleFactorFactory {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

impl FactorFactory for SimpleFactorFactory {
    fn name(&self) -> &str {
        &self.name
    }

    fn create(&self, _params: &HashMap<String, String>) -> String {
        self.name.clone()
    }
}
