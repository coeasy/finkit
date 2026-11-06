use finkit::indicators::volume_ext::cmf;
use finkit::talib_ffi::{TA_Initialize, TA_Shutdown, TA_SUCCESS};
use ndarray::Array1;
use std::hint::black_box;
use std::time::{Duration, Instant};

unsafe extern "C" {
    fn TA_CMF(
        start_idx: i32,
        end_idx: i32,
        high: *const f64,
        low: *const f64,
        close: *const f64,
        volume: *const f64,
        period: i32,
        out_begin: *mut i32,
        out_count: *mut i32,
        output: *mut f64,
    ) -> i32;
}

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

fn reference(high: &[f64], low: &[f64], close: &[f64], volume: &[f64], period: usize) -> Vec<f64> {
    let len = high.len();
    let mut output = Vec::<f64>::with_capacity(len);
    unsafe { output.set_len(len) };
    output[..period - 1].fill(f64::NAN);
    let mut ring = vec![(0.0, 0.0); period];
    let (mut sum_flow, mut sum_volume) = (0.0, 0.0);
    let mut null_run = 0usize;
    for i in 0..period {
        let range = high[i] - low[i];
        let flow = if range > 0.0 {
            (((close[i] - low[i]) - (high[i] - close[i])) / range) * volume[i]
        } else {
            0.0
        };
        ring[i] = (flow, volume[i]);
        sum_flow += flow;
        sum_volume += volume[i];
        let zero_volume = usize::from(volume[i] == 0.0);
        null_run = (null_run + zero_volume) * zero_volume;
    }
    output[period - 1] = if sum_volume > 0.0 {
        sum_flow / sum_volume
    } else {
        0.0
    };
    let mut ring_index = 0usize;
    for i in period..len {
        let (old_flow, old_volume) = ring[ring_index];
        sum_flow -= old_flow;
        sum_volume -= old_volume;
        let range = high[i] - low[i];
        let flow = if range > 0.0 {
            (((close[i] - low[i]) - (high[i] - close[i])) / range) * volume[i]
        } else {
            0.0
        };
        ring[ring_index] = (flow, volume[i]);
        sum_flow += flow;
        sum_volume += volume[i];
        let zero_volume = usize::from(volume[i] == 0.0);
        null_run = (null_run + zero_volume) * zero_volume;
        if null_run >= period {
            null_run = period;
            sum_flow = 0.0;
            sum_volume = 0.0;
            output[i] = 0.0;
        } else if sum_volume > 0.0 {
            output[i] = sum_flow / sum_volume;
        } else {
            output[i] = 0.0;
        }
        ring_index += 1;
        if ring_index == period {
            ring_index = 0;
        }
    }
    output
}

fn talib(high: &[f64], low: &[f64], close: &[f64], volume: &[f64], period: usize) -> Array1<f64> {
    let len = high.len();
    let mut output = vec![0.0; len];
    let (mut begin, mut count) = (0, 0);
    let status = unsafe {
        TA_CMF(
            0,
            (len - 1) as i32,
            high.as_ptr(),
            low.as_ptr(),
            close.as_ptr(),
            volume.as_ptr(),
            period as i32,
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

fn time(mut operation: impl FnMut(), budget: Duration) -> f64 {
    let started = Instant::now();
    let mut count = 0u64;
    while started.elapsed() < budget {
        operation();
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
        let (high, low, close, volume) = inputs(len);
        let old = || reference(&high, &low, &close, &volume, 20);
        let new = || cmf(&high, &low, &close, &volume, 20).unwrap();
        let c = || talib(&high, &low, &close, &volume, 20);
        let expected = old();
        let actual = new();
        let c_actual = c();
        for i in 0..len {
            assert_eq!(
                expected[i].is_nan(),
                actual[i].is_nan(),
                "old mask {len}/{i}"
            );
            assert_eq!(
                actual[i].is_nan(),
                c_actual[i].is_nan(),
                "TA-Lib mask {len}/{i}"
            );
            if expected[i].is_finite() {
                assert_eq!(
                    expected[i].to_bits(),
                    actual[i].to_bits(),
                    "old value {len}/{i}"
                );
                assert!(
                    (actual[i] - c_actual[i]).abs() <= 1e-8,
                    "TA-Lib value {len}/{i}"
                );
            }
        }
        let mut old_samples = [0.0; 5];
        let mut new_samples = [0.0; 5];
        let mut ta_samples = [0.0; 5];
        for turn in 0..5 {
            if turn % 2 == 0 {
                old_samples[turn] = time(
                    || {
                        black_box(old());
                    },
                    Duration::from_millis(80),
                );
                new_samples[turn] = time(
                    || {
                        black_box(new());
                    },
                    Duration::from_millis(80),
                );
                ta_samples[turn] = time(
                    || {
                        black_box(c());
                    },
                    Duration::from_millis(80),
                );
            } else {
                ta_samples[turn] = time(
                    || {
                        black_box(c());
                    },
                    Duration::from_millis(80),
                );
                new_samples[turn] = time(
                    || {
                        black_box(new());
                    },
                    Duration::from_millis(80),
                );
                old_samples[turn] = time(
                    || {
                        black_box(old());
                    },
                    Duration::from_millis(80),
                );
            }
        }
        let old_us = median(old_samples);
        let new_us = median(new_samples);
        let ta_us = median(ta_samples);
        println!(
            "CMF n={len}: tuple-reference={old_us:.2}us Finkit={new_us:.2}us TA-Lib={ta_us:.2}us TA-Lib/Finkit={:.3}x reference/Finkit={:.3}x parity=true",
            new_us / ta_us,
            old_us / new_us,
        );
    }
    unsafe { TA_Shutdown() };
}
