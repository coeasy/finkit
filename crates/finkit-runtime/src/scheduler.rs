//! Dependency-aware factor graph scheduling utilities.

use crate::FactorNode;
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchedulerError {
    DuplicateNode(String),
    MissingDependency { node: String, dependency: String },
    Cycle,
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateNode(id) => write!(f, "duplicate factor node: {id}"),
            Self::MissingDependency { node, dependency } => {
                write!(f, "node {node} depends on missing node {dependency}")
            }
            Self::Cycle => f.write_str("factor graph contains a dependency cycle"),
        }
    }
}

impl std::error::Error for SchedulerError {}

#[derive(Clone, Debug)]
pub struct Scheduler;

impl Scheduler {
    pub fn topological_order(nodes: &[FactorNode]) -> Result<Vec<FactorNode>, SchedulerError> {
        let mut by_id = HashMap::with_capacity(nodes.len());
        for node in nodes {
            if by_id.insert(node.id.clone(), node).is_some() {
                return Err(SchedulerError::DuplicateNode(node.id.clone()));
            }
        }

        let mut indegree = HashMap::with_capacity(nodes.len());
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
        for node in nodes {
            let mut unique_dependencies = BTreeSet::new();
            for dependency in &node.dependencies {
                if !by_id.contains_key(dependency) {
                    return Err(SchedulerError::MissingDependency {
                        node: node.id.clone(),
                        dependency: dependency.clone(),
                    });
                }
                unique_dependencies.insert(dependency.clone());
            }
            indegree.insert(node.id.clone(), unique_dependencies.len());
            for dependency in unique_dependencies {
                dependents
                    .entry(dependency)
                    .or_default()
                    .push(node.id.clone());
            }
        }

        let mut ready: BTreeSet<String> = indegree
            .iter()
            .filter_map(|(id, degree)| (*degree == 0).then_some(id.clone()))
            .collect();
        let mut ordered = Vec::with_capacity(nodes.len());
        while let Some(id) = ready.pop_first() {
            ordered.push(by_id[&id].clone());
            if let Some(children) = dependents.get(&id) {
                for child in children {
                    let degree = indegree
                        .get_mut(child)
                        .expect("dependent node was inserted into indegree");
                    *degree -= 1;
                    if *degree == 0 {
                        ready.insert(child.clone());
                    }
                }
            }
        }

        if ordered.len() != nodes.len() {
            return Err(SchedulerError::Cycle);
        }
        Ok(ordered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FactorNode;

    #[test]
    fn orders_dependencies_before_consumers() {
        let nodes = vec![
            FactorNode::new("ema", "EMA").depends_on("sma"),
            FactorNode::new("sma", "SMA"),
        ];
        let ordered = Scheduler::topological_order(&nodes).unwrap();
        assert_eq!(
            ordered
                .iter()
                .map(|node| node.id.as_str())
                .collect::<Vec<_>>(),
            ["sma", "ema"]
        );
    }

    #[test]
    fn rejects_missing_dependency_and_cycle() {
        let missing = [FactorNode::new("ema", "EMA").depends_on("missing")];
        assert!(matches!(
            Scheduler::topological_order(&missing),
            Err(SchedulerError::MissingDependency { .. })
        ));

        let cycle = vec![
            FactorNode::new("a", "SMA").depends_on("b"),
            FactorNode::new("b", "EMA").depends_on("a"),
        ];
        assert!(matches!(
            Scheduler::topological_order(&cycle),
            Err(SchedulerError::Cycle)
        ));
    }
}
