//! Volatility-formula shims: ATR/NATR/TRANGE, Bollinger family.

use super::prelude::*;

pub(crate) fn fn_std(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_var(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_boll(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_bollup(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    fn_boll(ctx, args)
}

pub(crate) fn fn_bolldn(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_bollmid(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_bollwidth(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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
        if !ma_vals[i].is_nan()
            && !std_vals[i].is_nan()
            && ma_vals[i].abs() > crate::utils::NUMERIC_EPSILON
        {
            result[i] = 2.0 * nbdev * std_vals[i] / ma_vals[i] * 100.0;
        }
    }

    Ok(result)
}

pub(crate) fn fn_atr(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_natr(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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
        if !atr_vals[i].is_nan() && close[i].abs() > crate::utils::NUMERIC_EPSILON {
            result[i] = atr_vals[i] / close[i] * 100.0;
        }
    }

    Ok(result)
}

pub(crate) fn fn_trange(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_atr_enhanced(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    fn_atr(ctx, args)
}

pub(crate) fn fn_boll_enhanced(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    fn_boll(ctx, args)
}

pub(crate) fn fn_tr(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    // Delegates to the indicator layer rather than re-implementing the loop, so
    // the tree path and the `CALL:TR` kernel cannot drift apart. Note the DZH
    // bar-0 convention (`high - low`): this is *not* `TRANGE`, whose bar 0 is
    // `NaN`.
    let mut out = nan_vec(ctx.data_len);
    let output = out.as_slice_mut().expect("owned Array1 is contiguous");
    crate::indicators::volatility::trange_dzh_into(
        ctx.high.as_slice(),
        ctx.low.as_slice(),
        ctx.close.as_slice(),
        output,
    )
    .map_err(|_| {
        FormulaError::InvalidParameter("TR: high/low/close must span the series".to_string())
    })?;
    Ok(out)
}
