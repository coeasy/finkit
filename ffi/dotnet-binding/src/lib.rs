#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(missing_docs)]
#![allow(missing_debug_implementations)]

use std::collections::HashMap;
use std::os::raw::{c_char, c_double, c_int};

use finkit::formula::{FormulaContext, FormulaEngine};
use finkit::indicators;
use finkit::math::moving_avg;
use finkit_ffi_common::panic::*;
use ndarray::Array1;
use serde::Serialize;
use std::ffi::{CStr, CString};

// Leak-detection allocator: installed only for the test binary so the FFI
// ownership contract (alloc via `ta_*` + free via `ta_free_*`) can be checked
// for heap growth. See `finkit_ffi_common::leak`.
#[cfg(test)]
#[global_allocator]
static TEST_ALLOC: finkit_ffi_common::leak::CountingAlloc = finkit_ffi_common::leak::CountingAlloc;

/// 将 `f64` 转为 JSON 友好的 `Option<f64>`，NaN/Inf 序列化为 `null`。
#[inline]
fn f64_to_json(v: f64) -> Option<f64> {
    if v.is_finite() {
        Some(v)
    } else {
        None
    }
}

/// 将 `Array1<f64>` 转为 `Vec<Option<f64>>`。
#[inline]
fn arr_to_json(arr: &Array1<f64>) -> Vec<Option<f64>> {
    arr.iter().map(|v| f64_to_json(*v)).collect()
}

#[derive(Serialize)]
struct MultiOutputDto {
    names: Vec<String>,
    values: Vec<Vec<Option<f64>>>,
    #[serde(rename = "__result__")]
    result: Vec<Option<f64>>,
}

// ============================================================================
// Memory Management
// ============================================================================

static VERSION: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();

#[no_mangle]
pub extern "C" fn ta_free(ptr: *mut c_double) {
    ffi_catch_void(|| {
        if ptr.is_null() {
            return;
        }
        unsafe {
            drop(Box::from_raw(ptr));
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_free_array(ptr: *mut c_double, length: c_int) {
    ffi_catch_void(|| {
        if ptr.is_null() || length <= 0 {
            return;
        }
        unsafe {
            drop(Vec::from_raw_parts(ptr, length as usize, length as usize));
        }
    })
}

#[no_mangle]
pub extern "C" fn ta_free_cstring(s: *mut c_char) {
    ffi_catch_void(|| {
        if !s.is_null() {
            unsafe {
                let _ = CString::from_raw(s);
            }
        }
    })
}

// ============================================================================
// Overlap Studies
// ============================================================================

#[cfg(test)]
#[no_mangle]
pub extern "C" fn ta_ffi_panic_test() -> c_int {
    ffi_catch_i32(|| -> c_int { panic!("ffi panic injection test") })
}

include!("generated.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use finkit::indicators::momentum::rsi;
    use finkit::math::moving_avg::sma;

    #[test]
    fn export_panic_test_returns_zero_not_abort() {
        // A panic inside the generated `ffi_catch_i32` guard must
        // yield a 0 sentinel, never unwind across the FFI boundary.
        let code = crate::ta_ffi_panic_test();
        assert_eq!(code, 0);
    }

    #[test]
    fn test_smoke_sma() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&input, 3).unwrap();
        // sma returns an array of same length as input, first (period-1) values are NaN
        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!((result[2] - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_smoke_rsi() {
        let input = vec![44.0, 44.34, 44.09, 43.61, 44.33, 44.83, 45.10];
        let result = rsi(&input, 5).unwrap();
        assert_eq!(result.len(), input.len());
        assert!(result[0].is_nan()); // first period-1 values are NaN
        assert!((result[5] - 50.0).abs() < 100.0); // just check it's a valid value
    }

    // A4 — FFI heap-ownership contract. .NET indicators write into a caller
    // buffer (no transfer); the only Rust-heap-returning path is
    // `ta_formula_eval` → `*mut c_char`, freed by `ta_free_cstring`. We also
    // exercise `ta_free` / `ta_free_array` directly. Loops assert the live
    // heap returns to baseline (catches a forgotten free).
    //
    // NOTE: `ta_formula_eval` borrows the NUL-terminated source string for the
    // duration of the call; the caller-owned CString remains valid until return.
    #[test]
    fn ffi_heap_no_leak_formula_eval_cycle() {
        use finkit_ffi_common::leak::live_bytes;
        let n: c_int = 512;
        let input: Vec<c_double> = (0..n as usize).map(|i| (i as f64).sin()).collect();

        for _ in 0..16 {
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
            crate::ta_free_cstring(fe);
        }
        let baseline = live_bytes();

        for _ in 0..400 {
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
            crate::ta_free_cstring(fe);

            // Free-path smoke: a scalar and an array handed back to the free fns.
            let p = Box::into_raw(Box::new(std::f64::consts::PI));
            crate::ta_free(p);
            let v = vec![1.0f64, 2.0, 3.0];
            let (vp, vl, _vc) = v.into_raw_parts();
            crate::ta_free_array(vp, vl as c_int);
        }

        let after = live_bytes();
        let growth = (after - baseline).abs();
        assert!(
            growth < 256 * 1024,
            "heap grew by {} bytes across 400 alloc/free cycles (baseline={}, after={})",
            after - baseline,
            baseline,
            after
        );
    }

    // T3 — FFI error surfacing. A user-reachable formula error (unknown
    // function) must be returned as an error *string* (non-null), not swallowed
    // into a null by the A3 `catch_unwind` guard. The Rust core returns
    // `Err(FormulaError)`; `ta_formula_eval` maps that to `format!("error: {}",
    // e)`. This guards the A3+T3 integration end-to-end.
    #[test]
    fn ffi_formula_eval_surfaces_error_not_null() {
        let n: c_int = 64;
        let input: Vec<c_double> = (0..n as usize).map(|i| (i as f64).sin()).collect();
        // `ta_formula_eval` borrows `source`; keep the caller-owned CString alive
        // until the native call returns.
        let src = std::ffi::CString::new("FOOBAR(CLOSE, 20)").unwrap();
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
            "bad formula must return an error string, not a null (swallowed panic)"
        );
        let msg = unsafe { std::ffi::CString::from_raw(fe) };
        let s = msg.to_string_lossy().to_lowercase();
        assert!(
            s.contains("error"),
            "error result should contain 'error', got: {:?}",
            s
        );
        // `from_raw` above already took ownership back; nothing to free.
    }
}

// ============================================================================
// Momentum Indicators
// ============================================================================

// ============================================================================
// Volume Indicators
// ============================================================================

#[no_mangle]
pub extern "C" fn ta_ad_osc(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    volume: *const c_double,
    length: c_int,
    fast_period: c_int,
    slow_period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {

    if high.is_null()
        || low.is_null()
        || close.is_null()
        || volume.is_null()
        || out.is_null()
        || length <= 0
    {
        return -1;
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    let volume_slice = unsafe { std::slice::from_raw_parts(volume, length as usize) };
    match indicators::adosc(
        high_slice,
        low_slice,
        close_slice,
        volume_slice,
        fast_period as usize,
        slow_period as usize,
    ) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }

    })
}

// ============================================================================
// Volatility Indicators
// ============================================================================

// ============================================================================
// Hilbert Transform Indicators
// ============================================================================

// ============================================================================
// Statistics Indicators
// ============================================================================

#[no_mangle]
pub extern "C" fn ta_std_dev(
    input: *const c_double,
    length: c_int,
    period: c_int,
    nb_dev: c_double,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {

    if input.is_null() || out.is_null() || length <= 0 || period <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match indicators::std_dev(input_slice, period as usize, nb_dev) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }

    })
}

// ============================================================================
// Mama (MESA Adaptive Moving Average)
// ============================================================================

// ============================================================================
// Utility
// ============================================================================

#[no_mangle]
pub extern "C" fn ta_version() -> *const c_char {
    VERSION.as_ptr().cast()
}

// ============================================================================
// Formula Engine
// ============================================================================

enum FormulaEvalMode { Standard, Jit, Simd }

fn read_c_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() { return None; }
    unsafe { CStr::from_ptr(ptr) }.to_str().ok().map(str::to_owned)
}

fn eval_formula_mode(
    source: *const c_char, open: *const c_double, high: *const c_double,
    low: *const c_double, close: *const c_double, volume: *const c_double,
    length: c_int, mode: FormulaEvalMode,
) -> *mut c_char {
    if source.is_null() || open.is_null() || high.is_null() || low.is_null()
        || close.is_null() || volume.is_null() || length <= 0
    {
        return CString::new("invalid input").unwrap().into_raw();
    }
    let Some(source_str) = read_c_string(source) else {
        return CString::new("invalid utf-8").unwrap().into_raw();
    };
    let length = length as usize;
    let open_arr = Array1::from(unsafe { std::slice::from_raw_parts(open, length) }.to_vec());
    let high_arr = Array1::from(unsafe { std::slice::from_raw_parts(high, length) }.to_vec());
    let low_arr = Array1::from(unsafe { std::slice::from_raw_parts(low, length) }.to_vec());
    let close_arr = Array1::from(unsafe { std::slice::from_raw_parts(close, length) }.to_vec());
    let volume_arr = Array1::from(unsafe { std::slice::from_raw_parts(volume, length) }.to_vec());
    let mut ctx = FormulaContext::new(open_arr, high_arr, low_arr, close_arr, volume_arr, None);
    let mut engine = FormulaEngine::new();
    let evaluation = match mode {
        FormulaEvalMode::Standard => engine.eval(&source_str, &mut ctx),
        FormulaEvalMode::Jit => engine.eval_jit(&source_str, &mut ctx),
        FormulaEvalMode::Simd => engine.eval_simd(&source_str, &mut ctx),
    };
    match evaluation {
        Ok(_) => {
            let var_map: HashMap<String, Vec<Option<f64>>> = ctx.variables.iter()
                .map(|(name, value)| (name.to_string(), arr_to_json(value))).collect();
            let json = serde_json::to_string(&var_map).unwrap_or_else(|_| "{}".to_string());
            CString::new(json)
                .unwrap_or_else(|_| CString::new("output serialization error").unwrap())
                .into_raw()
        }
        Err(error) => CString::new(format!("error: {}", error))
            .unwrap_or_else(|_| CString::new("evaluation error").unwrap())
            .into_raw(),
    }
}

#[no_mangle]
pub extern "C" fn ta_formula_eval(
    source: *const c_char, open: *const c_double, high: *const c_double,
    low: *const c_double, close: *const c_double, volume: *const c_double,
    length: c_int,
) -> *mut c_char {
    ffi_catch_ptr(|| eval_formula_mode(
        source, open, high, low, close, volume, length, FormulaEvalMode::Standard,
    ))
}

#[no_mangle]
pub extern "C" fn ta_formula_eval_jit(
    source: *const c_char, open: *const c_double, high: *const c_double,
    low: *const c_double, close: *const c_double, volume: *const c_double,
    length: c_int,
) -> *mut c_char {
    ffi_catch_ptr(|| eval_formula_mode(
        source, open, high, low, close, volume, length, FormulaEvalMode::Jit,
    ))
}

#[no_mangle]
pub extern "C" fn ta_formula_eval_simd(
    source: *const c_char, open: *const c_double, high: *const c_double,
    low: *const c_double, close: *const c_double, volume: *const c_double,
    length: c_int,
) -> *mut c_char {
    ffi_catch_ptr(|| eval_formula_mode(
        source, open, high, low, close, volume, length, FormulaEvalMode::Simd,
    ))
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
    ffi_catch_ptr(|| {

    if source.is_null()
        || open.is_null()
        || high.is_null()
        || low.is_null()
        || close.is_null()
        || volume.is_null()
        || length <= 0
    {
        let err = CString::new("invalid input").unwrap();
        // Caller must free with ta_free_cstring.
        return err.into_raw();
    }

    let source_str = match unsafe { CStr::from_ptr(source) }.to_str().map(str::to_owned) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
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
            let names: Vec<String> = multi.names().iter().map(|n| (*n).clone()).collect();
            let values: Vec<Vec<Option<f64>>> = names
                .iter()
                .map(|name| {
                    multi
                        .get(name)
                        .map(|arr| arr_to_json(arr))
                        .unwrap_or_default()
                })
                .collect();
            let result = arr_to_json(&multi.final_value);
            let dto = MultiOutputDto {
                names,
                values,
                result,
            };
            let json_str = serde_json::to_string(&dto).unwrap_or_else(|_| "{}".to_string());
            // Caller must free with ta_free_cstring.
            match CString::new(json_str) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    // Caller must free with ta_free_cstring.
                    let err = CString::new("output serialization error").unwrap();
                    err.into_raw()
                }
            }
        }
        Err(e) => {
            let err_msg = format!("error: {}", e);
            // Caller must free with ta_free_cstring.
            match CString::new(err_msg) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    // Caller must free with ta_free_cstring.
                    let err = CString::new("evaluation error").unwrap();
                    err.into_raw()
                }
            }
        }
    }

    })
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
    ffi_catch_ptr(|| {

    if source.is_null()
        || open.is_null()
        || high.is_null()
        || low.is_null()
        || close.is_null()
        || volume.is_null()
        || length <= 0
    {
        let err = CString::new("invalid input").unwrap();
        // Caller must free with ta_free_cstring.
        return err.into_raw();
    }

    let source_str = match unsafe { CStr::from_ptr(source) }.to_str().map(str::to_owned) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
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
            let payload = serde_json::json!({
                "drawCommands": draw_commands.commands,
            });
            let json_str = serde_json::to_string(&payload)
                .unwrap_or_else(|_| "{\"drawCommands\":[]}".to_string());
            // Caller must free with ta_free_cstring.
            match CString::new(json_str) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    // Caller must free with ta_free_cstring.
                    let err = CString::new("output serialization error").unwrap();
                    err.into_raw()
                }
            }
        }
        Err(e) => {
            let err_msg = format!("error: {}", e);
            // Caller must free with ta_free_cstring.
            match CString::new(err_msg) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    // Caller must free with ta_free_cstring.
                    let err = CString::new("evaluation error").unwrap();
                    err.into_raw()
                }
            }
        }
    }

    })
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
    ffi_catch_ptr(|| {

    if source.is_null()
        || open.is_null()
        || high.is_null()
        || low.is_null()
        || close.is_null()
        || volume.is_null()
        || length <= 0
    {
        let err = CString::new("invalid input").unwrap();
        // Caller must free with ta_free_cstring.
        return err.into_raw();
    }

    let source_str = match unsafe { CStr::from_ptr(source) }.to_str().map(str::to_owned) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
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
            let payload = serde_json::json!({
                "events": debugger.get_events(),
            });
            let json_str =
                serde_json::to_string(&payload).unwrap_or_else(|_| "{\"events\":[]}".to_string());
            // Caller must free with ta_free_cstring.
            match CString::new(json_str) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    // Caller must free with ta_free_cstring.
                    let err = CString::new("output serialization error").unwrap();
                    err.into_raw()
                }
            }
        }
        Err(e) => {
            let err_msg = format!("error: {}", e);
            // Caller must free with ta_free_cstring.
            match CString::new(err_msg) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    // Caller must free with ta_free_cstring.
                    let err = CString::new("evaluation error").unwrap();
                    err.into_raw()
                }
            }
        }
    }

    })
}

#[no_mangle]
pub extern "C" fn ta_formula_get_template(name: *const c_char) -> *mut c_char {
    ffi_catch_ptr(|| {

    use finkit::formula::FormulaEngine;

    if name.is_null() {
        let err = CString::new("invalid input").unwrap();
        // Caller must free with ta_free_cstring.
        return err.into_raw();
    }

    let name_str = match unsafe { CStr::from_ptr(name) }.to_str().map(str::to_owned) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let engine = FormulaEngine::new();
    match engine.get_template(&name_str) {
        Some(template) => {
            let json_str = serde_json::to_string(&template).unwrap_or_else(|_| "{}".to_string());
            // Caller must free with ta_free_cstring.
            match CString::new(json_str) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    // Caller must free with ta_free_cstring.
                    let err = CString::new("output serialization error").unwrap();
                    err.into_raw()
                }
            }
        }
        None => {
            // Caller must free with ta_free_cstring.
            let err = CString::new(format!("template '{}' not found", name_str)).unwrap();
            err.into_raw()
        }
    }

    })
}

#[no_mangle]
pub extern "C" fn ta_formula_search_templates(keyword: *const c_char) -> *mut c_char {
    ffi_catch_ptr(|| {

    use finkit::formula::FormulaEngine;

    if keyword.is_null() {
        let err = CString::new("invalid input").unwrap();
        // Caller must free with ta_free_cstring.
        return err.into_raw();
    }

    let keyword_str = match unsafe { CStr::from_ptr(keyword) }.to_str().map(str::to_owned) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let engine = FormulaEngine::new();
    let templates = engine.search_templates(&keyword_str);
    let json_str = serde_json::to_string(&templates).unwrap_or_else(|_| "[]".to_string());
    // Caller must free with ta_free_cstring.
    match CString::new(json_str) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => {
            // Caller must free with ta_free_cstring.
            let err = CString::new("output serialization error").unwrap();
            err.into_raw()
        }
    }

    })
}

#[no_mangle]
pub extern "C" fn ta_formula_list_categories() -> *mut c_char {
    ffi_catch_ptr(|| {

    use finkit::formula::templates::FormulaTemplates;

    let categories = FormulaTemplates::categories();
    let json_str = serde_json::to_string(&categories).unwrap_or_else(|_| "[]".to_string());
    // Caller must free with ta_free_cstring.
    match CString::new(json_str) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => {
            // Caller must free with ta_free_cstring.
            let err = CString::new("output serialization error").unwrap();
            err.into_raw()
        }
    }

    })
}

// ============================================================================
// Classic stock-trading chart patterns (FTA-native, added 2026-06-06).
// ============================================================================

#[no_mangle]
pub extern "C" fn ta_formula_validate(source: *const c_char) -> c_int {
    ffi_catch_i32(|| {

    if source.is_null() {
        return 0;
    }

    let source_str = match unsafe { CStr::from_ptr(source) }.to_str().map(str::to_owned) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    let mut engine = FormulaEngine::new();
    match engine.compile(&source_str) {
        Ok(_) => 1,
        Err(_) => 0,
    }

    })
}

#[no_mangle]
pub extern "C" fn ta_free_string(s: *mut c_char) {
    ffi_catch_void(|| {

    if !s.is_null() {
        unsafe {
            let _ = CString::from_raw(s);
        }
    }

    })
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
    ffi_catch_ptr(|| {

    if source.is_null()
        || open.is_null()
        || high.is_null()
        || low.is_null()
        || close.is_null()
        || volume.is_null()
        || length <= 0
    {
        let err = CString::new("invalid input").unwrap();
        // Caller must free with ta_free_cstring.
        return err.into_raw();
    }

    let source_str = match unsafe { CStr::from_ptr(source) }.to_str().map(str::to_owned) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
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
            let var_map: HashMap<String, Vec<Option<f64>>> = ctx
                .variables
                .iter()
                .map(|(name, value)| (name.to_string(), arr_to_json(value)))
                .collect();
            let json_str = serde_json::to_string(&var_map).unwrap_or_else(|_| "{}".to_string());
            // Caller must free with ta_free_cstring.
            match CString::new(json_str) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    // Caller must free with ta_free_cstring.
                    let err = CString::new("output serialization error").unwrap();
                    err.into_raw()
                }
            }
        }
        Err(e) => {
            let err_msg = format!("error: {}", e);
            // Caller must free with ta_free_cstring.
            match CString::new(err_msg) {
                Ok(c_str) => c_str.into_raw(),
                Err(_) => {
                    // Caller must free with ta_free_cstring.
                    let err = CString::new("evaluation error").unwrap();
                    err.into_raw()
                }
            }
        }
    }

    })
}
