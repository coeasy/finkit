use crate::error::{ResearchError, ResearchResult};
use finkit::math::regression::ols;
use serde::{Deserialize, Serialize};

/// Newey-West/HAC summary for a scalar time series mean.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HacMeanResult {
    pub mean: f64,
    pub standard_error: f64,
    pub t_stat: f64,
    pub lags: usize,
    pub observations: usize,
}

/// Estimate the mean with Bartlett-kernel Newey-West standard errors.
pub fn newey_west_mean(values: &[f64], lags: usize) -> HacMeanResult {
    let x: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    let n = x.len();
    if n == 0 {
        return HacMeanResult {
            mean: f64::NAN,
            standard_error: f64::NAN,
            t_stat: f64::NAN,
            lags,
            observations: 0,
        };
    }
    let mean = x.iter().sum::<f64>() / n as f64;
    let centered: Vec<f64> = x.iter().map(|v| v - mean).collect();
    let gamma0 = centered.iter().map(|v| v * v).sum::<f64>() / n as f64;
    let max_lag = lags.min(n.saturating_sub(1));
    let mut long_run_var = gamma0;
    for lag in 1..=max_lag {
        let gamma = centered[lag..]
            .iter()
            .zip(&centered[..n - lag])
            .map(|(a, b)| a * b)
            .sum::<f64>()
            / n as f64;
        let weight = 1.0 - lag as f64 / (max_lag + 1) as f64;
        long_run_var += 2.0 * weight * gamma;
    }
    let standard_error = (long_run_var.max(0.0) / n as f64).sqrt();
    let t_stat = if standard_error > 1e-15 {
        mean / standard_error
    } else {
        f64::NAN
    };
    HacMeanResult {
        mean,
        standard_error,
        t_stat,
        lags: max_lag,
        observations: n,
    }
}

/// Cross-sectional risk-model fit for one date.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossSectionalRiskFit {
    pub intercept: f64,
    pub factor_returns: Vec<f64>,
    pub specific_returns: Vec<f64>,
    pub r_squared: f64,
}

pub fn fit_cross_sectional_risk_model(
    asset_returns: &[f64],
    exposures: &[&[f64]],
) -> ResearchResult<CrossSectionalRiskFit> {
    if exposures.is_empty() {
        return Err(ResearchError::InvalidConfig(
            "risk model requires exposures".to_string(),
        ));
    }
    let fit = ols(asset_returns, exposures)?;
    Ok(CrossSectionalRiskFit {
        intercept: fit.intercept,
        factor_returns: fit.coefficients,
        specific_returns: fit.residuals,
        r_squared: fit.r_squared,
    })
}

/// Herfindahl-Hirschman concentration of normalized absolute weights.
///
/// This facade is retained for compatibility; the numerical owner lives in
/// `finkit::performance` and is shared with backtest/portfolio evaluation.
pub fn hhi(weights: &[f64]) -> f64 {
    finkit::performance::evaluate_portfolio(weights).hhi
}

/// Effective number of independent weight bets, `1 / HHI`.
pub fn effective_number_of_bets(weights: &[f64]) -> f64 {
    finkit::performance::evaluate_portfolio(weights).effective_number_of_bets
}

/// Exposure of a portfolio to each factor column.
pub fn portfolio_factor_exposure(
    weights: &[f64],
    exposures: &[&[f64]],
) -> ResearchResult<Vec<f64>> {
    if exposures.iter().any(|col| col.len() != weights.len()) {
        return Err(ResearchError::LengthMismatch {
            name: "exposure".to_string(),
            expected: weights.len(),
            actual: exposures
                .iter()
                .map(|c| c.len())
                .find(|&n| n != weights.len())
                .unwrap_or(0),
        });
    }
    Ok(exposures
        .iter()
        .map(|column| {
            weights
                .iter()
                .zip(*column)
                .filter(|(w, x)| w.is_finite() && x.is_finite())
                .map(|(w, x)| w * x)
                .sum()
        })
        .collect())
}

/// Contribution to variance under factor covariance plus diagonal specific variance.
pub fn variance_attribution(
    factor_exposure: &[f64],
    factor_covariance: &[Vec<f64>],
    asset_weights: &[f64],
    specific_variance: &[f64],
) -> ResearchResult<(f64, f64)> {
    if factor_covariance.len() != factor_exposure.len()
        || factor_covariance
            .iter()
            .any(|row| row.len() != factor_exposure.len())
    {
        return Err(ResearchError::InvalidConfig(
            "factor covariance shape mismatch".to_string(),
        ));
    }
    if asset_weights.len() != specific_variance.len() {
        return Err(ResearchError::LengthMismatch {
            name: "specific_variance".to_string(),
            expected: asset_weights.len(),
            actual: specific_variance.len(),
        });
    }
    let mut factor_var = 0.0;
    for i in 0..factor_exposure.len() {
        for j in 0..factor_exposure.len() {
            factor_var += factor_exposure[i] * factor_covariance[i][j] * factor_exposure[j];
        }
    }
    let specific_var = asset_weights
        .iter()
        .zip(specific_variance)
        .map(|(w, v)| w * w * v.max(0.0))
        .sum();
    Ok((factor_var.max(0.0), specific_var))
}
