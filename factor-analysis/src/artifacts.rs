//! Typed research materializations shared by batch and incremental execution.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ops::Range;

/// Half-open row range invalidated by a data change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirtyRange {
    pub start: usize,
    pub end: usize,
}

impl DirtyRange {
    /// Build a validated half-open dirty range.
    #[must_use]
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start: start.min(end),
            end: end.max(start),
        }
    }

    /// A dirty range covering all rows.
    #[must_use]
    pub fn full(rows: usize) -> Self {
        Self {
            start: 0,
            end: rows,
        }
    }

    /// Whether no rows are dirty.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Convert to a standard range.
    #[must_use]
    pub fn as_range(self) -> Range<usize> {
        self.start..self.end
    }

    /// Merge two invalidation ranges conservatively.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// Data revision and the rows changed since the previous materialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataDelta {
    pub revision: u64,
    pub dirty: DirtyRange,
}

/// Complete identity of one materialized research output.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MaterializationKey {
    pub stage_id: usize,
    pub output: String,
    pub data_revision: u64,
    pub data_fingerprint: u64,
    pub plan_fingerprint: u64,
    pub semantics_fingerprint: u64,
    pub parameter_fingerprint: u64,
    pub algorithm_version: u32,
    pub schema_version: u32,
}

impl MaterializationKey {
    /// Whether two keys describe the same stage/output contract independently
    /// of the concrete source-data revision.
    ///
    /// A positive result is only a prerequisite for incremental reuse. The
    /// executor must additionally verify the stage's incremental capability and
    /// the current [`DirtyRange`] before carrying a previous artifact forward.
    #[must_use]
    pub fn same_contract(&self, other: &Self) -> bool {
        self.stage_id == other.stage_id
            && self.output == other.output
            && self.plan_fingerprint == other.plan_fingerprint
            && self.semantics_fingerprint == other.semantics_fingerprint
            && self.parameter_fingerprint == other.parameter_fingerprint
            && self.algorithm_version == other.algorithm_version
            && self.schema_version == other.schema_version
    }

    /// Whether the full materialization identity, including data revision and
    /// fingerprint, matches exactly.
    #[must_use]
    pub fn same_revision_identity(&self, other: &Self) -> bool {
        self.same_contract(other)
            && self.data_revision == other.data_revision
            && self.data_fingerprint == other.data_fingerprint
    }
}

/// Typed values produced by research stages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum ResearchArtifact {
    Scalar(f64),
    Series(Vec<f64>),
    Quantiles(Vec<u16>),
    HorizonSeries(BTreeMap<usize, Vec<f64>>),
    Json(serde_json::Value),
}

impl ResearchArtifact {
    #[must_use]
    pub fn as_series(&self) -> Option<&[f64]> {
        match self {
            Self::Series(values) => Some(values),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_quantiles(&self) -> Option<&[u16]> {
        match self {
            Self::Quantiles(values) => Some(values),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_horizon_series(&self) -> Option<&BTreeMap<usize, Vec<f64>>> {
        match self {
            Self::HorizonSeries(values) => Some(values),
            _ => None,
        }
    }
}

/// In-memory materialization store used by the research executor.
///
/// The key includes data, plan, parameter and algorithm/schema identity so a
/// materialization cannot be silently reused under different semantics.
#[derive(Debug, Clone, Default)]
pub struct ResearchArtifactStore {
    values: BTreeMap<MaterializationKey, ResearchArtifact>,
}

impl ResearchArtifactStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn get(&self, key: &MaterializationKey) -> Option<&ResearchArtifact> {
        self.values.get(key)
    }

    /// Find the newest older materialization with the same execution contract.
    ///
    /// This method deliberately ignores only data revision/fingerprint. Callers
    /// must still apply stage capability + dirty-range checks before reuse. It
    /// exists so batch and incremental execution can converge on the same typed
    /// store rather than growing a second revision cache.
    #[must_use]
    pub fn latest_compatible_before(
        &self,
        desired: &MaterializationKey,
    ) -> Option<(&MaterializationKey, &ResearchArtifact)> {
        self.values
            .iter()
            .filter(|(key, _)| {
                key.data_revision < desired.data_revision && key.same_contract(desired)
            })
            .max_by_key(|(key, _)| key.data_revision)
    }

    pub fn insert(&mut self, key: MaterializationKey, artifact: ResearchArtifact) {
        self.values.insert(key, artifact);
    }

    pub fn invalidate_revision(&mut self, revision: u64) {
        self.values.retain(|key, _| key.data_revision != revision);
    }

    pub fn retain_revision(&mut self, revision: u64) {
        self.values.retain(|key, _| key.data_revision == revision);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&MaterializationKey, &ResearchArtifact)> {
        self.values.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(revision: u64) -> MaterializationKey {
        MaterializationKey {
            stage_id: 1,
            output: "series".to_string(),
            data_revision: revision,
            data_fingerprint: revision + 10,
            plan_fingerprint: 3,
            semantics_fingerprint: 4,
            parameter_fingerprint: 5,
            algorithm_version: 1,
            schema_version: 1,
        }
    }

    #[test]
    fn dirty_ranges_merge_conservatively() {
        assert_eq!(
            DirtyRange::new(5, 9).union(DirtyRange::new(2, 6)),
            DirtyRange::new(2, 9)
        );
        assert_eq!(
            DirtyRange::new(3, 3).union(DirtyRange::new(8, 10)),
            DirtyRange::new(8, 10)
        );
    }

    #[test]
    fn artifact_store_is_revision_safe() {
        let mut store = ResearchArtifactStore::new();
        store.insert(key(1), ResearchArtifact::Series(vec![1.0]));
        store.insert(key(2), ResearchArtifact::Series(vec![2.0]));
        store.retain_revision(2);
        assert!(store.get(&key(1)).is_none());
        assert_eq!(
            store.get(&key(2)),
            Some(&ResearchArtifact::Series(vec![2.0]))
        );
    }

    #[test]
    fn contract_compatibility_excludes_revision_but_not_semantics() {
        let previous = key(1);
        let current = key(2);
        assert!(previous.same_contract(&current));
        assert!(!previous.same_revision_identity(&current));

        let mut changed_policy = current.clone();
        changed_policy.parameter_fingerprint += 1;
        assert!(!previous.same_contract(&changed_policy));

        let mut changed_algorithm = current.clone();
        changed_algorithm.algorithm_version += 1;
        assert!(!previous.same_contract(&changed_algorithm));
    }

    #[test]
    fn artifact_store_returns_only_latest_older_compatible_revision() {
        let mut store = ResearchArtifactStore::new();
        store.insert(key(1), ResearchArtifact::Series(vec![1.0]));
        store.insert(key(3), ResearchArtifact::Series(vec![3.0]));

        let desired = key(4);
        let (matched, artifact) = store.latest_compatible_before(&desired).unwrap();
        assert_eq!(matched.data_revision, 3);
        assert_eq!(artifact, &ResearchArtifact::Series(vec![3.0]));

        let mut incompatible = desired;
        incompatible.schema_version += 1;
        assert!(store.latest_compatible_before(&incompatible).is_none());
    }

    #[test]
    fn typed_artifact_accessors_reject_wrong_variants() {
        let series = ResearchArtifact::Series(vec![1.0, 2.0]);
        assert_eq!(series.as_series(), Some([1.0, 2.0].as_slice()));
        assert!(series.as_quantiles().is_none());
        assert!(series.as_horizon_series().is_none());
    }
}
