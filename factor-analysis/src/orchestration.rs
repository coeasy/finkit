use crate::error::{ResearchError, ResearchResult};
use finkit::compute::{ComputeCapabilities, ComputeEffect, ComputeNode, ComputeNodeId, ComputePlan, LookbackRequirement};
use serde::{Deserialize, Serialize};

/// Semantic research stages. Dependency ordering is delegated to core `ComputePlan`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchStageSpec {
    pub id: usize,
    pub kind: ResearchStageKind,
    pub dependencies: Vec<usize>,
}

/// Research graph backed by the existing canonical compute planner.
#[derive(Debug, Clone)]
pub struct ResearchPlan {
    stages: Vec<ResearchStageSpec>,
    plan: ComputePlan,
}

impl ResearchPlan {
    pub fn compile(stages: Vec<ResearchStageSpec>) -> ResearchResult<Self> {
        let nodes = stages.iter().map(|stage| {
            ComputeNode::new(
                ComputeNodeId(stage.id),
                format!("research::{:?}", stage.kind),
                stage.dependencies.iter().copied().map(ComputeNodeId).collect(),
                ComputeCapabilities {
                    deterministic: true,
                    streaming: matches!(stage.kind, ResearchStageKind::Align | ResearchStageKind::ForwardReturns | ResearchStageKind::Rank | ResearchStageKind::Returns | ResearchStageKind::Information | ResearchStageKind::Turnover),
                    stateful: false,
                    lookback: LookbackRequirement::Dynamic,
                    effect: if matches!(stage.kind, ResearchStageKind::Report) { ComputeEffect::EmitOutput("factor-study-report".to_string()) } else { ComputeEffect::Pure },
                },
            )
        });
        let plan = ComputePlan::compile(nodes).map_err(|error| ResearchError::InvalidConfig(error.to_string()))?;
        Ok(Self { stages, plan })
    }

    pub fn execution_order(&self) -> Vec<usize> {
        self.plan.execution_order().iter().map(|id| id.0).collect()
    }

    pub fn stages(&self) -> &[ResearchStageSpec] { &self.stages }
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
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for part in parts { part.hash(&mut hasher); }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn research_plan_reuses_compute_plan_ordering() {
        let plan = ResearchPlan::compile(vec![
            ResearchStageSpec { id: 2, kind: ResearchStageKind::Information, dependencies: vec![1] },
            ResearchStageSpec { id: 1, kind: ResearchStageKind::ForwardReturns, dependencies: vec![] },
        ]).unwrap();
        assert_eq!(plan.execution_order(), vec![1, 2]);
    }
}
