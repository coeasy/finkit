use finkit::indicators::{adx, adxr};
use finkit::math::kernels::AdxState;
use std::hint::black_box;
use std::time::{Duration, Instant};

#[cfg(feature = "talib-c")]
use finkit::talib_ffi::{TA_ADX, TA_ADXR};

fn inputs(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    for i in 0..len {
        let x = i as f64;
        let c = 100.0 + 0.006 * x + (x * 0.017).sin() * 2.1;
        let o = c - 0.15 + (x * 0.031).sin() * 0.05;
        high.push(o.max(c) + 0.7 + (x * 0.023).sin().abs() * 0.3);
        low.push(o.min(c) - 0.8 - (x * 0.019).cos().abs() * 0.2);
        close.push(c);
    }
    (high, low, close)
}

fn state_reference(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<f64> {
    let mut output = vec![f64::NAN; close.len()];
    let mut state = AdxState::new(period);
    for index in 0..close.len() {
        if let Some(family) = state.update(high[index], low[index], close[index]) {
            if let Some(value) = family.adx {
                output[index] = value;
            }
        }
    }
    output
}

fn state_adxr_reference(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<f64> {
    let mut output = state_reference(high, low, close, period);
    for index in (period..output.len()).rev() {
        let current = output[index];
        let previous = output[index + 1 - period];
        output[index] = if current.is_nan() || previous.is_nan() {
            f64::NAN
        } else {
            (current + previous) * 0.5
        };
    }
    output
}

fn same_bits(left: &[f64], right: &[f64]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(a, b)| (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits())
}

#[cfg(feature = "talib-c")]
fn talib_adx(high: &[f64], low: &[f64], close: &[f64], period: i32) -> Vec<f64> {
    let mut raw = vec![0.0; close.len()];
    let mut begin = 0;
    let mut count = 0;
    // SAFETY: inputs and output are contiguous arrays of `close.len()` values;
    // start/end and period fit the TA-Lib C API's i32 interface for this probe.
    unsafe {
        TA_ADX(
            0,
            (close.len() - 1) as i32,
            high.as_ptr(),
            low.as_ptr(),
            close.as_ptr(),
            period,
            &mut begin,
            &mut count,
            raw.as_mut_ptr(),
        );
    }
    let mut output = vec![f64::NAN; close.len()];
    let begin = begin as usize;
    let count = count as usize;
    output[begin..begin + count].copy_from_slice(&raw[..count]);
    output
}

#[cfg(feature = "talib-c")]
fn talib_adxr(high: &[f64], low: &[f64], close: &[f64], period: i32) -> Vec<f64> {
    let mut raw = vec![0.0; close.len()];
    let mut begin = 0;
    let mut count = 0;
    // SAFETY: pointers reference contiguous arrays sized to the requested range.
    unsafe {
        TA_ADXR(
            0,
            (close.len() - 1) as i32,
            high.as_ptr(),
            low.as_ptr(),
            close.as_ptr(),
            period,
            &mut begin,
            &mut count,
            raw.as_mut_ptr(),
        );
    }
    let mut output = vec![f64::NAN; close.len()];
    let begin = begin as usize;
    let count = count as usize;
    output[begin..begin + count].copy_from_slice(&raw[..count]);
    output
}

fn close_enough(left: &[f64], right: &[f64]) -> (bool, f64) {
    if left.len() != right.len() {
        return (false, f64::INFINITY);
    }
    let mut max_abs: f64 = 0.0;
    for (a, b) in left.iter().zip(right) {
        if a.is_nan() && b.is_nan() {
            continue;
        }
        if !a.is_finite() || !b.is_finite() {
            return (false, f64::INFINITY);
        }
        let diff = (a - b).abs();
        max_abs = max_abs.max(diff);
        if diff > 1e-8 + 1e-8 * a.abs().max(b.abs()) {
            return (false, max_abs);
        }
    }
    (true, max_abs)
}

fn duration_per_call(mut operation: impl FnMut(), target: Duration) -> Duration {
    let started = Instant::now();
    let mut iterations = 0u32;
    while started.elapsed() < target {
        operation();
        iterations += 1;
    }
    started.elapsed() / iterations
}

fn main() {
    for len in [1_000, 100_000, 1_000_000] {
        let (high, low, close) = inputs(len);
        let expected = state_reference(&high, &low, &close, 14);
        let actual = adx(&high, &low, &close, 14).unwrap();
        let parity = same_bits(actual.as_slice().unwrap(), &expected);
        let expected_adxr = state_adxr_reference(&high, &low, &close, 14);
        let actual_adxr = adxr(&high, &low, &close, 14).unwrap();
        let adxr_parity = same_bits(actual_adxr.as_slice().unwrap(), &expected_adxr);
        #[cfg(feature = "talib-c")]
        let talib = talib_adx(&high, &low, &close, 14);
        #[cfg(feature = "talib-c")]
        let (talib_parity, max_abs) = close_enough(actual.as_slice().unwrap(), &talib);
        #[cfg(feature = "talib-c")]
        let talib_adxr_values = talib_adxr(&high, &low, &close, 14);
        #[cfg(feature = "talib-c")]
        let (talib_adxr_parity, adxr_max_abs) =
            close_enough(actual_adxr.as_slice().unwrap(), &talib_adxr_values);
        #[cfg(feature = "talib-c")]
        let talib_details = format!(
            " talib_parity={talib_parity} max_abs={max_abs:.3e} adxr_parity={talib_adxr_parity} adxr_max_abs={adxr_max_abs:.3e}"
        );
        #[cfg(not(feature = "talib-c"))]
        let talib_details = String::new();
        let optimized = duration_per_call(
            || {
                black_box(adx(black_box(&high), &low, &close, 14).unwrap());
            },
            Duration::from_millis(150),
        );
        let optimized_adxr = duration_per_call(
            || {
                black_box(adxr(black_box(&high), &low, &close, 14).unwrap());
            },
            Duration::from_millis(150),
        );
        #[cfg(feature = "talib-c")]
        let talib_time = duration_per_call(
            || {
                black_box(talib_adx(black_box(&high), &low, &close, 14));
            },
            Duration::from_millis(150),
        );
        #[cfg(feature = "talib-c")]
        let talib_adxr_time = duration_per_call(
            || {
                black_box(talib_adxr(black_box(&high), &low, &close, 14));
            },
            Duration::from_millis(150),
        );
        #[cfg(feature = "talib-c")]
        let talib_compare_details = format!(
            " talib={:.3}us ratio={:.3}x adxr={:.3}us talib_adxr={:.3}us adxr_ratio={:.3}x",
            talib_time.as_secs_f64() * 1e6,
            talib_time.as_secs_f64() / optimized.as_secs_f64(),
            optimized_adxr.as_secs_f64() * 1e6,
            talib_adxr_time.as_secs_f64() * 1e6,
            talib_adxr_time.as_secs_f64() / optimized_adxr.as_secs_f64(),
        );
        #[cfg(not(feature = "talib-c"))]
        let talib_compare_details = String::new();
        let reference = duration_per_call(
            || {
                black_box(state_reference(black_box(&high), &low, &close, 14));
            },
            Duration::from_millis(150),
        );
        println!(
            "n={len} bitwise_parity={parity} adxr_state_parity={adxr_parity}{} specialized={:.3}us state={:.3}us speedup={:.3}x{}",
            talib_details,
            optimized.as_secs_f64() * 1e6,
            reference.as_secs_f64() * 1e6,
            reference.as_secs_f64() / optimized.as_secs_f64(),
            talib_compare_details,
        );
    }
}
