//! Unified compute-planning primitives shared by formula, factor, indicator,
//! and future backend dispatch layers.
//!
//! The module deliberately separates *semantic planning* from execution. A
//! [`ComputePlan`] records dependency order and observable effects so future
//! optimizers do not need to infer safety from syntax alone. [`FactorPlan`]
//! performs the same up-front dependency validation for the existing factor
//! engine while preserving its public execution contract.

use crate::factors::{FactorContext, FactorEngine, FactorError, FactorRegistry, FactorResult};
use crate::registry::{FunctionSpec, LookbackSpec};
use crate::runtime::{MarketFrame, NanPolicy, RuntimeError, WarmupPolicy};
use std::collections::{BTreeMap, BTreeSet};

/// Stable identifier for one node in a compute graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ComputeNodeId(pub usize);

/// Lookback requirement known by the planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookbackRequirement {
    /// No historical rows are required.
    None,
    /// Lookback equals `period - 1` and is resolved from runtime parameters.
    PeriodMinusOne,
    /// Lookback equals `period` and is resolved from runtime parameters.
    Period,
    /// A fixed number of historical rows is required.
    Fixed(usize),
    /// The lookback depends on function-specific or dynamic semantics.
    Dynamic,
}

impl From<LookbackSpec> for LookbackRequirement {
    fn from(value: LookbackSpec) -> Self {
        match value {
            LookbackSpec::None => Self::None,
            LookbackSpec::PeriodMinusOne => Self::PeriodMinusOne,
            LookbackSpec::Period => Self::Period,
            LookbackSpec::Dynamic => Self::Dynamic,
        }
    }
}

/// Observable effect produced by a compute node.
///
/// Optimizers may freely eliminate or reorder [`Self::Pure`] nodes when the
/// data dependencies allow it. All other variants are observable and must be
/// retained unless a transformation can prove semantic equivalence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputeEffect {
    /// Pure value computation without an observable side effect.
    Pure,
    /// Formula-style assignment visible through the execution context.
    WriteVariable(String),
    /// Named output emitted to the caller.
    EmitOutput(String),
    /// Drawing/chart side effect.
    Draw,
    /// Stateful operation whose internal state changes across evaluations.
    Stateful,
}

impl ComputeEffect {
    /// Whether the node has no observable side effect.
    pub const fn is_pure(&self) -> bool {
        matches!(self, Self::Pure)
    }
}

/// Execution capabilities attached to a compute node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeCapabilities {
    /// Repeated execution with identical inputs returns identical outputs.
    pub deterministic: bool,
    /// The operation supports incremental/streaming evaluation.
    pub streaming: bool,
    /// The operation carries mutable state between updates.
    pub stateful: bool,
    /// Historical dependency requirement.
    pub lookback: LookbackRequirement,
    /// Observable execution effect.
    pub effect: ComputeEffect,
}

impl ComputeCapabilities {
    /// Build capability metadata from the canonical function registry.
    ///
    /// Registry entries describe public functions, which are pure value
    /// computations by default. Formula assignments/drawings override the
    /// effect when they are lowered from the AST into compute nodes.
    pub fn from_function_spec(spec: &FunctionSpec) -> Self {
        Self {
            deterministic: spec.deterministic,
            streaming: spec.streaming,
            stateful: spec.streaming,
            lookback: spec.lookback.into(),
            effect: ComputeEffect::Pure,
        }
    }
}

/// One semantic operation in a compute graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeNode {
    /// Stable node identifier within the plan.
    pub id: ComputeNodeId,
    /// Canonical operation name or semantic label.
    pub operation: String,
    /// Nodes that must execute before this node.
    pub dependencies: Vec<ComputeNodeId>,
    /// Planner-visible execution capabilities.
    pub capabilities: ComputeCapabilities,
}

impl ComputeNode {
    /// Construct a compute node.
    pub fn new(
        id: ComputeNodeId,
        operation: impl Into<String>,
        dependencies: Vec<ComputeNodeId>,
        capabilities: ComputeCapabilities,
    ) -> Self {
        Self {
            id,
            operation: operation.into(),
            dependencies,
            capabilities,
        }
    }
}

/// Errors produced while compiling a generic compute plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputePlanError {
    /// A node id appears more than once.
    DuplicateNode(ComputeNodeId),
    /// A node references a dependency that is not present in the plan.
    UnknownDependency {
        /// Node containing the invalid dependency.
        node: ComputeNodeId,
        /// Missing dependency id.
        dependency: ComputeNodeId,
    },
    /// A node has an empty semantic operation name.
    EmptyOperation(ComputeNodeId),
    /// The dependency graph contains a cycle. The list contains the nodes that
    /// remain cyclic after deterministic topological sorting.
    DependencyCycle(Vec<ComputeNodeId>),
}

/// Validated immutable compute graph with deterministic execution order.
#[derive(Debug, Clone)]
pub struct ComputePlan {
    nodes: BTreeMap<ComputeNodeId, ComputeNode>,
    execution_order: Vec<ComputeNodeId>,
    supports_streaming: bool,
    has_observable_effects: bool,
}

impl ComputePlan {
    /// Validate a graph and compile a stable topological execution order.
    pub fn compile(nodes: impl IntoIterator<Item = ComputeNode>) -> Result<Self, ComputePlanError> {
        let mut by_id = BTreeMap::new();
        for node in nodes {
            if node.operation.trim().is_empty() {
                return Err(ComputePlanError::EmptyOperation(node.id));
            }
            let id = node.id;
            if by_id.insert(id, node).is_some() {
                return Err(ComputePlanError::DuplicateNode(id));
            }
        }

        let mut indegree: BTreeMap<ComputeNodeId, usize> =
            by_id.keys().copied().map(|id| (id, 0)).collect();
        let mut outgoing: BTreeMap<ComputeNodeId, Vec<ComputeNodeId>> = BTreeMap::new();

        for node in by_id.values() {
            for &dependency in &node.dependencies {
                if !by_id.contains_key(&dependency) {
                    return Err(ComputePlanError::UnknownDependency {
                        node: node.id,
                        dependency,
                    });
                }
                *indegree
                    .get_mut(&node.id)
                    .expect("all compute nodes have an indegree entry") += 1;
                outgoing.entry(dependency).or_default().push(node.id);
            }
        }

        for children in outgoing.values_mut() {
            children.sort_unstable();
            children.dedup();
        }

        let mut ready: BTreeSet<ComputeNodeId> = indegree
            .iter()
            .filter_map(|(&id, &degree)| (degree == 0).then_some(id))
            .collect();
        let mut execution_order = Vec::with_capacity(by_id.len());

        while let Some(id) = ready.pop_first() {
            execution_order.push(id);
            if let Some(children) = outgoing.get(&id) {
                for child in children {
                    let degree = indegree
                        .get_mut(child)
                        .expect("all compute children have an indegree entry");
                    *degree -= 1;
                    if *degree == 0 {
                        ready.insert(*child);
                    }
                }
            }
        }

        if execution_order.len() != by_id.len() {
            let cycle = indegree
                .into_iter()
                .filter_map(|(id, degree)| (degree > 0).then_some(id))
                .collect();
            return Err(ComputePlanError::DependencyCycle(cycle));
        }

        let supports_streaming = by_id
            .values()
            .all(|node| node.capabilities.streaming);
        let has_observable_effects = by_id
            .values()
            .any(|node| !node.capabilities.effect.is_pure());

        Ok(Self {
            nodes: by_id,
            execution_order,
            supports_streaming,
            has_observable_effects,
        })
    }

    /// Return one node by id.
    pub fn node(&self, id: ComputeNodeId) -> Option<&ComputeNode> {
        self.nodes.get(&id)
    }

    /// Deterministic topological execution order.
    pub fn execution_order(&self) -> &[ComputeNodeId] {
        &self.execution_order
    }

    /// Whether every node supports streaming execution.
    pub const fn supports_streaming(&self) -> bool {
        self.supports_streaming
    }

    /// Whether at least one node has an observable side effect.
    pub const fn has_observable_effects(&self) -> bool {
        self.has_observable_effects
    }

    /// Number of nodes in the plan.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the plan contains no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// Runtime policies shared by planned computation entry points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExecutionPolicy {
    /// Missing-value handling at the runtime boundary.
    pub nan: NanPolicy,
    /// Warm-up representation in public outputs.
    pub warmup: WarmupPolicy,
}

/// Validated borrowed runtime input for future plan-based backends.
#[derive(Debug, Clone, Copy)]
pub struct ComputeInput<'a> {
    /// Zero-copy aligned market frame.
    pub frame: MarketFrame<'a>,
    /// Runtime data policies.
    pub policy: ExecutionPolicy,
}

impl<'a> ComputeInput<'a> {
    /// Validate a market frame and its requested missing-value policy.
    pub fn new(frame: MarketFrame<'a>, policy: ExecutionPolicy) -> Result<Self, RuntimeError> {
        frame.validate()?;
        frame.validate_nan_policy(policy.nan)?;
        Ok(Self { frame, policy })
    }
}

/// Precompiled dependency plan for the existing factor engine.
///
/// The first implementation intentionally delegates value computation to
/// [`FactorEngine`] so the public numerical semantics remain unchanged. The
/// plan moves graph validation, stable ordering and raw-input discovery out of
/// caller code and creates a stable seam for a future direct plan executor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactorPlan {
    targets: Vec<String>,
    execution_order: Vec<String>,
    required_raw_inputs: Vec<String>,
    dependencies: BTreeMap<String, Vec<String>>,
}

impl FactorPlan {
    /// Compile target factors against a registry.
    pub fn compile(registry: &FactorRegistry, targets: &[&str]) -> FactorResult<Self> {
        if targets.is_empty() {
            return Err(FactorError::InvalidParameter(
                "factor plan requires at least one target".to_string(),
            ));
        }

        let mut normalized_targets = Vec::with_capacity(targets.len());
        let mut seen_targets = BTreeSet::new();
        for &target in targets {
            if target.trim().is_empty() {
                return Err(FactorError::InvalidParameter(
                    "factor plan target must not be empty".to_string(),
                ));
            }
            if seen_targets.insert(target.to_string()) {
                normalized_targets.push(target.to_string());
            }
        }

        let mut visited = BTreeSet::new();
        let mut visiting = Vec::new();
        let mut execution_order = Vec::new();
        let mut raw_inputs = BTreeSet::new();
        let mut dependencies = BTreeMap::new();

        for target in &normalized_targets {
            visit_factor(
                registry,
                target,
                &mut visited,
                &mut visiting,
                &mut execution_order,
                &mut raw_inputs,
                &mut dependencies,
            )?;
        }

        Ok(Self {
            targets: normalized_targets,
            execution_order,
            required_raw_inputs: raw_inputs.into_iter().collect(),
            dependencies,
        })
    }

    /// Requested output factors in caller order, with duplicates removed.
    pub fn targets(&self) -> &[String] {
        &self.targets
    }

    /// Stable dependency-first execution order.
    pub fn execution_order(&self) -> &[String] {
        &self.execution_order
    }

    /// Raw series that must be supplied by the context.
    pub fn required_raw_inputs(&self) -> &[String] {
        &self.required_raw_inputs
    }

    /// Validate raw inputs before numerical execution starts.
    pub fn validate_context(&self, context: &FactorContext) -> FactorResult<()> {
        for input in &self.required_raw_inputs {
            if context.get(input).is_none() {
                return Err(FactorError::MissingInput(input.clone()));
            }
        }
        Ok(())
    }

    /// Execute the plan through the current factor engine implementation.
    ///
    /// This preserves the existing memoization and result semantics. A future
    /// optimized backend can consume [`Self::execution_order`] directly
    /// without changing this public planning contract.
    pub fn execute(
        &self,
        engine: &FactorEngine,
        context: &FactorContext,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        self.validate_context(context)?;
        self.validate_engine(engine)?;
        let targets: Vec<&str> = self.targets.iter().map(String::as_str).collect();
        engine.evaluate_many(&targets, context)
    }

    fn validate_engine(&self, engine: &FactorEngine) -> FactorResult<()> {
        for name in &self.execution_order {
            let current = engine
                .registry()
                .get(name)
                .ok_or_else(|| FactorError::UnknownFactor(name.clone()))?;
            let planned = self
                .dependencies
                .get(name)
                .expect("every planned factor stores its dependencies");
            if &current.dependencies != planned {
                return Err(FactorError::InvalidParameter(format!(
                    "factor plan is stale because dependencies changed for {name}"
                )));
            }
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_factor(
    registry: &FactorRegistry,
    name: &str,
    visited: &mut BTreeSet<String>,
    visiting: &mut Vec<String>,
    execution_order: &mut Vec<String>,
    raw_inputs: &mut BTreeSet<String>,
    dependencies: &mut BTreeMap<String, Vec<String>>,
) -> FactorResult<()> {
    if visited.contains(name) {
        return Ok(());
    }
    if let Some(position) = visiting.iter().position(|current| current == name) {
        let mut cycle = visiting[position..].to_vec();
        cycle.push(name.to_string());
        return Err(FactorError::DependencyCycle(cycle));
    }

    let factor = registry
        .get(name)
        .ok_or_else(|| FactorError::UnknownFactor(name.to_string()))?;
    visiting.push(name.to_string());
    dependencies.insert(name.to_string(), factor.dependencies.clone());

    for dependency in &factor.dependencies {
        if registry.get(dependency).is_some() {
            visit_factor(
                registry,
                dependency,
                visited,
                visiting,
                execution_order,
                raw_inputs,
                dependencies,
            )?;
        } else {
            raw_inputs.insert(dependency.clone());
        }
    }

    visiting.pop();
    visited.insert(name.to_string());
    execution_order.push(name.to_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factors::{FactorDefinition, FactorDirection, FactorKind};
    use crate::registry::builtin_function_registry;
    use std::sync::Arc;

    fn pure_capabilities(streaming: bool) -> ComputeCapabilities {
        ComputeCapabilities {
            deterministic: true,
            streaming,
            stateful: streaming,
            lookback: LookbackRequirement::None,
            effect: ComputeEffect::Pure,
        }
    }

    #[test]
    fn compute_plan_is_stable_and_retains_effect_metadata() {
        let nodes = vec![
            ComputeNode::new(ComputeNodeId(2), "MA", vec![ComputeNodeId(1)], pure_capabilities(true)),
            ComputeNode::new(ComputeNodeId(1), "CLOSE", vec![], pure_capabilities(true)),
            ComputeNode::new(
                ComputeNodeId(3),
                "ASSIGN_SELL",
                vec![ComputeNodeId(2)],
                ComputeCapabilities {
                    deterministic: true,
                    streaming: true,
                    stateful: false,
                    lookback: LookbackRequirement::None,
                    effect: ComputeEffect::WriteVariable("SELL".to_string()),
                },
            ),
        ];

        let plan = ComputePlan::compile(nodes).unwrap();
        assert_eq!(
            plan.execution_order(),
            &[ComputeNodeId(1), ComputeNodeId(2), ComputeNodeId(3)]
        );
        assert!(plan.supports_streaming());
        assert!(plan.has_observable_effects());
        assert_eq!(
            plan.node(ComputeNodeId(3)).unwrap().capabilities.effect,
            ComputeEffect::WriteVariable("SELL".to_string())
        );
    }

    #[test]
    fn compute_plan_rejects_unknown_dependency_and_cycle() {
        let unknown = ComputePlan::compile([ComputeNode::new(
            ComputeNodeId(1),
            "MA",
            vec![ComputeNodeId(99)],
            pure_capabilities(true),
        )]);
        assert!(matches!(
            unknown,
            Err(ComputePlanError::UnknownDependency { .. })
        ));

        let cycle = ComputePlan::compile([
            ComputeNode::new(
                ComputeNodeId(1),
                "A",
                vec![ComputeNodeId(2)],
                pure_capabilities(true),
            ),
            ComputeNode::new(
                ComputeNodeId(2),
                "B",
                vec![ComputeNodeId(1)],
                pure_capabilities(true),
            ),
        ]);
        assert!(matches!(cycle, Err(ComputePlanError::DependencyCycle(_))));
    }

    #[test]
    fn registry_specs_map_to_compute_capabilities() {
        let registry = builtin_function_registry();
        let sma = registry.get("SMA").unwrap();
        let capabilities = ComputeCapabilities::from_function_spec(sma);
        assert!(capabilities.deterministic);
        assert!(capabilities.streaming);
        assert_eq!(
            capabilities.lookback,
            LookbackRequirement::PeriodMinusOne
        );
        assert!(capabilities.effect.is_pure());
    }

    #[test]
    fn factor_plan_compiles_dependencies_and_executes_existing_engine() {
        let mut registry = FactorRegistry::new();
        registry
            .register(FactorDefinition::new(
                "base",
                ["close"],
                FactorKind::TimeSeries,
                FactorDirection::HigherBetter,
                Arc::new(|inputs| Ok(inputs.get("close")?.to_vec())),
            ))
            .unwrap();
        registry
            .register(FactorDefinition::new(
                "score",
                ["base", "volume"],
                FactorKind::TimeSeries,
                FactorDirection::HigherBetter,
                Arc::new(|inputs| {
                    let base = inputs.get("base")?;
                    let volume = inputs.get("volume")?;
                    Ok(base
                        .iter()
                        .zip(volume)
                        .map(|(left, right)| left + right)
                        .collect())
                }),
            ))
            .unwrap();

        let plan = FactorPlan::compile(&registry, &["score"]).unwrap();
        assert_eq!(plan.execution_order(), &["base", "score"]);
        assert_eq!(plan.required_raw_inputs(), &["close", "volume"]);

        let context = FactorContext::new()
            .with_series("close", vec![1.0, 2.0])
            .unwrap()
            .with_series("volume", vec![10.0, 20.0])
            .unwrap();
        let engine = FactorEngine::new(registry);
        let values = plan.execute(&engine, &context).unwrap();
        assert_eq!(values["score"], vec![11.0, 22.0]);
    }

    #[test]
    fn compute_input_enforces_runtime_nan_policy() {
        let frame = MarketFrame::new(
            &[1.0],
            &[2.0],
            &[0.5],
            &[f64::NAN],
            &[10.0],
        )
        .unwrap();
        let result = ComputeInput::new(
            frame,
            ExecutionPolicy {
                nan: NanPolicy::Error,
                warmup: WarmupPolicy::Nan,
            },
        );
        assert!(matches!(result, Err(RuntimeError::NonFinite { field: "close", .. })));
    }
}
