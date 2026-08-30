use criterion::{black_box, criterion_group, criterion_main, Criterion};
use alpha_ta_core::streaming::indicators::*;
use alpha_ta_core::streaming::{Ohlcv, StreamingIndicator};

const DATA_LEN: usize = 10_000;

type OhlcvVecs = (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>);

fn create_ohlcv_data(len: usize) -> OhlcvVecs {
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

fn generate_close_data(len: usize) -> Vec<f64> {
    let (_, _, _, close, _) = create_ohlcv_data(len);
    close
}

fn generate_ohlc_data(len: usize) -> Vec<(f64, f64, f64)> {
    let (_, high, low, close, _) = create_ohlcv_data(len);
    high
        .iter()
        .zip(low.iter())
        .zip(close.iter())
        .map(|((&h, &l), &c)| (h, l, c))
        .collect()
}

fn generate_ohlcv_data(len: usize) -> Vec<(f64, f64, f64, f64)> {
    let (_, high, low, close, volume) = create_ohlcv_data(len);
    high
        .iter()
        .zip(low.iter())
        .zip(close.iter())
        .zip(volume.iter())
        .map(|(((h, l), c), v)| (*h, *l, *c, *v))
        .collect()
}

fn generate_full_ohlcv_data(len: usize) -> Vec<(f64, f64, f64, f64, f64)> {
    let (open, high, low, close, volume) = create_ohlcv_data(len);
    open
        .into_iter()
        .zip(high)
        .zip(low)
        .zip(close)
        .zip(volume)
        .map(|((((o, h), l), c), v)| (o, h, l, c, v))
        .collect()
}

fn bench_streaming_sma(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("streaming_sma_10000", |b| {
        b.iter(|| {
            let mut sma = StreamingSma::new(20);
            for &val in &data {
                black_box(sma.next(val));
            }
        })
    });
}

fn bench_streaming_ema(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("streaming_ema_10000", |b| {
        b.iter(|| {
            let mut ema = StreamingEma::new(20);
            for &val in &data {
                black_box(ema.next(val));
            }
        })
    });
}

fn bench_streaming_rsi(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("streaming_rsi_10000", |b| {
        b.iter(|| {
            let mut rsi = StreamingRsi::new(14);
            for &val in &data {
                black_box(rsi.next(val));
            }
        })
    });
}

fn bench_streaming_macd(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("streaming_macd_10000", |b| {
        b.iter(|| {
            let mut macd = StreamingMacd::new(12, 26, 9);
            for &val in &data {
                black_box(macd.next(val));
            }
        })
    });
}

fn bench_streaming_boll(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("streaming_boll_10000", |b| {
        b.iter(|| {
            let mut boll = StreamingBoll::new(20, 2.0, 2.0);
            for &val in &data {
                black_box(boll.next(val));
            }
        })
    });
}

fn bench_streaming_atr(c: &mut Criterion) {
    let data = generate_ohlc_data(DATA_LEN);
    c.bench_function("streaming_atr_10000", |b| {
        b.iter(|| {
            let mut atr = StreamingAtr::new(14);
            for &(high, low, close) in &data {
                black_box(atr.next((high, low, close)));
            }
        })
    });
}

fn bench_streaming_kdj(c: &mut Criterion) {
    let data = generate_ohlc_data(DATA_LEN);
    c.bench_function("streaming_kdj_10000", |b| {
        b.iter(|| {
            let mut kdj = StreamingKdj::new(9, 3, 3);
            for &(high, low, close) in &data {
                black_box(kdj.next((high, low, close)));
            }
        })
    });
}

fn bench_streaming_bias(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("streaming_bias_10000", |b| {
        b.iter(|| {
            let mut bias = StreamingBias::new(6);
            for &val in &data {
                black_box(bias.next(val));
            }
        })
    });
}

fn bench_streaming_psy(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("streaming_psy_10000", |b| {
        b.iter(|| {
            let mut psy = StreamingPsy::new(12);
            for &val in &data {
                black_box(psy.next(val));
            }
        })
    });
}

fn bench_streaming_hma(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("streaming_hma_10000", |b| {
        b.iter(|| {
            let mut hma = StreamingHma::new(16);
            for &val in &data {
                black_box(hma.next(val));
            }
        })
    });
}

fn bench_streaming_alma(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("streaming_alma_10000", |b| {
        b.iter(|| {
            let mut alma = StreamingAlma::new(9, 6.0, 0.85);
            for &val in &data {
                black_box(alma.next(val));
            }
        })
    });
}

fn bench_streaming_cmf(c: &mut Criterion) {
    let data = generate_ohlcv_data(DATA_LEN);
    c.bench_function("streaming_cmf_10000", |b| {
        b.iter(|| {
            let mut cmf = StreamingCmf::new(20);
            for &(high, low, close, volume) in &data {
                black_box(cmf.next((high, low, close, volume)));
            }
        })
    });
}

fn bench_streaming_fisher(c: &mut Criterion) {
    let data = generate_ohlc_data(DATA_LEN);
    c.bench_function("streaming_fisher_10000", |b| {
        b.iter(|| {
            let mut fisher = StreamingFisher::new(10);
            for &(high, low, _) in &data {
                black_box(fisher.next((high, low)));
            }
        })
    });
}

fn bench_streaming_tsi(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("streaming_tsi_10000", |b| {
        b.iter(|| {
            let mut tsi = StreamingTsi::new(25, 13);
            for &val in &data {
                black_box(tsi.next(val));
            }
        })
    });
}

fn bench_streaming_chop(c: &mut Criterion) {
    let data = generate_ohlc_data(DATA_LEN);
    c.bench_function("streaming_chop_10000", |b| {
        b.iter(|| {
            let mut chop = StreamingChop::new(14);
            for &(high, low, close) in &data {
                black_box(chop.next((high, low, close)));
            }
        })
    });
}

fn bench_streaming_ao(c: &mut Criterion) {
    let data = generate_full_ohlcv_data(DATA_LEN);
    c.bench_function("streaming_ao_10000", |b| {
        b.iter(|| {
            let mut ao = StreamingAo::new(5, 34);
            for bar in &data {
                black_box(ao.next(bar as &dyn Ohlcv));
            }
        })
    });
}

fn bench_streaming_coppock(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("streaming_coppock_10000", |b| {
        b.iter(|| {
            let mut coppock = StreamingCoppock::new(14, 11, 10);
            for &val in &data {
                black_box(coppock.next(val));
            }
        })
    });
}

fn bench_streaming_kst(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("streaming_kst_10000", |b| {
        b.iter(|| {
            let mut kst = StreamingKst::new(10, 15, 20, 30, 10, 10, 10, 15, 9);
            for &val in &data {
                black_box(kst.next(val));
            }
        })
    });
}

fn bench_streaming_stc(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("streaming_stc_10000", |b| {
        b.iter(|| {
            let mut stc = StreamingStc::new(23, 50, 10);
            for &val in &data {
                black_box(stc.next(val));
            }
        })
    });
}

fn bench_streaming_force_index(c: &mut Criterion) {
    let data = generate_full_ohlcv_data(DATA_LEN);
    c.bench_function("streaming_force_index_10000", |b| {
        b.iter(|| {
            let mut fi = StreamingForceIndex::new(13);
            for bar in &data {
                black_box(fi.next(bar as &dyn Ohlcv));
            }
        })
    });
}

fn bench_streaming_eom(c: &mut Criterion) {
    let data = generate_full_ohlcv_data(DATA_LEN);
    c.bench_function("streaming_eom_10000", |b| {
        b.iter(|| {
            let mut eom = StreamingEom::new(14);
            for bar in &data {
                black_box(eom.next(bar as &dyn Ohlcv));
            }
        })
    });
}

fn bench_streaming_nvi(c: &mut Criterion) {
    let data = generate_full_ohlcv_data(DATA_LEN);
    c.bench_function("streaming_nvi_10000", |b| {
        b.iter(|| {
            let mut nvi = StreamingNvi::new();
            for bar in &data {
                black_box(nvi.next(bar as &dyn Ohlcv));
            }
        })
    });
}

fn bench_streaming_pvi(c: &mut Criterion) {
    let data = generate_full_ohlcv_data(DATA_LEN);
    c.bench_function("streaming_pvi_10000", |b| {
        b.iter(|| {
            let mut pvi = StreamingPvi::new();
            for bar in &data {
                black_box(pvi.next(bar as &dyn Ohlcv));
            }
        })
    });
}

fn bench_streaming_pvt(c: &mut Criterion) {
    let data = generate_full_ohlcv_data(DATA_LEN);
    c.bench_function("streaming_pvt_10000", |b| {
        b.iter(|| {
            let mut pvt = StreamingPvt::new();
            for bar in &data {
                black_box(pvt.next(bar as &dyn Ohlcv));
            }
        })
    });
}

fn bench_streaming_kvo(c: &mut Criterion) {
    let data = generate_full_ohlcv_data(DATA_LEN);
    c.bench_function("streaming_kvo_10000", |b| {
        b.iter(|| {
            let mut kvo = StreamingKvo::new(34, 55, 13);
            for bar in &data {
                black_box(kvo.next(bar as &dyn Ohlcv));
            }
        })
    });
}

fn bench_streaming_mass_index(c: &mut Criterion) {
    let data = generate_full_ohlcv_data(DATA_LEN);
    c.bench_function("streaming_mass_index_10000", |b| {
        b.iter(|| {
            let mut mi = StreamingMassIndex::new(9, 25);
            for bar in &data {
                black_box(mi.next(bar as &dyn Ohlcv));
            }
        })
    });
}

fn bench_streaming_ulcer_index(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("streaming_ulcer_index_10000", |b| {
        b.iter(|| {
            let mut ui = StreamingUlcerIndex::new(14);
            for &val in &data {
                black_box(ui.next(val));
            }
        })
    });
}

fn bench_streaming_rvi(c: &mut Criterion) {
    let data = generate_full_ohlcv_data(DATA_LEN);
    c.bench_function("streaming_rvi_10000", |b| {
        b.iter(|| {
            let mut rvi = StreamingRvi::new(10);
            for bar in &data {
                black_box(rvi.next(bar as &dyn Ohlcv));
            }
        })
    });
}

fn bench_streaming_mcginley(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("streaming_mcginley_10000", |b| {
        b.iter(|| {
            let mut md = StreamingMcGinley::new(14);
            for &val in &data {
                black_box(md.next(val));
            }
        })
    });
}

fn bench_streaming_zlema(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("streaming_zlema_10000", |b| {
        b.iter(|| {
            let mut zlema = StreamingZlema::new(14);
            for &val in &data {
                black_box(zlema.next(val));
            }
        })
    });
}

fn bench_streaming_vidya(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("streaming_vidya_10000", |b| {
        b.iter(|| {
            let mut vidya = StreamingVidya::new(14, 9);
            for &val in &data {
                black_box(vidya.next(val));
            }
        })
    });
}

fn bench_streaming_vwma(c: &mut Criterion) {
    let data = generate_full_ohlcv_data(DATA_LEN);
    c.bench_function("streaming_vwma_10000", |b| {
        b.iter(|| {
            let mut vwma = StreamingVwma::new(20);
            for bar in &data {
                black_box(vwma.next(bar as &dyn Ohlcv));
            }
        })
    });
}

fn bench_streaming_supertrend(c: &mut Criterion) {
    let data = generate_full_ohlcv_data(DATA_LEN);
    c.bench_function("streaming_supertrend_10000", |b| {
        b.iter(|| {
            let mut st = StreamingSuperTrend::new(14, 3.0);
            for bar in &data {
                black_box(st.next(bar as &dyn Ohlcv));
            }
        })
    });
}

fn bench_streaming_ichimoku(c: &mut Criterion) {
    let data = generate_full_ohlcv_data(DATA_LEN);
    c.bench_function("streaming_ichimoku_10000", |b| {
        b.iter(|| {
            let mut ich = StreamingIchimoku::new(9, 26, 52);
            for bar in &data {
                black_box(ich.next(bar as &dyn Ohlcv));
            }
        })
    });
}

fn bench_streaming_vwap(c: &mut Criterion) {
    let data = generate_full_ohlcv_data(DATA_LEN);
    c.bench_function("streaming_vwap_10000", |b| {
        b.iter(|| {
            let mut vwap = StreamingVwap::new();
            for bar in &data {
                black_box(vwap.next(bar as &dyn Ohlcv));
            }
        })
    });
}

criterion_group!(
    streaming_benches,
    bench_streaming_sma,
    bench_streaming_ema,
    bench_streaming_rsi,
    bench_streaming_macd,
    bench_streaming_boll,
    bench_streaming_atr,
    bench_streaming_kdj,
    bench_streaming_bias,
    bench_streaming_psy,
    bench_streaming_hma,
    bench_streaming_alma,
    bench_streaming_cmf,
    bench_streaming_fisher,
    bench_streaming_tsi,
    bench_streaming_chop,
    bench_streaming_ao,
    bench_streaming_coppock,
    bench_streaming_kst,
    bench_streaming_stc,
    bench_streaming_force_index,
    bench_streaming_eom,
    bench_streaming_nvi,
    bench_streaming_pvi,
    bench_streaming_pvt,
    bench_streaming_kvo,
    bench_streaming_mass_index,
    bench_streaming_ulcer_index,
    bench_streaming_rvi,
    bench_streaming_mcginley,
    bench_streaming_zlema,
    bench_streaming_vidya,
    bench_streaming_vwma,
    bench_streaming_supertrend,
    bench_streaming_ichimoku,
    bench_streaming_vwap,
    bench_repaint_sma,
    bench_repaint_ema,
    bench_ehlers_super_smoother,
    bench_ehlers_roofing_filter,
    bench_ehlers_decycler,
    bench_ehlers_bandpass,
    bench_ehlers_itrend,
);
criterion_main!(streaming_benches);

fn bench_repaint_sma(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("repaint_sma_14", |b| {
        b.iter(|| {
            let mut ind = StreamingSma::new(14);
            for (i, &val) in data.iter().enumerate() {
                let time = (i as i64 + 1) * 60_000;
                black_box(ind.next_with_time(val, time));
                // simulate one repaint per bar
                black_box(ind.next_with_time(val + 0.01, time));
            }
        })
    });
}

fn bench_repaint_ema(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("repaint_ema_14", |b| {
        b.iter(|| {
            let mut ind = StreamingEma::new(14);
            for (i, &val) in data.iter().enumerate() {
                let time = (i as i64 + 1) * 60_000;
                black_box(ind.next_with_time(val, time));
                black_box(ind.next_with_time(val + 0.01, time));
            }
        })
    });
}

fn bench_ehlers_super_smoother(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("ehlers_super_smoother_10000", |b| {
        b.iter(|| {
            let mut ind = StreamingSuperSmoother::new(14);
            for &val in &data { black_box(ind.next(val)); }
        })
    });
}

fn bench_ehlers_roofing_filter(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("ehlers_roofing_filter_10000", |b| {
        b.iter(|| {
            let mut ind = StreamingRoofingFilter::new(48, 10);
            for &val in &data { black_box(ind.next(val)); }
        })
    });
}

fn bench_ehlers_decycler(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("ehlers_decycler_10000", |b| {
        b.iter(|| {
            let mut ind = StreamingDecycler::new(20);
            for &val in &data { black_box(ind.next(val)); }
        })
    });
}

fn bench_ehlers_bandpass(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("ehlers_bandpass_10000", |b| {
        b.iter(|| {
            let mut ind = StreamingBandpass::new(20, 0.3);
            for &val in &data { black_box(ind.next(val)); }
        })
    });
}

fn bench_ehlers_itrend(c: &mut Criterion) {
    let data = generate_close_data(DATA_LEN);
    c.bench_function("ehlers_itrend_10000", |b| {
        b.iter(|| {
            let mut ind = StreamingInstantaneousTrendline::new(0.07);
            for &val in &data { black_box(ind.next(val)); }
        })
    });
}
