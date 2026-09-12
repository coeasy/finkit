//! Context-aware entry points for the canonical research executor.

use crate::context::ResearchContext;
use crate::error::ResearchResult;
use crate::executor::{ResearchExecutionRequest, ResearchExecutionResult, ResearchExecutor};
use crate::orchestration::ResearchPlan;
use crate::report::AnalysisMode;

impl ResearchExecutor {
    /// Execute the standard profile from a normalized revision-aware context.
    pub fn execute_standard_context(
        context: &ResearchContext<'_>,
        factor_column: &str,
        price_column: &str,
        mode: AnalysisMode,
    ) -> ResearchResult<ResearchExecutionResult> {
        let plan = ResearchPlan::standard_factor_study()?;
        Self::execute_context(&plan, context, factor_column, price_column, mode)
    }

    /// Execute an explicit plan from the same normalized context used by
    /// incremental sessions. This is the bridge that prevents batch and
    /// incremental APIs from growing separate orchestration paths.
    pub fn execute_context(
        plan: &ResearchPlan,
        context: &ResearchContext<'_>,
        factor_column: &str,
        price_column: &str,
        mode: AnalysisMode,
    ) -> ResearchResult<ResearchExecutionResult> {
        let policy = context.policy();
        Self::execute(
            plan,
            ResearchExecutionRequest {
                frame: context.frame(),
                factor_column,
                price_column,
                periods: &policy.periods,
                quantize: &policy.quantize,
                weights: &policy.weights,
                evaluation: policy.evaluation,
                execution_lag: policy.execution_lag,
                mode,
                provenance: context.provenance(),
                data_revision: context.revision(),
            },
        )
    }
}
