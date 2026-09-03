//! Panic-isolation helpers for FFI boundaries.
//!
//! A Rust panic must never unwind across an `extern "C"` / `extern "system"`
//! boundary — doing so is undefined behaviour and on most targets aborts the
//! entire host process. Every exported FFI function therefore wraps its body
//! in [`std::panic::catch_unwind`] via one of these guards and returns a safe
//! sentinel (null pointer / zero / `NaN`) on panic instead of propagating.
//!
//! The C and Android bindings already use the same pattern inline; these are
//! the shared, language-neutral variants so the eight bindings no longer each
//! re-implement the boilerplate. The `sync_bindings.py` generator emits the
//! matching `ffi_catch_*` call around every registry-driven generated function.

use std::panic::{catch_unwind, AssertUnwindSafe};

/// Run `f`, returning its `*mut T` result, or a null pointer if it panics.
#[inline]
pub fn ffi_catch_ptr<F, T>(f: F) -> *mut T
where
    F: FnOnce() -> *mut T,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => std::ptr::null_mut(),
    }
}

/// Run `f`, returning its `i32` result, or `0` if it panics.
#[inline]
pub fn ffi_catch_i32<F: FnOnce() -> i32>(f: F) -> i32 {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(0)
}

/// Run `f`, returning its `i32` result, or `-1` if it panics.
///
/// Use for bindings whose error convention is a negative return code
/// (e.g. iOS returns `-1` for invalid input).
#[inline]
pub fn ffi_catch_i32_neg<F: FnOnce() -> i32>(f: F) -> i32 {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(-1)
}

/// Run `f`, returning its `i64` result, or `0` if it panics.
#[inline]
pub fn ffi_catch_i64<F: FnOnce() -> i64>(f: F) -> i64 {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(0)
}

/// Run `f`, returning its `u8` result, or `0` if it panics.
///
/// JNI uses `u8` for `jboolean`, so this keeps Java boolean exports panic-safe.
#[inline]
pub fn ffi_catch_u8<F: FnOnce() -> u8>(f: F) -> u8 {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(0)
}

/// Run `f`, returning its `f64` result, or `NaN` if it panics.
#[inline]
pub fn ffi_catch_f64<F: FnOnce() -> f64>(f: F) -> f64 {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(f64::NAN)
}

/// Run `f` for its side effects, swallowing any panic.
///
/// Use for `extern "C"` functions that return `()` (e.g. memory-free helpers):
/// a panic during the free must not abort the host.
#[inline]
pub fn ffi_catch_void<F: FnOnce()>(f: F) {
    let _ = catch_unwind(AssertUnwindSafe(f));
}
