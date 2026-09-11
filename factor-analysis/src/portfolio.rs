use crate::analysis::{factor_weights, WeightConfig};
use crate::data::{AssetId, ResearchFrame};
use crate::error::{ResearchError, ResearchResult};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

/// Research portfolio constraints applied after signal-to-weight conversion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortfolioConstraints {
    pub gross_target: f64,
    pub max_abs_weight: f64,
    pub long_only: bool,
}

impl Default for PortfolioConstraints {
    fn default() -> Self {
        Self {
            gross_target: 1.0,
            max_abs_weight: 1.0,
            long_only: false,
        }
    }
}

/// Build constrained target weights from a factor column.
pub fn target_weights(
    frame: &ResearchFrame,
    factor_column: &str,
    weight_config: &WeightConfig,
    constraints: &PortfolioConstraints,
) -> ResearchResult<Vec<f64>> {
    if constraints.gross_target < 0.0 || constraints.max_abs_weight <= 0.0 {
        return Err(ResearchError::InvalidConfig(
            "invalid portfolio constraints".to_string(),
        ));
    }
    let mut weights = factor_weights(frame, factor_column, weight_config)?;
    for segment in 0..frame.index().date_segments().len() {
        let range = frame.index().date_segments().range(segment).unwrap();
        for row in range.clone() {
            if constraints.long_only && weights[row] < 0.0 {
                weights[row] = 0.0;
            }
            weights[row] =
                weights[row].clamp(-constraints.max_abs_weight, constraints.max_abs_weight);
        }
        let gross: f64 = range.clone().map(|row| weights[row].abs()).sum();
        if gross > f64::EPSILON {
            for row in range {
                weights[row] = weights[row] / gross * constraints.gross_target;
            }
        }
    }
    Ok(weights)
}

#[derive(Debug, Clone)]
struct Cohort {
    expires_on_date: usize,
    weights: BTreeMap<AssetId, f64>,
}

/// Overlapping holding-period engine used by factor research portfolios.
#[derive(Debug, Clone)]
pub struct HoldingPeriodPortfolioEngine {
    pub holding_dates: usize,
}

impl HoldingPeriodPortfolioEngine {
    pub fn new(holding_dates: usize) -> ResearchResult<Self> {
        if holding_dates == 0 {
            return Err(ResearchError::InvalidConfig(
                "holding_dates must be > 0".to_string(),
            ));
        }
        Ok(Self { holding_dates })
    }

    /// Produce one normalized asset-weight map per research date using zero execution lag.
    pub fn positions(
        &self,
        frame: &ResearchFrame,
        row_weights: &[f64],
    ) -> ResearchResult<Vec<BTreeMap<AssetId, f64>>> {
        self.positions_with_lag(frame, row_weights, 0)
    }

    /// Produce positions after delaying factor formation by `execution_lag` research dates.
    pub fn positions_with_lag(
        &self,
        frame: &ResearchFrame,
        row_weights: &[f64],
        execution_lag: usize,
    ) -> ResearchResult<Vec<BTreeMap<AssetId, f64>>> {
        if row_weights.len() != frame.index().len() {
            return Err(ResearchError::LengthMismatch {
                name: "row_weights".to_string(),
                expected: frame.index().len(),
                actual: row_weights.len(),
            });
        }
        let mut active: VecDeque<Cohort> = VecDeque::new();
        let mut output = Vec::with_capacity(frame.index().date_segments().len());
        for date_idx in 0..frame.index().date_segments().len() {
            while active
                .front()
                .is_some_and(|cohort| cohort.expires_on_date <= date_idx)
            {
                active.pop_front();
            }
            if let Some(formation_date) = date_idx.checked_sub(execution_lag) {
                let range = frame
                    .index()
                    .date_segments()
                    .range(formation_date)
                    .expect("valid formation date");
                let cohort_weights: BTreeMap<AssetId, f64> = range
                    .filter_map(|row| {
                        let weight = row_weights[row];
                        (weight.is_finite() && weight != 0.0)
                            .then_some((frame.index().assets()[row], weight))
                    })
                    .collect();
                if !cohort_weights.is_empty() {
                    active.push_back(Cohort {
                        expires_on_date: date_idx + self.holding_dates,
                        weights: cohort_weights,
                    });
                }
            }
            let mut combined = BTreeMap::<AssetId, f64>::new();
            for cohort in &active {
                for (&asset, &weight) in &cohort.weights {
                    *combined.entry(asset).or_default() += weight;
                }
            }
            let gross: f64 = combined.values().map(|weight| weight.abs()).sum();
            if gross > f64::EPSILON {
                for weight in combined.values_mut() {
                    *weight /= gross;
                }
            }
            output.push(combined);
        }
        Ok(output)
    }
}

/// Trade intent for transaction-cost estimation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TradeIntent {
    pub notional: f64,
    pub delta_weight: f64,
}

/// Liquidity observation used by cost/capacity models.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiquiditySnapshot {
    pub price: f64,
    pub adv_notional: f64,
    pub half_spread_bps: f64,
    pub volatility: f64,
}

/// Pluggable transaction cost model.
pub trait CostModel: Send + Sync {
    fn estimate(&self, trade: TradeIntent, market: LiquiditySnapshot) -> f64;
}

/// Linear commission + spread/slippage model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearCostModel {
    pub commission_bps: f64,
    pub slippage_bps: f64,
}

impl CostModel for LinearCostModel {
    fn estimate(&self, trade: TradeIntent, market: LiquiditySnapshot) -> f64 {
        let traded = trade.notional.abs() * trade.delta_weight.abs();
        let bps = self.commission_bps + self.slippage_bps + market.half_spread_bps.max(0.0);
        traded * bps * 1e-4
    }
}

/// Square-root market-impact model plus spread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SquareRootImpactModel {
    pub impact_coefficient: f64,
}

impl CostModel for SquareRootImpactModel {
    fn estimate(&self, trade: TradeIntent, market: LiquiditySnapshot) -> f64 {
        let traded = trade.notional.abs() * trade.delta_weight.abs();
        if traded == 0.0 || market.adv_notional <= 0.0 {
            return 0.0;
        }
        let participation = (traded / market.adv_notional).max(0.0);
        let impact_rate =
            self.impact_coefficient.max(0.0) * market.volatility.max(0.0) * participation.sqrt();
        traded * (impact_rate + market.half_spread_bps.max(0.0) * 1e-4)
    }
}

/// Evaluate estimated costs over a sequence of AUM levels.
pub fn capacity_curve(
    aum_levels: &[f64],
    delta_weight: f64,
    market: LiquiditySnapshot,
    model: &dyn CostModel,
) -> Vec<(f64, f64)> {
    aum_levels
        .iter()
        .copied()
        .map(|aum| {
            let cost = model.estimate(
                TradeIntent {
                    notional: aum,
                    delta_weight,
                },
                market,
            );
            (aum, if aum > 0.0 { cost / aum } else { 0.0 })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{PanelIndex, ResearchFrame};

    #[test]
    fn execution_lag_delays_cohort_activation() {
        let index = PanelIndex::new(
            vec![1, 1, 2, 2],
            vec![AssetId(1), AssetId(2), AssetId(1), AssetId(2)],
        )
        .unwrap();
        let frame = ResearchFrame::new(index);
        let positions = HoldingPeriodPortfolioEngine::new(1)
            .unwrap()
            .positions_with_lag(&frame, &[0.5, -0.5, 0.8, -0.2], 1)
            .unwrap();
        assert!(positions[0].is_empty());
        assert_eq!(positions[1].get(&AssetId(1)).copied(), Some(0.5));
        assert_eq!(positions[1].get(&AssetId(2)).copied(), Some(-0.5));
    }

    #[test]
    fn holding_engine_overlaps_cohorts() {
        let index = PanelIndex::new(
            vec![1, 1, 2, 2],
            vec![AssetId(1), AssetId(2), AssetId(1), AssetId(2)],
        )
        .unwrap();
        let frame = ResearchFrame::new(index);
        let positions = HoldingPeriodPortfolioEngine::new(2)
            .unwrap()
            .positions(&frame, &[0.5, -0.5, 0.8, -0.2])
            .unwrap();
        assert_eq!(positions.len(), 2);
        assert!((positions[1].values().map(|v| v.abs()).sum::<f64>() - 1.0).abs() < 1e-12);
    }
}
