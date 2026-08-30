// ─────────────────────────────────────────────────────────────────────
// GENERATED FILE — do not edit by hand.
// Source of truth: docs/indicator_registry.json (ffi.bodies.<lang>).
// Regenerate with: python3 scripts/sync_bindings.py --lang java --generate --rewrite
// ─────────────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_sma(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let input_vec = get_double_array(&mut env, input);
    match moving_avg::sma(&input_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_ema(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let input_vec = get_double_array(&mut env, input);
    match moving_avg::ema(&input_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_wma(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let input_vec = get_double_array(&mut env, input);
    match moving_avg::wma(&input_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_dema(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let input_vec = get_double_array(&mut env, input);
    match moving_avg::dema(&input_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_tema(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let input_vec = get_double_array(&mut env, input);
    match moving_avg::tema(&input_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_kama(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let input_vec = get_double_array(&mut env, input);
    match moving_avg::kama(&input_vec, period as usize, 2, 30) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_mama(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    fastlimit: jdouble,
    slowlimit: jdouble,
    result: JObject,
) {
    let input_vec = get_double_array(&mut env, input);
    if let Ok(res) = indicators::mama(&input_vec, fastlimit, slowlimit) {
        let mama_arr = to_double_array(&mut env, res.mama.into_raw_vec_and_offset().0);
        let fama_arr = to_double_array(&mut env, res.fama.into_raw_vec_and_offset().0);
        set_double_field(&mut env, &result, "mama", mama_arr);
        set_double_field(&mut env, &result, "fama", fama_arr);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_t3(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
    vfactor: jdouble,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let input_vec = get_double_array(&mut env, input);
    match indicators::t3(&input_vec, period as usize, vfactor) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_bbands(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    timeperiod: jint,
    nbdevup: jdouble,
    nbdevdn: jdouble,
    result: JObject,
) {
    let input_vec = get_double_array(&mut env, input);
    if let Ok(res) = indicators::bbands(&input_vec, timeperiod as usize, nbdevup, nbdevdn) {
        let upper_arr = to_double_array(&mut env, res.upper.into_raw_vec_and_offset().0);
        let middle_arr = to_double_array(&mut env, res.middle.into_raw_vec_and_offset().0);
        let lower_arr = to_double_array(&mut env, res.lower.into_raw_vec_and_offset().0);
        set_double_field(&mut env, &result, "upper", upper_arr);
        set_double_field(&mut env, &result, "middle", middle_arr);
        set_double_field(&mut env, &result, "lower", lower_arr);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_midpoint(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let input_vec = get_double_array(&mut env, input);
    match indicators::midpoint(&input_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_midprice(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    match indicators::midprice(&high_vec, &low_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_sar(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    acceleration: jdouble,
    maximum: jdouble,
    result: JObject,
) {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let _close_vec = get_double_array(&mut env, close);
    if let Ok(res) = indicators::sar(&high_vec, &low_vec, acceleration, maximum) {
        let sar_arr = to_double_array(&mut env, res.sar.into_raw_vec_and_offset().0);
        let af_arr = to_double_array(&mut env, res.af.into_raw_vec_and_offset().0);
        set_double_field(&mut env, &result, "sar", sar_arr);
        set_double_field(&mut env, &result, "af", af_arr);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_rsi(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let input_vec = get_double_array(&mut env, input);
    match indicators::rsi(&input_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_macd(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    fastperiod: jint,
    slowperiod: jint,
    signalperiod: jint,
    result: JObject,
) {
    let input_vec = get_double_array(&mut env, input);
    if let Ok(res) = indicators::macd(
        &input_vec,
        fastperiod as usize,
        slowperiod as usize,
        signalperiod as usize,
    ) {
        let macd_arr = to_double_array(&mut env, res.macd.into_raw_vec_and_offset().0);
        let signal_arr = to_double_array(&mut env, res.signal.into_raw_vec_and_offset().0);
        let hist_arr = to_double_array(&mut env, res.hist.into_raw_vec_and_offset().0);
        set_double_field(&mut env, &result, "macd", macd_arr);
        set_double_field(&mut env, &result, "signal", signal_arr);
        set_double_field(&mut env, &result, "hist", hist_arr);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_stoch(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    fastk: jint,
    slowk: jint,
    slowd: jint,
    result: JObject,
) {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    if let Ok(res) = indicators::stoch(
        &high_vec,
        &low_vec,
        &close_vec,
        fastk as usize,
        slowk as usize,
        slowd as usize,
    ) {
        let k_arr = to_double_array(&mut env, res.k.into_raw_vec_and_offset().0);
        let d_arr = to_double_array(&mut env, res.d.into_raw_vec_and_offset().0);
        set_double_field(&mut env, &result, "k", k_arr);
        set_double_field(&mut env, &result, "d", d_arr);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_adx(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    match indicators::adx(&high_vec, &low_vec, &close_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_aroon(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    period: jint,
    result: JObject,
) {
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    if let Ok(res) = indicators::aroon(&high_vec, &low_vec, period as usize) {
        let up_arr = to_double_array(&mut env, res.aroon_up.into_raw_vec_and_offset().0);
        let down_arr = to_double_array(&mut env, res.aroon_down.into_raw_vec_and_offset().0);
        set_double_field(&mut env, &result, "aroonUp", up_arr);
        set_double_field(&mut env, &result, "aroonDown", down_arr);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_cci(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    match indicators::cci(&high_vec, &low_vec, &close_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_mom(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let input_vec = get_double_array(&mut env, input);
    match indicators::mom(&input_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_roc(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let input_vec = get_double_array(&mut env, input);
    match indicators::roc(&input_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_willr(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    match indicators::willr(&high_vec, &low_vec, &close_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_apo(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    fastperiod: jint,
    slowperiod: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let input_vec = get_double_array(&mut env, input);
    match indicators::apo(&input_vec, fastperiod as usize, slowperiod as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_bop(
    mut env: JNIEnv,
    _class: JClass,
    open: JDoubleArray,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let open_vec = get_double_array(&mut env, open);
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    match indicators::bop(&open_vec, &high_vec, &low_vec, &close_vec) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_cmo(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let input_vec = get_double_array(&mut env, input);
    match indicators::cmo(&input_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_mfi(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    volume: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    let volume_vec = get_double_array(&mut env, volume);
    match indicators::mfi(
        &high_vec,
        &low_vec,
        &close_vec,
        &volume_vec,
        period as usize,
    ) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_trix(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let input_vec = get_double_array(&mut env, input);
    match indicators::trix(&input_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_atr(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    match indicators::atr(&high_vec, &low_vec, &close_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_natr(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    match indicators::natr(&high_vec, &low_vec, &close_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_trange(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    match indicators::trange(&high_vec, &low_vec, &close_vec) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_obv(
    mut env: JNIEnv,
    _class: JClass,
    close: JDoubleArray,
    volume: JDoubleArray,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let close_vec = get_double_array(&mut env, close);
    let volume_vec = get_double_array(&mut env, volume);
    match indicators::obv(&close_vec, &volume_vec) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_ad(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    volume: JDoubleArray,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    let volume_vec = get_double_array(&mut env, volume);
    match indicators::ad(&high_vec, &low_vec, &close_vec, &volume_vec) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_adosc(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
    volume: JDoubleArray,
    fastperiod: jint,
    slowperiod: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    let volume_vec = get_double_array(&mut env, volume);
    match indicators::adosc(
        &high_vec,
        &low_vec,
        &close_vec,
        &volume_vec,
        fastperiod as usize,
        slowperiod as usize,
    ) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_zscore(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let input_vec = get_double_array(&mut env, input);
    match indicators::zscore(&input_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_beta(
    mut env: JNIEnv,
    _class: JClass,
    asset: JDoubleArray,
    benchmark: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let asset_vec = get_double_array(&mut env, asset);
    let benchmark_vec = get_double_array(&mut env, benchmark);
    match indicators::beta(&asset_vec, &benchmark_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_correlation(
    mut env: JNIEnv,
    _class: JClass,
    inputA: JDoubleArray,
    inputB: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let a_vec = get_double_array(&mut env, inputA);
    let b_vec = get_double_array(&mut env, inputB);
    match indicators::correlation(&a_vec, &b_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_tsf(
    mut env: JNIEnv,
    _class: JClass,
    input: JDoubleArray,
    period: jint,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let input_vec = get_double_array(&mut env, input);
    match indicators::tsf(&input_vec, period as usize) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_avgprice(
    mut env: JNIEnv,
    _class: JClass,
    open: JDoubleArray,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let open_vec = get_double_array(&mut env, open);
    let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    match indicators::avgprice(&open_vec, &high_vec, &low_vec, &close_vec) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_medprice(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    match indicators::medprice(&high_vec, &low_vec) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_typprice(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    match indicators::typprice(&high_vec, &low_vec, &close_vec) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_wclprice(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    close: JDoubleArray,
) -> jdoubleArray {
    ffi_catch_ptr(|| {
let high_vec = get_double_array(&mut env, high);
    let low_vec = get_double_array(&mut env, low);
    let close_vec = get_double_array(&mut env, close);
    match indicators::wclprice(&high_vec, &low_vec, &close_vec) {
        Ok(result) => to_double_array(&mut env, result.into_raw_vec_and_offset().0),
        Err(_) => std::ptr::null_mut(),
    }
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_renko(
    mut env: JNIEnv,
    _class: JClass,
    high: JDoubleArray,
    low: JDoubleArray,
    box_size: jdouble,
) -> jobject {
    ffi_catch_ptr(|| {
let h = get_double_array(&mut env, high);
    let l = get_double_array(&mut env, low);
    let r = indicators::renko(&h, &l, box_size).unwrap();
    build_dto2(env, &r.bricks, &r.direction)
    })
}

#[no_mangle]
pub extern "system" fn Java_com_finkit_Indicators_kagi(
    mut env: JNIEnv,
    _class: JClass,
    close: JDoubleArray,
    reversal: jdouble,
) -> jobject {
    ffi_catch_ptr(|| {
let c = get_double_array(&mut env, close);
    let r = indicators::kagi(&c, reversal).unwrap();
    build_dto2(env, &r.kagi, &r.direction)
    })
}
