#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(missing_docs)]
#![allow(missing_debug_implementations)]
#![allow(deprecated)]

use std::ffi::CString;
use std::os::raw::{c_char, c_double, c_int, c_void};

use finkit::formula::{FormulaContext, FormulaEngine};
use finkit::indicators::{
    ad, adosc, adx, aroon, atr, bbands, beta, cci, correlation, dema, ema, ht_dcperiod, ht_dcphase,
    ht_phasor, ht_sine, ht_trendline, ht_trendmode, kama, linear_reg, macd, mom, natr, obv, roc,
    rsi, sma, std_dev, stoch, t3, tema, trange, tsf, willr, wma, zscore,
};
use finkit::streaming::indicators::{
    BollOutput, MacdOutput, StreamingAtr, StreamingBoll, StreamingEma, StreamingMacd, StreamingRsi,
    StreamingSma,
};
use finkit::streaming::StreamingIndicator;
use finkit_ffi_common::panic::*;
use ndarray::Array1;
use serde_json;

// Leak-detection allocator: installed only for the test binary so the FFI
// ownership contract (alloc via `ta_*` + free via `ta_free_*`) can be checked
// for heap growth. See `finkit_ffi_common::leak`.
#[cfg(test)]
#[global_allocator]
static TEST_ALLOC: finkit_ffi_common::leak::CountingAlloc = finkit_ffi_common::leak::CountingAlloc;

#[repr(C)]
pub struct TaResult {
    pub data: *mut f64,
    pub length: c_int,
    pub capacity: c_int,
    pub error: *mut c_char,
}

static VERSION: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();

#[no_mangle]
pub extern "C" fn ta_free_result(result: *mut TaResult) {
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

#[no_mangle]
pub extern "C" fn ta_free_string(s: *mut c_char) {
    ffi_catch_void(|| {
        if !s.is_null() {
            unsafe {
                drop(CString::from_raw(s));
            }
        }
    })
}

fn make_result_from_vec(v: Vec<f64>) -> *mut TaResult {
    let (data_ptr, length, capacity) = v.into_raw_parts();
    Box::into_raw(Box::new(TaResult {
        data: data_ptr,
        length: length as c_int,
        capacity: capacity as c_int,
        error: std::ptr::null_mut(),
    }))
}

fn make_result(data: Array1<f64>) -> *mut TaResult {
    make_result_from_vec(data.into_raw_vec_and_offset().0)
}

fn make_error_result(msg: &str) -> *mut TaResult {
    let c_string = CString::new(msg).unwrap_or_default();
    Box::into_raw(Box::new(TaResult {
        data: std::ptr::null_mut(),
        length: 0,
        capacity: 0,
        error: c_string.into_raw(),
    }))
}

fn validate_input(input: *const c_double, length: c_int) -> Option<&'static [f64]> {
    if input.is_null() || length <= 0 {
        return None;
    }
    Some(unsafe { std::slice::from_raw_parts(input, length as usize) })
}

// ============ Moving Averages ============

#[cfg(test)]
#[no_mangle]
pub extern "C" fn ta_ffi_panic_test() -> *mut TaResult {
    ffi_catch_ptr(|| -> *mut TaResult { panic!("ffi panic injection test") })
}


fn boxed_handle<T>(value: T) -> *mut c_void {
    Box::into_raw(Box::new(value)).cast()
}

#[no_mangle]
pub extern "C" fn ta_streaming_sma_new(period: c_int) -> *mut c_void {
    if period <= 0 { return std::ptr::null_mut(); }
    ffi_catch_ptr(|| boxed_handle(StreamingSma::new(period as usize)))
}

#[no_mangle]
pub extern "C" fn ta_streaming_sma_update(handle: *mut c_void, value: c_double) -> c_double {
    ffi_catch_f64(|| unsafe {
        (handle as *mut StreamingSma).as_mut()
            .and_then(|indicator| indicator.next(value))
            .unwrap_or(f64::NAN)
    })
}

#[no_mangle]
pub extern "C" fn ta_streaming_sma_reset(handle: *mut c_void) {
    ffi_catch_void(|| unsafe {
        if let Some(indicator) = (handle as *mut StreamingSma).as_mut() { indicator.reset(); }
    })
}

#[no_mangle]
pub extern "C" fn ta_streaming_sma_free(handle: *mut c_void) {
    ffi_catch_void(|| unsafe {
        if !handle.is_null() { drop(Box::from_raw(handle as *mut StreamingSma)); }
    })
}

#[no_mangle]
pub extern "C" fn ta_streaming_ema_new(period: c_int) -> *mut c_void {
    if period <= 0 { return std::ptr::null_mut(); }
    ffi_catch_ptr(|| boxed_handle(StreamingEma::new(period as usize)))
}

#[no_mangle]
pub extern "C" fn ta_streaming_ema_update(handle: *mut c_void, value: c_double) -> c_double {
    ffi_catch_f64(|| unsafe {
        (handle as *mut StreamingEma).as_mut()
            .and_then(|indicator| indicator.next(value))
            .unwrap_or(f64::NAN)
    })
}

#[no_mangle]
pub extern "C" fn ta_streaming_ema_reset(handle: *mut c_void) {
    ffi_catch_void(|| unsafe {
        if let Some(indicator) = (handle as *mut StreamingEma).as_mut() { indicator.reset(); }
    })
}

#[no_mangle]
pub extern "C" fn ta_streaming_ema_free(handle: *mut c_void) {
    ffi_catch_void(|| unsafe {
        if !handle.is_null() { drop(Box::from_raw(handle as *mut StreamingEma)); }
    })
}

#[no_mangle]
pub extern "C" fn ta_streaming_rsi_new(period: c_int) -> *mut c_void {
    if period <= 0 { return std::ptr::null_mut(); }
    ffi_catch_ptr(|| boxed_handle(StreamingRsi::new(period as usize)))
}

#[no_mangle]
pub extern "C" fn ta_streaming_rsi_update(handle: *mut c_void, value: c_double) -> c_double {
    ffi_catch_f64(|| unsafe {
        (handle as *mut StreamingRsi).as_mut()
            .and_then(|indicator| indicator.next(value))
            .unwrap_or(f64::NAN)
    })
}

#[no_mangle]
pub extern "C" fn ta_streaming_rsi_reset(handle: *mut c_void) {
    ffi_catch_void(|| unsafe {
        if let Some(indicator) = (handle as *mut StreamingRsi).as_mut() { indicator.reset(); }
    })
}

#[no_mangle]
pub extern "C" fn ta_streaming_rsi_free(handle: *mut c_void) {
    ffi_catch_void(|| unsafe {
        if !handle.is_null() { drop(Box::from_raw(handle as *mut StreamingRsi)); }
    })
}

#[no_mangle]
pub extern "C" fn ta_streaming_macd_new(fast: c_int, slow: c_int, signal: c_int) -> *mut c_void {
    if fast <= 0 || slow <= 0 || signal <= 0 { return std::ptr::null_mut(); }
    ffi_catch_ptr(|| boxed_handle(StreamingMacd::new(fast as usize, slow as usize, signal as usize)))
}

#[no_mangle]
pub extern "C" fn ta_streaming_macd_update(
    handle: *mut c_void, value: c_double, macd_out: *mut c_double,
    signal_out: *mut c_double, hist_out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| unsafe {
        if macd_out.is_null() || signal_out.is_null() || hist_out.is_null() { return 0; }
        let Some(indicator) = (handle as *mut StreamingMacd).as_mut() else { return 0; };
        match indicator.next(value) {
            Some(MacdOutput { macd, signal, histogram }) => {
                *macd_out = macd; *signal_out = signal; *hist_out = histogram; 1
            }
            None => 0,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_streaming_macd_reset(handle: *mut c_void) {
    ffi_catch_void(|| unsafe {
        if let Some(indicator) = (handle as *mut StreamingMacd).as_mut() { indicator.reset(); }
    })
}

#[no_mangle]
pub extern "C" fn ta_streaming_macd_free(handle: *mut c_void) {
    ffi_catch_void(|| unsafe {
        if !handle.is_null() { drop(Box::from_raw(handle as *mut StreamingMacd)); }
    })
}

#[no_mangle]
pub extern "C" fn ta_streaming_bbands_new(period: c_int, nb_dev_up: c_double, nb_dev_dn: c_double) -> *mut c_void {
    if period <= 1 { return std::ptr::null_mut(); }
    ffi_catch_ptr(|| boxed_handle(StreamingBoll::new(period as usize, nb_dev_up, nb_dev_dn)))
}

#[no_mangle]
pub extern "C" fn ta_streaming_bbands_update(
    handle: *mut c_void, value: c_double, upper_out: *mut c_double,
    middle_out: *mut c_double, lower_out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| unsafe {
        if upper_out.is_null() || middle_out.is_null() || lower_out.is_null() { return 0; }
        let Some(indicator) = (handle as *mut StreamingBoll).as_mut() else { return 0; };
        match indicator.next(value) {
            Some(BollOutput { upper, middle, lower }) => {
                *upper_out = upper; *middle_out = middle; *lower_out = lower; 1
            }
            None => 0,
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_streaming_bbands_reset(handle: *mut c_void) {
    ffi_catch_void(|| unsafe {
        if let Some(indicator) = (handle as *mut StreamingBoll).as_mut() { indicator.reset(); }
    })
}

#[no_mangle]
pub extern "C" fn ta_streaming_bbands_free(handle: *mut c_void) {
    ffi_catch_void(|| unsafe {
        if !handle.is_null() { drop(Box::from_raw(handle as *mut StreamingBoll)); }
    })
}

#[no_mangle]
pub extern "C" fn ta_streaming_atr_new(period: c_int) -> *mut c_void {
    if period <= 0 { return std::ptr::null_mut(); }
    ffi_catch_ptr(|| boxed_handle(StreamingAtr::new(period as usize)))
}

#[no_mangle]
pub extern "C" fn ta_streaming_atr_update_hlc(
    handle: *mut c_void, high: c_double, low: c_double, close: c_double,
) -> c_double {
    ffi_catch_f64(|| unsafe {
        (handle as *mut StreamingAtr).as_mut()
            .and_then(|indicator| indicator.next((high, low, close)))
            .unwrap_or(f64::NAN)
    })
}

#[no_mangle]
pub extern "C" fn ta_streaming_atr_reset(handle: *mut c_void) {
    ffi_catch_void(|| unsafe {
        if let Some(indicator) = (handle as *mut StreamingAtr).as_mut() { indicator.reset(); }
    })
}

#[no_mangle]
pub extern "C" fn ta_streaming_atr_free(handle: *mut c_void) {
    ffi_catch_void(|| unsafe {
        if !handle.is_null() { drop(Box::from_raw(handle as *mut StreamingAtr)); }
    })
}

include!("generated.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use finkit::indicators::rsi;
    use finkit::indicators::sma;

    #[test]
    fn export_panic_test_returns_null_not_abort() {
        // A panic inside the generated `ffi_catch_ptr` guard must
        // yield a null sentinel across the FFI boundary, never
        // unwind (which would abort the host process).
        let p = crate::ta_ffi_panic_test();
        assert!(p.is_null());
    }

    #[test]
    fn test_smoke_sma() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&input, 3).unwrap();
        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!((result[2] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_smoke_rsi() {
        let input = vec![44.0, 44.34, 44.09, 43.61, 44.33, 44.83, 45.10];
        let result = rsi(&input, 5).unwrap();
        assert_eq!(result.len(), input.len());
        assert!(result[5].is_finite());
    }

    // A4 — FFI heap-ownership contract: every `ta_*` result that transfers a
    // Rust allocation to the caller (`*mut TaResult` / `*mut c_char`) must be
    // paired with its `ta_free_*`. This loops alloc+free many times and asserts
    // the live-heap bytes return to baseline, catching forgotten frees.
    #[test]
    fn ffi_heap_no_leak_alloc_free_cycle() {
        use finkit_ffi_common::leak::live_bytes;
        let n: c_int = 512;
        let input: Vec<c_double> = (0..n as usize).map(|i| (i as f64).sin()).collect();

        // Warmup: let the std runtime settle its own long-lived allocations so
        // the baseline we snapshot afterwards is stable across the measured loop.
        for _ in 0..16 {
            let r = crate::ta_sma(input.as_ptr(), n, 14);
            crate::ta_free_result(r);
            let src = std::ffi::CString::new("close").unwrap();
            let fe = crate::ta_formula_eval(
                src.as_ptr(),
                input.as_ptr(),
                input.as_ptr(),
                input.as_ptr(),
                input.as_ptr(),
                input.as_ptr(),
                n,
            );
            crate::ta_free_string(fe);
        }
        let baseline = live_bytes();

        for _ in 0..400 {
            // Indicator result → *mut TaResult, freed via ta_free_result.
            let r = crate::ta_sma(input.as_ptr(), n, 14);
            assert!(!r.is_null(), "ta_sma must return a non-null TaResult");
            let len = unsafe { (*r).length };
            assert_eq!(len, n);
            crate::ta_free_result(r);

            // Formula eval → *mut c_char (error or value), freed via ta_free_string.
            let src = std::ffi::CString::new("close").unwrap();
            let fe = crate::ta_formula_eval(
                src.as_ptr(),
                input.as_ptr(),
                input.as_ptr(),
                input.as_ptr(),
                input.as_ptr(),
                input.as_ptr(),
                n,
            );
            assert!(
                !fe.is_null(),
                "ta_formula_eval must return a non-null CString to free"
            );
            crate::ta_free_string(fe);
        }

        let after = live_bytes();
        let growth = (after - baseline).abs();
        assert!(
            growth < 256 * 1024,
            "heap grew by {} bytes across 400 alloc/free cycles (baseline={}, after={}); \
             a forgotten ta_free_* would leak ~{} bytes/iter",
            after - baseline,
            baseline,
            after,
            (after - baseline).max(0) as f64 / 400.0
        );
    }
}

// ============ Momentum Indicators ============

// ============ Volume Indicators ============

#[no_mangle]
pub extern "C" fn ta_ad_osc(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    volume: *const c_double,
    length: c_int,
    fast_period: c_int,
    slow_period: c_int,
) -> *mut TaResult {
    if high.is_null() || low.is_null() || close.is_null() || volume.is_null() || length <= 0 {
        return make_error_result("invalid input");
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    let volume_slice = unsafe { std::slice::from_raw_parts(volume, length as usize) };

    match adosc(
        high_slice,
        low_slice,
        close_slice,
        volume_slice,
        fast_period as usize,
        slow_period as usize,
    ) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
}

// ============ Volatility Indicators ============

// ============ Hilbert Transform Indicators ============

// ============ Statistical Functions ============

#[no_mangle]
pub extern "C" fn ta_std_dev(
    input: *const c_double,
    length: c_int,
    period: c_int,
    nb_dev: c_double,
) -> *mut TaResult {
    let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    match std_dev(slice, period as usize, nb_dev) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
}

#[no_mangle]
pub extern "C" fn ta_version() -> *const c_char {
    VERSION.as_ptr().cast()
}

// ============ Formula Engine ============

#[no_mangle]
pub extern "C" fn ta_formula_eval(
    source: *const c_char,
    open: *const c_double,
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    volume: *const c_double,
    length: c_int,
) -> *mut c_char {
    if source.is_null()
        || open.is_null()
        || high.is_null()
        || low.is_null()
        || close.is_null()
        || volume.is_null()
        || length <= 0
    {
        let err = CString::new("invalid input").unwrap();
        return err.into_raw();
    }

    let source_str = match unsafe { std::ffi::CStr::from_ptr(source) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => {
            let err = CString::new("invalid source encoding").unwrap();
            return err.into_raw();
        }
    };

    let open_slice = unsafe { std::slice::from_raw_parts(open, length as usize) };
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    let volume_slice = unsafe { std::slice::from_raw_parts(volume, length as usize) };

    let open_arr = Array1::from_vec(open_slice.to_vec());
    let high_arr = Array1::from_vec(high_slice.to_vec());
    let low_arr = Array1::from_vec(low_slice.to_vec());
    let close_arr = Array1::from_vec(close_slice.to_vec());
    let volume_arr = Array1::from_vec(volume_slice.to_vec());

    let mut ctx = FormulaContext::new(open_arr, high_arr, low_arr, close_arr, volume_arr, None);
    let mut engine = FormulaEngine::new();

    match engine.eval(&source_str, &mut ctx) {
        Ok(_final_value) => {
            let mut map = serde_json::Map::new();
            for (name, value) in &ctx.variables {
                let arr: Vec<Option<f64>> = value
                    .iter()
                    .map(|v| if v.is_nan() { None } else { Some(*v) })
                    .collect();
                map.insert(
                    name.to_string(),
                    serde_json::Value::Array(
                        arr.into_iter().map(|v| serde_json::json!(v)).collect(),
                    ),
                );
            }
            let json_str = serde_json::to_string(&serde_json::Value::Object(map))
                .unwrap_or_else(|_| "{}".to_string());
            match CString::new(json_str) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    let err = CString::new("output serialization error").unwrap();
                    err.into_raw()
                }
            }
        }
        Err(e) => {
            let err_msg = format!("error: {}", e);
            match CString::new(err_msg) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    let err = CString::new("evaluation error").unwrap();
                    err.into_raw()
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn ta_formula_eval_multi(
    source: *const c_char,
    open: *const c_double,
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    volume: *const c_double,
    length: c_int,
) -> *mut c_char {
    if source.is_null()
        || open.is_null()
        || high.is_null()
        || low.is_null()
        || close.is_null()
        || volume.is_null()
        || length <= 0
    {
        let err = CString::new("invalid input").unwrap();
        return err.into_raw();
    }

    let source_str = match unsafe { std::ffi::CStr::from_ptr(source) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => {
            let err = CString::new("invalid source encoding").unwrap();
            return err.into_raw();
        }
    };

    let open_slice = unsafe { std::slice::from_raw_parts(open, length as usize) };
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    let volume_slice = unsafe { std::slice::from_raw_parts(volume, length as usize) };

    let open_arr = Array1::from_vec(open_slice.to_vec());
    let high_arr = Array1::from_vec(high_slice.to_vec());
    let low_arr = Array1::from_vec(low_slice.to_vec());
    let close_arr = Array1::from_vec(close_slice.to_vec());
    let volume_arr = Array1::from_vec(volume_slice.to_vec());

    let mut ctx = FormulaContext::new(open_arr, high_arr, low_arr, close_arr, volume_arr, None);
    let mut engine = FormulaEngine::new();

    match engine.eval_multi(&source_str, &mut ctx) {
        Ok(multi) => {
            let mut names = Vec::new();
            let mut values = Vec::new();
            for name in multi.names() {
                names.push(name.clone());
                if let Some(arr) = multi.get(name) {
                    values.push(arr.to_vec());
                } else {
                    values.push(vec![]);
                }
            }
            let json_value = serde_json::json!({
                "names": names,
                "values": values,
                "__result__": multi.final_value.to_vec(),
            });
            let json_str = serde_json::to_string(&json_value).unwrap_or_else(|_| "{}".to_string());
            match CString::new(json_str) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    let err = CString::new("output serialization error").unwrap();
                    err.into_raw()
                }
            }
        }
        Err(e) => {
            let err_msg = format!("error: {}", e);
            match CString::new(err_msg) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    let err = CString::new("evaluation error").unwrap();
                    err.into_raw()
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn ta_formula_eval_draw(
    source: *const c_char,
    open: *const c_double,
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    volume: *const c_double,
    length: c_int,
) -> *mut c_char {
    if source.is_null()
        || open.is_null()
        || high.is_null()
        || low.is_null()
        || close.is_null()
        || volume.is_null()
        || length <= 0
    {
        let err = CString::new("invalid input").unwrap();
        return err.into_raw();
    }

    let source_str = match unsafe { std::ffi::CStr::from_ptr(source) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => {
            let err = CString::new("invalid source encoding").unwrap();
            return err.into_raw();
        }
    };

    let open_slice = unsafe { std::slice::from_raw_parts(open, length as usize) };
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    let volume_slice = unsafe { std::slice::from_raw_parts(volume, length as usize) };

    let open_arr = Array1::from_vec(open_slice.to_vec());
    let high_arr = Array1::from_vec(high_slice.to_vec());
    let low_arr = Array1::from_vec(low_slice.to_vec());
    let close_arr = Array1::from_vec(close_slice.to_vec());
    let volume_arr = Array1::from_vec(volume_slice.to_vec());

    let mut ctx = FormulaContext::new(open_arr, high_arr, low_arr, close_arr, volume_arr, None);
    let mut engine = FormulaEngine::new();

    match engine.eval(&source_str, &mut ctx) {
        Ok(_final_value) => {
            let draw_commands = ctx.draw_commands.borrow();
            let json_value = serde_json::json!({
                "drawCommands": &draw_commands.commands,
            });
            let json_str = serde_json::to_string(&json_value).unwrap_or_else(|_| "{}".to_string());
            match CString::new(json_str) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    let err = CString::new("output serialization error").unwrap();
                    err.into_raw()
                }
            }
        }
        Err(e) => {
            let err_msg = format!("error: {}", e);
            match CString::new(err_msg) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    let err = CString::new("evaluation error").unwrap();
                    err.into_raw()
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn ta_formula_eval_debug(
    source: *const c_char,
    open: *const c_double,
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    volume: *const c_double,
    length: c_int,
) -> *mut c_char {
    if source.is_null()
        || open.is_null()
        || high.is_null()
        || low.is_null()
        || close.is_null()
        || volume.is_null()
        || length <= 0
    {
        let err = CString::new("invalid input").unwrap();
        return err.into_raw();
    }

    let source_str = match unsafe { std::ffi::CStr::from_ptr(source) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => {
            let err = CString::new("invalid source encoding").unwrap();
            return err.into_raw();
        }
    };

    let open_slice = unsafe { std::slice::from_raw_parts(open, length as usize) };
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    let volume_slice = unsafe { std::slice::from_raw_parts(volume, length as usize) };

    let open_arr = Array1::from_vec(open_slice.to_vec());
    let high_arr = Array1::from_vec(high_slice.to_vec());
    let low_arr = Array1::from_vec(low_slice.to_vec());
    let close_arr = Array1::from_vec(close_slice.to_vec());
    let volume_arr = Array1::from_vec(volume_slice.to_vec());

    let mut ctx = FormulaContext::new(open_arr, high_arr, low_arr, close_arr, volume_arr, None);
    let mut engine = FormulaEngine::new();

    match engine.eval_with_debug(&source_str, &mut ctx) {
        Ok((_final_value, debugger)) => {
            let json_value = serde_json::json!({
                "events": debugger.get_events(),
            });
            let json_str = serde_json::to_string(&json_value).unwrap_or_else(|_| "{}".to_string());
            match CString::new(json_str) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    let err = CString::new("output serialization error").unwrap();
                    err.into_raw()
                }
            }
        }
        Err(e) => {
            let err_msg = format!("error: {}", e);
            match CString::new(err_msg) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    let err = CString::new("evaluation error").unwrap();
                    err.into_raw()
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn ta_formula_get_template(name: *const c_char) -> *mut c_char {
    use finkit::formula::FormulaEngine;

    if name.is_null() {
        let err = CString::new("invalid input").unwrap();
        return err.into_raw();
    }

    let name_str = match unsafe { std::ffi::CStr::from_ptr(name) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => {
            let err = CString::new("invalid name encoding").unwrap();
            return err.into_raw();
        }
    };

    let engine = FormulaEngine::new();
    match engine.get_template(&name_str) {
        Some(template) => {
            let json_str = serde_json::to_string(template).unwrap_or_else(|_| "{}".to_string());
            match CString::new(json_str) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    let err = CString::new("output serialization error").unwrap();
                    err.into_raw()
                }
            }
        }
        None => {
            let err = CString::new(format!("template '{}' not found", name_str)).unwrap();
            err.into_raw()
        }
    }
}

#[no_mangle]
pub extern "C" fn ta_formula_search_templates(keyword: *const c_char) -> *mut c_char {
    use finkit::formula::FormulaEngine;

    if keyword.is_null() {
        let err = CString::new("invalid input").unwrap();
        return err.into_raw();
    }

    let keyword_str = match unsafe { std::ffi::CStr::from_ptr(keyword) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => {
            let err = CString::new("invalid keyword encoding").unwrap();
            return err.into_raw();
        }
    };

    let engine = FormulaEngine::new();
    let templates = engine.search_templates(&keyword_str);
    let json_str = serde_json::to_string(&templates).unwrap_or_else(|_| "[]".to_string());
    match CString::new(json_str) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => {
            let err = CString::new("output serialization error").unwrap();
            err.into_raw()
        }
    }
}

#[no_mangle]
pub extern "C" fn ta_formula_list_categories() -> *mut c_char {
    use finkit::formula::templates::FormulaTemplates;

    let categories = FormulaTemplates::categories();
    let json_str = serde_json::to_string(&categories).unwrap_or_else(|_| "[]".to_string());
    match CString::new(json_str) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => {
            let err = CString::new("output serialization error").unwrap();
            err.into_raw()
        }
    }
}

// ============================================================================
// Classic stock-trading chart patterns (FTA-native, added 2026-06-06).
// All return *mut c_char — JSON-encoded result. Caller must free with
// ta_free_cstring.
// ============================================================================

fn err_cstr<E: std::fmt::Display>(msg: E) -> *mut c_char {
    let s = format!("{{\"error\":\"{}\"}}", msg);
    CString::new(s)
        .map(|c| c.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "C" fn ta_formula_validate(source: *const c_char) -> c_int {
    if source.is_null() {
        return 0;
    }

    let source_str = match unsafe { std::ffi::CStr::from_ptr(source) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return 0,
    };

    let mut engine = FormulaEngine::new();
    match engine.compile(&source_str) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "C" fn ta_formula_eval_zc_exec(
    source: *const c_char,
    open: *const c_double,
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    volume: *const c_double,
    length: c_int,
) -> *mut c_char {
    if source.is_null()
        || open.is_null()
        || high.is_null()
        || low.is_null()
        || close.is_null()
        || volume.is_null()
        || length <= 0
    {
        let err = CString::new("invalid input").unwrap();
        return err.into_raw();
    }

    let source_str = match unsafe { std::ffi::CStr::from_ptr(source) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => {
            let err = CString::new("invalid source encoding").unwrap();
            return err.into_raw();
        }
    };

    let open_slice = unsafe { std::slice::from_raw_parts(open, length as usize) };
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    let volume_slice = unsafe { std::slice::from_raw_parts(volume, length as usize) };

    let open_arr = Array1::from_vec(open_slice.to_vec());
    let high_arr = Array1::from_vec(high_slice.to_vec());
    let low_arr = Array1::from_vec(low_slice.to_vec());
    let close_arr = Array1::from_vec(close_slice.to_vec());
    let volume_arr = Array1::from_vec(volume_slice.to_vec());

    let mut ctx = FormulaContext::new(open_arr, high_arr, low_arr, close_arr, volume_arr, None);
    let mut engine = FormulaEngine::new();

    match engine.eval_zero_copy(&source_str, &mut ctx) {
        Ok(_final_value) => {
            let mut map = serde_json::Map::new();
            for (name, value) in &ctx.variables {
                let arr: Vec<Option<f64>> = value
                    .iter()
                    .map(|v| if v.is_nan() { None } else { Some(*v) })
                    .collect();
                map.insert(
                    name.to_string(),
                    serde_json::Value::Array(
                        arr.into_iter().map(|v| serde_json::json!(v)).collect(),
                    ),
                );
            }
            let json_str = serde_json::to_string(&serde_json::Value::Object(map))
                .unwrap_or_else(|_| "{}".to_string());
            match CString::new(json_str) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    let err = CString::new("output serialization error").unwrap();
                    err.into_raw()
                }
            }
        }
        Err(e) => {
            let err_msg = format!("error: {}", e);
            match CString::new(err_msg) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    let err = CString::new("evaluation error").unwrap();
                    err.into_raw()
                }
            }
        }
    }
}
