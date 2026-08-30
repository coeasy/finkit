// ----------------------------------------------------------------------------
// finkit-android — Android JNI entry points.
//
// Thin wrapper that re-exports the JVM binding's `Java_com_rusttalib_*`
// symbols under the Android-specific package
// `com.finkit.indicators.Indicators` so they can be called from a
// `System.loadLibrary("finkit_android")` invocation in a Kotlin/Java
// Android module.
//
// The actual computation lives in the `finkit-java` crate; this file is
// just a relabelling shim so we can produce a separate Android `.aar`
// without recompiling the full indicator surface.
// ----------------------------------------------------------------------------
#![allow(non_snake_case)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(missing_docs)]
#![allow(missing_debug_implementations)]

use jni::objects::{JClass, JDoubleArray};
use jni::sys::{jarray, jdoubleArray, jint, jsize};
use jni::JNIEnv;
use std::panic;

// ---- helpers ---------------------------------------------------------------

fn to_double_array(env: &mut JNIEnv, data: Vec<f64>) -> jdoubleArray {
    let arr: JDoubleArray = env.new_double_array(data.len() as jsize).expect("alloc");
    env.set_double_array_region(&arr, 0, &data).expect("copy");
    arr.into_raw() as jdoubleArray
}

fn from_double_array(env: &mut JNIEnv, arr: jdoubleArray) -> Vec<f64> {
    let arr: JDoubleArray = unsafe { JDoubleArray::from_raw(arr as jarray) };
    let len = env.get_array_length(&arr).expect("len") as usize;
    let mut buf = vec![0.0f64; len];
    env.get_double_array_region(&arr, 0, &mut buf).expect("copy");
    buf
}

// ---- shim: same name, different Java class --------------------------------
//
// The JVM binding exports its JNI symbols under the class name
// `com.rusttalib.Indicators` (legacy package). On Android we use
// `com.finkit.indicators.Indicators`, so we forward to the same Rust
// function bodies but expose them under a fresh JNI symbol name.

macro_rules! shim_indicator {
    ($name:ident, $arg_ty:ty, $out_ty:ty) => {
        #[no_mangle]
        pub extern "system" fn $name(
            mut env: JNIEnv,
            _class: JClass,
            input: $arg_ty,
            period: jint,
        ) -> $out_ty {
            panic::catch_unwind(panic::AssertUnwindSafe(|| {
                let data = from_double_array(&mut env, input);
                // Delegate to the matching pure-Rust core function.
                let result = dispatch_ta(stringify!($name), &data, period as usize);
                to_double_array(&mut env, result)
            }))
            .unwrap_or(std::ptr::null_mut())
        }
    };
}

/// Map an Android JNI indicator name (the C-style `ta_*` name) to the
/// corresponding pure-Rust core function and compute the result.
///
/// The C FFI names (`ta_sma`, `ta_rsi`, …) do not match the core crate's
/// function names (`sma`, `rsi`, …) and they live in different modules, so a
/// small explicit dispatch is the cleanest way to bridge them without pulling
/// raw-pointer `extern "C"` signatures into the JNI shim.
fn dispatch_ta(name: &str, data: &[f64], period: usize) -> Vec<f64> {
    let res: Option<Result<ndarray::Array1<f64>, _>> = match name {
        "ta_sma" => Some(finkit::math::moving_avg::sma(data, period)),
        "ta_ema" => Some(finkit::math::moving_avg::ema(data, period)),
        "ta_wma" => Some(finkit::math::moving_avg::wma(data, period)),
        "ta_dema" => Some(finkit::math::moving_avg::dema(data, period)),
        "ta_tema" => Some(finkit::math::moving_avg::tema(data, period)),
        "ta_rsi" => Some(finkit::indicators::momentum::rsi(data, period)),
        "ta_mom" => Some(finkit::indicators::momentum::mom(data, period)),
        "ta_roc" => Some(finkit::indicators::momentum::roc(data, period)),
        "ta_cmo" => Some(finkit::indicators::momentum::cmo(data, period)),
        "ta_trix" => Some(finkit::indicators::momentum::trix(data, period)),
        "ta_midpoint" => Some(finkit::indicators::overlap::midpoint(data, period)),
        "ta_zscore" => Some(finkit::indicators::statistics::zscore(data, period)),
        "ta_tsf" => Some(finkit::indicators::statistics::tsf(data, period)),
        "ta_linear_reg" => Some(finkit::indicators::statistics::linearreg(data, period)),
        "ta_percent_rank" => Some(finkit::indicators::statistics::percent_rank(data, period)),
        _ => None,
    };
    res.and_then(|r| r.ok()).map(|a| a.to_vec()).unwrap_or_default()
}


include!("generated.rs");







/// ABI version exported to the Android side so the wrapper can refuse
/// to load a `.so` built against an incompatible core.
#[no_mangle]
pub extern "system" fn finkit_android_abi_version() -> jint {
    1
}

/// Library version, mirrors `Cargo.toml`.
#[no_mangle]
pub extern "system" fn finkit_android_version() -> jdoubleArray {
    // Version is encoded as a 3-element double array: [major, minor, patch].
    let v: [f64; 3] = {
        let ver = env!("CARGO_PKG_VERSION");
        let parts: Vec<&str> = ver.split('.').collect();
        [
            parts.first().and_then(|s| s.parse().ok()).unwrap_or(0.0),
            parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0),
            parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0),
        ]
    };
    let mut env = unsafe { std::mem::zeroed::<JNIEnv>() };
    to_double_array(&mut env, v.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_ta_sma() {
        // `math::moving_avg::sma` returns an aligned array: the first
        // `period-1` slots are NaN warm-up, then the rolling values.
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let out = dispatch_ta("ta_sma", &data, 3);
        assert_eq!(out.len(), data.len());
        assert!(out[0].is_nan() && out[1].is_nan());
        assert_eq!(&out[2..], &[2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_dispatch_ta_rsi_shape() {
        let data = vec![44.0, 44.34, 44.09, 43.61, 44.33, 44.83, 45.10, 45.42];
        let out = dispatch_ta("ta_rsi", &data, 5);
        // RSI returns one value per input once the warmup period is met.
        assert_eq!(out.len(), data.len());
    }

    #[test]
    fn test_dispatch_ta_unknown_returns_empty() {
        let data = vec![1.0, 2.0, 3.0];
        let out = dispatch_ta("ta_does_not_exist", &data, 2);
        assert!(out.is_empty());
    }
}
