//! Relative Strength Index (Wilder smoothing).

use super::prelude::*;

/// Relative Strength Index (RSI)
///
/// Measures the magnitude of recent price changes to evaluate overbought/oversold conditions.
///
/// # Arguments
/// * `input` - Input data series (typically close prices)
/// * `period` - Lookback period (default: 14)
///
/// # Returns
/// Array of RSI values (0-100 range)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::rsi(&close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(period, len = input.len())))]
pub fn rsi(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    // Only a non-finite value *after* the series has started is bad input; a
    // leading run is the warm-up prefix of an upstream rolling indicator, which
    // `rsi_simd_into` already skips. Rejecting it here made `RSI(MA(CLOSE, 5), 9)`
    // all-NaN on this path while the compiled plan path -- which calls `rsi_into`
    // and has no such check -- returned values.
    #[cfg(feature = "metrics")]
    {
        let started = crate::math::leading_warmup(input);
        if let Some(idx) = input[started..]
            .iter()
            .position(|v| !v.is_finite())
            .map(|offset| offset + started)
        {
            crate::metrics::input_rejected("rsi", "non_finite");
            return Err(TaError::InvalidParameter {
                name: "input".to_string(),
                constraint: format!("non-finite value at index {idx}"),
            });
        }
    }
    validate_input(input.len(), period + 1)?;

    #[cfg(feature = "metrics")]
    {
        crate::metrics::indicator_called("rsi");
        let start = std::time::Instant::now();
        let result = rsi_inner(input, period);
        crate::metrics::record_indicator_duration("rsi", start.elapsed().as_secs_f64());
        return result;
    }
    #[cfg(not(feature = "metrics"))]
    rsi_inner(input, period)
}

#[inline]
// Every slot is written by `rsi_simd_into` before it is read; the `expect` is
// deliberate (CI denies unfulfilled lint expectations, so a stale suppression
// fails the build rather than silently rotting).
#[expect(clippy::uninit_vec)]
pub(crate) fn rsi_inner(input: &[f64], period: usize) -> Result<Array1<f64>> {
    let len = input.len();
    // Every dispatch path inside `rsi_simd_into` (AVX-512 → AVX-2 → scalar)
    // writes the warm-up NaN slots itself, so the old full-length
    // `init_output` fill was a redundant memory pass over the whole output —
    // a measurable share of the runtime on million-bar series.
    let mut output = Vec::with_capacity(len);
    // SAFETY: `rsi_simd_into` below writes every slot of the `len`-slot buffer
    // (warm-up NaNs and values alike) before any slot is read.
    unsafe { output.set_len(len) };
    crate::math::simd_kernels::rsi_simd_into(input, period, output.as_mut_slice());
    Ok(Array1::from_vec(output))
}

/// Compute RSI writing results into a pre-allocated buffer.
///
/// `output` must have the same length as `input`. Warm-up values are written as NaN.
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let mut output = vec![0.0; close.len()];
/// indicators::rsi_into(&close, 5, &mut output).unwrap();
/// assert_eq!(output.len(), 10);
/// ```
pub fn rsi_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period + 1)?;
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }

    crate::math::simd_kernels::rsi_simd_into(input, period, output);

    Ok(())
}
