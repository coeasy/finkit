use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use finkit::formula::simd::SimdOps;
use finkit::math::linear;
use finkit::math::simd_ops;
use finkit::math::statistics;

const DATA_LEN: usize = 100_000;
const SMALL_DATA_LEN: usize = 10_000;

fn generate_close_data(len: usize) -> Vec<f64> {
    (0..len)
        .map(|i| {
            let t = i as f64;
            100.0 + t * 0.01 + (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5
        })
        .collect()
}

fn generate_benchmark_data(len: usize) -> Vec<f64> {
    (0..len)
        .map(|i| {
            let t = i as f64;
            1000.0 + t * 0.05 + (t * 0.23).sin() * 50.0 + (t * 0.87).cos() * 30.0
        })
        .collect()
}

fn generate_correlated_data(len: usize, correlation: f64) -> (Vec<f64>, Vec<f64>) {
    let x = generate_close_data(len);
    let y: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, xi)| {
            let noise = ((i as f64 * 0.5).sin() * 5.0) * (1.0 - correlation.abs());
            xi * correlation + noise + 50.0
        })
        .collect();
    (x, y)
}

fn bench_stddev(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    let mut group = c.benchmark_group("stddev");

    for period in [5, 14, 20, 50, 100] {
        let mut result_simd = vec![0.0; DATA_LEN];
        let mut result_scalar = vec![0.0; DATA_LEN];

        group.bench_with_input(BenchmarkId::new("simd", period), &period, |b, p| {
            b.iter(|| {
                SimdOps::stddev(black_box(&data), *p, &mut result_simd);
                black_box(&result_simd);
            })
        });

        group.bench_with_input(BenchmarkId::new("scalar_naive", period), &period, |b, p| {
            b.iter(|| {
                for i in 0..DATA_LEN {
                    if i + 1 < *p {
                        result_scalar[i] = f64::NAN;
                    } else {
                        let start = i + 1 - *p;
                        let window = &data[start..=i];
                        let mean: f64 = window.iter().sum::<f64>() / *p as f64;
                        let var: f64 = window.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
                            / (*p as f64 - 1.0);
                        result_scalar[i] = var.sqrt();
                    }
                }
                black_box(&result_scalar);
            })
        });

        group.bench_with_input(
            BenchmarkId::new("statistics_module", period),
            &period,
            |b, p| {
                b.iter(|| {
                    black_box(statistics::rolling_std_dev(black_box(&data), *p).unwrap());
                })
            },
        );
    }
    group.finish();
}

fn bench_zscore(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    let mut group = c.benchmark_group("zscore");

    for period in [5, 14, 20, 50] {
        let mut result_simd = vec![0.0; DATA_LEN];
        let mut result_scalar = vec![0.0; DATA_LEN];

        group.bench_with_input(
            BenchmarkId::new("simd_optimized", period),
            &period,
            |b, p| {
                b.iter(|| {
                    SimdOps::zscore_optimized(black_box(&data), *p, &mut result_simd);
                    black_box(&result_simd);
                })
            },
        );

        group.bench_with_input(BenchmarkId::new("simd_ops", period), &period, |b, p| {
            b.iter(|| {
                simd_ops::simd_zscore_optimized(black_box(&data), *p, &mut result_simd);
                black_box(&result_simd);
            })
        });

        group.bench_with_input(BenchmarkId::new("scalar_naive", period), &period, |b, p| {
            b.iter(|| {
                for i in 0..DATA_LEN {
                    if i + 1 < *p {
                        result_scalar[i] = f64::NAN;
                    } else {
                        let start = i + 1 - *p;
                        let window = &data[start..=i];
                        let mean: f64 = window.iter().sum::<f64>() / *p as f64;
                        let var: f64 = window.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
                            / (*p as f64 - 1.0);
                        let std = var.sqrt();
                        result_scalar[i] = if std.abs() < 1e-15 {
                            0.0
                        } else {
                            (data[i] - mean) / std
                        };
                    }
                }
                black_box(&result_scalar);
            })
        });
    }
    group.finish();
}

fn bench_correl(c: &mut Criterion) {
    let (x, y) = generate_correlated_data(SMALL_DATA_LEN, 0.8);
    let mut group = c.benchmark_group("correl");

    for period in [5, 14, 20, 50] {
        let mut result_simd = vec![0.0; SMALL_DATA_LEN];
        let mut result_scalar = vec![0.0; SMALL_DATA_LEN];

        group.bench_with_input(BenchmarkId::new("simd", period), &period, |b, p| {
            b.iter(|| {
                SimdOps::correl(black_box(&x), black_box(&y), *p, &mut result_simd);
                black_box(&result_simd);
            })
        });

        group.bench_with_input(BenchmarkId::new("simd_ops", period), &period, |b, p| {
            b.iter(|| {
                simd_ops::simd_correl(black_box(&x), black_box(&y), *p, &mut result_simd);
                black_box(&result_simd);
            })
        });

        group.bench_with_input(BenchmarkId::new("scalar_naive", period), &period, |b, p| {
            b.iter(|| {
                for i in 0..SMALL_DATA_LEN {
                    if i + 1 < *p {
                        result_scalar[i] = f64::NAN;
                    } else {
                        let start = i + 1 - *p;
                        let wx = &x[start..=i];
                        let wy = &y[start..=i];
                        let mean_x: f64 = wx.iter().sum::<f64>() / *p as f64;
                        let mean_y: f64 = wy.iter().sum::<f64>() / *p as f64;
                        let cov: f64 = wx
                            .iter()
                            .zip(wy.iter())
                            .map(|(xi, yi)| (xi - mean_x) * (yi - mean_y))
                            .sum::<f64>()
                            / (*p as f64 - 1.0);
                        let var_x: f64 = wx.iter().map(|xi| (xi - mean_x).powi(2)).sum::<f64>()
                            / (*p as f64 - 1.0);
                        let var_y: f64 = wy.iter().map(|yi| (yi - mean_y).powi(2)).sum::<f64>()
                            / (*p as f64 - 1.0);
                        let denom = (var_x * var_y).sqrt();
                        result_scalar[i] = if denom.abs() < 1e-15 {
                            f64::NAN
                        } else {
                            cov / denom
                        };
                    }
                }
                black_box(&result_scalar);
            })
        });
    }
    group.finish();
}

fn bench_beta(c: &mut Criterion) {
    let (asset, benchmark) = generate_correlated_data(SMALL_DATA_LEN, 0.9);
    let mut group = c.benchmark_group("beta");

    for period in [5, 14, 20, 50] {
        let mut result_simd = vec![0.0; SMALL_DATA_LEN];
        let mut result_scalar = vec![0.0; SMALL_DATA_LEN];

        group.bench_with_input(BenchmarkId::new("simd", period), &period, |b, p| {
            b.iter(|| {
                SimdOps::beta(
                    black_box(&asset),
                    black_box(&benchmark),
                    *p,
                    &mut result_simd,
                );
                black_box(&result_simd);
            })
        });

        group.bench_with_input(BenchmarkId::new("simd_ops", period), &period, |b, p| {
            b.iter(|| {
                simd_ops::simd_beta(
                    black_box(&asset),
                    black_box(&benchmark),
                    *p,
                    &mut result_simd,
                );
                black_box(&result_simd);
            })
        });

        group.bench_with_input(BenchmarkId::new("scalar_naive", period), &period, |b, p| {
            b.iter(|| {
                for i in 0..SMALL_DATA_LEN {
                    if i + 1 < *p {
                        result_scalar[i] = f64::NAN;
                    } else {
                        let start = i + 1 - *p;
                        let wa = &asset[start..=i];
                        let wb = &benchmark[start..=i];
                        let mean_a: f64 = wa.iter().sum::<f64>() / *p as f64;
                        let mean_b: f64 = wb.iter().sum::<f64>() / *p as f64;
                        let cov: f64 = wa
                            .iter()
                            .zip(wb.iter())
                            .map(|(a, b)| (a - mean_a) * (b - mean_b))
                            .sum::<f64>()
                            / (*p as f64 - 1.0);
                        let var_b: f64 = wb.iter().map(|b| (b - mean_b).powi(2)).sum::<f64>()
                            / (*p as f64 - 1.0);
                        result_scalar[i] = if var_b.abs() < 1e-15 {
                            f64::NAN
                        } else {
                            cov / var_b
                        };
                    }
                }
                black_box(&result_scalar);
            })
        });
    }
    group.finish();
}

fn bench_linear_reg_slope(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    let mut group = c.benchmark_group("linear_reg_slope");

    for period in [5, 14, 20, 50] {
        let mut result_simd = vec![0.0; DATA_LEN];
        let mut result_scalar = vec![0.0; DATA_LEN];

        group.bench_with_input(BenchmarkId::new("simd", period), &period, |b, p| {
            b.iter(|| {
                SimdOps::linear_reg_slope(black_box(&data), *p, &mut result_simd);
                black_box(&result_simd);
            })
        });

        group.bench_with_input(BenchmarkId::new("simd_ops", period), &period, |b, p| {
            b.iter(|| {
                simd_ops::simd_linreg_slope(black_box(&data), *p, &mut result_simd);
                black_box(&result_simd);
            })
        });

        group.bench_with_input(BenchmarkId::new("scalar_naive", period), &period, |b, p| {
            b.iter(|| {
                for i in 0..DATA_LEN {
                    if i + 1 < *p {
                        result_scalar[i] = f64::NAN;
                    } else {
                        let start = i + 1 - *p;
                        let window = &data[start..=i];
                        let n = *p as f64;
                        let sum_x = n * (n - 1.0) / 2.0;
                        let sum_x2 = n * (n - 1.0) * (2.0 * n - 1.0) / 6.0;
                        let sum_y: f64 = window.iter().sum();
                        let sum_xy: f64 =
                            window.iter().enumerate().map(|(j, v)| j as f64 * v).sum();
                        let denom = n * sum_x2 - sum_x * sum_x;
                        result_scalar[i] = (n * sum_xy - sum_x * sum_y) / denom;
                    }
                }
                black_box(&result_scalar);
            })
        });

        group.bench_with_input(
            BenchmarkId::new("linear_module", period),
            &period,
            |b, p| {
                b.iter(|| {
                    black_box(linear::linreg_slope(black_box(&data), *p).unwrap());
                })
            },
        );
    }
    group.finish();
}

fn bench_linear_reg(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    let mut group = c.benchmark_group("linear_reg");

    for period in [5, 14, 20, 50] {
        let mut result_simd = vec![0.0; DATA_LEN];
        let mut result_scalar = vec![0.0; DATA_LEN];

        group.bench_with_input(BenchmarkId::new("simd", period), &period, |b, p| {
            b.iter(|| {
                SimdOps::linear_reg(black_box(&data), *p, &mut result_simd);
                black_box(&result_simd);
            })
        });

        group.bench_with_input(BenchmarkId::new("simd_ops", period), &period, |b, p| {
            b.iter(|| {
                simd_ops::simd_linreg(black_box(&data), *p, &mut result_simd);
                black_box(&result_simd);
            })
        });

        group.bench_with_input(
            BenchmarkId::new("linear_module", period),
            &period,
            |b, p| {
                b.iter(|| {
                    black_box(linear::linreg(black_box(&data), *p).unwrap());
                })
            },
        );
    }
    group.finish();
}

fn bench_linear_reg_angle(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    let mut group = c.benchmark_group("linear_reg_angle");

    for period in [5, 14, 20, 50] {
        let mut result_simd = vec![0.0; DATA_LEN];

        group.bench_with_input(BenchmarkId::new("simd", period), &period, |b, p| {
            b.iter(|| {
                SimdOps::linear_reg_angle(black_box(&data), *p, &mut result_simd);
                black_box(&result_simd);
            })
        });

        group.bench_with_input(BenchmarkId::new("simd_ops", period), &period, |b, p| {
            b.iter(|| {
                simd_ops::simd_linreg_angle(black_box(&data), *p, &mut result_simd);
                black_box(&result_simd);
            })
        });

        group.bench_with_input(
            BenchmarkId::new("linear_module", period),
            &period,
            |b, p| {
                b.iter(|| {
                    black_box(linear::linreg_angle(black_box(&data), *p).unwrap());
                })
            },
        );
    }
    group.finish();
}

fn bench_linear_reg_r2(c: &mut Criterion) {
    let data = generate_close_data(SMALL_DATA_LEN);
    let mut group = c.benchmark_group("linear_reg_r2");

    for period in [5, 14, 20] {
        let mut result_simd = vec![0.0; SMALL_DATA_LEN];

        group.bench_with_input(BenchmarkId::new("simd", period), &period, |b, p| {
            b.iter(|| {
                SimdOps::linear_reg_r2(black_box(&data), *p, &mut result_simd);
                black_box(&result_simd);
            })
        });
    }
    group.finish();
}

fn bench_combined_statistics(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    let (x, y) = generate_correlated_data(DATA_LEN, 0.85);
    let mut group = c.benchmark_group("combined_statistics_100k");

    let period = 20;

    group.bench_function("stddev_simd", |b| {
        let mut result = vec![0.0; DATA_LEN];
        b.iter(|| {
            SimdOps::stddev(black_box(&data), period, &mut result);
            black_box(&result);
        })
    });

    group.bench_function("zscore_simd", |b| {
        let mut result = vec![0.0; DATA_LEN];
        b.iter(|| {
            SimdOps::zscore_optimized(black_box(&data), period, &mut result);
            black_box(&result);
        })
    });

    group.bench_function("correl_simd", |b| {
        let mut result = vec![0.0; DATA_LEN];
        b.iter(|| {
            SimdOps::correl(black_box(&x), black_box(&y), period, &mut result);
            black_box(&result);
        })
    });

    group.bench_function("beta_simd", |b| {
        let mut result = vec![0.0; DATA_LEN];
        b.iter(|| {
            SimdOps::beta(black_box(&x), black_box(&y), period, &mut result);
            black_box(&result);
        })
    });

    group.bench_function("linreg_slope_simd", |b| {
        let mut result = vec![0.0; DATA_LEN];
        b.iter(|| {
            SimdOps::linear_reg_slope(black_box(&data), period, &mut result);
            black_box(&result);
        })
    });

    group.bench_function("linreg_simd", |b| {
        let mut result = vec![0.0; DATA_LEN];
        b.iter(|| {
            SimdOps::linear_reg(black_box(&data), period, &mut result);
            black_box(&result);
        })
    });

    group.bench_function("linreg_angle_simd", |b| {
        let mut result = vec![0.0; DATA_LEN];
        b.iter(|| {
            SimdOps::linear_reg_angle(black_box(&data), period, &mut result);
            black_box(&result);
        })
    });

    group.finish();
}

criterion_group!(
    simd_statistics_benches,
    bench_stddev,
    bench_zscore,
    bench_correl,
    bench_beta,
    bench_linear_reg_slope,
    bench_linear_reg,
    bench_linear_reg_angle,
    bench_linear_reg_r2,
    bench_combined_statistics,
);
criterion_main!(simd_statistics_benches);
