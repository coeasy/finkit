use finkit::indicators::{std_dev, var};
use finkit::talib_ffi::{TA_STDDEV, TA_VAR};
use std::hint::black_box;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::time::Instant;

unsafe extern "C" {
    fn TA_GetVersionString() -> *const std::ffi::c_char;
}

fn inputs(len: usize) -> Vec<f64> {
    (0..len)
        .map(|index| {
            let x = index as f64;
            100.0 + 0.006 * x + (x * 0.017).sin() * 2.1
        })
        .collect()
}

fn talib_var(input: &[f64], period: usize) -> Vec<f64> {
    let count = input.len() - period + 1;
    let mut raw = Vec::<MaybeUninit<f64>>::with_capacity(count);
    // SAFETY: TA_VAR writes every element in its reported dense output range.
    unsafe { raw.set_len(count) };
    let mut begin = 0;
    let mut elements = 0;
    // SAFETY: the input and output buffers cover the requested range.
    let status = unsafe {
        TA_VAR(
            0,
            (input.len() - 1) as i32,
            input.as_ptr(),
            period as i32,
            1.0,
            &mut begin,
            &mut elements,
            raw.as_mut_ptr().cast(),
        )
    };
    assert_eq!(status, 0, "TA_VAR failed with status {status}");
    assert_eq!(begin, (period - 1) as i32);
    assert_eq!(elements, count as i32);
    let mut raw = ManuallyDrop::new(raw);
    // SAFETY: TA_VAR succeeded and initialized all `count` output elements.
    unsafe { Vec::from_raw_parts(raw.as_mut_ptr().cast(), count, raw.capacity()) }
}

fn talib_stddev(input: &[f64], period: usize) -> Vec<f64> {
    let count = input.len() - period + 1;
    let mut raw = Vec::<MaybeUninit<f64>>::with_capacity(count);
    // SAFETY: TA_STDDEV writes every element in its reported dense output range.
    unsafe { raw.set_len(count) };
    let mut begin = 0;
    let mut elements = 0;
    // SAFETY: the input and output buffers cover the requested range.
    let status = unsafe {
        TA_STDDEV(
            0,
            (input.len() - 1) as i32,
            input.as_ptr(),
            period as i32,
            1.0,
            &mut begin,
            &mut elements,
            raw.as_mut_ptr().cast(),
        )
    };
    assert_eq!(status, 0, "TA_STDDEV failed with status {status}");
    assert_eq!(begin, (period - 1) as i32);
    assert_eq!(elements, count as i32);
    let mut raw = ManuallyDrop::new(raw);
    // SAFETY: TA_STDDEV succeeded and initialized all `count` output elements.
    unsafe { Vec::from_raw_parts(raw.as_mut_ptr().cast(), count, raw.capacity()) }
}

fn elapsed_us(mut run: impl FnMut()) -> f64 {
    let start = Instant::now();
    run();
    start.elapsed().as_secs_f64() * 1e6
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn check_stddev_warmups() {
    let input = [f64::NAN, f64::NAN, 1.0, 2.0, 3.0, 4.0];
    let values = std_dev(&input, 3, 1.0).unwrap();
    let values = values.as_slice().unwrap();
    assert!(values[..4].iter().all(|value| value.is_nan()));
    let expected = (2.0_f64 / 3.0).sqrt();
    assert!((values[4] - expected).abs() < 1e-15);
    assert!((values[5] - expected).abs() < 1e-15);

    let no_full_window = std_dev(&input[..4], 3, 1.0).unwrap();
    assert!(no_full_window
        .as_slice()
        .unwrap()
        .iter()
        .all(|value| value.is_nan()));

    let regular = std_dev(&[1.0, 2.0, 3.0, 4.0], 3, 1.0).unwrap();
    let regular = regular.as_slice().unwrap();
    assert!(regular[..2].iter().all(|value| value.is_nan()));
    assert!((regular[2] - expected).abs() < 1e-15);
    assert!((regular[3] - expected).abs() < 1e-15);
}

fn run(len: usize, period: usize) {
    let input = inputs(len);
    let rust = var(&input, period, 1.0).unwrap();
    let c = talib_var(&input, period);
    let mut max_abs = 0.0f64;
    let mut mismatches = 0usize;
    for (&left, &right) in rust.iter().skip(period - 1).zip(&c) {
        let difference = (left - right).abs();
        max_abs = max_abs.max(difference);
        if difference > 1e-8 + 1e-8 * left.abs().max(right.abs()) {
            mismatches += 1;
        }
    }

    let mut rust_samples = [0.0; 9];
    let mut c_samples = [0.0; 9];
    for index in 0..rust_samples.len() {
        if index % 2 == 0 {
            rust_samples[index] = elapsed_us(|| {
                black_box(var(black_box(&input), period, 1.0).unwrap());
            });
            c_samples[index] = elapsed_us(|| {
                black_box(talib_var(black_box(&input), period));
            });
        } else {
            c_samples[index] = elapsed_us(|| {
                black_box(talib_var(black_box(&input), period));
            });
            rust_samples[index] = elapsed_us(|| {
                black_box(var(black_box(&input), period, 1.0).unwrap());
            });
        }
    }
    let rust_us = median(&mut rust_samples);
    let c_us = median(&mut c_samples);
    println!(
        "VAR n={len} period={period} mismatches={mismatches} max_abs={max_abs:.3e} rust={rust_us:.3}us c={c_us:.3}us speedup={:.3}x",
        c_us / rust_us,
    );

    let rust = std_dev(&input, period, 1.0).unwrap();
    let c = talib_stddev(&input, period);
    let mut max_abs = 0.0f64;
    let mut mismatches = 0usize;
    for (&left, &right) in rust.iter().skip(period - 1).zip(&c) {
        let difference = (left - right).abs();
        max_abs = max_abs.max(difference);
        if difference > 1e-8 + 1e-8 * left.abs().max(right.abs()) {
            mismatches += 1;
        }
    }

    let mut rust_samples = [0.0; 9];
    let mut c_samples = [0.0; 9];
    for index in 0..rust_samples.len() {
        if index % 2 == 0 {
            rust_samples[index] = elapsed_us(|| {
                black_box(std_dev(black_box(&input), period, 1.0).unwrap());
            });
            c_samples[index] = elapsed_us(|| {
                black_box(talib_stddev(black_box(&input), period));
            });
        } else {
            c_samples[index] = elapsed_us(|| {
                black_box(talib_stddev(black_box(&input), period));
            });
            rust_samples[index] = elapsed_us(|| {
                black_box(std_dev(black_box(&input), period, 1.0).unwrap());
            });
        }
    }
    let rust_us = median(&mut rust_samples);
    let c_us = median(&mut c_samples);
    assert_eq!(mismatches, 0);
    println!(
        "STDDEV n={len} period={period} mismatches={mismatches} max_abs={max_abs:.3e} rust={rust_us:.3}us c={c_us:.3}us speedup={:.3}x",
        c_us / rust_us,
    );
}

fn main() {
    check_stddev_warmups();
    // SAFETY: TA-Lib returns a pointer to its static NUL-terminated version string.
    let version = unsafe { std::ffi::CStr::from_ptr(TA_GetVersionString()) };
    println!("TA-Lib C version: {}", version.to_string_lossy());
    for period in [5, 20, 100] {
        for len in [1_000, 100_000, 1_000_000] {
            if len >= period {
                run(len, period);
            }
        }
    }
}
