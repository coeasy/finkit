//! Native-precision moving-average kernels for Architecture 3.0.
//!
//! The established public indicator surface remains f64-first. These kernels provide
//! the typed hot path required by FFI callers so an existing float32 NumPy buffer does
//! not need to be promoted to float64 merely to compute SMA/EMA.

use crate::error::{Result, TaError};

#[inline]
fn validate_f32(input: &[f32], period: usize, output: &[f32]) -> Result<()> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    if input.len() < period {
        return Err(TaError::InvalidParameter {
            name: "input".to_string(),
            constraint: format!("length must be at least period ({period})"),
        });
    }
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    if let Some(index) = input.iter().position(|value| !value.is_finite()) {
        return Err(TaError::InvalidParameter {
            name: "input".to_string(),
            constraint: format!("non-finite value at index {index}"),
        });
    }
    Ok(())
}

/// Compute an f32 SMA directly into caller-owned memory.
#[inline]
pub fn sma_f32_into(input: &[f32], period: usize, output: &mut [f32]) -> Result<()> {
    validate_f32(input, period, output)?;
    output[..period - 1].fill(f32::NAN);

    let inv_period = 1.0f32 / period as f32;
    let mut sum = 0.0f32;
    for &value in &input[..period] {
        sum += value;
    }
    output[period - 1] = sum * inv_period;

    for index in period..input.len() {
        sum += input[index] - input[index - period];
        output[index] = sum * inv_period;
    }
    Ok(())
}

/// Compute an f32 EMA using the same SMA-seed contract as `moving_avg::ema`.
#[inline]
pub fn ema_f32_into(input: &[f32], period: usize, output: &mut [f32]) -> Result<()> {
    validate_f32(input, period, output)?;
    output[..period - 1].fill(f32::NAN);

    let mut initial_sum = 0.0f32;
    for &value in &input[..period] {
        initial_sum += value;
    }
    let initial = initial_sum / period as f32;
    output[period - 1] = initial;

    let smoothing = 2.0f32 / (period as f32 + 1.0);
    let mut previous = initial;
    for index in period..input.len() {
        previous = (input[index] - previous).mul_add(smoothing, previous);
        output[index] = previous;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::moving_avg;

    #[test]
    fn f32_sma_tracks_f64_contract() {
        let input: Vec<f32> = (1..=64).map(|value| value as f32).collect();
        let input_f64: Vec<f64> = input.iter().map(|&value| value as f64).collect();
        let mut output = vec![0.0f32; input.len()];
        sma_f32_into(&input, 7, &mut output).unwrap();
        let expected = moving_avg::sma(&input_f64, 7).unwrap();

        for (actual, expected) in output.iter().zip(expected.iter()) {
            if expected.is_nan() {
                assert!(actual.is_nan());
            } else {
                assert!((*actual as f64 - expected).abs() < 1e-5);
            }
        }
    }

    #[test]
    fn f32_ema_tracks_f64_contract() {
        let input: Vec<f32> = (0..128)
            .map(|index| 100.0 + (index as f32 * 0.17).sin())
            .collect();
        let input_f64: Vec<f64> = input.iter().map(|&value| value as f64).collect();
        let mut output = vec![0.0f32; input.len()];
        ema_f32_into(&input, 14, &mut output).unwrap();
        let expected = moving_avg::ema(&input_f64, 14).unwrap();

        for (actual, expected) in output.iter().zip(expected.iter()) {
            if expected.is_nan() {
                assert!(actual.is_nan());
            } else {
                assert!((*actual as f64 - expected).abs() < 2e-4);
            }
        }
    }
}
