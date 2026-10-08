//! Price-transform formula shims (HLC/OHLC averages, Heikin-Ashi).

use super::prelude::*;

pub(crate) fn fn_avgprice(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_medprice(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_typprice(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_wclprice(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_heikin_ashi_close(
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

pub(crate) fn fn_avgprice_n(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_maxprice(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MAXPRICE", args, 1)?;
    let n = extract_n(args, 0, "MAXPRICE")?;
    let data_len = ctx.data_len;

    let high = ctx.high.as_slice();
    match lib_stat::rolling_max(high, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_minprice(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MINPRICE", args, 1)?;
    let n = extract_n(args, 0, "MINPRICE")?;
    let data_len = ctx.data_len;

    let low = ctx.low.as_slice();
    match lib_stat::rolling_min(low, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}
