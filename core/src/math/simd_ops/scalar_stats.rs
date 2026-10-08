//! Portable scalar fallbacks for statistics and regression.

use super::prelude::*;

pub(crate) fn stddev_scalar(data: &[f64], period: usize, result: &mut [f64]) {
    let len = data.len().min(result.len());
    if period < 2 || len == 0 || len < period {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    let inv_w = 1.0 / period as f64;
    let inv_w_minus_1 = 1.0 / (period as f64 - 1.0);

    let mut sum: f64 = data[..period].iter().sum();
    let mut sum_sq: f64 = data[..period].iter().map(|x| x * x).sum();
    let mean = sum * inv_w;
    let var = (sum_sq - sum * mean) * inv_w_minus_1;
    result[period - 1] = f64_sqrt(var.max(0.0));

    for i in period..len {
        let old = data[i - period];
        let new = data[i];
        sum += new - old;
        sum_sq += new * new - old * old;
        let m = sum * inv_w;
        let var = (sum_sq - sum * m) * inv_w_minus_1;
        result[i] = f64_sqrt(var.max(0.0));
    }

    for r in result.iter_mut().take(period - 1) {
        *r = f64::NAN;
    }
}

pub(crate) fn variance_scalar(data: &[f64], period: usize, result: &mut [f64]) {
    let len = data.len().min(result.len());
    if period < 2 || len == 0 || len < period {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    let inv_w = 1.0 / period as f64;
    let inv_w_minus_1 = 1.0 / (period as f64 - 1.0);

    let mut sum: f64 = data[..period].iter().sum();
    let mut sum_sq: f64 = data[..period].iter().map(|x| x * x).sum();
    let mean = sum * inv_w;
    let var = (sum_sq - sum * mean) * inv_w_minus_1;
    result[period - 1] = var.max(0.0);

    for i in period..len {
        let old = data[i - period];
        let new = data[i];
        sum += new - old;
        sum_sq += new * new - old * old;
        let m = sum * inv_w;
        let var = (sum_sq - sum * m) * inv_w_minus_1;
        result[i] = var.max(0.0);
    }

    for r in result.iter_mut().take(period - 1) {
        *r = f64::NAN;
    }
}

pub(crate) fn zscore_optimized_scalar(data: &[f64], period: usize, result: &mut [f64]) {
    let len = data.len().min(result.len());
    if period < 2 || len == 0 || len < period {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    let inv_w = 1.0 / period as f64;
    let inv_w_minus_1 = 1.0 / (period as f64 - 1.0);

    let mut sum: f64 = data[..period].iter().sum();
    let mut sum_sq: f64 = data[..period].iter().map(|x| x * x).sum();
    let mean = sum * inv_w;
    let var = (sum_sq - sum * mean) * inv_w_minus_1;
    let std = f64_sqrt(var.max(0.0));
    result[period - 1] = if std.abs() < crate::utils::NUMERIC_EPSILON {
        0.0
    } else {
        (data[period - 1] - mean) / std
    };

    for i in period..len {
        let old = data[i - period];
        let new = data[i];
        sum += new - old;
        sum_sq += new * new - old * old;
        let m = sum * inv_w;
        let var = (sum_sq - sum * m) * inv_w_minus_1;
        let std = f64_sqrt(var.max(0.0));
        result[i] = if std.abs() < crate::utils::NUMERIC_EPSILON {
            0.0
        } else {
            (data[i] - m) / std
        };
    }

    for r in result.iter_mut().take(period - 1) {
        *r = f64::NAN;
    }
}

#[allow(clippy::similar_names)]
pub(crate) fn correl_scalar(x: &[f64], y: &[f64], period: usize, result: &mut [f64]) {
    let len = x.len().min(y.len()).min(result.len());
    if period < 2 || len == 0 || len < period {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    let inv_w = 1.0 / period as f64;
    let inv_w_minus_1 = 1.0 / (period as f64 - 1.0);

    let mut sum_x: f64 = x[..period].iter().sum();
    let mut sum_y: f64 = y[..period].iter().sum();
    let mut sum_xy: f64 = x[..period]
        .iter()
        .zip(y[..period].iter())
        .map(|(xi, yi)| xi * yi)
        .sum();
    let mut sum_x2: f64 = x[..period].iter().map(|xi| xi * xi).sum();
    let mut sum_y2: f64 = y[..period].iter().map(|yi| yi * yi).sum();

    let mean_x = sum_x * inv_w;
    let mean_y = sum_y * inv_w;
    let cov = (sum_xy - sum_x * mean_y) * inv_w_minus_1;
    let var_x = (sum_x2 - sum_x * mean_x) * inv_w_minus_1;
    let var_y = (sum_y2 - sum_y * mean_y) * inv_w_minus_1;
    let denom = f64_sqrt(var_x.max(0.0) * var_y.max(0.0));
    result[period - 1] = if denom.abs() < crate::utils::NUMERIC_EPSILON {
        f64::NAN
    } else {
        cov / denom
    };

    for i in period..len {
        let old_x = x[i - period];
        let old_y = y[i - period];
        let new_x = x[i];
        let new_y = y[i];
        sum_x += new_x - old_x;
        sum_y += new_y - old_y;
        sum_xy += new_x * new_y - old_x * old_y;
        sum_x2 += new_x * new_x - old_x * old_x;
        sum_y2 += new_y * new_y - old_y * old_y;

        let mean_x = sum_x * inv_w;
        let mean_y = sum_y * inv_w;
        let cov = (sum_xy - sum_x * mean_y) * inv_w_minus_1;
        let var_x = (sum_x2 - sum_x * mean_x) * inv_w_minus_1;
        let var_y = (sum_y2 - sum_y * mean_y) * inv_w_minus_1;
        let denom = f64_sqrt(var_x.max(0.0) * var_y.max(0.0));
        result[i] = if denom.abs() < crate::utils::NUMERIC_EPSILON {
            f64::NAN
        } else {
            cov / denom
        };
    }

    for r in result.iter_mut().take(period - 1) {
        *r = f64::NAN;
    }
}

#[allow(clippy::similar_names)]
pub(crate) fn beta_scalar(asset: &[f64], benchmark: &[f64], period: usize, result: &mut [f64]) {
    let len = asset.len().min(benchmark.len()).min(result.len());
    if period < 2 || len == 0 || len < period {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    let inv_w = 1.0 / period as f64;
    let inv_w_minus_1 = 1.0 / (period as f64 - 1.0);

    let mut sum_a: f64 = asset[..period].iter().sum();
    let mut sum_b: f64 = benchmark[..period].iter().sum();
    let mut sum_ab: f64 = asset[..period]
        .iter()
        .zip(benchmark[..period].iter())
        .map(|(a, b)| a * b)
        .sum();
    let mut sum_b2: f64 = benchmark[..period].iter().map(|b| b * b).sum();

    let _mean_a = sum_a * inv_w;
    let mean_b = sum_b * inv_w;
    let cov = (sum_ab - sum_a * mean_b) * inv_w_minus_1;
    let var_b = (sum_b2 - sum_b * mean_b) * inv_w_minus_1;
    result[period - 1] = if var_b.abs() < crate::utils::NUMERIC_EPSILON {
        f64::NAN
    } else {
        cov / var_b
    };

    for i in period..len {
        let old_a = asset[i - period];
        let old_b = benchmark[i - period];
        let new_a = asset[i];
        let new_b = benchmark[i];
        sum_a += new_a - old_a;
        sum_b += new_b - old_b;
        sum_ab += new_a * new_b - old_a * old_b;
        sum_b2 += new_b * new_b - old_b * old_b;

        let _mean_a = sum_a * inv_w;
        let mean_b = sum_b * inv_w;
        let cov = (sum_ab - sum_a * mean_b) * inv_w_minus_1;
        let var_b = (sum_b2 - sum_b * mean_b) * inv_w_minus_1;
        result[i] = if var_b.abs() < crate::utils::NUMERIC_EPSILON {
            f64::NAN
        } else {
            cov / var_b.max(0.0)
        };
    }

    for r in result.iter_mut().take(period - 1) {
        *r = f64::NAN;
    }
}

pub(crate) fn linreg_slope_scalar(data: &[f64], period: usize, result: &mut [f64]) {
    let len = data.len().min(result.len());
    if period < 2 || len == 0 || len < period {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    // See `linreg_slope_avx2` for why the weights run oldest-first and why the
    // advance is `sum_xy += sum_y - period * old`. This scalar twin has to use
    // the same orientation, or the two paths stop agreeing to the tolerance the
    // numeric contract pins them to.
    let p = period as f64;
    let p1 = (period - 1) as f64;
    let sum_x = p * p1 / 2.0;
    let sum_x2 = p * p1 * (2.0 * p - 1.0) / 6.0;
    let divisor = sum_x * sum_x - p * sum_x2;
    let inv_divisor = 1.0 / divisor;

    let mut sum_y: f64 = data[..period].iter().sum();
    let mut sum_xy: f64 = 0.0;
    let mut weight = p1;
    for &val in data[..period].iter() {
        sum_xy += weight * val;
        weight -= 1.0;
    }
    result[period - 1] = (p * sum_xy - sum_x * sum_y) * inv_divisor;

    for i in period..len {
        let old_val = data[i - period];
        let new_val = data[i];
        sum_xy += sum_y - p * old_val;
        sum_y += new_val - old_val;
        result[i] = (p * sum_xy - sum_x * sum_y) * inv_divisor;
    }

    for r in result.iter_mut().take(period - 1) {
        *r = f64::NAN;
    }
}

#[allow(clippy::similar_names)]
pub(crate) fn linreg_scalar(data: &[f64], period: usize, result: &mut [f64]) {
    let len = data.len().min(result.len());
    if period < 2 || len == 0 || len < period {
        for r in result.iter_mut().take(len) {
            *r = f64::NAN;
        }
        return;
    }

    let p = period as f64;
    let last_x = (period - 1) as f64;
    // See `linreg_avx2`: oldest-first weights, and the same reason.
    let sum_x = p * last_x / 2.0;
    let sum_x2 = p * last_x * (2.0 * p - 1.0) / 6.0;
    let divisor = sum_x * sum_x - p * sum_x2;
    // Loop-invariant reciprocals, same as `linreg_slope_scalar`: the slope and
    // the intercept each divide by a quantity that never changes, so both
    // become multiplies and the `divsd` leaves the per-bar critical path.
    let inv_divisor = 1.0 / divisor;
    let inv_p = 1.0 / p;

    let mut sum_y: f64 = data[..period].iter().sum();
    let mut sum_xy: f64 = 0.0;
    let mut weight = last_x;
    for &val in data[..period].iter() {
        sum_xy += weight * val;
        weight -= 1.0;
    }
    let slope = (p * sum_xy - sum_x * sum_y) * inv_divisor;
    let intercept = (sum_y - slope * sum_x) * inv_p;
    result[period - 1] = slope * last_x + intercept;

    for i in period..len {
        let old_val = data[i - period];
        let new_val = data[i];
        sum_xy += sum_y - p * old_val;
        sum_y += new_val - old_val;
        let slope = (p * sum_xy - sum_x * sum_y) * inv_divisor;
        let intercept = (sum_y - slope * sum_x) * inv_p;
        result[i] = slope * last_x + intercept;
    }

    for r in result.iter_mut().take(period - 1) {
        *r = f64::NAN;
    }
}
