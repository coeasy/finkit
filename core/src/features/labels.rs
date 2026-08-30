//! ML label generation tools.
//!
//! Implements label generation methods commonly used in quantitative finance ML:
//! - Forward returns
//! - Triple barrier method (Marcos López de Prado, "Advances in Financial Machine Learning")
//! - Fixed horizon labels
//! - Binary classification labels

use ndarray::Array1;
use super::BarrierLabel;

/// Compute n-period forward log return: ln(close[i+n] / close[i]).
///
/// Last n values will be NaN (no future data available).
pub fn forward_return(close: &[f64], n: usize) -> Array1<f64> {
    let len = close.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    for i in 0..len.saturating_sub(n) {
        if close[i] > 0.0 && close[i + n] > 0.0 {
            out[i] = (close[i + n] / close[i]).ln();
        }
    }
    out
}

/// Compute n-period forward arithmetic return: (close[i+n] - close[i]) / close[i].
pub fn forward_return_arithmetic(close: &[f64], n: usize) -> Array1<f64> {
    let len = close.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    for i in 0..len.saturating_sub(n) {
        if close[i].abs() > 1e-15 {
            out[i] = (close[i + n] - close[i]) / close[i];
        }
    }
    out
}

/// Triple barrier method for label generation.
///
/// For each bar, determines which barrier is hit first:
/// - Upper barrier (profit-taking): price rises by `pt_factor` * daily_vol
/// - Lower barrier (stop-loss): price falls by `sl_factor` * daily_vol
/// - Vertical barrier (timeout): max_hold bars elapse
///
/// Reference: López de Prado, "Advances in Financial Machine Learning" (2018), Ch. 3.
pub fn triple_barrier(
    close: &[f64],
    high: &[f64],
    low: &[f64],
    pt_factor: f64,
    sl_factor: f64,
    max_hold: usize,
) -> Vec<BarrierLabel> {
    let len = close.len();
    let mut labels = Vec::with_capacity(len);

    // Estimate daily volatility using exponential moving window of returns
    let mut daily_vol = Vec::with_capacity(len);
    daily_vol.push(0.01); // initial estimate
    for i in 1..len {
        let ret = if close[i - 1] > 0.0 {
            ((close[i] / close[i - 1]).ln()).abs()
        } else {
            0.01
        };
        let prev = daily_vol[i - 1];
        daily_vol.push(prev * 0.94 + ret * 0.06);
    }

    for i in 0..len {
        let entry_price = close[i];
        let vol = daily_vol[i].max(1e-8);
        let upper_barrier = entry_price * (1.0 + pt_factor * vol);
        let lower_barrier = entry_price * (1.0 - sl_factor * vol);
        let end = (i + max_hold).min(len - 1);

        let mut label = BarrierLabel { label: 0, duration: end - i, ret: 0.0 };

        for j in (i + 1)..=end {
            if high[j] >= upper_barrier {
                let ret = (upper_barrier / entry_price).ln();
                label = BarrierLabel { label: 1, duration: j - i, ret };
                break;
            }
            if low[j] <= lower_barrier {
                let ret = (lower_barrier / entry_price).ln();
                label = BarrierLabel { label: -1, duration: j - i, ret };
                break;
            }
            if j == end {
                let ret = if entry_price > 0.0 { (close[j] / entry_price).ln() } else { 0.0 };
                label = BarrierLabel { label: 0, duration: j - i, ret };
            }
        }

        labels.push(label);
    }
    labels
}

/// Fixed horizon label: classify based on return over a fixed holding period.
///
/// Returns +1 if return > threshold, -1 if return < -threshold, 0 otherwise.
pub fn fixed_horizon_label(close: &[f64], horizon: usize, threshold: f64) -> Array1<f64> {
    let len = close.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    for i in 0..len.saturating_sub(horizon) {
        if close[i].abs() > 1e-15 {
            let ret = (close[i + horizon] - close[i]) / close[i];
            out[i] = if ret > threshold {
                1.0
            } else if ret < -threshold {
                -1.0
            } else {
                0.0
            };
        }
    }
    out
}

/// Binary label: 1 if n-period forward return > threshold, else 0.
pub fn binary_label(close: &[f64], n: usize, threshold: f64) -> Array1<f64> {
    let len = close.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    for i in 0..len.saturating_sub(n) {
        if close[i].abs() > 1e-15 {
            let ret = (close[i + n] - close[i]) / close[i];
            out[i] = if ret > threshold { 1.0 } else { 0.0 };
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forward_return() {
        let close = vec![100.0, 105.0, 110.0, 108.0, 115.0];
        let result = forward_return(&close, 2);
        assert!(!result[0].is_nan());
        assert!((result[0] - (110.0_f64 / 100.0).ln()).abs() < 1e-10);
        assert!(result[3].is_nan());
        assert!(result[4].is_nan());
    }

    #[test]
    fn test_triple_barrier() {
        let close = vec![100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 104.0, 103.0, 102.0, 101.0];
        let high: Vec<f64> = close.iter().map(|&c| c + 1.0).collect();
        let low: Vec<f64> = close.iter().map(|&c| c - 1.0).collect();
        let labels = triple_barrier(&close, &high, &low, 2.0, 2.0, 5);
        assert_eq!(labels.len(), 10);
        for label in &labels {
            assert!(label.label >= -1 && label.label <= 1);
        }
    }

    #[test]
    fn test_fixed_horizon_label() {
        let close = vec![100.0, 110.0, 90.0, 100.0, 100.0];
        let result = fixed_horizon_label(&close, 1, 0.05);
        assert_eq!(result[0], 1.0);  // 10% rise
        assert_eq!(result[1], -1.0); // ~18% fall
        assert!(result[3] == 0.0);   // 0% change
    }

    #[test]
    fn test_binary_label() {
        let close = vec![100.0, 105.0, 95.0, 110.0, 100.0];
        let result = binary_label(&close, 1, 0.02);
        assert_eq!(result[0], 1.0);  // 5% > 2%
        assert_eq!(result[1], 0.0);  // -9.5% not > 2%
        assert_eq!(result[2], 1.0);  // ~15.8% > 2%
    }
}
