//! GARCH(1,1) conditional volatility forecasting and regime transition matrices.

/// Validate GARCH(1,1) parameters: non-negative and strictly stationary (α + β < 1).
fn garch_params_valid(alpha: f64, beta: f64) -> bool {
    alpha >= 0.0 && beta >= 0.0 && alpha + beta < 1.0
}

/// Sample variance of a slice (population formula, divisor n).
fn sample_variance(data: &[f64]) -> f64 {
    let n = data.len();
    if n == 0 {
        return 0.0;
    }
    let mean = data.iter().sum::<f64>() / n as f64;
    data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64
}

/// GARCH(1,1) conditional variance path and multi-step volatility forecasts.
///
/// At each time step `t`, updates conditional variance
/// `σ²[t] = ω + α·r[t-1]² + β·σ²[t-1]` with
/// `ω = long_run_variance · (1 - α - β)` and `long_run_variance` equal to the
/// sample variance of `returns`. Then forecasts `horizon` steps ahead:
///
/// `σ²_forecast = ω/(1-α-β) + (α+β)^horizon · (σ²[t] - ω/(1-α-β))`
///
/// Returns `sqrt(σ²_forecast)` per time step. Invalid parameters (`α + β ≥ 1`,
/// negative `α` or `β`) yield an empty vector.
pub fn garch_forecast(returns: &[f64], alpha: f64, beta: f64, horizon: usize) -> Vec<f64> {
    if returns.is_empty() || !garch_params_valid(alpha, beta) {
        return Vec::new();
    }

    let long_run_variance = sample_variance(returns);
    let omega = long_run_variance * (1.0 - alpha - beta);
    let unconditional = long_run_variance;
    let persistence_pow = (alpha + beta).powi(horizon as i32);

    let mut sigma2 = long_run_variance;
    let mut forecasts = Vec::with_capacity(returns.len());

    for t in 0..returns.len() {
        if t > 0 {
            let r_prev = returns[t - 1];
            sigma2 = omega + alpha * r_prev.powi(2) + beta * sigma2;
        }
        let sigma2_forecast = unconditional + persistence_pow * (sigma2 - unconditional);
        let vol = sigma2_forecast.max(0.0).sqrt();
        forecasts.push(vol);
    }

    forecasts
}

/// Empirical regime transition probability matrix from an observed state sequence.
///
/// Entry `[i][j]` is `P(regime_j | regime_i)` estimated from consecutive pairs.
/// Rows sum to 1.0; states with no outgoing transitions receive a uniform row.
pub fn regime_transition_matrix(regimes: &[usize], n_states: usize) -> Vec<Vec<f64>> {
    if n_states == 0 {
        return Vec::new();
    }

    let uniform = 1.0 / n_states as f64;
    let mut counts = vec![vec![0usize; n_states]; n_states];

    for window in regimes.windows(2) {
        let from = window[0];
        let to = window[1];
        if from < n_states && to < n_states {
            counts[from][to] += 1;
        }
    }

    (0..n_states)
        .map(|i| {
            let row_sum: usize = counts[i].iter().sum();
            if row_sum == 0 {
                vec![uniform; n_states]
            } else {
                counts[i]
                    .iter()
                    .map(|&c| c as f64 / row_sum as f64)
                    .collect()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_garch_forecast_basic() {
        let returns: Vec<f64> = (0..50)
            .map(|i| {
                let x = (i as f64 * 0.3).sin();
                x * 0.01 + (i as f64 * 0.17).cos() * 0.005
            })
            .collect();
        let forecasts = garch_forecast(&returns, 0.1, 0.85, 5);
        assert_eq!(forecasts.len(), returns.len());
        for &v in &forecasts {
            assert!(v.is_finite());
            assert!(v > 0.0);
        }
    }

    #[test]
    fn test_garch_alpha_beta_constraint() {
        let returns = vec![0.01, -0.02, 0.015, -0.01, 0.005];
        assert!(garch_forecast(&returns, 0.6, 0.5, 1).is_empty());
        assert!(garch_forecast(&returns, 0.5, 0.5, 1).is_empty());
        assert!(garch_forecast(&returns, -0.1, 0.5, 1).is_empty());
        assert!(garch_forecast(&returns, 0.1, -0.1, 1).is_empty());
    }

    #[test]
    fn test_regime_transition_rows_sum_to_one() {
        let regimes = vec![0, 0, 1, 1, 1, 2, 2, 0, 1, 2, 2, 2];
        let matrix = regime_transition_matrix(&regimes, 3);
        assert_eq!(matrix.len(), 3);
        for row in &matrix {
            assert_eq!(row.len(), 3);
            let sum: f64 = row.iter().sum();
            assert!((sum - 1.0).abs() < 1e-9, "row sum {sum}");
            for &p in row {
                assert!((0.0..=1.0).contains(&p));
            }
        }
    }
}
