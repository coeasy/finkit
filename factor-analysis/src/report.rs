use crate::analysis::{
    cumulative_factor_returns, factor_alpha_beta, factor_returns, factor_weights,
    information_coefficient, mean_information_coefficient, mean_return_by_quantile,
    quantile_turnover, rank_autocorrelation, AlphaBeta, WeightConfig,
};
use crate::data::ResearchFrame;
use crate::error::ResearchResult;
use crate::factor_metrics::{quantile_diagnostics, summarize_ic, IcStatistics, QuantileDiagnostics};
use crate::orchestration::StudyProvenance;
use crate::performance::{
    build_performance_report, universe_returns, EvaluationConfig, PerformanceReport,
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
    pub factor_returns: BTreeMap<usize, Vec<f64>>,
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
    pub returns: ReturnsReport,
    pub information: InformationReport,
    pub turnover: TurnoverReport,
    /// Unified gross/net strategy, risk, benchmark, cost and portfolio evaluation.
    pub performance: PerformanceReport,
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

    pub fn mode(mut self, mode: AnalysisMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn provenance(mut self, provenance: StudyProvenance) -> Self {
        self.provenance = provenance;
        self
    }

    pub fn full_report(&self) -> ResearchResult<FactorStudyReport> {
        let forward_config =
            ForwardReturnConfig::new(self.periods.clone(), ReturnKind::Arithmetic)?;
        let forward = compute_forward_returns(self.frame, &self.price_column, &forward_config)?;
        let quantiles = quantize_factor(self.frame, &self.factor_column, &self.quantize)?;
        let weights = factor_weights(self.frame, &self.factor_column, &self.weights)?;
        let factor_ret = factor_returns(self.frame, &weights, &forward)?;
        let ic = information_coefficient(self.frame, &self.factor_column, &forward)?;
        let mean_ic = mean_information_coefficient(&ic);
        let ic_statistics = ic
            .iter()
            .map(|(&period, values)| (period, summarize_ic(values, period.saturating_sub(1))))
            .collect();
        let cumulative = cumulative_factor_returns(&factor_ret);
        let alpha_beta = factor_alpha_beta(self.frame, &factor_ret, &forward);
        let quantile_returns = mean_return_by_quantile(&quantiles, &forward);
        let quantile_diagnostics = quantile_diagnostics(&quantile_returns);
        let bottom_turnover = quantile_turnover(self.frame, &quantiles, 1, 1)?;
        let top_turnover = quantile_turnover(self.frame, &quantiles, self.quantize.quantiles, 1)?;
        let rank_auto = rank_autocorrelation(self.frame, &self.factor_column, 1)?;
        let universe = universe_returns(self.frame, &forward);
        let performance = build_performance_report(
            self.frame,
            &factor_ret,
            &universe,
            &weights,
            self.evaluation,
        );
        Ok(FactorStudyReport {
            mode: self.mode,
            provenance: self.provenance.clone(),
            data_quality: data_quality(self.frame, &self.factor_column)?,
            quantiles: self.quantize.quantiles,
            periods: self.periods.clone(),
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

    #[test]
    fn full_report_connects_core_research_chain() {
        let index = PanelIndex::new(
            vec![1, 1, 1, 2, 2, 2, 3, 3, 3],
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
            ],
        )
        .unwrap();
        let mut frame = ResearchFrame::new(index);
        frame
            .add_numeric(
                "factor",
                "factor",
                vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0, 1.0, 2.0, 3.0],
            )
            .unwrap();
        frame
            .add_numeric(
                "price",
                "market",
                vec![10.0, 10.0, 10.0, 11.0, 12.0, 13.0, 12.0, 14.0, 16.0],
            )
            .unwrap();
        let report = FactorStudy::new(&frame, "factor", "price", vec![1])
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
        assert_eq!(report.periods, vec![1]);
        assert!(report.information.mean_ic[&1] > 0.9);
        assert!(report.information.statistics[&1].positive_ratio > 0.0);
        assert!(report
            .returns
            .quantile_diagnostics
            .spread_by_horizon
            .contains_key(&1));
        let gross = &report.performance.by_horizon[&1];
        let net = &report.performance.after_cost_by_horizon[&1];
        assert!(gross.returns.observations > 0);
        assert!(gross.benchmark.is_some());
        assert!(net.returns.total_return <= gross.returns.total_return);
        assert_eq!(report.performance.portfolio.by_date.len(), 3);
        assert_eq!(report.performance.portfolio.turnover_by_date.len(), 3);
        serde_json::to_string(&report).unwrap();
    }
}