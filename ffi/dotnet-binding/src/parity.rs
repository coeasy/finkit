// ─────────────────────────────────────────────────────────────────────
// Registry-parity additions for the .NET binding.
//
// `generated.rs` covers the indicators the .NET binding shipped with. This file
// adds the ones it was missing relative to `docs/ffi_registry.json`, so the
// .NET binding exposes the same 78-indicator surface as the C binding.
//
// Conventions are those of `generated.rs`:
//   * `0` = success, negative = failure (a boundary panic maps to a negative
//     code, never `0`);
//   * every write into a caller buffer is clamped to the smaller of the core
//     result and the caller's `length`.
//
// Audited by: python3 scripts/audit_binding_parity.py
// ─────────────────────────────────────────────────────────────────────

use finkit::patterns::candlestick;

/// Clamped, panic-free copy of a core column into a caller buffer.
fn copy_out_f64(src: &Array1<f64>, out: *mut c_double, length: c_int) -> usize {
    let copy_len = src.len().min(length.max(0) as usize);
    let dst = unsafe { std::slice::from_raw_parts_mut(out, copy_len) };
    dst.copy_from_slice(&src.as_slice().expect("Array1 is contiguous")[..copy_len]);
    copy_len
}

fn copy_out_i32(src: &Array1<i32>, out: *mut c_int, length: c_int) -> usize {
    let copy_len = src.len().min(length.max(0) as usize);
    let dst = unsafe { std::slice::from_raw_parts_mut(out, copy_len) };
    dst.copy_from_slice(&src.as_slice().expect("Array1 is contiguous")[..copy_len]);
    copy_len
}

/// Every period must be positive and no larger than the input, matching the
/// validation the C binding performs before calling into the core.
fn periods_ok(length: c_int, periods: &[c_int]) -> bool {
    length > 0 && periods.iter().all(|p| *p > 0 && (*p as usize) <= length as usize)
}

// ============ Momentum / trend ============

#[no_mangle]
pub extern "C" fn ta_apo(
    input: *const c_double,
    length: c_int,
    fast_period: c_int,
    slow_period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if input.is_null() || out.is_null() || !periods_ok(length, &[fast_period, slow_period]) {
            return -1;
        }
        let data = unsafe { std::slice::from_raw_parts(input, length as usize) };
        match indicators::apo(data, fast_period as usize, slow_period as usize) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_cmo(
    input: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if input.is_null() || out.is_null() || !periods_ok(length, &[period]) {
            return -1;
        }
        let data = unsafe { std::slice::from_raw_parts(input, length as usize) };
        match indicators::cmo(data, period as usize) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_trix(
    input: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if input.is_null() || out.is_null() || !periods_ok(length, &[period]) {
            return -1;
        }
        let data = unsafe { std::slice::from_raw_parts(input, length as usize) };
        match indicators::trix(data, period as usize) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_percent_rank(
    input: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if input.is_null() || out.is_null() || !periods_ok(length, &[period]) {
            return -1;
        }
        let data = unsafe { std::slice::from_raw_parts(input, length as usize) };
        match indicators::percent_rank(data, period as usize) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_chande_forecast(
    close: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if close.is_null() || out.is_null() || !periods_ok(length, &[period]) {
            return -1;
        }
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        match indicators::chande_forecast_oscillator(c, period as usize) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_inertia(
    open: *const c_double,
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    rvi_period: c_int,
    linreg_period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if open.is_null() || high.is_null() || low.is_null() || close.is_null() || out.is_null() {
            return -1;
        }
        if !periods_ok(length, &[rvi_period, linreg_period]) {
            return -1;
        }
        let o = unsafe { std::slice::from_raw_parts(open, length as usize) };
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        match indicators::inertia(o, h, l, c, rvi_period as usize, linreg_period as usize) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_sar(
    high: *const c_double,
    low: *const c_double,
    length: c_int,
    acceleration: c_double,
    maximum: c_double,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if high.is_null() || low.is_null() || out.is_null() || length <= 0 {
            return -1;
        }
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        match indicators::sar(h, l, acceleration, maximum) {
            Ok(result) => {
                copy_out_f64(&result.sar, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

// ============ Price transforms ============

#[no_mangle]
pub extern "C" fn ta_avgprice(
    open: *const c_double,
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if open.is_null()
            || high.is_null()
            || low.is_null()
            || close.is_null()
            || out.is_null()
            || length <= 0
        {
            return -1;
        }
        let o = unsafe { std::slice::from_raw_parts(open, length as usize) };
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        match indicators::avgprice(o, h, l, c) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_medprice(
    high: *const c_double,
    low: *const c_double,
    length: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if high.is_null() || low.is_null() || out.is_null() || length <= 0 {
            return -1;
        }
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        match indicators::medprice(h, l) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_typprice(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if high.is_null() || low.is_null() || close.is_null() || out.is_null() || length <= 0 {
            return -1;
        }
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        match indicators::typprice(h, l, c) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_wclprice(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if high.is_null() || low.is_null() || close.is_null() || out.is_null() || length <= 0 {
            return -1;
        }
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        match indicators::wclprice(h, l, c) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_midpoint(
    input: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if input.is_null() || out.is_null() || !periods_ok(length, &[period]) {
            return -1;
        }
        let data = unsafe { std::slice::from_raw_parts(input, length as usize) };
        match indicators::midpoint(data, period as usize) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_midprice(
    high: *const c_double,
    low: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if high.is_null() || low.is_null() || out.is_null() || !periods_ok(length, &[period]) {
            return -1;
        }
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        match indicators::midprice(h, l, period as usize) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

// ============ Volume ============

#[no_mangle]
pub extern "C" fn ta_bop(
    open: *const c_double,
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if open.is_null()
            || high.is_null()
            || low.is_null()
            || close.is_null()
            || out.is_null()
            || length <= 0
        {
            return -1;
        }
        let o = unsafe { std::slice::from_raw_parts(open, length as usize) };
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        match indicators::bop(o, h, l, c) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_mfi(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    volume: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if high.is_null() || low.is_null() || close.is_null() || volume.is_null() || out.is_null() {
            return -1;
        }
        if !periods_ok(length, &[period]) {
            return -1;
        }
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        let v = unsafe { std::slice::from_raw_parts(volume, length as usize) };
        match indicators::mfi(h, l, c, v, period as usize) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_vzo(
    close: *const c_double,
    volume: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if close.is_null() || volume.is_null() || out.is_null() || !periods_ok(length, &[period]) {
            return -1;
        }
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        let v = unsafe { std::slice::from_raw_parts(volume, length as usize) };
        match indicators::vzo(c, v, period as usize) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_volume_momentum(
    volume: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if volume.is_null() || out.is_null() || !periods_ok(length, &[period]) {
            return -1;
        }
        let v = unsafe { std::slice::from_raw_parts(volume, length as usize) };
        match indicators::volume_momentum(v, period as usize) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_volume_roc(
    volume: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if volume.is_null() || out.is_null() || !periods_ok(length, &[period]) {
            return -1;
        }
        let v = unsafe { std::slice::from_raw_parts(volume, length as usize) };
        match indicators::volume_roc(v, period as usize) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_twiggs_mf(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    volume: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if high.is_null() || low.is_null() || close.is_null() || volume.is_null() || out.is_null() {
            return -1;
        }
        if !periods_ok(length, &[period]) {
            return -1;
        }
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        let v = unsafe { std::slice::from_raw_parts(volume, length as usize) };
        match indicators::twiggs_money_flow(h, l, c, v, period as usize) {
            Ok(result) => {
                copy_out_f64(&result, out, length);
                0
            }
            Err(_) => -2,
        }
    })
}

// ============ Volatility ============

#[no_mangle]
pub extern "C" fn ta_vortex(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    period: c_int,
    out_plus: *mut c_double,
    out_minus: *mut c_double,
) -> c_int {
    ffi_catch_i32_neg(|| {
        if high.is_null()
            || low.is_null()
            || close.is_null()
            || out_plus.is_null()
            || out_minus.is_null()
        {
            return -1;
        }
        if !periods_ok(length, &[period]) {
            return -1;
        }
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        match indicators::vortex(h, l, c, period as usize) {
            Ok(result) => {
                let written = copy_out_f64(&result.vi_plus, out_plus, length);
                let dst = unsafe { std::slice::from_raw_parts_mut(out_minus, written) };
                dst.copy_from_slice(&result.vi_minus.as_slice().expect("contiguous")[..written]);
                0
            }
            Err(_) => -2,
        }
    })
}

// ============ Candlestick patterns (integer results) ============

/// Shared guard + dispatch for the four-price pattern detectors.
fn cdl_dispatch<F>(
    open: *const c_double,
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    out: *mut c_int,
    detect: F,
) -> c_int
where
    F: FnOnce(&[f64], &[f64], &[f64], &[f64]) -> finkit::error::Result<Array1<i32>>,
{
    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || out.is_null() || length <= 0
    {
        return -1;
    }
    let o = unsafe { std::slice::from_raw_parts(open, length as usize) };
    let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
    match detect(o, h, l, c) {
        Ok(result) => {
            copy_out_i32(&result, out, length);
            0
        }
        Err(_) => -2,
    }
}

macro_rules! dotnet_cdl_plain {
    ($fname:ident, $detector:path) => {
        #[no_mangle]
        pub extern "C" fn $fname(
            open: *const c_double,
            high: *const c_double,
            low: *const c_double,
            close: *const c_double,
            length: c_int,
            out: *mut c_int,
        ) -> c_int {
            ffi_catch_i32_neg(|| {
                cdl_dispatch(open, high, low, close, length, out, |o, h, l, c| {
                    $detector(o, h, l, c)
                })
            })
        }
    };
}

macro_rules! dotnet_cdl_pct {
    ($fname:ident, $detector:path) => {
        #[no_mangle]
        pub extern "C" fn $fname(
            open: *const c_double,
            high: *const c_double,
            low: *const c_double,
            close: *const c_double,
            length: c_int,
            pct: c_double,
            out: *mut c_int,
        ) -> c_int {
            ffi_catch_i32_neg(|| {
                cdl_dispatch(open, high, low, close, length, out, |o, h, l, c| {
                    $detector(o, h, l, c, pct)
                })
            })
        }
    };
}

dotnet_cdl_pct!(ta_cdl_doji, candlestick::doji);
dotnet_cdl_pct!(ta_cdl_dragonfly_doji, candlestick::dragonfly_doji);
dotnet_cdl_pct!(ta_cdl_gravestone_doji, candlestick::gravestone_doji);
dotnet_cdl_pct!(ta_cdl_long_legged_doji, candlestick::long_legged_doji);
dotnet_cdl_pct!(ta_cdl_marubozu, candlestick::marubozu);
dotnet_cdl_plain!(ta_cdl_hammer, candlestick::hammer);
dotnet_cdl_plain!(ta_cdl_inverted_hammer, candlestick::inverted_hammer);
dotnet_cdl_plain!(ta_cdl_hanging_man, candlestick::hanging_man);
dotnet_cdl_plain!(ta_cdl_shooting_star, candlestick::shooting_star);
dotnet_cdl_plain!(ta_cdl_engulfing, candlestick::engulfing);
dotnet_cdl_plain!(ta_cdl_harami, candlestick::harami);
dotnet_cdl_plain!(ta_cdl_morning_star, candlestick::morning_star);
dotnet_cdl_plain!(ta_cdl_evening_star, candlestick::evening_star);
dotnet_cdl_plain!(ta_cdl_three_white_soldiers, candlestick::three_white_soldiers);
dotnet_cdl_plain!(ta_cdl_three_black_crows, candlestick::three_black_crows);

#[cfg(test)]
mod parity_tests {
    use super::*;

    fn series(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| 100.0 + (i as f64 * 0.11).sin() * 5.0 + i as f64 * 0.05)
            .collect()
    }

    #[test]
    fn parity_dev_zero_on_success_and_negative_on_bad_period() {
        let n = 96_usize;
        let close = series(n);
        let high: Vec<f64> = close.iter().map(|c| c + 1.5).collect();
        let low: Vec<f64> = close.iter().map(|c| c - 1.5).collect();
        let open: Vec<f64> = close.iter().map(|c| c - 0.4).collect();
        let vol: Vec<f64> = (0..n).map(|i| 1_000.0 + i as f64).collect();
        let mut out = vec![f64::NAN; n];

        assert_eq!(
            ta_apo(close.as_ptr(), n as c_int, 5, 20, out.as_mut_ptr()),
            0
        );
        assert!(out.iter().any(|v| v.is_finite()));
        assert_eq!(ta_cmo(close.as_ptr(), n as c_int, 14, out.as_mut_ptr()), 0);
        assert_eq!(ta_trix(close.as_ptr(), n as c_int, 9, out.as_mut_ptr()), 0);
        assert_eq!(
            ta_percent_rank(close.as_ptr(), n as c_int, 20, out.as_mut_ptr()),
            0
        );
        assert_eq!(
            ta_chande_forecast(close.as_ptr(), n as c_int, 14, out.as_mut_ptr()),
            0
        );
        assert_eq!(
            ta_midpoint(close.as_ptr(), n as c_int, 14, out.as_mut_ptr()),
            0
        );
        assert_eq!(
            ta_midprice(high.as_ptr(), low.as_ptr(), n as c_int, 14, out.as_mut_ptr()),
            0
        );
        assert_eq!(
            ta_sar(high.as_ptr(), low.as_ptr(), n as c_int, 0.02, 0.2, out.as_mut_ptr()),
            0
        );
        assert_eq!(
            ta_avgprice(
                open.as_ptr(),
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                n as c_int,
                out.as_mut_ptr()
            ),
            0
        );
        assert_eq!(
            ta_medprice(high.as_ptr(), low.as_ptr(), n as c_int, out.as_mut_ptr()),
            0
        );
        assert_eq!(
            ta_typprice(
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                n as c_int,
                out.as_mut_ptr()
            ),
            0
        );
        assert_eq!(
            ta_wclprice(
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                n as c_int,
                out.as_mut_ptr()
            ),
            0
        );
        assert_eq!(
            ta_bop(
                open.as_ptr(),
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                n as c_int,
                out.as_mut_ptr()
            ),
            0
        );
        assert_eq!(
            ta_mfi(
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                vol.as_ptr(),
                n as c_int,
                14,
                out.as_mut_ptr()
            ),
            0
        );
        assert_eq!(
            ta_vzo(close.as_ptr(), vol.as_ptr(), n as c_int, 14, out.as_mut_ptr()),
            0
        );
        assert_eq!(
            ta_volume_momentum(vol.as_ptr(), n as c_int, 14, out.as_mut_ptr()),
            0
        );
        assert_eq!(
            ta_volume_roc(vol.as_ptr(), n as c_int, 14, out.as_mut_ptr()),
            0
        );
        assert_eq!(
            ta_twiggs_mf(
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                vol.as_ptr(),
                n as c_int,
                21,
                out.as_mut_ptr()
            ),
            0
        );
        assert_eq!(
            ta_inertia(
                open.as_ptr(),
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                n as c_int,
                14,
                20,
                out.as_mut_ptr()
            ),
            0
        );

        // A period larger than the input must fail with a negative code,
        // never 0 -- the .NET ABI contract.
        assert!(ta_cmo(close.as_ptr(), n as c_int, 200, out.as_mut_ptr()) < 0);
        assert!(ta_apo(close.as_ptr(), n as c_int, 5, 200, out.as_mut_ptr()) < 0);
    }

    #[test]
    fn parity_vortex_writes_both_lines() {
        let n = 96_usize;
        let close = series(n);
        let high: Vec<f64> = close.iter().map(|c| c + 1.5).collect();
        let low: Vec<f64> = close.iter().map(|c| c - 1.5).collect();
        let mut plus = vec![f64::NAN; n];
        let mut minus = vec![f64::NAN; n];
        assert_eq!(
            ta_vortex(
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                n as c_int,
                14,
                plus.as_mut_ptr(),
                minus.as_mut_ptr()
            ),
            0
        );
        assert!(plus.iter().any(|v| v.is_finite()));
        assert!(minus.iter().any(|v| v.is_finite()));
    }

    #[test]
    fn parity_candlestick_emits_only_talib_values() {
        let n = 96_usize;
        let close = series(n);
        let high: Vec<f64> = close.iter().map(|c| c + 1.5).collect();
        let low: Vec<f64> = close.iter().map(|c| c - 1.5).collect();
        let open: Vec<f64> = close.iter().map(|c| c - 0.4).collect();
        let mut out = vec![0_i32; n];

        for code in [
            ta_cdl_doji(open.as_ptr(), high.as_ptr(), low.as_ptr(), close.as_ptr(), n as c_int, 0.1, out.as_mut_ptr()),
            ta_cdl_hammer(open.as_ptr(), high.as_ptr(), low.as_ptr(), close.as_ptr(), n as c_int, out.as_mut_ptr()),
            ta_cdl_engulfing(open.as_ptr(), high.as_ptr(), low.as_ptr(), close.as_ptr(), n as c_int, out.as_mut_ptr()),
            ta_cdl_marubozu(open.as_ptr(), high.as_ptr(), low.as_ptr(), close.as_ptr(), n as c_int, 0.1, out.as_mut_ptr()),
        ] {
            assert_eq!(code, 0);
            assert!(
                out.iter().all(|v| matches!(v, -100 | 0 | 100)),
                "pattern detector emitted a value outside -100/0/100"
            );
        }
    }
}
