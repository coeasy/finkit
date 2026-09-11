//! Serializable quantitative evaluation layer built on `finkit::performance`.
//!
//! Numerical formulas remain owned by the core SSOT. This module only adds
//! research semantics and a stable serde schema for all language bindings.

use crate::data::ResearchFrame;
use finkit::performance as core;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EvaluationConfig {
    pub annualization: usize,
    pub risk_free_rate: f64,
    pub minimum_acceptable_return: f64,
    pub var_confidence: f64,
}

impl Default for EvaluationConfig {
    fn default() -> Self {
        Self {
            annualization: 252,
            risk_free_rate: 0.0,
            minimum_acceptable_return: 0.0,
            var_confidence: 0.95,
        }
    }
}

impl From<EvaluationConfig> for core::PerformanceConfig {
    fn from(value: EvaluationConfig) -> Self {
        Self {
            annualization: value.annualization,
            risk_free_rate: value.risk_free_rate,
            minimum_acceptable_return: value.minimum_acceptable_return,
            var_confidence: value.var_confidence,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReturnMetricsReport {
    pub observations: usize,
    pub total_return: f64,
    pub annualized_return: f64,
    pub arithmetic_mean_return: f64,
    pub geometric_mean_return: f64,
    pub best_period_return: f64,
    pub worst_period_return: f64,
    pub positive_period_ratio: f64,
    pub negative_period_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrawdownMetricsReport {
    pub max_drawdown: f64,
    pub max_drawdown_peak_index: usize,
    pub max_drawdown_trough_index: usize,
    pub current_drawdown: f64,
    pub average_drawdown: f64,
    pub max_drawdown_duration: usize,
    pub average_drawdown_duration: f64,
    pub recovery_duration: Option<usize>,
    pub ulcer_index: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiskMetricsReport {
    pub annualized_volatility: f64,
    pub downside_volatility: f64,
    pub upside_volatility: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub calmar_ratio: f64,
    pub omega_ratio: f64,
    pub sterling_ratio: f64,
    pub burke_ratio: f64,
    pub historical_var: f64,
    pub parametric_var: f64,
    pub cvar: f64,
    pub tail_ratio: f64,
    pub skewness: f64,
    pub excess_kurtosis: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkMetricsReport {
    pub active_annualized_return: f64,
    pub tracking_error: f64,
    pub information_ratio: f64,
    pub alpha: f64,
    pub beta: f64,
    pub r_squared: f64,
    pub correlation: f64,
    pub upside_capture: f64,
    pub downside_capture: f64,
    pub treynor_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioMetricsReport {
    pub gross_exposure: f64,
    pub net_exposure: f64,
    pub long_exposure: f64,
    pub short_exposure: f64,
    pub leverage: f64,
    pub max_abs_weight: f64,
    pub hhi: f64,
    pub effective_number_of_bets: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioSummaryReport {
    pub by_date: Vec<PortfolioMetricsReport>,
    pub average_gross_exposure: f64,
    pub average_net_exposure: f64,
    pub average_hhi: f64,
    pub average_effective_number_of_bets: f64,
    pub maximum_abs_weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantEvaluationReport {
    pub returns: ReturnMetricsReport,
    pub risk: RiskMetricsReport,
    pub drawdown: DrawdownMetricsReport,
    pub benchmark: Option<BenchmarkMetricsReport>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub config: EvaluationConfig,
    pub by_horizon: BTreeMap<usize, QuantEvaluationReport>,
    pub portfolio: PortfolioSummaryReport,
}

#[inline]
fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

impl From<core::ReturnMetrics> for ReturnMetricsReport {
    fn from(value: core::ReturnMetrics) -> Self {
        Self {
            observations: value.observations,
            total_return: finite_or_zero(value.total_return),
            annualized_return: finite_or_zero(value.annualized_return),
            arithmetic_mean_return: finite_or_zero(value.arithmetic_mean_return),
            geometric_mean_return: finite_or_zero(value.geometric_mean_return),
            best_period_return: finite_or_zero(value.best_period_return),
            worst_period_return: finite_or_zero(value.worst_period_return),
            positive_period_ratio: finite_or_zero(value.positive_period_ratio),
            negative_period_ratio: finite_or_zero(value.negative_period_ratio),
        }
    }
}

impl From<core::DrawdownMetrics> for DrawdownMetricsReport {
    fn from(value: core::DrawdownMetrics) -> Self {
        Self {
            max_drawdown: finite_or_zero(value.max_drawdown),
            max_drawdown_peak_index: value.max_drawdown_peak_index,
            max_drawdown_trough_index: value.max_drawdown_trough_index,
            current_drawdown: finite_or_zero(value.current_drawdown),
            average_drawdown: finite_or_zero(value.average_drawdown),
            max_drawdown_duration: value.max_drawdown_duration,
            average_drawdown_duration: finite_or_zero(value.average_drawdown_duration),
            recovery_duration: value.recovery_duration,
            ulcer_index: finite_or_zero(value.ulcer_index),
        }
    }
}

impl From<core::RiskMetrics> for RiskMetricsReport {
    fn from(value: core::RiskMetrics) -> Self {
        Self {
            annualized_volatility: finite_or_zero(value.annualized_volatility),
            downside_volatility: finite_or_zero(value.downside_volatility),
            upside_volatility: finite_or_zero(value.upside_volatility),
            sharpe_ratio: finite_or_zero(value.sharpe_ratio),
            sortino_ratio: finite_or_zero(value.sortino_ratio),
            calmar_ratio: finite_or_zero(value.calmar_ratio),
            omega_ratio: finite_or_zero(value.omega_ratio),
            sterling_ratio: finite_or_zero(value.sterling_ratio),
            burke_ratio: finite_or_zero(value.burke_ratio),
            historical_var: finite_or_zero(value.historical_var),
            parametric_var: finite_or_zero(value.parametric_var),
            cvar: finite_or_zero(value.cvar),
            tail_ratio: finite_or_zero(value.tail_ratio),
            skewness: finite_or_zero(value.skewness),
            excess_kurtosis: finite_or_zero(value.excess_kurtosis),
        }
    }
}

impl From<core::BenchmarkMetrics> for BenchmarkMetricsReport {
    fn from(value: core::BenchmarkMetrics) -> Self {
        Self {
            active_annualized_return: finite_or_zero(value.active_annualized_return),
            tracking_error: finite_or_zero(value.tracking_error),
            information_ratio: finite_or_zero(value.information_ratio),
            alpha: finite_or_zero(value.alpha),
            beta: finite_or_zero(value.beta),
            r_squared: finite_or_zero(value.r_squared),
            correlation: finite_or_zero(value.correlation),
            upside_capture: finite_or_zero(value.upside_capture),
            downside_capture: finite_or_zero(value.downside_capture),
            treynor_ratio: finite_or_zero(value.treynor_ratio),
        }
    }
}

impl From<core::PortfolioMetrics> for PortfolioMetricsReport {
    fn from(value: core::PortfolioMetrics) -> Self {
        Self {
            gross_exposure: finite_or_zero(value.gross_exposure),
            net_exposure: finite_or_zero(value.net_exposure),
            long_exposure: finite_or_zero(value.long_exposure),
            short_exposure: finite_or_zero(value.short_exposure),
            leverage: finite_or_zero(value.leverage),
            max_abs_weight: finite_or_zero(value.max_abs_weight),
            hhi: finite_or_zero(value.hhi),
            effective_number_of_bets: finite_or_zero(value.effective_number_of_bets),
        }
    }
}

pub fn evaluate_quant_performance(
    returns: &[f64],
    benchmark: Option<&[f64]>,
    config: EvaluationConfig,
) -> QuantEvaluationReport {
    let core_config: core::PerformanceConfig = config.into();
    let evaluation = core::evaluate_returns(returns, core_config);
    QuantEvaluationReport {
        returns: evaluation.returns.into(),
        risk: evaluation.risk.into(),
        drawdown: evaluation.drawdown.into(),
        benchmark: benchmark.map(|values| core::evaluate_benchmark(returns, values, core_config).into()),
    }
}

/// Equal-weight universe return per research date for every forward horizon.
pub fn universe_returns(
    frame: &ResearchFrame,
    forward_returns: &BTreeMap<usize, Vec<f64>>,
) -> BTreeMap<usize, Vec<f64>> {
    forward_returns
        .iter()
        .map(|(&period, values)| {
            let per_date = frame.index().date_segments().map(|range| {
                let mut sum = 0.0;
                let mut count = 0usize;
                for row in range {
                    let value = values[row];
                    if value.is_finite() {
                        sum += value;
                        count += 1;
                    }
                }
                if count == 0 { f64::NAN } else { sum / count as f64 }
            });
            (period, per_date)
        })
        .collect()
}

pub fn evaluate_horizons(
    factor_returns: &BTreeMap<usize, Vec<f64>>,
    benchmark_returns: &BTreeMap<usize, Vec<f64>>,
    config: EvaluationConfig,
) -> BTreeMap<usize, QuantEvaluationReport> {
    factor_returns
        .iter()
        .map(|(&period, returns)| {
            let benchmark = benchmark_returns.get(&period).map(Vec::as_slice);
            (period, evaluate_quant_performance(returns, benchmark, config))
        })
        .collect()
}

pub fn evaluate_portfolio_by_date(frame: &ResearchFrame, weights: &[f64]) -> PortfolioSummaryReport {
    let by_date: Vec<PortfolioMetricsReport> = frame
        .index()
        .date_segments()
        .map(|range| core::evaluate_portfolio(&weights[range]).into())
        .collect();
    let count = by_date.len().max(1) as f64;
    PortfolioSummaryReport {
        average_gross_exposure: by_date.iter().map(|v| v.gross_exposure).sum::<f64>() / count,
        average_net_exposure: by_date.iter().map(|v| v.net_exposure).sum::<f64>() / count,
        average_hhi: by_date.iter().map(|v| v.hhi).sum::<f64>() / count,
        average_effective_number_of_bets: by_date
            .iter()
            .map(|v| v.effective_number_of_bets)
            .sum::<f64>()
            / count,
        maximum_abs_weight: by_date.iter().map(|v| v.max_abs_weight).fold(0.0, f64::max),
        by_date,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{AssetId, PanelIndex};

    #[test]
    fn research_performance_is_json_safe_and_complete() {
        let report = evaluate_quant_performance(
            &[0.03, -0.01, 0.02, 0.01],
            Some(&[0.01, -0.02, 0.01, 0.005]),
            EvaluationConfig::default(),
        );
        assert!(report.returns.total_return > 0.0);
        assert!(report.risk.annualized_volatility > 0.0);
        assert!(report.benchmark.unwrap().tracking_error >= 0.0);
        serde_json::to_string(&report).unwrap();
    }

    #[test]
    fn portfolio_summary_is_segmented_by_date() {
        let index = PanelIndex::new(
            vec![1, 1, 2, 2],
            vec![AssetId(1), AssetId(2), AssetId(1), AssetId(2)],
        )
        .unwrap();
        let frame = ResearchFrame::new(index);
        let report = evaluate_portfolio_by_date(&frame, &[0.5, -0.5, 0.75, -0.25]);
        assert_eq!(report.by_date.len(), 2);
        assert!(report.average_gross_exposure > 0.0);
    }
}
