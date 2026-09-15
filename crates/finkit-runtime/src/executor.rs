//! Factor execution engine.

use std::sync::Arc;

use crate::{ExecutionPlan, FactorCache, FactorConfig, FactorFactory, FactorFactoryRequest, FactorRegistry};

#[derive(Clone, Debug)]
pub struct Executor {
    pub plan: ExecutionPlan,
    pub registry: FactorRegistry,
    pub cache: FactorCache,
}

impl Executor {
    pub fn new(plan: ExecutionPlan, registry: FactorRegistry, cache: FactorCache) -> Self {
        Self {
            plan,
            registry,
            cache,
        }
    }

    pub fn node_count(&self) -> usize {
        self.plan.nodes.len()
    }

    pub fn resolve_factor(&self, request: &FactorFactoryRequest) -> Option<String> {
        self.registry.create_factor(request)
    }

    pub fn resolve_factory(
        &self,
        name: &str,
    ) -> Option<Arc<dyn FactorFactory + Send + Sync>> {
        self.registry.get_factory(name)
    }

    pub fn resolve_config(&self, request: &FactorFactoryRequest) -> Option<FactorConfig> {
        let mut config = FactorConfig::new(request.name.clone());
        for (key, value) in &request.params {
            config = config.with_param(key.clone(), value.clone());
        }
        self.registry.contains(&config.name).then_some(config)
    }
}
