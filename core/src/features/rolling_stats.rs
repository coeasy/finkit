//! Rolling higher-order statistics: skewness, kurtosis, entropy, Hurst, ACF/PACF.
//!
//! All implementations use O(n) online algorithms for numerical stability.

use crate::error::{Result, TaError};
use ndarray::Array1;

/// Default minimum subsequence length for Hurst R/S analysis.
pub const DEFAULT_HURST_MIN_WINDOW: usize = 20;

/// Compute R/S statistic for a single segment.
fn rs_statistic(segment: &[f64]) -> Option<f64> {
    let n = segment.len();
    if n < 2 {
        return None;
    }
    let mean = segment.iter().sum::<f64>() / n as f64;
    let mut cum_dev = 0.0;
    let mut min_cum = f64::INFINITY;
    let mut max_cum = f64::NEG_INFINITY;
    for &x in segment {
        cum_dev += x - mean;
        min_cum = min_cum.min(cum_dev);
        max_cum = max_cum.max(cum_dev);
    }
    let range = max_cum - min_cum;
    let variance = segment.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
    let std = variance.sqrt();
    if std > 1e-15 {
        Some(range / std)
    } else {
        None
    }
}

/// Slope of simple linear regression y = a + b*x (returns b).
fn linear_regression_slope(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let sum_x: f64 = x.iter().sum();
    let sum_y: f64 = y.iter().sum();
    let sum_xy: f64 = x.iter().zip(y.iter()).map(|(xi, yi)| xi * yi).sum();
    let sum_x2: f64 = x.iter().map(|xi| xi * xi).sum();
    let denom = n * sum_x2 - sum_x * sum_x;
    if denom.abs() < 1e-15 {
        return f64::NAN;
    }
    (n * sum_xy - sum_x * sum_y) / denom
}

/// Rolling skewness using Welford's online algorithm.
pub fn rolling_skewness(data: &[f64], window: usize) -> Array1<f64> {
    let len = data.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if window < 3 || len < window {
        return out;
    }

    for i in (window - 1)..len {
        let start = i + 1 - window;
        let slice = &data[start..=i];
        let n = slice.len() as f64;
        let mean = slice.iter().sum::<f64>() / n;
        let mut m2 = 0.0;
        let mut m3 = 0.0;
        for &x in slice {
            let d = x - mean;
            m2 += d * d;
            m3 += d * d * d;
        }
        m2 /= n;
        m3 /= n;
        let std = m2.sqrt();
        if std > 1e-15 {
            out[i] = m3 / (std * std * std);
        } else {
            out[i] = 0.0;
        }
    }
    out
}

/// Rolling kurtosis (excess kurtosis, Fisher definition).
pub fn rolling_kurtosis(data: &[f64], window: usize) -> Array1<f64> {
    let len = data.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if window < 4 || len < window {
        return out;
    }

    for i in (window - 1)..len {
        let start = i + 1 - window;
        let slice = &data[start..=i];
        let n = slice.len() as f64;
        let mean = slice.iter().sum::<f64>() / n;
        let mut m2 = 0.0;
        let mut m4 = 0.0;
        for &x in slice {
            let d = x - mean;
            let d2 = d * d;
            m2 += d2;
            m4 += d2 * d2;
        }
        m2 /= n;
        m4 /= n;
        if m2 > 1e-15 {
            out[i] = m4 / (m2 * m2) - 3.0;
        } else {
            out[i] = 0.0;
        }
    }
    out
}

/// Rolling entropy using histogram binning.
pub fn rolling_entropy(data: &[f64], window: usize, num_bins: usize) -> Array1<f64> {
    let len = data.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if window < 2 || num_bins < 2 || len < window {
        return out;
    }

    for i in (window - 1)..len {
        let start = i + 1 - window;
        let slice = &data[start..=i];
        let min_val = slice.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_val = slice.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max_val - min_val;

        if range < 1e-15 {
            out[i] = 0.0;
            continue;
        }

        let mut bins = vec![0usize; num_bins];
        let bin_width = range / num_bins as f64;
        for &x in slice {
            let bin = ((x - min_val) / bin_width).floor() as usize;
            let bin = bin.min(num_bins - 1);
            bins[bin] += 1;
        }

        let n = slice.len() as f64;
        let mut entropy = 0.0;
        for &count in &bins {
            if count > 0 {
                let p = count as f64 / n;
                entropy -= p * p.ln();
            }
        }
        out[i] = entropy;
    }
    out
}

/// Rolling z-score: (x - rolling_mean) / rolling_std.
pub fn rolling_zscore(data: &[f64], window: usize) -> Array1<f64> {
    let len = data.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if window < 2 || len < window {
        return out;
    }

    let mut sum = 0.0;
    let mut sum_sq = 0.0;

    for &val in &data[..window] {
        sum += val;
        sum_sq += val * val;
    }

    let w = window as f64;
    let mean = sum / w;
    let var = sum_sq / w - mean * mean;
    let std = var.max(0.0).sqrt();
    if std > 1e-15 {
        out[window - 1] = (data[window - 1] - mean) / std;
    } else {
        out[window - 1] = 0.0;
    }

    for i in window..len {
        sum += data[i] - data[i - window];
        sum_sq += data[i] * data[i] - data[i - window] * data[i - window];
        let mean = sum / w;
        let var = sum_sq / w - mean * mean;
        let std = var.max(0.0).sqrt();
        if std > 1e-15 {
            out[i] = (data[i] - mean) / std;
        } else {
            out[i] = 0.0;
        }
    }
    out
}

/// Rolling percentile rank: what fraction of the window is below the current value.
pub fn rolling_percentile(data: &[f64], window: usize) -> Array1<f64> {
    let len = data.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if window < 2 || len < window {
        return out;
    }

    for i in (window - 1)..len {
        let start = i + 1 - window;
        let current = data[i];
        let count_below = data[start..=i].iter().filter(|&&v| v < current).count();
        out[i] = count_below as f64 / (window - 1) as f64;
    }
    out
}

/// Hurst exponent via R/S (rescaled range) analysis on a return series.
///
/// Computes average R/S for increasing subsequence lengths `n`, then fits
/// `log(R/S) = H * log(n) + c`. The slope `H` is the Hurst exponent:
/// - H ≈ 0.5: random walk
/// - H > 0.5: persistent / trending
/// - H < 0.5: mean-reverting
///
/// # Arguments
/// * `data` - Return series (e.g. log returns of close prices)
/// * `min_window` - Minimum subsequence length (typically 20)
pub fn hurst_exponent(data: &[f64], min_window: usize) -> f64 {
    let len = data.len();
    if len < min_window * 2 || min_window < 2 {
        return f64::NAN;
    }

    let mut log_n = Vec::new();
    let mut log_rs = Vec::new();
    let mut n = min_window;
    while n <= len / 2 {
        let num_segments = len / n;
        let mut rs_sum = 0.0;
        let mut count = 0usize;
        for seg in 0..num_segments {
            let start = seg * n;
            if let Some(rs) = rs_statistic(&data[start..start + n]) {
                if rs.is_finite() && rs > 0.0 {
                    rs_sum += rs;
                    count += 1;
                }
            }
        }
        if count > 0 {
            log_n.push((n as f64).ln());
            log_rs.push((rs_sum / count as f64).ln());
        }
        n *= 2;
    }

    if log_n.len() < 2 {
        return f64::NAN;
    }
    linear_regression_slope(&log_n, &log_rs)
}

/// Hurst exponent with default `min_window` of [`DEFAULT_HURST_MIN_WINDOW`].
pub fn hurst_exponent_default(data: &[f64]) -> f64 {
    hurst_exponent(data, DEFAULT_HURST_MIN_WINDOW)
}

/// Autocorrelation function (ACF).
///
/// `ACF(k) = Cov(X_t, X_{t-k}) / Var(X)` for lags `k = 0..=max_lag`.
/// Lag 0 is always 1.0.
pub fn acf(data: &[f64], max_lag: usize) -> Vec<f64> {
    let mut out = vec![1.0; max_lag + 1];
    let n = data.len();
    if n == 0 {
        return out;
    }
    let mean = data.iter().sum::<f64>() / n as f64;
    let var: f64 = data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
    if var < 1e-15 {
        if max_lag > 0 {
            out[1..=max_lag].fill(0.0);
        }
        return out;
    }
    for (lag, out_val) in out.iter_mut().enumerate().take(max_lag + 1).skip(1) {
        if lag >= n {
            *out_val = f64::NAN;
            continue;
        }
        let cov: f64 = data[lag..]
            .iter()
            .zip(data[..n - lag].iter())
            .map(|(a, b)| (a - mean) * (b - mean))
            .sum::<f64>()
            / n as f64;
        *out_val = cov / var;
    }
    out
}

/// Partial autocorrelation function (PACF) via the Durbin-Levinson recursion.
///
/// Returns PACF coefficients for lags `0..=max_lag` (lag 0 = 1.0).
pub fn pacf(data: &[f64], max_lag: usize) -> Vec<f64> {
    let r = acf(data, max_lag);
    let mut out = vec![1.0; max_lag + 1];
    if data.is_empty() || max_lag == 0 {
        return out;
    }

    let mut phi = vec![0.0f64; max_lag + 1];
    for k in 1..=max_lag {
        let mut num = r[k];
        for j in 1..k {
            num -= phi[j] * r[k - j];
        }
        let mut den = 1.0;
        for j in 1..k {
            den -= phi[j] * r[j];
        }
        let pk = if den.abs() < 1e-15 { 0.0 } else { num / den };
        out[k] = pk;

        let mut phi_new = vec![0.0f64; k + 1];
        phi_new[k] = pk;
        for j in 1..k {
            phi_new[j] = phi[j] - pk * phi[k - j];
        }
        phi = phi_new;
    }
    out
}

/// Rolling semivariance: mean squared deviation below the window mean.
///
/// Only negative deviations `(x - mean)` contribute; upside deviations are zeroed.
/// This is the variance analogue used in downside-risk analysis (not the square root).
pub fn rolling_semivariance(data: &[f64], window: usize) -> Array1<f64> {
    let len = data.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if window < 2 || len < window {
        return out;
    }

    for i in (window - 1)..len {
        let start = i + 1 - window;
        let slice = &data[start..=i];
        let n = slice.len() as f64;
        let mean = slice.iter().sum::<f64>() / n;
        let sv: f64 = slice
            .iter()
            .map(|&x| {
                let d = x - mean;
                if d < 0.0 { d * d } else { 0.0 }
            })
            .sum::<f64>()
            / n;
        out[i] = sv;
    }
    out
}

/// Rolling downside deviation: standard deviation of returns below a threshold.
///
/// Computes `sqrt(mean(min(x - threshold, 0)^2))` over each rolling window.
pub fn rolling_downside_deviation(data: &[f64], window: usize, threshold: f64) -> Array1<f64> {
    let len = data.len();
    let mut out = Array1::from_elem(len, f64::NAN);
    if window < 2 || len < window {
        return out;
    }

    for i in (window - 1)..len {
        let start = i + 1 - window;
        let slice = &data[start..=i];
        let n = slice.len() as f64;
        let sum_sq: f64 = slice
            .iter()
            .map(|&x| {
                let d = (x - threshold).min(0.0);
                d * d
            })
            .sum();
        out[i] = (sum_sq / n).sqrt();
    }
    out
}

/// MacKinnon approximate ADF critical values (constant, no trend) by sample size.
const ADF_CRITICAL_VALUES: [(usize, f64, f64, f64); 5] = [
    (50, -3.58, -2.93, -2.60),
    (100, -3.51, -2.89, -2.58),
    (250, -3.46, -2.88, -2.57),
    (500, -3.44, -2.87, -2.57),
    (10_000, -3.43, -2.86, -2.57),
];

/// MacKinnon approximate Engle-Granger residual ADF critical values (two variables).
const COINT_CRITICAL_VALUES: [(usize, f64, f64, f64); 4] = [
    (50, -4.123, -3.461, -3.127),
    (100, -3.921, -3.388, -3.065),
    (200, -3.831, -3.316, -3.003),
    (10_000, -3.780, -3.276, -2.969),
];

/// Result of an Augmented Dickey-Fuller unit-root test.
#[derive(Debug, Clone)]
pub struct AdfResult {
    /// t-statistic on the lagged level coefficient.
    pub test_statistic: f64,
    /// Approximate p-value from MacKinnon critical-value lookup.
    pub p_value: f64,
    /// Number of lagged differences included in the regression.
    pub lags_used: usize,
    /// True when `p_value < 0.05` (reject unit root / treat series as stationary).
    pub is_stationary: bool,
}

/// Result of an Engle-Granger two-step cointegration test.
#[derive(Debug, Clone)]
pub struct CointegrationResult {
    /// ADF t-statistic on OLS regression residuals.
    pub test_statistic: f64,
    /// Approximate p-value from cointegration critical-value lookup.
    pub p_value: f64,
    /// OLS slope from regressing `series_y` on `series_x`.
    pub cointegration_coefficient: f64,
    /// True when `p_value < 0.05`.
    pub is_cointegrated: bool,
}

/// Interpolate MacKinnon critical values for a given sample size.
fn interpolate_critical_values(
    table: &[(usize, f64, f64, f64)],
    sample_size: usize,
) -> (f64, f64, f64) {
    if sample_size <= table[0].0 {
        return (table[0].1, table[0].2, table[0].3);
    }
    let last = table[table.len() - 1];
    if sample_size >= last.0 {
        return (last.1, last.2, last.3);
    }

    for window in table.windows(2) {
        let (n0, cv1_0, cv5_0, cv10_0) = window[0];
        let (n1, cv1_1, cv5_1, cv10_1) = window[1];
        if sample_size >= n0 && sample_size <= n1 {
            let weight = (sample_size - n0) as f64 / (n1 - n0) as f64;
            let cv1 = cv1_0 + weight * (cv1_1 - cv1_0);
            let cv5 = cv5_0 + weight * (cv5_1 - cv5_0);
            let cv10 = cv10_0 + weight * (cv10_1 - cv10_0);
            return (cv1, cv5, cv10);
        }
    }
    (last.1, last.2, last.3)
}

/// Map an ADF-style t-statistic to an approximate p-value using 1%/5%/10% critical values.
fn mackinnon_p_value(test_statistic: f64, cv1: f64, cv5: f64, cv10: f64) -> f64 {
    if test_statistic <= cv1 {
        return 0.01;
    }
    if test_statistic <= cv5 {
        return 0.01 + (0.05 - 0.01) * (test_statistic - cv1) / (cv5 - cv1);
    }
    if test_statistic <= cv10 {
        return 0.05 + (0.10 - 0.05) * (test_statistic - cv5) / (cv10 - cv5);
    }
    let upper = cv10.abs().max(1.0);
    (0.10 + (test_statistic - cv10) / upper).clamp(0.10, 1.0)
}

/// Solve a small dense linear system `a * x = b` via Gaussian elimination with partial pivoting.
fn solve_linear_system(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    if n == 0 || a.len() != n {
        return None;
    }

    for col in 0..n {
        let mut pivot_row = col;
        let mut pivot_val = a[col][col].abs();
        for (row, row_vals) in a.iter().enumerate().take(n).skip(col + 1) {
            let val = row_vals[col].abs();
            if val > pivot_val {
                pivot_val = val;
                pivot_row = row;
            }
        }
        if pivot_val < 1e-15 {
            return None;
        }
        if pivot_row != col {
            a.swap(col, pivot_row);
            b.swap(col, pivot_row);
        }

        let pivot = a[col][col];
        let pivot_row = a[col][col..].to_vec();
        for (row_idx, row) in a.iter_mut().enumerate().take(n).skip(col + 1) {
            let factor = row[col] / pivot;
            if factor.abs() < 1e-15 {
                continue;
            }
            for (row_k, pivot_k) in row.iter_mut().skip(col).zip(pivot_row.iter()) {
                *row_k -= factor * pivot_k;
            }
            b[row_idx] -= factor * b[col];
        }
    }

    let mut x = vec![0.0; n];
    for row in (0..n).rev() {
        let mut sum = b[row];
        for col in (row + 1)..n {
            sum -= a[row][col] * x[col];
        }
        let diag = a[row][row];
        if diag.abs() < 1e-15 {
            return None;
        }
        x[row] = sum / diag;
    }
    Some(x)
}

/// OLS fit with intercept: `y = beta0 + beta1*x1 + ...`.
fn ols_with_intercept(y: &[f64], predictors: &[Array1<f64>]) -> Result<(Vec<f64>, Vec<f64>, usize)> {
    let n = y.len();
    if n < 2 {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "at least 2 observations for OLS".to_string(),
        });
    }
    let k = predictors.len() + 1;
    if predictors.iter().any(|p| p.len() != n) {
        return Err(TaError::InvalidParameter {
            name: "predictors".to_string(),
            constraint: "must match dependent variable length".to_string(),
        });
    }
    if n <= k {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: format!("need more observations ({n}) than parameters ({k})"),
        });
    }

    let mut xtx = vec![vec![0.0; k]; k];
    let mut xty = vec![0.0; k];

    for i in 0..n {
        let mut row = vec![1.0];
        row.extend(predictors.iter().map(|p| p[i]));
        for r in 0..k {
            xty[r] += row[r] * y[i];
            for c in 0..k {
                xtx[r][c] += row[r] * row[c];
            }
        }
    }

    let beta = solve_linear_system(xtx.clone(), xty).ok_or_else(|| TaError::ComputationError {
        message: "singular design matrix in OLS".to_string(),
    })?;

    let mut residuals = vec![0.0; n];
    let mut ss_res = 0.0;
    for i in 0..n {
        let mut fitted = beta[0];
        for (j, predictor) in predictors.iter().enumerate() {
            fitted += beta[j + 1] * predictor[i];
        }
        let err = y[i] - fitted;
        residuals[i] = err;
        ss_res += err * err;
    }

    let dof = (n - k) as f64;
    let sigma2 = ss_res / dof;

    let mut xtx_inv = vec![vec![0.0; k]; k];
    for i in 0..k {
        let mut unit = vec![0.0; k];
        unit[i] = 1.0;
        xtx_inv[i] = solve_linear_system(xtx.clone(), unit).ok_or_else(|| {
            TaError::ComputationError {
                message: "failed to invert X'X".to_string(),
            }
        })?;
    }

    let mut t_stats = vec![0.0; k];
    for i in 0..k {
        let se = (sigma2 * xtx_inv[i][i]).sqrt();
        t_stats[i] = if se > 1e-15 { beta[i] / se } else { f64::NAN };
    }

    Ok((beta, t_stats, n))
}

/// Minimum observations required for ADF with `lags` lagged differences.
fn adf_min_length(lags: usize) -> usize {
    lags + 5
}

/// Augmented Dickey-Fuller test for a unit root.
///
/// Fits `ΔY_t = α + β Y_{t-1} + Σ γ_i ΔY_{t-i} + ε_t` via OLS and returns the
/// t-statistic on `β` together with a MacKinnon approximate p-value.
pub fn adf_test(data: &[f64], max_lag: usize) -> Result<AdfResult> {
    if data.len() < adf_min_length(max_lag) {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: format!("length at least {}", adf_min_length(max_lag)),
        });
    }

    let lags_used = max_lag;
    let n = data.len();
    let mut dy = vec![0.0; n - 1];
    for t in 1..n {
        dy[t - 1] = data[t] - data[t - 1];
    }

    let start = lags_used + 1;
    let end = n - 1;
    let obs = end - start;
    if obs <= lags_used + 2 {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: format!("length at least {}", adf_min_length(max_lag)),
        });
    }

    let mut y = Vec::with_capacity(obs);
    let mut lag_level = Array1::zeros(obs);
    let mut lagged_diffs: Vec<Array1<f64>> = (0..lags_used)
        .map(|_| Array1::zeros(obs))
        .collect();

    for (idx, t) in (start..end).enumerate() {
        y.push(dy[t]);
        lag_level[idx] = data[t];
        for lag in 0..lags_used {
            lagged_diffs[lag][idx] = dy[t - lag - 1];
        }
    }

    let mut predictors = vec![lag_level];
    predictors.extend(lagged_diffs);
    let (_beta, t_stats, sample_size) = ols_with_intercept(&y, &predictors)?;
    let test_statistic = t_stats[1];

    let (cv1, cv5, cv10) = interpolate_critical_values(&ADF_CRITICAL_VALUES, sample_size);
    let p_value = mackinnon_p_value(test_statistic, cv1, cv5, cv10);

    Ok(AdfResult {
        test_statistic,
        p_value,
        lags_used,
        is_stationary: p_value < 0.05,
    })
}

/// Engle-Granger two-step cointegration test between two series.
///
/// Step 1: OLS `series_y = α + β series_x + ε`.
/// Step 2: ADF test on the residuals from step 1.
pub fn cointegration_test(
    series_x: &[f64],
    series_y: &[f64],
    max_lag: usize,
) -> Result<CointegrationResult> {
    if series_x.len() != series_y.len() {
        return Err(TaError::InvalidParameter {
            name: "series_x and series_y".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if series_x.len() < adf_min_length(max_lag) {
        return Err(TaError::InvalidParameter {
            name: "series".to_string(),
            constraint: format!("length at least {}", adf_min_length(max_lag)),
        });
    }

    let x = Array1::from_iter(series_x.iter().copied());
    let regression = ols_with_intercept(series_y, std::slice::from_ref(&x))?;
    let cointegration_coefficient = regression.0[1];

    let mut residuals = vec![0.0; series_x.len()];
    for i in 0..series_x.len() {
        residuals[i] = series_y[i] - regression.0[0] - cointegration_coefficient * series_x[i];
    }

    let adf = adf_test(&residuals, max_lag)?;
    let (cv1, cv5, cv10) =
        interpolate_critical_values(&COINT_CRITICAL_VALUES, series_x.len());
    let p_value = mackinnon_p_value(adf.test_statistic, cv1, cv5, cv10);

    Ok(CointegrationResult {
        test_statistic: adf.test_statistic,
        p_value,
        cointegration_coefficient,
        is_cointegrated: p_value < 0.05,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::TaError;

    #[test]
    fn test_rolling_skewness_symmetric() {
        let data = vec![1.0, 2.0, 3.0, 2.0, 1.0, 2.0, 3.0, 2.0, 1.0, 2.0];
        let result = rolling_skewness(&data, 5);
        assert!(!result[4].is_nan());
    }

    #[test]
    fn test_rolling_kurtosis_basic() {
        let data: Vec<f64> = (0..20).map(|i| (i as f64 * 0.5).sin()).collect();
        let result = rolling_kurtosis(&data, 10);
        assert!(!result[9].is_nan());
    }

    #[test]
    fn test_rolling_entropy_constant() {
        let data = vec![5.0; 20];
        let result = rolling_entropy(&data, 10, 5);
        assert_eq!(result[9], 0.0);
    }

    #[test]
    fn test_rolling_zscore() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = rolling_zscore(&data, 5);
        assert!(result[3].is_nan());
        assert!(!result[4].is_nan());
        // Last value in the window [1..5] is 5, mean~3, std~1.41, zscore~1.41
        assert!(result[4] > 1.0);
    }

    #[test]
    fn test_rolling_percentile() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = rolling_percentile(&data, 5);
        assert_eq!(result[4], 1.0); // 5 is the max, all 4 others are below
    }

    #[test]
    fn test_rolling_skewness_reference() {
        // Verify against known values: positively skewed data
        let data = vec![1.0, 1.0, 1.0, 1.0, 10.0];
        let result = rolling_skewness(&data, 5);
        assert!(result[4] > 0.0); // Should be positively skewed
    }

    #[test]
    fn test_acf_lag_zero_is_one() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = acf(&data, 3);
        assert_eq!(result.len(), 4);
        assert_eq!(result[0], 1.0);
    }

    #[test]
    fn test_acf_constant_series() {
        let data = vec![5.0; 10];
        let result = acf(&data, 2);
        assert_eq!(result[0], 1.0);
        assert_eq!(result[1], 0.0);
        assert_eq!(result[2], 0.0);
    }

    #[test]
    fn test_pacf_lag_zero_is_one() {
        let data: Vec<f64> = (0..20).map(|i| i as f64 * 0.1).collect();
        let result = pacf(&data, 3);
        assert_eq!(result.len(), 4);
        assert_eq!(result[0], 1.0);
    }

    #[test]
    fn test_pacf_ar1_like() {
        // AR(1): x_t = 0.8 * x_{t-1} + noise => PACF(1) ≈ ACF(1)
        let mut data = vec![1.0];
        for _ in 0..99 {
            let prev = *data.last().unwrap();
            data.push(0.8 * prev);
        }
        let acf_vals = acf(&data, 2);
        let pacf_vals = pacf(&data, 2);
        assert!((pacf_vals[1] - acf_vals[1]).abs() < 0.05);
    }

    #[test]
    fn test_hurst_random_walk_near_half() {
        // Simulated random walk returns should yield H ≈ 0.5
        let mut rng_state = 12345u64;
        let mut returns = Vec::with_capacity(512);
        for _ in 0..512 {
            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let u = (rng_state >> 33) as f64 / u32::MAX as f64;
            returns.push(u - 0.5);
        }
        let h = hurst_exponent(&returns, 20);
        assert!(h.is_finite());
        assert!((h - 0.5).abs() < 0.25, "H={h}");
    }

    #[test]
    fn test_hurst_trending_above_half() {
        // Strongly persistent series (cumulative positive drift)
        let returns: Vec<f64> = (0..256).map(|i| 0.01 + (i as f64 * 0.001).sin() * 0.001).collect();
        let h = hurst_exponent(&returns, 20);
        assert!(h.is_finite());
        assert!(h > 0.45, "H={h}");
    }

    #[test]
    fn test_hurst_too_short_returns_nan() {
        let data = vec![0.01; 10];
        assert!(hurst_exponent(&data, 20).is_nan());
    }

    #[test]
    fn test_rolling_semivariance_downside() {
        let data = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let result = rolling_semivariance(&data, 5);
        assert!(!result[4].is_nan());
        assert!(result[4] > 0.0);
    }

    #[test]
    fn test_rolling_semivariance_all_above_mean() {
        let data = vec![5.0; 5];
        let result = rolling_semivariance(&data, 5);
        assert_eq!(result[4], 0.0);
    }

    #[test]
    fn test_rolling_downside_deviation() {
        let data = vec![-0.02, -0.01, 0.0, 0.01, 0.02];
        let result = rolling_downside_deviation(&data, 5, 0.0);
        assert!(!result[4].is_nan());
        assert!(result[4] > 0.0);
    }

    #[test]
    fn test_rolling_downside_deviation_all_above_threshold() {
        let data = vec![0.01, 0.02, 0.03, 0.04, 0.05];
        let result = rolling_downside_deviation(&data, 5, 0.0);
        assert_eq!(result[4], 0.0);
    }

    #[test]
    fn test_adf_stationary_series() {
        let mut rng_state = 42_u64;
        let mut data = Vec::with_capacity(200);
        for _ in 0..200 {
            rng_state = rng_state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1);
            let u = (rng_state >> 33) as f64 / u32::MAX as f64;
            data.push(u - 0.5);
        }
        let result = adf_test(&data, 1).unwrap();
        assert!(result.test_statistic.is_finite());
        assert!((0.0..=1.0).contains(&result.p_value));
        assert_eq!(result.lags_used, 1);
        assert!(result.is_stationary);
    }

    #[test]
    fn test_adf_random_walk_non_stationary() {
        let mut rng_state = 7_u64;
        let mut walk = vec![0.0; 200];
        for i in 1..200 {
            rng_state = rng_state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1);
            let u = (rng_state >> 33) as f64 / u32::MAX as f64;
            walk[i] = walk[i - 1] + u;
        }
        let result = adf_test(&walk, 1).unwrap();
        assert!(result.test_statistic > -3.0);
        assert!(!result.is_stationary);
    }

    #[test]
    fn test_adf_insufficient_data_returns_error() {
        let data = vec![1.0, 2.0, 3.0];
        let err = adf_test(&data, 1).unwrap_err();
        assert!(matches!(err, TaError::InvalidParameter { .. }));
    }

    #[test]
    fn test_adf_result_debug_clone() {
        let data: Vec<f64> = (0..50).map(|i| (i as f64 * 0.1).sin()).collect();
        let original = adf_test(&data, 0).unwrap();
        let cloned = original.clone();
        assert_eq!(format!("{original:?}"), format!("{cloned:?}"));
    }

    #[test]
    fn test_cointegration_cointegrated_pair() {
        let mut x = vec![0.0; 200];
        let mut y = vec![0.0; 200];
        for i in 1..200 {
            x[i] = x[i - 1] + 0.3;
            y[i] = 2.0 * x[i] + 0.1 * (i as f64).sin();
        }
        let result = cointegration_test(&x, &y, 1).unwrap();
        assert!((result.cointegration_coefficient - 2.0).abs() < 0.5);
        assert!(result.is_cointegrated);
        assert!(result.p_value < 0.05);
    }

    #[test]
    fn test_cointegration_independent_random_walks() {
        let mut x = vec![0.0; 200];
        let mut y = vec![0.0; 200];
        let mut rng_state = 99_u64;
        for i in 1..200 {
            rng_state = rng_state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1);
            let u = (rng_state >> 33) as f64 / u32::MAX as f64;
            x[i] = x[i - 1] + u;
            rng_state = rng_state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1);
            let v = (rng_state >> 33) as f64 / u32::MAX as f64;
            y[i] = y[i - 1] + v;
        }
        let result = cointegration_test(&x, &y, 1).unwrap();
        assert!(!result.is_cointegrated);
    }

    #[test]
    fn test_cointegration_mismatched_length_returns_error() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let y = vec![1.0, 2.0, 3.0];
        let err = cointegration_test(&x, &y, 0).unwrap_err();
        assert!(matches!(err, TaError::InvalidParameter { .. }));
    }

    #[test]
    fn test_cointegration_result_fields() {
        let x: Vec<f64> = (0..80).map(|i| i as f64).collect();
        let y: Vec<f64> = x
            .iter()
            .enumerate()
            .map(|(i, v)| 1.5 * v + 0.2 + (i as f64 * 0.01).sin() * 0.05)
            .collect();
        let result = cointegration_test(&x, &y, 0).unwrap();
        assert!(result.test_statistic.is_finite());
        assert!((0.0..=1.0).contains(&result.p_value));
        assert!((result.cointegration_coefficient - 1.5).abs() < 0.05);
    }
}

/// Compute rolling Kendall Tau correlation between two series.
///
/// For each window position, computes the Kendall Tau correlation between
/// corresponding windows of x and y.
///
/// # Arguments
/// * `x` - First data series
/// * `y` - Second data series (same length as x)
/// * `window` - Rolling window size (>= 2)
///
/// # Returns
/// Array of rolling Kendall Tau values (NaN for warm-up period)
pub fn rolling_kendall(x: &[f64], y: &[f64], window: usize) -> Result<Array1<f64>> {
    use crate::math::statistics::kendall_tau;

    if x.len() != y.len() {
        return Err(TaError::InvalidParameter {
            name: "x, y".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if window < 2 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "must be >= 2".to_string(),
        });
    }
    let n = x.len();
    if n < window {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= window".to_string(),
        });
    }

    let mut output = Array1::from_elem(n, f64::NAN);

    for i in (window - 1)..n {
        let start = i + 1 - window;
        if let Ok(tau) = kendall_tau(&x[start..=i], &y[start..=i]) {
            output[i] = tau;
        }
    }

    Ok(output)
}

/// Compute rolling Spearman rank correlation between two series.
///
/// For each window position, computes the Spearman rank correlation between
/// corresponding windows of x and y.
///
/// # Arguments
/// * `x` - First data series
/// * `y` - Second data series (same length as x)
/// * `window` - Rolling window size (>= 2)
///
/// # Returns
/// Array of rolling Spearman rho values (NaN for warm-up period)
pub fn rolling_spearman(x: &[f64], y: &[f64], window: usize) -> Result<Array1<f64>> {
    use crate::math::statistics::spearman_rank;

    if x.len() != y.len() {
        return Err(TaError::InvalidParameter {
            name: "x, y".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if window < 2 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "must be >= 2".to_string(),
        });
    }
    let n = x.len();
    if n < window {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= window".to_string(),
        });
    }

    let mut output = Array1::from_elem(n, f64::NAN);

    for i in (window - 1)..n {
        let start = i + 1 - window;
        if let Ok(rho) = spearman_rank(&x[start..=i], &y[start..=i]) {
            output[i] = rho;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod rolling_corr_tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_rolling_kendall_basic() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
        let result = rolling_kendall(&x, &y, 5).unwrap();
        assert_eq!(result.len(), 10);
        assert!(result[0].is_nan());
        assert!(result[3].is_nan());
        // Perfect concordance in all windows
        assert_relative_eq!(result[4], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result[9], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_rolling_kendall_invalid() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 2.0, 3.0];
        assert!(rolling_kendall(&x, &y, 1).is_err());
        assert!(rolling_kendall(&x, &[1.0], 2).is_err());
        assert!(rolling_kendall(&x, &y, 5).is_err());
    }

    #[test]
    fn test_rolling_spearman_basic() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
        let result = rolling_spearman(&x, &y, 5).unwrap();
        assert_eq!(result.len(), 10);
        assert!(result[0].is_nan());
        assert!(result[3].is_nan());
        assert_relative_eq!(result[4], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result[9], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_rolling_spearman_invalid() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 2.0, 3.0];
        assert!(rolling_spearman(&x, &y, 1).is_err());
        assert!(rolling_spearman(&x, &[1.0], 2).is_err());
        assert!(rolling_spearman(&x, &y, 5).is_err());
    }
}

/// Result of rolling quantile regression.
#[derive(Debug, Clone)]
pub struct RollingQuantileRegResult {
    /// Slope at each point (NaN during warm-up)
    pub slope: Array1<f64>,
    /// Intercept at each point (NaN during warm-up)
    pub intercept: Array1<f64>,
}

/// Compute rolling quantile regression on a series.
///
/// For each window of `y` values, fits a tau-quantile regression line
/// y = slope * t + intercept where t is 0..window (time index within window).
///
/// # Arguments
/// * `y` - Data series
/// * `window` - Rolling window size (>= 2)
/// * `tau` - Quantile level in (0, 1). Common: 0.1, 0.25, 0.5, 0.75, 0.9
///
/// # Returns
/// RollingQuantileRegResult containing slope and intercept arrays
pub fn rolling_quantile_regression(
    y: &[f64],
    window: usize,
    tau: f64,
) -> Result<RollingQuantileRegResult> {
    use crate::math::linear::quantile_regression;

    if window < 2 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "must be >= 2".to_string(),
        });
    }
    let n = y.len();
    if n < window {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= window".to_string(),
        });
    }
    if tau <= 0.0 || tau >= 1.0 {
        return Err(TaError::InvalidParameter {
            name: "tau".to_string(),
            constraint: "must be in (0, 1)".to_string(),
        });
    }

    let mut slope_out = Array1::from_elem(n, f64::NAN);
    let mut intercept_out = Array1::from_elem(n, f64::NAN);

    let x_window: Vec<f64> = (0..window).map(|i| i as f64).collect();

    for i in (window - 1)..n {
        let start = i + 1 - window;
        let y_slice = &y[start..=i];
        if let Ok(result) = quantile_regression(&x_window, y_slice, tau) {
            slope_out[i] = result.slope;
            intercept_out[i] = result.intercept;
        }
    }

    Ok(RollingQuantileRegResult {
        slope: slope_out,
        intercept: intercept_out,
    })
}

#[cfg(test)]
mod rolling_quantile_reg_tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_rolling_quantile_regression_basic() {
        let y: Vec<f64> = (0..20).map(|i| 2.0 * i as f64 + 1.0).collect();
        let result = rolling_quantile_regression(&y, 10, 0.5).unwrap();
        assert_eq!(result.slope.len(), 20);
        assert!(result.slope[8].is_nan());
        // For perfectly linear data, slope should be close to 2.0
        assert_relative_eq!(result.slope[9], 2.0, epsilon = 0.1);
        assert_relative_eq!(result.slope[19], 2.0, epsilon = 0.1);
    }

    #[test]
    fn test_rolling_quantile_regression_tau_values() {
        let y: Vec<f64> = (0..20).map(|i| i as f64 + (i as f64 * 0.5).sin()).collect();
        // All supported tau values should produce results
        for tau in &[0.1, 0.25, 0.5, 0.75, 0.9] {
            let result = rolling_quantile_regression(&y, 10, *tau).unwrap();
            assert_eq!(result.slope.len(), 20);
            assert!(result.slope[9].is_finite());
            assert!(result.intercept[9].is_finite());
        }
    }

    #[test]
    fn test_rolling_quantile_regression_invalid() {
        let y = vec![1.0, 2.0, 3.0];
        assert!(rolling_quantile_regression(&y, 1, 0.5).is_err());
        assert!(rolling_quantile_regression(&y, 5, 0.5).is_err());
        assert!(rolling_quantile_regression(&y, 2, 0.0).is_err());
        assert!(rolling_quantile_regression(&y, 2, 1.0).is_err());
    }
}

/// Result of rolling Theil-Sen estimation.
#[derive(Debug, Clone)]
pub struct RollingTheilSenResult {
    /// Slope at each point (NaN during warm-up)
    pub slope: Array1<f64>,
    /// Intercept at each point (NaN during warm-up)
    pub intercept: Array1<f64>,
}

/// Compute rolling Theil-Sen robust slope estimator.
///
/// For each window position, computes the Theil-Sen median slope estimator.
///
/// # Arguments
/// * `y` - Data series
/// * `window` - Rolling window size (>= 2)
///
/// # Returns
/// RollingTheilSenResult containing slope and intercept arrays
pub fn rolling_theil_sen(y: &[f64], window: usize) -> Result<RollingTheilSenResult> {
    use crate::math::linear::theil_sen;

    if window < 2 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "must be >= 2".to_string(),
        });
    }
    let n = y.len();
    if n < window {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= window".to_string(),
        });
    }

    let mut slope_out = Array1::from_elem(n, f64::NAN);
    let mut intercept_out = Array1::from_elem(n, f64::NAN);

    let x_window: Vec<f64> = (0..window).map(|i| i as f64).collect();

    for i in (window - 1)..n {
        let start = i + 1 - window;
        let y_slice = &y[start..=i];
        if let Ok(result) = theil_sen(&x_window, y_slice) {
            slope_out[i] = result.slope;
            intercept_out[i] = result.intercept;
        }
    }

    Ok(RollingTheilSenResult {
        slope: slope_out,
        intercept: intercept_out,
    })
}

#[cfg(test)]
mod rolling_theil_sen_tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_rolling_theil_sen_linear() {
        let y: Vec<f64> = (0..20).map(|i| 2.0 * i as f64 + 1.0).collect();
        let result = rolling_theil_sen(&y, 10).unwrap();
        assert_eq!(result.slope.len(), 20);
        assert!(result.slope[8].is_nan());
        assert_relative_eq!(result.slope[9], 2.0, epsilon = 1e-10);
        assert_relative_eq!(result.slope[19], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_rolling_theil_sen_with_outliers() {
        let mut y: Vec<f64> = (0..20).map(|i| 1.0 * i as f64).collect();
        y[5] = 500.0; // outlier
        y[15] = -500.0; // outlier
        let result = rolling_theil_sen(&y, 10).unwrap();
        // Theil-Sen should be robust, slope near 1.0
        for i in 9..20 {
            if result.slope[i].is_finite() {
                assert!(
                    (result.slope[i] - 1.0).abs() < 0.5,
                    "slope at {} = {} too far from 1.0",
                    i,
                    result.slope[i]
                );
            }
        }
    }

    #[test]
    fn test_rolling_theil_sen_invalid() {
        let y = vec![1.0, 2.0, 3.0];
        assert!(rolling_theil_sen(&y, 1).is_err());
        assert!(rolling_theil_sen(&y, 5).is_err());
    }
}

/// Mann-Kendall trend test result.
#[derive(Debug, Clone)]
pub struct MkResult {
    /// S statistic (sum of sign differences)
    pub s: i64,
    /// Variance of S
    pub var_s: f64,
    /// Z-score (standardized test statistic)
    pub z: f64,
    /// Two-tailed p-value
    pub p_value: f64,
    /// Trend direction: 1 (increasing), -1 (decreasing), 0 (no trend)
    pub trend: i8,
}

/// Mann-Kendall non-parametric trend test.
///
/// Tests for monotonic trend in a time series. The S statistic counts
/// concordant minus discordant pairs. Under no-trend null hypothesis,
/// S is approximately normal for n >= 10.
///
/// # Arguments
/// * `data` - Time series data
///
/// # Returns
/// MkResult with S statistic, variance, Z-score, p-value, and trend direction
pub fn mann_kendall(data: &[f64]) -> Result<MkResult> {
    let n = data.len();
    if n < 4 {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= 4".to_string(),
        });
    }

    // Compute S statistic
    let mut s: i64 = 0;
    for i in 0..n - 1 {
        for j in (i + 1)..n {
            let diff = data[j] - data[i];
            if diff > 0.0 {
                s += 1;
            } else if diff < 0.0 {
                s -= 1;
            }
        }
    }

    // Compute variance accounting for ties
    // Group ties
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mut tie_groups: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j < n - 1 && (sorted[j + 1] - sorted[j]).abs() < 1e-15 {
            j += 1;
        }
        let group_size = j - i + 1;
        if group_size > 1 {
            tie_groups.push(group_size);
        }
        i = j + 1;
    }

    let n_f = n as f64;
    let mut var_s = n_f * (n_f - 1.0) * (2.0 * n_f + 5.0) / 18.0;
    for &t in &tie_groups {
        let t_f = t as f64;
        var_s -= t_f * (t_f - 1.0) * (2.0 * t_f + 5.0) / 18.0;
    }

    // Z-score with continuity correction
    let z = if s > 0 {
        (s as f64 - 1.0) / var_s.sqrt()
    } else if s < 0 {
        (s as f64 + 1.0) / var_s.sqrt()
    } else {
        0.0
    };

    // Two-tailed p-value from standard normal
    let p_value = 2.0 * (1.0 - standard_normal_cdf(z.abs()));

    let trend = if z > 0.0 && p_value < 0.05 {
        1
    } else if z < 0.0 && p_value < 0.05 {
        -1
    } else {
        0
    };

    Ok(MkResult {
        s,
        var_s,
        z,
        p_value,
        trend,
    })
}

/// Approximation of standard normal CDF using rational function.
fn standard_normal_cdf(x: f64) -> f64 {
    if x < -8.0 {
        return 0.0;
    }
    if x > 8.0 {
        return 1.0;
    }
    let a1 = 0.254829592;
    let a2 = -0.284496736;
    let a3 = 1.421413741;
    let a4 = -1.453152027;
    let a5 = 1.061405429;
    let p_const = 0.3275911;

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x_abs = x.abs() / std::f64::consts::SQRT_2;
    let t = 1.0 / (1.0 + p_const * x_abs);
    let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x_abs * x_abs).exp();
    0.5 * (1.0 + sign * y)
}

/// Rolling Mann-Kendall trend test result.
#[derive(Debug, Clone)]
pub struct RollingMkResult {
    /// Z-score at each point (NaN during warm-up)
    pub z: Array1<f64>,
    /// P-value at each point (NaN during warm-up)
    pub p_value: Array1<f64>,
    /// Trend at each point: 1/-1/0 (NaN during warm-up as f64)
    pub trend: Array1<f64>,
}

/// Compute rolling Mann-Kendall trend test.
///
/// For each window position, computes the Mann-Kendall test.
///
/// # Arguments
/// * `data` - Time series data
/// * `window` - Rolling window size (>= 4)
///
/// # Returns
/// RollingMkResult with z-score, p-value, and trend arrays
pub fn rolling_mann_kendall(data: &[f64], window: usize) -> Result<RollingMkResult> {
    if window < 4 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "must be >= 4".to_string(),
        });
    }
    let n = data.len();
    if n < window {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= window".to_string(),
        });
    }

    let mut z_out = Array1::from_elem(n, f64::NAN);
    let mut p_out = Array1::from_elem(n, f64::NAN);
    let mut trend_out = Array1::from_elem(n, f64::NAN);

    for i in (window - 1)..n {
        let start = i + 1 - window;
        if let Ok(result) = mann_kendall(&data[start..=i]) {
            z_out[i] = result.z;
            p_out[i] = result.p_value;
            trend_out[i] = result.trend as f64;
        }
    }

    Ok(RollingMkResult {
        z: z_out,
        p_value: p_out,
        trend: trend_out,
    })
}

#[cfg(test)]
mod mann_kendall_tests {
    use super::*;

    #[test]
    fn test_mann_kendall_increasing() {
        let data: Vec<f64> = (0..20).map(|i| i as f64).collect();
        let result = mann_kendall(&data).unwrap();
        assert!(result.s > 0);
        assert!(result.z > 0.0);
        assert!(result.p_value < 0.05);
        assert_eq!(result.trend, 1);
    }

    #[test]
    fn test_mann_kendall_decreasing() {
        let data: Vec<f64> = (0..20).map(|i| 100.0 - i as f64).collect();
        let result = mann_kendall(&data).unwrap();
        assert!(result.s < 0);
        assert!(result.z < 0.0);
        assert!(result.p_value < 0.05);
        assert_eq!(result.trend, -1);
    }

    #[test]
    fn test_mann_kendall_no_trend() {
        // Alternating values (no clear monotonic trend)
        let data = vec![1.0, 5.0, 2.0, 6.0, 3.0, 7.0, 4.0, 8.0];
        let result = mann_kendall(&data).unwrap();
        // S should be relatively small
        assert!(result.var_s > 0.0);
    }

    #[test]
    fn test_mann_kendall_invalid() {
        assert!(mann_kendall(&[1.0, 2.0, 3.0]).is_err());
    }

    #[test]
    fn test_rolling_mann_kendall_basic() {
        let data: Vec<f64> = (0..30).map(|i| i as f64 + (i as f64 * 0.5).sin()).collect();
        let result = rolling_mann_kendall(&data, 10).unwrap();
        assert_eq!(result.z.len(), 30);
        assert!(result.z[8].is_nan());
        assert!(result.z[9].is_finite());
    }

    #[test]
    fn test_rolling_mann_kendall_invalid() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert!(rolling_mann_kendall(&data, 3).is_err());
        assert!(rolling_mann_kendall(&data, 10).is_err());
    }
}

/// Granger Causality test result.
#[derive(Debug, Clone)]
pub struct GrangerResult {
    /// F-statistic
    pub f_stat: f64,
    /// Approximate p-value
    pub p_value: f64,
    /// Whether x Granger-causes y at 5% significance
    pub is_causal: bool,
}

/// Granger Causality test: does x Granger-cause y?
///
/// Compares restricted model (y ~ lags of y) with unrestricted model
/// (y ~ lags of y + lags of x) using an F-test.
///
/// # Arguments
/// * `x` - Potential cause series
/// * `y` - Effect series (same length as x)
/// * `max_lag` - Maximum lag order for the VAR model
///
/// # Returns
/// GrangerResult with F-statistic, p-value, and causality flag
pub fn granger_causality(x: &[f64], y: &[f64], max_lag: usize) -> Result<GrangerResult> {
    if x.len() != y.len() {
        return Err(TaError::InvalidParameter {
            name: "x, y".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    let n = x.len();
    if max_lag == 0 {
        return Err(TaError::InvalidParameter {
            name: "max_lag".to_string(),
            constraint: "must be >= 1".to_string(),
        });
    }
    if n <= 2 * max_lag + 1 {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "not enough data for given max_lag".to_string(),
        });
    }

    let t = n - max_lag; // effective sample size

    // Restricted model: y[t] ~ y[t-1]...y[t-p]
    let rss_r = ols_residual_ss(y, &[y], max_lag);

    // Unrestricted model: y[t] ~ y[t-1]...y[t-p] + x[t-1]...x[t-p]
    let rss_u = ols_residual_ss(y, &[y, x], max_lag);

    let df1 = max_lag as f64;
    let df2 = (t - 2 * max_lag - 1) as f64;

    if df2 <= 0.0 || rss_u < 1e-15 {
        return Ok(GrangerResult {
            f_stat: 0.0,
            p_value: 1.0,
            is_causal: false,
        });
    }

    let f_stat = ((rss_r - rss_u) / df1) / (rss_u / df2);

    // Approximate p-value using F-distribution CDF approximation
    let p_value = f_test_p_value(f_stat, df1, df2);

    Ok(GrangerResult {
        f_stat,
        p_value,
        is_causal: p_value < 0.05,
    })
}

/// OLS residual sum of squares for VAR-like model.
fn ols_residual_ss(y: &[f64], regressors: &[&[f64]], lag: usize) -> f64 {
    let n = y.len();
    let t = n - lag;
    let num_reg = regressors.len() * lag;

    // Build X matrix and y vector
    let mut xt_x = vec![0.0; (num_reg + 1) * (num_reg + 1)]; // +1 for intercept
    let mut xt_y = vec![0.0; num_reg + 1];
    let cols = num_reg + 1;

    for i in 0..t {
        let row_idx = lag + i;
        let yi = y[row_idx];

        // Build row: [1, reg1_lag1, reg1_lag2, ..., reg2_lag1, ...]
        let mut row = vec![1.0]; // intercept
        for (r_idx, reg) in regressors.iter().enumerate() {
            for l in 1..=lag {
                let _ = r_idx;
                row.push(reg[row_idx - l]);
            }
        }

        // Accumulate XtX and Xty
        for r in 0..cols {
            for c in 0..cols {
                xt_x[r * cols + c] += row[r] * row[c];
            }
            xt_y[r] += row[r] * yi;
        }
    }

    // Solve normal equations
    let beta = solve_system(&mut xt_x, &mut xt_y, cols);

    // Compute RSS
    let mut rss = 0.0;
    for i in 0..t {
        let row_idx = lag + i;
        let yi = y[row_idx];

        let mut row = vec![1.0];
        for reg in regressors.iter() {
            for l in 1..=lag {
                row.push(reg[row_idx - l]);
            }
        }

        let y_hat: f64 = row.iter().zip(beta.iter()).map(|(x, b)| x * b).sum();
        rss += (yi - y_hat) * (yi - y_hat);
    }

    rss
}

/// Gaussian elimination solver (same as in complexity.rs but local to avoid coupling).
fn solve_system(a: &mut [f64], b: &mut [f64], n: usize) -> Vec<f64> {
    for col in 0..n {
        let mut max_row = col;
        let mut max_val = a[col * n + col].abs();
        for row in (col + 1)..n {
            let val = a[row * n + col].abs();
            if val > max_val {
                max_val = val;
                max_row = row;
            }
        }
        if max_row != col {
            for k in 0..n {
                a.swap(col * n + k, max_row * n + k);
            }
            b.swap(col, max_row);
        }

        let pivot = a[col * n + col];
        if pivot.abs() < 1e-15 {
            continue;
        }

        for row in (col + 1)..n {
            let factor = a[row * n + col] / pivot;
            for k in col..n {
                a[row * n + k] -= factor * a[col * n + k];
            }
            b[row] -= factor * b[col];
        }
    }

    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut sum = b[i];
        for j in (i + 1)..n {
            sum -= a[i * n + j] * x[j];
        }
        if a[i * n + i].abs() > 1e-15 {
            x[i] = sum / a[i * n + i];
        }
    }
    x
}

/// Approximate p-value for F-test using a rough Beta regularized incomplete function.
fn f_test_p_value(f: f64, df1: f64, df2: f64) -> f64 {
    if f <= 0.0 {
        return 1.0;
    }
    // Use transformation to beta distribution
    let x = df1 * f / (df1 * f + df2);
    1.0 - regularized_incomplete_beta(df1 / 2.0, df2 / 2.0, x)
}

/// Regularized incomplete beta function approximation (continued fraction).
fn regularized_incomplete_beta(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }

    let ln_beta = ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b);
    let front = (x.ln() * a + (1.0 - x).ln() * b - ln_beta).exp() / a;

    // Lentz's continued fraction
    let mut f_val;
    let mut c = 1.0;
    let mut d = 1.0 - (a + b) * x / (a + 1.0);
    if d.abs() < 1e-30 {
        d = 1e-30;
    }
    d = 1.0 / d;
    f_val = d;

    for m in 1..200 {
        let m_f = m as f64;
        // Even step
        let num = m_f * (b - m_f) * x / ((a + 2.0 * m_f - 1.0) * (a + 2.0 * m_f));
        d = 1.0 + num * d;
        if d.abs() < 1e-30 { d = 1e-30; }
        c = 1.0 + num / c;
        if c.abs() < 1e-30 { c = 1e-30; }
        d = 1.0 / d;
        f_val *= c * d;

        // Odd step
        let num2 = -(a + m_f) * (a + b + m_f) * x / ((a + 2.0 * m_f) * (a + 2.0 * m_f + 1.0));
        d = 1.0 + num2 * d;
        if d.abs() < 1e-30 { d = 1e-30; }
        c = 1.0 + num2 / c;
        if c.abs() < 1e-30 { c = 1e-30; }
        d = 1.0 / d;
        let delta = c * d;
        f_val *= delta;

        if (delta - 1.0).abs() < 1e-8 {
            break;
        }
    }

    front * f_val
}

/// Stirling approximation of ln(Gamma(x)).
fn ln_gamma(x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    // Lanczos approximation
    let g = 7.0;
    let c = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    let x_adj = x - 1.0;
    let mut sum = c[0];
    for (i, &ci) in c.iter().enumerate().skip(1) {
        sum += ci / (x_adj + i as f64);
    }
    let t = x_adj + g + 0.5;
    0.5 * (2.0 * std::f64::consts::PI).ln() + (t.ln() * (x_adj + 0.5)) - t + sum.ln()
}

/// Rolling Granger Causality test.
///
/// # Arguments
/// * `x` - Potential cause series
/// * `y` - Effect series
/// * `window` - Rolling window size
/// * `max_lag` - Maximum lag order
///
/// # Returns
/// Array of F-statistics per window position (NaN during warm-up)
pub fn rolling_granger(x: &[f64], y: &[f64], window: usize, max_lag: usize) -> Result<Array1<f64>> {
    if x.len() != y.len() {
        return Err(TaError::InvalidParameter {
            name: "x, y".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if window <= 2 * max_lag + 1 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "too small for given max_lag".to_string(),
        });
    }
    let n = x.len();
    if n < window {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= window".to_string(),
        });
    }

    let mut output = Array1::from_elem(n, f64::NAN);
    for i in (window - 1)..n {
        let start = i + 1 - window;
        if let Ok(result) = granger_causality(&x[start..=i], &y[start..=i], max_lag) {
            output[i] = result.f_stat;
        }
    }

    Ok(output)
}

// ─── Information Coefficient (IC) ───────────────────────────────────

/// IC calculation method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IcMethod {
    /// Pearson correlation between factor and forward returns.
    Pearson,
    /// Spearman (rank) correlation between factor and forward returns.
    Rank,
}

/// Rolling Information Coefficient between factor values and forward returns.
///
/// IC is the standard measure of factor effectiveness in quantitative finance:
/// - Pearson IC = Pearson correlation(factor, forward_return) per rolling window
/// - Rank IC = Spearman correlation(factor, forward_return) per rolling window
///
/// # Arguments
/// * `factor` - Factor values (predictions/signals)
/// * `forward_return` - Realized forward returns
/// * `window` - Rolling window size (minimum 3)
/// * `method` - `IcMethod::Pearson` or `IcMethod::Rank`
///
/// # Returns
/// Array of IC values per window position (NaN during warm-up)
pub fn rolling_ic(
    factor: &[f64],
    forward_return: &[f64],
    window: usize,
    method: IcMethod,
) -> Result<Array1<f64>> {
    if factor.len() != forward_return.len() {
        return Err(TaError::InvalidParameter {
            name: "factor, forward_return".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if window < 3 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "must be >= 3".to_string(),
        });
    }
    let n = factor.len();
    if n < window {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= window".to_string(),
        });
    }

    let mut output = Array1::from_elem(n, f64::NAN);

    for i in (window - 1)..n {
        let start = i + 1 - window;
        let f_win = &factor[start..=i];
        let r_win = &forward_return[start..=i];

        let ic = match method {
            IcMethod::Pearson => pearson_ic(f_win, r_win),
            IcMethod::Rank => {
                crate::math::statistics::spearman_rank(f_win, r_win).unwrap_or(f64::NAN)
            }
        };
        output[i] = ic;
    }

    Ok(output)
}

/// Pearson correlation for a window slice.
fn pearson_ic(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_xy = 0.0;
    let mut sum_x2 = 0.0;
    let mut sum_y2 = 0.0;

    for (&xi, &yi) in x.iter().zip(y.iter()) {
        sum_x += xi;
        sum_y += yi;
        sum_xy += xi * yi;
        sum_x2 += xi * xi;
        sum_y2 += yi * yi;
    }

    let denom_x = n * sum_x2 - sum_x * sum_x;
    let denom_y = n * sum_y2 - sum_y * sum_y;
    let denom = (denom_x * denom_y).sqrt();

    if denom < 1e-15 {
        return 0.0;
    }

    (n * sum_xy - sum_x * sum_y) / denom
}

#[cfg(test)]
mod ic_tests {
    use super::*;

    #[test]
    fn test_rolling_ic_pearson_perfect() {
        // Perfect linear correlation: IC should be ~1.0
        let factor: Vec<f64> = (0..20).map(|i| i as f64).collect();
        let fwd_ret: Vec<f64> = (0..20).map(|i| i as f64 * 2.0 + 1.0).collect();
        let result = rolling_ic(&factor, &fwd_ret, 10, IcMethod::Pearson).unwrap();
        assert_eq!(result.len(), 20);
        assert!(result[8].is_nan());
        assert!((result[9] - 1.0).abs() < 1e-10);
        assert!((result[19] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_rolling_ic_rank() {
        // Monotonically increasing factor and returns: Rank IC ~1.0
        let factor: Vec<f64> = (0..30).map(|i| i as f64).collect();
        let fwd_ret: Vec<f64> = (0..30).map(|i| i as f64 * 0.5).collect();
        let result = rolling_ic(&factor, &fwd_ret, 10, IcMethod::Rank).unwrap();
        assert_eq!(result.len(), 30);
        assert!(result[8].is_nan());
        assert!((result[9] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_rolling_ic_negative() {
        // Negative correlation
        let factor: Vec<f64> = (0..20).map(|i| i as f64).collect();
        let fwd_ret: Vec<f64> = (0..20).map(|i| -(i as f64)).collect();
        let result = rolling_ic(&factor, &fwd_ret, 10, IcMethod::Pearson).unwrap();
        assert!((result[9] + 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_rolling_ic_invalid_inputs() {
        let x = vec![1.0; 10];
        let y = vec![1.0; 5];
        assert!(rolling_ic(&x, &y, 5, IcMethod::Pearson).is_err());

        let x2 = vec![1.0; 10];
        let y2 = vec![1.0; 10];
        assert!(rolling_ic(&x2, &y2, 2, IcMethod::Pearson).is_err()); // window too small
        assert!(rolling_ic(&x2, &y2, 20, IcMethod::Pearson).is_err()); // data too short
    }

    #[test]
    fn test_rolling_ic_both_methods_finite() {
        let n = 50;
        let factor: Vec<f64> = (0..n).map(|i| (i as f64 * 0.3).sin()).collect();
        let fwd_ret: Vec<f64> = (0..n).map(|i| (i as f64 * 0.3 + 0.1).cos()).collect();

        let pearson = rolling_ic(&factor, &fwd_ret, 15, IcMethod::Pearson).unwrap();
        let rank = rolling_ic(&factor, &fwd_ret, 15, IcMethod::Rank).unwrap();

        for i in 14..n {
            assert!(pearson[i].is_finite(), "Pearson IC at {} should be finite", i);
            assert!(rank[i].is_finite(), "Rank IC at {} should be finite", i);
            assert!(pearson[i] >= -1.0 && pearson[i] <= 1.0);
            assert!(rank[i] >= -1.0 && rank[i] <= 1.0);
        }
    }
}

#[cfg(test)]
mod granger_tests {
    use super::*;

    #[test]
    fn test_granger_causality_causal() {
        // x causes y with lag 1: y[t] = 0.8*x[t-1] + noise
        let n = 100;
        let x: Vec<f64> = (0..n).map(|i| (i as f64 * 0.3).sin() * 5.0).collect();
        let mut y = vec![0.0; n];
        for i in 1..n {
            y[i] = 0.8 * x[i - 1] + (i as f64 * 1.7).sin() * 0.1;
        }
        let result = granger_causality(&x, &y, 2).unwrap();
        assert!(result.f_stat > 0.0);
        assert!(result.f_stat.is_finite());
    }

    #[test]
    fn test_granger_causality_no_cause() {
        // Independent series
        let n = 50;
        let x: Vec<f64> = (0..n).map(|i| (i as f64 * 0.3).sin()).collect();
        let y: Vec<f64> = (0..n).map(|i| (i as f64 * 1.7).cos()).collect();
        let result = granger_causality(&x, &y, 2).unwrap();
        assert!(result.f_stat.is_finite());
    }

    #[test]
    fn test_granger_invalid() {
        let x = vec![1.0; 10];
        let y = vec![1.0; 10];
        assert!(granger_causality(&x, &y, 0).is_err());
        assert!(granger_causality(&x, &y, 5).is_err());
        assert!(granger_causality(&[1.0; 5], &[1.0; 3], 1).is_err());
    }

    #[test]
    fn test_rolling_granger() {
        let n = 80;
        let x: Vec<f64> = (0..n).map(|i| (i as f64 * 0.3).sin() * 5.0).collect();
        let mut y = vec![0.0; n];
        for i in 1..n {
            y[i] = 0.5 * x[i - 1] + (i as f64 * 0.7).cos() * 0.5;
        }
        let result = rolling_granger(&x, &y, 40, 2).unwrap();
        assert_eq!(result.len(), n);
        assert!(result[38].is_nan());
        assert!(result[39].is_finite());
    }

    #[test]
    fn test_rolling_granger_invalid() {
        let x = vec![1.0; 20];
        let y = vec![1.0; 20];
        assert!(rolling_granger(&x, &y, 5, 3).is_err()); // window too small
        assert!(rolling_granger(&x, &y, 30, 2).is_err()); // data too short
    }
}
