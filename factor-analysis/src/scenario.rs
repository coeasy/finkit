use crate::error::{ResearchError, ResearchResult};
use serde::{Deserialize, Serialize};

/// Deterministic factor shock scenario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactorShockScenario {
    pub shocks: Vec<f64>,
}

/// Portfolio PnL approximation from factor exposures and factor shocks.
pub fn factor_shock_pnl(exposures: &[f64], scenario: &FactorShockScenario) -> ResearchResult<f64> {
    if exposures.len() != scenario.shocks.len() {
        return Err(ResearchError::LengthMismatch {
            name: "scenario shocks".to_string(),
            expected: exposures.len(),
            actual: scenario.shocks.len(),
        });
    }
    Ok(exposures.iter().zip(&scenario.shocks).map(|(exposure, shock)| exposure * shock).sum())
}

/// Apply a multiplicative volatility stress to a covariance matrix.
pub fn volatility_stress(covariance: &[Vec<f64>], multiplier: f64) -> ResearchResult<Vec<Vec<f64>>> {
    if multiplier < 0.0 || !multiplier.is_finite() {
        return Err(ResearchError::InvalidConfig("volatility multiplier must be finite and >= 0".to_string()));
    }
    if covariance.iter().any(|row| row.len() != covariance.len()) {
        return Err(ResearchError::InvalidConfig("covariance matrix must be square".to_string()));
    }
    let scale = multiplier * multiplier;
    Ok(covariance.iter().map(|row| row.iter().map(|value| value * scale).collect()).collect())
}

/// Blend off-diagonal correlations toward a stressed common-correlation level while preserving variances.
pub fn correlation_stress(covariance: &[Vec<f64>], target_correlation: f64) -> ResearchResult<Vec<Vec<f64>>> {
    if !(-1.0..=1.0).contains(&target_correlation) || covariance.iter().any(|row| row.len() != covariance.len()) {
        return Err(ResearchError::InvalidConfig("invalid correlation stress".to_string()));
    }
    let n = covariance.len();
    let stds: Vec<f64> = (0..n).map(|i| covariance[i][i].max(0.0).sqrt()).collect();
    let mut out = covariance.to_vec();
    for i in 0..n {
        for j in 0..n {
            if i != j { out[i][j] = target_correlation * stds[i] * stds[j]; }
        }
    }
    Ok(out)
}
