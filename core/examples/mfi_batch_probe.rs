use finkit::indicators::mfi;
use finkit::talib_ffi::TA_MFI;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn inputs(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut volume = Vec::with_capacity(len);
    for i in 0..len {
        let x = i as f64;
        let c = 100.0 + 0.006 * x + (x * 0.017).sin() * 2.1;
        let o = c - 0.15 + (x * 0.031).sin() * 0.05;
        high.push(o.max(c) + 0.7 + (x * 0.023).sin().abs() * 0.3);
        low.push(o.min(c) - 0.8 - (x * 0.019).cos().abs() * 0.2);
        close.push(c);
        volume.push(1_000_000.0 + (x % 997.0) * 170.0 + (x * 0.011).sin().abs() * 50_000.0);
    }
    (high, low, close, volume)
}

fn talib_mfi(high: &[f64], low: &[f64], close: &[f64], volume: &[f64], period: i32) -> Vec<f64> {
    let mut raw = vec![0.0; close.len()];
    let mut begin = 0;
    let mut count = 0;
    // SAFETY: all inputs and the output are contiguous and cover the requested range.
    let status = unsafe {
        TA_MFI(
            0,
            (close.len() - 1) as i32,
            high.as_ptr(),
            low.as_ptr(),
            close.as_ptr(),
            volume.as_ptr(),
            period,
            &mut begin,
            &mut count,
            raw.as_mut_ptr(),
        )
    };
    assert_eq!(status, 0);
    let mut output = vec![f64::NAN; close.len()];
    output[begin as usize..begin as usize + count as usize].copy_from_slice(&raw[..count as usize]);
    output
}

fn elapsed_per_call<R>(mut operation: impl FnMut() -> R, target: Duration) -> f64 {
    let started = Instant::now();
    let mut calls = 0_u32;
    while started.elapsed() < target {
        black_box(operation());
        calls += 1;
    }
    started.elapsed().as_secs_f64() * 1e6 / calls as f64
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn main() {
    for len in [1_000, 100_000, 1_000_000] {
        let (high, low, close, volume) = inputs(len);
        let expected = talib_mfi(&high, &low, &close, &volume, 14);
        let actual = mfi(&high, &low, &close, &volume, 14).unwrap();
        let mut max_abs = 0.0_f64;
        let mut mismatches = 0;
        for (a, b) in actual.iter().zip(&expected) {
            if a.is_nan() && b.is_nan() {
                continue;
            }
            let diff = (a - b).abs();
            max_abs = max_abs.max(diff);
            if diff > 1e-8 + 1e-8 * a.abs().max(b.abs()) {
                mismatches += 1;
            }
        }
        let rust = || {
            black_box(
                mfi(
                    black_box(&high),
                    black_box(&low),
                    black_box(&close),
                    black_box(&volume),
                    14,
                )
                .unwrap(),
            )
        };
        let c = || {
            black_box(talib_mfi(
                black_box(&high),
                black_box(&low),
                black_box(&close),
                black_box(&volume),
                14,
            ))
        };
        let mut rust_samples = [0.0; 7];
        let mut c_samples = [0.0; 7];
        for i in 0..rust_samples.len() {
            if i % 2 == 0 {
                rust_samples[i] = elapsed_per_call(rust, Duration::from_millis(100));
                c_samples[i] = elapsed_per_call(c, Duration::from_millis(100));
            } else {
                c_samples[i] = elapsed_per_call(c, Duration::from_millis(100));
                rust_samples[i] = elapsed_per_call(rust, Duration::from_millis(100));
            }
        }
        let rust_us = median(&mut rust_samples);
        let c_us = median(&mut c_samples);
        println!("MFI n={len} mismatches={mismatches} max_abs={max_abs:.3e} rust={rust_us:.3}us C={c_us:.3}us speedup={:.3}x", c_us / rust_us);
    }
}
