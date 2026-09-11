//! Normalized research policy shared by batch and incremental execution.
//!
//! Defaults and validation live here so API adapters and compatibility facades
//! do not silently create different research semantics.

use crate::analysis::WeightConfig;
use crate::error::{ResearchError, ResearchResult};
use crate::performance::EvaluationConfig;
use crate::prepare::QuantizeConfig;
use std::hash::{Hash, Hasher};

/// Missing-value handling at the research boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissingValuePolicy {
    /// Preserve non-finite observations and let each canonical service apply its
    /// documented finite-observation rules.
    Preserve,
    /// Reject non-finite factor or price values before execution.
    Error,
}

/// Complete normalized policy for the standard factor-study pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct ResearchPolicy {
    pub periods: Vec<usize>,
    pub quantize: QuantizeConfig,
    pub weights: WeightConfig,
    pub evaluation: EvaluationConfig,
    pub execution_lag: usize,
    pub missing_values: MissingValuePolicy,
}

impl ResearchPolicy {
    /// Normalize horizons and validate all policy values once at the boundary.
    pub fn new(
        mut periods: Vec<usize>,
        quantize: QuantizeConfig,
        weights: WeightConfig,
        evaluation: EvaluationConfig,
        execution_lag: usize,
    ) -> ResearchResult<Self> {
        periods.sort_unstable();
        periods.dedup();
        let policy = Self {
            periods,
            quantize,
            weights,
            evaluation,
            execution_lag,
            missing_values: MissingValuePolicy::Preserve,
        };
        policy.validate()?;
        Ok(policy)
    }

    /// Standard research defaults for the supplied forward-return horizons.
    pub fn standard(periods: Vec<usize>) -> ResearchResult<Self> {
        Self::new(
            periods,
            QuantizeConfig::default(),
            WeightConfig::default(),
            EvaluationConfig::default(),
            0,
        )
    }

    pub fn with_missing_values(mut self, policy: MissingValuePolicy) -> Self {
        self.missing_values = policy;
        self
    }

    pub fn validate(&self) -> ResearchResult<()> {
        if self.periods.is_empty() || self.periods.iter().any(|&period| period == 0) {
            return Err(ResearchError::InvalidConfig(
                "research periods must contain at least one positive horizon".to_string(),
            ));
        }
        if self.quantize.quantiles == 0 {
            return Err(ResearchError::InvalidConfig(
                "research quantiles must be greater than zero".to_string(),
            ));
        }
        if self.evaluation.annualization == 0 {
            return Err(ResearchError::InvalidConfig(
                "research annualization must be greater than zero".to_string(),
            ));
        }
        if !self.evaluation.risk_free_rate.is_finite()
            || !self.evaluation.minimum_acceptable_return.is_finite()
            || !(0.0 < self.evaluation.var_confidence && self.evaluation.var_confidence < 1.0)
            || !self.evaluation.transaction_cost_bps.is_finite()
            || self.evaluation.transaction_cost_bps < 0.0
            || !self.evaluation.slippage_bps.is_finite()
            || self.evaluation.slippage_bps < 0.0
        {
            return Err(ResearchError::InvalidConfig(
                "research evaluation policy contains invalid rates, confidence, or costs"
                    .to_string(),
            ));
        }
        Ok(())
    }

    /// Stable non-cryptographic semantic fingerprint used in materialization
    /// identity. Every field that can change research results participates.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.periods.hash(&mut hasher);
        self.quantize.quantiles.hash(&mut hasher);
        self.quantize.by_group.hash(&mut hasher);
        self.quantize.zero_aware.hash(&mut hasher);
        self.weights.demeaned.hash(&mut hasher);
        self.weights.group_adjust.hash(&mut hasher);
        self.weights.equal_weight.hash(&mut hasher);
        self.evaluation.annualization.hash(&mut hasher);
        self.evaluation.risk_free_rate.to_bits().hash(&mut hasher);
        self.evaluation
            .minimum_acceptable_return
            .to_bits()
            .hash(&mut hasher);
        self.evaluation.var_confidence.to_bits().hash(&mut hasher);
        self.evaluation
            .transaction_cost_bps
            .to_bits()
            .hash(&mut hasher);
        self.evaluation.slippage_bps.to_bits().hash(&mut hasher);
        self.execution_lag.hash(&mut hasher);
        self.missing_values.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_normalizes_horizons_and_fingerprints_semantics() {
        let first = ResearchPolicy::standard(vec![5, 1, 5]).unwrap();
        let second = ResearchPolicy::standard(vec![1, 5]).unwrap();
        assert_eq!(first.periods, vec![1, 5]);
        assert_eq!(first.fingerprint(), second.fingerprint());

        let mut changed = second.clone();
        changed.execution_lag = 1;
        assert_ne!(first.fingerprint(), changed.fingerprint());
    }

    #[test]
    fn policy_rejects_invalid_evaluation_before_execution() {
        let mut evaluation = EvaluationConfig::default();
        evaluation.var_confidence = 1.0;
        assert!(ResearchPolicy::new(
            vec![1],
            QuantizeConfig::default(),
            WeightConfig::default(),
            evaluation,
            0,
        )
        .is_err());
    }
}
