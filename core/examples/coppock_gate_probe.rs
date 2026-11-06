use finkit::indicators::momentum_ext::coppock;
use finkit::talib_ffi::{TA_Initialize, TA_Shutdown, TA_SUCCESS};
use ndarray::Array1;
use std::hint::black_box;
use std::time::{Duration, Instant};

unsafe extern "C" {
    fn TA_COPPOCK(
        start_idx: i32,
        end_idx: i32,
        input: *const f64,
        wma_period: i32,
        roc1_period: i32,
        roc2_period: i32,
        out_begin: *mut i32,
        out_count: *mut i32,
        output: *mut f64,
    ) -> i32;
}

fn close_data(len: usize) -> Vec<f64> {
    (0..len)
        .map(|i| {
            let x = i as f64;
            100.0 + 0.006 * x + (x * 0.017).sin() * 2.1
        })
        .collect()
}

#[inline(always)]
fn coppock_value_fixed<const LONG: usize, const SHORT: usize>(input: &[f64], index: usize) -> f64 {
    let long_base = input[index - LONG];
    let short_base = input[index - SHORT];
    let long_value = (input[index] - long_base) / long_base * 100.0;
    let short_value = (input[index] - short_base) / short_base * 100.0;
    long_value + short_value
}

fn coppock_ring_fixed<const WMA: usize, const LONG: usize, const SHORT: usize>(
    input: &[f64],
) -> Array1<f64> {
    let len = input.len();
    let start = LONG.max(SHORT);
    let first = start + WMA - 1;
    let mut output = Vec::with_capacity(len);
    output.resize(first, f64::NAN);
    let mut ring = [0.0; WMA];
    let mut sum = 0.0;
    let mut weighted = 0.0;
    for (offset, slot) in ring.iter_mut().enumerate() {
        let value = coppock_value_fixed::<LONG, SHORT>(input, start + offset);
        *slot = value;
        sum += value;
        weighted += (offset + 1) as f64 * value;
    }
    let inverse = 1.0 / (WMA * (WMA + 1) / 2) as f64;
    output.push(weighted * inverse);
    let mut head = 0usize;
    for index in first + 1..len {
        let old = ring[head];
        let new = coppock_value_fixed::<LONG, SHORT>(input, index);
        weighted += WMA as f64 * new - sum;
        sum += new - old;
        ring[head] = new;
        head += 1;
        if head == WMA {
            head = 0;
        }
        output.push(weighted * inverse);
    }
    Array1::from_vec(output)
}

fn coppock_ring_preflight(input: &[f64]) -> Option<Array1<f64>> {
    let denominator_end = input.len() - 11;
    if input.iter().enumerate().any(|(index, &value)| {
        !value.is_finite() || (index < denominator_end && value > -1e-8 && value < 1e-8)
    }) {
        return None;
    }
    Some(coppock_ring_fixed::<10, 11, 14>(input))
}

fn coppock_ring_fused_check(input: &[f64]) -> Option<Array1<f64>> {
    const WMA: usize = 10;
    const LONG: usize = 11;
    const SHORT: usize = 14;
    let len = input.len();
    let first = SHORT + WMA - 1;
    let mut output = Vec::with_capacity(len);
    output.resize(first, f64::NAN);
    let mut ring = [0.0; WMA];
    let mut sum = 0.0;
    let mut weighted = 0.0;

    #[inline(always)]
    fn checked_value(input: &[f64], index: usize) -> Option<f64> {
        const MAX_SAFE_INPUT: f64 = 1.0e150;
        let current = input[index];
        let long_base = input[index - LONG];
        let short_base = input[index - SHORT];
        if !current.is_finite()
            || !long_base.is_finite()
            || !short_base.is_finite()
            || current.abs() > MAX_SAFE_INPUT
            || long_base.abs() > MAX_SAFE_INPUT
            || short_base.abs() > MAX_SAFE_INPUT
            || (long_base > -1e-8 && long_base < 1e-8)
            || (short_base > -1e-8 && short_base < 1e-8)
        {
            return None;
        }
        let long_value = (current - long_base) / long_base * 100.0;
        let short_value = (current - short_base) / short_base * 100.0;
        Some(long_value + short_value)
    }

    for (offset, slot) in ring.iter_mut().enumerate() {
        let value = checked_value(input, SHORT + offset)?;
        *slot = value;
        sum += value;
        weighted += (offset + 1) as f64 * value;
    }
    let inverse = 1.0 / (WMA * (WMA + 1) / 2) as f64;
    output.push(weighted * inverse);
    let mut head = 0usize;
    for index in first + 1..len {
        let old = ring[head];
        let new = checked_value(input, index)?;
        weighted += WMA as f64 * new - sum;
        sum += new - old;
        ring[head] = new;
        head += 1;
        if head == WMA {
            head = 0;
        }
        output.push(weighted * inverse);
    }
    Some(Array1::from_vec(output))
}

fn old_batch(input: &[f64], wma_period: usize, long_roc: usize, short_roc: usize) -> Array1<f64> {
    let len = input.len();
    let roc_start = long_roc.max(short_roc);
    let mut combined = Array1::from_elem(len, f64::NAN);
    for i in roc_start..len {
        let long_base = input[i - long_roc];
        let short_base = input[i - short_roc];
        let long_value = if long_base > -1e-8 && long_base < 1e-8 {
            0.0
        } else {
            (input[i] - long_base) / long_base * 100.0
        };
        let short_value = if short_base > -1e-8 && short_base < 1e-8 {
            0.0
        } else {
            (input[i] - short_base) / short_base * 100.0
        };
        combined[i] = long_value + short_value;
    }

    let mut output = Array1::from_elem(len, f64::NAN);
    let inverse_weight_sum = 1.0 / (wma_period * (wma_period + 1) / 2) as f64;
    let first = roc_start + wma_period - 1;
    let combined = combined.as_slice().unwrap();
    let start = first + 1 - wma_period;
    let mut window_sum: f64 = combined[start..=first].iter().sum();
    let mut weighted_sum: f64 = combined[start..=first]
        .iter()
        .enumerate()
        .map(|(j, &value)| (j + 1) as f64 * value)
        .sum();
    output[first] = weighted_sum * inverse_weight_sum;
    let mut dirty = window_sum.is_nan();
    for i in first + 1..len {
        let old = combined[i - wma_period];
        let new = combined[i];
        if dirty || old.is_nan() || new.is_nan() {
            let start = i + 1 - wma_period;
            let window = &combined[start..=i];
            window_sum = window.iter().sum();
            weighted_sum = window
                .iter()
                .enumerate()
                .map(|(j, &value)| (j + 1) as f64 * value)
                .sum();
            dirty = window_sum.is_nan();
        } else {
            weighted_sum += wma_period as f64 * new - window_sum;
            window_sum += new - old;
        }
        output[i] = weighted_sum * inverse_weight_sum;
    }
    output
}

fn time(mut f: impl FnMut() -> Array1<f64>, duration: Duration) -> f64 {
    let started = Instant::now();
    let mut calls = 0u64;
    while started.elapsed() < duration {
        black_box(f());
        calls += 1;
    }
    started.elapsed().as_secs_f64() * 1e6 / calls as f64
}

fn talib(input: &[f64]) -> Array1<f64> {
    let len = input.len();
    let mut output = vec![0.0; len];
    let (mut begin, mut count) = (0, 0);
    let status = unsafe {
        TA_COPPOCK(
            0,
            len as i32 - 1,
            input.as_ptr(),
            10,
            11,
            14,
            &mut begin,
            &mut count,
            output.as_mut_ptr(),
        )
    };
    assert_eq!(status, TA_SUCCESS);
    let begin = begin as usize;
    let count = count as usize;
    output.copy_within(0..count, begin);
    output[..begin].fill(f64::NAN);
    output[begin + count..].fill(f64::NAN);
    Array1::from_vec(output)
}

fn main() {
    unsafe { TA_Initialize() };
    for len in [1_000usize, 100_000, 1_000_000] {
        let close = close_data(len);
        let old = || old_batch(&close, 10, 11, 14);
        let new = || coppock(&close, 10, 11, 14).unwrap();
        let fixed = || coppock_ring_fixed::<10, 11, 14>(&close);
        let preflight = || coppock_ring_preflight(&close).unwrap();
        let fused_check = || coppock_ring_fused_check(&close).unwrap();
        let c = || talib(&close);
        let expected = old();
        let actual = new();
        let fixed_result = fixed();
        let preflight_result = preflight();
        let fused_result = fused_check();
        let c_expected = c();
        for (i, (&left, &right)) in expected.iter().zip(actual.iter()).enumerate() {
            assert_eq!(left.is_nan(), right.is_nan(), "NaN mask at {i}");
            if left.is_finite() {
                assert_eq!(left.to_bits(), right.to_bits(), "value at {i}");
            }
        }
        for (i, (&left, &right)) in actual.iter().zip(c_expected.iter()).enumerate() {
            assert_eq!(left.is_nan(), right.is_nan(), "TA-Lib NaN mask at {i}");
            if left.is_finite() {
                assert!(
                    (left - right).abs() <= 1e-8,
                    "TA-Lib value at {i}: {left} != {right}"
                );
            }
        }
        for (index, (&actual, &fixed)) in actual.iter().zip(fixed_result.iter()).enumerate() {
            assert_eq!(
                actual.is_nan(),
                fixed.is_nan(),
                "fixed-ring mask at {index}"
            );
            if actual.is_finite() {
                assert_eq!(
                    actual.to_bits(),
                    fixed.to_bits(),
                    "fixed-ring value at {index}"
                );
            }
        }
        for (index, (&actual, &preflight)) in actual.iter().zip(preflight_result.iter()).enumerate()
        {
            assert_eq!(
                actual.is_nan(),
                preflight.is_nan(),
                "preflight mask at {index}"
            );
            if actual.is_finite() {
                assert_eq!(
                    actual.to_bits(),
                    preflight.to_bits(),
                    "preflight value at {index}"
                );
            }
        }
        for (index, (&actual, &fused)) in actual.iter().zip(fused_result.iter()).enumerate() {
            assert_eq!(
                actual.is_nan(),
                fused.is_nan(),
                "fused-check mask at {index}"
            );
            if actual.is_finite() {
                assert_eq!(
                    actual.to_bits(),
                    fused.to_bits(),
                    "fused-check value at {index}"
                );
            }
        }
        let (
            mut old_samples,
            mut new_samples,
            mut fixed_samples,
            mut preflight_samples,
            mut c_samples,
            mut fused_samples,
        ) = (
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        for turn in 0..7 {
            if turn % 2 == 0 {
                old_samples.push(time(&mut { || old() }, Duration::from_millis(120)));
                new_samples.push(time(&mut { || new() }, Duration::from_millis(120)));
                fixed_samples.push(time(&mut { || fixed() }, Duration::from_millis(120)));
                preflight_samples.push(time(&mut { || preflight() }, Duration::from_millis(120)));
                fused_samples.push(time(&mut { || fused_check() }, Duration::from_millis(120)));
                c_samples.push(time(&mut { || c() }, Duration::from_millis(120)));
            } else {
                c_samples.push(time(&mut { || c() }, Duration::from_millis(120)));
                fused_samples.push(time(&mut { || fused_check() }, Duration::from_millis(120)));
                preflight_samples.push(time(&mut { || preflight() }, Duration::from_millis(120)));
                fixed_samples.push(time(&mut { || fixed() }, Duration::from_millis(120)));
                new_samples.push(time(&mut { || new() }, Duration::from_millis(120)));
                old_samples.push(time(&mut { || old() }, Duration::from_millis(120)));
            }
        }
        old_samples.sort_by(f64::total_cmp);
        new_samples.sort_by(f64::total_cmp);
        fixed_samples.sort_by(f64::total_cmp);
        preflight_samples.sort_by(f64::total_cmp);
        fused_samples.sort_by(f64::total_cmp);
        c_samples.sort_by(f64::total_cmp);
        println!(
            "COPPOCK n={len}: batch={:.2}us ring={:.2}us fixed={:.2}us preflight={:.2}us fused-check={:.2}us TA-Lib={:.2}us ring/batch={:.3}x fixed/ring={:.3}x fused/TA-Lib={:.3}x exact={}",
            old_samples[3],
            new_samples[3],
            fixed_samples[3],
            preflight_samples[3],
            fused_samples[3],
            c_samples[3],
            old_samples[3] / new_samples[3],
            new_samples[3] / fixed_samples[3],
            fused_samples[3] / c_samples[3],
            actual.iter().zip(c_expected.iter()).all(|(a,b)| {
                if a.is_nan() { b.is_nan() } else { (a-b).abs() <= 1e-8 }
            })
        );
    }
    for (index, value) in [
        (100, 0.0),
        (200, 5e-9),
        (300, f64::NAN),
        (400, f64::INFINITY),
        (500, 1e151),
    ] {
        let mut input = close_data(2_048);
        input[index] = value;
        assert!(
            coppock_ring_fused_check(&input).is_none(),
            "fused checker must decline edge input at {index}"
        );
        let actual = coppock(&input, 10, 11, 14).unwrap();
        let expected = if index == 100 {
            talib(&input)
        } else {
            old_batch(&input, 10, 11, 14)
        };
        for (offset, (&actual, &expected)) in actual.iter().zip(expected.iter()).enumerate() {
            assert_eq!(
                actual.is_nan(),
                expected.is_nan(),
                "edge mask {index}/{offset}"
            );
            if actual.is_finite() {
                assert!(
                    (actual - expected).abs() <= 1e-8,
                    "edge value {index}/{offset}: {actual} != {expected}"
                );
            }
        }
    }
    println!("COPPOCK edge inputs: exact-zero matches TA-Lib; near-zero, NaN, Inf, and extreme-input fallback matches the prior reference");
    unsafe { TA_Shutdown() };
}
