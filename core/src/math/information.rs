//! Canonical information-theory helpers used by feature selection and research.

use super::quantile::{discretize, BinPolicy};
use std::collections::BTreeMap;

/// Mutual information between two discrete variables.
pub fn mutual_information_discrete(x: &[usize], y: &[usize]) -> f64 {
    if x.is_empty() || x.len() != y.len() {
        return 0.0;
    }
    let n = x.len() as f64;
    let mut joint = BTreeMap::<(usize, usize), usize>::new();
    let mut mx = BTreeMap::<usize, usize>::new();
    let mut my = BTreeMap::<usize, usize>::new();
    for (&xi, &yi) in x.iter().zip(y) {
        *joint.entry((xi, yi)).or_default() += 1;
        *mx.entry(xi).or_default() += 1;
        *my.entry(yi).or_default() += 1;
    }
    joint
        .into_iter()
        .map(|((xi, yi), count)| {
            let pxy = count as f64 / n;
            let px = mx[&xi] as f64 / n;
            let py = my[&yi] as f64 / n;
            pxy * (pxy / (px * py)).ln()
        })
        .sum::<f64>()
        .max(0.0)
}

/// Histogram-based mutual information between two continuous variables.
/// Non-finite pairs are discarded before binning.
pub fn mutual_information_continuous(x: &[f64], y: &[f64], bins: usize) -> f64 {
    if bins == 0 {
        return 0.0;
    }
    let valid: Vec<(f64, f64)> = x
        .iter()
        .copied()
        .zip(y.iter().copied())
        .filter(|(a, b)| a.is_finite() && b.is_finite())
        .collect();
    if valid.len() < 4 {
        return 0.0;
    }
    let xs: Vec<f64> = valid.iter().map(|v| v.0).collect();
    let ys: Vec<f64> = valid.iter().map(|v| v.1).collect();
    let xb = discretize(&xs, &BinPolicy::EqualWidth(bins));
    let yb = discretize(&ys, &BinPolicy::EqualWidth(bins));
    let xd: Vec<usize> = xb.into_iter().flatten().collect();
    let yd: Vec<usize> = yb.into_iter().flatten().collect();
    mutual_information_discrete(&xd, &yd)
}

/// Pairwise Pearson correlation after dropping non-finite pairs.
pub fn pairwise_pearson(x: &[f64], y: &[f64]) -> f64 {
    let valid: Vec<(f64, f64)> = x
        .iter()
        .copied()
        .zip(y.iter().copied())
        .filter(|(a, b)| a.is_finite() && b.is_finite())
        .collect();
    if valid.len() < 2 {
        return f64::NAN;
    }
    let n = valid.len() as f64;
    let mean_x = valid.iter().map(|v| v.0).sum::<f64>() / n;
    let mean_y = valid.iter().map(|v| v.1).sum::<f64>() / n;
    let mut cov = 0.0;
    let mut vx = 0.0;
    let mut vy = 0.0;
    for (a, b) in valid {
        let dx = a - mean_x;
        let dy = b - mean_y;
        cov += dx * dy;
        vx += dx * dx;
        vy += dy * dy;
    }
    let denom = (vx * vy).sqrt();
    if denom <= f64::EPSILON {
        0.0
    } else {
        cov / denom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mi_detects_dependency() {
        let x: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|v| v * 2.0).collect();
        assert!(mutual_information_continuous(&x, &y, 10) > 0.0);
    }
}
