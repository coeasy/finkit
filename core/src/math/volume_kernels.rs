//! Allocation-free volume indicator kernels for caller-owned output buffers.
//!
//! This module is deliberately below the public indicator layer.  It gives the
//! Python/FFI bindings and the canonical allocating APIs a common hot path without
//! forcing a temporary `Vec`/`Array1` conversion.

use crate::error::{Result, TaError};
use ndarray::Array1;

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
///
/// OBV is a serial recurrence.  A temporary delta vector plus a second prefix
/// pass looks SIMD-friendly, but is materially slower for large arrays because
/// it doubles memory traffic and allocates another full-length buffer.  Keep the
/// reference single-pass recurrence and let LLVM optimise the pointer loop.
#[inline]
pub fn obv_into(close: &[f64], volume: &[f64], output: &mut [f64]) -> Result<()> {
    validate_same_len("volume", close.len(), volume.len())?;
    validate_same_len("output", close.len(), output.len())?;
    if close.is_empty() {
        return Err(TaError::EmptyInput);
    }

    unsafe {
        let close_ptr = close.as_ptr();
        let volume_ptr = volume.as_ptr();
        let output_ptr = output.as_mut_ptr();
        let mut acc = *volume_ptr;
        *output_ptr = acc;

        for i in 1..close.len() {
            let current = *close_ptr.add(i);
            let previous = *close_ptr.add(i - 1);
            let volume_value = *volume_ptr.add(i);
            if current > previous {
                acc += volume_value;
            } else if current < previous {
                acc -= volume_value;
            }
            *output_ptr.add(i) = acc;
        }
    }
    Ok(())
}

/// Allocating OBV wrapper sharing the same canonical single-pass kernel.
pub fn obv(close: &[f64], volume: &[f64]) -> Result<Array1<f64>> {
    let mut output = vec![0.0; close.len()];
    obv_into(close, volume, &mut output)?;
    Ok(Array1::from_vec(output))
}

/// Compute the Accumulation/Distribution line directly into `output`.
///
/// The cumulative dependency makes a second prefix-sum pass unnecessary.  This
/// single loop has the same arithmetic order as the existing scalar reference
/// path while avoiding both scratch storage and an additional read/write pass.
#[inline]
pub fn ad_into(
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

    unsafe {
        let high_ptr = high.as_ptr();
        let low_ptr = low.as_ptr();
        let close_ptr = close.as_ptr();
        let volume_ptr = volume.as_ptr();
        let output_ptr = output.as_mut_ptr();
        let mut acc = 0.0;

        for i in 0..len {
            let h = *high_ptr.add(i);
            let l = *low_ptr.add(i);
            let range = h - l;
            if range.abs() >= 1e-15 {
                let c = *close_ptr.add(i);
                let multiplier = ((c - l) - (h - c)) / range;
                acc += multiplier * *volume_ptr.add(i);
            }
            *output_ptr.add(i) = acc;
        }
    }
    Ok(())
}

/// Allocating A/D wrapper sharing the canonical single-pass kernel.
pub fn ad(high: &[f64], low: &[f64], close: &[f64], volume: &[f64]) -> Result<Array1<f64>> {
    let mut output = vec![0.0; high.len()];
    ad_into(high, low, close, volume, &mut output)?;
    Ok(Array1::from_vec(output))
}

/// Compute Chaikin A/D Oscillator in a single pass.
///
/// This fuses AD accumulation and both EMA recurrences.  The previous hot path
/// materialised the entire AD line, then scanned it again for EMA smoothing;
/// the fused recurrence preserves the exact operation order but removes that
/// full-length scratch allocation and second memory pass.
#[inline]
pub fn adosc_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    fast_period: usize,
    slow_period: usize,
    output: &mut [f64],
) -> Result<()> {
    let len = high.len();
    validate_same_len("low", len, low.len())?;
    validate_same_len("close", len, close.len())?;
    validate_same_len("volume", len, volume.len())?;
    validate_same_len("output", len, output.len())?;
    if fast_period == 0 || slow_period == 0 {
        return Err(TaError::InvalidParameter {
            name: "fast_period and slow_period".to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    if len < slow_period {
        return Err(TaError::InsufficientData {
            length: len,
            required: slow_period,
        });
    }

    let fast_k = 2.0 / (fast_period as f64 + 1.0);
    let fast_one_k = 1.0 - fast_k;
    let slow_k = 2.0 / (slow_period as f64 + 1.0);
    let slow_one_k = 1.0 - slow_k;

    unsafe {
        let high_ptr = high.as_ptr();
        let low_ptr = low.as_ptr();
        let close_ptr = close.as_ptr();
        let volume_ptr = volume.as_ptr();
        let output_ptr = output.as_mut_ptr();
        let mut cumulative = 0.0;
        let mut fast_ema = 0.0;
        let mut slow_ema = 0.0;

        for i in 0..len {
            let h = *high_ptr.add(i);
            let l = *low_ptr.add(i);
            let range = h - l;
            if range.abs() >= 1e-15 {
                let c = *close_ptr.add(i);
                let multiplier = ((c - l) - (h - c)) / range;
                cumulative += multiplier * *volume_ptr.add(i);
            }

            if i == 0 {
                fast_ema = cumulative;
                slow_ema = cumulative;
            } else {
                fast_ema = cumulative * fast_k + fast_ema * fast_one_k;
                slow_ema = cumulative * slow_k + slow_ema * slow_one_k;
            }
            *output_ptr.add(i) = if i >= slow_period - 1 {
                fast_ema - slow_ema
            } else {
                0.0
            };
        }
    }
    Ok(())
}

/// Allocating ADOSC wrapper sharing the canonical fused recurrence.
pub fn adosc(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    fast_period: usize,
    slow_period: usize,
) -> Result<Array1<f64>> {
    let mut output = vec![0.0; high.len()];
    adosc_into(
        high,
        low,
        close,
        volume,
        fast_period,
        slow_period,
        &mut output,
    )?;
    Ok(Array1::from_vec(output))
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
    fn allocating_obv_wrapper_matches_into() {
        let close = [10.0, 11.0, 10.0, 12.0];
        let volume = [100.0, 50.0, 20.0, 10.0];
        let mut out = [0.0; 4];
        obv_into(&close, &volume, &mut out).unwrap();
        assert_eq!(obv(&close, &volume).unwrap().as_slice().unwrap(), &out);
    }

    #[test]
    fn ad_into_matches_scalar_reference() {
        let high = [10.0, 12.0, 14.0];
        let low = [8.0, 10.0, 12.0];
        let close = [9.0, 11.5, 12.5];
        let volume = [100.0, 120.0, 80.0];
        let mut out = [0.0; 3];
        ad_into(&high, &low, &close, &volume, &mut out).unwrap();
        assert_eq!(out[0], 0.0);
        assert!(out[1] > out[0]);
        assert!(out[2] < out[1]);
    }

    #[test]
    fn adosc_into_warms_up_at_slow_period_minus_one() {
        let high = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low = [8.0, 9.0, 10.0, 11.0, 12.0, 13.0];
        let close = [9.0, 10.5, 11.0, 12.5, 13.0, 14.5];
        let volume = [100.0, 110.0, 120.0, 130.0, 140.0, 150.0];
        let mut out = [0.0; 6];
        adosc_into(&high, &low, &close, &volume, 3, 5, &mut out).unwrap();
        assert_eq!(&out[..4], &[0.0; 4]);
        assert!(out[4].is_finite());
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
