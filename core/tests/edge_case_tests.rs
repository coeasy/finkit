use finkit::indicators;
use finkit::math::moving_avg;

#[test]
fn test_sma_empty_input() {
    let result = moving_avg::sma(&[], 3);
    assert!(result.is_err());
}

#[test]
fn test_sma_single_element() {
    let result = moving_avg::sma(&[42.0], 1);
    assert!(result.is_ok());
    let arr = result.unwrap();
    assert_eq!(arr.len(), 1);
    assert!((arr[0] - 42.0).abs() < 1e-10);
}

#[test]
fn test_sma_period_zero() {
    let result = moving_avg::sma(&[1.0, 2.0, 3.0], 0);
    assert!(result.is_err());
}

#[test]
fn test_sma_period_exceeds_length() {
    let result = moving_avg::sma(&[1.0, 2.0], 5);
    assert!(result.is_err());
}

#[test]
fn test_sma_with_nan() {
    // R-1: non-finite inputs are rejected with InvalidParameter (R-1 resilience).
    let data = vec![1.0, f64::NAN, 3.0, 4.0, 5.0];
    let result = moving_avg::sma(&data, 3);
    assert!(result.is_err());
}

#[test]
fn test_sma_with_infinity() {
    // R-1: non-finite inputs are rejected with InvalidParameter (R-1 resilience).
    let data = vec![1.0, f64::INFINITY, 3.0, 4.0, 5.0];
    let result = moving_avg::sma(&data, 3);
    assert!(result.is_err());
}

#[test]
fn test_sma_max_f64() {
    let data = vec![f64::MAX / 4.0, f64::MAX / 4.0, f64::MAX / 4.0];
    let result = moving_avg::sma(&data, 3).unwrap();
    assert!(result[2].is_finite());
}

#[test]
fn test_ema_empty_input() {
    let result = moving_avg::ema(&[], 3);
    assert!(result.is_err());
}

#[test]
fn test_ema_single_element() {
    let result = moving_avg::ema(&[42.0], 1);
    assert!(result.is_ok());
}

#[test]
fn test_rsi_constant_input() {
    let data = vec![50.0; 20];
    let result = indicators::rsi(&data, 14).unwrap();
    for i in 14..data.len() {
        assert!(
            !result[i].is_nan(),
            "RSI should not be NaN for constant input at index {}",
            i
        );
    }
}

#[test]
fn test_rsi_all_gains() {
    let data: Vec<f64> = (1..=20).map(|x| x as f64).collect();
    let result = indicators::rsi(&data, 14).unwrap();
    for i in 14..data.len() {
        assert!(
            (result[i] - 100.0).abs() < 1e-10,
            "RSI should be 100 for all-gain series"
        );
    }
}

#[test]
fn test_rsi_all_losses() {
    let data: Vec<f64> = (1..=20).rev().map(|x| x as f64).collect();
    let result = indicators::rsi(&data, 14).unwrap();
    for i in 14..data.len() {
        assert!(
            result[i].abs() < 1e-10,
            "RSI should be 0 for all-loss series"
        );
    }
}

#[test]
fn test_bbands_single_value_input() {
    let data = vec![10.0; 20];
    let result = indicators::bbands(&data, 14, 2.0, 2.0).unwrap();
    for i in 13..20 {
        assert!(
            (result.upper[i] - result.lower[i]).abs() < 1e-10,
            "Bands should converge for constant input"
        );
    }
}

#[test]
fn test_atr_equal_hlc() {
    let high = vec![50.0; 20];
    let low = vec![50.0; 20];
    let close = vec![50.0; 20];
    let result = indicators::volatility::atr(&high, &low, &close, 14).unwrap();
    for i in 14..20 {
        assert!(result[i].abs() < 1e-10, "ATR should be 0 for equal H=L=C");
    }
}

#[test]
fn test_macd_empty() {
    let result = indicators::macd(&[], 12, 26, 9);
    assert!(result.is_err());
}

#[test]
fn test_stoch_empty() {
    let result = indicators::stoch(&[], &[], &[], 5, 3, 3);
    assert!(result.is_err());
}

#[test]
fn test_wma_period_one() {
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let result = moving_avg::wma(&data, 1).unwrap();
    for (i, v) in result.iter().enumerate() {
        assert!((v - data[i]).abs() < 1e-10);
    }
}

#[test]
fn test_dema_short_input() {
    let data = vec![1.0, 2.0, 3.0];
    let result = moving_avg::dema(&data, 2);
    assert!(result.is_ok());
}

#[test]
fn test_tema_short_input() {
    let data = vec![1.0, 2.0, 3.0];
    let result = moving_avg::tema(&data, 2);
    assert!(result.is_ok());
}

#[test]
fn test_cci_period_larger_than_data() {
    let high = vec![10.0; 5];
    let low = vec![9.0; 5];
    let close = vec![9.5; 5];
    let result = indicators::cci(&high, &low, &close, 20);
    assert!(result.is_err());
}

#[test]
fn test_mom_zero_period() {
    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let result = indicators::mom(&data, 0);
    assert!(result.is_err());
}

#[test]
fn test_negative_prices_sma() {
    let data = vec![-5.0, -3.0, -1.0, 1.0, 3.0];
    let result = moving_avg::sma(&data, 3).unwrap();
    assert_eq!(result.len(), 5);
    assert!((result[2] - (-3.0)).abs() < 1e-10);
}

#[test]
fn test_very_small_values() {
    let data = vec![1e-300, 2e-300, 3e-300, 4e-300, 5e-300];
    let result = moving_avg::sma(&data, 3).unwrap();
    assert!((result[2] - 2e-300).abs() < 1e-310);
}

#[test]
fn test_obv_empty() {
    let result = indicators::obv(&[], &[]);
    assert!(result.is_err());
}
