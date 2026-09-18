use finkit::error::{FfiError, FormulaError, IndicatorError, TaError};
use finkit::indicators;
use finkit::math::moving_avg;
use finkit::patterns::candlestick;
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::slice;

/// Stable ABI error codes returned at the FFI boundary.
///
/// Legacy tiered codes (1, 2, 10+, 50+) remain in use for detailed errors;
/// this enum covers the unified top-level classification.
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FfiStatus {
    Ok = 0,
    NullPointer = -1,
    InvalidParameter = -2,
    InsufficientData = -3,
    InternalError = -4,
    InvalidUtf8 = -5,
    Unknown = -99,
}

impl FfiStatus {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

const TA_OK: i32 = 0;
const TA_ERR_INVALID_INPUT: i32 = -1;
const TA_ERR_CALCULATION: i32 = -2;

// FFI error code mapping (Phase 6). These codes are returned by
// `ta_last_error_code()` and follow the industrial three-tier
// classification:
//   * 0   — Ok / no error
//   * 1   — FFI boundary: null pointer
//   * 2   — FFI boundary: buffer too small
//   * 10+ — IndicatorError variants
//   * 50+ — FormulaError variants
const FFI_OK: i32 = 0;
const FFI_NULL_POINTER: i32 = 1;
const FFI_BUFFER_TOO_SMALL: i32 = 2;
const FFI_INDICATOR_BASE: i32 = 10;
const FFI_FORMULA_BASE: i32 = 50;

thread_local! {
    static LAST_ERROR: RefCell<String> = RefCell::new(String::new());
    static LAST_ERROR_CODE: RefCell<i32> = RefCell::new(FFI_OK);
}

fn set_last_error(msg: impl std::fmt::Display) {
    LAST_ERROR.with(|e| *e.borrow_mut() = msg.to_string());
}

fn set_last_error_code(code: i32) {
    LAST_ERROR_CODE.with(|c| *c.borrow_mut() = code);
}

#[allow(dead_code)]
fn reset_last_error() {
    LAST_ERROR.with(|e| *e.borrow_mut() = String::new());
    LAST_ERROR_CODE.with(|c| *c.borrow_mut() = FFI_OK);
}

/// Map a `TaError` to the FFI error code and remember the last error string.
fn map_ta_error(err: &TaError) -> i32 {
    set_last_error(err);
    match err {
        TaError::Ffi(FfiError::NullPointer) => {
            set_last_error_code(FFI_NULL_POINTER);
            FFI_NULL_POINTER
        }
        TaError::Ffi(FfiError::BufferTooSmall { .. }) => {
            set_last_error_code(FFI_BUFFER_TOO_SMALL);
            FFI_BUFFER_TOO_SMALL
        }
        TaError::Ffi(FfiError::Indicator(inner)) => {
            let code = FFI_INDICATOR_BASE + indicator_error_code(inner);
            set_last_error_code(code);
            code
        }
        TaError::Ffi(FfiError::Formula(inner)) => {
            let code = FFI_FORMULA_BASE + formula_error_code(inner);
            set_last_error_code(code);
            code
        }
        TaError::Indicator(inner) => {
            let code = FFI_INDICATOR_BASE + indicator_error_code(inner);
            set_last_error_code(code);
            code
        }
        TaError::Formula(inner) => {
            let code = FFI_FORMULA_BASE + formula_error_code(inner);
            set_last_error_code(code);
            code
        }
        // Legacy / compatibility variants fall through to a generic
        // "calculation error" code, but the human-readable message is
        // still preserved in the last-error string.
        _ => {
            set_last_error_code(TA_ERR_CALCULATION);
            TA_ERR_CALCULATION
        }
    }
}

fn indicator_error_code(err: &IndicatorError) -> i32 {
    match err {
        IndicatorError::InsufficientData { .. } => 0,
        IndicatorError::InvalidParameter { .. } => 1,
        IndicatorError::NumericOverflow { .. } => 2,
        IndicatorError::NanPropagation { .. } => 3,
    }
}

fn formula_error_code(err: &FormulaError) -> i32 {
    match err {
        FormulaError::Parse { .. } => 0,
        FormulaError::UndefinedFunction { .. } => 1,
        FormulaError::TypeMismatch { .. } => 2,
        FormulaError::Timeout { .. } => 3,
        FormulaError::MemoryLimit { .. } => 4,
        // Tuple-style compatibility variants.
        FormulaError::InsufficientData(_) => 5,
        FormulaError::InvalidParameter(_) => 6,
        FormulaError::RuntimeError(_) => 7,
        FormulaError::InvalidOperation(_) => 8,
        FormulaError::ParseError(_) => 9,
        FormulaError::UnsupportedFunction(_) => 10,
    }
}

fn invalid_input() -> i32 {
    set_last_error("Invalid input parameters");
    set_last_error_code(TA_ERR_INVALID_INPUT);
    TA_ERR_INVALID_INPUT
}

#[allow(dead_code)]
fn null_pointer() -> i32 {
    set_last_error("null pointer");
    set_last_error_code(FFI_NULL_POINTER);
    FFI_NULL_POINTER
}

fn calc_error(err: &TaError) -> i32 {
    map_ta_error(err)
}

unsafe fn copy_result(dst: *mut f64, src: &ndarray::Array1<f64>, dst_len: usize) {
    let copy_len = src.len().min(dst_len);
    let dst_slice = unsafe { std::slice::from_raw_parts_mut(dst, copy_len) };
    dst_slice.copy_from_slice(&src.as_slice().unwrap()[..copy_len]);
}

unsafe fn copy_int_result(dst: *mut i32, src: &ndarray::Array1<i32>, dst_len: usize) {
    let copy_len = src.len().min(dst_len);
    let dst_slice = unsafe { std::slice::from_raw_parts_mut(dst, copy_len) };
    dst_slice.copy_from_slice(&src.as_slice().unwrap()[..copy_len]);
}

fn internal_error_i32() -> i32 {
    set_last_error("internal error: panic at FFI boundary");
    set_last_error_code(FfiStatus::InternalError.as_i32());
    FfiStatus::InternalError.as_i32()
}

fn ffi_catch_i32<F>(f: F) -> i32
where
    F: FnOnce() -> i32,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => internal_error_i32(),
    }
}

fn ffi_catch_ptr<F>(f: F) -> *mut c_char
where
    F: FnOnce() -> *mut c_char,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => {
            set_last_error("internal error: panic at FFI boundary");
            set_last_error_code(FfiStatus::InternalError.as_i32());
            std::ptr::null_mut()
        }
    }
}

fn ffi_catch_void<F>(f: F)
where
    F: FnOnce(),
{
    if catch_unwind(AssertUnwindSafe(f)).is_err() {
        set_last_error("internal error: panic at FFI boundary");
        set_last_error_code(FfiStatus::InternalError.as_i32());
    }
}

#[no_mangle]
pub unsafe extern "C" fn ta_version() -> *mut c_char {
    ffi_catch_ptr(|| {
        let v = env!("CARGO_PKG_VERSION");
        CString::new(v).unwrap().into_raw()
    })
}

/// Return the canonical operation catalog as an owned UTF-8 JSON string.
///
/// The caller must release the returned pointer with `finkit_free_string`.
/// The envelope is shared by the C, C++, Go, Java, .NET and other binding
/// layers; it is generated from the Rust core operation registry.
#[no_mangle]
pub extern "C" fn ta_operation_catalog_json() -> *mut c_char {
    ffi_catch_ptr(|| {
        let json = finkit_ffi_common::operation::operation_catalog_json()
            .unwrap_or_else(|error| format!("{{\"error\":{}}}", serde_json::json!(error.to_string())));
        CString::new(json)
            .map(CString::into_raw)
            .unwrap_or_else(|_| std::ptr::null_mut())
    })
}

/// Return the canonical built-in Factor catalog as an owned UTF-8 JSON string.
#[no_mangle]
pub extern "C" fn ta_factor_catalog_json() -> *mut c_char {
    ffi_catch_ptr(|| {
        let json = finkit_ffi_common::factor_catalog::factor_catalog_json()
            .unwrap_or_else(|error| format!("{{\"error\":{}}}", serde_json::json!(error.to_string())));
        CString::new(json)
            .map(CString::into_raw)
            .unwrap_or_else(|_| std::ptr::null_mut())
    })
}

/// Execute one registered operation through the shared JSON result contract.
#[no_mangle]
pub unsafe extern "C" fn ta_operation_execute_json(
    request: *const c_char,
) -> *mut c_char {
    ffi_catch_ptr(|| {
        let payload = if request.is_null() {
            serde_json::json!({
                "schema_version": finkit_ffi_common::OPERATION_RESULT_SCHEMA_VERSION,
                "error": {"code": "invalid_request", "message": "request is null"},
            })
            .to_string()
        } else {
            match unsafe { std::ffi::CStr::from_ptr(request) }.to_str() {
                Ok(request) => finkit_ffi_common::execute_operation_json(request),
                Err(_) => serde_json::json!({
                    "schema_version": finkit_ffi_common::OPERATION_RESULT_SCHEMA_VERSION,
                    "error": {"code": "invalid_utf8", "message": "request is not valid UTF-8"},
                })
                .to_string(),
            }
        };
        CString::new(payload)
            .expect("operation result payload contains no NUL")
            .into_raw()
    })
}

/// Execute built-in factors through the shared JSON contract.
#[no_mangle]
pub unsafe extern "C" fn ta_factor_execute_json(request: *const c_char) -> *mut c_char {
    ffi_catch_ptr(|| {
        let payload = if request.is_null() {
            serde_json::json!({
                "schema_version": finkit_ffi_common::FACTOR_CONTRACT_SCHEMA_VERSION,
                "error": "request is null",
            })
            .to_string()
        } else {
            match unsafe { std::ffi::CStr::from_ptr(request) }.to_str() {
                Ok(request) => finkit_ffi_common::evaluate_factor_json(request)
                    .unwrap_or_else(|error| serde_json::json!({
                        "schema_version": finkit_ffi_common::FACTOR_CONTRACT_SCHEMA_VERSION,
                        "error": error,
                    }).to_string()),
                Err(_) => serde_json::json!({
                    "schema_version": finkit_ffi_common::FACTOR_CONTRACT_SCHEMA_VERSION,
                    "error": "request is not valid UTF-8",
                }).to_string(),
            }
        };
        CString::new(payload).expect("factor result payload contains no NUL").into_raw()
    })
}

/// Execute one cross-sectional Factor through the shared JSON contract.
#[no_mangle]
pub unsafe extern "C" fn ta_factor_cross_sectional_execute_json(
    request: *const c_char,
) -> *mut c_char {
    ffi_catch_ptr(|| {
        let payload = if request.is_null() {
            serde_json::json!({
                "schema_version": finkit_ffi_common::FACTOR_CROSS_SECTIONAL_CONTRACT_SCHEMA_VERSION,
                "error": "request is null",
            })
            .to_string()
        } else {
            match unsafe { CStr::from_ptr(request) }.to_str() {
                Ok(request) => finkit_ffi_common::evaluate_factor_cross_sectional_json(request)
                    .unwrap_or_else(|error| serde_json::json!({
                        "schema_version": finkit_ffi_common::FACTOR_CROSS_SECTIONAL_CONTRACT_SCHEMA_VERSION,
                        "error": error,
                    }).to_string()),
                Err(_) => serde_json::json!({
                    "schema_version": finkit_ffi_common::FACTOR_CROSS_SECTIONAL_CONTRACT_SCHEMA_VERSION,
                    "error": "request is not valid UTF-8",
                }).to_string(),
            }
        };
        CString::new(payload)
            .expect("cross-sectional factor result payload contains no NUL")
            .into_raw()
    })
}

/// Execute bounded Factor rows and return the portable streaming checkpoint.
#[no_mangle]
pub unsafe extern "C" fn ta_factor_stream_execute_json(request: *const c_char) -> *mut c_char {
    ffi_catch_ptr(|| {
        let payload = if request.is_null() {
            serde_json::json!({
                "schema_version": finkit_ffi_common::FACTOR_STREAM_CONTRACT_SCHEMA_VERSION,
                "error": "request is null",
            })
            .to_string()
        } else {
            match unsafe { std::ffi::CStr::from_ptr(request) }.to_str() {
                Ok(request) => finkit_ffi_common::evaluate_factor_stream_json(request)
                    .unwrap_or_else(|error| serde_json::json!({
                        "schema_version": finkit_ffi_common::FACTOR_STREAM_CONTRACT_SCHEMA_VERSION,
                        "error": error,
                    }).to_string()),
                Err(_) => serde_json::json!({
                    "schema_version": finkit_ffi_common::FACTOR_STREAM_CONTRACT_SCHEMA_VERSION,
                    "error": "request is not valid UTF-8",
                }).to_string(),
            }
        };
        CString::new(payload).expect("factor stream result payload contains no NUL").into_raw()
    })
}

/// Execute a dependency-aware Composite through the shared JSON contract.
#[no_mangle]
pub unsafe extern "C" fn ta_composite_execute_json(request: *const c_char) -> *mut c_char {
    ffi_catch_ptr(|| {
        let payload = if request.is_null() {
            serde_json::json!({
                "schema_version": finkit_ffi_common::COMPOSITE_CONTRACT_SCHEMA_VERSION,
                "error": "request is null",
            })
            .to_string()
        } else {
            match unsafe { std::ffi::CStr::from_ptr(request) }.to_str() {
                Ok(request) => finkit_ffi_common::evaluate_composite_json(request)
                    .unwrap_or_else(|error| serde_json::json!({
                        "schema_version": finkit_ffi_common::COMPOSITE_CONTRACT_SCHEMA_VERSION,
                        "error": error,
                    }).to_string()),
                Err(_) => serde_json::json!({
                    "schema_version": finkit_ffi_common::COMPOSITE_CONTRACT_SCHEMA_VERSION,
                    "error": "request is not valid UTF-8",
                }).to_string(),
            }
        };
        CString::new(payload).expect("composite result payload contains no NUL").into_raw()
    })
}

/// Execute bounded Composite rows and return the portable streaming checkpoint.
#[no_mangle]
pub unsafe extern "C" fn ta_composite_stream_execute_json(request: *const c_char) -> *mut c_char {
    ffi_catch_ptr(|| {
        let payload = if request.is_null() {
            serde_json::json!({
                "schema_version": finkit_ffi_common::COMPOSITE_STREAM_CONTRACT_SCHEMA_VERSION,
                "error": "request is null",
            })
            .to_string()
        } else {
            match unsafe { std::ffi::CStr::from_ptr(request) }.to_str() {
                Ok(request) => finkit_ffi_common::evaluate_composite_stream_json(request)
                    .unwrap_or_else(|error| serde_json::json!({
                        "schema_version": finkit_ffi_common::COMPOSITE_STREAM_CONTRACT_SCHEMA_VERSION,
                        "error": error,
                    }).to_string()),
                Err(_) => serde_json::json!({
                    "schema_version": finkit_ffi_common::COMPOSITE_STREAM_CONTRACT_SCHEMA_VERSION,
                    "error": "request is not valid UTF-8",
                }).to_string(),
            }
        };
        CString::new(payload).expect("composite stream result payload contains no NUL").into_raw()
    })
}

/// Execute a formula through the versioned cross-language result contract.
///
/// The returned JSON owns the named output arrays and represents non-finite
/// values as `null`. Release it with `finkit_free_string`.
#[no_mangle]
pub unsafe extern "C" fn ta_formula_eval_contract_json(
    source: *const c_char,
    dialect: *const c_char,
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    volume: *const f64,
    length: i32,
) -> *mut c_char {
    ffi_catch_ptr(|| {
        let error = |message: &str| {
            CString::new(serde_json::json!({
                "schema_version": finkit_ffi_common::FORMULA_CONTRACT_SCHEMA_VERSION,
                "error": message,
            }).to_string())
            .expect("formula contract error contains no NUL")
            .into_raw()
        };
        if source.is_null()
            || dialect.is_null()
            || open.is_null()
            || high.is_null()
            || low.is_null()
            || close.is_null()
            || volume.is_null()
            || length < 0
        {
            return error("invalid formula contract input");
        }
        let source = match unsafe { std::ffi::CStr::from_ptr(source) }.to_str() {
            Ok(value) => value,
            Err(_) => return error("formula source is not valid UTF-8"),
        };
        let dialect = match unsafe { std::ffi::CStr::from_ptr(dialect) }.to_str() {
            Ok(value) => value,
            Err(_) => return error("formula dialect is not valid UTF-8"),
        };
        let length = length as usize;
        let (open, high, low, close, volume) = unsafe {
            (
                slice::from_raw_parts(open, length),
                slice::from_raw_parts(high, length),
                slice::from_raw_parts(low, length),
                slice::from_raw_parts(close, length),
                slice::from_raw_parts(volume, length),
            )
        };
        let payload = finkit_ffi_common::evaluate_formula_json(
            source, dialect, open, high, low, close, volume,
        )
        .unwrap_or_else(|message| {
            serde_json::json!({
                "schema_version": finkit_ffi_common::FORMULA_CONTRACT_SCHEMA_VERSION,
                "error": message,
            })
            .to_string()
        });
        CString::new(payload)
            .expect("formula contract payload contains no NUL")
            .into_raw()
    })
}

/// Execute an explicit timestamped Formula request through the shared
/// multi-timeframe and point-in-time contract.
#[no_mangle]
pub unsafe extern "C" fn ta_formula_eval_temporal_contract_json(
    request: *const c_char,
) -> *mut c_char {
    ffi_catch_ptr(|| {
        let error = |message: &str| {
            CString::new(serde_json::json!({
                "schema_version": finkit_ffi_common::FORMULA_TEMPORAL_CONTRACT_SCHEMA_VERSION,
                "error": message,
            }).to_string())
            .expect("formula temporal contract error contains no NUL")
            .into_raw()
        };
        if request.is_null() {
            return error("formula temporal contract request is null");
        }
        let request = match unsafe { std::ffi::CStr::from_ptr(request) }.to_str() {
            Ok(value) => value,
            Err(_) => return error("formula temporal contract request is not valid UTF-8"),
        };
        let payload = finkit_ffi_common::evaluate_formula_temporal_json(request)
            .unwrap_or_else(|message| {
                serde_json::json!({
                    "schema_version": finkit_ffi_common::FORMULA_TEMPORAL_CONTRACT_SCHEMA_VERSION,
                    "error": message,
                }).to_string()
            });
        CString::new(payload)
            .expect("formula temporal contract payload contains no NUL")
            .into_raw()
    })
}

/// Execute one Formula independently for every explicit symbol/timeframe frame.
#[no_mangle]
pub unsafe extern "C" fn ta_formula_eval_panel_contract_json(
    request: *const c_char,
) -> *mut c_char {
    ffi_catch_ptr(|| {
        let error = |message: &str| {
            CString::new(serde_json::json!({
                "schema_version": finkit_ffi_common::FORMULA_PANEL_CONTRACT_SCHEMA_VERSION,
                "error": message,
            }).to_string())
            .expect("formula panel contract error contains no NUL")
            .into_raw()
        };
        if request.is_null() {
            return error("formula panel contract request is null");
        }
        let request = match unsafe { std::ffi::CStr::from_ptr(request) }.to_str() {
            Ok(value) => value,
            Err(_) => return error("formula panel contract request is not valid UTF-8"),
        };
        let payload = finkit_ffi_common::evaluate_formula_panel_json(request)
            .unwrap_or_else(|message| {
                serde_json::json!({
                    "schema_version": finkit_ffi_common::FORMULA_PANEL_CONTRACT_SCHEMA_VERSION,
                    "error": message,
                }).to_string()
            });
        CString::new(payload)
            .expect("formula panel contract payload contains no NUL")
            .into_raw()
    })
}

/// Execute the verified stateful Formula stream JSON contract.
///
/// The request and checkpoint are transport-neutral JSON; the returned string
/// must be released with `finkit_free_string`.
#[no_mangle]
pub unsafe extern "C" fn ta_formula_stream_execute_json(
    request: *const c_char,
) -> *mut c_char {
    ffi_catch_ptr(|| {
        let payload = if request.is_null() {
            serde_json::json!({
                "schema_version": finkit_ffi_common::FORMULA_STREAM_CONTRACT_SCHEMA_VERSION,
                "error": "formula stream request is null",
            })
            .to_string()
        } else {
            match unsafe { std::ffi::CStr::from_ptr(request) }.to_str() {
                Ok(request) => finkit_ffi_common::evaluate_formula_stream_json(request)
                    .unwrap_or_else(|message| {
                        serde_json::json!({
                            "schema_version": finkit_ffi_common::FORMULA_STREAM_CONTRACT_SCHEMA_VERSION,
                            "error": message,
                        })
                        .to_string()
                    }),
                Err(_) => serde_json::json!({
                    "schema_version": finkit_ffi_common::FORMULA_STREAM_CONTRACT_SCHEMA_VERSION,
                    "error": "formula stream request is not valid UTF-8",
                })
                .to_string(),
            }
        };
        CString::new(payload)
            .expect("formula stream payload contains no NUL")
            .into_raw()
    })
}

/// Inspect a formula through the shared language-neutral compatibility report.
///
/// The returned JSON owns the report and must be released with
/// `finkit_free_string`.
#[no_mangle]
pub unsafe extern "C" fn ta_formula_compatibility_report_json(
    source: *const c_char,
    terminal: *const c_char,
) -> *mut c_char {
    ffi_catch_ptr(|| {
        let error = |message: &str| {
            CString::new(serde_json::json!({
                "schema_version": finkit_ffi_common::FORMULA_COMPATIBILITY_SCHEMA_VERSION,
                "error": message,
            }).to_string())
            .expect("formula compatibility error contains no NUL")
            .into_raw()
        };
        if source.is_null() || terminal.is_null() {
            return error("formula compatibility input is null");
        }
        let source = match unsafe { std::ffi::CStr::from_ptr(source) }.to_str() {
            Ok(value) => value,
            Err(_) => return error("formula source is not valid UTF-8"),
        };
        let terminal = match unsafe { std::ffi::CStr::from_ptr(terminal) }.to_str() {
            Ok(value) => value,
            Err(_) => return error("formula terminal is not valid UTF-8"),
        };
        let payload = finkit_ffi_common::formula_compatibility_report_json(source, terminal)
            .unwrap_or_else(|message| {
                serde_json::json!({
                    "schema_version": finkit_ffi_common::FORMULA_COMPATIBILITY_SCHEMA_VERSION,
                    "error": message,
                })
                .to_string()
            });
        CString::new(payload)
            .expect("formula compatibility payload contains no NUL")
            .into_raw()
    })
}

/// Return the last FFI error code for the calling thread.
#[no_mangle]
pub extern "C" fn ta_last_error_code() -> i32 {
    LAST_ERROR_CODE.with(|code| *code.borrow())
}

#[no_mangle]
pub extern "C" fn ta_last_error() -> *mut c_char {
    ffi_catch_ptr(|| {
        let message = LAST_ERROR.with(|error| error.borrow().clone());
        CString::new(message)
            .map(CString::into_raw)
            .unwrap_or_else(|_| std::ptr::null_mut())
    })
}

/// Free a string allocated by a Finkit C-ABI function.
#[no_mangle]
pub unsafe extern "C" fn finkit_free_string(s: *mut c_char) {
    ffi_catch_void(|| {
        if !s.is_null() {
            drop(unsafe { CString::from_raw(s) });
        }
    })
}

include!("generated.rs");

/// Test-only export: panics inside the same `ffi_catch_i32` guard used by
/// production `ta_*` entry points. Used to verify panic isolation without
/// relying on undefined behaviour from invalid pointers.
#[cfg(test)]
#[no_mangle]
pub unsafe extern "C" fn ta_ffi_panic_test() -> i32 {
    ffi_catch_i32(|| panic!("ta_ffi_panic_test: injected panic"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use finkit::error::{FfiError, FormulaError, IndicatorError};
    use std::ffi::CStr;

    // These tests cover the Phase 6 FFI error-code mapping. Each test
    // exercises a different branch of `map_ta_error` to guarantee that
    // the three-tier classification is reported correctly to C callers
    // via `ta_last_error_code()`.

    fn read_code() -> i32 {
        LAST_ERROR_CODE.with(|c| *c.borrow())
    }

    #[test]
    fn last_error_roundtrip_and_free() {
        reset_last_error();
        set_last_error("roundtrip error");
        let ptr = ta_last_error();
        assert!(!ptr.is_null());
        let message = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        assert_eq!(message, "roundtrip error");
        unsafe { finkit_free_string(ptr) };
    }

    #[test]
    fn code_ok_after_init() {
        // The thread-local starts at 0. Just confirm the getter works.
        assert_eq!(ta_last_error_code(), 0);
    }

    #[test]
    fn operation_catalog_json_is_owned_and_contains_core_metadata() {
        let ptr = ta_operation_catalog_json();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert!(value["operations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|operation| operation["name"] == "EMA"));
        unsafe { finkit_free_string(ptr) };
    }

    #[test]
    fn operation_catalog_json_exposes_talib_parameter_contract() {
        let ptr = ta_operation_catalog_json();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let operations = value["operations"].as_array().unwrap();
        let find = |name: &str| {
            operations
                .iter()
                .find(|operation| operation["name"] == name)
                .unwrap_or_else(|| panic!("missing operation {name}"))
        };
        let stoch = find("STOCH");
        assert!(stoch["semantic_profiles"]
            .as_array()
            .unwrap()
            .iter()
            .any(|profile| profile == "talib_0_7_1"));
        assert_eq!(stoch["params"].as_array().unwrap().len(), 5);
        assert_eq!(stoch["params"][2]["name"], "slowk_matype");
        let bbands = find("BBANDS");
        assert_eq!(bbands["params"][3]["name"], "matype");
        unsafe { finkit_free_string(ptr) };
    }

    #[test]
    fn factor_catalog_json_is_owned_and_contains_dependency_metadata() {
        let ptr = ta_factor_catalog_json();
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert!(value["factors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|factor| factor["name"] == "reversal_5" && factor["dependencies"][0] == "momentum_5"));
        unsafe { finkit_free_string(ptr) };
    }

    #[test]
    fn operation_execute_json_dispatches_named_sma_output() {
        let request = std::ffi::CString::new(
            r#"{"operation":"SMA","input_order":["CLOSE"],"inputs":{"CLOSE":[1.0,2.0,3.0]},"params":[2]}"#,
        )
        .unwrap();
        let ptr = unsafe { ta_operation_execute_json(request.as_ptr()) };
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(value["operation"], "SMA");
        assert_eq!(value["primary"], "SMA");
        assert_eq!(value["values"]["SMA"][2], 2.25);
        unsafe { finkit_free_string(ptr) };
    }

    #[test]
    fn composite_execute_json_dispatches_shared_contract() {
        let request = std::ffi::CString::new(
            r#"{"schema_version":1,"inputs":{"close":[1.0,2.0,3.0]},"definitions":[{"name":"sum","function":"add","inputs":["close","const:1"],"params":[]}],"outputs":["sum"]}"#,
        )
        .unwrap();
        let ptr = unsafe { ta_composite_execute_json(request.as_ptr()) };
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["primary"], "sum");
        assert_eq!(value["values"]["sum"][2], 4.0);
        unsafe { finkit_free_string(ptr) };
    }

    #[test]
    fn factor_execute_json_dispatches_compiled_builtin_factor() {
        let request = std::ffi::CString::new(
            r#"{"schema_version":1,"targets":["momentum_5"],"inputs":{"close":[1.0,2.0,3.0,4.0,5.0,6.0]}}"#,
        )
        .unwrap();
        let ptr = unsafe { ta_factor_execute_json(request.as_ptr()) };
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["primary"], "momentum_5");
        assert_eq!(value["values"]["momentum_5"][5], 5.0);
        unsafe { finkit_free_string(ptr) };
    }

    #[test]
    fn factor_cross_sectional_execute_json_preserves_panel_axes() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/contracts/factor_cross_sectional_contract_v1.json"
        )))
        .unwrap();
        let request = std::ffi::CString::new(fixture["request"].to_string()).unwrap();
        let ptr = unsafe { ta_factor_cross_sectional_execute_json(request.as_ptr()) };
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(value["contract"], "factor.cross_sectional.v1");
        assert_eq!(value["symbols"], serde_json::json!(["AAA", "BBB", "CCC"]));
        assert_eq!(value["values"]["cross_rank"][1], 1.0);
        assert!(value["values"]["cross_rank"][4].is_null());
        unsafe { finkit_free_string(ptr) };
    }

    #[test]
    fn stream_json_exports_return_portable_checkpoints() {
        let factor_request = std::ffi::CString::new(
            r#"{"schema_version":1,"targets":["momentum_5"],"inputs":{"close":[1.0,2.0,3.0,4.0,5.0,6.0]}}"#,
        )
        .unwrap();
        let factor_ptr = unsafe { ta_factor_stream_execute_json(factor_request.as_ptr()) };
        assert!(!factor_ptr.is_null());
        let factor_json = unsafe { CStr::from_ptr(factor_ptr) }.to_str().unwrap();
        let factor_value: serde_json::Value = serde_json::from_str(factor_json).unwrap();
        assert_eq!(factor_value["execution"]["mode"], "streaming");
        assert!(factor_value["checkpoint"]["semantic_identity"].is_array());
        unsafe { finkit_free_string(factor_ptr) };

        let composite_request = std::ffi::CString::new(
            r#"{"schema_version":1,"inputs":{"close":[1.0,2.0,3.0]},"definitions":[{"name":"sma3","function":"sma","inputs":["close"],"params":[3]}],"outputs":["sma3"]}"#,
        )
        .unwrap();
        let composite_ptr = unsafe {
            ta_composite_stream_execute_json(composite_request.as_ptr())
        };
        assert!(!composite_ptr.is_null());
        let composite_json = unsafe { CStr::from_ptr(composite_ptr) }.to_str().unwrap();
        let composite_value: serde_json::Value = serde_json::from_str(composite_json).unwrap();
        assert_eq!(composite_value["execution"]["mode"], "streaming");
        assert!(composite_value["checkpoint"]["signature"].is_number());
        unsafe { finkit_free_string(composite_ptr) };

        let formula_request = std::ffi::CString::new(
            r#"{"schema_version":1,"source":"EMA(CLOSE,3)","dialect":"tdx","inputs":{"close":[10.0,11.0,12.0,15.0]}}"#,
        )
        .unwrap();
        let formula_ptr = unsafe { ta_formula_stream_execute_json(formula_request.as_ptr()) };
        assert!(!formula_ptr.is_null());
        let formula_json = unsafe { CStr::from_ptr(formula_ptr) }.to_str().unwrap();
        let formula_value: serde_json::Value = serde_json::from_str(formula_json).unwrap();
        assert_eq!(formula_value["execution"]["mode"], "stateful_streaming");
        assert_eq!(formula_value["values"]["__PRIMARY__"][2], 11.0);
        assert!(formula_value["checkpoint"]["state"].is_object());
        unsafe { finkit_free_string(formula_ptr) };
    }

    #[test]
    fn formula_contract_json_contains_version_and_dialect() {
        let values = [1.0, 2.0, 3.0];
        let source = std::ffi::CString::new("CLOSE").unwrap();
        let dialect = std::ffi::CString::new("tdx").unwrap();
        let ptr = unsafe {
            ta_formula_eval_contract_json(
                source.as_ptr(),
                dialect.as_ptr(),
                values.as_ptr(),
                values.as_ptr(),
                values.as_ptr(),
                values.as_ptr(),
                values.as_ptr(),
                values.len() as i32,
            )
        };
        let json = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["dialect"], "tdx");
        assert_eq!(value["primary"], "__PRIMARY__");
        unsafe { finkit_free_string(ptr) };
    }

    #[test]
    fn formula_temporal_contract_json_preserves_frame_and_asof_semantics() {
        let request = std::ffi::CString::new(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tests/contracts/formula_temporal_contract_v1.json"
            )),
        )
        .unwrap();
        let fixture: serde_json::Value = serde_json::from_str(request.to_str().unwrap()).unwrap();
        let request = std::ffi::CString::new(fixture["request"].to_string()).unwrap();
        let ptr = unsafe { ta_formula_eval_temporal_contract_json(request.as_ptr()) };
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(value["contract"], "formula.temporal.v1");
        assert_eq!(value["frame"]["symbol"], "AAA");
        assert_eq!(value["values"]["__PRIMARY__"][2], 304.0);
        unsafe { finkit_free_string(ptr) };
    }

    #[test]
    fn formula_panel_contract_json_keeps_frames_isolated() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/contracts/formula_panel_contract_v1.json"
        )))
        .unwrap();
        let request = std::ffi::CString::new(fixture["request"].to_string()).unwrap();
        let ptr = unsafe { ta_formula_eval_panel_contract_json(request.as_ptr()) };
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(value["contract"], "formula.panel.v1");
        assert_eq!(value["frames"][0]["symbol"], "AAA");
        assert_eq!(value["frames"][1]["values"]["__PRIMARY__"][1], 21.0);
        unsafe { finkit_free_string(ptr) };
    }

    #[test]
    fn formula_compatibility_report_json_contains_shared_capabilities() {
        let source = std::ffi::CString::new("X:=MA(CLOSE,5); X").unwrap();
        let terminal = std::ffi::CString::new("tdx").unwrap();
        let ptr = unsafe {
            ta_formula_compatibility_report_json(source.as_ptr(), terminal.as_ptr())
        };
        assert!(!ptr.is_null());
        let json = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["terminal"], "tdx");
        assert!(value["capabilities"].is_array());
        unsafe { finkit_free_string(ptr) };
    }

    #[test]
    fn code_null_pointer_for_ffi_null() {
        let err = TaError::Ffi(FfiError::NullPointer);
        let code = map_ta_error(&err);
        assert_eq!(code, 1);
        assert_eq!(read_code(), 1);
    }

    #[test]
    fn code_buffer_too_small_for_ffi_buffer() {
        let err = TaError::Ffi(FfiError::BufferTooSmall {
            required: 100,
            actual: 50,
        });
        let code = map_ta_error(&err);
        assert_eq!(code, 2);
        assert_eq!(read_code(), 2);
    }

    #[test]
    fn code_indicator_insufficient_data() {
        let err = TaError::Indicator(IndicatorError::InsufficientData {
            required: 30,
            actual: 5,
        });
        let code = map_ta_error(&err);
        assert_eq!(code, 10);
        assert_eq!(read_code(), 10);
    }

    #[test]
    fn code_indicator_invalid_parameter() {
        let err = TaError::Indicator(IndicatorError::InvalidParameter {
            param: "p".into(),
            reason: "x".into(),
        });
        let code = map_ta_error(&err);
        assert_eq!(code, 11);
        assert_eq!(read_code(), 11);
    }

    #[test]
    fn code_indicator_numeric_overflow() {
        let err = TaError::Indicator(IndicatorError::NumericOverflow {
            indicator: "ATR".into(),
            index: 1,
        });
        let code = map_ta_error(&err);
        assert_eq!(code, 12);
    }

    #[test]
    fn code_indicator_nan_propagation() {
        let err = TaError::Indicator(IndicatorError::NanPropagation {
            indicator: "RSI".into(),
        });
        let code = map_ta_error(&err);
        assert_eq!(code, 13);
    }

    #[test]
    fn code_formula_parse() {
        let err = TaError::Formula(FormulaError::Parse {
            line: 1,
            col: 2,
            message: "x".into(),
        });
        let code = map_ta_error(&err);
        assert_eq!(code, 50);
    }

    #[test]
    fn code_formula_undefined_function() {
        let err = TaError::Formula(FormulaError::UndefinedFunction { name: "f".into() });
        let code = map_ta_error(&err);
        assert_eq!(code, 51);
    }

    #[test]
    fn code_formula_type_mismatch() {
        let err = TaError::Formula(FormulaError::TypeMismatch {
            expected: "f64".into(),
            actual: "i32".into(),
        });
        let code = map_ta_error(&err);
        assert_eq!(code, 52);
    }

    #[test]
    fn code_formula_timeout() {
        let err = TaError::Formula(FormulaError::Timeout { elapsed_ms: 10 });
        let code = map_ta_error(&err);
        assert_eq!(code, 53);
    }

    #[test]
    fn code_formula_memory_limit() {
        let err = TaError::Formula(FormulaError::MemoryLimit { used: 1, limit: 2 });
        let code = map_ta_error(&err);
        assert_eq!(code, 54);
    }

    #[test]
    fn code_invalid_input_helper() {
        // invalid_input() must leave the legacy -1 code in place.
        let _ = invalid_input();
        assert_eq!(read_code(), -1);
    }

    #[test]
    fn code_zero_on_legacy_calc_error() {
        // A deprecated TaError variant falls through to the legacy
        // generic calculation code.
        #[allow(deprecated)]
        {
            let err = TaError::EmptyInput;
            let code = map_ta_error(&err);
            assert_eq!(code, -2);
            assert_eq!(read_code(), -2);
        }
    }

    #[test]
    fn ffi_catch_i32_maps_panic_to_internal_error() {
        let code = ffi_catch_i32(|| panic!("deliberate FFI panic injection"));
        assert_eq!(code, FfiStatus::InternalError.as_i32());
        assert_eq!(read_code(), FfiStatus::InternalError.as_i32());
    }

    #[test]
    fn export_panic_test_returns_internal_error_not_abort() {
        let code = unsafe { ta_ffi_panic_test() };
        assert_eq!(code, FfiStatus::InternalError.as_i32());
        assert_eq!(read_code(), FfiStatus::InternalError.as_i32());
    }

    #[test]
    fn export_ta_sma_null_input_returns_error_without_panic() {
        let code = unsafe { ta_sma(std::ptr::null(), std::ptr::null_mut(), 10, 5) };
        assert_eq!(code, TA_ERR_INVALID_INPUT);
        assert_ne!(code, FfiStatus::InternalError.as_i32());
    }
}
