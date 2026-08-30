use finkit::error::IndicatorError;
use finkit::streaming::builder::{Builder, IndicatorBuilder};
use finkit::streaming::indicators::*;
use finkit::streaming::{IndicatorMeta, StreamingIndicator};

// ---------------------------------------------------------------------------
// Single-period builders — happy path
// ---------------------------------------------------------------------------

#[test]
fn test_sma_builder_ok() {
    let sma = StreamingSma::builder().period(20).build().unwrap();
    assert_eq!(sma.warm_up_period(), 20);
}

#[test]
fn test_ema_builder_ok() {
    let ema = StreamingEma::builder().period(14).build().unwrap();
    assert_eq!(ema.warm_up_period(), 14);
}

#[test]
fn test_rsi_builder_ok() {
    let rsi = StreamingRsi::builder().period(14).build().unwrap();
    assert_eq!(rsi.warm_up_period(), 15);
}

#[test]
fn test_atr_builder_ok() {
    let atr = StreamingAtr::builder().period(14).build().unwrap();
    assert_eq!(atr.warm_up_period(), 14);
}

#[test]
fn test_wma_builder_ok() {
    let wma = StreamingWma::builder().period(10).build().unwrap();
    assert_eq!(wma.warm_up_period(), 10);
}

#[test]
fn test_dema_builder_ok() {
    let dema = StreamingDema::builder().period(20).build().unwrap();
    assert!(dema.warm_up_period() > 0);
}

#[test]
fn test_tema_builder_ok() {
    let tema = StreamingTema::builder().period(10).build().unwrap();
    assert!(tema.warm_up_period() > 0);
}

#[test]
fn test_roc_builder_ok() {
    let _roc = StreamingRoc::builder().period(10).build().unwrap();
}

#[test]
fn test_mom_builder_ok() {
    let _mom = StreamingMom::builder().period(10).build().unwrap();
}

#[test]
fn test_hma_builder_ok() {
    let _hma = StreamingHma::builder().period(9).build().unwrap();
}

#[test]
fn test_cci_builder_ok() {
    let _cci = StreamingCci::builder().period(20).build().unwrap();
}

// ---------------------------------------------------------------------------
// Single-period builders — error cases
// ---------------------------------------------------------------------------

#[test]
fn test_sma_builder_zero_period() {
    let result = StreamingSma::builder().period(0).build();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(matches!(err, IndicatorError::InvalidParameter { .. }));
}

#[test]
fn test_ema_builder_missing_period() {
    let result = StreamingEma::builder().build();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(matches!(err, IndicatorError::InvalidParameter { .. }));
}

#[test]
fn test_rsi_builder_zero_period() {
    let result = StreamingRsi::builder().period(0).build();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(matches!(err, IndicatorError::InvalidParameter { .. }));
}

// ---------------------------------------------------------------------------
// No-param builders
// ---------------------------------------------------------------------------

#[test]
fn test_obv_builder_ok() {
    let _obv = StreamingObv::builder().build().unwrap();
}

#[test]
fn test_avgprice_builder_ok() {
    let _ap = StreamingAvgPrice::builder().build().unwrap();
}

#[test]
fn test_trange_builder_ok() {
    let _tr = StreamingTrange::builder().build().unwrap();
}

#[test]
fn test_vwap_builder_ok() {
    let _vwap = StreamingVwap::builder().build().unwrap();
}

// ---------------------------------------------------------------------------
// Two-period builders
// ---------------------------------------------------------------------------

#[test]
fn test_apo_builder_ok() {
    let _apo = StreamingApo::builder()
        .fast_period(12)
        .slow_period(26)
        .build()
        .unwrap();
}

#[test]
fn test_apo_builder_missing_param() {
    let result = StreamingApo::builder().fast_period(12).build();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(matches!(err, IndicatorError::InvalidParameter { .. }));
}

#[test]
fn test_tsi_builder_ok() {
    let _tsi = StreamingTsi::builder()
        .long_period(25)
        .short_period(13)
        .build()
        .unwrap();
}

#[test]
fn test_vidya_builder_zero_period() {
    let result = StreamingVidya::builder()
        .period(0)
        .cmo_period(9)
        .build();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(matches!(err, IndicatorError::InvalidParameter { .. }));
}

// ---------------------------------------------------------------------------
// Three-period builders
// ---------------------------------------------------------------------------

#[test]
fn test_macd_builder_ok() {
    let macd = StreamingMacd::builder()
        .fast_period(12)
        .slow_period(26)
        .signal_period(9)
        .build()
        .unwrap();
    assert_eq!(macd.warm_up_period(), 34);
}

#[test]
fn test_stoch_builder_ok() {
    let _stoch = StreamingStoch::builder()
        .k_period(14)
        .k_slow(3)
        .d_period(3)
        .build()
        .unwrap();
}

#[test]
fn test_kdj_builder_ok() {
    let _kdj = StreamingKdj::builder()
        .n(9)
        .m1(3)
        .m2(3)
        .build()
        .unwrap();
}

#[test]
fn test_ichimoku_builder_ok() {
    let _ichi = StreamingIchimoku::builder()
        .tenkan(9)
        .kijun(26)
        .senkou_b(52)
        .build()
        .unwrap();
}

#[test]
fn test_macd_builder_missing_signal() {
    let result = StreamingMacd::builder()
        .fast_period(12)
        .slow_period(26)
        .build();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(matches!(err, IndicatorError::InvalidParameter { .. }));
}

// ---------------------------------------------------------------------------
// Special builders (mixed types)
// ---------------------------------------------------------------------------

#[test]
fn test_sar_builder_ok() {
    let _sar = StreamingSar::builder()
        .acceleration(0.02)
        .maximum(0.2)
        .build()
        .unwrap();
}

#[test]
fn test_sar_builder_negative_acceleration() {
    let result = StreamingSar::builder()
        .acceleration(-0.02)
        .maximum(0.2)
        .build();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(matches!(err, IndicatorError::InvalidParameter { .. }));
}

#[test]
fn test_supertrend_builder_ok() {
    let _st = StreamingSuperTrend::builder()
        .period(10)
        .multiplier(3.0)
        .build()
        .unwrap();
}

#[test]
fn test_supertrend_builder_zero_period() {
    let result = StreamingSuperTrend::builder()
        .period(0)
        .multiplier(3.0)
        .build();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(matches!(err, IndicatorError::InvalidParameter { .. }));
}

#[test]
fn test_boll_builder_ok() {
    let boll = StreamingBoll::builder()
        .period(20)
        .nb_dev_up(2.0)
        .nb_dev_dn(2.0)
        .build()
        .unwrap();
    assert_eq!(boll.warm_up_period(), 20);
}

#[test]
fn test_boll_builder_negative_dev() {
    let result = StreamingBoll::builder()
        .period(20)
        .nb_dev_up(-1.0)
        .nb_dev_dn(2.0)
        .build();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(matches!(err, IndicatorError::InvalidParameter { .. }));
}

#[test]
fn test_alma_builder_ok() {
    let _alma = StreamingAlma::builder()
        .period(9)
        .sigma(6.0)
        .offset(0.85)
        .build()
        .unwrap();
}

#[test]
fn test_alma_builder_invalid_offset() {
    let result = StreamingAlma::builder()
        .period(9)
        .sigma(6.0)
        .offset(1.5)
        .build();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(matches!(err, IndicatorError::InvalidParameter { .. }));
}

#[test]
fn test_keltner_builder_ok() {
    let _kelt = StreamingKeltner::builder()
        .ema_period(20)
        .atr_period(10)
        .multiplier(1.5)
        .build()
        .unwrap();
}

#[test]
fn test_ene_builder_ok() {
    let _ene = StreamingEne::builder()
        .period(25)
        .k1(11.0)
        .k2(9.0)
        .build()
        .unwrap();
}

#[test]
fn test_kst_builder_ok() {
    let _kst = StreamingKst::builder()
        .roc1(10)
        .roc2(15)
        .roc3(20)
        .roc4(30)
        .sma1(10)
        .sma2(10)
        .sma3(10)
        .sma4(15)
        .signal_period(9)
        .build()
        .unwrap();
}

#[test]
fn test_kst_builder_missing_param() {
    let result = StreamingKst::builder()
        .roc1(10)
        .roc2(15)
        .build();
    assert!(result.is_err());
    let err = result.err().unwrap();
    assert!(matches!(err, IndicatorError::InvalidParameter { .. }));
}

// ---------------------------------------------------------------------------
// Functional: builder produces equivalent indicator to new()
// ---------------------------------------------------------------------------

#[test]
fn test_sma_builder_equivalence() {
    let mut sma_new = StreamingSma::new(5);
    let mut sma_builder = StreamingSma::builder().period(5).build().unwrap();

    let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    for &v in &data {
        let a = sma_new.next(v);
        let b = sma_builder.next(v);
        assert_eq!(a, b);
    }
}

#[test]
fn test_macd_builder_equivalence() {
    let mut macd_new = StreamingMacd::new(12, 26, 9);
    let mut macd_builder = StreamingMacd::builder()
        .fast_period(12)
        .slow_period(26)
        .signal_period(9)
        .build()
        .unwrap();

    let data: Vec<f64> = (0..50).map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0).collect();
    for &v in &data {
        let a = macd_new.next(v);
        let b = macd_builder.next(v);
        assert_eq!(a, b);
    }
}

// ---------------------------------------------------------------------------
// All 69 indicators have IndicatorBuilder — compilation coverage
// ---------------------------------------------------------------------------

#[test]
fn test_all_single_period_builders_compile() {
    let _ = StreamingAdx::builder().period(14).build().unwrap();
    let _ = StreamingAr::builder().period(26).build().unwrap();
    let _ = StreamingAroon::builder().period(25).build().unwrap();
    let _ = StreamingBias::builder().period(6).build().unwrap();
    let _ = StreamingBr::builder().period(26).build().unwrap();
    let _ = StreamingChop::builder().period(14).build().unwrap();
    let _ = StreamingCmf::builder().period(20).build().unwrap();
    let _ = StreamingCr::builder().period(26).build().unwrap();
    let _ = StreamingDonchian::builder().period(20).build().unwrap();
    let _ = StreamingDpo::builder().period(20).build().unwrap();
    let _ = StreamingEom::builder().period(14).build().unwrap();
    let _ = StreamingFisher::builder().period(9).build().unwrap();
    let _ = StreamingForceIndex::builder().period(13).build().unwrap();
    let _ = StreamingKama::builder().period(10).build().unwrap();
    let _ = StreamingLinReg::builder().period(14).build().unwrap();
    let _ = StreamingMcGinley::builder().period(14).build().unwrap();
    let _ = StreamingMfi::builder().period(14).build().unwrap();
    let _ = StreamingNatr::builder().period(14).build().unwrap();
    let _ = StreamingPsy::builder().period(12).build().unwrap();
    let _ = StreamingRvi::builder().period(10).build().unwrap();
    let _ = StreamingStc::builder().fast_period(23).slow_period(50).cycle(10).build().unwrap();
    let _ = StreamingT3::builder().period(5).build().unwrap();
    let _ = StreamingTrix::builder().period(15).build().unwrap();
    let _ = StreamingUlcerIndex::builder().period(14).build().unwrap();
    let _ = StreamingVr::builder().period(26).build().unwrap();
    let _ = StreamingVwma::builder().period(20).build().unwrap();
    let _ = StreamingWillR::builder().period(14).build().unwrap();
    let _ = StreamingZlema::builder().period(20).build().unwrap();
    let _ = StreamingZscore::builder().period(20).build().unwrap();
}

#[test]
fn test_all_two_period_builders_compile() {
    let _ = StreamingAo::builder().fast_period(5).slow_period(34).build().unwrap();
    let _ = StreamingExpma::builder().short_period(12).long_period(50).build().unwrap();
    let _ = StreamingMassIndex::builder().period(25).ema_period(9).build().unwrap();
}

#[test]
fn test_all_three_period_builders_compile() {
    let _ = StreamingCoppock::builder().wma_period(10).long_roc(14).short_roc(11).build().unwrap();
    let _ = StreamingDma::builder().short_period(10).long_period(50).ama_period(10).build().unwrap();
    let _ = StreamingKvo::builder().fast_period(34).slow_period(55).signal_period(13).build().unwrap();
}

#[test]
fn test_all_no_param_builders_compile() {
    let _ = StreamingMedPrice::builder().build().unwrap();
    let _ = StreamingNvi::builder().build().unwrap();
    let _ = StreamingPvi::builder().build().unwrap();
    let _ = StreamingPvt::builder().build().unwrap();
    let _ = StreamingTypPrice::builder().build().unwrap();
}
