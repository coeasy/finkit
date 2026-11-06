use finkit::indicators::momentum::cmo_fast_into;
use finkit::talib_ffi::TA_CMO;
use std::hint::black_box;
use std::time::Instant;

const PERIOD: usize = 14;

fn input(len: usize) -> Vec<f64> {
    (0..len)
        .map(|index| {
            let x = index as f64;
            100.0 + 0.006 * x + (x * 0.017).sin() * 2.1
        })
        .collect()
}

fn legacy(input: &[f64], period: usize, output: &mut [f64]) {
    output[..=period].fill(f64::NAN);
    let mut up = 0.0;
    let mut down = 0.0;
    for index in 1..=period {
        let change = input[index] - input[index - 1];
        if change > 0.0 {
            up += change;
        } else {
            down -= change;
        }
    }
    let denominator = up + down;
    if !(denominator > -1e-8 && denominator < 1e-8) {
        output[period] = (up - down) / denominator * 100.0;
    }
    let inverse = 1.0 / period as f64;
    let factor = period as f64 - 1.0;
    up *= inverse;
    down *= inverse;
    for index in period + 1..input.len() {
        let change = input[index] - input[index - 1];
        let positive = if change > 0.0 { change } else { 0.0 };
        let negative = if change < 0.0 { -change } else { 0.0 };
        up = (up * factor + positive) * inverse;
        down = (down * factor + negative) * inverse;
        let denominator = up + down;
        output[index] = if !(denominator > -1e-8 && denominator < 1e-8) {
            (up - down) / denominator * 100.0
        } else {
            f64::NAN
        };
    }
}

fn check_small_magnitudes() {
    for scale in [0.0, 1e-9, 1e-8, 1e-7, 1e-6] {
        let input: Vec<_> = (0..4096)
            .map(|index| {
                let x = index as f64;
                scale * (0.2 * x + (x * 0.19).sin())
            })
            .collect();
        let mut current = vec![f64::NAN; input.len()];
        let mut old = vec![f64::NAN; input.len()];
        cmo_fast_into(&input, PERIOD, &mut current).unwrap();
        legacy(&input, PERIOD, &mut old);
        let mut mask_mismatches = 0;
        let mut value_mismatches = 0;
        for (&new, &previous) in current.iter().zip(&old) {
            if new.is_nan() != previous.is_nan() {
                mask_mismatches += 1;
            } else if new.is_finite() && (new - previous).abs() > 1e-8 + 1e-8 * previous.abs() {
                value_mismatches += 1;
            }
        }
        println!(
            "CMO edge scale={scale:.0e} mask_mismatches={mask_mismatches} value_mismatches={value_mismatches}"
        );
    }
}

fn talib(input: &[f64], period: usize, output: &mut [f64]) -> (usize, usize) {
    let mut begin = 0;
    let mut count = 0;
    let status = unsafe {
        TA_CMO(
            0,
            input.len() as i32 - 1,
            input.as_ptr(),
            period as i32,
            &mut begin,
            &mut count,
            output.as_mut_ptr(),
        )
    };
    assert_eq!(status, 0);
    (begin as usize, count as usize)
}

fn time(mut f: impl FnMut()) -> f64 {
    for _ in 0..3 {
        f();
    }
    let target_ns = 100_000_000_u128;
    let started = Instant::now();
    let mut iterations = 0_u64;
    while started.elapsed().as_nanos() < target_ns {
        f();
        iterations += 1;
    }
    started.elapsed().as_secs_f64() * 1e6 / iterations as f64
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn main() {
    check_small_magnitudes();
    for len in [1_000, 100_000, 1_000_000] {
        let input = input(len);
        let mut current = vec![f64::NAN; len];
        let mut old = vec![f64::NAN; len];
        let mut c = vec![f64::NAN; len];
        cmo_fast_into(&input, PERIOD, &mut current).unwrap();
        legacy(&input, PERIOD, &mut old);
        let (begin, count) = talib(&input, PERIOD, &mut c);
        c.copy_within(0..count, begin);
        c[..begin].fill(f64::NAN);
        let mut current_max = 0.0f64;
        let mut old_max = 0.0f64;
        let mut current_bad = 0;
        let mut old_bad = 0;
        for index in begin..begin + count {
            current_max = current_max.max((current[index] - c[index]).abs());
            old_max = old_max.max((old[index] - c[index]).abs());
            if (current[index] - c[index]).abs() > 1e-8 + 1e-8 * c[index].abs() {
                current_bad += 1;
            }
            if (old[index] - c[index]).abs() > 1e-8 + 1e-8 * c[index].abs() {
                old_bad += 1;
            }
        }
        let mut current_samples = [0.0; 5];
        let mut old_samples = [0.0; 5];
        let mut talib_samples = [0.0; 5];
        for round in 0..5 {
            if round % 2 == 0 {
                current_samples[round] = time(|| {
                    cmo_fast_into(black_box(&input), PERIOD, black_box(&mut current)).unwrap();
                });
                old_samples[round] = time(|| {
                    legacy(black_box(&input), PERIOD, black_box(&mut old));
                });
                talib_samples[round] = time(|| {
                    black_box(talib(black_box(&input), PERIOD, black_box(&mut c)));
                });
            } else {
                talib_samples[round] = time(|| {
                    black_box(talib(black_box(&input), PERIOD, black_box(&mut c)));
                });
                old_samples[round] = time(|| {
                    legacy(black_box(&input), PERIOD, black_box(&mut old));
                });
                current_samples[round] = time(|| {
                    cmo_fast_into(black_box(&input), PERIOD, black_box(&mut current)).unwrap();
                });
            }
        }
        let current_us = median(&mut current_samples);
        let old_us = median(&mut old_samples);
        let talib_us = median(&mut talib_samples);
        println!(
            "CMO n={len} parity_bad current={current_bad} old={old_bad} max_abs current={current_max:.3e} old={old_max:.3e} us current={current_us:.3} old={old_us:.3} talib={talib_us:.3} speedup current={:.3}x old={:.3}x",
            talib_us / current_us,
            talib_us / old_us,
        );
    }
}
