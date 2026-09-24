//! Batch and rolling helpers for feature engineering hot paths.
//!
//! The arithmetic is delegated to the crate's canonical `math::` kernels rather
//! than re-derived here. These entry points used to carry their own one-pass
//! moment loops (`sum_sq - sum^2/n`) and their own sliding-window recurrences,
//! which agreed with the canonical kernels on ordinary test data and diverged
//! badly on a large baseline — a series and its exact affine image `2x + 3` are
//! perfectly correlated, yet this module reported `r = 0.667` at a `1e9`
//! baseline, and its rolling standard deviation reported `0.0` where the true
//! value is `1.0`. Two spellings of the same statistic must not answer
//! differently, so the spellings now share one implementation.

use crate::formula::simd::SimdOps;
use crate::math::kernels::{rolling_sample_stddev_into, sma_into};
use ndarray::Array1;

/// Rolling mean, delegated to the canonical O(1) moving-average state.
///
/// Equivalent to [`crate::math::statistics::rolling_mean`] and to
/// [`crate::math::kernels::sma_into`], including their leading warm-up rule: a
/// leading non-finite run is treated as an upstream indicator's warm-up prefix
/// and skipped, not as data.
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

    sma_into(data, window, &mut out).expect("lengths were validated above");
    out
}

/// Rolling sample standard deviation (Bessel correction), delegated to the
/// canonical compat kernel.
///
/// Equivalent to [`crate::math::statistics::rolling_std_dev`] and to
/// [`crate::math::kernels::rolling_sample_stddev_into`]. The sliding
/// sum/sum-of-squares window this used to run cancelled catastrophically at a
/// large baseline — measured `0.0` for every window of `1e12 + i` with
/// `window = 3`, where the true sample deviation is exactly `1.0`.
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

    rolling_sample_stddev_into(data, window, &mut out).expect("lengths were validated above");
    out
}

/// Batch z-score normalisation, `(x - mean) / stddev` with the population
/// deviation.
///
/// The deviation comes from the centred moments, so the result depends only on
/// the spread of the data and not on its offset: an arithmetic ramp scores the
/// same whether it starts at `0` or at `1e9`. The previous one-pass form did
/// not — at a `1e9` baseline it reported `-1.607` where the offset-invariant
/// answer is `-1.705`.
#[inline]
pub fn batch_zscore_simd(data: &[f64]) -> Array1<f64> {
    let n = data.len();
    if n == 0 {
        return Array1::zeros(0);
    }

    let (_, sum_dx_dx, _) = crate::math::centred_moments(data, data);
    let var = sum_dx_dx / n as f64;
    let std = var.max(0.0).sqrt();

    if std < 1e-15 {
        return Array1::zeros(n);
    }

    let inv_std = 1.0 / std;
    let mean = data.iter().sum::<f64>() / n as f64;

    // Use SimdOps for (data - mean) * inv_std
    let mean_vec = vec![mean; n];
    let inv_std_vec = vec![inv_std; n];
    let mut diff = vec![0.0; n];
    let mut out = vec![0.0; n];
    SimdOps::sub(data, &mean_vec, &mut diff);
    SimdOps::mul(&diff, &inv_std_vec, &mut out);

    Array1::from_vec(out)
}

/// Batch min-max normalization to `[0, 1]`.
///
/// The min/max reduction is scalar and the scaling step uses `SimdOps::sub` /
/// `SimdOps::mul`. `SimdOps::min_elementwise` / `max_elementwise` are
/// element-wise kernels and cannot perform a reduction, so they are not used
/// here. A constant series (zero range) maps to `0.5`.
#[inline]
pub fn batch_minmax_simd(data: &[f64]) -> Array1<f64> {
    let n = data.len();
    if n == 0 {
        return Array1::zeros(0);
    }

    // Scalar reduction for min and max. `SimdOps::min_elementwise` /
    // `max_elementwise` are *element-wise* kernels (two arrays in, element-wise
    // result out) and cannot perform a reduction, so they are not usable here —
    // the previous doc comment claimed they were.
    let mut min_val = data[0];
    let mut max_val = data[0];
    for &v in &data[1..] {
        if v < min_val {
            min_val = v;
        }
        if v > max_val {
            max_val = v;
        }
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

/// Pearson correlation coefficient between two arrays.
///
/// Delegates to [`crate::math::centred_moments`], so the coefficient is
/// invariant under the offset of the data: `b = 2a + 3` scores exactly `1.0`
/// whether `a` runs over `0..n` or over `1e9..1e9+n`. The previous one-pass
/// form reported `0.667` at the large baseline — a silent 33% error in a
/// function that is exposed to Python as `correlation`.
///
/// Returns `0.0` when either series has no spread (or fewer than three points),
/// matching this entry point's historical degenerate contract.
#[inline]
pub fn correlation_simd(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len());
    if a.len() < 3 {
        return 0.0;
    }

    let (cov, sum_da_da, sum_db_db) = crate::math::centred_moments(a, b);
    let denom = (sum_da_da * sum_db_db).sqrt();

    if denom > 1e-15 {
        cov / denom
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::statistics;

    #[test]
    fn test_rolling_mean_simd_accuracy() {
        let data: Vec<f64> = (0..500)
            .map(|i| (i as f64 * 0.17).sin() * 10.0 + i as f64 * 0.003)
            .collect();
        for &window in &[5, 10, 20, 50] {
            let expected = statistics::rolling_mean(&data, window).unwrap();
            let simd = rolling_mean_simd(&data, window);
            assert_eq!(simd.len(), expected.len());
            for (a, b) in simd.iter().zip(expected.iter()) {
                if a.is_nan() {
                    assert!(b.is_nan());
                } else {
                    assert!(
                        (a - b).abs() < 1e-12,
                        "window={window} diff={}",
                        (a - b).abs()
                    );
                }
            }
        }
    }

    #[test]
    fn test_rolling_std_simd_accuracy() {
        let data: Vec<f64> = (0..500)
            .map(|i| (i as f64 * 0.23).cos() * 5.0 + i as f64 * 0.007)
            .collect();
        for &window in &[5, 10, 20, 50] {
            let expected = statistics::rolling_std_dev(&data, window).unwrap();
            let simd = rolling_std_simd(&data, window);
            assert_eq!(simd.len(), expected.len());
            for (a, b) in simd.iter().zip(expected.iter()) {
                if a.is_nan() {
                    assert!(b.is_nan());
                } else {
                    assert!(
                        (a - b).abs() < 1e-12,
                        "window={window} diff={}",
                        (a - b).abs()
                    );
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
