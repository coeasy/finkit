//! Edge case: NaN / ±Inf input rejection (R-1).
//!
//! Verifies that the public batch indicator entry points return
//! `Err(TaError::InvalidParameter { .. })` (and never panic) when the
//! input slice contains non-finite floating-point values. Also covers
//! the `metrics` + `tracing` warning paths by simply ensuring the error
//! surfaces.

use alpha_ta_core::error::TaError;
use alpha_ta_core::indicators::{bbands, macd, rsi};
use alpha_ta_core::math::moving_avg::{dema, ema, kama, sma, wma};

fn assert_invalid_param(err: alpha_ta_core::error::Result<ndarray::Array1<f64>>, needle: &str) {
    match err {
        Err(TaError::InvalidParameter { name, constraint }) => {
            assert!(
                name == "input" || name == "period" || name == "output" || name == "close",
                "unexpected param name: {name}"
            );
            assert!(
                constraint.contains(needle),
                "constraint {constraint:?} should contain {needle:?}"
            );
        }
        Err(other) => panic!("expected InvalidParameter, got {other:?}"),
        Ok(v) => panic!("expected error, got Ok({v:?})"),
    }
}

#[test]
fn sma_rejects_nan() {
    let input = vec![1.0, f64::NAN, 3.0, 4.0, 5.0];
    assert_invalid_param(sma(&input, 2), "non-finite value at index 1");
}

#[test]
fn sma_rejects_positive_infinity() {
    let input = vec![1.0, f64::INFINITY, 3.0, 4.0, 5.0];
    assert_invalid_param(sma(&input, 2), "non-finite value at index 1");
}

#[test]
fn sma_rejects_negative_infinity() {
    let input = vec![1.0, 2.0, 3.0, f64::NEG_INFINITY, 5.0];
    assert_invalid_param(sma(&input, 2), "non-finite value at index 3");
}

#[test]
fn ema_rejects_nan() {
    let input = vec![1.0, 2.0, 3.0, f64::NAN, 5.0];
    assert_invalid_param(ema(&input, 2), "non-finite value at index 3");
}

#[test]
fn wma_rejects_nan() {
    let input = vec![1.0, 2.0, f64::NAN, 4.0, 5.0];
    assert_invalid_param(wma(&input, 2), "non-finite value at index 2");
}

#[test]
fn dema_rejects_nan() {
    let input = vec![1.0, 2.0, 3.0, 4.0, f64::NAN];
    assert_invalid_param(dema(&input, 2), "non-finite value at index 4");
}

#[test]
fn kama_rejects_nan() {
    let input = vec![1.0, 2.0, 3.0, 4.0, f64::NAN, 6.0, 7.0, 8.0];
    assert_invalid_param(kama(&input, 4, 2, 30), "non-finite value at index 4");
}

#[test]
fn rsi_rejects_nan() {
    let input = vec![1.0, 2.0, 3.0, 4.0, f64::NAN, 6.0];
    // RSI may have its own pre-checks; the point is that we never panic
    // and the result is an Err, not an Ok with poisoned NaN values.
    let r = rsi(&input, 2);
    assert!(r.is_err(), "rsi should error on NaN input");
}

#[test]
fn macd_rejects_nan() {
    let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, f64::NAN, 7.0];
    let r = macd(&input, 3, 5, 2);
    assert!(r.is_err(), "macd should error on NaN input");
}

#[test]
fn bbands_rejects_nan() {
    let input = vec![1.0, 2.0, 3.0, 4.0, f64::NAN, 6.0];
    let r = bbands(&input, 3, 2.0, 2.0);
    assert!(r.is_err(), "bbands should error on NaN input");
}

#[test]
fn sma_still_works_on_clean_input() {
    // Regression guard: the new guard must not affect the happy path.
    let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let r = sma(&input, 3).expect("clean input should succeed");
    assert_eq!(r.len(), 10);
}

#[test]
fn sma_zero_period_returns_error() {
    let input = vec![1.0, 2.0, 3.0];
    match sma(&input, 0) {
        Err(TaError::InvalidParameter { name, constraint }) => {
            assert_eq!(name, "period");
            assert!(constraint.contains("greater than 0"));
        }
        other => panic!("expected InvalidParameter, got {other:?}"),
    }
}
