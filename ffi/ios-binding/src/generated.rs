// ─────────────────────────────────────────────────────────────────────
// GENERATED FILE — do not edit by hand.
// Source of truth: docs/indicator_registry.json (ffi.bodies.<lang>).
// Regenerate with: python3 scripts/sync_bindings.py --lang ios --generate --rewrite
// ─────────────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn alpha_ta_sma(
    input: *const f64,
    len: i32,
    period: i32,
    out: *mut f64,
) -> i32 {
    ffi_catch_i32_neg(|| {
if period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = moving_avg::sma(data, period as usize).unwrap_or_default();
    write_result(out, result.as_slice().unwrap());
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
if period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = moving_avg::ema(data, period as usize).unwrap_or_default();
    write_result(out, result.as_slice().unwrap());
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
if period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = moving_avg::wma(data, period as usize).unwrap_or_default();
    write_result(out, result.as_slice().unwrap());
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
if period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = moving_avg::dema(data, period as usize).unwrap_or_default();
    write_result(out, result.as_slice().unwrap());
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
if period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = moving_avg::tema(data, period as usize).unwrap_or_default();
    write_result(out, result.as_slice().unwrap());
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
if period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = indicators::midpoint(data, period as usize).unwrap_or_default();
    write_result(out, result.as_slice().unwrap());
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
if period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = indicators::rsi(data, period as usize).unwrap_or_default();
    write_result(out, result.as_slice().unwrap());
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
if period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = indicators::mom(data, period as usize).unwrap_or_default();
    write_result(out, result.as_slice().unwrap());
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
if period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = indicators::roc(data, period as usize).unwrap_or_default();
    write_result(out, result.as_slice().unwrap());
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
if period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = indicators::cmo(data, period as usize).unwrap_or_default();
    write_result(out, result.as_slice().unwrap());
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
if period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = indicators::trix(data, period as usize).unwrap_or_default();
    write_result(out, result.as_slice().unwrap());
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
if period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = indicators::zscore(data, period as usize).unwrap_or_default();
    write_result(out, result.as_slice().unwrap());
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
if period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = indicators::tsf(data, period as usize).unwrap_or_default();
    write_result(out, result.as_slice().unwrap());
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
if period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = indicators::linearreg(data, period as usize).unwrap_or_default();
    write_result(out, result.as_slice().unwrap());
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
if period <= 0 || len < period {
        return -1;
    }
    let data = from_raw(input, len);
    let result = indicators::percent_rank(data, period as usize).unwrap_or_default();
    write_result(out, result.as_slice().unwrap());
    0
    })
}
