//! Factor execution engine.

use crate::ExecutionPlan;

#[derive(Clone, Debug)]
pub struct Executor {
    pub plan: ExecutionPlan,
}

impl Executor {
    pub fn new(plan: ExecutionPlan) -> Self {
        Self { plan }
    }

    pub fn node_count(&self) -> usize {
        self.plan.nodes.len()
    }
}
