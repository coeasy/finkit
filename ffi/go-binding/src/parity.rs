// ─────────────────────────────────────────────────────────────────────
// Registry-parity additions for the Go binding.
//
// `generated.rs` covers the indicators the Go binding shipped with.  This file
// adds the ones it was missing relative to `docs/ffi_registry.json`, so the Go
// binding exposes the same 78-indicator surface as the C binding.
//
// Audited by: python3 scripts/audit_binding_parity.py
// ─────────────────────────────────────────────────────────────────────

use finkit::indicators;
use finkit::patterns::candlestick;

// NOTE: this file is `include!`d into `lib.rs`, so it shares that module's
// scope. `c_char`/`c_double`/`c_int`/`CString`/`Array1` and the `make_*` /
// `ffi_catch_*` helpers are already in scope from `lib.rs`; importing them
// again here would be a duplicate-import error (E0252).

/// Integer-result carrier, mirroring `TaResult` for candlestick patterns.
///
/// TA-Lib (and the C binding) return pattern detectors as integers
/// (+100 / 0 / -100), not doubles.  Widening them into `TaResult.data` would
/// work numerically but would lie about the type, so the Go binding carries a
/// second, int-typed result instead.
#[repr(C)]
pub struct TaIntResult {
    pub data: *mut c_int,
    pub length: c_int,
    pub capacity: c_int,
    pub error: *mut c_char,
}

fn make_int_result(v: Vec<c_int>) -> *mut TaIntResult {
    let (data_ptr, length, capacity) = v.into_raw_parts();
    Box::into_raw(Box::new(TaIntResult {
        data: data_ptr,
        length: length as c_int,
        capacity: capacity as c_int,
        error: std::ptr::null_mut(),
    }))
}

fn make_int_error_result(msg: &str) -> *mut TaIntResult {
    let c_string = CString::new(msg).unwrap_or_default();
    Box::into_raw(Box::new(TaIntResult {
        data: std::ptr::null_mut(),
        length: 0,
        capacity: 0,
        error: c_string.into_raw(),
    }))
}

#[no_mangle]
pub extern "C" fn ta_free_int_result(result: *mut TaIntResult) {
    ffi_catch_void(|| {
        if result.is_null() {
            return;
        }
        unsafe {
            let boxed = Box::from_raw(result);
            if !boxed.data.is_null() {
                drop(Vec::from_raw_parts(
                    boxed.data,
                    boxed.length as usize,
                    boxed.capacity as usize,
                ));
            }
            if !boxed.error.is_null() {
                drop(CString::from_raw(boxed.error));
            }
        }
    })
}

/// Concatenates one or more equal-length result columns into the single
/// `TaResult` buffer the Go layer splits back apart by `length`.
fn concat_cols(cols: &[ndarray::Array1<f64>]) -> *mut TaResult {
    let total: usize = cols.iter().map(|c| c.len()).sum();
    let mut out = Vec::with_capacity(total);
    for col in cols {
        out.extend_from_slice(col.as_slice().expect("Array1 is contiguous"));
    }
    make_result_from_vec(out)
}

/// Shared validation for the two-period indicators.
fn valid_periods(length: c_int, periods: &[c_int]) -> bool {
    length > 0 && periods.iter().all(|p| *p > 0 && (*p as usize) <= length as usize)
}

// ============ Momentum / trend ============

#[no_mangle]
pub extern "C" fn ta_apo(
    input: *const c_double,
    length: c_int,
    fast_period: c_int,
    slow_period: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        let data = match validate_input(input, length) {
            Some(s) => s,
            None => return make_error_result("invalid input"),
        };
        if !valid_periods(length, &[fast_period, slow_period]) {
            return make_error_result("invalid period");
        }
        match indicators::apo(data, fast_period as usize, slow_period as usize) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_cmo(
    input: *const c_double,
    length: c_int,
    period: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        let data = match validate_input(input, length) {
            Some(s) => s,
            None => return make_error_result("invalid input"),
        };
        if !valid_periods(length, &[period]) {
            return make_error_result("invalid period");
        }
        match indicators::cmo(data, period as usize) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_trix(
    input: *const c_double,
    length: c_int,
    period: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        let data = match validate_input(input, length) {
            Some(s) => s,
            None => return make_error_result("invalid input"),
        };
        if !valid_periods(length, &[period]) {
            return make_error_result("invalid period");
        }
        match indicators::trix(data, period as usize) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_percent_rank(
    input: *const c_double,
    length: c_int,
    period: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        let data = match validate_input(input, length) {
            Some(s) => s,
            None => return make_error_result("invalid input"),
        };
        if !valid_periods(length, &[period]) {
            return make_error_result("invalid period");
        }
        match indicators::percent_rank(data, period as usize) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_mama(
    input: *const c_double,
    length: c_int,
    fast_limit: c_double,
    slow_limit: c_double,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        let data = match validate_input(input, length) {
            Some(s) => s,
            None => return make_error_result("invalid input"),
        };
        match indicators::mama(data, fast_limit, slow_limit) {
            Ok(result) => concat_cols(&[result.mama, result.fama]),
            Err(e) => make_error_result(&format!("{e}")),
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
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        if high.is_null() || low.is_null() || length <= 0 {
            return make_error_result("invalid input");
        }
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        match indicators::sar(h, l, acceleration, maximum) {
            Ok(result) => make_result(result.sar),
            Err(e) => make_error_result(&format!("{e}")),
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
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        if open.is_null() || high.is_null() || low.is_null() || close.is_null() || length <= 0 {
            return make_error_result("invalid input");
        }
        let o = unsafe { std::slice::from_raw_parts(open, length as usize) };
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        match indicators::avgprice(o, h, l, c) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_medprice(
    high: *const c_double,
    low: *const c_double,
    length: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        if high.is_null() || low.is_null() || length <= 0 {
            return make_error_result("invalid input");
        }
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        match indicators::medprice(h, l) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_typprice(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        if high.is_null() || low.is_null() || close.is_null() || length <= 0 {
            return make_error_result("invalid input");
        }
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        match indicators::typprice(h, l, c) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_wclprice(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        if high.is_null() || low.is_null() || close.is_null() || length <= 0 {
            return make_error_result("invalid input");
        }
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        match indicators::wclprice(h, l, c) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_midpoint(
    input: *const c_double,
    length: c_int,
    period: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        let data = match validate_input(input, length) {
            Some(s) => s,
            None => return make_error_result("invalid input"),
        };
        if !valid_periods(length, &[period]) {
            return make_error_result("invalid period");
        }
        match indicators::midpoint(data, period as usize) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_midprice(
    high: *const c_double,
    low: *const c_double,
    length: c_int,
    period: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        if high.is_null() || low.is_null() || !valid_periods(length, &[period]) {
            return make_error_result("invalid input");
        }
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        match indicators::midprice(h, l, period as usize) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
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
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        if open.is_null() || high.is_null() || low.is_null() || close.is_null() || length <= 0 {
            return make_error_result("invalid input");
        }
        let o = unsafe { std::slice::from_raw_parts(open, length as usize) };
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        match indicators::bop(o, h, l, c) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
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
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        if high.is_null()
            || low.is_null()
            || close.is_null()
            || volume.is_null()
            || !valid_periods(length, &[period])
        {
            return make_error_result("invalid input");
        }
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        let v = unsafe { std::slice::from_raw_parts(volume, length as usize) };
        match indicators::mfi(h, l, c, v, period as usize) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_vzo(
    close: *const c_double,
    volume: *const c_double,
    length: c_int,
    period: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        if close.is_null() || volume.is_null() || !valid_periods(length, &[period]) {
            return make_error_result("invalid input");
        }
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        let v = unsafe { std::slice::from_raw_parts(volume, length as usize) };
        match indicators::vzo(c, v, period as usize) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_volume_momentum(
    volume: *const c_double,
    length: c_int,
    period: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        let v = match validate_input(volume, length) {
            Some(s) => s,
            None => return make_error_result("invalid input"),
        };
        if !valid_periods(length, &[period]) {
            return make_error_result("invalid period");
        }
        match indicators::volume_momentum(v, period as usize) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_volume_roc(
    volume: *const c_double,
    length: c_int,
    period: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        let v = match validate_input(volume, length) {
            Some(s) => s,
            None => return make_error_result("invalid input"),
        };
        if !valid_periods(length, &[period]) {
            return make_error_result("invalid period");
        }
        match indicators::volume_roc(v, period as usize) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
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
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        if high.is_null()
            || low.is_null()
            || close.is_null()
            || volume.is_null()
            || !valid_periods(length, &[period])
        {
            return make_error_result("invalid input");
        }
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        let v = unsafe { std::slice::from_raw_parts(volume, length as usize) };
        match indicators::twiggs_money_flow(h, l, c, v, period as usize) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
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
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        if high.is_null() || low.is_null() || close.is_null() || !valid_periods(length, &[period]) {
            return make_error_result("invalid input");
        }
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        match indicators::vortex(h, l, c, period as usize) {
            Ok(result) => concat_cols(&[result.vi_plus, result.vi_minus]),
            Err(e) => make_error_result(&format!("{e}")),
        }
    })
}

// ============ Multi-input composites ============

#[no_mangle]
pub extern "C" fn ta_chande_forecast(
    close: *const c_double,
    length: c_int,
    period: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        let c = match validate_input(close, length) {
            Some(s) => s,
            None => return make_error_result("invalid input"),
        };
        if !valid_periods(length, &[period]) {
            return make_error_result("invalid period");
        }
        match indicators::chande_forecast_oscillator(c, period as usize) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
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
) -> *mut TaResult {
    ffi_catch_ptr(|| {
        if open.is_null() || high.is_null() || low.is_null() || close.is_null() {
            return make_error_result("invalid input");
        }
        if !valid_periods(length, &[rvi_period, linreg_period]) {
            return make_error_result("invalid period");
        }
        let o = unsafe { std::slice::from_raw_parts(open, length as usize) };
        let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
        let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
        let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
        match indicators::inertia(o, h, l, c, rvi_period as usize, linreg_period as usize) {
            Ok(result) => make_result(result),
            Err(e) => make_error_result(&format!("{e}")),
        }
    })
}

// ============ Candlestick patterns (integer results) ============

/// Shared call shape for the four-price pattern detectors.
unsafe fn cdl_core(
    open: *const c_double,
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    detect: impl FnOnce(
        &[f64],
        &[f64],
        &[f64],
        &[f64],
    ) -> finkit::error::Result<ndarray::Array1<i32>>,
) -> *mut TaIntResult {
    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || length <= 0 {
        return make_int_error_result("invalid input");
    }
    let o = unsafe { std::slice::from_raw_parts(open, length as usize) };
    let h = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let l = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let c = unsafe { std::slice::from_raw_parts(close, length as usize) };
    match detect(o, h, l, c) {
        Ok(result) => make_int_result(result.into_raw_vec_and_offset().0),
        Err(e) => make_int_error_result(&format!("{e}")),
    }
}

macro_rules! cdl_plain {
    ($fname:ident, $detector:path) => {
        #[no_mangle]
        pub extern "C" fn $fname(
            open: *const c_double,
            high: *const c_double,
            low: *const c_double,
            close: *const c_double,
            length: c_int,
        ) -> *mut TaIntResult {
            ffi_catch_ptr(|| unsafe { cdl_core(open, high, low, close, length, $detector) })
        }
    };
}

macro_rules! cdl_threshold {
    ($fname:ident, $detector:path) => {
        #[no_mangle]
        pub extern "C" fn $fname(
            open: *const c_double,
            high: *const c_double,
            low: *const c_double,
            close: *const c_double,
            length: c_int,
            pct: c_double,
        ) -> *mut TaIntResult {
            ffi_catch_ptr(|| unsafe {
                cdl_core(open, high, low, close, length, |o, h, l, c| $detector(o, h, l, c, pct))
            })
        }
    };
}

cdl_threshold!(ta_cdl_doji, candlestick::doji);
cdl_threshold!(ta_cdl_dragonfly_doji, candlestick::dragonfly_doji);
cdl_threshold!(ta_cdl_gravestone_doji, candlestick::gravestone_doji);
cdl_threshold!(ta_cdl_long_legged_doji, candlestick::long_legged_doji);
cdl_plain!(ta_cdl_hammer, candlestick::hammer);
cdl_plain!(ta_cdl_inverted_hammer, candlestick::inverted_hammer);
cdl_plain!(ta_cdl_hanging_man, candlestick::hanging_man);
cdl_plain!(ta_cdl_shooting_star, candlestick::shooting_star);
cdl_plain!(ta_cdl_engulfing, candlestick::engulfing);
cdl_plain!(ta_cdl_harami, candlestick::harami);
cdl_plain!(ta_cdl_morning_star, candlestick::morning_star);
cdl_plain!(ta_cdl_evening_star, candlestick::evening_star);
cdl_plain!(ta_cdl_three_white_soldiers, candlestick::three_white_soldiers);
cdl_plain!(ta_cdl_three_black_crows, candlestick::three_black_crows);
cdl_threshold!(ta_cdl_marubozu, candlestick::marubozu);

#[cfg(test)]
mod parity_tests {
    use super::*;

    fn series(n: usize) -> Vec<c_double> {
        (0..n)
            .map(|i| 100.0 + (i as f64 * 0.11).sin() * 5.0 + i as f64 * 0.05)
            .collect()
    }

    /// Asserts a parity entry point succeeded with `want_len` values, then
    /// releases the buffer and returns its length.
    fn expect_ok(result: *mut TaResult, want_len: c_int) -> usize {
        assert!(!result.is_null(), "parity entry point returned null");
        unsafe {
            assert!(
                (*result).error.is_null(),
                "unexpected error: {:?}",
                CStr::from_ptr((*result).error).to_string_lossy()
            );
            assert_eq!((*result).length, want_len);
            let len = (*result).length as usize;
            crate::ta_free_result(result);
            len
        }
    }

    fn expect_err(result: *mut TaResult) {
        assert!(!result.is_null(), "error path must still return a TaResult");
        unsafe {
            assert!(
                !(*result).error.is_null(),
                "invalid period must populate TaResult.error"
            );
            crate::ta_free_result(result);
        }
    }

    /// One call per new single-output entry point: exercises the null/period
    /// guards and the `make_result` ownership transfer.
    #[test]
    fn parity_single_output_indicators() {
        let n: c_int = 96;
        let close = series(n as usize);
        let high: Vec<c_double> = close.iter().map(|c| c + 1.5).collect();
        let low: Vec<c_double> = close.iter().map(|c| c - 1.5).collect();
        let open: Vec<c_double> = close.iter().map(|c| c - 0.4).collect();
        let vol: Vec<c_double> = (0..n as usize).map(|i| 1_000.0 + i as f64).collect();

        expect_ok(crate::ta_apo(close.as_ptr(), n, 5, 20), n);
            expect_ok(crate::ta_cmo(close.as_ptr(), n, 14), n);
            expect_ok(crate::ta_trix(close.as_ptr(), n, 9), n);
            expect_ok(crate::ta_percent_rank(close.as_ptr(), n, 20), n);
            expect_ok(crate::ta_chande_forecast(close.as_ptr(), n, 14), n);
            expect_ok(crate::ta_midpoint(close.as_ptr(), n, 14), n);
            expect_ok(crate::ta_midprice(high.as_ptr(), low.as_ptr(), n, 14), n);
            expect_ok(crate::ta_sar(high.as_ptr(), low.as_ptr(), n, 0.02, 0.2), n);
            expect_ok(
                crate::ta_avgprice(open.as_ptr(), high.as_ptr(), low.as_ptr(), close.as_ptr(), n),
                n,
            );
            expect_ok(crate::ta_medprice(high.as_ptr(), low.as_ptr(), n), n);
            expect_ok(crate::ta_typprice(high.as_ptr(), low.as_ptr(), close.as_ptr(), n), n);
            expect_ok(crate::ta_wclprice(high.as_ptr(), low.as_ptr(), close.as_ptr(), n), n);
            expect_ok(
                crate::ta_bop(open.as_ptr(), high.as_ptr(), low.as_ptr(), close.as_ptr(), n),
                n,
            );
            expect_ok(
                crate::ta_mfi(high.as_ptr(), low.as_ptr(), close.as_ptr(), vol.as_ptr(), n, 14),
                n,
            );
            expect_ok(crate::ta_vzo(close.as_ptr(), vol.as_ptr(), n, 14), n);
            expect_ok(crate::ta_volume_momentum(vol.as_ptr(), n, 14), n);
            expect_ok(crate::ta_volume_roc(vol.as_ptr(), n, 14), n);
            expect_ok(
                crate::ta_twiggs_mf(
                    high.as_ptr(),
                    low.as_ptr(),
                    close.as_ptr(),
                    vol.as_ptr(),
                    n,
                    21,
                ),
                n,
            );
            expect_ok(
                crate::ta_inertia(
                    open.as_ptr(),
                    high.as_ptr(),
                    low.as_ptr(),
                    close.as_ptr(),
                    n,
                    14,
                    20,
                ),
                n,
            );
    }

    /// Two-line results are concatenated; the Go layer splits them back by
    /// `length`, so a 2n buffer is the contract, not an accident.
    #[test]
    fn parity_two_output_indicators_concatenate_by_length() {
        let n: c_int = 96;
        let close = series(n as usize);
        let high: Vec<c_double> = close.iter().map(|c| c + 1.5).collect();
        let low: Vec<c_double> = close.iter().map(|c| c - 1.5).collect();
        expect_ok(
            crate::ta_vortex(high.as_ptr(), low.as_ptr(), close.as_ptr(), n, 14),
            n * 2,
        );
        expect_ok(crate::ta_mama(close.as_ptr(), n, 0.5, 0.05), n * 2);
    }

    #[test]
    fn parity_rejects_period_larger_than_length() {
        let n: c_int = 8;
        let close = series(n as usize);
        expect_err(crate::ta_apo(close.as_ptr(), n, 5, 64));
        expect_err(crate::ta_cmo(close.as_ptr(), n, 64));
        expect_err(crate::ta_mfi(
            close.as_ptr(),
            close.as_ptr(),
            close.as_ptr(),
            close.as_ptr(),
            n,
            64,
        ));
    }

    #[test]
    fn parity_candlestick_returns_integers() {
        let n: c_int = 96;
        let close = series(n as usize);
        let high: Vec<c_double> = close.iter().map(|c| c + 1.5).collect();
        let low: Vec<c_double> = close.iter().map(|c| c - 1.5).collect();
        let open: Vec<c_double> = close.iter().map(|c| c - 0.4).collect();

        let r = crate::ta_cdl_doji(
            open.as_ptr(),
            high.as_ptr(),
            low.as_ptr(),
            close.as_ptr(),
            n,
            0.1,
        );
        assert!(!r.is_null(), "ta_cdl_doji returned null");
        unsafe {
            assert!((*r).error.is_null());
            assert_eq!((*r).length, n);
            let values = std::slice::from_raw_parts((*r).data, n as usize);
            // TA-Lib contract: a pattern detector emits only +100 / 0 / -100.
            assert!(
                values.iter().all(|v| matches!(v, -100 | 0 | 100)),
                "unexpected pattern value outside -100/0/100"
            );
            crate::ta_free_int_result(r);
        }
    }

    /// The int carrier has its own alloc/free pair; a forgotten
    /// `ta_free_int_result` would leak both the `Vec<i32>` and the box.
    #[test]
    fn parity_int_result_no_leak_alloc_free_cycle() {
        use finkit_ffi_common::leak::live_bytes;
        let n: c_int = 256;
        let close = series(n as usize);
        let high: Vec<c_double> = close.iter().map(|c| c + 1.5).collect();
        let low: Vec<c_double> = close.iter().map(|c| c - 1.5).collect();
        let open: Vec<c_double> = close.iter().map(|c| c - 0.4).collect();

        let call = || {
            crate::ta_cdl_hammer(
                open.as_ptr(),
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                n,
            )
        };

        for _ in 0..16 {
            crate::ta_free_int_result(call());
        }
        let baseline = live_bytes();

        for _ in 0..400 {
            let r = call();
            assert!(!r.is_null());
            crate::ta_free_int_result(r);
        }

        let after = live_bytes();
        assert!(
            (after - baseline).abs() < 256 * 1024,
            "heap grew by {} bytes across 400 int-result alloc/free cycles",
            after - baseline
        );
    }
}
