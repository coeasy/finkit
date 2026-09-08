use ndarray::Array1;
use std::collections::HashMap;

use crate::formula::types::{FormulaContext, FormulaError};

type FormulaFn = fn(&FormulaContext, &[Array1<f64>]) -> Result<Array1<f64>, FormulaError>;

#[inline]
fn nan_vec(len: usize) -> Array1<f64> {
    Array1::from_elem(len, f64::NAN)
}

#[inline]
fn ensure_args_len(name: &str, args: &[Array1<f64>], expected: usize) -> Result<(), FormulaError> {
    if args.len() < expected {
        return Err(FormulaError::InvalidParameter(format!(
            "{} requires at least {} arguments, got {}",
            name,
            expected,
            args.len()
        )));
    }
    Ok(())
}

#[inline]
fn extract_n(args: &[Array1<f64>], idx: usize, name: &str) -> Result<usize, FormulaError> {
    if idx >= args.len() {
        return Err(FormulaError::RuntimeError(format!(
            "{}: missing argument at index {}",
            name, idx
        )));
    }
    let n = args[idx][0] as usize;
    if n == 0 {
        return Err(FormulaError::InvalidParameter(format!(
            "{}: period must be > 0",
            name
        )));
    }
    Ok(n)
}

#[inline]
fn optional_f64(args: &[Array1<f64>], idx: usize, default: f64) -> f64 {
    args.get(idx)
        .and_then(|arg| arg.get(0))
        .copied()
        .filter(|value| !value.is_nan())
        .unwrap_or(default)
}

#[inline]
fn canonical_atr(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ATR", args, 4)?;
    let period = extract_n(args, 3, "ATR")?;
    match crate::indicators::volatility::atr(
        args[0].as_slice().unwrap(),
        args[1].as_slice().unwrap(),
        args[2].as_slice().unwrap(),
        period,
    ) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

#[inline]
fn canonical_natr(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("NATR", args, 4)?;
    let period = extract_n(args, 3, "NATR")?;
    match crate::indicators::volatility::natr(
        args[0].as_slice().unwrap(),
        args[1].as_slice().unwrap(),
        args[2].as_slice().unwrap(),
        period,
    ) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

#[inline]
fn canonical_trange(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TRANGE", args, 3)?;
    match crate::indicators::volatility::trange(
        args[0].as_slice().unwrap(),
        args[1].as_slice().unwrap(),
        args[2].as_slice().unwrap(),
    ) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

#[inline]
fn canonical_trima(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TRIMA", args, 2)?;
    let period = extract_n(args, 1, "TRIMA")?;
    match crate::math::moving_avg::trima(args[0].as_slice().unwrap(), period) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

#[inline]
fn canonical_std(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("STD", args, 2)?;
    let period = extract_n(args, 1, "STD")?;
    match crate::indicators::statistics::std_dev(args[0].as_slice().unwrap(), period, 1.0) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

#[inline]
fn canonical_var(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("VAR", args, 2)?;
    let period = extract_n(args, 1, "VAR")?;
    match crate::indicators::statistics::var(args[0].as_slice().unwrap(), period, 1.0) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

#[derive(Clone, Copy)]
enum BbandComponent {
    Upper,
    Middle,
    Lower,
    Width,
}

#[inline]
fn canonical_bband_component(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
    name: &str,
    component: BbandComponent,
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len(name, args, 2)?;
    let period = extract_n(args, 1, name)?;
    let nbdev = optional_f64(args, 2, 2.0);
    let result =
        match crate::indicators::overlap::bbands(args[0].as_slice().unwrap(), period, nbdev, nbdev)
        {
            Ok(result) => result,
            Err(_) => return Ok(nan_vec(ctx.data_len)),
        };

    match component {
        BbandComponent::Upper => Ok(result.upper),
        BbandComponent::Middle => Ok(result.middle),
        BbandComponent::Lower => Ok(result.lower),
        BbandComponent::Width => {
            let mut width = nan_vec(ctx.data_len);
            for i in 0..ctx.data_len {
                let middle = result.middle[i];
                if middle.is_finite() && middle.abs() > 1e-15 {
                    width[i] = (result.upper[i] - result.lower[i]) / middle * 100.0;
                }
            }
            Ok(width)
        }
    }
}

#[inline]
fn canonical_boll(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    canonical_bband_component(ctx, args, "BOLL", BbandComponent::Upper)
}

#[inline]
fn canonical_bolldn(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    canonical_bband_component(ctx, args, "BOLLDN", BbandComponent::Lower)
}

#[inline]
fn canonical_bollmid(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    canonical_bband_component(ctx, args, "BOLLMID", BbandComponent::Middle)
}

#[inline]
fn canonical_bollwidth(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    canonical_bband_component(ctx, args, "BOLLWIDTH", BbandComponent::Width)
}

#[inline]
fn canonical_obv(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("OBV", args, 2)?;
    match crate::math::volume_kernels::obv(args[0].as_slice().unwrap(), args[1].as_slice().unwrap())
    {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

#[inline]
fn canonical_ad(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("AD", args, 4)?;
    match crate::math::volume_kernels::ad(
        args[0].as_slice().unwrap(),
        args[1].as_slice().unwrap(),
        args[2].as_slice().unwrap(),
        args[3].as_slice().unwrap(),
    ) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

#[inline]
fn canonical_adosc(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ADOSC", args, 4)?;
    let fast_period = args
        .get(4)
        .map(|_| extract_n(args, 4, "ADOSC"))
        .transpose()?
        .unwrap_or(3);
    let slow_period = args
        .get(5)
        .map(|_| extract_n(args, 5, "ADOSC"))
        .transpose()?
        .unwrap_or(10);
    match crate::math::volume_kernels::adosc(
        args[0].as_slice().unwrap(),
        args[1].as_slice().unwrap(),
        args[2].as_slice().unwrap(),
        args[3].as_slice().unwrap(),
        fast_period,
        slow_period,
    ) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

#[inline]
fn canonical_mfi(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("MFI", args, 4)?;
    let period = args
        .get(4)
        .map(|_| extract_n(args, 4, "MFI"))
        .transpose()?
        .unwrap_or(14);
    match crate::math::mfi::mfi(
        args[0].as_slice().unwrap(),
        args[1].as_slice().unwrap(),
        args[2].as_slice().unwrap(),
        args[3].as_slice().unwrap(),
        period,
    ) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

/// Build the formula function table from the compatibility surface, then replace
/// duplicate hot-path implementations with the same canonical kernels used by
/// the public indicator APIs and language bindings.
///
/// Keeping compatibility-only functions in the legacy table avoids a risky
/// all-at-once rewrite of the large formula catalogue, while these overrides
/// make the TA-Lib-sensitive core semantics single-source-of-truth.
pub fn get_builtin_functions() -> HashMap<String, FormulaFn> {
    let legacy = super::functions_legacy::get_builtin_functions();
    let mut map: HashMap<String, FormulaFn> = HashMap::with_capacity(legacy.len());
    for (name, function) in legacy {
        map.insert(name, function);
    }

    map.insert("ATR".to_string(), canonical_atr as FormulaFn);
    map.insert("NATR".to_string(), canonical_natr as FormulaFn);
    map.insert("TRANGE".to_string(), canonical_trange as FormulaFn);
    map.insert("TRIMA".to_string(), canonical_trima as FormulaFn);

    map.insert("STD".to_string(), canonical_std as FormulaFn);
    map.insert("STDDEV".to_string(), canonical_std as FormulaFn);
    map.insert("VAR".to_string(), canonical_var as FormulaFn);

    map.insert("BOLL".to_string(), canonical_boll as FormulaFn);
    map.insert("BOLLUP".to_string(), canonical_boll as FormulaFn);
    map.insert("BBANDS".to_string(), canonical_boll as FormulaFn);
    map.insert("BOLLDN".to_string(), canonical_bolldn as FormulaFn);
    map.insert("BOLLMID".to_string(), canonical_bollmid as FormulaFn);
    map.insert("BOLLWIDTH".to_string(), canonical_bollwidth as FormulaFn);

    map.insert("OBV".to_string(), canonical_obv as FormulaFn);
    map.insert("AD".to_string(), canonical_ad as FormulaFn);
    map.insert("ADOSC".to_string(), canonical_adosc as FormulaFn);
    map.insert("MFI".to_string(), canonical_mfi as FormulaFn);

    // Registry aliases must follow the replaced canonical function pointers too.
    // This preserves the alias identity invariant after the canonical overrides.
    for spec in crate::registry::builtin_function_registry().iter() {
        let canonical = map.get(spec.name).copied();
        if let Some(canonical) = canonical {
            for &alias in spec.aliases {
                map.insert(alias.to_string(), canonical);
            }
        }
    }

    map
}
