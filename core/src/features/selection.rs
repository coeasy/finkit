//! Feature selection utilities backed by canonical math kernels.

use super::{FeatureMatrix, FeatureRanking};
use crate::math::information::{mutual_information_continuous, pairwise_pearson};

/// Filter out features with variance below a threshold.
pub fn variance_threshold(matrix: &FeatureMatrix, min_var: f64) -> FeatureMatrix {
    let mut result = FeatureMatrix::new();
    for i in 0..matrix.cols() {
        let col = matrix.column(i);
        let valid: Vec<f64> = col.iter().copied().filter(|v| v.is_finite()).collect();
        if valid.is_empty() { continue; }
        let mean = valid.iter().sum::<f64>() / valid.len() as f64;
        let var = valid.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / valid.len() as f64;
        if var >= min_var {
            result.add_column(matrix.features()[i].clone(), col.to_vec());
        }
    }
    result
}

/// Remove features that are highly correlated with an earlier retained feature.
pub fn correlation_filter(matrix: &FeatureMatrix, max_corr: f64) -> FeatureMatrix {
    let n_cols = matrix.cols();
    let mut should_drop = vec![false; n_cols];
    for i in 0..n_cols {
        if should_drop[i] { continue; }
        for (j, drop_flag) in should_drop.iter_mut().enumerate().skip(i + 1) {
            if *drop_flag { continue; }
            let corr = pairwise_pearson(matrix.column(i), matrix.column(j));
            if corr.is_finite() && corr.abs() > max_corr { *drop_flag = true; }
        }
    }
    let mut result = FeatureMatrix::new();
    for (i, dropped) in should_drop.into_iter().enumerate() {
        if !dropped { result.add_column(matrix.features()[i].clone(), matrix.column(i).to_vec()); }
    }
    result
}

/// Rank feature columns by histogram mutual information with target labels.
pub fn mutual_information(matrix: &FeatureMatrix, labels: &[f64], num_bins: usize) -> FeatureRanking {
    let mut rankings: Vec<(String, f64)> = (0..matrix.cols())
        .map(|i| (
            matrix.features()[i].name.clone(),
            mutual_information_continuous(matrix.column(i), labels, num_bins),
        ))
        .collect();
    rankings.sort_by(|a, b| b.1.total_cmp(&a.1));
    FeatureRanking { rankings }
}

#[cfg(test)]
mod tests {
    use super::super::Feature;
    use super::*;

    #[test]
    fn test_variance_threshold() {
        let mut m = FeatureMatrix::new();
        m.add_column(Feature::new("const", "cat", 0), vec![5.0; 5]);
        m.add_column(Feature::new("varied", "cat", 0), vec![1.0,2.0,3.0,4.0,5.0]);
        assert_eq!(variance_threshold(&m, 0.1).column_names(), vec!["varied"]);
    }

    #[test]
    fn test_correlation_filter() {
        let mut m = FeatureMatrix::new();
        let a: Vec<f64> = (0..20).map(|i| i as f64).collect();
        let b: Vec<f64> = a.iter().map(|v| v * 2.0 + 1.0).collect();
        let c: Vec<f64> = (0..20).map(|i| (i as f64).sin()).collect();
        m.add_column(Feature::new("a", "cat", 0), a);
        m.add_column(Feature::new("b", "cat", 0), b);
        m.add_column(Feature::new("c", "cat", 0), c);
        assert_eq!(correlation_filter(&m, 0.95).cols(), 2);
    }

    #[test]
    fn test_mutual_information() {
        let mut m = FeatureMatrix::new();
        let x: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let noise: Vec<f64> = (0..100).map(|i| (i as f64 * 7.7).sin()).collect();
        m.add_column(Feature::new("signal", "cat", 0), x.clone());
        m.add_column(Feature::new("noise", "cat", 0), noise);
        let labels: Vec<f64> = x.iter().map(|&v| if v > 50.0 { 1.0 } else { 0.0 }).collect();
        assert_eq!(mutual_information(&m, &labels, 10).rankings[0].0, "signal");
    }
}
