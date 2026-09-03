// ─────────────────────────────────────────────────────────────────────
// GENERATED FILE — do not edit by hand.
// Source of truth: docs/indicator_registry.json (ffi block).
// Regenerate with: python3 scripts/gen_binding.py --lang c --rewrite-cbinding
// ─────────────────────────────────────────────────────────────────────

#[no_mangle]
pub unsafe extern "C" fn ta_sma(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match moving_avg::sma(data, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_ema(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match moving_avg::ema(data, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_wma(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match moving_avg::wma(data, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_dema(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match moving_avg::dema(data, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_tema(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match moving_avg::tema(data, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_kama(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match moving_avg::kama(data, period as usize, 2, 30) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_mama(
    input: *const f64,
    mama_out: *mut f64,
    fama_out: *mut f64,
    len: i32,
    fast_limit: f64,
    slow_limit: f64,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || mama_out.is_null() || fama_out.is_null() || len <= 0 {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::mama(data, fast_limit, slow_limit) {
        Ok(result) => {
            copy_result(mama_out, &result.mama, len as usize);
            copy_result(fama_out, &result.fama, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_t3(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
    vfactor: f64,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::t3(data, period as usize, vfactor) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_bbands(
    input: *const f64,
    upper: *mut f64,
    middle: *mut f64,
    lower: *mut f64,
    len: i32,
    period: i32,
    nbdevup: f64,
    nbdevdn: f64,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || upper.is_null() || middle.is_null() || lower.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::bbands(data, period as usize, nbdevup, nbdevdn) {
        Ok(result) => {
            copy_result(upper, &result.upper, len as usize);
            copy_result(middle, &result.middle, len as usize);
            copy_result(lower, &result.lower, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_midpoint(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::midpoint(data, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_midprice(
    high: *const f64,
    low: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    match indicators::midprice(h, l, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_sar(
    high: *const f64,
    low: *const f64,
    output: *mut f64,
    len: i32,
    acceleration: f64,
    maximum: f64,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    match indicators::sar(h, l, acceleration, maximum) {
        Ok(result) => {
            copy_result(output, &result.sar, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_rsi(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::rsi(data, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_macd(
    input: *const f64,
    macd_out: *mut f64,
    signal_out: *mut f64,
    hist_out: *mut f64,
    len: i32,
    fast_period: i32,
    slow_period: i32,
    signal_period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || macd_out.is_null() || signal_out.is_null() || hist_out.is_null() || len <= 0 {
        return invalid_input();
    }
    if fast_period <= 0 || slow_period <= 0 || signal_period <= 0 {
        return invalid_input();
    }
    if fast_period as usize > len as usize || slow_period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::macd(data, fast_period as usize, slow_period as usize, signal_period as usize) {
        Ok(result) => {
            copy_result(macd_out, &result.macd, len as usize);
            copy_result(signal_out, &result.signal, len as usize);
            copy_result(hist_out, &result.hist, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_stoch(
    high: *const f64,
    low: *const f64,
    close: *const f64,
    slowk: *mut f64,
    slowd: *mut f64,
    len: i32,
    fastk_period: i32,
    slowk_period: i32,
    slowd_period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || close.is_null() || slowk.is_null() || slowd.is_null() || len <= 0 {
        return invalid_input();
    }
    if fastk_period <= 0 || slowk_period <= 0 || slowd_period <= 0 {
        return invalid_input();
    }
    if fastk_period as usize > len as usize {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match indicators::stoch(h, l, c, fastk_period as usize, slowk_period as usize, slowd_period as usize) {
        Ok(result) => {
            copy_result(slowk, &result.k, len as usize);
            copy_result(slowd, &result.d, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_adx(
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match indicators::adx(h, l, c, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_aroon(
    high: *const f64,
    low: *const f64,
    aroon_up: *mut f64,
    aroon_down: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || aroon_up.is_null() || aroon_down.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    match indicators::aroon(h, l, period as usize) {
        Ok(result) => {
            copy_result(aroon_up, &result.aroon_up, len as usize);
            copy_result(aroon_down, &result.aroon_down, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_cci(
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match indicators::cci(h, l, c, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_mom(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::mom(data, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_roc(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::roc(data, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_willr(
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match indicators::willr(h, l, c, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_apo(
    input: *const f64,
    output: *mut f64,
    len: i32,
    fast_period: i32,
    slow_period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || fast_period <= 0 || slow_period <= 0 {
        return invalid_input();
    }
    if fast_period as usize > len as usize || slow_period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::apo(data, fast_period as usize, slow_period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_bop(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut f64,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match indicators::bop(o, h, l, c) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_cmo(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::cmo(data, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_mfi(
    high: *const f64,
    low: *const f64,
    close: *const f64,
    volume: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || close.is_null() || volume.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    let v = slice::from_raw_parts(volume, len as usize);
    match indicators::mfi(h, l, c, v, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_trix(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::trix(data, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_vortex(
    high: *const f64,
    low: *const f64,
    close: *const f64,
    vi_plus: *mut f64,
    vi_minus: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || close.is_null() || vi_plus.is_null() || vi_minus.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match indicators::vortex(h, l, c, period as usize) {
        Ok(result) => {
            copy_result(vi_plus, &result.vi_plus, len as usize);
            copy_result(vi_minus, &result.vi_minus, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_vzo(
    close: *const f64,
    volume: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if close.is_null() || volume.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let c = slice::from_raw_parts(close, len as usize);
    let v = slice::from_raw_parts(volume, len as usize);
    match indicators::vzo(c, v, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_volume_momentum(
    volume: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if volume.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let v = slice::from_raw_parts(volume, len as usize);
    match indicators::volume_momentum(v, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_volume_roc(
    volume: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if volume.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let v = slice::from_raw_parts(volume, len as usize);
    match indicators::volume_roc(v, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_chande_forecast(
    close: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if close.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let c = slice::from_raw_parts(close, len as usize);
    match indicators::chande_forecast_oscillator(c, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_twiggs_mf(
    high: *const f64,
    low: *const f64,
    close: *const f64,
    volume: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || close.is_null() || volume.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    let v = slice::from_raw_parts(volume, len as usize);
    match indicators::twiggs_money_flow(h, l, c, v, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_inertia(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut f64,
    len: i32,
    rvi_period: i32,
    linreg_period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    if rvi_period <= 0 || linreg_period <= 0 {
        return invalid_input();
    }
    if rvi_period as usize > len as usize || linreg_period as usize > len as usize {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match indicators::inertia(o, h, l, c, rvi_period as usize, linreg_period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_atr(
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match indicators::atr(h, l, c, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_natr(
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match indicators::natr(h, l, c, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_trange(
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut f64,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match indicators::trange(h, l, c) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_obv(
    close: *const f64,
    volume: *const f64,
    output: *mut f64,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if close.is_null() || volume.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let c = slice::from_raw_parts(close, len as usize);
    let v = slice::from_raw_parts(volume, len as usize);
    match indicators::obv(c, v) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_ad(
    high: *const f64,
    low: *const f64,
    close: *const f64,
    volume: *const f64,
    output: *mut f64,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || close.is_null() || volume.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    let v = slice::from_raw_parts(volume, len as usize);
    match indicators::ad(h, l, c, v) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_adosc(
    high: *const f64,
    low: *const f64,
    close: *const f64,
    volume: *const f64,
    output: *mut f64,
    len: i32,
    fast_period: i32,
    slow_period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || close.is_null() || volume.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    if fast_period <= 0 || slow_period <= 0 {
        return invalid_input();
    }
    if fast_period as usize > len as usize || slow_period as usize > len as usize {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    let v = slice::from_raw_parts(volume, len as usize);
    match indicators::adosc(h, l, c, v, fast_period as usize, slow_period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_ht_dcperiod(
    input: *const f64,
    output: *mut f64,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::ht_dcperiod(data) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_ht_dcphase(
    input: *const f64,
    output: *mut f64,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::ht_dcphase(data) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_ht_phasor(
    input: *const f64,
    in_phase: *mut f64,
    quadrature: *mut f64,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || in_phase.is_null() || quadrature.is_null() || len <= 0 {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::ht_phasor(data) {
        Ok((ip, quad)) => {
            copy_result(in_phase, &ip, len as usize);
            copy_result(quadrature, &quad, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_ht_sine(
    input: *const f64,
    sine: *mut f64,
    lead_sine: *mut f64,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || sine.is_null() || lead_sine.is_null() || len <= 0 {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::ht_sine(data) {
        Ok((s, ls)) => {
            copy_result(sine, &s, len as usize);
            copy_result(lead_sine, &ls, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_ht_trendmode(
    input: *const f64,
    output: *mut f64,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::ht_trendmode(data) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_ht_trendline(
    input: *const f64,
    output: *mut f64,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::ht_trendline(data) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_zscore(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::zscore(data, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_beta(
    asset: *const f64,
    benchmark: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if asset.is_null() || benchmark.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let a = slice::from_raw_parts(asset, len as usize);
    let b = slice::from_raw_parts(benchmark, len as usize);
    match indicators::beta(a, b, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_correlation(
    input_a: *const f64,
    input_b: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input_a.is_null() || input_b.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let a = slice::from_raw_parts(input_a, len as usize);
    let b = slice::from_raw_parts(input_b, len as usize);
    match indicators::correlation(a, b, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_stddev(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
    nb_dev: f64,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::std_dev(data, period as usize, nb_dev) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_tsf(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::tsf(data, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_linear_reg(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::linearreg(data, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_percent_rank(
    input: *const f64,
    output: *mut f64,
    len: i32,
    period: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if input.is_null() || output.is_null() || len <= 0 || period <= 0 {
        return invalid_input();
    }
    if period as usize > len as usize {
        return invalid_input();
    }
    let data = slice::from_raw_parts(input, len as usize);
    match indicators::percent_rank(data, period as usize) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_avgprice(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut f64,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match indicators::avgprice(o, h, l, c) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_medprice(
    high: *const f64,
    low: *const f64,
    output: *mut f64,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    match indicators::medprice(h, l) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_typprice(
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut f64,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match indicators::typprice(h, l, c) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_wclprice(
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut f64,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match indicators::wclprice(h, l, c) {
        Ok(result) => {
            copy_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_cdl_doji(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut i32,
    len: i32,
    doji_pct: f64,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match candlestick::doji(o, h, l, c, doji_pct) {
        Ok(result) => {
            copy_int_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_cdl_dragonfly_doji(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut i32,
    len: i32,
    doji_pct: f64,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match candlestick::dragonfly_doji(o, h, l, c, doji_pct) {
        Ok(result) => {
            copy_int_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_cdl_gravestone_doji(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut i32,
    len: i32,
    doji_pct: f64,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match candlestick::gravestone_doji(o, h, l, c, doji_pct) {
        Ok(result) => {
            copy_int_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_cdl_long_legged_doji(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut i32,
    len: i32,
    doji_pct: f64,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match candlestick::long_legged_doji(o, h, l, c, doji_pct) {
        Ok(result) => {
            copy_int_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_cdl_hammer(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut i32,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match candlestick::hammer(o, h, l, c) {
        Ok(result) => {
            copy_int_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_cdl_inverted_hammer(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut i32,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match candlestick::inverted_hammer(o, h, l, c) {
        Ok(result) => {
            copy_int_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_cdl_hanging_man(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut i32,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match candlestick::hanging_man(o, h, l, c) {
        Ok(result) => {
            copy_int_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_cdl_shooting_star(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut i32,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match candlestick::shooting_star(o, h, l, c) {
        Ok(result) => {
            copy_int_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_cdl_engulfing(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut i32,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match candlestick::engulfing(o, h, l, c) {
        Ok(result) => {
            copy_int_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_cdl_harami(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut i32,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match candlestick::harami(o, h, l, c) {
        Ok(result) => {
            copy_int_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_cdl_morning_star(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut i32,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match candlestick::morning_star(o, h, l, c) {
        Ok(result) => {
            copy_int_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_cdl_evening_star(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut i32,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match candlestick::evening_star(o, h, l, c) {
        Ok(result) => {
            copy_int_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_cdl_three_white_soldiers(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut i32,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match candlestick::three_white_soldiers(o, h, l, c) {
        Ok(result) => {
            copy_int_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_cdl_three_black_crows(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut i32,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match candlestick::three_black_crows(o, h, l, c) {
        Ok(result) => {
            copy_int_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_cdl_marubozu(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    output: *mut i32,
    len: i32,
    shadow_pct: f64,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || output.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match candlestick::marubozu(o, h, l, c, shadow_pct) {
        Ok(result) => {
            copy_int_result(output, &result, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_darvas_box(
    high: *const f64,
    low: *const f64,
    close: *const f64,
    out_top: *mut f64,
    out_bottom: *mut f64,
    out_signal: *mut i32,
    len: i32,
    lookback: i32,
    confirmation: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || close.is_null() || len <= 0 {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    let lb = if lookback > 0 { lookback as usize } else { 5 };
    let conf = if confirmation > 0 { confirmation as usize } else { 3 };
    match indicators::darvas_box(h, l, c, lb, conf) {
        Ok(r) => {
            if !out_top.is_null() {
                copy_result(out_top, &r.box_top, len as usize);
            }
            if !out_bottom.is_null() {
                copy_result(out_bottom, &r.box_bottom, len as usize);
            }
            if !out_signal.is_null() {
                copy_int_result(out_signal, &r.signal, len as usize);
            }
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_renko(
    high: *const f64,
    low: *const f64,
    out_bricks: *mut f64,
    out_dir: *mut i32,
    len: i32,
    box_size: f64,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || out_bricks.is_null() || len <= 0 || box_size <= 0.0 {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    match indicators::renko(h, l, box_size) {
        Ok(r) => {
            copy_result(out_bricks, &r.bricks, len as usize);
            if !out_dir.is_null() {
                copy_int_result(out_dir, &r.direction, len as usize);
            }
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_kagi(
    close: *const f64,
    out_kagi: *mut f64,
    out_dir: *mut i32,
    len: i32,
    reversal: f64,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if close.is_null() || out_kagi.is_null() || len <= 0 || reversal <= 0.0 {
        return invalid_input();
    }
    let c = slice::from_raw_parts(close, len as usize);
    match indicators::kagi(c, reversal) {
        Ok(r) => {
            copy_result(out_kagi, &r.kagi, len as usize);
            if !out_dir.is_null() {
                copy_int_result(out_dir, &r.direction, len as usize);
            }
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_point_and_figure(
    high: *const f64,
    low: *const f64,
    out_pnf: *mut f64,
    out_col: *mut i32,
    out_new: *mut i32,
    len: i32,
    box_size: f64,
    reversal: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if high.is_null() || low.is_null() || out_pnf.is_null() || len <= 0 || box_size <= 0.0 {
        return invalid_input();
    }
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let rev = if reversal > 0 { reversal as usize } else { 3 };
    match indicators::point_and_figure(h, l, box_size, rev) {
        Ok(r) => {
            copy_result(out_pnf, &r.pnf, len as usize);
            if !out_col.is_null() {
                copy_int_result(out_col, &r.column_type, len as usize);
            }
            if !out_new.is_null() {
                copy_int_result(out_new, &r.new_column, len as usize);
            }
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_three_line_break(
    close: *const f64,
    out_line: *mut f64,
    out_dir: *mut i32,
    len: i32,
    lines: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if close.is_null() || out_line.is_null() || len <= 0 || lines <= 0 {
        return invalid_input();
    }
    let c = slice::from_raw_parts(close, len as usize);
    let n = lines as usize;
    match indicators::three_line_break(c, n) {
        Ok(r) => {
            copy_result(out_line, &r.line, len as usize);
            if !out_dir.is_null() {
                copy_int_result(out_dir, &r.direction, len as usize);
            }
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_williams_alligator(
    close: *const f64,
    out_jaw: *mut f64,
    out_teeth: *mut f64,
    out_lips: *mut f64,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if close.is_null() || out_jaw.is_null() || out_teeth.is_null() || out_lips.is_null() || len <= 0 {
        return invalid_input();
    }
    let c = slice::from_raw_parts(close, len as usize);
    match indicators::williams_alligator(c) {
        Ok(r) => {
            copy_result(out_jaw, &r.jaw, len as usize);
            copy_result(out_teeth, &r.teeth, len as usize);
            copy_result(out_lips, &r.lips, len as usize);
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}


#[no_mangle]
pub unsafe extern "C" fn ta_heikin_ashi(
    open: *const f64,
    high: *const f64,
    low: *const f64,
    close: *const f64,
    out_o: *mut f64,
    out_h: *mut f64,
    out_l: *mut f64,
    out_c: *mut f64,
    len: i32,
) -> i32 {

    ffi_catch_i32(|| unsafe {

    if open.is_null() || high.is_null() || low.is_null() || close.is_null() || len <= 0 {
        return invalid_input();
    }
    let o = slice::from_raw_parts(open, len as usize);
    let h = slice::from_raw_parts(high, len as usize);
    let l = slice::from_raw_parts(low, len as usize);
    let c = slice::from_raw_parts(close, len as usize);
    match indicators::heikin_ashi(o, h, l, c) {
        Ok(r) => {
            if !out_o.is_null() {
                copy_result(out_o, &r.ha_open, len as usize);
            }
            if !out_h.is_null() {
                copy_result(out_h, &r.ha_high, len as usize);
            }
            if !out_l.is_null() {
                copy_result(out_l, &r.ha_low, len as usize);
            }
            if !out_c.is_null() {
                copy_result(out_c, &r.ha_close, len as usize);
            }
            TA_OK
        }
        Err(e) => calc_error(&e),
    }
    })
}

