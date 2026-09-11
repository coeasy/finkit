//! Revision-aware research execution context.

use crate::artifacts::{DataDelta, DirtyRange};
use crate::data::ResearchFrame;
use crate::error::ResearchResult;
use crate::orchestration::StudyProvenance;
use crate::policy::ResearchPolicy;

/// Normalized data, policy, provenance and revision identity consumed by the
/// authoritative research executor.
#[derive(Debug, Clone)]
pub struct ResearchContext<'a> {
    frame: &'a ResearchFrame,
    policy: ResearchPolicy,
    provenance: StudyProvenance,
    delta: DataDelta,
}

impl<'a> ResearchContext<'a> {
    /// Create a validated context for a full/batch materialization.
    pub fn batch(
        frame: &'a ResearchFrame,
        policy: ResearchPolicy,
        provenance: StudyProvenance,
    ) -> ResearchResult<Self> {
        Self::new(
            frame,
            policy,
            provenance,
            DataDelta {
                revision: 0,
                dirty: DirtyRange::full(frame.index().len()),
            },
        )
    }

    /// Create a validated context carrying an explicit revision and dirty range.
    pub fn new(
        frame: &'a ResearchFrame,
        policy: ResearchPolicy,
        mut provenance: StudyProvenance,
        delta: DataDelta,
    ) -> ResearchResult<Self> {
        policy.validate()?;
        provenance.config_fingerprint = policy.fingerprint();
        Ok(Self {
            frame,
            policy,
            provenance,
            delta,
        })
    }

    #[must_use]
    pub fn frame(&self) -> &'a ResearchFrame {
        self.frame
    }

    #[must_use]
    pub fn policy(&self) -> &ResearchPolicy {
        &self.policy
    }

    #[must_use]
    pub fn provenance(&self) -> &StudyProvenance {
        &self.provenance
    }

    #[must_use]
    pub fn revision(&self) -> u64 {
        self.delta.revision
    }

    #[must_use]
    pub fn dirty_range(&self) -> DirtyRange {
        self.delta.dirty
    }

    #[must_use]
    pub fn delta(&self) -> DataDelta {
        self.delta
    }

    /// Build the next context after a data mutation without losing normalized
    /// policy or provenance identity.
    pub fn next_revision(&self, frame: &'a ResearchFrame, dirty: DirtyRange) -> ResearchResult<Self> {
        Self::new(
            frame,
            self.policy.clone(),
            self.provenance.clone(),
            DataDelta {
                revision: self.delta.revision.saturating_add(1),
                dirty,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{AssetId, PanelIndex};

    #[test]
    fn context_carries_policy_identity_across_revisions() {
        let frame = ResearchFrame::new(PanelIndex::new(vec![1], vec![AssetId(1)]).unwrap());
        let policy = ResearchPolicy::standard(vec![1]).unwrap();
        let context = ResearchContext::batch(&frame, policy.clone(), StudyProvenance::default())
            .unwrap();
        assert_eq!(context.provenance().config_fingerprint, policy.fingerprint());
        assert_eq!(context.revision(), 0);
        assert_eq!(context.dirty_range(), DirtyRange::full(1));

        let next = context
            .next_revision(&frame, DirtyRange::new(1, 1))
            .unwrap();
        assert_eq!(next.revision(), 1);
        assert!(next.dirty_range().is_empty());
        assert_eq!(
            next.provenance().config_fingerprint,
            context.provenance().config_fingerprint
        );
    }
}
