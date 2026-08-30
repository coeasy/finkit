//! Zero-allocation micro-benchmarks: SIMD `_into` variants vs scalar `_into`.
//!
//! These benchmarks exercise the pre-allocated-output versions of the
//! streaming indicators. The `_into` family writes into a caller-provided
//! slice instead of returning a freshly-allocated `Vec`, so the hot path
//! performs **no heap allocation** at all (after the initial setup). This
//! is the path used by the FFI bindings and the JIT formula compiler.
//!
//! Run with:
//! ```bash
//! cargo bench -p finkit --bench zero_alloc_bench
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use finkit::math::simd_kernels as k;

const N_SMALL: usize = 1_000;
const N_MEDIUM: usize = 10_000;
const N_LARGE: usize = 100_000;

fn gen_data(len: usize) -> Vec<f64> {
    (0..len)
        .map(|i| {
            let t = i as f64;
            100.0 + t * 0.01 + (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5
        })
        .collect()
}

fn bench_sma_zero_alloc(c: &mut Criterion) {
    let mut g = c.benchmark_group("zero_alloc_sma");
    for &n in &[N_SMALL, N_MEDIUM, N_LARGE] {
        let data = gen_data(n);
        let period = 14usize;
        let mut out_simd = vec![0.0; n];
        let mut out_scalar = vec![0.0; n];
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(BenchmarkId::new("simd_into", n), &n, |b, _| {
            b.iter(|| k::sma_simd_into(black_box(&data), period, &mut out_simd))
        });
        g.bench_with_input(BenchmarkId::new("scalar_into", n), &n, |b, _| {
            b.iter(|| k::sma_scalar_into(black_box(&data), period, &mut out_scalar))
        });
        g.bench_with_input(BenchmarkId::new("scalar_naive_into", n), &n, |b, _| {
            b.iter(|| {
                k::sma_scalar_naive_into(black_box(&data), period, &mut out_scalar)
            })
        });
    }
    g.finish();
}

fn bench_ema_zero_alloc(c: &mut Criterion) {
    let mut g = c.benchmark_group("zero_alloc_ema");
    for &n in &[N_SMALL, N_MEDIUM, N_LARGE] {
        let data = gen_data(n);
        let period = 20usize;
        let mut out = vec![0.0; n];
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(BenchmarkId::new("simd_into", n), &n, |b, _| {
            b.iter(|| k::ema_simd_into(black_box(&data), period, &mut out))
        });
    }
    g.finish();
}

fn bench_rsi_zero_alloc(c: &mut Criterion) {
    let mut g = c.benchmark_group("zero_alloc_rsi");
    for &n in &[N_SMALL, N_MEDIUM, N_LARGE] {
        let data = gen_data(n);
        let period = 14usize;
        let mut out = vec![0.0; n];
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(BenchmarkId::new("simd_into", n), &n, |b, _| {
            b.iter(|| k::rsi_simd_into(black_box(&data), period, &mut out))
        });
    }
    g.finish();
}

fn bench_macd_zero_alloc(c: &mut Criterion) {
    let mut g = c.benchmark_group("zero_alloc_macd");
    for &n in &[N_SMALL, N_MEDIUM, N_LARGE] {
        let data = gen_data(n);
        let (mut macd_line, mut signal_line, mut hist) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(BenchmarkId::new("simd_into", n), &n, |b, _| {
            b.iter(|| {
                k::macd_simd_into(
                    black_box(&data),
                    12,
                    26,
                    9,
                    &mut macd_line,
                    &mut signal_line,
                    &mut hist,
                )
            })
        });
    }
    g.finish();
}

criterion_group!(
    benches,
    bench_sma_zero_alloc,
    bench_ema_zero_alloc,
    bench_rsi_zero_alloc,
    bench_macd_zero_alloc,
);
criterion_main!(benches);
