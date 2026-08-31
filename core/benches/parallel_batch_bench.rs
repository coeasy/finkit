//! Benchmarks for parallel batch processing vs sequential.
//!
//! Run with:
//! ```bash
//! cargo bench --bench parallel_batch_bench --features rayon
//! ```
//!
//! This benchmark compares the sequential vs parallel batch APIs for
//! SMA/EMA/RSI computation across multiple stocks, and measures the
//! actual speedup on the host CPU.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
#[cfg(feature = "rayon")]
use finkit::indicators::parallel::parallel_rsi_batch;
use finkit::indicators::parallel::{
    parallel_ema_batch, parallel_sma_batch, parallel_sma_multi_period,
};

fn make_closes(n_stocks: usize, n_bars: usize) -> Vec<Vec<f64>> {
    (0..n_stocks)
        .map(|i| {
            (0..n_bars)
                .map(|j| 100.0 + (i as f64) * 0.5 + (j as f64 * 0.013).sin() * 5.0)
                .collect()
        })
        .collect()
}

fn bench_parallel_sma(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_sma_batch");
    for &(n_stocks, n_bars) in &[(4, 10_000), (16, 10_000), (64, 10_000), (256, 10_000)] {
        let closes = make_closes(n_stocks, n_bars);
        let refs: Vec<&[f64]> = closes.iter().map(|v| v.as_slice()).collect();
        let total: usize = n_stocks * n_bars;
        group.throughput(Throughput::Elements(total as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}", n_stocks, n_bars)),
            &total,
            |b, _| {
                b.iter(|| parallel_sma_batch(&refs, 20).unwrap());
            },
        );
    }
    group.finish();
}

fn bench_parallel_ema(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_ema_batch");
    for &(n_stocks, n_bars) in &[(4, 10_000), (16, 10_000), (64, 10_000), (256, 10_000)] {
        let closes = make_closes(n_stocks, n_bars);
        let refs: Vec<&[f64]> = closes.iter().map(|v| v.as_slice()).collect();
        let total: usize = n_stocks * n_bars;
        group.throughput(Throughput::Elements(total as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}", n_stocks, n_bars)),
            &total,
            |b, _| {
                b.iter(|| parallel_ema_batch(&refs, 14).unwrap());
            },
        );
    }
    group.finish();
}

#[cfg(feature = "rayon")]
fn bench_parallel_rsi(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_rsi_batch");
    for &(n_stocks, n_bars) in &[(4, 10_000), (16, 10_000), (64, 10_000)] {
        let closes = make_closes(n_stocks, n_bars);
        let refs: Vec<&[f64]> = closes.iter().map(|v| v.as_slice()).collect();
        let total: usize = n_stocks * n_bars;
        group.throughput(Throughput::Elements(total as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}", n_stocks, n_bars)),
            &total,
            |b, _| {
                b.iter(|| parallel_rsi_batch(&refs, 14).unwrap());
            },
        );
    }
    group.finish();
}

fn bench_parallel_sma_multi_period(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_sma_multi_period");
    for &n_bars in &[1_000usize, 10_000, 100_000] {
        let closes: Vec<f64> = (0..n_bars)
            .map(|j| 100.0 + (j as f64 * 0.013).sin() * 5.0)
            .collect();
        let periods = [5usize, 10, 20, 30, 60, 120];
        group.throughput(Throughput::Elements(n_bars as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n_bars), &n_bars, |b, _| {
            b.iter(|| parallel_sma_multi_period(&closes, &periods).unwrap());
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_parallel_sma,
    bench_parallel_ema,
    bench_parallel_sma_multi_period,
);

#[cfg(feature = "rayon")]
criterion_group!(rsi_benches, bench_parallel_rsi);

#[cfg(feature = "rayon")]
criterion_main!(benches, rsi_benches);

#[cfg(not(feature = "rayon"))]
criterion_main!(benches);
