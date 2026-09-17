//! Runtime factor registry.

use std::collections::HashMap;
use std::sync::Arc;

use crate::factories::{EmaFactory, MacdFactory, RsiFactory, SmaFactory};
use crate::factory::{FactorFactory, FactorFactoryRequest};

#[derive(Clone)]
pub struct FactorRegistry {
    factories: HashMap<String, Arc<dyn FactorFactory + Send + Sync>>,
}

impl std::fmt::Debug for FactorRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FactorRegistry")
            .field("factory_count", &self.factories.len())
            .finish()
    }
}

impl Default for FactorRegistry {
    fn default() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }
}

impl FactorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtin() -> Self {
        let mut registry = Self::new();
        registry.register_factory(EmaFactory);
        registry.register_factory(SmaFactory);
        registry.register_factory(RsiFactory);
        registry.register_factory(MacdFactory);
        registry
    }

    pub fn register_factory<F>(&mut self, factory: F)
    where
        F: FactorFactory + Send + Sync + 'static,
    {
        self.factories
            .insert(factory.name().to_string(), Arc::new(factory));
    }

    pub fn get_factory(&self, name: &str) -> Option<Arc<dyn FactorFactory + Send + Sync>> {
        self.factories.get(name).cloned()
    }

    pub fn create_factor(&self, request: &FactorFactoryRequest) -> Option<String> {
        let factory = self.factories.get(&request.name)?;
        Some(factory.create(&request.params_map()))
    }

    pub fn contains(&self, name: &str) -> bool {
        self.factories.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.factories.len()
    }
}
