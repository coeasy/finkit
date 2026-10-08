//! Moving-average / overlap-study formula shims (delegate to `math::moving_avg`).

use super::prelude::*;

pub(crate) fn fn_ma(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_ema(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

/// RMA(X, N): Wilder's moving average, as used by Pine `ta.rma`.
/// The first value is seeded from the first N non-NaN observations; later
/// values use the Wilder recurrence `prev + (x - prev) / N`.
///
/// The recursion itself lives in `math::moving_avg::rma_into` so the tree path
/// and the compiled-plan `CALL:RMA` kernel share one implementation.
pub(crate) fn fn_rma(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("RMA", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "RMA")?;
    let mut output = nan_vec(ctx.data_len);
    crate::math::moving_avg::rma_into(
        input.as_slice().unwrap_or_default(),
        n,
        output.as_slice_mut().expect("owned Array1 is contiguous"),
    )
    .map_err(|_| FormulaError::InvalidParameter("RMA period is invalid".to_string()))?;
    Ok(output)
}

pub(crate) fn fn_sma(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    if args.len() < 2 || args.len() > 3 {
        return Err(FormulaError::InvalidParameter(format!(
            "SMA requires 2 or 3 arguments, got {}",
            args.len()
        )));
    }
    let input = &args[0];
    let n = extract_n(args, 1, "SMA")?;
    // Terminal SMA(X, N[, M]) is recursive smoothing. With two arguments, M defaults to 1; it is not the same algorithm as simple MA(X, N).
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
pub(crate) fn fn_math_avg(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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
pub(crate) fn fn_isna(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

/// `FIXNAN(x)` — forward-fill missing values, preserving leading NaN values.
/// This is the Pine `fixnan` contract and is intentionally distinct from a
/// zero-fill or unconditional carry-forward transform.
pub(crate) fn fn_fixnan(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FIXNAN", args, 1)?;
    let input = &args[0];
    let mut out = nan_vec(input.len());
    let mut previous = f64::NAN;
    for (index, &value) in input.iter().enumerate() {
        if !value.is_nan() {
            previous = value;
        }
        out[index] = previous;
    }
    Ok(out)
}

/// `VWMA(close, volume, n)` — volume-weighted moving average.
pub(crate) fn fn_vwma_indicator(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("VWMA", args, 3)?;
    let close = &args[0];
    let volume = &args[1];
    let n = extract_n(args, 2, "VWMA")?;
    let data_len = ctx.data_len;
    let mut out = nan_vec(data_len);

    // Two sliding sums (`Σ close*volume`, `Σ volume`) make this O(1) per bar.
    // A bar whose close or volume is missing keeps the sums finite by being
    // excluded and is tracked in `missing`, so any window that contained a
    // missing bar still reports NaN — exactly what the per-window rescan did.
    let mut pv_sum = 0.0f64;
    let mut v_sum = 0.0f64;
    let mut missing = 0usize;
    for i in 0..data_len {
        let (c, v) = (close[i], volume[i]);
        if c.is_nan() || v.is_nan() {
            missing += 1;
        } else {
            pv_sum += c * v;
            v_sum += v;
        }
        if i >= n {
            let (oc, ov) = (close[i - n], volume[i - n]);
            if oc.is_nan() || ov.is_nan() {
                missing -= 1;
            } else {
                pv_sum -= oc * ov;
                v_sum -= ov;
            }
        }
        if i + 1 >= n && missing == 0 && v_sum.abs() > crate::utils::NUMERIC_EPSILON {
            out[i] = pv_sum / v_sum;
        }
    }
    Ok(out)
}

pub(crate) fn fn_wma(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_dma(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_dema(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DEMA", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "DEMA")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    // See `functions::warmup_offset`: a leading non-finite run is an upstream
    // indicator's warm-up prefix, and `dema` seeds its EMA chain from `input[0]`.
    let start = crate::formula::functions::warmup_offset(values);
    match lib_ma::dema(&values[start..], n) {
        Ok(result) => Ok(crate::formula::functions::shift_back(
            result, start, data_len,
        )),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_tema(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TEMA", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "TEMA")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    // See `functions::warmup_offset`.
    let start = crate::formula::functions::warmup_offset(values);
    match lib_ma::tema(&values[start..], n) {
        Ok(result) => Ok(crate::formula::functions::shift_back(
            result, start, data_len,
        )),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_kama(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_t3(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_trima(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_mavp(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_sarext(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_midpoint(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MIDPOINT", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "MIDPOINT")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    let mut result = nan_vec(data_len);
    rolling_minmax_visit(values, values, n, |index, maximum, minimum| {
        if !maximum.is_nan() && !minimum.is_nan() {
            result[index] = (maximum + minimum) / 2.0;
        }
    });

    Ok(result)
}

pub(crate) fn fn_midprice(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MIDPRICE", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let n = extract_n(args, 2, "MIDPRICE")?;

    let data_len = ctx.data_len;
    let high_values = high.as_slice().unwrap();
    let low_values = low.as_slice().unwrap();
    let mut result = nan_vec(data_len);
    rolling_minmax_visit(high_values, low_values, n, |index, maximum, minimum| {
        if !maximum.is_nan() && !minimum.is_nan() {
            result[index] = (maximum + minimum) / 2.0;
        }
    });

    Ok(result)
}

pub(crate) fn fn_hma(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_alma(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_sar(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    if args.len() < 4 || args.len() > 5 {
        return Err(FormulaError::InvalidParameter(format!(
            "SAR requires 4 arguments (high, low, step, max) or 5 arguments (high, low, start, increment, max), got {}",
            args.len()
        )));
    }
    let high = &args[0];
    let low = &args[1];
    let af_start = extract_f64_arg(args, 2, "SAR")?;
    let (af_increment, af_max) = if args.len() == 5 {
        (
            extract_f64_arg(args, 3, "SAR")?,
            extract_f64_arg(args, 4, "SAR")?,
        )
    } else {
        (af_start, extract_f64_arg(args, 3, "SAR")?)
    };
    if !(af_start > 0.0 && af_increment > 0.0 && af_max >= af_start) {
        return Err(FormulaError::InvalidParameter(
            "SAR acceleration factors must satisfy start > 0, increment > 0, max >= start"
                .to_string(),
        ));
    }

    // The Wilder recursion lives in the indicator layer so the compiled-plan
    // kernel and this executor cannot drift apart.
    let data_len = high.len().min(low.len());
    let mut result = nan_vec(data_len);
    if crate::indicators::overlap::sar_with_factors_into(
        &high.as_slice().unwrap()[..data_len],
        &low.as_slice().unwrap()[..data_len],
        af_start,
        af_increment,
        af_max,
        result.as_slice_mut().unwrap(),
    )
    .is_err()
    {
        return Ok(nan_vec(data_len));
    }
    Ok(result)
}

pub(crate) fn fn_ichimoku_tenkan(
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

    // Fused high/low extrema kernel, O(1) per bar. The previous fold also
    // underflowed on `n == 0` (`(n - 1)` on usize); the kernel just leaves
    // the output missing.
    ichimoku_midpoint(high, low, n, &mut result);
    Ok(result)
}

pub(crate) fn fn_ichimoku_kijun(
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

    ichimoku_midpoint(high, low, n, &mut result);
    Ok(result)
}

/// `(HHV(high, n) + LLV(low, n)) / 2` via the shared extrema kernel.
pub(crate) fn ichimoku_midpoint(
    high: &Array1<f64>,
    low: &Array1<f64>,
    n: usize,
    result: &mut Array1<f64>,
) {
    if n == 0 || high.len() != low.len() {
        return;
    }
    rolling_minmax_visit(
        high.as_slice().unwrap(),
        low.as_slice().unwrap(),
        n,
        |index, highest, lowest| {
            result[index] = (highest + lowest) / 2.0;
        },
    );
}

#[allow(unused_assignments)]
pub(crate) fn fn_supertrend(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SUPERTREND", args, 4)?;
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
    let data_len = args[0].len().min(args[1].len()).min(args[2].len());

    // Delegated to the golden-pinned indicator instead of keeping a second
    // hand-rolled copy. The local version smoothed TR with a *simple* moving
    // average, while `indicators::supertrend::supertrend` uses Wilder ATR --
    // which is what `tests/golden/talib/supertrend.json` pins through
    // `golden_talib_all_indicators`. The two diverge from the very first bar
    // (SMA is defined at `atr_n - 1`; Wilder ATR needs its own seed), so this
    // is a "has a value at all" difference, not a rounding one.
    match crate::indicators::supertrend::supertrend(
        &args[0].as_slice().unwrap()[..data_len],
        &args[1].as_slice().unwrap()[..data_len],
        &args[2].as_slice().unwrap()[..data_len],
        atr_n,
        mult,
    ) {
        Ok(result) => Ok(result.trend_line),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_donchian(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DONCHIAN", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let n = extract_n(args, 2, "DONCHIAN")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    ichimoku_midpoint(high, low, n, &mut result);
    Ok(result)
}

pub(crate) fn fn_donchian_upper(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DONCHIAN_UPPER", args, 3)?;
    let high = &args[0];
    let _low = &args[1];
    let n = extract_n(args, 2, "DONCHIAN_UPPER")?;

    // Shared single-series kernel, O(1) per bar. The old fold reported `+inf`
    // for an all-missing window; the kernel reports NaN there, consistent
    // with the rest of the rolling-extrema family.
    match lib_stat::rolling_max(high.as_slice().unwrap(), n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

pub(crate) fn fn_donchian_lower(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DONCHIAN_LOWER", args, 3)?;
    let _high = &args[0];
    let low = &args[1];
    let n = extract_n(args, 2, "DONCHIAN_LOWER")?;

    match lib_stat::rolling_min(low.as_slice().unwrap(), n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

pub(crate) fn fn_donchian_middle(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    fn_donchian(ctx, args)
}

pub(crate) fn fn_donchian_width(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("DONCHIAN_WIDTH", args, 3)?;
    let high = &args[0];
    let low = &args[1];
    let n = extract_n(args, 2, "DONCHIAN_WIDTH")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    // Fused high/low extrema kernel, O(1) per bar.
    if n > 0 && high.len() == low.len() {
        rolling_minmax_visit(
            high.as_slice().unwrap(),
            low.as_slice().unwrap(),
            n,
            |index, highest, lowest| {
                result[index] = highest - lowest;
            },
        );
    }

    Ok(result)
}
