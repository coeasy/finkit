//! Hilbert-transform cycle formula shims.

use super::prelude::*;

// ======================== CYCLE INDICATORS (Hilbert Transform) ========================

pub(crate) fn fn_ht_phasor_inner(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HT_PHASOR", args, 1)?;
    let data = args[0].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_cycle::ht_phasor(data) {
        Ok((in_phase, _quadrature)) => Ok(in_phase),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_ht_sine_inner(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HT_SINE", args, 1)?;
    let data = args[0].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_cycle::ht_sine(data) {
        Ok((sine, _lead_sine)) => Ok(sine),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_ht_dcperiod(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HT_DCPERIOD", args, 1)?;
    let data = args[0].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_cycle::ht_dcperiod(data) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_ht_dcphase(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HT_DCPHASE", args, 1)?;
    let data = args[0].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_cycle::ht_dcphase(data) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_ht_trendmode(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HT_TRENDMODE", args, 1)?;
    let data = args[0].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_cycle::ht_trendmode(data) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_ht_trendline(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HT_TRENDLINE", args, 1)?;
    let data = args[0].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_cycle::ht_trendline(data) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_ht_measurement(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HT_MEASUREMENT", args, 1)?;
    let data = args[0].as_slice().unwrap();
    let data_len = ctx.data_len;
    match lib_cycle::ht_measurement(data) {
        Ok(r) => Ok(r),
        Err(_) => Ok(nan_vec(data_len)),
    }
}
