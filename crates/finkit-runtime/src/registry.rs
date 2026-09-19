//! Runtime factor registry.

use std::collections::HashMap;
use std::sync::Arc;

use crate::factories::{EmaFactory, MacdFactory, RsiFactory, SmaFactory};
use crate::factory::{DynFactor, FactorFactoryError, FactorFactoryRequest, FactorProvider};

#[derive(Clone)]
pub struct FactorRegistry {
    providers: HashMap<String, Arc<dyn FactorProvider>>,
}

impl std::fmt::Debug for FactorRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FactorRegistry")
            .field("provider_count", &self.providers.len())
            .finish()
    }
}

impl Default for FactorRegistry {
    fn default() -> Self {
        Self {
            providers: HashMap::new(),
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

    pub fn register_provider<P>(&mut self, provider: P)
    where
        P: FactorProvider + 'static,
    {
        self.providers
            .insert(provider.name().to_ascii_uppercase(), Arc::new(provider));
    }

    /// Register a provider using the factory terminology used by the public
    /// request model.
    pub fn register_factory<P>(&mut self, provider: P)
    where
        P: FactorProvider + 'static,
    {
        self.register_provider(provider);
    }

    pub fn get_provider(&self, name: &str) -> Option<Arc<dyn FactorProvider>> {
        self.providers.get(&name.to_ascii_uppercase()).cloned()
    }

    pub fn create_factor(
        &self,
        request: &FactorFactoryRequest,
    ) -> Result<DynFactor, FactorRegistryError> {
        let name = request.name.to_ascii_uppercase();
        let provider = self
            .get_provider(&name)
            .ok_or_else(|| FactorRegistryError::UnknownFactor(name.clone()))?;
        let params = request
            .try_params_map()
            .map_err(FactorRegistryError::Factory)?;
        provider
            .create(&params)
            .map_err(FactorRegistryError::Factory)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.providers.contains_key(&name.to_ascii_uppercase())
    }

    pub fn len(&self) -> usize {
        self.providers.len()
    }
}

/// Registry-level construction error with enough context for bindings to
/// expose a structured invalid-request response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FactorRegistryError {
    UnknownFactor(String),
    Factory(FactorFactoryError),
}

impl std::fmt::Display for FactorRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownFactor(name) => write!(f, "unknown factor: {name}"),
            Self::Factory(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for FactorRegistryError {}
