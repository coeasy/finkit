//! Probe: how much of `FLOOR` / `CEIL` is the `libm` call?
//!
//! `llvm.floor.f64` / `llvm.ceil.f64` do **not** lower to an instruction on the
//! baseline x86-64 target — they become out-of-line `libm` calls, one per
//! element. `FLOOR` and `CEIL` are pure element-wise transforms, so that call
//! *is* the indicator, and a `f64::floor`-per-element loop is one call per bar.
//!
//! AVX2 has `vroundpd`: the same IEEE operation, four lanes per instruction, and
//! bit-identical (NaN propagates, `±inf` is fixed, `±0` keeps its sign — this is
//! not an approximation). `math::simd_ops::simd_floor` / `simd_ceil` are that
//! kernel behind a runtime dispatch.
//!
//! Run with: `cargo run --release -p finkit --example rounding_kernel_probe`
//!
//! Reference point for the same 10k-bar series from `benches/talib_c_comparison`
//! (TA-Lib C `TA_FLOOR` / `TA_CEIL`, which also call `floor()`/`ceil()`):
//!     TA_FLOOR  ~25 µs      TA_CEIL  ~30 µs

use std::hint::black_box;
use std::time::Instant;

const LEN: usize = 10_000;
const REPS: usize = 500;

fn series(len: usize) -> Vec<f64> {
    (0..len)
        .map(|i| {
            let t = i as f64;
            100.0 + t * 0.01 + (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5
        })
        .collect()
}

/// `min` of `ROUNDS` repetitions, so a single scheduler hiccup cannot move the
/// number the way a mean would.
fn time_us(mut body: impl FnMut()) -> f64 {
    for _ in 0..20 {
        body();
    }
    let mut best = f64::MAX;
    for _ in 0..5 {
        let start = Instant::now();
        for _ in 0..REPS {
            body();
        }
        let per_call_us = start.elapsed().as_secs_f64() / REPS as f64 * 1e6;
        best = best.min(per_call_us);
    }
    best
}

fn main() {
    let data = series(LEN);
    println!("series length {LEN}, avx2 = {}", avx2_available());
    println!();
    println!("{:<44} {:>10}", "kernel", "us/call");
    println!("{}", "-".repeat(56));

    // The shape the crate used before the SIMD kernels existed: one `f64::floor`
    // call per element, through a collecting iterator.
    let iter_floor = time_us(|| {
        let out: Vec<f64> = data.iter().map(|&x| x.floor()).collect();
        black_box(out);
    });
    let iter_ceil = time_us(|| {
        let out: Vec<f64> = data.iter().map(|&x| x.ceil()).collect();
        black_box(out);
    });
    println!(
        "{:<44} {:>10.2}",
        "collect(map(f64::floor))  <-- was", iter_floor
    );
    println!(
        "{:<44} {:>10.2}",
        "collect(map(f64::ceil))   <-- was", iter_ceil
    );

    let mut out = vec![0.0f64; LEN];
    let simd_floor = time_us(|| {
        finkit::math::simd_ops::simd_floor(&data, &mut out);
        black_box(&out);
    });
    let simd_ceil = time_us(|| {
        finkit::math::simd_ops::simd_ceil(&data, &mut out);
        black_box(&out);
    });
    println!("{:<44} {:>10.2}", "math::simd_ops::simd_floor", simd_floor);
    println!("{:<44} {:>10.2}", "math::simd_ops::simd_ceil", simd_ceil);

    // End to end, i.e. the public indicator the TA-Lib comparison benchmarks:
    // adds the input validation and the `Array1` wrap on top of the kernel.
    let public_floor = time_us(|| {
        black_box(finkit::indicators::floor(&data).unwrap());
    });
    let public_ceil = time_us(|| {
        black_box(finkit::indicators::ceil(&data).unwrap());
    });
    println!(
        "{:<44} {:>10.2}",
        "indicators::floor (public, end to end)", public_floor
    );
    println!(
        "{:<44} {:>10.2}",
        "indicators::ceil  (public, end to end)", public_ceil
    );
    println!();
    println!(
        "floor speedup over the libm loop: {:.1}x",
        iter_floor / simd_floor
    );
    println!(
        "ceil  speedup over the libm loop: {:.1}x",
        iter_ceil / simd_ceil
    );

    // The equivalence claim, checked rather than asserted in prose.
    let mut a = vec![0.0f64; LEN];
    let mut b = vec![0.0f64; LEN];
    finkit::math::simd_ops::simd_floor(&data, &mut a);
    for (slot, &x) in b.iter_mut().zip(data.iter()) {
        *slot = x.floor();
    }
    assert_eq!(a, b, "simd_floor diverged from f64::floor");
    finkit::math::simd_ops::simd_ceil(&data, &mut a);
    for (slot, &x) in b.iter_mut().zip(data.iter()) {
        *slot = x.ceil();
    }
    assert_eq!(a, b, "simd_ceil diverged from f64::ceil");

    // IEEE corner cases, where a polynomial approximation would have failed.
    for probe in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -0.0,
        -0.5,
        -1.0,
        2.5,
        -2.5,
        1e300,
        -1e300,
    ] {
        let mut got = [0.0f64; 1];
        finkit::math::simd_ops::simd_floor(&[probe], &mut got);
        assert_eq!(
            got[0].to_bits(),
            probe.floor().to_bits(),
            "floor({probe}) bit pattern"
        );
        finkit::math::simd_ops::simd_ceil(&[probe], &mut got);
        assert_eq!(
            got[0].to_bits(),
            probe.ceil().to_bits(),
            "ceil({probe}) bit pattern"
        );
    }
    println!("bit-exact against f64::floor / f64::ceil, corner cases included");
}

#[cfg(target_arch = "x86_64")]
fn avx2_available() -> bool {
    is_x86_feature_detected!("avx2")
}

#[cfg(not(target_arch = "x86_64"))]
fn avx2_available() -> bool {
    false
}
