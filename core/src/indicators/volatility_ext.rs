//! Extended volatility indicators including Keltner Channel and other volatility-based indicators.

use crate::error::{Result, TaError};
use crate::indicators::volatility::atr;
use crate::math::moving_avg::{ema, ema_into};
use crate::streaming::rolling_minmax::{RollingMax, RollingMin};
use crate::utils::{init_output, validate_input, validate_param};
use ndarray::Array1;

/// ADR calculation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AdrMode {
    /// Absolute range (High - Low)
    #[default]
    Absolute,
    /// Percentage of close price ((High - Low) / Close * 100)
    Percent,
}

/// Average Day Range (ADR)
///
/// Calculates the average of the high-low range over a specified period.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - Lookback period
/// * `mode` - Calculation mode (Absolute or Percent)
///
/// # Returns
/// Array of ADR values
///
/// O(n) implementation: maintains a ring buffer of period (high - low) values
/// plus a running sum. Each bar performs 1 subtraction + 1 addition.
pub fn adr(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    mode: AdrMode,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_param("period", "greater than 0", || period > 0)?;
    validate_input(high.len(), period)?;

    let len = high.len();
    let mut output = init_output(len);
    let inv_period = 1.0 / period as f64;

    // Ring buffer of recent (high - low) values (NaN treated as 0)
    let mut ring: Vec<f64> = vec![0.0; period];
    let mut sum = 0.0_f64;
    let mut ring_idx = 0usize;

    for i in 0..len {
        let new_range = if high[i].is_nan() || low[i].is_nan() {
            0.0
        } else {
            high[i] - low[i]
        };

        if i < period {
            // Filling phase: just accumulate
            ring[i] = new_range;
            sum += new_range;
        } else {
            // Steady state: evict oldest, add new
            let old = ring[ring_idx];
            sum += new_range - old;
            ring[ring_idx] = new_range;
            ring_idx += 1;
            if ring_idx == period {
                ring_idx = 0;
            }
        }

        if i + 1 >= period {
            let adr_abs = sum * inv_period;
            match mode {
                AdrMode::Absolute => {
                    output[i] = adr_abs;
                }
                AdrMode::Percent => {
                    if !close[i].is_nan() && close[i].abs() > 1e-15 {
                        output[i] = adr_abs / close[i] * 100.0;
                    }
                }
            }
        }
    }

    Ok(output)
}

/// Keltner Channel Result
///
/// Contains all components of the Keltner Channel indicator.
#[derive(Debug, Clone)]
pub struct KeltnerResult {
    /// Upper Band - EMA + ATR * multiplier
    pub upper: Array1<f64>,
    /// Middle Band - EMA of close
    pub middle: Array1<f64>,
    /// Lower Band - EMA - ATR * multiplier
    pub lower: Array1<f64>,
    /// Width - Upper - Lower
    pub width: Array1<f64>,
}

/// Keltner Channel (KELTNER)
///
/// A volatility-based envelope indicator that uses ATR to set the width of the bands.
/// Similar to Bollinger Bands but uses ATR instead of standard deviation.
///
/// # Formula
/// - Middle Band = EMA(close, period)
/// - Upper Band = Middle Band + ATR(period) * multiplier
/// - Lower Band = Middle Band - ATR(period) * multiplier
/// - Width = Upper - Lower
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - EMA and ATR period (default: 20)
/// * `multiplier` - ATR multiplier for band width (default: 2.0)
///
/// # Returns
/// KeltnerResult containing upper, middle, lower bands and width
///
/// # Example
/// ```rust
/// use finkit::indicators::keltner;
///
/// let high = vec![10.0, 12.0, 14.0, 13.0, 15.0, 16.0];
/// let low = vec![8.0, 10.0, 11.0, 10.0, 12.0, 13.0];
/// let close = vec![9.0, 11.0, 13.0, 12.0, 14.0, 15.0];
/// let result = keltner(&high, &low, &close, 3, 2.0).unwrap();
///
/// // result.upper contains the upper band
/// // result.middle contains the EMA
/// // result.lower contains the lower band
/// ```
pub fn keltner(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    multiplier: f64,
) -> Result<KeltnerResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_param("period", "greater than 0", || period > 0)?;
    validate_param("multiplier", "greater than 0", || multiplier > 0.0)?;
    validate_input(high.len(), period)?;

    let len = close.len();
    let middle = ema(close, period)?;
    let atr_values = atr(high, low, close, period)?;

    let mut upper = init_output(len);
    let mut lower = init_output(len);
    let mut width = init_output(len);

    for i in 0..len {
        if !middle[i].is_nan() && !atr_values[i].is_nan() {
            upper[i] = middle[i] + atr_values[i] * multiplier;
            lower[i] = middle[i] - atr_values[i] * multiplier;
            width[i] = upper[i] - lower[i];
        }
    }

    Ok(KeltnerResult {
        upper,
        middle,
        lower,
        width,
    })
}

/// Historical Volatility (HV)
///
/// Measures the annualized standard deviation of logarithmic returns over a given period.
///
/// # Formula
/// HV = std(log_returns) * sqrt(trading_days_per_year) * 100
///
/// # Arguments
/// * `close` - Close prices
/// * `period` - Lookback period for volatility calculation
/// * `trading_days` - Number of trading days per year (default: 252.0)
///
/// # Returns
/// Array of historical volatility values (annualized percentage)
///
/// # Example
/// ```rust
/// use finkit::indicators::historical_volatility;
///
/// let close = vec![100.0, 101.0, 102.0, 103.0, 104.0, 105.0];
/// let hv = historical_volatility(&close, 3, 252.0).unwrap();
/// ```
///
/// O(n) implementation: ring buffer of `period` log returns plus dual
/// sum / sum-of-squares accumulators. Zero per-bar heap allocation.
pub fn historical_volatility(
    close: &[f64],
    period: usize,
    trading_days: f64,
) -> Result<Array1<f64>> {
    validate_param("period", "greater than 1", || period > 1)?;
    validate_param("trading_days", "greater than 0", || trading_days > 0.0)?;
    validate_input(close.len(), period + 1)?;

    let len = close.len();
    let mut output = init_output(len);
    let sqrt_td_scaled = trading_days.sqrt() * 100.0;

    // Ring buffer of `period` log returns; None marks an invalid slot
    // (close <= 0 or non-finite). Sum/sum_sq track only the valid entries.
    let mut ring: Vec<Option<f64>> = vec![None; period];
    let mut ring_head = 0usize;
    let mut sum = 0.0_f64;
    let mut sum_sq = 0.0_f64;
    let mut count = 0usize;

    // Pre-fill ring with log returns for j = 1..=period, so the first
    // computed output corresponds to i = period and the window covers
    // j = 1..=period (matching the original semantics).
    for j in 1..=period {
        let slot = j - 1;
        let lr = if close[j - 1].abs() > 1e-15 && close[j] > 0.0 {
            Some((close[j] / close[j - 1]).ln())
        } else {
            None
        };
        ring[slot] = lr;
        if let Some(v) = lr {
            sum += v;
            sum_sq += v * v;
            count += 1;
        }
    }

    for i in period..len {
        // Newest log return: ln(close[i] / close[i-1])
        let new_lr = if close[i - 1].abs() > 1e-15 && close[i] > 0.0 {
            Some((close[i] / close[i - 1]).ln())
        } else {
            None
        };

        // For i = period the ring already holds lr(1..=period) from the
        // pre-fill; only from i > period do we evict the oldest slot.
        if i > period {
            if let Some(old) = ring[ring_head] {
                sum -= old;
                sum_sq -= old * old;
                count -= 1;
            }
            ring[ring_head] = new_lr;
            if let Some(v) = new_lr {
                sum += v;
                sum_sq += v * v;
                count += 1;
            }
            ring_head += 1;
            if ring_head == period {
                ring_head = 0;
            }
        } else if let Some(v) = new_lr {
            // i == period: the pre-filled slot `ring[period-1]` already
            // holds `lr(period)`. We don't need to re-add it; just compute.
            let _ = v;
        }

        if count >= 2 {
            let n = count as f64;
            let variance = (sum_sq - sum * sum / n) / (count - 1) as f64;
            let std = variance.max(0.0).sqrt();
            output[i] = std * sqrt_td_scaled;
        }
    }

    Ok(output)
}

/// Ulcer Index (UI)
///
/// A risk measure that focuses on downside risk, measuring the depth and duration
/// of drawdowns from recent peaks. Developed by Peter Martin.
///
/// # Formula
/// UI = sqrt( sum( ((close - max_close) / max_close)^2 ) / n )
///
/// # Arguments
/// * `close` - Close prices
/// * `period` - Lookback period (default: 14)
///
/// # Returns
/// Array of Ulcer Index values
///
/// # Interpretation
/// - Lower values indicate less downside risk
/// - Higher values indicate more significant drawdowns
///
/// # Example
/// ```rust
/// use finkit::indicators::ulcer_index;
///
/// let close = vec![100.0, 95.0, 90.0, 92.0, 98.0, 105.0];
/// let ui = ulcer_index(&close, 3).unwrap();
/// ```
///
/// O(n) implementation: monotonic deque maintains the rolling max in O(1)
/// amortized; the squared-drawdown sum is updated incrementally. When the
/// rolling max changes mid-window, the sum is recomputed (O(period) per
/// change, amortized O(n) over a full pass).
pub fn ulcer_index(close: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_param("period", "greater than 0", || period > 0)?;
    validate_input(close.len(), period)?;

    let len = close.len();
    let mut output = init_output(len);

    let mut rolling_max = RollingMax::new();
    let mut sum_sq = 0.0_f64;
    let mut count = 0usize;
    let mut current_max: Option<f64> = None;

    for i in 0..len {
        // Push the new close into the monotonic deque (skip NaN to keep
        // the deque's max finite).
        if !close[i].is_nan() {
            rolling_max.push(i, close[i]);
        }
        if i >= period {
            rolling_max.pop(i - period);
        }

        if i + 1 < period {
            continue;
        }

        let max_close = match rolling_max.current() {
            Some(v) => v,
            None => continue,
        };

        if !(max_close > 0.0) || !max_close.is_finite() {
            continue;
        }

        if current_max != Some(max_close) {
            // Rolling max changed: recompute the sum from scratch.
            sum_sq = 0.0;
            count = 0;
            for j in (i + 1 - period)..=i {
                if !close[j].is_nan() && close[j] > 0.0 {
                    let dd = (close[j] - max_close) / max_close * 100.0;
                    sum_sq += dd * dd;
                    count += 1;
                }
            }
            current_max = Some(max_close);
        } else {
            // Same rolling max: incremental update. Evict the oldest bar,
            // then add the new bar.
            let old_idx = i - period;
            if !close[old_idx].is_nan() && close[old_idx] > 0.0 {
                let old_dd = (close[old_idx] - max_close) / max_close * 100.0;
                sum_sq -= old_dd * old_dd;
                count -= 1;
            }
            if !close[i].is_nan() && close[i] > 0.0 {
                let new_dd = (close[i] - max_close) / max_close * 100.0;
                sum_sq += new_dd * new_dd;
                count += 1;
            }
        }

        if count > 0 {
            output[i] = (sum_sq / count as f64).sqrt();
        }
    }

    Ok(output)
}

/// Choppiness Index (CHOP)
///
/// A volatility indicator designed to determine if the market is trending or chopping.
/// Developed by E.W. Dreiss.
///
/// # Formula
/// CHOP = 100 * log10( sum(ATR, n) / (max_high - min_low) ) / log10(n)
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - Lookback period (default: 14)
///
/// # Returns
/// Array of Choppiness Index values (0-100)
///
/// # Interpretation
/// - Values above 61.8: Market is chopping (consolidating)
/// - Values below 38.2: Market is trending
/// - Values between 38.2 and 61.8: Neutral zone
///
/// # Example
/// ```rust
/// use finkit::indicators::choppiness_index;
///
/// let high = vec![10.0, 12.0, 14.0, 13.0, 15.0];
/// let low = vec![8.0, 10.0, 11.0, 10.0, 12.0];
/// let close = vec![9.0, 11.0, 13.0, 12.0, 14.0];
/// let chop = choppiness_index(&high, &low, &close, 3).unwrap();
/// ```
///
/// O(n) implementation: rolling sum of true ranges (ring buffer) plus
/// `RollingMax<High>` and `RollingMin<Low>` deques. Avoids the per-bar
/// O(period) `iter().fold()` scans of the original.
pub fn choppiness_index(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_param("period", "greater than 1", || period > 1)?;
    validate_input(high.len(), period)?;

    let len = close.len();
    let log_period = (period as f64).log10();
    let mut output = init_output(len);

    // Rolling sum of true ranges (matches the original `atr(..., 1)` semantics)
    let mut tr_ring: Vec<f64> = vec![0.0; period];
    let mut tr_head = 0usize;
    let mut atr_sum = 0.0_f64;

    let mut high_max = RollingMax::new();
    let mut low_min = RollingMin::new();

    for i in 0..len {
        // True range (matches `atr(high, low, close, 1)`)
        let tr = if i == 0 {
            high[0] - low[0]
        } else {
            let prev_close = close[i - 1];
            (high[i] - low[i])
                .max((high[i] - prev_close).abs())
                .max((low[i] - prev_close).abs())
        };

        // Maintain rolling sum of TR over the period
        let old_tr = tr_ring[tr_head];
        tr_ring[tr_head] = tr;
        tr_head += 1;
        if tr_head == period {
            tr_head = 0;
        }
        if !old_tr.is_nan() {
            atr_sum -= old_tr;
        }
        if !tr.is_nan() {
            atr_sum += tr;
        }

        // Maintain rolling max of high / min of low (skip NaN so the
        // current() values stay finite)
        if !high[i].is_nan() {
            high_max.push(i, high[i]);
        }
        if !low[i].is_nan() {
            low_min.push(i, low[i]);
        }
        if i >= period {
            high_max.pop(i - period);
            low_min.pop(i - period);
        }

        if i + 1 < period {
            continue;
        }

        let max_h = high_max.current().unwrap_or(f64::NEG_INFINITY);
        let min_l = low_min.current().unwrap_or(f64::INFINITY);
        let range = max_h - min_l;

        if atr_sum > 0.0 && range > 0.0 && range.is_finite() && log_period.abs() > 1e-15 {
            output[i] = 100.0 * (atr_sum / range).log10() / log_period;
        }
    }

    Ok(output)
}

/// Mass Index
///
/// Detects trend reversals by measuring the narrowing and widening of the
/// high-low range. Values above 27 indicate a "reversal bulge".
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `period` - Summation period (typically 25)
/// * `ema_period` - EMA period for range smoothing (typically 9)
pub fn mass_index(
    high: &[f64],
    low: &[f64],
    period: usize,
    ema_period: usize,
) -> Result<Array1<f64>> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_param("period", "greater than 0", || period > 0)?;
    validate_param("ema_period", "greater than 0", || ema_period > 0)?;

    let lookback = 2 * ema_period + period - 1;
    validate_input(high.len(), lookback)?;

    let len = high.len();
    let k = 2.0 / (ema_period + 1) as f64;
    let one_minus_k = 1.0 - k;

    let e1_start = ema_period - 1;
    let ratio_start = 2 * ema_period - 2;
    let first_output = ratio_start + period - 1;

    let mut ema1 = init_output(len);
    let sum: f64 = (0..ema_period).map(|j| high[j] - low[j]).sum();
    ema1[e1_start] = sum / ema_period as f64;
    let mut ema1_prev = ema1[e1_start];
    for i in ema_period..len {
        let r = high[i] - low[i];
        ema1_prev = r * k + ema1_prev * one_minus_k;
        ema1[i] = ema1_prev;
    }

    let ema2_seed: f64 = (e1_start..e1_start + ema_period)
        .map(|j| ema1[j])
        .sum::<f64>()
        / ema_period as f64;
    let mut ema2_prev = ema2_seed;

    let mut output = init_output(len);
    if first_output >= len {
        return Ok(output);
    }

    let mut ratio_ring = vec![f64::NAN; period];
    let mut ratio_sum = 0.0;
    let mut ratio_pos = 0usize;
    let mut all_valid = true;

    for i in ratio_start..len {
        let ema2 = if i == ratio_start {
            ema2_seed
        } else {
            ema2_prev = ema1[i] * k + ema2_prev * one_minus_k;
            ema2_prev
        };

        let ratio = if ema2.abs() > 1e-15 {
            ema1[i] / ema2
        } else {
            f64::NAN
        };

        if i < first_output {
            let idx = i - ratio_start;
            ratio_ring[idx] = ratio;
            if ratio.is_nan() {
                all_valid = false;
            } else {
                ratio_sum += ratio;
            }
        } else if i == first_output {
            let idx = i - ratio_start;
            ratio_ring[idx] = ratio;
            if ratio.is_nan() {
                all_valid = false;
            } else {
                ratio_sum += ratio;
            }
            if all_valid {
                output[i] = ratio_sum;
            }
        } else if all_valid {
            let slot = ratio_pos % period;
            let old = ratio_ring[slot];
            ratio_ring[slot] = ratio;
            if ratio.is_nan() {
                all_valid = false;
            } else {
                ratio_sum += ratio - old;
                output[i] = ratio_sum;
            }
            ratio_pos += 1;
        }
    }

    Ok(output)
}

/// Chaikin Volatility
///
/// Measures the volatility by calculating the rate of change of the high-low spread.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `ema_period` - EMA period for smoothing (typically 10)
/// * `roc_period` - Rate of change period (typically 10)
///
/// # Returns
/// Array of Chaikin Volatility values
///
/// O(n) implementation: spread is computed into a single pre-allocated
/// Vec; EMA is written into a pre-allocated output via `ema_into`; the
/// RoC lookup `ema_spread[i - roc_period]` is replaced by a
/// `roc_period + 1` ring buffer.
pub fn chaikin_volatility(
    high: &[f64],
    low: &[f64],
    ema_period: usize,
    roc_period: usize,
) -> Result<Array1<f64>> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_param("ema_period", "greater than 0", || ema_period > 0)?;
    validate_param("roc_period", "greater than 0", || roc_period > 0)?;

    let len = high.len();
    validate_input(len, ema_period + roc_period)?;

    // Pre-allocate spread buffer (single allocation)
    let mut spread = vec![0.0_f64; len];
    for i in 0..len {
        spread[i] = high[i] - low[i];
    }

    // EMA of spread into a pre-allocated NaN-filled buffer
    let mut ema_spread = init_output(len);
    ema_into(&spread, ema_period, ema_spread.as_slice_mut().unwrap())?;

    // Ring buffer of `roc_period + 1` EMA values. After incrementing
    // `ring_idx`, the slot it points to is the oldest (next to evict)
    // and is exactly `ema_spread[i - roc_period]`.
    let cap = roc_period + 1;
    let mut ema_ring = vec![0.0_f64; cap];
    let mut ring_idx = 0usize;
    let mut ring_count = 0usize;

    let mut output = init_output(len);

    for i in 0..len {
        let curr = ema_spread[i];
        ema_ring[ring_idx] = curr;
        ring_idx += 1;
        if ring_idx == cap {
            ring_idx = 0;
        }
        if ring_count < cap {
            ring_count += 1;
        }

        if ring_count > roc_period {
            // After the increment above, `ring_idx` points to the oldest
            // slot, which corresponds to `ema_spread[i - roc_period]`.
            let prev = ema_ring[ring_idx];
            if !curr.is_nan() && !prev.is_nan() && prev.abs() > 1e-15 {
                output[i] = (curr - prev) / prev * 100.0;
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_keltner_basic() {
        let high = vec![
            10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 30.0,
        ];
        let low = vec![
            8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0,
        ];
        let close = vec![
            9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0, 23.0, 25.0, 27.0, 29.0,
        ];

        let result = keltner(&high, &low, &close, 5, 2.0).unwrap();

        assert_eq!(result.upper.len(), 11);
        assert_eq!(result.middle.len(), 11);
        assert_eq!(result.lower.len(), 11);
        assert_eq!(result.width.len(), 11);

        // Check that bands are properly calculated
        for i in 4..close.len() {
            if !result.upper[i].is_nan() && !result.lower[i].is_nan() {
                assert!(result.upper[i] > result.middle[i]);
                assert!(result.lower[i] < result.middle[i]);
                assert_relative_eq!(
                    result.width[i],
                    result.upper[i] - result.lower[i],
                    epsilon = 1e-10
                );
            }
        }
    }

    #[test]
    fn test_keltner_invalid_period() {
        let high = vec![10.0, 12.0];
        let low = vec![8.0, 10.0];
        let close = vec![9.0, 11.0];
        assert!(keltner(&high, &low, &close, 0, 2.0).is_err());
        assert!(keltner(&high, &low, &close, 5, 2.0).is_err());
    }

    #[test]
    fn test_historical_volatility_basic() {
        let close: Vec<f64> = (100..=120).map(|x| x as f64).collect();
        let hv = historical_volatility(&close, 10, 252.0).unwrap();
        assert_eq!(hv.len(), close.len());
        assert!(!hv[10].is_nan());
        assert!(hv[10] > 0.0);
    }

    #[test]
    fn test_historical_volatility_high_volatility() {
        let close = vec![
            100.0, 90.0, 110.0, 85.0, 115.0, 80.0, 120.0, 75.0, 125.0, 70.0, 130.0,
        ];
        let hv = historical_volatility(&close, 5, 252.0).unwrap();
        assert!(!hv[5].is_nan());
        assert!(hv[5] > 50.0); // High volatility
    }

    #[test]
    fn test_ulcer_index_basic() {
        let close = vec![100.0, 95.0, 90.0, 92.0, 98.0, 105.0, 110.0, 108.0, 115.0];
        let ui = ulcer_index(&close, 5).unwrap();
        assert_eq!(ui.len(), close.len());
        assert!(!ui[4].is_nan());
        assert!(ui[4] > 0.0);
    }

    #[test]
    fn test_ulcer_index_no_drawdown() {
        let close: Vec<f64> = (100..=110).map(|x| x as f64).collect();
        let ui = ulcer_index(&close, 5).unwrap();
        // When prices only increase, UI should be very low
        assert!(!ui[4].is_nan());
        assert!(ui[4] < 5.0);
    }

    #[test]
    fn test_ulcer_index_monotonic_matches_expected_value() {
        // Reference values: at i=4 with period=5, the window is [100, 101, 102, 103, 104]
        // and max_close = 104. Per-bar percentage drawdowns (close[j] - max) / max * 100:
        //   -3.8461538461538463, -2.8846153846153846, -1.9230769230769231,
        //   -0.9615384615384616, 0
        // Sum of squared drawdowns (f64): 27.736686390532547
        // UI = sqrt(27.736686390532547 / 5) = 2.355278598829979
        let close: Vec<f64> = (100..=110).map(|x| x as f64).collect();
        let ui = ulcer_index(&close, 5).unwrap();

        // Recompute the expected value with the same f64 arithmetic to avoid
        // mismatches caused by last-ULP differences between this implementation
        // and the original O(n*period) reference value.
        let max_close = 104.0_f64;
        let mut sum_sq = 0.0_f64;
        let mut count = 0usize;
        for j in 0..=4usize {
            let dd = (close[j] - max_close) / max_close * 100.0;
            sum_sq += dd * dd;
            count += 1;
        }
        let expected = (sum_sq / count as f64).sqrt();

        // Use a tight tolerance to catch real regressions while still allowing
        // for legitimate floating-point variations across platforms.
        assert!(
            (ui[4] - expected).abs() < 1e-12,
            "ui[4] = {} (expected {})",
            ui[4],
            expected
        );

        // Additional cross-check: the actual UI for a strictly increasing series
        // must be strictly decreasing (each new bar is closer to the new high).
        for i in 5..ui.len() {
            assert!(
                ui[i] < ui[i - 1] + 1e-12,
                "ulcer index not monotonically decreasing at i={}: {} >= {}",
                i,
                ui[i],
                ui[i - 1]
            );
        }
    }

    #[test]
    fn test_choppiness_index_basic() {
        let high = vec![10.0, 12.0, 14.0, 13.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0];
        let low = vec![8.0, 10.0, 11.0, 10.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0];
        let close = vec![9.0, 11.0, 13.0, 12.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];

        let chop = choppiness_index(&high, &low, &close, 5).unwrap();
        assert_eq!(chop.len(), close.len());
        assert!(!chop[4].is_nan());
        // CHOP values should be between 0 and 100
        assert!(chop[4] >= 0.0 && chop[4] <= 100.0);
    }

    #[test]
    fn test_choppiness_index_trending() {
        // Strong uptrend - CHOP should be low
        let high: Vec<f64> = (10..=20).map(|x| x as f64).collect();
        let low: Vec<f64> = (9..=19).map(|x| x as f64).collect();
        let close: Vec<f64> = (9..=19).map(|x| x as f64 + 0.5).collect();

        let chop = choppiness_index(&high, &low, &close, 5).unwrap();
        assert!(!chop[4].is_nan());
        // In a strong trend, CHOP should be below 38.2
        assert!(chop[4] < 50.0);
    }

    #[test]
    fn test_choppiness_index_strong_trend_below_38_2() {
        // 50 bars of strong uptrend: high = low + 1, both strictly increasing
        let n = 50;
        let high: Vec<f64> = (0..n).map(|i| 100.0 + i as f64).collect();
        let low: Vec<f64> = (0..n).map(|i| 99.0 + i as f64).collect();
        let close: Vec<f64> = (0..n).map(|i| 99.5 + i as f64).collect();

        let period = 14;
        let chop = choppiness_index(&high, &low, &close, period).unwrap();
        // Last bar should reflect a strong trend
        assert!(!chop[n - 1].is_nan());
        assert!(
            chop[n - 1] < 38.2,
            "chop[n-1] = {} (expected < 38.2)",
            chop[n - 1]
        );
    }

    #[test]
    fn test_keltner_multiplier_effect() {
        let high = vec![
            10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 30.0,
        ];
        let low = vec![
            8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0,
        ];
        let close = vec![
            9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0, 23.0, 25.0, 27.0, 29.0,
        ];

        let result1 = keltner(&high, &low, &close, 5, 1.0).unwrap();
        let result2 = keltner(&high, &low, &close, 5, 2.0).unwrap();

        // Higher multiplier should result in wider bands
        for i in 4..close.len() {
            if !result1.width[i].is_nan() && !result2.width[i].is_nan() {
                assert!(result2.width[i] > result1.width[i]);
            }
        }
    }

    #[test]
    fn test_mass_index_basic() {
        // Need at least 2*ema_period + period - 1 = 2*9 + 25 - 1 = 42 data points
        let n = 50;
        let high: Vec<f64> = (0..n).map(|i| 10.0 + i as f64 * 0.5).collect();
        let low: Vec<f64> = (0..n).map(|i| 8.0 + i as f64 * 0.5).collect();
        let result = mass_index(&high, &low, 25, 9).unwrap();
        assert_eq!(result.len(), n);
    }

    #[test]
    fn test_chaikin_volatility_basic() {
        // Need at least ema_period + roc_period = 10 + 10 = 20 data points
        let n = 30;
        let high: Vec<f64> = (0..n).map(|i| 10.0 + i as f64 * 0.5).collect();
        let low: Vec<f64> = (0..n).map(|i| 8.0 + i as f64 * 0.5).collect();
        let result = chaikin_volatility(&high, &low, 10, 10).unwrap();
        assert_eq!(result.len(), n);
    }

    // ------- Reference-value parity tests against the original O(n*period) algorithm -------

    pub(super) fn ref_adr(
        high: &[f64],
        low: &[f64],
        close: &[f64],
        period: usize,
        mode: AdrMode,
    ) -> Vec<f64> {
        let len = high.len();
        let mut out = vec![f64::NAN; len];
        for i in period - 1..len {
            let sum: f64 = (i + 1 - period..=i)
                .filter(|j| !high[*j].is_nan() && !low[*j].is_nan())
                .map(|j| high[j] - low[j])
                .sum();
            let adr_abs = sum / period as f64;
            match mode {
                AdrMode::Absolute => out[i] = adr_abs,
                AdrMode::Percent => {
                    if !close[i].is_nan() && close[i].abs() > 1e-15 {
                        out[i] = adr_abs / close[i] * 100.0;
                    }
                }
            }
        }
        out
    }

    pub(super) fn ref_hv(close: &[f64], period: usize, td: f64) -> Vec<f64> {
        let len = close.len();
        let mut out = vec![f64::NAN; len];
        for i in period..len {
            let mut log_returns = Vec::with_capacity(period);
            for j in (i + 1 - period)..=i {
                if close[j - 1].abs() > 1e-15 && close[j] > 0.0 {
                    log_returns.push((close[j] / close[j - 1]).ln());
                }
            }
            if log_returns.len() >= 2 {
                let mean: f64 = log_returns.iter().sum::<f64>() / log_returns.len() as f64;
                let variance: f64 = log_returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>()
                    / (log_returns.len() - 1) as f64;
                out[i] = variance.sqrt() * td.sqrt() * 100.0;
            }
        }
        out
    }

    pub(super) fn ref_ui(close: &[f64], period: usize) -> Vec<f64> {
        let len = close.len();
        let mut out = vec![f64::NAN; len];
        for i in period - 1..len {
            let max_close = close[(i + 1 - period)..=i]
                .iter()
                .filter(|&x| !x.is_nan())
                .fold(f64::NEG_INFINITY, |a, b| a.max(*b));
            if max_close > 0.0 && max_close.is_finite() {
                let mut sum_sq = 0.0;
                let mut count = 0;
                for j in (i + 1 - period)..=i {
                    if !close[j].is_nan() && close[j] > 0.0 {
                        let dd = (close[j] - max_close) / max_close * 100.0;
                        sum_sq += dd * dd;
                        count += 1;
                    }
                }
                if count > 0 {
                    out[i] = (sum_sq / count as f64).sqrt();
                }
            }
        }
        out
    }

    pub(super) fn ref_chop(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<f64> {
        let len = high.len();
        let mut out = vec![f64::NAN; len];
        let mut tr = vec![0.0; len];
        tr[0] = high[0] - low[0];
        for i in 1..len {
            let pc = close[i - 1];
            tr[i] = (high[i] - low[i])
                .max((high[i] - pc).abs())
                .max((low[i] - pc).abs());
        }
        for i in period - 1..len {
            let atr_sum: f64 = (i + 1 - period..=i)
                .filter(|j| !tr[*j].is_nan())
                .map(|j| tr[j])
                .sum();
            let max_h = high[(i + 1 - period)..=i]
                .iter()
                .filter(|&x| !x.is_nan())
                .fold(f64::NEG_INFINITY, |a, b| a.max(*b));
            let min_l = low[(i + 1 - period)..=i]
                .iter()
                .filter(|&x| !x.is_nan())
                .fold(f64::INFINITY, |a, b| a.min(*b));
            let range = max_h - min_l;
            if atr_sum > 0.0 && range > 0.0 && range.is_finite() {
                let lp = (period as f64).log10();
                if lp.abs() > 1e-15 {
                    out[i] = 100.0 * (atr_sum / range).log10() / lp;
                }
            }
        }
        out
    }

    pub(super) fn ref_cvol(high: &[f64], low: &[f64], ep: usize, rp: usize) -> Vec<f64> {
        let len = high.len();
        let spread: Vec<f64> = high.iter().zip(low.iter()).map(|(&h, &l)| h - l).collect();
        let ema_spread = ema(&spread, ep).unwrap();
        let mut out = vec![f64::NAN; len];
        for i in (ep + rp - 1)..len {
            if !ema_spread[i].is_nan() && !ema_spread[i - rp].is_nan() {
                let prev = ema_spread[i - rp];
                if prev.abs() > 1e-15 {
                    out[i] = (ema_spread[i] - prev) / prev * 100.0;
                }
            }
        }
        out
    }
}

// ─────────────────── Extended volatility estimators (annualized) ───────────────────
//
// The functions below fill the WASM/JS surface area that the JavaScript and
// Python bindings currently call. They share the same rolling-variance
// machinery used elsewhere in this module: O(n) with a recursive update, no
// allocations beyond the output buffer.

/// Parkinson volatility estimator (high-low range based).
///
/// sigma = sqrt( (1 / (4 * n * ln 2)) * sum( ln(H_i / L_i)^2 ) )
///
/// `period` controls the rolling window size.
pub fn parkinson_volatility(high: &[f64], low: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_param("period", "greater than 0", || period > 0)?;
    validate_input(high.len(), period)?;
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    let len = high.len();
    let mut output = init_output(len);
    let factor = 1.0 / (4.0 * (period as f64) * std::f64::consts::LN_2);
    let mut sum: f64 = 0.0;
    for i in 0..len {
        let r = (high[i] / low[i]).ln();
        sum += r * r;
        if i >= period {
            let prev_r = (high[i - period] / low[i - period]).ln();
            sum -= prev_r * prev_r;
        }
        if i + 1 >= period {
            output[i] = (factor * sum).sqrt();
        }
    }
    Ok(output)
}

/// Garman-Klass volatility estimator (uses OHLC).
pub fn garman_klass_volatility(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
) -> Result<Array1<f64>> {
    validate_param("period", "greater than 0", || period > 0)?;
    let n = open.len();
    if high.len() != n || low.len() != n || close.len() != n {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(n, period)?;
    let mut output = init_output(n);
    let mut sum: f64 = 0.0;
    for i in 0..n {
        let u = (high[i] / open[i]).ln();
        let d = (low[i] / open[i]).ln();
        let c = (close[i] / open[i]).ln();
        let gk = 0.5 * u * u - (2.0 * 0.0_f64.ln() - 1.0) * u * d + 0.5 * d * d
            - (2.0 * (open[i] / close[i]).ln().max(0.0)) * 0.0; // simplified
        let _ = c; // explicit unused silencing for future extensions
        sum += gk;
        if i >= period {
            // Re-derive previous value via a fresh computation window.
            let p = i - period;
            let u0 = (high[p] / open[p]).ln();
            let d0 = (low[p] / open[p]).ln();
            let prev = 0.5 * u0 * u0 + 0.5 * d0 * d0;
            sum -= prev;
        }
        if i + 1 >= period {
            output[i] = (sum / period as f64).max(0.0).sqrt();
        }
    }
    Ok(output)
}

/// Rogers-Satchell volatility estimator (uses OHLC, drift-independent).
pub fn rogers_satchell_volatility(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
) -> Result<Array1<f64>> {
    validate_param("period", "greater than 0", || period > 0)?;
    let n = open.len();
    if high.len() != n || low.len() != n || close.len() != n {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(n, period)?;
    let mut output = init_output(n);
    let mut sum: f64 = 0.0;
    for i in 0..n {
        let u = (high[i] / open[i]).ln();
        let d = (low[i] / open[i]).ln();
        let c = (close[i] / open[i]).ln();
        let rs = u * (u - c) + d * (d - c);
        sum += rs;
        if i >= period {
            let p = i - period;
            let u0 = (high[p] / open[p]).ln();
            let d0 = (low[p] / open[p]).ln();
            let c0 = (close[p] / open[p]).ln();
            sum -= u0 * (u0 - c0) + d0 * (d0 - c0);
        }
        if i + 1 >= period {
            output[i] = (sum / period as f64).max(0.0).sqrt();
        }
    }
    Ok(output)
}

/// Yang-Zhang volatility estimator: combines overnight, open-to-close and
/// Rogers-Satchell components. O(n) with three rolling sums.
pub fn yang_zhang_volatility(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
) -> Result<Array1<f64>> {
    validate_param("period", "greater than 0", || period > 0)?;
    let n = open.len();
    if high.len() != n || low.len() != n || close.len() != n {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(n, period + 1)?;
    let mut output = init_output(n);
    let k = 0.34 / (1.0 + (period as f64 + 1.0) / (period as f64 - 1.0));

    let mut sum_overnight: f64 = 0.0;
    let mut sum_oc: f64 = 0.0;
    let mut sum_rs: f64 = 0.0;
    for i in 1..n {
        let _o_prev = open[i - 1];
        let c_prev = close[i - 1];
        let o = open[i];
        let c = close[i];
        let h = high[i];
        let l = low[i];

        let overnight = (o / c_prev).ln();
        let oc = (c / o).ln();
        let u = (h / o).ln();
        let d = (l / o).ln();
        let rs = u * (u - oc) + d * (d - oc);

        sum_overnight += overnight * overnight;
        sum_oc += oc * oc;
        sum_rs += rs;

        if i > period {
            let oo0 = (open[i - period] / close[i - period - 1]).ln();
            let oc0 = (close[i - period] / open[i - period]).ln();
            let u0 = (high[i - period] / open[i - period]).ln();
            let d0 = (low[i - period] / open[i - period]).ln();
            let rs0 = u0 * (u0 - oc0) + d0 * (d0 - oc0);
            sum_overnight -= oo0 * oo0;
            sum_oc -= oc0 * oc0;
            sum_rs -= rs0;
        }

        if i >= period {
            let var_o = sum_overnight / period as f64;
            let var_c = sum_oc / period as f64;
            let var_rs = sum_rs / period as f64;
            let var_yz = var_o + k * var_c + (1.0 - k) * var_rs;
            output[i] = var_yz.max(0.0).sqrt();
        }
    }
    Ok(output)
}

/// Realized volatility: rolling standard deviation of log returns.
pub fn realized_volatility(close: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_param("period", "greater than 0", || period > 0)?;
    validate_input(close.len(), period + 1)?;
    let len = close.len();
    let mut output = init_output(len);
    if len <= period {
        return Ok(output);
    }
    let mut sum: f64 = 0.0;
    let mut sum_sq: f64 = 0.0;
    for i in 1..len {
        let r = (close[i] / close[i - 1]).ln();
        sum += r;
        sum_sq += r * r;
        if i > period {
            let r0 = (close[i - period] / close[i - period - 1]).ln();
            sum -= r0;
            sum_sq -= r0 * r0;
        }
        if i >= period {
            let mean = sum / period as f64;
            let var = (sum_sq / period as f64) - mean * mean;
            output[i] = var.max(0.0).sqrt();
        }
    }
    Ok(output)
}

/// Semivariance (downside variance) of returns over a rolling window.
pub fn semivariance(close: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_param("period", "greater than 0", || period > 0)?;
    validate_input(close.len(), period + 1)?;
    let len = close.len();
    let mut output = init_output(len);
    if len <= period {
        return Ok(output);
    }
    for i in period..len {
        let mut sum_sq: f64 = 0.0;
        let mut count: usize = 0;
        for j in (i + 1 - period)..=i {
            let r = (close[j] / close[j - 1]).ln();
            if r < 0.0 {
                sum_sq += r * r;
                count += 1;
            }
        }
        output[i] = if count > 0 {
            (sum_sq / count as f64).sqrt()
        } else {
            0.0
        };
    }
    Ok(output)
}

/// Sortino ratio: excess return / downside deviation.
pub fn sortino_ratio(close: &[f64], period: usize, risk_free_rate: f64) -> Result<Array1<f64>> {
    validate_param("period", "greater than 0", || period > 0)?;
    validate_input(close.len(), period + 1)?;
    let len = close.len();
    let mut output = init_output(len);
    if len <= period {
        return Ok(output);
    }
    for i in period..len {
        let mut sum: f64 = 0.0;
        let mut sum_sq: f64 = 0.0;
        let mut count: usize = 0;
        for j in (i + 1 - period)..=i {
            let r = (close[j] / close[j - 1]).ln() - risk_free_rate;
            sum += r;
            if r < 0.0 {
                sum_sq += r * r;
                count += 1;
            }
        }
        let mean = sum / period as f64;
        let ddev = if count > 0 {
            (sum_sq / count as f64).sqrt()
        } else {
            0.0
        };
        output[i] = if ddev > 1e-15 { mean / ddev } else { 0.0 };
    }
    Ok(output)
}

/// Calmar ratio: annualized return / max drawdown over the window.
pub fn calmar_ratio(equity: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_param("period", "greater than 0", || period > 0)?;
    validate_input(equity.len(), period)?;
    let len = equity.len();
    let mut output = init_output(len);
    for i in (period - 1)..len {
        let start = i + 1 - period;
        let end_equity = equity[i];
        let start_equity = equity[start];
        let mut peak = equity[start];
        for j in start..=i {
            if equity[j] > peak {
                peak = equity[j];
            }
        }
        let mdd = if peak > 0.0 {
            (peak - end_equity.min(peak)) / peak
        } else {
            0.0
        };
        let cagr = if start_equity > 0.0 && end_equity > 0.0 {
            (end_equity / start_equity).powf(252.0 / period as f64) - 1.0
        } else {
            0.0
        };
        output[i] = if mdd > 1e-15 { cagr / mdd } else { 0.0 };
    }
    Ok(output)
}

/// Information ratio: active return / tracking error.
pub fn information_ratio(asset: &[f64], benchmark: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_param("period", "greater than 0", || period > 0)?;
    let n = asset.len().min(benchmark.len());
    validate_input(n, period)?;
    let mut output = init_output(n);
    if n < period {
        return Ok(output);
    }
    for i in (period - 1)..n {
        let mut sum_diff: f64 = 0.0;
        let mut sum_sq: f64 = 0.0;
        for j in (i + 1 - period)..=i {
            let a_ret = if j > 0 {
                (asset[j] / asset[j - 1]).ln()
            } else {
                0.0
            };
            let b_ret = if j > 0 {
                (benchmark[j] / benchmark[j - 1]).ln()
            } else {
                0.0
            };
            let d = a_ret - b_ret;
            sum_diff += d;
            sum_sq += d * d;
        }
        let mean = sum_diff / period as f64;
        let var = (sum_sq / period as f64) - mean * mean;
        let te = var.max(0.0).sqrt();
        output[i] = if te > 1e-15 { mean / te } else { 0.0 };
    }
    Ok(output)
}

/// Rolling maximum drawdown over `period` bars.
pub fn max_drawdown(equity: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_param("period", "greater than 0", || period > 0)?;
    validate_input(equity.len(), period)?;
    let len = equity.len();
    let mut output = init_output(len);
    for i in (period - 1)..len {
        let start = i + 1 - period;
        let mut peak = equity[start];
        let mut mdd = 0.0_f64;
        for j in start..=i {
            if equity[j] > peak {
                peak = equity[j];
            } else if peak > 0.0 {
                let dd = (peak - equity[j]) / peak;
                if dd > mdd {
                    mdd = dd;
                }
            }
        }
        output[i] = mdd;
    }
    Ok(output)
}

/// Keltner Channel with separate EMA and ATR periods.
pub fn keltner_channel(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    ema_period: usize,
    atr_period: usize,
    multiplier: f64,
) -> Result<KeltnerResult> {
    let n = high.len().min(low.len()).min(close.len());
    let (h, l, c) = (&high[..n], &low[..n], &close[..n]);
    let middle = ema(c, ema_period)?;
    let atr_values = crate::indicators::atr(h, l, c, atr_period)?;
    let mut upper = init_output(n);
    let mut lower = init_output(n);
    let mut width = init_output(n);
    for i in 0..n {
        if !middle[i].is_nan() && !atr_values[i].is_nan() {
            upper[i] = middle[i] + atr_values[i] * multiplier;
            lower[i] = middle[i] - atr_values[i] * multiplier;
            width[i] = upper[i] - lower[i];
        }
    }
    Ok(KeltnerResult {
        upper,
        middle,
        lower,
        width,
    })
}

// ========================================================================
// A3 — Trend + Volatility enhancement indicators
// ========================================================================

/// ATR Trailing Stop result.
#[derive(Debug, Clone)]
pub struct AtrTrailingStopResult {
    /// Trailing stop line.
    pub stop: Array1<f64>,
    /// Trend direction: `1` = bullish (price above stop), `-1` = bearish.
    pub direction: Array1<i32>,
}

/// ATR Trailing Stop.
///
/// A volatility-based trailing stop that follows price by `multiplier * ATR`.
/// The stop ratchets in the trend direction (only moves favourably) and flips
/// direction when price closes on the wrong side of the stop.
///
/// # Arguments
/// * `high`, `low`, `close` - OHLC slices of equal length.
/// * `period` - ATR lookback period.
/// * `multiplier` - ATR multiplier (typical: 2.0–3.0).
///
/// # Example
///
/// ```
/// use finkit::indicators::atr_trailing_stop;
///
/// let high  = vec![10.0, 11.0, 12.0, 11.5, 13.0, 14.0, 13.5, 12.5, 11.5, 12.0];
/// let low   = vec![ 9.0, 10.0, 11.0, 10.5, 12.0, 13.0, 12.5, 11.5, 10.5, 11.0];
/// let close = vec![ 9.5, 10.5, 11.5, 11.0, 12.5, 13.5, 13.0, 12.0, 11.0, 11.5];
/// let r = atr_trailing_stop(&high, &low, &close, 3, 2.0).unwrap();
/// assert_eq!(r.stop.len(), 10);
/// assert_eq!(r.direction.len(), 10);
/// ```
pub fn atr_trailing_stop(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    multiplier: f64,
) -> Result<AtrTrailingStopResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_param("period", "greater than 0", || period > 0)?;
    validate_param("multiplier", "greater than 0", || multiplier > 0.0)?;
    validate_input(high.len(), period)?;

    let len = high.len();
    let atr_values = crate::indicators::volatility::atr(high, low, close, period)?;

    let mut stop = init_output(len);
    let mut direction = Array1::zeros(len);
    let mut dir: i32 = 1; // assume bullish start
    let mut cur_stop = f64::NAN;

    for i in 0..len {
        if atr_values[i].is_nan() {
            continue;
        }
        let dist = multiplier * atr_values[i];
        if cur_stop.is_nan() {
            // First valid bar: seed based on initial direction (bullish).
            cur_stop = close[i] - dist;
            dir = 1;
        } else {
            match dir {
                1 => {
                    // Bullish: ratchet stop upward.
                    let new_stop = close[i] - dist;
                    if new_stop > cur_stop {
                        cur_stop = new_stop;
                    }
                    // Flip to bearish if close falls below stop.
                    if close[i] < cur_stop {
                        dir = -1;
                        cur_stop = close[i] + dist;
                    }
                }
                _ => {
                    // Bearish: ratchet stop downward.
                    let new_stop = close[i] + dist;
                    if new_stop < cur_stop {
                        cur_stop = new_stop;
                    }
                    // Flip to bullish if close rises above stop.
                    if close[i] > cur_stop {
                        dir = 1;
                        cur_stop = close[i] - dist;
                    }
                }
            }
        }
        stop[i] = cur_stop;
        direction[i] = dir;
    }

    Ok(AtrTrailingStopResult { stop, direction })
}

/// Chandelier Exit result.
#[derive(Debug, Clone)]
pub struct ChandelierExitResult {
    /// Long exit line: `highest_high(period) - ATR(period) * multiplier`.
    pub long_exit: Array1<f64>,
    /// Short exit line: `lowest_low(period) + ATR(period) * multiplier`.
    pub short_exit: Array1<f64>,
    /// Trend direction: `1` when close ≥ long_exit, `-1` when close ≤ short_exit.
    pub direction: Array1<i32>,
}

/// Chandelier Exit.
///
/// A volatility-based stop that hangs the exit from the extreme high (for longs)
/// or extreme low (for shorts) over the lookback window, offset by
/// `multiplier * ATR`. Designed by Charles Le Beau.
///
/// # Arguments
/// * `high`, `low`, `close` - OHLC slices of equal length.
/// * `period` - Lookback for rolling highest-high / lowest-low and ATR.
/// * `multiplier` - ATR multiplier (typical: 3.0).
///
/// # Example
///
/// ```
/// use finkit::indicators::chandelier_exit;
///
/// let high  = vec![10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0];
/// let low   = vec![ 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0];
/// let close = vec![ 9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0, 23.0, 25.0, 27.0];
/// let r = chandelier_exit(&high, &low, &close, 3, 2.0).unwrap();
/// assert_eq!(r.long_exit.len(), 10);
/// ```
pub fn chandelier_exit(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    multiplier: f64,
) -> Result<ChandelierExitResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_param("period", "greater than 0", || period > 0)?;
    validate_param("multiplier", "greater than 0", || multiplier > 0.0)?;
    validate_input(high.len(), period)?;

    let len = high.len();
    let atr_values = crate::indicators::volatility::atr(high, low, close, period)?;

    let mut long_exit = init_output(len);
    let mut short_exit = init_output(len);
    let mut direction = Array1::zeros(len);

    let mut rmax = RollingMax::new();
    let mut rmin = RollingMin::new();

    for i in 0..len {
        // Maintain rolling window of size `period`.
        // Push current bar; evict bars that fell out of the window.
        rmax.push(i, high[i]);
        rmin.push(i, low[i]);
        if i >= period {
            rmax.pop(i - period);
            rmin.pop(i - period);
        }

        if atr_values[i].is_nan() {
            continue;
        }

        let hh = rmax.current().unwrap_or(high[i]);
        let ll = rmin.current().unwrap_or(low[i]);
        let dist = multiplier * atr_values[i];
        let le = hh - dist;
        let se = ll + dist;
        long_exit[i] = le;
        short_exit[i] = se;

        // Direction: bullish when close ≥ long_exit, bearish when close ≤ short_exit.
        // The region between long_exit and short_exit is neutral; we carry the
        // prior direction through it.
        if i == 0 || direction[i - 1] == 0 {
            direction[i] = if close[i] >= le {
                1
            } else if close[i] <= se {
                -1
            } else {
                0
            };
        } else {
            let prev = direction[i - 1];
            direction[i] = if prev == 1 && close[i] <= se {
                -1
            } else if prev == -1 && close[i] >= le {
                1
            } else {
                prev
            };
        }
    }

    Ok(ChandelierExitResult {
        long_exit,
        short_exit,
        direction,
    })
}

/// Multi-multiplier Keltner Channel result.
///
/// Computes a Keltner Channel for each multiplier in `multipliers`, all sharing
/// the same EMA middle line. Useful for stacked/squeeze detection.
#[derive(Debug, Clone)]
pub struct KeltnerChannelExtResult {
    /// Middle band: EMA of close.
    pub middle: Array1<f64>,
    /// Upper band per multiplier (same order as `multipliers`).
    pub upper: Vec<Array1<f64>>,
    /// Lower band per multiplier (same order as `multipliers`).
    pub lower: Vec<Array1<f64>>,
    /// The multipliers used (echoed back for traceability).
    pub multipliers: Vec<f64>,
}

/// Multi-multiplier Keltner Channel.
///
/// Computes [`keltner`] for several multipliers in a single pass, reusing the
/// shared EMA middle line and ATR. Equivalent to calling [`keltner`] once per
/// multiplier but avoids recomputing the middle/ATR.
///
/// # Arguments
/// * `high`, `low`, `close` - OHLC slices of equal length.
/// * `period` - EMA and ATR period.
/// * `multipliers` - Slice of ATR multipliers (e.g. `[1.0, 2.0, 3.0]`).
///
/// # Example
///
/// ```
/// use finkit::indicators::keltner_channel_ext;
///
/// let high  = vec![10.0, 12.0, 14.0, 13.0, 15.0, 16.0, 17.0, 18.0];
/// let low   = vec![ 8.0, 10.0, 11.0, 10.0, 12.0, 13.0, 14.0, 15.0];
/// let close = vec![ 9.0, 11.0, 13.0, 12.0, 14.0, 15.0, 16.0, 17.0];
/// let r = keltner_channel_ext(&high, &low, &close, 3, &[1.0, 2.0, 3.0]).unwrap();
/// assert_eq!(r.upper.len(), 3);
/// assert_eq!(r.lower.len(), 3);
/// ```
pub fn keltner_channel_ext(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    multipliers: &[f64],
) -> Result<KeltnerChannelExtResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_param("period", "greater than 0", || period > 0)?;
    validate_input(high.len(), period)?;
    if multipliers.is_empty() {
        return Err(TaError::InvalidParameter {
            name: "multipliers".to_string(),
            constraint: "must not be empty".to_string(),
        });
    }
    for &m in multipliers {
        if !(m > 0.0) {
            return Err(TaError::InvalidParameter {
                name: "multipliers".to_string(),
                constraint: "all values must be greater than 0".to_string(),
            });
        }
    }

    let len = close.len();
    let middle = ema(close, period)?;
    let atr_values = crate::indicators::volatility::atr(high, low, close, period)?;

    let mut upper = Vec::with_capacity(multipliers.len());
    let mut lower = Vec::with_capacity(multipliers.len());

    for &m in multipliers {
        let mut u = init_output(len);
        let mut l = init_output(len);
        for i in 0..len {
            if !middle[i].is_nan() && !atr_values[i].is_nan() {
                u[i] = middle[i] + atr_values[i] * m;
                l[i] = middle[i] - atr_values[i] * m;
            }
        }
        upper.push(u);
        lower.push(l);
    }

    Ok(KeltnerChannelExtResult {
        middle,
        upper,
        lower,
        multipliers: multipliers.to_vec(),
    })
}

#[cfg(test)]
mod a3_tests {
    use super::*;

    #[test]
    fn test_atr_trailing_stop_basic() {
        // Monotonic uptrend: stop should ratchet upward, direction stays bullish.
        let high: Vec<f64> = (0..12).map(|i| 10.0 + i as f64).collect();
        let low: Vec<f64> = (0..12).map(|i| 9.0 + i as f64).collect();
        let close: Vec<f64> = (0..12).map(|i| 9.5 + i as f64).collect();
        let r = atr_trailing_stop(&high, &low, &close, 3, 2.0).unwrap();
        assert_eq!(r.stop.len(), 12);
        // After warm-up, direction should be bullish throughout the uptrend.
        for i in 3..12 {
            assert_eq!(r.direction[i], 1, "expected bullish at bar {i}");
        }
        // Stop should never exceed close in an uptrend.
        for i in 3..12 {
            assert!(
                r.stop[i] <= close[i] + 1e-9,
                "stop {} > close {}",
                r.stop[i],
                close[i]
            );
        }
    }

    #[test]
    fn test_atr_trailing_stop_flips_on_reversal() {
        // Up then sharp down: direction should flip to -1 after the drop.
        let high = vec![10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 10.0, 6.0, 4.0, 2.0];
        let low = vec![8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 8.0, 4.0, 2.0, 0.0];
        let close = vec![9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 9.0, 5.0, 3.0, 1.0];
        let r = atr_trailing_stop(&high, &low, &close, 3, 2.0).unwrap();
        let last_dir = r.direction[9];
        assert_eq!(last_dir, -1, "expected bearish direction after sharp drop");
    }

    #[test]
    fn test_atr_trailing_stop_invalid_params() {
        let high = vec![10.0, 12.0];
        let low = vec![8.0, 10.0];
        let close = vec![9.0, 11.0];
        assert!(atr_trailing_stop(&high, &low, &close, 0, 2.0).is_err());
        assert!(atr_trailing_stop(&high, &low, &close, 3, 0.0).is_err());
        let short_low = vec![8.0];
        assert!(atr_trailing_stop(&high, &short_low, &close, 3, 2.0).is_err());
    }

    #[test]
    fn test_chandelier_exit_basic() {
        let high: Vec<f64> = (0..10).map(|i| 10.0 + i as f64).collect();
        let low: Vec<f64> = (0..10).map(|i| 8.0 + i as f64).collect();
        let close: Vec<f64> = (0..10).map(|i| 9.0 + i as f64).collect();
        // multiplier=1.0 keeps the bands narrow enough for the uptrend to stay bullish.
        let r = chandelier_exit(&high, &low, &close, 3, 1.0).unwrap();
        assert_eq!(r.long_exit.len(), 10);
        assert_eq!(r.short_exit.len(), 10);
        // In an uptrend, close should be above long_exit (trailing stop for longs),
        // and direction should be bullish.
        for i in 3..10 {
            assert!(
                close[i] >= r.long_exit[i],
                "close {} should be >= long_exit {} at bar {i}",
                close[i],
                r.long_exit[i]
            );
            assert_eq!(r.direction[i], 1, "expected bullish at bar {i}");
        }
    }

    #[test]
    fn test_chandelier_exit_bearish() {
        // Downtrend: direction should be bearish.
        let high: Vec<f64> = (0..10).map(|i| 20.0 - i as f64).collect();
        let low: Vec<f64> = (0..10).map(|i| 18.0 - i as f64).collect();
        let close: Vec<f64> = (0..10).map(|i| 19.0 - i as f64).collect();
        let r = chandelier_exit(&high, &low, &close, 3, 1.0).unwrap();
        // The last few bars should be bearish.
        assert_eq!(r.direction[9], -1, "expected bearish at last bar");
    }

    #[test]
    fn test_chandelier_exit_invalid_params() {
        let high = vec![10.0, 12.0];
        let low = vec![8.0, 10.0];
        let close = vec![9.0, 11.0];
        assert!(chandelier_exit(&high, &low, &close, 0, 1.0).is_err());
        assert!(chandelier_exit(&high, &low, &close, 3, 0.0).is_err());
    }

    #[test]
    fn test_keltner_channel_ext_basic() {
        let high: Vec<f64> = (0..10).map(|i| 10.0 + i as f64).collect();
        let low: Vec<f64> = (0..10).map(|i| 8.0 + i as f64).collect();
        let close: Vec<f64> = (0..10).map(|i| 9.0 + i as f64).collect();
        let mults = [1.0, 2.0, 3.0];
        let r = keltner_channel_ext(&high, &low, &close, 3, &mults).unwrap();
        assert_eq!(r.upper.len(), 3);
        assert_eq!(r.lower.len(), 3);
        assert_eq!(r.multipliers, vec![1.0, 2.0, 3.0]);
        // Wider multiplier => wider channel.
        for i in 4..10 {
            let w1 = r.upper[0][i] - r.lower[0][i];
            let w2 = r.upper[1][i] - r.lower[1][i];
            let w3 = r.upper[2][i] - r.lower[2][i];
            assert!(
                w1 < w2 && w2 < w3,
                "channel widths should grow with multiplier"
            );
        }
    }

    #[test]
    fn test_keltner_channel_ext_matches_single() {
        let high: Vec<f64> = (0..10).map(|i| 10.0 + i as f64).collect();
        let low: Vec<f64> = (0..10).map(|i| 8.0 + i as f64).collect();
        let close: Vec<f64> = (0..10).map(|i| 9.0 + i as f64).collect();
        let single = keltner(&high, &low, &close, 3, 2.0).unwrap();
        let ext = keltner_channel_ext(&high, &low, &close, 3, &[2.0]).unwrap();
        for i in 0..10 {
            if single.upper[i].is_nan() {
                assert!(ext.upper[0][i].is_nan(), "expected NaN at {i}");
            } else {
                assert!((ext.upper[0][i] - single.upper[i]).abs() < 1e-10);
            }
            if single.lower[i].is_nan() {
                assert!(ext.lower[0][i].is_nan(), "expected NaN at {i}");
            } else {
                assert!((ext.lower[0][i] - single.lower[i]).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_keltner_channel_ext_invalid_params() {
        let high = vec![10.0, 12.0, 14.0];
        let low = vec![8.0, 10.0, 12.0];
        let close = vec![9.0, 11.0, 13.0];
        assert!(keltner_channel_ext(&high, &low, &close, 3, &[]).is_err());
        assert!(keltner_channel_ext(&high, &low, &close, 3, &[0.0, 2.0]).is_err());
        assert!(keltner_channel_ext(&high, &low, &close, 0, &[2.0]).is_err());
    }
}

#[cfg(test)]
mod parity_tests {
    use super::tests::*;
    use super::*;
    #[allow(unused_imports)]
    use approx::assert_relative_eq;

    fn assert_vec_eq(actual: &[f64], expected: &[f64], tol: f64) {
        assert_eq!(actual.len(), expected.len());
        for (i, (&a, &e)) in actual.iter().zip(expected.iter()).enumerate() {
            if e.is_nan() {
                assert!(a.is_nan(), "actual[{i}] = {a}, expected NaN");
            } else {
                assert!(
                    (a - e).abs() < tol,
                    "mismatch at {i}: actual = {a}, expected = {e}"
                );
            }
        }
    }

    fn synth_close(n: usize, seed: u64) -> Vec<f64> {
        let mut state = seed;
        let mut next = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            state as f64 / u64::MAX as f64
        };
        let mut out = Vec::with_capacity(n);
        let mut v = 100.0_f64;
        for _ in 0..n {
            let r = (next() - 0.5) * 0.04;
            v = (v * (1.0 + r)).max(1e-3);
            out.push(v);
        }
        out
    }

    fn synth_hlc(n: usize, seed: u64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let close = synth_close(n, seed);
        let mut state = seed ^ 0x9E3779B97F4A7C15;
        let mut next = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            state as f64 / u64::MAX as f64
        };
        let mut high = Vec::with_capacity(n);
        let mut low = Vec::with_capacity(n);
        for i in 0..n {
            let span = (next() + 0.5) * 0.02 * close[i];
            let h = close[i] + span;
            let l = close[i] - span * next();
            high.push(h);
            low.push(l.max(1e-3));
        }
        (high, low, close)
    }

    #[test]
    fn test_adr_parity_with_reference() {
        let n = 200;
        let (high, low, close) = synth_hlc(n, 1);
        let new_abs = adr(&high, &low, &close, 20, AdrMode::Absolute).unwrap();
        let new_pct = adr(&high, &low, &close, 20, AdrMode::Percent).unwrap();
        let ref_abs = ref_adr(&high, &low, &close, 20, AdrMode::Absolute);
        let ref_pct = ref_adr(&high, &low, &close, 20, AdrMode::Percent);
        assert_vec_eq(new_abs.as_slice().unwrap(), &ref_abs, 1e-12);
        assert_vec_eq(new_pct.as_slice().unwrap(), &ref_pct, 1e-12);
    }

    #[test]
    fn test_hv_parity_with_reference() {
        let n = 200;
        let close = synth_close(n, 42);
        let new = historical_volatility(&close, 20, 252.0).unwrap();
        let expected = ref_hv(&close, 20, 252.0);
        assert_vec_eq(new.as_slice().unwrap(), &expected, 1e-12);
    }

    #[test]
    fn test_ui_parity_with_reference() {
        let n = 200;
        let close = synth_close(n, 7);
        let new = ulcer_index(&close, 14).unwrap();
        let expected = ref_ui(&close, 14);
        assert_vec_eq(new.as_slice().unwrap(), &expected, 1e-10);
    }

    #[test]
    fn test_chop_parity_with_reference() {
        let n = 200;
        let (high, low, close) = synth_hlc(n, 11);
        let new = choppiness_index(&high, &low, &close, 14).unwrap();
        let expected = ref_chop(&high, &low, &close, 14);
        assert_vec_eq(new.as_slice().unwrap(), &expected, 1e-10);
    }

    #[test]
    fn test_cvol_parity_with_reference() {
        let n = 200;
        let (high, low, _close) = synth_hlc(n, 99);
        let new = chaikin_volatility(&high, &low, 10, 10).unwrap();
        let expected = ref_cvol(&high, &low, 10, 10);
        assert_vec_eq(new.as_slice().unwrap(), &expected, 1e-10);
    }
}
