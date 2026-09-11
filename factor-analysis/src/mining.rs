use crate::data::ResearchFrame;
use crate::error::ResearchResult;
use finkit::math::information::pairwise_pearson;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Candidate factor metadata used by Formula/Composite/FactorRegistry adapters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactorCandidate {
    pub name: String,
    pub expression: String,
    pub fingerprint: u64,
}

impl FactorCandidate {
    pub fn new(name: impl Into<String>, expression: impl Into<String>) -> Self {
        let name = name.into();
        let expression = expression.into();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        expression.trim().hash(&mut hasher);
        let fingerprint = hasher.finish();
        Self { name, expression, fingerprint }
    }
}

/// Remove expression-identical candidates by stable fingerprint, preserving first occurrence.
pub fn deduplicate_candidates(candidates: &[FactorCandidate]) -> Vec<FactorCandidate> {
    let mut seen = std::collections::BTreeSet::new();
    candidates.iter().filter(|candidate| seen.insert(candidate.fingerprint)).cloned().collect()
}

/// Minimum finite-value coverage for one factor column.
pub fn coverage(frame: &ResearchFrame, factor: &str) -> ResearchResult<f64> {
    let values = frame.column(factor)?;
    Ok(if values.is_empty() { 0.0 } else { values.iter().filter(|v| v.is_finite()).count() as f64 / values.len() as f64 })
}

/// Greedy redundancy filter: preserve the first factor from each highly correlated cluster.
pub fn correlation_screen<'a>(
    frame: &ResearchFrame,
    factors: &'a [&'a str],
    max_abs_correlation: f64,
) -> ResearchResult<Vec<&'a str>> {
    let mut kept: Vec<&'a str> = Vec::new();
    for &candidate in factors {
        let values = frame.column(candidate)?;
        let redundant = kept.iter().any(|existing| {
            frame.column(existing).ok().is_some_and(|other| {
                let corr = pairwise_pearson(values, other);
                corr.is_finite() && corr.abs() > max_abs_correlation
            })
        });
        if !redundant { kept.push(candidate); }
    }
    Ok(kept)
}
