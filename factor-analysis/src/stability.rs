use finkit::features::{psi, BinningMethod};
use serde::{Deserialize, Serialize};

/// Rolling mean that ignores non-finite observations.
pub fn rolling_mean(values: &[f64], window: usize) -> Vec<f64> {
    let mut out = vec![f64::NAN; values.len()];
    if window == 0 { return out; }
    for i in window.saturating_sub(1)..values.len() {
        let start = i + 1 - window;
        let valid: Vec<f64> = values[start..=i].iter().copied().filter(|v| v.is_finite()).collect();
        if !valid.is_empty() { out[i] = valid.iter().sum::<f64>() / valid.len() as f64; }
    }
    out
}

/// Estimate a factor IC half-life from an IC decay curve.
pub fn estimate_half_life(periods: &[usize], ic: &[f64]) -> f64 {
    let pairs: Vec<(f64, f64)> = periods.iter().copied().zip(ic.iter().copied())
        .filter(|(_, value)| value.is_finite() && *value > 0.0)
        .map(|(period, value)| (period as f64, value.ln()))
        .collect();
    if pairs.len() < 2 { return f64::NAN; }
    let n = pairs.len() as f64;
    let mx = pairs.iter().map(|v| v.0).sum::<f64>() / n;
    let my = pairs.iter().map(|v| v.1).sum::<f64>() / n;
    let cov = pairs.iter().map(|(x,y)| (x-mx)*(y-my)).sum::<f64>();
    let var = pairs.iter().map(|(x,_)| (x-mx).powi(2)).sum::<f64>();
    if var <= f64::EPSILON { return f64::NAN; }
    let slope = cov / var;
    if slope >= 0.0 { f64::INFINITY } else { -std::f64::consts::LN_2 / slope }
}

/// Reuse the existing PSI implementation for factor distribution drift.
pub fn factor_psi(baseline: &[f64], current: &[f64], bins: usize) -> f64 {
    psi(baseline, current, bins, BinningMethod::EqualFrequency)
}

/// Compact factor health summary suitable for live/incremental monitoring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactorHealth {
    pub mean_ic: f64,
    pub recent_ic: f64,
    pub psi: f64,
    pub coverage: f64,
    pub degraded: bool,
}

pub fn factor_health(
    ic: &[f64],
    baseline_factor: &[f64],
    current_factor: &[f64],
    recent_window: usize,
    psi_threshold: f64,
) -> FactorHealth {
    let valid_ic: Vec<f64> = ic.iter().copied().filter(|v| v.is_finite()).collect();
    let mean_ic = if valid_ic.is_empty() { f64::NAN } else { valid_ic.iter().sum::<f64>() / valid_ic.len() as f64 };
    let recent: Vec<f64> = ic.iter().rev().copied().filter(|v| v.is_finite()).take(recent_window).collect();
    let recent_ic = if recent.is_empty() { f64::NAN } else { recent.iter().sum::<f64>() / recent.len() as f64 };
    let drift = factor_psi(baseline_factor, current_factor, 10);
    let coverage = if current_factor.is_empty() { 0.0 } else { current_factor.iter().filter(|v| v.is_finite()).count() as f64 / current_factor.len() as f64 };
    let degraded = drift.is_finite() && drift > psi_threshold || (recent_ic.is_finite() && mean_ic.is_finite() && recent_ic < mean_ic * 0.5);
    FactorHealth { mean_ic, recent_ic, psi: drift, coverage, degraded }
}
