//! Volume and money-flow formula shims.

use super::prelude::*;

pub(crate) fn fn_obv(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_ad(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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
        if hl_diff.abs() > crate::utils::NUMERIC_EPSILON {
            let clv = ((close[i] - low[i]) - (high[i] - close[i])) / hl_diff;
            result[i] = if i > 0 { result[i - 1] } else { 0.0 } + clv * volume[i];
        } else {
            result[i] = if i > 0 { result[i - 1] } else { 0.0 };
        }
    }
    Ok(result)
}

pub(crate) fn fn_adosc(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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
        if hl_diff.abs() > crate::utils::NUMERIC_EPSILON {
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

pub(crate) fn fn_cmf(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_vwap(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let (price, volume) = match args.len() {
        2 => (args[0].clone(), &args[1]),
        len if len >= 4 => {
            let high = &args[0];
            let low = &args[1];
            let close = &args[2];
            let data_len = high.len().min(low.len()).min(close.len());
            let mut typical = Array1::zeros(data_len);
            for i in 0..data_len {
                typical[i] = (high[i] + low[i] + close[i]) / 3.0;
            }
            (typical, &args[3])
        }
        _ => {
            return Err(FormulaError::InvalidParameter(format!(
                "VWAP requires (source, volume) or (high, low, close, volume), got {} arguments",
                args.len()
            )))
        }
    };

    let len = price.len().min(volume.len());
    let mut result = Array1::zeros(len);
    let mut cum_price_volume = 0.0f64;
    let mut cum_volume = 0.0f64;
    for i in 0..len {
        cum_price_volume += price[i] * volume[i];
        cum_volume += volume[i];
        result[i] = if cum_volume.abs() > crate::utils::NUMERIC_EPSILON {
            cum_price_volume / cum_volume
        } else {
            f64::NAN
        };
    }
    Ok(result)
}

pub(crate) fn fn_obv_enhanced(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    fn_obv(ctx, args)
}

// ======================== DZH MONEY FLOW FUNCTIONS ========================

pub(crate) fn fn_moneyflow(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_netinflow(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_bigorder(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_smallorder(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_maininflow(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_maininflowpct(
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

pub(crate) fn fn_superbigorder(
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

pub(crate) fn fn_totalvol(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TOTALVOL", args, 1)?;
    let n = extract_n(args, 0, "TOTALVOL")?;
    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    let vol = ctx.volume.as_slice();

    // Sliding window sum, O(1) per bar. Missing bars are excluded from the
    // running sum and tracked in `missing`, so only windows that actually
    // contain one report NaN — the per-window rescan's rule.
    let mut sum = 0.0f64;
    let mut missing = 0usize;
    for i in 0..data_len {
        if vol[i].is_nan() {
            missing += 1;
        } else {
            sum += vol[i];
        }
        if i >= n {
            if vol[i - n].is_nan() {
                missing -= 1;
            } else {
                sum -= vol[i - n];
            }
        }
        if i + 1 >= n && missing == 0 {
            result[i] = sum;
        }
    }

    Ok(result)
}
