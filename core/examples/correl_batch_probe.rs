use finkit::indicators::correlation;
use finkit::talib_ffi::TA_CORREL;
use std::ffi::CStr;
use std::hint::black_box;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::time::Instant;

unsafe extern "C" {
    fn TA_GetVersionString() -> *const std::ffi::c_char;
}

fn inputs(len: usize) -> (Vec<f64>, Vec<f64>) {
    let mut left = Vec::with_capacity(len);
    let mut right = Vec::with_capacity(len);
    for i in 0..len {
        let x = i as f64;
        let close = 100.0 + 0.006 * x + (x * 0.017).sin() * 2.1;
        let open = close - 0.15 + (x * 0.031).sin() * 0.05;
        left.push(close);
        right.push(open);
    }
    (left, right)
}

fn c_correlation(left: &[f64], right: &[f64], period: usize) -> Vec<f64> {
    let count = left.len() - period + 1;
    let mut raw = Vec::<MaybeUninit<f64>>::with_capacity(count);
    // SAFETY: MaybeUninit permits the output slots to stay uninitialized until
    // TA_CORREL writes its reported dense result range.
    unsafe { raw.set_len(count) };
    let mut begin = 0;
    let mut elements = 0;
    // SAFETY: both input slices and the output allocation cover the requested
    // range and remain alive for the duration of the C call.
    let status = unsafe {
        TA_CORREL(
            0,
            (left.len() - 1) as i32,
            left.as_ptr(),
            right.as_ptr(),
            period as i32,
            &mut begin,
            &mut elements,
            raw.as_mut_ptr().cast(),
        )
    };
    assert_eq!(status, 0, "TA_CORREL failed with status {status}");
    assert_eq!(begin, (period - 1) as i32);
    assert_eq!(elements, count as i32);
    let mut raw = ManuallyDrop::new(raw);
    // SAFETY: TA_CORREL succeeded and wrote every one of the `count` output
    // elements. `MaybeUninit<f64>` and `f64` have identical layout/alignment.
    unsafe { Vec::from_raw_parts(raw.as_mut_ptr().cast(), count, raw.capacity()) }
}

fn one_call_us(mut run: impl FnMut()) -> f64 {
    let started = Instant::now();
    run();
    started.elapsed().as_secs_f64() * 1e6
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn run(len: usize) {
    let period = 20usize;
    let (left, right) = inputs(len);
    let actual = correlation(&left, &right, period).unwrap();
    let expected = c_correlation(&left, &right, period);
    let mut max_abs = 0.0f64;
    let mut mismatches = 0usize;
    for (rust, c) in actual.iter().skip(period - 1).zip(&expected) {
        let difference = (rust - c).abs();
        max_abs = max_abs.max(difference);
        if difference > 1e-8 + 1e-8 * rust.abs().max(c.abs()) {
            mismatches += 1;
        }
    }

    let mut rust_samples = [0.0; 9];
    let mut c_samples = [0.0; 9];
    for index in 0..rust_samples.len() {
        if index % 2 == 0 {
            rust_samples[index] = one_call_us(|| {
                black_box(correlation(black_box(&left), black_box(&right), period).unwrap());
            });
            c_samples[index] = one_call_us(|| {
                black_box(c_correlation(black_box(&left), black_box(&right), period));
            });
        } else {
            c_samples[index] = one_call_us(|| {
                black_box(c_correlation(black_box(&left), black_box(&right), period));
            });
            rust_samples[index] = one_call_us(|| {
                black_box(correlation(black_box(&left), black_box(&right), period).unwrap());
            });
        }
    }
    let rust_us = median(&mut rust_samples);
    let c_us = median(&mut c_samples);
    println!(
        "CORREL n={len} period={period} mismatches={mismatches} max_abs={max_abs:.3e} rust={rust_us:.3}us c={c_us:.3}us speedup={:.3}x",
        c_us / rust_us,
    );
}

fn main() {
    // SAFETY: TA-Lib returns a pointer to its static NUL-terminated version string.
    let version = unsafe { CStr::from_ptr(TA_GetVersionString()) };
    println!("TA-Lib C version: {}", version.to_string_lossy());
    for len in [1_000, 100_000, 1_000_000] {
        run(len);
    }
}
