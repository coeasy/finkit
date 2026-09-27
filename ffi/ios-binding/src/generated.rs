// ─────────────────────────────────────────────────────────────────────
// GENERATED FILE — do not edit by hand.
// This file has no in-tree generator: the `ffi.bodies` block it used to come
// from is not present in docs/indicator_registry.json or docs/ffi_registry.json,
// and `scripts/sync_bindings.py` reports the `ios` language as DEFERRED, so it
// stores no bodies to regenerate from. Edit it in place, and keep the error
// contract below in sync with `ffi/c-binding/src/generated.rs`: a core failure
// must return -1, never a success code with the caller's buffer left untouched.
// ─────────────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn alpha_ta_sma(
    input: *const f64,
    len: i32,
    period: i32,
    out: *mut f64,
) -> i32 {
    ffi_catch_i32_neg(|| {
if input.is_null() || out.is_null() || period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = match moving_avg::sma(data, period as usize) {
        Ok(result) => result,
        Err(_) => return -1,
    };
    if !write_result(out, result.as_slice().unwrap()) {
        return -1;
    }
    0
    })
}

#[no_mangle]
pub extern "C" fn alpha_ta_ema(
    input: *const f64,
    len: i32,
    period: i32,
    out: *mut f64,
) -> i32 {
    ffi_catch_i32_neg(|| {
if input.is_null() || out.is_null() || period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = match moving_avg::ema(data, period as usize) {
        Ok(result) => result,
        Err(_) => return -1,
    };
    if !write_result(out, result.as_slice().unwrap()) {
        return -1;
    }
    0
    })
}

#[no_mangle]
pub extern "C" fn alpha_ta_wma(
    input: *const f64,
    len: i32,
    period: i32,
    out: *mut f64,
) -> i32 {
    ffi_catch_i32_neg(|| {
if input.is_null() || out.is_null() || period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = match moving_avg::wma(data, period as usize) {
        Ok(result) => result,
        Err(_) => return -1,
    };
    if !write_result(out, result.as_slice().unwrap()) {
        return -1;
    }
    0
    })
}

#[no_mangle]
pub extern "C" fn alpha_ta_dema(
    input: *const f64,
    len: i32,
    period: i32,
    out: *mut f64,
) -> i32 {
    ffi_catch_i32_neg(|| {
if input.is_null() || out.is_null() || period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = match moving_avg::dema(data, period as usize) {
        Ok(result) => result,
        Err(_) => return -1,
    };
    if !write_result(out, result.as_slice().unwrap()) {
        return -1;
    }
    0
    })
}

#[no_mangle]
pub extern "C" fn alpha_ta_tema(
    input: *const f64,
    len: i32,
    period: i32,
    out: *mut f64,
) -> i32 {
    ffi_catch_i32_neg(|| {
if input.is_null() || out.is_null() || period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = match moving_avg::tema(data, period as usize) {
        Ok(result) => result,
        Err(_) => return -1,
    };
    if !write_result(out, result.as_slice().unwrap()) {
        return -1;
    }
    0
    })
}

#[no_mangle]
pub extern "C" fn alpha_ta_midpoint(
    input: *const f64,
    len: i32,
    period: i32,
    out: *mut f64,
) -> i32 {
    ffi_catch_i32_neg(|| {
if input.is_null() || out.is_null() || period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = match indicators::midpoint(data, period as usize) {
        Ok(result) => result,
        Err(_) => return -1,
    };
    if !write_result(out, result.as_slice().unwrap()) {
        return -1;
    }
    0
    })
}

#[no_mangle]
pub extern "C" fn alpha_ta_rsi(
    input: *const f64,
    len: i32,
    period: i32,
    out: *mut f64,
) -> i32 {
    ffi_catch_i32_neg(|| {
if input.is_null() || out.is_null() || period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = match indicators::rsi(data, period as usize) {
        Ok(result) => result,
        Err(_) => return -1,
    };
    if !write_result(out, result.as_slice().unwrap()) {
        return -1;
    }
    0
    })
}

#[no_mangle]
pub extern "C" fn alpha_ta_mom(
    input: *const f64,
    len: i32,
    period: i32,
    out: *mut f64,
) -> i32 {
    ffi_catch_i32_neg(|| {
if input.is_null() || out.is_null() || period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = match indicators::mom(data, period as usize) {
        Ok(result) => result,
        Err(_) => return -1,
    };
    if !write_result(out, result.as_slice().unwrap()) {
        return -1;
    }
    0
    })
}

#[no_mangle]
pub extern "C" fn alpha_ta_roc(
    input: *const f64,
    len: i32,
    period: i32,
    out: *mut f64,
) -> i32 {
    ffi_catch_i32_neg(|| {
if input.is_null() || out.is_null() || period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = match indicators::roc(data, period as usize) {
        Ok(result) => result,
        Err(_) => return -1,
    };
    if !write_result(out, result.as_slice().unwrap()) {
        return -1;
    }
    0
    })
}

#[no_mangle]
pub extern "C" fn alpha_ta_cmo(
    input: *const f64,
    len: i32,
    period: i32,
    out: *mut f64,
) -> i32 {
    ffi_catch_i32_neg(|| {
if input.is_null() || out.is_null() || period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = match indicators::cmo(data, period as usize) {
        Ok(result) => result,
        Err(_) => return -1,
    };
    if !write_result(out, result.as_slice().unwrap()) {
        return -1;
    }
    0
    })
}

#[no_mangle]
pub extern "C" fn alpha_ta_trix(
    input: *const f64,
    len: i32,
    period: i32,
    out: *mut f64,
) -> i32 {
    ffi_catch_i32_neg(|| {
if input.is_null() || out.is_null() || period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = match indicators::trix(data, period as usize) {
        Ok(result) => result,
        Err(_) => return -1,
    };
    if !write_result(out, result.as_slice().unwrap()) {
        return -1;
    }
    0
    })
}

#[no_mangle]
pub extern "C" fn alpha_ta_zscore(
    input: *const f64,
    len: i32,
    period: i32,
    out: *mut f64,
) -> i32 {
    ffi_catch_i32_neg(|| {
if input.is_null() || out.is_null() || period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = match indicators::zscore(data, period as usize) {
        Ok(result) => result,
        Err(_) => return -1,
    };
    if !write_result(out, result.as_slice().unwrap()) {
        return -1;
    }
    0
    })
}

#[no_mangle]
pub extern "C" fn alpha_ta_tsf(
    input: *const f64,
    len: i32,
    period: i32,
    out: *mut f64,
) -> i32 {
    ffi_catch_i32_neg(|| {
if input.is_null() || out.is_null() || period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = match indicators::tsf(data, period as usize) {
        Ok(result) => result,
        Err(_) => return -1,
    };
    if !write_result(out, result.as_slice().unwrap()) {
        return -1;
    }
    0
    })
}

#[no_mangle]
pub extern "C" fn alpha_ta_linear_reg(
    input: *const f64,
    len: i32,
    period: i32,
    out: *mut f64,
) -> i32 {
    ffi_catch_i32_neg(|| {
if input.is_null() || out.is_null() || period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = match indicators::linearreg(data, period as usize) {
        Ok(result) => result,
        Err(_) => return -1,
    };
    if !write_result(out, result.as_slice().unwrap()) {
        return -1;
    }
    0
    })
}

#[no_mangle]
pub extern "C" fn alpha_ta_percent_rank(
    input: *const f64,
    len: i32,
    period: i32,
    out: *mut f64,
) -> i32 {
    ffi_catch_i32_neg(|| {
if input.is_null() || out.is_null() || period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = match indicators::percent_rank(data, period as usize) {
        Ok(result) => result,
        Err(_) => return -1,
    };
    if !write_result(out, result.as_slice().unwrap()) {
        return -1;
    }
    0
    })
}
