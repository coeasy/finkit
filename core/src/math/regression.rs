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
