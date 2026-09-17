//! Canonical return primitives shared by factors, labels, research, and backtests.

use crate::error::{Result, TaError};

/// Return representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnKind {
    /// Arithmetic return: `end / start - 1`.
    Arithmetic,
    /// Log return: `ln(end / start)`.
    Log,
}

/// Compute one return between two prices.
#[inline]
pub fn return_between(start: f64, end: f64, kind: ReturnKind) -> f64 {
    if !start.is_finite() || !end.is_finite() || start == 0.0 {
        return f64::NAN;
    }
    match kind {
        ReturnKind::Arithmetic => end / start - 1.0,
        ReturnKind::Log => {
            if start > 0.0 && end > 0.0 {
                (end / start).ln()
            } else {
                f64::NAN
            }
        }
    }
}

/// Compute lagged returns. The first `period` entries are `NaN`.
pub fn lagged_return(values: &[f64], period: usize, kind: ReturnKind) -> Result<Vec<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "must be greater than zero".to_string(),
        });
    }
    let mut out = vec![f64::NAN; values.len()];
    for i in period..values.len() {
        out[i] = return_between(values[i - period], values[i], kind);
    }
    Ok(out)
}

/// Compute one-period returns.
pub fn one_period_returns(values: &[f64], kind: ReturnKind) -> Vec<f64> {
    if values.is_empty() {
        return Vec::new();
    }
    let mut out = vec![f64::NAN; values.len()];
    for i in 1..values.len() {
        out[i] = return_between(values[i - 1], values[i], kind);
    }
    out
}

/// Compute forward returns. The final `period` entries are `NaN`.
pub fn forward_return(values: &[f64], period: usize, kind: ReturnKind) -> Result<Vec<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "must be greater than zero".to_string(),
        });
    }
    let mut out = vec![f64::NAN; values.len()];
    for i in 0..values.len().saturating_sub(period) {
        out[i] = return_between(values[i], values[i + period], kind);
    }
    Ok(out)
}

/// Compute multiple forward-return horizons while preserving caller order.
pub fn forward_returns_many(
    values: &[f64],
    periods: &[usize],
    kind: ReturnKind,
) -> Result<Vec<Vec<f64>>> {
    periods
        .iter()
        .map(|&period| forward_return(values, period, kind))
        .collect()
}

/// Convert simple returns into a cumulative wealth curve.
pub fn cumulative_returns(returns: &[f64], starting_value: f64) -> Vec<f64> {
    let mut wealth = starting_value;
    returns
        .iter()
        .map(|&ret| {
            if ret.is_finite() {
                wealth *= 1.0 + ret;
            }
            wealth
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_and_log_return_are_consistent() {
        let arithmetic = return_between(100.0, 110.0, ReturnKind::Arithmetic);
        let log = return_between(100.0, 110.0, ReturnKind::Log);
        assert!((arithmetic - 0.1).abs() < 1e-12);
        assert!((log.exp() - 1.1).abs() < 1e-12);
    }

    #[test]
    fn forward_and_lagged_alignment_match() {
        let prices = [100.0, 101.0, 103.0, 106.0];
        let fwd = forward_return(&prices, 2, ReturnKind::Arithmetic).unwrap();
        let lag = lagged_return(&prices, 2, ReturnKind::Arithmetic).unwrap();
        assert!((fwd[0] - lag[2]).abs() < 1e-12);
        assert!(fwd[2].is_nan());
        assert!(lag[0].is_nan());
    }
}
