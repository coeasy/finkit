use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use finkit::formula::{FormulaContext, FormulaEngine};
use finkit::indicators;
use finkit::math::moving_avg;
use ndarray::Array1;

#[allow(clippy::type_complexity)]
fn create_ohlcv_data(len: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut close = Vec::with_capacity(len);
    let mut open = Vec::with_capacity(len);
    let mut high = Vec::with_capacity(len);
    let mut low = Vec::with_capacity(len);
    let mut volume = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f64;
        let noise = (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5 + (t * 3.71).sin() * 0.8;
        let trend = t * 0.01;
        let price = 100.0 + trend + noise;
        open.push(price - 0.3);
        high.push(price + 1.0 + ((t * 0.7).sin().abs() * 0.5));
        low.push(price - 1.0 - ((t * 0.5).cos().abs() * 0.5));
        close.push(price);
        volume.push(10000.0 + (t * 10.0).sin() * 3000.0 + 2000.0 * (t * 2.3).cos().abs());
    }
    (open, high, low, close, volume)
}

fn create_ctx(len: usize) -> FormulaContext {
    let (open, high, low, close, volume) = create_ohlcv_data(len);
    FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
        None,
    )
}

fn bench_sma(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_SMA_20");
        let close_c = close.clone();

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(indicators::sma(&close_c, 20).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("MA(CLOSE, 20)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("MA(CLOSE, 20)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_alloc("MA(CLOSE, 20)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_ema(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_EMA_12");
        let close_c = close.clone();

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(indicators::ema(&close_c, 12).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("EMA(CLOSE, 12)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("EMA(CLOSE, 12)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_alloc("EMA(CLOSE, 12)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_wma(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_WMA_20");
        let close_c = close.clone();

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(moving_avg::wma(&close_c, 20).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("WMA(CLOSE, 20)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("WMA(CLOSE, 20)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_alloc("WMA(CLOSE, 20)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_dema(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_DEMA_10");
        let close_c = close.clone();

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(moving_avg::dema(&close_c, 10).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("DEMA(CLOSE, 10)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("DEMA(CLOSE, 10)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_alloc("DEMA(CLOSE, 10)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_tema(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_TEMA_10");
        let close_c = close.clone();

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(moving_avg::tema(&close_c, 10).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("TEMA(CLOSE, 10)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("TEMA(CLOSE, 10)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_alloc("TEMA(CLOSE, 10)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_kama(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_KAMA_10");
        let close_c = close.clone();

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(indicators::kama(&close_c, 10, 2, 30).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("KAMA(CLOSE, 10)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("KAMA(CLOSE, 10)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_alloc("KAMA(CLOSE, 10)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_rsi(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_RSI_14");
        let close_c = close.clone();

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(indicators::rsi(&close_c, 14).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("RSI(CLOSE, 14)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("RSI(CLOSE, 14)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_alloc("RSI(CLOSE, 14)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_macd(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_MACD");
        let close_c = close.clone();

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(indicators::macd(&close_c, 12, 26, 9).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("MACD(C,12,26)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("MACD(C,12,26)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_alloc("MACD(C,12,26)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_cci(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, high, low, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_CCI_14");

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(indicators::cci(&high, &low, &close, 14).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("CCI(H,L,C,14)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("CCI(H,L,C,14)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_alloc("CCI(H,L,C,14)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_adx(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, high, low, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_ADX_14");

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(indicators::adx(&high, &low, &close, 14).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("ADX(H,L,C,14)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("ADX(H,L,C,14)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_alloc("ADX(H,L,C,14)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_atr(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, high, low, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_ATR_14");

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(indicators::atr(&high, &low, &close, 14).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("ATR_ENHANCED(H,L,C,14)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("ATR_ENHANCED(H,L,C,14)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ = black_box(
                            engine
                                .eval_zero_alloc("ATR_ENHANCED(H,L,C,14)", &mut ctx)
                                .unwrap(),
                        );
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_bbands(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_BBANDS");
        let close_c = close.clone();

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(indicators::bbands(&close_c, 20, 2.0, 2.0).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("BOLL(C,20,2)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("BOLL(C,20,2)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_alloc("BOLL(C,20,2)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_stoch(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, high, low, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_STOCH");

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(indicators::stoch(&high, &low, &close, 14, 3, 3).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("STOCH(H,L,C,14,3,3)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("STOCH(H,L,C,14,3,3)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ = black_box(
                            engine
                                .eval_zero_alloc("STOCH(H,L,C,14,3,3)", &mut ctx)
                                .unwrap(),
                        );
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_willr(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, high, low, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_WILLR_14");

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(indicators::willr(&high, &low, &close, 14).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("WILLR(H,L,C,14)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("WILLR(H,L,C,14)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_alloc("WILLR(H,L,C,14)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_roc(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_ROC_10");
        let close_c = close.clone();

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(indicators::roc(&close_c, 10).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("ROC(CLOSE, 10)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("ROC(CLOSE, 10)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_alloc("ROC(CLOSE, 10)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_mom(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_MOM_10");
        let close_c = close.clone();

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(indicators::mom(&close_c, 10).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("MOM(CLOSE, 10)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("MOM(CLOSE, 10)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_alloc("MOM(CLOSE, 10)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_stddev(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_STDDEV");
        let close_c = close.clone();

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(indicators::std_dev(&close_c, 20, 1.0).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("STD(CLOSE, 20)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("STD(CLOSE, 20)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_alloc("STD(CLOSE, 20)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_linear_reg(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_LINEAR_REG_14");
        let close_c = close.clone();

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(indicators::linear_reg(&close_c, 14).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("LINEARREG(CLOSE, 14)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("LINEARREG(CLOSE, 14)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ = black_box(
                            engine
                                .eval_zero_alloc("LINEARREG(CLOSE, 14)", &mut ctx)
                                .unwrap(),
                        );
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

fn bench_trix(c: &mut Criterion) {
    for size in [10_000, 100_000, 1_000_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        let mut g = c.benchmark_group("triple_TRIX_14");
        let close_c = close.clone();

        g.bench_with_input(BenchmarkId::new("native", size), &size, |b, _| {
            b.iter(|| black_box(indicators::trix(&close_c, 14).unwrap()))
        });

        g.bench_with_input(BenchmarkId::new("formula_eval", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("TRIX(CLOSE, 14)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(BenchmarkId::new("formula_builtin", size), &size, |b, _| {
            let mut engine = FormulaEngine::new();
            b.iter_batched(
                || create_ctx(size),
                |mut ctx| {
                    let _ = black_box(engine.eval("TRIX(CLOSE, 14)", &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        g.bench_with_input(
            BenchmarkId::new("formula_zero_alloc", size),
            &size,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ =
                            black_box(engine.eval_zero_alloc("TRIX(CLOSE, 14)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        g.finish();
    }
}

criterion_group!(
    benches,
    bench_sma,
    bench_ema,
    bench_wma,
    bench_dema,
    bench_tema,
    bench_kama,
    bench_rsi,
    bench_macd,
    bench_cci,
    bench_adx,
    bench_atr,
    bench_bbands,
    bench_stoch,
    bench_willr,
    bench_roc,
    bench_mom,
    bench_stddev,
    bench_linear_reg,
    bench_trix,
);
criterion_main!(benches);
