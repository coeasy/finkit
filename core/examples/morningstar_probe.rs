use finkit::patterns::candlestick::morning_star;
use finkit::talib_ffi::{TA_Initialize, TA_Shutdown, TA_CDLMORNINGSTAR, TA_SUCCESS};
use std::hint::black_box;
use std::time::{Duration, Instant};

fn inputs(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    for i in 0..len {
        let x = i as f64;
        let close_value = 100.0 + 0.006 * x + (x * 0.017).sin() * 2.1;
        let open_value = close_value - 0.15 + (x * 0.031).sin() * 0.05;
        open.push(open_value);
        high.push(open_value.max(close_value) + 0.7 + (x * 0.023).sin().abs() * 0.3);
        low.push(open_value.min(close_value) - 0.8 - (x * 0.019).cos().abs() * 0.2);
        close.push(close_value);
    }
    (open, high, low, close)
}

fn old_morning_star(open: &[f64], close: &[f64]) -> Vec<i32> {
    let len = open.len();
    let bodies = open
        .iter()
        .zip(close)
        .map(|(open, close)| (close - open).abs())
        .collect::<Vec<_>>();
    let mut output = vec![0; len];
    if bodies.iter().any(|value| !value.is_finite()) || len <= 12 {
        return output;
    }
    let mut long_sum = bodies[0..10].iter().sum::<f64>();
    let mut star_sum = bodies[1..11].iter().sum::<f64>();
    let mut last_sum = bodies[2..12].iter().sum::<f64>();
    for i in 12..len {
        let first = bodies[i - 2];
        if open[i - 2] > close[i - 2]
            && close[i] > open[i]
            && open[i - 1].max(close[i - 1]) < open[i - 2].min(close[i - 2])
            && close[i] > close[i - 2]
            && first > long_sum / 10.0
            && bodies[i - 1] <= star_sum / 10.0
            && bodies[i] > last_sum / 10.0
        {
            output[i] = 100;
        }
        if i + 1 < len {
            long_sum += bodies[i - 2] - bodies[i - 12];
            star_sum += bodies[i - 1] - bodies[i - 11];
            last_sum += bodies[i] - bodies[i - 10];
        }
    }
    output
}

fn talib_morning_star(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Vec<i32> {
    let mut output = vec![0; close.len()];
    let (mut begin, mut count) = (0i32, 0i32);
    // SAFETY: inputs and output are contiguous and equal-length; TA-Lib uses
    // 32-bit indices, and this probe only constructs supported row counts.
    let status = unsafe {
        TA_CDLMORNINGSTAR(
            0,
            close.len() as i32 - 1,
            open.as_ptr(),
            high.as_ptr(),
            low.as_ptr(),
            close.as_ptr(),
            0.3,
            &mut begin,
            &mut count,
            output.as_mut_ptr(),
        )
    };
    assert_eq!(status, TA_SUCCESS);
    let begin = begin as usize;
    let count = count as usize;
    output.copy_within(0..count, begin);
    output[..begin].fill(0);
    output[begin + count..].fill(0);
    output
}

fn time<T>(mut f: impl FnMut() -> T, duration: Duration) -> f64 {
    let started = Instant::now();
    let mut calls = 0u64;
    while started.elapsed() < duration {
        black_box(f());
        calls += 1;
    }
    started.elapsed().as_secs_f64() * 1e6 / calls as f64
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn compare_and_measure(len: usize) {
    let (open, high, low, close) = inputs(len);
    let old = old_morning_star(&open, &close);
    let current = morning_star(&open, &high, &low, &close)
        .unwrap()
        .into_raw_vec_and_offset()
        .0;
    let talib = talib_morning_star(&open, &high, &low, &close);
    assert_eq!(current, old, "rolling result differs from old path n={len}");
    assert_eq!(current, talib, "result differs from TA-Lib n={len}");

    let mut old_samples = Vec::with_capacity(7);
    let mut current_samples = Vec::with_capacity(7);
    let mut talib_samples = Vec::with_capacity(7);
    for round in 0..7 {
        if round % 2 == 0 {
            old_samples.push(time(
                || old_morning_star(&open, &close),
                Duration::from_millis(100),
            ));
            current_samples.push(time(
                || morning_star(&open, &high, &low, &close).unwrap(),
                Duration::from_millis(100),
            ));
            talib_samples.push(time(
                || talib_morning_star(&open, &high, &low, &close),
                Duration::from_millis(100),
            ));
        } else {
            talib_samples.push(time(
                || talib_morning_star(&open, &high, &low, &close),
                Duration::from_millis(100),
            ));
            current_samples.push(time(
                || morning_star(&open, &high, &low, &close).unwrap(),
                Duration::from_millis(100),
            ));
            old_samples.push(time(
                || old_morning_star(&open, &close),
                Duration::from_millis(100),
            ));
        }
    }
    let old_us = median(&mut old_samples);
    let current_us = median(&mut current_samples);
    let talib_us = median(&mut talib_samples);
    println!(
        "CDLMORNINGSTAR n={len}: old={old_us:.2}us current={current_us:.2}us TA-Lib={talib_us:.2}us candidate_speedup={:.3}x Finkit/TA-Lib={:.3}x exact=true",
        old_us / current_us,
        talib_us / current_us,
    );
}

fn main() {
    unsafe { TA_Initialize() };
    for len in [1_000, 10_000, 16_383, 16_384, 100_000, 1_000_000] {
        compare_and_measure(len);
    }
    unsafe { TA_Shutdown() };
}
