//! TA-Lib 0.7.1-compatible rolling statistics for the public compatibility path.
//!
//! The installed-wheel release gate currently benchmarks against TA-Lib core
//! 0.7.1. That release uses raw rolling sums for VAR/STDDEV/CORREL and a
//! precomputed-SMA specialization for BBANDS. The operation order below mirrors
//! those C loops deliberately: changing add/remove order or replacing division
//! with multiplication by a reciprocal is enough to create long-series parity
//! drift.

use crate::error::{Result, TaError};

const TA_EPSILON: f64 = 0.00000000000001;

#[inline]
fn validate_period(len: usize, period: usize, minimum: usize) -> Result<()> {
    if period < minimum {
        return Err(TaError::InvalidParameter {
            name: "timeperiod".to_string(),
            constraint: format!("at least {minimum}"),
        });
    }
    if len < period {
        return Err(TaError::InsufficientData {
            length: len,
            required: period,
        });
    }
    Ok(())
}

#[inline]
fn is_zero_or_negative(value: f64) -> bool {
    value < TA_EPSILON
}

/// Population variance with the exact rolling update order used by TA_VAR 0.7.1.
pub fn variance(input: &[f64], period: usize) -> Result<Vec<f64>> {
    validate_period(input.len(), period, 1)?;

    let lookback = period - 1;
    let mut output = vec![f64::NAN; input.len()];
    let mut period_total1 = 0.0;
    let mut period_total2 = 0.0;
    let mut trailing_idx = 0usize;
    let mut i = trailing_idx;

    if period > 1 {
        while i < lookback {
            let mut temp_real = input[i];
            i += 1;
            period_total1 += temp_real;
            temp_real *= temp_real;
            period_total2 += temp_real;
        }
    }

    while i < input.len() {
        let mut temp_real = input[i];
        i += 1;
        period_total1 += temp_real;
        temp_real *= temp_real;
        period_total2 += temp_real;

        let mean_value1 = period_total1 / period as f64;
        let mean_value2 = period_total2 / period as f64;

        temp_real = input[trailing_idx];
        trailing_idx += 1;
        period_total1 -= temp_real;
        temp_real *= temp_real;
        period_total2 -= temp_real;

        output[i - 1] = mean_value2 - mean_value1 * mean_value1;
    }

    Ok(output)
}

/// Standard deviation as TA_STDDEV 0.7.1: VAR followed by guarded sqrt/scale.
pub fn stddev(input: &[f64], period: usize, nb_dev: f64) -> Result<Vec<f64>> {
    validate_period(input.len(), period, 2)?;
    let mut output = variance(input, period)?;

    if nb_dev != 1.0 {
        for value in output.iter_mut().skip(period - 1) {
            let temp_real = *value;
            *value = if !is_zero_or_negative(temp_real) {
                temp_real.sqrt() * nb_dev
            } else {
                0.0
            };
        }
    } else {
        for value in output.iter_mut().skip(period - 1) {
            let temp_real = *value;
            *value = if !is_zero_or_negative(temp_real) {
                temp_real.sqrt()
            } else {
                0.0
            };
        }
    }

    Ok(output)
}

/// Pearson correlation with the exact add/remove sequencing of TA_CORREL 0.7.1.
pub fn correlation(input_a: &[f64], input_b: &[f64], period: usize) -> Result<Vec<f64>> {
    if input_a.len() != input_b.len() {
        return Err(TaError::InvalidParameter {
            name: "input_a and input_b".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_period(input_a.len(), period, 1)?;

    let lookback = period - 1;
    let mut output = vec![f64::NAN; input_a.len()];
    let mut trailing_idx = 0usize;
    let mut sum_y2 = 0.0;
    let mut sum_x2 = sum_y2;
    let mut sum_y = sum_x2;
    let mut sum_x = sum_y;
    let mut sum_xy = sum_x;

    let mut today = trailing_idx;
    while today <= lookback {
        let x = input_a[today];
        sum_x += x;
        sum_x2 += x * x;
        let y = input_b[today];
        sum_xy += x * y;
        sum_y += y;
        sum_y2 += y * y;
        today += 1;
    }

    let mut trailing_x = input_a[trailing_idx];
    let mut trailing_y = input_b[trailing_idx];
    trailing_idx += 1;

    let mut temp_real =
        (sum_x2 - sum_x * sum_x / period as f64) * (sum_y2 - sum_y * sum_y / period as f64);
    output[lookback] = if !is_zero_or_negative(temp_real) {
        (sum_xy - sum_x * sum_y / period as f64) / temp_real.sqrt()
    } else {
        0.0
    };

    while today < input_a.len() {
        sum_x -= trailing_x;
        sum_x2 -= trailing_x * trailing_x;
        sum_xy -= trailing_x * trailing_y;
        sum_y -= trailing_y;
        sum_y2 -= trailing_y * trailing_y;

        let x = input_a[today];
        sum_x += x;
        sum_x2 += x * x;
        let y = input_b[today];
        today += 1;
        sum_xy += x * y;
        sum_y += y;
        sum_y2 += y * y;

        trailing_x = input_a[trailing_idx];
        trailing_y = input_b[trailing_idx];
        trailing_idx += 1;

        temp_real =
            (sum_x2 - sum_x * sum_x / period as f64) * (sum_y2 - sum_y * sum_y / period as f64);
        output[today - 1] = if !is_zero_or_negative(temp_real) {
            (sum_xy - sum_x * sum_y / period as f64) / temp_real.sqrt()
        } else {
            0.0
        };
    }

    Ok(output)
}

/// SMA Bollinger Bands matching the TA_BBANDS 0.7.1 SMA specialization.
///
/// TA-Lib computes SMA and the rolling square sum in two helper passes. The
/// two accumulators are independent, so interleaving those updates preserves
/// each helper's exact add/remove order while halving the input traversal and
/// eliminating the hot-loop branch tree for deviation multipliers.
pub fn bbands_sma(
    input: &[f64],
    period: usize,
    nb_dev_up: f64,
    nb_dev_down: f64,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    validate_period(input.len(), period, 2)?;

    let len = input.len();
    let lookback = period - 1;
    let mut upper = Vec::with_capacity(len);
    let mut middle = Vec::with_capacity(len);
    let mut lower = Vec::with_capacity(len);
    upper.resize(lookback, f64::NAN);
    middle.resize(lookback, f64::NAN);
    lower.resize(lookback, f64::NAN);

    let mut period_total = 0.0;
    let mut period_total2 = 0.0;
    for &value in &input[..lookback] {
        period_total += value;
        let mut squared = value;
        squared *= squared;
        period_total2 += squared;
    }

    let period_f = period as f64;
    let mut trailing_idx = 0usize;
    for index in lookback..len {
        let current = input[index];

        // TA_INT_SMA update order.
        period_total += current;
        let middle_value = period_total / period_f;
        period_total -= input[trailing_idx];

        // TA_INT_stddev_using_precalc_ma update order.
        let mut squared = current;
        squared *= squared;
        period_total2 += squared;
        let mut variance = period_total2 / period_f;
        let mut outgoing_squared = input[trailing_idx];
        outgoing_squared *= outgoing_squared;
        period_total2 -= outgoing_squared;
        let mut middle_squared = middle_value;
        middle_squared *= middle_squared;
        variance -= middle_squared;

        let stddev = if !is_zero_or_negative(variance) {
            variance.sqrt()
        } else {
            0.0
        };

        // Multiplication by 1.0 is exact for finite IEEE-754 values, so this
        // branch-free form is numerically identical to TA-Lib's 1.0 special
        // cases while removing per-row parameter branches.
        upper.push(middle_value + stddev * nb_dev_up);
        middle.push(middle_value);
        lower.push(middle_value - stddev * nb_dev_down);
        trailing_idx += 1;
    }

    Ok((upper, middle, lower))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variance_matches_population_variance_for_first_window() {
        let input = [1.0, 2.0, 3.0, 4.0, 5.0];
        let result = variance(&input, 5).unwrap();
        assert!(result[..4].iter().all(|value| value.is_nan()));
        assert_eq!(result[4], 2.0);
    }

    #[test]
    fn stddev_guards_talib_negative_zero_band() {
        let input = vec![42.0; 64];
        let result = stddev(&input, 20, 1.0).unwrap();
        assert!(result[..19].iter().all(|value| value.is_nan()));
        assert!(result[19..].iter().all(|value| *value == 0.0));
    }

    #[test]
    fn correlation_first_window_is_one_for_affine_series() {
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        let y = [9.0, 11.0, 13.0, 15.0, 17.0];
        let result = correlation(&x, &y, 5).unwrap();
        assert!(result[..4].iter().all(|value| value.is_nan()));
        assert!((result[4] - 1.0).abs() < 1.0e-15);
    }

    #[test]
    fn bbands_middle_matches_talib_sma_sequence() {
        let input: Vec<f64> = (1..=128).map(|value| value as f64).collect();
        let (upper, middle, lower) = bbands_sma(&input, 20, 2.0, 2.0).unwrap();
        assert_eq!(upper.len(), input.len());
        assert_eq!(middle.len(), input.len());
        assert_eq!(lower.len(), input.len());
        assert!((middle[19] - 10.5).abs() < 1.0e-12);
        assert!((middle[127] - 118.5).abs() < 1.0e-12);
        assert!(upper[19] > middle[19]);
        assert!(lower[19] < middle[19]);
    }
}
