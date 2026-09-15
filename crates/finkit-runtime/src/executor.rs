//! Factor execution engine.

use crate::{ExecutionPlan, FactorCache, FactorRegistry};

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
}
