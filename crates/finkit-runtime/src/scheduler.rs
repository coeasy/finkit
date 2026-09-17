//! Factor graph scheduling utilities.

use crate::FactorNode;

#[derive(Clone, Debug)]
pub struct Scheduler;

impl Scheduler {
    pub fn topological_order(nodes: &[FactorNode]) -> Vec<FactorNode> {
        // Initial implementation keeps insertion order. The runtime contract
        // allows replacing this with a dependency-aware DAG scheduler.
        nodes.to_vec()
    }
}
