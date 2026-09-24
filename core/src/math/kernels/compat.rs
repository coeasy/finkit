//! Compatibility bridge used while public indicator APIs migrate to canonical kernels.
//!
//! These functions preserve existing warm-up/alignment contracts and provide
//! direct parity tests against the legacy implementations before call sites
//! are switched over.

use super::{
    AdxState, AtrState, MonotonicExtrema, MovingAverageKind, MovingAverageState,
    RollingExtremaPair, RollingWelfordState,
};
use std::fmt;

/// Canonical-kernel compatibility execution errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelCompatError {
    InvalidWindow(usize),
    LengthMismatch { input: usize, output: usize },
    PairLengthMismatch { left: usize, right: usize },
    OhlcLengthMismatch,
}

impl fmt::Display for KernelCompatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWindow(window) => write!(f, "invalid rolling window {window}"),
            Self::LengthMismatch { input, output } => write!(
                f,
                "kernel compatibility output length mismatch: input={input}, output={output}"
            ),
            Self::PairLengthMismatch { left, right } => {
                write!(
                    f,
                    "kernel compatibility pair length mismatch: left={left}, right={right}"
                )
            }
            Self::OhlcLengthMismatch => write!(f, "OHLC inputs must have identical lengths"),
        }
    }
}

impl std::error::Error for KernelCompatError {}

fn validate(input: &[f64], window: usize, output: &[f64]) -> Result<(), KernelCompatError> {
    if window == 0 {
        return Err(KernelCompatError::InvalidWindow(window));
    }
    if input.len() != output.len() {
        return Err(KernelCompatError::LengthMismatch {
            input: input.len(),
            output: output.len(),
        });
    }
    Ok(())
}

fn validate_ohlc(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    output: &[f64],
) -> Result<(), KernelCompatError> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(KernelCompatError::OhlcLengthMismatch);
    }
    if high.len() != output.len() {
        return Err(KernelCompatError::LengthMismatch {
            input: high.len(),
            output: output.len(),
        });
    }
    Ok(())
}

/// Legacy-aligned SMA using the canonical O(1) moving-average state.
pub fn sma_into(input: &[f64], window: usize, output: &mut [f64]) -> Result<(), KernelCompatError> {
    validate(input, window, output)?;
    output.fill(f64::NAN);
    if window > input.len() {
        return Ok(());
    }
    // Feed the state only from the first finite value: a leading warm-up run
    // from an upstream rolling indicator would otherwise poison every window
    // (`MovingAverageState` is a running recurrence, so `NaN` is absorbing).
    // `start == 0` leaves the loop bit-identical to the unfixed version.
    let start = crate::math::leading_warmup(input);
    let mut state = MovingAverageState::new(MovingAverageKind::Sma, window);
    for (index, value) in input.iter().copied().enumerate().skip(start) {
        let current = state.update(value);
        if index + 1 >= start + window {
            output[index] = current;
        }
    }
    Ok(())
}

/// Legacy-aligned rolling mean using the canonical O(1) moving-average state.
///
/// This named entry point makes the statistical meaning explicit at call
/// sites while keeping SMA and rolling mean on the same implementation path.
#[inline]
pub fn rolling_mean_into(
    input: &[f64],
    window: usize,
    output: &mut [f64],
) -> Result<(), KernelCompatError> {
    sma_into(input, window, output)
}

/// Legacy-aligned WMA using the canonical O(1) weighted recurrence.
pub fn wma_into(input: &[f64], window: usize, output: &mut [f64]) -> Result<(), KernelCompatError> {
    validate(input, window, output)?;
    output.fill(f64::NAN);
    if window > input.len() {
        return Ok(());
    }
    // Same leading warm-up rule as `sma_into`: skip the upstream NaN prefix so
    // the weighted recurrence starts from a real value.
    let start = crate::math::leading_warmup(input);
    let mut state = MovingAverageState::new(MovingAverageKind::Wma, window);
    for (index, value) in input.iter().copied().enumerate().skip(start) {
        let current = state.update(value);
        if index + 1 >= start + window {
            output[index] = current;
        }
    }
    Ok(())
}

/// Legacy-aligned rolling sample variance using removable Welford state.
pub fn rolling_sample_variance_into(
    input: &[f64],
    window: usize,
    output: &mut [f64],
) -> Result<(), KernelCompatError> {
    validate(input, window, output)?;
    if window < 2 {
        return Err(KernelCompatError::InvalidWindow(window));
    }
    output.fill(f64::NAN);
    if window > input.len() {
        return Ok(());
    }
    // Feed the Welford state only from the first finite value: a leading warm-up
    // run from an upstream rolling indicator would otherwise poison every window
    // (`variance` is a running moment, so `NaN` is absorbing).
    let start = crate::math::leading_warmup(input);
    let mut state = RollingWelfordState::new(window);
    for (index, value) in input.iter().copied().enumerate().skip(start) {
        let current = state.update(value);
        if index + 1 >= start + window {
            // Sample variance is a sum of squared deviations, so a negative
            // value is floating-point residue rather than data. Clamping at the
            // source keeps `rolling_sample_stddev_into` exactly the square root
            // of this number and removes the last route by which a `sqrt` could
            // be handed a negative argument. For any window with real signal the
            // residue is positive and this is a no-op.
            let sample = current.variance * current.count as f64 / (current.count - 1) as f64;
            output[index] = sample.max(0.0);
        }
    }
    Ok(())
}

/// Legacy-aligned rolling sample standard deviation.
pub fn rolling_sample_stddev_into(
    input: &[f64],
    window: usize,
    output: &mut [f64],
) -> Result<(), KernelCompatError> {
    rolling_sample_variance_into(input, window, output)?;
    for value in output.iter_mut() {
        if !value.is_nan() {
            *value = value.max(0.0).sqrt();
        }
    }
    Ok(())
}

fn rolling_extrema_into(
    input: &[f64],
    window: usize,
    output: &mut [f64],
    max: bool,
) -> Result<(), KernelCompatError> {
    validate(input, window, output)?;
    output.fill(f64::NAN);
    if window > input.len() {
        return Ok(());
    }
    let mut state = MonotonicExtrema::new(window, max);
    for (index, value) in input.iter().copied().enumerate() {
        let current = state.update(index, value);
        if index + 1 >= window {
            output[index] = current;
        }
    }
    Ok(())
}

/// Legacy-aligned rolling maximum using the monotonic deque kernel.
pub fn rolling_max_into(
    input: &[f64],
    window: usize,
    output: &mut [f64],
) -> Result<(), KernelCompatError> {
    rolling_extrema_into(input, window, output, true)
}

/// Legacy-aligned rolling minimum using the monotonic deque kernel.
pub fn rolling_min_into(
    input: &[f64],
    window: usize,
    output: &mut [f64],
) -> Result<(), KernelCompatError> {
    rolling_extrema_into(input, window, output, false)
}

fn validate_pair(
    left: &[f64],
    right: &[f64],
    window: usize,
    output: &[f64],
) -> Result<(), KernelCompatError> {
    if window < 2 {
        return Err(KernelCompatError::InvalidWindow(window));
    }
    if left.len() != right.len() {
        return Err(KernelCompatError::PairLengthMismatch {
            left: left.len(),
            right: right.len(),
        });
    }
    if left.len() != output.len() {
        return Err(KernelCompatError::LengthMismatch {
            input: left.len(),
            output: output.len(),
        });
    }
    Ok(())
}

/// Compute a rolling Pearson correlation with O(1) amortized updates.
///
/// Windows containing a non-finite pair remain `NaN`, matching the formula
/// compatibility path. The sums are retained in a ring buffer so callers do
/// not allocate or copy a temporary window for every output row.
pub fn rolling_correlation_into(
    left: &[f64],
    right: &[f64],
    window: usize,
    output: &mut [f64],
) -> Result<(), KernelCompatError> {
    validate_pair(left, right, window, output)?;
    output.fill(f64::NAN);
    if window > left.len() {
        return Ok(());
    }

    let mut ring = vec![(0.0, 0.0, false); window];
    let mut cursor = 0usize;
    let mut count = 0usize;
    let mut invalid = 0usize;
    let mut sum_left = 0.0;
    let mut sum_right = 0.0;
    let mut sum_left_sq = 0.0;
    let mut sum_right_sq = 0.0;
    let mut sum_product = 0.0;

    for index in 0..left.len() {
        if count == window {
            let (old_left, old_right, old_valid) = ring[cursor];
            if old_valid {
                sum_left -= old_left;
                sum_right -= old_right;
                sum_left_sq -= old_left * old_left;
                sum_right_sq -= old_right * old_right;
                sum_product -= old_left * old_right;
            } else {
                invalid -= 1;
            }
        } else {
            count += 1;
        }

        let current_left = left[index];
        let current_right = right[index];
        let valid = current_left.is_finite() && current_right.is_finite();
        ring[cursor] = (current_left, current_right, valid);
        if valid {
            sum_left += current_left;
            sum_right += current_right;
            sum_left_sq += current_left * current_left;
            sum_right_sq += current_right * current_right;
            sum_product += current_left * current_right;
        } else {
            invalid += 1;
        }
        cursor += 1;
        if cursor == window {
            cursor = 0;
        }

        if count == window && invalid == 0 {
            let size = window as f64;
            let var_left = sum_left_sq - sum_left * sum_left / size;
            let var_right = sum_right_sq - sum_right * sum_right / size;
            // A window with no variance has no correlation, so it must stay
            // `NaN`. The test has to be relative to the floating-point noise
            // floor of `sum_sq - sum^2/n`, not an absolute constant: for a
            // constant window at a price of ~100 that residue is ~3e-11, which
            // an absolute `1e-15` threshold does not catch, so the guard used to
            // let a correlation through from a window with zero variance. See
            // `crate::math::degenerate_variance`.
            if !crate::math::degenerate_variance(var_left, sum_left_sq, size)
                && !crate::math::degenerate_variance(var_right, sum_right_sq, size)
            {
                let covariance = sum_product - sum_left * sum_right / size;
                output[index] = covariance / (var_left * var_right).sqrt();
            }
        }
    }
    Ok(())
}

/// Compute a rolling beta (`cov(left, right) / var(right)`) with O(1)
/// amortized updates. The common divisor cancels, so the population sums are
/// numerically equivalent to the sample covariance/sample variance pair.
pub fn rolling_beta_into(
    left: &[f64],
    right: &[f64],
    window: usize,
    output: &mut [f64],
) -> Result<(), KernelCompatError> {
    validate_pair(left, right, window, output)?;
    output.fill(f64::NAN);
    if window > left.len() {
        return Ok(());
    }

    let mut ring = vec![(0.0, 0.0, false); window];
    let mut cursor = 0usize;
    let mut count = 0usize;
    let mut invalid = 0usize;
    let mut sum_left = 0.0;
    let mut sum_right = 0.0;
    let mut sum_right_sq = 0.0;
    let mut sum_product = 0.0;

    for index in 0..left.len() {
        if count == window {
            let (old_left, old_right, old_valid) = ring[cursor];
            if old_valid {
                sum_left -= old_left;
                sum_right -= old_right;
                sum_right_sq -= old_right * old_right;
                sum_product -= old_left * old_right;
            } else {
                invalid -= 1;
            }
        } else {
            count += 1;
        }

        let current_left = left[index];
        let current_right = right[index];
        let valid = current_left.is_finite() && current_right.is_finite();
        ring[cursor] = (current_left, current_right, valid);
        if valid {
            sum_left += current_left;
            sum_right += current_right;
            sum_right_sq += current_right * current_right;
            sum_product += current_left * current_right;
        } else {
            invalid += 1;
        }
        cursor += 1;
        if cursor == window {
            cursor = 0;
        }

        if count == window && invalid == 0 {
            let size = window as f64;
            let variance_right = sum_right_sq - sum_right * sum_right / size;
            if variance_right.abs() >= 1e-15 {
                let covariance = sum_product - sum_left * sum_right / size;
                output[index] = covariance / variance_right;
            }
        }
    }
    Ok(())
}

/// Legacy-aligned MIDPOINT from one input series.
pub fn midpoint_into(
    input: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<(), KernelCompatError> {
    validate(input, period, output)?;
    output.fill(f64::NAN);
    let mut state = RollingExtremaPair::new(period);
    for (index, value) in input.iter().copied().enumerate() {
        let (highest, lowest) = state.update(value, value);
        if state.is_ready() {
            output[index] = (highest + lowest) * 0.5;
        }
    }
    Ok(())
}

/// Legacy-aligned MIDPRICE from high/low series.
pub fn midprice_into(
    high: &[f64],
    low: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<(), KernelCompatError> {
    if high.len() != low.len() {
        return Err(KernelCompatError::OhlcLengthMismatch);
    }
    validate(high, period, output)?;
    output.fill(f64::NAN);
    let mut state = RollingExtremaPair::new(period);
    for index in 0..high.len() {
        let (highest, lowest) = state.update(high[index], low[index]);
        if state.is_ready() {
            output[index] = (highest + lowest) * 0.5;
        }
    }
    Ok(())
}

/// TA-Lib-aligned Williams %R from the paired extrema kernel.
pub fn willr_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<(), KernelCompatError> {
    validate_ohlc(high, low, close, output)?;
    if period == 0 {
        return Err(KernelCompatError::InvalidWindow(period));
    }
    output.fill(f64::NAN);
    let mut state = RollingExtremaPair::new(period);
    for index in 0..high.len() {
        let (highest, lowest) = state.update(high[index], low[index]);
        if state.is_ready() {
            let denominator = highest - lowest;
            output[index] = if denominator > 1e-15 {
                (highest - close[index]) / denominator * -100.0
            } else {
                0.0
            };
        }
    }
    Ok(())
}

/// TA-Lib-aligned ATR using the canonical streaming state.
pub fn atr_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<(), KernelCompatError> {
    validate_ohlc(high, low, close, output)?;
    if period == 0 {
        return Err(KernelCompatError::InvalidWindow(period));
    }
    output.fill(f64::NAN);
    let mut state = AtrState::new(period);
    for index in 0..high.len() {
        if let Some(value) = state.update(high[index], low[index], close[index]) {
            output[index] = value;
        }
    }
    Ok(())
}

/// TA-Lib-aligned ADX using the canonical shared DMI state.
pub fn adx_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<(), KernelCompatError> {
    validate_ohlc(high, low, close, output)?;
    if period == 0 {
        return Err(KernelCompatError::InvalidWindow(period));
    }
    output.fill(f64::NAN);
    let mut state = AdxState::new(period);
    for index in 0..high.len() {
        if let Some(family) = state.update(high[index], low[index], close[index]) {
            if let Some(adx) = family.adx {
                output[index] = adx;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators;
    use crate::math::{moving_avg, statistics};

    fn assert_series_eq(left: &[f64], right: &[f64], tolerance: f64) {
        assert_eq!(left.len(), right.len());
        for (index, (lhs, rhs)) in left.iter().zip(right.iter()).enumerate() {
            if lhs.is_nan() && rhs.is_nan() {
                continue;
            }
            assert!(
                (lhs - rhs).abs() <= tolerance,
                "series mismatch at {index}: {lhs} vs {rhs}"
            );
        }
    }

    #[test]
    fn canonical_sma_matches_legacy_public_api() {
        let input: Vec<f64> = (0..128).map(|i| 100.0 + (i as f64 * 0.17).sin()).collect();
        let legacy = moving_avg::sma(&input, 14).unwrap();
        let mut canonical = vec![0.0; input.len()];
        sma_into(&input, 14, &mut canonical).unwrap();
        assert_series_eq(legacy.as_slice().unwrap(), &canonical, 1e-12);
    }

    #[test]
    fn canonical_wma_matches_legacy_public_api() {
        let input: Vec<f64> = (0..128).map(|i| 50.0 + i as f64 * 0.25).collect();
        let legacy = moving_avg::wma(&input, 10).unwrap();
        let mut canonical = vec![0.0; input.len()];
        wma_into(&input, 10, &mut canonical).unwrap();
        assert_series_eq(legacy.as_slice().unwrap(), &canonical, 1e-10);
    }

    #[test]
    fn rolling_welford_matches_legacy_variance_and_stddev() {
        let input: Vec<f64> = (0..160)
            .map(|i| 20.0 + (i as f64 * 0.11).sin() * 3.0 + i as f64 * 0.01)
            .collect();
        let legacy_var = statistics::rolling_variance(&input, 20).unwrap();
        let legacy_std = statistics::rolling_std_dev(&input, 20).unwrap();
        let mut canonical_var = vec![0.0; input.len()];
        let mut canonical_std = vec![0.0; input.len()];
        rolling_sample_variance_into(&input, 20, &mut canonical_var).unwrap();
        rolling_sample_stddev_into(&input, 20, &mut canonical_std).unwrap();
        assert_series_eq(legacy_var.as_slice().unwrap(), &canonical_var, 1e-9);
        assert_series_eq(legacy_std.as_slice().unwrap(), &canonical_std, 1e-9);
    }

    #[test]
    fn monotonic_extrema_matches_legacy_rolling_min_max() {
        let input: Vec<f64> = (0..160)
            .map(|i| (i as f64 * 0.31).sin() * 7.0 + (i % 9) as f64)
            .collect();
        let legacy_max = statistics::rolling_max(&input, 17).unwrap();
        let legacy_min = statistics::rolling_min(&input, 17).unwrap();
        let mut canonical_max = vec![0.0; input.len()];
        let mut canonical_min = vec![0.0; input.len()];
        rolling_max_into(&input, 17, &mut canonical_max).unwrap();
        rolling_min_into(&input, 17, &mut canonical_min).unwrap();
        assert_series_eq(legacy_max.as_slice().unwrap(), &canonical_max, 1e-12);
        assert_series_eq(legacy_min.as_slice().unwrap(), &canonical_min, 1e-12);
    }

    #[test]
    fn rolling_pair_kernels_match_legacy_formula_windows() {
        let left: Vec<f64> = (0..160)
            .map(|i| 10.0 + (i as f64 * 0.17).sin() + i as f64 * 0.03)
            .collect();
        let right: Vec<f64> = (0..160)
            .map(|i| 20.0 + (i as f64 * 0.11).cos() + i as f64 * 0.05)
            .collect();
        let window = 19;
        let mut canonical_corr = vec![f64::NAN; left.len()];
        let mut canonical_beta = vec![f64::NAN; left.len()];
        rolling_correlation_into(&left, &right, window, &mut canonical_corr).unwrap();
        rolling_beta_into(&left, &right, window, &mut canonical_beta).unwrap();

        for index in (window - 1)..left.len() {
            let start = index + 1 - window;
            let x = &left[start..=index];
            let y = &right[start..=index];
            let expected_corr = statistics::correlation(x, y).unwrap();
            let expected_beta =
                statistics::covariance(x, y).unwrap() / statistics::variance(y).unwrap();
            assert!((canonical_corr[index] - expected_corr).abs() < 1e-9);
            assert!(
                (canonical_beta[index] - expected_beta).abs() < 1e-9,
                "beta mismatch at {index}: {} vs {} (diff {})",
                canonical_beta[index],
                expected_beta,
                (canonical_beta[index] - expected_beta).abs()
            );
        }
    }

    #[test]
    fn paired_extrema_matches_midpoint_and_midprice() {
        let input: Vec<f64> = (0..160)
            .map(|i| 30.0 + (i as f64 * 0.19).sin() * 5.0 + (i % 4) as f64)
            .collect();
        let high: Vec<f64> = input.iter().map(|value| value + 1.2).collect();
        let low: Vec<f64> = input.iter().map(|value| value - 0.8).collect();
        let legacy_midpoint = indicators::midpoint(&input, 15).unwrap();
        let legacy_midprice = indicators::midprice(&high, &low, 15).unwrap();
        let mut canonical_midpoint = vec![0.0; input.len()];
        let mut canonical_midprice = vec![0.0; input.len()];
        midpoint_into(&input, 15, &mut canonical_midpoint).unwrap();
        midprice_into(&high, &low, 15, &mut canonical_midprice).unwrap();
        assert_series_eq(
            legacy_midpoint.as_slice().unwrap(),
            &canonical_midpoint,
            1e-12,
        );
        assert_series_eq(
            legacy_midprice.as_slice().unwrap(),
            &canonical_midprice,
            1e-12,
        );
    }

    #[test]
    fn paired_extrema_matches_legacy_willr() {
        let close: Vec<f64> = (0..180)
            .map(|i| 70.0 + (i as f64 * 0.13).sin() * 4.0 + i as f64 * 0.01)
            .collect();
        let high: Vec<f64> = close.iter().map(|value| value + 1.4).collect();
        let low: Vec<f64> = close.iter().map(|value| value - 1.1).collect();
        let legacy = indicators::willr(&high, &low, &close, 14).unwrap();
        let mut canonical = vec![0.0; close.len()];
        willr_into(&high, &low, &close, 14, &mut canonical).unwrap();
        assert_series_eq(legacy.as_slice().unwrap(), &canonical, 1e-12);
    }

    #[test]
    fn canonical_atr_matches_legacy_public_api() {
        let close: Vec<f64> = (0..160)
            .map(|i| 100.0 + (i as f64 * 0.09).sin() * 4.0 + i as f64 * 0.02)
            .collect();
        let high: Vec<f64> = close.iter().map(|value| value + 1.25).collect();
        let low: Vec<f64> = close.iter().map(|value| value - 0.85).collect();
        let legacy = indicators::atr(&high, &low, &close, 14).unwrap();
        let mut canonical = vec![0.0; close.len()];
        atr_into(&high, &low, &close, 14, &mut canonical).unwrap();
        assert_series_eq(legacy.as_slice().unwrap(), &canonical, 1e-12);
    }

    #[test]
    fn canonical_adx_matches_legacy_public_api() {
        let close: Vec<f64> = (0..240)
            .map(|i| 80.0 + (i as f64 * 0.07).sin() * 6.0 + i as f64 * 0.04)
            .collect();
        let high: Vec<f64> = close
            .iter()
            .enumerate()
            .map(|(i, value)| value + 1.0 + (i % 5) as f64 * 0.03)
            .collect();
        let low: Vec<f64> = close
            .iter()
            .enumerate()
            .map(|(i, value)| value - 0.9 - (i % 7) as f64 * 0.02)
            .collect();
        let legacy = indicators::adx(&high, &low, &close, 14).unwrap();
        let mut canonical = vec![0.0; close.len()];
        adx_into(&high, &low, &close, 14, &mut canonical).unwrap();
        assert_series_eq(legacy.as_slice().unwrap(), &canonical, 1e-10);
    }
}
