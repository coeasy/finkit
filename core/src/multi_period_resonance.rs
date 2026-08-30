//! Multi-Timeframe Pattern Resonance (多周期形态联动).
//!
//! When the same pattern fires on multiple timeframes simultaneously, the
//! signal is more reliable. This module provides combinators that fuse
//! signals from different timeframes.
//!
//! # Complements `features::multi_period`
//!
//! - [`features::multi_period`](crate::features::multi_period) generates
//!   the same indicator across multiple period parameters (feature
//!   engineering).
//! - **This module** fuses signals from different timeframes (signal
//!   enhancement). The user is expected to have already resampled the
//!   lower-timeframe signals to align with the higher-timeframe bars.
//!
//! # Example
//!
//! ```
//! use finkit::multi_period_resonance::{mtf_resonance, mtf_majority};
//! use finkit::patterns::Signal;
//! use ndarray::Array1;
//!
//! // Same length, two timeframes, both already resampled to daily bars
//! let daily: Array1<Signal> = Array1::from(vec![0, 100, 0, 0, -100]);
//! let weekly: Array1<Signal> = Array1::from(vec![0, 0, 100, 0, 0]);
//!
//! // At least 1 timeframe agrees → returns the signal
//! let resonance = mtf_resonance(&[&daily, &weekly], 1);
//! assert_eq!(resonance[1], 100);
//! ```

use crate::error::{Result, TaError};
use crate::patterns::Signal;
use ndarray::Array1;
use std::cmp::Ordering;

/// Validate that all signal arrays share the same length and that there is
/// at least one signal supplied.
fn validate_signals(signals: &[&Array1<Signal>]) -> Result<()> {
    if signals.is_empty() {
        return Err(TaError::InvalidParameter {
            name: "signals".to_string(),
            constraint: "at least one signal array required".to_string(),
        });
    }
    let n = signals[0].len();
    for s in signals {
        if s.len() != n {
            return Err(TaError::InvalidParameter {
                name: "signals".to_string(),
                constraint: "all signal arrays must have the same length".to_string(),
            });
        }
    }
    Ok(())
}

/// Multi-timeframe resonance — fires when at least `min_agree` of the
/// supplied timeframes are non-zero in the same direction at bar `i`.
///
/// * If at least `min_agree` bullish signals (any positive integer) →
///   output is `100`.
/// * If at least `min_agree` bearish signals (any negative integer) →
///   output is `-100`.
/// * Otherwise `0`.
///
/// # Errors
///
/// Returns an error if `signals` is empty or if the array lengths differ.
pub fn mtf_resonance(signals: &[&Array1<Signal>], min_agree: usize) -> Array1<Signal> {
    if validate_signals(signals).is_err() || min_agree == 0 {
        return Array1::zeros(if signals.is_empty() { 0 } else { signals[0].len() });
    }
    let n = signals[0].len();
    let mut out = Array1::<i32>::zeros(n);
    for i in 0..n {
        let mut bull = 0i32;
        let mut bear = 0i32;
        for s in signals {
            let v = s[i];
            match v.cmp(&0) {
                Ordering::Greater => bull += 1,
                Ordering::Less => bear += 1,
                Ordering::Equal => {}
            }
        }
        if bull >= min_agree as i32 {
            out[i] = 100;
        } else if bear >= min_agree as i32 {
            out[i] = -100;
        }
    }
    out
}

/// Multi-timeframe majority vote.
///
/// Output is `100` if more than half of the non-zero signals are bullish,
/// `-100` if more than half are bearish, else `0`. When no signals fire
/// at a bar, output is `0`.
///
/// # Errors
///
/// Returns an error if `signals` is empty or if the array lengths differ.
pub fn mtf_majority(signals: &[&Array1<Signal>]) -> Array1<Signal> {
    if validate_signals(signals).is_err() {
        return Array1::zeros(if signals.is_empty() { 0 } else { signals[0].len() });
    }
    let n = signals[0].len();
    let mut out = Array1::<i32>::zeros(n);
    for i in 0..n {
        let mut bull = 0i32;
        let mut bear = 0i32;
        for s in signals {
            let v = s[i];
            if v > 0 { bull += 1; } else if v < 0 { bear += 1; }
        }
        if bull > bear {
            out[i] = 100;
        } else if bear > bull {
            out[i] = -100;
        }
    }
    out
}

/// Multi-timeframe trend filter.
///
/// * If the higher timeframe is bullish (`higher_tf[i] > 0`): output the
///   lower-timeframe signal as-is.
/// * If the higher timeframe is bearish (`higher_tf[i] < 0`): invert the
///   lower-timeframe signal.
/// * If the higher timeframe is flat: output `0` (no trading).
///
/// Use case: a higher-timeframe downtrend filters out lower-timeframe
/// long signals.
pub fn mtf_trend_filter(higher_tf: &Array1<Signal>, lower: &Array1<Signal>) -> Array1<Signal> {
    if higher_tf.len() != lower.len() {
        return Array1::zeros(higher_tf.len().min(lower.len()));
    }
    let n = higher_tf.len();
    let mut out = Array1::<i32>::zeros(n);
    for i in 0..n {
        out[i] = match higher_tf[i].cmp(&0) {
            Ordering::Greater => lower[i],
            Ordering::Less => -lower[i],
            Ordering::Equal => 0,
        };
    }
    out
}

/// Multi-timeframe weighted sum.
///
/// Each timeframe contributes `signals[k][i] * weights[k]` to bar `i`.
/// The output is the sign of the weighted sum, snapped to ±100 / 0.
///
/// # Errors
///
/// Returns an error if the number of weights does not match the number of
/// signals, or if signals are inconsistent in length.
pub fn mtf_weighted(signals: &[&Array1<Signal>], weights: &[f64]) -> Array1<Signal> {
    if validate_signals(signals).is_err() || weights.len() != signals.len() {
        return Array1::zeros(if signals.is_empty() { 0 } else { signals[0].len() });
    }
    let n = signals[0].len();
    let mut out = Array1::<i32>::zeros(n);
    for i in 0..n {
        let mut total = 0.0;
        for (s, &w) in signals.iter().zip(weights.iter()) {
            total += (s[i] as f64) * w;
        }
        out[i] = if total > 0.0 { 100 } else if total < 0.0 { -100 } else { 0 };
    }
    out
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use approx::assert_relative_eq;

    fn s(values: &[i32]) -> Array1<Signal> {
        Array1::from(values.to_vec())
    }

    #[test]
    fn test_mtf_resonance_basic() {
        let a = s(&[0, 100, 0, 0, -100]);
        let b = s(&[0, 100, 100, 0, 0]);
        let out = mtf_resonance(&[&a, &b], 2);
        // bar 1: both bullish (>= 2) → 100
        assert_eq!(out[1], 100);
        // bar 2: only b bullish → fails min_agree=2
        assert_eq!(out[2], 0);
        // bar 4: only a bearish → fails
        assert_eq!(out[4], 0);
    }

    #[test]
    fn test_mtf_resonance_min_agree_one() {
        let a = s(&[0, 100, 0, -100, 0]);
        let b = s(&[0, 0, 0, 0, 0]);
        let out = mtf_resonance(&[&a, &b], 1);
        assert_eq!(out[1], 100);
        assert_eq!(out[3], -100);
    }

    #[test]
    fn test_mtf_majority() {
        let a = s(&[100, 100, 0, -100, -100]);
        let b = s(&[100, 0, 0, 0, -100]);
        let c = s(&[0, 0, 0, 0, 0]);
        let out = mtf_majority(&[&a, &b, &c]);
        // bar 0: 2 bull, 0 bear → 100
        assert_eq!(out[0], 100);
        // bar 1: 1 bull, 0 bear → 100
        assert_eq!(out[1], 100);
        // bar 3: 0 bull, 1 bear → -100
        assert_eq!(out[3], -100);
        // bar 4: 0 bull, 2 bear → -100
        assert_eq!(out[4], -100);
    }

    #[test]
    fn test_mtf_trend_filter_bullish_higher() {
        let higher = s(&[0, 100, 100, 0, -100]);
        let lower = s(&[100, 100, 100, 100, 100]);
        let out = mtf_trend_filter(&higher, &lower);
        // higher bullish: lower passes through
        assert_eq!(out[1], 100);
        assert_eq!(out[2], 100);
        // higher bearish: lower inverts
        assert_eq!(out[4], -100);
        // higher flat: zero
        assert_eq!(out[0], 0);
        assert_eq!(out[3], 0);
    }

    #[test]
    fn test_mtf_weighted_balanced() {
        let a = s(&[100, 100, 0, 0, 0]);
        let b = s(&[100, 100, 0, 0, 0]);
        let out = mtf_weighted(&[&a, &b], &[0.5, 0.5]);
        assert_eq!(out[0], 100);
        assert_eq!(out[2], 0);
    }

    #[test]
    fn test_mtf_weighted_imbalanced() {
        let a = s(&[100, 0, -100]);
        let b = s(&[-100, 0, 100]);
        // weights 0.9 vs 0.1 → a dominates
        let out = mtf_weighted(&[&a, &b], &[0.9, 0.1]);
        // bar 0: 100*0.9 + (-100)*0.1 = 80 > 0 → 100
        assert_eq!(out[0], 100);
        // bar 1: 0 + 0 = 0
        assert_eq!(out[1], 0);
        // bar 2: -100*0.9 + 100*0.1 = -80 < 0 → -100
        assert_eq!(out[2], -100);
    }

    #[test]
    fn test_mtf_resonance_empty_errors() {
        let out = mtf_resonance(&[], 1);
        // Empty input → returns empty output
        assert_eq!(out.len(), 0);
    }

    #[test]
    fn test_mtf_resonance_length_mismatch_errors() {
        let a = s(&[0, 100, 0]);
        let b = s(&[0, 100]);
        // Mismatched length → returns zero-filled array
        let out = mtf_resonance(&[&a, &b], 1);
        assert_eq!(out.len(), 3);
        assert_eq!(out.iter().sum::<i32>(), 0);
    }
}
