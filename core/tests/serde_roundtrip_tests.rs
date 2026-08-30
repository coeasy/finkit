//! Comprehensive serde roundtrip tests for all streaming indicators with CheckpointState.

use finkit::streaming::indicators::*;
use finkit::streaming::{CheckpointState, Ohlcv, OhlcvBar, StreamingIndicator};

fn make_bars(n: usize) -> Vec<OhlcvBar> {
    (0..n)
        .map(|i| {
            let base = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            OhlcvBar::new(base, base + 2.0, base - 1.5, base + 0.5, 1000.0 + i as f64 * 10.0)
        })
        .collect()
}

fn gen_data(n: usize) -> Vec<f64> {
    (0..n).map(|i| 50.0 + (i as f64 * 0.3).sin() * 10.0).collect()
}

// =============================================================================
// Macro for f64→f64 single-period indicators
// =============================================================================

macro_rules! checkpoint_test_f64 {
    ($name:ident, $create:expr, $warmup:expr) => {
        #[test]
        fn $name() {
            let data = gen_data($warmup + 20);
            let mut ind = $create;
            for &v in &data[..$warmup] { ind.next(v); }
            let bytes = ind.save_state().unwrap();
            let mut restored = decltype_restore(&bytes, &ind);
            for &v in &data[$warmup..] {
                assert_eq!(ind.next(v), restored.next(v));
            }
            assert!(ind.state_size_hint() > 0);
        }
    };
}

fn decltype_restore<T: CheckpointState>(bytes: &[u8], _hint: &T) -> T {
    T::restore_state(bytes).unwrap()
}

// =============================================================================
// Macro for HLC tuple indicators
// =============================================================================

macro_rules! checkpoint_test_hlc {
    ($name:ident, $create:expr, $warmup:expr) => {
        #[test]
        fn $name() {
            let mut ind = $create;
            for i in 0..$warmup {
                let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
                ind.next((base + 1.5, base - 1.5, base));
            }
            let bytes = ind.save_state().unwrap();
            let mut restored = decltype_restore(&bytes, &ind);
            for i in $warmup..($warmup + 20) {
                let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
                let input = (base + 1.5, base - 1.5, base);
                assert_eq!(ind.next(input), restored.next(input));
            }
        }
    };
}

// =============================================================================
// Macro for &dyn Ohlcv → f64 indicators
// =============================================================================

macro_rules! checkpoint_test_ohlcv {
    ($name:ident, $create:expr, $warmup:expr) => {
        #[test]
        fn $name() {
            let bars = make_bars($warmup + 20);
            let mut ind = $create;
            for bar in &bars[..$warmup] {
                ind.next(bar as &dyn Ohlcv);
            }
            let bytes = ind.save_state().unwrap();
            let mut restored = decltype_restore(&bytes, &ind);
            for bar in &bars[$warmup..] {
                assert_eq!(
                    ind.next(bar as &dyn Ohlcv),
                    restored.next(bar as &dyn Ohlcv)
                );
            }
        }
    };
}

// =============================================================================
// f64 → f64 indicators
// =============================================================================

checkpoint_test_f64!(test_checkpoint_sma, StreamingSma::new(14), 20);
checkpoint_test_f64!(test_checkpoint_ema, StreamingEma::new(14), 20);
checkpoint_test_f64!(test_checkpoint_wma, StreamingWma::new(14), 20);
checkpoint_test_f64!(test_checkpoint_dema, StreamingDema::new(14), 30);
checkpoint_test_f64!(test_checkpoint_tema, StreamingTema::new(14), 50);
checkpoint_test_f64!(test_checkpoint_kama, StreamingKama::new(14), 20);
checkpoint_test_f64!(test_checkpoint_t3, StreamingT3::new(5), 40);
checkpoint_test_f64!(test_checkpoint_rsi, StreamingRsi::new(14), 20);
checkpoint_test_f64!(test_checkpoint_mom, StreamingMom::new(10), 15);
checkpoint_test_f64!(test_checkpoint_roc, StreamingRoc::new(10), 15);
checkpoint_test_f64!(test_checkpoint_trix, StreamingTrix::new(5), 20);
checkpoint_test_f64!(test_checkpoint_hma, StreamingHma::new(9), 20);
checkpoint_test_f64!(test_checkpoint_zlema, StreamingZlema::new(14), 20);
checkpoint_test_f64!(test_checkpoint_bias, StreamingBias::new(14), 20);
checkpoint_test_f64!(test_checkpoint_cmo, StreamingCmo::new(14), 20);
checkpoint_test_f64!(test_checkpoint_dpo, StreamingDpo::new(14), 30);
checkpoint_test_f64!(test_checkpoint_var, StreamingVar::new(14), 20);
checkpoint_test_f64!(test_checkpoint_stddev, StreamingStdDev::new(14), 20);
checkpoint_test_f64!(test_checkpoint_zscore, StreamingZscore::new(14), 20);
checkpoint_test_f64!(test_checkpoint_mcginley, StreamingMcGinley::new(14), 20);
checkpoint_test_f64!(test_checkpoint_linreg, StreamingLinReg::new(14), 20);
checkpoint_test_f64!(test_checkpoint_linreg_slope, StreamingLinRegSlope::new(14), 20);
checkpoint_test_f64!(test_checkpoint_linreg_angle, StreamingLinRegAngle::new(14), 20);
checkpoint_test_f64!(test_checkpoint_linreg_intercept, StreamingLinRegIntercept::new(14), 20);
checkpoint_test_f64!(test_checkpoint_tsf, StreamingTsf::new(14), 20);
checkpoint_test_f64!(test_checkpoint_ulcer_index, StreamingUlcerIndex::new(14), 20);
checkpoint_test_f64!(test_checkpoint_psy, StreamingPsy::new(14), 20);
checkpoint_test_f64!(test_checkpoint_vidya, StreamingVidya::new(14, 10), 30);
checkpoint_test_f64!(test_checkpoint_apo, StreamingApo::new(5, 10), 15);
checkpoint_test_f64!(test_checkpoint_ppo, StreamingPpo::new(5, 10), 15);
checkpoint_test_f64!(test_checkpoint_alma, StreamingAlma::new(9, 6.0, 0.85), 15);
checkpoint_test_f64!(test_checkpoint_coppock, StreamingCoppock::new(10, 14, 11), 40);
checkpoint_test_f64!(test_checkpoint_stc, StreamingStc::new(12, 26, 10), 50);
checkpoint_test_f64!(test_checkpoint_tsi, StreamingTsi::new(25, 13), 50);
checkpoint_test_f64!(test_checkpoint_ht_dcperiod, StreamingHtDcPeriod::new(), 50);
checkpoint_test_f64!(test_checkpoint_ht_trendmode, StreamingHtTrendMode::new(), 50);
checkpoint_test_f64!(test_checkpoint_ht_trendline, StreamingHtTrendline::new(), 50);
checkpoint_test_f64!(test_checkpoint_ht_dcphase, StreamingHtDcPhase::new(), 50);

// HT_SINE returns struct
#[test]
fn test_checkpoint_ht_sine() {
    let data = gen_data(80);
    let mut ind = StreamingHtSine::new();
    for &v in &data[..60] { ind.next(v); }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingHtSine::restore_state(&bytes).unwrap();
    for &v in &data[60..] {
        assert_eq!(ind.next(v), restored.next(v));
    }
}

// MACD returns struct
#[test]
fn test_checkpoint_macd() {
    let data = gen_data(30);
    let mut ind = StreamingMacd::new(3, 5, 3);
    for &v in &data[..15] { ind.next(v); }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingMacd::restore_state(&bytes).unwrap();
    for &v in &data[15..] {
        assert_eq!(ind.next(v), restored.next(v));
    }
}

// BOLL returns struct
#[test]
fn test_checkpoint_boll() {
    let data = gen_data(25);
    let mut ind = StreamingBoll::new(5, 2.0, 2.0);
    for &v in &data[..10] { ind.next(v); }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingBoll::restore_state(&bytes).unwrap();
    for &v in &data[10..] {
        assert_eq!(ind.next(v), restored.next(v));
    }
}

// DMA returns struct
#[test]
fn test_checkpoint_dma() {
    let data = gen_data(35);
    let mut ind = StreamingDma::new(5, 10, 5);
    for &v in &data[..20] { ind.next(v); }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingDma::restore_state(&bytes).unwrap();
    for &v in &data[20..] {
        assert_eq!(ind.next(v), restored.next(v));
    }
}

// EXPMA returns struct
#[test]
fn test_checkpoint_expma() {
    let data = gen_data(30);
    let mut ind = StreamingExpma::new(5, 10);
    for &v in &data[..15] { ind.next(v); }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingExpma::restore_state(&bytes).unwrap();
    for &v in &data[15..] {
        assert_eq!(ind.next(v), restored.next(v));
    }
}

// ENE returns struct
#[test]
fn test_checkpoint_ene() {
    let data = gen_data(30);
    let mut ind = StreamingEne::new(10, 6.0, 6.0);
    for &v in &data[..15] { ind.next(v); }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingEne::restore_state(&bytes).unwrap();
    for &v in &data[15..] {
        assert_eq!(ind.next(v), restored.next(v));
    }
}

// KST returns struct
#[test]
fn test_checkpoint_kst() {
    let data = gen_data(80);
    let mut ind = StreamingKst::new(10, 15, 20, 30, 10, 10, 10, 15, 9);
    for &v in &data[..60] { ind.next(v); }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingKst::restore_state(&bytes).unwrap();
    for &v in &data[60..] {
        assert_eq!(ind.next(v), restored.next(v));
    }
}

// StochRSI returns struct (dual f64 → (f64, f64))
#[test]
fn test_checkpoint_stoch_rsi() {
    let data = gen_data(60);
    let mut ind = StreamingStochRsi::new(14, 14, 3, 3);
    for &v in &data[..40] { ind.next(v); }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingStochRsi::restore_state(&bytes).unwrap();
    for &v in &data[40..] {
        assert_eq!(ind.next(v), restored.next(v));
    }
}

// =============================================================================
// (high, low, close) → f64 indicators
// =============================================================================

checkpoint_test_hlc!(test_checkpoint_atr, StreamingAtr::new(14), 20);
checkpoint_test_hlc!(test_checkpoint_adx, StreamingAdx::new(14), 30);
checkpoint_test_hlc!(test_checkpoint_cci, StreamingCci::new(14), 20);
checkpoint_test_hlc!(test_checkpoint_dx, StreamingDx::new(14), 30);
checkpoint_test_hlc!(test_checkpoint_adxr, StreamingAdxr::new(14), 50);
checkpoint_test_hlc!(test_checkpoint_plus_di, StreamingPlusDi::new(14), 20);
checkpoint_test_hlc!(test_checkpoint_minus_di, StreamingMinusDi::new(14), 20);
checkpoint_test_hlc!(test_checkpoint_trange, StreamingTrange::new(), 5);
checkpoint_test_hlc!(test_checkpoint_chop, StreamingChop::new(14), 20);

// (high, low, close) → struct
#[test]
fn test_checkpoint_stoch() {
    let mut ind = StreamingStoch::new(14, 3, 3);
    for i in 0..30 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        ind.next((base + 1.5, base - 1.5, base));
    }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingStoch::restore_state(&bytes).unwrap();
    for i in 30..50 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        let input = (base + 1.5, base - 1.5, base);
        assert_eq!(ind.next(input), restored.next(input));
    }
}

#[test]
fn test_checkpoint_stoch_f() {
    let mut ind = StreamingStochF::new(14, 3);
    for i in 0..20 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        ind.next((base + 1.5, base - 1.5, base));
    }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingStochF::restore_state(&bytes).unwrap();
    for i in 20..40 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        let input = (base + 1.5, base - 1.5, base);
        assert_eq!(ind.next(input), restored.next(input));
    }
}

#[test]
fn test_checkpoint_kdj() {
    let mut ind = StreamingKdj::new(9, 3, 3);
    for i in 0..20 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        ind.next((base + 1.5, base - 1.5, base));
    }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingKdj::restore_state(&bytes).unwrap();
    for i in 20..40 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        let input = (base + 1.5, base - 1.5, base);
        assert_eq!(ind.next(input), restored.next(input));
    }
}

#[test]
fn test_checkpoint_ult_osc() {
    let mut ind = StreamingUltOsc::new(7, 14, 28);
    for i in 0..40 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        ind.next((base + 1.5, base - 1.5, base));
    }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingUltOsc::restore_state(&bytes).unwrap();
    for i in 40..60 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        let input = (base + 1.5, base - 1.5, base);
        assert_eq!(ind.next(input), restored.next(input));
    }
}

#[test]
fn test_checkpoint_mass_index() {
    let bars = make_bars(60);
    let mut ind = StreamingMassIndex::new(25, 9);
    for bar in &bars[..40] { ind.next(bar as &dyn Ohlcv); }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingMassIndex::restore_state(&bytes).unwrap();
    for bar in &bars[40..] {
        assert_eq!(ind.next(bar as &dyn Ohlcv), restored.next(bar as &dyn Ohlcv));
    }
}

// =============================================================================
// (high, low) → output indicators
// =============================================================================

#[test]
fn test_checkpoint_aroon() {
    let mut ind = StreamingAroon::new(14);
    for i in 0..20 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        ind.next((base + 1.5, base - 1.5));
    }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingAroon::restore_state(&bytes).unwrap();
    for i in 20..40 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        assert_eq!(ind.next((base + 1.5, base - 1.5)), restored.next((base + 1.5, base - 1.5)));
    }
}

#[test]
fn test_checkpoint_aroon_osc() {
    let mut ind = StreamingAroonOsc::new(14);
    for i in 0..20 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        ind.next((base + 1.5, base - 1.5));
    }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingAroonOsc::restore_state(&bytes).unwrap();
    for i in 20..40 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        assert_eq!(ind.next((base + 1.5, base - 1.5)), restored.next((base + 1.5, base - 1.5)));
    }
}

#[test]
fn test_checkpoint_fisher() {
    let mut ind = StreamingFisher::new(9);
    for i in 0..15 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        ind.next((base + 1.5, base - 1.5));
    }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingFisher::restore_state(&bytes).unwrap();
    for i in 15..35 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        assert_eq!(ind.next((base + 1.5, base - 1.5)), restored.next((base + 1.5, base - 1.5)));
    }
}

// =============================================================================
// Dual-input indicators (correl, beta) - use next_pair()
// =============================================================================

#[test]
fn test_checkpoint_correl() {
    let mut ind = StreamingCorrel::new(14);
    for i in 0..20 {
        ind.next_pair(i as f64, (i as f64) * 2.0 + 1.0);
    }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingCorrel::restore_state(&bytes).unwrap();
    for i in 20..40 {
        assert_eq!(
            ind.next_pair(i as f64, (i as f64) * 2.0 + 1.0),
            restored.next_pair(i as f64, (i as f64) * 2.0 + 1.0)
        );
    }
}

#[test]
fn test_checkpoint_beta() {
    let mut ind = StreamingBeta::new(14);
    for i in 0..20 {
        ind.next_pair(50.0 + i as f64, 100.0 + (i as f64) * 1.5);
    }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingBeta::restore_state(&bytes).unwrap();
    for i in 20..40 {
        assert_eq!(
            ind.next_pair(50.0 + i as f64, 100.0 + (i as f64) * 1.5),
            restored.next_pair(50.0 + i as f64, 100.0 + (i as f64) * 1.5)
        );
    }
}

// =============================================================================
// &dyn Ohlcv → f64 indicators
// =============================================================================

checkpoint_test_ohlcv!(test_checkpoint_obv, StreamingObv::new(), 10);
checkpoint_test_ohlcv!(test_checkpoint_vwap, StreamingVwap::new(), 10);
checkpoint_test_ohlcv!(test_checkpoint_ad, StreamingAd::new(), 10);
checkpoint_test_ohlcv!(test_checkpoint_nvi, StreamingNvi::new(), 10);
checkpoint_test_ohlcv!(test_checkpoint_pvi, StreamingPvi::new(), 10);
checkpoint_test_ohlcv!(test_checkpoint_pvt, StreamingPvt::new(), 10);
checkpoint_test_ohlcv!(test_checkpoint_anchored_vwap, StreamingAnchoredVwap::new(), 10);
checkpoint_test_ohlcv!(test_checkpoint_ao, StreamingAo::new(5, 34), 40);
checkpoint_test_ohlcv!(test_checkpoint_eom, StreamingEom::new(14), 20);
checkpoint_test_ohlcv!(test_checkpoint_force_index, StreamingForceIndex::new(13), 20);
checkpoint_test_ohlcv!(test_checkpoint_mfi, StreamingMfi::new(14), 20);
checkpoint_test_ohlcv!(test_checkpoint_natr, StreamingNatr::new(14), 20);
checkpoint_test_ohlcv!(test_checkpoint_willr, StreamingWillR::new(14), 20);
checkpoint_test_ohlcv!(test_checkpoint_vwma, StreamingVwma::new(14), 20);
checkpoint_test_ohlcv!(test_checkpoint_avgprice, StreamingAvgPrice::new(), 5);
checkpoint_test_ohlcv!(test_checkpoint_medprice, StreamingMedPrice::new(), 5);
checkpoint_test_ohlcv!(test_checkpoint_typprice, StreamingTypPrice::new(), 5);
checkpoint_test_ohlcv!(test_checkpoint_ar, StreamingAr::new(14), 20);
checkpoint_test_ohlcv!(test_checkpoint_br, StreamingBr::new(14), 20);
checkpoint_test_ohlcv!(test_checkpoint_cr, StreamingCr::new(14), 20);
checkpoint_test_ohlcv!(test_checkpoint_vr, StreamingVr::new(14), 20);

// &dyn Ohlcv → struct output
checkpoint_test_ohlcv!(test_checkpoint_donchian, StreamingDonchian::new(14), 20);
checkpoint_test_ohlcv!(test_checkpoint_ichimoku, StreamingIchimoku::new(9, 26, 52), 60);
checkpoint_test_ohlcv!(test_checkpoint_supertrend, StreamingSuperTrend::new(10, 3.0), 20);
checkpoint_test_ohlcv!(test_checkpoint_keltner, StreamingKeltner::new(20, 10, 2.0), 30);
// ElderRay takes (f64,f64,f64) tuple
#[test]
fn test_checkpoint_elder_ray() {
    let mut ind = StreamingElderRay::new(13);
    for i in 0..20 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        ind.next((base + 1.5, base - 1.5, base));
    }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingElderRay::restore_state(&bytes).unwrap();
    for i in 20..40 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        let input = (base + 1.5, base - 1.5, base);
        assert_eq!(ind.next(input), restored.next(input));
    }
}
checkpoint_test_ohlcv!(test_checkpoint_rvi, StreamingRvi::new(10), 15);
checkpoint_test_ohlcv!(test_checkpoint_sar, StreamingSar::new(0.02, 0.2), 20);

// =============================================================================
// OHLCV-based special (ADOSC, KVO, VWAP_BANDS, CMF)
// =============================================================================

#[test]
fn test_checkpoint_adosc() {
    let bars = make_bars(40);
    let mut ind = StreamingAdosc::new(3, 10);
    for bar in &bars[..20] { ind.next(bar as &dyn Ohlcv); }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingAdosc::restore_state(&bytes).unwrap();
    for bar in &bars[20..] {
        assert_eq!(ind.next(bar as &dyn Ohlcv), restored.next(bar as &dyn Ohlcv));
    }
}

#[test]
fn test_checkpoint_kvo() {
    let bars = make_bars(80);
    let mut ind = StreamingKvo::new(34, 55, 13);
    for bar in &bars[..60] { ind.next(bar as &dyn Ohlcv); }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingKvo::restore_state(&bytes).unwrap();
    for bar in &bars[60..] {
        assert_eq!(ind.next(bar as &dyn Ohlcv), restored.next(bar as &dyn Ohlcv));
    }
}

#[test]
fn test_checkpoint_vwap_bands() {
    let bars = make_bars(45);
    let mut ind = StreamingVwapBands::new(20, 2.0);
    for bar in &bars[..25] { ind.next(bar as &dyn Ohlcv); }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingVwapBands::restore_state(&bytes).unwrap();
    for bar in &bars[25..] {
        assert_eq!(ind.next(bar as &dyn Ohlcv), restored.next(bar as &dyn Ohlcv));
    }
}

#[test]
fn test_checkpoint_cmf() {
    let mut ind = StreamingCmf::new(14);
    for i in 0..20 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        ind.next((base + 1.5, base - 1.5, base, 1000.0 + i as f64 * 10.0));
    }
    let bytes = ind.save_state().unwrap();
    let mut restored = StreamingCmf::restore_state(&bytes).unwrap();
    for i in 20..40 {
        let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
        let input = (base + 1.5, base - 1.5, base, 1000.0 + i as f64 * 10.0);
        assert_eq!(ind.next(input), restored.next(input));
    }
}

// =============================================================================
// proptest: save→restore consistency for diverse indicators
// =============================================================================

#[cfg(test)]
mod proptest_checkpoint {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_sma_checkpoint(period in 2usize..50, warmup in 10usize..100, verify in 5usize..30) {
            let data: Vec<f64> = (0..(warmup + verify)).map(|i| 50.0 + (i as f64 * 0.1).sin() * 20.0).collect();
            let mut ind = StreamingSma::new(period);
            for &v in &data[..warmup] { ind.next(v); }
            let bytes = ind.save_state().unwrap();
            let mut restored = StreamingSma::restore_state(&bytes).unwrap();
            for &v in &data[warmup..] { prop_assert_eq!(ind.next(v), restored.next(v)); }
        }

        #[test]
        fn prop_ema_checkpoint(period in 2usize..50, warmup in 10usize..100, verify in 5usize..30) {
            let data: Vec<f64> = (0..(warmup + verify)).map(|i| 50.0 + (i as f64 * 0.1).sin() * 20.0).collect();
            let mut ind = StreamingEma::new(period);
            for &v in &data[..warmup] { ind.next(v); }
            let bytes = ind.save_state().unwrap();
            let mut restored = StreamingEma::restore_state(&bytes).unwrap();
            for &v in &data[warmup..] { prop_assert_eq!(ind.next(v), restored.next(v)); }
        }

        #[test]
        fn prop_rsi_checkpoint(period in 2usize..30, warmup in 20usize..80) {
            let data: Vec<f64> = (0..(warmup + 20)).map(|i| 50.0 + (i as f64 * 0.2).sin() * 15.0).collect();
            let mut ind = StreamingRsi::new(period);
            for &v in &data[..warmup] { ind.next(v); }
            let bytes = ind.save_state().unwrap();
            let mut restored = StreamingRsi::restore_state(&bytes).unwrap();
            for &v in &data[warmup..] { prop_assert_eq!(ind.next(v), restored.next(v)); }
        }

        #[test]
        fn prop_macd_checkpoint(fast in 2usize..10, slow in 10usize..30, signal in 2usize..10) {
            let data: Vec<f64> = (0..80).map(|i| 50.0 + (i as f64 * 0.15).sin() * 20.0).collect();
            let mut ind = StreamingMacd::new(fast, slow, signal);
            for &v in &data[..50] { ind.next(v); }
            let bytes = ind.save_state().unwrap();
            let mut restored = StreamingMacd::restore_state(&bytes).unwrap();
            for &v in &data[50..] { prop_assert_eq!(ind.next(v), restored.next(v)); }
        }

        #[test]
        fn prop_atr_checkpoint(period in 2usize..30, warmup in 15usize..60) {
            let mut ind = StreamingAtr::new(period);
            for i in 0..warmup {
                let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
                ind.next((base + 1.0, base - 1.0, base));
            }
            let bytes = ind.save_state().unwrap();
            let mut restored = StreamingAtr::restore_state(&bytes).unwrap();
            for i in warmup..(warmup + 20) {
                let base = 100.0 + (i as f64 * 0.2).sin() * 5.0;
                let input = (base + 1.0, base - 1.0, base);
                prop_assert_eq!(ind.next(input), restored.next(input));
            }
        }
    }
}
