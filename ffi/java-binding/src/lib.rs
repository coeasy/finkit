#![allow(non_snake_case)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(missing_docs)]
#![allow(missing_debug_implementations)]
#![allow(deprecated)]

use jni::objects::{JClass, JDoubleArray, JObject, JPrimitiveArray, JString, JValue};
use jni::sys::{jboolean, jdouble, jdoubleArray, jint, jintArray, jobject, jsize};
use jni::JNIEnv;
use ndarray::Array1;
use serde_json;
use finkit::formula::*;
use finkit::indicators;
use finkit::math::moving_avg;
use finkit::patterns::{candlestick, chart};
use finkit_ffi_common::panic::*;

// ============================================================================
// FFI memory-ownership contract (Java)
// ----------------------------------------------------------------------------
// Java exports return JVM-managed handles (`jdoubleArray` / `jobject` /
// `jstring`), NOT Rust-owned heap. Two ownership classes still apply:
//
// 1. JNI local references. Every `jstring` returned to Java (formula evals,
//    K-line `toSvg`, etc.) is a local ref that the *caller's* JNI frame owns
//    and must release with `freeJString` (or let the frame detach). Forgetting
//    to release leaks a JVM local-ref slot. `freeJString` is null-safe.
//
// 2. Long-lived Rust state keyed by handle. `klineDataNew` / `klineChartNew`
//    store Rust objects in process-global maps and return an `i64` handle;
//    they MUST be released with `klineDataFree` / `klineChartFree`. A leaked
//    handle leaks the underlying `KlineData` / `KlineChart` for the process
//    lifetime.
//
// NOTE: these paths require a live `JNIEnv` and therefore cannot be exercised
// by `cargo test` (no JVM in the test binary). The contract is validated by
// the Android / host-JVM integration tests instead. See
// `docs/FFI_MEMORY_CONTRACT.md`.
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_freeJString(
    env: JNIEnv,
    _class: JClass,
    s: jni::sys::jstring,
) {
    ffi_catch_void(|| {
        if !s.is_null() {
            let _ = env.delete_local_ref(unsafe { JString::from_raw(s) });
        }
    })
}

fn get_double_array(env: &mut JNIEnv, arr: JDoubleArray) -> Vec<f64> {
    let len = env.get_array_length(&arr).unwrap() as usize;
    let mut buf = vec![0.0f64; len];
    env.get_double_array_region(&arr, 0, &mut buf).unwrap();
    buf
}

fn to_double_array(env: &mut JNIEnv, data: Vec<f64>) -> jdoubleArray {
    let java_arr = env.new_double_array(data.len() as jsize).unwrap();
    env.set_double_array_region(&java_arr, 0, &data).unwrap();
    java_arr.into_raw()
}

fn to_int_array(env: &mut JNIEnv, data: Vec<i32>) -> jintArray {
    let java_arr = env.new_int_array(data.len() as jsize).unwrap();
    env.set_int_array_region(&java_arr, 0, &data).unwrap();
    java_arr.into_raw()
}

fn set_double_field(env: &mut JNIEnv, obj: &JObject, field: &str, arr: jdoubleArray) {
    let jobj = unsafe { JObject::from_raw(arr) };
    env.set_field(obj, field, "[D", JValue::Object(&jobj))
        .unwrap();
}

fn formula_eval_helper<F>(
    env: &mut JNIEnv,
    source: JString,
    open: JDoubleArray,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    volume: JDoubleArray,
    eval_fn: F,
) -> jobject
where
    F: FnOnce(&mut FormulaEngine, &str, &mut FormulaContext) -> Result<Array1<f64>, FormulaError>,
{
    let source_str: String = match env.get_string(&source) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let open_vec = get_double_array(env, open);
    let high_vec = get_double_array(env, high);
    let low_vec = get_double_array(env, low);
    let close_vec = get_double_array(env, close);
    let volume_vec = get_double_array(env, volume);

    let open_arr = Array1::from_vec(open_vec);
    let high_arr = Array1::from_vec(high_vec);
    let low_arr = Array1::from_vec(low_vec);
    let close_arr = Array1::from_vec(close_vec);
    let volume_arr = Array1::from_vec(volume_vec);

    let mut ctx = FormulaContext::new(open_arr, high_arr, low_arr, close_arr, volume_arr, None);
    let mut engine = FormulaEngine::new();

    let result = eval_fn(&mut engine, &source_str, &mut ctx);
    match result {
        Ok(final_value) => {
            let hashmap_class = match env.find_class("java/util/HashMap") {
                Ok(c) => c,
                Err(_) => return std::ptr::null_mut(),
            };
            let hashmap = match env.new_object(&hashmap_class, "()V", &[]) {
                Ok(o) => o,
                Err(_) => return std::ptr::null_mut(),
            };

            let put_method = "put";
            let put_sig = "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;";

            for (name, value) in ctx.variables {
                let java_arr = to_double_array(env, value.into_raw_vec_and_offset().0);
                let j_name = match env.new_string(name.as_ref()) {
                    Ok(s) => s,
                    Err(_) => return std::ptr::null_mut(),
                };
                let j_obj = unsafe { JObject::from_raw(java_arr) };
                let _ = env.call_method(
                    &hashmap,
                    put_method,
                    put_sig,
                    &[
                        JValue::Object(&JObject::from(j_name)),
                        JValue::Object(&j_obj),
                    ],
                );
            }

            let java_final = to_double_array(env, final_value.into_raw_vec_and_offset().0);
            let j_final_key = match env.new_string("__final__") {
                Ok(s) => s,
                Err(_) => return std::ptr::null_mut(),
            };
            let j_final_obj = unsafe { JObject::from_raw(java_final) };
            let _ = env.call_method(
                &hashmap,
                put_method,
                put_sig,
                &[
                    JValue::Object(&JObject::from(j_final_key)),
                    JValue::Object(&j_final_obj),
                ],
            );

            hashmap.into_raw()
        }
        Err(_) => std::ptr::null_mut(),
    }
}

// ============================================================================
// Overlap Studies
// ============================================================================


#[cfg(test)]
#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_ffiPanicTest() -> jobject {
    ffi_catch_ptr(|| -> jobject { panic!("ffi panic injection test") })
}

include!("generated.rs");

#[cfg(test)]
mod tests {
    use finkit::math::moving_avg::sma;
    use finkit::indicators::momentum::rsi;

    #[test]
    fn export_panic_test_returns_null_not_abort() {
        // A panic inside the generated `ffi_catch_ptr` guard must
        // yield a null `jobject`, never unwind across the JNI boundary.
        let p = crate::Java_com_finkit_Indicators_ffiPanicTest();
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
}


#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_formulaEvalMulti(
    mut env: JNIEnv,
    _class: JClass,
    source: JString,
    open: JDoubleArray,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    volume: JDoubleArray,
) -> jni::sys::jstring {
    let source_str: String = match env.get_string(&source) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let open_vec = get_double_array(&mut env, open);
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    let volume_vec = get_double_array(&mut env, volume);

    let open_arr = Array1::from_vec(open_vec);
    let high_arr = Array1::from_vec(high_vec);
    let low_arr = Array1::from_vec(low_vec);
    let close_arr = Array1::from_vec(close_vec);
    let volume_arr = Array1::from_vec(volume_vec);

    let mut ctx = FormulaContext::new(open_arr, high_arr, low_arr, close_arr, volume_arr, None);
    let mut engine = FormulaEngine::new();

    match engine.eval_multi(&source_str, &mut ctx) {
        Ok(multi) => {
            let mut names_vec = Vec::new();
            let mut values_vec = Vec::new();
            for name in multi.names() {
                names_vec.push(name.clone());
                if let Some(arr) = multi.get(name) {
                    values_vec.push(arr.to_vec());
                } else {
                    values_vec.push(vec![]);
                }
            }

            let json_value = serde_json::json!({
                "names": names_vec,
                "values": values_vec,
                "__result__": multi.final_value.to_vec(),
            });
            let json_str = serde_json::to_string(&json_value).unwrap_or_else(|_| "{}".to_string());
            match env.new_string(json_str) {
                // Caller must free with freeJString.
                Ok(jstr) => jstr.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_formulaEvalDraw(
    mut env: JNIEnv,
    _class: JClass,
    source: JString,
    open: JDoubleArray,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    volume: JDoubleArray,
) -> jni::sys::jstring {
    let source_str: String = match env.get_string(&source) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let open_vec = get_double_array(&mut env, open);
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    let volume_vec = get_double_array(&mut env, volume);

    let open_arr = Array1::from_vec(open_vec);
    let high_arr = Array1::from_vec(high_vec);
    let low_arr = Array1::from_vec(low_vec);
    let close_arr = Array1::from_vec(close_vec);
    let volume_arr = Array1::from_vec(volume_vec);

    let mut ctx = FormulaContext::new(open_arr, high_arr, low_arr, close_arr, volume_arr, None);
    let mut engine = FormulaEngine::new();

    match engine.eval(&source_str, &mut ctx) {
        Ok(_final_value) => {
            let draw_commands = ctx.draw_commands.borrow();
            let json_value = serde_json::json!({
                "drawCommands": draw_commands.commands,
            });
            let json_str = serde_json::to_string(&json_value).unwrap_or_else(|_| "{}".to_string());
            match env.new_string(json_str) {
                // Caller must free with freeJString.
                Ok(jstr) => jstr.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_formulaEvalDebug(
    mut env: JNIEnv,
    _class: JClass,
    source: JString,
    open: JDoubleArray,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    volume: JDoubleArray,
) -> jni::sys::jstring {
    let source_str: String = match env.get_string(&source) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let open_vec = get_double_array(&mut env, open);
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    let volume_vec = get_double_array(&mut env, volume);

    let open_arr = Array1::from_vec(open_vec);
    let high_arr = Array1::from_vec(high_vec);
    let low_arr = Array1::from_vec(low_vec);
    let close_arr = Array1::from_vec(close_vec);
    let volume_arr = Array1::from_vec(volume_vec);

    let mut ctx = FormulaContext::new(open_arr, high_arr, low_arr, close_arr, volume_arr, None);
    let mut engine = FormulaEngine::new();

    match engine.eval_with_debug(&source_str, &mut ctx) {
        Ok((_final_value, debugger)) => {
            let json_value = serde_json::json!({
                "events": debugger.get_events(),
            });
            let json_str = serde_json::to_string(&json_value).unwrap_or_else(|_| "{}".to_string());
            match env.new_string(json_str) {
                // Caller must free with freeJString.
                Ok(jstr) => jstr.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_formulaGetTemplate(
    mut env: JNIEnv,
    _class: JClass,
    name: JString,
) -> jni::sys::jstring {
    let name_str: String = match env.get_string(&name) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let engine = FormulaEngine::new();
    match engine.get_template(&name_str) {
        Some(template) => {
            let json_str = serde_json::to_string(template).unwrap_or_else(|_| "{}".to_string());
            match env.new_string(json_str) {
                // Caller must free with freeJString.
                Ok(jstr) => jstr.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_formulaSearchTemplates(
    mut env: JNIEnv,
    _class: JClass,
    keyword: JString,
) -> jni::sys::jstring {
    let keyword_str: String = match env.get_string(&keyword) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let engine = FormulaEngine::new();
    let templates = engine.search_templates(&keyword_str);
    let json_str = serde_json::to_string(&templates).unwrap_or_else(|_| "[]".to_string());
    match env.new_string(json_str) {
        // Caller must free with freeJString.
        Ok(jstr) => jstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_formulaListCategories(
    env: JNIEnv,
    _class: JClass,
) -> jni::sys::jstring {
    use finkit::formula::templates::FormulaTemplates;

    let categories = FormulaTemplates::categories();
    let json_str = serde_json::to_string(&categories).unwrap_or_else(|_| "[]".to_string());
    match env.new_string(json_str) {
        // Caller must free with freeJString.
        Ok(jstr) => jstr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}























// ============================================================================
// Momentum Indicators
// ============================================================================















#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_dx(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    match indicators::dx(&high_vec, &low_vec, &close_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}













#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_plus_di(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    match indicators::plus_di(&high_vec, &low_vec, &close_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_minus_di(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    match indicators::minus_di(&high_vec, &low_vec, &close_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}



// ============================================================================
// Volume Indicators
// ============================================================================







// ============================================================================
// Volatility Indicators
// ============================================================================







// ============================================================================
// Price Transforms
// ============================================================================









// ============================================================================
// Cycle Indicators (Hilbert Transform)
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_htDcperiod(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
) -> jdoubleArray {
    let input_vec = get_double_array(&mut env, input);
    match indicators::ht_dcperiod(&input_vec) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_htDcphase(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
) -> jdoubleArray {
    let input_vec = get_double_array(&mut env, input);
    match indicators::ht_dcphase(&input_vec) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_htPhasor(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    result: JObject,
) {
    let input_vec = get_double_array(&mut env, input);
    if let Ok((in_phase, quadrature)) = indicators::ht_phasor(&input_vec) {
        let in_phase_arr = to_double_array(&mut env, in_phase.into_raw_vec_and_offset().0);
        let quadrature_arr = to_double_array(&mut env, quadrature.into_raw_vec_and_offset().0);
        set_double_field(&mut env, &result, "inPhase", in_phase_arr);
        set_double_field(&mut env, &result, "quadrature", quadrature_arr);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_htSine(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    result: JObject,
) {
    let input_vec = get_double_array(&mut env, input);
    if let Ok((sine, lead_sine)) = indicators::ht_sine(&input_vec) {
        let sine_arr = to_double_array(&mut env, sine.into_raw_vec_and_offset().0);
        let lead_sine_arr = to_double_array(&mut env, lead_sine.into_raw_vec_and_offset().0);
        set_double_field(&mut env, &result, "sine", sine_arr);
        set_double_field(&mut env, &result, "leadSine", lead_sine_arr);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_htTrendmode(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
) -> jdoubleArray {
    let input_vec = get_double_array(&mut env, input);
    match indicators::ht_trendmode(&input_vec) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_htTrendline(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
) -> jdoubleArray {
    let input_vec = get_double_array(&mut env, input);
    match indicators::ht_trendline(&input_vec) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

// ============================================================================
// Statistical Indicators
// ============================================================================



#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_percentRank(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    let input_vec = get_double_array(&mut env, input);
    match indicators::percent_rank(&input_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}





#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_stdDev(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
    nbDev: jdouble,
) -> jdoubleArray {
    let input_vec = get_double_array(&mut env, input);
    match indicators::std_dev(&input_vec, period as usize, nbDev) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_linearReg(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    let input_vec = get_double_array(&mut env, input);
    match indicators::linear_reg(&input_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}



// ============================================================================
// Candlestick Patterns
// ============================================================================

macro_rules! impl_cdl_pattern {
    ($fn_name:ident, $rust_fn:ident, $arg_name:ident : $arg_type:ty) => {
        #[no_mangle]
        pub extern "system" fn $fn_name(
            mut env: JNIEnv,
            _class: JClass,
            open: JDoubleArray,
            high: JDoubleArray,
            low: JDoubleArray,
            close: JDoubleArray,
            $arg_name: $arg_type,
        ) -> jintArray {
            let open_vec = get_double_array(&mut env, open);
            let high_vec = get_double_array(&mut env, high);
            let low_vec = get_double_array(&mut env, low);
            let close_vec = get_double_array(&mut env, close);
            match candlestick::$rust_fn(&open_vec, &high_vec, &low_vec, &close_vec, $arg_name) {
                Ok(result) => to_int_array(&mut env, result.into_raw_vec_and_offset().0),
                Err(_) => std::ptr::null_mut(),
            }
        }
    };
}

macro_rules! impl_cdl_pattern_5arg {
    ($fn_name:ident, $rust_fn:ident, $default:expr) => {
        #[no_mangle]
        pub extern "system" fn $fn_name(
            mut env: JNIEnv,
            _class: JClass,
            open: JDoubleArray,
            high: JDoubleArray,
            low: JDoubleArray,
            close: JDoubleArray,
        ) -> jintArray {
            let open_vec = get_double_array(&mut env, open);
            let high_vec = get_double_array(&mut env, high);
            let low_vec = get_double_array(&mut env, low);
            let close_vec = get_double_array(&mut env, close);
            match candlestick::$rust_fn(&open_vec, &high_vec, &low_vec, &close_vec, $default) {
                Ok(result) => to_int_array(&mut env, result.into_raw_vec_and_offset().0),
                Err(_) => std::ptr::null_mut(),
            }
        }
    };
}

macro_rules! impl_cdl_pattern_4arg {
    ($fn_name:ident, $rust_fn:ident) => {
        #[no_mangle]
        pub extern "system" fn $fn_name(
            mut env: JNIEnv,
            _class: JClass,
            open: JDoubleArray,
            high: JDoubleArray,
            low: JDoubleArray,
            close: JDoubleArray,
        ) -> jintArray {
            let open_vec = get_double_array(&mut env, open);
            let high_vec = get_double_array(&mut env, high);
            let low_vec = get_double_array(&mut env, low);
            let close_vec = get_double_array(&mut env, close);
            match candlestick::$rust_fn(&open_vec, &high_vec, &low_vec, &close_vec) {
                Ok(result) => to_int_array(&mut env, result.into_raw_vec_and_offset().0),
                Err(_) => std::ptr::null_mut(),
            }
        }
    };
}

impl_cdl_pattern_5arg!(Java_com_finkit_Patterns_cdlDoji, doji, 0.1);
impl_cdl_pattern_5arg!(
    Java_com_finkit_Patterns_cdlDragonflyDoji,
    dragonfly_doji,
    0.1
);
impl_cdl_pattern_5arg!(
    Java_com_finkit_Patterns_cdlGravestoneDoji,
    gravestone_doji,
    0.1
);
impl_cdl_pattern_5arg!(
    Java_com_finkit_Patterns_cdlLongLeggedDoji,
    long_legged_doji,
    1.0
);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlDoji4Prices, doji_4prices);
impl_cdl_pattern_5arg!(Java_com_finkit_Patterns_cdlMarubozu, marubozu, 0.1);
impl_cdl_pattern_5arg!(
    Java_com_finkit_Patterns_cdlMorningDojiStar,
    morning_doji_star,
    0.1
);
impl_cdl_pattern_5arg!(
    Java_com_finkit_Patterns_cdlEveningDojiStar,
    evening_doji_star,
    0.1
);
impl_cdl_pattern_5arg!(
    Java_com_finkit_Patterns_cdlAbandonedBaby,
    abandoned_baby,
    0.3
);

impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlHammer, hammer);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlInvertedHammer,
    inverted_hammer
);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlHangingMan, hanging_man);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlShootingStar, shooting_star);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlEngulfing, engulfing);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlHarami, harami);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlHaramiCross, harami_cross);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlMorningStar, morning_star);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlEveningStar, evening_star);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlThreeWhiteSoldiers,
    three_white_soldiers
);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlThreeBlackCrows,
    three_black_crows
);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlThreeInsideUp,
    three_inside_up
);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlThreeOutsideUp,
    three_outside_up
);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlThreeInsideDown,
    three_inside_down
);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlThreeOutsideDown,
    three_outside_down
);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlThreeStarsInSouth,
    three_stars_in_south
);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlThreeLineStrike,
    three_line_strike
);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlStickSandwich, stick_sandwich);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlBeltHold, belt_hold);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlClosingMarubozu,
    closing_marubozu
);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlSpinningTop, spinning_top);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlHighWave, high_wave);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlRickshawMan, rickshaw_man);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlShortLine, short_line);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlLongLine, long_line);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlPiercing, piercing);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlDarkCloudCover,
    dark_cloud_cover
);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlTweezerTop, tweezer_top);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlTweezerBot, tweezer_bot);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlUpsideGap2Crows,
    upside_gap_2crows
);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlUpsideGap3Methods,
    upside_gap_3methods
);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlMatHold, mat_hold);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlTasukiGap, tasuki_gap);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlSeparatingLines,
    separating_lines
);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlCounterAttack, counter_attack);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlMatchingLow, matching_low);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlIdentical3Crows,
    identical_3crows
);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlUnique3River, unique_3_river);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlBreakaway, breakaway);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlConcealingBabySwallow,
    concealing_baby_swallow
);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlKicking, kicking);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlKickingByLength,
    kicking_by_length
);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlAdvanceBlock, advance_block);
impl_cdl_pattern_4arg!(
    Java_com_finkit_Patterns_cdlStalledPattern,
    stalled_pattern
);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlThrusting, thrusting);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlInNeck, in_neck);
impl_cdl_pattern_4arg!(Java_com_finkit_Patterns_cdlOnNeck, on_neck);

impl_cdl_pattern!(Java_com_finkit_Patterns_cdlDojiWithThreshold, doji, threshold: jdouble);
impl_cdl_pattern!(Java_com_finkit_Patterns_cdlMarubozuWithThreshold, marubozu, threshold: jdouble);
impl_cdl_pattern!(Java_com_finkit_Patterns_cdlMorningDojiStarWithThreshold, morning_doji_star, threshold: jdouble);
impl_cdl_pattern!(Java_com_finkit_Patterns_cdlEveningDojiStarWithThreshold, evening_doji_star, threshold: jdouble);
impl_cdl_pattern!(Java_com_finkit_Patterns_cdlAbandonedBabyWithThreshold, abandoned_baby, threshold: jdouble);

// ============================================================================
// Chart Patterns
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_com_finkit_ChartPatterns_detectHeadShouldersTop(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    minBars: jint,
    headRatio: jdouble,
) -> jintArray {
    let high_vec = get_double_array(&mut env, high);
    match chart::head_and_shoulders_top(&high_vec, minBars as usize, headRatio) {
        Ok(result) => to_int_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_ChartPatterns_detectHeadShouldersBottom(
    mut env: JNIEnv,
    _class: JClass,
    low: JDoubleArray,
    minBars: jint,
    headRatio: jdouble,
) -> jintArray {
    let low_vec = get_double_array(&mut env, low);
    match chart::head_and_shoulders_bottom(&low_vec, minBars as usize, headRatio) {
        Ok(result) => to_int_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_ChartPatterns_detectDoubleTop(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    lookback: jint,
    tolerance: jdouble,
) -> jintArray {
    let high_vec = get_double_array(&mut env, high);
    match chart::double_top(&high_vec, lookback as usize, tolerance) {
        Ok(result) => to_int_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_ChartPatterns_detectDoubleBottom(
    mut env: JNIEnv,
    _class: JClass,
    low: JDoubleArray,
    lookback: jint,
    tolerance: jdouble,
) -> jintArray {
    let low_vec = get_double_array(&mut env, low);
    match chart::double_bottom(&low_vec, lookback as usize, tolerance) {
        Ok(result) => to_int_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_ChartPatterns_detectTripleTop(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    lookback: jint,
    tolerance: jdouble,
) -> jintArray {
    let high_vec = get_double_array(&mut env, high);
    match chart::triple_top(&high_vec, lookback as usize, tolerance) {
        Ok(result) => to_int_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_ChartPatterns_detectTripleBottom(
    mut env: JNIEnv,
    _class: JClass,
    low: JDoubleArray,
    lookback: jint,
    tolerance: jdouble,
) -> jintArray {
    let low_vec = get_double_array(&mut env, low);
    match chart::triple_bottom(&low_vec, lookback as usize, tolerance) {
        Ok(result) => to_int_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_ChartPatterns_detectAscendingTriangle(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    lookback: jint,
    tolerance: jdouble,
) -> jintArray {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    match chart::ascending_triangle(&high_vec, &low_vec, lookback as usize, tolerance) {
        Ok(result) => to_int_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_ChartPatterns_detectDescendingTriangle(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    lookback: jint,
    tolerance: jdouble,
) -> jintArray {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    match chart::descending_triangle(&high_vec, &low_vec, lookback as usize, tolerance) {
        Ok(result) => to_int_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_ChartPatterns_detectSymmetricalTriangle(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    lookback: jint,
) -> jintArray {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    match chart::symmetrical_triangle(&high_vec, &low_vec, lookback as usize) {
        Ok(result) => to_int_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_ChartPatterns_detectRisingWedge(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    lookback: jint,
) -> jintArray {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    match chart::rising_wedge(&high_vec, &low_vec, lookback as usize) {
        Ok(result) => to_int_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_ChartPatterns_detectFallingWedge(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    lookback: jint,
) -> jintArray {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    match chart::falling_wedge(&high_vec, &low_vec, lookback as usize) {
        Ok(result) => to_int_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_ChartPatterns_detectPennant(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    flagpolePeriod: jint,
    pennantPeriod: jint,
) -> jintArray {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    match chart::pennant(
        &high_vec,
        &low_vec,
        &close_vec,
        flagpolePeriod as usize,
        pennantPeriod as usize,
    ) {
        Ok(result) => to_int_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_ChartPatterns_detectFlag(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    flagpolePeriod: jint,
    flagPeriod: jint,
) -> jintArray {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    match chart::flag(
        &high_vec,
        &low_vec,
        &close_vec,
        flagpolePeriod as usize,
        flagPeriod as usize,
    ) {
        Ok(result) => to_int_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_ChartPatterns_detectRectangle(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    lookback: jint,
    tolerance: jdouble,
) -> jintArray {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    match chart::rectangle(&high_vec, &low_vec, lookback as usize, tolerance) {
        Ok(result) => to_int_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

// ============================================================================
// Advanced Indicators
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_ichimoku(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    tenkanPeriod: jint,
    kijunPeriod: jint,
    senkouBPeriod: jint,
    displacement: jint,
    result: JObject,
) {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    if let Ok(res) = indicators::ichimoku(
        &high_vec,
        &low_vec,
        &close_vec,
        tenkanPeriod as usize,
        kijunPeriod as usize,
        senkouBPeriod as usize,
        displacement as usize,
    ) {
        let tenkan_arr = to_double_array(&mut env, res.tenkan_sen.into_raw_vec_and_offset().0);
        let kijun_arr = to_double_array(&mut env, res.kijun_sen.into_raw_vec_and_offset().0);
        let span_a_arr = to_double_array(&mut env, res.senkou_span_a.into_raw_vec_and_offset().0);
        let span_b_arr = to_double_array(&mut env, res.senkou_span_b.into_raw_vec_and_offset().0);
        let chikou_arr = to_double_array(&mut env, res.chikou_span.into_raw_vec_and_offset().0);
        set_double_field(&mut env, &result, "tenkanSen", tenkan_arr);
        set_double_field(&mut env, &result, "kijunSen", kijun_arr);
        set_double_field(&mut env, &result, "senkouSpanA", span_a_arr);
        set_double_field(&mut env, &result, "senkouSpanB", span_b_arr);
        set_double_field(&mut env, &result, "chikouSpan", chikou_arr);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_supertrend(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    atrPeriod: jint,
    multiplier: jdouble,
    result: JObject,
) {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    if let Ok(res) = indicators::supertrend(
        &high_vec,
        &low_vec,
        &close_vec,
        atrPeriod as usize,
        multiplier,
    ) {
        let direction_arr = {
            let len = res.direction.len();
            let java_arr = env.new_int_array(len as jsize).unwrap();
            let data: Vec<i32> = res.direction.iter().copied().collect();
            env.set_int_array_region(&java_arr, 0, &data).unwrap();
            java_arr.into_raw()
        };
        let trend_arr = to_double_array(&mut env, res.trend_line.into_raw_vec_and_offset().0);
        let upper_arr = to_double_array(&mut env, res.upper_band.into_raw_vec_and_offset().0);
        let lower_arr = to_double_array(&mut env, res.lower_band.into_raw_vec_and_offset().0);
        env.set_field(
            &result,
            "direction",
            "[I",
            JValue::Int(direction_arr as i32),
        )
        .unwrap();
        set_double_field(&mut env, &result, "trendLine", trend_arr);
        set_double_field(&mut env, &result, "upperBand", upper_arr);
        set_double_field(&mut env, &result, "lowerBand", lower_arr);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_vwap(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    volume: JDoubleArray,
) -> jdoubleArray {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    let volume_vec = get_double_array(&mut env, volume);
    match indicators::vwap(&high_vec, &low_vec, &close_vec, &volume_vec) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_anchoredVwap(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    volume: JDoubleArray,
    startIndex: jint,
) -> jdoubleArray {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    let volume_vec = get_double_array(&mut env, volume);
    match indicators::anchored_vwap(
        &high_vec,
        &low_vec,
        &close_vec,
        &volume_vec,
        startIndex as usize,
    ) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_vwapBands(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    volume: JDoubleArray,
    timeperiod: jint,
    nbDev: jdouble,
    result: JObject,
) {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    let volume_vec = get_double_array(&mut env, volume);
    if let Ok(res) = indicators::vwap_bands(
        &high_vec,
        &low_vec,
        &close_vec,
        &volume_vec,
        timeperiod as usize,
        nbDev,
    ) {
        let vwap_arr = to_double_array(&mut env, res.vwap.into_raw_vec_and_offset().0);
        let upper_arr = to_double_array(&mut env, res.upper.into_raw_vec_and_offset().0);
        let lower_arr = to_double_array(&mut env, res.lower.into_raw_vec_and_offset().0);
        set_double_field(&mut env, &result, "vwap", vwap_arr);
        set_double_field(&mut env, &result, "upper", upper_arr);
        set_double_field(&mut env, &result, "lower", lower_arr);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_elderRay(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    volume: JDoubleArray,
    period: jint,
    result: JObject,
) {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    let volume_vec = get_double_array(&mut env, volume);
    if let Ok(res) = indicators::elder_ray(
        &high_vec,
        &low_vec,
        &close_vec,
        &volume_vec,
        period as usize,
    ) {
        let force_arr = to_double_array(&mut env, res.force_index.into_raw_vec_and_offset().0);
        let bull_arr = to_double_array(&mut env, res.bull_power.into_raw_vec_and_offset().0);
        let bear_arr = to_double_array(&mut env, res.bear_power.into_raw_vec_and_offset().0);
        set_double_field(&mut env, &result, "forceIndex", force_arr);
        set_double_field(&mut env, &result, "bullPower", bull_arr);
        set_double_field(&mut env, &result, "bearPower", bear_arr);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_donchian(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    period: jint,
    result: JObject,
) {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    if let Ok(res) = indicators::donchian(&high_vec, &low_vec, period as usize) {
        let upper_arr = to_double_array(&mut env, res.upper.into_raw_vec_and_offset().0);
        let lower_arr = to_double_array(&mut env, res.lower.into_raw_vec_and_offset().0);
        let middle_arr = to_double_array(&mut env, res.middle.into_raw_vec_and_offset().0);
        let width_arr = to_double_array(&mut env, res.width.into_raw_vec_and_offset().0);
        set_double_field(&mut env, &result, "upper", upper_arr);
        set_double_field(&mut env, &result, "lower", lower_arr);
        set_double_field(&mut env, &result, "middle", middle_arr);
        set_double_field(&mut env, &result, "width", width_arr);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_volumeProfile(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    volume: JDoubleArray,
    numBins: jint,
    result: JObject,
) {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    let volume_vec = get_double_array(&mut env, volume);
    if let Ok(res) = indicators::volume_profile(
        &high_vec,
        &low_vec,
        &close_vec,
        &volume_vec,
        numBins as usize,
    ) {
        let poc = res.poc;
        let vah = res.vah;
        let val = res.val;
        let profile_arr = to_double_array(&mut env, res.profile);
        let bin_prices_arr = to_double_array(&mut env, res.bin_prices);
        env.set_field(&result, "poc", "D", JValue::Double(poc))
            .unwrap();
        env.set_field(&result, "vah", "D", JValue::Double(vah))
            .unwrap();
        env.set_field(&result, "val", "D", JValue::Double(val))
            .unwrap();
        set_double_field(&mut env, &result, "profile", profile_arr);
        set_double_field(&mut env, &result, "binPrices", bin_prices_arr);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_fibonacciRetracement(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    startIndex: jint,
    endIndex: jint,
    result: JObject,
) {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    if let Ok(res) = indicators::fibonacci_retracement(
        &high_vec,
        &low_vec,
        startIndex as usize,
        endIndex as usize,
    ) {
        let trend = res.trend;
        let high_price = res.high_price;
        let low_price = res.low_price;
        let high_index = res.high_index as i32;
        let low_index = res.low_index as i32;
        let levels_len = res.levels.len();
        let ratios_arr = {
            let java_arr = env.new_double_array(levels_len as jsize).unwrap();
            let data: Vec<f64> = res.levels.iter().map(|l| l.ratio).collect();
            env.set_double_array_region(&java_arr, 0, &data).unwrap();
            java_arr.into_raw()
        };
        let prices_arr = {
            let java_arr = env.new_double_array(levels_len as jsize).unwrap();
            let data: Vec<f64> = res.levels.iter().map(|l| l.price).collect();
            env.set_double_array_region(&java_arr, 0, &data).unwrap();
            java_arr.into_raw()
        };
        env.set_field(&result, "trend", "I", JValue::Int(trend))
            .unwrap();
        env.set_field(&result, "highPrice", "D", JValue::Double(high_price))
            .unwrap();
        env.set_field(&result, "lowPrice", "D", JValue::Double(low_price))
            .unwrap();
        env.set_field(&result, "highIndex", "I", JValue::Int(high_index))
            .unwrap();
        env.set_field(&result, "lowIndex", "I", JValue::Int(low_index))
            .unwrap();
        set_double_field(&mut env, &result, "ratios", ratios_arr);
        set_double_field(&mut env, &result, "prices", prices_arr);
    }
}

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::RwLock;

static KLINE_DATA_HANDLE: AtomicI64 = AtomicI64::new(1);
static KLINE_CHART_HANDLE: AtomicI64 = AtomicI64::new(1);

lazy_static::lazy_static! {
    static ref KLINE_DATA_MAP: RwLock<HashMap<i64, finkit_visualization::data::KlineData>> =
        RwLock::new(HashMap::new());
    static ref KLINE_CHART_MAP: RwLock<HashMap<i64, (finkit_visualization::chart::KlineChart, Option<finkit_visualization::data::KlineData>)>> =
        RwLock::new(HashMap::new());
}

fn get_string_array(env: &mut JNIEnv, arr: jni::sys::jobjectArray) -> Vec<String> {
    let len = env
        .get_array_length(&unsafe { jni::objects::JObjectArray::from_raw(arr) })
        .unwrap() as usize;
    let mut result = Vec::with_capacity(len);
    for i in 0..len {
        let jstr = env
            .get_object_array_element(
                &unsafe { jni::objects::JObjectArray::from_raw(arr) },
                i as jsize,
            )
            .unwrap();
        let s: String = env
            .get_string(&unsafe { jni::objects::JString::from_raw(jstr.into_raw()) })
            .unwrap()
            .into();
        result.push(s);
    }
    result
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_KlineChart_klineDataNew(
    mut env: JNIEnv,
    _class: JClass,
    dates: jni::sys::jobjectArray,
    opens: JDoubleArray,
    highs: JDoubleArray,
    lows: JDoubleArray,
    closes: JDoubleArray,
    volumes: JDoubleArray,
) -> jni::sys::jlong {
    let dates_vec = get_string_array(&mut env, dates);
    let opens_vec = get_double_array(&mut env, opens);
    let highs_vec = get_double_array(&mut env, highs);
    let lows_vec = get_double_array(&mut env, lows);
    let closes_vec = get_double_array(&mut env, closes);
    let volumes_vec = get_double_array(&mut env, volumes);
    let data = finkit_visualization::data::KlineData::new(
        dates_vec,
        opens_vec,
        highs_vec,
        lows_vec,
        closes_vec,
        volumes_vec,
    );
    let handle = KLINE_DATA_HANDLE.fetch_add(1, Ordering::SeqCst);
    KLINE_DATA_MAP.write().unwrap().insert(handle, data);
    handle
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_KlineChart_klineDataFree(
    _env: JNIEnv,
    _class: JClass,
    handle: jni::sys::jlong,
) {
    KLINE_DATA_MAP.write().unwrap().remove(&handle);
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_KlineChart_klineDataValidate(
    _env: JNIEnv,
    _class: JClass,
    handle: jni::sys::jlong,
) -> jni::sys::jboolean {
    let map = KLINE_DATA_MAP.read().unwrap();
    match map.get(&handle) {
        Some(data) => data.validate() as jni::sys::jboolean,
        None => 0,
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_KlineChart_klineChartNew(
    mut env: JNIEnv,
    _class: JClass,
    data_handle: jni::sys::jlong,
    language: jni::objects::JString,
    title: jni::objects::JString,
    width: jint,
    height: jint,
) -> jni::sys::jlong {
    let data_map = KLINE_DATA_MAP.read().unwrap();
    let data = match data_map.get(&data_handle) {
        Some(d) => d.clone(),
        None => return -1,
    };
    drop(data_map);

    let lang_str: String = env.get_string(&language).unwrap().into();
    let title_str: String = env.get_string(&title).unwrap().into();
    let lang = match lang_str.as_str() {
        "zh-CN" | "zh" => finkit_visualization::language::Language::ZhCn,
        _ => finkit_visualization::language::Language::EnUs,
    };
    let config = finkit_visualization::config::ChartConfigBuilder::new()
        .with_title(&title_str)
        .with_language(lang)
        .with_dimensions(width as u32, height as u32)
        .build();
    let mut chart = finkit_visualization::chart::KlineChart::new(config);
    chart.set_data(data.clone());
    let _ = chart.build_draw_list(&data, &[]);
    let handle = KLINE_CHART_HANDLE.fetch_add(1, Ordering::SeqCst);
    KLINE_CHART_MAP
        .write()
        .unwrap()
        .insert(handle, (chart, Some(data)));
    handle
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_KlineChart_klineChartFree(
    _env: JNIEnv,
    _class: JClass,
    handle: jni::sys::jlong,
) {
    KLINE_CHART_MAP.write().unwrap().remove(&handle);
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_KlineChart_klineChartAddMa(
    _env: JNIEnv,
    _class: JClass,
    handle: jni::sys::jlong,
    periods: jintArray,
) {
    let mut map = KLINE_CHART_MAP.write().unwrap();
    if let Some((chart, Some(ref d))) = map.get_mut(&handle) {
        let periods_arr: JPrimitiveArray<i32> = unsafe { JPrimitiveArray::from_raw(periods) };
        let len = _env.get_array_length(&periods_arr).unwrap() as usize;
        let mut buf = vec![0i32; len];
        _env.get_int_array_region(&periods_arr, 0, &mut buf)
            .unwrap();
        let p: Vec<usize> = buf.iter().map(|&x| x as usize).collect();
        chart.add_ma(d, &p);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_KlineChart_klineChartAddMacd(
    _env: JNIEnv,
    _class: JClass,
    handle: jni::sys::jlong,
    fast: jint,
    slow: jint,
    signal: jint,
) {
    let mut map = KLINE_CHART_MAP.write().unwrap();
    if let Some((chart, Some(ref d))) = map.get_mut(&handle) {
        chart.add_macd(d, fast as usize, slow as usize, signal as usize, 1);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_KlineChart_klineChartAddRsi(
    _env: JNIEnv,
    _class: JClass,
    handle: jni::sys::jlong,
    period: jint,
) {
    let mut map = KLINE_CHART_MAP.write().unwrap();
    if let Some((chart, Some(ref d))) = map.get_mut(&handle) {
        chart.add_rsi(d, period as usize, 1);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_KlineChart_klineChartAddBoll(
    _env: JNIEnv,
    _class: JClass,
    handle: jni::sys::jlong,
    period: jint,
    nb_dev: jdouble,
) {
    let mut map = KLINE_CHART_MAP.write().unwrap();
    if let Some((chart, Some(ref d))) = map.get_mut(&handle) {
        chart.add_boll(d, period as usize, nb_dev);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_KlineChart_klineChartSaveAsSvg(
    mut env: JNIEnv,
    _class: JClass,
    handle: jni::sys::jlong,
    path: jni::objects::JString,
) {
    let path_str: String = env.get_string(&path).unwrap().into();
    let map = KLINE_CHART_MAP.read().unwrap();
    if let Some((chart, _)) = map.get(&handle) {
        let _ = chart.save_as_svg(&path_str);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_KlineChart_klineChartToSvg(
    env: JNIEnv,
    _class: JClass,
    handle: jni::sys::jlong,
) -> jni::sys::jstring {
    let map = KLINE_CHART_MAP.read().unwrap();
    if let Some((chart, _)) = map.get(&handle) {
        match chart.to_svg_string() {
            Ok(svg) => {
                let jstr = env.new_string(svg).unwrap();
                // Caller must free with freeJString.
                return jstr.into_raw();
            }
            Err(_) => return std::ptr::null_mut(),
        }
    }
    std::ptr::null_mut()
}

// ============================================================================
// Classic stock-trading chart patterns (FTA-native, added 2026-06-06).
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_darvasBox(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    lookback: jint,
    confirmation: jint,
) -> jobject {
    let h = get_double_array(&mut env, high);
    let l = get_double_array(&mut env, low);
    let c = get_double_array(&mut env, close);
    let lb = if lookback > 0 { lookback as usize } else { 5 };
    let conf = if confirmation > 0 { confirmation as usize } else { 3 };
    let r = indicators::darvas_box(&h, &l, &c, lb, conf).unwrap();
    build_dto_2d_1i(env, &r.box_top, &r.box_bottom, &r.signal)
}





#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_pointAndFigure(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    box_size: jdouble,
    reversal: jint,
) -> jobject {
    let h = get_double_array(&mut env, high);
    let l = get_double_array(&mut env, low);
    let rev = if reversal > 0 { reversal as usize } else { 3 };
    let r = indicators::point_and_figure(&h, &l, box_size, rev).unwrap();
    build_dto3(env, &r.pnf, &r.column_type, &r.new_column)
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_threeLineBreak(
    mut env: JNIEnv,
    _class: JClass,
    close: JDoubleArray,
    lines: jint,
) -> jobject {
    let c = get_double_array(&mut env, close);
    let r = indicators::three_line_break(&c, lines as usize).unwrap();
    build_dto2(env, &r.line, &r.direction)
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_williamsAlligator(
    mut env: JNIEnv,
    _class: JClass,
    close: JDoubleArray,
) -> jobject {
    let c = get_double_array(&mut env, close);
    let r = indicators::williams_alligator(&c).unwrap();
    build_dto_3d(env, &r.jaw, &r.teeth, &r.lips)
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_heikinAshi(
    mut env: JNIEnv,
    _class: JClass,
    open: JDoubleArray,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
) -> jobject {
    let o = get_double_array(&mut env, open);
    let h = get_double_array(&mut env, high);
    let l = get_double_array(&mut env, low);
    let c = get_double_array(&mut env, close);
    let r = indicators::heikin_ashi(&o, &h, &l, &c).unwrap();
    build_dto4(env, &r.ha_open, &r.ha_high, &r.ha_low, &r.ha_close)
}

fn set_int_field(env: &mut JNIEnv, obj: &JObject, field: &str, arr: jintArray) {
    let jobj = unsafe { JObject::from_raw(arr) };
    env.set_field(obj, field, "[I", JValue::Object(&jobj))
        .unwrap();
}

fn build_dto2(
    mut env: JNIEnv,
    a: &Array1<f64>,
    b: &Array1<i32>,
) -> jobject {
    let cls = env.find_class("com/finkit/DoubleOutput").unwrap();
    let obj = env.alloc_object(&cls).unwrap();
    let a_arr = to_double_array(&mut env, a.to_vec());
    let b_arr = {
        let jarr = env.new_int_array(b.len() as jsize).unwrap();
        let mut buf = b.to_vec();
        env.set_int_array_region(&jarr, 0, &mut buf).unwrap();
        jarr.into_raw()
    };
    set_double_field(&mut env, &obj, "a", a_arr);
    set_int_field(&mut env, &obj, "b", b_arr);
    obj.into_raw()
}

/// Triple of (double[], double[], int[]) — used by Darvas Box.
fn build_dto_2d_1i(
    mut env: JNIEnv,
    a: &Array1<f64>,
    b: &Array1<f64>,
    c: &Array1<i32>,
) -> jobject {
    let cls = env.find_class("com/finkit/TripleOutput").unwrap();
    let obj = env.alloc_object(&cls).unwrap();
    let a_arr = to_double_array(&mut env, a.to_vec());
    let b_arr = to_double_array(&mut env, b.to_vec());
    let c_arr = {
        let jarr = env.new_int_array(c.len() as jsize).unwrap();
        let mut buf = c.to_vec();
        env.set_int_array_region(&jarr, 0, &mut buf).unwrap();
        jarr.into_raw()
    };
    set_double_field(&mut env, &obj, "a", a_arr);
    set_double_field(&mut env, &obj, "b", b_arr);
    set_int_field(&mut env, &obj, "c", c_arr);
    obj.into_raw()
}

/// Triple of (double[], int[], int[]) — used by Point & Figure.
fn build_dto3(
    mut env: JNIEnv,
    a: &Array1<f64>,
    b: &Array1<i32>,
    c: &Array1<i32>,
) -> jobject {
    let cls = env.find_class("com/finkit/TripleOutput").unwrap();
    let obj = env.alloc_object(&cls).unwrap();
    let a_arr = to_double_array(&mut env, a.to_vec());
    let b_arr = {
        let jarr = env.new_int_array(b.len() as jsize).unwrap();
        let mut buf = b.to_vec();
        env.set_int_array_region(&jarr, 0, &mut buf).unwrap();
        jarr.into_raw()
    };
    let c_arr = {
        let jarr = env.new_int_array(c.len() as jsize).unwrap();
        let mut buf = c.to_vec();
        env.set_int_array_region(&jarr, 0, &mut buf).unwrap();
        jarr.into_raw()
    };
    set_double_field(&mut env, &obj, "a", a_arr);
    set_int_field(&mut env, &obj, "b", b_arr);
    set_int_field(&mut env, &obj, "c", c_arr);
    obj.into_raw()
}

/// Triple of (double[], double[], double[]) — used by Williams Alligator.
fn build_dto_3d(
    mut env: JNIEnv,
    a: &Array1<f64>,
    b: &Array1<f64>,
    c: &Array1<f64>,
) -> jobject {
    let cls = env.find_class("com/finkit/TripleOutput").unwrap();
    let obj = env.alloc_object(&cls).unwrap();
    let a_arr = to_double_array(&mut env, a.to_vec());
    let b_arr = to_double_array(&mut env, b.to_vec());
    let c_arr = to_double_array(&mut env, c.to_vec());
    set_double_field(&mut env, &obj, "a", a_arr);
    set_double_field(&mut env, &obj, "b", b_arr);
    set_double_field(&mut env, &obj, "c", c_arr);
    obj.into_raw()
}

fn build_dto4(
    mut env: JNIEnv,
    a: &Array1<f64>,
    b: &Array1<f64>,
    c: &Array1<f64>,
    d: &Array1<f64>,
) -> jobject {
    let cls = env.find_class("com/finkit/QuadOutput").unwrap();
    let obj = env.alloc_object(&cls).unwrap();
    let a_arr = to_double_array(&mut env, a.to_vec());
    let b_arr = to_double_array(&mut env, b.to_vec());
    let c_arr = to_double_array(&mut env, c.to_vec());
    let d_arr = to_double_array(&mut env, d.to_vec());
    set_double_field(&mut env, &obj, "a", a_arr);
    set_double_field(&mut env, &obj, "b", b_arr);
    set_double_field(&mut env, &obj, "c", c_arr);
    set_double_field(&mut env, &obj, "d", d_arr);
    obj.into_raw()
}

// ============================================================================ 
// Formula Engine
// ============================================================================

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_formulaEval(
    mut env: JNIEnv,
    _class: JClass,
    source: JString,
    open: JDoubleArray,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    volume: JDoubleArray,
) -> jobject {
    let source_str: String = match env.get_string(&source) {
        Ok(s) => s.into(),
        Err(_) => return std::ptr::null_mut(),
    };

    let open_vec = get_double_array(&mut env, open);
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    let volume_vec = get_double_array(&mut env, volume);

    let _data_len = open_vec.len();
    let open_arr = Array1::from_vec(open_vec);
    let high_arr = Array1::from_vec(high_vec);
    let low_arr = Array1::from_vec(low_vec);
    let close_arr = Array1::from_vec(close_vec);
    let volume_arr = Array1::from_vec(volume_vec);

    let mut ctx = FormulaContext::new(open_arr, high_arr, low_arr, close_arr, volume_arr, None);
    let mut engine = FormulaEngine::new();

    let result = engine.eval(&source_str, &mut ctx);
    match result {
        Ok(final_value) => {
            let hashmap_class = match env.find_class("java/util/HashMap") {
                Ok(c) => c,
                Err(_) => return std::ptr::null_mut(),
            };
            let hashmap = match env.new_object(&hashmap_class, "()V", &[]) {
                Ok(o) => o,
                Err(_) => return std::ptr::null_mut(),
            };

            let put_method = "put";
            let put_sig = "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;";

            for (name, value) in ctx.variables {
                let java_arr = to_double_array(&mut env, value.into_raw_vec_and_offset().0);
                let j_name = match env.new_string(name.as_ref()) {
                    Ok(s) => s,
                    Err(_) => return std::ptr::null_mut(),
                };
                let j_obj = unsafe { JObject::from_raw(java_arr) };
                let _ = env.call_method(
                    &hashmap,
                    put_method,
                    put_sig,
                    &[
                        JValue::Object(&JObject::from(j_name)),
                        JValue::Object(&j_obj),
                    ],
                );
            }

            let java_final = to_double_array(&mut env, final_value.into_raw_vec_and_offset().0);
            let j_final_key = match env.new_string("__final__") {
                Ok(s) => s,
                Err(_) => return std::ptr::null_mut(),
            };
            let j_final_obj = unsafe { JObject::from_raw(java_final) };
            let _ = env.call_method(
                &hashmap,
                put_method,
                put_sig,
                &[
                    JValue::Object(&JObject::from(j_final_key)),
                    JValue::Object(&j_final_obj),
                ],
            );

            hashmap.into_raw()
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_formulaValidate(
    mut env: JNIEnv,
    _class: JClass,
    source: JString,
) -> jboolean {
    let source_str: String = match env.get_string(&source) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };

    let mut engine = FormulaEngine::new();
    match engine.compile(&source_str) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_formulaEvalJit(
    mut env: JNIEnv,
    _class: JClass,
    source: JString,
    open: JDoubleArray,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    volume: JDoubleArray,
) -> jobject {
    formula_eval_helper(&mut env, source, open, high, low, close, volume,
        |engine, source, ctx| engine.eval_jit(source, ctx))
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_formulaEvalSimd(
    mut env: JNIEnv,
    _class: JClass,
    source: JString,
    open: JDoubleArray,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    volume: JDoubleArray,
) -> jobject {
    formula_eval_helper(&mut env, source, open, high, low, close, volume,
        |engine, source, ctx| engine.eval_simd(source, ctx))
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_formulaEvalZeroCopy(
    mut env: JNIEnv,
    _class: JClass,
    source: JString,
    open: JDoubleArray,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    volume: JDoubleArray,
) -> jobject {
    formula_eval_helper(&mut env, source, open, high, low, close, volume,
        |engine, source, ctx| engine.eval_zero_copy(source, ctx))
}
