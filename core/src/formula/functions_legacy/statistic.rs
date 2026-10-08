//! Rolling-statistic formula shims (delegate to `math::statistics`).

use super::prelude::*;

pub(crate) fn fn_avgdev(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("AVGDEV", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "AVGDEV")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_avgdev(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_zscore(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ZSCORE", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "ZSCORE")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    // Delegate to the canonical implementation rather than hand-rolling
    // `rolling_mean` + `rolling_std_dev`. The plan path's kernel calls
    // `statistics::zscore_into`, which is bit-identical to `statistics::zscore`,
    // but `rolling_std_dev` uses Welford's removable variance and disagreed with
    // it by ~2.5e-10 absolute -- a live tree-vs-plan divergence on ordinary
    // NaN-free input that the all-NaN warm-up bug had been masking.
    match lib_zscore(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_histvol(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HISTVOL", args, 2)?;
    let close = &args[0];
    let n = extract_n(args, 1, "HISTVOL")?;

    let data_len = ctx.data_len;
    let mut log_returns = Array1::zeros(data_len);
    log_returns[0] = 0.0;
    for i in 1..data_len {
        if close[i - 1].abs() > crate::utils::NUMERIC_EPSILON {
            log_returns[i] = (close[i] / close[i - 1]).ln();
        }
    }

    match lib_stat::rolling_std_dev(log_returns.as_slice().unwrap(), n) {
        Ok(result) => Ok(result.mapv(|v| v * (252.0_f64).sqrt())),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

// ======================== MATH / STATISTICS EXTENSIONS (TDX) ========================

pub(crate) fn fn_avedev(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("AVEDEV", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "AVEDEV")?;
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let values = input.as_slice().unwrap();
    for i in (n - 1)..len {
        let window = &values[i + 1 - n..=i];
        let mean: f64 = window.iter().sum::<f64>() / n as f64;
        let avedev: f64 = window.iter().map(|x| (x - mean).abs()).sum::<f64>() / n as f64;
        out[i] = avedev;
    }
    Ok(out)
}

pub(crate) fn fn_devsq(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DEVSQ", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "DEVSQ")?;
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let values = input.as_slice().unwrap();
    for i in (n - 1)..len {
        let window = &values[i + 1 - n..=i];
        let mean: f64 = window.iter().sum::<f64>() / n as f64;
        let devsq: f64 = window.iter().map(|x| (x - mean).powi(2)).sum::<f64>();
        out[i] = devsq;
    }
    Ok(out)
}

pub(crate) fn fn_slope(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SLOPE", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "SLOPE")?;
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let values = input.as_slice().unwrap();
    for i in (n - 1)..len {
        let window = &values[i + 1 - n..=i];
        let n_f = n as f64;
        let sum_x: f64 = (0..n).map(|j| j as f64).sum();
        let sum_y: f64 = window.iter().sum();
        let sum_xy: f64 = window.iter().enumerate().map(|(j, &y)| j as f64 * y).sum();
        let sum_x2: f64 = (0..n).map(|j| (j as f64).powi(2)).sum();
        let denom = n_f * sum_x2 - sum_x * sum_x;
        if denom.abs() > crate::utils::NUMERIC_EPSILON {
            out[i] = (n_f * sum_xy - sum_x * sum_y) / denom;
        } else {
            out[i] = 0.0;
        }
    }
    Ok(out)
}

pub(crate) fn fn_forcast(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FORCAST", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "FORCAST")?;
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let values = input.as_slice().unwrap();
    for i in (n - 1)..len {
        let window = &values[i + 1 - n..=i];
        let n_f = n as f64;
        let sum_x: f64 = (0..n).map(|j| j as f64).sum();
        let sum_y: f64 = window.iter().sum();
        let sum_xy: f64 = window.iter().enumerate().map(|(j, &y)| j as f64 * y).sum();
        let sum_x2: f64 = (0..n).map(|j| (j as f64).powi(2)).sum();
        let denom = n_f * sum_x2 - sum_x * sum_x;
        if denom.abs() > crate::utils::NUMERIC_EPSILON {
            let slope = (n_f * sum_xy - sum_x * sum_y) / denom;
            let intercept = (sum_y - slope * sum_x) / n_f;
            out[i] = intercept + slope * (n_f - 1.0);
        } else {
            out[i] = window.iter().sum::<f64>() / n_f;
        }
    }
    Ok(out)
}

pub(crate) fn fn_range(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("RANGE", args, 3)?;
    let x = &args[0];
    let a = &args[1];
    let b = &args[2];
    let len = x.len();
    let mut out = Array1::zeros(len);
    for i in 0..len {
        if x[i] > a[i] && x[i] < b[i] {
            out[i] = 1.0;
        }
    }
    Ok(out)
}

/// PERCENTILE(X, N, P): P-th percentile over N-bar window
pub(crate) fn fn_percentile(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("PERCENTILE", args, 3)?;
    let input = &args[0];
    let n = extract_n(args, 1, "PERCENTILE")?;
    let p = args[2][0] / 100.0; // P is 0-100
    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in 0..data_len {
        if i + 1 < n {
            continue;
        }
        let start = i + 1 - n;
        let mut window: Vec<f64> = input
            .slice(s![start..=i])
            .iter()
            .copied()
            .filter(|v| !v.is_nan())
            .collect();
        if window.is_empty() {
            continue;
        }
        window.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = (p * (window.len() - 1) as f64).round() as usize;
        let idx = idx.min(window.len() - 1);
        result[i] = window[idx];
    }
    Ok(result)
}

/// MEDIAN(X, N): Median over N-bar window
///
/// Delegates to `math::statistics::rolling_median` so the tree path and the
/// compiled-plan `CALL:MEDIAN` kernel share one implementation.
pub(crate) fn fn_median(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MEDIAN", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "MEDIAN")?;
    if input.is_empty() {
        return Ok(nan_vec(ctx.data_len));
    }
    crate::math::statistics::rolling_median(input.as_slice().unwrap_or_default(), n)
        .map_err(|_| FormulaError::InvalidParameter("MEDIAN window is invalid".to_string()))
}

/// ROLLING_RANGE(X, N): rolling maximum minus rolling minimum.
/// Kept separate from the TDX `RANGE(X, A, B)` predicate so dialects cannot
/// silently inherit incompatible arity or boolean semantics.
///
/// Delegates to `math::statistics::rolling_range` so the tree path and the
/// compiled-plan `CALL:ROLLING_RANGE` kernel share one implementation.
pub(crate) fn fn_rolling_range(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ROLLING_RANGE", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "ROLLING_RANGE")?;
    if input.is_empty() {
        return Ok(nan_vec(ctx.data_len));
    }
    crate::math::statistics::rolling_range(input.as_slice().unwrap_or_default(), n)
        .map_err(|_| FormulaError::InvalidParameter("ROLLING_RANGE window is invalid".into()))
}

// === Higher-order statistics ===

/// SKEW(X, N): Rolling skewness over N-bar window
pub(crate) fn fn_skew(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SKEW", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "SKEW")?;
    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in 0..data_len {
        if i + 1 < n {
            continue;
        }
        let start = i + 1 - n;
        let window: Vec<f64> = input
            .slice(s![start..=i])
            .iter()
            .copied()
            .filter(|v| !v.is_nan())
            .collect();
        let count = window.len();
        if count < 3 {
            continue;
        }
        let mean = window.iter().sum::<f64>() / count as f64;
        let m2: f64 = window.iter().map(|x| (x - mean).powi(2)).sum();
        let m3: f64 = window.iter().map(|x| (x - mean).powi(3)).sum();
        let variance = m2 / count as f64;
        if variance.abs() < f64::EPSILON {
            result[i] = 0.0;
        } else {
            let std_dev = variance.sqrt();
            result[i] = (m3 / count as f64) / std_dev.powi(3);
        }
    }
    Ok(result)
}

/// KURT(X, N): Rolling kurtosis over N-bar window (excess kurtosis)
pub(crate) fn fn_kurt(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("KURT", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "KURT")?;
    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in 0..data_len {
        if i + 1 < n {
            continue;
        }
        let start = i + 1 - n;
        let window: Vec<f64> = input
            .slice(s![start..=i])
            .iter()
            .copied()
            .filter(|v| !v.is_nan())
            .collect();
        let count = window.len();
        if count < 4 {
            continue;
        }
        let mean = window.iter().sum::<f64>() / count as f64;
        let m2: f64 = window.iter().map(|x| (x - mean).powi(2)).sum();
        let m4: f64 = window.iter().map(|x| (x - mean).powi(4)).sum();
        let variance = m2 / count as f64;
        if variance.abs() < f64::EPSILON {
            result[i] = 0.0;
        } else {
            result[i] = (m4 / count as f64) / variance.powi(2) - 3.0;
        }
    }
    Ok(result)
}

/// MODE(X, N): Most frequent value in N-bar window (approximated by rounding to 2 decimals)
pub(crate) fn fn_mode(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MODE", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "MODE")?;
    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    // One reused map instead of a `Vec` allocation per bar. Ties are broken by
    // the **earliest occurrence in the window**: `max_by_key(count)` over a
    // `HashMap` iteration picked an arbitrary winner, and Rust's `RandomState`
    // makes that order differ between processes — the same formula could return
    // different values on different runs.
    let mut counts: std::collections::HashMap<i64, (usize, usize, f64)> =
        std::collections::HashMap::new();
    for i in 0..data_len {
        if i + 1 < n {
            continue;
        }
        let start = i + 1 - n;
        counts.clear();
        for (offset, &value) in input.slice(s![start..=i]).iter().enumerate() {
            if value.is_nan() {
                continue;
            }
            let key = (value * 100.0).round() as i64;
            let entry = counts.entry(key).or_insert((0, start + offset, value));
            entry.0 += 1;
        }
        let mut best: Option<(usize, usize, f64)> = None;
        for (_, entry) in counts.iter() {
            let (count, first_idx, value) = *entry;
            let wins = match best {
                None => true,
                Some((best_count, best_first, _)) => {
                    count > best_count || (count == best_count && first_idx < best_first)
                }
            };
            if wins {
                best = Some((count, first_idx, value));
            }
        }
        if let Some((_, _, value)) = best {
            result[i] = value;
        }
    }
    Ok(result)
}

/// SORT(X, N, DIR): Sort the last N values. DIR=1 ascending, DIR=0 descending.
/// Returns the sorted rank position (1-based) of current value within its window.
pub(crate) fn fn_sort(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SORT", args, 3)?;
    let input = &args[0];
    let n = extract_n(args, 1, "SORT")?;
    let dir = args[2][0] as i32; // 1=asc, 0=desc
    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in 0..data_len {
        if i + 1 < n {
            continue;
        }
        let start = i + 1 - n;
        let mut window: Vec<f64> = input
            .slice(s![start..=i])
            .iter()
            .copied()
            .filter(|v| !v.is_nan())
            .collect();
        if window.is_empty() {
            continue;
        }
        let current = input[i];
        if current.is_nan() {
            continue;
        }
        if dir == 1 {
            window.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        } else {
            window.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        }
        let rank = window
            .iter()
            .position(|&v| (v - current).abs() < f64::EPSILON)
            .map(|p| p + 1)
            .unwrap_or(0);
        result[i] = rank as f64;
    }
    Ok(result)
}

/// RANK(X, N): Percentile rank of current value within N-bar window (0-100)
pub(crate) fn fn_rank(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("RANK", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "RANK")?;
    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in 0..data_len {
        if i + 1 < n {
            continue;
        }
        let start = i + 1 - n;
        let current = input[i];
        if current.is_nan() {
            continue;
        }
        let window: Vec<f64> = input
            .slice(s![start..=i])
            .iter()
            .copied()
            .filter(|v| !v.is_nan())
            .collect();
        if window.is_empty() {
            continue;
        }
        let below = window.iter().filter(|&&v| v < current).count();
        result[i] = (below as f64 / (window.len() - 1).max(1) as f64) * 100.0;
    }
    Ok(result)
}
