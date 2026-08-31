//! Feature selection utilities: mutual information, variance threshold, correlation filter.

use super::{FeatureMatrix, FeatureRanking};

/// Filter out features with variance below a threshold.
pub fn variance_threshold(matrix: &FeatureMatrix, min_var: f64) -> FeatureMatrix {
    let mut result = FeatureMatrix::new();
    for i in 0..matrix.cols() {
        let col = matrix.column(i);
        let n = col.len() as f64;
        let mean = col.iter().filter(|v| !v.is_nan()).sum::<f64>() / n;
        let var = col
            .iter()
            .filter(|v| !v.is_nan())
            .map(|v| (v - mean).powi(2))
            .sum::<f64>()
            / n;
        if var >= min_var {
            result.add_column(matrix.features()[i].clone(), col.to_vec());
        }
    }
    result
}

/// Remove features that are highly correlated with others (keep one of each group).
pub fn correlation_filter(matrix: &FeatureMatrix, max_corr: f64) -> FeatureMatrix {
    let n_cols = matrix.cols();
    let mut should_drop = vec![false; n_cols];

    for i in 0..n_cols {
        if should_drop[i] {
            continue;
        }
        for (j, drop_flag) in should_drop.iter_mut().enumerate().skip(i + 1) {
            if *drop_flag {
                continue;
            }
            let corr = pearson_correlation(matrix.column(i), matrix.column(j));
            if corr.abs() > max_corr {
                *drop_flag = true;
            }
        }
    }

    let mut result = FeatureMatrix::new();
    for (i, &dropped) in should_drop.iter().enumerate() {
        if !dropped {
            result.add_column(matrix.features()[i].clone(), matrix.column(i).to_vec());
        }
    }
    result
}

/// Compute mutual information between each feature and the target labels.
///
/// Uses histogram-based MI estimation (discretization approach).
pub fn mutual_information(
    matrix: &FeatureMatrix,
    labels: &[f64],
    num_bins: usize,
) -> FeatureRanking {
    let mut rankings = Vec::with_capacity(matrix.cols());

    for i in 0..matrix.cols() {
        let feature = matrix.column(i);
        let mi = compute_mi(feature, labels, num_bins);
        rankings.push((matrix.features()[i].name.clone(), mi));
    }

    rankings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    FeatureRanking { rankings }
}

fn compute_mi(x: &[f64], y: &[f64], num_bins: usize) -> f64 {
    let n = x.len().min(y.len());
    if n == 0 || num_bins == 0 {
        return 0.0;
    }

    let valid: Vec<(f64, f64)> = x
        .iter()
        .zip(y.iter())
        .filter(|(&a, &b)| !a.is_nan() && !b.is_nan())
        .map(|(&a, &b)| (a, b))
        .collect();
    let n_valid = valid.len() as f64;
    if n_valid < 4.0 {
        return 0.0;
    }

    let x_vals: Vec<f64> = valid.iter().map(|v| v.0).collect();
    let y_vals: Vec<f64> = valid.iter().map(|v| v.1).collect();

    let x_bins = discretize(&x_vals, num_bins);
    let y_bins = discretize(&y_vals, num_bins);

    let mut joint = vec![vec![0usize; num_bins]; num_bins];
    let mut x_marginal = vec![0usize; num_bins];
    let mut y_marginal = vec![0usize; num_bins];

    for i in 0..valid.len() {
        joint[x_bins[i]][y_bins[i]] += 1;
        x_marginal[x_bins[i]] += 1;
        y_marginal[y_bins[i]] += 1;
    }

    let mut mi = 0.0;
    for xi in 0..num_bins {
        for yi in 0..num_bins {
            if joint[xi][yi] > 0 {
                let p_xy = joint[xi][yi] as f64 / n_valid;
                let p_x = x_marginal[xi] as f64 / n_valid;
                let p_y = y_marginal[yi] as f64 / n_valid;
                if p_x > 0.0 && p_y > 0.0 {
                    mi += p_xy * (p_xy / (p_x * p_y)).ln();
                }
            }
        }
    }
    mi.max(0.0)
}

fn discretize(data: &[f64], num_bins: usize) -> Vec<usize> {
    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range < 1e-15 {
        return vec![0; data.len()];
    }
    let bin_width = range / num_bins as f64;
    data.iter()
        .map(|&x| {
            let bin = ((x - min) / bin_width).floor() as usize;
            bin.min(num_bins - 1)
        })
        .collect()
}

fn pearson_correlation(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len());
    if n < 3 {
        return 0.0;
    }

    let mut sum_a = 0.0;
    let mut sum_b = 0.0;
    let mut count = 0.0;

    for i in 0..n {
        if !a[i].is_nan() && !b[i].is_nan() {
            sum_a += a[i];
            sum_b += b[i];
            count += 1.0;
        }
    }
    if count < 3.0 {
        return 0.0;
    }

    let mean_a = sum_a / count;
    let mean_b = sum_b / count;

    let mut cov = 0.0;
    let mut var_a = 0.0;
    let mut var_b = 0.0;

    for i in 0..n {
        if !a[i].is_nan() && !b[i].is_nan() {
            let da = a[i] - mean_a;
            let db = b[i] - mean_b;
            cov += da * db;
            var_a += da * da;
            var_b += db * db;
        }
    }

    let denom = (var_a * var_b).sqrt();
    if denom > 1e-15 {
        cov / denom
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::super::Feature;
    use super::*;

    #[test]
    fn test_variance_threshold() {
        let mut m = FeatureMatrix::new();
        m.add_column(
            Feature::new("const", "cat", 0),
            vec![5.0, 5.0, 5.0, 5.0, 5.0],
        );
        m.add_column(
            Feature::new("varied", "cat", 0),
            vec![1.0, 2.0, 3.0, 4.0, 5.0],
        );
        let filtered = variance_threshold(&m, 0.1);
        assert_eq!(filtered.cols(), 1);
        assert_eq!(filtered.column_names(), vec!["varied"]);
    }

    #[test]
    fn test_correlation_filter() {
        let mut m = FeatureMatrix::new();
        let a: Vec<f64> = (0..20).map(|i| i as f64).collect();
        let b: Vec<f64> = (0..20).map(|i| i as f64 * 2.0 + 1.0).collect();
        let c: Vec<f64> = (0..20).map(|i| (i as f64).sin()).collect();
        m.add_column(Feature::new("a", "cat", 0), a);
        m.add_column(Feature::new("b", "cat", 0), b);
        m.add_column(Feature::new("c", "cat", 0), c);
        let filtered = correlation_filter(&m, 0.95);
        // a and b are perfectly correlated, one should be dropped
        assert_eq!(filtered.cols(), 2);
    }

    #[test]
    fn test_mutual_information() {
        let mut m = FeatureMatrix::new();
        let x: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let noise: Vec<f64> = (0..100).map(|i| (i as f64 * 7.7).sin()).collect();
        m.add_column(Feature::new("signal", "cat", 0), x.clone());
        m.add_column(Feature::new("noise", "cat", 0), noise);
        let labels: Vec<f64> = x
            .iter()
            .map(|&v| if v > 50.0 { 1.0 } else { 0.0 })
            .collect();
        let ranking = mutual_information(&m, &labels, 10);
        // "signal" should rank higher than "noise"
        assert_eq!(ranking.rankings[0].0, "signal");
    }
}
