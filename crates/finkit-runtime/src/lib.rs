//! Execution runtime foundation for factor graphs.

mod cache;
mod executor;
mod graph;
mod scheduler;

pub use cache::FactorCache;
pub use executor::Executor;
pub use graph::{FactorGraph, FactorNode};
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
