// ─────────────────────────────────────────────────────────────────────
// GENERATED FILE — do not edit by hand.
// Source of truth: docs/indicator_registry.json (ffi.bodies.<lang>).
// Regenerate with: python3 scripts/sync_bindings.py --lang dotnet --generate --rewrite
// ─────────────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn ta_sma(
    input: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out.is_null() || length <= 0 || period <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match moving_avg::sma(input_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_ema(
    input: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out.is_null() || length <= 0 || period <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match moving_avg::ema(input_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_wma(
    input: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out.is_null() || length <= 0 || period <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match moving_avg::wma(input_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_dema(
    input: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out.is_null() || length <= 0 || period <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match moving_avg::dema(input_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_tema(
    input: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out.is_null() || length <= 0 || period <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match moving_avg::tema(input_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_kama(
    input: *const c_double,
    length: c_int,
    period: c_int,
    fast_period: c_int,
    slow_period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out.is_null() || length <= 0 || period <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match moving_avg::kama(
        input_slice,
        period as usize,
        fast_period as usize,
        slow_period as usize,
    ) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_mama(
    input: *const c_double,
    length: c_int,
    fast_limit: c_double,
    slow_limit: c_double,
    out_mama: *mut c_double,
    out_fama: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out_mama.is_null() || out_fama.is_null() || length <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match indicators::mama(input_slice, fast_limit, slow_limit) {
        Ok(result) => {
            let mama_slice = unsafe { std::slice::from_raw_parts_mut(out_mama, length as usize) };
            let fama_slice = unsafe { std::slice::from_raw_parts_mut(out_fama, length as usize) };
            for i in 0..length as usize {
                mama_slice[i] = result.mama[i];
                fama_slice[i] = result.fama[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_t3(
    input: *const c_double,
    length: c_int,
    period: c_int,
    vfactor: c_double,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out.is_null() || length <= 0 || period <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match indicators::t3(input_slice, period as usize, vfactor) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_bbands(
    input: *const c_double,
    length: c_int,
    period: c_int,
    nb_dev_up: c_double,
    nb_dev_dn: c_double,
    out_upper: *mut c_double,
    out_middle: *mut c_double,
    out_lower: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null()
        || out_upper.is_null()
        || out_middle.is_null()
        || out_lower.is_null()
        || length <= 0
        || period <= 0
    {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match indicators::bbands(input_slice, period as usize, nb_dev_up, nb_dev_dn) {
        Ok(result) => {
            let upper_slice = unsafe { std::slice::from_raw_parts_mut(out_upper, length as usize) };
            let middle_slice =
                unsafe { std::slice::from_raw_parts_mut(out_middle, length as usize) };
            let lower_slice = unsafe { std::slice::from_raw_parts_mut(out_lower, length as usize) };
            for i in 0..length as usize {
                upper_slice[i] = result.upper[i];
                middle_slice[i] = result.middle[i];
                lower_slice[i] = result.lower[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_rsi(
    input: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out.is_null() || length <= 0 || period <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match indicators::rsi(input_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_macd(
    input: *const c_double,
    length: c_int,
    fast_period: c_int,
    slow_period: c_int,
    signal_period: c_int,
    out_macd: *mut c_double,
    out_signal: *mut c_double,
    out_hist: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null()
        || out_macd.is_null()
        || out_signal.is_null()
        || out_hist.is_null()
        || length <= 0
    {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match indicators::macd(
        input_slice,
        fast_period as usize,
        slow_period as usize,
        signal_period as usize,
    ) {
        Ok(result) => {
            let macd_slice = unsafe { std::slice::from_raw_parts_mut(out_macd, length as usize) };
            let signal_slice =
                unsafe { std::slice::from_raw_parts_mut(out_signal, length as usize) };
            let hist_slice = unsafe { std::slice::from_raw_parts_mut(out_hist, length as usize) };
            for i in 0..length as usize {
                macd_slice[i] = result.macd[i];
                signal_slice[i] = result.signal[i];
                hist_slice[i] = result.hist[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_stoch(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    k_period: c_int,
    k_slow: c_int,
    d_period: c_int,
    out_k: *mut c_double,
    out_d: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if high.is_null()
        || low.is_null()
        || close.is_null()
        || out_k.is_null()
        || out_d.is_null()
        || length <= 0
    {
        return -1;
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    match indicators::stoch(
        high_slice,
        low_slice,
        close_slice,
        k_period as usize,
        k_slow as usize,
        d_period as usize,
    ) {
        Ok(result) => {
            let k_slice = unsafe { std::slice::from_raw_parts_mut(out_k, length as usize) };
            let d_slice = unsafe { std::slice::from_raw_parts_mut(out_d, length as usize) };
            for i in 0..length as usize {
                k_slice[i] = result.k[i];
                d_slice[i] = result.d[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_adx(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if high.is_null()
        || low.is_null()
        || close.is_null()
        || out.is_null()
        || length <= 0
        || period <= 0
    {
        return -1;
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    match indicators::adx(high_slice, low_slice, close_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_aroon(
    high: *const c_double,
    low: *const c_double,
    length: c_int,
    period: c_int,
    out_up: *mut c_double,
    out_down: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if high.is_null()
        || low.is_null()
        || out_up.is_null()
        || out_down.is_null()
        || length <= 0
        || period <= 0
    {
        return -1;
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    match indicators::aroon(high_slice, low_slice, period as usize) {
        Ok(result) => {
            let up_slice = unsafe { std::slice::from_raw_parts_mut(out_up, length as usize) };
            let down_slice = unsafe { std::slice::from_raw_parts_mut(out_down, length as usize) };
            for i in 0..length as usize {
                up_slice[i] = result.aroon_up[i];
                down_slice[i] = result.aroon_down[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_cci(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if high.is_null()
        || low.is_null()
        || close.is_null()
        || out.is_null()
        || length <= 0
        || period <= 0
    {
        return -1;
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    match indicators::cci(high_slice, low_slice, close_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_mom(
    input: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out.is_null() || length <= 0 || period <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match indicators::mom(input_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_roc(
    input: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out.is_null() || length <= 0 || period <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match indicators::roc(input_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_willr(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if high.is_null()
        || low.is_null()
        || close.is_null()
        || out.is_null()
        || length <= 0
        || period <= 0
    {
        return -1;
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    match indicators::willr(high_slice, low_slice, close_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_atr(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if high.is_null()
        || low.is_null()
        || close.is_null()
        || out.is_null()
        || length <= 0
        || period <= 0
    {
        return -1;
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    match indicators::atr(high_slice, low_slice, close_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_natr(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if high.is_null()
        || low.is_null()
        || close.is_null()
        || out.is_null()
        || length <= 0
        || period <= 0
    {
        return -1;
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    match indicators::natr(high_slice, low_slice, close_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_trange(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if high.is_null() || low.is_null() || close.is_null() || out.is_null() || length <= 0 {
        return -1;
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    match indicators::trange(high_slice, low_slice, close_slice) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_obv(
    close: *const c_double,
    volume: *const c_double,
    length: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if close.is_null() || volume.is_null() || out.is_null() || length <= 0 {
        return -1;
    }
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    let volume_slice = unsafe { std::slice::from_raw_parts(volume, length as usize) };
    match indicators::obv(close_slice, volume_slice) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_ad(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    volume: *const c_double,
    length: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if high.is_null()
        || low.is_null()
        || close.is_null()
        || volume.is_null()
        || out.is_null()
        || length <= 0
    {
        return -1;
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    let volume_slice = unsafe { std::slice::from_raw_parts(volume, length as usize) };
    match indicators::ad(high_slice, low_slice, close_slice, volume_slice) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_ht_dcperiod(
    input: *const c_double,
    length: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out.is_null() || length <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match indicators::ht_dcperiod(input_slice) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_ht_dcphase(
    input: *const c_double,
    length: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out.is_null() || length <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match indicators::ht_dcphase(input_slice) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_ht_phasor(
    input: *const c_double,
    length: c_int,
    out_inphase: *mut c_double,
    out_quadrature: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out_inphase.is_null() || out_quadrature.is_null() || length <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match indicators::ht_phasor(input_slice) {
        Ok((in_phase, quadrature)) => {
            let inphase_slice =
                unsafe { std::slice::from_raw_parts_mut(out_inphase, length as usize) };
            let quadrature_slice =
                unsafe { std::slice::from_raw_parts_mut(out_quadrature, length as usize) };
            for i in 0..length as usize {
                inphase_slice[i] = in_phase[i];
                quadrature_slice[i] = quadrature[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_ht_sine(
    input: *const c_double,
    length: c_int,
    out_sine: *mut c_double,
    out_lead: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out_sine.is_null() || out_lead.is_null() || length <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match indicators::ht_sine(input_slice) {
        Ok((sine, lead_sine)) => {
            let sine_slice = unsafe { std::slice::from_raw_parts_mut(out_sine, length as usize) };
            let lead_slice = unsafe { std::slice::from_raw_parts_mut(out_lead, length as usize) };
            for i in 0..length as usize {
                sine_slice[i] = sine[i];
                lead_slice[i] = lead_sine[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_ht_trendmode(
    input: *const c_double,
    length: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out.is_null() || length <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match indicators::ht_trendmode(input_slice) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_ht_trendline(
    input: *const c_double,
    length: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out.is_null() || length <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match indicators::ht_trendline(input_slice) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_zscore(
    input: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out.is_null() || length <= 0 || period <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match indicators::zscore(input_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_beta(
    asset: *const c_double,
    benchmark: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if asset.is_null() || benchmark.is_null() || out.is_null() || length <= 0 || period <= 0 {
        return -1;
    }
    let asset_slice = unsafe { std::slice::from_raw_parts(asset, length as usize) };
    let benchmark_slice = unsafe { std::slice::from_raw_parts(benchmark, length as usize) };
    match indicators::beta(asset_slice, benchmark_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_correlation(
    input_a: *const c_double,
    input_b: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input_a.is_null() || input_b.is_null() || out.is_null() || length <= 0 || period <= 0 {
        return -1;
    }
    let a_slice = unsafe { std::slice::from_raw_parts(input_a, length as usize) };
    let b_slice = unsafe { std::slice::from_raw_parts(input_b, length as usize) };
    match indicators::correlation(a_slice, b_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_tsf(
    input: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out.is_null() || length <= 0 || period <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match indicators::tsf(input_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_linear_reg(
    input: *const c_double,
    length: c_int,
    period: c_int,
    out: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if input.is_null() || out.is_null() || length <= 0 || period <= 0 {
        return -1;
    }
    let input_slice = unsafe { std::slice::from_raw_parts(input, length as usize) };
    match indicators::linearreg(input_slice, period as usize) {
        Ok(result) => {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(out, length as usize) };
            for i in 0..length as usize {
                out_slice[i] = result[i];
            }
            0
        }
        Err(_) => -2,
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_darvas_box(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    lookback: c_int,
    confirmation: c_int,
    out_top: *mut c_double,
    out_bottom: *mut c_double,
    out_signal: *mut c_int,
) -> c_int {
    ffi_catch_i32(|| {
if high.is_null() || low.is_null() || close.is_null() || length <= 0 {
        return -1;
    }
    unsafe {
        let h = std::slice::from_raw_parts(high, length as usize);
        let l = std::slice::from_raw_parts(low, length as usize);
        let c = std::slice::from_raw_parts(close, length as usize);
        let lb = if lookback > 0 { lookback as usize } else { 5 };
        let conf = if confirmation > 0 { confirmation as usize } else { 3 };
        match indicators::darvas_box(h, l, c, lb, conf) {
            Ok(r) => {
                if !out_top.is_null() {
                    std::ptr::copy_nonoverlapping(r.box_top.as_ptr(), out_top, length as usize);
                }
                if !out_bottom.is_null() {
                    std::ptr::copy_nonoverlapping(
                        r.box_bottom.as_ptr(),
                        out_bottom,
                        length as usize,
                    );
                }
                if !out_signal.is_null() {
                    std::ptr::copy_nonoverlapping(
                        r.signal.as_ptr(),
                        out_signal,
                        length as usize,
                    );
                }
                0
            }
            Err(_) => -2,
        }
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_renko(
    high: *const c_double,
    low: *const c_double,
    length: c_int,
    box_size: c_double,
    out_bricks: *mut c_double,
    out_dir: *mut c_int,
) -> c_int {
    ffi_catch_i32(|| {
if high.is_null() || low.is_null() || out_bricks.is_null() || length <= 0 || box_size <= 0.0 {
        return -1;
    }
    unsafe {
        let h = std::slice::from_raw_parts(high, length as usize);
        let l = std::slice::from_raw_parts(low, length as usize);
        match indicators::renko(h, l, box_size) {
            Ok(r) => {
                std::ptr::copy_nonoverlapping(r.bricks.as_ptr(), out_bricks, length as usize);
                if !out_dir.is_null() {
                    std::ptr::copy_nonoverlapping(r.direction.as_ptr(), out_dir, length as usize);
                }
                0
            }
            Err(_) => -2,
        }
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_kagi(
    close: *const c_double,
    length: c_int,
    reversal: c_double,
    out_kagi: *mut c_double,
    out_dir: *mut c_int,
) -> c_int {
    ffi_catch_i32(|| {
if close.is_null() || out_kagi.is_null() || length <= 0 || reversal <= 0.0 {
        return -1;
    }
    unsafe {
        let c = std::slice::from_raw_parts(close, length as usize);
        match indicators::kagi(c, reversal) {
            Ok(r) => {
                std::ptr::copy_nonoverlapping(r.kagi.as_ptr(), out_kagi, length as usize);
                if !out_dir.is_null() {
                    std::ptr::copy_nonoverlapping(r.direction.as_ptr(), out_dir, length as usize);
                }
                0
            }
            Err(_) => -2,
        }
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_point_and_figure(
    high: *const c_double,
    low: *const c_double,
    length: c_int,
    box_size: c_double,
    reversal: c_int,
    out_pnf: *mut c_double,
    out_col: *mut c_int,
    out_new: *mut c_int,
) -> c_int {
    ffi_catch_i32(|| {
if high.is_null() || low.is_null() || out_pnf.is_null() || length <= 0 || box_size <= 0.0 {
        return -1;
    }
    unsafe {
        let h = std::slice::from_raw_parts(high, length as usize);
        let l = std::slice::from_raw_parts(low, length as usize);
        let rev = if reversal > 0 { reversal as usize } else { 3 };
        match indicators::point_and_figure(h, l, box_size, rev) {
            Ok(r) => {
                std::ptr::copy_nonoverlapping(r.pnf.as_ptr(), out_pnf, length as usize);
                if !out_col.is_null() {
                    std::ptr::copy_nonoverlapping(r.column_type.as_ptr(), out_col, length as usize);
                }
                if !out_new.is_null() {
                    std::ptr::copy_nonoverlapping(r.new_column.as_ptr(), out_new, length as usize);
                }
                0
            }
            Err(_) => -2,
        }
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_three_line_break(
    close: *const c_double,
    length: c_int,
    lines: c_int,
    out_line: *mut c_double,
    out_dir: *mut c_int,
) -> c_int {
    ffi_catch_i32(|| {
if close.is_null() || out_line.is_null() || length <= 0 || lines <= 0 {
        return -1;
    }
    unsafe {
        let c = std::slice::from_raw_parts(close, length as usize);
        match indicators::three_line_break(c, lines as usize) {
            Ok(r) => {
                std::ptr::copy_nonoverlapping(r.line.as_ptr(), out_line, length as usize);
                if !out_dir.is_null() {
                    std::ptr::copy_nonoverlapping(r.direction.as_ptr(), out_dir, length as usize);
                }
                0
            }
            Err(_) => -2,
        }
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_williams_alligator(
    close: *const c_double,
    length: c_int,
    out_jaw: *mut c_double,
    out_teeth: *mut c_double,
    out_lips: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if close.is_null() || out_jaw.is_null() || out_teeth.is_null() || out_lips.is_null() || length <= 0 {
        return -1;
    }
    unsafe {
        let c = std::slice::from_raw_parts(close, length as usize);
        match indicators::williams_alligator(c) {
            Ok(r) => {
                std::ptr::copy_nonoverlapping(r.jaw.as_ptr(), out_jaw, length as usize);
                std::ptr::copy_nonoverlapping(r.teeth.as_ptr(), out_teeth, length as usize);
                std::ptr::copy_nonoverlapping(r.lips.as_ptr(), out_lips, length as usize);
                0
            }
            Err(_) => -2,
        }
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_heikin_ashi(
    open: *const c_double,
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    out_o: *mut c_double,
    out_h: *mut c_double,
    out_l: *mut c_double,
    out_c: *mut c_double,
) -> c_int {
    ffi_catch_i32(|| {
if open.is_null() || high.is_null() || low.is_null() || close.is_null() || length <= 0 {
        return -1;
    }
    unsafe {
        let o = std::slice::from_raw_parts(open, length as usize);
        let h = std::slice::from_raw_parts(high, length as usize);
        let l = std::slice::from_raw_parts(low, length as usize);
        let c = std::slice::from_raw_parts(close, length as usize);
        match indicators::heikin_ashi(o, h, l, c) {
            Ok(r) => {
                if !out_o.is_null() {
                    std::ptr::copy_nonoverlapping(r.ha_open.as_ptr(), out_o, length as usize);
                }
                if !out_h.is_null() {
                    std::ptr::copy_nonoverlapping(r.ha_high.as_ptr(), out_h, length as usize);
                }
                if !out_l.is_null() {
                    std::ptr::copy_nonoverlapping(r.ha_low.as_ptr(), out_l, length as usize);
                }
                if !out_c.is_null() {
                    std::ptr::copy_nonoverlapping(r.ha_close.as_ptr(), out_c, length as usize);
                }
                0
            }
            Err(_) => -2,
        }
    }
    })
}
