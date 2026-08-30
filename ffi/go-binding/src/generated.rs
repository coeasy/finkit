// ─────────────────────────────────────────────────────────────────────
// GENERATED FILE — do not edit by hand.
// Source of truth: docs/indicator_registry.json (ffi.bodies.<lang>).
// Regenerate with: python3 scripts/sync_bindings.py --lang go --generate --rewrite
// ─────────────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn ta_sma(input: *const c_double, length: c_int, period: c_int) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    if period <= 0 {
        return make_error_result("period must be positive");
    }
    match sma(slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_ema(input: *const c_double, length: c_int, period: c_int) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    if period <= 0 {
        return make_error_result("period must be positive");
    }
    match ema(slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_wma(input: *const c_double, length: c_int, period: c_int) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    if period <= 0 {
        return make_error_result("period must be positive");
    }
    match wma(slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_dema(input: *const c_double, length: c_int, period: c_int) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    if period <= 0 {
        return make_error_result("period must be positive");
    }
    match dema(slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_tema(input: *const c_double, length: c_int, period: c_int) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    if period <= 0 {
        return make_error_result("period must be positive");
    }
    match tema(slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_kama(input: *const c_double, length: c_int, period: c_int) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    if period <= 0 {
        return make_error_result("period must be positive");
    }
    match kama(slice, period as usize, 2, 30) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_t3(
    input: *const c_double,
    length: c_int,
    period: c_int,
    vfactor: c_double,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    if period <= 0 {
        return make_error_result("period must be positive");
    }
    match t3(slice, period as usize, vfactor) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
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
) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    match bbands(slice, period as usize, nb_dev_up, nb_dev_dn) {
        Ok(result) => {
            let upper_vec = result.upper.into_raw_vec_and_offset().0;
            let middle_vec = result.middle.into_raw_vec_and_offset().0;
            let lower_vec = result.lower.into_raw_vec_and_offset().0;

            let out_len = (length * 3) as usize;
            let mut out = Vec::with_capacity(out_len);
            out.extend_from_slice(&upper_vec);
            out.extend_from_slice(&middle_vec);
            out.extend_from_slice(&lower_vec);

            make_result_from_vec(out)
        }
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_rsi(input: *const c_double, length: c_int, period: c_int) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    if period <= 0 {
        return make_error_result("period must be positive");
    }
    match rsi(slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
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
) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    match macd(
        slice,
        fast_period as usize,
        slow_period as usize,
        signal_period as usize,
    ) {
        Ok(result) => {
            let macd_vec = result.macd.into_raw_vec_and_offset().0;
            let signal_vec = result.signal.into_raw_vec_and_offset().0;
            let hist_vec = result.hist.into_raw_vec_and_offset().0;

            let out_len = (length * 3) as usize;
            let mut out = Vec::with_capacity(out_len);
            out.extend_from_slice(&macd_vec);
            out.extend_from_slice(&signal_vec);
            out.extend_from_slice(&hist_vec);

            make_result_from_vec(out)
        }
        Err(e) => make_error_result(&format!("{}", e)),
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
) -> *mut TaResult {
    ffi_catch_ptr(|| {
if high.is_null() || low.is_null() || close.is_null() || length <= 0 {
        return make_error_result("invalid input");
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };

    match stoch(
        high_slice,
        low_slice,
        close_slice,
        k_period as usize,
        k_slow as usize,
        d_period as usize,
    ) {
        Ok(result) => {
            let k_vec = result.k.into_raw_vec_and_offset().0;
            let d_vec = result.d.into_raw_vec_and_offset().0;

            let out_len = (length * 2) as usize;
            let mut out = Vec::with_capacity(out_len);
            out.extend_from_slice(&k_vec);
            out.extend_from_slice(&d_vec);

            make_result_from_vec(out)
        }
        Err(e) => make_error_result(&format!("{}", e)),
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
) -> *mut TaResult {
    ffi_catch_ptr(|| {
if high.is_null() || low.is_null() || close.is_null() || length <= 0 {
        return make_error_result("invalid input");
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };

    match adx(high_slice, low_slice, close_slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_aroon(
    high: *const c_double,
    low: *const c_double,
    length: c_int,
    period: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
if high.is_null() || low.is_null() || length <= 0 {
        return make_error_result("invalid input");
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };

    match aroon(high_slice, low_slice, period as usize) {
        Ok(result) => {
            let up_vec = result.aroon_up.into_raw_vec_and_offset().0;
            let down_vec = result.aroon_down.into_raw_vec_and_offset().0;

            let out_len = (length * 2) as usize;
            let mut out = Vec::with_capacity(out_len);
            out.extend_from_slice(&up_vec);
            out.extend_from_slice(&down_vec);

            make_result_from_vec(out)
        }
        Err(e) => make_error_result(&format!("{}", e)),
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
) -> *mut TaResult {
    ffi_catch_ptr(|| {
if high.is_null() || low.is_null() || close.is_null() || length <= 0 {
        return make_error_result("invalid input");
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };

    match cci(high_slice, low_slice, close_slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_mom(input: *const c_double, length: c_int, period: c_int) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    match mom(slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_roc(input: *const c_double, length: c_int, period: c_int) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    match roc(slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
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
) -> *mut TaResult {
    ffi_catch_ptr(|| {
if high.is_null() || low.is_null() || close.is_null() || length <= 0 {
        return make_error_result("invalid input");
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };

    match willr(high_slice, low_slice, close_slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
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
) -> *mut TaResult {
    ffi_catch_ptr(|| {
if high.is_null() || low.is_null() || close.is_null() || length <= 0 {
        return make_error_result("invalid input");
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };

    match atr(high_slice, low_slice, close_slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
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
) -> *mut TaResult {
    ffi_catch_ptr(|| {
if high.is_null() || low.is_null() || close.is_null() || length <= 0 {
        return make_error_result("invalid input");
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };

    match natr(high_slice, low_slice, close_slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_trange(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
if high.is_null() || low.is_null() || close.is_null() || length <= 0 {
        return make_error_result("invalid input");
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };

    match trange(high_slice, low_slice, close_slice) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_obv(
    close: *const c_double,
    volume: *const c_double,
    length: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
if close.is_null() || volume.is_null() || length <= 0 {
        return make_error_result("invalid input");
    }
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    let volume_slice = unsafe { std::slice::from_raw_parts(volume, length as usize) };

    match obv(close_slice, volume_slice) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
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
) -> *mut TaResult {
    ffi_catch_ptr(|| {
if high.is_null() || low.is_null() || close.is_null() || volume.is_null() || length <= 0 {
        return make_error_result("invalid input");
    }
    let high_slice = unsafe { std::slice::from_raw_parts(high, length as usize) };
    let low_slice = unsafe { std::slice::from_raw_parts(low, length as usize) };
    let close_slice = unsafe { std::slice::from_raw_parts(close, length as usize) };
    let volume_slice = unsafe { std::slice::from_raw_parts(volume, length as usize) };

    match ad(high_slice, low_slice, close_slice, volume_slice) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_ht_dcperiod(input: *const c_double, length: c_int) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    match ht_dcperiod(slice) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_ht_dcphase(input: *const c_double, length: c_int) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    match ht_dcphase(slice) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_ht_phasor(input: *const c_double, length: c_int) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    match ht_phasor(slice) {
        Ok((in_phase, quadrature)) => {
            let ip_vec = in_phase.into_raw_vec_and_offset().0;
            let quad_vec = quadrature.into_raw_vec_and_offset().0;

            let out_len = (length * 2) as usize;
            let mut out = Vec::with_capacity(out_len);
            out.extend_from_slice(&ip_vec);
            out.extend_from_slice(&quad_vec);

            make_result_from_vec(out)
        }
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_ht_sine(input: *const c_double, length: c_int) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    match ht_sine(slice) {
        Ok((sine, lead_sine)) => {
            let sine_vec = sine.into_raw_vec_and_offset().0;
            let lead_vec = lead_sine.into_raw_vec_and_offset().0;

            let out_len = (length * 2) as usize;
            let mut out = Vec::with_capacity(out_len);
            out.extend_from_slice(&sine_vec);
            out.extend_from_slice(&lead_vec);

            make_result_from_vec(out)
        }
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_ht_trendmode(input: *const c_double, length: c_int) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    match ht_trendmode(slice) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_ht_trendline(input: *const c_double, length: c_int) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    match ht_trendline(slice) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_zscore(input: *const c_double, length: c_int, period: c_int) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    match zscore(slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_beta(
    asset: *const c_double,
    benchmark: *const c_double,
    length: c_int,
    period: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
if asset.is_null() || benchmark.is_null() || length <= 0 {
        return make_error_result("invalid input");
    }
    let asset_slice = unsafe { std::slice::from_raw_parts(asset, length as usize) };
    let benchmark_slice = unsafe { std::slice::from_raw_parts(benchmark, length as usize) };

    match beta(asset_slice, benchmark_slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_correlation(
    input_a: *const c_double,
    input_b: *const c_double,
    length: c_int,
    period: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
if input_a.is_null() || input_b.is_null() || length <= 0 {
        return make_error_result("invalid input");
    }
    let a_slice = unsafe { std::slice::from_raw_parts(input_a, length as usize) };
    let b_slice = unsafe { std::slice::from_raw_parts(input_b, length as usize) };

    match correlation(a_slice, b_slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_tsf(input: *const c_double, length: c_int, period: c_int) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    match tsf(slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_linear_reg(
    input: *const c_double,
    length: c_int,
    period: c_int,
) -> *mut TaResult {
    ffi_catch_ptr(|| {
let slice = match validate_input(input, length) {
        Some(s) => s,
        None => return make_error_result("invalid input"),
    };
    match linear_reg(slice, period as usize) {
        Ok(result) => make_result(result),
        Err(e) => make_error_result(&format!("{}", e)),
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_darvas_box_json(
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
    lookback: c_int,
    confirmation: c_int,
) -> *mut c_char {
    ffi_catch_ptr(|| {
if high.is_null() || low.is_null() || close.is_null() || length <= 0 {
        return err_cstr("invalid input");
    }
    unsafe {
        let h = std::slice::from_raw_parts(high, length as usize);
        let l = std::slice::from_raw_parts(low, length as usize);
        let c = std::slice::from_raw_parts(close, length as usize);
        let lb = if lookback > 0 { lookback as usize } else { 5 };
        let conf = if confirmation > 0 { confirmation as usize } else { 3 };
        match finkit::indicators::darvas_box(h, l, c, lb, conf) {
            Ok(r) => {
                #[derive(serde::Serialize)]
                struct Out {
                    box_top: Vec<Option<f64>>,
                    box_bottom: Vec<Option<f64>>,
                    signal: Vec<i32>,
                }
                let to_opt = |v: &Array1<f64>| v.iter().map(|x| if x.is_finite() { Some(*x) } else { None }).collect();
                let out = Out {
                    box_top: to_opt(&r.box_top),
                    box_bottom: to_opt(&r.box_bottom),
                    signal: r.signal.to_vec(),
                };
                let s = serde_json::to_string(&out).unwrap_or_else(|_| "{}".to_string());
                CString::new(s).map(|c| c.into_raw()).unwrap_or_else(|_| err_cstr("json error"))
            }
            Err(e) => err_cstr(e),
        }
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_renko_json(
    high: *const c_double,
    low: *const c_double,
    length: c_int,
    box_size: c_double,
) -> *mut c_char {
    ffi_catch_ptr(|| {
if high.is_null() || low.is_null() || length <= 0 || box_size <= 0.0 {
        return err_cstr("invalid input");
    }
    unsafe {
        let h = std::slice::from_raw_parts(high, length as usize);
        let l = std::slice::from_raw_parts(low, length as usize);
        match finkit::indicators::renko(h, l, box_size) {
            Ok(r) => {
                #[derive(serde::Serialize)]
                struct Out {
                    bricks: Vec<Option<f64>>,
                    direction: Vec<i32>,
                }
                let to_opt = |v: &Array1<f64>| v.iter().map(|x| if x.is_finite() { Some(*x) } else { None }).collect();
                let out = Out {
                    bricks: to_opt(&r.bricks),
                    direction: r.direction.to_vec(),
                };
                let s = serde_json::to_string(&out).unwrap_or_else(|_| "{}".to_string());
                CString::new(s).map(|c| c.into_raw()).unwrap_or_else(|_| err_cstr("json error"))
            }
            Err(e) => err_cstr(e),
        }
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_kagi_json(
    close: *const c_double,
    length: c_int,
    reversal: c_double,
) -> *mut c_char {
    ffi_catch_ptr(|| {
if close.is_null() || length <= 0 || reversal <= 0.0 {
        return err_cstr("invalid input");
    }
    unsafe {
        let c = std::slice::from_raw_parts(close, length as usize);
        match finkit::indicators::kagi(c, reversal) {
            Ok(r) => {
                #[derive(serde::Serialize)]
                struct Out {
                    kagi: Vec<Option<f64>>,
                    direction: Vec<i32>,
                }
                let to_opt = |v: &Array1<f64>| v.iter().map(|x| if x.is_finite() { Some(*x) } else { None }).collect();
                let out = Out {
                    kagi: to_opt(&r.kagi),
                    direction: r.direction.to_vec(),
                };
                let s = serde_json::to_string(&out).unwrap_or_else(|_| "{}".to_string());
                CString::new(s).map(|c| c.into_raw()).unwrap_or_else(|_| err_cstr("json error"))
            }
            Err(e) => err_cstr(e),
        }
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_point_and_figure_json(
    high: *const c_double,
    low: *const c_double,
    length: c_int,
    box_size: c_double,
    reversal: c_int,
) -> *mut c_char {
    ffi_catch_ptr(|| {
if high.is_null() || low.is_null() || length <= 0 || box_size <= 0.0 {
        return err_cstr("invalid input");
    }
    unsafe {
        let h = std::slice::from_raw_parts(high, length as usize);
        let l = std::slice::from_raw_parts(low, length as usize);
        let rev = if reversal > 0 { reversal as usize } else { 3 };
        match finkit::indicators::point_and_figure(h, l, box_size, rev) {
            Ok(r) => {
                #[derive(serde::Serialize)]
                struct Out {
                    pnf: Vec<Option<f64>>,
                    column_type: Vec<i32>,
                    new_column: Vec<i32>,
                }
                let to_opt = |v: &Array1<f64>| v.iter().map(|x| if x.is_finite() { Some(*x) } else { None }).collect();
                let out = Out {
                    pnf: to_opt(&r.pnf),
                    column_type: r.column_type.to_vec(),
                    new_column: r.new_column.to_vec(),
                };
                let s = serde_json::to_string(&out).unwrap_or_else(|_| "{}".to_string());
                CString::new(s).map(|c| c.into_raw()).unwrap_or_else(|_| err_cstr("json error"))
            }
            Err(e) => err_cstr(e),
        }
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_three_line_break_json(
    close: *const c_double,
    length: c_int,
    lines: c_int,
) -> *mut c_char {
    ffi_catch_ptr(|| {
if close.is_null() || length <= 0 || lines <= 0 {
        return err_cstr("invalid input");
    }
    unsafe {
        let c = std::slice::from_raw_parts(close, length as usize);
        match finkit::indicators::three_line_break(c, lines as usize) {
            Ok(r) => {
                #[derive(serde::Serialize)]
                struct Out {
                    line: Vec<Option<f64>>,
                    direction: Vec<i32>,
                }
                let to_opt = |v: &Array1<f64>| v.iter().map(|x| if x.is_finite() { Some(*x) } else { None }).collect();
                let out = Out {
                    line: to_opt(&r.line),
                    direction: r.direction.to_vec(),
                };
                let s = serde_json::to_string(&out).unwrap_or_else(|_| "{}".to_string());
                CString::new(s).map(|c| c.into_raw()).unwrap_or_else(|_| err_cstr("json error"))
            }
            Err(e) => err_cstr(e),
        }
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_williams_alligator_json(
    close: *const c_double,
    length: c_int,
) -> *mut c_char {
    ffi_catch_ptr(|| {
if close.is_null() || length <= 0 {
        return err_cstr("invalid input");
    }
    unsafe {
        let c = std::slice::from_raw_parts(close, length as usize);
        match finkit::indicators::williams_alligator(c) {
            Ok(r) => {
                #[derive(serde::Serialize)]
                struct Out {
                    jaw: Vec<Option<f64>>,
                    teeth: Vec<Option<f64>>,
                    lips: Vec<Option<f64>>,
                }
                let to_opt = |v: &Array1<f64>| v.iter().map(|x| if x.is_finite() { Some(*x) } else { None }).collect();
                let out = Out {
                    jaw: to_opt(&r.jaw),
                    teeth: to_opt(&r.teeth),
                    lips: to_opt(&r.lips),
                };
                let s = serde_json::to_string(&out).unwrap_or_else(|_| "{}".to_string());
                CString::new(s).map(|c| c.into_raw()).unwrap_or_else(|_| err_cstr("json error"))
            }
            Err(e) => err_cstr(e),
        }
    }
    })
}

#[no_mangle]
pub extern "C" fn ta_heikin_ashi_json(
    open: *const c_double,
    high: *const c_double,
    low: *const c_double,
    close: *const c_double,
    length: c_int,
) -> *mut c_char {
    ffi_catch_ptr(|| {
if open.is_null() || high.is_null() || low.is_null() || close.is_null() || length <= 0 {
        return err_cstr("invalid input");
    }
    unsafe {
        let o = std::slice::from_raw_parts(open, length as usize);
        let h = std::slice::from_raw_parts(high, length as usize);
        let l = std::slice::from_raw_parts(low, length as usize);
        let c = std::slice::from_raw_parts(close, length as usize);
        match finkit::indicators::heikin_ashi(o, h, l, c) {
            Ok(r) => {
                #[derive(serde::Serialize)]
                struct Out {
                    ha_open: Vec<Option<f64>>,
                    ha_high: Vec<Option<f64>>,
                    ha_low: Vec<Option<f64>>,
                    ha_close: Vec<Option<f64>>,
                }
                let to_opt = |v: &Array1<f64>| v.iter().map(|x| if x.is_finite() { Some(*x) } else { None }).collect();
                let out = Out {
                    ha_open: to_opt(&r.ha_open),
                    ha_high: to_opt(&r.ha_high),
                    ha_low: to_opt(&r.ha_low),
                    ha_close: to_opt(&r.ha_close),
                };
                let s = serde_json::to_string(&out).unwrap_or_else(|_| "{}".to_string());
                CString::new(s).map(|c| c.into_raw()).unwrap_or_else(|_| err_cstr("json error"))
            }
            Err(e) => err_cstr(e),
        }
    }
    })
}
