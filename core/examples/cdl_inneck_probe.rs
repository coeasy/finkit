use finkit::patterns::candlestick::in_neck;
use finkit::talib_ffi::TA_CDLINNECK;
use std::hint::black_box;
use std::time::Instant;

fn input(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    for i in 0..len {
        let x = i as f64;
        let c = 100.0 + 0.006 * x + (x * 0.017).sin() * 2.1;
        let o = c - 0.15 + (x * 0.031).sin() * 0.05;
        open.push(o);
        high.push(o.max(c) + 0.7 + (x * 0.023).sin().abs() * 0.3);
        low.push(o.min(c) - 0.8 - (x * 0.019).cos().abs() * 0.2);
        close.push(c);
    }
    (open, high, low, close)
}

fn talib(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> (Vec<i32>, usize, usize) {
    let mut output = vec![0; close.len()];
    let mut begin = 0;
    let mut count = 0;
    // SAFETY: all inputs and output are contiguous and have the same length;
    // the input size fits TA-Lib's i32 interface.
    unsafe {
        TA_CDLINNECK(
            0,
            close.len() as i32 - 1,
            open.as_ptr(),
            high.as_ptr(),
            low.as_ptr(),
            close.as_ptr(),
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
    for len in [1_000, 100_000, 1_000_000] {
        let (open, high, low, close) = input(len);
        let actual = in_neck(&open, &high, &low, &close).unwrap();
        let (raw_expected, begin, count) = talib(&open, &high, &low, &close);
        let mut expected = vec![0; len];
        expected[begin..begin + count].copy_from_slice(&raw_expected[..count]);
        let mismatches = actual.iter().zip(&expected).filter(|(a, b)| a != b).count();

        let iterations = match len {
            0..=1_000 => 512,
            1_001..=100_000 => 16,
            _ => 2,
        };
        let mut rust_times = Vec::with_capacity(10);
        let mut c_times = Vec::with_capacity(10);
        for _ in 0..2 {
            black_box(in_neck(&open, &high, &low, &close).unwrap());
            black_box(talib(&open, &high, &low, &close));
        }
        for round in 0..10 {
            if round % 2 == 0 {
                rust_times.push(time_block(
                    || {
                        black_box(
                            in_neck(
                                black_box(&open),
                                black_box(&high),
                                black_box(&low),
                                black_box(&close),
                            )
                            .unwrap(),
                        );
                    },
                    iterations,
                ));
                c_times.push(time_block(
                    || {
                        black_box(talib(
                            black_box(&open),
                            black_box(&high),
                            black_box(&low),
                            black_box(&close),
                        ));
                    },
                    iterations,
                ));
            } else {
                c_times.push(time_block(
                    || {
                        black_box(talib(
                            black_box(&open),
                            black_box(&high),
                            black_box(&low),
                            black_box(&close),
                        ));
                    },
                    iterations,
                ));
                rust_times.push(time_block(
                    || {
                        black_box(
                            in_neck(
                                black_box(&open),
                                black_box(&high),
                                black_box(&low),
                                black_box(&close),
                            )
                            .unwrap(),
                        );
                    },
                    iterations,
                ));
            }
        }
        let rust_us = median(&mut rust_times);
        let c_us = median(&mut c_times);
        println!(
            "n={len} begin={begin} count={count} mismatches={mismatches} rust={rust_us:.3}us talib={c_us:.3}us speedup={:.3}x",
            c_us / rust_us,
        );
    }
}
