//! Trend indicator benchmarks: SuperTrend and Ichimoku Cloud.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use finkit::indicators;

const DATA_LEN: usize = 10_000;

fn create_hlc_data(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f64;
        let noise = (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5;
        let trend = t * 0.01;
        let price = 100.0 + trend + noise;
        close.push(price);
        high.push(price + 1.0 + (t * 0.7).sin().abs() * 0.5);
        low.push(price - 1.0 - (t * 0.5).cos().abs() * 0.5);
    }
    (high, low, close)
}

fn bench_supertrend(c: &mut Criterion) {
    let mut group = c.benchmark_group("supertrend");
    let (high, low, close) = create_hlc_data(DATA_LEN);

    group.bench_function("supertrend_10000", |b| {
        b.iter(|| black_box(indicators::supertrend(&high, &low, &close, 14, 3.0).unwrap()))
    });

    group.finish();
}

fn bench_ichimoku(c: &mut Criterion) {
    let mut group = c.benchmark_group("ichimoku");
    let (high, low, close) = create_hlc_data(DATA_LEN);

    group.bench_function("ichimoku_10000", |b| {
        b.iter(|| black_box(indicators::ichimoku(&high, &low, &close, 9, 26, 52, 26).unwrap()))
    });

    group.finish();
}

criterion_group!(benches, bench_supertrend, bench_ichimoku);
criterion_main!(benches);
