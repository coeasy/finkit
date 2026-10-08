//! Linear-regression / correlation / beta formula shims.

use super::prelude::*;

pub(crate) fn fn_linear_reg(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_linear_reg_angle(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LINEARREG_ANGLE", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "LINEARREG_ANGLE")?;
    match lib_linear::linreg_angle(input.as_slice().unwrap(), n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

pub(crate) fn fn_linear_reg_intercept(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LINEARREG_INTERCEPT", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "LINEARREG_INTERCEPT")?;
    match lib_linear::linreg_intercept(input.as_slice().unwrap(), n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

pub(crate) fn fn_linear_reg_slope(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LINEARREG_SLOPE", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "LINEARREG_SLOPE")?;
    match lib_linear::linreg_slope(input.as_slice().unwrap(), n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

pub(crate) fn fn_tsf(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_correl(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CORREL", args, 3)?;
    let x = &args[0];
    let y = &args[1];
    let n = extract_n(args, 2, "CORREL")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);
    if x.len() != data_len || y.len() != data_len {
        return Ok(result);
    }
    if rolling_correlation_into(
        x.as_slice().unwrap(),
        y.as_slice().unwrap(),
        n,
        result.as_slice_mut().unwrap(),
    )
    .is_err()
    {
        return Ok(nan_vec(data_len));
    }

    Ok(result)
}

pub(crate) fn fn_beta(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BETA", args, 3)?;
    let x = &args[0];
    let y = &args[1];
    let n = extract_n(args, 2, "BETA")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);
    if x.len() != data_len || y.len() != data_len {
        return Ok(result);
    }
    if rolling_beta_into(
        x.as_slice().unwrap(),
        y.as_slice().unwrap(),
        n,
        result.as_slice_mut().unwrap(),
    )
    .is_err()
    {
        return Ok(nan_vec(data_len));
    }

    Ok(result)
}
