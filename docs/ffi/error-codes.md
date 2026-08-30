# FFI Error Codes

This document describes the stable error codes returned across the AlphaTA/AlphaTA C FFI boundary (`ffi/c-binding`). All `extern "C"` exports use `std::panic::catch_unwind`; panics are converted to `InternalError` instead of aborting the host process.

## Unified `FfiStatus` (top-level classification)

Defined in `ffi/c-binding/src/lib.rs` as `#[repr(i32)]` for stable ABI:

| Code | `FfiStatus` variant | Meaning |
|------|---------------------|---------|
| `0` | `Ok` | Success / no error |
| `-1` | `NullPointer` | Required pointer argument was null |
| `-2` | `InvalidParameter` | Invalid length, period, or other argument |
| `-3` | `InsufficientData` | Not enough input data for the requested operation |
| `-4` | `InternalError` | Rust panic caught at the FFI boundary (`catch_unwind`) |
| `-5` | `InvalidUtf8` | C string argument is not valid UTF-8 |
| `-99` | `Unknown` | Unclassified error |

Retrieve the most recent code on the current thread with `ta_last_error_code()`. Human-readable detail is available from `ta_last_error()` (allocate with `ta_last_error`, release with `alphata_free_string`).

## Legacy and detailed codes (`ta_last_error_code`)

For backward compatibility, many functions still return legacy negative codes directly from the function return value, while `ta_last_error_code()` may hold a finer-grained positive tier:

| Code | Category | Meaning |
|------|----------|---------|
| `0` | OK | Success |
| `-1` | Legacy | Generic invalid input (`TA_ERR_INVALID_INPUT`) |
| `-2` | Legacy | Generic calculation error (`TA_ERR_CALCULATION`) |
| `1` | FFI | Null pointer (`FFI_NULL_POINTER`) |
| `2` | FFI | Output buffer too small (`FFI_BUFFER_TOO_SMALL`) |
| `10` | Indicator | Insufficient data |
| `11` | Indicator | Invalid parameter |
| `12` | Indicator | Numeric overflow |
| `13` | Indicator | NaN propagation |
| `50` | Formula | Parse error |
| `51` | Formula | Undefined function |
| `52` | Formula | Type mismatch |
| `53` | Formula | Timeout |
| `54` | Formula | Memory limit |
| `55`–`59` | Formula | Tuple-style compatibility variants |

When a Rust panic is caught at the boundary, both the function return (where applicable) and `ta_last_error_code()` use **`FfiStatus::InternalError` (`-4`)**, and `ta_last_error()` contains `"internal error: panic at FFI boundary"`.

## Language bindings

Python, Node.js, JVM, and other bindings should map `FfiStatus` values to native exceptions using this table. Detailed tier codes (`10+`, `50+`) can be mapped to more specific exception types when `ta_last_error_code()` is greater than zero.

## Panic isolation

Every `#[no_mangle] pub unsafe extern "C"` entry point in `lib.rs` is wrapped with `ffi_catch_i32`, `ffi_catch_i64`, `ffi_catch_ptr`, or `ffi_catch_void`, each built on `std::panic::catch_unwind`. Host code must still treat invalid pointers as undefined behaviour if passed to the FFI layer; null checks cover explicit null arguments only.
