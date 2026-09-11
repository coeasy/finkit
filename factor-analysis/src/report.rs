use crate::analysis::{
    factor_alpha_beta, factor_returns, factor_weights, information_coefficient,
    mean_information_coefficient, mean_return_by_quantile, quantile_turnover, rank_autocorrelation,
    AlphaBeta, WeightConfig,
};
use crate::data::ResearchFrame;
use crate::error::ResearchResult;
use crate::factor_metrics::{
    quantile_diagnostics, summarize_ic, IcStatistics, QuantileDiagnostics,
};
use crate::orchestration::StudyProvenance;
use crate::performance::EvaluationConfig;
use crate::portfolio_performance::{
    evaluate_factor_holding_periods, FactorPortfolioPerformanceReport,
};
use crate::prepare::{
    compute_forward_returns, data_quality, quantize_factor, DataQualityReport, ForwardReturnConfig,
    QuantizeConfig,
};
use finkit::returns::ReturnKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Native vs Alphalens-compatible public semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnalysisMode {
    Native,
    AlphalensCompat,
}

/// Returns section of a factor study.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReturnsReport {
    /// Horizon return observations used for factor diagnostics and Alphalens parity.
    pub factor_returns: BTreeMap<usize, Vec<f64>>,
    /// Compatibility cumulative curves over horizon observations. For tradable
    /// multi-day portfolio performance use `performance.by_holding_period`,
    /// which correctly models overlapping cohorts using daily P&L.
    pub cumulative_returns: BTreeMap<usize, Vec<f64>>,
    pub alpha_beta: BTreeMap<usize, AlphaBeta>,
    pub mean_return_by_quantile: BTreeMap<u16, BTreeMap<usize, f64>>,
    pub quantile_diagnostics: QuantileDiagnostics,
}

/// Information-coefficient section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InformationReport {
    pub ic_by_horizon: BTreeMap<usize, Vec<f64>>,
    pub mean_ic: BTreeMap<usize, f64>,
    /// IC dispersion, ICIR, naive t-stat, HAC/Newey-West t-stat and sign ratios.
    pub statistics: BTreeMap<usize, IcStatistics>,
}

/// Turnover and rank-persistence section.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurnoverReport {
    pub bottom_quantile_turnover: Vec<f64>,
    pub top_quantile_turnover: Vec<f64>,
    pub rank_autocorrelation: Vec<f64>,
}

/// Serializable computation-first report model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactorStudyReport {
    pub mode: AnalysisMode,
    pub provenance: StudyProvenance,
    pub data_quality: DataQualityReport,
    pub quantiles: u16,
    pub periods: Vec<usize>,
    /// Number of research dates between signal formation and portfolio activation.
    pub execution_lag: usize,
    pub returns: ReturnsReport,
    pub information: InformationReport,
    pub turnover: TurnoverReport,
    /// Tradable daily P&L evaluation for each requested holding period. Multi-day
    /// periods use overlapping cohorts rather than naively compounding forward returns.
    pub performance: FactorPortfolioPerformanceReport,
}

impl FactorStudyReport {
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// End-to-end factor research study.
#[derive(Debug)]
pub struct FactorStudy<'a> {
    frame: &'a ResearchFrame,
    factor_column: String,
    price_column: String,
    periods: Vec<usize>,
    quantize: QuantizeConfig,
    weights: WeightConfig,
    evaluation: EvaluationConfig,
    execution_lag: usize,
    mode: AnalysisMode,
    provenance: StudyProvenance,
}

impl<'a> FactorStudy<'a> {
    pub fn new(
        frame: &'a ResearchFrame,
        factor_column: impl Into<String>,
        price_column: impl Into<String>,
        periods: Vec<usize>,
    ) -> Self {
        Self {
            frame,
            factor_column: factor_column.into(),
            price_column: price_column.into(),
            periods,
            quantize: QuantizeConfig::default(),
            weights: WeightConfig::default(),
            evaluation: EvaluationConfig::default(),
            execution_lag: 0,
            mode: AnalysisMode::Native,
            provenance: StudyProvenance {
                library_version: env!("CARGO_PKG_VERSION").to_string(),
                ..StudyProvenance::default()
            },
        }
    }

    pub fn quantize_config(mut self, config: QuantizeConfig) -> Self {
        self.quantize = config;
        self
    }

    pub fn weight_config(mut self, config: WeightConfig) -> Self {
        self.weights = config;
        self
    }

    pub fn evaluation_config(mut self, config: EvaluationConfig) -> Self {
        self.evaluation = config;
        self
    }

    pub fn execution_lag(mut self, execution_lag: usize) -> Self {
        self.execution_lag = execution_lag;
        self
    }

    pub fn mode(mut self, mode: AnalysisMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn provenance(mut self, provenance: StudyProvenance) -> Self {
        self.provenance = provenance;
        self
    }

    pub fn full_report(&self) -> ResearchResult<FactorStudyReport> {
        // Performance for an H-day factor must use actual daily P&L of overlapping
        // H-day cohorts. Compute the one-day asset return once even when the caller
        // only requested longer research horizons.
        let mut calculation_periods = self.periods.clone();
        if !calculation_periods.contains(&1) {
            calculation_periods.push(1);
        }
        calculation_periods.sort_unstable();
        calculation_periods.dedup();
        let forward_config = ForwardReturnConfig::new(calculation_periods, ReturnKind::Arithmetic)?;
        let mut forward = compute_forward_returns(self.frame, &self.price_column, &forward_config)?;
        let daily_asset_returns = forward.get(&1).cloned().unwrap_or_default();
        if !self.periods.contains(&1) {
            forward.remove(&1);
        }

        let quantiles = quantize_factor(self.frame, &self.factor_column, &self.quantize)?;
        let weights = factor_weights(self.frame, &self.factor_column, &self.weights)?;
        let factor_ret = factor_returns(self.frame, &weights, &forward)?;
        let ic = information_coefficient(self.frame, &self.factor_column, &forward)?;
        let mean_ic = mean_information_coefficient(&ic);
        let ic_statistics = ic
            .iter()
            .map(|(&period, values)| (period, summarize_ic(values, period.saturating_sub(1))))
            .collect();
        let alpha_beta = factor_alpha_beta(self.frame, &factor_ret, &forward);
        let quantile_returns = mean_return_by_quantile(&quantiles, &forward);
        let quantile_diagnostics = quantile_diagnostics(&quantile_returns);
        let bottom_turnover = quantile_turnover(self.frame, &quantiles, 1, 1)?;
        let top_turnover = quantile_turnover(self.frame, &quantiles, self.quantize.quantiles, 1)?;
        let rank_auto = rank_autocorrelation(self.frame, &self.factor_column, 1)?;
        let performance = evaluate_factor_holding_periods(
            self.frame,
            &weights,
            &daily_asset_returns,
            &self.periods,
            self.execution_lag,
            self.evaluation,
        )?;
        let cumulative = performance
            .by_holding_period
            .iter()
            .map(|(&period, result)| (period, result.gross_cumulative_wealth.clone()))
            .collect();

        Ok(FactorStudyReport {
            mode: self.mode,
            provenance: self.provenance.clone(),
            data_quality: data_quality(self.frame, &self.factor_column)?,
            quantiles: self.quantize.quantiles,
            periods: self.periods.clone(),
            execution_lag: self.execution_lag,
            returns: ReturnsReport {
                factor_returns: factor_ret,
                cumulative_returns: cumulative,
                alpha_beta,
                mean_return_by_quantile: quantile_returns,
                quantile_diagnostics,
            },
            information: InformationReport {
                ic_by_horizon: ic,
                mean_ic,
                statistics: ic_statistics,
            },
            turnover: TurnoverReport {
                bottom_quantile_turnover: bottom_turnover,
                top_quantile_turnover: top_turnover,
                rank_autocorrelation: rank_auto,
            },
            performance,
        })
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
    fn full_report_connects_core_research_chain() {
        let frame = frame();
        let report = FactorStudy::new(&frame, "factor", "price", vec![1, 2])
            .quantize_config(QuantizeConfig {
                quantiles: 3,
                by_group: None,
                zero_aware: false,
            })
            .evaluation_config(EvaluationConfig {
                transaction_cost_bps: 5.0,
                slippage_bps: 2.0,
                ..EvaluationConfig::default()
            })
            .full_report()
            .unwrap();
        assert_eq!(report.periods, vec![1, 2]);
        assert_eq!(report.execution_lag, 0);
        assert_eq!(
            report.returns.cumulative_returns[&1],
            report.performance.by_holding_period[&1].gross_cumulative_wealth
        );
        assert!(report.information.mean_ic[&1] > 0.9);
        assert!(report.information.statistics[&1].positive_ratio > 0.0);
        assert!(report
            .returns
            .quantile_diagnostics
            .spread_by_horizon
            .contains_key(&1));
        let one_day = &report.performance.by_holding_period[&1];
        let two_day = &report.performance.by_holding_period[&2];
        assert!(one_day.gross.returns.observations > 0);
        assert!(one_day.gross.benchmark.is_some());
        assert!(one_day.after_cost.returns.total_return <= one_day.gross.returns.total_return);
        assert_eq!(two_day.portfolio.by_date.len(), 4);
        assert_eq!(two_day.portfolio.turnover_by_date.len(), 4);
        serde_json::to_string(&report).unwrap();
    }

    #[test]
    fn longer_horizon_does_not_require_user_to_request_one_day_output() {
        let frame = frame();
        let report = FactorStudy::new(&frame, "factor", "price", vec![2])
            .full_report()
            .unwrap();
        assert_eq!(report.periods, vec![2]);
        assert!(!report.returns.factor_returns.contains_key(&1));
        assert!(report.performance.by_holding_period.contains_key(&2));
    }
}
