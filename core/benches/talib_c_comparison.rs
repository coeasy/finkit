//! Head-to-head benchmark: FTA Rust vs TA-Lib C (real FFI calls).
//!
//! Covers 30+ equivalent indicators across all categories.
//! Run with: cargo bench --bench talib_c_comparison --features talib-c

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use alpha_ta_core::indicators;
#[allow(unused_imports)]
use alpha_ta_core::math::moving_avg;
use alpha_ta_core::talib_ffi::*;

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
// TA-Lib C wrapper helpers
// ============================================================================

fn ta_sma(data: &[f64], period: i32) -> Vec<f64> {
    call_single_in(TA_SMA, data, period)
}
fn ta_ema(data: &[f64], period: i32) -> Vec<f64> {
    call_single_in(TA_EMA, data, period)
}
fn ta_wma(data: &[f64], period: i32) -> Vec<f64> {
    call_single_in(TA_WMA, data, period)
}
fn ta_dema(data: &[f64], period: i32) -> Vec<f64> {
    call_single_in(TA_DEMA, data, period)
}
fn ta_tema(data: &[f64], period: i32) -> Vec<f64> {
    call_single_in(TA_TEMA, data, period)
}
fn ta_kama(data: &[f64], period: i32) -> Vec<f64> {
    call_single_in(TA_KAMA, data, period)
}
fn ta_trima(data: &[f64], period: i32) -> Vec<f64> {
    call_single_in(TA_TRIMA, data, period)
}
fn ta_rsi(data: &[f64], period: i32) -> Vec<f64> {
    call_single_in(TA_RSI, data, period)
}
fn ta_roc(data: &[f64], period: i32) -> Vec<f64> {
    call_single_in(TA_ROC, data, period)
}
fn ta_mom(data: &[f64], period: i32) -> Vec<f64> {
    call_single_in(TA_MOM, data, period)
}
fn ta_cmo(data: &[f64], period: i32) -> Vec<f64> {
    call_single_in(TA_CMO, data, period)
}
fn ta_trix(data: &[f64], period: i32) -> Vec<f64> {
    call_single_in(TA_TRIX, data, period)
}

#[allow(dead_code)]
fn ta_t3(data: &[f64], period: i32) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_T3(
            0, (len - 1) as i32, data.as_ptr(),
            period, 0.7,
            &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_macd(data: &[f64], fast: i32, slow: i32, signal: i32) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = data.len();
    let mut macd = vec![0.0f64; len];
    let mut macd_signal = vec![0.0f64; len];
    let mut macd_hist = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_MACD(
            0, (len - 1) as i32, data.as_ptr(),
            fast, slow, signal,
            &mut beg, &mut nb,
            macd.as_mut_ptr(), macd_signal.as_mut_ptr(), macd_hist.as_mut_ptr(),
        );
    }
    (macd, macd_signal, macd_hist)
}

fn ta_bbands(data: &[f64], period: i32, dev_up: f64, dev_dn: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let len = data.len();
    let mut upper = vec![0.0f64; len];
    let mut middle = vec![0.0f64; len];
    let mut lower = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_BBANDS(
            0, (len - 1) as i32, data.as_ptr(),
            period, dev_up, dev_dn, TA_MAType_SMA,
            &mut beg, &mut nb,
            upper.as_mut_ptr(), middle.as_mut_ptr(), lower.as_mut_ptr(),
        );
    }
    (upper, middle, lower)
}

fn ta_atr(high: &[f64], low: &[f64], close: &[f64], period: i32) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_ATR(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(), close.as_ptr(),
            period, &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_natr(high: &[f64], low: &[f64], close: &[f64], period: i32) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_NATR(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(), close.as_ptr(),
            period, &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_stoch(
    high: &[f64], low: &[f64], close: &[f64],
    fastk: i32, slowk: i32, slowd: i32,
) -> (Vec<f64>, Vec<f64>) {
    let len = high.len();
    let mut out_k = vec![0.0f64; len];
    let mut out_d = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_STOCH(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(), close.as_ptr(),
            fastk, slowk, TA_MAType_SMA, slowd, TA_MAType_SMA,
            &mut beg, &mut nb,
            out_k.as_mut_ptr(), out_d.as_mut_ptr(),
        );
    }
    (out_k, out_d)
}

fn ta_stochf(
    high: &[f64], low: &[f64], close: &[f64],
    fastk: i32, fastd: i32,
) -> (Vec<f64>, Vec<f64>) {
    let len = high.len();
    let mut out_k = vec![0.0f64; len];
    let mut out_d = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_STOCHF(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(), close.as_ptr(),
            fastk, fastd, TA_MAType_SMA,
            &mut beg, &mut nb,
            out_k.as_mut_ptr(), out_d.as_mut_ptr(),
        );
    }
    (out_k, out_d)
}

fn ta_cci(high: &[f64], low: &[f64], close: &[f64], period: i32) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_CCI(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(), close.as_ptr(),
            period, &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_willr(high: &[f64], low: &[f64], close: &[f64], period: i32) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_WILLR(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(), close.as_ptr(),
            period, &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_mfi(high: &[f64], low: &[f64], close: &[f64], volume: &[f64], period: i32) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_MFI(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(), close.as_ptr(), volume.as_ptr(),
            period, &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_ad(high: &[f64], low: &[f64], close: &[f64], volume: &[f64]) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_AD(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(), close.as_ptr(), volume.as_ptr(),
            &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_obv(close: &[f64], volume: &[f64]) -> Vec<f64> {
    let len = close.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_OBV(
            0, (len - 1) as i32,
            close.as_ptr(), volume.as_ptr(),
            &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_adx(high: &[f64], low: &[f64], close: &[f64], period: i32) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_ADX(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(), close.as_ptr(),
            period, &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_adxr(high: &[f64], low: &[f64], close: &[f64], period: i32) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_ADXR(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(), close.as_ptr(),
            period, &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_plus_di(high: &[f64], low: &[f64], close: &[f64], period: i32) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_PLUS_DI(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(), close.as_ptr(),
            period, &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_minus_di(high: &[f64], low: &[f64], close: &[f64], period: i32) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_MINUS_DI(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(), close.as_ptr(),
            period, &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_aroon(high: &[f64], low: &[f64], period: i32) -> (Vec<f64>, Vec<f64>) {
    let len = high.len();
    let mut out_down = vec![0.0f64; len];
    let mut out_up = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_AROON(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(),
            period, &mut beg, &mut nb,
            out_down.as_mut_ptr(), out_up.as_mut_ptr(),
        );
    }
    (out_down, out_up)
}

fn ta_adosc(high: &[f64], low: &[f64], close: &[f64], volume: &[f64], fast: i32, slow: i32) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_ADOSC(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(), close.as_ptr(), volume.as_ptr(),
            fast, slow,
            &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_ultosc(high: &[f64], low: &[f64], close: &[f64], p1: i32, p2: i32, p3: i32) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_ULTOSC(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(), close.as_ptr(),
            p1, p2, p3,
            &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_stddev(data: &[f64], period: i32) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_STDDEV(
            0, (len - 1) as i32, data.as_ptr(),
            period, 1.0,
            &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_linearreg(data: &[f64], period: i32) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_LINEARREG(
            0, (len - 1) as i32, data.as_ptr(),
            period, &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_linearreg_slope(data: &[f64], period: i32) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_LINEARREG_SLOPE(
            0, (len - 1) as i32, data.as_ptr(),
            period, &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_ht_phasor(data: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let len = data.len();
    let mut in_phase = vec![0.0f64; len];
    let mut quadrature = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_HT_PHASOR(
            0, (len - 1) as i32, data.as_ptr(),
            &mut beg, &mut nb,
            in_phase.as_mut_ptr(), quadrature.as_mut_ptr(),
        );
    }
    (in_phase, quadrature)
}

fn ta_ht_sine(data: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let len = data.len();
    let mut sine = vec![0.0f64; len];
    let mut lead_sine = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_HT_SINE(
            0, (len - 1) as i32, data.as_ptr(),
            &mut beg, &mut nb,
            sine.as_mut_ptr(), lead_sine.as_mut_ptr(),
        );
    }
    (sine, lead_sine)
}

fn ta_stochrsi(
    data: &[f64],
    time_period: i32, fastk_period: i32, fastd_period: i32,
) -> (Vec<f64>, Vec<f64>) {
    let len = data.len();
    let mut out_k = vec![0.0f64; len];
    let mut out_d = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_STOCHRSI(
            0, (len - 1) as i32, data.as_ptr(),
            time_period, fastk_period, fastd_period, TA_MAType_SMA,
            &mut beg, &mut nb,
            out_k.as_mut_ptr(), out_d.as_mut_ptr(),
        );
    }
    (out_k, out_d)
}

fn ta_aroonosc(high: &[f64], low: &[f64], period: i32) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_AROONOSC(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(),
            period, &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_plus_dm(high: &[f64], low: &[f64], period: i32) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_PLUS_DM(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(),
            period, &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_minus_dm(high: &[f64], low: &[f64], period: i32) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_MINUS_DM(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(),
            period, &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_var(data: &[f64], period: i32) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_VAR(
            0, (len - 1) as i32, data.as_ptr(),
            period, 1.0,
            &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

fn ta_wclprice(high: &[f64], low: &[f64], close: &[f64]) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_WCLPRICE(
            0, (len - 1) as i32,
            high.as_ptr(), low.as_ptr(), close.as_ptr(),
            &mut beg, &mut nb, out.as_mut_ptr(),
        );
    }
    out
}

// ============================================================================
// §1: Overlap Studies — 8 indicators
// ============================================================================
fn bench_overlap(c: &mut Criterion) {
    talib_init();
    let mut group = c.benchmark_group("overlap_vs_talib");
    let (_, _, _, close, _) = create_ohlcv_data(DATA_LEN);

    // 1. SMA
    group.bench_function("FTA_SMA_20", |b| {
        b.iter(|| black_box(indicators::sma(&close, 20).unwrap()))
    });
    group.bench_function("TALib_SMA_20", |b| {
        b.iter(|| black_box(ta_sma(&close, 20)))
    });

    // 2. EMA
    group.bench_function("FTA_EMA_12", |b| {
        b.iter(|| black_box(indicators::ema(&close, 12).unwrap()))
    });
    group.bench_function("TALib_EMA_12", |b| {
        b.iter(|| black_box(ta_ema(&close, 12)))
    });

    // 3. WMA
    group.bench_function("FTA_WMA_20", |b| {
        b.iter(|| black_box(indicators::wma(&close, 20).unwrap()))
    });
    group.bench_function("TALib_WMA_20", |b| {
        b.iter(|| black_box(ta_wma(&close, 20)))
    });

    // 4. DEMA
    group.bench_function("FTA_DEMA_20", |b| {
        b.iter(|| black_box(indicators::dema(&close, 20).unwrap()))
    });
    group.bench_function("TALib_DEMA_20", |b| {
        b.iter(|| black_box(ta_dema(&close, 20)))
    });

    // 5. TEMA
    group.bench_function("FTA_TEMA_20", |b| {
        b.iter(|| black_box(indicators::tema(&close, 20).unwrap()))
    });
    group.bench_function("TALib_TEMA_20", |b| {
        b.iter(|| black_box(ta_tema(&close, 20)))
    });

    // 6. KAMA
    group.bench_function("FTA_KAMA_30", |b| {
        b.iter(|| black_box(indicators::kama(&close, 30, 2, 30).unwrap()))
    });
    group.bench_function("TALib_KAMA_30", |b| {
        b.iter(|| black_box(ta_kama(&close, 30)))
    });

    // 7. TRIMA
    group.bench_function("FTA_TRIMA_20", |b| {
        b.iter(|| black_box(indicators::trima(&close, 20).unwrap()))
    });
    group.bench_function("TALib_TRIMA_20", |b| {
        b.iter(|| black_box(ta_trima(&close, 20)))
    });

    // 8. BBANDS
    group.bench_function("FTA_BBANDS_20", |b| {
        b.iter(|| black_box(indicators::bbands(&close, 20, 2.0, 2.0).unwrap()))
    });
    group.bench_function("TALib_BBANDS_20", |b| {
        b.iter(|| black_box(ta_bbands(&close, 20, 2.0, 2.0)))
    });

    group.finish();
}

// ============================================================================
// §2: Momentum — 10 indicators
// ============================================================================
fn bench_momentum(c: &mut Criterion) {
    let mut group = c.benchmark_group("momentum_vs_talib");
    let (_, high, low, close, volume) = create_ohlcv_data(DATA_LEN);

    // 9. RSI
    group.bench_function("FTA_RSI_14", |b| {
        b.iter(|| black_box(indicators::rsi(&close, 14).unwrap()))
    });
    group.bench_function("TALib_RSI_14", |b| {
        b.iter(|| black_box(ta_rsi(&close, 14)))
    });

    // 10. MACD
    group.bench_function("FTA_MACD_12_26_9", |b| {
        b.iter(|| black_box(indicators::macd(&close, 12, 26, 9).unwrap()))
    });
    group.bench_function("TALib_MACD_12_26_9", |b| {
        b.iter(|| black_box(ta_macd(&close, 12, 26, 9)))
    });

    // 11. ROC
    group.bench_function("FTA_ROC_10", |b| {
        b.iter(|| black_box(indicators::roc(&close, 10).unwrap()))
    });
    group.bench_function("TALib_ROC_10", |b| {
        b.iter(|| black_box(ta_roc(&close, 10)))
    });

    // 12. MOM
    group.bench_function("FTA_MOM_10", |b| {
        b.iter(|| black_box(indicators::mom(&close, 10).unwrap()))
    });
    group.bench_function("TALib_MOM_10", |b| {
        b.iter(|| black_box(ta_mom(&close, 10)))
    });

    // 13. CMO
    group.bench_function("FTA_CMO_14", |b| {
        b.iter(|| black_box(indicators::cmo(&close, 14).unwrap()))
    });
    group.bench_function("TALib_CMO_14", |b| {
        b.iter(|| black_box(ta_cmo(&close, 14)))
    });

    // 14. TRIX
    group.bench_function("FTA_TRIX_15", |b| {
        b.iter(|| black_box(indicators::trix(&close, 15).unwrap()))
    });
    group.bench_function("TALib_TRIX_15", |b| {
        b.iter(|| black_box(ta_trix(&close, 15)))
    });

    // 15. CCI
    group.bench_function("FTA_CCI_14", |b| {
        b.iter(|| black_box(indicators::cci(&high, &low, &close, 14).unwrap()))
    });
    group.bench_function("TALib_CCI_14", |b| {
        b.iter(|| black_box(ta_cci(&high, &low, &close, 14)))
    });

    // 16. WILLR
    group.bench_function("FTA_WILLR_14", |b| {
        b.iter(|| black_box(indicators::willr(&high, &low, &close, 14).unwrap()))
    });
    group.bench_function("TALib_WILLR_14", |b| {
        b.iter(|| black_box(ta_willr(&high, &low, &close, 14)))
    });

    // 17. STOCH
    group.bench_function("FTA_STOCH_14_3_3", |b| {
        b.iter(|| black_box(indicators::stoch(&high, &low, &close, 14, 3, 3).unwrap()))
    });
    group.bench_function("TALib_STOCH_14_3_3", |b| {
        b.iter(|| black_box(ta_stoch(&high, &low, &close, 14, 3, 3)))
    });

    // 18. STOCHF
    group.bench_function("FTA_STOCHF_14_3", |b| {
        b.iter(|| black_box(indicators::stochf(&high, &low, &close, 14, 3).unwrap()))
    });
    group.bench_function("TALib_STOCHF_14_3", |b| {
        b.iter(|| black_box(ta_stochf(&high, &low, &close, 14, 3)))
    });

    // 19. ULTOSC
    group.bench_function("FTA_ULTOSC_7_14_28", |b| {
        b.iter(|| black_box(indicators::ultosc(&high, &low, &close, 7, 14, 28).unwrap()))
    });
    group.bench_function("TALib_ULTOSC_7_14_28", |b| {
        b.iter(|| black_box(ta_ultosc(&high, &low, &close, 7, 14, 28)))
    });

    // 20. MFI
    group.bench_function("FTA_MFI_14", |b| {
        b.iter(|| black_box(indicators::mfi(&high, &low, &close, &volume, 14).unwrap()))
    });
    group.bench_function("TALib_MFI_14", |b| {
        b.iter(|| black_box(ta_mfi(&high, &low, &close, &volume, 14)))
    });

    // 21. STOCHRSI
    group.bench_function("FTA_STOCHRSI_14_14_3_3", |b| {
        b.iter(|| black_box(indicators::stochrsi(&close, 14, 14, 3, 3).unwrap()))
    });
    group.bench_function("TALib_STOCHRSI_14_14_3_3", |b| {
        b.iter(|| black_box(ta_stochrsi(&close, 14, 14, 3)))
    });

    // 22. AROONOSC
    group.bench_function("FTA_AROONOSC_14", |b| {
        b.iter(|| black_box(indicators::aroonosc(&high, &low, 14).unwrap()))
    });
    group.bench_function("TALib_AROONOSC_14", |b| {
        b.iter(|| black_box(ta_aroonosc(&high, &low, 14)))
    });

    // 23. PLUS_DM
    group.bench_function("FTA_PLUS_DM_14", |b| {
        b.iter(|| black_box(indicators::plus_dm(&high, &low).unwrap()))
    });
    group.bench_function("TALib_PLUS_DM_14", |b| {
        b.iter(|| black_box(ta_plus_dm(&high, &low, 14)))
    });

    // 24. MINUS_DM
    group.bench_function("FTA_MINUS_DM_14", |b| {
        b.iter(|| black_box(indicators::minus_dm(&high, &low).unwrap()))
    });
    group.bench_function("TALib_MINUS_DM_14", |b| {
        b.iter(|| black_box(ta_minus_dm(&high, &low, 14)))
    });

    group.finish();
}

// ============================================================================
// §3: Directional Movement — 5 indicators
// ============================================================================
fn bench_directional(c: &mut Criterion) {
    let mut group = c.benchmark_group("directional_vs_talib");
    let (_, high, low, close, _) = create_ohlcv_data(DATA_LEN);

    // 21. ADX
    group.bench_function("FTA_ADX_14", |b| {
        b.iter(|| black_box(indicators::adx(&high, &low, &close, 14).unwrap()))
    });
    group.bench_function("TALib_ADX_14", |b| {
        b.iter(|| black_box(ta_adx(&high, &low, &close, 14)))
    });

    // 22. ADXR
    group.bench_function("FTA_ADXR_14", |b| {
        b.iter(|| black_box(indicators::adxr(&high, &low, &close, 14).unwrap()))
    });
    group.bench_function("TALib_ADXR_14", |b| {
        b.iter(|| black_box(ta_adxr(&high, &low, &close, 14)))
    });

    // 23. PLUS_DI
    group.bench_function("FTA_PLUS_DI_14", |b| {
        b.iter(|| black_box(indicators::plus_di(&high, &low, &close, 14).unwrap()))
    });
    group.bench_function("TALib_PLUS_DI_14", |b| {
        b.iter(|| black_box(ta_plus_di(&high, &low, &close, 14)))
    });

    // 24. MINUS_DI
    group.bench_function("FTA_MINUS_DI_14", |b| {
        b.iter(|| black_box(indicators::minus_di(&high, &low, &close, 14).unwrap()))
    });
    group.bench_function("TALib_MINUS_DI_14", |b| {
        b.iter(|| black_box(ta_minus_di(&high, &low, &close, 14)))
    });

    // 25. AROON
    group.bench_function("FTA_AROON_14", |b| {
        b.iter(|| black_box(indicators::aroon(&high, &low, 14).unwrap()))
    });
    group.bench_function("TALib_AROON_14", |b| {
        b.iter(|| black_box(ta_aroon(&high, &low, 14)))
    });

    group.finish();
}

// ============================================================================
// §4: Volatility — 3 indicators
// ============================================================================
fn bench_volatility(c: &mut Criterion) {
    let mut group = c.benchmark_group("volatility_vs_talib");
    let (_, high, low, close, _) = create_ohlcv_data(DATA_LEN);

    // 26. ATR
    group.bench_function("FTA_ATR_14", |b| {
        b.iter(|| black_box(indicators::atr(&high, &low, &close, 14).unwrap()))
    });
    group.bench_function("TALib_ATR_14", |b| {
        b.iter(|| black_box(ta_atr(&high, &low, &close, 14)))
    });

    // 27. NATR
    group.bench_function("FTA_NATR_14", |b| {
        b.iter(|| black_box(indicators::natr(&high, &low, &close, 14).unwrap()))
    });
    group.bench_function("TALib_NATR_14", |b| {
        b.iter(|| black_box(ta_natr(&high, &low, &close, 14)))
    });

    // 28. STDDEV
    group.bench_function("FTA_STDDEV_20", |b| {
        b.iter(|| black_box(indicators::std_dev(&close, 20, 1.0).unwrap()))
    });
    group.bench_function("TALib_STDDEV_20", |b| {
        b.iter(|| black_box(ta_stddev(&close, 20)))
    });

    group.finish();
}

// ============================================================================
// §5: Volume — 4 indicators
// ============================================================================
fn bench_volume(c: &mut Criterion) {
    let mut group = c.benchmark_group("volume_vs_talib");
    let (_, high, low, close, volume) = create_ohlcv_data(DATA_LEN);

    // 29. OBV
    group.bench_function("FTA_OBV", |b| {
        b.iter(|| black_box(indicators::obv(&close, &volume).unwrap()))
    });
    group.bench_function("TALib_OBV", |b| {
        b.iter(|| black_box(ta_obv(&close, &volume)))
    });

    // 30. AD
    group.bench_function("FTA_AD", |b| {
        b.iter(|| black_box(indicators::ad(&high, &low, &close, &volume).unwrap()))
    });
    group.bench_function("TALib_AD", |b| {
        b.iter(|| black_box(ta_ad(&high, &low, &close, &volume)))
    });

    // 31. ADOSC
    group.bench_function("FTA_ADOSC_3_10", |b| {
        b.iter(|| black_box(indicators::adosc(&high, &low, &close, &volume, 3, 10).unwrap()))
    });
    group.bench_function("TALib_ADOSC_3_10", |b| {
        b.iter(|| black_box(ta_adosc(&high, &low, &close, &volume, 3, 10)))
    });

    group.finish();
}

// ============================================================================
// §6: Statistics — 2 indicators
// ============================================================================
fn bench_statistics(c: &mut Criterion) {
    let mut group = c.benchmark_group("statistics_vs_talib");
    let (_, _, _, close, _) = create_ohlcv_data(DATA_LEN);

    // 32. LINEARREG
    group.bench_function("FTA_LINEARREG_14", |b| {
        b.iter(|| black_box(indicators::linear_reg(&close, 14).unwrap()))
    });
    group.bench_function("TALib_LINEARREG_14", |b| {
        b.iter(|| black_box(ta_linearreg(&close, 14)))
    });

    // 33. LINEARREG_SLOPE
    group.bench_function("FTA_LINREG_SLOPE_14", |b| {
        b.iter(|| black_box(alpha_ta_core::math::linear::linreg_slope(&close, 14).unwrap()))
    });
    group.bench_function("TALib_LINREG_SLOPE_14", |b| {
        b.iter(|| black_box(ta_linearreg_slope(&close, 14)))
    });

    // 34. VAR
    group.bench_function("FTA_VAR_20", |b| {
        b.iter(|| black_box(indicators::var(&close, 20, 1.0).unwrap()))
    });
    group.bench_function("TALib_VAR_20", |b| {
        b.iter(|| black_box(ta_var(&close, 20)))
    });

    group.finish();
}

// ============================================================================
// §6.1: Cycle Indicators — Hilbert Transform (2 indicators)
// ============================================================================
fn bench_cycle(c: &mut Criterion) {
    talib_init();
    let mut group = c.benchmark_group("cycle_vs_talib");
    let (_, _, _, close, _) = create_ohlcv_data(DATA_LEN);

    // 35. HT_PHASOR
    group.bench_function("FTA_HT_PHASOR", |b| {
        b.iter(|| black_box(indicators::ht_phasor(&close).unwrap()))
    });
    group.bench_function("TALib_HT_PHASOR", |b| {
        b.iter(|| black_box(ta_ht_phasor(&close)))
    });

    // 36. HT_SINE
    group.bench_function("FTA_HT_SINE", |b| {
        b.iter(|| black_box(indicators::ht_sine(&close).unwrap()))
    });
    group.bench_function("TALib_HT_SINE", |b| {
        b.iter(|| black_box(ta_ht_sine(&close)))
    });

    group.finish();
}

// ============================================================================
// §6.2: Price Transform (1 indicator)
// ============================================================================
fn bench_price_transform(c: &mut Criterion) {
    talib_init();
    let mut group = c.benchmark_group("price_transform_vs_talib");
    let (_, high, low, close, _) = create_ohlcv_data(DATA_LEN);

    // 37. WCLPRICE
    group.bench_function("FTA_WCLPRICE", |b| {
        b.iter(|| black_box(indicators::wclprice(&high, &low, &close).unwrap()))
    });
    group.bench_function("TALib_WCLPRICE", |b| {
        b.iter(|| black_box(ta_wclprice(&high, &low, &close)))
    });

    group.finish();
}

// ============================================================================
// §7: Scaled comparison at 10K / 100K / 1M for key indicators
// ============================================================================
fn bench_scaled_at(c: &mut Criterion, group_name: &str, size: usize) {
    let mut group = c.benchmark_group(group_name);
    let (_, high, low, close, _volume) = create_ohlcv_data(size);

    group.bench_with_input(BenchmarkId::new("FTA_SMA_20", size), &size, |b, _| {
        b.iter(|| black_box(indicators::sma(&close, 20).unwrap()))
    });
    group.bench_with_input(BenchmarkId::new("TALib_SMA_20", size), &size, |b, _| {
        b.iter(|| black_box(ta_sma(&close, 20)))
    });

    group.bench_with_input(BenchmarkId::new("FTA_EMA_12", size), &size, |b, _| {
        b.iter(|| black_box(indicators::ema(&close, 12).unwrap()))
    });
    group.bench_with_input(BenchmarkId::new("TALib_EMA_12", size), &size, |b, _| {
        b.iter(|| black_box(ta_ema(&close, 12)))
    });

    group.bench_with_input(BenchmarkId::new("FTA_RSI_14", size), &size, |b, _| {
        b.iter(|| black_box(indicators::rsi(&close, 14).unwrap()))
    });
    group.bench_with_input(BenchmarkId::new("TALib_RSI_14", size), &size, |b, _| {
        b.iter(|| black_box(ta_rsi(&close, 14)))
    });

    group.bench_with_input(BenchmarkId::new("FTA_MACD", size), &size, |b, _| {
        b.iter(|| black_box(indicators::macd(&close, 12, 26, 9).unwrap()))
    });
    group.bench_with_input(BenchmarkId::new("TALib_MACD", size), &size, |b, _| {
        b.iter(|| black_box(ta_macd(&close, 12, 26, 9)))
    });

    group.bench_with_input(BenchmarkId::new("FTA_BBANDS_20", size), &size, |b, _| {
        b.iter(|| black_box(indicators::bbands(&close, 20, 2.0, 2.0).unwrap()))
    });
    group.bench_with_input(BenchmarkId::new("TALib_BBANDS_20", size), &size, |b, _| {
        b.iter(|| black_box(ta_bbands(&close, 20, 2.0, 2.0)))
    });

    group.bench_with_input(BenchmarkId::new("FTA_ATR_14", size), &size, |b, _| {
        b.iter(|| black_box(indicators::atr(&high, &low, &close, 14).unwrap()))
    });
}

// ============================================================================
// §7.5: ema_multi_periods vs N × single ema (D.1 / D.7 FMA optimization)
// ============================================================================
fn bench_ema_multi_periods(c: &mut Criterion) {
    let data: Vec<f64> = (0..10_000).map(|i| 100.0 + (i as f64 * 0.013).sin() * 5.0).collect();
    let periods: [usize; 6] = [5, 10, 20, 30, 60, 120];
    let mut group = c.benchmark_group("ema_multi_periods_vs_single");

    // Baseline: call ema 6 times
    group.bench_function("FTA_6x_single_ema", |b| {
        b.iter(|| {
            for &p in &periods {
                let _ = black_box(indicators::ema(&data, p).unwrap());
            }
        })
    });

    // New: one call, FMA + zero-alloc batch
    group.bench_function("FTA_ema_multi_periods", |b| {
        let mut bufs: Vec<Vec<f64>> = periods.iter().map(|_| vec![0.0; data.len()]).collect();
        b.iter(|| {
            let mut refs: Vec<&mut [f64]> = bufs.iter_mut().map(|b| b.as_mut_slice()).collect();
            black_box(moving_avg::ema_multi_periods(&data, &periods, &mut refs).unwrap());
        })
    });

    group.finish();
}

fn bench_scaled_comparison(c: &mut Criterion) {
    bench_scaled_at(c, "scaled_10k_vs_talib", 10_000);
    bench_scaled_at(c, "scaled_100k_vs_talib", 100_000);
    bench_scaled_at(c, "scaled_1m_vs_talib", 1_000_000);
}

criterion_group!(
    benches,
    bench_overlap,
    bench_momentum,
    bench_directional,
    bench_volatility,
    bench_volume,
    bench_statistics,
    bench_cycle,
    bench_price_transform,
    bench_overlap_extra,
    bench_momentum_extra,
    bench_cycle_extra,
    bench_price_transform_full,
    bench_statistics_extra,
    bench_math_transform,
    bench_math_operators,
    bench_scaled_comparison,
    bench_ema_multi_periods,
);
criterion_main!(benches);

// ============================================================================
// §8: Overlap Studies — Extended (MA, T3, SAR, MAVP)
// ============================================================================
fn ta_ma(data: &[f64], period: i32, ma_type: i32) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_MA(0, (len - 1) as i32, data.as_ptr(), period, ma_type, &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}
fn ta_sar(high: &[f64], low: &[f64], acc: f64, max: f64) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_SAR(0, (len - 1) as i32, high.as_ptr(), low.as_ptr(), acc, max, &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}
fn bench_overlap_extra(c: &mut Criterion) {
    talib_init();
    let mut group = c.benchmark_group("overlap_extra_vs_talib");
    let (_, high, low, close, _) = create_ohlcv_data(DATA_LEN);

    // MA
    group.bench_function("FTA_MA_20", |b| {
        b.iter(|| black_box(indicators::sma(&close, 20).unwrap()))
    });
    group.bench_function("TALib_MA_20", |b| {
        b.iter(|| black_box(ta_ma(&close, 20, TA_MAType_SMA)))
    });

    // T3
    group.bench_function("FTA_T3_5", |b| {
        b.iter(|| black_box(indicators::t3(&close, 5, 0.7).unwrap()))
    });
    group.bench_function("TALib_T3_5", |b| {
        b.iter(|| black_box(ta_t3(&close, 5)))
    });

    // SAR
    group.bench_function("FTA_SAR", |b| {
        b.iter(|| black_box(indicators::sar(&high, &low, 0.02, 0.2).unwrap().sar))
    });
    group.bench_function("TALib_SAR", |b| {
        b.iter(|| black_box(ta_sar(&high, &low, 0.02, 0.2)))
    });

    group.finish();
}

// ============================================================================
// §9: Momentum — Extended (APO, PPO, BOP, ULTOSC, MAMA)
// ============================================================================
fn ta_apo(data: &[f64], fast: i32, slow: i32) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_APO(0, (len - 1) as i32, data.as_ptr(), fast, slow, TA_MAType_SMA, &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}
fn ta_ppo(data: &[f64], fast: i32, slow: i32) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_PPO(0, (len - 1) as i32, data.as_ptr(), fast, slow, TA_MAType_SMA, &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}
fn ta_bop(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Vec<f64> {
    let len = open.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_BOP(0, (len - 1) as i32, open.as_ptr(), high.as_ptr(), low.as_ptr(), close.as_ptr(),
               &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}
fn ta_mama(data: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let len = data.len();
    let mut mama = vec![0.0f64; len];
    let mut fama = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_MAMA(0, (len - 1) as i32, data.as_ptr(), 0.5, 0.05,
                &mut beg, &mut nb, mama.as_mut_ptr(), fama.as_mut_ptr());
    }
    (mama, fama)
}
fn bench_momentum_extra(c: &mut Criterion) {
    talib_init();
    let mut group = c.benchmark_group("momentum_extra_vs_talib");
    let (open, high, low, close, _) = create_ohlcv_data(DATA_LEN);

    // APO
    group.bench_function("FTA_APO_12_26", |b| {
        b.iter(|| black_box(indicators::apo(&close, 12, 26).unwrap()))
    });
    group.bench_function("TALib_APO_12_26", |b| {
        b.iter(|| black_box(ta_apo(&close, 12, 26)))
    });

    // PPO
    group.bench_function("FTA_PPO_12_26", |b| {
        b.iter(|| black_box(indicators::ppo(&close, 12, 26).unwrap()))
    });
    group.bench_function("TALib_PPO_12_26", |b| {
        b.iter(|| black_box(ta_ppo(&close, 12, 26)))
    });

    // BOP
    group.bench_function("FTA_BOP", |b| {
        b.iter(|| black_box(indicators::bop(&open, &high, &low, &close).unwrap()))
    });
    group.bench_function("TALib_BOP", |b| {
        b.iter(|| black_box(ta_bop(&open, &high, &low, &close)))
    });

    // MAMA
    group.bench_function("FTA_MAMA", |b| {
        b.iter(|| black_box(indicators::mama(&close, 0.5, 0.05).unwrap().mama))
    });
    group.bench_function("TALib_MAMA", |b| {
        b.iter(|| black_box(ta_mama(&close).0))
    });

    group.finish();
}

// ============================================================================
// §10: Cycle Indicators — Extended (HT_DCPERIOD, HT_DCPHASE, HT_TRENDLINE)
// ============================================================================
fn ta_ht_dcperiod(data: &[f64]) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_HT_DCPERIOD(0, (len - 1) as i32, data.as_ptr(), &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}
fn ta_ht_dcphase(data: &[f64]) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_HT_DCPHASE(0, (len - 1) as i32, data.as_ptr(), &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}
fn ta_ht_trendline(data: &[f64]) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_HT_TRENDLINE(0, (len - 1) as i32, data.as_ptr(), &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}
fn bench_cycle_extra(c: &mut Criterion) {
    talib_init();
    let mut group = c.benchmark_group("cycle_extra_vs_talib");
    let (_, _, _, close, _) = create_ohlcv_data(DATA_LEN);

    group.bench_function("FTA_HT_DCPERIOD", |b| {
        b.iter(|| black_box(indicators::ht_dcperiod(&close).unwrap()))
    });
    group.bench_function("TALib_HT_DCPERIOD", |b| {
        b.iter(|| black_box(ta_ht_dcperiod(&close)))
    });

    group.bench_function("FTA_HT_DCPHASE", |b| {
        b.iter(|| black_box(indicators::ht_dcphase(&close).unwrap()))
    });
    group.bench_function("TALib_HT_DCPHASE", |b| {
        b.iter(|| black_box(ta_ht_dcphase(&close)))
    });

    group.bench_function("FTA_HT_TRENDLINE", |b| {
        b.iter(|| black_box(indicators::ht_trendline(&close).unwrap()))
    });
    group.bench_function("TALib_HT_TRENDLINE", |b| {
        b.iter(|| black_box(ta_ht_trendline(&close)))
    });

    group.finish();
}

// ============================================================================
// §11: Price Transform — Full coverage (AVGPRICE, MEDPRICE, TYPPRICE)
// ============================================================================
fn ta_avgprice(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Vec<f64> {
    let len = open.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_AVGPRICE(0, (len - 1) as i32, open.as_ptr(), high.as_ptr(), low.as_ptr(), close.as_ptr(),
                    &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}
fn ta_medprice(high: &[f64], low: &[f64]) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_MEDPRICE(0, (len - 1) as i32, high.as_ptr(), low.as_ptr(), &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}
fn ta_typprice(high: &[f64], low: &[f64], close: &[f64]) -> Vec<f64> {
    let len = high.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_TYPPRICE(0, (len - 1) as i32, high.as_ptr(), low.as_ptr(), close.as_ptr(),
                    &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}
fn bench_price_transform_full(c: &mut Criterion) {
    talib_init();
    let mut group = c.benchmark_group("price_transform_full_vs_talib");
    let (open, high, low, close, _) = create_ohlcv_data(DATA_LEN);

    group.bench_function("FTA_AVGPRICE", |b| {
        b.iter(|| black_box(indicators::avgprice(&open, &high, &low, &close).unwrap()))
    });
    group.bench_function("TALib_AVGPRICE", |b| {
        b.iter(|| black_box(ta_avgprice(&open, &high, &low, &close)))
    });

    group.bench_function("FTA_MEDPRICE", |b| {
        b.iter(|| black_box(indicators::medprice(&high, &low).unwrap()))
    });
    group.bench_function("TALib_MEDPRICE", |b| {
        b.iter(|| black_box(ta_medprice(&high, &low)))
    });

    group.bench_function("FTA_TYPPRICE", |b| {
        b.iter(|| black_box(indicators::typprice(&high, &low, &close).unwrap()))
    });
    group.bench_function("TALib_TYPPRICE", |b| {
        b.iter(|| black_box(ta_typprice(&high, &low, &close)))
    });

    group.finish();
}

// ============================================================================
// §12: Statistics — Extended (TSF, LINREG_*, BETA, CORREL, PERCENTRANK, AVGDEV)
// ============================================================================
fn ta_tsf(data: &[f64], period: i32) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_TSF(0, (len - 1) as i32, data.as_ptr(), period, &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}
fn ta_linearreg_intercept(data: &[f64], period: i32) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_LINEARREG_INTERCEPT(0, (len - 1) as i32, data.as_ptr(), period, &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}
fn ta_linearreg_angle(data: &[f64], period: i32) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_LINEARREG_ANGLE(0, (len - 1) as i32, data.as_ptr(), period, &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}
fn ta_correl(a: &[f64], b: &[f64], period: i32) -> Vec<f64> {
    let len = a.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_CORREL(0, (len - 1) as i32, a.as_ptr(), b.as_ptr(), period, &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}
fn ta_percentrank(data: &[f64], period: i32) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_PERCENTRANK(0, (len - 1) as i32, data.as_ptr(), period, &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}
fn ta_avgdev(data: &[f64], period: i32) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe {
        TA_AVGDEV(0, (len - 1) as i32, data.as_ptr(), period, &mut beg, &mut nb, out.as_mut_ptr());
    }
    out
}
fn bench_statistics_extra(c: &mut Criterion) {
    talib_init();
    let mut group = c.benchmark_group("statistics_extra_vs_talib");
    let (_, _, _, close, _) = create_ohlcv_data(DATA_LEN);
    let benchmark = close.clone();
    let (_, _, _, mkt, _) = create_ohlcv_data(DATA_LEN);

    group.bench_function("FTA_TSF_14", |b| {
        b.iter(|| black_box(indicators::tsf(&close, 14).unwrap()))
    });
    group.bench_function("TALib_TSF_14", |b| {
        b.iter(|| black_box(ta_tsf(&close, 14)))
    });

    group.bench_function("FTA_LINREG_INTERCEPT_14", |b| {
        b.iter(|| black_box(indicators::linearreg_intercept(&close, 14).unwrap()))
    });
    group.bench_function("TALib_LINREG_INTERCEPT_14", |b| {
        b.iter(|| black_box(ta_linearreg_intercept(&close, 14)))
    });

    group.bench_function("FTA_LINREG_ANGLE_14", |b| {
        b.iter(|| black_box(indicators::linearreg_angle(&close, 14).unwrap()))
    });
    group.bench_function("TALib_LINREG_ANGLE_14", |b| {
        b.iter(|| black_box(ta_linearreg_angle(&close, 14)))
    });

    group.bench_function("FTA_CORREL_30", |b| {
        b.iter(|| black_box(indicators::correlation(&close, &benchmark, 30).unwrap()))
    });
    group.bench_function("TALib_CORREL_30", |b| {
        b.iter(|| black_box(ta_correl(&close, &mkt, 30)))
    });

    group.bench_function("FTA_PERCENTRANK_30", |b| {
        b.iter(|| black_box(indicators::percent_rank(&close, 30).unwrap()))
    });
    group.bench_function("TALib_PERCENTRANK_30", |b| {
        b.iter(|| black_box(ta_percentrank(&close, 30)))
    });

    group.bench_function("FTA_AVGDEV_14", |b| {
        b.iter(|| black_box(indicators::avgdev(&close, 14).unwrap()))
    });
    group.bench_function("TALib_AVGDEV_14", |b| {
        b.iter(|| black_box(ta_avgdev(&close, 14)))
    });

    group.finish();
}

// ============================================================================
// §13: Math Transform (15 unary functions) — FTA vs TA-Lib
// ============================================================================
fn ta_acos(data: &[f64]) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe { TA_ACOS(0, (len - 1) as i32, data.as_ptr(), &mut beg, &mut nb, out.as_mut_ptr()); }
    out
}
fn ta_sin(data: &[f64]) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe { TA_SIN(0, (len - 1) as i32, data.as_ptr(), &mut beg, &mut nb, out.as_mut_ptr()); }
    out
}
fn ta_cos(data: &[f64]) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe { TA_COS(0, (len - 1) as i32, data.as_ptr(), &mut beg, &mut nb, out.as_mut_ptr()); }
    out
}
fn ta_sqrt(data: &[f64]) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe { TA_SQRT(0, (len - 1) as i32, data.as_ptr(), &mut beg, &mut nb, out.as_mut_ptr()); }
    out
}
fn ta_ln(data: &[f64]) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe { TA_LN(0, (len - 1) as i32, data.as_ptr(), &mut beg, &mut nb, out.as_mut_ptr()); }
    out
}
fn ta_exp(data: &[f64]) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe { TA_EXP(0, (len - 1) as i32, data.as_ptr(), &mut beg, &mut nb, out.as_mut_ptr()); }
    out
}
fn ta_ceil(data: &[f64]) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe { TA_CEIL(0, (len - 1) as i32, data.as_ptr(), &mut beg, &mut nb, out.as_mut_ptr()); }
    out
}
fn ta_floor(data: &[f64]) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe { TA_FLOOR(0, (len - 1) as i32, data.as_ptr(), &mut beg, &mut nb, out.as_mut_ptr()); }
    out
}
fn ta_tanh(data: &[f64]) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe { TA_TANH(0, (len - 1) as i32, data.as_ptr(), &mut beg, &mut nb, out.as_mut_ptr()); }
    out
}
fn bench_math_transform(c: &mut Criterion) {
    talib_init();
    let mut group = c.benchmark_group("math_transform_vs_talib");
    let (_, _, _, close, _) = create_ohlcv_data(DATA_LEN);
    // Use abs() of close to keep math functions in valid range
    let data: Vec<f64> = close.iter().map(|v| v.abs() * 0.001 + 0.5).collect();

    group.bench_function("FTA_ACOS", |b| {
        b.iter(|| black_box(indicators::acos(&data).unwrap()))
    });
    group.bench_function("TALib_ACOS", |b| {
        b.iter(|| black_box(ta_acos(&data)))
    });
    group.bench_function("FTA_SIN", |b| {
        b.iter(|| black_box(indicators::sin(&data).unwrap()))
    });
    group.bench_function("TALib_SIN", |b| {
        b.iter(|| black_box(ta_sin(&data)))
    });
    group.bench_function("FTA_COS", |b| {
        b.iter(|| black_box(indicators::cos(&data).unwrap()))
    });
    group.bench_function("TALib_COS", |b| {
        b.iter(|| black_box(ta_cos(&data)))
    });
    group.bench_function("FTA_SQRT", |b| {
        b.iter(|| black_box(indicators::sqrt(&data).unwrap()))
    });
    group.bench_function("TALib_SQRT", |b| {
        b.iter(|| black_box(ta_sqrt(&data)))
    });
    group.bench_function("FTA_LN", |b| {
        b.iter(|| black_box(indicators::ln(&data).unwrap()))
    });
    group.bench_function("TALib_LN", |b| {
        b.iter(|| black_box(ta_ln(&data)))
    });
    group.bench_function("FTA_EXP", |b| {
        b.iter(|| black_box(indicators::exp(&data).unwrap()))
    });
    group.bench_function("TALib_EXP", |b| {
        b.iter(|| black_box(ta_exp(&data)))
    });
    group.bench_function("FTA_CEIL", |b| {
        b.iter(|| black_box(indicators::ceil(&data).unwrap()))
    });
    group.bench_function("TALib_CEIL", |b| {
        b.iter(|| black_box(ta_ceil(&data)))
    });
    group.bench_function("FTA_FLOOR", |b| {
        b.iter(|| black_box(indicators::floor(&data).unwrap()))
    });
    group.bench_function("TALib_FLOOR", |b| {
        b.iter(|| black_box(ta_floor(&data)))
    });
    group.bench_function("FTA_TANH", |b| {
        b.iter(|| black_box(indicators::tanh(&data).unwrap()))
    });
    group.bench_function("TALib_TANH", |b| {
        b.iter(|| black_box(ta_tanh(&data)))
    });

    group.finish();
}

// ============================================================================
// §14: Math Operators (ADD, SUB, MULT, DIV, SUM, MAX, MIN, MAXINDEX, MININDEX)
// ============================================================================
fn ta_add(a: &[f64], b: &[f64]) -> Vec<f64> {
    let len = a.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe { TA_ADD(0, (len - 1) as i32, a.as_ptr(), b.as_ptr(), &mut beg, &mut nb, out.as_mut_ptr()); }
    out
}
fn ta_sub(a: &[f64], b: &[f64]) -> Vec<f64> {
    let len = a.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe { TA_SUB(0, (len - 1) as i32, a.as_ptr(), b.as_ptr(), &mut beg, &mut nb, out.as_mut_ptr()); }
    out
}
fn ta_mult(a: &[f64], b: &[f64]) -> Vec<f64> {
    let len = a.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe { TA_MULT(0, (len - 1) as i32, a.as_ptr(), b.as_ptr(), &mut beg, &mut nb, out.as_mut_ptr()); }
    out
}
fn ta_sum(data: &[f64], period: i32) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe { TA_SUM(0, (len - 1) as i32, data.as_ptr(), period, &mut beg, &mut nb, out.as_mut_ptr()); }
    out
}
fn ta_max(data: &[f64], period: i32) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe { TA_MAX(0, (len - 1) as i32, data.as_ptr(), period, &mut beg, &mut nb, out.as_mut_ptr()); }
    out
}
fn ta_min(data: &[f64], period: i32) -> Vec<f64> {
    let len = data.len();
    let mut out = vec![0.0f64; len];
    let mut beg = 0i32;
    let mut nb = 0i32;
    unsafe { TA_MIN(0, (len - 1) as i32, data.as_ptr(), period, &mut beg, &mut nb, out.as_mut_ptr()); }
    out
}
fn bench_math_operators(c: &mut Criterion) {
    talib_init();
    let mut group = c.benchmark_group("math_operators_vs_talib");
    let (_, _, _, close, _) = create_ohlcv_data(DATA_LEN);
    let (_, _, _, ref_a, _) = create_ohlcv_data(DATA_LEN);
    let series_a = close.clone();
    let series_b = ref_a;

    group.bench_function("FTA_ADD", |bench| {
        bench.iter(|| black_box(indicators::add(&series_a, &series_b).unwrap()))
    });
    group.bench_function("TALib_ADD", |bench| {
        bench.iter(|| black_box(ta_add(&series_a, &series_b)))
    });

    group.bench_function("FTA_SUB", |bench| {
        bench.iter(|| black_box(indicators::sub(&series_a, &series_b).unwrap()))
    });
    group.bench_function("TALib_SUB", |bench| {
        bench.iter(|| black_box(ta_sub(&series_a, &series_b)))
    });

    group.bench_function("FTA_MULT", |bench| {
        bench.iter(|| black_box(indicators::mult(&series_a, &series_b).unwrap()))
    });
    group.bench_function("TALib_MULT", |bench| {
        bench.iter(|| black_box(ta_mult(&series_a, &series_b)))
    });

    group.bench_function("FTA_SUM_30", |bench| {
        bench.iter(|| black_box(indicators::sum(&close, 30).unwrap()))
    });
    group.bench_function("TALib_SUM_30", |bench| {
        bench.iter(|| black_box(ta_sum(&close, 30)))
    });

    group.bench_function("FTA_MAX_30", |bench| {
        bench.iter(|| black_box(indicators::max(&close, 30).unwrap()))
    });
    group.bench_function("TALib_MAX_30", |bench| {
        bench.iter(|| black_box(ta_max(&close, 30)))
    });

    group.bench_function("FTA_MIN_30", |bench| {
        bench.iter(|| black_box(indicators::min(&close, 30).unwrap()))
    });
    group.bench_function("TALib_MIN_30", |bench| {
        bench.iter(|| black_box(ta_min(&close, 30)))
    });

    group.finish();
}
