//! Reusable DAG execution plan with common-subexpression interning.
//!
//! The plan is deliberately backend-neutral: Formula, Factor and indicator
//! frontends can compile semantic work into the same node/dependency model.
//! Fixed-window, recursive, and runtime-resolved dependencies are represented
//! separately so DirtyRange scheduling never guesses a history contract.

use crate::math::kernels::KernelFamily;
use std::collections::{BTreeMap, VecDeque};
use std::fmt;

/// Stable zero-based identifier for one plan node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub usize);

/// Historical dependency contract for a plan node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyHorizon {
    /// Output depends on at most this many prior rows.
    Fixed(usize),
    /// Output recursively depends on all prior state unless replay starts from
    /// a valid checkpoint.
    Recursive,
    /// A finite horizon exists, but runtime parameters must resolve its exact
    /// value before a minimal DirtyRange can be proven.
    Dynamic,
}

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
    /// Historical dependency contract for this operation.
    pub horizon: DependencyHorizon,
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
    cumulative_lookback: Vec<Option<usize>>,
    max_lookback: Option<usize>,
    has_recursive_horizon: bool,
    has_dynamic_horizon: bool,
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

        let has_recursive_horizon = nodes
            .iter()
            .any(|node| node.horizon == DependencyHorizon::Recursive);
        let has_dynamic_horizon = nodes
            .iter()
            .any(|node| node.horizon == DependencyHorizon::Dynamic);

        let mut cumulative_lookback = vec![Some(0_usize); nodes.len()];
        for node_id in &order {
            let node = &nodes[node_id.0];
            let dependency_lookback =
                node.dependencies
                    .iter()
                    .try_fold(0_usize, |current, dependency| {
                        cumulative_lookback[dependency.0].map(|value| current.max(value))
                    });
            cumulative_lookback[node_id.0] = match (dependency_lookback, node.horizon) {
                (Some(dependency), DependencyHorizon::Fixed(lookback)) => {
                    Some(dependency.saturating_add(lookback))
                }
                _ => None,
            };
        }

        let max_lookback = cumulative_lookback
            .iter()
            .copied()
            .try_fold(0_usize, |current, lookback| {
                lookback.map(|value| current.max(value))
            });

        Ok(Self {
            nodes,
            order,
            cumulative_lookback,
            max_lookback,
            has_recursive_horizon,
            has_dynamic_horizon,
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

    /// Cumulative finite lookback for one node. `None` means the node itself
    /// or one dependency is recursive/dynamic.
    #[must_use]
    pub fn cumulative_lookback(&self, id: NodeId) -> Option<usize> {
        self.cumulative_lookback.get(id.0).copied().flatten()
    }

    /// Maximum finite lookback across the plan. `None` means the plan first
    /// needs a checkpoint (recursive) or runtime parameter resolution (dynamic).
    #[must_use]
    pub const fn max_lookback(&self) -> Option<usize> {
        self.max_lookback
    }

    /// Whether at least one node has recursive state.
    #[must_use]
    pub const fn has_recursive_horizon(&self) -> bool {
        self.has_recursive_horizon
    }

    /// Whether at least one node still needs a runtime-resolved finite horizon.
    #[must_use]
    pub const fn has_dynamic_horizon(&self) -> bool {
        self.has_dynamic_horizon
    }

    /// Whether DirtyRange can be derived from finite lookbacks alone.
    #[must_use]
    pub const fn supports_dirty_range_without_checkpoint(&self) -> bool {
        self.max_lookback.is_some()
    }
}

/// Builder that interns semantically identical nodes before plan compilation.
#[derive(Debug, Default)]
pub struct ExecutionPlanBuilder {
    nodes: Vec<PlanNode>,
    interned: BTreeMap<(u8, String, Vec<NodeId>, DependencyHorizon), NodeId>,
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

    /// Add or reuse a fixed-window semantic kernel node.
    pub fn intern_node(
        &mut self,
        family: KernelFamily,
        key: impl Into<String>,
        dependencies: Vec<NodeId>,
        lookback: usize,
    ) -> Result<NodeId, ExecutionPlanError> {
        self.intern_with_horizon(
            family,
            key,
            dependencies,
            DependencyHorizon::Fixed(lookback),
        )
    }

    /// Add or reuse a recursive state node such as EMA, ATR or ADX.
    pub fn intern_recursive_node(
        &mut self,
        family: KernelFamily,
        key: impl Into<String>,
        dependencies: Vec<NodeId>,
    ) -> Result<NodeId, ExecutionPlanError> {
        self.intern_with_horizon(family, key, dependencies, DependencyHorizon::Recursive)
    }

    /// Add or reuse a finite-window node whose horizon must be resolved from
    /// runtime parameters before minimal DirtyRange scheduling is allowed.
    pub fn intern_dynamic_node(
        &mut self,
        family: KernelFamily,
        key: impl Into<String>,
        dependencies: Vec<NodeId>,
    ) -> Result<NodeId, ExecutionPlanError> {
        self.intern_with_horizon(family, key, dependencies, DependencyHorizon::Dynamic)
    }

    fn intern_with_horizon(
        &mut self,
        family: KernelFamily,
        key: impl Into<String>,
        dependencies: Vec<NodeId>,
        horizon: DependencyHorizon,
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
            horizon,
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
            horizon,
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
        KernelFamily::Scalar => 0,
        KernelFamily::MovingAverage => 1,
        KernelFamily::Statistics => 2,
        KernelFamily::Volatility => 3,
        KernelFamily::Extrema => 4,
        KernelFamily::Momentum => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planner_interns_common_subexpressions() {
        let mut builder = ExecutionPlanBuilder::new();
        let input = builder
            .intern_node(KernelFamily::Scalar, "close", vec![], 0)
            .unwrap();
        let sma_a = builder
            .intern_node(KernelFamily::MovingAverage, "sma:12", vec![input], 11)
            .unwrap();
        let sma_b = builder
            .intern_node(KernelFamily::MovingAverage, "sma:12", vec![input], 11)
            .unwrap();
        assert_eq!(sma_a, sma_b);
        let plan = builder.build().unwrap();
        assert_eq!(plan.len(), 2);
    }

    #[test]
    fn cumulative_lookback_follows_dependency_chain() {
        let mut builder = ExecutionPlanBuilder::new();
        let input = builder
            .intern_node(KernelFamily::Scalar, "close", vec![], 0)
            .unwrap();
        let rolling = builder
            .intern_node(KernelFamily::MovingAverage, "sma:20", vec![input], 19)
            .unwrap();
        let stats = builder
            .intern_node(KernelFamily::Statistics, "stddev:10", vec![rolling], 9)
            .unwrap();
        let plan = builder.build().unwrap();
        assert_eq!(plan.cumulative_lookback(stats), Some(28));
        assert_eq!(plan.max_lookback(), Some(28));
        assert!(plan.supports_dirty_range_without_checkpoint());
    }

    #[test]
    fn recursive_dependency_propagates_through_downstream_nodes() {
        let mut builder = ExecutionPlanBuilder::new();
        let input = builder
            .intern_node(KernelFamily::Scalar, "close", vec![], 0)
            .unwrap();
        let ema = builder
            .intern_recursive_node(KernelFamily::MovingAverage, "ema:12", vec![input])
            .unwrap();
        let stats = builder
            .intern_node(KernelFamily::Statistics, "stddev:10", vec![ema], 9)
            .unwrap();
        let plan = builder.build().unwrap();
        assert_eq!(plan.cumulative_lookback(ema), None);
        assert_eq!(plan.cumulative_lookback(stats), None);
        assert_eq!(plan.max_lookback(), None);
        assert!(plan.has_recursive_horizon());
        assert!(!plan.has_dynamic_horizon());
    }

    #[test]
    fn dynamic_dependency_remains_unresolved() {
        let mut builder = ExecutionPlanBuilder::new();
        let input = builder
            .intern_node(KernelFamily::Scalar, "close", vec![], 0)
            .unwrap();
        let dynamic = builder
            .intern_dynamic_node(KernelFamily::MovingAverage, "sma:runtime", vec![input])
            .unwrap();
        let plan = builder.build().unwrap();
        assert_eq!(plan.cumulative_lookback(dynamic), None);
        assert_eq!(plan.max_lookback(), None);
        assert!(plan.has_dynamic_horizon());
        assert!(!plan.has_recursive_horizon());
    }

    #[test]
    fn cycle_is_rejected() {
        let nodes = vec![
            PlanNode {
                id: NodeId(0),
                family: KernelFamily::Scalar,
                key: "a".to_string(),
                dependencies: vec![NodeId(1)],
                horizon: DependencyHorizon::Fixed(0),
            },
            PlanNode {
                id: NodeId(1),
                family: KernelFamily::Statistics,
                key: "b".to_string(),
                dependencies: vec![NodeId(0)],
                horizon: DependencyHorizon::Fixed(0),
            },
        ];
        assert_eq!(
            ExecutionPlan::from_nodes(nodes).unwrap_err(),
            ExecutionPlanError::Cycle
        );
    }
}
