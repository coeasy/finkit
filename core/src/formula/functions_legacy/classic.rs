//! Classical chart-pattern formula shims.

use super::prelude::*;

// ======================== CLASSIC CHART PATTERNS ========================
// These are first-class stock-trading patterns (Darvas, Renko, Kagi, PnF,
// TLB, Alligator). They return a single per-bar series chosen to match
// the canonical "trend filter" used by traders.

pub(crate) fn fn_darvas_box_top(
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

pub(crate) fn fn_renko(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_kagi(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
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

pub(crate) fn fn_point_and_figure(
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

pub(crate) fn fn_three_line_break(
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

pub(crate) fn fn_williams_alligator_lips(
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
