use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

fn response_ptr(response: String) -> *mut c_char {
    CString::new(response)
        .map(CString::into_raw)
        .unwrap_or(ptr::null_mut())
}

fn request_text(request_json: *const c_char) -> Result<String, String> {
    if request_json.is_null() {
        return Err("null_pointer\0request_json is null".to_string());
    }
    let request = unsafe { CStr::from_ptr(request_json) };
    request
        .to_str()
        .map(str::to_owned)
        .map_err(|error| format!("invalid_utf8\0{error}"))
}

fn split_boundary_error(error: &str) -> (&str, &str) {
    error
        .split_once('\0')
        .unwrap_or(("invalid_request", error))
}

/// Run the canonical factor-research JSON contract.
///
/// The returned UTF-8 string is owned by the caller and must be released with
/// `finkit_factor_study_free_string` (or the legacy `finkit_free_string`).
#[no_mangle]
pub unsafe extern "C" fn finkit_factor_study_json(request_json: *const c_char) -> *mut c_char {
    let result = catch_unwind(AssertUnwindSafe(|| match request_text(request_json) {
        Ok(request) => finkit_ffi_common::factor_study_json(&request),
        Err(error) => {
            let (code, message) = split_boundary_error(&error);
            finkit_ffi_common::factor_study_error_json(code, message)
        }
    }));
    match result {
        Ok(response) => response_ptr(response),
        Err(_) => response_ptr(finkit_ffi_common::factor_study_error_json(
            "panic_caught",
            "factor research engine panicked at the FFI boundary",
        )),
    }
}

/// Run the generic quantitative strategy/portfolio evaluation JSON contract.
#[no_mangle]
pub unsafe extern "C" fn finkit_quant_evaluation_json(
    request_json: *const c_char,
) -> *mut c_char {
    let result = catch_unwind(AssertUnwindSafe(|| match request_text(request_json) {
        Ok(request) => finkit_ffi_common::quant_evaluation_json(&request),
        Err(error) => {
            let (code, message) = split_boundary_error(&error);
            finkit_ffi_common::quant_evaluation_error_json(code, message)
        }
    }));
    match result {
        Ok(response) => response_ptr(response),
        Err(_) => response_ptr(finkit_ffi_common::quant_evaluation_error_json(
            "panic_caught",
            "quantitative evaluation engine panicked at the FFI boundary",
        )),
    }
}

/// Release strings returned by the research/evaluation JSON APIs.
#[no_mangle]
pub unsafe extern "C" fn finkit_factor_study_free_string(value: *mut c_char) {
    if !value.is_null() {
        drop(unsafe { CString::from_raw(value) });
    }
}