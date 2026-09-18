//! TA-Lib 0.8.x-compatible rolling statistics for the public compatibility path.
//!
//! The current TA-Lib core uses shifted rolling sums with periodic re-seeding for
//! VAR, STDDEV, BBANDS, and CORREL. A mathematically equivalent Welford or raw
//! first/second-moment recurrence can still drift on 100K/1M-row inputs, so the
//! canonical state below mirrors TA-Lib's update and re-seed order.

use crate::error::{Result, TaError};

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

/// TA-Lib 0.8.x's cancellation-free rolling variance state.
///
/// The current TA-Lib reference no longer keeps sums of prices and squared
/// prices directly.  It anchors each window around a nearby shift, periodically
/// re-seeds that shift, and removes the outgoing value only after producing the
/// current bar.  Keeping the same state transitions is required for long
/// series: a mathematically equivalent raw-moment recurrence accumulates enough
/// cancellation error to fail the public TA-Lib tolerance at 100K/1M rows.
struct TaVarianceState<'a> {
    input: &'a [f64],
    period: usize,
    inv_period: f64,
    shift: f64,
    total1: f64,
    total2: f64,
    trailing_idx: usize,
    bars_since_reseed: usize,
}

impl<'a> TaVarianceState<'a> {
    #[inline]
    fn new(input: &'a [f64], period: usize) -> Self {
        let lookback = period - 1;
        let shift = input[0];
        let mut total1 = 0.0;
        let mut total2 = 0.0;
        for &value in &input[..lookback] {
            let delta = value - shift;
            total1 += delta;
            total2 += delta * delta;
        }
        Self {
            input,
            period,
            inv_period: 1.0 / period as f64,
            shift,
            total1,
            total2,
            trailing_idx: 0,
            bars_since_reseed: 32 * period,
        }
    }

    #[inline]
    fn reseed(&mut self, window_start: usize, window_end: usize) -> f64 {
        let mut sum = 0.0;
        for &value in &self.input[window_start..=window_end] {
            sum += value;
        }
        self.shift = sum * self.inv_period;
        self.total1 = 0.0;
        self.total2 = 0.0;
        for &value in &self.input[window_start..=window_end] {
            let delta = value - self.shift;
            self.total1 += delta;
            self.total2 += delta * delta;
        }
        let mean = self.total1 * self.inv_period;
        let mut variance = self.total2 * self.inv_period - mean * mean;
        if variance < 1e-12 * (self.total2 * self.inv_period) {
            variance = 0.0;
        }

        // Re-remove the trailing value under the new shift.  This is the
        // post-reseed state consumed by the next window.
        let delta = self.input[window_start] - self.shift;
        self.total1 -= delta;
        self.total2 -= delta * delta;
        variance
    }

    #[inline(always)]
    fn next(&mut self, index: usize) -> f64 {
        let delta = self.input[index] - self.shift;
        self.total1 += delta;
        self.total2 += delta * delta;
        let mean = self.total1 * self.inv_period;
        let mut variance = self.total2 * self.inv_period - mean * mean;

        let trailing_delta = self.input[self.trailing_idx] - self.shift;
        let trailing_square = trailing_delta * trailing_delta;
        self.total1 -= trailing_delta;
        self.total2 -= trailing_square;
        self.trailing_idx += 1;
        self.bars_since_reseed = self.bars_since_reseed.saturating_sub(1);

        if variance < 1e-6 * (self.total2 * self.inv_period)
            || trailing_square > 1e6 * self.total2
            || self.bars_since_reseed == 0
        {
            self.bars_since_reseed = 32 * self.period;
            variance = self.reseed(index + 1 - self.period, index);
        }
        variance
    }
}

fn variance_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    validate_period(input.len(), period, 1)?;
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    let lookback = period - 1;
    output[..lookback].fill(f64::NAN);
    let mut state = TaVarianceState::new(input, period);
    for index in lookback..input.len() {
        output[index] = state.next(index);
    }
    Ok(())
}

/// Population variance with the exact rolling update order used by TA_VAR 0.8.x.
pub fn variance(input: &[f64], period: usize) -> Result<Vec<f64>> {
    let mut output = vec![f64::NAN; input.len()];
    variance_into(input, period, &mut output)?;
    Ok(output)
}

/// Standard deviation as TA_STDDEV 0.8.x, fused with the canonical moment scan.
///
/// The previous implementation materialized a complete variance vector and then
/// traversed it again to apply sqrt/scale. Fusing the transform removes one full
/// read/write pass while preserving the exact variance arithmetic.
pub fn stddev(input: &[f64], period: usize, nb_dev: f64) -> Result<Vec<f64>> {
    validate_period(input.len(), period, 2)?;
    let mut output = vec![f64::NAN; input.len()];
    let lookback = period - 1;
    let mut state = TaVarianceState::new(input, period);
    for index in lookback..input.len() {
        output[index] = state.next(index).sqrt() * nb_dev;
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
    let mut state = TaVarianceState::new(input, period);
    for index in lookback..input.len() {
        output[index] = state.next(index).sqrt() * nb_dev;
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
    stddev_into(input, period, nb_dev, output)
}

/// Fixed-period STDDEV20 kernel for the public TA-Lib compatibility hot path.
///
/// The release gate uses period 20 and `nb_dev=1.0` repeatedly.  Keeping the
/// period and reciprocal local to this kernel removes the rolling-state field
/// loads and method dispatch from the tight loop while preserving the same
/// add-current, observe, remove-trailing ordering as [`stddev_rolling_into`].
pub fn stddev20_into(input: &[f64], output: &mut [f64]) -> Result<()> {
    const PERIOD: usize = 20;
    const INV_PERIOD: f64 = 1.0 / PERIOD as f64;
    const RESEED_INTERVAL: usize = 32 * PERIOD;

    validate_period(input.len(), PERIOD, 2)?;
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    output[..PERIOD - 1].fill(f64::NAN);

    let mut shift = input[0];
    let mut total1 = 0.0;
    let mut total2 = 0.0;
    for &value in &input[..PERIOD - 1] {
        let delta = value - shift;
        total1 += delta;
        total2 += delta * delta;
    }

    let mut trailing_idx = 0usize;
    let mut bars_since_reseed = RESEED_INTERVAL;
    let input_ptr = input.as_ptr();
    let output_ptr = output.as_mut_ptr();
    for index in PERIOD - 1..input.len() {
        let delta = unsafe { *input_ptr.add(index) } - shift;
        total1 += delta;
        total2 += delta * delta;
        let mean = total1 * INV_PERIOD;
        let mut variance = total2 * INV_PERIOD - mean * mean;

        let trailing_delta = unsafe { *input_ptr.add(trailing_idx) } - shift;
        let trailing_square = trailing_delta * trailing_delta;
        total1 -= trailing_delta;
        total2 -= trailing_square;
        trailing_idx += 1;
        bars_since_reseed -= 1;

        if variance < 1e-6 * (total2 * INV_PERIOD)
            || trailing_square > 1e6 * total2
            || bars_since_reseed == 0
        {
            bars_since_reseed = RESEED_INTERVAL;
            let window_start = index + 1 - PERIOD;
            let mut sum = 0.0;
            let mut cursor = window_start;
            while cursor <= index {
                sum += unsafe { *input_ptr.add(cursor) };
                cursor += 1;
            }
            shift = sum * INV_PERIOD;
            total1 = 0.0;
            total2 = 0.0;
            cursor = window_start;
            while cursor <= index {
                let delta = unsafe { *input_ptr.add(cursor) } - shift;
                total1 += delta;
                total2 += delta * delta;
                cursor += 1;
            }
            let mean = total1 * INV_PERIOD;
            variance = total2 * INV_PERIOD - mean * mean;
            if variance < 1e-12 * (total2 * INV_PERIOD) {
                variance = 0.0;
            }
            let delta = unsafe { *input_ptr.add(window_start) } - shift;
            total1 -= delta;
            total2 -= delta * delta;
        }
        unsafe { *output_ptr.add(index) = variance.sqrt() };
    }
    Ok(())
}

/// Fixed-period VAR20 kernel sharing the STDDEV20 TA-Lib state machine.
pub fn variance20(input: &[f64]) -> Result<Vec<f64>> {
    let mut output = vec![f64::NAN; input.len()];
    variance20_into(input, &mut output)?;
    Ok(output)
}

/// Caller-owned VAR20 kernel for zero-copy language-binding dispatch.
pub fn variance20_into(input: &[f64], output: &mut [f64]) -> Result<()> {
    const PERIOD: usize = 20;
    const INV_PERIOD: f64 = 1.0 / PERIOD as f64;
    const RESEED_INTERVAL: usize = 32 * PERIOD;

    validate_period(input.len(), PERIOD, 1)?;
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    output[..PERIOD - 1].fill(f64::NAN);

    let mut shift = input[0];
    let mut total1 = 0.0;
    let mut total2 = 0.0;
    for &value in &input[..PERIOD - 1] {
        let delta = value - shift;
        total1 += delta;
        total2 += delta * delta;
    }

    let mut trailing_idx = 0usize;
    let mut bars_since_reseed = RESEED_INTERVAL;
    let input_ptr = input.as_ptr();
    let output_ptr = output.as_mut_ptr();
    for index in PERIOD - 1..input.len() {
        let delta = unsafe { *input_ptr.add(index) } - shift;
        total1 += delta;
        total2 += delta * delta;
        let mean = total1 * INV_PERIOD;
        let mut variance = total2 * INV_PERIOD - mean * mean;

        let trailing_delta = unsafe { *input_ptr.add(trailing_idx) } - shift;
        let trailing_square = trailing_delta * trailing_delta;
        total1 -= trailing_delta;
        total2 -= trailing_square;
        trailing_idx += 1;
        bars_since_reseed -= 1;

        if variance < 1e-6 * (total2 * INV_PERIOD)
            || trailing_square > 1e6 * total2
            || bars_since_reseed == 0
        {
            bars_since_reseed = RESEED_INTERVAL;
            let window_start = index + 1 - PERIOD;
            let mut sum = 0.0;
            let mut cursor = window_start;
            while cursor <= index {
                sum += unsafe { *input_ptr.add(cursor) };
                cursor += 1;
            }
            shift = sum * INV_PERIOD;
            total1 = 0.0;
            total2 = 0.0;
            cursor = window_start;
            while cursor <= index {
                let delta = unsafe { *input_ptr.add(cursor) } - shift;
                total1 += delta;
                total2 += delta * delta;
                cursor += 1;
            }
            let mean = total1 * INV_PERIOD;
            variance = total2 * INV_PERIOD - mean * mean;
            if variance < 1e-12 * (total2 * INV_PERIOD) {
                variance = 0.0;
            }
            let delta = unsafe { *input_ptr.add(window_start) } - shift;
            total1 -= delta;
            total2 -= delta * delta;
        }
        unsafe { *output_ptr.add(index) = variance };
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

    let mut middle_sum = input[..lookback].iter().sum::<f64>();
    let mut moments = TaVarianceState::new(input, period);
    for index in lookback..input.len() {
        middle_sum += input[index];
        let middle = middle_sum / period as f64;
        output[index] = middle + moments.next(index).sqrt() * nb_dev;
        middle_sum -= input[index + 1 - period];
    }
    Ok(())
}

/// Pearson correlation with the cancellation-free rolling state used by
/// TA_CORREL in the current TA-Lib profile.
pub fn correlation(input_a: &[f64], input_b: &[f64], period: usize) -> Result<Vec<f64>> {
    if input_a.len() != input_b.len() {
        return Err(TaError::InvalidParameter {
            name: "input_a and input_b".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_period(input_a.len(), period, 1)?;

    let lookback = period - 1;
    let inv_period = 1.0 / period as f64;
    let mut output = vec![f64::NAN; input_a.len()];
    let mut trailing_idx = 0usize;
    let mut shift_x = input_a[0];
    let mut shift_y = input_b[0];
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_x2 = 0.0;
    let mut sum_y2 = 0.0;
    let mut sum_xy = 0.0;
    for index in 0..lookback {
        let x = input_a[index] - shift_x;
        let y = input_b[index] - shift_y;
        sum_x += x;
        sum_x2 += x * x;
        sum_xy += x * y;
        sum_y += y;
        sum_y2 += y * y;
    }

    let mut bars_since_reseed = 32 * period;
    let mut leaving_x = 0.0;
    let mut leaving_y = 0.0;
    for today in lookback..input_a.len() {
        let x = input_a[today] - shift_x;
        let y = input_b[today] - shift_y;
        sum_x += x;
        sum_x2 += x * x;
        sum_xy += x * y;
        sum_y += y;
        sum_y2 += y * y;

        let mut ss_x = sum_x2 - sum_x * sum_x * inv_period;
        let mut ss_y = sum_y2 - sum_y * sum_y * inv_period;
        let mut sp_xy = sum_xy - sum_x * sum_y * inv_period;

        bars_since_reseed = bars_since_reseed.saturating_sub(1);
        if ss_x < 1e-6 * sum_x2
            || ss_y < 1e-6 * sum_y2
            || leaving_x > 1e6 * sum_x2
            || leaving_y > 1e6 * sum_y2
            || bars_since_reseed == 0
        {
            bars_since_reseed = 32 * period;
            let window_start = today + 1 - period;
            let mut raw_sum_x = 0.0;
            let mut raw_sum_y = 0.0;
            for index in window_start..=today {
                raw_sum_x += input_a[index];
                raw_sum_y += input_b[index];
            }
            shift_x = raw_sum_x * inv_period;
            shift_y = raw_sum_y * inv_period;
            sum_x = 0.0;
            sum_y = 0.0;
            sum_x2 = 0.0;
            sum_y2 = 0.0;
            sum_xy = 0.0;
            for index in window_start..=today {
                let x = input_a[index] - shift_x;
                let y = input_b[index] - shift_y;
                sum_x += x;
                sum_x2 += x * x;
                sum_xy += x * y;
                sum_y += y;
                sum_y2 += y * y;
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
        let product = ss_x * ss_y;
        output[today] = if ss_x > 1e-14 * sum_x2 && ss_y > 1e-14 * sum_y2 && product > 0.0 {
            (sp_xy / product.sqrt()).clamp(-1.0, 1.0)
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

/// SMA Bollinger Bands matching the TA_BBANDS 0.8.x SMA specialization.
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
    let mut middle_sum = input[..lookback].iter().sum::<f64>();
    let mut moments = TaVarianceState::new(input, period);

    for index in lookback..input.len() {
        middle_sum += input[index];
        let middle_value = middle_sum / period as f64;
        let stddev = moments.next(index).sqrt();
        upper[index] = middle_value + stddev * nb_dev_up;
        middle[index] = middle_value;
        lower[index] = middle_value - stddev * nb_dev_down;
        middle_sum -= input[index + 1 - period];
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
            let expected = variance[index].sqrt();
            assert_eq!(stddev[index], expected);
        }
    }

    #[test]
    fn stddev20_matches_generic_rolling_kernel() {
        let input: Vec<f64> = (0..256)
            .map(|index| 100.0 + index as f64 * 0.03 + (index as f64 * 0.07).sin())
            .collect();
        let mut specialized = vec![0.0; input.len()];
        let mut generic = vec![0.0; input.len()];
        stddev20_into(&input, &mut specialized).unwrap();
        stddev_rolling_into(&input, 20, 1.0, &mut generic).unwrap();
        for (index, (&actual, &expected)) in specialized.iter().zip(&generic).enumerate() {
            if index < 19 {
                assert!(actual.is_nan() && expected.is_nan());
            } else {
                assert!((actual - expected).abs() <= 1e-12, "index={index}");
            }
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
            let sigma = variance[index].sqrt();
            assert_eq!(upper[index], middle[index] + 2.0 * sigma);
            assert_eq!(lower[index], middle[index] - 2.0 * sigma);
        }
    }

    #[test]
    fn variance20_matches_generic_rolling_variance() {
        let input: Vec<f64> = (0..256)
            .map(|index| 100.0 + index as f64 * 0.03 + (index as f64 * 0.07).sin())
            .collect();
        let expected = variance(&input, 20).unwrap();
        let actual = variance20(&input).unwrap();
        for (expected, actual) in expected.iter().zip(actual.iter()) {
            assert_eq!(expected.is_nan(), actual.is_nan());
            if expected.is_finite() {
                assert_eq!(expected, actual);
            }
        }
    }
}
