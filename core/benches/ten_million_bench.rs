//! 10M-scale throughput baseline (P-8).
//!
//! Establishes a 10-million-bar performance floor for the core watch-list
//! indicators so that future regressions are detectable from CI artifacts.
//!
//! These benchmarks are *expensive* (each iteration processes 80 MB of
//! `f64`s), so the Criterion sample size is set low and only one or two
//! measurements are taken per configuration.  This is intentional — the
//! goal is to record a stable ns/bar SLA, not a high-resolution
//! distribution.
//!
//! Run with:
//! ```bash
//! cargo bench -p alpha_ta-core --bench ten_million_bench -- --warm-up-time=1 --measurement-time=3 --sample-size=10
//! ```
//!
//! Output is also written to `docs/benchmark-baseline.json` for the
//! perf-gate CI to ingest.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use alpha_ta_core::indicators;
use alpha_ta_core::math::simd_kernels as k;

const N_TEN_MILLION: usize = 10_000_000;

fn gen_data_10m() -> Vec<f64> {
    (0..N_TEN_MILLION)
        .map(|i| {
            let t = i as f64;
            100.0 + t * 0.01 + (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5
        })
        .collect()
}

fn gen_ohlcv_10m() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let close = gen_data_10m();
    let mut open = Vec::with_capacity(N_TEN_MILLION);
    let mut high = Vec::with_capacity(N_TEN_MILLION);
    let mut low = Vec::with_capacity(N_TEN_MILLION);
    let mut vol = Vec::with_capacity(N_TEN_MILLION);
    for (i, c) in close.iter().enumerate() {
        let t = i as f64;
        open.push(c - 0.3);
        high.push(c + 1.0 + (t * 0.7).sin().abs() * 0.5);
        low.push(c - 1.0 - (t * 0.5).cos().abs() * 0.5);
        vol.push(10_000.0 + (t * 10.0).sin() * 3000.0);
    }
    (open, high, low, close, vol)
}

fn bench_sma_10m(c: &mut Criterion) {
    let data = gen_data_10m();
    let period = 20usize;
    let mut g = c.benchmark_group("10m_sma_simd");
    g.throughput(Throughput::Elements(N_TEN_MILLION as u64));
    g.bench_with_input(
        BenchmarkId::from_parameter(N_TEN_MILLION),
        &N_TEN_MILLION,
        |b, _| {
            let mut out = vec![0.0; N_TEN_MILLION];
            b.iter(|| k::sma_simd_into(black_box(&data), period, &mut out))
        },
    );
    g.finish();
}

fn bench_ema_10m(c: &mut Criterion) {
    let data = gen_data_10m();
    let period = 26usize;
    let mut g = c.benchmark_group("10m_ema_simd");
    g.throughput(Throughput::Elements(N_TEN_MILLION as u64));
    g.bench_with_input(
        BenchmarkId::from_parameter(N_TEN_MILLION),
        &N_TEN_MILLION,
        |b, _| {
            let mut out = vec![0.0; N_TEN_MILLION];
            b.iter(|| k::ema_simd_into(black_box(&data), period, &mut out))
        },
    );
    g.finish();
}

fn bench_rsi_10m(c: &mut Criterion) {
    let data = gen_data_10m();
    let period = 14usize;
    let mut g = c.benchmark_group("10m_rsi_simd");
    g.throughput(Throughput::Elements(N_TEN_MILLION as u64));
    g.bench_with_input(
        BenchmarkId::from_parameter(N_TEN_MILLION),
        &N_TEN_MILLION,
        |b, _| {
            b.iter(|| k::rsi_simd(black_box(&data), period))
        },
    );
    g.finish();
}

fn bench_macd_10m(c: &mut Criterion) {
    let data = gen_data_10m();
    let mut g = c.benchmark_group("10m_macd");
    g.throughput(Throughput::Elements(N_TEN_MILLION as u64));
    g.bench_with_input(
        BenchmarkId::from_parameter(N_TEN_MILLION),
        &N_TEN_MILLION,
        |b, _| b.iter(|| k::macd_simd(black_box(&data), 12, 26, 9)),
    );
    g.finish();
}

fn bench_bbands_10m(c: &mut Criterion) {
    let data = gen_data_10m();
    let mut g = c.benchmark_group("10m_bbands");
    g.throughput(Throughput::Elements(N_TEN_MILLION as u64));
    g.bench_with_input(
        BenchmarkId::from_parameter(N_TEN_MILLION),
        &N_TEN_MILLION,
        |b, _| b.iter(|| indicators::bbands(black_box(&data), 20, 2.0, 2.0).unwrap()),
    );
    g.finish();
}

fn bench_atr_10m(c: &mut Criterion) {
    let (_open, high, low, close, _vol) = gen_ohlcv_10m();
    let mut g = c.benchmark_group("10m_atr");
    g.throughput(Throughput::Elements(N_TEN_MILLION as u64));
    g.bench_with_input(
        BenchmarkId::from_parameter(N_TEN_MILLION),
        &N_TEN_MILLION,
        |b, _| {
            b.iter(|| {
                indicators::atr(black_box(&high), black_box(&low), black_box(&close), 14).unwrap()
            })
        },
    );
    g.finish();
}

fn bench_obv_10m(c: &mut Criterion) {
    let (_open, _high, _low, close, vol) = gen_ohlcv_10m();
    let mut g = c.benchmark_group("10m_obv");
    g.throughput(Throughput::Elements(N_TEN_MILLION as u64));
    g.bench_with_input(
        BenchmarkId::from_parameter(N_TEN_MILLION),
        &N_TEN_MILLION,
        |b, _| b.iter(|| indicators::obv(black_box(&close), black_box(&vol))),
    );
    g.finish();
}

criterion_group!(
    benches_10m,
    bench_sma_10m,
    bench_ema_10m,
    bench_rsi_10m,
    bench_macd_10m,
    bench_bbands_10m,
    bench_atr_10m,
    bench_obv_10m,
);
criterion_main!(benches_10m);
