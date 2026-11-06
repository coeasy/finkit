use finkit::indicators::{minus_dm_with_period, plus_dm_with_period};
use std::hint::black_box;
use std::time::{Duration, Instant};

#[cfg(feature = "talib-c")]
use finkit::talib_ffi::{TA_MINUS_DM, TA_PLUS_DM};
#[cfg(feature = "talib-c")]
use std::mem::MaybeUninit;

fn inputs(len: usize) -> (Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    for i in 0..len {
        let x = i as f64;
        let close = 100.0 + 0.006 * x + (x * 0.017).sin() * 2.1;
        let open = close - 0.15 + (x * 0.031).sin() * 0.05;
        high.push(open.max(close) + 0.7 + (x * 0.023).sin().abs() * 0.3);
        low.push(open.min(close) - 0.8 - (x * 0.019).cos().abs() * 0.2);
    }
    (high, low)
}

fn reference<const PLUS: bool>(high: &[f64], low: &[f64], period: usize) -> Vec<f64> {
    let mut output = vec![0.0; high.len()];
    output[..period - 1].fill(f64::NAN);
    let mut smoothed = 0.0;
    let movement = |index: usize| {
        let up = high[index] - high[index - 1];
        let down = low[index - 1] - low[index];
        if PLUS {
            if up > 0.0 && up > down {
                up
            } else {
                0.0
            }
        } else if down > 0.0 && down > up {
            down
        } else {
            0.0
        }
    };
    for index in 1..period {
        smoothed += movement(index);
    }
    output[period - 1] = smoothed;
    let p = period as f64;
    for index in period..high.len() {
        smoothed = smoothed - smoothed / p + movement(index);
        output[index] = smoothed;
    }
    output
}

fn same_bits(left: &[f64], right: &[f64]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(a, b)| (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits())
}

fn close_enough(left: &[f64], right: &[f64]) -> (bool, f64) {
    if left.len() != right.len() {
        return (false, f64::INFINITY);
    }
    let mut max_abs: f64 = 0.0;
    for (a, b) in left.iter().zip(right) {
        if a.is_nan() && b.is_nan() {
            continue;
        }
        if !a.is_finite() || !b.is_finite() {
            return (false, f64::INFINITY);
        }
        let diff = (a - b).abs();
        max_abs = max_abs.max(diff);
        if diff > 1e-8 + 1e-8 * a.abs().max(b.abs()) {
            return (false, max_abs);
        }
    }
    (true, max_abs)
}

#[cfg(feature = "talib-c")]
fn talib_raw<const PLUS: bool>(
    high: &[f64],
    low: &[f64],
    period: i32,
) -> (Vec<MaybeUninit<f64>>, usize, usize) {
    let mut raw = Vec::<MaybeUninit<f64>>::with_capacity(high.len());
    // SAFETY: `MaybeUninit<f64>` permits uninitialized elements, and TA-Lib
    // writes `count` values before any of them are read below.
    unsafe { raw.set_len(high.len()) };
    let mut begin = 0;
    let mut count = 0;
    // SAFETY: inputs and output are contiguous arrays covering the requested range.
    unsafe {
        if PLUS {
            TA_PLUS_DM(
                0,
                (high.len() - 1) as i32,
                high.as_ptr(),
                low.as_ptr(),
                period,
                &mut begin,
                &mut count,
                raw.as_mut_ptr().cast(),
            );
        } else {
            TA_MINUS_DM(
                0,
                (high.len() - 1) as i32,
                high.as_ptr(),
                low.as_ptr(),
                period,
                &mut begin,
                &mut count,
                raw.as_mut_ptr().cast(),
            );
        }
    }
    (raw, begin as usize, count as usize)
}

fn duration_per_call(mut operation: impl FnMut()) -> Duration {
    let target = Duration::from_millis(200);
    let started = Instant::now();
    let mut iterations = 0u32;
    while started.elapsed() < target {
        operation();
        iterations += 1;
    }
    started.elapsed() / iterations
}

fn run<const PLUS: bool>(high: &[f64], low: &[f64], label: &str) {
    let expected = reference::<PLUS>(high, low, 14);
    let actual = if PLUS {
        plus_dm_with_period(high, low, 14).unwrap()
    } else {
        minus_dm_with_period(high, low, 14).unwrap()
    };
    let old_parity = same_bits(actual.as_slice().unwrap(), &expected);
    let (old_close, old_max_abs) = close_enough(actual.as_slice().unwrap(), &expected);
    #[cfg(feature = "talib-c")]
    let c_values = {
        let (c_raw, c_begin, c_count) = talib_raw::<PLUS>(high, low, 14);
        let mut values = vec![f64::NAN; high.len()];
        for (destination, source) in values[c_begin..c_begin + c_count]
            .iter_mut()
            .zip(c_raw.iter().take(c_count))
        {
            // SAFETY: TA-Lib wrote every element in its reported output range.
            *destination = unsafe { source.assume_init() };
        }
        values
    };
    #[cfg(feature = "talib-c")]
    let (c_parity, max_abs) = close_enough(actual.as_slice().unwrap(), &c_values);
    #[cfg(feature = "talib-c")]
    let c_details = format!(" talib_parity={c_parity} max_abs={max_abs:.3e}");
    #[cfg(not(feature = "talib-c"))]
    let c_details = String::new();

    let optimized = duration_per_call(|| {
        if PLUS {
            black_box(plus_dm_with_period(black_box(high), low, 14).unwrap());
        } else {
            black_box(minus_dm_with_period(black_box(high), low, 14).unwrap());
        }
    });
    let reference_time = duration_per_call(|| {
        black_box(reference::<PLUS>(black_box(high), low, 14));
    });
    #[cfg(feature = "talib-c")]
    let talib_time = duration_per_call(|| {
        black_box(talib_raw::<PLUS>(black_box(high), low, 14));
    });
    #[cfg(feature = "talib-c")]
    let talib_details = format!(
        " talib={:.3}us speedup={:.3}x",
        talib_time.as_secs_f64() * 1e6,
        talib_time.as_secs_f64() / optimized.as_secs_f64(),
    );
    #[cfg(not(feature = "talib-c"))]
    let talib_details = String::new();
    println!(
        "{label} n={} bitwise_parity={old_parity} old_close={old_close} old_max_abs={old_max_abs:.3e}{c_details} optimized={:.3}us old={:.3}us old_speedup={:.3}x{talib_details}",
        high.len(),
        optimized.as_secs_f64() * 1e6,
        reference_time.as_secs_f64() * 1e6,
        reference_time.as_secs_f64() / optimized.as_secs_f64(),
    );
}

fn main() {
    for len in [1_000, 100_000, 1_000_000] {
        let (high, low) = inputs(len);
        run::<true>(&high, &low, "PLUS_DM");
        run::<false>(&high, &low, "MINUS_DM");
    }
}
