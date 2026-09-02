// ----------------------------------------------------------------------------
// finkit-ios — Objective-C bridge for the Finkit Rust core.
//
// The crate is compiled as a `staticlib` and linked directly into the Swift
// wrapper that ships in the .xcframework (see `include/finkit.h`).
//
// All exported symbols follow the C ABI, are marked `#[no_mangle] extern "C"`,
// and are safe to call from the simulator (aarch64-apple-ios-sim) and from
// physical devices (aarch64-apple-ios / x86_64-apple-ios).
// ----------------------------------------------------------------------------
#![allow(non_snake_case)]
#![allow(missing_docs)]

use finkit::indicators;
use finkit::math::moving_avg;
use finkit::patterns;
use finkit_ffi_common::panic::*;
use std::slice;

// Leak-detection allocator: installed only for the test binary. iOS indicators
// write into a caller-owned buffer (no heap transfer across the FFI boundary),
// but this still guards against the Rust side leaking internal scratch buffers.
// See `finkit_ffi_common::leak`.
#[cfg(test)]
#[global_allocator]
static TEST_ALLOC: finkit_ffi_common::leak::CountingAlloc = finkit_ffi_common::leak::CountingAlloc;

/// ABI version exported to the Swift wrapper so the framework can refuse
/// to load a `.a` built against an incompatible core.
#[no_mangle]
pub extern "C" fn alpha_ta_ios_abi_version() -> i32 {
    1
}

fn from_raw<'a>(input: *const f64, len: i32) -> &'a [f64] {
    if input.is_null() || len <= 0 {
        return &[];
    }
    unsafe { slice::from_raw_parts(input, len as usize) }
}

fn write_result(out: *mut f64, values: &[f64]) -> bool {
    if out.is_null() {
        return false;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(values.as_ptr(), out, values.len());
    }
    true
}

// ---- moving averages -------------------------------------------------------

#[cfg(test)]
#[no_mangle]
pub extern "C" fn alpha_ta_ffi_panic_test() -> i32 {
    ffi_catch_i32_neg(|| -> i32 { panic!("ffi panic injection test") })
}

include!("generated.rs");

// ---- momentum --------------------------------------------------------------

// ---- candlestick patterns --------------------------------------------------
#[no_mangle]
pub extern "C" fn alpha_ta_detect_candlestick(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    len: i32,
) -> i32 {
    ffi_catch_i32_neg(|| {
        if open.is_null() || high.is_null() || low.is_null() || close.is_null() || len <= 0 {
            return -1;
        }
        let o = from_raw(open, len);
        let h = from_raw(high, len);
        let l = from_raw(low, len);
        let c = from_raw(close, len);
        // Return count of detected patterns (sum of non-zero signals).
        let doji = patterns::candlestick::doji(o, h, l, c, 0.05).unwrap_or_default();
        let hammer = patterns::candlestick::hammer(o, h, l, c).unwrap_or_default();
        let engulfing = patterns::candlestick::engulfing(o, h, l, c).unwrap_or_default();

        (doji.iter().filter(|&&x| x != 0).count()
            + hammer.iter().filter(|&&x| x != 0).count()
            + engulfing.iter().filter(|&&x| x != 0).count()) as i32
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_panic_test_returns_neg_one_not_abort() {
        // A panic inside the generated `ffi_catch_i32_neg` guard must
        // yield -1, never unwind across the FFI boundary.
        let code = crate::alpha_ta_ffi_panic_test();
        assert_eq!(code, -1);
    }

    #[test]
    fn test_alpha_ta_sma() {
        // `moving_avg::sma` returns an aligned array: first `period-1` slots
        // are NaN warm-up, then the rolling values.
        let input = [1.0, 2.0, 3.0, 4.0, 5.0];
        let mut out = vec![0.0f64; input.len()];
        let ret = alpha_ta_sma(input.as_ptr(), input.len() as i32, 3, out.as_mut_ptr());
        assert_eq!(ret, 0);
        assert_eq!(out.len(), input.len());
        assert!(out[0].is_nan() && out[1].is_nan());
        assert_eq!(&out[2..], &[2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_alpha_ta_rsi_shape() {
        let input = [44.0, 44.34, 44.09, 43.61, 44.33, 44.83, 45.10, 45.42];
        let mut out = vec![0.0f64; input.len()];
        let ret = alpha_ta_rsi(input.as_ptr(), input.len() as i32, 5, out.as_mut_ptr());
        assert_eq!(ret, 0);
        assert_eq!(out.len(), input.len());
    }

    #[test]
    fn test_alpha_ta_short_input_returns_error() {
        let input = [1.0, 2.0];
        let mut out = vec![0.0f64; 4];
        let ret = alpha_ta_sma(input.as_ptr(), input.len() as i32, 5, out.as_mut_ptr());
        // period > len must be rejected with -1 (no write to out).
        assert_eq!(ret, -1);
    }

    // A4 — even though iOS transfers no heap ownership (caller owns the `out`
    // buffer), this loops the indicator + candlestick paths and asserts the
    // Rust side's internal scratch allocations are fully reclaimed each cycle.
    #[test]
    fn ffi_heap_no_leak_indicator_cycle() {
        use finkit_ffi_common::leak::live_bytes;
        let n: i32 = 512;
        let input: Vec<f64> = (0..n as usize).map(|i| (i as f64).sin()).collect();

        for _ in 0..16 {
            let mut out = vec![0.0f64; n as usize];
            let ret = crate::alpha_ta_sma(input.as_ptr(), n, 14, out.as_mut_ptr());
            assert_eq!(ret, 0);
        }
        let baseline = live_bytes();

        for _ in 0..400 {
            let mut out = vec![0.0f64; n as usize];
            let ret = crate::alpha_ta_sma(input.as_ptr(), n, 14, out.as_mut_ptr());
            assert_eq!(ret, 0);
            let cnt = crate::alpha_ta_detect_candlestick(
                input.as_ptr(),
                input.as_ptr(),
                input.as_ptr(),
                input.as_ptr(),
                n,
            );
            let _ = cnt;
        }

        let after = live_bytes();
        let growth = (after - baseline).abs();
        assert!(
            growth < 256 * 1024,
            "heap grew by {} bytes across 400 indicator cycles (baseline={}, after={})",
            after - baseline,
            baseline,
            after
        );
    }
}
