//! Volatility regime classification and market regime detection.
//!
//! Provides threshold-based classification, a simplified Gaussian HMM with EM
//! parameter estimation and Viterbi decoding, and regime change signals.

use ndarray::Array1;

use super::{Feature, FeatureEngine, FeatureMatrix};
use crate::math::statistics::rolling_std_dev;

/// Detected regime change event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegimeChange {
    /// Index where the regime switched.
    pub index: usize,
    /// Previous regime state.
    pub from_state: u8,
    /// New regime state.
    pub to_state: u8,
}

/// Output of [`hmm_regime`]: decoded states and fitted Gaussian parameters.
#[derive(Debug, Clone)]
pub struct HmmResult {
    /// Most likely state sequence (Viterbi decode).
    pub states: Array1<f64>,
    /// Per-timestep posterior state probabilities from the final EM E-step.
    pub state_probs: Vec<Vec<f64>>,
    /// Gaussian emission means per state.
    pub means: Vec<f64>,
    /// Gaussian emission standard deviations per state.
    pub stds: Vec<f64>,
}

/// Threshold-based volatility regime classifier.
///
/// Computes rolling standard deviation of log returns, then assigns each bar to
/// low (0), medium (1), or high (2) volatility using global percentile thresholds.
///
/// # Arguments
///
/// * `data` - Close price series (or any positive price-like series).
/// * `window` - Rolling window for volatility estimation.
/// * `low_pct` - Percentile below which regime is classified as low (e.g. 25.0).
/// * `high_pct` - Percentile above which regime is classified as high (e.g. 75.0).
pub fn threshold_regime(
    data: &[f64],
    window: usize,
    low_pct: f64,
    high_pct: f64,
) -> Array1<f64> {
    let len = data.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if len < 2 || window < 2 {
        return out;
    }

    let returns = log_returns(data);
    let vol = rolling_std_dev(returns.as_slice().unwrap_or(&[]), window)
        .unwrap_or_else(|_| Array1::from_elem(len, f64::NAN));

    let valid: Vec<f64> = vol.iter().copied().filter(|v| v.is_finite() && *v >= 0.0).collect();
    if valid.is_empty() {
        return out;
    }

    let low_threshold = percentile(&valid, low_pct);
    let high_threshold = percentile(&valid, high_pct);

    for (i, &v) in vol.iter().enumerate() {
        if !v.is_finite() {
            continue;
        }
        out[i] = if v <= low_threshold {
            0.0
        } else if v >= high_threshold {
            2.0
        } else {
            1.0
        };
    }
    out
}

/// Fit a simplified Gaussian HMM and decode the most likely regime sequence.
///
/// Uses rolling return volatility as the emission feature. Parameters are estimated
/// with EM (up to `max_iter` iterations), then states are decoded via the Viterbi
/// algorithm. State 0 corresponds to low volatility, state 1 to high volatility
/// when `n_states == 2`.
pub fn hmm_regime(data: &[f64], n_states: usize, max_iter: usize) -> HmmResult {
    let empty = HmmResult {
        states: Array1::from_elem(data.len(), f64::NAN),
        state_probs: Vec::new(),
        means: Vec::new(),
        stds: Vec::new(),
    };

    if data.len() < 3 || n_states < 1 || max_iter == 0 {
        return empty;
    }

    let vol_window = 10.min(data.len() / 2).max(3);
    let returns = log_returns(data);
    let vol = rolling_std_dev(returns.as_slice().unwrap_or(&[]), vol_window)
        .unwrap_or_else(|_| Array1::from_elem(data.len(), f64::NAN));
    let observations: Vec<f64> = vol.iter().copied().filter(|v| v.is_finite()).collect();

    if observations.len() < n_states {
        return empty;
    }

    let (means, stds, start_prob, trans) = em_gaussian_hmm(&observations, n_states, max_iter);
    let (states_idx, _) = viterbi(&observations, &means, &stds, &start_prob, &trans);

    let mut states = Array1::from_elem(data.len(), f64::NAN);
    let mut obs_idx = 0;
    for (i, &v) in vol.iter().enumerate() {
        if v.is_finite() {
            if obs_idx < states_idx.len() {
                states[i] = states_idx[obs_idx] as f64;
            }
            obs_idx += 1;
        }
    }

    let state_probs = forward_backward_probs(&observations, &means, &stds, &start_prob, &trans);

    HmmResult {
        states,
        state_probs,
        means,
        stds,
    }
}

/// Detect regime switch points in a state sequence.
///
/// Ignores NaN states and only reports transitions between consecutive valid states.
pub fn regime_signal(states: &[f64]) -> Vec<RegimeChange> {
    let mut changes = Vec::new();
    let mut prev: Option<u8> = None;

    for (i, &state) in states.iter().enumerate() {
        if !state.is_finite() {
            continue;
        }
        let current = state.round().clamp(0.0, 255.0) as u8;
        if let Some(from) = prev {
            if current != from {
                changes.push(RegimeChange {
                    index: i,
                    from_state: from,
                    to_state: current,
                });
            }
        }
        prev = Some(current);
    }
    changes
}

/// Feature engine wrapping [`threshold_regime`] with regime change markers.
pub struct RegimeFeature {
    window: usize,
    low_pct: f64,
    high_pct: f64,
}

impl RegimeFeature {
    /// Create a threshold regime feature generator.
    pub fn new(window: usize, low_pct: f64, high_pct: f64) -> Self {
        Self {
            window,
            low_pct,
            high_pct,
        }
    }

    /// Default 20-bar window with 25th/75th percentile thresholds.
    pub fn default_threshold() -> Self {
        Self::new(20, 25.0, 75.0)
    }
}

impl FeatureEngine for RegimeFeature {
    fn generate(&self, close: &[f64]) -> FeatureMatrix {
        let regimes = threshold_regime(close, self.window, self.low_pct, self.high_pct);
        let changes = regime_signal(regimes.as_slice().unwrap_or(&[]));

        let mut change_flags = vec![0.0; close.len()];
        for change in &changes {
            if change.index < close.len() {
                change_flags[change.index] = 1.0;
            }
        }

        let mut matrix = FeatureMatrix::with_capacity(close.len(), 2);
        matrix.add_column(
            Feature::new(
                format!("regime_w{}_{}_{}", self.window, self.low_pct as u32, self.high_pct as u32),
                "regime",
                self.window,
            ),
            regimes.to_vec(),
        );
        matrix.add_column(
            Feature::new(
                format!("regime_change_w{}", self.window),
                "regime",
                self.window,
            ),
            change_flags,
        );
        matrix
    }

    fn feature_names(&self) -> Vec<String> {
        vec![
            format!(
                "regime_w{}_{}_{}",
                self.window, self.low_pct as u32, self.high_pct as u32
            ),
            format!("regime_change_w{}", self.window),
        ]
    }
}

fn log_returns(data: &[f64]) -> Array1<f64> {
    let len = data.len();
    let mut out = Array1::from_elem(len, 0.0);
    for i in 1..len {
        if data[i - 1] > 0.0 && data[i] > 0.0 {
            out[i] = (data[i] / data[i - 1]).ln();
        }
    }
    out
}

fn percentile(values: &[f64], pct: f64) -> f64 {
    let mut sorted: Vec<f64> = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    if n == 1 {
        return sorted[0];
    }
    let rank = (pct / 100.0) * (n - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let frac = rank - lo as f64;
        sorted[lo] * (1.0 - frac) + sorted[hi] * frac
    }
}

fn gaussian_pdf(x: f64, mean: f64, std: f64) -> f64 {
    let s = std.max(1e-8);
    let z = (x - mean) / s;
    (-0.5 * z * z).exp() / (s * (2.0 * std::f64::consts::PI).sqrt())
}

fn em_gaussian_hmm(
    obs: &[f64],
    n_states: usize,
    max_iter: usize,
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<Vec<f64>>) {
    let n = obs.len();
    let sorted = {
        let mut v = obs.to_vec();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        v
    };

    let mut means: Vec<f64> = (0..n_states)
        .map(|k| {
            let idx = ((k as f64 + 0.5) / n_states as f64 * n as f64) as usize;
            sorted[idx.min(n - 1)]
        })
        .collect();

    let global_std = {
        let m = obs.iter().sum::<f64>() / n as f64;
        let var = obs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n as f64;
        var.sqrt().max(1e-6)
    };
    let mut stds = vec![global_std; n_states];
    let mut start_prob = vec![1.0 / n_states as f64; n_states];
    let mut trans = vec![vec![1.0 / n_states as f64; n_states]; n_states];

    for _ in 0..max_iter {
        let (alpha, scale) = forward_scaled(obs, &means, &stds, &start_prob, &trans);
        let beta = backward_scaled(obs, &means, &stds, &trans, &scale);
        let gamma = compute_gamma(&alpha, &beta);
        let xi = compute_xi(obs, &alpha, &beta, &means, &stds, &trans, &scale);

        for i in 0..n_states {
            let weight: f64 = gamma.iter().map(|g| g[i]).sum();
            if weight > 1e-12 {
                means[i] = gamma
                    .iter()
                    .zip(obs.iter())
                    .map(|(g, &x)| g[i] * x)
                    .sum::<f64>()
                    / weight;
                let var = gamma
                    .iter()
                    .zip(obs.iter())
                    .map(|(g, &x)| g[i] * (x - means[i]).powi(2))
                    .sum::<f64>()
                    / weight;
                stds[i] = var.sqrt().max(1e-6);
            }
        }

        start_prob[..n_states].copy_from_slice(&gamma[0][..n_states]);

        for i in 0..n_states {
            let denom: f64 = (0..n - 1).map(|t| xi[t][i].iter().sum::<f64>()).sum();
            for j in 0..n_states {
                let numer: f64 = (0..n - 1).map(|t| xi[t][i][j]).sum();
                trans[i][j] = if denom > 1e-12 {
                    numer / denom
                } else {
                    1.0 / n_states as f64
                };
            }
        }
    }

    (means, stds, start_prob, trans)
}

fn emission_matrix(obs: &[f64], means: &[f64], stds: &[f64]) -> Vec<Vec<f64>> {
    let n_states = means.len();
    obs.iter()
        .map(|&x| {
            (0..n_states)
                .map(|s| gaussian_pdf(x, means[s], stds[s]))
                .collect()
        })
        .collect()
}

fn forward_scaled(
    obs: &[f64],
    means: &[f64],
    stds: &[f64],
    start_prob: &[f64],
    trans: &[Vec<f64>],
) -> (Vec<Vec<f64>>, Vec<f64>) {
    let n = obs.len();
    let n_states = means.len();
    let emit = emission_matrix(obs, means, stds);
    let mut alpha = vec![vec![0.0; n_states]; n];
    let mut scale = vec![0.0; n];

    for (i, ai) in alpha[0].iter_mut().enumerate().take(n_states) {
        *ai = start_prob[i] * emit[0][i];
        scale[0] += *ai;
    }
    if scale[0] > 0.0 {
        for v in alpha[0].iter_mut().take(n_states) {
            *v /= scale[0];
        }
    }

    for t in 1..n {
        scale[t] = 0.0;
        for j in 0..n_states {
            let mut sum = 0.0;
            for i in 0..n_states {
                sum += alpha[t - 1][i] * trans[i][j];
            }
            alpha[t][j] = sum * emit[t][j];
            scale[t] += alpha[t][j];
        }
        if scale[t] > 0.0 {
            for v in alpha[t].iter_mut().take(n_states) {
                *v /= scale[t];
            }
        }
    }
    (alpha, scale)
}

fn backward_scaled(
    obs: &[f64],
    means: &[f64],
    stds: &[f64],
    trans: &[Vec<f64>],
    scale: &[f64],
) -> Vec<Vec<f64>> {
    let n = obs.len();
    let n_states = means.len();
    let emit = emission_matrix(obs, means, stds);
    let mut beta = vec![vec![0.0; n_states]; n];

    for v in beta[n - 1].iter_mut().take(n_states) {
        *v = 1.0;
    }

    for t in (0..n - 1).rev() {
        for i in 0..n_states {
            let mut sum = 0.0;
            for j in 0..n_states {
                sum += trans[i][j] * emit[t + 1][j] * beta[t + 1][j];
            }
            beta[t][i] = sum;
        }
        if scale[t + 1] > 0.0 {
            for v in beta[t].iter_mut().take(n_states) {
                *v /= scale[t + 1];
            }
        }
    }
    beta
}

fn compute_gamma(alpha: &[Vec<f64>], beta: &[Vec<f64>]) -> Vec<Vec<f64>> {
    alpha
        .iter()
        .zip(beta.iter())
        .map(|(a, b)| {
            let mut g: Vec<f64> = a.iter().zip(b.iter()).map(|(&ai, &bi)| ai * bi).collect();
            let sum: f64 = g.iter().sum();
            if sum > 0.0 {
                for v in &mut g {
                    *v /= sum;
                }
            }
            g
        })
        .collect()
}

fn compute_xi(
    obs: &[f64],
    alpha: &[Vec<f64>],
    beta: &[Vec<f64>],
    means: &[f64],
    stds: &[f64],
    trans: &[Vec<f64>],
    scale: &[f64],
) -> Vec<Vec<Vec<f64>>> {
    let n = obs.len();
    let n_states = means.len();
    let emit = emission_matrix(obs, means, stds);
    let mut xi = vec![vec![vec![0.0; n_states]; n_states]; n.saturating_sub(1)];

    for t in 0..n.saturating_sub(1) {
        let mut denom = 0.0;
        for i in 0..n_states {
            for j in 0..n_states {
                let val = alpha[t][i] * trans[i][j] * emit[t + 1][j] * beta[t + 1][j];
                xi[t][i][j] = val;
                denom += val;
            }
        }
        if denom > 0.0 {
            for row in xi[t].iter_mut().take(n_states) {
                for v in row.iter_mut().take(n_states) {
                    *v /= denom;
                }
            }
        } else if scale[t] > 0.0 && scale[t + 1] > 0.0 {
            let inv = 1.0 / (scale[t] * scale[t + 1]);
            for (i, row) in xi[t].iter_mut().enumerate().take(n_states) {
                for (j, v) in row.iter_mut().enumerate().take(n_states) {
                    *v = alpha[t][i] * trans[i][j] * emit[t + 1][j] * beta[t + 1][j] * inv;
                }
            }
        }
    }
    xi
}

fn forward_backward_probs(
    obs: &[f64],
    means: &[f64],
    stds: &[f64],
    start_prob: &[f64],
    trans: &[Vec<f64>],
) -> Vec<Vec<f64>> {
    let (alpha, _) = forward_scaled(obs, means, stds, start_prob, trans);
    let beta = backward_scaled(obs, means, stds, trans, &vec![1.0; obs.len()]);
    compute_gamma(&alpha, &beta)
}

fn viterbi(
    obs: &[f64],
    means: &[f64],
    stds: &[f64],
    start_prob: &[f64],
    trans: &[Vec<f64>],
) -> (Vec<usize>, f64) {
    let n = obs.len();
    let n_states = means.len();
    if n == 0 {
        return (Vec::new(), f64::NEG_INFINITY);
    }

    let emit = emission_matrix(obs, means, stds);
    let mut delta = vec![vec![f64::NEG_INFINITY; n_states]; n];
    let mut psi = vec![vec![0usize; n_states]; n];

    for i in 0..n_states {
        let e = emit[0][i].max(1e-300);
        let s = start_prob[i].max(1e-300);
        delta[0][i] = e.ln() + s.ln();
    }

    for t in 1..n {
        for j in 0..n_states {
            let e = emit[t][j].max(1e-300);
            for i in 0..n_states {
                let score = delta[t - 1][i] + trans[i][j].max(1e-300).ln() + e.ln();
                if score > delta[t][j] {
                    delta[t][j] = score;
                    psi[t][j] = i;
                }
            }
        }
    }

    let mut path = vec![0usize; n];
    path[n - 1] = (0..n_states)
        .max_by(|&a, &b| {
            delta[n - 1][a]
                .partial_cmp(&delta[n - 1][b])
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(0);

    for t in (1..n).rev() {
        path[t - 1] = psi[t][path[t]];
    }

    let final_log_prob = delta[n - 1][path[n - 1]];
    (path, final_log_prob)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oscillating_prices(n: usize, low_amp: f64, high_amp: f64, switch_at: usize) -> Vec<f64> {
        (0..n)
            .map(|i| {
                let amp = if i < switch_at { low_amp } else { high_amp };
                100.0 + amp * (i as f64 * 0.3).sin()
            })
            .collect()
    }

    #[test]
    fn test_threshold_regime_constant_low_vol() {
        let data = vec![100.0; 30];
        let result = threshold_regime(&data, 5, 25.0, 75.0);
        for i in 5..30 {
            assert_eq!(result[i], 0.0, "constant series should be low regime at {i}");
        }
    }

    #[test]
    fn test_threshold_regime_high_vol_segment() {
        let mut data: Vec<f64> = vec![100.0; 40];
        for i in 0..40 {
            data.push(100.0 + if i % 2 == 0 { 15.0 } else { -15.0 });
        }
        let result = threshold_regime(&data, 10, 25.0, 75.0);
        let valid: Vec<f64> = result.iter().copied().filter(|v| v.is_finite()).collect();
        assert!(valid.contains(&0.0));
        assert!(valid.contains(&2.0));
    }

    #[test]
    fn test_hmm_regime_two_states() {
        let data = oscillating_prices(80, 0.5, 8.0, 40);
        let result = hmm_regime(&data, 2, 50);
        let valid: Vec<f64> = result.states.iter().copied().filter(|v| v.is_finite()).collect();
        assert_eq!(valid.len(), data.len() - 9);
        assert_eq!(result.means.len(), 2);
        assert_eq!(result.stds.len(), 2);
        assert!(result.means[0].abs() <= result.means[1].abs() + 0.05 || result.means[1] <= result.means[0]);
    }

    #[test]
    fn test_hmm_regime_state_probs_shape() {
        let data = oscillating_prices(50, 1.0, 6.0, 25);
        let result = hmm_regime(&data, 2, 30);
        assert_eq!(result.state_probs.len(), data.len() - 9);
        for probs in &result.state_probs {
            assert_eq!(probs.len(), 2);
            let sum: f64 = probs.iter().sum();
            assert!((sum - 1.0).abs() < 0.01 || sum == 0.0);
        }
    }

    #[test]
    fn test_regime_signal_detects_changes() {
        let states = vec![0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 2.0];
        let changes = regime_signal(&states);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0], RegimeChange { index: 2, from_state: 0, to_state: 1 });
        assert_eq!(changes[1], RegimeChange { index: 4, from_state: 1, to_state: 2 });
    }

    #[test]
    fn test_regime_signal_ignores_nan() {
        let states = vec![f64::NAN, 0.0, 0.0, 1.0];
        let changes = regime_signal(&states);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].index, 3);
    }

    #[test]
    fn test_regime_feature_generates_columns() {
        let close: Vec<f64> = (0..40).map(|i| 100.0 + (i as f64 * 0.2).sin() * (1.0 + i as f64 / 10.0)).collect();
        let engine = RegimeFeature::default_threshold();
        let matrix = engine.generate(&close);
        assert_eq!(matrix.cols(), 2);
        assert_eq!(matrix.rows(), close.len());
        assert_eq!(engine.feature_names().len(), 2);
    }

    #[test]
    fn test_regime_feature_change_flags() {
        let states = vec![0.0, 0.0, 1.0, 1.0, 2.0];
        let changes = regime_signal(&states);
        let mut flags = vec![0.0; states.len()];
        for c in &changes {
            flags[c.index] = 1.0;
        }
        assert_eq!(flags, vec![0.0, 0.0, 1.0, 0.0, 1.0]);
    }
}
