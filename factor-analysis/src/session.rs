//! Stateful research session that reuses the same plan/executor for batch and
//! revision-aware incremental calls.

use crate::artifacts::DirtyRange;
use crate::context::ResearchContext;
use crate::error::ResearchResult;
use crate::executor::{ResearchExecutionResult, ResearchExecutor};
use crate::orchestration::ResearchPlan;
use crate::report::AnalysisMode;

/// Executor decision for the current revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchReuseDecision {
    /// Data and semantics are unchanged; reuse the previous materialization.
    ReuseAll,
    /// At least one row or semantic input changed. Correctness-first fallback
    /// executes the same plan from scratch until every stage advertises a safe
    /// append/range implementation.
    FullRecompute { dirty: DirtyRange },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SessionIdentity {
    data_fingerprint: u64,
    factor_fingerprint: u64,
    universe_fingerprint: u64,
    calendar_fingerprint: u64,
    config_fingerprint: u64,
    plan_fingerprint: u64,
}

impl SessionIdentity {
    fn from_context(plan: &ResearchPlan, context: &ResearchContext<'_>) -> Self {
        let provenance = context.provenance();
        Self {
            data_fingerprint: provenance.data_fingerprint,
            factor_fingerprint: provenance.factor_fingerprint,
            universe_fingerprint: provenance.universe_fingerprint,
            calendar_fingerprint: provenance.calendar_fingerprint,
            config_fingerprint: provenance.config_fingerprint,
            plan_fingerprint: plan.fingerprint(),
        }
    }
}

/// Reusable plan-bound research session.
///
/// The session never owns a second implementation of any research stage. It
/// decides whether a previous result is reusable and otherwise dispatches to
/// `ResearchExecutor::execute_context` using the same semantic `ResearchPlan`.
#[derive(Debug, Clone)]
pub struct ResearchSession {
    plan: ResearchPlan,
    last_identity: Option<SessionIdentity>,
    last_result: Option<ResearchExecutionResult>,
}

impl ResearchSession {
    pub fn new(plan: ResearchPlan) -> Self {
        Self {
            plan,
            last_identity: None,
            last_result: None,
        }
    }

    pub fn standard() -> ResearchResult<Self> {
        Ok(Self::new(ResearchPlan::standard_factor_study()?))
    }

    #[must_use]
    pub fn plan(&self) -> &ResearchPlan {
        &self.plan
    }

    /// Execute a revision through the authoritative executor and return the
    /// reuse decision. Empty dirty ranges can reuse the previous result only
    /// when all semantic/data fingerprints still match.
    pub fn execute(
        &mut self,
        context: &ResearchContext<'_>,
        factor_column: &str,
        price_column: &str,
        mode: AnalysisMode,
    ) -> ResearchResult<(ResearchExecutionResult, ResearchReuseDecision)> {
        let identity = SessionIdentity::from_context(&self.plan, context);
        if context.dirty_range().is_empty()
            && self.last_identity == Some(identity)
            && self.last_result.is_some()
        {
            return Ok((
                self.last_result.as_ref().expect("checked above").clone(),
                ResearchReuseDecision::ReuseAll,
            ));
        }

        let result = ResearchExecutor::execute_context(
            &self.plan,
            context,
            factor_column,
            price_column,
            mode,
        )?;
        self.last_identity = Some(identity);
        self.last_result = Some(result.clone());
        Ok((
            result,
            ResearchReuseDecision::FullRecompute {
                dirty: context.dirty_range(),
            },
        ))
    }

    /// Clear all retained materializations without changing the compiled plan.
    pub fn clear(&mut self) {
        self.last_identity = None;
        self.last_result = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{AssetId, PanelIndex, ResearchFrame};
    use crate::orchestration::StudyProvenance;
    use crate::policy::ResearchPolicy;

    fn frame() -> ResearchFrame {
        let index = PanelIndex::new(
            vec![1, 1, 2, 2, 3, 3],
            vec![
                AssetId(1),
                AssetId(2),
                AssetId(1),
                AssetId(2),
                AssetId(1),
                AssetId(2),
            ],
        )
        .unwrap();
        let mut frame = ResearchFrame::new(index);
        frame
            .add_numeric("factor", "factor", vec![1.0, 2.0, 1.0, 2.0, 1.0, 2.0])
            .unwrap();
        frame
            .add_numeric("price", "market", vec![10.0, 10.0, 11.0, 12.0, 12.0, 14.0])
            .unwrap();
        frame
    }

    #[test]
    fn unchanged_revision_reuses_same_plan_materialization() {
        let frame = frame();
        let policy = ResearchPolicy::standard(vec![1]).unwrap();
        let mut provenance = StudyProvenance::default();
        provenance.data_fingerprint = 42;
        provenance.factor_fingerprint = 7;
        let context = ResearchContext::batch(&frame, policy, provenance).unwrap();
        let mut session = ResearchSession::standard().unwrap();

        let (_, first) = session
            .execute(&context, "factor", "price", AnalysisMode::Native)
            .unwrap();
        assert!(matches!(first, ResearchReuseDecision::FullRecompute { .. }));

        let unchanged = context
            .next_revision(
                &frame,
                DirtyRange::new(frame.index().len(), frame.index().len()),
            )
            .unwrap();
        let (_, second) = session
            .execute(&unchanged, "factor", "price", AnalysisMode::Native)
            .unwrap();
        assert_eq!(second, ResearchReuseDecision::ReuseAll);
    }
}
