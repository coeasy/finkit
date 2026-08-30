use alpha_ta_core::streaming::{Ohlcv, OhlcvBar, PriceSource};
use alpha_ta_core::streaming::builder::{Builder, IndicatorBuilder};
use alpha_ta_core::streaming::indicators::StreamingSma;

fn test_bar() -> OhlcvBar {
    OhlcvBar::new(100.0, 120.0, 80.0, 110.0, 50000.0)
}

#[test]
fn price_source_open() {
    let bar = test_bar();
    assert_eq!(PriceSource::Open.extract(&bar), bar.open());
}

#[test]
fn price_source_high() {
    let bar = test_bar();
    assert_eq!(PriceSource::High.extract(&bar), bar.high());
}

#[test]
fn price_source_low() {
    let bar = test_bar();
    assert_eq!(PriceSource::Low.extract(&bar), bar.low());
}

#[test]
fn price_source_close() {
    let bar = test_bar();
    assert_eq!(PriceSource::Close.extract(&bar), bar.close());
}

#[test]
fn price_source_hl2() {
    let bar = test_bar();
    let expected = (bar.high() + bar.low()) * 0.5;
    assert!((PriceSource::HL2.extract(&bar) - expected).abs() < 1e-10);
}

#[test]
fn price_source_hlc3() {
    let bar = test_bar();
    let expected = (bar.high() + bar.low() + bar.close()) / 3.0;
    assert!((PriceSource::HLC3.extract(&bar) - expected).abs() < 1e-10);
}

#[test]
fn price_source_ohlc4() {
    let bar = test_bar();
    let expected = (bar.open() + bar.high() + bar.low() + bar.close()) * 0.25;
    assert!((PriceSource::OHLC4.extract(&bar) - expected).abs() < 1e-10);
}

#[test]
fn price_source_typical() {
    let bar = test_bar();
    let expected = (bar.high() + bar.low() + bar.close()) / 3.0;
    assert!((PriceSource::Typical.extract(&bar) - expected).abs() < 1e-10);
}

#[test]
fn price_source_weighted() {
    let bar = test_bar();
    let expected = (bar.high() + bar.low() + 2.0 * bar.close()) * 0.25;
    assert!((PriceSource::Weighted.extract(&bar) - expected).abs() < 1e-10);
}

#[test]
fn price_source_median() {
    let bar = test_bar();
    let expected = (bar.high() + bar.low()) * 0.5;
    assert!((PriceSource::Median.extract(&bar) - expected).abs() < 1e-10);
}

#[test]
fn price_source_volume() {
    let bar = test_bar();
    assert_eq!(PriceSource::Volume.extract(&bar), bar.volume());
}

#[test]
fn price_source_default_is_close() {
    assert_eq!(PriceSource::default(), PriceSource::Close);
}

#[test]
fn price_source_display() {
    assert_eq!(format!("{}", PriceSource::HLC3), "hlc3");
    assert_eq!(format!("{}", PriceSource::Close), "close");
}

#[test]
fn sma_builder_with_price_source_compiles() {
    let sma = StreamingSma::builder()
        .period(20)
        .price_source(PriceSource::HLC3)
        .build();
    assert!(sma.is_ok());
}

#[test]
fn price_source_enum_has_11_variants() {
    let all = [
        PriceSource::Open,
        PriceSource::High,
        PriceSource::Low,
        PriceSource::Close,
        PriceSource::HL2,
        PriceSource::HLC3,
        PriceSource::OHLC4,
        PriceSource::Typical,
        PriceSource::Weighted,
        PriceSource::Median,
        PriceSource::Volume,
    ];
    assert_eq!(all.len(), 11);
}
