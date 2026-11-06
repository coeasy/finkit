use finkit::indicators::talib_ext::cvi;
use finkit::math::moving_avg::{ema, simd_horizontal_sum};
use finkit::talib_ffi::{TA_Initialize, TA_Shutdown, TA_SUCCESS};
use ndarray::Array1;
use std::hint::black_box;
use std::time::{Duration, Instant};

unsafe extern "C" {
    fn TA_CVI(
        start_idx: i32,
        end_idx: i32,
        high: *const f64,
        low: *const f64,
        period: i32,
        roc_period: i32,
        out_begin: *mut i32,
        out_count: *mut i32,
        output: *mut f64,
    ) -> i32;
}

fn inputs(len: usize) -> (Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    for i in 0..len {
        let x = i as f64;
        let close = 100.0 + 0.006 * x + (x * 0.017).sin() * 2.1;
        high.push(close + 0.7 + (x * 0.023).sin().abs() * 0.3);
        low.push(close - 0.8 - (x * 0.019).cos().abs() * 0.2);
    }
    (high, low)
}

fn old_cvi(high: &[f64], low: &[f64], period: usize, roc_period: usize) -> Array1<f64> {
    let range: Vec<_> = high.iter().zip(low).map(|(h, l)| h - l).collect();
    let average = ema(&range, period).unwrap();
    let mut output = Array1::from_elem(high.len(), f64::NAN);
    for i in period - 1 + roc_period..high.len() {
        let previous = average[i - roc_period];
        output[i] = if previous == 0.0 {
            0.0
        } else {
            100.0 * (average[i] - previous) / previous
        };
    }
    output
}

#[expect(clippy::uninit_vec)]
fn fused_candidate(high: &[f64], low: &[f64], period: usize, roc_period: usize) -> Array1<f64> {
    let len = high.len();
    let mut seed_ranges = Vec::with_capacity(period);
    for i in 0..period {
        seed_ranges.push(high[i] - low[i]);
    }
    let mut average = simd_horizontal_sum(&seed_ranges) / period as f64;
    let alpha = 2.0 / (period as f64 + 1.0);
    #[cfg(target_arch = "x86_64")]
    let use_fma = std::is_x86_feature_detected!("avx512f")
        || (std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma"));
    #[cfg(not(target_arch = "x86_64"))]
    let use_fma = false;

    let mut ring = vec![0.0; roc_period];
    ring[0] = average;
    let mut ring_len = 1usize;
    let mut ring_head = 0usize;
    let first_output = period - 1 + roc_period;
    let mut output = Vec::<f64>::with_capacity(len);
    unsafe { output.set_len(len) };
    output[..first_output].fill(f64::NAN);

    for i in period..len {
        let range = high[i] - low[i];
        average = if use_fma {
            (range - average).mul_add(alpha, average)
        } else {
            (range - average) * alpha + average
        };

        if ring_len < roc_period {
            ring[(ring_head + ring_len) % roc_period] = average;
            ring_len += 1;
        } else {
            let previous = ring[ring_head];
            if i >= first_output {
                output[i] = if previous == 0.0 {
                    0.0
                } else {
                    100.0 * (average - previous) / previous
                };
            }
            ring[ring_head] = average;
            ring_head += 1;
            if ring_head == roc_period {
                ring_head = 0;
            }
        }
    }
    Array1::from_vec(output)
}

fn talib_cvi(high: &[f64], low: &[f64], period: usize, roc_period: usize) -> Array1<f64> {
    let len = high.len();
    let mut output = vec![0.0; len];
    let (mut begin, mut count) = (0, 0);
    let status = unsafe {
        TA_CVI(
            0,
            (len - 1) as i32,
            high.as_ptr(),
            low.as_ptr(),
            period as i32,
            roc_period as i32,
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

fn time(mut operation: impl FnMut() -> Array1<f64>, budget: Duration) -> f64 {
    let started = Instant::now();
    let mut count = 0u64;
    while started.elapsed() < budget {
        black_box(operation());
        count += 1;
    }
    started.elapsed().as_secs_f64() * 1e6 / count as f64
}

fn median(mut samples: [f64; 5]) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[2]
}

fn main() {
    unsafe { TA_Initialize() };
    for len in [1_000usize, 100_000, 1_000_000] {
        let (high, low) = inputs(len);
        let old = || old_cvi(&high, &low, 10, 10);
        let current = || cvi(&high, &low, 10, 10).unwrap();
        let fused = || fused_candidate(&high, &low, 10, 10);
        let ta = || talib_cvi(&high, &low, 10, 10);
        let expected = old();
        let current_values = current();
        let fused_values = fused();
        let ta_values = ta();
        for i in 0..len {
            assert_eq!(
                expected[i].is_nan(),
                current_values[i].is_nan(),
                "current mask {len}/{i}"
            );
            assert_eq!(
                expected[i].is_nan(),
                fused_values[i].is_nan(),
                "fused mask {len}/{i}"
            );
            assert_eq!(
                expected[i].is_nan(),
                ta_values[i].is_nan(),
                "TA-Lib mask {len}/{i}"
            );
            if expected[i].is_finite() {
                assert_eq!(
                    expected[i].to_bits(),
                    fused_values[i].to_bits(),
                    "fused value {len}/{i}"
                );
                assert!(
                    (expected[i] - current_values[i]).abs() <= 1e-8,
                    "current value {len}/{i}"
                );
                assert!(
                    (expected[i] - ta_values[i]).abs() <= 1e-8,
                    "TA-Lib value {len}/{i}"
                );
            }
        }
        if len == 100_000 {
            let mut bad_seed = high.clone();
            bad_seed[3] = f64::NAN;
            assert!(cvi(&bad_seed, &low, 10, 10).is_err());
            let mut bad_recurrence = high.clone();
            bad_recurrence[50_000] = f64::INFINITY;
            assert!(cvi(&bad_recurrence, &low, 10, 10).is_err());

            let flat_high = vec![1.0; len];
            let flat_low = vec![1.0; len];
            let flat = cvi(&flat_high, &flat_low, 10, 10).unwrap();
            let flat = flat.as_slice().unwrap();
            assert!(flat[..19].iter().all(|value| value.is_nan()));
            assert!(flat[19..].iter().all(|&value| value == 0.0));
        }

        let mut old_samples = [0.0; 5];
        let mut current_samples = [0.0; 5];
        let mut fused_samples = [0.0; 5];
        let mut ta_samples = [0.0; 5];
        for turn in 0..5 {
            if turn % 2 == 0 {
                old_samples[turn] = time(|| old(), Duration::from_millis(80));
                current_samples[turn] = time(|| current(), Duration::from_millis(80));
                fused_samples[turn] = time(|| fused(), Duration::from_millis(80));
                ta_samples[turn] = time(|| ta(), Duration::from_millis(80));
            } else {
                ta_samples[turn] = time(|| ta(), Duration::from_millis(80));
                fused_samples[turn] = time(|| fused(), Duration::from_millis(80));
                current_samples[turn] = time(|| current(), Duration::from_millis(80));
                old_samples[turn] = time(|| old(), Duration::from_millis(80));
            }
        }
        let old_us = median(old_samples);
        let current_us = median(current_samples);
        let fused_us = median(fused_samples);
        let ta_us = median(ta_samples);
        println!(
            "CVI n={len}: old={old_us:.2}us current={current_us:.2}us fused={fused_us:.2}us TA-Lib={ta_us:.2}us current/fused={:.3}x TA-Lib/fused={:.3}x exact=true",
            current_us / fused_us,
            ta_us / fused_us,
        );
    }
    unsafe { TA_Shutdown() };
}
