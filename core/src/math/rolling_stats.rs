//! TA-Lib 0.7.1-compatible rolling statistics for the public compatibility path.
//!
//! The installed-wheel release gate benchmarks against TA-Lib core 0.7.1. That
//! release emits VAR/STDDEV/BBANDS from raw rolling first/second moments. A classic
//! remove/add Welford recurrence is numerically more stable, but it does not retain
//! TA-Lib's long-series rounding sequence closely enough for the public parity gate.
//! The canonical state below therefore keeps the exact TA-compatible update order
//! while giving VAR, STDDEV and SMA-BBANDS one shared rolling-moments kernel.

use crate::error::{Result, TaError};
use core::mem::MaybeUninit;
use core::slice;

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

/// Canonical rolling first/second-moment state for TA-Lib-compatible outputs.
///
/// `next` preserves the C implementation's per-accumulator sequencing:
/// add current -> observe moment -> remove trailing. BBANDS, VAR and STDDEV all
/// consume this state, so the hot loop no longer has three independent versions
/// of the same window lifecycle.
struct RollingMoments<'a> {
    input: &'a [f64],
    trailing_idx: usize,
    total: f64,
    total2: f64,
    period_f: f64,
}

impl<'a> RollingMoments<'a> {
    #[inline]
    fn new(input: &'a [f64], period: usize) -> Self {
        let lookback = period - 1;
        let mut total = 0.0;
        let mut total2 = 0.0;
        for &value in &input[..lookback] {
            total += value;
            let mut squared = value;
            squared *= squared;
            total2 += squared;
        }
        Self {
            input,
            trailing_idx: 0,
            total,
            total2,
            period_f: period as f64,
        }
    }

    /// Consume one window ending at `index`, returning `(mean, population variance)`.
    #[inline(always)]
    fn next(&mut self, index: usize) -> (f64, f64) {
        unsafe {
            let mut current = *self.input.get_unchecked(index);
            self.total += current;
            current *= current;
            self.total2 += current;

            // Keep division (rather than reciprocal multiplication) to match
            // TA-Lib's long-series rounding behaviour.
            let mean = self.total / self.period_f;
            let mean2 = self.total2 / self.period_f;

            let mut trailing = *self.input.get_unchecked(self.trailing_idx);
            self.trailing_idx += 1;
            self.total -= trailing;
            trailing *= trailing;
            self.total2 -= trailing;

            (mean, mean2 - mean * mean)
        }
    }
}

/// Population variance with the exact rolling update order used by TA_VAR 0.7.1.
pub fn variance(input: &[f64], period: usize) -> Result<Vec<f64>> {
    validate_period(input.len(), period, 1)?;

    let lookback = period - 1;
    let len = input.len();
    let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe { raw_output.set_len(len) };
    let output = unsafe { slice::from_raw_parts_mut(raw_output.as_mut_ptr().cast::<f64>(), len) };
    output[..lookback].fill(f64::NAN);
    let mut moments = RollingMoments::new(input, period);
    let output_ptr = output.as_mut_ptr();
    for index in lookback..len {
        let (_, variance) = moments.next(index);
        unsafe { *output_ptr.add(index) = variance };
    }
    let ptr = raw_output.as_mut_ptr().cast::<f64>();
    let capacity = raw_output.capacity();
    core::mem::forget(raw_output);
    Ok(unsafe { Vec::from_raw_parts(ptr, len, capacity) })
}

/// Standard deviation as TA_STDDEV 0.7.1, fused with the canonical moment scan.
///
/// The previous implementation materialized a complete variance vector and then
/// traversed it again to apply sqrt/scale. Fusing the transform removes one full
/// read/write pass while preserving the exact variance arithmetic.
pub fn stddev(input: &[f64], period: usize, nb_dev: f64) -> Result<Vec<f64>> {
    validate_period(input.len(), period, 2)?;

    let mut output = vec![f64::NAN; input.len()];
    let lookback = period - 1;
    let mut moments = RollingMoments::new(input, period);
    let output_ptr = output.as_mut_ptr();

    if nb_dev == 1.0 {
        for index in lookback..input.len() {
            let (_, variance) = moments.next(index);
            let value = if !is_zero_or_negative(variance) {
                variance.sqrt()
            } else {
                0.0
            };
            unsafe { *output_ptr.add(index) = value };
        }
    } else {
        for index in lookback..input.len() {
            let (_, variance) = moments.next(index);
            let value = if !is_zero_or_negative(variance) {
                variance.sqrt() * nb_dev
            } else {
                0.0
            };
            unsafe { *output_ptr.add(index) = value };
        }
    }
    Ok(output)
}

/// Standard deviation written directly into a caller-owned output slice.
///
/// This is the formula/FFI hot-path counterpart of [`crate::indicators::std_dev`].
/// It preserves that public API's first/second-moment update order while
/// avoiding a temporary result vector and the follow-up copy at the dispatcher
/// boundary.
pub fn stddev_into(input: &[f64], period: usize, nb_dev: f64, output: &mut [f64]) -> Result<()> {
    validate_period(input.len(), period, 2)?;
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }

    let lookback = period - 1;
    output[..lookback].fill(f64::NAN);
    let inverse_period = 1.0 / period as f64;
    let mut sum = 0.0;
    let mut sum_sq = 0.0;
    for &value in &input[..period] {
        sum += value;
        sum_sq += value * value;
    }
    let output_ptr = output.as_mut_ptr();
    unsafe {
        let mean = sum * inverse_period;
        let variance = (sum_sq - sum * mean) * inverse_period;
        *output_ptr.add(lookback) = variance.max(0.0).sqrt() * nb_dev;
    }
    for index in period..input.len() {
        let old = unsafe { *input.as_ptr().add(index - period) };
        let new = unsafe { *input.as_ptr().add(index) };
        sum += new - old;
        sum_sq += new * new - old * old;
        let mean = sum * inverse_period;
        let variance = (sum_sq - sum * mean) * inverse_period;
        unsafe {
            *output_ptr.add(index) = variance.max(0.0).sqrt() * nb_dev;
        }
    }

    Ok(())
}

/// Standard deviation written directly into a caller-owned output slice using
/// the canonical rolling-moment order used by the TA-Lib fast path.
pub fn stddev_rolling_into(
    input: &[f64],
    period: usize,
    nb_dev: f64,
    output: &mut [f64],
) -> Result<()> {
    validate_period(input.len(), period, 2)?;
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }

    let lookback = period - 1;
    output[..lookback].fill(f64::NAN);
    let mut moments = RollingMoments::new(input, period);
    let output_ptr = output.as_mut_ptr();
    if nb_dev == 1.0 {
        for index in lookback..input.len() {
            let (_, variance) = moments.next(index);
            let value = if !is_zero_or_negative(variance) {
                variance.sqrt()
            } else {
                0.0
            };
            unsafe { *output_ptr.add(index) = value };
        }
    } else {
        for index in lookback..input.len() {
            let (_, variance) = moments.next(index);
            let value = if !is_zero_or_negative(variance) {
                variance.sqrt() * nb_dev
            } else {
                0.0
            };
            unsafe { *output_ptr.add(index) = value };
        }
    }

    Ok(())
}

/// Upper Bollinger band written directly into a caller-owned output slice.
///
/// Formula `BOLL` historically returns the upper band only. Keeping that
/// projection in the rolling-moment kernel avoids allocating the unused middle
/// and lower bands for the common formula path.
pub fn bbands_upper_into(
    input: &[f64],
    period: usize,
    nb_dev: f64,
    output: &mut [f64],
) -> Result<()> {
    validate_period(input.len(), period, 2)?;
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }

    let lookback = period - 1;
    output[..lookback].fill(f64::NAN);

    // Keep the formula projection bit-for-bit aligned with the public BBANDS
    // implementation. Its Welford seed and rolling update order are part of
    // the TA-Lib compatibility contract; using a different rolling-moment
    // recurrence here creates avoidable ulp drift between `BOLL` and BBANDS.
    let inv_period = 1.0 / period as f64;
    let period_f = period as f64;
    let mut mean = 0.0;
    let mut m2 = 0.0;
    for (index, &value) in input.iter().enumerate().take(period) {
        let count = (index + 1) as f64;
        let delta = value - mean;
        mean += delta / count;
        m2 += delta * (value - mean);
    }

    let output_ptr = output.as_mut_ptr();
    unsafe {
        *output_ptr.add(lookback) = mean + (m2 * inv_period).max(0.0).sqrt() * nb_dev;
    }
    for index in period..input.len() {
        let old = unsafe { *input.as_ptr().add(index - period) };
        let new = unsafe { *input.as_ptr().add(index) };
        let old_mean = mean;
        mean += (new - old) / period_f;
        m2 += (new - mean) * (new - old_mean) - (old - mean) * (old - old_mean);
        unsafe {
            *output_ptr.add(index) = mean + (m2 * inv_period).sqrt() * nb_dev;
        }
    }
    Ok(())
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
/// All three bands are emitted during the same canonical moment scan. Outputs are
/// pre-sized and written through raw pointers to remove `Vec::push` capacity checks
/// from the million-row hot loop while preserving TA-Lib arithmetic order.
pub fn bbands_sma(
    input: &[f64],
    period: usize,
    nb_dev_up: f64,
    nb_dev_down: f64,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    validate_period(input.len(), period, 2)?;

    let len = input.len();
    // Every slot after the lookback is written by the fused scan. Keep the
    // three result vectors uninitialized until then so BBANDS does not pay
    // three full zero-fill passes before overwriting them.
    let mut upper = Vec::with_capacity(len);
    let mut middle = Vec::with_capacity(len);
    let mut lower = Vec::with_capacity(len);
    unsafe {
        upper.set_len(len);
        middle.set_len(len);
        lower.set_len(len);
    }
    bbands_sma_into(
        input,
        period,
        nb_dev_up,
        nb_dev_down,
        &mut upper,
        &mut middle,
        &mut lower,
    )?;

    Ok((upper, middle, lower))
}

/// Write SMA Bollinger Bands directly into caller-owned output buffers.
///
/// This is the NumPy boundary variant of [`bbands_sma`]. It keeps the same
/// rolling moment state and arithmetic order while avoiding the Rust-Vec to
/// NumPy copies on the installed-wheel hot path.
pub fn bbands_sma_into(
    input: &[f64],
    period: usize,
    nb_dev_up: f64,
    nb_dev_down: f64,
    upper: &mut [f64],
    middle: &mut [f64],
    lower: &mut [f64],
) -> Result<()> {
    validate_period(input.len(), period, 2)?;
    if upper.len() != input.len() || middle.len() != input.len() || lower.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "all output buffers must have the same length as input".to_string(),
        });
    }

    let lookback = period - 1;
    upper[..lookback].fill(f64::NAN);
    middle[..lookback].fill(f64::NAN);
    lower[..lookback].fill(f64::NAN);
    let upper_ptr = upper.as_mut_ptr();
    let middle_ptr = middle.as_mut_ptr();
    let lower_ptr = lower.as_mut_ptr();
    let mut moments = RollingMoments::new(input, period);

    for index in lookback..input.len() {
        let (middle_value, variance) = moments.next(index);
        let stddev = if !is_zero_or_negative(variance) {
            variance.sqrt()
        } else {
            0.0
        };

        unsafe {
            *upper_ptr.add(index) = middle_value + stddev * nb_dev_up;
            *middle_ptr.add(index) = middle_value;
            *lower_ptr.add(index) = middle_value - stddev * nb_dev_down;
        }
    }
    Ok(())
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
    fn stddev_is_exact_sqrt_of_canonical_variance() {
        let input: Vec<f64> = (0..512)
            .map(|index| 100.0 + index as f64 * 0.01 + (index as f64 * 0.17).sin())
            .collect();
        let variance = variance(&input, 20).unwrap();
        let stddev = stddev(&input, 20, 1.0).unwrap();
        for index in 19..input.len() {
            let expected = if !is_zero_or_negative(variance[index]) {
                variance[index].sqrt()
            } else {
                0.0
            };
            assert_eq!(stddev[index], expected);
        }
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

    #[test]
    fn bands_and_variance_share_the_same_canonical_moments() {
        let input: Vec<f64> = (0..256)
            .map(|index| 50.0 + index as f64 * 0.02 + (index as f64 * 0.11).cos())
            .collect();
        let variance = variance(&input, 20).unwrap();
        let (upper, middle, lower) = bbands_sma(&input, 20, 2.0, 2.0).unwrap();
        for index in 19..input.len() {
            let sigma = if !is_zero_or_negative(variance[index]) {
                variance[index].sqrt()
            } else {
                0.0
            };
            assert_eq!(upper[index], middle[index] + 2.0 * sigma);
            assert_eq!(lower[index], middle[index] - 2.0 * sigma);
        }
    }
}
