//! Factor dependency graph primitives.

#[derive(Clone, Debug)]
pub struct FactorNode {
    pub id: String,
    pub factor_name: String,
    pub dependencies: Vec<String>,
}

impl FactorNode {
    pub fn new(id: impl Into<String>, factor_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            factor_name: factor_name.into(),
            dependencies: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct FactorGraph {
    pub nodes: Vec<FactorNode>,
}

impl FactorGraph {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_node(&mut self, node: FactorNode) {
        self.nodes.push(node);
    }
}
