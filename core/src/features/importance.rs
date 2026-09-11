//! Feature-importance ranking backed by canonical information-theory kernels.

use crate::math::information::{mutual_information_continuous, mutual_information_discrete};

const DEFAULT_BINS: usize = 10;

/// Mutual information between two discrete variables using the canonical math kernel.
pub fn mutual_info_discrete(x: &[usize], y: &[usize]) -> f64 {
    mutual_information_discrete(x, y)
}

/// Mutual information between two continuous variables using canonical histogram binning.
pub fn mutual_info_continuous(x: &[f64], y: &[f64], bins: usize) -> f64 {
    mutual_information_continuous(x, y, bins)
}

/// Rank feature columns by mutual information with a continuous target.
pub fn feature_importance_rank(
    features: &[&[f64]],
    target: &[f64],
    bins: usize,
) -> Vec<(usize, f64)> {
    let mut scores: Vec<(usize, f64)> = features
        .iter()
        .enumerate()
        .map(|(idx, col)| {
            let score = if col.len() == target.len() {
                mutual_information_continuous(col, target, bins)
            } else {
                0.0
            };
            (idx, score)
        })
        .collect();
    scores.sort_by(|a, b| b.1.total_cmp(&a.1));
    scores
}

/// Rank feature columns by mutual information with the default 10-bin estimator.
pub fn mutual_information_ranking(features: &[&[f64]], target: &[f64]) -> Vec<(usize, f64)> {
    feature_importance_rank(features, target, DEFAULT_BINS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discrete_mi_detects_dependency() {
        let x = [0,1,0,1,0,1];
        assert!(mutual_info_discrete(&x, &x) > 0.0);
    }

    #[test]
    fn continuous_mi_ranks_signal_first() {
        let n = 100;
        let signal: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let noise: Vec<f64> = (0..n).map(|i| (i as f64 * 7.7).sin()).collect();
        let target: Vec<f64> = signal.iter().map(|&v| if v > 50.0 { 1.0 } else { 0.0 }).collect();
        let cols: Vec<&[f64]> = vec![signal.as_slice(), noise.as_slice()];
        assert_eq!(feature_importance_rank(&cols, &target, 10)[0].0, 0);
    }
}
