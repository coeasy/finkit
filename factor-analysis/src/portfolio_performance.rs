//! Tradable holding-period portfolio performance.
//!
//! Multi-day forward factor returns overlap in time and therefore must not be
//! naively compounded as if they were independent daily P&L. This module uses
//! the existing overlapping-cohort engine to create actual daily positions,
//! then evaluates daily portfolio returns and costs with the canonical
//! performance SSOT.

use crate::data::{AssetId, ResearchFrame};
use crate::error::{ResearchError, ResearchResult};
use crate::performance::{
    evaluate_costs, evaluate_quant_performance, CostSummaryReport, EvaluationConfig,
    PortfolioMetricsReport, PortfolioSummaryReport, QuantEvaluationReport,
};
use crate::portfolio::HoldingPeriodPortfolioEngine;
use finkit::performance as core;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HorizonPortfolioPerformance {
    /// Actual daily P&L of overlapping active cohorts before costs.
    pub gross: QuantEvaluationReport,
    /// Actual daily P&L after linear turnover costs/slippage.
    pub after_cost: QuantEvaluationReport,
    pub portfolio: PortfolioSummaryReport,
    pub costs: CostSummaryReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactorPortfolioPerformanceReport {
    pub config: EvaluationConfig,
    pub by_holding_period: BTreeMap<usize, HorizonPortfolioPerformance>,
}

fn position_metrics(positions: &[BTreeMap<AssetId, f64>]) -> Vec<PortfolioMetricsReport> {
    positions
        .iter()
        .map(|position| {
            let weights: Vec<f64> = position.values().copied().collect();
            core::evaluate_portfolio(&weights).into()
        })
        .collect()
}

fn position_turnover(positions: &[BTreeMap<AssetId, f64>]) -> Vec<f64> {
    let mut result = Vec::with_capacity(positions.len());
    for (date, current) in positions.iter().enumerate() {
        if date == 0 {
            result.push(current.values().map(|value| value.abs()).sum());
            continue;
        }
        let previous = &positions[date - 1];
        let assets: BTreeSet<AssetId> = current.keys().chain(previous.keys()).copied().collect();
        result.push(
            0.5 * assets
                .iter()
                .map(|asset| {
                    (current.get(asset).copied().unwrap_or(0.0)
                        - previous.get(asset).copied().unwrap_or(0.0))
                    .abs()
                })
                .sum::<f64>(),
        );
    }
    result
}

fn summarize_positions(positions: &[BTreeMap<AssetId, f64>]) -> PortfolioSummaryReport {
    let by_date = position_metrics(positions);
    let turnover_by_date = position_turnover(positions);
    let count = by_date.len().max(1) as f64;
    PortfolioSummaryReport {
        average_turnover: turnover_by_date.iter().sum::<f64>()
            / turnover_by_date.len().max(1) as f64,
        turnover_by_date,
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

fn daily_portfolio_returns(
    frame: &ResearchFrame,
    positions: &[BTreeMap<AssetId, f64>],
    daily_asset_returns: &[f64],
) -> ResearchResult<Vec<f64>> {
    if daily_asset_returns.len() != frame.index().len() {
        return Err(ResearchError::LengthMismatch {
            name: "daily_asset_returns".to_string(),
            expected: frame.index().len(),
            actual: daily_asset_returns.len(),
        });
    }
    if positions.len() != frame.index().date_segments().len() {
        return Err(ResearchError::LengthMismatch {
            name: "positions".to_string(),
            expected: frame.index().date_segments().len(),
            actual: positions.len(),
        });
    }
    let mut output = Vec::with_capacity(positions.len());
    for (date, position) in positions.iter().enumerate() {
        let range = frame.index().date_segments().range(date).unwrap();
        let returns: BTreeMap<AssetId, f64> = range
            .filter_map(|row| {
                let value = daily_asset_returns[row];
                value
                    .is_finite()
                    .then_some((frame.index().assets()[row], value))
            })
            .collect();
        let mut pnl = 0.0_f64;
        let mut represented_weight = 0.0_f64;
        for (&asset, &weight) in position {
            if let Some(&ret) = returns.get(&asset) {
                pnl += weight * ret;
                represented_weight += weight.abs();
            }
        }
        output.push(
            if position.is_empty() || represented_weight <= f64::EPSILON {
                f64::NAN
            } else {
                pnl
            },
        );
    }
    Ok(output)
}

fn equal_weight_benchmark_returns(
    frame: &ResearchFrame,
    daily_asset_returns: &[f64],
) -> ResearchResult<Vec<f64>> {
    if daily_asset_returns.len() != frame.index().len() {
        return Err(ResearchError::LengthMismatch {
            name: "daily_asset_returns".to_string(),
            expected: frame.index().len(),
            actual: daily_asset_returns.len(),
        });
    }
    Ok(frame
        .index()
        .date_segments()
        .map(|range| {
            let valid: Vec<f64> = range
                .filter_map(|row| {
                    let value = daily_asset_returns[row];
                    value.is_finite().then_some(value)
                })
                .collect();
            if valid.is_empty() {
                f64::NAN
            } else {
                valid.iter().sum::<f64>() / valid.len() as f64
            }
        })
        .collect())
}

pub fn evaluate_factor_holding_periods(
    frame: &ResearchFrame,
    target_row_weights: &[f64],
    daily_asset_returns: &[f64],
    holding_periods: &[usize],
    config: EvaluationConfig,
) -> ResearchResult<FactorPortfolioPerformanceReport> {
    if target_row_weights.len() != frame.index().len() {
        return Err(ResearchError::LengthMismatch {
            name: "target_row_weights".to_string(),
            expected: frame.index().len(),
            actual: target_row_weights.len(),
        });
    }
    let benchmark = equal_weight_benchmark_returns(frame, daily_asset_returns)?;
    let mut by_holding_period = BTreeMap::new();
    for &period in holding_periods {
        let positions =
            HoldingPeriodPortfolioEngine::new(period)?.positions(frame, target_row_weights)?;
        let gross_returns = daily_portfolio_returns(frame, &positions, daily_asset_returns)?;
        let portfolio = summarize_positions(&positions);
        let costs = evaluate_costs(&portfolio.turnover_by_date, config);
        let net_returns: Vec<f64> = gross_returns
            .iter()
            .enumerate()
            .map(|(date, value)| {
                if value.is_finite() {
                    *value
                        - costs
                            .estimated_cost_rate_by_date
                            .get(date)
                            .copied()
                            .unwrap_or(0.0)
                } else {
                    *value
                }
            })
            .collect();
        by_holding_period.insert(
            period,
            HorizonPortfolioPerformance {
                gross: evaluate_quant_performance(&gross_returns, Some(&benchmark), config),
                after_cost: evaluate_quant_performance(&net_returns, Some(&benchmark), config),
                portfolio,
                costs,
            },
        );
    }
    Ok(FactorPortfolioPerformanceReport {
        config,
        by_holding_period,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PanelIndex;

    #[test]
    fn multi_day_horizon_uses_daily_overlapping_portfolio_pnl() {
        let index = PanelIndex::new(
            vec![1, 1, 2, 2, 3, 3, 4, 4],
            vec![
                AssetId(1),
                AssetId(2),
                AssetId(1),
                AssetId(2),
                AssetId(1),
                AssetId(2),
                AssetId(1),
                AssetId(2),
            ],
        )
        .unwrap();
        let frame = ResearchFrame::new(index);
        let weights = [0.5, -0.5, 0.8, -0.2, 0.6, -0.4, 0.5, -0.5];
        let daily_returns = [0.01, -0.01, 0.02, 0.00, -0.01, 0.02, f64::NAN, f64::NAN];
        let report = evaluate_factor_holding_periods(
            &frame,
            &weights,
            &daily_returns,
            &[1, 2],
            EvaluationConfig::default(),
        )
        .unwrap();
        assert_eq!(report.by_holding_period.len(), 2);
        assert_eq!(report.by_holding_period[&2].portfolio.by_date.len(), 4);
        assert!(report.by_holding_period[&2].gross.returns.observations >= 2);
    }
}
