//! A-share specific formula shims (KDJ/BIAS/PSY and block data).

use super::prelude::*;

pub(crate) enum KdjLine {
    K,
    D,
    J,
}

pub(crate) fn extract_kdj_params(args: &[Array1<f64>]) -> (usize, usize, usize) {
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

pub(crate) fn fn_kdj_line(
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

pub(crate) fn fn_kdj(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    fn_kdj_line(ctx, args, KdjLine::K)
}

pub(crate) fn fn_kdj_d(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    fn_kdj_line(ctx, args, KdjLine::D)
}

pub(crate) fn fn_kdj_j(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    fn_kdj_line(ctx, args, KdjLine::J)
}

pub(crate) fn fn_bias(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_psy(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

// ======================== A-SHARE SPECIFIC INDICATORS ========================

pub(crate) fn fn_main_net_inflow(
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

pub(crate) fn fn_money_flow(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_limit_up(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_limit_down(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_consecutive_limit(
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

pub(crate) fn fn_turnover(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TURNOVER", args, 2)?;
    let vol = args[0].as_slice().unwrap();
    let free_float = args[1].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_astock::turnover(vol, free_float) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_rs_ratio(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

// ======================== DZH BLOCK FUNCTIONS ========================

pub(crate) fn fn_blockdata(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_blockindex(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_blockavg(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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
