//! Watch-list self-benchmark: the 9 indicators that historically sit closest
//! to TA-Lib (per docs/BENCHMARK_REPORT.md).
//!
//! This bench does **not** require TA-Lib to be installed — it measures the
//! absolute throughput of the FTA implementations across three data sizes so
//! that we can spot regressions and relative hot spots within the watch list.
//!
//! Run with:
//! ```bash
//! cargo bench -p alpha_ta-core --bench watchlist_self_bench
//! ```

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

use alpha_ta_core::indicators;

const SIZES: &[usize] = &[1_000, 10_000, 100_000];

fn make_ohlcv(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut close = Vec::with_capacity(len);
    let mut vol = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f64;
        let noise =
            (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5 + (t * 3.71).sin() * 0.8;
        let trend = t * 0.01;
        let p = 100.0 + trend + noise;
        open.push(p - 0.3);
        high.push(p + 1.0 + (t * 0.7).sin().abs() * 0.5);
        low.push(p - 1.0 - (t * 0.5).cos().abs() * 0.5);
        close.push(p);
        vol.push(10_000.0 + (t * 10.0).sin() * 3000.0 + 2000.0 * (t * 2.3).cos().abs());
    }
    (open, high, low, close, vol)
}

fn bench_aroon(c: &mut Criterion) {
    let mut g = c.benchmark_group("watchlist_aroon");
    for &n in SIZES {
        let (h, l, _) = {
            let (_open, h, l, c, _v) = make_ohlcv(n);
            (h, l, c)
        };
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(criterion::BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| indicators::aroon(black_box(&h), black_box(&l), 14).unwrap())
        });
    }
    g.finish();
}

fn bench_willr(c: &mut Criterion) {
    let mut g = c.benchmark_group("watchlist_willr");
    for &n in SIZES {
        let (_, h, l, c, _) = make_ohlcv(n);
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(criterion::BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| indicators::willr(black_box(&h), black_box(&l), black_box(&c), 14).unwrap())
        });
    }
    g.finish();
}

fn bench_wma(c: &mut Criterion) {
    let mut g = c.benchmark_group("watchlist_wma");
    for &n in SIZES {
        let (_, _, _, c, _) = make_ohlcv(n);
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(criterion::BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| alpha_ta_core::math::moving_avg::wma(black_box(&c), 20).unwrap())
        });
    }
    g.finish();
}

fn bench_kama(c: &mut Criterion) {
    use alpha_ta_core::math::moving_avg;
    let mut g = c.benchmark_group("watchlist_kama");
    for &n in SIZES {
        let (_, _, _, c, _) = make_ohlcv(n);
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(criterion::BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| moving_avg::kama(black_box(&c), 30, 2, 30).unwrap())
        });
    }
    g.finish();
}

fn bench_mfi(c: &mut Criterion) {
    let mut g = c.benchmark_group("watchlist_mfi");
    for &n in SIZES {
        let (_, h, l, c, v) = make_ohlcv(n);
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(criterion::BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                indicators::mfi(
                    black_box(&h),
                    black_box(&l),
                    black_box(&c),
                    black_box(&v),
                    14,
                )
                .unwrap()
            })
        });
    }
    g.finish();
}

fn bench_stochf(c: &mut Criterion) {
    let mut g = c.benchmark_group("watchlist_stochf");
    for &n in SIZES {
        let (_, h, l, c, _) = make_ohlcv(n);
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(criterion::BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                indicators::stochf(black_box(&h), black_box(&l), black_box(&c), 14, 3)
                    .unwrap()
            })
        });
    }
    g.finish();
}

fn bench_ad(c: &mut Criterion) {
    let mut g = c.benchmark_group("watchlist_ad");
    for &n in SIZES {
        let (_, h, l, c, v) = make_ohlcv(n);
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(criterion::BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                indicators::ad(
                    black_box(&h),
                    black_box(&l),
                    black_box(&c),
                    black_box(&v),
                )
            })
        });
    }
    g.finish();
}

fn bench_adosc(c: &mut Criterion) {
    let mut g = c.benchmark_group("watchlist_adosc");
    for &n in SIZES {
        let (_, h, l, c, v) = make_ohlcv(n);
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(criterion::BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                indicators::adosc(
                    black_box(&h),
                    black_box(&l),
                    black_box(&c),
                    black_box(&v),
                    3,
                    10,
                )
                .unwrap()
            })
        });
    }
    g.finish();
}

fn bench_obv(c: &mut Criterion) {
    let mut g = c.benchmark_group("watchlist_obv");
    for &n in SIZES {
        let (_, _, _, c, v) = make_ohlcv(n);
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(criterion::BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| indicators::obv(black_box(&c), black_box(&v)))
        });
    }
    g.finish();
}

fn bench_rsi(c: &mut Criterion) {
    let mut g = c.benchmark_group("watchlist_rsi");
    for &n in SIZES {
        let (_, _, _, c, _) = make_ohlcv(n);
        let mut out = vec![0.0f64; n];
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(criterion::BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| indicators::rsi_into(black_box(&c), 14, black_box(&mut out)).unwrap())
        });
    }
    g.finish();
}

fn bench_stoch(c: &mut Criterion) {
    let mut g = c.benchmark_group("watchlist_stoch");
    for &n in SIZES {
        let (_, h, l, c, _) = make_ohlcv(n);
        let mut k_out = vec![0.0f64; n];
        let mut d_out = vec![0.0f64; n];
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(criterion::BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                indicators::stoch_into(
                    black_box(&h),
                    black_box(&l),
                    black_box(&c),
                    14,
                    3,
                    3,
                    black_box(&mut k_out),
                    black_box(&mut d_out),
                )
                .unwrap()
            })
        });
    }
    g.finish();
}

fn bench_rsi_scalar(c: &mut Criterion) {
    let mut g = c.benchmark_group("watchlist_rsi_scalar");
    for &n in SIZES {
        let (_, _, _, c, _) = make_ohlcv(n);
        let mut out = vec![0.0f64; n];
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(criterion::BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| alpha_ta_core::math::simd_kernels::rsi_scalar(black_box(&c), 14, black_box(&mut out)))
        });
    }
    g.finish();
}

fn bench_stoch_scalar(c: &mut Criterion) {
    let mut g = c.benchmark_group("watchlist_stoch_scalar");
    for &n in SIZES {
        let (_, h, l, c, _) = make_ohlcv(n);
        let mut k_out = vec![0.0f64; n];
        let mut d_out = vec![0.0f64; n];
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(criterion::BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| {
                alpha_ta_core::math::simd_kernels::stoch_scalar(
                    black_box(&h),
                    black_box(&l),
                    black_box(&c),
                    14,
                    3,
                    3,
                    black_box(&mut k_out),
                    black_box(&mut d_out),
                )
            })
        });
    }
    g.finish();
}

fn bench_cci(c: &mut Criterion) {
    let mut g = c.benchmark_group("watchlist_cci");
    for &n in SIZES {
        let (_, h, l, c, _) = make_ohlcv(n);
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(criterion::BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| indicators::cci(black_box(&h), black_box(&l), black_box(&c), 14).unwrap())
        });
    }
    g.finish();
}

fn bench_stochrsi(c: &mut Criterion) {
    let mut g = c.benchmark_group("watchlist_stochrsi");
    for &n in SIZES {
        let (_o, _h, _l, c, _v) = make_ohlcv(n);
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(criterion::BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| indicators::stochrsi(black_box(&c), 14, 14, 3, 3).unwrap())
        });
    }
    g.finish();
}

criterion_group!(
    benches,
    bench_aroon,
    bench_willr,
    bench_wma,
    bench_kama,
    bench_mfi,
    bench_stochf,
    bench_ad,
    bench_adosc,
    bench_obv,
    bench_rsi,
    bench_stoch,
    bench_rsi_scalar,
    bench_stoch_scalar,
    bench_cci,
    bench_stochrsi,
);
criterion_main!(benches);
