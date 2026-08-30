//! Volume indicator benchmarks: VWAP.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use alpha_ta_core::indicators;

const DATA_LEN: usize = 10_000;

fn create_ohlcv_data(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut volume = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f64;
        let noise = (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5;
        let trend = t * 0.01;
        let price = 100.0 + trend + noise;
        close.push(price);
        high.push(price + 1.0 + (t * 0.7).sin().abs() * 0.5);
        low.push(price - 1.0 - (t * 0.5).cos().abs() * 0.5);
        volume.push(10_000.0 + (t * 10.0).sin() * 3_000.0 + 2_000.0 * (t * 2.3).cos().abs());
    }
    (high, low, close, volume)
}

fn bench_vwap(c: &mut Criterion) {
    let mut group = c.benchmark_group("vwap");
    let (high, low, close, volume) = create_ohlcv_data(DATA_LEN);

    group.bench_function("vwap_10000", |b| {
        b.iter(|| black_box(indicators::vwap(&high, &low, &close, &volume).unwrap()))
    });

    group.bench_function("vwap_bands_10000", |b| {
        b.iter(|| black_box(indicators::vwap_bands(&high, &low, &close, &volume, 20, 2.0).unwrap()))
    });

    group.finish();
}

fn bench_twiggs_mf(c: &mut Criterion) {
    let mut group = c.benchmark_group("twiggs_mf");
    let (high, low, close, volume) = create_ohlcv_data(DATA_LEN);

    group.bench_function("twiggs_mf_14_10000", |b| {
        b.iter(|| {
            black_box(indicators::twiggs_money_flow(&high, &low, &close, &volume, 14).unwrap())
        })
    });

    group.finish();
}

fn bench_vzo(c: &mut Criterion) {
    let mut group = c.benchmark_group("vzo");
    let (_, _, close, volume) = create_ohlcv_data(DATA_LEN);

    group.bench_function("vzo_14_10000", |b| {
        b.iter(|| black_box(indicators::vzo(&close, &volume, 14).unwrap()))
    });

    group.finish();
}

fn bench_volume_momentum(c: &mut Criterion) {
    let mut group = c.benchmark_group("volume_momentum");
    let (_, _, _, volume) = create_ohlcv_data(DATA_LEN);

    group.bench_function("volume_momentum_14_10000", |b| {
        b.iter(|| black_box(indicators::volume_momentum(&volume, 14).unwrap()))
    });

    group.bench_function("volume_roc_14_10000", |b| {
        b.iter(|| black_box(indicators::volume_roc(&volume, 14).unwrap()))
    });

    group.finish();
}

fn bench_vwap_mtf(c: &mut Criterion) {
    let mut group = c.benchmark_group("vwap_mtf");
    let (high, low, close, volume) = create_ohlcv_data(DATA_LEN);
    let session_start: Vec<bool> = (0..DATA_LEN).map(|i| i % 78 == 0).collect();

    group.bench_function("vwap_mtf_10000", |b| {
        b.iter(|| {
            black_box(indicators::vwap_mtf(&high, &low, &close, &volume, &session_start).unwrap())
        })
    });

    group.finish();
}

criterion_group!(benches, bench_vwap, bench_twiggs_mf, bench_vzo, bench_volume_momentum, bench_vwap_mtf);
criterion_main!(benches);
