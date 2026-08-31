use finkit::error::{FfiError, FormulaError, IndicatorError, TaError};
use finkit::indicators;
use finkit::math::moving_avg;
use finkit::patterns::candlestick;
use std::cell::RefCell;
use std::ffi::CString;
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

/// Best-effort calculation error handler for non-`TaError` error types
/// (e.g. `VisualizationError`). Records the formatted message and
/// returns the generic legacy code.
fn calc_error_display(err: impl std::fmt::Display) -> i32 {
    set_last_error(err);
    set_last_error_code(TA_ERR_CALCULATION);
    TA_ERR_CALCULATION
}

unsafe fn copy_result(dst: *mut f64, src: &ndarray::Array1<f64>, dst_len: usize) {
    let copy_len = src.len().min(dst_len);
    let dst_slice = std::slice::from_raw_parts_mut(dst, copy_len);
    dst_slice.copy_from_slice(&src.as_slice().unwrap()[..copy_len]);
}

unsafe fn copy_int_result(dst: *mut i32, src: &ndarray::Array1<i32>, dst_len: usize) {
    let copy_len = src.len().min(dst_len);
    let dst_slice = std::slice::from_raw_parts_mut(dst, copy_len);
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

fn ffi_catch_i64<F>(f: F) -> i64
where
    F: FnOnce() -> i64,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => {
            set_last_error("internal error: panic at FFI boundary");
            set_last_error_code(FfiStatus::InternalError.as_i32());
            0
        }
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
    ffi_catch_ptr(|| unsafe {
        let v = env!("CARGO_PKG_VERSION");
        CString::new(v).unwrap().into_raw()
    })
}

#[no_mangle]

include!("generated.rs");

/// Test-only export: panics inside the same `ffi_catch_i32` guard used by
/// production `ta_*` entry points. Used to verify panic isolation without
/// relying on undefined behaviour from invalid pointers.
#[cfg(test)]
#[no_mangle]
pub unsafe extern "C" fn ta_ffi_panic_test() -> i32 {
    ffi_catch_i32(|| unsafe {
        panic!("ta_ffi_panic_test: injected panic");
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use finkit::error::{FfiError, FormulaError, IndicatorError};

    // These tests cover the Phase 6 FFI error-code mapping. Each test
    // exercises a different branch of `map_ta_error` to guarantee that
    // the three-tier classification is reported correctly to C callers
    // via `ta_last_error_code()`.

    fn read_code() -> i32 {
        LAST_ERROR_CODE.with(|c| *c.borrow())
    }

    #[test]
    fn code_ok_after_init() {
        // The thread-local starts at 0. Just confirm the getter works.
        assert_eq!(unsafe { ta_last_error_code() }, 0);
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
        assert_eq!(
            unsafe { ta_last_error_code() },
            FfiStatus::InternalError.as_i32()
        );
    }

    #[test]
    fn export_ta_sma_null_input_returns_error_without_panic() {
        let code = unsafe { ta_sma(std::ptr::null(), std::ptr::null_mut(), 10, 5) };
        assert_eq!(code, TA_ERR_INVALID_INPUT);
        assert_ne!(code, FfiStatus::InternalError.as_i32());
    }
}
