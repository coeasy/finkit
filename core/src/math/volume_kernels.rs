//! Allocation-free volume indicator kernels for caller-owned output buffers.
//!
//! This module is deliberately below the public indicator layer.  It gives the
//! Python/FFI bindings and the canonical allocating APIs a common hot path without
//! forcing a temporary `Vec`/`Array1` conversion.

use crate::error::{Result, TaError};

#[inline]
fn validate_same_len(name: &'static str, expected: usize, actual: usize) -> Result<()> {
    if expected != actual {
        return Err(TaError::InvalidParameter {
            name: name.to_string(),
            constraint: "must have the same length as the primary input".to_string(),
        });
    }
    Ok(())
}

/// Compute On-Balance Volume directly into `output`.
#[inline]
pub fn obv_into(close: &[f64], volume: &[f64], output: &mut [f64]) -> Result<()> {
    validate_same_len("volume", close.len(), volume.len())?;
    validate_same_len("output", close.len(), output.len())?;
    if close.is_empty() {
        return Err(TaError::EmptyInput);
    }
    crate::math::simd_ops::simd_obv(close, volume, output);
    Ok(())
}

/// Compute cumulative VWAP directly into `output`.
///
/// This is a single-pass recurrence with no scratch allocation.  A zero cumulative
/// volume leaves the corresponding output at `0.0`, matching the current public VWAP
/// implementation.
#[inline]
pub fn vwap_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    output: &mut [f64],
) -> Result<()> {
    let len = high.len();
    validate_same_len("low", len, low.len())?;
    validate_same_len("close", len, close.len())?;
    validate_same_len("volume", len, volume.len())?;
    validate_same_len("output", len, output.len())?;
    if len == 0 {
        return Err(TaError::EmptyInput);
    }

    let mut cum_tp_vol = 0.0;
    let mut cum_volume = 0.0;
    for i in 0..len {
        let typical_price = (high[i] + low[i] + close[i]) * (1.0 / 3.0);
        cum_tp_vol = typical_price.mul_add(volume[i], cum_tp_vol);
        cum_volume += volume[i];
        output[i] = if cum_volume.abs() > 1e-15 {
            cum_tp_vol / cum_volume
        } else {
            0.0
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obv_into_matches_expected_sequence() {
        let close = [10.0, 11.0, 10.0, 10.0, 12.0];
        let volume = [100.0, 50.0, 20.0, 30.0, 10.0];
        let mut out = [0.0; 5];
        obv_into(&close, &volume, &mut out).unwrap();
        let expected = [100.0, 150.0, 130.0, 130.0, 140.0];
        assert_eq!(out, expected);
    }

    #[test]
    fn vwap_into_is_cumulative_and_allocation_free_at_api_boundary() {
        let high = [10.0, 12.0, 14.0];
        let low = [8.0, 10.0, 12.0];
        let close = [9.0, 11.0, 13.0];
        let volume = [1.0, 2.0, 1.0];
        let mut out = [0.0; 3];
        vwap_into(&high, &low, &close, &volume, &mut out).unwrap();
        assert!((out[0] - 9.0).abs() < 1e-12);
        assert!((out[1] - (31.0 / 3.0)).abs() < 1e-12);
        assert!((out[2] - 11.0).abs() < 1e-12);
    }

    #[test]
    fn output_length_is_part_of_the_contract() {
        let close = [1.0, 2.0];
        let volume = [1.0, 1.0];
        let mut out = [0.0; 1];
        assert!(obv_into(&close, &volume, &mut out).is_err());
    }
}
