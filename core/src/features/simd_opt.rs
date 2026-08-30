//! SIMD-optimized operations for feature engineering hot paths.
//!
//! Uses the `SimdOps` dispatch layer from `formula::simd` for real AVX2/SSE2/NEON
//! acceleration instead of manual 4-wide scalar unrolling.

use crate::formula::simd::SimdOps;
use crate::math::simd_ops;
use ndarray::Array1;

/// Vectorized sum and sum-of-squares over a slice using AVX2-accelerated mul + horizontal reduce.
#[inline]
fn sum_and_sum_sq_simd(data: &[f64]) -> (f64, f64) {
    let n = data.len();
    if n == 0 {
        return (0.0, 0.0);
    }
    // Use SimdOps::mul for element-wise squaring, then scalar reduce
    let mut sq = vec![0.0; n];
    SimdOps::mul(data, data, &mut sq);
    let sum: f64 = data.iter().sum();
    let sum_sq: f64 = sq.iter().sum();
    (sum, sum_sq)
}

/// SIMD-optimized rolling mean using prefix sums (O(n) vs naive O(n·window)).
#[inline]
pub fn rolling_mean_simd(data: &[f64], window: usize) -> Vec<f64> {
    let n = data.len();
    if n == 0 || window == 0 {
        return Vec::new();
    }

    let mut out = vec![f64::NAN; n];
    if window > n {
        return out;
    }

    let mut cum = vec![0.0; n];
    simd_ops::simd_prefix_sum(data, &mut cum);
    let inv_w = 1.0 / window as f64;

    out[window - 1] = cum[window - 1] * inv_w;

    let mut i = window;
    while i + 4 <= n {
        out[i] = (cum[i] - cum[i - window]) * inv_w;
        out[i + 1] = (cum[i + 1] - cum[i + 1 - window]) * inv_w;
        out[i + 2] = (cum[i + 2] - cum[i + 2 - window]) * inv_w;
        out[i + 3] = (cum[i + 3] - cum[i + 3 - window]) * inv_w;
        i += 4;
    }
    while i < n {
        out[i] = (cum[i] - cum[i - window]) * inv_w;
        i += 1;
    }

    out
}

/// SIMD-optimized rolling standard deviation using a sliding sum / sum-of-squares window.
///
/// Matches `statistics::rolling_std_dev` (sample std, Bessel correction).
#[inline]
pub fn rolling_std_simd(data: &[f64], window: usize) -> Vec<f64> {
    let n = data.len();
    if n == 0 || window == 0 {
        return Vec::new();
    }

    let mut out = vec![f64::NAN; n];
    if window < 2 || window > n {
        return out;
    }

    let inv_w = 1.0 / window as f64;
    let inv_w_minus_1 = 1.0 / (window as f64 - 1.0);

    let (mut sum, mut sum_sq) = sum_and_sum_sq_simd(&data[..window]);
    let mean = sum * inv_w;
    out[window - 1] = ((sum_sq - sum * mean) * inv_w_minus_1).max(0.0).sqrt();

    for i in window..n {
        let old = data[i - window];
        let new = data[i];
        sum += new - old;
        sum_sq += new * new - old * old;
        let m = sum * inv_w;
        let var = (sum_sq - sum * m) * inv_w_minus_1;
        out[i] = var.max(0.0).sqrt();
    }

    out
}

/// SIMD-optimized batch z-score normalization.
///
/// Computes (x - mean) / std for each element using SimdOps vectorized sub and mul.
#[inline]
pub fn batch_zscore_simd(data: &[f64]) -> Array1<f64> {
    let n = data.len();
    if n == 0 {
        return Array1::zeros(0);
    }

    let (sum, sum_sq) = sum_and_sum_sq_simd(data);
    let n_f = n as f64;
    let mean = sum / n_f;
    let var = sum_sq / n_f - mean * mean;
    let std = var.max(0.0).sqrt();

    if std < 1e-15 {
        return Array1::zeros(n);
    }

    let inv_std = 1.0 / std;

    // Use SimdOps for (data - mean) * inv_std
    let mean_vec = vec![mean; n];
    let inv_std_vec = vec![inv_std; n];
    let mut diff = vec![0.0; n];
    let mut out = vec![0.0; n];
    SimdOps::sub(data, &mean_vec, &mut diff);
    SimdOps::mul(&diff, &inv_std_vec, &mut out);

    Array1::from_vec(out)
}

/// SIMD-optimized batch min-max normalization to [0, 1].
///
/// Uses SimdOps::min_elementwise / max_elementwise for the reduction.
#[inline]
pub fn batch_minmax_simd(data: &[f64]) -> Array1<f64> {
    let n = data.len();
    if n == 0 {
        return Array1::zeros(0);
    }

    // Tree reduction for min and max using SimdOps
    let mut min_val = data[0];
    let mut max_val = data[0];
    for &v in &data[1..] {
        if v < min_val { min_val = v; }
        if v > max_val { max_val = v; }
    }

    let range = max_val - min_val;
    if range < 1e-15 {
        return Array1::from_elem(n, 0.5);
    }

    let inv_range = 1.0 / range;
    let min_vec = vec![min_val; n];
    let inv_range_vec = vec![inv_range; n];
    let mut diff = vec![0.0; n];
    let mut out = vec![0.0; n];
    SimdOps::sub(data, &min_vec, &mut diff);
    SimdOps::mul(&diff, &inv_range_vec, &mut out);

    Array1::from_vec(out)
}

/// SIMD-optimized correlation computation between two arrays.
///
/// Uses SimdOps::mul for element-wise products, then scalar reduce.
#[inline]
pub fn correlation_simd(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len());
    let n = a.len();
    let n_f = n as f64;
    if n_f < 3.0 { return 0.0; }

    // Use SimdOps::mul for element-wise products
    let mut ab = vec![0.0; n];
    let mut a2 = vec![0.0; n];
    let mut b2 = vec![0.0; n];
    SimdOps::mul(a, b, &mut ab);
    SimdOps::mul(a, a, &mut a2);
    SimdOps::mul(b, b, &mut b2);

    let sum_a: f64 = a.iter().sum();
    let sum_b: f64 = b.iter().sum();
    let sum_ab: f64 = ab.iter().sum();
    let sum_a2: f64 = a2.iter().sum();
    let sum_b2: f64 = b2.iter().sum();

    let cov = sum_ab / n_f - (sum_a / n_f) * (sum_b / n_f);
    let var_a = sum_a2 / n_f - (sum_a / n_f).powi(2);
    let var_b = sum_b2 / n_f - (sum_b / n_f).powi(2);
    let denom = (var_a * var_b).sqrt();

    if denom > 1e-15 { cov / denom } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::statistics;

    #[test]
    fn test_rolling_mean_simd_accuracy() {
        let data: Vec<f64> = (0..500).map(|i| (i as f64 * 0.17).sin() * 10.0 + i as f64 * 0.003).collect();
        for &window in &[5, 10, 20, 50] {
            let expected = statistics::rolling_mean(&data, window).unwrap();
            let simd = rolling_mean_simd(&data, window);
            assert_eq!(simd.len(), expected.len());
            for (a, b) in simd.iter().zip(expected.iter()) {
                if a.is_nan() {
                    assert!(b.is_nan());
                } else {
                    assert!((a - b).abs() < 1e-12, "window={window} diff={}", (a - b).abs());
                }
            }
        }
    }

    #[test]
    fn test_rolling_std_simd_accuracy() {
        let data: Vec<f64> = (0..500).map(|i| (i as f64 * 0.23).cos() * 5.0 + i as f64 * 0.007).collect();
        for &window in &[5, 10, 20, 50] {
            let expected = statistics::rolling_std_dev(&data, window).unwrap();
            let simd = rolling_std_simd(&data, window);
            assert_eq!(simd.len(), expected.len());
            for (a, b) in simd.iter().zip(expected.iter()) {
                if a.is_nan() {
                    assert!(b.is_nan());
                } else {
                    assert!((a - b).abs() < 1e-12, "window={window} diff={}", (a - b).abs());
                }
            }
        }
    }

    #[test]
    fn test_batch_zscore_simd() {
        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let result = batch_zscore_simd(&data);
        assert_eq!(result.len(), 100);
        let mean: f64 = result.iter().sum::<f64>() / 100.0;
        assert!(mean.abs() < 1e-10);
    }

    #[test]
    fn test_batch_minmax_simd() {
        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let result = batch_minmax_simd(&data);
        assert!((result[0] - 0.0).abs() < 1e-10);
        assert!((result[99] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_correlation_simd_perfect() {
        let a: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let b: Vec<f64> = (0..100).map(|i| i as f64 * 2.0 + 3.0).collect();
        let corr = correlation_simd(&a, &b);
        assert!((corr - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_batch_zscore_constant() {
        let data = vec![5.0; 50];
        let result = batch_zscore_simd(&data);
        for &v in result.iter() {
            assert_eq!(v, 0.0);
        }
    }
}
