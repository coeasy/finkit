//! Authoritative plan-based factor research executor.
//!
//! `FactorStudy` remains a compatibility facade; stage sequencing lives here
//! so batch, incremental materialization and future custom profiles share one
//! orchestration source of truth.

use crate::analysis::{
    factor_alpha_beta, factor_returns, factor_weights, information_coefficient,
    mean_information_coefficient, mean_return_by_quantile, quantile_turnover, rank_autocorrelation,
    AlphaBeta, WeightConfig,
};
use crate::artifacts::{MaterializationKey, ResearchArtifact, ResearchArtifactStore};
use crate::data::ResearchFrame;
use crate::error::{ResearchError, ResearchResult};
use crate::factor_metrics::{
    quantile_diagnostics, summarize_ic, IcStatistics, QuantileDiagnostics,
};
use crate::orchestration::{ResearchPlan, ResearchStageKind, StudyProvenance};
use crate::performance::EvaluationConfig;
use crate::portfolio_performance::{
    evaluate_factor_holding_periods, FactorPortfolioPerformanceReport,
};
use crate::prepare::{
    compute_forward_returns, data_quality, quantize_factor, DataQualityReport, ForwardReturnConfig,
    QuantizeConfig,
};
use crate::report::{
    AnalysisMode, FactorStudyReport, InformationReport, ReturnsReport, TurnoverReport,
};
use finkit::returns::ReturnKind;
use std::collections::BTreeMap;

/// Borrowed, normalized inputs to one research execution.
#[derive(Debug, Clone, Copy)]
pub struct ResearchExecutionRequest<'a> {
    pub frame: &'a ResearchFrame,
    pub factor_column: &'a str,
    pub price_column: &'a str,
    pub periods: &'a [usize],
    pub quantize: &'a QuantizeConfig,
    pub weights: &'a WeightConfig,
    pub evaluation: EvaluationConfig,
    pub execution_lag: usize,
    pub mode: AnalysisMode,
    pub provenance: &'a StudyProvenance,
    /// Batch executions use revision zero. Incremental callers should advance
    /// this identity whenever their source frame changes.
    pub data_revision: u64,
}

/// Executed-stage evidence emitted alongside the report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchExecutionTrace {
    pub plan_fingerprint: u64,
    pub executed_stages: Vec<ResearchStageKind>,
}

/// Complete result of a plan-based research execution.
#[derive(Debug, Clone)]
pub struct ResearchExecutionResult {
    pub report: FactorStudyReport,
    pub artifacts: ResearchArtifactStore,
    pub trace: ResearchExecutionTrace,
}

/// Canonical executor for semantic `ResearchPlan` nodes.
#[derive(Debug, Clone, Copy, Default)]
pub struct ResearchExecutor;

impl ResearchExecutor {
    /// Execute the standard factor-study profile.
    pub fn execute_standard(
        request: ResearchExecutionRequest<'_>,
    ) -> ResearchResult<ResearchExecutionResult> {
        let plan = ResearchPlan::standard_factor_study()?;
        Self::execute(&plan, request)
    }

    /// Execute an explicit research plan.
    ///
    /// A report-producing plan must include all stages needed by the report.
    /// Advanced custom plans can add semantic stages as executor bindings are
    /// introduced without creating another orchestration path.
    pub fn execute(
        plan: &ResearchPlan,
        request: ResearchExecutionRequest<'_>,
    ) -> ResearchResult<ResearchExecutionResult> {
        validate_request(&request)?;

        let plan_fingerprint = plan.fingerprint();
        let mut artifacts = ResearchArtifactStore::new();
        let mut executed_stages = Vec::with_capacity(plan.stages().len());

        let mut quality: Option<DataQualityReport> = None;
        let mut forward: Option<BTreeMap<usize, Vec<f64>>> = None;
        let mut daily_asset_returns: Option<Vec<f64>> = None;
        let mut quantiles: Option<Vec<u16>> = None;
        let mut weights: Option<Vec<f64>> = None;
        let mut factor_ret: Option<BTreeMap<usize, Vec<f64>>> = None;
        let mut alpha_beta: Option<BTreeMap<usize, AlphaBeta>> = None;
        let mut quantile_returns: Option<BTreeMap<u16, BTreeMap<usize, f64>>> = None;
        let mut quantile_diag: Option<QuantileDiagnostics> = None;
        let mut ic: Option<BTreeMap<usize, Vec<f64>>> = None;
        let mut mean_ic: Option<BTreeMap<usize, f64>> = None;
        let mut ic_statistics: Option<BTreeMap<usize, IcStatistics>> = None;
        let mut bottom_turnover: Option<Vec<f64>> = None;
        let mut top_turnover: Option<Vec<f64>> = None;
        let mut rank_auto: Option<Vec<f64>> = None;
        let mut performance: Option<FactorPortfolioPerformanceReport> = None;
        let mut final_report: Option<FactorStudyReport> = None;

        for stage_id in plan.execution_order() {
            let stage = plan.stage(stage_id).ok_or_else(|| {
                ResearchError::InvalidConfig(format!(
                    "research plan execution order references unknown stage {stage_id}"
                ))
            })?;
            executed_stages.push(stage.kind);

            match stage.kind {
                ResearchStageKind::Align => {}
                ResearchStageKind::Clean => {
                    quality = Some(data_quality(request.frame, request.factor_column)?);
                }
                ResearchStageKind::ForwardReturns => {
                    let mut calculation_periods = request.periods.to_vec();
                    if !calculation_periods.contains(&1) {
                        calculation_periods.push(1);
                    }
                    calculation_periods.sort_unstable();
                    calculation_periods.dedup();
                    let config =
                        ForwardReturnConfig::new(calculation_periods, ReturnKind::Arithmetic)?;
                    let mut values =
                        compute_forward_returns(request.frame, request.price_column, &config)?;
                    daily_asset_returns = values.get(&1).cloned();
                    if !request.periods.contains(&1) {
                        values.remove(&1);
                    }
                    artifacts.insert(
                        artifact_key(&request, plan_fingerprint, stage_id, "forward_returns"),
                        ResearchArtifact::HorizonSeries(values.clone()),
                    );
                    forward = Some(values);
                }
                ResearchStageKind::Quantize => {
                    let values =
                        quantize_factor(request.frame, request.factor_column, request.quantize)?;
                    artifacts.insert(
                        artifact_key(&request, plan_fingerprint, stage_id, "quantiles"),
                        ResearchArtifact::Quantiles(values.clone()),
                    );
                    quantiles = Some(values);
                }
                ResearchStageKind::Returns => {
                    let forward_ref = required(&forward, "ForwardReturns", stage.kind)?;
                    let quantile_ref = required(&quantiles, "Quantize", stage.kind)?;
                    let computed_weights =
                        factor_weights(request.frame, request.factor_column, request.weights)?;
                    let returns = factor_returns(request.frame, &computed_weights, forward_ref)?;
                    let alpha = factor_alpha_beta(request.frame, &returns, forward_ref);
                    let by_quantile = mean_return_by_quantile(quantile_ref, forward_ref);
                    let diagnostics = quantile_diagnostics(&by_quantile);
                    artifacts.insert(
                        artifact_key(&request, plan_fingerprint, stage_id, "factor_returns"),
                        ResearchArtifact::HorizonSeries(returns.clone()),
                    );
                    weights = Some(computed_weights);
                    factor_ret = Some(returns);
                    alpha_beta = Some(alpha);
                    quantile_returns = Some(by_quantile);
                    quantile_diag = Some(diagnostics);
                }
                ResearchStageKind::Information => {
                    let forward_ref = required(&forward, "ForwardReturns", stage.kind)?;
                    let values =
                        information_coefficient(request.frame, request.factor_column, forward_ref)?;
                    let means = mean_information_coefficient(&values);
                    let stats = values
                        .iter()
                        .map(|(&period, series)| {
                            (period, summarize_ic(series, period.saturating_sub(1)))
                        })
                        .collect();
                    artifacts.insert(
                        artifact_key(
                            &request,
                            plan_fingerprint,
                            stage_id,
                            "information_coefficient",
                        ),
                        ResearchArtifact::HorizonSeries(values.clone()),
                    );
                    ic = Some(values);
                    mean_ic = Some(means);
                    ic_statistics = Some(stats);
                }
                ResearchStageKind::Turnover => {
                    let quantile_ref = required(&quantiles, "Quantize", stage.kind)?;
                    bottom_turnover = Some(quantile_turnover(request.frame, quantile_ref, 1, 1)?);
                    top_turnover = Some(quantile_turnover(
                        request.frame,
                        quantile_ref,
                        request.quantize.quantiles,
                        1,
                    )?);
                    let autocorrelation =
                        rank_autocorrelation(request.frame, request.factor_column, 1)?;
                    artifacts.insert(
                        artifact_key(&request, plan_fingerprint, stage_id, "rank_autocorrelation"),
                        ResearchArtifact::Series(autocorrelation.clone()),
                    );
                    rank_auto = Some(autocorrelation);
                }
                ResearchStageKind::Portfolio => {
                    let computed_weights = required(&weights, "Returns", stage.kind)?;
                    let daily = required(&daily_asset_returns, "ForwardReturns", stage.kind)?;
                    let value = evaluate_factor_holding_periods(
                        request.frame,
                        computed_weights,
                        daily,
                        request.periods,
                        request.execution_lag,
                        request.evaluation,
                    )?;
                    let cumulative = value
                        .by_holding_period
                        .iter()
                        .map(|(&period, result)| (period, result.gross_cumulative_wealth.clone()))
                        .collect();
                    artifacts.insert(
                        artifact_key(&request, plan_fingerprint, stage_id, "cumulative_returns"),
                        ResearchArtifact::HorizonSeries(cumulative),
                    );
                    performance = Some(value);
                }
                ResearchStageKind::Report => {
                    let performance = required(&performance, "Portfolio", stage.kind)?.clone();
                    let cumulative = performance
                        .by_holding_period
                        .iter()
                        .map(|(&period, result)| (period, result.gross_cumulative_wealth.clone()))
                        .collect();
                    final_report = Some(FactorStudyReport {
                        mode: request.mode,
                        provenance: request.provenance.clone(),
                        data_quality: required(&quality, "Clean", stage.kind)?.clone(),
                        quantiles: request.quantize.quantiles,
                        periods: request.periods.to_vec(),
                        execution_lag: request.execution_lag,
                        returns: ReturnsReport {
                            factor_returns: required(&factor_ret, "Returns", stage.kind)?.clone(),
                            cumulative_returns: cumulative,
                            alpha_beta: required(&alpha_beta, "Returns", stage.kind)?.clone(),
                            mean_return_by_quantile: required(
                                &quantile_returns,
                                "Returns",
                                stage.kind,
                            )?
                            .clone(),
                            quantile_diagnostics: required(&quantile_diag, "Returns", stage.kind)?
                                .clone(),
                        },
                        information: InformationReport {
                            ic_by_horizon: required(&ic, "Information", stage.kind)?.clone(),
                            mean_ic: required(&mean_ic, "Information", stage.kind)?.clone(),
                            statistics: required(&ic_statistics, "Information", stage.kind)?
                                .clone(),
                        },
                        turnover: TurnoverReport {
                            bottom_quantile_turnover: required(
                                &bottom_turnover,
                                "Turnover",
                                stage.kind,
                            )?
                            .clone(),
                            top_quantile_turnover: required(&top_turnover, "Turnover", stage.kind)?
                                .clone(),
                            rank_autocorrelation: required(&rank_auto, "Turnover", stage.kind)?
                                .clone(),
                        },
                        performance,
                    });
                }
                ResearchStageKind::Groups
                | ResearchStageKind::Rank
                | ResearchStageKind::Neutralize
                | ResearchStageKind::Event => {
                    return Err(ResearchError::InvalidConfig(format!(
                        "research stage {:?} has no executor binding in this profile",
                        stage.kind
                    )));
                }
            }
        }

        let report = final_report.ok_or_else(|| {
            ResearchError::InvalidConfig("research plan did not materialize a report".to_string())
        })?;
        Ok(ResearchExecutionResult {
            report,
            artifacts,
            trace: ResearchExecutionTrace {
                plan_fingerprint,
                executed_stages,
            },
        })
    }
}

fn validate_request(request: &ResearchExecutionRequest<'_>) -> ResearchResult<()> {
    if request.periods.is_empty() || request.periods.iter().any(|&period| period == 0) {
        return Err(ResearchError::InvalidConfig(
            "factor study periods must contain at least one positive horizon".to_string(),
        ));
    }
    request.frame.column(request.factor_column)?;
    request.frame.column(request.price_column)?;
    Ok(())
}

fn required<'a, T>(
    value: &'a Option<T>,
    dependency: &str,
    stage: ResearchStageKind,
) -> ResearchResult<&'a T> {
    value.as_ref().ok_or_else(|| {
        ResearchError::InvalidConfig(format!(
            "research stage {stage:?} requires materialized stage {dependency}"
        ))
    })
}

fn artifact_key(
    request: &ResearchExecutionRequest<'_>,
    plan_fingerprint: u64,
    stage_id: usize,
    output: &str,
) -> MaterializationKey {
    MaterializationKey {
        stage_id,
        output: output.to_string(),
        data_revision: request.data_revision,
        data_fingerprint: request.provenance.data_fingerprint,
        plan_fingerprint,
        semantics_fingerprint: request.provenance.factor_fingerprint,
        parameter_fingerprint: request.provenance.config_fingerprint,
        algorithm_version: 1,
        schema_version: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{AssetId, PanelIndex, ResearchFrame};

    fn frame() -> ResearchFrame {
        let index = PanelIndex::new(
            vec![1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4],
            vec![
                AssetId(1),
                AssetId(2),
                AssetId(3),
                AssetId(1),
                AssetId(2),
                AssetId(3),
                AssetId(1),
                AssetId(2),
                AssetId(3),
                AssetId(1),
                AssetId(2),
                AssetId(3),
            ],
        )
        .unwrap();
        let mut frame = ResearchFrame::new(index);
        frame
            .add_numeric(
                "factor",
                "factor",
                vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0, 1.0, 2.0, 3.0, 1.0, 2.0, 3.0],
            )
            .unwrap();
        frame
            .add_numeric(
                "price",
                "market",
                vec![
                    10.0, 10.0, 10.0, 11.0, 12.0, 13.0, 12.0, 14.0, 16.0, 13.0, 17.0, 21.0,
                ],
            )
            .unwrap();
        frame
    }

    #[test]
    fn standard_executor_uses_plan_and_materializes_typed_artifacts() {
        let frame = frame();
        let periods = vec![1, 2];
        let quantize = QuantizeConfig {
            quantiles: 3,
            ..QuantizeConfig::default()
        };
        let weights = WeightConfig::default();
        let provenance = StudyProvenance::default();
        let result = ResearchExecutor::execute_standard(ResearchExecutionRequest {
            frame: &frame,
            factor_column: "factor",
            price_column: "price",
            periods: &periods,
            quantize: &quantize,
            weights: &weights,
            evaluation: EvaluationConfig::default(),
            execution_lag: 0,
            mode: AnalysisMode::Native,
            provenance: &provenance,
            data_revision: 0,
        })
        .unwrap();

        assert_eq!(result.report.periods, periods);
        assert_eq!(
            result.trace.executed_stages.last(),
            Some(&ResearchStageKind::Report)
        );
        assert!(!result.artifacts.is_empty());
    }
}
