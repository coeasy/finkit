use ndarray::{s, Array1};
use std::collections::HashMap;

use crate::formula::simd::SimdOps;
use crate::formula::types::*;
use crate::indicators::astock as lib_astock;
use crate::indicators::chart::zigzag as lib_zigzag;
use crate::indicators::china::{bias as lib_bias, kdj as lib_kdj, psy as lib_psy};
use crate::indicators::classic_patterns as lib_classic;
use crate::indicators::cycle as lib_cycle;
use crate::indicators::momentum as lib_momentum;
use crate::indicators::momentum_ext::{chop as lib_chop, fisher as lib_fisher, tsi as lib_tsi};
use crate::indicators::overlap::sarext as lib_sarext;
use crate::indicators::statistics::avgdev as lib_avgdev;
use crate::indicators::volume_ext::cmf as lib_cmf;
use crate::math::linear as lib_linear;
use crate::math::moving_avg as lib_ma;
use crate::math::statistics as lib_stat;

type FormulaFn = fn(&FormulaContext, &[Array1<f64>]) -> Result<Array1<f64>, FormulaError>;

fn nan_vec(len: usize) -> Array1<f64> {
    Array1::from_elem(len, f64::NAN)
}

fn ensure_args_len(name: &str, args: &[Array1<f64>], expected: usize) -> Result<(), FormulaError> {
    if args.len() < expected {
        return Err(FormulaError::InvalidParameter(format!(
            "{} requires at least {} arguments, got {}",
            name,
            expected,
            args.len()
        )));
    }
    Ok(())
}

fn get_string_from_hash(ctx: &FormulaContext, idx_val: f64) -> Option<String> {
    if idx_val.is_nan() || idx_val < 0.0 {
        return None;
    }
    let idx = idx_val as usize;
    ctx.string_table.get(idx).cloned()
}

fn extract_n(args: &[Array1<f64>], idx: usize, name: &str) -> Result<usize, FormulaError> {
    if idx >= args.len() {
        return Err(FormulaError::RuntimeError(format!(
            "{}: missing argument at index {}",
            name, idx
        )));
    }
    let n = args[idx][0] as usize;
    if n == 0 {
        return Err(FormulaError::InvalidParameter(format!(
            "{}: period must be > 0",
            name
        )));
    }
    Ok(n)
}

fn extract_f64_arg(args: &[Array1<f64>], idx: usize, name: &str) -> Result<f64, FormulaError> {
    if idx >= args.len() {
        return Err(FormulaError::RuntimeError(format!(
            "{}: missing argument at index {}",
            name, idx
        )));
    }
    Ok(args[idx][0])
}

fn fn_ma(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MA", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "MA")?;

    let data_len = ctx.data_len;
    if n == 1 {
        return Ok(input.clone());
    }

    let values = input.as_slice().unwrap();
    match lib_ma::sma(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_ema(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("EMA", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "EMA")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_ma::ema(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_sma(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    if args.len() < 2 || args.len() > 3 {
        return Err(FormulaError::InvalidParameter(format!(
            "SMA requires 2 or 3 arguments, got {}",
            args.len()
        )));
    }
    let input = &args[0];
    let n = extract_n(args, 1, "SMA")?;
    // Pine `ta.sma(src, length)` passes only 2 args (m defaults to 1 = plain SMA).
    let m = if args.len() == 3 {
        extract_f64_arg(args, 2, "SMA")?
    } else {
        1.0
    };

    let data_len = ctx.data_len;
    let mut output = nan_vec(data_len);

    if n == 0 {
        return Err(FormulaError::InvalidParameter(
            "SMA: N must be > 0".to_string(),
        ));
    }

    let mut prev_sma: Option<f64> = None;

    for i in 0..data_len {
        let cur = input[i];
        if cur.is_nan() {
            continue;
        }

        if let Some(sma_val) = prev_sma {
            output[i] = (m * cur + (n as f64 - m) * sma_val) / n as f64;
        } else {
            output[i] = cur;
        }
        prev_sma = Some(output[i]);
    }

    Ok(output)
}

/// `math.avg(a, b, ...)` — elementwise arithmetic mean of all arguments.
fn fn_math_avg(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    if args.is_empty() {
        return Err(FormulaError::InvalidParameter(
            "MATH_AVG requires at least 1 argument".to_string(),
        ));
    }
    let len = args[0].len();
    let mut out = Array1::zeros(len);
    for i in 0..len {
        let mut sum = 0.0;
        for a in args {
            sum += a[i];
        }
        out[i] = sum / args.len() as f64;
    }
    Ok(out)
}

/// `ISNA(x)` — returns 1.0 where `x` is NaN, else 0.0 (truthy for `IF`/ternary).
/// Backs Pine `nz(x, y)` → `IF(ISNA(x), y, x)` and `na(x)` → `ISNA(x)`.
fn fn_isna(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    if args.is_empty() {
        return Err(FormulaError::InvalidParameter(
            "ISNA requires 1 argument".to_string(),
        ));
    }
    let x = &args[0];
    let len = x.len();
    let mut out = Array1::zeros(len);
    for i in 0..len {
        out[i] = if x[i].is_nan() { 1.0 } else { 0.0 };
    }
    Ok(out)
}

/// `VWMA(close, volume, n)` — volume-weighted moving average.
fn fn_vwma_indicator(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("VWMA", args, 3)?;
    let close = &args[0];
    let volume = &args[1];
    let n = extract_n(args, 2, "VWMA")?;
    let data_len = ctx.data_len;
    let mut out = nan_vec(data_len);
    for i in (n - 1)..data_len {
        let start = (i + 1).saturating_sub(n);
        let mut pv = 0.0f64;
        let mut v = 0.0f64;
        for j in start..=i {
            pv += close[j] * volume[j];
            v += volume[j];
        }
        if v.abs() > 1e-15 {
            out[i] = pv / v;
        }
    }
    Ok(out)
}

fn fn_wma(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("WMA", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "WMA")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_ma::wma(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_dma(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DMA", args, 2)?;
    let input = &args[0];
    let alpha_arr = &args[1];

    let data_len = ctx.data_len;
    let mut output = nan_vec(data_len);

    let mut prev_dma: Option<f64> = None;

    for i in 0..data_len {
        let cur = input[i];
        let alpha = alpha_arr[i];
        if cur.is_nan() || alpha.is_nan() {
            continue;
        }
        if let Some(prev) = prev_dma {
            output[i] = alpha * cur + (1.0 - alpha) * prev;
        } else {
            output[i] = cur;
        }
        prev_dma = Some(output[i]);
    }

    Ok(output)
}

fn fn_hhv(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HHV", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "HHV")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_stat::rolling_max(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_llv(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LLV", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "LLV")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_stat::rolling_min(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_hhvbars(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HHVBARS", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "HHVBARS")?;

    let data_len = ctx.data_len;
    let mut output = nan_vec(data_len);

    // Scan the source slice in place. The previous implementation allocated
    // and copied a temporary window for every bar, which made this O(bars)
    // allocations on top of the O(bars * period) scan.
    for i in 0..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let mut max_value = f64::NEG_INFINITY;
        let mut max_index = None;

        for j in window_start..=i {
            let value = input[j];
            if !value.is_nan() && value > max_value {
                max_value = value;
                max_index = Some(j);
            }
        }

        if let Some(index) = max_index {
            output[i] = (i - index) as f64;
        }
    }

    Ok(output)
}

fn fn_llvbars(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LLVBARS", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "LLVBARS")?;

    let data_len = ctx.data_len;
    let mut output = nan_vec(data_len);

    // Keep the first occurrence on ties, matching the old temporary-window
    // implementation while avoiding one Vec allocation per bar.
    for i in 0..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let mut min_value = f64::INFINITY;
        let mut min_index = None;

        for j in window_start..=i {
            let value = input[j];
            if !value.is_nan() && value < min_value {
                min_value = value;
                min_index = Some(j);
            }
        }

        if let Some(index) = min_index {
            output[i] = (i - index) as f64;
        }
    }

    Ok(output)
}

fn fn_ref(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("REF", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "REF")?;

    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);
    for i in 0..data_len {
        if i >= n {
            result[i] = input[i - n];
        } else {
            result[i] = f64::NAN;
        }
    }
    Ok(result)
}

fn fn_cross(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CROSS", args, 2)?;
    let a = &args[0];
    let b = &args[1];
    let len = a.len().min(b.len());
    let mut result = Array1::zeros(len);
    if len > 0 {
        result[0] = 0.0;
    }
    for i in 1..len {
        if a[i - 1] <= b[i - 1] && a[i] > b[i] {
            result[i] = 1.0;
        }
    }
    Ok(result)
}

fn fn_crossbelow(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CROSSBELOW", args, 2)?;
    let a = &args[0];
    let b = &args[1];
    let len = a.len().min(b.len());
    let mut result = Array1::zeros(len);
    if len > 0 {
        result[0] = 0.0;
    }
    for i in 1..len {
        if a[i - 1] >= b[i - 1] && a[i] < b[i] {
            result[i] = 1.0;
        }
    }
    Ok(result)
}

fn fn_longcross(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LONGCROSS", args, 3)?;
    let a = &args[0];
    let b = &args[1];
    let n = args[2][0] as usize;
    let len = a.len().min(b.len());
    let mut result = Array1::zeros(len);

    for i in 1..len {
        if a[i] > b[i] {
            let mut below_for_n = true;
            if i < n {
                below_for_n = false;
            } else {
                for j in (i - n)..i {
                    if a[j] > b[j] {
                        below_for_n = false;
                        break;
                    }
                }
            }
            if below_for_n && a[i - 1] <= b[i - 1] {
                result[i] = 1.0;
            }
        }
    }

    Ok(result)
}

fn fn_if(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("IF", args, 3)?;
    let cond = &args[0];
    let then_val = &args[1];
    let else_val = &args[2];
    let len = cond.len().min(then_val.len()).min(else_val.len());

    if len >= 16 {
        Ok(SimdOps::simd_select_arrays(cond, then_val, else_val))
    } else {
        Ok(cond
            .iter()
            .zip(then_val.iter())
            .zip(else_val.iter())
            .map(|((&c, &t), &e)| if c > 0.0 { t } else { e })
            .collect())
    }
}

fn fn_count(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("COUNT", args, 2)?;
    let cond = &args[0];
    let n = extract_n(args, 1, "COUNT")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let mut count = 0usize;
        for j in window_start..=i {
            if cond[j] > 0.0 {
                count += 1;
            }
        }
        result[i] = count as f64;
    }

    Ok(result)
}

fn fn_dkcol(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

fn fn_em_cross(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

fn fn_em_ref(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

fn fn_em_zig(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("EM_ZIG", args, 2)?;
    let _k = extract_f64_arg(args, 0, "EM_ZIG")?;
    let n = extract_f64_arg(args, 1, "EM_ZIG")?;
    let high = ctx.high.as_slice().unwrap();
    let low = ctx.low.as_slice().unwrap();

    match lib_zigzag(high, low, n) {
        Ok(result) => Ok(result.zigzag),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

fn fn_em_trough(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

    for i in 0..data_len {
        let mut count = 0usize;
        for &(pidx, pval) in troughs.iter().rev() {
            if pidx < i {
                count += 1;
                if count == m {
                    result[i] = pval;
                    break;
                }
            }
        }
    }

    Ok(result)
}

fn fn_em_peak(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

    for i in 0..data_len {
        let mut count = 0usize;
        for &(pidx, pval) in peaks.iter().rev() {
            if pidx < i {
                count += 1;
                if count == m {
                    result[i] = pval;
                    break;
                }
            }
        }
    }

    Ok(result)
}

fn fn_em_troughbars(
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

    for i in 0..data_len {
        let mut count = 0usize;
        for &pidx in troughs.iter().rev() {
            if pidx < i {
                count += 1;
                if count == m {
                    result[i] = (i - pidx) as f64;
                    break;
                }
            }
        }
    }

    Ok(result)
}

fn fn_em_peakbars(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

    for i in 0..data_len {
        let mut count = 0usize;
        for &pidx in peaks.iter().rev() {
            if pidx < i {
                count += 1;
                if count == m {
                    result[i] = (i - pidx) as f64;
                    break;
                }
            }
        }
    }

    Ok(result)
}

fn fn_em_costex(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

fn fn_em_zlccv(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

fn fn_sum(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SUM", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "SUM")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    let mut running_sum = 0.0;
    for i in 0..data_len {
        running_sum += input[i];
        if i >= n {
            running_sum -= input[i - n];
            result[i] = running_sum;
        } else if i == n - 1 {
            result[i] = running_sum;
        }
    }

    Ok(result)
}

fn fn_every(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("EVERY", args, 2)?;
    let cond = &args[0];
    let n = extract_n(args, 1, "EVERY")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let mut all_true = true;
        for j in window_start..=i {
            if cond[j] <= 0.0 {
                all_true = false;
                break;
            }
        }
        result[i] = if all_true { 1.0 } else { 0.0 };
    }

    Ok(result)
}

fn fn_exist(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("EXIST", args, 2)?;
    let cond = &args[0];
    let n = extract_n(args, 1, "EXIST")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let mut any_true = false;
        for j in window_start..=i {
            if cond[j] > 0.0 {
                any_true = true;
                break;
            }
        }
        result[i] = if any_true { 1.0 } else { 0.0 };
    }

    Ok(result)
}

fn fn_filter(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FILTER", args, 2)?;
    let cond = &args[0];
    let n = extract_n(args, 1, "FILTER")?;

    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);
    let mut last_signal: Option<usize> = None;

    for i in 0..data_len {
        if cond[i] > 0.0 {
            if let Some(last) = last_signal {
                if i - last >= n {
                    result[i] = 1.0;
                    last_signal = Some(i);
                }
            } else {
                result[i] = 1.0;
                last_signal = Some(i);
            }
        }
    }

    Ok(result)
}

fn fn_barslast(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BARSLAST", args, 1)?;
    let cond = &args[0];

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);
    let mut last_true: Option<usize> = None;

    for i in 0..data_len {
        if cond[i] > 0.0 {
            result[i] = 0.0;
            last_true = Some(i);
        } else if let Some(last) = last_true {
            result[i] = (i - last) as f64;
        }
    }

    Ok(result)
}

fn fn_backset(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BACKSET", args, 2)?;
    let cond = &args[0];
    let n = extract_n(args, 1, "BACKSET")?;

    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    for i in 0..data_len {
        if cond[i] > 0.0 {
            for j in (i + 1).saturating_sub(n)..=i {
                result[j] = 1.0;
            }
        }
    }

    Ok(result)
}

fn fn_between(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BETWEEN", args, 3)?;
    let x = &args[0];
    let a = &args[1];
    let b = &args[2];
    let len = x.len().min(a.len()).min(b.len());
    let mut result = Array1::zeros(len);
    for i in 0..len {
        let lo = a[i].min(b[i]);
        let hi = a[i].max(b[i]);
        result[i] = if x[i] >= lo && x[i] <= hi { 1.0 } else { 0.0 };
    }
    Ok(result)
}

fn fn_abs(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ABS", args, 1)?;
    Ok(args[0].mapv(|v| v.abs()))
}

fn fn_max(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MAX", args, 2)?;
    let a = &args[0];
    let b = &args[1];
    let len = a.len().min(b.len());
    let mut result = Array1::zeros(len);
    for i in 0..len {
        result[i] = a[i].max(b[i]);
    }
    Ok(result)
}

fn fn_min(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MIN", args, 2)?;
    let a = &args[0];
    let b = &args[1];
    let len = a.len().min(b.len());
    let mut result = Array1::zeros(len);
    for i in 0..len {
        result[i] = a[i].min(b[i]);
    }
    Ok(result)
}

fn fn_add(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ADD", args, 2)?;
    let a = &args[0];
    let b = &args[1];
    let len = a.len().min(b.len());
    let mut result = Array1::zeros(len);
    for i in 0..len {
        result[i] = a[i] + b[i];
    }
    Ok(result)
}

fn fn_sub(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SUB", args, 2)?;
    let a = &args[0];
    let b = &args[1];
    let len = a.len().min(b.len());
    let mut result = Array1::zeros(len);
    for i in 0..len {
        result[i] = a[i] - b[i];
    }
    Ok(result)
}

fn fn_mult(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MULT", args, 2)?;
    let a = &args[0];
    let b = &args[1];
    let len = a.len().min(b.len());
    let mut result = Array1::zeros(len);
    for i in 0..len {
        result[i] = a[i] * b[i];
    }
    Ok(result)
}

fn fn_div(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DIV", args, 2)?;
    let a = &args[0];
    let b = &args[1];
    let len = a.len().min(b.len());
    let mut result = Array1::zeros(len);
    for i in 0..len {
        result[i] = if b[i] == 0.0 { f64::NAN } else { a[i] / b[i] };
    }
    Ok(result)
}

fn fn_minus(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MINUS", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "MINUS")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in 0..data_len {
        if i >= n {
            result[i] = input[i] - input[i - n];
        }
    }

    Ok(result)
}

fn fn_maxindex(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MAXINDEX", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "MAXINDEX")?;

    let data_len = ctx.data_len;
    let mut output = nan_vec(data_len);

    for i in 0..data_len {
        if i + 1 < n {
            continue;
        }
        let window_start = i + 1 - n;
        let mut max_val = f64::NEG_INFINITY;
        let mut max_idx: f64 = 0.0;

        for j in window_start..=i {
            if !input[j].is_nan() && input[j] > max_val {
                max_val = input[j];
                max_idx = (j - window_start) as f64;
            }
        }

        if max_val > f64::NEG_INFINITY {
            output[i] = max_idx;
        }
    }

    Ok(output)
}

fn fn_minindex(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MININDEX", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "MININDEX")?;

    let data_len = ctx.data_len;
    let mut output = nan_vec(data_len);

    for i in 0..data_len {
        if i + 1 < n {
            continue;
        }
        let window_start = i + 1 - n;
        let mut min_val = f64::INFINITY;
        let mut min_idx: f64 = 0.0;

        for j in window_start..=i {
            if !input[j].is_nan() && input[j] < min_val {
                min_val = input[j];
                min_idx = (j - window_start) as f64;
            }
        }

        if min_val < f64::INFINITY {
            output[i] = min_idx;
        }
    }

    Ok(output)
}

fn fn_sqrt(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SQRT", args, 1)?;
    Ok(args[0].mapv(|v| if v < 0.0 { f64::NAN } else { v.sqrt() }))
}

fn fn_pow(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("POW", args, 2)?;
    let base = &args[0];
    let exp = &args[1];
    let len = base.len().min(exp.len());
    let mut result = Array1::zeros(len);
    for i in 0..len {
        result[i] = base[i].powf(exp[i]);
    }
    Ok(result)
}

fn fn_exp(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("EXP", args, 1)?;
    Ok(args[0].mapv(|v| v.exp()))
}

fn fn_log(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LOG", args, 1)?;
    Ok(args[0].mapv(|v| if v <= 0.0 { f64::NAN } else { v.ln() }))
}

fn fn_log10(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LOG10", args, 1)?;
    Ok(args[0].mapv(|v| if v <= 0.0 { f64::NAN } else { v.log10() }))
}

fn fn_sign(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SIGN", args, 1)?;
    Ok(args[0].mapv(|v| v.signum()))
}

fn fn_floor(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FLOOR", args, 1)?;
    Ok(args[0].mapv(|v| v.floor()))
}

fn fn_ceil(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CEIL", args, 1)?;
    Ok(args[0].mapv(|v| v.ceil()))
}

fn fn_round(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ROUND", args, 1)?;
    Ok(args[0].mapv(|v| v.round()))
}

fn fn_sin(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SIN", args, 1)?;
    Ok(args[0].mapv(|v| v.sin()))
}

fn fn_cos(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("COS", args, 1)?;
    Ok(args[0].mapv(|v| v.cos()))
}

fn fn_tan(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TAN", args, 1)?;
    Ok(args[0].mapv(|v| v.tan()))
}

fn fn_sinh(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SINH", args, 1)?;
    Ok(args[0].mapv(|v| v.sinh()))
}

fn fn_cosh(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("COSH", args, 1)?;
    Ok(args[0].mapv(|v| v.cosh()))
}

fn fn_tanh(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TANH", args, 1)?;
    Ok(args[0].mapv(|v| v.tanh()))
}

fn fn_asin(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ASIN", args, 1)?;
    Ok(args[0].mapv(|v| v.asin()))
}

fn fn_acos(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ACOS", args, 1)?;
    Ok(args[0].mapv(|v| v.acos()))
}

fn fn_atan(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ATAN", args, 1)?;
    Ok(args[0].mapv(|v| v.atan()))
}

fn fn_dema(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DEMA", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "DEMA")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_ma::dema(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_tema(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TEMA", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "TEMA")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_ma::tema(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_kama(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("KAMA", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "KAMA")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_ma::kama(values, n, 2, 30) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_t3(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("T3", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "T3")?;
    let v_factor = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0]
    } else {
        0.7
    };

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    let ema1 = match lib_ma::ema(values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let ema1_vec: Vec<f64> = ema1
        .iter()
        .map(|&x| if x.is_nan() { 0.0 } else { x })
        .collect();
    let ema2 = match lib_ma::ema(&ema1_vec, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let ema2_vec: Vec<f64> = ema2
        .iter()
        .map(|&x| if x.is_nan() { 0.0 } else { x })
        .collect();
    let ema3 = match lib_ma::ema(&ema2_vec, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let ema3_vec: Vec<f64> = ema3
        .iter()
        .map(|&x| if x.is_nan() { 0.0 } else { x })
        .collect();
    let ema4 = match lib_ma::ema(&ema3_vec, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let ema5 = {
        let ema4_vec: Vec<f64> = ema4
            .iter()
            .map(|&x| if x.is_nan() { 0.0 } else { x })
            .collect();
        match lib_ma::ema(&ema4_vec, n) {
            Ok(r) => r,
            Err(_) => return Ok(nan_vec(data_len)),
        }
    };
    let ema6 = {
        let ema5_vec: Vec<f64> = ema5
            .iter()
            .map(|&x| if x.is_nan() { 0.0 } else { x })
            .collect();
        match lib_ma::ema(&ema5_vec, n) {
            Ok(r) => r,
            Err(_) => return Ok(nan_vec(data_len)),
        }
    };

    let mut output = nan_vec(data_len);
    for i in 0..data_len {
        if !ema1[i].is_nan()
            && !ema2[i].is_nan()
            && !ema3[i].is_nan()
            && !ema4[i].is_nan()
            && !ema5[i].is_nan()
            && !ema6[i].is_nan()
        {
            let gd = (ema1[i] * (1.0 + v_factor)
                - ema2[i] * (2.0 * v_factor + v_factor * v_factor)
                + ema3[i] * (1.0 + 2.0 * v_factor + v_factor * v_factor))
                .max(0.0);
            output[i] =
                -ema4[i] * gd * gd * gd + ema5[i] * 3.0 * gd * gd - ema6[i] * 3.0 * gd + ema3[i];
        }
    }

    Ok(output)
}

fn fn_trima(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TRIMA", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "TRIMA")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_ma::trima(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_mavp(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    // MAVP(close, periods, min_period, max_period) — periods is a per-bar
    // length-N series of integer periods. We accept the period values as a
    // single series (the second arg) and clamp each entry to [min_period, max_period].
    ensure_args_len("MAVP", args, 4)?;
    let input = &args[0];
    let periods = args[1].as_slice().unwrap();
    let min_period = extract_n(args, 2, "MAVP")?;
    let max_period = extract_n(args, 3, "MAVP")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_ma::mavp(values, periods, min_period, max_period) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_sarext(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    // SAREXT(high, low, start_value, offset_on_reverse, af_init_long, af_long,
    //        af_max_long, af_init_short, af_short, af_max_short)
    ensure_args_len("SAREXT", args, 10)?;
    let high = args[0].as_slice().unwrap();
    let low = args[1].as_slice().unwrap();
    let start_value = extract_f64_arg(args, 2, "SAREXT")?;
    let offset_on_reverse = extract_f64_arg(args, 3, "SAREXT")?;
    let af_init_long = extract_f64_arg(args, 4, "SAREXT")?;
    let af_long = extract_f64_arg(args, 5, "SAREXT")?;
    let af_max_long = extract_f64_arg(args, 6, "SAREXT")?;
    let af_init_short = extract_f64_arg(args, 7, "SAREXT")?;
    let af_short = extract_f64_arg(args, 8, "SAREXT")?;
    let af_max_short = extract_f64_arg(args, 9, "SAREXT")?;

    let data_len = ctx.data_len;
    match lib_sarext(
        high,
        low,
        start_value,
        offset_on_reverse,
        af_init_long,
        af_long,
        af_max_long,
        af_init_short,
        af_short,
        af_max_short,
    ) {
        Ok(r) => Ok(r.sar),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_macdext(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    // MACDEXT(close, fast_period, fast_ma, slow_period, slow_ma, signal_period, signal_ma)
    // fast_ma / slow_ma / signal_ma are integer codes: 0 = Sma, 1 = Ema.
    ensure_args_len("MACDEXT", args, 7)?;
    let input = &args[0];
    let fast_period = extract_n(args, 1, "MACDEXT")?;
    let fast_ma = extract_n(args, 2, "MACDEXT")?;
    let slow_period = extract_n(args, 3, "MACDEXT")?;
    let slow_ma = extract_n(args, 4, "MACDEXT")?;
    let signal_period = extract_n(args, 5, "MACDEXT")?;
    let signal_ma = extract_n(args, 6, "MACDEXT")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    if fast_period >= slow_period {
        return Ok(nan_vec(data_len));
    }

    let fast_kind = match fast_ma {
        0 => crate::indicators::overlap::MaType::Sma,
        _ => crate::indicators::overlap::MaType::Ema,
    };
    let slow_kind = match slow_ma {
        0 => crate::indicators::overlap::MaType::Sma,
        _ => crate::indicators::overlap::MaType::Ema,
    };
    let signal_kind = match signal_ma {
        0 => crate::indicators::overlap::MaType::Sma,
        _ => crate::indicators::overlap::MaType::Ema,
    };

    match lib_momentum::macdext(
        values,
        fast_period,
        fast_kind,
        slow_period,
        slow_kind,
        signal_period,
        signal_kind,
    ) {
        Ok(r) => Ok(r.macd),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

#[allow(dead_code)]
fn fn_macdfix(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    // MACDFIX(close, signal_period)
    ensure_args_len("MACDFIX", args, 2)?;
    let input = &args[0];
    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    // Batch macdfix uses fixed 12/26 periods. We delegate to standard macd so
    // that a custom signal_period is honoured.
    let signal_period = extract_n(args, 1, "MACDFIX")?;
    match lib_momentum::macd(values, 12, 26, signal_period) {
        Ok(r) => Ok(r.macd),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

#[allow(dead_code)]
fn fn_rocp(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ROCP", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "ROCP")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_momentum::rocp(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

#[allow(dead_code)]
fn fn_rocr(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ROCR", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "ROCR")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_momentum::rocr(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

#[allow(dead_code)]
fn fn_rocr100(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ROCR100", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "ROCR100")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_momentum::rocr100(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_avgdev(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

fn fn_std(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("STD", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "STD")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_stat::rolling_std_dev(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_var(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("VAR", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "VAR")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_stat::rolling_variance(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_rsi(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("RSI", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "RSI")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_momentum::rsi(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_macd(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MACD", args, 2)?;
    let input = &args[0];
    let fast_n = extract_n(args, 1, "MACD")?;
    let slow_n = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0] as usize
    } else {
        26
    };
    let signal_n = if args.len() > 3 && !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0] as usize
    } else {
        9
    };

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    if fast_n >= slow_n {
        return Ok(nan_vec(data_len));
    }

    match lib_momentum::macd(values, fast_n, slow_n, signal_n) {
        Ok(result) => Ok(result.macd),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_diff(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DIFF", args, 3)?;
    let input = &args[0];
    let fast_n = extract_n(args, 1, "DIFF")?;
    let slow_n = extract_n(args, 2, "DIFF")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    if fast_n >= slow_n {
        return Ok(nan_vec(data_len));
    }

    match lib_momentum::macd(values, fast_n, slow_n, 9) {
        Ok(result) => Ok(result.macd),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_dea(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DEA", args, 2)?;
    let input = &args[0];
    let fast_n = extract_n(args, 1, "DEA")?;
    let slow_n = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0] as usize
    } else {
        26
    };
    let signal_n = if args.len() > 3 && !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0] as usize
    } else {
        9
    };

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    if fast_n >= slow_n {
        return Ok(nan_vec(data_len));
    }

    match lib_momentum::macd(values, fast_n, slow_n, signal_n) {
        Ok(result) => Ok(result.signal),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_boll(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BOLL", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "BOLL")?;
    let nbdev = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0]
    } else {
        2.0
    };

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    let ma_vals = match lib_ma::sma(values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let std_vals = match lib_stat::rolling_std_dev(values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut result = nan_vec(data_len);
    for i in 0..data_len {
        if !ma_vals[i].is_nan() && !std_vals[i].is_nan() {
            result[i] = ma_vals[i] + nbdev * std_vals[i];
        }
    }

    Ok(result)
}

fn fn_bollup(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    fn_boll(ctx, args)
}

fn fn_bolldn(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BOLLDN", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "BOLLDN")?;
    let nbdev = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0]
    } else {
        2.0
    };

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    let ma_vals = match lib_ma::sma(values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let std_vals = match lib_stat::rolling_std_dev(values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut result = nan_vec(data_len);
    for i in 0..data_len {
        if !ma_vals[i].is_nan() && !std_vals[i].is_nan() {
            result[i] = ma_vals[i] - nbdev * std_vals[i];
        }
    }

    Ok(result)
}

fn fn_bollmid(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BOLLMID", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "BOLLMID")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_ma::sma(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_bollwidth(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BOLLWIDTH", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "BOLLWIDTH")?;
    let nbdev = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0]
    } else {
        2.0
    };

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    let ma_vals = match lib_ma::sma(values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let std_vals = match lib_stat::rolling_std_dev(values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut result = nan_vec(data_len);
    for i in 0..data_len {
        if !ma_vals[i].is_nan() && !std_vals[i].is_nan() && ma_vals[i].abs() > 1e-15 {
            result[i] = 2.0 * nbdev * std_vals[i] / ma_vals[i] * 100.0;
        }
    }

    Ok(result)
}

fn fn_atr(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ATR", args, 4)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let n = extract_n(args, 3, "ATR")?;

    let data_len = ctx.data_len;
    let mut tr = Array1::zeros(data_len);
    tr[0] = high[0] - low[0];
    for i in 1..data_len {
        let hl = high[i] - low[i];
        let hc = (high[i] - close[i - 1]).abs();
        let lc = (low[i] - close[i - 1]).abs();
        tr[i] = hl.max(hc).max(lc);
    }

    match lib_ma::sma(tr.as_slice().unwrap(), n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_natr(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("NATR", args, 4)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let n = extract_n(args, 3, "NATR")?;

    let data_len = ctx.data_len;
    let mut tr = Array1::zeros(data_len);
    tr[0] = high[0] - low[0];
    for i in 1..data_len {
        let hl = high[i] - low[i];
        let hc = (high[i] - close[i - 1]).abs();
        let lc = (low[i] - close[i - 1]).abs();
        tr[i] = hl.max(hc).max(lc);
    }

    let atr_vals = match lib_ma::sma(tr.as_slice().unwrap(), n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut result = nan_vec(data_len);
    for i in 0..data_len {
        if !atr_vals[i].is_nan() && close[i].abs() > 1e-15 {
            result[i] = atr_vals[i] / close[i] * 100.0;
        }
    }

    Ok(result)
}

fn fn_trange(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TRANGE", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];

    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);
    result[0] = high[0] - low[0];
    for i in 1..data_len {
        let hl = high[i] - low[i];
        let hc = (high[i] - close[i - 1]).abs();
        let lc = (low[i] - close[i - 1]).abs();
        result[i] = hl.max(hc).max(lc);
    }

    Ok(result)
}

fn fn_avgprice(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("AVGPRICE", args, 4)?;
    let open = &args[0];
    let high = &args[1];
    let low = &args[2];
    let close = &args[3];
    let len = open.len().min(high.len()).min(low.len()).min(close.len());
    let mut result = Array1::zeros(len);
    for i in 0..len {
        result[i] = (open[i] + high[i] + low[i] + close[i]) / 4.0;
    }
    Ok(result)
}

fn fn_medprice(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MEDPRICE", args, 2)?;
    let high = &args[0];
    let low = &args[1];
    let len = high.len().min(low.len());
    let mut result = Array1::zeros(len);
    for i in 0..len {
        result[i] = (high[i] + low[i]) / 2.0;
    }
    Ok(result)
}

fn fn_typprice(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TYPPRICE", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let len = high.len().min(low.len()).min(close.len());
    let mut result = Array1::zeros(len);
    for i in 0..len {
        result[i] = (high[i] + low[i] + close[i]) / 3.0;
    }
    Ok(result)
}

fn fn_wclprice(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("WCLPRICE", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let len = high.len().min(low.len()).min(close.len());
    let mut result = Array1::zeros(len);
    for i in 0..len {
        result[i] = (high[i] + low[i] + close[i] * 2.0) / 4.0;
    }
    Ok(result)
}

fn fn_obv(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("OBV", args, 2)?;
    let close = &args[0];
    let volume = &args[1];
    let len = close.len().min(volume.len());
    let mut result = Array1::zeros(len);
    if len == 0 {
        return Ok(result);
    }
    result[0] = volume[0];
    for i in 1..len {
        if close[i] > close[i - 1] {
            result[i] = result[i - 1] + volume[i];
        } else if close[i] < close[i - 1] {
            result[i] = result[i - 1] - volume[i];
        } else {
            result[i] = result[i - 1];
        }
    }
    Ok(result)
}

fn fn_ad(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("AD", args, 4)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let volume = &args[3];
    let len = high.len().min(low.len()).min(close.len()).min(volume.len());
    let mut result = Array1::zeros(len);
    if len == 0 {
        return Ok(result);
    }
    for i in 0..len {
        let hl_diff = high[i] - low[i];
        if hl_diff.abs() > 1e-15 {
            let clv = ((close[i] - low[i]) - (high[i] - close[i])) / hl_diff;
            result[i] = if i > 0 { result[i - 1] } else { 0.0 } + clv * volume[i];
        } else {
            result[i] = if i > 0 { result[i - 1] } else { 0.0 };
        }
    }
    Ok(result)
}

fn fn_adosc(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ADOSC", args, 4)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let volume = &args[3];
    let fast = if args.len() > 4 && !args[4].is_empty() && !args[4][0].is_nan() {
        args[4][0] as usize
    } else {
        3
    };
    let slow = if args.len() > 5 && !args[5].is_empty() && !args[5][0].is_nan() {
        args[5][0] as usize
    } else {
        10
    };

    let data_len = ctx.data_len;
    let mut ad_vals = Array1::zeros(data_len);
    for i in 0..data_len {
        let hl_diff = high[i] - low[i];
        if hl_diff.abs() > 1e-15 {
            let clv = ((close[i] - low[i]) - (high[i] - close[i])) / hl_diff;
            ad_vals[i] = if i > 0 { ad_vals[i - 1] } else { 0.0 } + clv * volume[i];
        } else {
            ad_vals[i] = if i > 0 { ad_vals[i - 1] } else { 0.0 };
        }
    }

    let fast_ema = match lib_ma::ema(ad_vals.as_slice().unwrap(), fast) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let slow_ema = match lib_ma::ema(ad_vals.as_slice().unwrap(), slow) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut result = nan_vec(data_len);
    for i in 0..data_len {
        if !fast_ema[i].is_nan() && !slow_ema[i].is_nan() {
            result[i] = fast_ema[i] - slow_ema[i];
        }
    }

    Ok(result)
}

fn fn_mfi(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MFI", args, 5)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let volume = &args[3];
    let n = extract_n(args, 4, "MFI")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let mut pos_flow = 0.0f64;
        let mut neg_flow = 0.0f64;
        for j in window_start..=i {
            if j == 0 {
                continue;
            }
            let tp = (high[j] + low[j] + close[j]) / 3.0;
            let prev_tp = (high[j - 1] + low[j - 1] + close[j - 1]) / 3.0;
            let mf = tp * volume[j];
            if tp > prev_tp {
                pos_flow += mf;
            } else {
                neg_flow += mf;
            }
        }
        if neg_flow.abs() < 1e-15 {
            result[i] = 100.0;
        } else {
            result[i] = 100.0 - (100.0 / (1.0 + pos_flow / neg_flow));
        }
    }

    Ok(result)
}

fn fn_cci(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CCI", args, 4)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let n = extract_n(args, 3, "CCI")?;

    let data_len = ctx.data_len;
    let mut tp = Array1::zeros(data_len);
    for i in 0..data_len {
        tp[i] = (high[i] + low[i] + close[i]) / 3.0;
    }

    let mut result = nan_vec(data_len);
    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let sum: f64 = (window_start..=i).map(|j| tp[j]).sum();
        let mean = sum / n as f64;
        let mean_dev: f64 = (window_start..=i)
            .map(|j| (tp[j] - mean).abs())
            .sum::<f64>()
            / n as f64;
        if mean_dev > 1e-15 {
            result[i] = (tp[i] - mean) / (0.015 * mean_dev);
        }
    }

    Ok(result)
}

fn fn_willr(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("WILLR", args, 4)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let n = extract_n(args, 3, "WILLR")?;

    let data_len = high.len().min(low.len()).min(close.len());
    let mut result = nan_vec(data_len);

    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let hh = (window_start..=i)
            .map(|j| high[j])
            .fold(f64::NEG_INFINITY, f64::max);
        let ll = (window_start..=i)
            .map(|j| low[j])
            .fold(f64::INFINITY, f64::min);
        let range = hh - ll;
        if range.abs() > 1e-15 {
            result[i] = (hh - close[i]) / range * (-100.0);
        }
    }

    Ok(result)
}

fn fn_mom(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MOM", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "MOM")?;

    let data_len = input.len();
    let mut result = nan_vec(data_len);
    for i in n..data_len {
        result[i] = input[i] - input[i - n];
    }
    Ok(result)
}

fn fn_roc(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ROC", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "ROC")?;

    let data_len = input.len();
    let mut result = nan_vec(data_len);
    for i in n..data_len {
        if input[i - n].abs() > 1e-15 {
            result[i] = (input[i] - input[i - n]) / input[i - n] * 100.0;
        }
    }
    Ok(result)
}

fn fn_cmo(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CMO", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "CMO")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let mut sum_up = 0.0f64;
        let mut sum_down = 0.0f64;
        for j in (window_start + 1)..=i {
            let diff = input[j] - input[j - 1];
            if diff > 0.0 {
                sum_up += diff;
            } else {
                sum_down += diff.abs();
            }
        }
        let total = sum_up + sum_down;
        if total > 1e-15 {
            result[i] = (sum_up - sum_down) / total * 100.0;
        }
    }

    Ok(result)
}

fn fn_ppo(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("PPO", args, 3)?;
    let input = &args[0];
    let fast = extract_n(args, 1, "PPO")?;
    let slow = extract_n(args, 2, "PPO")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    let fast_ema = match lib_ma::ema(values, fast) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let slow_ema = match lib_ma::ema(values, slow) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut result = nan_vec(data_len);
    for i in 0..data_len {
        if !fast_ema[i].is_nan() && !slow_ema[i].is_nan() && slow_ema[i].abs() > 1e-15 {
            result[i] = ((fast_ema[i] - slow_ema[i]) / slow_ema[i]) * 100.0;
        }
    }

    Ok(result)
}

fn fn_trix(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TRIX", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "TRIX")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    let ema1 = match lib_ma::ema(values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let ema1_vec: Vec<f64> = ema1
        .iter()
        .map(|&x| if x.is_nan() { 0.0 } else { x })
        .collect();
    let ema2 = match lib_ma::ema(&ema1_vec, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let ema2_vec: Vec<f64> = ema2
        .iter()
        .map(|&x| if x.is_nan() { 0.0 } else { x })
        .collect();
    let ema3 = match lib_ma::ema(&ema2_vec, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut result = nan_vec(data_len);
    for i in 1..data_len {
        if !ema3[i].is_nan() && !ema3[i - 1].is_nan() && ema3[i - 1].abs() > 1e-15 {
            result[i] = (ema3[i] - ema3[i - 1]) / ema3[i - 1] * 100.0;
        }
    }

    Ok(result)
}

fn fn_bop(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BOP", args, 4)?;
    let open = &args[0];
    let high = &args[1];
    let low = &args[2];
    let close = &args[3];
    let len = open.len().min(high.len()).min(low.len()).min(close.len());
    let mut result = Array1::zeros(len);
    for i in 0..len {
        let range = high[i] - low[i];
        if range.abs() > 1e-15 {
            result[i] = (close[i] - open[i]) / range;
        } else {
            result[i] = 0.0;
        }
    }
    Ok(result)
}

fn fn_apo(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("APO", args, 3)?;
    let input = &args[0];
    let fast = extract_n(args, 1, "APO")?;
    let slow = extract_n(args, 2, "APO")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    let fast_ema = match lib_ma::ema(values, fast) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let slow_ema = match lib_ma::ema(values, slow) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut result = nan_vec(data_len);
    for i in 0..data_len {
        if !fast_ema[i].is_nan() && !slow_ema[i].is_nan() {
            result[i] = fast_ema[i] - slow_ema[i];
        }
    }

    Ok(result)
}

fn fn_dpo(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DPO", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "DPO")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    let ma_vals = match lib_ma::sma(values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let shift = n / 2 + 1;
    let mut result = nan_vec(data_len);
    for i in shift..data_len {
        if !ma_vals[i - shift].is_nan() {
            result[i] = input[i] - ma_vals[i - shift];
        }
    }

    Ok(result)
}

fn fn_linear_reg(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LINEAR_REG", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "LINEAR_REG")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_linear::linreg(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_tsf(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TSF", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "TSF")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    let reg = match lib_linear::linreg(values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let slope_vals = match lib_linear::linreg_slope(values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut result = nan_vec(data_len);
    for i in 0..data_len {
        if !reg[i].is_nan() && !slope_vals[i].is_nan() {
            result[i] = reg[i] + slope_vals[i];
        }
    }

    Ok(result)
}

fn fn_correl(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CORREL", args, 3)?;
    let x = &args[0];
    let y = &args[1];
    let n = extract_n(args, 2, "CORREL")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let x_window: Vec<f64> = (window_start..=i).map(|j| x[j]).collect();
        let y_window: Vec<f64> = (window_start..=i).map(|j| y[j]).collect();
        if let Ok(corr) = lib_stat::correlation(&x_window, &y_window) {
            result[i] = corr;
        }
    }

    Ok(result)
}

fn fn_beta(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BETA", args, 3)?;
    let x = &args[0];
    let y = &args[1];
    let n = extract_n(args, 2, "BETA")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let x_window: Vec<f64> = (window_start..=i).map(|j| x[j]).collect();
        let y_window: Vec<f64> = (window_start..=i).map(|j| y[j]).collect();
        if let (Ok(cov), Ok(var)) = (
            lib_stat::covariance(&x_window, &y_window),
            lib_stat::variance(&y_window),
        ) {
            if var.abs() > 1e-15 {
                result[i] = cov / var;
            }
        }
    }

    Ok(result)
}

fn fn_percent_rank(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("PERCENT_RANK", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "PERCENT_RANK")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let val = input[i];
        let count = (window_start..=i).filter(|&j| input[j] < val).count();
        result[i] = count as f64 / n as f64 * 100.0;
    }

    Ok(result)
}

fn fn_midpoint(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MIDPOINT", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "MIDPOINT")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    let max_vals = match lib_stat::rolling_max(values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let min_vals = match lib_stat::rolling_min(values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut result = nan_vec(data_len);
    for i in 0..data_len {
        if !max_vals[i].is_nan() && !min_vals[i].is_nan() {
            result[i] = (max_vals[i] + min_vals[i]) / 2.0;
        }
    }

    Ok(result)
}

fn fn_midprice(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MIDPRICE", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let n = extract_n(args, 2, "MIDPRICE")?;

    let data_len = ctx.data_len;
    let high_values = high.as_slice().unwrap();
    let low_values = low.as_slice().unwrap();

    let max_vals = match lib_stat::rolling_max(high_values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let min_vals = match lib_stat::rolling_min(low_values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut result = nan_vec(data_len);
    for i in 0..data_len {
        if !max_vals[i].is_nan() && !min_vals[i].is_nan() {
            result[i] = (max_vals[i] + min_vals[i]) / 2.0;
        }
    }

    Ok(result)
}

fn fn_zscore(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ZSCORE", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "ZSCORE")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();

    let mean_vals = match lib_stat::rolling_mean(values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let std_vals = match lib_stat::rolling_std_dev(values, n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut result = nan_vec(data_len);
    for i in 0..data_len {
        if !mean_vals[i].is_nan() && !std_vals[i].is_nan() && std_vals[i].abs() > 1e-15 {
            result[i] = (input[i] - mean_vals[i]) / std_vals[i];
        }
    }

    Ok(result)
}

fn fn_stoch(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("STOCH", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let fastk = if args.len() > 3 && !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0] as usize
    } else {
        14
    };
    let slowk = if args.len() > 4 && !args[4].is_empty() && !args[4][0].is_nan() {
        args[4][0] as usize
    } else {
        3
    };
    let slowd = if args.len() > 5 && !args[5].is_empty() && !args[5][0].is_nan() {
        args[5][0] as usize
    } else {
        3
    };

    let data_len = ctx.data_len;
    let high_values = high.as_slice().unwrap();
    let low_values = low.as_slice().unwrap();
    let close_values = close.as_slice().unwrap();

    match lib_momentum::stoch(high_values, low_values, close_values, fastk, slowk, slowd) {
        Ok(result) => Ok(result.k),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

enum KdjLine {
    K,
    D,
    J,
}

fn extract_kdj_params(args: &[Array1<f64>]) -> (usize, usize, usize) {
    let n = if args.len() > 3 && !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0] as usize
    } else {
        9
    };
    let m1 = if args.len() > 4 && !args[4].is_empty() && !args[4][0].is_nan() {
        args[4][0] as usize
    } else {
        3
    };
    let m2 = if args.len() > 5 && !args[5].is_empty() && !args[5][0].is_nan() {
        args[5][0] as usize
    } else {
        3
    };
    (n, m1, m2)
}

fn fn_kdj_line(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
    line: KdjLine,
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("KDJ", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let (n, m1, m2) = extract_kdj_params(args);

    let data_len = ctx.data_len;
    let high_values = high.as_slice().unwrap();
    let low_values = low.as_slice().unwrap();
    let close_values = close.as_slice().unwrap();

    match lib_kdj(high_values, low_values, close_values, n, m1, m2) {
        Ok(result) => Ok(match line {
            KdjLine::K => result.k,
            KdjLine::D => result.d,
            KdjLine::J => result.j,
        }),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_kdj(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    fn_kdj_line(ctx, args, KdjLine::K)
}

fn fn_kdj_d(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    fn_kdj_line(ctx, args, KdjLine::D)
}

fn fn_kdj_j(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    fn_kdj_line(ctx, args, KdjLine::J)
}

fn fn_bias(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BIAS", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "BIAS")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_bias(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_psy(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("PSY", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "PSY")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_psy(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_hma(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HMA", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "HMA")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_ma::hma(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_alma(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ALMA", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "ALMA")?;
    let sigma = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0]
    } else {
        6.0
    };
    let offset = if args.len() > 3 && !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0]
    } else {
        0.85
    };

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_ma::alma(values, n, sigma, offset) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_cmf(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CMF", args, 5)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let volume = &args[3];
    let n = extract_n(args, 4, "CMF")?;

    let data_len = ctx.data_len;
    let high_values = high.as_slice().unwrap();
    let low_values = low.as_slice().unwrap();
    let close_values = close.as_slice().unwrap();
    let volume_values = volume.as_slice().unwrap();

    match lib_cmf(high_values, low_values, close_values, volume_values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_fisher(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FISHER", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let n = extract_n(args, 2, "FISHER")?;

    let data_len = ctx.data_len;
    let high_values = high.as_slice().unwrap();
    let low_values = low.as_slice().unwrap();

    match lib_fisher(high_values, low_values, n) {
        Ok(result) => Ok(result.fisher),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_fisher_signal(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FISHER_SIGNAL", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let n = extract_n(args, 2, "FISHER_SIGNAL")?;

    let data_len = ctx.data_len;
    let high_values = high.as_slice().unwrap();
    let low_values = low.as_slice().unwrap();

    match lib_fisher(high_values, low_values, n) {
        Ok(result) => Ok(result.signal),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_tsi(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TSI", args, 2)?;
    let input = &args[0];
    let long_n = extract_n(args, 1, "TSI")?;
    let short_n = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0] as usize
    } else {
        13
    };

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_tsi(values, long_n, short_n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_chop(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CHOP", args, 4)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let n = extract_n(args, 3, "CHOP")?;

    let data_len = ctx.data_len;
    let high_values = high.as_slice().unwrap();
    let low_values = low.as_slice().unwrap();
    let close_values = close.as_slice().unwrap();

    match lib_chop(high_values, low_values, close_values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_adx(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ADX", args, 4)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let n = extract_n(args, 3, "ADX")?;

    let data_len = ctx.data_len;
    let mut plus_dm = Array1::zeros(data_len);
    let mut minus_dm = Array1::zeros(data_len);
    let mut tr = Array1::zeros(data_len);

    tr[0] = high[0] - low[0];
    for i in 1..data_len {
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];
        plus_dm[i] = if up_move > down_move && up_move > 0.0 {
            up_move
        } else {
            0.0
        };
        minus_dm[i] = if down_move > up_move && down_move > 0.0 {
            down_move
        } else {
            0.0
        };
        let hl = high[i] - low[i];
        let hc = (high[i] - close[i - 1]).abs();
        let lc = (low[i] - close[i - 1]).abs();
        tr[i] = hl.max(hc).max(lc);
    }

    let atr_vals = match lib_ma::sma(tr.as_slice().unwrap(), n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let apdm_vals = match lib_ma::sma(plus_dm.as_slice().unwrap(), n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };
    let amdm_vals = match lib_ma::sma(minus_dm.as_slice().unwrap(), n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut dx = nan_vec(data_len);
    for i in 0..data_len {
        if !atr_vals[i].is_nan() && atr_vals[i].abs() > 1e-15 {
            let pdi = apdm_vals[i] / atr_vals[i] * 100.0;
            let mdi = amdm_vals[i] / atr_vals[i] * 100.0;
            let sum = pdi + mdi;
            if sum.abs() > 1e-15 {
                dx[i] = (pdi - mdi).abs() / sum * 100.0;
            }
        }
    }

    let dx_vec: Vec<f64> = dx
        .iter()
        .map(|&v| if v.is_nan() { 0.0 } else { v })
        .collect();
    match lib_ma::sma(&dx_vec, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_sar(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SAR", args, 4)?;
    let high = &args[0];
    let low = &args[1];
    let af_step = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0]
    } else {
        0.02
    };
    let af_max = if args.len() > 3 && !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0]
    } else {
        0.2
    };

    let data_len = high.len().min(low.len());
    if data_len < 2 {
        return Ok(nan_vec(data_len));
    }

    let mut result = nan_vec(data_len);
    let mut is_long = high[1] - low[1] > 0.0;
    let mut af = af_step;
    let mut ep = if is_long { high[0] } else { low[0] };
    result[0] = if is_long { low[0] } else { high[0] };

    for i in 1..data_len {
        let prev_sar = result[i - 1];
        let mut sar = prev_sar + af * (ep - prev_sar);

        if is_long {
            sar = sar.min(low[i - 1]);
            if i >= 2 {
                sar = sar.min(low[i - 2]);
            }
            if low[i] < sar {
                is_long = false;
                sar = ep;
                af = af_step;
                ep = low[i];
            } else {
                if high[i] > ep {
                    ep = high[i];
                    af = (af + af_step).min(af_max);
                }
            }
        } else {
            sar = sar.max(high[i - 1]);
            if i >= 2 {
                sar = sar.max(high[i - 2]);
            }
            if high[i] > sar {
                is_long = true;
                sar = ep;
                af = af_step;
                ep = high[i];
            } else {
                if low[i] < ep {
                    ep = low[i];
                    af = (af + af_step).min(af_max);
                }
            }
        }

        result[i] = sar;
    }

    Ok(result)
}

fn fn_ichimoku_tenkan(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ICHIMOKU_TENKAN", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let n = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0] as usize
    } else {
        9
    };

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let hh = (window_start..=i)
            .map(|j| high[j])
            .fold(f64::NEG_INFINITY, f64::max);
        let ll = (window_start..=i)
            .map(|j| low[j])
            .fold(f64::INFINITY, f64::min);
        result[i] = (hh + ll) / 2.0;
    }

    Ok(result)
}

fn fn_ichimoku_kijun(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ICHIMOKU_KIJUN", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let n = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0] as usize
    } else {
        26
    };

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let hh = (window_start..=i)
            .map(|j| high[j])
            .fold(f64::NEG_INFINITY, f64::max);
        let ll = (window_start..=i)
            .map(|j| low[j])
            .fold(f64::INFINITY, f64::min);
        result[i] = (hh + ll) / 2.0;
    }

    Ok(result)
}

#[allow(unused_assignments)]
fn fn_supertrend(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SUPERTREND", args, 4)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let atr_n = if args.len() > 3 && !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0] as usize
    } else {
        14
    };
    let mult = if args.len() > 4 && !args[4].is_empty() && !args[4][0].is_nan() {
        args[4][0]
    } else {
        3.0
    };

    let data_len = ctx.data_len;
    let mut tr = Array1::zeros(data_len);
    tr[0] = high[0] - low[0];
    for i in 1..data_len {
        let hl = high[i] - low[i];
        let hc = (high[i] - close[i - 1]).abs();
        let lc = (low[i] - close[i - 1]).abs();
        tr[i] = hl.max(hc).max(lc);
    }

    let atr_vals = match lib_ma::sma(tr.as_slice().unwrap(), atr_n) {
        Ok(r) => r,
        Err(_) => return Ok(nan_vec(data_len)),
    };

    let mut result = nan_vec(data_len);
    let mut upper_band = f64::NAN;
    let mut lower_band = f64::NAN;
    let mut prev_upper = f64::NAN;
    let mut prev_lower = f64::NAN;
    let mut is_long = true;

    for i in 0..data_len {
        if atr_vals[i].is_nan() {
            continue;
        }
        let hl2 = (high[i] + low[i]) / 2.0;
        upper_band = hl2 + mult * atr_vals[i];
        lower_band = hl2 - mult * atr_vals[i];

        if !prev_lower.is_nan() {
            if !(lower_band < prev_lower || close[i - 1] < prev_lower) {
                lower_band = prev_lower;
            }
            if !(upper_band > prev_upper || close[i - 1] > prev_upper) {
                upper_band = prev_upper;
            }
        }

        if is_long {
            if close[i] < lower_band {
                is_long = false;
                result[i] = upper_band;
            } else {
                result[i] = lower_band;
            }
        } else {
            if close[i] > upper_band {
                is_long = true;
                result[i] = lower_band;
            } else {
                result[i] = upper_band;
            }
        }

        prev_upper = upper_band;
        prev_lower = lower_band;
    }

    Ok(result)
}

fn fn_vwap(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("VWAP", args, 4)?;
    let high = &args[0];
    let low = &args[1];
    let close = &args[2];
    let volume = &args[3];
    let len = high.len().min(low.len()).min(close.len()).min(volume.len());
    let mut result = Array1::zeros(len);
    let mut cum_tp_vol = 0.0f64;
    let mut cum_vol = 0.0f64;
    for i in 0..len {
        let tp = (high[i] + low[i] + close[i]) / 3.0;
        cum_tp_vol += tp * volume[i];
        cum_vol += volume[i];
        if cum_vol.abs() > 1e-15 {
            result[i] = cum_tp_vol / cum_vol;
        }
    }
    Ok(result)
}

fn fn_donchian(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DONCHIAN", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let n = extract_n(args, 2, "DONCHIAN")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let hh = (window_start..=i)
            .map(|j| high[j])
            .fold(f64::NEG_INFINITY, f64::max);
        let ll = (window_start..=i)
            .map(|j| low[j])
            .fold(f64::INFINITY, f64::min);
        result[i] = (hh + ll) / 2.0;
    }

    Ok(result)
}

fn fn_donchian_upper(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DONCHIAN_UPPER", args, 3)?;
    let high = &args[0];
    let _low = &args[1];
    let n = extract_n(args, 2, "DONCHIAN_UPPER")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        result[i] = (window_start..=i)
            .map(|j| high[j])
            .fold(f64::NEG_INFINITY, f64::max);
    }

    Ok(result)
}

fn fn_donchian_lower(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DONCHIAN_LOWER", args, 3)?;
    let _high = &args[0];
    let low = &args[1];
    let n = extract_n(args, 2, "DONCHIAN_LOWER")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        result[i] = (window_start..=i)
            .map(|j| low[j])
            .fold(f64::INFINITY, f64::min);
    }

    Ok(result)
}

fn fn_donchian_middle(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    fn_donchian(ctx, args)
}

fn fn_donchian_width(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DONCHIAN_WIDTH", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let n = extract_n(args, 2, "DONCHIAN_WIDTH")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let hh = (window_start..=i)
            .map(|j| high[j])
            .fold(f64::NEG_INFINITY, f64::max);
        let ll = (window_start..=i)
            .map(|j| low[j])
            .fold(f64::INFINITY, f64::min);
        result[i] = hh - ll;
    }

    Ok(result)
}

fn fn_strcat(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("STRCAT", args, 2)?;
    Err(FormulaError::InvalidOperation(
        "String concatenation (&) is not supported for numeric values. Use STRCAT() function instead.".to_string()
    ))
}

fn fn_ifthen(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("IFTHEN", args, 2)?;
    let cond = &args[0];
    let then_val = &args[1];
    let len = cond.len().min(then_val.len());
    let mut result = Array1::zeros(len);
    for i in 0..len {
        result[i] = if cond[i] > 0.0 { then_val[i] } else { 0.0 };
    }
    Ok(result)
}

fn fn_not(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("NOT", args, 1)?;
    Ok(args[0].mapv(|v| if v > 0.0 { 0.0 } else { 1.0 }))
}

fn fn_histvol(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HISTVOL", args, 2)?;
    let close = &args[0];
    let n = extract_n(args, 1, "HISTVOL")?;

    let data_len = ctx.data_len;
    let mut log_returns = Array1::zeros(data_len);
    log_returns[0] = 0.0;
    for i in 1..data_len {
        if close[i - 1].abs() > 1e-15 {
            log_returns[i] = (close[i] / close[i - 1]).ln();
        }
    }

    match lib_stat::rolling_std_dev(log_returns.as_slice().unwrap(), n) {
        Ok(result) => Ok(result.mapv(|v| v * (252.0_f64).sqrt())),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_obv_enhanced(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    fn_obv(ctx, args)
}

fn fn_atr_enhanced(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    fn_atr(ctx, args)
}

fn fn_boll_enhanced(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    fn_boll(ctx, args)
}

fn fn_dmi(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    fn_adx(ctx, args)
}

fn fn_dx(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let (high, low, close, n) = resolve_hlc_args("DX", ctx, args)?;
    let data_len = ctx.data_len;
    match crate::indicators::momentum::dx(
        high.as_slice().unwrap(),
        low.as_slice().unwrap(),
        close.as_slice().unwrap(),
        n,
    ) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_plus_di(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let (high, low, close, n) = resolve_hlc_args("PLUS_DI", ctx, args)?;
    let data_len = ctx.data_len;
    match crate::indicators::momentum::plus_di(
        high.as_slice().unwrap(),
        low.as_slice().unwrap(),
        close.as_slice().unwrap(),
        n,
    ) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_minus_di(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let (high, low, close, n) = resolve_hlc_args("MINUS_DI", ctx, args)?;
    let data_len = ctx.data_len;
    match crate::indicators::momentum::minus_di(
        high.as_slice().unwrap(),
        low.as_slice().unwrap(),
        close.as_slice().unwrap(),
        n,
    ) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_adxr(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let (high, low, close, n) = resolve_hlc_args("ADXR", ctx, args)?;
    let data_len = ctx.data_len;
    match crate::indicators::momentum::adxr(
        high.as_slice().unwrap(),
        low.as_slice().unwrap(),
        close.as_slice().unwrap(),
        n,
    ) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_aroonosc(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let (high, low, n) = resolve_hl_args("AROONOSC", ctx, args)?;
    let data_len = ctx.data_len;
    match crate::indicators::momentum::aroonosc(
        high.as_slice().unwrap(),
        low.as_slice().unwrap(),
        n,
    ) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_aroon_up(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let (high, low, n) = resolve_hl_args("AROON_UP", ctx, args)?;
    let data_len = ctx.data_len;
    match crate::indicators::momentum::aroon(high.as_slice().unwrap(), low.as_slice().unwrap(), n) {
        Ok(r) => Ok(r.aroon_up),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_aroon_dn(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let (high, low, n) = resolve_hl_args("AROON_DN", ctx, args)?;
    let data_len = ctx.data_len;
    match crate::indicators::momentum::aroon(high.as_slice().unwrap(), low.as_slice().unwrap(), n) {
        Ok(r) => Ok(r.aroon_down),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

/// Resolve (HIGH, LOW, CLOSE, N) or (CLOSE, N) → auto-expand from context
#[allow(clippy::type_complexity)]
fn resolve_hlc_args<'a>(
    name: &str,
    ctx: &'a FormulaContext,
    args: &'a [Array1<f64>],
) -> Result<(&'a Array1<f64>, &'a Array1<f64>, &'a Array1<f64>, usize), FormulaError> {
    if args.len() >= 4 {
        let n = extract_n(args, 3, name)?;
        Ok((&args[0], &args[1], &args[2], n))
    } else if args.len() >= 2 {
        let n = extract_n(args, 1, name)?;
        Ok((&ctx.high, &ctx.low, &ctx.close, n))
    } else {
        Err(FormulaError::RuntimeError(format!(
            "{name} requires at least 2 arguments (CLOSE,N) or 4 arguments (HIGH,LOW,CLOSE,N), got {}",
            args.len()
        )))
    }
}

/// Resolve (HIGH, LOW, N) or (CLOSE, N) → auto-expand from context
fn resolve_hl_args<'a>(
    name: &str,
    ctx: &'a FormulaContext,
    args: &'a [Array1<f64>],
) -> Result<(&'a Array1<f64>, &'a Array1<f64>, usize), FormulaError> {
    if args.len() >= 3 {
        let n = extract_n(args, 2, name)?;
        Ok((&args[0], &args[1], n))
    } else if !args.is_empty() {
        let n = extract_n(args, 0, name)?;
        Ok((&ctx.high, &ctx.low, n))
    } else {
        Err(FormulaError::RuntimeError(format!(
            "{name} requires at least 1 argument (N) or 3 arguments (HIGH,LOW,N), got {}",
            args.len()
        )))
    }
}

// ======================== CYCLE INDICATORS (TA-Lib C compat) ========================

fn fn_stochf(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("STOCHF", args, 3)?;
    let h = args[0].as_slice().unwrap();
    let l = args[1].as_slice().unwrap();
    let c = args[2].as_slice().unwrap();
    let fastk = if args.len() > 3 && !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0] as usize
    } else {
        5
    };
    let fastd = if args.len() > 4 && !args[4].is_empty() && !args[4][0].is_nan() {
        args[4][0] as usize
    } else {
        3
    };
    let data_len = ctx.data_len;
    match lib_momentum::stochf(h, l, c, fastk, fastd) {
        Ok(result) => Ok(result.k),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_stochrsi(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("STOCHRSI", args, 1)?;
    let data = args[0].as_slice().unwrap();
    let rsi_period = if args.len() > 1 && !args[1].is_empty() && !args[1][0].is_nan() {
        args[1][0] as usize
    } else {
        14
    };
    let stoch_period = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0] as usize
    } else {
        14
    };
    let fastk_period = if args.len() > 3 && !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0] as usize
    } else {
        3
    };
    let fastd_period = if args.len() > 4 && !args[4].is_empty() && !args[4][0].is_nan() {
        args[4][0] as usize
    } else {
        3
    };
    let data_len = ctx.data_len;
    match lib_momentum::stochrsi(data, rsi_period, stoch_period, fastk_period, fastd_period) {
        Ok(result) => Ok(result.k),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_ultosc(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let (high, low, close) = resolve_hlc_for_ultosc("ULTOSC", ctx, args)?;
    let p1 = if args.len() > 3 && !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0] as usize
    } else {
        7
    };
    let p2 = if args.len() > 4 && !args[4].is_empty() && !args[4][0].is_nan() {
        args[4][0] as usize
    } else {
        14
    };
    let p3 = if args.len() > 5 && !args[5].is_empty() && !args[5][0].is_nan() {
        args[5][0] as usize
    } else {
        28
    };
    let data_len = ctx.data_len;
    match lib_momentum::ultosc(
        high.as_slice().unwrap(),
        low.as_slice().unwrap(),
        close.as_slice().unwrap(),
        p1,
        p2,
        p3,
    ) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

/// Resolve (HIGH, LOW, CLOSE) for ULTOSC: either 3 args or auto-fill from context.
fn resolve_hlc_for_ultosc<'a>(
    name: &str,
    _ctx: &'a FormulaContext,
    args: &'a [Array1<f64>],
) -> Result<(&'a Array1<f64>, &'a Array1<f64>, &'a Array1<f64>), FormulaError> {
    if args.len() >= 3 {
        Ok((&args[0], &args[1], &args[2]))
    } else {
        Err(FormulaError::RuntimeError(format!(
            "{name} requires at least 3 arguments (HIGH,LOW,CLOSE), got {}",
            args.len()
        )))
    }
}

fn fn_plus_dm(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let (high, low) = resolve_hl_for_dm("PLUS_DM", ctx, args)?;
    let data_len = ctx.data_len;
    match lib_momentum::plus_dm(high.as_slice().unwrap(), low.as_slice().unwrap()) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_minus_dm(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let (high, low) = resolve_hl_for_dm("MINUS_DM", ctx, args)?;
    let data_len = ctx.data_len;
    match lib_momentum::minus_dm(high.as_slice().unwrap(), low.as_slice().unwrap()) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn resolve_hl_for_dm<'a>(
    name: &str,
    _ctx: &'a FormulaContext,
    args: &'a [Array1<f64>],
) -> Result<(&'a Array1<f64>, &'a Array1<f64>), FormulaError> {
    if args.len() >= 2 {
        Ok((&args[0], &args[1]))
    } else {
        Err(FormulaError::RuntimeError(format!(
            "{name} requires at least 2 arguments (HIGH,LOW), got {}",
            args.len()
        )))
    }
}

// ======================== CYCLE INDICATORS (Hilbert Transform) ========================

fn fn_ht_phasor_inner(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HT_PHASOR", args, 1)?;
    let data = args[0].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_cycle::ht_phasor(data) {
        Ok((in_phase, _quadrature)) => Ok(in_phase),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_ht_sine_inner(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HT_SINE", args, 1)?;
    let data = args[0].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_cycle::ht_sine(data) {
        Ok((sine, _lead_sine)) => Ok(sine),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_ht_dcperiod(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HT_DCPERIOD", args, 1)?;
    let data = args[0].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_cycle::ht_dcperiod(data) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_ht_dcphase(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HT_DCPHASE", args, 1)?;
    let data = args[0].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_cycle::ht_dcphase(data) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_ht_trendmode(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HT_TRENDMODE", args, 1)?;
    let data = args[0].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_cycle::ht_trendmode(data) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_ht_trendline(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HT_TRENDLINE", args, 1)?;
    let data = args[0].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_cycle::ht_trendline(data) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_ht_measurement(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HT_MEASUREMENT", args, 1)?;
    let data = args[0].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_cycle::ht_measurement(data) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

// ======================== CLASSIC CHART PATTERNS ========================
// These are first-class stock-trading patterns (Darvas, Renko, Kagi, PnF,
// TLB, Alligator). They return a single per-bar series chosen to match
// the canonical "trend filter" used by traders.

fn fn_darvas_box_top(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DARVAS_BOX", args, 3)?;
    let h = args[0].as_slice().unwrap();
    let l = args[1].as_slice().unwrap();
    let c = args[2].as_slice().unwrap();
    let lookback = if args.len() > 3 && !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0] as usize
    } else {
        5
    };
    let confirmation = if args.len() > 4 && !args[4].is_empty() && !args[4][0].is_nan() {
        args[4][0] as usize
    } else {
        3
    };
    let data_len = ctx.data_len;
    match lib_classic::darvas_box(h, l, c, lookback, confirmation) {
        Ok(r) => Ok(r.box_top),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_renko(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("RENKO", args, 3)?;
    let h = args[0].as_slice().unwrap();
    let l = args[1].as_slice().unwrap();
    let box_size = if !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0]
    } else {
        1.0
    };
    let data_len = ctx.data_len;
    match lib_classic::renko(h, l, box_size) {
        Ok(r) => Ok(r.bricks),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_kagi(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("KAGI", args, 2)?;
    let c = args[0].as_slice().unwrap();
    let reversal = if !args[1].is_empty() && !args[1][0].is_nan() {
        args[1][0]
    } else {
        1.0
    };
    let data_len = ctx.data_len;
    match lib_classic::kagi(c, reversal) {
        Ok(r) => Ok(r.kagi),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_point_and_figure(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("POINT_AND_FIGURE", args, 4)?;
    let h = args[0].as_slice().unwrap();
    let l = args[1].as_slice().unwrap();
    let box_size = if !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0]
    } else {
        1.0
    };
    let reversal = if !args[3].is_empty() && !args[3][0].is_nan() {
        args[3][0] as usize
    } else {
        3
    };
    let data_len = ctx.data_len;
    match lib_classic::point_and_figure(h, l, box_size, reversal) {
        Ok(r) => Ok(r.pnf),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_three_line_break(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("THREE_LINE_BREAK", args, 2)?;
    let c = args[0].as_slice().unwrap();
    let lines = if !args[1].is_empty() && !args[1][0].is_nan() {
        args[1][0] as usize
    } else {
        3
    };
    let data_len = ctx.data_len;
    match lib_classic::three_line_break(c, lines) {
        Ok(r) => Ok(r.line),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_williams_alligator_lips(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("WILLIAMS_ALLIGATOR", args, 1)?;
    let c = args[0].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_classic::williams_alligator(c) {
        Ok(r) => Ok(r.lips),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_heikin_ashi_close(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HEIKIN_ASHI", args, 4)?;
    let o = args[0].as_slice().unwrap();
    let h = args[1].as_slice().unwrap();
    let l = args[2].as_slice().unwrap();
    let c = args[3].as_slice().unwrap();
    let data_len = ctx.data_len;
    match crate::indicators::heikin_ashi(o, h, l, c) {
        Ok(r) => Ok(r.ha_close),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

// ======================== A-SHARE SPECIFIC INDICATORS ========================

fn fn_main_net_inflow(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MAIN_NET_INFLOW", args, 2)?;
    let close = args[0].as_slice().unwrap();
    let vol = args[1].as_slice().unwrap();
    let threshold = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0]
    } else {
        0.0
    };
    let data_len = ctx.data_len;
    match lib_astock::main_net_inflow(close, vol, threshold) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_money_flow(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MONEY_FLOW", args, 4)?;
    let h = args[0].as_slice().unwrap();
    let l = args[1].as_slice().unwrap();
    let c = args[2].as_slice().unwrap();
    let v = args[3].as_slice().unwrap();
    let period = if args.len() > 4 && !args[4].is_empty() && !args[4][0].is_nan() {
        args[4][0] as usize
    } else {
        14
    };
    let data_len = ctx.data_len;
    match lib_astock::money_flow(h, l, c, v, period) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_limit_up(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LIMIT_UP", args, 2)?;
    let close = args[0].as_slice().unwrap();
    let prev_close = args[1].as_slice().unwrap();
    let threshold = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0]
    } else {
        0.10
    };
    let data_len = ctx.data_len;
    match lib_astock::limit_up(close, prev_close, threshold) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_limit_down(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LIMIT_DOWN", args, 2)?;
    let close = args[0].as_slice().unwrap();
    let prev_close = args[1].as_slice().unwrap();
    let threshold = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0]
    } else {
        0.10
    };
    let data_len = ctx.data_len;
    match lib_astock::limit_down(close, prev_close, threshold) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_consecutive_limit(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CONSECUTIVE_LIMIT", args, 1)?;
    let signal = args[0].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_astock::consecutive_limit(signal) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_turnover(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TURNOVER", args, 2)?;
    let vol = args[0].as_slice().unwrap();
    let free_float = args[1].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_astock::turnover(vol, free_float) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_rs_ratio(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("RS_RATIO", args, 2)?;
    let close = args[0].as_slice().unwrap();
    let benchmark = args[1].as_slice().unwrap();
    let period = if args.len() > 2 && !args[2].is_empty() && !args[2][0].is_nan() {
        args[2][0] as usize
    } else {
        20
    };
    let data_len = ctx.data_len;
    match lib_astock::rs_ratio(close, benchmark, period) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

// ======================== TIME / BAR FUNCTIONS (TDX) ========================

fn datetime_component(ctx: &FormulaContext, extract: fn(i64) -> f64) -> Array1<f64> {
    let len = ctx.data_len;
    match &ctx.datetime {
        Some(dt) => dt.mapv(extract),
        None => nan_vec(len),
    }
}

fn ts_to_date_parts(ts: i64) -> (i32, u32, u32, u32, u32, u32) {
    let total_days = (ts / 86400) as i32;
    let time_of_day = ts.rem_euclid(86400) as u32;
    let hour = time_of_day / 3600;
    let minute = (time_of_day % 3600) / 60;
    let second = time_of_day % 60;

    let mut y = 1970;
    let mut remaining = total_days;
    loop {
        let days_in_year = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let month_days: [i32; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0u32;
    for md in &month_days {
        if remaining < *md {
            break;
        }
        remaining -= *md;
        m += 1;
    }
    (y, m + 1, remaining as u32 + 1, hour, minute, second)
}

fn fn_year(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| ts_to_date_parts(ts).0 as f64))
}

fn fn_month(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| ts_to_date_parts(ts).1 as f64))
}

fn fn_day(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| ts_to_date_parts(ts).2 as f64))
}

fn fn_hour(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| ts_to_date_parts(ts).3 as f64))
}

fn fn_minute(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| ts_to_date_parts(ts).4 as f64))
}

fn fn_time(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| {
        let (_, _, _, h, m, _s) = ts_to_date_parts(ts);
        (h * 100 + m) as f64
    }))
}

fn fn_weekday(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| {
        let days = ts / 86400;
        ((days % 7 + 4) % 7) as f64 // 1970-01-01 was Thursday(4), 0=Sun
    }))
}

fn fn_currbarscount(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let len = ctx.data_len;
    Ok(Array1::from_vec(
        (0..len).map(|i| (len - i) as f64).collect(),
    ))
}

fn fn_totalbarscount(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    Ok(Array1::from_elem(ctx.data_len, ctx.data_len as f64))
}

fn fn_barssince(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BARSSINCE", args, 1)?;
    let cond = &args[0];
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let mut last_true: Option<usize> = None;
    for i in 0..len {
        if cond[i] != 0.0 && !cond[i].is_nan() {
            last_true = Some(i);
        }
        if let Some(lt) = last_true {
            out[i] = (i - lt) as f64;
        }
    }
    Ok(out)
}

fn fn_barssincen(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BARSSINCEN", args, 2)?;
    let cond = &args[0];
    let n = extract_n(args, 1, "BARSSINCEN")?;
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let mut count = 0usize;
    let mut last_bar: Option<usize> = None;
    for i in 0..len {
        if cond[i] != 0.0 && !cond[i].is_nan() {
            count += 1;
            if count >= n {
                last_bar = Some(i);
            }
        }
        if let Some(lb) = last_bar {
            out[i] = (i - lb) as f64;
        }
    }
    Ok(out)
}

fn fn_barscount(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BARSCOUNT", args, 1)?;
    let data = &args[0];
    let len = ctx.data_len;
    let mut out = Array1::zeros(len);
    let mut valid_count = 0.0f64;
    for i in 0..len {
        if !data[i].is_nan() {
            valid_count += 1.0;
        }
        out[i] = valid_count;
    }
    Ok(out)
}

fn fn_barstatus(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let len = ctx.data_len;
    let mut out = Array1::from_elem(len, 0.0);
    if len > 0 {
        out[0] = 1.0; // first bar
        out[len - 1] = 2.0; // last bar
    }
    Ok(out)
}

fn fn_islastbar(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let len = ctx.data_len;
    let mut out = Array1::zeros(len);
    if len > 0 {
        out[len - 1] = 1.0;
    }
    Ok(out)
}

fn fn_fromopen(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| {
        let (_, _, _, h, m, _) = ts_to_date_parts(ts);
        let minutes_since_930 = (h as i32 - 9) * 60 + m as i32 - 30;
        minutes_since_930.max(0) as f64
    }))
}

fn fn_date_tdx(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| {
        let (y, m, d, _, _, _) = ts_to_date_parts(ts);
        ((y - 1900) * 10000 + m as i32 * 100 + d as i32) as f64
    }))
}

// ======================== MATH / STATISTICS EXTENSIONS (TDX) ========================

fn fn_avedev(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

fn fn_devsq(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

fn fn_slope(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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
        if denom.abs() > 1e-15 {
            out[i] = (n_f * sum_xy - sum_x * sum_y) / denom;
        } else {
            out[i] = 0.0;
        }
    }
    Ok(out)
}

fn fn_forcast(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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
        if denom.abs() > 1e-15 {
            let slope = (n_f * sum_xy - sum_x * sum_y) / denom;
            let intercept = (sum_y - slope * sum_x) / n_f;
            out[i] = intercept + slope * (n_f - 1.0);
        } else {
            out[i] = window.iter().sum::<f64>() / n_f;
        }
    }
    Ok(out)
}

fn fn_range(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

fn fn_const_val(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CONST", args, 1)?;
    let last_val = args[0][ctx.data_len.saturating_sub(1)];
    Ok(Array1::from_elem(ctx.data_len, last_val))
}

fn fn_sumbars(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SUMBARS", args, 2)?;
    let input = &args[0];
    let target = &args[1];
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let values = input.as_slice().unwrap();
    for i in 0..len {
        let threshold = target[i];
        let mut cumsum = 0.0;
        let mut bars = 0.0;
        for j in (0..=i).rev() {
            cumsum += values[j];
            bars += 1.0;
            if cumsum >= threshold {
                break;
            }
        }
        out[i] = bars;
    }
    Ok(out)
}

fn fn_intpart(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("INTPART", args, 1)?;
    Ok(args[0].mapv(|x| x.trunc()))
}

fn fn_fracpart(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FRACPART", args, 1)?;
    Ok(args[0].mapv(|x| x.fract()))
}

fn fn_mod(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MOD", args, 2)?;
    let a = &args[0];
    let b = &args[1];
    let len = a.len();
    let mut out = Array1::zeros(len);
    for i in 0..len {
        if b[i].abs() > 1e-15 {
            out[i] = a[i] % b[i];
        } else {
            out[i] = f64::NAN;
        }
    }
    Ok(out)
}

fn fn_reverse(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("REVERSE", args, 1)?;
    let input = &args[0];
    let len = ctx.data_len;
    let mut out = Array1::zeros(len);
    for i in 0..len {
        out[i] = input[len - 1 - i];
    }
    Ok(out)
}

fn fn_tr(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let len = ctx.data_len;
    let mut out = Array1::zeros(len);
    let h = ctx.high.as_slice().unwrap();
    let l = ctx.low.as_slice().unwrap();
    let c = ctx.close.as_slice().unwrap();
    out[0] = h[0] - l[0];
    for i in 1..len {
        let hl = h[i] - l[i];
        let hc = (h[i] - c[i - 1]).abs();
        let lc = (l[i] - c[i - 1]).abs();
        out[i] = hl.max(hc).max(lc);
    }
    Ok(out)
}

// ======================== INDEX / FINANCE / CHIP FUNCTIONS ========================

fn fn_indexc(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    match &ctx.index_data {
        Some(idx) => Ok(idx.close.clone().unwrap_or_else(|| nan_vec(ctx.data_len))),
        None => Ok(nan_vec(ctx.data_len)),
    }
}

fn fn_indexo(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    match &ctx.index_data {
        Some(idx) => Ok(idx.open.clone().unwrap_or_else(|| nan_vec(ctx.data_len))),
        None => Ok(nan_vec(ctx.data_len)),
    }
}

fn fn_indexh(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    match &ctx.index_data {
        Some(idx) => Ok(idx.high.clone().unwrap_or_else(|| nan_vec(ctx.data_len))),
        None => Ok(nan_vec(ctx.data_len)),
    }
}

fn fn_indexl(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    match &ctx.index_data {
        Some(idx) => Ok(idx.low.clone().unwrap_or_else(|| nan_vec(ctx.data_len))),
        None => Ok(nan_vec(ctx.data_len)),
    }
}

fn fn_indexv(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    match &ctx.index_data {
        Some(idx) => Ok(idx.volume.clone().unwrap_or_else(|| nan_vec(ctx.data_len))),
        None => Ok(nan_vec(ctx.data_len)),
    }
}

fn fn_indexa(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    match &ctx.index_data {
        Some(idx) => Ok(idx.amount.clone().unwrap_or_else(|| nan_vec(ctx.data_len))),
        None => Ok(nan_vec(ctx.data_len)),
    }
}

fn fn_capital(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let val = ctx.capital.unwrap_or(f64::NAN);
    Ok(Array1::from_elem(ctx.data_len, val))
}

fn fn_finance(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FINANCE", args, 1)?;
    let field_id = args[0][0] as usize;
    let val = ctx
        .finance_data
        .as_ref()
        .and_then(|fd| fd.fields.get(&field_id).copied())
        .unwrap_or(f64::NAN);
    Ok(Array1::from_elem(ctx.data_len, val))
}

fn fn_dynainfo(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DYNAINFO", args, 1)?;
    let field_id = args[0][0] as usize;
    let data_len = ctx.data_len;

    match &ctx.dynainfo {
        Some(di) => {
            let val = di.fields.get(&field_id).copied().unwrap_or(f64::NAN);
            Ok(Array1::from_elem(data_len, val))
        }
        None => Ok(nan_vec(data_len)),
    }
}

fn fn_winner(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("WINNER", args, 1)?;
    let price_input = &args[0];
    let data_len = ctx.data_len;

    match &ctx.chip_data {
        Some(chip) => {
            let mut result = Array1::zeros(data_len);
            for i in 0..data_len {
                result[i] = chip.winner(price_input[i]);
            }
            Ok(result)
        }
        None => Ok(nan_vec(data_len)),
    }
}

fn fn_lwinner(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LWINNER", args, 2)?;
    let price_input = &args[0];
    let n_days = args[1][0] as usize;
    let data_len = ctx.data_len;

    match &ctx.chip_data {
        Some(chip) => {
            let mut result = Array1::zeros(data_len);
            for i in 0..data_len {
                if i >= n_days {
                    result[i] = chip.winner(price_input[i]);
                } else {
                    result[i] = f64::NAN;
                }
            }
            Ok(result)
        }
        None => Ok(nan_vec(data_len)),
    }
}

fn fn_cost(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("COST", args, 1)?;
    let ratio_input = &args[0];
    let data_len = ctx.data_len;

    match &ctx.chip_data {
        Some(chip) => {
            let mut result = Array1::zeros(data_len);
            for i in 0..data_len {
                let ratio = ratio_input[i] / 100.0;
                if (0.0..=1.0).contains(&ratio) {
                    result[i] = chip.cost(ratio);
                } else {
                    result[i] = f64::NAN;
                }
            }
            Ok(result)
        }
        None => Ok(nan_vec(data_len)),
    }
}

// ======================== DZH BLOCK FUNCTIONS ========================

fn fn_blockdata(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BLOCKDATA", args, 2)?;
    let block_name = get_string_from_hash(ctx, args[0][0]).ok_or_else(|| {
        FormulaError::InvalidParameter("BLOCKDATA: block name must be a valid string".to_string())
    })?;
    let field = get_string_from_hash(ctx, args[1][0]).ok_or_else(|| {
        FormulaError::InvalidParameter("BLOCKDATA: field must be a valid string".to_string())
    })?;

    let data_len = ctx.data_len;
    match &ctx.block_data {
        Some(block_data) => {
            let field_upper = field.to_uppercase();
            match field_upper.as_str() {
                "INDEX" | "CLOSE" | "C" => block_data
                    .index_close
                    .get(&block_name)
                    .map(|v| {
                        if v.len() == data_len {
                            v.clone()
                        } else {
                            nan_vec(data_len)
                        }
                    })
                    .ok_or_else(|| {
                        FormulaError::RuntimeError(format!(
                            "BLOCKDATA: block '{}' not found",
                            block_name
                        ))
                    }),
                "AVG" | "AVGPRICE" => block_data
                    .avg_price
                    .get(&block_name)
                    .map(|v| {
                        if v.len() == data_len {
                            v.clone()
                        } else {
                            nan_vec(data_len)
                        }
                    })
                    .ok_or_else(|| {
                        FormulaError::RuntimeError(format!(
                            "BLOCKDATA: block '{}' not found",
                            block_name
                        ))
                    }),
                "PCT" | "PCTCHANGE" | "CHANGE" => block_data
                    .pct_change
                    .get(&block_name)
                    .map(|v| {
                        if v.len() == data_len {
                            v.clone()
                        } else {
                            nan_vec(data_len)
                        }
                    })
                    .ok_or_else(|| {
                        FormulaError::RuntimeError(format!(
                            "BLOCKDATA: block '{}' not found",
                            block_name
                        ))
                    }),
                "VOL" | "VOLUME" | "V" => block_data
                    .volume
                    .get(&block_name)
                    .map(|v| {
                        if v.len() == data_len {
                            v.clone()
                        } else {
                            nan_vec(data_len)
                        }
                    })
                    .ok_or_else(|| {
                        FormulaError::RuntimeError(format!(
                            "BLOCKDATA: block '{}' not found",
                            block_name
                        ))
                    }),
                "AMOUNT" | "A" => block_data
                    .amount
                    .get(&block_name)
                    .map(|v| {
                        if v.len() == data_len {
                            v.clone()
                        } else {
                            nan_vec(data_len)
                        }
                    })
                    .ok_or_else(|| {
                        FormulaError::RuntimeError(format!(
                            "BLOCKDATA: block '{}' not found",
                            block_name
                        ))
                    }),
                _ => block_data
                    .custom_fields
                    .get(&block_name)
                    .and_then(|fields| fields.get(&field_upper))
                    .map(|v| {
                        if v.len() == data_len {
                            v.clone()
                        } else {
                            nan_vec(data_len)
                        }
                    })
                    .ok_or_else(|| {
                        FormulaError::RuntimeError(format!(
                            "BLOCKDATA: field '{}' not found for block '{}'",
                            field, block_name
                        ))
                    }),
            }
        }
        None => Ok(nan_vec(data_len)),
    }
}

fn fn_blockindex(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BLOCKINDEX", args, 1)?;
    let block_name = get_string_from_hash(ctx, args[0][0]).ok_or_else(|| {
        FormulaError::InvalidParameter("BLOCKINDEX: block name must be a valid string".to_string())
    })?;

    let data_len = ctx.data_len;
    match &ctx.block_data {
        Some(block_data) => block_data
            .index_close
            .get(&block_name)
            .map(|v| {
                if v.len() == data_len {
                    v.clone()
                } else {
                    nan_vec(data_len)
                }
            })
            .ok_or_else(|| {
                FormulaError::RuntimeError(format!("BLOCKINDEX: block '{}' not found", block_name))
            }),
        None => Ok(nan_vec(data_len)),
    }
}

fn fn_blockavg(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BLOCKAVG", args, 1)?;
    let block_name = get_string_from_hash(ctx, args[0][0]).ok_or_else(|| {
        FormulaError::InvalidParameter("BLOCKAVG: block name must be a valid string".to_string())
    })?;

    let data_len = ctx.data_len;
    match &ctx.block_data {
        Some(block_data) => block_data
            .avg_price
            .get(&block_name)
            .map(|v| {
                if v.len() == data_len {
                    v.clone()
                } else {
                    nan_vec(data_len)
                }
            })
            .ok_or_else(|| {
                FormulaError::RuntimeError(format!("BLOCKAVG: block '{}' not found", block_name))
            }),
        None => Ok(nan_vec(data_len)),
    }
}

// ======================== DZH MONEY FLOW FUNCTIONS ========================

fn fn_moneyflow(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let data_len = ctx.data_len;
    match &ctx.money_flow_data {
        Some(mf) => {
            if mf.money_flow.len() == data_len {
                Ok(mf.money_flow.clone())
            } else {
                Ok(nan_vec(data_len))
            }
        }
        None => Ok(nan_vec(data_len)),
    }
}

fn fn_netinflow(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let data_len = ctx.data_len;
    let level = if !args.is_empty() && !args[0].is_empty() {
        args[0][0] as i32
    } else {
        0
    };

    match &ctx.money_flow_data {
        Some(mf) => {
            let source = match level {
                0 => &mf.main_inflow,
                1 => &mf.super_big_inflow,
                2 => &mf.big_inflow,
                3 => &mf.medium_inflow,
                4 => &mf.small_inflow,
                _ => &mf.main_inflow,
            };
            if source.len() == data_len {
                Ok(source.clone())
            } else {
                Ok(nan_vec(data_len))
            }
        }
        None => Ok(nan_vec(data_len)),
    }
}

fn fn_bigorder(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let data_len = ctx.data_len;
    match &ctx.money_flow_data {
        Some(mf) => {
            if mf.big_order_pct.len() == data_len {
                Ok(mf.big_order_pct.clone())
            } else {
                Ok(nan_vec(data_len))
            }
        }
        None => Ok(nan_vec(data_len)),
    }
}

fn fn_smallorder(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let data_len = ctx.data_len;
    match &ctx.money_flow_data {
        Some(mf) => {
            if mf.small_order_pct.len() == data_len {
                Ok(mf.small_order_pct.clone())
            } else {
                Ok(nan_vec(data_len))
            }
        }
        None => Ok(nan_vec(data_len)),
    }
}

fn fn_maininflow(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let data_len = ctx.data_len;
    match &ctx.money_flow_data {
        Some(mf) => {
            if mf.main_inflow.len() == data_len {
                Ok(mf.main_inflow.clone())
            } else {
                Ok(nan_vec(data_len))
            }
        }
        None => Ok(nan_vec(data_len)),
    }
}

fn fn_maininflowpct(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let data_len = ctx.data_len;
    match &ctx.money_flow_data {
        Some(mf) => {
            if mf.main_inflow_pct.len() == data_len {
                Ok(mf.main_inflow_pct.clone())
            } else {
                Ok(nan_vec(data_len))
            }
        }
        None => Ok(nan_vec(data_len)),
    }
}

fn fn_superbigorder(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let data_len = ctx.data_len;
    match &ctx.money_flow_data {
        Some(mf) => {
            if mf.super_big_inflow.len() == data_len {
                Ok(mf.super_big_inflow.clone())
            } else {
                Ok(nan_vec(data_len))
            }
        }
        None => Ok(nan_vec(data_len)),
    }
}

// ======================== TDX ALIASES ========================

fn fn_pdi(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    fn_plus_di(ctx, args)
}

fn fn_mdi(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    fn_minus_di(ctx, args)
}

fn fn_mtm(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    fn_mom(ctx, args)
}

// THS-specific aliases
fn fn_close1(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let c = ctx.close.as_slice().unwrap();
    for i in 1..len {
        out[i] = c[i - 1];
    }
    Ok(out)
}

fn fn_open1(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let o = ctx.open.as_slice().unwrap();
    for i in 1..len {
        out[i] = o[i - 1];
    }
    Ok(out)
}

fn fn_valuewhen(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("VALUEWHEN", args, 2)?;
    let cond = &args[0];
    let x = &args[1];
    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);
    let mut last_value = f64::NAN;

    for i in 0..data_len {
        if cond[i] > 0.0 && !cond[i].is_nan() {
            last_value = x[i];
        }
        result[i] = last_value;
    }

    Ok(result)
}

fn fn_last(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LAST", args, 3)?;
    let cond = &args[0];
    let a = args[1][0] as usize;
    let b = args[2][0] as usize;
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    if a < b {
        return Err(FormulaError::InvalidParameter(
            "LAST: A must be >= B".to_string(),
        ));
    }

    for i in 0..data_len {
        if i < a {
            continue;
        }
        let start = i - a;
        let end = i - b;
        let mut all_true = true;
        for j in start..=end {
            if cond[j] <= 0.0 || cond[j].is_nan() {
                all_true = false;
                break;
            }
        }
        result[i] = if all_true { 1.0 } else { 0.0 };
    }

    Ok(result)
}

fn fn_barslastcount(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BARSLASTCOUNT", args, 1)?;
    let cond = &args[0];
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    for i in 0..data_len {
        if cond[i] > 0.0 && !cond[i].is_nan() {
            if i > 0 {
                result[i] = result[i - 1] + 1.0;
            } else {
                result[i] = 1.0;
            }
        }
    }

    Ok(result)
}

fn compute_zigzag_pivots(
    ctx: &FormulaContext,
    threshold_pct: f64,
) -> (Vec<(usize, f64, bool)>, usize) {
    let high = ctx.high.as_slice().unwrap();
    let low = ctx.low.as_slice().unwrap();
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

fn fn_peak(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

    for i in 0..data_len {
        let mut count = 0usize;
        for &(pidx, pval) in peaks.iter().rev() {
            if pidx < i {
                count += 1;
                if count == m {
                    result[i] = pval;
                    break;
                }
            }
        }
    }

    Ok(result)
}

fn fn_trough(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

    for i in 0..data_len {
        let mut count = 0usize;
        for &(pidx, pval) in troughs.iter().rev() {
            if pidx < i {
                count += 1;
                if count == m {
                    result[i] = pval;
                    break;
                }
            }
        }
    }

    Ok(result)
}

fn fn_peakbars(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

    for i in 0..data_len {
        let mut count = 0usize;
        for &pidx in peaks.iter().rev() {
            if pidx < i {
                count += 1;
                if count == m {
                    result[i] = (i - pidx) as f64;
                    break;
                }
            }
        }
    }

    Ok(result)
}

fn fn_troughbars(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

    for i in 0..data_len {
        let mut count = 0usize;
        for &pidx in troughs.iter().rev() {
            if pidx < i {
                count += 1;
                if count == m {
                    result[i] = (i - pidx) as f64;
                    break;
                }
            }
        }
    }

    Ok(result)
}

fn fn_zigzag(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ZIGZAG", args, 2)?;
    let n = extract_f64_arg(args, 1, "ZIGZAG")?;
    let high = ctx.high.as_slice().unwrap();
    let low = ctx.low.as_slice().unwrap();

    match lib_zigzag(high, low, n) {
        Ok(result) => Ok(result.zigzag),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

fn fn_findhigh(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

fn fn_findlow(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

fn fn_topn(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TOPN", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "TOPN")?;
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    let mut indexed: Vec<(usize, f64)> = input
        .iter()
        .enumerate()
        .filter(|(_, v)| !v.is_nan())
        .map(|(i, &v)| (i, v))
        .collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    for &(idx, _) in indexed.iter().take(n) {
        result[idx] = 1.0;
    }

    Ok(result)
}

fn fn_drawnull(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    Ok(nan_vec(ctx.data_len))
}

fn fn_ceiling(_ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CEILING", args, 2)?;
    let input = &args[0];
    let precision = args[1][0];
    let len = input.len();
    let mut result = Array1::zeros(len);

    if precision.abs() < f64::EPSILON {
        for i in 0..len {
            result[i] = input[i].ceil();
        }
    } else {
        for i in 0..len {
            if input[i].is_nan() {
                result[i] = f64::NAN;
            } else {
                result[i] = (input[i] / precision).ceil() * precision;
            }
        }
    }

    Ok(result)
}

// === Signal filtering functions (文华财经 compat) ===

/// AUTOFILTER: returns a constant 1.0 series (marks formula as auto-filter mode).
/// Actual filtering logic happens at the evaluation layer.
fn fn_autofilter(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    Ok(Array1::ones(ctx.data_len))
}

/// CHECKSIG(BuyCond, SellCond, N): Signal confirmation.
/// N=1: signal confirmed on same bar. N=0: signal confirmed on next bar.
/// Returns filtered buy signal (1.0 = buy, -1.0 = sell, 0 = no signal).
/// Consecutive same-direction signals are suppressed.
fn fn_checksig(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CHECKSIG", args, 3)?;
    let buy_cond = &args[0];
    let sell_cond = &args[1];
    let confirm_mode = args[2][0] as i32;
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    let mut last_signal = 0i32; // 0=none, 1=buy, -1=sell

    for i in 0..data_len {
        let buy = buy_cond[i] != 0.0 && !buy_cond[i].is_nan();
        let sell = sell_cond[i] != 0.0 && !sell_cond[i].is_nan();

        if buy && last_signal != 1 {
            if confirm_mode == 1 {
                result[i] = 1.0;
            } else if i + 1 < data_len {
                result[i + 1] = 1.0;
            }
            last_signal = 1;
        } else if sell && last_signal != -1 {
            if confirm_mode == 1 {
                result[i] = -1.0;
            } else if i + 1 < data_len {
                result[i + 1] = -1.0;
            }
            last_signal = -1;
        }
    }

    Ok(result)
}

/// MULTSIG(BuyCond, SellCond, N, M): Multi-signal mode.
/// Allows up to M signals within N bars of the same direction.
fn fn_multsig(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MULTSIG", args, 4)?;
    let buy_cond = &args[0];
    let sell_cond = &args[1];
    let n = args[2][0] as usize;
    let m = args[3][0] as usize;
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    let mut last_signal = 0i32;
    let mut same_dir_count = 0usize;
    let mut last_signal_bar = 0usize;

    for i in 0..data_len {
        let buy = buy_cond[i] != 0.0 && !buy_cond[i].is_nan();
        let sell = sell_cond[i] != 0.0 && !sell_cond[i].is_nan();

        if buy {
            if last_signal != 1 {
                result[i] = 1.0;
                last_signal = 1;
                same_dir_count = 1;
                last_signal_bar = i;
            } else if same_dir_count < m && (n == 0 || i - last_signal_bar <= n) {
                result[i] = 1.0;
                same_dir_count += 1;
                last_signal_bar = i;
            }
        } else if sell {
            if last_signal != -1 {
                result[i] = -1.0;
                last_signal = -1;
                same_dir_count = 1;
                last_signal_bar = i;
            } else if same_dir_count < m && (n == 0 || i - last_signal_bar <= n) {
                result[i] = -1.0;
                same_dir_count += 1;
                last_signal_bar = i;
            }
        }
    }

    Ok(result)
}

/// ENTERLONG: Alias for buy signal marker (returns input as-is or 1.0 series)
fn fn_enterlong(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    if args.is_empty() {
        Ok(Array1::ones(ctx.data_len))
    } else {
        Ok(args[0].clone())
    }
}

/// EXITLONG: Alias for sell signal marker
fn fn_exitlong(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    if args.is_empty() {
        Ok(Array1::ones(ctx.data_len))
    } else {
        Ok(args[0].clone())
    }
}

/// ENTERSHORT: Alias for short entry signal marker
fn fn_entershort(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    if args.is_empty() {
        Ok(Array1::ones(ctx.data_len))
    } else {
        Ok(args[0].clone())
    }
}

/// EXITSHORT: Alias for short exit signal marker
fn fn_exitshort(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    if args.is_empty() {
        Ok(Array1::ones(ctx.data_len))
    } else {
        Ok(args[0].clone())
    }
}

// === Cumulative / Sequence operations ===

/// CUMSUM / CUM: Cumulative sum
fn fn_cumsum(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CUMSUM", args, 1)?;
    let input = &args[0];
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);
    let mut sum = 0.0;
    for i in 0..data_len {
        if !input[i].is_nan() {
            sum += input[i];
        }
        result[i] = sum;
    }
    Ok(result)
}

/// CUMMAX: Cumulative maximum
fn fn_cummax(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CUMMAX", args, 1)?;
    let input = &args[0];
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);
    let mut max = f64::NEG_INFINITY;
    for i in 0..data_len {
        if !input[i].is_nan() && input[i] > max {
            max = input[i];
        }
        result[i] = if max == f64::NEG_INFINITY {
            f64::NAN
        } else {
            max
        };
    }
    Ok(result)
}

/// CUMMIN: Cumulative minimum
fn fn_cummin(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CUMMIN", args, 1)?;
    let input = &args[0];
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);
    let mut min = f64::INFINITY;
    for i in 0..data_len {
        if !input[i].is_nan() && input[i] < min {
            min = input[i];
        }
        result[i] = if min == f64::INFINITY { f64::NAN } else { min };
    }
    Ok(result)
}

/// PERCENTILE(X, N, P): P-th percentile over N-bar window
fn fn_percentile(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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
fn fn_median(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MEDIAN", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "MEDIAN")?;
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
        let mid = window.len() / 2;
        result[i] = if window.len().is_multiple_of(2) {
            (window[mid - 1] + window[mid]) / 2.0
        } else {
            window[mid]
        };
    }
    Ok(result)
}

// === Higher-order statistics ===

/// SKEW(X, N): Rolling skewness over N-bar window
fn fn_skew(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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
fn fn_kurt(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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
fn fn_mode(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MODE", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "MODE")?;
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
        if window.is_empty() {
            continue;
        }
        let mut counts: HashMap<i64, (usize, f64)> = HashMap::new();
        for &v in &window {
            let key = (v * 100.0).round() as i64;
            let entry = counts.entry(key).or_insert((0, v));
            entry.0 += 1;
        }
        let mode_val = counts
            .values()
            .max_by_key(|(c, _)| *c)
            .map(|(_, v)| *v)
            .unwrap_or(f64::NAN);
        result[i] = mode_val;
    }
    Ok(result)
}

/// SORT(X, N, DIR): Sort the last N values. DIR=1 ascending, DIR=0 descending.
/// Returns the sorted rank position (1-based) of current value within its window.
fn fn_sort(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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
fn fn_rank(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

// === Multi-period / cross-timeframe functions ===

/// PERIODTYPE(): Returns current period type (0=daily, 1=weekly, 2=monthly, 3=minute)
fn fn_periodtype(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    Ok(Array1::from_elem(ctx.data_len, ctx.period_type as f64))
}

/// REFDATE(X, DATE): Reference value of X at a specific date (bar index).
/// If DATE is an index, returns constant series of X[DATE].
fn fn_refdate(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("REFDATE", args, 2)?;
    let input = &args[0];
    let date_idx = args[1][0] as usize;
    let data_len = ctx.data_len;
    let val = if date_idx < data_len {
        input[date_idx]
    } else {
        f64::NAN
    };
    Ok(Array1::from_elem(data_len, val))
}

pub fn get_builtin_functions() -> HashMap<String, FormulaFn> {
    let mut map: HashMap<String, FormulaFn> = HashMap::new();

    map.insert("MA".to_string(), fn_ma);
    map.insert("EMA".to_string(), fn_ema);
    map.insert("SMA".to_string(), fn_sma);
    map.insert("WMA".to_string(), fn_wma);
    map.insert("DMA".to_string(), fn_dma);
    map.insert("DEMA".to_string(), fn_dema);
    map.insert("TEMA".to_string(), fn_tema);
    map.insert("KAMA".to_string(), fn_kama);
    map.insert("T3".to_string(), fn_t3);
    map.insert("TRIMA".to_string(), fn_trima);
    map.insert("MAVP".to_string(), fn_mavp);
    map.insert("SAREXT".to_string(), fn_sarext);

    map.insert("HHV".to_string(), fn_hhv);
    map.insert("LLV".to_string(), fn_llv);
    map.insert("HHVBARS".to_string(), fn_hhvbars);
    map.insert("LLVBARS".to_string(), fn_llvbars);

    map.insert("REF".to_string(), fn_ref);
    map.insert("CROSS".to_string(), fn_cross);
    map.insert("CROSSBELOW".to_string(), fn_crossbelow);
    map.insert("LONGCROSS".to_string(), fn_longcross);
    map.insert("IF".to_string(), fn_if);
    map.insert("IFTHEN".to_string(), fn_ifthen);
    map.insert("COUNT".to_string(), fn_count);
    map.insert("SUM".to_string(), fn_sum);
    map.insert("MINUS".to_string(), fn_minus);
    map.insert("EVERY".to_string(), fn_every);
    map.insert("EXIST".to_string(), fn_exist);
    map.insert("FILTER".to_string(), fn_filter);
    map.insert("BARSLAST".to_string(), fn_barslast);
    map.insert("BACKSET".to_string(), fn_backset);
    map.insert("BETWEEN".to_string(), fn_between);
    map.insert("NOT".to_string(), fn_not);

    map.insert("ABS".to_string(), fn_abs);
    map.insert("MAX".to_string(), fn_max);
    map.insert("MIN".to_string(), fn_min);
    map.insert("MAXINDEX".to_string(), fn_maxindex);
    map.insert("MININDEX".to_string(), fn_minindex);
    map.insert("SQRT".to_string(), fn_sqrt);
    map.insert("POW".to_string(), fn_pow);
    map.insert("ADD".to_string(), fn_add);
    map.insert("SUB".to_string(), fn_sub);
    map.insert("MULT".to_string(), fn_mult);
    map.insert("DIV".to_string(), fn_div);
    map.insert("EXP".to_string(), fn_exp);
    map.insert("LOG".to_string(), fn_log);
    map.insert("LN".to_string(), fn_log);
    map.insert("LOG10".to_string(), fn_log10);
    map.insert("SIGN".to_string(), fn_sign);
    map.insert("FLOOR".to_string(), fn_floor);
    map.insert("CEIL".to_string(), fn_ceil);
    map.insert("ROUND".to_string(), fn_round);
    map.insert("SIN".to_string(), fn_sin);
    map.insert("COS".to_string(), fn_cos);
    map.insert("TAN".to_string(), fn_tan);
    map.insert("SINH".to_string(), fn_sinh);
    map.insert("COSH".to_string(), fn_cosh);
    map.insert("TANH".to_string(), fn_tanh);
    map.insert("ASIN".to_string(), fn_asin);
    map.insert("ACOS".to_string(), fn_acos);
    map.insert("ATAN".to_string(), fn_atan);

    map.insert("STD".to_string(), fn_std);
    map.insert("STDDEV".to_string(), fn_std);
    map.insert("VAR".to_string(), fn_var);
    map.insert("ZSCORE".to_string(), fn_zscore);
    map.insert("CORREL".to_string(), fn_correl);
    map.insert("BETA".to_string(), fn_beta);
    map.insert("LINEAR_REG".to_string(), fn_linear_reg);
    map.insert("TSF".to_string(), fn_tsf);
    map.insert("PERCENT_RANK".to_string(), fn_percent_rank);
    map.insert("MIDPOINT".to_string(), fn_midpoint);
    map.insert("MIDPRICE".to_string(), fn_midprice);

    map.insert("RSI".to_string(), fn_rsi);
    map.insert("MACD".to_string(), fn_macd);
    map.insert("DIFF".to_string(), fn_diff);
    map.insert("DEA".to_string(), fn_dea);
    map.insert("BOLL".to_string(), fn_boll);
    map.insert("BOLLUP".to_string(), fn_bollup);
    map.insert("BOLLDN".to_string(), fn_bolldn);
    map.insert("BOLLMID".to_string(), fn_bollmid);
    map.insert("BOLLWIDTH".to_string(), fn_bollwidth);
    map.insert("BBANDS".to_string(), fn_boll);

    map.insert("ATR".to_string(), fn_atr);
    map.insert("NATR".to_string(), fn_natr);
    map.insert("TRANGE".to_string(), fn_trange);

    map.insert("AVGPRICE".to_string(), fn_avgprice);
    map.insert("MEDPRICE".to_string(), fn_medprice);
    map.insert("TYPPRICE".to_string(), fn_typprice);
    map.insert("WCLPRICE".to_string(), fn_wclprice);

    map.insert("OBV".to_string(), fn_obv);
    map.insert("AD".to_string(), fn_ad);
    map.insert("ADOSC".to_string(), fn_adosc);
    map.insert("MFI".to_string(), fn_mfi);

    // Pine-compat helpers used by the formula engine's Pine Script mapper.
    map.insert("MATH_AVG".to_string(), fn_math_avg);
    map.insert("ISNA".to_string(), fn_isna);
    map.insert("VWMA".to_string(), fn_vwma_indicator);

    map.insert("CCI".to_string(), fn_cci);
    map.insert("WILLR".to_string(), fn_willr);
    map.insert("WR".to_string(), fn_willr);
    map.insert("MOM".to_string(), fn_mom);
    map.insert("ROC".to_string(), fn_roc);
    map.insert("CMO".to_string(), fn_cmo);
    map.insert("TRIX".to_string(), fn_trix);
    map.insert("BOP".to_string(), fn_bop);
    map.insert("APO".to_string(), fn_apo);
    map.insert("PPO".to_string(), fn_ppo);
    map.insert("DPO".to_string(), fn_dpo);

    map.insert("ADX".to_string(), fn_adx);
    map.insert("ADXR".to_string(), fn_adxr);
    map.insert("DMI".to_string(), fn_dmi);
    map.insert("DX".to_string(), fn_dx);
    map.insert("PLUS_DI".to_string(), fn_plus_di);
    map.insert("MINUS_DI".to_string(), fn_minus_di);
    map.insert("AROONOSC".to_string(), fn_aroonosc);
    map.insert("AROON_UP".to_string(), fn_aroon_up);
    map.insert("AROON_DN".to_string(), fn_aroon_dn);
    map.insert("SAR".to_string(), fn_sar);
    map.insert("PSAR".to_string(), fn_sar);

    map.insert("STOCH".to_string(), fn_stoch);
    map.insert("KDJ".to_string(), fn_kdj);
    map.insert("KD".to_string(), fn_kdj);
    map.insert("KDJ_K".to_string(), fn_kdj);
    map.insert("KDJ_D".to_string(), fn_kdj_d);
    map.insert("KDJ_J".to_string(), fn_kdj_j);
    map.insert("BIAS".to_string(), fn_bias);
    map.insert("PSY".to_string(), fn_psy);
    map.insert("HMA".to_string(), fn_hma);
    map.insert("ALMA".to_string(), fn_alma);
    map.insert("CMF".to_string(), fn_cmf);
    map.insert("FISHER".to_string(), fn_fisher);
    map.insert("FISHER_SIGNAL".to_string(), fn_fisher_signal);
    map.insert("TSI".to_string(), fn_tsi);
    map.insert("CHOP".to_string(), fn_chop);

    map.insert("ICHIMOKU_TENKAN".to_string(), fn_ichimoku_tenkan);
    map.insert("ICHIMOKU_KIJUN".to_string(), fn_ichimoku_kijun);
    map.insert("SUPERTREND".to_string(), fn_supertrend);
    map.insert("VWAP".to_string(), fn_vwap);

    map.insert("DONCHIAN".to_string(), fn_donchian);
    map.insert("DONCHIAN_UPPER".to_string(), fn_donchian_upper);
    map.insert("DONCHIAN_LOWER".to_string(), fn_donchian_lower);
    map.insert("DONCHIAN_MIDDLE".to_string(), fn_donchian_middle);
    map.insert("DONCHIAN_WIDTH".to_string(), fn_donchian_width);

    map.insert("STRCAT".to_string(), fn_strcat);
    map.insert("HISTVOL".to_string(), fn_histvol);
    map.insert("OBV_ENHANCED".to_string(), fn_obv_enhanced);
    map.insert("ATR_ENHANCED".to_string(), fn_atr_enhanced);
    map.insert("BOLL_ENHANCED".to_string(), fn_boll_enhanced);

    // Time / Bar functions (TDX)
    map.insert("DATE".to_string(), fn_date_tdx);
    map.insert("TIME".to_string(), fn_time);
    map.insert("YEAR".to_string(), fn_year);
    map.insert("MONTH".to_string(), fn_month);
    map.insert("DAY".to_string(), fn_day);
    map.insert("HOUR".to_string(), fn_hour);
    map.insert("MINUTE".to_string(), fn_minute);
    map.insert("WEEKDAY".to_string(), fn_weekday);
    map.insert("CURRBARSCOUNT".to_string(), fn_currbarscount);
    map.insert("TOTALBARSCOUNT".to_string(), fn_totalbarscount);
    map.insert("BARSSINCE".to_string(), fn_barssince);
    map.insert("BARSSINCEN".to_string(), fn_barssincen);
    map.insert("BARSCOUNT".to_string(), fn_barscount);
    map.insert("BARSTATUS".to_string(), fn_barstatus);
    map.insert("ISLASTBAR".to_string(), fn_islastbar);
    map.insert("FROMOPEN".to_string(), fn_fromopen);

    // Math / Statistics extensions (TDX)
    map.insert("AVEDEV".to_string(), fn_avedev);
    map.insert("AVGDEV".to_string(), fn_avgdev);
    map.insert("DEVSQ".to_string(), fn_devsq);
    map.insert("SLOPE".to_string(), fn_slope);
    map.insert("FORCAST".to_string(), fn_forcast);
    map.insert("RANGE".to_string(), fn_range);
    map.insert("CONST".to_string(), fn_const_val);
    map.insert("SUMBARS".to_string(), fn_sumbars);
    map.insert("INTPART".to_string(), fn_intpart);
    map.insert("FRACPART".to_string(), fn_fracpart);
    map.insert("MOD".to_string(), fn_mod);
    map.insert("REVERSE".to_string(), fn_reverse);
    map.insert("TR".to_string(), fn_tr);

    // Index / Finance / Chip data (TDX)
    map.insert("INDEXC".to_string(), fn_indexc);
    map.insert("INDEXO".to_string(), fn_indexo);
    map.insert("INDEXH".to_string(), fn_indexh);
    map.insert("INDEXL".to_string(), fn_indexl);
    map.insert("INDEXV".to_string(), fn_indexv);
    map.insert("INDEXA".to_string(), fn_indexa);
    map.insert("CAPITAL".to_string(), fn_capital);
    map.insert("FINANCE".to_string(), fn_finance);
    map.insert("DYNAINFO".to_string(), fn_dynainfo);
    map.insert("WINNER".to_string(), fn_winner);
    map.insert("LWINNER".to_string(), fn_lwinner);
    map.insert("COST".to_string(), fn_cost);

    // DZH Block functions (大智慧板块引用)
    map.insert("BLOCKDATA".to_string(), fn_blockdata as FormulaFn);
    map.insert("BLOCKINDEX".to_string(), fn_blockindex as FormulaFn);
    map.insert("BLOCKAVG".to_string(), fn_blockavg as FormulaFn);

    // DZH Money flow functions (大智慧资金流向)
    map.insert("MONEYFLOW".to_string(), fn_moneyflow as FormulaFn);
    map.insert("NETINFLOW".to_string(), fn_netinflow as FormulaFn);
    map.insert("BIGORDER".to_string(), fn_bigorder as FormulaFn);
    map.insert("SMALLORDER".to_string(), fn_smallorder as FormulaFn);
    map.insert("MAININFLOW".to_string(), fn_maininflow as FormulaFn);
    map.insert("MAININFLOWPCT".to_string(), fn_maininflowpct as FormulaFn);
    map.insert("SUPERBIGORDER".to_string(), fn_superbigorder as FormulaFn);

    // TDX Aliases
    map.insert("PDI".to_string(), fn_pdi);
    map.insert("MDI".to_string(), fn_mdi);
    map.insert("MTM".to_string(), fn_mtm);

    // THS (同花顺) Aliases
    map.insert("CLOSE1".to_string(), fn_close1);
    map.insert("OPEN1".to_string(), fn_open1);

    // Core reference/condition functions
    map.insert("VALUEWHEN".to_string(), fn_valuewhen);
    map.insert("LAST".to_string(), fn_last);
    map.insert("BARSLASTCOUNT".to_string(), fn_barslastcount);

    // ZigZag series functions
    map.insert("PEAK".to_string(), fn_peak as FormulaFn);
    map.insert("TROUGH".to_string(), fn_trough as FormulaFn);
    map.insert("PEAKBARS".to_string(), fn_peakbars as FormulaFn);
    map.insert("TROUGHBARS".to_string(), fn_troughbars as FormulaFn);
    map.insert("ZIGZAG".to_string(), fn_zigzag as FormulaFn);

    // Advanced find functions
    map.insert("FINDHIGH".to_string(), fn_findhigh as FormulaFn);
    map.insert("FINDLOW".to_string(), fn_findlow as FormulaFn);
    map.insert("TOPN".to_string(), fn_topn as FormulaFn);
    map.insert("DRAWNULL".to_string(), fn_drawnull as FormulaFn);
    map.insert("CEILING".to_string(), fn_ceiling as FormulaFn);

    // Signal filtering functions (文华财经 compat)
    map.insert("AUTOFILTER".to_string(), fn_autofilter as FormulaFn);
    map.insert("CHECKSIG".to_string(), fn_checksig as FormulaFn);
    map.insert("MULTSIG".to_string(), fn_multsig as FormulaFn);
    map.insert("ENTERLONG".to_string(), fn_enterlong as FormulaFn);
    map.insert("EXITLONG".to_string(), fn_exitlong as FormulaFn);
    map.insert("ENTERSHORT".to_string(), fn_entershort as FormulaFn);
    map.insert("EXITSHORT".to_string(), fn_exitshort as FormulaFn);
    map.insert("BUY".to_string(), fn_enterlong as FormulaFn);
    map.insert("SELL".to_string(), fn_exitlong as FormulaFn);

    // Cumulative / sequence operations
    map.insert("CUMSUM".to_string(), fn_cumsum as FormulaFn);
    map.insert("CUM".to_string(), fn_cumsum as FormulaFn);
    map.insert("CUMMAX".to_string(), fn_cummax as FormulaFn);
    map.insert("CUMMIN".to_string(), fn_cummin as FormulaFn);
    map.insert("PERCENTILE".to_string(), fn_percentile as FormulaFn);
    map.insert("MEDIAN".to_string(), fn_median as FormulaFn);

    // Higher-order statistics
    map.insert("SKEW".to_string(), fn_skew as FormulaFn);
    map.insert("KURT".to_string(), fn_kurt as FormulaFn);
    map.insert("MODE".to_string(), fn_mode as FormulaFn);
    map.insert("SORT".to_string(), fn_sort as FormulaFn);
    map.insert("RANK".to_string(), fn_rank as FormulaFn);

    // Multi-period functions
    map.insert("PERIODTYPE".to_string(), fn_periodtype as FormulaFn);
    map.insert("REFDATE".to_string(), fn_refdate as FormulaFn);

    // THS (同花顺) Smart Selection Functions
    map.insert("SMARTSELECT".to_string(), fn_smartselect as FormulaFn);
    map.insert("SELECTCOND".to_string(), fn_selectcond as FormulaFn);

    // THS Alert Functions
    map.insert("ALERT".to_string(), fn_alert as FormulaFn);
    map.insert("ALERTONCE".to_string(), fn_alertonce as FormulaFn);

    // THS Statistical Functions
    map.insert("AVGPRICE_N".to_string(), fn_avgprice_n as FormulaFn);
    map.insert("TOTALVOL".to_string(), fn_totalvol as FormulaFn);
    map.insert("MAXPRICE".to_string(), fn_maxprice as FormulaFn);
    map.insert("MINPRICE".to_string(), fn_minprice as FormulaFn);

    // THS Additional Aliases (already implemented as CLOSE1/OPEN1)
    map.insert("HIGH1".to_string(), fn_high1 as FormulaFn);
    map.insert("LOW1".to_string(), fn_low1 as FormulaFn);
    map.insert("VOL1".to_string(), fn_vol1 as FormulaFn);

    // EM (东方财富) Functions
    map.insert("DKCOL".to_string(), fn_dkcol as FormulaFn);
    map.insert("EM_CROSS".to_string(), fn_em_cross as FormulaFn);
    map.insert("EM_REF".to_string(), fn_em_ref as FormulaFn);
    map.insert("EM_ZIG".to_string(), fn_em_zig as FormulaFn);
    map.insert("EM_TROUGH".to_string(), fn_em_trough as FormulaFn);
    map.insert("EM_PEAK".to_string(), fn_em_peak as FormulaFn);
    map.insert("EM_TROUGHBARS".to_string(), fn_em_troughbars as FormulaFn);
    map.insert("EM_PEAKBARS".to_string(), fn_em_peakbars as FormulaFn);
    map.insert("EM_COSTEX".to_string(), fn_em_costex as FormulaFn);
    map.insert("EM_ZLCCV".to_string(), fn_em_zlccv as FormulaFn);

    // FoxTrader (飞狐交易师) compatibility functions
    map.insert("FOX_ZIG".to_string(), fn_fox_zig as FormulaFn);
    map.insert("FOX_TROUGH".to_string(), fn_fox_trough as FormulaFn);
    map.insert("FOX_PEAK".to_string(), fn_fox_peak as FormulaFn);
    map.insert("FOX_TROUGHBARS".to_string(), fn_fox_troughbars as FormulaFn);
    map.insert("FOX_PEAKBARS".to_string(), fn_fox_peakbars as FormulaFn);
    map.insert("FOX_BUY".to_string(), fn_fox_buy as FormulaFn);
    map.insert("FOX_SELL".to_string(), fn_fox_sell as FormulaFn);
    map.insert(
        "FOX_TRADE_SIGNAL".to_string(),
        fn_fox_trade_signal as FormulaFn,
    );
    map.insert("FOX_BACKTEST".to_string(), fn_fox_backtest as FormulaFn);
    map.insert(
        "FOX_PROFIT_RATIO".to_string(),
        fn_fox_profit_ratio as FormulaFn,
    );
    map.insert("FOX_WIN_RATE".to_string(), fn_fox_win_rate as FormulaFn);
    map.insert(
        "FOX_MAX_DRAWDOWN".to_string(),
        fn_fox_max_drawdown as FormulaFn,
    );
    map.insert(
        "FOX_TRADE_COUNT".to_string(),
        fn_fox_trade_count as FormulaFn,
    );

    // TA-Lib C compatibility — additional momentum indicators
    map.insert("STOCHF".to_string(), fn_stochf);
    map.insert("STOCHRSI".to_string(), fn_stochrsi);
    map.insert("ULTOSC".to_string(), fn_ultosc);
    map.insert("PLUS_DM".to_string(), fn_plus_dm);
    map.insert("MINUS_DM".to_string(), fn_minus_dm);

    // TA-Lib C compatibility — Hilbert Transform cycle indicators
    map.insert("HT_PHASOR".to_string(), fn_ht_phasor_inner as FormulaFn);
    map.insert("HT_SINE".to_string(), fn_ht_sine_inner as FormulaFn);
    map.insert("HT_DCPERIOD".to_string(), fn_ht_dcperiod as FormulaFn);
    map.insert("HT_DCPHASE".to_string(), fn_ht_dcphase as FormulaFn);
    map.insert("HT_TRENDMODE".to_string(), fn_ht_trendmode as FormulaFn);
    map.insert("HT_TRENDLINE".to_string(), fn_ht_trendline as FormulaFn);
    map.insert("HT_MEASUREMENT".to_string(), fn_ht_measurement as FormulaFn);

    // TA-Lib C compatibility — additional momentum / statistics
    map.insert("MACDEXT".to_string(), fn_macdext);
    map.insert("PR".to_string(), fn_percent_rank);

    // Classic stock-trading chart patterns (FTA-native, not in TA-Lib)
    map.insert("DARVAS_BOX".to_string(), fn_darvas_box_top as FormulaFn);
    map.insert("RENKO".to_string(), fn_renko as FormulaFn);
    map.insert("KAGI".to_string(), fn_kagi as FormulaFn);
    map.insert(
        "POINT_AND_FIGURE".to_string(),
        fn_point_and_figure as FormulaFn,
    );
    map.insert(
        "THREE_LINE_BREAK".to_string(),
        fn_three_line_break as FormulaFn,
    );
    map.insert(
        "WILLIAMS_ALLIGATOR".to_string(),
        fn_williams_alligator_lips as FormulaFn,
    );
    map.insert("HEIKIN_ASHI".to_string(), fn_heikin_ashi_close as FormulaFn);

    // A-share specific indicators
    map.insert(
        "MAIN_NET_INFLOW".to_string(),
        fn_main_net_inflow as FormulaFn,
    );
    map.insert("MONEY_FLOW".to_string(), fn_money_flow as FormulaFn);
    map.insert("LIMIT_UP".to_string(), fn_limit_up as FormulaFn);
    map.insert("LIMIT_DOWN".to_string(), fn_limit_down as FormulaFn);
    map.insert(
        "CONSECUTIVE_LIMIT".to_string(),
        fn_consecutive_limit as FormulaFn,
    );
    map.insert("TURNOVER".to_string(), fn_turnover as FormulaFn);
    map.insert("RS_RATIO".to_string(), fn_rs_ratio as FormulaFn);

    map
}

fn fn_high1(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let h = ctx.high.as_slice().unwrap();
    for i in 1..len {
        out[i] = h[i - 1];
    }
    Ok(out)
}

fn fn_low1(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let l = ctx.low.as_slice().unwrap();
    for i in 1..len {
        out[i] = l[i - 1];
    }
    Ok(out)
}

fn fn_vol1(ctx: &FormulaContext, _args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let v = ctx.volume.as_slice().unwrap();
    for i in 1..len {
        out[i] = v[i - 1];
    }
    Ok(out)
}

fn fn_smartselect(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SMARTSELECT", args, 2)?;
    let cond = &args[0];
    let mode = args[1][0] as u8;
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    match mode {
        0 => {
            for i in 0..data_len {
                if cond[i] > 0.0 && !cond[i].is_nan() {
                    result[i] = 1.0;
                }
            }
        }
        1 => {
            let mut last_signal = false;
            for i in 0..data_len {
                if cond[i] > 0.0 && !cond[i].is_nan() && !last_signal {
                    result[i] = 1.0;
                    last_signal = true;
                } else if cond[i] <= 0.0 || cond[i].is_nan() {
                    last_signal = false;
                }
            }
        }
        2 => {
            let mut count = 0usize;
            for i in 0..data_len {
                if cond[i] > 0.0 && !cond[i].is_nan() {
                    count += 1;
                    if count == 1 {
                        result[i] = 1.0;
                    }
                } else {
                    count = 0;
                }
            }
        }
        _ => {
            for i in 0..data_len {
                if cond[i] > 0.0 && !cond[i].is_nan() {
                    result[i] = 1.0;
                }
            }
        }
    }

    Ok(result)
}

fn fn_selectcond(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SELECTCOND", args, 1)?;
    let cond = &args[0];
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    for i in 0..data_len {
        if cond[i] > 0.0 && !cond[i].is_nan() {
            result[i] = 1.0;
        }
    }

    Ok(result)
}

fn fn_alert(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ALERT", args, 2)?;
    let cond = &args[0];
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    for i in 0..data_len {
        if cond[i] > 0.0 && !cond[i].is_nan() {
            result[i] = 1.0;
        }
    }

    Ok(result)
}

fn fn_alertonce(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ALERTONCE", args, 2)?;
    let cond = &args[0];
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);
    let mut triggered = false;

    for i in 0..data_len {
        if cond[i] > 0.0 && !cond[i].is_nan() && !triggered {
            result[i] = 1.0;
            triggered = true;
        }
    }

    Ok(result)
}

fn fn_avgprice_n(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("AVGPRICE_N", args, 1)?;
    let n = extract_n(args, 0, "AVGPRICE_N")?;
    let data_len = ctx.data_len;

    let mut tp = Array1::zeros(data_len);
    for i in 0..data_len {
        tp[i] = (ctx.high[i] + ctx.low[i] + ctx.close[i]) / 3.0;
    }

    let values = tp.as_slice().unwrap();
    match lib_ma::sma(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_totalvol(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TOTALVOL", args, 1)?;
    let n = extract_n(args, 0, "TOTALVOL")?;
    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    let vol = ctx.volume.as_slice().unwrap();
    for i in (n - 1)..data_len {
        let window_start = (i + 1).saturating_sub(n);
        let sum: f64 = (window_start..=i).map(|j| vol[j]).sum();
        result[i] = sum;
    }

    Ok(result)
}

fn fn_maxprice(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MAXPRICE", args, 1)?;
    let n = extract_n(args, 0, "MAXPRICE")?;
    let data_len = ctx.data_len;

    let high = ctx.high.as_slice().unwrap();
    match lib_stat::rolling_max(high, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_minprice(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MINPRICE", args, 1)?;
    let n = extract_n(args, 0, "MINPRICE")?;
    let data_len = ctx.data_len;

    let low = ctx.low.as_slice().unwrap();
    match lib_stat::rolling_min(low, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

fn fn_fox_zig(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FOX_ZIG", args, 2)?;
    let n = extract_f64_arg(args, 1, "FOX_ZIG")?;
    let high = ctx.high.as_slice().unwrap();
    let low = ctx.low.as_slice().unwrap();

    match lib_zigzag(high, low, n) {
        Ok(result) => Ok(result.zigzag),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

fn fn_fox_trough(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

    for i in 0..data_len {
        let mut count = 0usize;
        for &(pidx, pval) in troughs.iter().rev() {
            if pidx < i {
                count += 1;
                if count == m {
                    result[i] = pval;
                    break;
                }
            }
        }
    }

    Ok(result)
}

fn fn_fox_peak(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
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

    for i in 0..data_len {
        let mut count = 0usize;
        for &(pidx, pval) in peaks.iter().rev() {
            if pidx < i {
                count += 1;
                if count == m {
                    result[i] = pval;
                    break;
                }
            }
        }
    }

    Ok(result)
}

fn fn_fox_troughbars(
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

    for i in 0..data_len {
        let mut count = 0usize;
        for &pidx in troughs.iter().rev() {
            if pidx < i {
                count += 1;
                if count == m {
                    result[i] = (i - pidx) as f64;
                    break;
                }
            }
        }
    }

    Ok(result)
}

fn fn_fox_peakbars(
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

    for i in 0..data_len {
        let mut count = 0usize;
        for &pidx in peaks.iter().rev() {
            if pidx < i {
                count += 1;
                if count == m {
                    result[i] = (i - pidx) as f64;
                    break;
                }
            }
        }
    }

    Ok(result)
}

fn fn_fox_buy(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FOX_BUY", args, 2)?;
    let cond = &args[0];
    let price = &args[1];
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    for i in 0..data_len {
        if cond[i] != 0.0 && !cond[i].is_nan() {
            result[i] = price[i];
        }
    }

    Ok(result)
}

fn fn_fox_sell(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FOX_SELL", args, 2)?;
    let cond = &args[0];
    let price = &args[1];
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    for i in 0..data_len {
        if cond[i] != 0.0 && !cond[i].is_nan() {
            result[i] = -price[i];
        }
    }

    Ok(result)
}

fn fn_fox_trade_signal(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FOX_TRADE_SIGNAL", args, 2)?;
    let buy_cond = &args[0];
    let sell_cond = &args[1];
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    let mut last_signal = 0i32;

    for i in 0..data_len {
        let buy = buy_cond[i] != 0.0 && !buy_cond[i].is_nan();
        let sell = sell_cond[i] != 0.0 && !sell_cond[i].is_nan();

        if buy && last_signal != 1 {
            result[i] = 1.0;
            last_signal = 1;
        } else if sell && last_signal != -1 {
            result[i] = -1.0;
            last_signal = -1;
        }
    }

    Ok(result)
}

fn fn_fox_backtest(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FOX_BACKTEST", args, 3)?;
    let buy_cond = &args[0];
    let sell_cond = &args[1];
    let price = &args[2];
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    let mut holding = false;
    let mut entry_price = 0.0f64;
    let mut cum_pnl = 0.0f64;

    for i in 0..data_len {
        let buy = buy_cond[i] != 0.0 && !buy_cond[i].is_nan();
        let sell = sell_cond[i] != 0.0 && !sell_cond[i].is_nan();

        if buy && !holding {
            holding = true;
            entry_price = price[i];
        } else if sell && holding {
            if !entry_price.is_nan() && !price[i].is_nan() && entry_price != 0.0 {
                cum_pnl += (price[i] - entry_price) / entry_price;
            }
            holding = false;
            entry_price = 0.0;
        }

        result[i] = cum_pnl;
    }

    Ok(result)
}

fn fn_fox_profit_ratio(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FOX_PROFIT_RATIO", args, 3)?;
    let buy_cond = &args[0];
    let sell_cond = &args[1];
    let price = &args[2];
    let data_len = ctx.data_len;

    let mut holding = false;
    let mut entry_price = 0.0f64;
    let mut total_profit = 0.0f64;
    let mut total_loss = 0.0f64;

    for i in 0..data_len {
        let buy = buy_cond[i] != 0.0 && !buy_cond[i].is_nan();
        let sell = sell_cond[i] != 0.0 && !sell_cond[i].is_nan();

        if buy && !holding {
            holding = true;
            entry_price = price[i];
        } else if sell && holding {
            if !entry_price.is_nan() && !price[i].is_nan() && entry_price != 0.0 {
                let pnl = (price[i] - entry_price) / entry_price;
                if pnl > 0.0 {
                    total_profit += pnl;
                } else {
                    total_loss += pnl.abs();
                }
            }
            holding = false;
            entry_price = 0.0;
        }
    }

    let ratio = if total_loss > 0.0 {
        total_profit / total_loss
    } else if total_profit > 0.0 {
        f64::INFINITY
    } else {
        0.0
    };

    Ok(Array1::from_elem(data_len, ratio))
}

fn fn_fox_win_rate(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FOX_WIN_RATE", args, 3)?;
    let buy_cond = &args[0];
    let sell_cond = &args[1];
    let price = &args[2];
    let data_len = ctx.data_len;

    let mut holding = false;
    let mut entry_price = 0.0f64;
    let mut wins = 0usize;
    let mut total = 0usize;

    for i in 0..data_len {
        let buy = buy_cond[i] != 0.0 && !buy_cond[i].is_nan();
        let sell = sell_cond[i] != 0.0 && !sell_cond[i].is_nan();

        if buy && !holding {
            holding = true;
            entry_price = price[i];
        } else if sell && holding {
            if !entry_price.is_nan() && !price[i].is_nan() && entry_price != 0.0 {
                let pnl = (price[i] - entry_price) / entry_price;
                total += 1;
                if pnl > 0.0 {
                    wins += 1;
                }
            }
            holding = false;
            entry_price = 0.0;
        }
    }

    let rate = if total > 0 {
        wins as f64 / total as f64
    } else {
        0.0
    };

    Ok(Array1::from_elem(data_len, rate))
}

fn fn_fox_max_drawdown(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FOX_MAX_DRAWDOWN", args, 3)?;
    let buy_cond = &args[0];
    let sell_cond = &args[1];
    let price = &args[2];
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    let mut holding = false;
    let mut entry_price = 0.0f64;
    let mut cum_pnl = 0.0f64;
    let mut peak_equity = 0.0f64;
    let mut max_dd = 0.0f64;

    for i in 0..data_len {
        let buy = buy_cond[i] != 0.0 && !buy_cond[i].is_nan();
        let sell = sell_cond[i] != 0.0 && !sell_cond[i].is_nan();

        if buy && !holding {
            holding = true;
            entry_price = price[i];
        } else if sell && holding {
            if !entry_price.is_nan() && !price[i].is_nan() && entry_price != 0.0 {
                cum_pnl += (price[i] - entry_price) / entry_price;
            }
            holding = false;
            entry_price = 0.0;
        }

        if cum_pnl > peak_equity {
            peak_equity = cum_pnl;
        }
        let dd = peak_equity - cum_pnl;
        if dd > max_dd {
            max_dd = dd;
        }
        result[i] = max_dd;
    }

    Ok(result)
}

fn fn_fox_trade_count(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FOX_TRADE_COUNT", args, 2)?;
    let buy_cond = &args[0];
    let sell_cond = &args[1];
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    let mut holding = false;
    let mut count = 0usize;

    for i in 0..data_len {
        let buy = buy_cond[i] != 0.0 && !buy_cond[i].is_nan();
        let sell = sell_cond[i] != 0.0 && !sell_cond[i].is_nan();

        if buy && !holding {
            holding = true;
        } else if sell && holding {
            count += 1;
            holding = false;
        }

        result[i] = count as f64;
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// Tests for TA-Lib compatible Math Transform & Math Operator functions
// ---------------------------------------------------------------------------

#[cfg(test)]
mod talib_compat_tests {
    use super::*;
    use crate::formula::engine::FormulaEngine;

    fn make_ctx(len: usize) -> FormulaContext {
        let open = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.1).collect());
        let high = Array1::from_vec((0..len).map(|i| 11.0 + i as f64 * 0.2).collect());
        let low = Array1::from_vec((0..len).map(|i| 9.0 + i as f64 * 0.1).collect());
        let close = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.15).collect());
        let volume = Array1::from_vec((0..len).map(|i| 1000.0 + i as f64 * 10.0).collect());
        FormulaContext::new(open, high, low, close, volume, None)
    }

    fn build_args(arrays: Vec<Array1<f64>>) -> Vec<Array1<f64>> {
        arrays
    }

    // ---------------- Hyperbolic: SINH / COSH / TANH ----------------

    #[test]
    fn test_fn_sinh() {
        let ctx = make_ctx(3);
        let args = build_args(vec![Array1::from_vec(vec![0.0, 1.0, -1.0])]);
        let r = fn_sinh(&ctx, &args).unwrap();
        assert!((r[0] - 0.0).abs() < 1e-12);
        assert!((r[1] - 1.0_f64.sinh()).abs() < 1e-12);
        assert!((r[2] - (-1.0_f64).sinh()).abs() < 1e-12);
    }

    #[test]
    fn test_fn_cosh() {
        let ctx = make_ctx(3);
        let args = build_args(vec![Array1::from_vec(vec![0.0, 1.0, 2.0])]);
        let r = fn_cosh(&ctx, &args).unwrap();
        assert!((r[0] - 1.0).abs() < 1e-12);
        assert!((r[1] - 1.0_f64.cosh()).abs() < 1e-12);
        assert!((r[2] - 2.0_f64.cosh()).abs() < 1e-12);
    }

    #[test]
    fn test_fn_tanh() {
        let ctx = make_ctx(3);
        let args = build_args(vec![Array1::from_vec(vec![0.0, 1.0, -1.0])]);
        let r = fn_tanh(&ctx, &args).unwrap();
        assert!((r[0] - 0.0).abs() < 1e-12);
        assert!((r[1] - 1.0_f64.tanh()).abs() < 1e-12);
        assert!((r[2] - (-1.0_f64).tanh()).abs() < 1e-12);
    }

    // ---------------- Arithmetic two-input: ADD / SUB / MULT / DIV ----------------

    #[test]
    fn test_fn_add() {
        let ctx = make_ctx(3);
        let args = build_args(vec![
            Array1::from_vec(vec![1.0, 2.0, 3.0]),
            Array1::from_vec(vec![10.0, 20.0, 30.0]),
        ]);
        let r = fn_add(&ctx, &args).unwrap();
        assert_eq!(r.to_vec(), vec![11.0, 22.0, 33.0]);
    }

    #[test]
    fn test_fn_sub() {
        let ctx = make_ctx(3);
        let args = build_args(vec![
            Array1::from_vec(vec![10.0, 20.0, 30.0]),
            Array1::from_vec(vec![1.0, 5.0, 100.0]),
        ]);
        let r = fn_sub(&ctx, &args).unwrap();
        assert_eq!(r.to_vec(), vec![9.0, 15.0, -70.0]);
    }

    #[test]
    fn test_fn_mult() {
        let ctx = make_ctx(3);
        let args = build_args(vec![
            Array1::from_vec(vec![2.0, 3.0, 4.0]),
            Array1::from_vec(vec![5.0, 6.0, 7.0]),
        ]);
        let r = fn_mult(&ctx, &args).unwrap();
        assert_eq!(r.to_vec(), vec![10.0, 18.0, 28.0]);
    }

    #[test]
    fn test_fn_div() {
        let ctx = make_ctx(4);
        let args = build_args(vec![
            Array1::from_vec(vec![10.0, 20.0, 30.0, 5.0]),
            Array1::from_vec(vec![2.0, 4.0, 5.0, 0.0]),
        ]);
        let r = fn_div(&ctx, &args).unwrap();
        assert!((r[0] - 5.0).abs() < 1e-12);
        assert!((r[1] - 5.0).abs() < 1e-12);
        assert!((r[2] - 6.0).abs() < 1e-12);
        assert!(r[3].is_nan());
    }

    // ---------------- MINUS (period difference) ----------------

    #[test]
    fn test_fn_minus_basic() {
        let ctx = make_ctx(5);
        let data: Array1<f64> = Array1::from_vec(vec![1.0, 2.0, 4.0, 7.0, 11.0]);
        let n: Array1<f64> = Array1::from_vec(vec![2.0]);
        let r = fn_minus(&ctx, &[data, n]).unwrap();
        assert!(r[0].is_nan() && r[1].is_nan());
        assert!((r[2] - 3.0).abs() < 1e-12);
        assert!((r[3] - 5.0).abs() < 1e-12);
        assert!((r[4] - 7.0).abs() < 1e-12);
        // result length should match ctx.data_len
        assert_eq!(r.len(), ctx.data_len);
    }

    #[test]
    fn test_fn_minus_period_one() {
        let ctx = make_ctx(4);
        let data = Array1::from_vec(vec![1.0, 3.0, 6.0, 10.0]);
        let n = Array1::from_vec(vec![1.0]);
        let r = fn_minus(&ctx, &[data, n]).unwrap();
        assert!(r[0].is_nan());
        assert!((r[1] - 2.0).abs() < 1e-12);
        assert!((r[2] - 3.0).abs() < 1e-12);
        assert!((r[3] - 4.0).abs() < 1e-12);
    }

    // ---------------- MAXINDEX / MININDEX ----------------

    #[test]
    fn test_fn_maxindex() {
        let ctx = make_ctx(5);
        let data = Array1::from_vec(vec![3.0, 1.0, 4.0, 1.0, 5.0]);
        let n = Array1::from_vec(vec![3.0]);
        let r = fn_maxindex(&ctx, &[data, n]).unwrap();
        assert!(r[0].is_nan() && r[1].is_nan());
        // [3,1,4] max=4 at offset 2
        assert!((r[2] - 2.0).abs() < 1e-12);
        // [1,4,1] max=4 at offset 1
        assert!((r[3] - 1.0).abs() < 1e-12);
        // [4,1,5] max=5 at offset 2
        assert!((r[4] - 2.0).abs() < 1e-12);
        assert_eq!(r.len(), ctx.data_len);
    }

    #[test]
    fn test_fn_minindex() {
        let ctx = make_ctx(5);
        let data = Array1::from_vec(vec![3.0, 1.0, 4.0, 1.0, 5.0]);
        let n = Array1::from_vec(vec![3.0]);
        let r = fn_minindex(&ctx, &[data, n]).unwrap();
        assert!(r[0].is_nan() && r[1].is_nan());
        // [3,1,4] min=1 at offset 1
        assert!((r[2] - 1.0).abs() < 1e-12);
        // [1,4,1] min=1 at offset 0
        assert!((r[3] - 0.0).abs() < 1e-12);
        // [4,1,5] min=1 at offset 1
        assert!((r[4] - 1.0).abs() < 1e-12);
        assert_eq!(r.len(), ctx.data_len);
    }

    // ---------------- get_builtin_functions registration ----------------

    #[test]
    fn test_get_builtin_functions_contains_new_funcs() {
        let funcs = get_builtin_functions();
        for name in &[
            "SINH", "COSH", "TANH", "ADD", "SUB", "MULT", "DIV", "MINUS", "MAXINDEX", "MININDEX",
        ] {
            assert!(
                funcs.contains_key(*name),
                "missing function in registry: {}",
                name
            );
        }
    }

    // ---------------- Integration tests via FormulaEngine ----------------

    #[test]
    fn test_engine_sqrt_close() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval("SQRT(CLOSE)", &mut ctx).unwrap();
        for i in 0..5 {
            let expected = (10.0 + i as f64 * 0.15).sqrt();
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_engine_max_element_wise() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval("MAX(CLOSE, 20)", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            assert!((result[i] - close_val.max(20.0)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_engine_sum_vol_window() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(20);
        let result = engine.eval("SUM(VOL, 10)", &mut ctx).unwrap();
        // 前 9 个应为 NaN（窗口不足）
        for i in 0..9 {
            assert!(result[i].is_nan(), "expected NaN at index {}", i);
        }
        // 验证第 9 个及之后
        for i in 9..20 {
            let window_start = i + 1 - 10;
            let mut expected = 0.0_f64;
            for k in window_start..=i {
                expected += 1000.0 + k as f64 * 10.0;
            }
            assert!(
                (result[i] - expected).abs() < 1e-9,
                "mismatch at {}: {} vs {}",
                i,
                result[i],
                expected
            );
        }
    }

    #[test]
    fn test_engine_min_element_wise() {
        // MIN(CLOSE, 11) 在公式引擎中是 element-wise 版本（min(close, 11)）
        // 实际窗口版 MIN 见 indicators::math_operators::min
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval("MIN(CLOSE, 11)", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            assert!((result[i] - close_val.min(11.0)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_engine_sinh_cosh_tanh() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(3);
        let r_sinh = engine.eval("SINH(CLOSE)", &mut ctx).unwrap();
        let r_cosh = engine.eval("COSH(CLOSE)", &mut ctx).unwrap();
        let r_tanh = engine.eval("TANH(CLOSE)", &mut ctx).unwrap();
        for i in 0..3 {
            let c = 10.0 + i as f64 * 0.15;
            assert!((r_sinh[i] - c.sinh()).abs() < 1e-9);
            assert!((r_cosh[i] - c.cosh()).abs() < 1e-9);
            assert!((r_tanh[i] - c.tanh()).abs() < 1e-9);
        }
    }

    #[test]
    fn test_engine_add_sub_mult_div() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let r_add = engine.eval("ADD(CLOSE, OPEN)", &mut ctx).unwrap();
        let r_sub = engine.eval("SUB(CLOSE, OPEN)", &mut ctx).unwrap();
        let r_mult = engine.eval("MULT(CLOSE, 2)", &mut ctx).unwrap();
        let r_div = engine.eval("DIV(CLOSE, 2)", &mut ctx).unwrap();
        for i in 0..5 {
            let c = 10.0 + i as f64 * 0.15;
            let o = 10.0 + i as f64 * 0.1;
            assert!((r_add[i] - (c + o)).abs() < 1e-10);
            assert!((r_sub[i] - (c - o)).abs() < 1e-10);
            assert!((r_mult[i] - (c * 2.0)).abs() < 1e-10);
            assert!((r_div[i] - (c / 2.0)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_engine_minus() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval("MINUS(CLOSE, 2)", &mut ctx).unwrap();
        // 前 2 个为 NaN
        for i in 0..2 {
            assert!(result[i].is_nan());
        }
        for i in 2..5 {
            let cur = 10.0 + i as f64 * 0.15;
            let prev = 10.0 + (i - 2) as f64 * 0.15;
            let expected = cur - prev;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_engine_maxindex_minindex() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let r_max = engine.eval("MAXINDEX(CLOSE, 3)", &mut ctx).unwrap();
        let r_min = engine.eval("MININDEX(CLOSE, 3)", &mut ctx).unwrap();
        // 前 2 个为 NaN
        for i in 0..2 {
            assert!(r_max[i].is_nan());
            assert!(r_min[i].is_nan());
        }
        // close 是单调递增的，max 永远在最后 (offset 2)，min 永远在起点 (offset 0)
        for i in 2..5 {
            assert!((r_max[i] - 2.0).abs() < 1e-10);
            assert!((r_min[i] - 0.0).abs() < 1e-10);
        }
    }
}
