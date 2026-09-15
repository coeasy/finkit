//! Execution runtime foundation for factor graphs.

mod graph;
mod cache;

pub use graph::{FactorGraph, FactorNode};
pub use cache::FactorCache;

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
