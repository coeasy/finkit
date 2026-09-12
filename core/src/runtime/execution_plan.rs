//! Reusable DAG execution plan with common-subexpression interning.
//!
//! The plan is deliberately backend-neutral: Formula, Factor and indicator
//! frontends can compile semantic work into the same node/dependency model.

use crate::math::kernels::KernelFamily;
use std::collections::{BTreeMap, VecDeque};
use std::fmt;

/// Stable zero-based identifier for one plan node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub usize);

/// One reusable kernel node in an execution plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNode {
    /// Node identifier. Must equal this node's index in the compiled node list.
    pub id: NodeId,
    /// Canonical kernel family.
    pub family: KernelFamily,
    /// Semantic operation key used for common-subexpression interning.
    pub key: String,
    /// Direct dependencies consumed by this node.
    pub dependencies: Vec<NodeId>,
    /// Historical rows required by this node itself.
    pub lookback: usize,
}

/// Plan validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionPlanError {
    /// Node ids must match their position in the plan.
    InvalidNodeId { expected: usize, actual: usize },
    /// A node references a dependency not present in the plan.
    UnknownDependency { node: NodeId, dependency: NodeId },
    /// Dependency graph contains a cycle.
    Cycle,
}

impl fmt::Display for ExecutionPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNodeId { expected, actual } => {
                write!(f, "invalid plan node id {actual}; expected {expected}")
            }
            Self::UnknownDependency { node, dependency } => write!(
                f,
                "plan node {} references unknown dependency {}",
                node.0, dependency.0
            ),
            Self::Cycle => write!(f, "execution plan dependency graph contains a cycle"),
        }
    }
}

impl std::error::Error for ExecutionPlanError {}

/// Fully validated reusable execution plan.
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    nodes: Vec<PlanNode>,
    order: Vec<NodeId>,
    cumulative_lookback: Vec<usize>,
    max_lookback: usize,
}

impl ExecutionPlan {
    /// Compile and validate an arbitrary node list.
    pub fn from_nodes(nodes: Vec<PlanNode>) -> Result<Self, ExecutionPlanError> {
        for (index, node) in nodes.iter().enumerate() {
            if node.id.0 != index {
                return Err(ExecutionPlanError::InvalidNodeId {
                    expected: index,
                    actual: node.id.0,
                });
            }
            for dependency in &node.dependencies {
                if dependency.0 >= nodes.len() {
                    return Err(ExecutionPlanError::UnknownDependency {
                        node: node.id,
                        dependency: *dependency,
                    });
                }
            }
        }

        let mut indegree = vec![0_usize; nodes.len()];
        let mut dependents = vec![Vec::<NodeId>::new(); nodes.len()];
        for node in &nodes {
            indegree[node.id.0] = node.dependencies.len();
            for dependency in &node.dependencies {
                dependents[dependency.0].push(node.id);
            }
        }

        let mut queue = VecDeque::new();
        for (index, degree) in indegree.iter().enumerate() {
            if *degree == 0 {
                queue.push_back(NodeId(index));
            }
        }

        let mut order = Vec::with_capacity(nodes.len());
        while let Some(node) = queue.pop_front() {
            order.push(node);
            for dependent in &dependents[node.0] {
                let degree = &mut indegree[dependent.0];
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(*dependent);
                }
            }
        }
        if order.len() != nodes.len() {
            return Err(ExecutionPlanError::Cycle);
        }

        let mut cumulative_lookback = vec![0_usize; nodes.len()];
        let mut max_lookback = 0_usize;
        for node_id in &order {
            let node = &nodes[node_id.0];
            let dependency_lookback = node
                .dependencies
                .iter()
                .map(|dependency| cumulative_lookback[dependency.0])
                .max()
                .unwrap_or(0);
            let lookback = dependency_lookback.saturating_add(node.lookback);
            cumulative_lookback[node_id.0] = lookback;
            max_lookback = max_lookback.max(lookback);
        }

        Ok(Self {
            nodes,
            order,
            cumulative_lookback,
            max_lookback,
        })
    }

    /// Number of unique nodes after planner interning.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the plan contains no work.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Topological execution order.
    #[must_use]
    pub fn execution_order(&self) -> &[NodeId] {
        &self.order
    }

    /// Resolve one node.
    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&PlanNode> {
        self.nodes.get(id.0)
    }

    /// Cumulative historical dependency requirement for one node.
    #[must_use]
    pub fn cumulative_lookback(&self, id: NodeId) -> Option<usize> {
        self.cumulative_lookback.get(id.0).copied()
    }

    /// Maximum cumulative lookback across the complete plan.
    #[must_use]
    pub const fn max_lookback(&self) -> usize {
        self.max_lookback
    }
}

/// Builder that interns semantically identical nodes before plan compilation.
#[derive(Debug, Default)]
pub struct ExecutionPlanBuilder {
    nodes: Vec<PlanNode>,
    interned: BTreeMap<(u8, String, Vec<NodeId>, usize), NodeId>,
}

impl ExecutionPlanBuilder {
    /// Create an empty planner.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            nodes: Vec::new(),
            interned: BTreeMap::new(),
        }
    }

    /// Add or reuse one semantic kernel node.
    ///
    /// `dependencies` are order-sensitive so non-commutative operations are
    /// never incorrectly merged.
    pub fn intern_node(
        &mut self,
        family: KernelFamily,
        key: impl Into<String>,
        dependencies: Vec<NodeId>,
        lookback: usize,
    ) -> Result<NodeId, ExecutionPlanError> {
        for dependency in &dependencies {
            if dependency.0 >= self.nodes.len() {
                return Err(ExecutionPlanError::UnknownDependency {
                    node: NodeId(self.nodes.len()),
                    dependency: *dependency,
                });
            }
        }

        let key = key.into();
        let signature = (
            family_rank(family),
            key.clone(),
            dependencies.clone(),
            lookback,
        );
        if let Some(existing) = self.interned.get(&signature) {
            return Ok(*existing);
        }

        let id = NodeId(self.nodes.len());
        self.nodes.push(PlanNode {
            id,
            family,
            key,
            dependencies,
            lookback,
        });
        self.interned.insert(signature, id);
        Ok(id)
    }

    /// Validate dependencies and produce an immutable plan.
    pub fn build(self) -> Result<ExecutionPlan, ExecutionPlanError> {
        ExecutionPlan::from_nodes(self.nodes)
    }
}

const fn family_rank(family: KernelFamily) -> u8 {
    match family {
        KernelFamily::MovingAverage => 0,
        KernelFamily::Statistics => 1,
        KernelFamily::Volatility => 2,
        KernelFamily::Extrema => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planner_interns_common_subexpressions() {
        let mut builder = ExecutionPlanBuilder::new();
        let input = builder
            .intern_node(KernelFamily::MovingAverage, "close", vec![], 0)
            .unwrap();
        let ema_a = builder
            .intern_node(KernelFamily::MovingAverage, "ema:12", vec![input], 11)
            .unwrap();
        let ema_b = builder
            .intern_node(KernelFamily::MovingAverage, "ema:12", vec![input], 11)
            .unwrap();
        assert_eq!(ema_a, ema_b);
        let plan = builder.build().unwrap();
        assert_eq!(plan.len(), 2);
    }

    #[test]
    fn cumulative_lookback_follows_dependency_chain() {
        let mut builder = ExecutionPlanBuilder::new();
        let input = builder
            .intern_node(KernelFamily::MovingAverage, "close", vec![], 0)
            .unwrap();
        let rolling = builder
            .intern_node(KernelFamily::MovingAverage, "sma:20", vec![input], 19)
            .unwrap();
        let stats = builder
            .intern_node(KernelFamily::Statistics, "stddev:10", vec![rolling], 9)
            .unwrap();
        let plan = builder.build().unwrap();
        assert_eq!(plan.cumulative_lookback(stats), Some(28));
        assert_eq!(plan.max_lookback(), 28);
    }

    #[test]
    fn cycle_is_rejected() {
        let nodes = vec![
            PlanNode {
                id: NodeId(0),
                family: KernelFamily::MovingAverage,
                key: "a".to_string(),
                dependencies: vec![NodeId(1)],
                lookback: 0,
            },
            PlanNode {
                id: NodeId(1),
                family: KernelFamily::Statistics,
                key: "b".to_string(),
                dependencies: vec![NodeId(0)],
                lookback: 0,
            },
        ];
        assert_eq!(
            ExecutionPlan::from_nodes(nodes).unwrap_err(),
            ExecutionPlanError::Cycle
        );
    }
}
