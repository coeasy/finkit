//! Benchmark for `ema_multi_periods` vs N×single ema (D.1 / D.7 FMA optimization).
//!
//! Run with: cargo bench -p finkit --bench ema_multi_periods_bench

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use finkit::indicators::ema;
use finkit::math::moving_avg::ema_multi_periods;

fn bench_ema_multi_periods(c: &mut Criterion) {
    let mut group = c.benchmark_group("ema_multi_periods");

    for &size in &[1_000usize, 10_000, 100_000] {
        let data: Vec<f64> = (0..size)
            .map(|i| 100.0 + (i as f64 * 0.013).sin() * 5.0)
            .collect();
        let periods: [usize; 6] = [5, 10, 20, 30, 60, 120];

        // Baseline: 6× single ema (allocates 6 Vec<Array1>)
        group.bench_with_input(BenchmarkId::new("6x_single_ema", size), &size, |b, _| {
            b.iter(|| {
                for &p in &periods {
                    let _ = black_box(ema(&data, p).unwrap());
                }
            })
        });

        // Optimized: one pass, FMA, zero-alloc on the hot path
        group.bench_with_input(
            BenchmarkId::new("ema_multi_periods", size),
            &size,
            |b, _| {
                let mut bufs: Vec<Vec<f64>> = periods.iter().map(|_| vec![0.0; size]).collect();
                b.iter(|| {
                    let mut refs: Vec<&mut [f64]> =
                        bufs.iter_mut().map(|b| b.as_mut_slice()).collect();
                    black_box(ema_multi_periods(&data, &periods, &mut refs).unwrap());
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_ema_multi_periods);
criterion_main!(benches);
