use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput,
};
use finkit::formula::FormulaEngine;
use finkit::indicators::momentum::{macd, rsi};
use finkit::math::moving_avg::{ema, sma};
use finkit::math::statistics::{rolling_max, rolling_min};
use ndarray::Array1;

fn create_ohlcv_data(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut close = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut volume = Vec::with_capacity(len);
    let mut price;
    for i in 0..len {
        let t = i as f64;
        let noise = (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5 + (t * 3.71).sin() * 0.8;
        let trend = t * 0.01;
        price = 100.0 + trend + noise;
        let o = price - 0.3;
        let h = price + 1.0 + ((t * 0.7).sin().abs() * 0.5);
        let l = price - 1.0 - ((t * 0.5).cos().abs() * 0.5);
        let v = 10000.0 + (t * 10.0).sin() * 3000.0 + 2000.0 * (t * 2.3).cos().abs();
        close.push(price);
        open.push(o);
        high.push(h);
        low.push(l);
        volume.push(v);
    }
    (open, high, low, close, volume)
}

fn create_ctx(len: usize) -> finkit::formula::FormulaContext {
    let (open, high, low, close, volume) = create_ohlcv_data(len);
    finkit::formula::FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
        None,
    )
}

fn get_close_slice(len: usize) -> Vec<f64> {
    let (_, _, _, close, _) = create_ohlcv_data(len);
    close
}

fn benchmark_ma_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("ma_performance");
    group.sampling_mode(criterion::SamplingMode::Flat);

    for data_len in [1000, 10000, 100000, 1000000] {
        let close = get_close_slice(data_len);
        group.throughput(Throughput::Elements(data_len as u64));

        group.bench_with_input(
            BenchmarkId::new("native_sma_20", data_len),
            &close,
            |b, data| {
                b.iter(|| {
                    let _ = black_box(sma(data, 20));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_MA_20", data_len),
            &data_len,
            |b, len| {
                let mut engine = FormulaEngine::new();
                let formula = engine.compile("MA(CLOSE, 20)").unwrap();
                b.iter_batched(
                    || create_ctx(*len),
                    |mut ctx| {
                        let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }
    group.finish();
}

fn benchmark_ema_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("ema_performance");
    group.sampling_mode(criterion::SamplingMode::Flat);

    for data_len in [1000, 10000, 100000, 1000000] {
        let close = get_close_slice(data_len);
        group.throughput(Throughput::Elements(data_len as u64));

        group.bench_with_input(
            BenchmarkId::new("native_ema_12", data_len),
            &close,
            |b, data| {
                b.iter(|| {
                    let _ = black_box(ema(data, 12));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_EMA_12", data_len),
            &data_len,
            |b, len| {
                let mut engine = FormulaEngine::new();
                let formula = engine.compile("EMA(CLOSE, 12)").unwrap();
                b.iter_batched(
                    || create_ctx(*len),
                    |mut ctx| {
                        let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }
    group.finish();
}

fn benchmark_rsi_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("rsi_performance");
    group.sampling_mode(criterion::SamplingMode::Flat);

    for data_len in [1000, 10000, 100000, 1000000] {
        let close = get_close_slice(data_len);
        group.throughput(Throughput::Elements(data_len as u64));

        group.bench_with_input(
            BenchmarkId::new("native_rsi_14", data_len),
            &close,
            |b, data| {
                b.iter(|| {
                    let _ = black_box(rsi(data, 14));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_RSI_14", data_len),
            &data_len,
            |b, len| {
                let mut engine = FormulaEngine::new();
                let formula = engine.compile("RSI(CLOSE, 14)").unwrap();
                b.iter_batched(
                    || create_ctx(*len),
                    |mut ctx| {
                        let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }
    group.finish();
}

fn benchmark_macd_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("macd_performance");
    group.sampling_mode(criterion::SamplingMode::Flat);

    for data_len in [1000, 10000, 100000, 1000000] {
        let close = get_close_slice(data_len);
        group.throughput(Throughput::Elements(data_len as u64));

        group.bench_with_input(
            BenchmarkId::new("native_macd_12_26_9", data_len),
            &close,
            |b, data| {
                b.iter(|| {
                    let _ = black_box(macd(data, 12, 26, 9));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_MACD_12_26_9", data_len),
            &data_len,
            |b, len| {
                let mut engine = FormulaEngine::new();
                let formula = engine.compile("MACD(CLOSE, 12, 26, 9)").unwrap();
                b.iter_batched(
                    || create_ctx(*len),
                    |mut ctx| {
                        let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }
    group.finish();
}

fn benchmark_hhv_llv_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("hhv_llv_performance");
    group.sampling_mode(criterion::SamplingMode::Flat);

    for data_len in [1000, 10000, 100000, 1000000] {
        let close = get_close_slice(data_len);
        group.throughput(Throughput::Elements(data_len as u64));

        group.bench_with_input(
            BenchmarkId::new("native_rolling_max_20", data_len),
            &close,
            |b, data| {
                b.iter(|| {
                    let _ = black_box(rolling_max(data, 20));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_HHV_20", data_len),
            &data_len,
            |b, len| {
                let mut engine = FormulaEngine::new();
                let formula = engine.compile("HHV(CLOSE, 20)").unwrap();
                b.iter_batched(
                    || create_ctx(*len),
                    |mut ctx| {
                        let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("native_rolling_min_20", data_len),
            &close,
            |b, data| {
                b.iter(|| {
                    let _ = black_box(rolling_min(data, 20));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_LLV_20", data_len),
            &data_len,
            |b, len| {
                let mut engine = FormulaEngine::new();
                let formula = engine.compile("LLV(CLOSE, 20)").unwrap();
                b.iter_batched(
                    || create_ctx(*len),
                    |mut ctx| {
                        let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }
    group.finish();
}

fn benchmark_complex_formula(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_formula_performance");
    group.sampling_mode(criterion::SamplingMode::Flat);

    let formulas = [
        ("MA_CROSS", "MA(CLOSE, 5) > MA(CLOSE, 20)"),
        ("MACD_SIGNAL", "MACD(CLOSE, 12, 26, 9) > 0"),
        ("RSI_OVERBOUGHT", "RSI(CLOSE, 14) > 70"),
        ("BOLL_UPPER", "MA(CLOSE, 20) + 2 * STD(CLOSE, 20)"),
        ("KDJ_K", "KDJ(HIGH, LOW, CLOSE, 9, 3, 3)"),
    ];

    for data_len in [1000, 10000, 100000] {
        group.throughput(Throughput::Elements(data_len as u64));

        for (name, formula_src) in formulas.iter() {
            group.bench_with_input(BenchmarkId::new(*name, data_len), &data_len, |b, len| {
                let mut engine = FormulaEngine::new();
                let formula = engine.compile(formula_src).unwrap();
                b.iter_batched(
                    || create_ctx(*len),
                    |mut ctx| {
                        let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            });
        }
    }
    group.finish();
}

fn benchmark_bytecode_vs_ast(c: &mut Criterion) {
    let mut group = c.benchmark_group("bytecode_vs_ast");
    group.sampling_mode(criterion::SamplingMode::Flat);

    let data_len = 10000;
    group.throughput(Throughput::Elements(data_len as u64));

    let formulas = [
        ("MA_20", "MA(CLOSE, 20)"),
        ("EMA_12", "EMA(CLOSE, 12)"),
        ("RSI_14", "RSI(CLOSE, 14)"),
        ("MACD", "MACD(CLOSE, 12, 26, 9)"),
    ];

    for (name, formula_src) in formulas.iter() {
        group.bench_function(format!("{}_ast", name), |b| {
            let mut engine = FormulaEngine::new();
            let formula = engine.compile(formula_src).unwrap();
            b.iter_batched(
                || create_ctx(data_len),
                |mut ctx| {
                    let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        group.bench_function(format!("{}_bytecode", name), |b| {
            let mut engine = FormulaEngine::new();
            let bytecode = engine.compile_bytecode(formula_src).unwrap();
            b.iter_batched(
                || create_ctx(data_len),
                |mut ctx| {
                    let _ = black_box(engine.execute_bytecode(&bytecode, &ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

fn benchmark_zero_copy_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("zero_copy_performance");
    group.sampling_mode(criterion::SamplingMode::Flat);

    for data_len in [1000, 10000, 100000] {
        group.throughput(Throughput::Elements(data_len as u64));

        group.bench_with_input(
            BenchmarkId::new("normal_MA_20", data_len),
            &data_len,
            |b, len| {
                let mut engine = FormulaEngine::new();
                let formula = engine.compile("MA(CLOSE, 20)").unwrap();
                b.iter_batched(
                    || create_ctx(*len),
                    |mut ctx| {
                        let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("zero_copy_MA_20", data_len),
            &data_len,
            |b, len| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(*len),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_copy("MA(CLOSE, 20)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("normal_RSI_14", data_len),
            &data_len,
            |b, len| {
                let mut engine = FormulaEngine::new();
                let formula = engine.compile("RSI(CLOSE, 14)").unwrap();
                b.iter_batched(
                    || create_ctx(*len),
                    |mut ctx| {
                        let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("zero_copy_RSI_14", data_len),
            &data_len,
            |b, len| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(*len),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_copy("RSI(CLOSE, 14)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }
    group.finish();
}

fn benchmark_eval_into_reuse(c: &mut Criterion) {
    let mut group = c.benchmark_group("eval_into_reuse");
    group.sampling_mode(criterion::SamplingMode::Flat);

    for data_len in [1000, 10000, 100000] {
        group.throughput(Throughput::Elements(data_len as u64));
        group.bench_with_input(BenchmarkId::new("MA_20", data_len), &data_len, |b, len| {
            let mut engine = FormulaEngine::new();
            let formula = engine.compile("MA(CLOSE, 20)").unwrap();
            b.iter_batched(
                || (create_ctx(*len), Array1::zeros(*len)),
                |(mut ctx, mut output)| {
                    let _ = black_box(engine.eval_into(&formula, &mut ctx, &mut output));
                },
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

criterion_group!(
    performance_benches,
    benchmark_ma_performance,
    benchmark_ema_performance,
    benchmark_rsi_performance,
    benchmark_macd_performance,
    benchmark_hhv_llv_performance,
    benchmark_complex_formula,
    benchmark_bytecode_vs_ast,
    benchmark_zero_copy_performance,
    benchmark_eval_into_reuse,
);
criterion_main!(performance_benches);
