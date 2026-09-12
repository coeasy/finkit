//! Factor-specific evaluation summaries built from existing research outputs.

use crate::risk_model::newey_west_mean;
use finkit::math::statistics::{correlation, std_dev};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IcStatistics {
    pub observations: usize,
    pub mean: f64,
    pub standard_deviation: f64,
    pub icir: f64,
    pub naive_t_stat: f64,
    pub hac_t_stat: f64,
    pub positive_ratio: f64,
    pub negative_ratio: f64,
}

pub fn summarize_ic(values: &[f64], hac_lags: usize) -> IcStatistics {
    let finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if finite.is_empty() {
        return IcStatistics {
            observations: 0,
            mean: 0.0,
            standard_deviation: 0.0,
            icir: 0.0,
            naive_t_stat: 0.0,
            hac_t_stat: 0.0,
            positive_ratio: 0.0,
            negative_ratio: 0.0,
        };
    }
    let mean = finite.iter().sum::<f64>() / finite.len() as f64;
    let std = if finite.len() >= 2 {
        std_dev(&finite).unwrap_or(0.0)
    } else {
        0.0
    };
    let naive_t = if std > f64::EPSILON {
        mean / (std / (finite.len() as f64).sqrt())
    } else {
        0.0
    };
    let hac = newey_west_mean(&finite, hac_lags);
    IcStatistics {
        observations: finite.len(),
        mean,
        standard_deviation: std,
        icir: if std > f64::EPSILON { mean / std } else { 0.0 },
        naive_t_stat: naive_t,
        hac_t_stat: if hac.t_stat.is_finite() {
            hac.t_stat
        } else {
            0.0
        },
        positive_ratio: finite.iter().filter(|v| **v > 0.0).count() as f64 / finite.len() as f64,
        negative_ratio: finite.iter().filter(|v| **v < 0.0).count() as f64 / finite.len() as f64,
    }
}

pub fn summarize_ic_by_horizon(
    ic: &BTreeMap<usize, Vec<f64>>,
    hac_lags: usize,
) -> BTreeMap<usize, IcStatistics> {
    ic.iter()
        .map(|(&period, values)| (period, summarize_ic(values, hac_lags)))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuantileDiagnostics {
    pub spread_by_horizon: BTreeMap<usize, f64>,
    pub monotonicity_by_horizon: BTreeMap<usize, f64>,
}

/// Top-minus-bottom quantile spread and rank/return monotonicity.
pub fn quantile_diagnostics(
    mean_return_by_quantile: &BTreeMap<u16, BTreeMap<usize, f64>>,
) -> QuantileDiagnostics {
    let min_q = mean_return_by_quantile.keys().copied().min();
    let max_q = mean_return_by_quantile.keys().copied().max();
    let mut spread = BTreeMap::new();
    let mut monotonicity = BTreeMap::new();
    let horizons: Vec<usize> = mean_return_by_quantile
        .values()
        .flat_map(|values| values.keys().copied())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    for period in horizons {
        if let (Some(low_q), Some(high_q)) = (min_q, max_q) {
            let low = mean_return_by_quantile
                .get(&low_q)
                .and_then(|values| values.get(&period))
                .copied()
                .unwrap_or(f64::NAN);
            let high = mean_return_by_quantile
                .get(&high_q)
                .and_then(|values| values.get(&period))
                .copied()
                .unwrap_or(f64::NAN);
            if low.is_finite() && high.is_finite() {
                spread.insert(period, high - low);
            }
        }

        let pairs: Vec<(f64, f64)> = mean_return_by_quantile
            .iter()
            .filter_map(|(&quantile, values)| {
                let ret = values.get(&period).copied()?;
                ret.is_finite().then_some((quantile as f64, ret))
            })
            .collect();
        let value = if pairs.len() >= 2 {
            let q: Vec<f64> = pairs.iter().map(|v| v.0).collect();
            let r: Vec<f64> = pairs.iter().map(|v| v.1).collect();
            correlation(&q, &r).unwrap_or(0.0)
        } else {
            0.0
        };
        monotonicity.insert(period, value);
    }

    QuantileDiagnostics {
        spread_by_horizon: spread,
        monotonicity_by_horizon: monotonicity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ic_summary_reports_quality_and_hac() {
        let stats = summarize_ic(&[0.1, 0.2, 0.15, -0.05, 0.12], 1);
        assert_eq!(stats.observations, 5);
        assert!(stats.mean > 0.0);
        assert!(stats.positive_ratio > 0.5);
    }

    #[test]
    fn quantile_diagnostics_detect_monotonic_returns() {
        let mut input = BTreeMap::new();
        input.insert(1, BTreeMap::from([(5, -0.02)]));
        input.insert(2, BTreeMap::from([(5, 0.00)]));
        input.insert(3, BTreeMap::from([(5, 0.03)]));
        let result = quantile_diagnostics(&input);
        assert!((result.spread_by_horizon[&5] - 0.05).abs() < 1e-12);
        assert!(result.monotonicity_by_horizon[&5] > 0.9);
    }
}
