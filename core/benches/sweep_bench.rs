use criterion::{criterion_group, criterion_main, Criterion};
use finkit::indicators::sweep::{ema_sweep, rsi_sweep, sma_sweep};
use finkit::indicators::sweep_engine::{SweepEngine, ParamRange};
use finkit::indicators::sweepable::SmaSweepable;
use finkit::math::moving_avg::{ema, sma};

fn generate_data(n: usize) -> Vec<f64> {
    let mut data = Vec::with_capacity(n);
    let mut price = 100.0;
    for i in 0..n {
        price += ((i as f64 * 0.1).sin() * 2.0) + 0.01;
        data.push(price);
    }
    data
}

fn bench_sma_sweep(c: &mut Criterion) {
    let data = generate_data(10_000);
    let periods = [5, 10, 20, 50, 100, 200];

    c.bench_function("sma_sweep_6_periods", |b| {
        b.iter(|| sma_sweep(&data, &periods).unwrap())
    });

    c.bench_function("sma_individual_6_periods", |b| {
        b.iter(|| {
            for &p in &periods {
                let _ = sma(&data, p).unwrap();
            }
        })
    });
}

fn bench_ema_sweep(c: &mut Criterion) {
    let data = generate_data(10_000);
    let periods = [5, 10, 20, 50, 100, 200];

    c.bench_function("ema_sweep_6_periods", |b| {
        b.iter(|| ema_sweep(&data, &periods).unwrap())
    });

    c.bench_function("ema_individual_6_periods", |b| {
        b.iter(|| {
            for &p in &periods {
                let _ = ema(&data, p).unwrap();
            }
        })
    });
}

fn bench_rsi_sweep(c: &mut Criterion) {
    let data = generate_data(10_000);
    let periods = [5, 10, 14, 20, 50, 100];

    c.bench_function("rsi_sweep_6_periods", |b| {
        b.iter(|| rsi_sweep(&data, &periods).unwrap())
    });

    c.bench_function("rsi_individual_6_periods", |b| {
        b.iter(|| {
            for &p in &periods {
                let _ = finkit::indicators::momentum::rsi(&data, p).unwrap();
            }
        })
    });
}

fn bench_sweep_engine(c: &mut Criterion) {
    let data = generate_data(50_000);
    let engine = SweepEngine::new();

    c.bench_function("sweep_engine_sma_20_periods", |b| {
        b.iter(|| {
            engine.run(&SmaSweepable, &data, &[ParamRange::new(5, 105, 5)]).unwrap()
        })
    });

    c.bench_function("naive_loop_sma_20_periods", |b| {
        b.iter(|| {
            let periods: Vec<usize> = (1..=20).map(|i| i * 5).collect();
            for &p in &periods {
                let _ = sma(&data, p).unwrap();
            }
        })
    });

    c.bench_function("sweep_engine_sma_50_periods", |b| {
        b.iter(|| {
            engine.run(&SmaSweepable, &data, &[ParamRange::new(2, 52, 1)]).unwrap()
        })
    });

    c.bench_function("naive_loop_sma_50_periods", |b| {
        b.iter(|| {
            for p in 2..52 {
                let _ = sma(&data, p).unwrap();
            }
        })
    });
}

criterion_group!(
    benches,
    bench_sma_sweep,
    bench_ema_sweep,
    bench_rsi_sweep,
    bench_sweep_engine
);
criterion_main!(benches);
