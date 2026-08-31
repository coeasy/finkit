use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use finkit::formula::{FormulaContext, FormulaEngine};
use finkit::indicators;
use finkit::math::moving_avg;
use finkit::streaming::indicators::*;
use finkit::streaming::StreamingIndicator;
use ndarray::Array1;

const NEW_IND_DATA_LEN: usize = 10_000;

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

fn bench_native_sma(c: &mut Criterion) {
    let mut group = c.benchmark_group("native_SMA");
    for size in [10_000, 100_000, 500_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| black_box(indicators::sma(&close, 20).unwrap()))
        });
    }
    group.finish();
}

fn bench_native_ema(c: &mut Criterion) {
    let mut group = c.benchmark_group("native_EMA");
    for size in [10_000, 100_000, 500_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| black_box(indicators::ema(&close, 12).unwrap()))
        });
    }
    group.finish();
}

fn bench_native_rsi(c: &mut Criterion) {
    let mut group = c.benchmark_group("native_RSI");
    for size in [10_000, 100_000, 500_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| black_box(indicators::rsi(&close, 14).unwrap()))
        });
    }
    group.finish();
}

fn bench_native_macd(c: &mut Criterion) {
    let mut group = c.benchmark_group("native_MACD");
    for size in [10_000, 100_000, 500_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| black_box(indicators::macd(&close, 12, 26, 9).unwrap()))
        });
    }
    group.finish();
}

fn bench_native_boll(c: &mut Criterion) {
    let mut group = c.benchmark_group("native_BOLL");
    for size in [10_000, 100_000, 500_000] {
        let (_, _, _, close, _) = create_ohlcv_data(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| black_box(indicators::bbands(&close, 20, 2.0, 2.0).unwrap()))
        });
    }
    group.finish();
}

fn bench_native_atr(c: &mut Criterion) {
    let mut group = c.benchmark_group("native_ATR");
    for size in [10_000, 100_000, 500_000] {
        let (_, high, low, close, _) = create_ohlcv_data(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| black_box(indicators::atr(&high, &low, &close, 14).unwrap()))
        });
    }
    group.finish();
}

fn bench_formula_engine(c: &mut Criterion) {
    let mut group = c.benchmark_group("formula_engine");
    let formulas = [
        ("SMA_20", "MA(CLOSE, 20)"),
        ("EMA_12", "EMA(CLOSE, 12)"),
        ("RSI_14", "RSI(C,14)"),
        ("MACD", "MACD(C,12,26)"),
        ("BOLL", "BOLL(C,20,2)"),
        ("ATR", "ATR_ENHANCED(H,L,C,14)"),
    ];
    for size in [10_000, 100_000] {
        for (name, formula) in &formulas {
            let id = format!("{}/{}", name, size);
            let mut engine = FormulaEngine::new();
            group.bench_function(&id, |b| {
                b.iter_batched(
                    || create_ctx(size),
                    |mut ctx| {
                        let _ = black_box(engine.eval(formula, &mut ctx).unwrap());
                    },
                    criterion::BatchSize::SmallInput,
                )
            });
        }
    }
    group.finish();
}

fn bench_streaming_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming");

    for size in [10_000, 100_000, 500_000] {
        let (_, high, low, close, _) = create_ohlcv_data(size);

        group.bench_with_input(BenchmarkId::new("SMA_20", size), &size, |b, _| {
            b.iter(|| {
                let mut ind = StreamingSma::new(20);
                for &v in &close {
                    black_box(ind.next(v));
                }
            })
        });

        group.bench_with_input(BenchmarkId::new("EMA_12", size), &size, |b, _| {
            b.iter(|| {
                let mut ind = StreamingEma::new(12);
                for &v in &close {
                    black_box(ind.next(v));
                }
            })
        });

        group.bench_with_input(BenchmarkId::new("RSI_14", size), &size, |b, _| {
            b.iter(|| {
                let mut ind = StreamingRsi::new(14);
                for &v in &close {
                    black_box(ind.next(v));
                }
            })
        });

        group.bench_with_input(BenchmarkId::new("MACD_12_26_9", size), &size, |b, _| {
            b.iter(|| {
                let mut ind = StreamingMacd::new(12, 26, 9);
                for &v in &close {
                    black_box(ind.next(v));
                }
            })
        });

        group.bench_with_input(BenchmarkId::new("BOLL_20", size), &size, |b, _| {
            b.iter(|| {
                let mut ind = StreamingBoll::new(20, 2.0, 2.0);
                for &v in &close {
                    black_box(ind.next(v));
                }
            })
        });

        let ohlc: Vec<(f64, f64, f64)> = high
            .iter()
            .zip(low.iter())
            .zip(close.iter())
            .map(|((&h, &l), &c)| (h, l, c))
            .collect();
        group.bench_with_input(BenchmarkId::new("ATR_14", size), &size, |b, _| {
            b.iter(|| {
                let mut ind = StreamingAtr::new(14);
                for &(h, l, cl) in &ohlc {
                    black_box(ind.next((h, l, cl)));
                }
            })
        });
    }
    group.finish();
}

fn bench_native_china(c: &mut Criterion) {
    let mut group = c.benchmark_group("native_china");
    let (open, high, low, close, volume) = create_ohlcv_data(NEW_IND_DATA_LEN);

    group.bench_function("KDJ_9_3_3", |b| {
        b.iter(|| black_box(indicators::kdj(&high, &low, &close, 9, 3, 3).unwrap()))
    });
    group.bench_function("BIAS_6", |b| {
        b.iter(|| black_box(indicators::bias(&close, 6).unwrap()))
    });
    group.bench_function("PSY_12", |b| {
        b.iter(|| black_box(indicators::psy(&close, 12).unwrap()))
    });
    group.bench_function("VR_26", |b| {
        b.iter(|| black_box(indicators::vr(&close, &volume, 26).unwrap()))
    });
    group.bench_function("CR_26", |b| {
        b.iter(|| black_box(indicators::cr(&high, &low, &close, 26).unwrap()))
    });
    group.bench_function("DPO_20", |b| {
        b.iter(|| black_box(indicators::dpo(&close, 20).unwrap()))
    });
    group.bench_function("AR_26", |b| {
        b.iter(|| black_box(indicators::ar(&open, &high, &low, 26).unwrap()))
    });
    group.bench_function("BR_26", |b| {
        b.iter(|| black_box(indicators::br(&high, &low, &close, 26).unwrap()))
    });
    group.bench_function("DMA_10_50_10", |b| {
        b.iter(|| black_box(indicators::dma(&close, 10, 50, 10).unwrap()))
    });
    group.bench_function("ENE_10", |b| {
        b.iter(|| black_box(indicators::ene(&close, 10, 11.0, 9.0).unwrap()))
    });
    group.bench_function("EXPMA_12_50", |b| {
        b.iter(|| black_box(indicators::expma(&close, 12, 50).unwrap()))
    });
    group.finish();
}

fn bench_native_momentum_ext(c: &mut Criterion) {
    let mut group = c.benchmark_group("native_momentum_ext");
    let (_, high, low, close, _) = create_ohlcv_data(NEW_IND_DATA_LEN);

    group.bench_function("AO_5_34", |b| {
        b.iter(|| black_box(indicators::ao(&high, &low, 5, 34).unwrap()))
    });
    group.bench_function("Fisher_10", |b| {
        b.iter(|| black_box(indicators::fisher(&high, &low, 10).unwrap()))
    });
    group.bench_function("TSI_25_13", |b| {
        b.iter(|| black_box(indicators::tsi(&close, 25, 13).unwrap()))
    });
    group.bench_function("Coppock_10_14_11", |b| {
        b.iter(|| black_box(indicators::coppock(&close, 10, 14, 11).unwrap()))
    });
    group.bench_function("KST", |b| {
        b.iter(|| black_box(indicators::kst(&close, 10, 15, 20, 30, 10, 10, 10, 15, 9).unwrap()))
    });
    group.bench_function("STC_23_50_10", |b| {
        b.iter(|| black_box(indicators::stc(&close, 23, 50, 10).unwrap()))
    });
    group.bench_function("CHOP_14", |b| {
        b.iter(|| black_box(indicators::chop(&high, &low, &close, 14).unwrap()))
    });
    group.finish();
}

fn bench_native_volume_ext(c: &mut Criterion) {
    let mut group = c.benchmark_group("native_volume_ext");
    let (_, high, low, close, volume) = create_ohlcv_data(NEW_IND_DATA_LEN);

    group.bench_function("CMF_20", |b| {
        b.iter(|| black_box(indicators::cmf(&high, &low, &close, &volume, 20).unwrap()))
    });
    group.bench_function("ForceIndex_13", |b| {
        b.iter(|| black_box(indicators::force_index(&close, &volume, 13).unwrap()))
    });
    group.bench_function("EOM_14", |b| {
        b.iter(|| black_box(indicators::eom(&high, &low, &volume, 14).unwrap()))
    });
    group.bench_function("NVI", |b| {
        b.iter(|| black_box(indicators::nvi(&close, &volume).unwrap()))
    });
    group.bench_function("PVI", |b| {
        b.iter(|| black_box(indicators::pvi(&close, &volume).unwrap()))
    });
    group.bench_function("PVT", |b| {
        b.iter(|| black_box(indicators::pvt(&close, &volume).unwrap()))
    });
    group.finish();
}

fn bench_native_volatility_ext(c: &mut Criterion) {
    let mut group = c.benchmark_group("native_volatility_ext");
    let (open, high, low, close, _) = create_ohlcv_data(NEW_IND_DATA_LEN);

    group.bench_function("MassIndex_25_9", |b| {
        b.iter(|| black_box(indicators::mass_index(&high, &low, 25, 9).unwrap()))
    });
    group.bench_function("UlcerIndex_14", |b| {
        b.iter(|| black_box(indicators::ulcer_index(&close, 14).unwrap()))
    });
    group.bench_function("RVI_10", |b| {
        b.iter(|| black_box(indicators::rvi(&open, &high, &low, &close, 10).unwrap()))
    });
    group.finish();
}

fn bench_native_chart(c: &mut Criterion) {
    let mut group = c.benchmark_group("native_chart");
    let (open, high, low, close, _) = create_ohlcv_data(NEW_IND_DATA_LEN);

    group.bench_function("HeikinAshi", |b| {
        b.iter(|| black_box(indicators::heikin_ashi(&open, &high, &low, &close).unwrap()))
    });
    group.bench_function("ZigZag_5", |b| {
        b.iter(|| black_box(indicators::zigzag(&high, &low, 5.0).unwrap()))
    });
    group.finish();
}

fn bench_native_moving_avg(c: &mut Criterion) {
    let mut group = c.benchmark_group("native_moving_avg");
    let (_, _, _, close, volume) = create_ohlcv_data(NEW_IND_DATA_LEN);

    group.bench_function("HMA_16", |b| {
        b.iter(|| black_box(moving_avg::hma(&close, 16).unwrap()))
    });
    group.bench_function("ALMA_9", |b| {
        b.iter(|| black_box(moving_avg::alma(&close, 9, 6.0, 0.85).unwrap()))
    });
    group.bench_function("McGinley_14", |b| {
        b.iter(|| black_box(moving_avg::mcginley(&close, 14).unwrap()))
    });
    group.bench_function("ZLEMA_20", |b| {
        b.iter(|| black_box(moving_avg::zlema(&close, 20).unwrap()))
    });
    group.bench_function("VIDYA_14_9", |b| {
        b.iter(|| black_box(moving_avg::vidya(&close, 14, 9).unwrap()))
    });
    group.bench_function("VWMA_20", |b| {
        b.iter(|| black_box(moving_avg::vwma(&close, &volume, 20).unwrap()))
    });
    group.finish();
}

fn bench_into_variants(c: &mut Criterion) {
    let mut group = c.benchmark_group("into_variants");
    let size = 100_000;
    let (_, high, low, close, _) = create_ohlcv_data(size);
    let mut sma_out = vec![0.0; size];
    let mut ema_out = vec![0.0; size];
    let mut rsi_out = vec![0.0; size];
    let mut atr_out = vec![0.0; size];
    let mut k_out = vec![0.0; size];
    let mut d_out = vec![0.0; size];

    group.bench_function("sma_allocating", |b| {
        b.iter(|| black_box(indicators::sma(&close, 20).unwrap()))
    });
    group.bench_function("sma_into", |b| {
        b.iter(|| {
            indicators::sma_into(&close, 20, &mut sma_out).unwrap();
            black_box(sma_out[size - 1])
        })
    });
    group.bench_function("ema_allocating", |b| {
        b.iter(|| black_box(indicators::ema(&close, 12).unwrap()))
    });
    group.bench_function("ema_into", |b| {
        b.iter(|| {
            indicators::ema_into(&close, 12, &mut ema_out).unwrap();
            black_box(ema_out[size - 1])
        })
    });
    group.bench_function("rsi_allocating", |b| {
        b.iter(|| black_box(indicators::rsi(&close, 14).unwrap()))
    });
    group.bench_function("rsi_into", |b| {
        b.iter(|| {
            indicators::rsi_into(&close, 14, &mut rsi_out).unwrap();
            black_box(rsi_out[size - 1])
        })
    });
    group.bench_function("atr_allocating", |b| {
        b.iter(|| black_box(indicators::atr(&high, &low, &close, 14).unwrap()))
    });
    group.bench_function("atr_into", |b| {
        b.iter(|| {
            indicators::atr_into(&high, &low, &close, 14, &mut atr_out).unwrap();
            black_box(atr_out[size - 1])
        })
    });
    group.bench_function("stoch_allocating", |b| {
        b.iter(|| black_box(indicators::stoch(&high, &low, &close, 14, 3, 3).unwrap()))
    });
    group.bench_function("stoch_into", |b| {
        b.iter(|| {
            indicators::stoch_into(&high, &low, &close, 14, 3, 3, &mut k_out, &mut d_out).unwrap();
            black_box(k_out[size - 1] + d_out[size - 1])
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_native_sma,
    bench_native_ema,
    bench_native_rsi,
    bench_native_macd,
    bench_native_boll,
    bench_native_atr,
    bench_formula_engine,
    bench_streaming_all,
    bench_native_china,
    bench_native_momentum_ext,
    bench_native_volume_ext,
    bench_native_volatility_ext,
    bench_native_chart,
    bench_native_moving_avg,
    bench_into_variants,
);
criterion_main!(benches);
