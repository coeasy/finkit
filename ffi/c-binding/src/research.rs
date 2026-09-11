use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

fn response_ptr(response: String) -> *mut c_char {
    CString::new(response)
        .map(CString::into_raw)
        .unwrap_or(ptr::null_mut())
}

fn error_json(code: &str, message: &str) -> String {
    serde_json::json!({
        "schema_version": finkit_ffi_common::FACTOR_STUDY_SCHEMA_VERSION,
        "library_version": env!("CARGO_PKG_VERSION"),
        "ok": false,
        "error": {"code": code, "message": message}
    })
    .to_string()
}

/// Run the canonical factor-research JSON contract.
///
/// The returned UTF-8 string is owned by the caller and must be released with
/// `finkit_factor_study_free_string` (or the legacy `finkit_free_string`).
#[no_mangle]
pub unsafe extern "C" fn finkit_factor_study_json(request_json: *const c_char) -> *mut c_char {
    if request_json.is_null() {
        return response_ptr(error_json("null_pointer", "request_json is null"));
    }
    let result = catch_unwind(AssertUnwindSafe(|| {
        let request = CStr::from_ptr(request_json);
        match request.to_str() {
            Ok(request) => finkit_ffi_common::factor_study_json(request),
            Err(error) => error_json("invalid_utf8", &error.to_string()),
        }
    }));
    match result {
        Ok(response) => response_ptr(response),
        Err(_) => response_ptr(error_json(
            "panic_caught",
            "factor research engine panicked at the FFI boundary",
        )),
    }
}

/// Release strings returned by `finkit_factor_study_json`.
#[no_mangle]
pub unsafe extern "C" fn finkit_factor_study_free_string(value: *mut c_char) {
    if !value.is_null() {
        drop(CString::from_raw(value));
    }
}
