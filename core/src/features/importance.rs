//! Feature importance ranking via mutual information.

use std::collections::HashMap;

const DEFAULT_BINS: usize = 10;

/// Mutual information between two discrete variables using empirical probabilities.
///
/// Both slices must have the same length. Returns `0.0` for empty or mismatched input.
///
/// # Example
///
/// ```
/// use alpha_ta_core::features::mutual_info_discrete;
///
/// let x = vec![0, 1, 0, 1, 0, 1];
/// let y = vec![0, 1, 0, 1, 0, 1];
/// let mi = mutual_info_discrete(&x, &y);
/// assert!(mi > 0.0);
/// ```
pub fn mutual_info_discrete(x: &[usize], y: &[usize]) -> f64 {
    if x.is_empty() || x.len() != y.len() {
        return 0.0;
    }

    let n = x.len() as f64;
    let mut joint: HashMap<(usize, usize), usize> = HashMap::new();
    let mut x_marginal: HashMap<usize, usize> = HashMap::new();
    let mut y_marginal: HashMap<usize, usize> = HashMap::new();

    for (&xi, &yi) in x.iter().zip(y.iter()) {
        *joint.entry((xi, yi)).or_insert(0) += 1;
        *x_marginal.entry(xi).or_insert(0) += 1;
        *y_marginal.entry(yi).or_insert(0) += 1;
    }

    let mut mi = 0.0;
    for (&(xi, yi), &count) in &joint {
        let p_xy = count as f64 / n;
        let p_x = *x_marginal.get(&xi).unwrap_or(&0) as f64 / n;
        let p_y = *y_marginal.get(&yi).unwrap_or(&0) as f64 / n;
        if p_x > 0.0 && p_y > 0.0 {
            mi += p_xy * (p_xy / (p_x * p_y)).ln();
        }
    }
    mi.max(0.0)
}

/// Mutual information between two continuous variables using histogram binning.
///
/// NaN pairs are skipped. Returns `0.0` when fewer than four valid samples remain.
///
/// # Example
///
/// ```
/// use alpha_ta_core::features::mutual_info_continuous;
///
/// let x: Vec<f64> = (0..50).map(|i| i as f64).collect();
/// let y: Vec<f64> = x.iter().map(|&v| v * 2.0 + 0.1).collect();
/// let mi = mutual_info_continuous(&x, &y, 10);
/// assert!(mi > 0.0);
/// ```
pub fn mutual_info_continuous(x: &[f64], y: &[f64], bins: usize) -> f64 {
    compute_mi(x, y, bins)
}

/// Rank feature columns by mutual information with a continuous target.
///
/// `features[i]` is the i-th feature column; all columns must align with `target` length.
/// Returns `(feature_index, mi_score)` sorted by MI descending.
///
/// # Example
///
/// ```
/// use alpha_ta_core::features::feature_importance_rank;
///
/// let signal: Vec<f64> = (0..20).map(|i| i as f64).collect();
/// let noise: Vec<f64> = (0..20).map(|i| (i as f64 * 3.1).sin()).collect();
/// let target: Vec<f64> = signal.iter().map(|&v| if v > 10.0 { 1.0 } else { 0.0 }).collect();
/// let cols: Vec<&[f64]> = vec![signal.as_slice(), noise.as_slice()];
/// let ranking = feature_importance_rank(&cols, &target, 10);
/// assert_eq!(ranking[0].0, 0);
/// assert!(ranking[0].1 >= ranking[1].1);
/// ```
pub fn feature_importance_rank(
    features: &[&[f64]],
    target: &[f64],
    bins: usize,
) -> Vec<(usize, f64)> {
    let n = target.len();
    if n == 0 || features.is_empty() {
        return Vec::new();
    }

    let mut scores: Vec<(usize, f64)> = features
        .iter()
        .enumerate()
        .map(|(idx, col)| {
            let mi = if col.len() == n {
                mutual_info_continuous(col, target, bins)
            } else {
                0.0
            };
            (idx, mi)
        })
        .collect();

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores
}

/// Rank feature columns by mutual information with a continuous target (default 10 bins).
///
/// `features[i]` is the i-th feature column; all columns must align with `target` length.
/// Returns `(feature_index, mi_score)` sorted by MI descending.
pub fn mutual_information_ranking(features: &[&[f64]], target: &[f64]) -> Vec<(usize, f64)> {
    feature_importance_rank(features, target, DEFAULT_BINS)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutual_info_discrete_basic() {
        let x = vec![0, 1, 0, 1, 0, 1, 0, 1];
        let y = vec![0, 1, 0, 1, 0, 1, 0, 1];
        let mi = mutual_info_discrete(&x, &y);
        assert!(mi > 0.0, "perfectly correlated discrete vars should have MI > 0");

        let independent_x = vec![0, 0, 1, 1, 0, 0, 1, 1];
        let independent_y = vec![0, 1, 0, 1, 1, 0, 1, 0];
        let mi_indep = mutual_info_discrete(&independent_x, &independent_y);
        assert!(mi > mi_indep, "correlated MI should exceed independent MI");
    }

    #[test]
    fn test_mutual_info_continuous_basic() {
        let x: Vec<f64> = (0..50).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&v| v * 2.0 + 0.1).collect();
        let mi = mutual_info_continuous(&x, &y, 10);
        assert!(mi > 0.0, "linearly related continuous vars should have MI > 0");

        let noise: Vec<f64> = (0..50).map(|i| (i as f64 * 7.7).sin()).collect();
        let mi_noise = mutual_info_continuous(&x, &noise, 10);
        assert!(mi > mi_noise, "correlated MI should exceed uncorrelated MI");
    }

    #[test]
    fn test_feature_importance_rank_ordering() {
        let n = 100;
        let signal: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let noise: Vec<f64> = (0..n).map(|i| (i as f64 * 7.7).sin()).collect();
        let target: Vec<f64> = signal.iter().map(|&v| if v > 50.0 { 1.0 } else { 0.0 }).collect();

        let cols: Vec<&[f64]> = vec![signal.as_slice(), noise.as_slice()];
        let ranking = feature_importance_rank(&cols, &target, 10);

        assert_eq!(ranking.len(), 2);
        assert_eq!(ranking[0].0, 0, "signal feature should rank first");
        assert!(ranking[0].1 > ranking[1].1);
    }

    #[test]
    fn test_mutual_information_ranking() {
        let n = 100;
        let signal: Vec<f64> = (0..n).map(|i| i as f64).collect();
        let noise: Vec<f64> = (0..n).map(|i| (i as f64 * 7.7).sin()).collect();
        let target: Vec<f64> = signal.iter().map(|&v| if v > 50.0 { 1.0 } else { 0.0 }).collect();

        let cols: Vec<&[f64]> = vec![signal.as_slice(), noise.as_slice()];
        let ranking = mutual_information_ranking(&cols, &target);

        assert_eq!(ranking.len(), 2);
        assert_eq!(ranking[0].0, 0, "signal feature should rank first");
        assert!(ranking[0].1 > ranking[1].1);
    }
}
