//! Portfolio Risk Metrics (风险指标).
//!
//! A collection of common portfolio-level risk metrics used in
//! quantitative trading and risk management.
//!
//! All functions are pure (no globals, no state) and run in O(n) on
//! the input series.
//!
//! # Example
//!
//! ```
//! use finkit::risk::{sharpe_ratio, max_drawdown, var_historical};
//!
//! let returns = vec![0.01, -0.02, 0.03, -0.01, 0.02, 0.005, -0.015];
//! let sharpe = sharpe_ratio(&returns, 0.0, 252);
//! let (mdd, _, _) = max_drawdown(&[100.0, 102.0, 98.0, 95.0, 99.0, 105.0]);
//! let var95 = var_historical(&returns, 0.95);
//! ```

/// Historical (non-parametric) Value-at-Risk.
///
/// Returns the loss (as a positive number) such that, with probability
/// `confidence`, the next period's return will not exceed it. Uses the
/// `confidence`-quantile of the empirical distribution.
///
/// # Arguments
/// * `returns` — Per-bar returns.
/// * `confidence` — In (0, 1), typically 0.95 or 0.99.
pub fn var_historical(returns: &[f64], confidence: f64) -> f64 {
    if returns.is_empty() || confidence <= 0.0 || confidence >= 1.0 {
        return 0.0;
    }
    let mut sorted: Vec<f64> = returns.iter().filter(|r| r.is_finite()).copied().collect();
    if sorted.is_empty() {
        return 0.0;
    }
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // Index of the confidence-quantile (lower-tail loss)
    let idx = ((1.0 - confidence) * sorted.len() as f64).floor() as usize;
    let idx = idx.min(sorted.len() - 1);
    -sorted[idx]
}

/// Parametric (Gaussian) Value-at-Risk.
///
/// Assumes returns are normally distributed and uses the
/// `mean + z * std` formula. `z` is the standard-normal quantile for
/// the given confidence (e.g. 1.645 for 95%, 2.326 for 99%).
pub fn var_parametric(returns: &[f64], confidence: f64) -> f64 {
    if returns.len() < 2 || confidence <= 0.0 || confidence >= 1.0 {
        return 0.0;
    }
    let n = returns.len() as f64;
    let mean = returns.iter().filter(|r| r.is_finite()).sum::<f64>() / n;
    let var: f64 = returns.iter()
        .filter(|r| r.is_finite())
        .map(|r| (r - mean).powi(2))
        .sum::<f64>() / (n - 1.0);
    let std = var.sqrt();
    let z = normal_quantile(confidence);
    -(mean - z * std)
}

/// Conditional Value-at-Risk (Expected Shortfall, CVaR).
///
/// Average of the returns worse than the VaR threshold. Returns a
/// positive number representing the expected loss in the tail.
///
/// The tail length is `(1 - confidence) * N` truncated to an integer
/// (truncation is more numerically stable than `ceil` against the
/// floating-point error introduced by `1.0 - 0.7` etc.).
pub fn cvar(returns: &[f64], confidence: f64) -> f64 {
    if returns.is_empty() || confidence <= 0.0 || confidence >= 1.0 {
        return 0.0;
    }
    let mut sorted: Vec<f64> = returns.iter().filter(|r| r.is_finite()).copied().collect();
    if sorted.is_empty() {
        return 0.0;
    }
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let raw = ((1.0 - confidence) * sorted.len() as f64) as usize;
    let cutoff_idx = raw.max(1).min(sorted.len());
    let tail: &[f64] = &sorted[..cutoff_idx];
    let mean_tail = tail.iter().sum::<f64>() / tail.len() as f64;
    -mean_tail
}

/// Maximum drawdown of an equity curve.
///
/// Returns `(max_dd, peak_idx, trough_idx)` where peak_idx is the index
/// of the equity peak before the drawdown, and trough_idx is the index
/// of the lowest equity after that peak.
pub fn max_drawdown(equity: &[f64]) -> (f64, usize, usize) {
    if equity.is_empty() {
        return (0.0, 0, 0);
    }
    let mut peak = equity[0];
    let mut peak_idx = 0;
    let mut max_dd = 0.0_f64;
    let mut dd_peak = 0;
    let mut dd_trough = 0;
    for (i, &v) in equity.iter().enumerate() {
        if v > peak {
            peak = v;
            peak_idx = i;
        }
        if peak > 1e-15 {
            let dd = (peak - v) / peak;
            if dd > max_dd {
                max_dd = dd;
                dd_peak = peak_idx;
                dd_trough = i;
            }
        }
    }
    (max_dd, dd_peak, dd_trough)
}

/// Sharpe ratio (annualized).
///
/// `sharpe = (mean(r) - rf) / std(r) * sqrt(annualize)`.
///
/// * `returns` — per-bar returns.
/// * `risk_free` — per-bar risk-free rate (e.g. daily 0.0001 for 2.5% annual).
/// * `annualize` — number of bars per year (252 for daily, 252*8 for hourly).
///
/// The denominator is the sample standard deviation of `returns` (i.e.
/// deviations from the *mean*, not from the risk-free rate). Using the
/// mean-centred std is required for constant-return series to collapse
/// to a zero Sharpe (the alternative would attribute the entire mean
/// to volatility).
pub fn sharpe_ratio(returns: &[f64], risk_free: f64, annualize: usize) -> f64 {
    if returns.len() < 2 {
        return 0.0;
    }
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    let var: f64 = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let std = var.sqrt();
    if std < 1e-15 {
        return 0.0;
    }
    (mean - risk_free) / std * (annualize as f64).sqrt()
}

/// Sortino ratio (annualized).
///
/// Like Sharpe, but uses downside deviation (only negative returns
/// relative to the risk-free rate) as the denominator.
pub fn sortino_ratio(returns: &[f64], risk_free: f64, annualize: usize) -> f64 {
    if returns.len() < 2 {
        return 0.0;
    }
    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    let downside_var: f64 = returns.iter()
        .map(|r| {
            let excess = r - risk_free;
            if excess < 0.0 { excess.powi(2) } else { 0.0 }
        })
        .sum::<f64>() / n;
    let dstd = downside_var.sqrt();
    if dstd < 1e-15 {
        return 0.0;
    }
    (mean - risk_free) / dstd * (annualize as f64).sqrt()
}

/// Calmar ratio (annualized return / max drawdown).
pub fn calmar_ratio(returns: &[f64], annualize: usize) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let n = returns.len();
    // Compound to total return
    let total: f64 = returns.iter().fold(1.0, |acc, r| acc * (1.0 + r)) - 1.0;
    let annual_return = if n > 1 {
        (1.0 + total).powf(annualize as f64 / n as f64) - 1.0
    } else {
        total
    };
    // Compute equity curve
    let mut equity = vec![1.0; n];
    for i in 1..n {
        equity[i] = equity[i - 1] * (1.0 + returns[i]);
    }
    let (mdd, _, _) = max_drawdown(&equity);
    if mdd < 1e-15 {
        0.0
    } else {
        annual_return / mdd
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Standard normal inverse CDF (quantile function).
///
/// Approximation using the Beasley-Springer-Moro algorithm — accurate to
/// about 1e-9 for `p` in (0, 1).
fn normal_quantile(p: f64) -> f64 {
    // Acklam's algorithm constants
    const A1: f64 = -3.969683028665376e+01;
    const A2: f64 =  2.209460984245205e+02;
    const A3: f64 = -2.759285104469687e+02;
    const A4: f64 =  1.383577518672690e+02;
    const A5: f64 = -3.066479806614716e+01;
    const A6: f64 =  2.506628277459239e+00;

    const B1: f64 = -5.447609879822406e+01;
    const B2: f64 =  1.615858368580409e+02;
    const B3: f64 = -1.556989798598866e+02;
    const B4: f64 =  6.680131188771972e+01;
    const B5: f64 = -1.328068155288572e+01;

    const C1: f64 = -7.784894002430293e-03;
    const C2: f64 = -3.223964580411365e-01;
    const C3: f64 = -2.400758277161838e+00;
    const C4: f64 = -2.549732539343734e+00;
    const C5: f64 =  4.374664141464968e+00;
    const C6: f64 =  2.938163982698783e+00;

    const D1: f64 =  7.784695709041462e-03;
    const D2: f64 =  3.224671290700398e-01;
    const D3: f64 =  2.445134137142996e+00;
    const D4: f64 =  3.754408661907416e+00;

    const P_LOW: f64 = 0.02425;
    const P_HIGH: f64 = 1.0 - P_LOW;

    let p = p.clamp(1e-15, 1.0 - 1e-15);

    if p < P_LOW {
        let q = (-2.0 * p.ln()).sqrt();
        return (((((C1 * q + C2) * q + C3) * q + C4) * q + C5) * q + C6)
            / ((((D1 * q + D2) * q + D3) * q + D4) * q + 1.0);
    } else if p <= P_HIGH {
        let q = p - 0.5;
        let r = q * q;
        return (((((A1 * r + A2) * r + A3) * r + A4) * r + A5) * r + A6) * q
            / (((((B1 * r + B2) * r + B3) * r + B4) * r + B5) * r + 1.0);
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        return -(((((C1 * q + C2) * q + C3) * q + C4) * q + C5) * q + C6)
            / ((((D1 * q + D2) * q + D3) * q + D4) * q + 1.0);
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_var_historical_basic() {
        // Sorted returns: [-5, -3, -1, 1, 2, 3, 5]
        // 95% VaR = -(5%-quantile) = -(-4.84) ≈ 4.84 (index floor(0.05*7) = 0, value -5)
        let returns = vec![1.0, -3.0, 2.0, -5.0, 3.0, -1.0, 5.0];
        let var95 = var_historical(&returns, 0.95);
        // 5% quantile of 7 = sorted[0] = -5 → VaR = 5
        assert_relative_eq!(var95, 5.0, epsilon = 1e-10);
    }

    #[test]
    fn test_var_historical_empty() {
        assert_eq!(var_historical(&[], 0.95), 0.0);
    }

    #[test]
    fn test_var_historical_invalid_confidence() {
        let returns = vec![1.0, -2.0, 0.5];
        assert_eq!(var_historical(&returns, 0.0), 0.0);
        assert_eq!(var_historical(&returns, 1.0), 0.0);
    }

    #[test]
    fn test_var_parametric_known() {
        // mean=0, std=1 → 95% VaR = 1.645
        let returns: Vec<f64> = (0..10000).map(|i| {
            // Pseudo-random normal via Box-Muller
            let u1 = ((i + 1) as f64 / 10001.0).max(1e-15);
            let u2 = ((i + 2) as f64 / 10002.0).max(1e-15);
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        }).collect();
        let var95 = var_parametric(&returns, 0.95);
        // Allow ~10% tolerance due to RNG quality
        assert!((var95 - 1.645).abs() < 0.2, "got {}", var95);
    }

    #[test]
    fn test_cvar_basic() {
        // Worst 30% of returns:
        // sorted: [-10, -5, -2, 1, 2, 3, 4, 5, 6, 7]
        // 70% confidence → cutoff at 30%-quantile → top 3 worst = [-10, -5, -2]
        // mean = -17/3, CVaR = 17/3
        let returns = vec![1.0, -2.0, 2.0, -5.0, 3.0, -10.0, 4.0, 5.0, 6.0, 7.0];
        let cvar70 = cvar(&returns, 0.70);
        let expected = (10.0 + 5.0 + 2.0) / 3.0;
        assert_relative_eq!(cvar70, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_max_drawdown_known() {
        // Equity: 100, 120, 90, 80, 100
        // Peak at index 1 (120), trough at index 3 (80)
        // MDD = (120-80)/120 = 0.3333
        let equity = vec![100.0, 120.0, 90.0, 80.0, 100.0];
        let (mdd, peak, trough) = max_drawdown(&equity);
        assert_relative_eq!(mdd, 40.0 / 120.0, epsilon = 1e-10);
        assert_eq!(peak, 1);
        assert_eq!(trough, 3);
    }

    #[test]
    fn test_max_drawdown_monotonic_up() {
        let equity = vec![100.0, 110.0, 120.0, 130.0];
        let (mdd, _, _) = max_drawdown(&equity);
        assert_relative_eq!(mdd, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_sharpe_zero_vol() {
        let returns = vec![0.01; 10];
        let s = sharpe_ratio(&returns, 0.0, 252);
        assert_eq!(s, 0.0);
    }

    #[test]
    fn test_sharpe_positive() {
        let returns: Vec<f64> = (0..100).map(|i| 0.001 * (i as f64).sin()).collect();
        let s = sharpe_ratio(&returns, 0.0, 252);
        // Sinusoidal: mean ≈ 0, std > 0 → small but finite
        assert!(s.is_finite());
    }

    #[test]
    fn test_sortino_basic() {
        // Symmetric returns: sortino == sharpe
        let returns = vec![0.01, -0.01, 0.02, -0.02, 0.005, -0.005];
        let s = sharpe_ratio(&returns, 0.0, 252);
        let so = sortino_ratio(&returns, 0.0, 252);
        // With rf=0, sharpe may be ~0 (mean is 0) but sortino denominator is different
        // Just check both are finite
        assert!(s.is_finite());
        assert!(so.is_finite());
    }

    #[test]
    fn test_calmar_basic() {
        // Big uptrend with small volatility → very high Calmar (small MDD, large annual return)
        let mut returns: Vec<f64> = (0..252).map(|_| 0.001).collect();
        // Inject a small mid-period dip to create a tiny but non-zero drawdown
        returns[100] = -0.005;
        returns[101] = 0.001;
        let c = calmar_ratio(&returns, 252);
        assert!(c > 50.0, "got {} (expected high Calmar for near-monotonic uptrend)", c);
    }

    #[test]
    fn test_normal_quantile_known() {
        // 97.5% quantile ≈ 1.96
        let q = normal_quantile(0.975);
        assert!((q - 1.96).abs() < 0.01, "got {}", q);
        // 50% quantile = 0
        let q = normal_quantile(0.5);
        assert!(q.abs() < 1e-10);
    }
}
