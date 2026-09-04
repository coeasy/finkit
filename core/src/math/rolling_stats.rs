//! Cancellation-resistant rolling statistics shared by indicator and Python hot paths.
//!
//! TA-Lib 0.7.x moved VAR/STDDEV/CORREL to shifted accumulators with periodic
//! re-anchoring. Keeping the same state evolution here prevents long-series
//! cancellation drift while retaining O(n) steady-state execution.

use crate::error::{Result, TaError};

const RESEED_WINDOWS: usize = 32;
const RESEED_RATIO: f64 = 1.0e-6;
const RESEED_OUTLIER_RATIO: f64 = 1.0e6;
const RESEED_FLOOR_RATIO: f64 = 1.0e-12;
const CORREL_FACTOR_EPSILON: f64 = 1.0e-14;

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

/// TA-Lib-compatible rolling population variance.
///
/// The running sums contain deviations from a nearby shift, not raw prices.
/// State is rebuilt when cancellation risk grows or every 32 windows, matching
/// the 2026 TA_VAR stability strategy.
pub fn variance(input: &[f64], period: usize) -> Result<Vec<f64>> {
    validate_period(input.len(), period, 1)?;

    let len = input.len();
    let mut output = vec![f64::NAN; len];
    let lookback = period - 1;
    let inv_period = 1.0 / period as f64;
    let mut trailing_idx = 0usize;
    let mut shift = input[trailing_idx];
    let mut total1 = 0.0;
    let mut total2 = 0.0;

    for &value in &input[trailing_idx..lookback] {
        let delta = value - shift;
        total1 += delta;
        total2 += delta * delta;
    }

    let mut bars_since_reseed = RESEED_WINDOWS.saturating_mul(period);
    for i in lookback..len {
        let mut temp = input[i] - shift;
        total1 += temp;
        total2 += temp * temp;
        let mean1 = total1 * inv_period;
        let mut current_variance = total2 * inv_period - mean1 * mean1;

        temp = input[trailing_idx] - shift;
        total1 -= temp;
        temp *= temp;
        total2 -= temp;
        trailing_idx += 1;

        bars_since_reseed = bars_since_reseed.saturating_sub(1);
        if current_variance < RESEED_RATIO * (total2 * inv_period)
            || temp > RESEED_OUTLIER_RATIO * total2
            || bars_since_reseed == 0
        {
            bars_since_reseed = RESEED_WINDOWS.saturating_mul(period);
            let window_start = i - lookback;

            let mut sum = 0.0;
            for &value in &input[window_start..=i] {
                sum += value;
            }
            shift = sum * inv_period;

            total1 = 0.0;
            total2 = 0.0;
            for &value in &input[window_start..=i] {
                let delta = value - shift;
                total1 += delta;
                total2 += delta * delta;
            }
            let mean1 = total1 * inv_period;
            current_variance = total2 * inv_period - mean1 * mean1;
            if current_variance < RESEED_FLOOR_RATIO * (total2 * inv_period) {
                current_variance = 0.0;
            }

            let trailing = input[window_start] - shift;
            total1 -= trailing;
            total2 -= trailing * trailing;
        }

        output[i] = current_variance;
    }

    Ok(output)
}

/// Rolling population standard deviation using the stable variance core.
pub fn stddev(input: &[f64], period: usize, nb_dev: f64) -> Result<Vec<f64>> {
    validate_period(input.len(), period, 2)?;
    let mut output = variance(input, period)?;
    for value in output.iter_mut().skip(period - 1) {
        *value = value.sqrt() * nb_dev;
    }
    Ok(output)
}

/// Cancellation-resistant rolling Pearson correlation matching TA-Lib 0.7.x.
pub fn correlation(input_a: &[f64], input_b: &[f64], period: usize) -> Result<Vec<f64>> {
    if input_a.len() != input_b.len() {
        return Err(TaError::InvalidParameter {
            name: "input_a and input_b".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_period(input_a.len(), period, 2)?;

    let len = input_a.len();
    let lookback = period - 1;
    let inv_period = 1.0 / period as f64;
    let mut output = vec![f64::NAN; len];
    let mut trailing_idx = 0usize;
    let mut shift_x = input_a[trailing_idx];
    let mut shift_y = input_b[trailing_idx];
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_xy = 0.0;
    let mut sum_x2 = 0.0;
    let mut sum_y2 = 0.0;

    for j in trailing_idx..lookback {
        let x = input_a[j] - shift_x;
        let y = input_b[j] - shift_y;
        sum_x += x;
        sum_y += y;
        sum_xy += x * y;
        sum_x2 += x * x;
        sum_y2 += y * y;
    }

    let mut bars_since_reseed = RESEED_WINDOWS.saturating_mul(period);
    let mut leaving_x = 0.0;
    let mut leaving_y = 0.0;

    for today in lookback..len {
        let x = input_a[today] - shift_x;
        let y = input_b[today] - shift_y;
        sum_x += x;
        sum_y += y;
        sum_xy += x * y;
        sum_x2 += x * x;
        sum_y2 += y * y;

        let mut ss_x = sum_x2 - sum_x * sum_x * inv_period;
        let mut ss_y = sum_y2 - sum_y * sum_y * inv_period;
        let mut sp_xy = sum_xy - sum_x * sum_y * inv_period;

        bars_since_reseed = bars_since_reseed.saturating_sub(1);
        if ss_x < RESEED_RATIO * sum_x2
            || ss_y < RESEED_RATIO * sum_y2
            || leaving_x > RESEED_OUTLIER_RATIO * sum_x2
            || leaving_y > RESEED_OUTLIER_RATIO * sum_y2
            || bars_since_reseed == 0
        {
            bars_since_reseed = RESEED_WINDOWS.saturating_mul(period);
            let window_start = today - lookback;

            let mut raw_x = 0.0;
            let mut raw_y = 0.0;
            for j in window_start..=today {
                raw_x += input_a[j];
                raw_y += input_b[j];
            }
            shift_x = raw_x * inv_period;
            shift_y = raw_y * inv_period;

            sum_x = 0.0;
            sum_y = 0.0;
            sum_xy = 0.0;
            sum_x2 = 0.0;
            sum_y2 = 0.0;
            for j in window_start..=today {
                let dx = input_a[j] - shift_x;
                let dy = input_b[j] - shift_y;
                sum_x += dx;
                sum_y += dy;
                sum_xy += dx * dy;
                sum_x2 += dx * dx;
                sum_y2 += dy * dy;
            }
            ss_x = sum_x2 - sum_x * sum_x * inv_period;
            ss_y = sum_y2 - sum_y * sum_y * inv_period;
            sp_xy = sum_xy - sum_x * sum_y * inv_period;
            if ss_x < 0.0 {
                ss_x = 0.0;
            }
            if ss_y < 0.0 {
                ss_y = 0.0;
            }
        }

        let trailing_x = input_a[trailing_idx] - shift_x;
        let trailing_y = input_b[trailing_idx] - shift_y;
        trailing_idx += 1;

        output[today] = if ss_x > CORREL_FACTOR_EPSILON * sum_x2
            && ss_y > CORREL_FACTOR_EPSILON * sum_y2
        {
            (sp_xy / (ss_x * ss_y).sqrt()).clamp(-1.0, 1.0)
        } else {
            0.0
        };

        leaving_x = trailing_x * trailing_x;
        leaving_y = trailing_y * trailing_y;
        sum_x -= trailing_x;
        sum_x2 -= leaving_x;
        sum_xy -= trailing_x * trailing_y;
        sum_y -= trailing_y;
        sum_y2 -= leaving_y;
    }

    Ok(output)
}

/// Fused SMA Bollinger Bands using the same stable variance recurrence as VAR.
pub fn bbands_sma(
    input: &[f64],
    period: usize,
    nb_dev_up: f64,
    nb_dev_down: f64,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    validate_period(input.len(), period, 2)?;

    let len = input.len();
    let lookback = period - 1;
    let inv_period = 1.0 / period as f64;
    let mut upper = vec![f64::NAN; len];
    let mut middle = vec![f64::NAN; len];
    let mut lower = vec![f64::NAN; len];

    let mut trailing_idx = 0usize;
    let mut shift = input[trailing_idx];
    let mut ma_total = 0.0;
    let mut var_total1 = 0.0;
    let mut var_total2 = 0.0;
    for &value in &input[trailing_idx..lookback] {
        ma_total += value;
        let delta = value - shift;
        var_total1 += delta;
        var_total2 += delta * delta;
    }

    let mut bars_since_reseed = RESEED_WINDOWS.saturating_mul(period);
    for i in lookback..len {
        ma_total += input[i];
        let mut temp = input[i] - shift;
        var_total1 += temp;
        var_total2 += temp * temp;
        let mean1 = var_total1 * inv_period;
        let mut current_variance = var_total2 * inv_period - mean1 * mean1;
        let current_middle = ma_total * inv_period;

        ma_total -= input[trailing_idx];
        temp = input[trailing_idx] - shift;
        var_total1 -= temp;
        temp *= temp;
        var_total2 -= temp;
        trailing_idx += 1;

        bars_since_reseed = bars_since_reseed.saturating_sub(1);
        if current_variance < RESEED_RATIO * (var_total2 * inv_period)
            || temp > RESEED_OUTLIER_RATIO * var_total2
            || bars_since_reseed == 0
        {
            bars_since_reseed = RESEED_WINDOWS.saturating_mul(period);
            let window_start = i - lookback;
            let mut raw_sum = 0.0;
            for &value in &input[window_start..=i] {
                raw_sum += value;
            }
            shift = raw_sum * inv_period;
            var_total1 = 0.0;
            var_total2 = 0.0;
            for &value in &input[window_start..=i] {
                let delta = value - shift;
                var_total1 += delta;
                var_total2 += delta * delta;
            }
            let mean1 = var_total1 * inv_period;
            current_variance = var_total2 * inv_period - mean1 * mean1;
            if current_variance < RESEED_FLOOR_RATIO * (var_total2 * inv_period) {
                current_variance = 0.0;
            }
            let trailing = input[window_start] - shift;
            var_total1 -= trailing;
            var_total2 -= trailing * trailing;
        }

        let deviation = if current_variance != 0.0 {
            current_variance.sqrt()
        } else {
            0.0
        };
        middle[i] = current_middle;
        upper[i] = current_middle + nb_dev_up * deviation;
        lower[i] = current_middle - nb_dev_down * deviation;
    }

    Ok((upper, middle, lower))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_variance_is_zero_after_warmup() {
        let values = vec![42.0; 256];
        let result = variance(&values, 20).unwrap();
        assert!(result[..19].iter().all(|value| value.is_nan()));
        assert!(result[19..].iter().all(|value| *value == 0.0));
    }

    #[test]
    fn perfect_correlation_remains_stable_over_long_series() {
        let x: Vec<f64> = (0..200_000).map(|i| 100.0 + i as f64 * 1.0e-5).collect();
        let y: Vec<f64> = x.iter().map(|value| value * 1.5 + 7.0).collect();
        let result = correlation(&x, &y, 30).unwrap();
        assert!(result[..29].iter().all(|value| value.is_nan()));
        assert!(result[29..]
            .iter()
            .all(|value| (*value - 1.0).abs() < 1.0e-8));
    }

    #[test]
    fn bbands_middle_matches_rolling_sma() {
        let input: Vec<f64> = (1..=128).map(|value| value as f64).collect();
        let (_, middle, _) = bbands_sma(&input, 20, 2.0, 2.0).unwrap();
        assert!((middle[19] - 10.5).abs() < 1.0e-12);
        assert!((middle[127] - 118.5).abs() < 1.0e-12);
    }
}
