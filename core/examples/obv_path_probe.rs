//! Compare the installed OBV allocation/dispatch paths at the gate's sizes.
//!
//! Run with: cargo run -p qx-finkit --release --features rayon --example obv_path_probe

use std::hint::black_box;
use std::time::Instant;

fn inputs(n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut close = Vec::with_capacity(n);
    let mut volume = Vec::with_capacity(n);
    for i in 0..n {
        let x = i as f64;
        close.push(100.0 + 0.006 * x + (x * 0.017).sin() * 2.1);
        volume.push(1_000_000.0 + (x % 997.0) * 170.0 + (x * 0.011).sin().abs() * 50_000.0);
    }
    (close, volume)
}

fn median_us<F: FnMut()>(mut f: F, n: usize) -> f64 {
    let loops = (2_000_000 / n).clamp(2, 200);
    let mut samples = Vec::with_capacity(7);
    for _ in 0..7 {
        let started = Instant::now();
        for _ in 0..loops {
            f();
        }
        samples.push(started.elapsed().as_secs_f64() * 1_000_000.0 / loops as f64);
    }
    samples.sort_by(f64::total_cmp);
    samples[3]
}

fn obv_branchless(close: &[f64], volume: &[f64]) -> Vec<f64> {
    let mut output = Vec::with_capacity(close.len());
    let mut acc = volume[0];
    output.push(acc);
    for i in 1..close.len() {
        let direction = (close[i] > close[i - 1]) as i8 - (close[i] < close[i - 1]) as i8;
        acc += volume[i] * direction as f64;
        output.push(acc);
    }
    output
}

fn obv_safe_iter(close: &[f64], volume: &[f64]) -> Vec<f64> {
    let mut output = Vec::with_capacity(close.len());
    let mut acc = volume[0];
    let mut previous = close[0];
    output.push(acc);
    for (&current, &value) in close[1..].iter().zip(&volume[1..]) {
        if current > previous {
            acc += value;
        } else if current < previous {
            acc -= value;
        }
        previous = current;
        output.push(acc);
    }
    output
}

fn obv_simd_fresh(close: &[f64], volume: &[f64]) -> Vec<f64> {
    let mut output = vec![0.0; close.len()];
    finkit::math::simd_ops::simd_obv(close, volume, &mut output);
    output
}

fn obv_uninit_serial(close: &[f64], volume: &[f64]) -> Vec<f64> {
    let mut raw = Vec::<std::mem::MaybeUninit<f64>>::with_capacity(close.len());
    unsafe { raw.set_len(close.len()) };
    let output =
        unsafe { std::slice::from_raw_parts_mut(raw.as_mut_ptr().cast::<f64>(), close.len()) };
    finkit::math::volume_kernels::obv_into(close, volume, output).unwrap();
    let ptr = raw.as_mut_ptr().cast::<f64>();
    let len = raw.len();
    let capacity = raw.capacity();
    std::mem::forget(raw);
    unsafe { Vec::from_raw_parts(ptr, len, capacity) }
}

fn main() {
    for n in [1_000, 100_000, 1_000_000] {
        let (close, volume) = inputs(n);
        let parallel_vec = median_us(
            || {
                black_box(finkit::math::volume_kernels::obv_vec(&close, &volume).unwrap());
            },
            n,
        );
        let serial_public = median_us(
            || {
                black_box(finkit::indicators::obv(&close, &volume).unwrap());
            },
            n,
        );
        let branchless = median_us(
            || {
                black_box(obv_branchless(&close, &volume));
            },
            n,
        );
        let safe_iter = median_us(
            || {
                black_box(obv_safe_iter(&close, &volume));
            },
            n,
        );
        let simd = median_us(
            || {
                black_box(obv_simd_fresh(&close, &volume));
            },
            n,
        );
        let uninit_serial = median_us(
            || {
                black_box(obv_uninit_serial(&close, &volume));
            },
            n,
        );
        let baseline = finkit::math::volume_kernels::obv_vec(&close, &volume).unwrap();
        let simd_output = obv_simd_fresh(&close, &volume);
        let max_abs = baseline
            .iter()
            .zip(&simd_output)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f64, f64::max);
        println!(
            "n={n:>7}: obv_vec={parallel_vec:>9.2}us serial_public={serial_public:>9.2}us uninit_serial={uninit_serial:>9.2}us branchless={branchless:>9.2}us iter={safe_iter:>9.2}us simd={simd:>9.2}us simd_max_abs={max_abs:.3e}",
        );
    }
}
