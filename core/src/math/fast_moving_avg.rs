//! Architecture v3 hot moving-average kernels.
//!
//! These functions preserve the arithmetic order of the established public
//! implementations while removing avoidable bounds checks / full-input
//! validation passes from the installed-wheel hot path.

use crate::error::{Result, TaError};
use crate::utils::{smoothing_factor, validate_input};
use ndarray::Array1;
use std::mem::{forget, MaybeUninit};

#[inline]
fn invalid_period() -> TaError {
    TaError::InvalidParameter {
        name: "period".to_string(),
        constraint: "greater than 0".to_string(),
    }
}

#[inline]
fn non_finite(index: usize) -> TaError {
    TaError::InvalidParameter {
        name: "input".to_string(),
        constraint: format!("non-finite value at index {index}"),
    }
}

/// EMA into a caller-owned output buffer using the same SMA seed and FMA
/// recurrence as the legacy implementation, but with a bounds-check-free hot
/// loop.
pub fn ema_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    if period == 0 {
        return Err(invalid_period());
    }
    validate_input(input.len(), period)?;
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }

    crate::utils::simd_fill_nan(&mut output[..period - 1]);
    let initial_sma =
        super::moving_avg_legacy::simd_horizontal_sum(&input[..period]) / period as f64;
    output[period - 1] = initial_sma;

    let len = input.len();
    let k = smoothing_factor(period);
    let mut previous = initial_sma;
    unsafe {
        let input_ptr = input.as_ptr();
        let output_ptr = output.as_mut_ptr();
        for index in period..len {
            let value = *input_ptr.add(index);
            previous = (value - previous).mul_add(k, previous);
            *output_ptr.add(index) = previous;
        }
    }
    Ok(())
}

/// WMA into a caller-owned output buffer using the O(n) TA-Lib-style rolling
/// weighted-sum recurrence without bounds checks in the main loop.
pub fn wma_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    if period == 0 {
        return Err(invalid_period());
    }
    validate_input(input.len(), period)?;
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }

    crate::utils::simd_fill_nan(&mut output[..period - 1]);
    let denominator = (period * (period + 1) / 2) as f64;
    let inv_weight_sum = 1.0 / denominator;
    let period_f = period as f64;

    let mut window_sum = 0.0;
    let mut weighted_sum = 0.0;
    unsafe {
        let input_ptr = input.as_ptr();
        for index in 0..period {
            let value = *input_ptr.add(index);
            window_sum += value;
            weighted_sum += (index + 1) as f64 * value;
        }

        let output_ptr = output.as_mut_ptr();
        *output_ptr.add(period - 1) = weighted_sum * inv_weight_sum;
        for index in period..input.len() {
            let old = *input_ptr.add(index - period);
            let new = *input_ptr.add(index);
            weighted_sum += period_f * new - window_sum;
            window_sum += new - old;
            *output_ptr.add(index) = weighted_sum * inv_weight_sum;
        }
    }
    Ok(())
}

/// Kaufman's Adaptive Moving Average with the established Finkit/TA-Lib
/// arithmetic order. Non-finite rejection is fused into the computation so a
/// successful call no longer performs an extra O(n) validation pass first.
pub fn kama(
    input: &[f64],
    period: usize,
    fast_period: usize,
    slow_period: usize,
) -> Result<Array1<f64>> {
    if period == 0 || fast_period == 0 || slow_period == 0 {
        return Err(invalid_period());
    }
    // KAMA reads input[period] to produce the first recursive value.
    validate_input(input.len(), period + 1)?;

    let len = input.len();
    let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(len);
    let fast_sc = 2.0 / (fast_period as f64 + 1.0);
    let slow_sc = 2.0 / (slow_period as f64 + 1.0);
    let sc_diff = fast_sc - slow_sc;

    let output = unsafe {
        raw_output.set_len(len);
        let input_ptr = input.as_ptr();
        let output_ptr = raw_output.as_mut_ptr();
        let first_value = *input_ptr;
        if !first_value.is_finite() {
            return Err(non_finite(0));
        }

        for index in 0..period - 1 {
            output_ptr.add(index).write(MaybeUninit::new(f64::NAN));
        }

        let mut volatility = 0.0;
        let mut previous_input = first_value;
        for index in 1..=period {
            let current = *input_ptr.add(index);
            if !current.is_finite() {
                return Err(non_finite(index));
            }
            volatility += (current - previous_input).abs();
            previous_input = current;
        }

        let seed = *input_ptr.add(period - 1);
        output_ptr.add(period - 1).write(MaybeUninit::new(seed));
        let direction = (*input_ptr.add(period) - *input_ptr).abs();
        let efficiency = if volatility != 0.0 {
            direction / volatility
        } else {
            0.0
        };
        let smoothing = efficiency * sc_diff + slow_sc;
        let smoothing = smoothing * smoothing;
        let period_value = *input_ptr.add(period);
        let mut previous_kama = seed + smoothing * (period_value - seed);
        output_ptr
            .add(period)
            .write(MaybeUninit::new(previous_kama));

        for index in period + 1..len {
            let current = *input_ptr.add(index);
            if !current.is_finite() {
                return Err(non_finite(index));
            }
            let previous = *input_ptr.add(index - 1);
            let outgoing_current = *input_ptr.add(index - period);
            let outgoing_previous = *input_ptr.add(index - period - 1);
            volatility += (current - previous).abs() - (outgoing_current - outgoing_previous).abs();

            let direction = (current - outgoing_current).abs();
            let efficiency = if volatility != 0.0 {
                direction / volatility
            } else {
                0.0
            };
            let smoothing = efficiency * sc_diff + slow_sc;
            let smoothing = smoothing * smoothing;
            previous_kama += smoothing * (current - previous_kama);
            output_ptr.add(index).write(MaybeUninit::new(previous_kama));
        }

        let ptr = raw_output.as_mut_ptr().cast::<f64>();
        let capacity = raw_output.capacity();
        let length = raw_output.len();
        forget(raw_output);
        Vec::from_raw_parts(ptr, length, capacity)
    };

    Ok(Array1::from_vec(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kama_requires_period_plus_one_values() {
        let input = vec![1.0; 14];
        assert!(kama(&input, 14, 2, 30).is_err());
    }

    #[test]
    fn wma_into_has_expected_warmup() {
        let input = [1.0, 2.0, 3.0, 4.0, 5.0];
        let mut output = [0.0; 5];
        wma_into(&input, 3, &mut output).unwrap();
        assert!(output[0].is_nan());
        assert!(output[1].is_nan());
        assert!((output[2] - (14.0 / 6.0)).abs() < 1e-15);
    }
}
