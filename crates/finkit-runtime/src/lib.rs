//! Execution runtime foundation for factor graphs.

mod cache;
mod cache_key;
mod executor;
mod graph;
mod registry;
mod scheduler;

pub use cache::FactorCache;
pub use cache_key::FactorCacheKey;
pub use executor::Executor;
pub use graph::{FactorGraph, FactorNode};
pub use registry::{FactorDescriptor, FactorRegistry};
pub use scheduler::Scheduler;

#[derive(Clone, Debug)]
pub struct ExecutionPlan {
    pub name: String,
    pub nodes: Vec<FactorNode>,
}

impl ExecutionPlan {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
        }
    }

    pub fn from_graph(name: impl Into<String>, graph: &FactorGraph) -> Self {
        Self {
            name: name.into(),
            nodes: graph.nodes.clone(),
        }
    }
}
