use finkit::indicators::{dx, dx_into};
use finkit::talib_ffi::{TA_Initialize, TA_Shutdown, TA_DX, TA_SUCCESS};
use std::hint::black_box;
use std::time::{Duration, Instant};

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

fn time_per_call(mut operation: impl FnMut(), target: Duration) -> f64 {
    let started = Instant::now();
    let mut calls = 0u64;
    while started.elapsed() < target {
        operation();
        calls += 1;
    }
    started.elapsed().as_secs_f64() * 1e6 / calls as f64
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn previous_dx_into(high: &[f64], low: &[f64], close: &[f64], output: &mut [f64]) {
    const PERIOD: usize = 14;
    output.fill(f64::NAN);
    let p = PERIOD as f64;
    let (mut plus, mut minus, mut tr) = (0.0, 0.0, 0.0);
    for i in 1..PERIOD {
        let up = high[i] - high[i - 1];
        let down = low[i - 1] - low[i];
        tr += finkit::utils::true_range(high[i], low[i], close[i - 1]);
        if up > down && up > 0.0 {
            plus += up;
        }
        if down > up && down > 0.0 {
            minus += down;
        }
    }
    for i in PERIOD..close.len() {
        let up = high[i] - high[i - 1];
        let down = low[i - 1] - low[i];
        let tr_today = finkit::utils::true_range(high[i], low[i], close[i - 1]);
        let pdm = if up > down && up > 0.0 { up } else { 0.0 };
        let mdm = if down > up && down > 0.0 { down } else { 0.0 };
        plus = plus - plus / p + pdm;
        minus = minus - minus / p + mdm;
        tr = tr - tr / p + tr_today;
        if !finkit::utils::is_zero(tr) {
            let pdi = plus / tr * 100.0;
            let mdi = minus / tr * 100.0;
            let sum = pdi + mdi;
            output[i] = if !finkit::utils::is_zero(sum) {
                (pdi - mdi).abs() / sum * 100.0
            } else {
                0.0
            };
        } else {
            output[i] = 0.0;
        }
    }
}

fn main() {
    assert_eq!(unsafe { TA_Initialize() }, TA_SUCCESS);
    for len in [1_000, 100_000, 1_000_000] {
        let (high, low, close) = inputs(len);
        let reference = dx(&high, &low, &close, 14).unwrap();
        let mut candidate = vec![0.0; len];
        dx_into(&high, &low, &close, 14, &mut candidate).unwrap();

        let mut talib_raw = vec![0.0; len];
        let (mut begin, mut count) = (0i32, 0i32);
        let status = unsafe {
            TA_DX(
                0,
                (len - 1) as i32,
                high.as_ptr(),
                low.as_ptr(),
                close.as_ptr(),
                14,
                &mut begin,
                &mut count,
                talib_raw.as_mut_ptr(),
            )
        };
        assert_eq!(status, TA_SUCCESS);
        assert_eq!(begin, 14);
        assert_eq!(count as usize, len - begin as usize);
        let mut talib_output = vec![f64::NAN; len];
        talib_output[begin as usize..begin as usize + count as usize]
            .copy_from_slice(&talib_raw[..count as usize]);
        let (mut reference_close, mut max_ref_abs, mut max_abs, mut max_rel) =
            (true, 0.0f64, 0.0f64, 0.0f64);
        for ((&actual, &expected), &talib) in candidate
            .iter()
            .zip(reference.iter())
            .zip(talib_output.iter())
        {
            if actual.is_nan() && expected.is_nan() && talib.is_nan() {
                continue;
            }
            assert!(actual.is_finite() && expected.is_finite() && talib.is_finite());
            let ref_diff = (actual - expected).abs();
            reference_close &= ref_diff <= 1e-8 + 1e-8 * actual.abs().max(expected.abs());
            max_ref_abs = max_ref_abs.max(ref_diff);
            max_abs = max_abs.max((actual - talib).abs());
            max_rel = max_rel.max((actual - talib).abs() / actual.abs().max(talib.abs()).max(1.0));
        }

        let mut output = vec![0.0; len];
        let previous_us = median(
            (0..7)
                .map(|_| {
                    time_per_call(
                        || {
                            previous_dx_into(black_box(&high), &low, &close, &mut output);
                            black_box(&output);
                        },
                        Duration::from_millis(100),
                    )
                })
                .collect(),
        );
        let candidate_us = median(
            (0..7)
                .map(|_| {
                    time_per_call(
                        || {
                            dx_into(black_box(&high), &low, &close, 14, &mut output).unwrap();
                            black_box(&output);
                        },
                        Duration::from_millis(100),
                    )
                })
                .collect(),
        );
        let talib_us = median(
            (0..7)
                .map(|_| {
                    time_per_call(
                        || unsafe {
                            let mut out_begin = 0;
                            let mut out_count = 0;
                            let result = TA_DX(
                                0,
                                (len - 1) as i32,
                                black_box(high.as_ptr()),
                                low.as_ptr(),
                                close.as_ptr(),
                                14,
                                &mut out_begin,
                                &mut out_count,
                                talib_output.as_mut_ptr().add(out_begin as usize),
                            );
                            assert_eq!(result, TA_SUCCESS);
                            black_box(out_count);
                        },
                        Duration::from_millis(100),
                    )
                })
                .collect(),
        );
        println!(
            "DX n={len}: reference_close={reference_close} max_reference_abs={max_ref_abs:.3e} old={previous_us:.2}us candidate={candidate_us:.2}us candidate_gain={:.3}x TA-Lib={talib_us:.2}us speedup={:.3}x parity_max_abs={max_abs:.3e} parity_max_rel={max_rel:.3e}",
            previous_us / candidate_us,
            talib_us / candidate_us,
        );
    }
    unsafe { TA_Shutdown() };
}
