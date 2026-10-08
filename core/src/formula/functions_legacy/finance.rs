//! Fundamental / financial-data passthrough functions.

use super::prelude::*;

// ======================== INDEX / FINANCE / CHIP FUNCTIONS ========================

pub(crate) fn fn_indexc(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    match &ctx.index_data {
        Some(idx) => Ok(idx.close.clone().unwrap_or_else(|| nan_vec(ctx.data_len))),
        None => Ok(nan_vec(ctx.data_len)),
    }
}

pub(crate) fn fn_indexo(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    match &ctx.index_data {
        Some(idx) => Ok(idx.open.clone().unwrap_or_else(|| nan_vec(ctx.data_len))),
        None => Ok(nan_vec(ctx.data_len)),
    }
}

pub(crate) fn fn_indexh(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    match &ctx.index_data {
        Some(idx) => Ok(idx.high.clone().unwrap_or_else(|| nan_vec(ctx.data_len))),
        None => Ok(nan_vec(ctx.data_len)),
    }
}

pub(crate) fn fn_indexl(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    match &ctx.index_data {
        Some(idx) => Ok(idx.low.clone().unwrap_or_else(|| nan_vec(ctx.data_len))),
        None => Ok(nan_vec(ctx.data_len)),
    }
}

pub(crate) fn fn_indexv(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    match &ctx.index_data {
        Some(idx) => Ok(idx.volume.clone().unwrap_or_else(|| nan_vec(ctx.data_len))),
        None => Ok(nan_vec(ctx.data_len)),
    }
}

pub(crate) fn fn_indexa(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    match &ctx.index_data {
        Some(idx) => Ok(idx.amount.clone().unwrap_or_else(|| nan_vec(ctx.data_len))),
        None => Ok(nan_vec(ctx.data_len)),
    }
}

pub(crate) fn fn_capital(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let val = ctx.capital.unwrap_or(f64::NAN);
    Ok(Array1::from_elem(ctx.data_len, val))
}

pub(crate) fn fn_finance(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FINANCE", args, 1)?;
    let field_id = args[0][0] as usize;
    let val = ctx
        .finance_data
        .as_ref()
        .and_then(|fd| fd.fields.get(&field_id).copied())
        .unwrap_or(f64::NAN);
    Ok(Array1::from_elem(ctx.data_len, val))
}

pub(crate) fn fn_dynainfo(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_winner(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_lwinner(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_cost(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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
