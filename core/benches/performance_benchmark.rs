use criterion::{black_box, criterion_group, criterion_main, Criterion};
use finkit::indicators;
use finkit::math::moving_avg;

fn generate_test_data(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut close = Vec::with_capacity(n);
    let mut high = Vec::with_capacity(n);
    let mut low = Vec::with_capacity(n);
    let mut open = Vec::with_capacity(n);
    let mut volume = Vec::with_capacity(n);

    let mut price = 100.0;
    for i in 0..n {
        let change = ((i as f64 * 0.1).sin() * 2.0) + ((i as f64 * 0.05).cos() * 1.5);
        price += change;

        let h = price + (i as f64 * 0.03).sin().abs() * 3.0;
        let l = price - (i as f64 * 0.04).cos().abs() * 3.0;
        let o = price + (i as f64 * 0.02).sin() * 1.0;
        let v = 1000000.0 + (i as f64 * 0.1).sin() * 500000.0;

        close.push(price);
        high.push(h);
        low.push(l);
        open.push(o);
        volume.push(v);
    }

    (open, high, low, close, volume)
}

fn benchmark_overlap_indicators(c: &mut Criterion) {
    let (_open, _high, _low, close, _volume) = generate_test_data(10000);

    let mut group = c.benchmark_group("Overlap Indicators");

    group.bench_function("SMA_20", |b| {
        b.iter(|| moving_avg::sma(black_box(&close), 20).unwrap())
    });

    group.bench_function("EMA_20", |b| {
        b.iter(|| moving_avg::ema(black_box(&close), 20).unwrap())
    });

    group.bench_function("WMA_20", |b| {
        b.iter(|| moving_avg::wma(black_box(&close), 20).unwrap())
    });

    group.bench_function("DEMA_20", |b| {
        b.iter(|| moving_avg::dema(black_box(&close), 20).unwrap())
    });

    group.bench_function("TEMA_20", |b| {
        b.iter(|| moving_avg::tema(black_box(&close), 20).unwrap())
    });

    group.bench_function("KAMA_10", |b| {
        b.iter(|| moving_avg::kama(black_box(&close), 10, 2, 30).unwrap())
    });

    group.bench_function("MAMA", |b| {
        b.iter(|| indicators::mama(black_box(&close), 0.5, 0.05).unwrap())
    });

    group.bench_function("T3_20", |b| {
        b.iter(|| indicators::t3(black_box(&close), 20, 0.7).unwrap())
    });

    group.finish();
}

fn benchmark_momentum_indicators(c: &mut Criterion) {
    let (_open, high, low, close, _volume) = generate_test_data(10000);

    let mut group = c.benchmark_group("Momentum Indicators");

    group.bench_function("RSI_14", |b| {
        b.iter(|| indicators::rsi(black_box(&close), 14).unwrap())
    });

    group.bench_function("MACD", |b| {
        b.iter(|| indicators::macd(black_box(&close), 12, 26, 9).unwrap())
    });

    group.bench_function("ADX_14", |b| {
        b.iter(|| indicators::adx(black_box(&high), &low, &close, 14).unwrap())
    });

    group.bench_function("CCI_20", |b| {
        b.iter(|| indicators::cci(black_box(&high), &low, &close, 20).unwrap())
    });

    group.bench_function("WILLR_14", |b| {
        b.iter(|| indicators::willr(black_box(&high), &low, &close, 14).unwrap())
    });

    group.bench_function("AROON_14", |b| {
        b.iter(|| indicators::aroon(black_box(&high), &low, 14).unwrap())
    });

    group.bench_function("MOM_10", |b| {
        b.iter(|| indicators::mom(black_box(&close), 10).unwrap())
    });

    group.bench_function("ROC_10", |b| {
        b.iter(|| indicators::roc(black_box(&close), 10).unwrap())
    });

    group.bench_function("APO", |b| {
        b.iter(|| indicators::apo(black_box(&close), 12, 26).unwrap())
    });

    group.bench_function("PPO", |b| {
        b.iter(|| indicators::ppo(black_box(&close), 12, 26).unwrap())
    });

    group.bench_function("TRIX_30", |b| {
        b.iter(|| indicators::trix(black_box(&close), 30).unwrap())
    });

    group.finish();
}

fn benchmark_volatility_indicators(c: &mut Criterion) {
    let (_open, high, low, close, _volume) = generate_test_data(10000);

    let mut group = c.benchmark_group("Volatility Indicators");

    group.bench_function("BBANDS_20", |b| {
        b.iter(|| indicators::bbands(black_box(&close), 20, 2.0, 2.0).unwrap())
    });

    group.bench_function("ATR_14", |b| {
        b.iter(|| indicators::atr(black_box(&high), &low, &close, 14).unwrap())
    });

    group.bench_function("NATR_14", |b| {
        b.iter(|| indicators::natr(black_box(&high), &low, &close, 14).unwrap())
    });

    group.finish();
}

fn benchmark_volume_indicators(c: &mut Criterion) {
    let (_open, high, low, close, volume) = generate_test_data(10000);

    let mut group = c.benchmark_group("Volume Indicators");

    group.bench_function("AD", |b| {
        b.iter(|| indicators::ad(black_box(&high), &low, &close, &volume).unwrap())
    });

    group.bench_function("ADOSC", |b| {
        b.iter(|| indicators::adosc(black_box(&high), &low, &close, &volume, 3, 10).unwrap())
    });

    group.bench_function("OBV", |b| {
        b.iter(|| indicators::obv(black_box(&close), &volume).unwrap())
    });

    group.finish();
}

fn benchmark_cycle_indicators(c: &mut Criterion) {
    let (_open, _high, _low, close, _volume) = generate_test_data(10000);

    let mut group = c.benchmark_group("Cycle Indicators");

    group.bench_function("HT_DCPERIOD", |b| {
        b.iter(|| indicators::ht_dcperiod(black_box(&close)).unwrap())
    });

    group.bench_function("HT_DCPHASE", |b| {
        b.iter(|| indicators::ht_dcphase(black_box(&close)).unwrap())
    });

    group.bench_function("HT_PHASOR", |b| {
        b.iter(|| indicators::ht_phasor(black_box(&close)).unwrap())
    });

    group.bench_function("HT_SINE", |b| {
        b.iter(|| indicators::ht_sine(black_box(&close)).unwrap())
    });

    group.bench_function("HT_TRENDLINE", |b| {
        b.iter(|| indicators::ht_trendline(black_box(&close)).unwrap())
    });

    group.bench_function("HT_TRENDMODE", |b| {
        b.iter(|| indicators::ht_trendmode(black_box(&close)).unwrap())
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_overlap_indicators,
    benchmark_momentum_indicators,
    benchmark_volatility_indicators,
    benchmark_volume_indicators,
    benchmark_cycle_indicators
);
criterion_main!(benches);
