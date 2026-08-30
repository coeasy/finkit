//! Momentum indicator benchmarks: TTM Squeeze, Vortex, Inertia.

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

fn create_ohlc_data(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let (high, low, close) = create_hlc_data(len);
    let open: Vec<f64> = close.iter().map(|c| c - 0.1).collect();
    (open, high, low, close)
}

fn bench_squeeze(c: &mut Criterion) {
    let mut group = c.benchmark_group("squeeze");
    let (high, low, close) = create_hlc_data(DATA_LEN);

    group.bench_function("ttm_squeeze_10000", |b| {
        b.iter(|| {
            black_box(
                indicators::ttm_squeeze(&high, &low, &close, 20, 2.0, 20, 1.5).unwrap(),
            )
        })
    });

    group.finish();
}

fn bench_vortex(c: &mut Criterion) {
    let mut group = c.benchmark_group("vortex");
    let (high, low, close) = create_hlc_data(DATA_LEN);

    group.bench_function("vortex_14_10000", |b| {
        b.iter(|| black_box(indicators::vortex(&high, &low, &close, 14).unwrap()))
    });

    group.finish();
}

fn bench_inertia(c: &mut Criterion) {
    let mut group = c.benchmark_group("inertia");
    let (open, high, low, close) = create_ohlc_data(DATA_LEN);

    group.bench_function("inertia_10_14_10000", |b| {
        b.iter(|| black_box(indicators::inertia(&open, &high, &low, &close, 10, 14).unwrap()))
    });

    group.finish();
}

fn bench_squeeze_momentum(c: &mut Criterion) {
    let mut group = c.benchmark_group("squeeze_momentum");
    let (_, high, low, close) = create_ohlc_data(DATA_LEN);

    group.bench_function("squeeze_momentum_20_10000", |b| {
        b.iter(|| {
            black_box(indicators::squeeze_momentum(&high, &low, &close, 20, 2.0, 20, 1.5).unwrap())
        })
    });

    group.finish();
}

fn bench_qstick(c: &mut Criterion) {
    let mut group = c.benchmark_group("qstick");
    let (open, _, _, close) = create_ohlc_data(DATA_LEN);

    group.bench_function("qstick_14_10000", |b| {
        b.iter(|| {
            black_box(indicators::qstick(&open, &close, 14, indicators::MaType::Sma).unwrap())
        })
    });

    group.finish();
}

fn bench_cfo(c: &mut Criterion) {
    let mut group = c.benchmark_group("cfo");
    let (_, _, close) = create_hlc_data(DATA_LEN);

    group.bench_function("cfo_14_10000", |b| {
        b.iter(|| black_box(indicators::chande_forecast_oscillator(&close, 14).unwrap()))
    });

    group.finish();
}

criterion_group!(benches, bench_squeeze, bench_vortex, bench_inertia, bench_squeeze_momentum, bench_qstick, bench_cfo);
criterion_main!(benches);
