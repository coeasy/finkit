//! Stock Selection Factor Engine (选股因子合成).
//!
//! Combine 100+ indicators into a single factor value per stock per bar.
//! Supports rank-based normalization, cross-sectional ranking, and
//! weighted aggregation — the core building blocks of a quantitative
//! stock-selection pipeline.
//!
//! # Example
//!
//! ```
//! use alpha_ta_core::selectors::{rank_zscore, Direction, Factor, FactorEngine};
//! use alpha_ta_core::error::Result;
//! use ndarray::Array1;
//! use std::sync::Arc;
//!
//! // Define a single factor
//! let factor = Factor::new(
//!     "momentum_20",
//!     Arc::new(|data: &[f64]| -> Result<Array1<f64>> {
//!         // Simple momentum: pct_change over 20 bars
//!         let n = data.len();
//!         let mut out = Array1::from(vec![f64::NAN; n]);
//!         for i in 20..n {
//!             if data[i - 20] != 0.0 {
//!                 out[i] = (data[i] - data[i - 20]) / data[i - 20];
//!             }
//!         }
//!         Ok(out)
//!     }),
//!     Direction::HigherBetter,
//! );
//!
//! // Create engine and compute
//! let engine = FactorEngine::new(vec![factor], vec![1.0]);
//! let data: Vec<f64> = (0..50).map(|i| 100.0 + i as f64).collect();
//! let factor_value = engine.compute(&data).unwrap();
//! assert_eq!(factor_value.len(), 50);
//! ```

use crate::error::{Result, TaError};
use crate::utils::validate_input;
use ndarray::Array1;
use std::sync::Arc;

/// 因子方向：值越高越好 / 越低越好
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Higher factor value is better (e.g. momentum, ROE).
    HigherBetter,
    /// Lower factor value is better (e.g. P/E, volatility).
    LowerBetter,
}

/// 因子定义 — a named indicator computation with a directional bias.
pub struct Factor {
    /// Human-readable factor name (e.g. "momentum_20").
    pub name: String,
    /// The factor's computation closure. Receives the raw input series
    /// and returns a per-bar factor value. NaN bars are treated as
    /// missing by the engine.
    pub compute: Arc<dyn Fn(&[f64]) -> Result<Array1<f64>> + Send + Sync>,
    /// Direction of "goodness" for ranking.
    pub direction: Direction,
}

impl Factor {
    /// Create a new factor.
    pub fn new(
        name: impl Into<String>,
        compute: Arc<dyn Fn(&[f64]) -> Result<Array1<f64>> + Send + Sync>,
        direction: Direction,
    ) -> Self {
        Self {
            name: name.into(),
            compute,
            direction,
        }
    }
}

/// 因子引擎 — orchestrates computation of multiple factors, applies
/// direction-aware ranking, and produces a single composite score.
pub struct FactorEngine {
    pub factors: Vec<Factor>,
    pub weights: Vec<f64>,
    pub rank_window: usize,
}

impl FactorEngine {
    /// Create a new factor engine with the given factors and weights.
    /// The number of weights must equal the number of factors.
    pub fn new(factors: Vec<Factor>, weights: Vec<f64>) -> Self {
        Self {
            factors,
            weights,
            rank_window: 0,
        }
    }

    /// Set the rolling window for [`rank_zscore`]-style normalization.
    /// 0 = no normalization (use raw values).
    pub fn with_rank_window(mut self, window: usize) -> Self {
        self.rank_window = window;
        self
    }

    /// Compute the composite factor value for each bar of `data`.
    ///
    /// For each factor: compute its raw series, then:
    /// - If `Direction::LowerBetter`, multiply by -1 so higher is always better.
    /// - If `rank_window > 0`, apply rolling z-score normalization.
    /// - Ignore missing values and re-normalize weights independently per row.
    ///
    /// Per-row normalization is important when factors have different warm-up
    /// lengths: a missing factor must not dilute another valid factor.
    pub fn compute(&self, data: &[f64]) -> Result<Array1<f64>> {
        if self.factors.len() != self.weights.len() {
            return Err(TaError::InvalidParameter {
                name: "weights".to_string(),
                constraint: "must match number of factors".to_string(),
            });
        }
        validate_input(data.len(), 1)?;
        let n = data.len();
        let mut composite = Array1::<f64>::zeros(n);
        let mut effective_weights = vec![0.0_f64; n];

        for (factor, &weight) in self.factors.iter().zip(self.weights.iter()) {
            if !weight.is_finite() || weight == 0.0 {
                continue;
            }
            let mut raw = (factor.compute)(data)?;
            if raw.len() != n {
                return Err(TaError::InvalidParameter {
                    name: "factor output".to_string(),
                    constraint: format!(
                        "factor {} returned len={}, expected {}",
                        factor.name,
                        raw.len(),
                        n
                    ),
                });
            }
            if factor.direction == Direction::LowerBetter {
                raw.mapv_inplace(|value| if value.is_finite() { -value } else { f64::NAN });
            }
            if self.rank_window > 1 {
                raw = rank_zscore(&raw, self.rank_window);
            }
            for index in 0..n {
                if raw[index].is_finite() {
                    composite[index] += raw[index] * weight;
                    effective_weights[index] += weight.abs();
                }
            }
        }

        for index in 0..n {
            if effective_weights[index] > 1e-15 {
                composite[index] /= effective_weights[index];
            } else {
                composite[index] = f64::NAN;
            }
        }
        Ok(composite)
    }

    /// Cross-sectional rank: given `per_stock[i]` = factor value for stock
    /// `i`, return the percentile rank (0..1) for each stock.
    /// Uses the supplied `direction` to decide whether highest is best
    /// (HigherBetter) or lowest is best (LowerBetter).
    pub fn cross_sectional_rank(
        &self,
        per_stock: &[f64],
        direction: Direction,
    ) -> Vec<f64> {
        cross_sectional_rank_impl(per_stock, direction)
    }
}

/// Rolling z-score (mean / std) over a window. For each bar `i`, computes
/// `(x[i] - mean(window)) / std(window)` over the last `window` bars
/// (including the current). Bars where the window is incomplete or std
/// is zero are left as NaN.
pub fn rank_zscore(values: &Array1<f64>, window: usize) -> Array1<f64> {
    if window == 0 {
        return values.clone();
    }
    let n = values.len();
    let mut out = Array1::<f64>::from(vec![f64::NAN; n]);
    if n < window {
        return out;
    }
    for i in (window - 1)..n {
        let owned: Vec<f64>;
        let slice: &[f64];
        if let Some(values) = values.as_slice() {
            slice = &values[i + 1 - window..=i];
        } else {
            owned = values.to_vec();
            slice = &owned[i + 1 - window..=i];
        }
        let mut sum = 0.0;
        let mut count = 0;
        for &value in slice {
            if value.is_finite() {
                sum += value;
                count += 1;
            }
        }
        if count == 0 {
            continue;
        }
        let mean = sum / count as f64;
        let mut var_sum = 0.0;
        for &value in slice {
            if value.is_finite() {
                let delta = value - mean;
                var_sum += delta * delta;
            }
        }
        let std = (var_sum / count as f64).sqrt();
        if std < 1e-15 {
            continue;
        }
        if values[i].is_finite() {
            out[i] = (values[i] - mean) / std;
        }
    }
    out
}

/// Cross-sectional rank: given a vector of values (one per stock), return
/// the percentile rank (0..1) of each finite value. Ties receive their average
/// rank and non-finite values remain NaN. Direction-aware: for
/// `HigherBetter`, the highest value gets rank 1.0; for `LowerBetter`,
/// the lowest value gets rank 1.0.
pub fn cross_sectional_rank(values: &[f64]) -> Vec<f64> {
    cross_sectional_rank_impl(values, Direction::HigherBetter)
}

fn cross_sectional_rank_impl(values: &[f64], direction: Direction) -> Vec<f64> {
    let mut indexed: Vec<(usize, f64)> = values
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, value)| value.is_finite())
        .collect();
    let mut ranks = vec![f64::NAN; values.len()];
    if indexed.is_empty() {
        return ranks;
    }
    if indexed.len() == 1 {
        ranks[indexed[0].0] = 0.5;
        return ranks;
    }

    indexed.sort_by(|left, right| left.1.total_cmp(&right.1));
    let denominator = (indexed.len() - 1) as f64;
    let mut start = 0;
    while start < indexed.len() {
        let mut end = start + 1;
        while end < indexed.len() && indexed[end].1 == indexed[start].1 {
            end += 1;
        }
        let average_position = (start + end - 1) as f64 / 2.0;
        let ascending_rank = average_position / denominator;
        let rank = match direction {
            Direction::HigherBetter => ascending_rank,
            Direction::LowerBetter => 1.0 - ascending_rank,
        };
        for &(original_index, _) in &indexed[start..end] {
            ranks[original_index] = rank;
        }
        start = end;
    }
    ranks
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn make_factor(name: &str, bias: f64) -> Factor {
        Factor::new(
            name.to_string(),
            Arc::new(move |data: &[f64]| -> Result<Array1<f64>> {
                Ok(Array1::from(
                    data.iter().map(|&value| value + bias).collect::<Vec<_>>(),
                ))
            }),
            Direction::HigherBetter,
        )
    }

    #[test]
    fn test_factor_engine_basic() {
        let factors = vec![make_factor("a", 0.0), make_factor("b", 1.0)];
        let engine = FactorEngine::new(factors, vec![0.5, 0.5]);
        let data: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let out = engine.compute(&data).unwrap();
        for i in 0..10 {
            assert_relative_eq!(out[i], i as f64 + 0.5, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_factor_engine_weight_mismatch() {
        let factors = vec![make_factor("a", 0.0)];
        let engine = FactorEngine::new(factors, vec![0.5, 0.5]);
        let data = vec![1.0; 10];
        assert!(engine.compute(&data).is_err());
    }

    #[test]
    fn test_factor_engine_lower_better() {
        let factor = Factor::new(
            "invert".to_string(),
            Arc::new(|data: &[f64]| -> Result<Array1<f64>> {
                Ok(Array1::from(
                    data.iter().map(|&value| 100.0 - value).collect::<Vec<f64>>(),
                ))
            }),
            Direction::LowerBetter,
        );
        let engine = FactorEngine::new(vec![factor], vec![1.0]);
        let data: Vec<f64> = (0..10).map(|i| i as f64).collect();
        let out = engine.compute(&data).unwrap();
        for i in 0..10 {
            assert_relative_eq!(out[i], i as f64 - 100.0, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_factor_engine_renormalizes_missing_rows() {
        let a = Factor::new(
            "a",
            Arc::new(|_| Ok(Array1::from(vec![2.0, 2.0]))),
            Direction::HigherBetter,
        );
        let b = Factor::new(
            "b",
            Arc::new(|_| Ok(Array1::from(vec![2.0, f64::NAN]))),
            Direction::HigherBetter,
        );
        let engine = FactorEngine::new(vec![a, b], vec![1.0, 1.0]);
        let out = engine.compute(&[1.0, 2.0]).unwrap();
        assert_relative_eq!(out[0], 2.0, epsilon = 1e-10);
        assert_relative_eq!(out[1], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_rank_zscore_constant() {
        let values = Array1::from(vec![5.0; 10]);
        let out = rank_zscore(&values, 5);
        for &value in &out {
            assert!(value.is_nan());
        }
    }

    #[test]
    fn test_rank_zscore_known() {
        let values = Array1::from(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        let out = rank_zscore(&values, 5);
        assert_relative_eq!(out[4], 2.0 / 2f64.sqrt(), epsilon = 1e-10);
        assert!(out[2].is_nan(), "early bar should be NaN, got {}", out[2]);

        let out3 = rank_zscore(&values, 3);
        let std3 = (2.0f64 / 3.0).sqrt();
        assert_relative_eq!(out3[2], (3.0 - 2.0) / std3, epsilon = 1e-10);
    }

    #[test]
    fn test_cross_sectional_rank_higher_better() {
        let values = vec![1.0, 3.0, 2.0, 5.0, 4.0];
        let ranks = cross_sectional_rank(&values);
        assert_relative_eq!(ranks[3], 1.0, epsilon = 1e-10);
        assert_relative_eq!(ranks[0], 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_cross_sectional_rank_lower_better() {
        let values = vec![1.0, 3.0, 2.0, 5.0, 4.0];
        let ranks = cross_sectional_rank_impl(&values, Direction::LowerBetter);
        assert_relative_eq!(ranks[0], 1.0, epsilon = 1e-10);
        assert_relative_eq!(ranks[3], 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_cross_sectional_rank_ties_and_nan() {
        let ranks = cross_sectional_rank(&[3.0, 1.0, 3.0, f64::NAN, 2.0]);
        assert_relative_eq!(ranks[0], 5.0 / 6.0, epsilon = 1e-10);
        assert_relative_eq!(ranks[2], 5.0 / 6.0, epsilon = 1e-10);
        assert_relative_eq!(ranks[1], 0.0, epsilon = 1e-10);
        assert!(ranks[3].is_nan());
        assert_relative_eq!(ranks[4], 1.0 / 3.0, epsilon = 1e-10);
    }

    #[test]
    fn test_cross_sectional_rank_empty() {
        let ranks = cross_sectional_rank(&[]);
        assert!(ranks.is_empty());
    }
}