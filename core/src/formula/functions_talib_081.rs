//! Formula-layer bridge for the TA-Lib 0.7/0.8 additions.
//!
//! Every kernel in this module already existed in `core::indicators` /
//! `core::math` and already has a checked-in golden vector in
//! `tests/golden/talib`. What was missing is the *formula surface*: a user
//! writing `ZLEMA(CLOSE, 20)` used to get "unknown function" even though
//! `math::moving_avg::zlema` was fully covered by the numeric contract.
//!
//! This module is pure wiring. Two rules keep it honest:
//!
//! 1. **`warmup_offset` + `shift_back`** — every kernel runs on the tail that
//!    starts at the first finite input value. When the input carries no
//!    warm-up prefix (`start == 0`) the call is bit-identical to calling the
//!    kernel directly, so golden vectors and benchmarks cannot move.
//! 2. **No re-implementation** — each wrapper delegates to the same library
//!    function the numeric contract is checked against. Duplicating an
//!    algorithm here would create the tree/plan divergence documented in
//!    `.workbuddy-ai/memory/TRAPS.md`.
//!
//! Multi-output TA-Lib functions expose their primary output under the plain
//! name and every output under a `<NAME>_<OUTPUT>` suffix, mirroring the
//! existing `BOLL` / `BOLLMID` / `BOLLDN` convention.

use ndarray::Array1;

use super::functions::{shift_back, warmup_offset, FormulaFn};
use super::types::{FormulaContext, FormulaError};

type FormulaResult = Result<Array1<f64>, FormulaError>;

#[inline]
fn nan_vec(len: usize) -> Array1<f64> {
    Array1::from_elem(len, f64::NAN)
}

#[inline]
fn need(name: &str, args: &[Array1<f64>], expected: usize) -> Result<(), FormulaError> {
    if args.len() < expected {
        return Err(FormulaError::InvalidParameter(format!(
            "{name} requires at least {expected} arguments, got {}",
            args.len()
        )));
    }
    Ok(())
}

/// Read an optional trailing period argument, falling back to `default`.
#[inline]
fn period_at(
    name: &str,
    args: &[Array1<f64>],
    idx: usize,
    default: usize,
) -> Result<usize, FormulaError> {
    match args.get(idx).and_then(|arg| arg.get(0)) {
        Some(value) if value.is_finite() => {
            let n = *value as usize;
            if n == 0 {
                return Err(FormulaError::InvalidParameter(format!(
                    "{name}: period must be > 0"
                )));
            }
            Ok(n)
        }
        _ => Ok(default),
    }
}

/// Read an optional trailing float argument, falling back to `default`.
#[inline]
fn float_at(args: &[Array1<f64>], idx: usize, default: f64) -> f64 {
    args.get(idx)
        .and_then(|arg| arg.get(0))
        .copied()
        .filter(|value| value.is_finite())
        .unwrap_or(default)
}

#[inline]
fn slice(args: &[Array1<f64>], idx: usize) -> &[f64] {
    args[idx].as_slice().unwrap_or(&[])
}

/// Run `kernel` on the shared warm-up tail of every input series.
///
/// All inputs of a TA-Lib profile share one warm-up prefix (they are OHLCV
/// columns of the same frame, or an upstream rolling indicator applied to one
/// of them), so a single offset applies to the whole argument list.
fn rolling(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
    inputs: usize,
    kernel: impl FnOnce(&[&[f64]]) -> crate::error::Result<Array1<f64>>,
) -> FormulaResult {
    let mut heads: Vec<&[f64]> = Vec::with_capacity(inputs);
    for idx in 0..inputs {
        heads.push(slice(args, idx));
    }
    let len = heads[0].len();
    if heads.iter().any(|series| series.len() != len) {
        return Ok(nan_vec(ctx.data_len));
    }
    let start = warmup_offset(heads[0]);
    if start >= len {
        return Ok(nan_vec(ctx.data_len));
    }
    let tails: Vec<&[f64]> = heads.iter().map(|series| &series[start..]).collect();
    match kernel(&tails) {
        Ok(result) => Ok(shift_back(result, start, ctx.data_len)),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

/// [`rolling`] for kernels returning a multi-output struct.
fn rolling_multi<T>(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
    inputs: usize,
    kernel: impl FnOnce(&[&[f64]]) -> crate::error::Result<T>,
    pick: impl FnOnce(T) -> Array1<f64>,
) -> FormulaResult {
    let mut heads: Vec<&[f64]> = Vec::with_capacity(inputs);
    for idx in 0..inputs {
        heads.push(slice(args, idx));
    }
    let len = heads[0].len();
    if heads.iter().any(|series| series.len() != len) {
        return Ok(nan_vec(ctx.data_len));
    }
    let start = warmup_offset(heads[0]);
    if start >= len {
        return Ok(nan_vec(ctx.data_len));
    }
    let tails: Vec<&[f64]> = heads.iter().map(|series| &series[start..]).collect();
    match kernel(&tails) {
        Ok(result) => Ok(shift_back(pick(result), start, ctx.data_len)),
        Err(_) => Ok(nan_vec(ctx.data_len)),
    }
}

/// Replace non-finite values with `0.0`, mirroring the dispatcher boundary.
///
/// TA-Lib emits `0` (not NaN) for the undefined rows of its pattern-style and
/// cumulative outputs, and `tests/contracts/talib_numeric_contract_v1.json`
/// pins that convention for `WAD` and `FRACTAL`. The Core kernels keep NaN for
/// those rows on purpose, and `ffi/ffi-common/src/execute.rs` normalizes at the
/// compatibility boundary *instead of changing the Core API*. The formula
/// surface is held to the same numbers as the dispatcher, so it applies the
/// same normalization rather than forking the kernel.
#[inline]
fn zero_for_undefined(values: Array1<f64>) -> Array1<f64> {
    values.mapv(|value| if value.is_finite() { value } else { 0.0 })
}

// ---------------------------------------------------------------------------
// Single-output kernels
// ---------------------------------------------------------------------------

/// `AC(HIGH, LOW[, fast, slow, signal])` — Accumulation/Distribution
/// oscillator added in TA-Lib 0.7.
pub fn canonical_ac(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("AC", args, 2)?;
    let fast = period_at("AC", args, 2, 5)?;
    let slow = period_at("AC", args, 3, 34)?;
    let signal = period_at("AC", args, 4, 5)?;
    rolling(ctx, args, 2, |input| {
        crate::indicators::talib_ext::ac(input[0], input[1], fast, slow, signal)
    })
}

/// `ADR(HIGH, LOW[, period])` — Average Daily Range.
pub fn canonical_adr(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("ADR", args, 2)?;
    let period = period_at("ADR", args, 2, 14)?;
    rolling(ctx, args, 2, |input| {
        crate::indicators::talib_ext::adr(input[0], input[1], period)
    })
}

/// `AO(HIGH, LOW[, fast, slow])` — Awesome Oscillator.
pub fn canonical_ao(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("AO", args, 2)?;
    let fast = period_at("AO", args, 2, 5)?;
    let slow = period_at("AO", args, 3, 34)?;
    rolling(ctx, args, 2, |input| {
        crate::indicators::momentum_ext::ao(input[0], input[1], fast, slow)
    })
}

/// `CMOU(CLOSE[, period])` — Chande Momentum Oscillator (TA-Lib 0.8 name).
pub fn canonical_cmou(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("CMOU", args, 1)?;
    let period = period_at("CMOU", args, 1, 14)?;
    rolling(ctx, args, 1, |input| {
        crate::indicators::talib_ext::cmou(input[0], period)
    })
}

/// `COPPOCK(CLOSE[, wma, roc_long, roc_short])` — Coppock Curve.
pub fn canonical_coppock(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("COPPOCK", args, 1)?;
    let wma = period_at("COPPOCK", args, 1, 10)?;
    let long_roc = period_at("COPPOCK", args, 2, 14)?;
    let short_roc = period_at("COPPOCK", args, 3, 11)?;
    rolling(ctx, args, 1, |input| {
        crate::indicators::momentum_ext::coppock(input[0], wma, long_roc, short_roc)
    })
}

/// `CVI(HIGH, LOW[, period, roc])` — Chaikin Volatility Index.
pub fn canonical_cvi(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("CVI", args, 2)?;
    let period = period_at("CVI", args, 2, 10)?;
    let roc = period_at("CVI", args, 3, 10)?;
    rolling(ctx, args, 2, |input| {
        crate::indicators::talib_ext::cvi(input[0], input[1], period, roc)
    })
}

/// `EFI(CLOSE, VOLUME[, period])` — Elder Force Index.
pub fn canonical_efi(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("EFI", args, 2)?;
    let period = period_at("EFI", args, 2, 13)?;
    rolling(ctx, args, 2, |input| {
        crate::indicators::talib_ext::efi(input[0], input[1], period)
    })
}

/// `ER(CLOSE[, period])` — Kaufman Efficiency Ratio.
pub fn canonical_er(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("ER", args, 1)?;
    let period = period_at("ER", args, 1, 10)?;
    rolling(ctx, args, 1, |input| {
        crate::indicators::overlap::efficiency_ratio(input[0], period)
    })
}

/// `FOSC(CLOSE[, period])` — Forecast Oscillator.
pub fn canonical_fosc(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("FOSC", args, 1)?;
    let period = period_at("FOSC", args, 1, 5)?;
    rolling(ctx, args, 1, |input| {
        crate::indicators::talib_ext::fosc(input[0], period)
    })
}

/// `MARKETFI(HIGH, LOW, VOLUME)` — Market Facilitation Index.
pub fn canonical_marketfi(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("MARKETFI", args, 3)?;
    rolling(ctx, args, 3, |input| {
        crate::indicators::talib_ext::marketfi(input[0], input[1], input[2])
    })
}

/// `MASSI(HIGH, LOW[, fast, slow])` — Mass Index.
pub fn canonical_massi(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("MASSI", args, 2)?;
    let fast = period_at("MASSI", args, 2, 9)?;
    let slow = period_at("MASSI", args, 3, 25)?;
    rolling(ctx, args, 2, |input| {
        crate::indicators::talib_ext::massi(input[0], input[1], fast, slow)
    })
}

/// `NVI(CLOSE, VOLUME)` — Negative Volume Index.
pub fn canonical_nvi(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("NVI", args, 2)?;
    rolling(ctx, args, 2, |input| {
        crate::indicators::volume_ext::nvi(input[0], input[1])
    })
}

/// `PERCENTRANK(CLOSE[, period])` — TA-Lib `PERCENTRANK` (0..100 scale).
pub fn canonical_percentrank(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("PERCENTRANK", args, 1)?;
    let period = period_at("PERCENTRANK", args, 1, 100)?;
    rolling(ctx, args, 1, |input| {
        crate::indicators::statistics::percent_rank(input[0], period)
    })
}

/// `PVI(CLOSE, VOLUME)` — Positive Volume Index.
pub fn canonical_pvi(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("PVI", args, 2)?;
    rolling(ctx, args, 2, |input| {
        crate::indicators::volume_ext::pvi(input[0], input[1])
    })
}

/// `PVO(VOLUME[, fast, slow, matype])` — Percentage Volume Oscillator.
pub fn canonical_pvo(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("PVO", args, 1)?;
    let fast = period_at("PVO", args, 1, 12)?;
    let slow = period_at("PVO", args, 2, 26)?;
    let ma_type = period_at("PVO", args, 3, 1)?;
    rolling(ctx, args, 1, |input| {
        crate::indicators::talib_ext::pvo(input[0], fast, slow, ma_type)
    })
}

/// `PVT(CLOSE, VOLUME)` — Price Volume Trend.
pub fn canonical_pvt(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("PVT", args, 2)?;
    rolling(ctx, args, 2, |input| {
        crate::indicators::volume_ext::pvt(input[0], input[1])
    })
}

/// `QSTICK(OPEN, CLOSE[, period])` — Quantitative Stick.
pub fn canonical_qstick(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("QSTICK", args, 2)?;
    let period = period_at("QSTICK", args, 2, 10)?;
    rolling(ctx, args, 2, |input| {
        crate::indicators::talib_ext::qstick(input[0], input[1], period)
    })
}

/// `RVI(CLOSE[, period, stddev])` — Relative Vigor Index (TA-Lib profile form).
pub fn canonical_rvi(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("RVI", args, 1)?;
    let period = period_at("RVI", args, 1, 14)?;
    let stddev = period_at("RVI", args, 2, 10)?;
    rolling(ctx, args, 1, |input| {
        crate::indicators::talib_ext::rvi_profile(input[0], period, stddev)
    })
}

/// `RVOL(VOLUME[, period])` — Relative Volume.
pub fn canonical_rvol(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("RVOL", args, 1)?;
    let period = period_at("RVOL", args, 1, 20)?;
    rolling(ctx, args, 1, |input| {
        crate::indicators::talib_ext::rvol(input[0], period)
    })
}

/// `VHF(CLOSE[, period])` — Vertical Horizontal Filter.
pub fn canonical_vhf(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("VHF", args, 1)?;
    let period = period_at("VHF", args, 1, 28)?;
    rolling(ctx, args, 1, |input| {
        crate::indicators::talib_ext::vhf(input[0], period)
    })
}

/// `WAD(HIGH, LOW, CLOSE)` — Williams Accumulation/Distribution.
pub fn canonical_wad(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("WAD", args, 3)?;
    let result = rolling(ctx, args, 3, |input| {
        crate::indicators::talib_ext::wad(input[0], input[1], input[2])
    })?;
    Ok(zero_for_undefined(result))
}

/// `ZLEMA(CLOSE[, period])` — Zero-Lag Exponential Moving Average.
pub fn canonical_zlema(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("ZLEMA", args, 1)?;
    let period = period_at("ZLEMA", args, 1, 30)?;
    rolling(ctx, args, 1, |input| {
        crate::math::moving_avg::zlema(input[0], period)
    })
}

// ---------------------------------------------------------------------------
// Multi-output kernels
// ---------------------------------------------------------------------------

/// `ACCBANDS(HIGH, LOW, CLOSE[, period])` — Acceleration Bands (upper).
pub fn canonical_accbands(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("ACCBANDS", args, 3)?;
    let period = period_at("ACCBANDS", args, 3, 20)?;
    rolling_multi(
        ctx,
        args,
        3,
        |input| crate::indicators::overlap::accbands(input[0], input[1], input[2], period),
        |result| result.upper,
    )
}

/// `ACCBANDS_MID(...)` — middle band of [`canonical_accbands`].
pub fn canonical_accbands_mid(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("ACCBANDS_MID", args, 3)?;
    let period = period_at("ACCBANDS_MID", args, 3, 20)?;
    rolling_multi(
        ctx,
        args,
        3,
        |input| crate::indicators::overlap::accbands(input[0], input[1], input[2], period),
        |result| result.middle,
    )
}

/// `ACCBANDS_LOWER(...)` — lower band of [`canonical_accbands`].
pub fn canonical_accbands_lower(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("ACCBANDS_LOWER", args, 3)?;
    let period = period_at("ACCBANDS_LOWER", args, 3, 20)?;
    rolling_multi(
        ctx,
        args,
        3,
        |input| crate::indicators::overlap::accbands(input[0], input[1], input[2], period),
        |result| result.lower,
    )
}

/// `AROON(HIGH, LOW[, period])` — Aroon Up (see also `AROON_UP` / `AROON_DN`).
pub fn canonical_aroon(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("AROON", args, 2)?;
    let period = period_at("AROON", args, 2, 14)?;
    rolling_multi(
        ctx,
        args,
        2,
        |input| crate::indicators::momentum::aroon(input[0], input[1], period),
        |result| result.aroon_up,
    )
}

/// `AROON_DOWN(...)` — explicit down-leg companion of [`canonical_aroon`].
pub fn canonical_aroon_down(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("AROON_DOWN", args, 2)?;
    let period = period_at("AROON_DOWN", args, 2, 14)?;
    rolling_multi(
        ctx,
        args,
        2,
        |input| crate::indicators::momentum::aroon(input[0], input[1], period),
        |result| result.aroon_down,
    )
}

/// `ERI(HIGH, LOW, CLOSE[, period])` — Elder Ray bull power.
pub fn canonical_eri(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("ERI", args, 3)?;
    let period = period_at("ERI", args, 3, 13)?;
    rolling_multi(
        ctx,
        args,
        3,
        |input| crate::indicators::talib_ext::eri(input[0], input[1], input[2], period),
        |result| result.bullpower,
    )
}

/// `ERI_BULL(...)` — bull power of [`canonical_eri`].
pub fn canonical_eri_bull(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    canonical_eri(ctx, args)
}

/// `ERI_BEAR(...)` — bear power of [`canonical_eri`].
pub fn canonical_eri_bear(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("ERI_BEAR", args, 3)?;
    let period = period_at("ERI_BEAR", args, 3, 13)?;
    rolling_multi(
        ctx,
        args,
        3,
        |input| crate::indicators::talib_ext::eri(input[0], input[1], input[2], period),
        |result| result.bearpower,
    )
}

/// `FRACTAL(HIGH, LOW[, left, right])` — swing-high marker (100 / 0).
pub fn canonical_fractal(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("FRACTAL", args, 2)?;
    let left = period_at("FRACTAL", args, 2, 2)?;
    let right = period_at("FRACTAL", args, 3, 2)?;
    let result = rolling_multi(
        ctx,
        args,
        2,
        |input| crate::indicators::talib_ext::fractal(input[0], input[1], left, right),
        |result| result.swinghigh,
    )?;
    Ok(zero_for_undefined(result))
}

/// `FRACTAL_HIGH(...)` — swing-high marker of [`canonical_fractal`].
pub fn canonical_fractal_high(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    canonical_fractal(ctx, args)
}

/// `FRACTAL_LOW(...)` — swing-low marker of [`canonical_fractal`].
pub fn canonical_fractal_low(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("FRACTAL_LOW", args, 2)?;
    let left = period_at("FRACTAL_LOW", args, 2, 2)?;
    let right = period_at("FRACTAL_LOW", args, 3, 2)?;
    let result = rolling_multi(
        ctx,
        args,
        2,
        |input| crate::indicators::talib_ext::fractal(input[0], input[1], left, right),
        |result| result.swinglow,
    )?;
    Ok(zero_for_undefined(result))
}

/// `HA(OPEN, HIGH, LOW, CLOSE)` — Heikin-Ashi close.
pub fn canonical_ha(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("HA", args, 4)?;
    rolling_multi(
        ctx,
        args,
        4,
        |input| crate::indicators::chart::heikin_ashi(input[0], input[1], input[2], input[3]),
        |result| result.ha_close,
    )
}

/// `HA_OPEN(...)` — Heikin-Ashi open.
pub fn canonical_ha_open(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("HA_OPEN", args, 4)?;
    rolling_multi(
        ctx,
        args,
        4,
        |input| crate::indicators::chart::heikin_ashi(input[0], input[1], input[2], input[3]),
        |result| result.ha_open,
    )
}

/// `HA_HIGH(...)` — Heikin-Ashi high.
pub fn canonical_ha_high(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("HA_HIGH", args, 4)?;
    rolling_multi(
        ctx,
        args,
        4,
        |input| crate::indicators::chart::heikin_ashi(input[0], input[1], input[2], input[3]),
        |result| result.ha_high,
    )
}

/// `HA_LOW(...)` — Heikin-Ashi low.
pub fn canonical_ha_low(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("HA_LOW", args, 4)?;
    rolling_multi(
        ctx,
        args,
        4,
        |input| crate::indicators::chart::heikin_ashi(input[0], input[1], input[2], input[3]),
        |result| result.ha_low,
    )
}

/// `HA_CLOSE(...)` — Heikin-Ashi close (explicit form of [`canonical_ha`]).
pub fn canonical_ha_close(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    canonical_ha(ctx, args)
}

/// `KC(HIGH, LOW, CLOSE[, period, atr_period, nbdev])` — Keltner upper band.
pub fn canonical_kc(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("KC", args, 3)?;
    let period = period_at("KC", args, 3, 20)?;
    let atr_period = period_at("KC", args, 4, 10)?;
    let nbdev = float_at(args, 5, 2.0);
    rolling_multi(
        ctx,
        args,
        3,
        |input| {
            crate::indicators::talib_ext::kc(
                input[0], input[1], input[2], period, atr_period, nbdev,
            )
        },
        |result| result.upperband,
    )
}

/// `KC_MID(...)` — Keltner middle band.
pub fn canonical_kc_mid(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("KC_MID", args, 3)?;
    let period = period_at("KC_MID", args, 3, 20)?;
    let atr_period = period_at("KC_MID", args, 4, 10)?;
    let nbdev = float_at(args, 5, 2.0);
    rolling_multi(
        ctx,
        args,
        3,
        |input| {
            crate::indicators::talib_ext::kc(
                input[0], input[1], input[2], period, atr_period, nbdev,
            )
        },
        |result| result.middleband,
    )
}

/// `KC_LOWER(...)` — Keltner lower band.
pub fn canonical_kc_lower(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("KC_LOWER", args, 3)?;
    let period = period_at("KC_LOWER", args, 3, 20)?;
    let atr_period = period_at("KC_LOWER", args, 4, 10)?;
    let nbdev = float_at(args, 5, 2.0);
    rolling_multi(
        ctx,
        args,
        3,
        |input| {
            crate::indicators::talib_ext::kc(
                input[0], input[1], input[2], period, atr_period, nbdev,
            )
        },
        |result| result.lowerband,
    )
}

/// `MAMA(CLOSE[, fast_limit, slow_limit])` — Mesa Adaptive Moving Average.
pub fn canonical_mama(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("MAMA", args, 1)?;
    let fast = float_at(args, 1, 0.5);
    let slow = float_at(args, 2, 0.05);
    rolling_multi(
        ctx,
        args,
        1,
        |input| crate::indicators::overlap::mama(input[0], fast, slow),
        |result| result.mama,
    )
}

/// `MAMA_FAMA(...)` — Following Adaptive Moving Average.
pub fn canonical_mama_fama(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("MAMA_FAMA", args, 1)?;
    let fast = float_at(args, 1, 0.5);
    let slow = float_at(args, 2, 0.05);
    rolling_multi(
        ctx,
        args,
        1,
        |input| crate::indicators::overlap::mama(input[0], fast, slow),
        |result| result.fama,
    )
}

/// `SMI(HIGH, LOW, CLOSE[, period, fast, slow, signal])` — Stochastic
/// Momentum Index.
pub fn canonical_smi(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("SMI", args, 3)?;
    let period = period_at("SMI", args, 3, 13)?;
    let fast = period_at("SMI", args, 4, 2)?;
    let slow = period_at("SMI", args, 5, 25)?;
    let signal = period_at("SMI", args, 6, 9)?;
    rolling_multi(
        ctx,
        args,
        3,
        |input| {
            crate::indicators::talib_ext::smi(
                input[0], input[1], input[2], period, fast, slow, signal,
            )
        },
        |result| result.smi,
    )
}

/// `SMI_SIGNAL(...)` — signal line of [`canonical_smi`].
pub fn canonical_smi_signal(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("SMI_SIGNAL", args, 3)?;
    let period = period_at("SMI_SIGNAL", args, 3, 13)?;
    let fast = period_at("SMI_SIGNAL", args, 4, 2)?;
    let slow = period_at("SMI_SIGNAL", args, 5, 25)?;
    let signal = period_at("SMI_SIGNAL", args, 6, 9)?;
    rolling_multi(
        ctx,
        args,
        3,
        |input| {
            crate::indicators::talib_ext::smi(
                input[0], input[1], input[2], period, fast, slow, signal,
            )
        },
        |result| result.smisignal,
    )
}

/// `VORTEX(HIGH, LOW, CLOSE[, period])` — Vortex positive (VI+).
pub fn canonical_vortex(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("VORTEX", args, 3)?;
    let period = period_at("VORTEX", args, 3, 14)?;
    rolling_multi(
        ctx,
        args,
        3,
        |input| crate::indicators::momentum_ext::vortex(input[0], input[1], input[2], period),
        |result| result.vi_plus,
    )
}

/// `VORTEX_PLUS(...)` — VI+ of [`canonical_vortex`].
pub fn canonical_vortex_plus(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    canonical_vortex(ctx, args)
}

/// `VORTEX_MINUS(...)` — VI- of [`canonical_vortex`].
pub fn canonical_vortex_minus(ctx: &FormulaContext, args: &[Array1<f64>]) -> FormulaResult {
    need("VORTEX_MINUS", args, 3)?;
    let period = period_at("VORTEX_MINUS", args, 3, 14)?;
    rolling_multi(
        ctx,
        args,
        3,
        |input| crate::indicators::momentum_ext::vortex(input[0], input[1], input[2], period),
        |result| result.vi_minus,
    )
}

/// Names of the TA-Lib 0.7/0.8 additions wired by this module.
///
/// `core/tests/talib_coverage_matrix.rs` asserts that every name listed in the
/// coverage matrix's `formula_surface` resolves through
/// `get_builtin_functions()`, so this list is the single source of truth for
/// "the numeric contract is reachable from the formula DSL".
pub const FORMULA_SURFACE_NAMES: &[&str] = &[
    "AC",
    "ACCBANDS",
    "ADR",
    "AO",
    "AROON",
    "CMOU",
    "COPPOCK",
    "CVI",
    "EFI",
    "ER",
    "ERI",
    "FOSC",
    "FRACTAL",
    "HA",
    "KC",
    "MAMA",
    "MARKETFI",
    "MASSI",
    "NVI",
    "PERCENTRANK",
    "PVI",
    "PVO",
    "PVT",
    "QSTICK",
    "RVI",
    "RVOL",
    "SMI",
    "VHF",
    "VORTEX",
    "WAD",
    "ZLEMA",
];

/// Register every TA-Lib 0.7/0.8 formula bridge into the function table.
///
/// The registration shape below is deliberate: `scripts/gen_ssot_docs.py`
/// parses the literal `map.insert` call sites in this file to build
/// `docs/generated/formula-functions.md`, and
/// `core/tests/formula_function_ssot.rs` asserts that generated catalogue equals
/// the runtime table. A tuple-driven loop would compile and run identically but
/// silently drop these 48 names from the generated documentation -- and, worse,
/// the doc parser would then pick up the tuple label as a function name.
pub fn register(map: &mut std::collections::HashMap<String, FormulaFn>) {
    map.insert("AC".to_string(), canonical_ac as FormulaFn);
    map.insert("ACCBANDS".to_string(), canonical_accbands as FormulaFn);
    map.insert(
        "ACCBANDS_MID".to_string(),
        canonical_accbands_mid as FormulaFn,
    );
    map.insert(
        "ACCBANDS_LOWER".to_string(),
        canonical_accbands_lower as FormulaFn,
    );
    map.insert("ADR".to_string(), canonical_adr as FormulaFn);
    map.insert("AO".to_string(), canonical_ao as FormulaFn);
    map.insert("AROON".to_string(), canonical_aroon as FormulaFn);
    map.insert("AROON_DOWN".to_string(), canonical_aroon_down as FormulaFn);
    map.insert("CMOU".to_string(), canonical_cmou as FormulaFn);
    map.insert("COPPOCK".to_string(), canonical_coppock as FormulaFn);
    map.insert("CVI".to_string(), canonical_cvi as FormulaFn);
    map.insert("EFI".to_string(), canonical_efi as FormulaFn);
    map.insert("ER".to_string(), canonical_er as FormulaFn);
    map.insert("ERI".to_string(), canonical_eri as FormulaFn);
    map.insert("ERI_BULL".to_string(), canonical_eri_bull as FormulaFn);
    map.insert("ERI_BEAR".to_string(), canonical_eri_bear as FormulaFn);
    map.insert("FOSC".to_string(), canonical_fosc as FormulaFn);
    map.insert("FRACTAL".to_string(), canonical_fractal as FormulaFn);
    map.insert(
        "FRACTAL_HIGH".to_string(),
        canonical_fractal_high as FormulaFn,
    );
    map.insert(
        "FRACTAL_LOW".to_string(),
        canonical_fractal_low as FormulaFn,
    );
    map.insert("HA".to_string(), canonical_ha as FormulaFn);
    map.insert("HA_OPEN".to_string(), canonical_ha_open as FormulaFn);
    map.insert("HA_HIGH".to_string(), canonical_ha_high as FormulaFn);
    map.insert("HA_LOW".to_string(), canonical_ha_low as FormulaFn);
    map.insert("HA_CLOSE".to_string(), canonical_ha_close as FormulaFn);
    map.insert("KC".to_string(), canonical_kc as FormulaFn);
    map.insert("KC_MID".to_string(), canonical_kc_mid as FormulaFn);
    map.insert("KC_LOWER".to_string(), canonical_kc_lower as FormulaFn);
    map.insert("MAMA".to_string(), canonical_mama as FormulaFn);
    map.insert("MAMA_FAMA".to_string(), canonical_mama_fama as FormulaFn);
    map.insert("MARKETFI".to_string(), canonical_marketfi as FormulaFn);
    map.insert("MASSI".to_string(), canonical_massi as FormulaFn);
    map.insert("NVI".to_string(), canonical_nvi as FormulaFn);
    map.insert(
        "PERCENTRANK".to_string(),
        canonical_percentrank as FormulaFn,
    );
    map.insert("PVI".to_string(), canonical_pvi as FormulaFn);
    map.insert("PVO".to_string(), canonical_pvo as FormulaFn);
    map.insert("PVT".to_string(), canonical_pvt as FormulaFn);
    map.insert("QSTICK".to_string(), canonical_qstick as FormulaFn);
    map.insert("RVI".to_string(), canonical_rvi as FormulaFn);
    map.insert("RVOL".to_string(), canonical_rvol as FormulaFn);
    map.insert("SMI".to_string(), canonical_smi as FormulaFn);
    map.insert("SMI_SIGNAL".to_string(), canonical_smi_signal as FormulaFn);
    map.insert("VHF".to_string(), canonical_vhf as FormulaFn);
    map.insert("VORTEX".to_string(), canonical_vortex as FormulaFn);
    map.insert(
        "VORTEX_PLUS".to_string(),
        canonical_vortex_plus as FormulaFn,
    );
    map.insert(
        "VORTEX_MINUS".to_string(),
        canonical_vortex_minus as FormulaFn,
    );
    map.insert("WAD".to_string(), canonical_wad as FormulaFn);
    map.insert("ZLEMA".to_string(), canonical_zlema as FormulaFn);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array1;

    fn context(len: usize) -> FormulaContext {
        let ramp = |offset: f64| Array1::from_vec((0..len).map(|i| offset + i as f64).collect());
        FormulaContext::new(
            ramp(10.0),
            ramp(11.0),
            ramp(9.0),
            ramp(10.0),
            ramp(100.0),
            None,
        )
    }

    #[test]
    fn every_declared_name_is_registered() {
        let functions = super::super::functions::get_builtin_functions();
        for name in FORMULA_SURFACE_NAMES {
            assert!(
                functions.contains_key(*name),
                "formula surface missing {name}"
            );
        }
    }

    #[test]
    fn warmup_prefix_does_not_poison_composition() {
        // Mirrors the trap documented in TRAPS.md: a rolling kernel composed on
        // top of another rolling kernel used to return an all-NaN series
        // because NaN is absorbing for the incremental accumulators.
        let ctx = context(64);
        let seed = Array1::from_vec((0..64).map(|i| 10.0 + i as f64).collect());
        let inner = canonical_zlema(&ctx, &[seed, Array1::from_elem(1, 10.0)]).expect("ZLEMA");
        let finite = inner.iter().filter(|value| value.is_finite()).count();
        assert!(finite > 0, "ZLEMA produced no finite values");
        let outer = canonical_zlema(&ctx, &[inner, Array1::from_elem(1, 5.0)]).expect("ZLEMA*2");
        let finite_outer = outer.iter().filter(|value| value.is_finite()).count();
        assert!(
            finite_outer > 0,
            "ZLEMA(ZLEMA(x)) collapsed to NaN -- warm-up handling regressed"
        );
    }
}
