//! TDX extended-market (`EM_`/`FOX_`) pivot and signal functions.

use super::prelude::*;

pub(crate) fn fn_dkcol(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let data_len = ctx.data_len;
    match &ctx.em_data {
        Some(em) => {
            let buy_key = "BUYVOL";
            let sell_key = "SELLVOL";
            let buy_vol = em.dkcol_data.get(buy_key);
            let sell_vol = em.dkcol_data.get(sell_key);
            match (buy_vol, sell_vol) {
                (Some(bv), Some(sv)) => {
                    if bv.len() == data_len && sv.len() == data_len {
                        let mut result = Array1::zeros(data_len);
                        for i in 0..data_len {
                            result[i] = bv[i] - sv[i];
                        }
                        Ok(result)
                    } else {
                        Ok(nan_vec(data_len))
                    }
                }
                _ => Ok(nan_vec(data_len)),
            }
        }
        None => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_em_cross(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("EM_CROSS", args, 2)?;
    let a = &args[0];
    let b = &args[1];
    let len = a.len().min(b.len());
    let mut result = Array1::zeros(len);
    if len > 0 {
        result[0] = 0.0;
    }
    for i in 1..len {
        if !a[i].is_nan() && !b[i].is_nan() && !a[i - 1].is_nan() && !b[i - 1].is_nan() {
            if a[i - 1] <= b[i - 1] && a[i] > b[i] {
                result[i] = 1.0;
            }
        }
    }
    Ok(result)
}

pub(crate) fn fn_em_ref(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("EM_REF", args, 2)?;
    let name = get_string_from_hash(ctx, args[0][0]).ok_or_else(|| {
        FormulaError::InvalidParameter("EM_REF: name must be a valid string".to_string())
    })?;
    let n = extract_n(args, 1, "EM_REF")?;

    let data_len = ctx.data_len;
    match &ctx.em_data {
        Some(em) => match em.external_data.get(&name) {
            Some(data) => {
                if data.len() != data_len {
                    return Ok(nan_vec(data_len));
                }
                let mut result = Array1::zeros(data_len);
                for i in 0..data_len {
                    if i >= n {
                        result[i] = data[i - n];
                    } else {
                        result[i] = f64::NAN;
                    }
                }
                Ok(result)
            }
            None => Err(FormulaError::RuntimeError(format!(
                "EM_REF: external data '{}' not found",
                name
            ))),
        },
        None => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_em_zig(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("EM_ZIG", args, 2)?;
    let _k = extract_f64_arg(args, 0, "EM_ZIG")?;
    let n = extract_f64_arg(args, 1, "EM_ZIG")?;
    let high = ctx.high.as_slice();
    let low = ctx.low.as_slice();

    match lib_zigzag(high, low, n) {
        Ok(result) => Ok(result.zigzag),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

/// Fill `result[i]` with `value_of(i, pidx, pval)` for the m-th most recent
/// pivot whose index is strictly below bar `i`, leaving the slot untouched
/// when fewer than `m` pivots qualify. `pivots` must be sorted ascending by
/// index (the zig-zag scan already produces them that way).
///
/// The old implementation re-walked the whole pivot list backwards for every
/// bar — O(data_len · pivots). Since `i` only grows, the qualifying set only
/// grows at its most-recent end, so a deque capped at the last `m` qualifying
/// pivots turns the scan into one push per pivot and one read per bar.
pub(crate) fn fill_mth_recent_pivot(
    pivots: &[(usize, f64)],
    m: usize,
    data_len: usize,
    result: &mut [f64],
    value_of: fn(usize, usize, f64) -> f64,
) {
    if m == 0 {
        // The old loop counted up from 1 before comparing against `m`, so
        // `m == 0` never matched and every slot stayed NaN.
        return;
    }
    // The single forward walk below is only valid for ascending indices; every
    // caller builds its list from the zig-zag scan, which emits them in bar
    // order. Cheap enough to assert in debug builds so a future caller that
    // sorts differently fails loudly instead of silently shifting answers.
    debug_assert!(
        pivots.windows(2).all(|pair| pair[0].0 <= pair[1].0),
        "fill_mth_recent_pivot requires pivots sorted ascending by index"
    );
    let mut window: std::collections::VecDeque<(usize, f64)> =
        std::collections::VecDeque::with_capacity(m + 1);
    let mut next = 0usize;
    for i in 0..data_len {
        while next < pivots.len() && pivots[next].0 < i {
            window.push_back(pivots[next]);
            if window.len() > m {
                window.pop_front();
            }
            next += 1;
        }
        if window.len() == m {
            let &(pidx, pval) = window.front().expect("len == m >= 1");
            result[i] = value_of(i, pidx, pval);
        }
    }
}

pub(crate) fn fn_em_trough(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("EM_TROUGH", args, 3)?;
    let _k = extract_f64_arg(args, 0, "EM_TROUGH")?;
    let n = extract_f64_arg(args, 1, "EM_TROUGH")?;
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

pub(crate) fn fn_em_peak(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("EM_PEAK", args, 3)?;
    let _k = extract_f64_arg(args, 0, "EM_PEAK")?;
    let n = extract_f64_arg(args, 1, "EM_PEAK")?;
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

pub(crate) fn fn_em_troughbars(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("EM_TROUGHBARS", args, 3)?;
    let _k = extract_f64_arg(args, 0, "EM_TROUGHBARS")?;
    let n = extract_f64_arg(args, 1, "EM_TROUGHBARS")?;
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

pub(crate) fn fn_em_peakbars(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("EM_PEAKBARS", args, 3)?;
    let _k = extract_f64_arg(args, 0, "EM_PEAKBARS")?;
    let n = extract_f64_arg(args, 1, "EM_PEAKBARS")?;
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

pub(crate) fn fn_em_costex(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("EM_COSTEX", args, 2)?;
    let price = &args[0];
    let volume = &args[1];
    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    let mut cum_cost = 0.0f64;
    let mut cum_vol = 0.0f64;

    for i in 0..data_len {
        if !price[i].is_nan() && !volume[i].is_nan() && volume[i] > 0.0 {
            cum_cost += price[i] * volume[i];
            cum_vol += volume[i];
            if cum_vol > 0.0 {
                result[i] = cum_cost / cum_vol;
            }
        }
    }

    Ok(result)
}

pub(crate) fn fn_em_zlccv(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let data_len = ctx.data_len;
    match &ctx.em_data {
        Some(em) => match em.dkcol_data.get("ZLCCV") {
            Some(data) => {
                if data.len() == data_len {
                    Ok(data.clone())
                } else {
                    Ok(nan_vec(data_len))
                }
            }
            None => Ok(nan_vec(data_len)),
        },
        None => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_fox_zig(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FOX_ZIG", args, 2)?;
    let n = extract_f64_arg(args, 1, "FOX_ZIG")?;
    let high = ctx.high.as_slice();
    let low = ctx.low.as_slice();

    match lib_zigzag(high, low, n) {
        Ok(result) => Ok(result.zigzag),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

pub(crate) fn fn_fox_trough(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FOX_TROUGH", args, 3)?;
    let n = extract_f64_arg(args, 1, "FOX_TROUGH")?;
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

pub(crate) fn fn_fox_peak(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FOX_PEAK", args, 3)?;
    let n = extract_f64_arg(args, 1, "FOX_PEAK")?;
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

pub(crate) fn fn_fox_troughbars(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FOX_TROUGHBARS", args, 3)?;
    let n = extract_f64_arg(args, 1, "FOX_TROUGHBARS")?;
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

pub(crate) fn fn_fox_peakbars(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FOX_PEAKBARS", args, 3)?;
    let n = extract_f64_arg(args, 1, "FOX_PEAKBARS")?;
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
