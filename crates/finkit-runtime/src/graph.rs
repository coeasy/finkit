//! Factor dependency graph primitives.

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FactorNode {
    pub id: String,
    pub factor_name: String,
    pub dependencies: Vec<String>,
    pub params: HashMap<String, String>,
}

impl FactorNode {
    pub fn new(id: impl Into<String>, factor_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            factor_name: factor_name.into(),
            dependencies: Vec::new(),
            params: HashMap::new(),
        }
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
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
