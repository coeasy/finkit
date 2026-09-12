//! Compatibility bridge used while public indicator APIs migrate to canonical kernels.
//!
//! These functions preserve existing warm-up/alignment contracts and provide
//! direct parity tests against the legacy implementations before call sites
//! are switched over.

use super::{
    AdxState, AtrState, MonotonicExtrema, MovingAverageKind, MovingAverageState,
    RollingWelfordState,
};
use std::fmt;

/// Canonical-kernel compatibility execution errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelCompatError {
    InvalidWindow(usize),
    LengthMismatch { input: usize, output: usize },
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
    let mut state = MovingAverageState::new(MovingAverageKind::Sma, window);
    for (index, value) in input.iter().copied().enumerate() {
        let current = state.update(value);
        if index + 1 >= window {
            output[index] = current;
        }
    }
    Ok(())
}

/// Legacy-aligned WMA using the canonical O(1) weighted recurrence.
pub fn wma_into(input: &[f64], window: usize, output: &mut [f64]) -> Result<(), KernelCompatError> {
    validate(input, window, output)?;
    output.fill(f64::NAN);
    if window > input.len() {
        return Ok(());
    }
    let mut state = MovingAverageState::new(MovingAverageKind::Wma, window);
    for (index, value) in input.iter().copied().enumerate() {
        let current = state.update(value);
        if index + 1 >= window {
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
    let mut state = RollingWelfordState::new(window);
    for (index, value) in input.iter().copied().enumerate() {
        let current = state.update(value);
        if index + 1 >= window {
            output[index] = current.variance * current.count as f64 / (current.count - 1) as f64;
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
