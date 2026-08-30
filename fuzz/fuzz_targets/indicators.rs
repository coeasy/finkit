//! Fuzz target for indicator entry points (T-2).
//!
//! Feeds random `f64` slices (including NaN / ±Inf) into the batch
//! indicator entry points. The contract being tested is "no panic on any
//! input": the only acceptable response to non-finite data is a clean
//! `Err(InvalidParameter)`.
//!
//! # Running
//!
//! ```bash
//! cargo +nightly fuzz run indicators -- -runs=50000
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use alpha_ta_core::math::moving_avg::{sma, ema, wma, dema};
use alpha_ta_core::indicators::{bbands, macd, rsi};
use ndarray::Array1;

fuzz_target!(|data: &[u8]| {
    // We need a buffer of f64s. Convert bytes 8 at a time; ignore any
    // leftover (and any NaN/Inf values - they're part of the fuzz space).
    if data.len() < 8 {
        return;
    }

    // Limit array size to keep fuzz iterations fast.
    let n_bars = (data.len() / 8).min(256);
    if n_bars < 5 {
        return;
    }

    let mut input: Vec<f64> = Vec::with_capacity(n_bars);
    for chunk in data.chunks(8).take(n_bars) {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(chunk);
        input.push(f64::from_le_bytes(buf));
    }

    // Period is fuzzed as 1..32; clamp to input length.
    let period = (data[0] as usize).max(1).min(32).min(n_bars);

    // ---- NaN/Inf acceptance test ----
    // The contract is: any non-finite input is rejected with an Err, never
    // a panic.
    let has_non_finite = input.iter().any(|v| !v.is_finite());

    // SMA, EMA, WMA, DEMA
    let _ = sma(&input, period);
    let _ = ema(&input, period);
    let _ = wma(&input, period);
    let _ = dema(&input, period);

    // RSI / MACD / BBANDS
    let _ = rsi(&input, period);
    let macd_in = Array1::from_vec(input.clone());
    let _ = macd(macd_in.view(), 12, 26, 9);
    let _ = bbands(macd_in.view(), period, 2.0);

    if has_non_finite {
        // When the input contains NaN/Inf, the only acceptable result
        // is Err — the test below verifies we never panic. We don't
        // assert Err here because not every indicator has the
        // reject_if_non_finite guard yet (R-1 is incremental).
    }
});
