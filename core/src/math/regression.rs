//! Canonical regression and residualization kernels for factors and research.

use crate::error::{Result, TaError};

/// Multivariate linear-regression result.
#[derive(Debug, Clone, PartialEq)]
pub struct RegressionResult {
    pub intercept: f64,
    pub coefficients: Vec<f64>,
    pub r_squared: f64,
    pub residuals: Vec<f64>,
    pub observations: usize,
}

fn solve_linear(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for col in 0..n {
        let pivot = (col..n).max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))?;
        if a[pivot][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, pivot);
        b.swap(col, pivot);
        let div = a[col][col];
        for j in col..n {
            a[col][j] /= div;
        }
        b[col] /= div;
        for row in 0..n {
            if row == col {
                continue;
            }
            let factor = a[row][col];
            if factor == 0.0 {
                continue;
            }
            for j in col..n {
                a[row][j] -= factor * a[col][j];
            }
            b[row] -= factor * b[col];
        }
    }
    Some(b)
}

fn fit_impl(
    y: &[f64],
    x: &[&[f64]],
    weights: Option<&[f64]>,
    ridge: f64,
) -> Result<RegressionResult> {
    if x.iter().any(|col| col.len() != y.len()) {
        return Err(TaError::InvalidParameter {
            name: "x".to_string(),
            constraint: "all exposure columns must align with y".to_string(),
        });
    }
    if let Some(w) = weights {
        if w.len() != y.len() {
            return Err(TaError::InvalidParameter {
                name: "weights".to_string(),
                constraint: "must align with y".to_string(),
            });
        }
    }
    let p = x.len() + 1;
    let valid_rows: Vec<usize> = (0..y.len())
        .filter(|&row| {
            y[row].is_finite()
                && x.iter().all(|col| col[row].is_finite())
                && weights.map_or(true, |w| w[row].is_finite() && w[row] > 0.0)
        })
        .collect();
    if valid_rows.len() < p {
        return Err(TaError::InsufficientData {
            length: valid_rows.len(),
            required: p,
        });
    }

    let mut xtx = vec![vec![0.0; p]; p];
    let mut xty = vec![0.0; p];
    for &row in &valid_rows {
        let weight = weights.map_or(1.0, |w| w[row]);
        let mut design = Vec::with_capacity(p);
        design.push(1.0);
        for col in x {
            design.push(col[row]);
        }
        for i in 0..p {
            xty[i] += weight * design[i] * y[row];
            for j in 0..p {
                xtx[i][j] += weight * design[i] * design[j];
            }
        }
    }
    if ridge > 0.0 {
        for i in 1..p {
            xtx[i][i] += ridge;
        }
    }
    let beta = solve_linear(xtx, xty).ok_or_else(|| TaError::ComputationError {
        message: "regression design matrix is singular".to_string(),
    })?;

    let mean_y = valid_rows.iter().map(|&row| y[row]).sum::<f64>() / valid_rows.len() as f64;
    let mut ss_res = 0.0;
    let mut ss_tot = 0.0;
    let mut residuals = vec![f64::NAN; y.len()];
    for &row in &valid_rows {
        let prediction = beta[0]
            + x.iter()
                .enumerate()
                .map(|(j, col)| beta[j + 1] * col[row])
                .sum::<f64>();
        let residual = y[row] - prediction;
        residuals[row] = residual;
        ss_res += residual * residual;
        let centered = y[row] - mean_y;
        ss_tot += centered * centered;
    }
    let r_squared = if ss_tot <= f64::EPSILON {
        1.0
    } else {
        1.0 - ss_res / ss_tot
    };
    Ok(RegressionResult {
        intercept: beta[0],
        coefficients: beta[1..].to_vec(),
        r_squared,
        residuals,
        observations: valid_rows.len(),
    })
}

/// Ordinary least squares with intercept.
pub fn ols(y: &[f64], x: &[&[f64]]) -> Result<RegressionResult> {
    fit_impl(y, x, None, 0.0)
}

/// Weighted least squares with intercept.
pub fn weighted_least_squares(
    y: &[f64],
    x: &[&[f64]],
    weights: &[f64],
) -> Result<RegressionResult> {
    fit_impl(y, x, Some(weights), 0.0)
}

/// Ridge-stabilized OLS; the intercept is not penalized.
pub fn ridge(y: &[f64], x: &[&[f64]], lambda: f64) -> Result<RegressionResult> {
    if lambda < 0.0 || !lambda.is_finite() {
        return Err(TaError::InvalidParameter {
            name: "lambda".to_string(),
            constraint: "must be finite and >= 0".to_string(),
        });
    }
    fit_impl(y, x, None, lambda)
}

/// Residualize a target against one or more exposures.
pub fn residualize(y: &[f64], x: &[&[f64]]) -> Result<Vec<f64>> {
    Ok(ols(y, x)?.residuals)
}

/// Simple OLS slope without materializing residuals; used by rolling/statistical helpers.
pub fn simple_slope(y: &[f64], x: &[f64]) -> Result<f64> {
    if y.len() != x.len() {
        return Err(TaError::InvalidParameter {
            name: "x, y".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    let valid: Vec<(f64, f64)> = y
        .iter()
        .copied()
        .zip(x.iter().copied())
        .filter(|(yv, xv)| yv.is_finite() && xv.is_finite())
        .collect();
    if valid.len() < 2 {
        return Err(TaError::InsufficientData {
            length: valid.len(),
            required: 2,
        });
    }
    let n = valid.len() as f64;
    let sum_x = valid.iter().map(|(_, xv)| *xv).sum::<f64>();
    let sum_y = valid.iter().map(|(yv, _)| *yv).sum::<f64>();
    let sum_xy = valid.iter().map(|(yv, xv)| yv * xv).sum::<f64>();
    let sum_x2 = valid.iter().map(|(_, xv)| xv * xv).sum::<f64>();
    let denom = n * sum_x2 - sum_x * sum_x;
    if denom.abs() < 1e-15 {
        return Err(TaError::ComputationError {
            message: "regression design is singular".to_string(),
        });
    }
    Ok((n * sum_xy - sum_x * sum_y) / denom)
}

/// Simple OLS facade for one exposure.
pub fn simple_ols(y: &[f64], x: &[f64]) -> Result<RegressionResult> {
    ols(y, &[x])
}

/// Index design used by the rolling regression operators, `idx = 0..window`.
///
/// Only the *spread* of the index matters to a slope, an `R²` or a residual, so
/// this matches Qlib's `x = 1..window` accumulators (`qlib/data/_libs/rolling.pyx`)
/// to within floating-point noise while avoiding the extra `+1`.
#[allow(clippy::cast_precision_loss)] // `window` is a bar count, far below 2^53
fn index_design(window: usize) -> Vec<f64> {
    (0..window).map(|step| step as f64).collect()
}

/// Is the window's variance numerically absent?
///
/// Uses the same noise-floor predicate [`crate::math::degenerate_variance`]
/// applies to `CORREL`, so `RSQUARE` and `CORREL` cannot disagree about which
/// windows are degenerate. An exactly-constant window is the extreme case; a
/// merely *nearly* constant one is also caught, which is the honest answer,
/// since an `R²` computed from such a window is cancellation noise rather than
/// a fit quality.
#[allow(clippy::cast_precision_loss)] // `window.len()` is a bar count, far below 2^53
fn variance_is_absent(window: &[f64]) -> bool {
    let size = window.len() as f64;
    let sum: f64 = window.iter().sum();
    let sum_sq: f64 = window.iter().map(|value| value * value).sum();
    crate::math::degenerate_variance(sum_sq - sum * sum / size, sum_sq, size)
}

/// Rolling `R²` of a linear fit against the bar index, written into caller-owned
/// output.
///
/// This is Qlib's `Rsquare` (`qlib/data/ops.py::Rsquare`, backed by
/// `qlib/data/_libs/rolling.pyx::Rsquare`), which squares the correlation
/// between the window and its indices — for a one-regressor OLS with intercept
/// that is exactly `RegressionResult::r_squared`, so the shared
/// [`simple_ols`] kernel is reused rather than re-derived.
///
/// One deliberate departure from Qlib: Qlib additionally NaNs out any window
/// whose rolling std is within `atol=2e-05` of zero. That is a Qlib data-cleaning
/// policy, not a property of the statistic, so finkit does not adopt it — the
/// only windows finkit reports as `NaN` are ones where `R²` is genuinely
/// undefined, i.e. where the response has no variance, detected by
/// `variance_is_absent`. On data whose windows are either numerically constant
/// or comfortably non-degenerate, the two policies agree exactly; they can only
/// differ in a narrow band of near-constant windows, where Qlib's absolute
/// `2e-05` is scale-dependent and finkit's floor is not.
///
/// A bar is also `NaN` unless its window is full and every value in it is
/// finite; see [`crate::math::quantile::rolling_quantile_into`] for why finkit's
/// window convention differs from Qlib's `min_periods=1`.
///
/// # Errors
///
/// Returns [`TaError::EmptyInput`] for empty `data`, [`TaError::InvalidParameter`]
/// for `window == 0` or an `output` that does not match `data` in length.
pub fn rolling_rsquare_into(data: &[f64], window: usize, output: &mut [f64]) -> Result<()> {
    rolling_regression_into(data, window, output, RegressionOutput::RSquared)
}

/// Rolling residual of the **newest** bar in each window from its own linear
/// fit, written into caller-owned output.
///
/// This is Qlib's `Resi` (`qlib/data/ops.py::Resi`, backed by
/// `qlib/data/_libs/rolling.pyx::Resi`): the kernel returns
/// `val - (slope * window + interp)`, i.e. the deviation of the current bar from
/// the fitted line. [`RegressionResult::residuals`] carries exactly that, so no
/// second formulation of the fit is needed.
///
/// Unlike [`rolling_rsquare_into`] this needs no zero-variance guard: a constant
/// window has slope `0`, so the residual is `0.0` — which is also what Qlib's
/// kernel returns, since its numerator vanishes while its denominator does not.
///
/// # Errors
///
/// Same as [`rolling_rsquare_into`].
pub fn rolling_resi_into(data: &[f64], window: usize, output: &mut [f64]) -> Result<()> {
    rolling_regression_into(data, window, output, RegressionOutput::Residual)
}

/// Which part of the per-window fit [`rolling_regression_into`] publishes.
#[derive(Clone, Copy)]
enum RegressionOutput {
    RSquared,
    Residual,
}

/// Shared driver for the rolling regression operators.
fn rolling_regression_into(
    data: &[f64],
    window: usize,
    output: &mut [f64],
    want: RegressionOutput,
) -> Result<()> {
    if data.is_empty() {
        return Err(TaError::EmptyInput);
    }
    if window == 0 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    if output.len() != data.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as data".to_string(),
        });
    }

    output.fill(f64::NAN);
    if window > data.len() {
        return Ok(());
    }
    let design = index_design(window);
    for index in window - 1..data.len() {
        let slice = &data[index + 1 - window..=index];
        if slice.iter().any(|value| !value.is_finite()) {
            continue;
        }
        if matches!(want, RegressionOutput::RSquared) && variance_is_absent(slice) {
            continue;
        }
        let Ok(fit) = simple_ols(slice, &design) else {
            continue;
        };
        output[index] = match want {
            RegressionOutput::RSquared => fit.r_squared,
            RegressionOutput::Residual => fit.residuals[window - 1],
        };
    }
    Ok(())
}

/// Rolling `R²` as an owned series.
///
/// # Errors
///
/// Propagates [`rolling_rsquare_into`]'s errors.
pub fn rolling_rsquare(data: &[f64], window: usize) -> Result<Vec<f64>> {
    let mut output = vec![f64::NAN; data.len()];
    rolling_rsquare_into(data, window, &mut output)?;
    Ok(output)
}

/// Rolling newest-bar residual as an owned series.
///
/// # Errors
///
/// Propagates [`rolling_resi_into`]'s errors.
pub fn rolling_resi(data: &[f64], window: usize) -> Result<Vec<f64>> {
    let mut output = vec![f64::NAN; data.len()];
    rolling_resi_into(data, window, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_two_factor_model() {
        let x1 = [1.0, 2.0, 3.0, 4.0, 5.0];
        let x2 = [2.0, 1.0, 0.0, 1.0, 2.0];
        let y: Vec<f64> = x1
            .iter()
            .zip(x2)
            .map(|(&a, b)| 3.0 + 2.0 * a - 0.5 * b)
            .collect();
        let fit = ols(&y, &[&x1, &x2]).unwrap();
        assert!((fit.intercept - 3.0).abs() < 1e-9);
        assert!((fit.coefficients[0] - 2.0).abs() < 1e-9);
        assert!((fit.coefficients[1] + 0.5).abs() < 1e-9);
    }
}
