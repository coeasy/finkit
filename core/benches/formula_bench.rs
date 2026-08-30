use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use ndarray::Array1;
use finkit::formula::{
    compile_to_bytecode, parse_formula, BytecodeVM, FormulaContext, FormulaEngine, FormulaExecutor,
    FormulaOptimizer, JitCompiler, SimdOps,
};

fn native_sma(data: &[f64], period: usize) -> Vec<f64> {
    let mut result = vec![f64::NAN; data.len()];
    if period == 0 || data.len() < period {
        return result;
    }
    for i in (period - 1)..data.len() {
        let sum: f64 = data[i + 1 - period..=i].iter().sum();
        result[i] = sum / period as f64;
    }
    result
}

fn native_ema(data: &[f64], period: usize) -> Vec<f64> {
    let mut result = vec![f64::NAN; data.len()];
    if period == 0 || data.len() < period {
        return result;
    }
    let k = 2.0 / (period as f64 + 1.0);
    let sum: f64 = data[..period].iter().sum();
    result[period - 1] = sum / period as f64;
    for i in period..data.len() {
        result[i] = data[i] * k + result[i - 1] * (1.0 - k);
    }
    result
}

fn native_rsi(data: &[f64], period: usize) -> Vec<f64> {
    let mut result = vec![f64::NAN; data.len()];
    if data.len() < period + 1 {
        return result;
    }
    let mut gains = 0.0f64;
    let mut losses = 0.0f64;
    for i in 1..=period {
        let diff = data[i] - data[i - 1];
        if diff > 0.0 {
            gains += diff;
        } else {
            losses += diff.abs();
        }
    }
    let avg_gain = gains / period as f64;
    let avg_loss = losses / period as f64;
    if avg_loss == 0.0 {
        result[period] = 100.0;
    } else {
        let rs = avg_gain / avg_loss;
        result[period] = 100.0 - (100.0 / (1.0 + rs));
    }
    let mut avg_gain = avg_gain;
    let mut avg_loss = avg_loss;
    for i in (period + 1)..data.len() {
        let diff = data[i] - data[i - 1];
        let gain = if diff > 0.0 { diff } else { 0.0 };
        let loss = if diff < 0.0 { diff.abs() } else { 0.0 };
        avg_gain = (avg_gain * (period as f64 - 1.0) + gain) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + loss) / period as f64;
        if avg_loss == 0.0 {
            result[i] = 100.0;
        } else {
            let rs = avg_gain / avg_loss;
            result[i] = 100.0 - (100.0 / (1.0 + rs));
        }
    }
    result
}

fn native_macd(
    data: &[f64],
    fast: usize,
    slow: usize,
    signal: usize,
) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let dif = native_ema(data, fast)
        .iter()
        .zip(native_ema(data, slow).iter())
        .map(|(&f, &s)| {
            if f.is_nan() || s.is_nan() {
                f64::NAN
            } else {
                f - s
            }
        })
        .collect::<Vec<_>>();
    let dea = native_ema(&dif, signal);
    let macd_hist: Vec<f64> = dif
        .iter()
        .zip(dea.iter())
        .map(|(&d, &e)| {
            if d.is_nan() || e.is_nan() {
                f64::NAN
            } else {
                (d - e) * 2.0
            }
        })
        .collect();
    (dif, dea, macd_hist)
}

fn native_boll(data: &[f64], period: usize, mult: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mid = native_sma(data, period);
    let mut upper = vec![f64::NAN; data.len()];
    let mut lower = vec![f64::NAN; data.len()];
    for i in (period - 1)..data.len() {
        let window = &data[i + 1 - period..=i];
        let mean = mid[i];
        let variance: f64 = window.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / period as f64;
        let std_dev = variance.sqrt();
        upper[i] = mean + mult * std_dev;
        lower[i] = mean - mult * std_dev;
    }
    (mid, upper, lower)
}

#[allow(clippy::type_complexity)]
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

fn get_close_vec(len: usize) -> Vec<f64> {
    let (_, _, _, close, _) = create_ohlcv_data(len);
    close
}

fn benchmark_formula_vs_native(c: &mut Criterion) {
    let mut group = c.benchmark_group("formula_vs_native");

    for data_len in [1000, 10000, 100000] {
        let close = get_close_vec(data_len);

        group.bench_with_input(
            BenchmarkId::new("native_SMA_20", data_len),
            &data_len,
            |b, _| {
                b.iter(|| {
                    let _ = black_box(native_sma(&close, 20));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_SMA_20", data_len),
            &data_len,
            |b, _| {
                let mut engine = FormulaEngine::new();
                let formula = engine.compile("MA(CLOSE, 20)").unwrap();
                b.iter_batched(
                    || create_ctx(data_len),
                    |mut ctx| {
                        let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("native_EMA_12", data_len),
            &data_len,
            |b, _| {
                b.iter(|| {
                    let _ = black_box(native_ema(&close, 12));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_EMA_12", data_len),
            &data_len,
            |b, _| {
                let mut engine = FormulaEngine::new();
                let formula = engine.compile("EMA(CLOSE, 12)").unwrap();
                b.iter_batched(
                    || create_ctx(data_len),
                    |mut ctx| {
                        let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("native_RSI_14", data_len),
            &data_len,
            |b, _| {
                b.iter(|| {
                    let _ = black_box(native_rsi(&close, 14));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_RSI_14", data_len),
            &data_len,
            |b, _| {
                let mut engine = FormulaEngine::new();
                let formula = engine.compile("RSI(CLOSE, 14)").unwrap();
                b.iter_batched(
                    || create_ctx(data_len),
                    |mut ctx| {
                        let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("native_MACD", data_len),
            &data_len,
            |b, _| {
                b.iter(|| {
                    let _ = black_box(native_macd(&close, 12, 26, 9));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_MACD", data_len),
            &data_len,
            |b, _| {
                let source =
                    "DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26); DEA := EMA(DIF, 9); (DIF - DEA) * 2";
                let mut engine = FormulaEngine::new();
                let formula = engine.compile(source).unwrap();
                b.iter_batched(
                    || create_ctx(data_len),
                    |mut ctx| {
                        let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("native_BOLL", data_len),
            &data_len,
            |b, _| {
                b.iter(|| {
                    let _ = black_box(native_boll(&close, 20, 2.0));
                })
            },
        );

        group.bench_with_input(BenchmarkId::new("formula_BOLL", data_len), &data_len, |b, _| {
            let source = "MID := MA(CLOSE, 20); UPPER := MID + 2 * STD(CLOSE, 20); LOWER := MID - 2 * STD(CLOSE, 20); (UPPER + LOWER) / 2";
            let mut engine = FormulaEngine::new();
            let formula = engine.compile(source).unwrap();
            b.iter_batched(
                || create_ctx(data_len),
                |mut ctx| {
                    let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });

        group.bench_with_input(
            BenchmarkId::new("formula_RSI_builtin", data_len),
            &data_len,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(data_len),
                    |mut ctx| {
                        let _ = black_box(engine.eval("RSI(C,14)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_MACD_builtin", data_len),
            &data_len,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(data_len),
                    |mut ctx| {
                        let _ = black_box(engine.eval("MACD(C,12,26)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_BOLL_builtin", data_len),
            &data_len,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(data_len),
                    |mut ctx| {
                        let _ = black_box(engine.eval("BOLL(C,20,2)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

fn benchmark_execution_modes(c: &mut Criterion) {
    let mut group = c.benchmark_group("execution_modes");
    let data_len = 10000;
    // Use a single-expression formula so dead-code elimination in the optimizer
    // does not strip intermediate assignments required by multi-line MACD.
    let source = "EMA(CLOSE, 12) - EMA(CLOSE, 26)";

    let ast = parse_formula(source).unwrap();
    let optimized_ast = FormulaOptimizer::optimize(&ast);
    let bytecode = compile_to_bytecode(&ast, source).unwrap();
    let mut jit = JitCompiler::new();
    let optimized_bc = jit.compile(bytecode.clone());

    group.bench_function("AST_interpreter", |b| {
        let executor = FormulaExecutor::new();
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(executor.execute(&ast, &mut ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("Bytecode_VM", |b| {
        b.iter_batched(
            || create_ctx(data_len),
            |ctx| {
                let mut vm = BytecodeVM::new();
                let _ = black_box(vm.execute(&bytecode, &ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("Optimized_AST", |b| {
        let executor = FormulaExecutor::new();
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(executor.execute(&optimized_ast, &mut ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("JIT_optimized", |b| {
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(jit.execute(&optimized_bc, &mut ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn benchmark_function_categories(c: &mut Criterion) {
    let data_len = 10000;

    let mut ma_group = c.benchmark_group("func_moving_averages");
    for (name, formula) in [
        ("MA_20", "MA(CLOSE, 20)"),
        ("EMA_12", "EMA(CLOSE, 12)"),
        ("SMA_14", "SMA(CLOSE, 14, 1)"),
        ("DEMA_20", "DEMA(CLOSE, 20)"),
        ("TEMA_20", "TEMA(CLOSE, 20)"),
        ("KAMA_10", "KAMA(CLOSE, 10)"),
        ("T3_5", "T3(CLOSE, 5)"),
    ] {
        ma_group.bench_function(name, |b| {
            let mut engine = FormulaEngine::new();
            let f = engine.compile(formula).unwrap();
            b.iter_batched(
                || create_ctx(data_len),
                |mut ctx| {
                    let _ = black_box(engine.execute(&f, &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });
    }
    ma_group.finish();

    let mut trend_group = c.benchmark_group("func_trend");
    for (name, formula) in [
        ("ADX_14", "ADX(H,L,C,14)"),
        ("DMI_14", "DMI(H,L,C,14)"),
        ("CCI_14", "CCI(H,L,C,14)"),
        ("WILLR_14", "WILLR(H,L,C,14)"),
        ("CMO_14", "CMO(CLOSE, 14)"),
    ] {
        trend_group.bench_function(name, |b| {
            let mut engine = FormulaEngine::new();
            let f = engine.compile(formula).unwrap();
            b.iter_batched(
                || create_ctx(data_len),
                |mut ctx| {
                    let _ = black_box(engine.execute(&f, &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });
    }
    trend_group.finish();

    let mut osc_group = c.benchmark_group("func_oscillators");
    for (name, formula) in [
        ("RSI_14", "RSI(CLOSE, 14)"),
        ("STOCH_14_3_3", "STOCH(H,L,C,14,3,3)"),
        (
            "MACD_12_26_9",
            "DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26); DEA := EMA(DIF, 9); (DIF - DEA) * 2",
        ),
        ("DPO_20", "DPO(CLOSE, 20)"),
    ] {
        osc_group.bench_function(name, |b| {
            let mut engine = FormulaEngine::new();
            let f = engine.compile(formula).unwrap();
            b.iter_batched(
                || create_ctx(data_len),
                |mut ctx| {
                    let _ = black_box(engine.execute(&f, &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });
    }
    osc_group.finish();

    let mut vol_group = c.benchmark_group("func_volume");
    for (name, formula) in [
        ("OBV_ENHANCED", "OBV_ENHANCED(C,V)"),
        ("MFI_14", "MFI(H,L,C,V,14)"),
        ("AD", "AD(H,L,C,V)"),
        ("ADOSC_3_10", "ADOSC(H,L,C,V,3,10)"),
    ] {
        vol_group.bench_function(name, |b| {
            let mut engine = FormulaEngine::new();
            let f = engine.compile(formula).unwrap();
            b.iter_batched(
                || create_ctx(data_len),
                |mut ctx| {
                    let _ = black_box(engine.execute(&f, &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });
    }
    vol_group.finish();

    let mut volat_group = c.benchmark_group("func_volatility");
    for (name, formula) in [
        ("ATR_ENHANCED_14", "ATR_ENHANCED(H,L,C,14)"),
        ("BOLL_ENHANCED_20", "BOLL_ENHANCED(C,20,2)"),
        ("NATR_14", "NATR(H,L,C,14)"),
        ("HISTVOL_20", "HISTVOL(C,20)"),
    ] {
        volat_group.bench_function(name, |b| {
            let mut engine = FormulaEngine::new();
            let f = engine.compile(formula).unwrap();
            b.iter_batched(
                || create_ctx(data_len),
                |mut ctx| {
                    let _ = black_box(engine.execute(&f, &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });
    }
    volat_group.finish();

    let mut stat_group = c.benchmark_group("func_statistics");
    for (name, formula) in [
        ("STD_20", "STD(CLOSE, 20)"),
        ("VAR_20", "VAR(CLOSE, 20)"),
        ("CORREL_20", "CORREL(CLOSE, OPEN, 20)"),
        ("BETA_20", "BETA(CLOSE, OPEN, 20)"),
    ] {
        stat_group.bench_function(name, |b| {
            let mut engine = FormulaEngine::new();
            let f = engine.compile(formula).unwrap();
            b.iter_batched(
                || create_ctx(data_len),
                |mut ctx| {
                    let _ = black_box(engine.execute(&f, &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });
    }
    stat_group.finish();
}

fn benchmark_data_size_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_size_scaling");

    for data_len in [100, 500, 1000, 5000, 10000, 50000, 100000] {
        let mut engine = FormulaEngine::new();
        let formula = engine.compile("MA(CLOSE, 20)").unwrap();

        group.bench_with_input(
            BenchmarkId::new("MA_CLOSE_20", data_len),
            &data_len,
            |b, _| {
                b.iter_batched(
                    || create_ctx(data_len),
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

fn benchmark_simd_vs_scalar(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_vs_scalar");
    let data_len = 100000;

    let arr_a: Vec<f64> = (0..data_len)
        .map(|i| (i as f64 * 0.1).sin() * 100.0 + 200.0)
        .collect();
    let arr_b: Vec<f64> = (0..data_len)
        .map(|i| (i as f64 * 0.15).cos() * 50.0 + 150.0)
        .collect();

    group.bench_function("SimdOps_add", |bencher| {
        let mut result = vec![0.0; data_len];
        bencher.iter(|| {
            SimdOps::add(&arr_a, &arr_b, &mut result);
            black_box(&result);
        })
    });

    group.bench_function("scalar_add", |bencher| {
        bencher.iter(|| {
            let result: Vec<f64> = arr_a
                .iter()
                .zip(arr_b.iter())
                .map(|(&x, &y)| x + y)
                .collect();
            black_box(&result);
        })
    });

    group.bench_function("SimdOps_mul", |bencher| {
        let mut result = vec![0.0; data_len];
        bencher.iter(|| {
            SimdOps::mul(&arr_a, &arr_b, &mut result);
            black_box(&result);
        })
    });

    group.bench_function("scalar_mul", |bencher| {
        bencher.iter(|| {
            let result: Vec<f64> = arr_a
                .iter()
                .zip(arr_b.iter())
                .map(|(&x, &y)| x * y)
                .collect();
            black_box(&result);
        })
    });

    group.bench_function("SimdOps_sma", |bencher| {
        let mut result = vec![0.0; data_len];
        bencher.iter(|| {
            SimdOps::sma(&arr_a, 20, &mut result);
            black_box(&result);
        })
    });

    group.bench_function("scalar_sma", |bencher| {
        bencher.iter(|| {
            let result = native_sma(&arr_a, 20);
            black_box(&result);
        })
    });

    group.bench_function("SimdOps_ema", |bencher| {
        let mut result = vec![0.0; data_len];
        bencher.iter(|| {
            SimdOps::ema(&arr_a, 12, &mut result);
            black_box(&result);
        })
    });

    group.bench_function("scalar_ema", |bencher| {
        bencher.iter(|| {
            let result = native_ema(&arr_a, 12);
            black_box(&result);
        })
    });

    let arr_mod_b: Vec<f64> = (0..data_len)
        .map(|i| (i % 7 + 1) as f64)
        .collect();

    group.bench_function("SimdOps_mod", |bencher| {
        let mut result = vec![0.0; data_len];
        bencher.iter(|| {
            SimdOps::simd_mod(&arr_a, &arr_mod_b, &mut result);
            black_box(&result);
        })
    });

    group.bench_function("scalar_mod", |bencher| {
        bencher.iter(|| {
            let result: Vec<f64> = arr_a
                .iter()
                .zip(arr_mod_b.iter())
                .map(|(&x, &y)| {
                    if y.abs() < 1e-15 {
                        f64::NAN
                    } else {
                        x - (x / y).floor() * y
                    }
                })
                .collect();
            black_box(&result);
        })
    });

    let arr_pow_b: Vec<f64> = (0..data_len)
        .map(|i| ((i % 5 + 1) as f64 * 0.5))
        .collect();

    group.bench_function("SimdOps_pow", |bencher| {
        let mut result = vec![0.0; data_len];
        bencher.iter(|| {
            SimdOps::simd_pow(&arr_a, &arr_pow_b, &mut result);
            black_box(&result);
        })
    });

    group.bench_function("scalar_pow", |bencher| {
        bencher.iter(|| {
            let result: Vec<f64> = arr_a
                .iter()
                .zip(arr_pow_b.iter())
                .map(|(&x, &y)| x.powf(y))
                .collect();
            black_box(&result);
        })
    });

    group.finish();
}

fn benchmark_func_price(c: &mut Criterion) {
    let data_len = 10000;
    let mut group = c.benchmark_group("func_price");
    for (name, formula) in [
        ("AVGPRICE", "AVGPRICE(O,H,L,C)"),
        ("MEDPRICE", "MEDPRICE(H,L)"),
        ("TYPPRICE", "TYPPRICE(H,L,C)"),
        ("SAR", "SAR(H,L,0.02,0.2)"),
    ] {
        group.bench_function(name, |b| {
            let mut engine = FormulaEngine::new();
            let f = engine.compile(formula).unwrap();
            b.iter_batched(
                || create_ctx(data_len),
                |mut ctx| {
                    let _ = black_box(engine.execute(&f, &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

fn benchmark_func_bar_stats(c: &mut Criterion) {
    let data_len = 10000;
    let mut group = c.benchmark_group("func_bar_stats");
    for (name, formula) in [
        ("BARSLAST", "BARSLAST(C>O)"),
        ("BACKSET", "BACKSET(C>O,5)"),
        ("FILTER", "FILTER(C>O,5)"),
    ] {
        group.bench_function(name, |b| {
            let mut engine = FormulaEngine::new();
            let f = engine.compile(formula).unwrap();
            b.iter_batched(
                || create_ctx(data_len),
                |mut ctx| {
                    let _ = black_box(engine.execute(&f, &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

fn benchmark_func_standard_indicators(c: &mut Criterion) {
    let data_len = 10000;
    let mut group = c.benchmark_group("func_standard_indicators");
    for (name, formula) in [
        ("RSI", "RSI(C,14)"),
        ("MACD", "MACD(C,12,26)"),
        ("BOLL", "BOLL(C,20,2)"),
        ("KDJ", "KDJ(H,L,C,9,3,3)"),
        ("DMI", "DMI(H,L,C,14)"),
    ] {
        group.bench_function(name, |b| {
            let mut engine = FormulaEngine::new();
            let f = engine.compile(formula).unwrap();
            b.iter_batched(
                || create_ctx(data_len),
                |mut ctx| {
                    let _ = black_box(engine.execute(&f, &mut ctx).unwrap());
                },
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

fn benchmark_zero_copy_vs_normal(c: &mut Criterion) {
    let data_len = 10000;
    let mut group = c.benchmark_group("zero_copy_vs_normal");

    group.bench_function("eval_MA_C_5", |b| {
        let mut engine = FormulaEngine::new();
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(engine.eval("MA(C,5)", &mut ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("zero_copy_MA_C_5", |b| {
        let mut engine = FormulaEngine::new();
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(engine.eval_zero_copy("MA(C,5)", &mut ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("zero_copy_cached_MA_C_5", |b| {
        let mut engine = FormulaEngine::new();
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(engine.eval_zero_copy_cached("MA(C,5)", &mut ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("eval_EMA_C_10_plus_MA_C_20", |b| {
        let mut engine = FormulaEngine::new();
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(engine.eval("EMA(C,10)+MA(C,20)", &mut ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("zero_copy_EMA_C_10_plus_MA_C_20", |b| {
        let mut engine = FormulaEngine::new();
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(
                    engine
                        .eval_zero_copy("EMA(C,10)+MA(C,20)", &mut ctx)
                        .unwrap(),
                );
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("zero_copy_cached_EMA_C_10_plus_MA_C_20", |b| {
        let mut engine = FormulaEngine::new();
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(
                    engine
                        .eval_zero_copy_cached("EMA(C,10)+MA(C,20)", &mut ctx)
                        .unwrap(),
                );
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("eval_RSI_C_14", |b| {
        let mut engine = FormulaEngine::new();
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(engine.eval("RSI(C,14)", &mut ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("zero_copy_RSI_C_14", |b| {
        let mut engine = FormulaEngine::new();
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(engine.eval_zero_copy("RSI(C,14)", &mut ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("zero_copy_cached_RSI_C_14", |b| {
        let mut engine = FormulaEngine::new();
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(engine.eval_zero_copy_cached("RSI(C,14)", &mut ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("eval_MACD_complex", |b| {
        let mut engine = FormulaEngine::new();
        let source = "DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26); DEA := EMA(DIF, 9); MACD: (DIF - DEA) * 2";
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(engine.eval(source, &mut ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("zero_copy_MACD_complex", |b| {
        let mut engine = FormulaEngine::new();
        let source = "DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26); DEA := EMA(DIF, 9); MACD: (DIF - DEA) * 2";
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(engine.eval_zero_copy(source, &mut ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("zero_copy_cached_MACD_complex", |b| {
        let mut engine = FormulaEngine::new();
        let source = "DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26); DEA := EMA(DIF, 9); MACD: (DIF - DEA) * 2";
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(engine.eval_zero_copy_cached(source, &mut ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn benchmark_jma(c: &mut Criterion) {
    let mut group = c.benchmark_group("jma");
    let data: Vec<f64> = (0..10_000)
        .map(|i| 100.0 + (i as f64 * 0.1).sin() * 5.0)
        .collect();

    group.bench_function("jma_7_10000", |b| {
        b.iter(|| black_box(finkit::indicators::jma(&data, 7, 0.0, 2.0).unwrap()))
    });

    group.bench_function("jma_14_10000", |b| {
        b.iter(|| black_box(finkit::indicators::jma(&data, 14, 0.0, 2.0).unwrap()))
    });

    group.finish();
}

fn benchmark_optimized_vs_native(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimized_vs_native");
    group.sample_size(50);

    for data_len in [10000, 100000, 1000000] {
        let close = get_close_vec(data_len);

        group.bench_with_input(
            BenchmarkId::new("native_sma_20", data_len),
            &data_len,
            |b, _| {
                b.iter(|| {
                    let _ = black_box(native_sma(&close, 20));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_MA_20_optimized", data_len),
            &data_len,
            |b, _| {
                let mut engine = FormulaEngine::new();
                let formula = engine.compile("MA(CLOSE, 20)").unwrap();
                b.iter_batched(
                    || create_ctx(data_len),
                    |mut ctx| {
                        let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_MA_20_zero_alloc", data_len),
            &data_len,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(data_len),
                    |mut ctx| {
                        let _ = black_box(engine.eval_zero_alloc("MA(CLOSE, 20)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("native_macd", data_len),
            &data_len,
            |b, _| {
                b.iter(|| {
                    let _ = black_box(native_macd(&close, 12, 26, 9));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_MACD_composition", data_len),
            &data_len,
            |b, _| {
                let source =
                    "DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26); DEA := EMA(DIF, 9); (DIF - DEA) * 2";
                let mut engine = FormulaEngine::new();
                let formula = engine.compile(source).unwrap();
                b.iter_batched(
                    || create_ctx(data_len),
                    |mut ctx| {
                        let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_MACD_zero_alloc", data_len),
            &data_len,
            |b, _| {
                let source =
                    "DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26); DEA := EMA(DIF, 9); (DIF - DEA) * 2";
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(data_len),
                    |mut ctx| {
                        let _ = black_box(engine.eval_zero_alloc(source, &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("native_rsi_14", data_len),
            &data_len,
            |b, _| {
                b.iter(|| {
                    let _ = black_box(native_rsi(&close, 14));
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_RSI_14_optimized", data_len),
            &data_len,
            |b, _| {
                let mut engine = FormulaEngine::new();
                let formula = engine.compile("RSI(CLOSE, 14)").unwrap();
                b.iter_batched(
                    || create_ctx(data_len),
                    |mut ctx| {
                        let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(
            BenchmarkId::new("formula_RSI_14_zero_alloc", data_len),
            &data_len,
            |b, _| {
                let mut engine = FormulaEngine::new();
                b.iter_batched(
                    || create_ctx(data_len),
                    |mut ctx| {
                        let _ = black_box(engine.eval_zero_alloc("RSI(CLOSE, 14)", &mut ctx).unwrap());
                    },
                    BatchSize::SmallInput,
                )
            },
        );
    }

    group.finish();
}

fn benchmark_scalar_optimization(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalar_optimization");
    let data_len = 100000;

    group.bench_function("CLOSE_plus_scalar_eval", |b| {
        let mut engine = FormulaEngine::new();
        let formula = engine.compile("CLOSE + 1").unwrap();
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("CLOSE_gt_scalar_eval", |b| {
        let mut engine = FormulaEngine::new();
        let formula = engine.compile("CLOSE > 10.5").unwrap();
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("scalar_plus_scalar_eval", |b| {
        let mut engine = FormulaEngine::new();
        let formula = engine.compile("10 + 20").unwrap();
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("compound_scalar_array_eval", |b| {
        let mut engine = FormulaEngine::new();
        let formula = engine.compile("MA(CLOSE, 5) * 2 + 1").unwrap();
        b.iter_batched(
            || create_ctx(data_len),
            |mut ctx| {
                let _ = black_box(engine.execute(&formula, &mut ctx).unwrap());
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_formula_vs_native,
    benchmark_execution_modes,
    benchmark_function_categories,
    benchmark_data_size_scaling,
    benchmark_simd_vs_scalar,
    benchmark_func_price,
    benchmark_func_bar_stats,
    benchmark_func_standard_indicators,
    benchmark_zero_copy_vs_normal,
    benchmark_jma,
    benchmark_optimized_vs_native,
    benchmark_scalar_optimization,
);
criterion_main!(benches);
