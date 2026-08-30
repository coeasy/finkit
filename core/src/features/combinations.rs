//! Feature combinations: ratio, spread, correlation matrix.

use ndarray::Array1;
use super::{Feature, FeatureMatrix};

/// Compute element-wise ratio: a[i] / b[i].
pub fn feature_ratio(a: &[f64], b: &[f64]) -> Array1<f64> {
    assert_eq!(a.len(), b.len());
    let mut out = Array1::zeros(a.len());
    for i in 0..a.len() {
        if b[i].abs() > 1e-15 {
            out[i] = a[i] / b[i];
        } else {
            out[i] = f64::NAN;
        }
    }
    out
}

/// Compute element-wise spread: a[i] - b[i].
pub fn feature_spread(a: &[f64], b: &[f64]) -> Array1<f64> {
    assert_eq!(a.len(), b.len());
    Array1::from_iter(a.iter().zip(b.iter()).map(|(&x, &y)| x - y))
}

/// Compute rolling Pearson correlation between two series.
pub fn rolling_correlation(a: &[f64], b: &[f64], window: usize) -> Array1<f64> {
    assert_eq!(a.len(), b.len());
    let len = a.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if window < 3 || len < window {
        return out;
    }

    for i in (window - 1)..len {
        let start = i + 1 - window;
        let sa = &a[start..=i];
        let sb = &b[start..=i];
        let n = window as f64;
        let mean_a = sa.iter().sum::<f64>() / n;
        let mean_b = sb.iter().sum::<f64>() / n;

        let mut cov = 0.0;
        let mut var_a = 0.0;
        let mut var_b = 0.0;
        for j in 0..window {
            let da = sa[j] - mean_a;
            let db = sb[j] - mean_b;
            cov += da * db;
            var_a += da * da;
            var_b += db * db;
        }

        let denom = (var_a * var_b).sqrt();
        if denom > 1e-15 {
            out[i] = cov / denom;
        } else {
            out[i] = 0.0;
        }
    }
    out
}

/// Compute rolling correlation matrix for N features.
///
/// Returns a FeatureMatrix where each column is the rolling correlation
/// between a pair of input columns, named "corr_{name_a}_{name_b}".
pub fn rolling_correlation_matrix(
    columns: &[(&str, &[f64])],
    window: usize,
) -> FeatureMatrix {
    let n = columns.len();
    let mut matrix = FeatureMatrix::new();

    for i in 0..n {
        for j in (i + 1)..n {
            let corr = rolling_correlation(columns[i].1, columns[j].1, window);
            let name = format!("corr_{}_{}", columns[i].0, columns[j].0);
            matrix.add_column(
                Feature::new(name, "correlation", window),
                corr.to_vec(),
            );
        }
    }
    matrix
}

/// Automatically generate all pairwise ratio and spread combinations.
pub fn auto_combine(columns: &[(&str, &[f64])]) -> FeatureMatrix {
    let n = columns.len();
    let mut matrix = FeatureMatrix::new();

    for i in 0..n {
        for j in (i + 1)..n {
            let ratio = feature_ratio(columns[i].1, columns[j].1);
            let spread = feature_spread(columns[i].1, columns[j].1);

            matrix.add_column(
                Feature::new(
                    format!("ratio_{}_{}", columns[i].0, columns[j].0),
                    "combination",
                    0,
                ),
                ratio.to_vec(),
            );
            matrix.add_column(
                Feature::new(
                    format!("spread_{}_{}", columns[i].0, columns[j].0),
                    "combination",
                    0,
                ),
                spread.to_vec(),
            );
        }
    }
    matrix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_ratio() {
        let a = vec![10.0, 20.0, 30.0];
        let b = vec![2.0, 4.0, 6.0];
        let result = feature_ratio(&a, &b);
        assert_eq!(result[0], 5.0);
        assert_eq!(result[1], 5.0);
        assert_eq!(result[2], 5.0);
    }

    #[test]
    fn test_feature_spread() {
        let a = vec![10.0, 20.0, 30.0];
        let b = vec![2.0, 5.0, 10.0];
        let result = feature_spread(&a, &b);
        assert_eq!(result[0], 8.0);
        assert_eq!(result[1], 15.0);
        assert_eq!(result[2], 20.0);
    }

    #[test]
    fn test_rolling_correlation_perfect() {
        let a: Vec<f64> = (0..20).map(|i| i as f64).collect();
        let b: Vec<f64> = (0..20).map(|i| i as f64 * 2.0 + 1.0).collect();
        let result = rolling_correlation(&a, &b, 10);
        assert!((result[9] - 1.0).abs() < 1e-10); // perfect positive correlation
    }

    #[test]
    fn test_rolling_correlation_matrix_pairs() {
        let a: Vec<f64> = (0..20).map(|i| i as f64).collect();
        let b: Vec<f64> = (0..20).map(|i| (i as f64).sin()).collect();
        let c: Vec<f64> = (0..20).map(|i| (i as f64).cos()).collect();
        let cols = vec![("a", a.as_slice()), ("b", b.as_slice()), ("c", c.as_slice())];
        let matrix = rolling_correlation_matrix(&cols, 10);
        // 3 columns => 3 pairs
        assert_eq!(matrix.cols(), 3);
    }

    #[test]
    fn test_auto_combine() {
        let a = vec![10.0, 20.0, 30.0];
        let b = vec![2.0, 5.0, 10.0];
        let c = vec![1.0, 1.0, 1.0];
        let cols = vec![("a", a.as_slice()), ("b", b.as_slice()), ("c", c.as_slice())];
        let matrix = auto_combine(&cols);
        // 3 pairs * 2 (ratio + spread) = 6 columns
        assert_eq!(matrix.cols(), 6);
    }
}
