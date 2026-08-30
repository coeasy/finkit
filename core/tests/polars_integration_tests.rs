#![cfg(feature = "fta-polars")]

use polars::prelude::*;
use finkit::polars_ext::{TaDataFrame, TaSeries};

fn sample_close_series() -> Series {
    let data: Vec<f64> = (1..=50).map(|i| 100.0 + (i as f64).sin() * 10.0).collect();
    Series::new("close".into(), data)
}

fn sample_dataframe() -> DataFrame {
    let close: Vec<f64> = (1..=50).map(|i| 100.0 + (i as f64).sin() * 10.0).collect();
    let high: Vec<f64> = close.iter().map(|x| x + 2.0).collect();
    let low: Vec<f64> = close.iter().map(|x| x - 2.0).collect();
    DataFrame::new(vec![
        Column::new("close".into(), close),
        Column::new("high".into(), high),
        Column::new("low".into(), low),
    ])
    .unwrap()
}

#[test]
fn test_series_ta_sma() {
    let series = sample_close_series();
    let result = series.ta_sma(5).unwrap();
    assert!(!result.is_empty());
    assert_eq!(result.name().as_str(), "close");
}

#[test]
fn test_series_ta_sma_period_20() {
    let series = sample_close_series();
    let result = series.ta_sma(20).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn test_series_ta_ema() {
    let series = sample_close_series();
    let result = series.ta_ema(12).unwrap();
    assert!(!result.is_empty());
    assert_eq!(result.dtype(), &DataType::Float64);
}

#[test]
fn test_series_ta_rsi() {
    let series = sample_close_series();
    let result = series.ta_rsi(14).unwrap();
    assert!(!result.is_empty());
    let ca = result.f64().unwrap();
    for val in ca.into_no_null_iter() {
        if !val.is_nan() {
            assert!((0.0..=100.0).contains(&val), "RSI value {} out of range", val);
        }
    }
}

#[test]
fn test_series_ta_bbands() {
    let series = sample_close_series();
    let (upper, middle, lower) = series.ta_bbands(20, 2.0).unwrap();
    assert_eq!(upper.len(), middle.len());
    assert_eq!(middle.len(), lower.len());
    assert!(!upper.is_empty());
}

#[test]
fn test_dataframe_ta_sma() {
    let df = sample_dataframe();
    let result = df.ta().sma("close", 5).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn test_dataframe_ta_ema() {
    let df = sample_dataframe();
    let result = df.ta().ema("close", 10).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn test_dataframe_ta_rsi() {
    let df = sample_dataframe();
    let result = df.ta().rsi("close", 14).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn test_dataframe_ta_bbands() {
    let df = sample_dataframe();
    let (upper, _middle, lower) = df.ta().bbands("close", 20, 2.0).unwrap();
    assert!(!upper.is_empty());
    assert!(!lower.is_empty());
}

#[test]
fn test_series_ta_sma_empty_error() {
    let series = Series::new("empty".into(), Vec::<f64>::new());
    let result = series.ta_sma(5);
    assert!(result.is_err());
}

#[test]
fn test_dataframe_invalid_column() {
    let df = sample_dataframe();
    let result = df.ta().sma("nonexistent", 5);
    assert!(result.is_err());
}

#[test]
fn test_series_sma_returns_series_type() {
    let series = sample_close_series();
    let result = series.ta_sma(5).unwrap();
    assert_eq!(result.dtype(), &DataType::Float64);
}

#[test]
fn test_series_ta_sma_values_reasonable() {
    let data: Vec<f64> = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];
    let series = Series::new("price".into(), data);
    let result = series.ta_sma(3).unwrap();
    let ca = result.f64().unwrap();
    let values: Vec<f64> = ca.into_no_null_iter().collect();
    let last = values.last().unwrap();
    assert!((*last - 90.0).abs() < 1e-10);
}
