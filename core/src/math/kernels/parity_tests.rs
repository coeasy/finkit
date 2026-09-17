//! Cross-check canonical streaming kernels against stable public APIs.

use super::RsiState;
use crate::indicators;

fn assert_series_eq(left: &[f64], right: &[f64], tolerance: f64) {
    assert_eq!(left.len(), right.len());
    for (index, (lhs, rhs)) in left.iter().zip(right.iter()).enumerate() {
        if lhs.is_nan() && rhs.is_nan() {
            continue;
        }
        assert!(
            (lhs - rhs).abs() <= tolerance,
            "series mismatch at {index}: {lhs} vs {rhs}"
        );
    }
}

#[test]
fn canonical_rsi_matches_public_api() {
    let input: Vec<f64> = (0..256)
        .map(|i| 100.0 + (i as f64 * 0.13).sin() * 8.0 + (i as f64 * 0.031).cos())
        .collect();
    let period = 14;
    let legacy = indicators::rsi(&input, period).unwrap();
    let mut canonical = vec![f64::NAN; input.len()];
    let mut state = RsiState::new(period);
    for (index, value) in input.iter().copied().enumerate() {
        if let Some(rsi) = state.update(value) {
            canonical[index] = rsi;
        }
    }
    assert_series_eq(legacy.as_slice().unwrap(), &canonical, 1e-10);
}

#[test]
fn canonical_rsi_preserves_warmup_alignment() {
    let input: Vec<f64> = (1..=32).map(|value| value as f64).collect();
    let period = 5;
    let legacy = indicators::rsi(&input, period).unwrap();
    let mut canonical = vec![f64::NAN; input.len()];
    let mut state = RsiState::new(period);
    for (index, value) in input.iter().copied().enumerate() {
        if let Some(rsi) = state.update(value) {
            canonical[index] = rsi;
        }
    }
    assert_series_eq(legacy.as_slice().unwrap(), &canonical, 1e-12);
}
