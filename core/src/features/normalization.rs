//! Feature normalization and standardization with look-ahead bias protection.

use ndarray::Array1;

/// Rolling z-score normalization: (x - rolling_mean) / rolling_std.
///
/// Uses only past data within the window to avoid look-ahead bias.
pub fn rolling_zscore_normalize(data: &[f64], window: usize) -> Array1<f64> {
    super::rolling_stats::rolling_zscore(data, window)
}

/// Rolling min-max normalization to [0, 1] range.
///
/// Uses only the rolling window to determine min/max bounds.
pub fn rolling_minmax(data: &[f64], window: usize) -> Array1<f64> {
    let len = data.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if window < 2 || len < window {
        return out;
    }

    for i in (window - 1)..len {
        let start = i + 1 - window;
        let slice = &data[start..=i];
        let min = slice.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = slice.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;
        if range > 1e-15 {
            out[i] = (data[i] - min) / range;
        } else {
            out[i] = 0.5;
        }
    }
    out
}

/// Robust scaler using median and IQR (interquartile range).
///
/// (x - median) / IQR, resistant to outliers.
pub fn robust_scaler(data: &[f64], window: usize) -> Array1<f64> {
    let len = data.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if window < 4 || len < window {
        return out;
    }

    for i in (window - 1)..len {
        let start = i + 1 - window;
        let mut sorted: Vec<f64> = data[start..=i].to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = sorted.len();
        let median = if n.is_multiple_of(2) {
            (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
        } else {
            sorted[n / 2]
        };
        let q1 = sorted[n / 4];
        let q3 = sorted[3 * n / 4];
        let iqr = q3 - q1;
        if iqr > 1e-15 {
            out[i] = (data[i] - median) / iqr;
        } else {
            out[i] = 0.0;
        }
    }
    out
}

/// Rank normalization: transforms values to their percentile rank within the window.
///
/// Returns values in [0, 1] representing the fraction of window values below.
pub fn rank_normalize(data: &[f64], window: usize) -> Array1<f64> {
    super::rolling_stats::rolling_percentile(data, window)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rolling_minmax_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = rolling_minmax(&data, 5);
        assert!(result[3].is_nan());
        // At index 4, window is [1,2,3,4,5], val=5, min=1, max=5 => (5-1)/(5-1) = 1.0
        assert!((result[4] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_rolling_minmax_range() {
        let data: Vec<f64> = (0..100).map(|i| (i as f64 * 0.1).sin()).collect();
        let result = rolling_minmax(&data, 20);
        for i in 19..100 {
            let v = result[i];
            assert!(!v.is_nan());
            assert!((0.0..=1.0).contains(&v), "Value {} out of range at index {}", v, i);
        }
    }

    #[test]
    fn test_robust_scaler_median_zero() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = robust_scaler(&data, 5);
        assert!(!result[4].is_nan());
        // Median of [1,2,3,4,5] = 3, Q1=1.25~sorted[1]=2, Q3=sorted[3]=4, IQR=2
        // (5-3)/2 = 1.0
    }

    #[test]
    fn test_rank_normalize_range() {
        let data = vec![5.0, 3.0, 8.0, 1.0, 9.0, 2.0, 7.0, 4.0, 6.0, 10.0];
        let result = rank_normalize(&data, 5);
        for i in 4..10 {
            let v = result[i];
            assert!(!v.is_nan());
            assert!((0.0..=1.0).contains(&v));
        }
    }
}
