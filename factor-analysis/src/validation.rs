//! Validation and overfitting-control utilities.

pub use finkit::features::{CombinatorialPurgedCV, EmbargoKFold, PurgedKFold, WalkForwardSplit};
use finkit::performance::{excess_kurtosis, skewness};
use serde::{Deserialize, Serialize};

/// Benjamini-Hochberg FDR adjusted p-values in original input order.
pub fn benjamini_hochberg(p_values: &[f64]) -> Vec<f64> {
    let mut indexed: Vec<(usize, f64)> = p_values
        .iter()
        .copied()
        .enumerate()
        .map(|(i, p)| (i, p.clamp(0.0, 1.0)))
        .collect();
    indexed.sort_by(|a, b| a.1.total_cmp(&b.1));
    let m = indexed.len();
    let mut adjusted_sorted = vec![1.0; m];
    let mut running = 1.0_f64;
    for rank0 in (0..m).rev() {
        let rank = rank0 + 1;
        let candidate = indexed[rank0].1 * m as f64 / rank as f64;
        running = running.min(candidate).min(1.0);
        adjusted_sorted[rank0] = running;
    }
    let mut out = vec![1.0; m];
    for (rank0, &(original, _)) in indexed.iter().enumerate() {
        out[original] = adjusted_sorted[rank0];
    }
    out
}

/// Bonferroni adjusted p-values.
pub fn bonferroni(p_values: &[f64]) -> Vec<f64> {
    let m = p_values.len() as f64;
    p_values
        .iter()
        .map(|p| (p.clamp(0.0, 1.0) * m).min(1.0))
        .collect()
}

/// Holm step-down adjusted p-values.
pub fn holm(p_values: &[f64]) -> Vec<f64> {
    let mut indexed: Vec<(usize, f64)> = p_values.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| a.1.total_cmp(&b.1));
    let m = indexed.len();
    let mut out = vec![1.0; m];
    let mut running = 0.0_f64;
    for (rank0, &(original, p)) in indexed.iter().enumerate() {
        let adjusted = ((m - rank0) as f64 * p.clamp(0.0, 1.0)).min(1.0);
        running = running.max(adjusted);
        out[original] = running;
    }
    out
}

/// Interval metadata for label-aware purging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabelInterval {
    pub start: i64,
    pub end: i64,
}

impl LabelInterval {
    pub fn overlaps(self, other: Self) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}

/// Purge train indices whose label intervals overlap any test interval.
pub fn purge_overlapping_intervals(
    intervals: &[LabelInterval],
    train_indices: &[usize],
    test_indices: &[usize],
) -> Vec<usize> {
    let tests: Vec<LabelInterval> = test_indices
        .iter()
        .filter_map(|&i| intervals.get(i).copied())
        .collect();
    train_indices
        .iter()
        .copied()
        .filter(|&i| {
            intervals
                .get(i)
                .is_some_and(|candidate| !tests.iter().any(|test| candidate.overlaps(*test)))
        })
        .collect()
}

#[inline]
fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state
}

/// Deterministic block-bootstrap sample indices using a tiny LCG.
pub fn block_bootstrap_indices(n: usize, block: usize, seed: u64) -> Vec<usize> {
    if n == 0 || block == 0 {
        return Vec::new();
    }
    let mut state = seed.max(1);
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        let start = (lcg_next(&mut state) as usize) % n;
        for offset in 0..block {
            if out.len() == n {
                break;
            }
            out.push((start + offset) % n);
        }
    }
    out
}

/// Deterministic permutation of `0..n`.
pub fn permutation_indices(n: usize, seed: u64) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..n).collect();
    let mut state = seed.max(1);
    for i in (1..n).rev() {
        let j = (lcg_next(&mut state) as usize) % (i + 1);
        indices.swap(i, j);
    }
    indices
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ConfidenceInterval {
    pub estimate: f64,
    pub lower: f64,
    pub upper: f64,
    pub confidence: f64,
    pub samples: usize,
}

fn quantile_sorted(values: &[f64], probability: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let position = probability.clamp(0.0, 1.0) * (values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        values[lower]
    } else {
        let weight = position - lower as f64;
        values[lower] * (1.0 - weight) + values[upper] * weight
    }
}

/// Deterministic circular block-bootstrap confidence interval for the mean.
pub fn block_bootstrap_mean_ci(
    values: &[f64],
    block: usize,
    samples: usize,
    confidence: f64,
    seed: u64,
) -> ConfidenceInterval {
    let finite: Vec<f64> = values.iter().copied().filter(|value| value.is_finite()).collect();
    if finite.is_empty() || block == 0 || samples == 0 || !(0.0 < confidence && confidence < 1.0) {
        return ConfidenceInterval {
            estimate: 0.0,
            lower: 0.0,
            upper: 0.0,
            confidence,
            samples: 0,
        };
    }
    let estimate = finite.iter().sum::<f64>() / finite.len() as f64;
    let mut bootstrap_means = Vec::with_capacity(samples);
    for sample in 0..samples {
        let indices = block_bootstrap_indices(
            finite.len(),
            block.min(finite.len()),
            seed.wrapping_add(sample as u64 + 1),
        );
        bootstrap_means.push(
            indices.iter().map(|&index| finite[index]).sum::<f64>() / indices.len() as f64,
        );
    }
    bootstrap_means.sort_by(f64::total_cmp);
    let alpha = (1.0 - confidence) * 0.5;
    ConfidenceInterval {
        estimate,
        lower: quantile_sorted(&bootstrap_means, alpha),
        upper: quantile_sorted(&bootstrap_means, 1.0 - alpha),
        confidence,
        samples,
    }
}

/// Two-sided deterministic sign-flip randomization p-value for a zero mean.
pub fn sign_flip_mean_p_value(values: &[f64], samples: usize, seed: u64) -> f64 {
    let finite: Vec<f64> = values.iter().copied().filter(|value| value.is_finite()).collect();
    if finite.is_empty() || samples == 0 {
        return 1.0;
    }
    let observed = (finite.iter().sum::<f64>() / finite.len() as f64).abs();
    let mut state = seed.max(1);
    let mut extreme = 0usize;
    for _ in 0..samples {
        let randomized = finite
            .iter()
            .map(|value| if lcg_next(&mut state) & 1 == 0 { *value } else { -*value })
            .sum::<f64>()
            / finite.len() as f64;
        if randomized.abs() >= observed {
            extreme += 1;
        }
    }
    (extreme + 1) as f64 / (samples + 1) as f64
}

/// Fast standard-normal CDF approximation, sufficient for significance diagnostics.
fn standard_normal_cdf(x: f64) -> f64 {
    if !x.is_finite() {
        return if x.is_sign_negative() { 0.0 } else { 1.0 };
    }
    let ax = x.abs();
    let t = 1.0 / (1.0 + 0.2316419 * ax);
    let density = (-0.5 * ax * ax).exp() / (2.0 * std::f64::consts::PI).sqrt();
    let polynomial = t
        * (0.319381530
            + t * (-0.356563782
                + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
    let upper = 1.0 - density * polynomial;
    if x >= 0.0 { upper } else { 1.0 - upper }
}

fn inverse_standard_normal(probability: f64) -> f64 {
    let p = probability.clamp(1e-12, 1.0 - 1e-12);
    let mut low = -8.0_f64;
    let mut high = 8.0_f64;
    for _ in 0..80 {
        let mid = (low + high) * 0.5;
        if standard_normal_cdf(mid) < p {
            low = mid;
        } else {
            high = mid;
        }
    }
    (low + high) * 0.5
}

/// Probabilistic Sharpe Ratio (PSR).
///
/// `skew` is sample skewness and `excess_kurt` is excess kurtosis.
pub fn probabilistic_sharpe_ratio(
    observed_sharpe: f64,
    benchmark_sharpe: f64,
    observations: usize,
    skew: f64,
    excess_kurt: f64,
) -> f64 {
    if observations < 2 || !observed_sharpe.is_finite() || !benchmark_sharpe.is_finite() {
        return 0.0;
    }
    let kurtosis = excess_kurt + 3.0;
    let variance_term = 1.0 - skew * observed_sharpe
        + ((kurtosis - 1.0) / 4.0) * observed_sharpe * observed_sharpe;
    if variance_term <= f64::EPSILON || !variance_term.is_finite() {
        return 0.0;
    }
    let z = (observed_sharpe - benchmark_sharpe) * ((observations - 1) as f64).sqrt()
        / variance_term.sqrt();
    standard_normal_cdf(z).clamp(0.0, 1.0)
}

/// PSR calculated directly from per-period returns.
pub fn probabilistic_sharpe_from_returns(returns: &[f64], benchmark_sharpe: f64) -> f64 {
    let finite: Vec<f64> = returns.iter().copied().filter(|value| value.is_finite()).collect();
    if finite.len() < 2 {
        return 0.0;
    }
    let mean = finite.iter().sum::<f64>() / finite.len() as f64;
    let variance = finite.iter().map(|value| (value - mean).powi(2)).sum::<f64>()
        / (finite.len() - 1) as f64;
    if variance <= f64::EPSILON {
        return 0.0;
    }
    let sharpe = mean / variance.sqrt();
    probabilistic_sharpe_ratio(
        sharpe,
        benchmark_sharpe,
        finite.len(),
        skewness(&finite),
        excess_kurtosis(&finite),
    )
}

/// Deflated Sharpe Ratio using the observed dispersion across strategy trials.
pub fn deflated_sharpe_ratio(
    observed_sharpe: f64,
    observations: usize,
    skew: f64,
    excess_kurt: f64,
    candidate_sharpes: &[f64],
) -> f64 {
    let trials: Vec<f64> = candidate_sharpes
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect();
    if trials.len() <= 1 {
        return probabilistic_sharpe_ratio(observed_sharpe, 0.0, observations, skew, excess_kurt);
    }
    let mean = trials.iter().sum::<f64>() / trials.len() as f64;
    let variance = trials.iter().map(|value| (value - mean).powi(2)).sum::<f64>()
        / (trials.len() - 1) as f64;
    let trial_std = variance.sqrt();
    let n = trials.len() as f64;
    const EULER_GAMMA: f64 = 0.5772156649015329;
    let expected_max = trial_std
        * ((1.0 - EULER_GAMMA) * inverse_standard_normal(1.0 - 1.0 / n)
            + EULER_GAMMA * inverse_standard_normal(1.0 - 1.0 / (n * std::f64::consts::E)));
    probabilistic_sharpe_ratio(
        observed_sharpe,
        expected_max,
        observations,
        skew,
        excess_kurt,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PboResult {
    pub probability: f64,
    pub evaluated_splits: usize,
    pub overfit_splits: usize,
    pub mean_oos_rank: f64,
}

/// Probability of Backtest Overfitting from paired in-sample/out-of-sample score matrices.
///
/// Each row is one split and each column is one candidate configuration. The
/// best in-sample candidate is overfit when its OOS percentile rank is at or
/// below the median of the candidate set.
pub fn probability_of_backtest_overfitting(
    in_sample_scores: &[Vec<f64>],
    out_of_sample_scores: &[Vec<f64>],
) -> PboResult {
    let rows = in_sample_scores.len().min(out_of_sample_scores.len());
    let mut evaluated = 0usize;
    let mut overfit = 0usize;
    let mut rank_sum = 0.0_f64;
    for row in 0..rows {
        let is = &in_sample_scores[row];
        let oos = &out_of_sample_scores[row];
        let width = is.len().min(oos.len());
        if width < 2 {
            continue;
        }
        let Some((best_index, _)) = is
            .iter()
            .take(width)
            .enumerate()
            .filter(|(_, value)| value.is_finite())
            .max_by(|a, b| a.1.total_cmp(b.1))
        else {
            continue;
        };
        let selected = oos[best_index];
        if !selected.is_finite() {
            continue;
        }
        let valid: Vec<f64> = oos.iter().take(width).copied().filter(|v| v.is_finite()).collect();
        if valid.len() < 2 {
            continue;
        }
        let below_or_equal = valid.iter().filter(|value| **value <= selected).count();
        let percentile = (below_or_equal as f64 - 0.5) / valid.len() as f64;
        evaluated += 1;
        rank_sum += percentile;
        if percentile <= 0.5 {
            overfit += 1;
        }
    }
    PboResult {
        probability: if evaluated == 0 {
            0.0
        } else {
            overfit as f64 / evaluated as f64
        },
        evaluated_splits: evaluated,
        overfit_splits: overfit,
        mean_oos_rank: if evaluated == 0 {
            0.0
        } else {
            rank_sum / evaluated as f64
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiple_testing_adjustments_stay_in_probability_range() {
        let p = [0.01, 0.04, 0.03, 0.2];
        let bh = benjamini_hochberg(&p);
        let bonf = bonferroni(&p);
        let holm_values = holm(&p);
        assert!(bh.iter().chain(&bonf).chain(&holm_values).all(|v| (0.0..=1.0).contains(v)));
        assert!(bh[0] <= bh[3]);
    }

    #[test]
    fn interval_purge_removes_overlap() {
        let intervals = [
            LabelInterval { start: 0, end: 5 },
            LabelInterval { start: 6, end: 8 },
            LabelInterval { start: 4, end: 7 },
        ];
        assert_eq!(
            purge_overlapping_intervals(&intervals, &[0, 1], &[2]),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn bootstrap_ci_and_sign_flip_are_deterministic() {
        let values = [0.01, 0.02, -0.01, 0.03, 0.015, 0.005];
        let a = block_bootstrap_mean_ci(&values, 2, 200, 0.95, 42);
        let b = block_bootstrap_mean_ci(&values, 2, 200, 0.95, 42);
        assert_eq!(a, b);
        assert!(a.lower <= a.estimate && a.estimate <= a.upper);
        let p = sign_flip_mean_p_value(&values, 200, 7);
        assert!((0.0..=1.0).contains(&p));
    }

    #[test]
    fn probabilistic_and_deflated_sharpe_are_probabilities() {
        let psr = probabilistic_sharpe_ratio(0.8, 0.0, 252, 0.1, 0.5);
        let dsr = deflated_sharpe_ratio(0.8, 252, 0.1, 0.5, &[0.1, 0.2, 0.3, 0.8]);
        assert!(psr > 0.5 && psr <= 1.0);
        assert!((0.0..=1.0).contains(&dsr));
        assert!(dsr <= psr);
    }

    #[test]
    fn pbo_detects_bad_oos_selection() {
        let in_sample = vec![vec![3.0, 2.0, 1.0], vec![1.0, 3.0, 2.0]];
        let out_of_sample = vec![vec![0.0, 1.0, 2.0], vec![2.0, 0.0, 1.0]];
        let result = probability_of_backtest_overfitting(&in_sample, &out_of_sample);
        assert_eq!(result.evaluated_splits, 2);
        assert!(result.probability > 0.5);
    }
}