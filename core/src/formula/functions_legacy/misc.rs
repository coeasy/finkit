//! Formula functions that do not belong to any of the named families.

use super::prelude::*;

pub(crate) fn fn_strcat(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("STRCAT", args, 2)?;
    Err(FormulaError::InvalidOperation(
        "String concatenation (&) is not supported for numeric values. Use STRCAT() function instead.".to_string()
    ))
}

pub(crate) fn fn_ifthen(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_not(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("NOT", args, 1)?;
    Ok(args[0]
        .mapv(|v| crate::formula::truth::logical_bool(!crate::formula::truth::is_logical_true(v))))
}

// THS-specific aliases
pub(crate) fn fn_close1(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let c = ctx.close.as_slice();
    for i in 1..len {
        out[i] = c[i - 1];
    }
    Ok(out)
}

pub(crate) fn fn_open1(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let o = ctx.open.as_slice();
    for i in 1..len {
        out[i] = o[i - 1];
    }
    Ok(out)
}

pub(crate) fn fn_high1(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let h = ctx.high.as_slice();
    for i in 1..len {
        out[i] = h[i - 1];
    }
    Ok(out)
}

pub(crate) fn fn_low1(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let l = ctx.low.as_slice();
    for i in 1..len {
        out[i] = l[i - 1];
    }
    Ok(out)
}

pub(crate) fn fn_vol1(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let v = ctx.volume.as_slice();
    for i in 1..len {
        out[i] = v[i - 1];
    }
    Ok(out)
}

pub(crate) fn fn_alert(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_alertonce(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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
