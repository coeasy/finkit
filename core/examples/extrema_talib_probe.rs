use finkit::indicators::math_operators::{max, min, minmax};
use finkit::talib_ffi::{TA_Initialize, TA_Shutdown, TA_MAX, TA_MIN, TA_SUCCESS};
use ndarray::Array1;
use std::hint::black_box;
use std::time::{Duration, Instant};

fn data(len: usize) -> Vec<f64> {
    (0..len)
        .map(|i| {
            let x = i as f64;
            100.0 + 0.006 * x + (x * 0.017).sin() * 2.1
        })
        .collect()
}

fn talib(input: &[f64], want_max: bool) -> Vec<f64> {
    let len = input.len();
    let mut output = vec![0.0; len];
    let (mut begin, mut count) = (0i32, 0i32);
    let status = unsafe {
        if want_max {
            TA_MAX(
                0,
                (len - 1) as i32,
                input.as_ptr(),
                30,
                &mut begin,
                &mut count,
                output.as_mut_ptr(),
            )
        } else {
            TA_MIN(
                0,
                (len - 1) as i32,
                input.as_ptr(),
                30,
                &mut begin,
                &mut count,
                output.as_mut_ptr(),
            )
        }
    };
    assert_eq!(status, TA_SUCCESS);
    assert_eq!(begin, 29);
    assert_eq!(count as usize, len - 29);
    output
}

fn rust(input: &[f64], want_max: bool) -> Array1<f64> {
    if want_max {
        max(input, 30).unwrap()
    } else {
        min(input, 30).unwrap()
    }
}

fn talib_minmax(input: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let len = input.len();
    let mut minimum = vec![0.0; len];
    let mut maximum = vec![0.0; len];
    let (mut begin, mut count) = (0i32, 0i32);
    let status = unsafe {
        finkit::talib_ffi::TA_MINMAX(
            0,
            (len - 1) as i32,
            input.as_ptr(),
            30,
            &mut begin,
            &mut count,
            minimum.as_mut_ptr(),
            maximum.as_mut_ptr(),
        )
    };
    assert_eq!(status, TA_SUCCESS);
    assert_eq!(begin, 29);
    assert_eq!(count as usize, len - 29);
    (minimum, maximum)
}

fn measure_minmax(input: &[f64]) {
    let expected = talib_minmax(input);
    let actual = minmax(input, 30).unwrap();
    for index in 0..input.len() - 29 {
        assert_eq!(
            expected.0[index],
            actual.0[index + 29],
            "MINMAX min at {index}"
        );
        assert_eq!(
            expected.1[index],
            actual.1[index + 29],
            "MINMAX max at {index}"
        );
    }
    assert!(actual.0.as_slice().unwrap()[..29]
        .iter()
        .all(|value| value.is_nan()));
    assert!(actual.1.as_slice().unwrap()[..29]
        .iter()
        .all(|value| value.is_nan()));

    let mut finkit_samples = Vec::new();
    let mut talib_samples = Vec::new();
    for round in 0..9 {
        if round % 2 == 0 {
            finkit_samples.push(time(
                || minmax(input, 30).unwrap(),
                Duration::from_millis(120),
            ));
            talib_samples.push(time(|| talib_minmax(input), Duration::from_millis(120)));
        } else {
            talib_samples.push(time(|| talib_minmax(input), Duration::from_millis(120)));
            finkit_samples.push(time(
                || minmax(input, 30).unwrap(),
                Duration::from_millis(120),
            ));
        }
    }
    finkit_samples.sort_by(f64::total_cmp);
    talib_samples.sort_by(f64::total_cmp);
    println!(
        "MINMAX n={}: Finkit={:.2}us TA-Lib={:.2}us speedup={:.3}x",
        input.len(),
        finkit_samples[4],
        talib_samples[4],
        talib_samples[4] / finkit_samples[4]
    );
}

fn previous_precheck_path(input: &[f64], want_max: bool) -> Array1<f64> {
    if want_max {
        previous_precheck_path_const::<true>(input)
    } else {
        previous_precheck_path_const::<false>(input)
    }
}

fn previous_precheck_path_const<const WANT_MAX: bool>(input: &[f64]) -> Array1<f64> {
    const PERIOD: usize = 30;
    let len = input.len();
    let mut output = Vec::with_capacity(len);
    // SAFETY: the old block loop writes every index from PERIOD - 1 onward;
    // the warm-up prefix is initialized immediately below.
    unsafe { output.set_len(len) };
    output[..PERIOD - 1].fill(f64::NAN);
    assert!(input.iter().all(|value| value.is_finite()));

    let mut suffix = [0.0; PERIOD];
    let mut prefix = [0.0; PERIOD];
    let mut block_start = 0usize;
    let mut today = PERIOD - 1;
    while today < len {
        let block_end = block_start + PERIOD - 1;
        let mut extreme = input[block_end];
        suffix[PERIOD - 1] = extreme;
        let mut offset = PERIOD - 1;
        while offset > 0 {
            offset -= 1;
            let value = input[block_start + offset];
            if if WANT_MAX {
                value > extreme
            } else {
                value < extreme
            } {
                extreme = value;
            }
            suffix[offset] = extreme;
        }
        output[today] = suffix[0];

        let block_next = block_start + PERIOD;
        if block_next >= len {
            break;
        }
        let n_available = (len - block_next).min(PERIOD - 1);
        extreme = input[block_next];
        prefix[0] = extreme;
        let mut grown = 1usize;
        while grown < n_available {
            let value = input[block_next + grown];
            if if WANT_MAX {
                value > extreme
            } else {
                value < extreme
            } {
                extreme = value;
            }
            prefix[grown] = extreme;
            grown += 1;
        }
        for offset in 1..=n_available {
            let left = suffix[offset];
            let right = prefix[offset - 1];
            output[today + offset] = if if WANT_MAX { right > left } else { right < left } {
                right
            } else {
                left
            };
        }
        block_start += PERIOD;
        today += n_available + 1;
    }
    Array1::from_vec(output)
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

fn measure(input: &[f64], want_max: bool) {
    let label = if want_max { "MAX" } else { "MIN" };
    let mut rust_samples = Vec::new();
    let mut talib_samples = Vec::new();
    for round in 0..9 {
        if round % 2 == 0 {
            rust_samples.push(time(|| rust(input, want_max), Duration::from_millis(120)));
            talib_samples.push(time(|| talib(input, want_max), Duration::from_millis(120)));
        } else {
            talib_samples.push(time(|| talib(input, want_max), Duration::from_millis(120)));
            rust_samples.push(time(|| rust(input, want_max), Duration::from_millis(120)));
        }
    }
    rust_samples.sort_by(f64::total_cmp);
    talib_samples.sort_by(f64::total_cmp);
    let rust_us = rust_samples[4];
    let talib_us = talib_samples[4];
    println!(
        "{label} n={}: Finkit={rust_us:.2}us TA-Lib={talib_us:.2}us speedup={:.3}x",
        input.len(),
        talib_us / rust_us
    );

    let mut old_samples = Vec::new();
    let mut current_samples = Vec::new();
    for round in 0..9 {
        if round % 2 == 0 {
            old_samples.push(time(
                || previous_precheck_path(input, want_max),
                Duration::from_millis(120),
            ));
            current_samples.push(time(|| rust(input, want_max), Duration::from_millis(120)));
        } else {
            current_samples.push(time(|| rust(input, want_max), Duration::from_millis(120)));
            old_samples.push(time(
                || previous_precheck_path(input, want_max),
                Duration::from_millis(120),
            ));
        }
    }
    old_samples.sort_by(f64::total_cmp);
    current_samples.sort_by(f64::total_cmp);
    println!(
        "{label} n={}: old-precheck={:.2}us fused-detect={:.2}us gain={:.3}x",
        input.len(),
        old_samples[4],
        current_samples[4],
        old_samples[4] / current_samples[4]
    );
}

fn main() {
    assert_eq!(unsafe { TA_Initialize() }, TA_SUCCESS);
    for len in [1_000usize, 100_000, 1_000_000] {
        let input = data(len);
        for want_max in [true, false] {
            let expected = talib(&input, want_max);
            let actual = rust(&input, want_max);
            let previous = previous_precheck_path(&input, want_max);
            let actual = actual.as_slice().unwrap();
            assert!(previous.iter().zip(actual).all(|(&left, &right)| {
                (left.is_nan() && right.is_nan()) || left.to_bits() == right.to_bits()
            }));
            assert!(actual[..29].iter().all(|value| value.is_nan()));
            for (index, (&left, &right)) in
                expected[..len - 29].iter().zip(&actual[29..]).enumerate()
            {
                if left.is_finite() {
                    assert_eq!(left, right, "value mismatch at {}", index + 29);
                }
            }
            measure(&input, want_max);
        }
        measure_minmax(&input);
    }
    assert_eq!(unsafe { TA_Shutdown() }, TA_SUCCESS);
}
