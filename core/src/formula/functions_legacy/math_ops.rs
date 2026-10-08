//! Elementwise math operators exposed as formula functions.

use super::prelude::*;

pub(crate) fn fn_abs(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ABS", args, 1)?;
    Ok(args[0].mapv(|v| v.abs()))
}

pub(crate) fn fn_max(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_min(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_add(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_sub(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_mult(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_div(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_minus(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_maxindex(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MAXINDEX", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "MAXINDEX")?;

    let data_len = ctx.data_len;
    let mut output = nan_vec(data_len);

    // Monotonic deque: amortized O(1) per bar; see [`ArgExtremeDeque`]. The
    // reported index stays relative to the window start, as before. Every bar
    // is offered — bars before the first full window must still be tracked.
    let mut deque = ArgExtremeDeque::<true>::default();
    for i in 0..data_len {
        deque.offer(i, input[i]);
        if i + 1 < n {
            continue;
        }
        let window_start = i + 1 - n;
        deque.expire_before(window_start);
        if let Some(index) = deque.extreme_index() {
            output[i] = (index - window_start) as f64;
        }
    }

    Ok(output)
}

pub(crate) fn fn_minindex(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MININDEX", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "MININDEX")?;

    let data_len = ctx.data_len;
    let mut output = nan_vec(data_len);

    let mut deque = ArgExtremeDeque::<false>::default();
    for i in 0..data_len {
        deque.offer(i, input[i]);
        if i + 1 < n {
            continue;
        }
        let window_start = i + 1 - n;
        deque.expire_before(window_start);
        if let Some(index) = deque.extreme_index() {
            output[i] = (index - window_start) as f64;
        }
    }

    Ok(output)
}

// MINMAX and MINMAXINDEX are multi-output operations in the canonical
// Operation API. The scalar FormulaFn ABI has one return slot, so the formula
// fallback exposes the first (minimum) output while the typed operation
// dispatcher returns both named outputs.
pub(crate) fn fn_minmax(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MINMAX", args, 2)?;
    let period = extract_n(args, 1, "MINMAX")?;
    lib_math_operators::min(args[0].as_slice().unwrap(), period)
        .map(|values| values.to_owned())
        .map_err(|error| FormulaError::RuntimeError(error.to_string()))
        .or_else(|_| Ok(nan_vec(ctx.data_len)))
}

pub(crate) fn fn_minmaxindex(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MINMAXINDEX", args, 2)?;
    let period = extract_n(args, 1, "MINMAXINDEX")?;
    lib_math_operators::minindex(args[0].as_slice().unwrap(), period)
        .map(|values| values.mapv(|value| value as f64))
        .map_err(|error| FormulaError::RuntimeError(error.to_string()))
        .or_else(|_| Ok(nan_vec(ctx.data_len)))
}

pub(crate) fn fn_sqrt(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SQRT", args, 1)?;
    Ok(args[0].mapv(|v| if v < 0.0 { f64::NAN } else { v.sqrt() }))
}

pub(crate) fn fn_pow(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_exp(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("EXP", args, 1)?;
    Ok(args[0].mapv(|v| v.exp()))
}

pub(crate) fn fn_log(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LOG", args, 1)?;
    Ok(args[0].mapv(|v| if v <= 0.0 { f64::NAN } else { v.ln() }))
}

pub(crate) fn fn_log10(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LOG10", args, 1)?;
    Ok(args[0].mapv(|v| if v <= 0.0 { f64::NAN } else { v.log10() }))
}

pub(crate) fn fn_sign(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SIGN", args, 1)?;
    // Superseded in `get_builtin_functions` by `canonical_sign`, but kept
    // semantically identical: this table is parsed statically by the docs and
    // dialect-contract generators, and an unreachable-but-wrong body here is
    // exactly the kind of thing a later reader copies. Three-way, not
    // `signum` — see `math::three_way_sign`.
    Ok(args[0].mapv(crate::math::three_way_sign))
}

pub(crate) fn fn_floor(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FLOOR", args, 1)?;
    Ok(args[0].mapv(|v| v.floor()))
}

pub(crate) fn fn_ceil(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CEIL", args, 1)?;
    Ok(args[0].mapv(|v| v.ceil()))
}

pub(crate) fn fn_round(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ROUND", args, 1)?;
    Ok(args[0].mapv(|v| v.round()))
}

pub(crate) fn fn_sin(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SIN", args, 1)?;
    Ok(args[0].mapv(|v| v.sin()))
}

pub(crate) fn fn_cos(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("COS", args, 1)?;
    Ok(args[0].mapv(|v| v.cos()))
}

pub(crate) fn fn_tan(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TAN", args, 1)?;
    Ok(args[0].mapv(|v| v.tan()))
}

pub(crate) fn fn_sinh(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SINH", args, 1)?;
    Ok(args[0].mapv(|v| v.sinh()))
}

pub(crate) fn fn_cosh(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("COSH", args, 1)?;
    Ok(args[0].mapv(|v| v.cosh()))
}

pub(crate) fn fn_tanh(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TANH", args, 1)?;
    Ok(args[0].mapv(|v| v.tanh()))
}

pub(crate) fn fn_asin(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ASIN", args, 1)?;
    Ok(args[0].mapv(|v| v.asin()))
}

pub(crate) fn fn_acos(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ACOS", args, 1)?;
    Ok(args[0].mapv(|v| v.acos()))
}

pub(crate) fn fn_atan(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ATAN", args, 1)?;
    Ok(args[0].mapv(|v| v.atan()))
}

pub(crate) fn fn_const_val(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CONST", args, 1)?;
    let last_val = args[0][ctx.data_len.saturating_sub(1)];
    Ok(Array1::from_elem(ctx.data_len, last_val))
}

pub(crate) fn fn_intpart(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("INTPART", args, 1)?;
    Ok(args[0].mapv(|x| x.trunc()))
}

pub(crate) fn fn_fracpart(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FRACPART", args, 1)?;
    Ok(args[0].mapv(|x| x.fract()))
}

pub(crate) fn fn_mod(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MOD", args, 2)?;
    let a = &args[0];
    let b = &args[1];
    let len = a.len();
    let mut out = Array1::zeros(len);
    for i in 0..len {
        if b[i].abs() > crate::utils::NUMERIC_EPSILON {
            out[i] = a[i] % b[i];
        } else {
            out[i] = f64::NAN;
        }
    }
    Ok(out)
}

pub(crate) fn fn_reverse(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("REVERSE", args, 1)?;
    let input = &args[0];
    let len = ctx.data_len;
    let mut out = Array1::zeros(len);
    for i in 0..len {
        out[i] = input[len - 1 - i];
    }
    Ok(out)
}
