use crate::error::{ResearchError, ResearchResult};
use finkit::compute::{
    ComputeCapabilities, ComputeEffect, ComputePlan, DependencyShape, LookbackRequirement,
};
use finkit::semantic_graph::{NodeKind, SemanticGraphBuilder, SemanticNodeId};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Semantic research stages. Dependency ordering is delegated to core `ComputePlan`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ResearchStageKind {
    Align,
    ForwardReturns,
    Groups,
    Clean,
    Quantize,
    Rank,
    Neutralize,
    Returns,
    Information,
    Turnover,
    Event,
    Portfolio,
    Report,
}

/// Incremental execution guarantee implemented by one research stage.
///
/// This is deliberately conservative. A stage is not marked append/range safe
/// until its executor can reuse the authoritative typed materialization without
/// changing batch semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageIncrementalCapability {
    /// The stage currently requires a full recomputation for any dirty input.
    FullOnly,
    /// The stage can safely extend a previous materialization for appended rows.
    AppendSafe,
    /// The stage can safely recompute an arbitrary dirty range.
    RangeSafe,
}

impl StageIncrementalCapability {
    #[must_use]
    pub const fn supports_append(self) -> bool {
        matches!(self, Self::AppendSafe | Self::RangeSafe)
    }

    #[must_use]
    pub const fn supports_range(self) -> bool {
        matches!(self, Self::RangeSafe)
    }
}

impl ResearchStageKind {
    /// Authoritative incremental capability for this semantic stage.
    ///
    /// `Align` is a stateless orchestration boundary and therefore cannot
    /// invalidate incremental execution by itself. `ForwardReturns` is backed
    /// by a dedicated append-aware engine. Other stages remain correctness-first
    /// `FullOnly` until their typed materializations are consumed directly by
    /// the common executor.
    #[must_use]
    pub const fn incremental_capability(self) -> StageIncrementalCapability {
        match self {
            Self::Align => StageIncrementalCapability::RangeSafe,
            Self::ForwardReturns => StageIncrementalCapability::AppendSafe,
            _ => StageIncrementalCapability::FullOnly,
        }
    }
}

/// Stable built-in orchestration profiles. Custom callers can still compile an
/// explicit `ResearchPlan` from stage specifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResearchProfile {
    CoreStudy,
    FullStudy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchStageSpec {
    pub id: usize,
    pub kind: ResearchStageKind,
    pub dependencies: Vec<usize>,
}

impl ResearchStageSpec {
    #[must_use]
    pub const fn incremental_capability(&self) -> StageIncrementalCapability {
        self.kind.incremental_capability()
    }
}

/// Research graph backed by the existing canonical compute planner.
#[derive(Debug, Clone)]
pub struct ResearchPlan {
    stages: Vec<ResearchStageSpec>,
    plan: ComputePlan,
    /// Stage id for each plan node, indexed by plan node id.
    ///
    /// The semantic-graph builder assigns node ids in push order; this mapping
    /// keeps the public API speaking stage ids even if a caller ever declares
    /// stages with sparse or non-sequential ids.
    node_stage: Vec<usize>,
}

impl ResearchPlan {
    pub fn compile(stages: Vec<ResearchStageSpec>) -> ResearchResult<Self> {
        // §16/§24a: the factor frontend lowers through the one semantic graph
        // model, not by assembling `ComputeNode`s directly. The builder assigns
        // ids in push order and delegates validation to `ComputePlan::compile`,
        // so the plan produced here is identical to the old direct assembly —
        // while the graph-level services (content hash, CSE, scheduling levels)
        // become available to the factor side without a second lowering path.
        //
        // The builder requires operands to exist before their users, while the
        // old direct assembly accepted any declaration order and let the plan
        // validator do the work. Stages are therefore pushed in dependency
        // order (stable: declaration order breaks ties), and a dependency that
        // can never be satisfied is the same `InvalidConfig` the plan validator
        // would have raised for an unknown operand or a cycle.
        let mut unique_ids: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
        for stage in &stages {
            if !unique_ids.insert(stage.id) {
                return Err(ResearchError::InvalidConfig(format!(
                    "duplicate research stage id {}",
                    stage.id
                )));
            }
        }
        let push_order: Vec<usize> = {
            let mut resolved: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
            let mut order = Vec::with_capacity(stages.len());
            let mut pending: Vec<usize> = (0..stages.len()).collect();
            while !pending.is_empty() {
                let mut progressed = false;
                pending.retain(|&index| {
                    let stage = &stages[index];
                    if stage.dependencies.iter().all(|dep| resolved.contains(dep)) {
                        resolved.insert(stage.id);
                        order.push(index);
                        progressed = true;
                        false
                    } else {
                        true
                    }
                });
                if !progressed {
                    let unresolved: Vec<String> = pending
                        .iter()
                        .map(|&index| {
                            format!(
                                "stage {} depends on {}",
                                stages[index].id,
                                stages[index]
                                    .dependencies
                                    .iter()
                                    .filter(|dep| !resolved.contains(dep))
                                    .map(|dep| dep.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        })
                        .collect();
                    return Err(ResearchError::InvalidConfig(format!(
                        "research stages have unresolvable dependencies: {}",
                        unresolved.join("; ")
                    )));
                }
            }
            order
        };
        let mut builder = SemanticGraphBuilder::new();
        let mut pushed: std::collections::BTreeMap<usize, SemanticNodeId> =
            std::collections::BTreeMap::new();
        for index in push_order {
            let stage = &stages[index];
            let inputs = stage
                .dependencies
                .iter()
                .copied()
                .map(|dependency| {
                    pushed.get(&dependency).copied().ok_or_else(|| {
                        ResearchError::InvalidConfig(format!(
                            "research stage {} depends on stage {dependency}, \
                             which is not declared before it",
                            stage.id
                        ))
                    })
                })
                .collect::<ResearchResult<Vec<_>>>()?;
            let capabilities = ComputeCapabilities {
                deterministic: true,
                streaming: stage.kind.incremental_capability().supports_append(),
                stateful: false,
                lookback: LookbackRequirement::Dynamic,
                dependency: DependencyShape::Dynamic,
                effect: if matches!(stage.kind, ResearchStageKind::Report) {
                    ComputeEffect::EmitOutput("factor-study-report".to_string())
                } else {
                    ComputeEffect::Pure
                },
            };
            let id = builder.push(
                NodeKind::Factor,
                format!("research::{:?}", stage.kind),
                inputs
                    .into_iter()
                    .map(|input| SemanticNodeId(input.0))
                    .collect(),
                capabilities,
            );
            if matches!(stage.kind, ResearchStageKind::Report) {
                builder.target(id);
            }
            pushed.insert(stage.id, id);
        }
        let graph = builder
            .build()
            .map_err(|error| ResearchError::InvalidConfig(error.to_string()))?;
        let plan = graph
            .lower()
            .map_err(|error| ResearchError::InvalidConfig(error.to_string()))?;
        let mut node_stage = vec![0usize; stages.len()];
        for stage in &stages {
            if let Some(node) = pushed.get(&stage.id) {
                node_stage[node.0] = stage.id;
            }
        }
        Ok(Self {
            stages,
            plan,
            node_stage,
        })
    }

    /// Compile the built-in core/full factor-study profile.
    ///
    /// `FullStudy` currently shares the same mandatory stages as `CoreStudy`;
    /// optional advanced services are added through explicit custom plans until
    /// each service has a typed executor artifact contract.
    pub fn for_profile(profile: ResearchProfile) -> ResearchResult<Self> {
        match profile {
            ResearchProfile::CoreStudy | ResearchProfile::FullStudy => {
                Self::standard_factor_study()
            }
        }
    }

    /// Canonical plan behind the compatibility `FactorStudy::full_report` API.
    pub fn standard_factor_study() -> ResearchResult<Self> {
        Self::compile(vec![
            ResearchStageSpec {
                id: 0,
                kind: ResearchStageKind::Align,
                dependencies: vec![],
            },
            ResearchStageSpec {
                id: 1,
                kind: ResearchStageKind::ForwardReturns,
                dependencies: vec![0],
            },
            ResearchStageSpec {
                id: 2,
                kind: ResearchStageKind::Clean,
                dependencies: vec![0],
            },
            ResearchStageSpec {
                id: 3,
                kind: ResearchStageKind::Quantize,
                dependencies: vec![0, 2],
            },
            ResearchStageSpec {
                id: 4,
                kind: ResearchStageKind::Returns,
                dependencies: vec![1, 3],
            },
            ResearchStageSpec {
                id: 5,
                kind: ResearchStageKind::Information,
                dependencies: vec![1],
            },
            ResearchStageSpec {
                id: 6,
                kind: ResearchStageKind::Turnover,
                dependencies: vec![3],
            },
            ResearchStageSpec {
                id: 7,
                kind: ResearchStageKind::Portfolio,
                dependencies: vec![1, 4],
            },
            ResearchStageSpec {
                id: 8,
                kind: ResearchStageKind::Report,
                dependencies: vec![2, 3, 4, 5, 6, 7],
            },
        ])
    }

    pub fn execution_order(&self) -> Vec<usize> {
        self.plan
            .execution_order()
            .iter()
            .map(|id| self.node_stage[id.0])
            .collect()
    }

    pub fn stages(&self) -> &[ResearchStageSpec] {
        &self.stages
    }

    pub fn stage(&self, id: usize) -> Option<&ResearchStageSpec> {
        self.stages.iter().find(|stage| stage.id == id)
    }

    /// First stage in execution order that prevents append-only incremental
    /// execution. `None` means every stage has an append-safe implementation.
    #[must_use]
    pub fn first_append_blocker(&self) -> Option<&ResearchStageSpec> {
        self.plan.execution_order().iter().find_map(|id| {
            let stage = self.stage(self.node_stage[id.0])?;
            (!stage.incremental_capability().supports_append()).then_some(stage)
        })
    }

    /// First stage in execution order that prevents arbitrary dirty-range
    /// incremental execution.
    #[must_use]
    pub fn first_range_blocker(&self) -> Option<&ResearchStageSpec> {
        self.plan.execution_order().iter().find_map(|id| {
            let stage = self.stage(self.node_stage[id.0])?;
            (!stage.incremental_capability().supports_range()).then_some(stage)
        })
    }

    #[must_use]
    pub fn supports_append_incremental(&self) -> bool {
        self.first_append_blocker().is_none()
    }

    #[must_use]
    pub fn supports_range_incremental(&self) -> bool {
        self.first_range_blocker().is_none()
    }

    /// Deterministic semantic identity used by materialization keys.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for stage in &self.stages {
            stage.id.hash(&mut hasher);
            stage.kind.hash(&mut hasher);
            stage.dependencies.hash(&mut hasher);
        }
        hasher.finish()
    }
}

/// Reproducibility metadata persisted with reports.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudyProvenance {
    pub library_version: String,
    pub data_fingerprint: u64,
    pub factor_fingerprint: u64,
    pub universe_fingerprint: u64,
    pub calendar_fingerprint: u64,
    pub config_fingerprint: u64,
    pub random_seed: Option<u64>,
}

/// Stable hash helper for plan/data metadata. Not intended as a cryptographic digest.
pub fn fingerprint(parts: &[&str]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for part in parts {
        part.hash(&mut hasher);
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_plan_reuses_compute_plan_ordering() {
        let plan = ResearchPlan::compile(vec![
            ResearchStageSpec {
                id: 2,
                kind: ResearchStageKind::Information,
                dependencies: vec![1],
            },
            ResearchStageSpec {
                id: 1,
                kind: ResearchStageKind::ForwardReturns,
                dependencies: vec![],
            },
        ])
        .unwrap();
        assert_eq!(plan.execution_order(), vec![1, 2]);
    }

    #[test]
    fn standard_factor_study_is_a_single_dependency_order() {
        let plan = ResearchPlan::standard_factor_study().unwrap();
        assert_eq!(plan.execution_order(), (0..=8).collect::<Vec<_>>());
        assert_eq!(plan.stage(8).unwrap().kind, ResearchStageKind::Report);
        assert_ne!(plan.fingerprint(), 0);
    }

    #[test]
    fn incremental_capability_is_conservative_and_explicit() {
        assert_eq!(
            ResearchStageKind::Align.incremental_capability(),
            StageIncrementalCapability::RangeSafe
        );
        assert_eq!(
            ResearchStageKind::ForwardReturns.incremental_capability(),
            StageIncrementalCapability::AppendSafe
        );
        assert_eq!(
            ResearchStageKind::Report.incremental_capability(),
            StageIncrementalCapability::FullOnly
        );
        assert!(ResearchStageKind::Align
            .incremental_capability()
            .supports_range());
        assert!(ResearchStageKind::ForwardReturns
            .incremental_capability()
            .supports_append());
        assert!(!ResearchStageKind::ForwardReturns
            .incremental_capability()
            .supports_range());
    }

    #[test]
    fn standard_plan_reports_the_first_incremental_blocker() {
        let plan = ResearchPlan::standard_factor_study().unwrap();
        let append_blocker = plan.first_append_blocker().unwrap();
        let range_blocker = plan.first_range_blocker().unwrap();
        assert_eq!(append_blocker.id, 2);
        assert_eq!(append_blocker.kind, ResearchStageKind::Clean);
        assert_eq!(range_blocker.id, 1);
        assert_eq!(range_blocker.kind, ResearchStageKind::ForwardReturns);
        assert!(!plan.supports_append_incremental());
        assert!(!plan.supports_range_incremental());
    }
}
