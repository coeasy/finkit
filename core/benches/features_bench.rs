//! Feature engineering performance benchmarks.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use alpha_ta_core::features::*;
use alpha_ta_core::math::statistics;

const DATA_LEN: usize = 10_000;

fn create_close_data(len: usize) -> Vec<f64> {
    (0..len).map(|i| 100.0 + (i as f64 * 0.01).sin() * 10.0 + i as f64 * 0.001).collect()
}

fn create_volume_data(len: usize) -> Vec<f64> {
    (0..len)
        .map(|i| 10_000.0 + (i as f64 * 10.0).sin() * 3_000.0 + 2_000.0 * (i as f64 * 2.3).cos().abs())
        .collect()
}

fn log_returns_from_close(close: &[f64]) -> Vec<f64> {
    let mut returns = vec![0.0; close.len()];
    for i in 1..close.len() {
        if close[i - 1] > 0.0 && close[i] > 0.0 {
            returns[i] = (close[i] / close[i - 1]).ln();
        }
    }
    returns
}

fn bench_multi_period(c: &mut Criterion) {
    let mut group = c.benchmark_group("features");
    let close = create_close_data(100_000);

    group.bench_function("multi_period_sma_4periods_100k", |b| {
        let gen = MultiPeriodFeature::new("sma".into(), vec![5, 14, 50, 200]);
        b.iter(|| black_box(gen.generate(&close)))
    });

    group.bench_function("multi_period_rsi_3periods_100k", |b| {
        let gen = MultiPeriodFeature::new("rsi".into(), vec![5, 14, 21]);
        b.iter(|| black_box(gen.generate(&close)))
    });

    group.finish();
}

fn bench_rolling_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("features");
    let close = create_close_data(100_000);

    group.bench_function("rolling_skewness_20_100k", |b| {
        b.iter(|| black_box(rolling_skewness(&close, 20)))
    });

    group.bench_function("rolling_kurtosis_20_100k", |b| {
        b.iter(|| black_box(rolling_kurtosis(&close, 20)))
    });

    group.bench_function("rolling_entropy_20_100k", |b| {
        b.iter(|| black_box(rolling_entropy(&close, 20, 10)))
    });

    group.bench_function("rolling_zscore_20_100k", |b| {
        b.iter(|| black_box(rolling_zscore(&close, 20)))
    });

    let close_10k = create_close_data(DATA_LEN);
    let returns = log_returns_from_close(&close_10k);

    group.bench_function("hurst_exponent_10k", |b| {
        b.iter(|| black_box(hurst_exponent(&returns, DEFAULT_HURST_MIN_WINDOW)))
    });

    group.bench_function("acf_lag20_10k", |b| {
        b.iter(|| black_box(acf(&close_10k, 20)))
    });

    group.bench_function("pacf_lag20_10k", |b| {
        b.iter(|| black_box(pacf(&close_10k, 20)))
    });

    group.bench_function("rolling_semivariance_20_10k", |b| {
        b.iter(|| black_box(rolling_semivariance(&close_10k, 20)))
    });

    group.bench_function("rolling_downside_deviation_20_10k", |b| {
        b.iter(|| black_box(rolling_downside_deviation(&close_10k, 20, 0.0)))
    });

    group.finish();
}

fn bench_cross_features(c: &mut Criterion) {
    let mut group = c.benchmark_group("features_cross");
    let close = create_close_data(DATA_LEN);
    let volume = create_volume_data(DATA_LEN);

    group.bench_function("feature_cross_10k", |b| {
        b.iter(|| black_box(feature_cross(&close, &volume)))
    });

    group.bench_function("auto_cross_3cols_10k", |b| {
        let slow: Vec<f64> = close.iter().map(|v| v * 0.99).collect();
        let columns = [("close", close.as_slice()), ("slow", slow.as_slice()), ("volume", volume.as_slice())];
        b.iter(|| black_box(auto_cross(&columns)))
    });

    group.finish();
}

fn bench_microstructure(c: &mut Criterion) {
    let mut group = c.benchmark_group("features_microstructure");
    let close = create_close_data(DATA_LEN);
    let volume = create_volume_data(DATA_LEN);

    group.bench_function("tick_imbalance_20_10k", |b| {
        b.iter(|| black_box(tick_imbalance(&close, 20)))
    });

    group.bench_function("volume_imbalance_20_10k", |b| {
        b.iter(|| black_box(volume_imbalance(&close, &volume, 20)))
    });

    group.bench_function("kyle_lambda_20_10k", |b| {
        b.iter(|| black_box(kyle_lambda(&close, &volume, 20)))
    });

    group.bench_function("roll_spread_20_10k", |b| {
        b.iter(|| black_box(roll_spread(&close, 20)))
    });

    group.finish();
}

fn bench_regime(c: &mut Criterion) {
    let mut group = c.benchmark_group("features_regime");
    let close = create_close_data(DATA_LEN);
    let states = threshold_regime(&close, 20, 25.0, 75.0);

    group.bench_function("threshold_regime_20_10k", |b| {
        b.iter(|| black_box(threshold_regime(&close, 20, 25.0, 75.0)))
    });

    group.bench_function("hmm_regime_2state_10k", |b| {
        b.iter(|| black_box(hmm_regime(&close, 2, 10)))
    });

    group.bench_function("regime_signal_10k", |b| {
        let states_slice = states.as_slice().unwrap();
        b.iter(|| black_box(regime_signal(states_slice)))
    });

    group.finish();
}

fn bench_simd_vs_scalar_rolling_mean(c: &mut Criterion) {
    let mut group = c.benchmark_group("features_simd_vs_scalar");
    let close = create_close_data(100_000);
    let window = 20;

    group.bench_function("rolling_mean_scalar_100k_w20", |b| {
        b.iter(|| black_box(statistics::rolling_mean(&close, window).unwrap()))
    });

    group.bench_function("rolling_mean_simd_100k_w20", |b| {
        b.iter(|| black_box(rolling_mean_simd(&close, window)))
    });

    group.finish();
}

fn rolling_std_scalar_naive(data: &[f64], window: usize) -> Vec<f64> {
    let n = data.len();
    let mut out = vec![f64::NAN; n];
    if window < 2 || window > n {
        return out;
    }
    let inv_w = 1.0 / window as f64;
    let inv_w_minus_1 = 1.0 / (window as f64 - 1.0);
    for i in (window - 1)..n {
        let start = i + 1 - window;
        let slice = &data[start..=i];
        let sum: f64 = slice.iter().sum();
        let mean = sum * inv_w;
        let var: f64 = slice
            .iter()
            .map(|x| {
                let d = x - mean;
                d * d
            })
            .sum::<f64>()
            * inv_w_minus_1;
        out[i] = var.max(0.0).sqrt();
    }
    out
}

fn bench_simd_vs_scalar_rolling_std(c: &mut Criterion) {
    let mut group = c.benchmark_group("features_simd_vs_scalar");
    let close = create_close_data(100_000);
    let window = 20;

    group.bench_function("rolling_std_scalar_100k_w20", |b| {
        b.iter(|| black_box(rolling_std_scalar_naive(&close, window)))
    });

    group.bench_function("rolling_std_simd_100k_w20", |b| {
        b.iter(|| black_box(rolling_std_simd(&close, window)))
    });

    group.finish();
}

fn bench_simd_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("features_simd");
    let close = create_close_data(100_000);

    group.bench_function("batch_zscore_simd_100k", |b| {
        b.iter(|| black_box(batch_zscore_simd(&close)))
    });

    group.bench_function("batch_minmax_simd_100k", |b| {
        b.iter(|| black_box(batch_minmax_simd(&close)))
    });

    let b_data: Vec<f64> = (0..100_000).map(|i| (i as f64 * 0.3).cos() * 5.0).collect();
    group.bench_function("correlation_simd_100k", |b| {
        b.iter(|| black_box(correlation_simd(&close, &b_data)))
    });

    group.finish();
}

fn bench_signals(c: &mut Criterion) {
    let mut group = c.benchmark_group("features");
    let close = create_close_data(100_000);
    let slow: Vec<f64> = close.iter().map(|v| v * 0.99).collect();

    group.bench_function("crossover_signal_100k", |b| {
        b.iter(|| black_box(crossover(&close, &slow)))
    });

    group.bench_function("threshold_signal_100k", |b| {
        b.iter(|| black_box(threshold_cross(&close, 100.0)))
    });

    group.finish();
}

fn bench_labels(c: &mut Criterion) {
    let mut group = c.benchmark_group("features");
    let close = create_close_data(10_000);

    group.bench_function("forward_return_5_10k", |b| {
        b.iter(|| black_box(forward_return(&close, 5)))
    });

    group.bench_function("binary_label_5_10k", |b| {
        b.iter(|| black_box(binary_label(&close, 5, 0.01)))
    });

    let high: Vec<f64> = close.iter().map(|v| v + 1.0).collect();
    let low: Vec<f64> = close.iter().map(|v| v - 1.0).collect();
    group.bench_function("triple_barrier_label_10k", |b| {
        b.iter(|| black_box(triple_barrier(&close, &high, &low, 2.0, 2.0, 20)))
    });

    group.finish();
}

fn bench_kendall_spearman(c: &mut Criterion) {
    let mut group = c.benchmark_group("correlation");
    let close = create_close_data(DATA_LEN);
    let volume = create_volume_data(DATA_LEN);

    group.bench_function("kendall_tau_1000", |b| {
        let x = &close[..1000];
        let y = &volume[..1000];
        b.iter(|| black_box(statistics::kendall_tau(x, y).unwrap()))
    });

    group.bench_function("spearman_rank_1000", |b| {
        let x = &close[..1000];
        let y = &volume[..1000];
        b.iter(|| black_box(statistics::spearman_rank(x, y).unwrap()))
    });

    group.bench_function("rolling_kendall_20_10k", |b| {
        b.iter(|| black_box(rolling_kendall(&close, &volume, 20).unwrap()))
    });

    group.bench_function("rolling_spearman_20_10k", |b| {
        b.iter(|| black_box(rolling_spearman(&close, &volume, 20).unwrap()))
    });

    group.finish();
}

fn bench_fractal_dimension(c: &mut Criterion) {
    let mut group = c.benchmark_group("fractal");
    let close = create_close_data(DATA_LEN);

    group.bench_function("higuchi_fd_1000", |b| {
        let data = &close[..1000];
        b.iter(|| black_box(fractal_dimension_higuchi(data, 10).unwrap()))
    });

    group.bench_function("box_fd_1000", |b| {
        let data = &close[..1000];
        b.iter(|| black_box(fractal_dimension_box(data, 8).unwrap()))
    });

    group.bench_function("rolling_higuchi_20_10k", |b| {
        b.iter(|| black_box(rolling_fractal_dimension_higuchi(&close, 50, 10).unwrap()))
    });

    group.finish();
}

fn bench_wavelet(c: &mut Criterion) {
    let mut group = c.benchmark_group("wavelet");
    let close = create_close_data(DATA_LEN);

    group.bench_function("dwt_haar_1024", |b| {
        let data = &close[..1024];
        b.iter(|| black_box(dwt_features(data, WaveletBasis::Haar, 5).unwrap()))
    });

    group.bench_function("dwt_db4_1024", |b| {
        let data = &close[..1024];
        b.iter(|| black_box(dwt_features(data, WaveletBasis::Db4, 5).unwrap()))
    });

    group.finish();
}

fn bench_fft(c: &mut Criterion) {
    let mut group = c.benchmark_group("fft");
    let close = create_close_data(DATA_LEN);

    group.bench_function("fft_features_1024", |b| {
        let data = &close[..1024];
        b.iter(|| black_box(fft_features(data, 5).unwrap()))
    });

    group.bench_function("rolling_fft_64_10k", |b| {
        b.iter(|| black_box(rolling_fft(&close, 64).unwrap()))
    });

    group.finish();
}

fn bench_rolling_ic(c: &mut Criterion) {
    let mut group = c.benchmark_group("rolling_ic");
    let close = create_close_data(DATA_LEN);
    let returns = log_returns_from_close(&close);

    group.bench_function("rolling_ic_pearson_20_10k", |b| {
        b.iter(|| black_box(rolling_ic(&close, &returns, 20, IcMethod::Pearson).unwrap()))
    });

    group.bench_function("rolling_ic_rank_20_10k", |b| {
        b.iter(|| black_box(rolling_ic(&close, &returns, 20, IcMethod::Rank).unwrap()))
    });

    group.finish();
}

fn bench_complexity(c: &mut Criterion) {
    use alpha_ta_core::features::complexity::*;

    let mut group = c.benchmark_group("complexity");
    let close = create_close_data(1000);

    group.bench_function("approx_entropy_1000", |b| {
        b.iter(|| black_box(approx_entropy(&close, 2, 0.2).unwrap()))
    });

    group.bench_function("sample_entropy_1000", |b| {
        b.iter(|| black_box(sample_entropy(&close, 2, 0.2).unwrap()))
    });

    group.bench_function("dfa_1000", |b| {
        b.iter(|| black_box(dfa(&close, 1).unwrap()))
    });

    group.bench_function("lyapunov_500", |b| {
        let data = &close[..500];
        b.iter(|| black_box(lyapunov_exponent(data, 3, 2, 50).unwrap()))
    });

    group.finish();
}

fn bench_granger(c: &mut Criterion) {
    let mut group = c.benchmark_group("granger");
    let x = create_close_data(200);
    let y: Vec<f64> = x.iter().enumerate().map(|(i, &v)| v * 0.5 + (i as f64 * 0.3).sin()).collect();

    group.bench_function("granger_causality_200_lag2", |b| {
        b.iter(|| black_box(granger_causality(&x, &y, 2).unwrap()))
    });

    group.bench_function("rolling_granger_200_w50_lag2", |b| {
        b.iter(|| black_box(rolling_granger(&x, &y, 50, 2).unwrap()))
    });

    group.finish();
}

fn bench_cross_correlation(c: &mut Criterion) {
    let mut group = c.benchmark_group("cross_correlation");
    let n = 1000;
    let s1 = create_close_data(n);
    let s2: Vec<f64> = s1.iter().map(|v| v * 1.1 + 5.0).collect();
    let s3: Vec<f64> = s1.iter().enumerate().map(|(i, _)| (i as f64 * 0.5).sin() * 50.0).collect();
    let series: Vec<&[f64]> = vec![s1.as_slice(), s2.as_slice(), s3.as_slice()];

    group.bench_function("cross_corr_3x_1000_pearson", |b| {
        b.iter(|| black_box(cross_correlation_matrix(&series, CorrelationMethod::Pearson).unwrap()))
    });

    group.bench_function("cross_corr_3x_1000_spearman", |b| {
        b.iter(|| black_box(cross_correlation_matrix(&series, CorrelationMethod::Spearman).unwrap()))
    });

    group.finish();
}

criterion_group!(
    feature_benches,
    bench_multi_period,
    bench_rolling_stats,
    bench_cross_features,
    bench_microstructure,
    bench_regime,
    bench_simd_vs_scalar_rolling_mean,
    bench_simd_vs_scalar_rolling_std,
    bench_simd_ops,
    bench_signals,
    bench_labels,
    bench_kendall_spearman,
    bench_fractal_dimension,
    bench_wavelet,
    bench_fft,
    bench_rolling_ic,
    bench_complexity,
    bench_granger,
    bench_cross_correlation,
);
criterion_main!(feature_benches);
