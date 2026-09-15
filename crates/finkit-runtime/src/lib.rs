//! Execution runtime foundation for factor graphs.

#[derive(Clone, Debug)]
pub struct ExecutionPlan {
    pub name: String,
}

impl ExecutionPlan {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}
