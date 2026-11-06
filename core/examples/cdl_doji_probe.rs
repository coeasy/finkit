use finkit::patterns::candlestick::{dragonfly_doji, gravestone_doji};
use finkit::talib_ffi::{TA_Initialize, TA_Shutdown, TA_CDLDRAGONFLYDOJI, TA_CDLGRAVESTONEDOJI};
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

fn gap_input(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    for i in 0..len {
        // Alternating price gaps keep true range very different from the
        // high-low range used by TA-Lib's default Doji candle settings.
        let base = 100.0 + (i as f64) * 0.001 + if i % 2 == 0 { 5.0 } else { -5.0 };
        let (o, h, l, c) = if i % 23 == 0 {
            (base, base + 1.0, base - 0.01, base)
        } else if i % 19 == 0 {
            (base, base + 0.01, base - 1.0, base)
        } else {
            (base - 0.01, base + 0.2, base - 0.2, base + 0.01)
        };
        open.push(o);
        high.push(h);
        low.push(l);
        close.push(c);
    }
    (open, high, low, close)
}

macro_rules! talib_call {
    ($name:ident, $symbol:ident) => {
        fn $name(
            open: &[f64],
            high: &[f64],
            low: &[f64],
            close: &[f64],
        ) -> (Vec<i32>, usize, usize) {
            let mut output = vec![0; close.len()];
            let mut begin = 0;
            let mut count = 0;
            // SAFETY: all buffers are contiguous, equal length, and fit the
            // signed index types used by the TA-Lib C interface.
            unsafe {
                $symbol(
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
    };
}

talib_call!(talib_dragonfly, TA_CDLDRAGONFLYDOJI);
talib_call!(talib_gravestone, TA_CDLGRAVESTONEDOJI);

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

fn compare(
    name: &str,
    len: usize,
    rust: impl Fn() -> Vec<i32>,
    talib: impl Fn() -> (Vec<i32>, usize, usize),
) {
    let actual = rust();
    let (raw_expected, begin, count) = talib();
    let mut expected = vec![0; len];
    expected[begin..begin + count].copy_from_slice(&raw_expected[..count]);
    let mismatches = actual.iter().zip(&expected).filter(|(a, b)| a != b).count();
    if mismatches != 0 {
        let mut shown = 0;
        for (index, (rust, c)) in actual.iter().zip(&expected).enumerate() {
            if rust != c {
                println!("  mismatch i={index} rust={rust} talib={c}");
                shown += 1;
                if shown == 10 {
                    break;
                }
            }
        }
    }
    assert_eq!(mismatches, 0, "{name} diverged from TA-Lib at n={len}");
    let iterations = match len {
        0..=1_000 => 512,
        1_001..=100_000 => 16,
        _ => 2,
    };
    let mut rust_times = Vec::with_capacity(10);
    let mut c_times = Vec::with_capacity(10);
    for _ in 0..2 {
        black_box(rust());
        black_box(talib());
    }
    for round in 0..10 {
        if round % 2 == 0 {
            rust_times.push(time_block(
                || {
                    black_box(rust());
                },
                iterations,
            ));
            c_times.push(time_block(
                || {
                    black_box(talib());
                },
                iterations,
            ));
        } else {
            c_times.push(time_block(
                || {
                    black_box(talib());
                },
                iterations,
            ));
            rust_times.push(time_block(
                || {
                    black_box(rust());
                },
                iterations,
            ));
        }
    }
    let rust_us = median(&mut rust_times);
    let c_us = median(&mut c_times);
    println!(
        "{name} n={len} mismatches={mismatches} rust={rust_us:.3}us talib={c_us:.3}us speedup={:.3}x",
        c_us / rust_us,
    );
}

fn main() {
    // Restore TA-Lib's default candle settings before using the C reference.
    assert_eq!(unsafe { TA_Initialize() }, 0);
    for len in [1_000, 100_000, 1_000_000] {
        let (open, high, low, close) = input(len);
        compare(
            "CDLDRAGONFLYDOJI",
            len,
            || {
                dragonfly_doji(&open, &high, &low, &close, 0.1)
                    .unwrap()
                    .into_raw_vec_and_offset()
                    .0
            },
            || talib_dragonfly(&open, &high, &low, &close),
        );
        compare(
            "CDLGRAVESTONEDOJI",
            len,
            || {
                gravestone_doji(&open, &high, &low, &close, 0.1)
                    .unwrap()
                    .into_raw_vec_and_offset()
                    .0
            },
            || talib_gravestone(&open, &high, &low, &close),
        );
    }

    let (open, high, low, close) = gap_input(100_000);
    compare(
        "CDLDRAGONFLYDOJI-GAPS",
        close.len(),
        || {
            dragonfly_doji(&open, &high, &low, &close, 0.1)
                .unwrap()
                .into_raw_vec_and_offset()
                .0
        },
        || talib_dragonfly(&open, &high, &low, &close),
    );
    compare(
        "CDLGRAVESTONEDOJI-GAPS",
        close.len(),
        || {
            gravestone_doji(&open, &high, &low, &close, 0.1)
                .unwrap()
                .into_raw_vec_and_offset()
                .0
        },
        || talib_gravestone(&open, &high, &low, &close),
    );
    assert_eq!(unsafe { TA_Shutdown() }, 0);
}
