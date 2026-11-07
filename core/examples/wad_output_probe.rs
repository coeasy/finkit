use finkit::indicators::talib_ext::wad;
use finkit::talib_ffi::{TA_Initialize, TA_Shutdown, TA_SUCCESS};
use ndarray::Array1;
use std::hint::black_box;
use std::time::{Duration, Instant};

unsafe extern "C" {
    fn TA_WAD(
        start_idx: i32,
        end_idx: i32,
        high: *const f64,
        low: *const f64,
        close: *const f64,
        out_begin: *mut i32,
        out_count: *mut i32,
        output: *mut f64,
    ) -> i32;
}

fn data(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    for i in 0..len {
        let x = i as f64;
        let c = 100.0 + 0.006 * x + (x * 0.017).sin() * 2.1;
        let o = c - 0.15 + (x * 0.031).sin() * 0.05;
        high.push(o.max(c) + 0.7 + (x * 0.023).sin().abs() * 0.3);
        low.push(c.min(o) - 0.8 - (x * 0.019).cos().abs() * 0.2);
        close.push(c);
    }
    (high, low, close)
}

#[expect(clippy::uninit_vec)]
fn indexed(high: &[f64], low: &[f64], close: &[f64]) -> Array1<f64> {
    let mut output = Vec::<f64>::with_capacity(close.len());
    // SAFETY: slot 0 and every slot in the recurrence below are written before
    // the vector is converted into an Array1.
    unsafe { output.set_len(close.len()) };
    output[0] = f64::NAN;
    let mut sum = 0.0;
    for i in 1..close.len() {
        let previous = close[i - 1];
        let true_high = high[i].max(previous);
        let true_low = low[i].min(previous);
        if close[i] > previous {
            sum += close[i] - true_low;
        } else if close[i] < previous {
            sum += close[i] - true_high;
        }
        output[i] = sum;
    }
    Array1::from_vec(output)
}

#[expect(clippy::uninit_vec)]
fn zipped(high: &[f64], low: &[f64], close: &[f64]) -> Array1<f64> {
    let mut output = Vec::<f64>::with_capacity(close.len());
    // SAFETY: slot 0 and every slot in the zipped recurrence are written
    // before the vector is converted into an Array1.
    unsafe { output.set_len(close.len()) };
    output[0] = f64::NAN;
    let output_ptr = output.as_mut_ptr();
    let mut sum = 0.0;
    for (index, (((&previous, &current), &current_high), &current_low)) in close
        .iter()
        .zip(&close[1..])
        .zip(&high[1..])
        .zip(&low[1..])
        .enumerate()
    {
        let true_high = current_high.max(previous);
        let true_low = current_low.min(previous);
        if current > previous {
            sum += current - true_low;
        } else if current < previous {
            sum += current - true_high;
        }
        // SAFETY: the probe supplies equal-length slices, and their zipped
        // iteration guarantees `index + 1 < close.len()`.
        unsafe { output_ptr.add(index + 1).write(sum) };
    }
    Array1::from_vec(output)
}

fn talib_wad(high: &[f64], low: &[f64], close: &[f64]) -> Array1<f64> {
    let mut output = vec![0.0; close.len()];
    let (mut begin, mut count) = (0, 0);
    let status = unsafe {
        TA_WAD(
            0,
            (close.len() - 1) as i32,
            high.as_ptr(),
            low.as_ptr(),
            close.as_ptr(),
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

fn time(mut f: impl FnMut() -> Array1<f64>, duration: Duration) -> f64 {
    let started = Instant::now();
    let mut calls = 0u64;
    while started.elapsed() < duration {
        black_box(f());
        calls += 1;
    }
    started.elapsed().as_secs_f64() * 1e6 / calls as f64
}

fn main() {
    unsafe { TA_Initialize() };
    for len in [1_000usize, 100_000, 1_000_000] {
        let (high, low, close) = data(len);
        let before = || wad(&high, &low, &close).unwrap();
        let after = || indexed(&high, &low, &close);
        let zipped_output = || zipped(&high, &low, &close);
        let ta = || talib_wad(&high, &low, &close);
        let expected = before();
        let actual = after();
        let zipped_actual = zipped_output();
        let ta_actual = ta();
        for (i, (&left, &right)) in expected.iter().zip(actual.iter()).enumerate() {
            assert_eq!(left.is_nan(), right.is_nan(), "NaN mask at {i}");
            if !left.is_nan() {
                assert_eq!(left.to_bits(), right.to_bits(), "value at {i}");
            }
        }
        for (i, (&left, &right)) in actual.iter().zip(zipped_actual.iter()).enumerate() {
            assert_eq!(left.is_nan(), right.is_nan(), "zipped NaN mask at {i}");
            if !left.is_nan() {
                assert_eq!(left.to_bits(), right.to_bits(), "zipped value at {i}");
            }
        }
        for (i, (&left, &right)) in actual.iter().zip(ta_actual.iter()).enumerate() {
            // The Python compatibility wrapper explicitly maps WAD's first
            // Finkit NaN slot to zero, matching TA-Lib's no-lookback output.
            let comparable = if i == 0 { 0.0 } else { left };
            assert_eq!(
                comparable.is_nan(),
                right.is_nan(),
                "TA-Lib NaN mask at {i}"
            );
            if !comparable.is_nan() {
                assert!((comparable - right).abs() <= 1e-8, "TA-Lib value at {i}");
            }
        }
        let (mut old_samples, mut new_samples, mut zipped_samples, mut ta_samples) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for turn in 0..7 {
            if turn % 2 == 0 {
                old_samples.push(time(&mut { || before() }, Duration::from_millis(120)));
                new_samples.push(time(&mut { || after() }, Duration::from_millis(120)));
                ta_samples.push(time(&mut { || ta() }, Duration::from_millis(120)));
                zipped_samples.push(time(
                    &mut { || zipped_output() },
                    Duration::from_millis(120),
                ));
            } else {
                ta_samples.push(time(&mut { || ta() }, Duration::from_millis(120)));
                zipped_samples.push(time(
                    &mut { || zipped_output() },
                    Duration::from_millis(120),
                ));
                new_samples.push(time(&mut { || after() }, Duration::from_millis(120)));
                old_samples.push(time(&mut { || before() }, Duration::from_millis(120)));
            }
        }
        old_samples.sort_by(f64::total_cmp);
        new_samples.sort_by(f64::total_cmp);
        zipped_samples.sort_by(f64::total_cmp);
        ta_samples.sort_by(f64::total_cmp);
        println!(
            "WAD {len}: public={:.2}us indexed={:.2}us zipped={:.2}us TA-Lib={:.2}us indexed/zipped={:.3}x TA-Lib/Finkit={:.3}x exact=true",
            old_samples[3],
            new_samples[3],
            zipped_samples[3],
            ta_samples[3],
            new_samples[3] / zipped_samples[3],
            ta_samples[3] / old_samples[3]
        );
    }
    unsafe { TA_Shutdown() };
}
