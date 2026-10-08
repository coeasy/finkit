//! `ZigZag` pivot discovery and its PEAK/TROUGH/FINDHIGH family.

use super::prelude::*;

pub(crate) fn compute_zigzag_pivots(
    ctx: &FormulaContext,
    threshold_pct: f64,
) -> (Vec<(usize, f64, bool)>, usize) {
    let high = ctx.high.as_slice();
    let low = ctx.low.as_slice();
    let data_len = ctx.data_len;

    let zz_result = lib_zigzag(high, low, threshold_pct);
    match zz_result {
        Ok(result) => {
            let mut classified: Vec<(usize, f64, bool)> = Vec::new();
            for i in 0..result.pivots.len() {
                let (idx, price) = result.pivots[i];
                let is_peak = if i == 0 {
                    if result.pivots.len() > 1 {
                        price > result.pivots[1].1
                    } else {
                        true
                    }
                } else {
                    price > result.pivots[i - 1].1
                };
                classified.push((idx, price, is_peak));
            }
            (classified, data_len)
        }
        Err(_) => (Vec::new(), data_len),
    }
}

pub(crate) fn fn_peak(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("PEAK", args, 3)?;
    let n = extract_f64_arg(args, 1, "PEAK")?;
    let m = args[2][0] as usize;
    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    let (pivots, _) = compute_zigzag_pivots(ctx, n);
    let peaks: Vec<(usize, f64)> = pivots
        .iter()
        .filter(|(_, _, is_peak)| *is_peak)
        .map(|(idx, price, _)| (*idx, *price))
        .collect();

    fill_mth_recent_pivot(
        &peaks,
        m,
        data_len,
        result.as_slice_mut().unwrap(),
        |_, _, pval| pval,
    );

    Ok(result)
}

pub(crate) fn fn_trough(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TROUGH", args, 3)?;
    let n = extract_f64_arg(args, 1, "TROUGH")?;
    let m = args[2][0] as usize;
    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    let (pivots, _) = compute_zigzag_pivots(ctx, n);
    let troughs: Vec<(usize, f64)> = pivots
        .iter()
        .filter(|(_, _, is_peak)| !*is_peak)
        .map(|(idx, price, _)| (*idx, *price))
        .collect();

    fill_mth_recent_pivot(
        &troughs,
        m,
        data_len,
        result.as_slice_mut().unwrap(),
        |_, _, pval| pval,
    );

    Ok(result)
}

pub(crate) fn fn_peakbars(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("PEAKBARS", args, 3)?;
    let n = extract_f64_arg(args, 1, "PEAKBARS")?;
    let m = args[2][0] as usize;
    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    let (pivots, _) = compute_zigzag_pivots(ctx, n);
    let peaks: Vec<usize> = pivots
        .iter()
        .filter(|(_, _, is_peak)| *is_peak)
        .map(|(idx, _, _)| *idx)
        .collect();

    let pivot_values: Vec<(usize, f64)> = peaks.iter().map(|&idx| (idx, 0.0)).collect();
    fill_mth_recent_pivot(
        &pivot_values,
        m,
        data_len,
        result.as_slice_mut().unwrap(),
        |i, pidx, _| (i - pidx) as f64,
    );

    Ok(result)
}

pub(crate) fn fn_troughbars(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TROUGHBARS", args, 3)?;
    let n = extract_f64_arg(args, 1, "TROUGHBARS")?;
    let m = args[2][0] as usize;
    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    let (pivots, _) = compute_zigzag_pivots(ctx, n);
    let troughs: Vec<usize> = pivots
        .iter()
        .filter(|(_, _, is_peak)| !*is_peak)
        .map(|(idx, _, _)| *idx)
        .collect();

    let pivot_values: Vec<(usize, f64)> = troughs.iter().map(|&idx| (idx, 0.0)).collect();
    fill_mth_recent_pivot(
        &pivot_values,
        m,
        data_len,
        result.as_slice_mut().unwrap(),
        |i, pidx, _| (i - pidx) as f64,
    );

    Ok(result)
}

pub(crate) fn fn_zigzag(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ZIGZAG", args, 2)?;
    let n = extract_f64_arg(args, 1, "ZIGZAG")?;
    let high = ctx.high.as_slice();
    let low = ctx.low.as_slice();

    match lib_zigzag(high, low, n) {
        Ok(result) => Ok(result.zigzag),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

pub(crate) fn fn_findhigh(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FINDHIGH", args, 4)?;
    let input = &args[0];
    let n = extract_n(args, 1, "FINDHIGH")?;
    let m = args[2][0] as usize;
    let _t = args[3][0] as usize;
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    for i in 0..data_len {
        let start = (i + 1).saturating_sub(n);
        let mut is_high = true;
        let val = input[i];
        if val.is_nan() {
            continue;
        }
        let left = i.saturating_sub(m);
        let right = (i + m).min(data_len - 1);
        let check_start = left.max(start);
        let check_end = right.min(start + n - 1);
        for j in check_start..=check_end {
            if j != i && !input[j].is_nan() && input[j] >= val {
                is_high = false;
                break;
            }
        }
        result[i] = if is_high { 1.0 } else { 0.0 };
    }

    Ok(result)
}

pub(crate) fn fn_findlow(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FINDLOW", args, 4)?;
    let input = &args[0];
    let n = extract_n(args, 1, "FINDLOW")?;
    let m = args[2][0] as usize;
    let _t = args[3][0] as usize;
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    for i in 0..data_len {
        let start = (i + 1).saturating_sub(n);
        let mut is_low = true;
        let val = input[i];
        if val.is_nan() {
            continue;
        }
        let left = i.saturating_sub(m);
        let right = (i + m).min(data_len - 1);
        let check_start = left.max(start);
        let check_end = right.min(start + n - 1);
        for j in check_start..=check_end {
            if j != i && !input[j].is_nan() && input[j] <= val {
                is_low = false;
                break;
            }
        }
        result[i] = if is_low { 1.0 } else { 0.0 };
    }

    Ok(result)
}
