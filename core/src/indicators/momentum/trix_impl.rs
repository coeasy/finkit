//! TRIX (triple-smoothed rate of change).

use super::prelude::*;

/// Triple Exponential Average (TRIX)
///
/// A momentum oscillator that calculates a triple smoothed EMA.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
///
/// # Returns
/// Array of TRIX values (percentage change)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close: Vec<f64> = (1..=15).map(|x| x as f64).collect();
/// let result = indicators::trix(&close, 5).unwrap();
/// assert_eq!(result.len(), 15);
/// ```
pub fn trix(input: &[f64], period: usize) -> Result<Array1<f64>> {
    let len = input.len();
    let mut output = init_output(len);
    trix_into(input, period, output.as_slice_mut().unwrap())?;
    Ok(output)
}

/// Compute TRIX directly into a caller-owned buffer.
pub fn trix_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    validate_input(input.len(), period)?;
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }

    let len = input.len();
    crate::utils::simd_fill_nan(output);

    // See `math::leading_warmup`: recurse on the valid tail so a leading warm-up
    // run from an upstream indicator is skipped instead of poisoning the
    // three-stage EMA seed (which reads `input[..period]`).
    let start = crate::math::leading_warmup(input);
    if start > 0 {
        if start + period <= len {
            let (tail_in, tail_out) = (&input[start..], &mut output[start..]);
            trix_into(tail_in, period, tail_out)?;
        }
        return Ok(());
    }

    let s1 = period - 1;
    let s2 = 2 * s1;
    let first_trix = s2 + period;
    let k = smoothing_factor(period);
    let one_k = 1.0 - k;
    let inv_p = 1.0 / period as f64;

    // Reuse output as EMA1. It is fully consumed before each entry is
    // overwritten with the final TRIX value, so no EMA1 allocation is needed.
    let sma1 = input[..period].iter().sum::<f64>() * inv_p;
    output[s1] = sma1;
    let mut e1 = sma1;
    for i in period..len {
        e1 = input[i] * k + e1 * one_k;
        output[i] = e1;
    }

    if s1 + period <= len {
        // EMA2 seed is the SMA of EMA1[s1..s1+period]. Accumulate the EMA3
        // seed in the same pass, keeping the entire pipeline scalar after
        // the first EMA.
        let sma2 = output[s1..s1 + period].iter().sum::<f64>() * inv_p;
        let mut e2 = sma2;
        if first_trix <= len {
            let mut sum3 = e2;
            for i in (s1 + period)..first_trix {
                e2 = output[i] * k + e2 * one_k;
                sum3 += e2;
            }
            crate::utils::simd_fill_nan(&mut output[..first_trix]);
            let mut e3_prev = sum3 * inv_p;
            for i in first_trix..len {
                e2 = output[i] * k + e2 * one_k;
                let e3 = e2 * k + e3_prev * one_k;
                if !crate::utils::is_zero(e3_prev) {
                    output[i] = (e3 - e3_prev) / e3_prev * 100.0;
                }
                e3_prev = e3;
            }
        } else {
            crate::utils::simd_fill_nan(output);
        }
    } else {
        crate::utils::simd_fill_nan(output);
    }
    Ok(())
}
