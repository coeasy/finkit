use criterion::{black_box, criterion_group, criterion_main, Criterion};
use finkit::math::simd_kernels;
use finkit::math::simd_ops;
use finkit::formula::SimdOps;

const DATA_LEN: usize = 10_000;
const KERNEL_LEN: usize = 100_000;

fn generate_close_data(len: usize) -> Vec<f64> {
    (0..len)
        .map(|i| {
            let t = i as f64;
            100.0 + t * 0.01 + (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5
        })
        .collect()
}

fn generate_volume_data(len: usize) -> Vec<f64> {
    (0..len)
        .map(|i| 10000.0 + (i as f64 * 10.0).sin() * 3000.0)
        .collect()
}

fn generate_hlc_data(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let close = generate_close_data(len);
    let high: Vec<f64> = close.iter().map(|c| c + 1.5).collect();
    let low: Vec<f64> = close.iter().map(|c| c - 1.5).collect();
    (high, low, close)
}

fn bench_simd_prefix_sum(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    let mut result = vec![0.0; DATA_LEN];
    c.bench_function("simd_prefix_sum_10000", |b| {
        b.iter(|| simd_ops::simd_prefix_sum(black_box(&data), &mut result))
    });
}

fn bench_simd_diff(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    let mut result = vec![0.0; DATA_LEN];
    c.bench_function("simd_diff_10000", |b| {
        b.iter(|| simd_ops::simd_diff(black_box(&data), &mut result))
    });
}

fn bench_simd_scale(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    let mut result = vec![0.0; DATA_LEN];
    c.bench_function("simd_scale_10000", |b| {
        b.iter(|| simd_ops::simd_scale(black_box(&data), 2.5, &mut result))
    });
}

fn bench_simd_pct_change(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    let mut result = vec![0.0; DATA_LEN];
    c.bench_function("simd_pct_change_10000", |b| {
        b.iter(|| simd_ops::simd_pct_change(black_box(&data), &mut result))
    });
}

fn bench_simd_clamp(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    let mut result = vec![0.0; DATA_LEN];
    c.bench_function("simd_clamp_10000", |b| {
        b.iter(|| simd_ops::simd_clamp(black_box(&data), 95.0, 105.0, &mut result))
    });
}

fn bench_simd_weighted_sum(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    let weights: Vec<f64> = (0..DATA_LEN).map(|i| 1.0 / (i as f64 + 1.0)).collect();
    let mut result = vec![0.0; DATA_LEN];
    c.bench_function("simd_weighted_sum_10000", |b| {
        b.iter(|| simd_ops::simd_weighted_sum(black_box(&data), &weights, &mut result))
    });
}

fn bench_simd_true_range(c: &mut Criterion) {
    let (high, low, close) = generate_hlc_data(DATA_LEN);
    let mut prev_close = vec![0.0; DATA_LEN];
    prev_close[0] = close[0];
    prev_close[1..].copy_from_slice(&close[..DATA_LEN - 1]);
    let mut result = vec![0.0; DATA_LEN];
    c.bench_function("simd_true_range_10000", |b| {
        b.iter(|| {
            simd_ops::simd_true_range(
                black_box(&high),
                &low,
                &prev_close,
                &mut result,
            )
        })
    });
}

fn bench_simd_typical_price(c: &mut Criterion) {
    let (high, low, close) = generate_hlc_data(DATA_LEN);
    let mut result = vec![0.0; DATA_LEN];
    c.bench_function("simd_typical_price_10000", |b| {
        b.iter(|| {
            simd_ops::simd_typical_price(black_box(&high), &low, &close, &mut result)
        })
    });
}

fn bench_simd_median_price(c: &mut Criterion) {
    let (high, low, _) = generate_hlc_data(DATA_LEN);
    let mut result = vec![0.0; DATA_LEN];
    c.bench_function("simd_median_price_10000", |b| {
        b.iter(|| simd_ops::simd_median_price(black_box(&high), &low, &mut result))
    });
}

fn bench_simd_log_return(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    let mut result = vec![0.0; DATA_LEN];
    c.bench_function("simd_log_return_10000", |b| {
        b.iter(|| simd_ops::simd_log_return(black_box(&data), &mut result))
    });
}

fn bench_simd_zscore(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    let mut result = vec![0.0; DATA_LEN];
    c.bench_function("simd_zscore_14_10000", |b| {
        b.iter(|| simd_ops::simd_zscore(black_box(&data), 14, &mut result))
    });
}

fn bench_simd_cumsum(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    let mut result = vec![0.0; DATA_LEN];
    c.bench_function("simd_cumsum_10000", |b| {
        b.iter(|| simd_ops::simd_cumsum(black_box(&data), &mut result))
    });
}

fn bench_simd_shift(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    let mut result = vec![0.0; DATA_LEN];
    c.bench_function("simd_shift_5_10000", |b| {
        b.iter(|| simd_ops::simd_shift(black_box(&data), 5, f64::NAN, &mut result))
    });
}

fn bench_simd_obv(c: &mut Criterion) {
    let close = generate_close_data(DATA_LEN);
    let volume = generate_volume_data(DATA_LEN);
    let mut result = vec![0.0; DATA_LEN];
    c.bench_function("simd_obv_10000", |b| {
        b.iter(|| simd_ops::simd_obv(black_box(&close), &volume, &mut result))
    });
}

fn bench_simd_ad_line(c: &mut Criterion) {
    let (high, low, close) = generate_hlc_data(DATA_LEN);
    let volume = generate_volume_data(DATA_LEN);
    let mut result = vec![0.0; DATA_LEN];
    c.bench_function("simd_ad_line_10000", |b| {
        b.iter(|| {
            simd_ops::simd_ad_line(
                black_box(&high),
                &low,
                &close,
                &volume,
                &mut result,
            )
        })
    });
}

fn bench_simd_roc(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    let mut result = vec![0.0; DATA_LEN];
    c.bench_function("simd_roc_14_10000", |b| {
        b.iter(|| simd_ops::simd_roc(black_box(&data), 14, &mut result))
    });
}

fn bench_indicator_kernels(c: &mut Criterion) {
    let data = generate_close_data(KERNEL_LEN);
    let mut sma_buf = vec![0.0; KERNEL_LEN];
    let mut group = c.benchmark_group("indicator_kernels_100k");

    group.bench_function("sma_scalar", |b| {
        b.iter(|| {
            simd_kernels::sma_scalar_naive_into(black_box(&data), 20, &mut sma_buf);
            black_box(&sma_buf);
        })
    });
    group.bench_function("sma_simd", |b| {
        b.iter(|| {
            simd_kernels::sma_simd_into(black_box(&data), 20, &mut sma_buf);
            black_box(&sma_buf);
        })
    });
    group.bench_function("sma_rolling_scalar", |b| {
        b.iter(|| {
            simd_kernels::sma_scalar_into(black_box(&data), 20, &mut sma_buf);
            black_box(&sma_buf);
        })
    });
    group.bench_function("ema_scalar", |b| {
        b.iter(|| black_box(simd_kernels::ema_scalar_ref(black_box(&data), 20)))
    });
    group.bench_function("ema_simd", |b| {
        b.iter(|| black_box(simd_kernels::ema_simd(black_box(&data), 20)))
    });
    group.bench_function("rsi_simd", |b| {
        b.iter(|| black_box(simd_kernels::rsi_simd(black_box(&data), 14)))
    });
    group.bench_function("macd_simd", |b| {
        b.iter(|| black_box(simd_kernels::macd_simd(black_box(&data), 12, 26, 9)))
    });

    group.finish();
}

#[cfg(feature = "nightly-avx512")]
fn bench_avx512_comparison(c: &mut Criterion) {
    let data = generate_close_data(KERNEL_LEN);
    let mut sma_buf = vec![0.0; KERNEL_LEN];
    let mut ema_buf = vec![0.0; KERNEL_LEN];
    let mut group = c.benchmark_group("avx512_comparison_100k");

    group.bench_function("sma_simd_with_avx512", |b| {
        b.iter(|| {
            simd_kernels::sma_simd_into(black_box(&data), 20, &mut sma_buf);
            black_box(&sma_buf);
        })
    });
    group.bench_function("ema_simd_with_avx512", |b| {
        b.iter(|| {
            simd_kernels::ema_simd_into(black_box(&data), 20, &mut ema_buf);
            black_box(&ema_buf);
        })
    });

    for period in [5, 10, 20, 50, 100, 200] {
        group.bench_function(format!("sma_period_{}", period), |b| {
            b.iter(|| {
                simd_kernels::sma_simd_into(black_box(&data), period, &mut sma_buf);
                black_box(&sma_buf);
            })
        });
        group.bench_function(format!("ema_period_{}", period), |b| {
            b.iter(|| {
                simd_kernels::ema_simd_into(black_box(&data), period, &mut ema_buf);
                black_box(&ema_buf);
            })
        });
    }

    group.finish();
}

fn bench_stoch_cci_kernels(c: &mut Criterion) {
    let (high, low, close) = generate_hlc_data(KERNEL_LEN);
    let mut k_buf = vec![0.0; KERNEL_LEN];
    let mut d_buf = vec![0.0; KERNEL_LEN];
    let mut cci_buf = vec![0.0; KERNEL_LEN];
    let mut group = c.benchmark_group("stoch_cci_100k");

    group.bench_function("stoch_scalar_14_3_3", |b| {
        b.iter(|| {
            simd_kernels::stoch_scalar(
                black_box(&high),
                &low,
                &close,
                14,
                3,
                3,
                &mut k_buf,
                &mut d_buf,
            );
            black_box(&k_buf);
            black_box(&d_buf);
        })
    });
    group.bench_function("stoch_simd_14_3_3", |b| {
        b.iter(|| {
            simd_kernels::stoch_simd_into(
                black_box(&high),
                &low,
                &close,
                14,
                3,
                3,
                &mut k_buf,
                &mut d_buf,
            );
            black_box(&k_buf);
            black_box(&d_buf);
        })
    });
    group.bench_function("cci_scalar_14", |b| {
        b.iter(|| {
            simd_kernels::cci_scalar(black_box(&high), &low, &close, 14, &mut cci_buf);
            black_box(&cci_buf);
        })
    });
    group.bench_function("cci_simd_14", |b| {
        b.iter(|| {
            simd_kernels::cci_simd_into(black_box(&high), &low, &close, 14, &mut cci_buf);
            black_box(&cci_buf);
        })
    });

    group.finish();
}

fn bench_mod_pow_simd(c: &mut Criterion) {
    let a = generate_close_data(DATA_LEN);
    let b_data: Vec<f64> = a.iter().map(|x| x * 0.1 + 1.0).collect();
    let mut result_simd = vec![0.0; DATA_LEN];
    let mut result_scalar = vec![0.0; DATA_LEN];
    let mut group = c.benchmark_group("mod_pow_10000");

    group.bench_function("mod_simd", |bencher| {
        bencher.iter(|| {
            SimdOps::simd_mod(black_box(&a), black_box(&b_data), &mut result_simd);
            black_box(&result_simd);
        })
    });
    group.bench_function("mod_scalar", |bencher| {
        bencher.iter(|| {
            for i in 0..DATA_LEN {
                result_scalar[i] = if b_data[i].abs() < 1e-15 {
                    f64::NAN
                } else {
                    a[i] - (a[i] / b_data[i]).floor() * b_data[i]
                };
            }
            black_box(&result_scalar);
        })
    });
    group.bench_function("pow_simd", |bencher| {
        bencher.iter(|| {
            SimdOps::simd_pow(black_box(&a), black_box(&b_data), &mut result_simd);
            black_box(&result_simd);
        })
    });
    group.bench_function("pow_scalar", |bencher| {
        bencher.iter(|| {
            for i in 0..DATA_LEN {
                result_scalar[i] = a[i].powf(b_data[i]);
            }
            black_box(&result_scalar);
        })
    });

    group.finish();
}

fn bench_mod_pow_large(c: &mut Criterion) {
    let a = generate_close_data(KERNEL_LEN);
    let b_data: Vec<f64> = a.iter().map(|x| x * 0.1 + 1.0).collect();
    let mut result_simd = vec![0.0; KERNEL_LEN];
    let mut result_scalar = vec![0.0; KERNEL_LEN];
    let mut group = c.benchmark_group("mod_pow_100k");

    group.bench_function("mod_simd_100k", |bencher| {
        bencher.iter(|| {
            SimdOps::simd_mod(black_box(&a), black_box(&b_data), &mut result_simd);
            black_box(&result_simd);
        })
    });
    group.bench_function("mod_scalar_100k", |bencher| {
        bencher.iter(|| {
            for i in 0..KERNEL_LEN {
                result_scalar[i] = if b_data[i].abs() < 1e-15 {
                    f64::NAN
                } else {
                    a[i] - (a[i] / b_data[i]).floor() * b_data[i]
                };
            }
            black_box(&result_scalar);
        })
    });
    group.bench_function("pow_simd_100k", |bencher| {
        bencher.iter(|| {
            SimdOps::simd_pow(black_box(&a), black_box(&b_data), &mut result_simd);
            black_box(&result_simd);
        })
    });
    group.bench_function("pow_scalar_100k", |bencher| {
        bencher.iter(|| {
            for i in 0..KERNEL_LEN {
                result_scalar[i] = a[i].powf(b_data[i]);
            }
            black_box(&result_scalar);
        })
    });

    group.finish();
}

#[cfg(feature = "nightly-avx512")]
criterion_group!(avx512_benches, bench_avx512_comparison);

criterion_group!(
    simd_benches,
    bench_simd_prefix_sum,
    bench_simd_diff,
    bench_simd_scale,
    bench_simd_pct_change,
    bench_simd_clamp,
    bench_simd_weighted_sum,
    bench_simd_true_range,
    bench_simd_typical_price,
    bench_simd_median_price,
    bench_simd_log_return,
    bench_simd_zscore,
    bench_simd_cumsum,
    bench_simd_shift,
    bench_simd_obv,
    bench_simd_ad_line,
    bench_simd_roc,
    bench_indicator_kernels,
    bench_stoch_cci_kernels,
    bench_mod_pow_simd,
    bench_mod_pow_large,
);

#[cfg(feature = "nightly-avx512")]
criterion_main!(simd_benches, avx512_benches);

#[cfg(not(feature = "nightly-avx512"))]
criterion_main!(simd_benches);
