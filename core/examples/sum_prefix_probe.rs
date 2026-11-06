use finkit::indicators::math_operators::sum;
use finkit::talib_ffi::TA_SUM;
use std::hint::black_box;
use std::time::Instant;

fn input(len: usize) -> Vec<f64> {
    (0..len)
        .map(|i| {
            let x = i as f64;
            100.0 + 0.01 * x + (x * 0.37).sin() * 2.0 + (x * 1.13).cos() * 1.5
        })
        .collect()
}

fn talib_sum(data: &[f64], period: i32) -> (Vec<f64>, usize, usize) {
    let mut output = vec![0.0; data.len()];
    let mut begin = 0;
    let mut count = 0;
    // SAFETY: data and output are contiguous, equal-length buffers; the
    // selected lengths and period fit TA-Lib's i32 interface.
    unsafe {
        TA_SUM(
            0,
            data.len() as i32 - 1,
            data.as_ptr(),
            period,
            &mut begin,
            &mut count,
            output.as_mut_ptr(),
        );
    }
    (output, begin as usize, count as usize)
}

fn time_block(mut run: impl FnMut(), iterations: usize) -> f64 {
    let started = Instant::now();
    for _ in 0..iterations {
        run();
    }
    started.elapsed().as_secs_f64() * 1e6 / iterations as f64
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn main() {
    const PERIOD: usize = 30;
    for len in [1_000, 100_000, 1_000_000] {
        let data = input(len);
        let actual = sum(&data, PERIOD).unwrap();
        let (raw_expected, begin, count) = talib_sum(&data, PERIOD as i32);
        let expected = &raw_expected[..count];
        let actual_valid = &actual.as_slice().unwrap()[begin..begin + count];
        let max_abs = actual_valid
            .iter()
            .zip(expected)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f64, f64::max);
        let prefix_ok = actual.as_slice().unwrap()[..begin]
            .iter()
            .all(|value| value.is_nan());

        let iterations = match len {
            0..=1_000 => 4096,
            1_001..=100_000 => 128,
            _ => 16,
        };
        let mut finkit_times = Vec::with_capacity(12);
        let mut talib_times = Vec::with_capacity(12);
        for _ in 0..2 {
            black_box(sum(black_box(&data), PERIOD).unwrap());
            black_box(talib_sum(black_box(&data), PERIOD as i32));
        }
        for round in 0..12 {
            if round % 2 == 0 {
                finkit_times.push(time_block(
                    || {
                        black_box(sum(black_box(&data), PERIOD).unwrap());
                    },
                    iterations,
                ));
                talib_times.push(time_block(
                    || {
                        black_box(talib_sum(black_box(&data), PERIOD as i32));
                    },
                    iterations,
                ));
            } else {
                talib_times.push(time_block(
                    || {
                        black_box(talib_sum(black_box(&data), PERIOD as i32));
                    },
                    iterations,
                ));
                finkit_times.push(time_block(
                    || {
                        black_box(sum(black_box(&data), PERIOD).unwrap());
                    },
                    iterations,
                ));
            }
        }
        let finkit = median(&mut finkit_times);
        let talib = median(&mut talib_times);
        println!(
            "n={len} begin={begin} count={count} prefix_ok={prefix_ok} max_abs={max_abs:.3e} finkit={finkit:.3}us talib={talib:.3}us speedup={:.3}x",
            talib / finkit,
        );
    }
}
