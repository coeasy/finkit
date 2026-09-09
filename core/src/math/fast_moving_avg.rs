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
fn reject_if_non_finite(input: &[f64]) -> Result<()> {
    if let Some(index) = input.iter().position(|value| !value.is_finite()) {
        return Err(TaError::InvalidParameter {
            name: "input".to_string(),
            constraint: format!("non-finite value at index {index}"),
        });
    }
    Ok(())
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

    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    if is_x86_feature_detected!("avx2") && input.len() >= period.saturating_add(8) {
        // SAFETY: the runtime check above enables the target feature and the
        // validation guarantees matching, in-bounds slices.
        unsafe { wma_into_avx2(input, period, output) };
        return Ok(());
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

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn wma_into_avx2(input: &[f64], period: usize, output: &mut [f64]) {
    unsafe {
        crate::utils::simd_fill_nan(&mut output[..period - 1]);
        let inv_weight_sum = 1.0 / (period * (period + 1) / 2) as f64;
        let period_f = period as f64;
        let input_ptr = input.as_ptr();
        let mut window_sum = 0.0;
        let mut weighted_sum = 0.0;
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
}

/// Kaufman's Adaptive Moving Average with the established Finkit/TA-Lib
/// arithmetic order. The hot loop intentionally follows TA-Lib's numeric
/// contract and does not carry a per-row input-validation branch.
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
    reject_if_non_finite(input)?;

    let len = input.len();
    let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe {
        raw_output.set_len(len);
        if period == 20 && fast_period == 2 && slow_period == 30 {
            dispatch_kama_kernel::<20, 2, 30>(
                input,
                period,
                fast_period,
                slow_period,
                raw_output.as_mut_ptr().cast::<f64>(),
            );
        } else {
            dispatch_kama_kernel::<0, 0, 0>(
                input,
                period,
                fast_period,
                slow_period,
                raw_output.as_mut_ptr().cast::<f64>(),
            );
        }
        let ptr = raw_output.as_mut_ptr().cast::<f64>();
        let capacity = raw_output.capacity();
        let length = raw_output.len();
        forget(raw_output);
        Ok(Array1::from_vec(Vec::from_raw_parts(ptr, length, capacity)))
    }
}

/// Caller-owned KAMA adapter for the unified Formula/Batch executor.
///
pub fn kama_into(
    input: &[f64],
    period: usize,
    fast_period: usize,
    slow_period: usize,
    output: &mut [f64],
) -> Result<()> {
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    if period == 0 || fast_period == 0 || slow_period == 0 {
        return Err(invalid_period());
    }
    validate_input(input.len(), period + 1)?;
    reject_if_non_finite(input)?;
    unsafe {
        if period == 20 && fast_period == 2 && slow_period == 30 {
            dispatch_kama_kernel::<20, 2, 30>(
                input,
                period,
                fast_period,
                slow_period,
                output.as_mut_ptr(),
            )
        } else {
            dispatch_kama_kernel::<0, 0, 0>(
                input,
                period,
                fast_period,
                slow_period,
                output.as_mut_ptr(),
            )
        }
    };
    Ok(())
}

#[inline(always)]
unsafe fn dispatch_kama_kernel<
    const KAMA_PERIOD: usize,
    const KAMA_FAST: usize,
    const KAMA_SLOW: usize,
>(
    input: &[f64],
    period: usize,
    fast_period: usize,
    slow_period: usize,
    output_ptr: *mut f64,
) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    if is_x86_feature_detected!("fma") {
        return unsafe {
            kama_kernel_fma::<KAMA_PERIOD, KAMA_FAST, KAMA_SLOW>(
                input,
                period,
                fast_period,
                slow_period,
                output_ptr,
            )
        };
    }
    unsafe {
        kama_kernel::<KAMA_PERIOD, KAMA_FAST, KAMA_SLOW, false>(
            input,
            period,
            fast_period,
            slow_period,
            output_ptr,
        );
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "fma")]
unsafe fn kama_kernel_fma<
    const KAMA_PERIOD: usize,
    const KAMA_FAST: usize,
    const KAMA_SLOW: usize,
>(
    input: &[f64],
    period: usize,
    fast_period: usize,
    slow_period: usize,
    output_ptr: *mut f64,
) {
    unsafe {
        kama_kernel::<KAMA_PERIOD, KAMA_FAST, KAMA_SLOW, true>(
            input,
            period,
            fast_period,
            slow_period,
            output_ptr,
        );
    }
}

#[inline(always)]
unsafe fn kama_kernel<
    const KAMA_PERIOD: usize,
    const KAMA_FAST: usize,
    const KAMA_SLOW: usize,
    const USE_FMA: bool,
>(
    input: &[f64],
    period: usize,
    fast_period: usize,
    slow_period: usize,
    output_ptr: *mut f64,
) {
    unsafe {
        let period = if KAMA_PERIOD == 0 {
            period
        } else {
            KAMA_PERIOD
        };
        let fast_period = if KAMA_FAST == 0 {
            fast_period
        } else {
            KAMA_FAST
        };
        let slow_period = if KAMA_SLOW == 0 {
            slow_period
        } else {
            KAMA_SLOW
        };
        let len = input.len();
        let input_ptr = input.as_ptr();
        let first_value = *input_ptr;

        for index in 0..period {
            output_ptr.add(index).write(f64::NAN);
        }

        let fast_sc = 2.0 / (fast_period as f64 + 1.0);
        let slow_sc = 2.0 / (slow_period as f64 + 1.0);
        let sc_diff = fast_sc - slow_sc;
        let mut volatility = 0.0;
        let mut previous_input = first_value;
        for index in 1..=period {
            let current = *input_ptr.add(index);
            volatility += (current - previous_input).abs();
            previous_input = current;
        }

        let seed = *input_ptr.add(period - 1);
        let period_change = *input_ptr.add(period) - *input_ptr;
        let efficiency = if volatility <= 0.0 || volatility <= period_change {
            1.0
        } else {
            (period_change / volatility).abs().min(1.0)
        };
        let smoothing = if USE_FMA {
            efficiency.mul_add(sc_diff, slow_sc)
        } else {
            efficiency * sc_diff + slow_sc
        };
        let smoothing = smoothing * smoothing;
        let period_value = *input_ptr.add(period);
        let mut previous_kama = if USE_FMA {
            smoothing.mul_add(period_value - seed, seed)
        } else {
            seed + smoothing * (period_value - seed)
        };
        output_ptr.add(period).write(previous_kama);
        let mut previous_input = period_value;
        let mut previous_outgoing = *input_ptr;

        let mut current_ptr = input_ptr.add(period + 1);
        let mut outgoing_ptr = input_ptr.add(1);
        let mut output_cursor = output_ptr.add(period + 1);
        let end_ptr = input_ptr.add(len);
        while current_ptr < end_ptr {
            let current = *current_ptr;
            let outgoing_current = *outgoing_ptr;
            volatility +=
                (current - previous_input).abs() - (outgoing_current - previous_outgoing).abs();
            previous_input = current;
            previous_outgoing = outgoing_current;

            let period_change = current - outgoing_current;
            let efficiency = if volatility <= 0.0 || volatility <= period_change {
                1.0
            } else {
                (period_change / volatility).abs().min(1.0)
            };
            let smoothing = if USE_FMA {
                efficiency.mul_add(sc_diff, slow_sc)
            } else {
                efficiency * sc_diff + slow_sc
            };
            let smoothing = smoothing * smoothing;
            previous_kama = if USE_FMA {
                smoothing.mul_add(current - previous_kama, previous_kama)
            } else {
                previous_kama + smoothing * (current - previous_kama)
            };
            output_cursor.write(previous_kama);
            current_ptr = current_ptr.add(1);
            outgoing_ptr = outgoing_ptr.add(1);
            output_cursor = output_cursor.add(1);
        }
    }
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
    fn kama_writes_the_complete_talib_warmup_prefix() {
        let input: Vec<f64> = (0..32).map(|value| value as f64 + 1.0).collect();
        let output = kama(&input, 20, 2, 30).unwrap();
        assert!(output.iter().take(20).all(|value| value.is_nan()));
        assert!(output.iter().skip(20).all(|value| value.is_finite()));
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
