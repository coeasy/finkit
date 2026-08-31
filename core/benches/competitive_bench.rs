//! Competitive benchmark: FTA vs ta-rs (open-source Rust TA library).
//!
//! Compares equivalent indicators head-to-head across batch and streaming modes.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use finkit::indicators;
use finkit::streaming::indicators::*;
use finkit::streaming::StreamingIndicator;

use ta::indicators::{
    AverageTrueRange as TaRsAtr, BollingerBands as TaRsBoll, ExponentialMovingAverage as TaRsEma,
    MovingAverageConvergenceDivergence as TaRsMacd, RelativeStrengthIndex as TaRsRsi,
    SimpleMovingAverage as TaRsSma,
};
use ta::Next;

const DATA_LEN: usize = 10_000;

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

// ============================================================================
// §1: Batch mode — FTA vs ta-rs equivalent indicators
// ============================================================================
fn bench_batch_sma(c: &mut Criterion) {
    let mut group = c.benchmark_group("competitive_batch");
    let (_, _, _, close, _) = create_ohlcv_data(DATA_LEN);

    group.bench_function(BenchmarkId::new("fta_sma_20", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::sma(&close, 20).unwrap()))
    });
    group.bench_function(BenchmarkId::new("ta_rs_sma_20", DATA_LEN), |b| {
        b.iter(|| {
            let mut ind = TaRsSma::new(20).unwrap();
            let mut out = Vec::with_capacity(close.len());
            for &v in &close {
                out.push(ind.next(v));
            }
            black_box(out)
        })
    });
    group.finish();
}

fn bench_batch_ema(c: &mut Criterion) {
    let mut group = c.benchmark_group("competitive_batch");
    let (_, _, _, close, _) = create_ohlcv_data(DATA_LEN);

    group.bench_function(BenchmarkId::new("fta_ema_12", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::ema(&close, 12).unwrap()))
    });
    group.bench_function(BenchmarkId::new("ta_rs_ema_12", DATA_LEN), |b| {
        b.iter(|| {
            let mut ind = TaRsEma::new(12).unwrap();
            let mut out = Vec::with_capacity(close.len());
            for &v in &close {
                out.push(ind.next(v));
            }
            black_box(out)
        })
    });
    group.finish();
}

fn bench_batch_rsi(c: &mut Criterion) {
    let mut group = c.benchmark_group("competitive_batch");
    let (_, _, _, close, _) = create_ohlcv_data(DATA_LEN);

    group.bench_function(BenchmarkId::new("fta_rsi_14", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::rsi(&close, 14).unwrap()))
    });
    group.bench_function(BenchmarkId::new("ta_rs_rsi_14", DATA_LEN), |b| {
        b.iter(|| {
            let mut ind = TaRsRsi::new(14).unwrap();
            let mut out = Vec::with_capacity(close.len());
            for &v in &close {
                out.push(ind.next(v));
            }
            black_box(out)
        })
    });
    group.finish();
}

fn bench_batch_macd(c: &mut Criterion) {
    let mut group = c.benchmark_group("competitive_batch");
    let (_, _, _, close, _) = create_ohlcv_data(DATA_LEN);

    group.bench_function(BenchmarkId::new("fta_macd", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::macd(&close, 12, 26, 9).unwrap()))
    });
    group.bench_function(BenchmarkId::new("ta_rs_macd", DATA_LEN), |b| {
        b.iter(|| {
            let mut ind = TaRsMacd::new(12, 26, 9).unwrap();
            let mut out = Vec::with_capacity(close.len());
            for &v in &close {
                out.push(ind.next(v));
            }
            black_box(out)
        })
    });
    group.finish();
}

fn bench_batch_bbands(c: &mut Criterion) {
    let mut group = c.benchmark_group("competitive_batch");
    let (_, _, _, close, _) = create_ohlcv_data(DATA_LEN);

    group.bench_function(BenchmarkId::new("fta_bbands_20", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::bbands(&close, 20, 2.0, 2.0).unwrap()))
    });
    group.bench_function(BenchmarkId::new("ta_rs_bbands_20", DATA_LEN), |b| {
        b.iter(|| {
            let mut ind = TaRsBoll::new(20, 2.0).unwrap();
            let mut out = Vec::with_capacity(close.len());
            for &v in &close {
                out.push(ind.next(v));
            }
            black_box(out)
        })
    });
    group.finish();
}

fn bench_batch_atr(c: &mut Criterion) {
    let mut group = c.benchmark_group("competitive_batch");
    let (_, high, low, close, _) = create_ohlcv_data(DATA_LEN);

    group.bench_function(BenchmarkId::new("fta_atr_14", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::atr(&high, &low, &close, 14).unwrap()))
    });
    group.bench_function(BenchmarkId::new("ta_rs_atr_14", DATA_LEN), |b| {
        b.iter(|| {
            let mut ind = TaRsAtr::new(14).unwrap();
            let mut out = Vec::with_capacity(close.len());
            for i in 0..close.len() {
                let bar = ta::DataItem::builder()
                    .open(close[i])
                    .high(high[i])
                    .low(low[i])
                    .close(close[i])
                    .volume(1000.0)
                    .build()
                    .unwrap();
                out.push(ind.next(&bar));
            }
            black_box(out)
        })
    });
    group.finish();
}

// ============================================================================
// §2: Streaming mode — FTA streaming vs ta-rs (both are streaming)
// ============================================================================
fn bench_streaming_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("competitive_streaming");
    let (_, _, _, close, _) = create_ohlcv_data(DATA_LEN);

    group.bench_function(BenchmarkId::new("fta_streaming_sma_20", DATA_LEN), |b| {
        b.iter(|| {
            let mut ind = StreamingSma::new(20);
            for &v in &close {
                black_box(ind.next(v));
            }
        })
    });
    group.bench_function(BenchmarkId::new("ta_rs_sma_20", DATA_LEN), |b| {
        b.iter(|| {
            let mut ind = TaRsSma::new(20).unwrap();
            for &v in &close {
                black_box(ind.next(v));
            }
        })
    });

    group.bench_function(BenchmarkId::new("fta_streaming_ema_12", DATA_LEN), |b| {
        b.iter(|| {
            let mut ind = StreamingEma::new(12);
            for &v in &close {
                black_box(ind.next(v));
            }
        })
    });
    group.bench_function(BenchmarkId::new("ta_rs_ema_12", DATA_LEN), |b| {
        b.iter(|| {
            let mut ind = TaRsEma::new(12).unwrap();
            for &v in &close {
                black_box(ind.next(v));
            }
        })
    });

    group.bench_function(BenchmarkId::new("fta_streaming_rsi_14", DATA_LEN), |b| {
        b.iter(|| {
            let mut ind = StreamingRsi::new(14);
            for &v in &close {
                black_box(ind.next(v));
            }
        })
    });
    group.bench_function(BenchmarkId::new("ta_rs_rsi_14", DATA_LEN), |b| {
        b.iter(|| {
            let mut ind = TaRsRsi::new(14).unwrap();
            for &v in &close {
                black_box(ind.next(v));
            }
        })
    });

    group.finish();
}

// ============================================================================
// §3: Additional batch indicators (no ta-rs equivalent, FTA standalone)
// ============================================================================
fn bench_fta_additional(c: &mut Criterion) {
    let mut group = c.benchmark_group("fta_additional");
    let (_, high, low, close, _) = create_ohlcv_data(DATA_LEN);

    group.bench_function(BenchmarkId::new("fta_wma_20", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::wma(&close, 20).unwrap()))
    });
    group.bench_function(BenchmarkId::new("fta_dema_20", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::dema(&close, 20).unwrap()))
    });
    group.bench_function(BenchmarkId::new("fta_tema_20", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::tema(&close, 20).unwrap()))
    });
    group.bench_function(BenchmarkId::new("fta_roc_10", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::roc(&close, 10).unwrap()))
    });
    group.bench_function(BenchmarkId::new("fta_mom_10", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::mom(&close, 10).unwrap()))
    });
    group.bench_function(BenchmarkId::new("fta_cci_14", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::cci(&high, &low, &close, 14).unwrap()))
    });
    group.bench_function(BenchmarkId::new("fta_willr_14", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::willr(&high, &low, &close, 14).unwrap()))
    });
    group.bench_function(BenchmarkId::new("fta_adx_14", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::adx(&high, &low, &close, 14).unwrap()))
    });
    group.bench_function(BenchmarkId::new("fta_stoch", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::stoch(&high, &low, &close, 14, 3, 3).unwrap()))
    });
    group.bench_function(BenchmarkId::new("fta_obv", DATA_LEN), |b| {
        let volume: Vec<f64> = (0..DATA_LEN)
            .map(|i| 10000.0 + (i as f64 * 10.0).sin() * 3000.0)
            .collect();
        b.iter(|| black_box(indicators::obv(&close, &volume).unwrap()))
    });

    group.finish();
}

fn bench_fta_overlap(c: &mut Criterion) {
    let mut group = c.benchmark_group("fta_overlap");
    let (_, _, _, close, _) = create_ohlcv_data(DATA_LEN);

    group.bench_function(BenchmarkId::new("fta_hma_20", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::hma(&close, 20).unwrap()))
    });
    group.bench_function(BenchmarkId::new("fta_alma_20", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::alma(&close, 20, 0.85, 6.0).unwrap()))
    });
    group.bench_function(BenchmarkId::new("fta_vidya_9_20", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::vidya(&close, 9, 20).unwrap()))
    });
    group.bench_function(BenchmarkId::new("fta_mama", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::mama(&close, 0.5, 0.05).unwrap()))
    });
    group.bench_function(BenchmarkId::new("fta_frama_20", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::frama(&close, 20).unwrap()))
    });

    group.finish();
}

fn bench_fta_momentum_ext(c: &mut Criterion) {
    let mut group = c.benchmark_group("fta_momentum_ext");
    let (open, high, low, close, _) = create_ohlcv_data(DATA_LEN);

    group.bench_function(BenchmarkId::new("fta_connors_rsi", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::connors_rsi(&close, 3, 2, 100).unwrap()))
    });
    group.bench_function(BenchmarkId::new("fta_stoch_rsi", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::stoch_rsi(&close, 14, 14, 3, 3).unwrap()))
    });
    group.bench_function(BenchmarkId::new("fta_rvi_10", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::rvi(&open, &high, &low, &close, 10).unwrap()))
    });

    group.finish();
}

fn bench_fta_volatility_ext(c: &mut Criterion) {
    let mut group = c.benchmark_group("fta_volatility_ext");
    let (open, high, low, close, _) = create_ohlcv_data(DATA_LEN);

    group.bench_function(BenchmarkId::new("fta_garman_klass_vol_20", DATA_LEN), |b| {
        b.iter(|| {
            black_box(indicators::garman_klass_volatility(&open, &high, &low, &close, 20).unwrap())
        })
    });
    group.bench_function(BenchmarkId::new("fta_parkinson_vol_20", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::parkinson_volatility(&high, &low, 20).unwrap()))
    });
    group.bench_function(
        BenchmarkId::new("fta_rogers_satchell_vol_20", DATA_LEN),
        |b| {
            b.iter(|| {
                black_box(
                    indicators::rogers_satchell_volatility(&open, &high, &low, &close, 20).unwrap(),
                )
            })
        },
    );
    group.bench_function(BenchmarkId::new("fta_yang_zhang_vol_20", DATA_LEN), |b| {
        b.iter(|| {
            black_box(indicators::yang_zhang_volatility(&open, &high, &low, &close, 20).unwrap())
        })
    });
    group.bench_function(BenchmarkId::new("fta_realized_vol_20", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::realized_volatility(&close, 20).unwrap()))
    });
    group.bench_function(BenchmarkId::new("fta_semivariance_20", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::semivariance(&close, 20).unwrap()))
    });

    group.finish();
}

fn bench_fta_volume_ext(c: &mut Criterion) {
    let mut group = c.benchmark_group("fta_volume_ext");
    let (_, _, _, close, volume) = create_ohlcv_data(DATA_LEN);

    group.bench_function(BenchmarkId::new("fta_vwmacd", DATA_LEN), |b| {
        b.iter(|| black_box(indicators::vwmacd(&close, &volume, 12, 26, 9).unwrap()))
    });

    group.finish();
}

criterion_group!(
    competitive_benches,
    bench_batch_sma,
    bench_batch_ema,
    bench_batch_rsi,
    bench_batch_macd,
    bench_batch_bbands,
    bench_batch_atr,
    bench_streaming_comparison,
    bench_fta_additional,
    bench_fta_overlap,
    bench_fta_momentum_ext,
    bench_fta_volatility_ext,
    bench_fta_volume_ext,
);
criterion_main!(competitive_benches);
