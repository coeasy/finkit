//! Commodity Channel Index.

use super::prelude::*;

/// Commodity Channel Index (CCI)
///
/// Measures the current price level relative to an average price level over a given period.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - Lookback period
///
/// # Returns
/// Array of CCI values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::cci(&high, &low, &close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
/// Fixed-period CCI kernel for the common TA-Lib period-14 path.
///
/// The generic implementation below remains available for arbitrary periods,
/// while this path keeps the hot ring on the stack and removes dynamic-vector
/// indexing from the million-row benchmark case. The result is written
/// directly into caller-owned storage so formula and FFI paths do not need an
/// intermediate Array1 or copy.
///
/// This used to be `cci_period14_into_impl<const USE_AVX2: bool>` with a
/// `let _ = USE_AVX2;` at the top and an `is_x86_feature_detected!("avx2")`
/// branch at the call site — a vectorized path in name only, since the body was
/// (and still is) the same unrolled scalar code either way. The parameter is
/// gone rather than filled in: vectorizing the mean and the mean deviation
/// means reassociating two `f64` sums of a window whose deviation can approach
/// zero, and `CCI` is compared against TA-Lib's golden series at an *absolute*
/// `1e-8`, where that reassociation is measurable. The measurement agrees — the
/// AVX2 form of these two scans came out 18% slower on the 10,000-bar series
/// *and* moved 13 of 10,000 values past `1e-8`. See V4 plan §43.
#[inline(always)]
pub(crate) fn cci_period14_into_impl(high: &[f64], low: &[f64], close: &[f64], output: &mut [f64]) {
    let len = close.len();
    output[..13].fill(f64::NAN);
    let mut ring = [0.0_f64; 14];
    let mut ring_idx = 0usize;
    for index in 0..13 {
        ring[ring_idx] = typical_price(high[index], low[index], close[index]);
        ring_idx += 1;
    }
    let ring_ptr = ring.as_mut_ptr();

    for index in 13..len {
        let current = typical_price(high[index], low[index], close[index]);
        unsafe {
            *ring_ptr.add(ring_idx) = current;
        }

        // Preserve the canonical TA-Lib operation order. This avoids the
        // long-series drift of a rolling sum while keeping the window small
        // and stack-resident for the common period-14 path.
        let mut mean = 0.0;
        unsafe {
            mean += *ring_ptr.add(0);
            mean += *ring_ptr.add(1);
            mean += *ring_ptr.add(2);
            mean += *ring_ptr.add(3);
            mean += *ring_ptr.add(4);
            mean += *ring_ptr.add(5);
            mean += *ring_ptr.add(6);
            mean += *ring_ptr.add(7);
            mean += *ring_ptr.add(8);
            mean += *ring_ptr.add(9);
            mean += *ring_ptr.add(10);
            mean += *ring_ptr.add(11);
            mean += *ring_ptr.add(12);
            mean += *ring_ptr.add(13);
        }
        mean /= 14.0;
        let mut mean_deviation = 0.0;
        unsafe {
            mean_deviation += (*ring_ptr.add(0) - mean).abs();
            mean_deviation += (*ring_ptr.add(1) - mean).abs();
            mean_deviation += (*ring_ptr.add(2) - mean).abs();
            mean_deviation += (*ring_ptr.add(3) - mean).abs();
            mean_deviation += (*ring_ptr.add(4) - mean).abs();
            mean_deviation += (*ring_ptr.add(5) - mean).abs();
            mean_deviation += (*ring_ptr.add(6) - mean).abs();
            mean_deviation += (*ring_ptr.add(7) - mean).abs();
            mean_deviation += (*ring_ptr.add(8) - mean).abs();
            mean_deviation += (*ring_ptr.add(9) - mean).abs();
            mean_deviation += (*ring_ptr.add(10) - mean).abs();
            mean_deviation += (*ring_ptr.add(11) - mean).abs();
            mean_deviation += (*ring_ptr.add(12) - mean).abs();
            mean_deviation += (*ring_ptr.add(13) - mean).abs();
        }
        let delta = current - mean;
        output[index] = if delta != 0.0 && mean_deviation != 0.0 {
            delta / (0.015 * (mean_deviation / 14.0))
        } else {
            0.0
        };

        ring_idx += 1;
        if ring_idx == 14 {
            ring_idx = 0;
        }
    }
}

#[inline]
pub(crate) fn cci_period14_into(high: &[f64], low: &[f64], close: &[f64], output: &mut [f64]) {
    cci_period14_into_impl(high, low, close, output);
}

pub(crate) fn cci_generic_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    output: &mut [f64],
) {
    let len = close.len();
    // The warm-up prefix and every valid bar are written explicitly below;
    // avoid clearing the full result vector before the CCI scan.
    output[..period - 1].fill(f64::NAN);
    let output_ptr = output.as_mut_ptr();
    let inv_p = 1.0 / period as f64;

    // Keep the raw window in a ring. For the small periods used by TA-Lib's
    // public CCI contract, scanning the window in input order is materially
    // cheaper than maintaining a sorted vector plus prefix sums (every bar
    // otherwise performs two shifts and two binary searches). It also keeps
    // the mean-deviation accumulation order aligned with TA-Lib.
    let mut ring: Vec<f64> = vec![0.0; period];
    let mut tp_sum = 0.0;
    for j in 0..period {
        let tp = typical_price(high[j], low[j], close[j]);
        ring[j] = tp;
        tp_sum += tp;
    }

    let first = period - 1;
    {
        let tp_mean = tp_sum * inv_p;
        let mut mean_dev = 0.0;
        for &value in &ring {
            mean_dev += (value - tp_mean).abs();
        }
        mean_dev *= inv_p;
        unsafe {
            // TA-Lib's `TA_CCI` writes `0.0` when the deviation is zero, and the
            // period-14 path above has always done the same. This generic path
            // used to write `NaN`, so one indicator had two answers depending
            // only on the period it was called with; the contract is now one.
            *output_ptr.add(first) = if !crate::utils::is_zero(mean_dev) {
                (ring[period - 1] - tp_mean) / (0.015 * mean_dev)
            } else {
                0.0
            };
        }
    }

    let mut ring_idx = 0;
    for i in period..len {
        let new_tp = typical_price(high[i], low[i], close[i]);
        let old_tp = ring[ring_idx];
        tp_sum += new_tp - old_tp;

        ring[ring_idx] = new_tp;
        ring_idx = (ring_idx + 1) % period;

        let tp_mean = tp_sum * inv_p;
        let mut mean_dev = 0.0;
        for &value in &ring {
            mean_dev += (value - tp_mean).abs();
        }
        mean_dev *= inv_p;
        unsafe {
            // See the first-window branch above: `0.0`, matching `TA_CCI` and
            // the period-14 path, instead of the `NaN` this branch used to
            // emit for the same flat window.
            *output_ptr.add(i) = if !crate::utils::is_zero(mean_dev) {
                (new_tp - tp_mean) / (0.015 * mean_dev)
            } else {
                0.0
            };
        }
    }
}

/// Commodity Channel Index over an arbitrary source series.
///
/// This is the two-operand `CCI(source, period)` contract lifted out of the
/// formula engine so the tree-walking executor and the compiled-plan kernel
/// share one implementation. The four-operand HLC form is the same computation
/// applied to the typical price `(high + low + close) / 3`, which the caller
/// builds.
///
/// `output` is fully written: NaN where the contract has no value yet.
///
/// # Errors
///
/// Returns `TaError::InvalidParameter` if `source` is shorter than `output`, or
/// if `period` is zero.
#[allow(clippy::cast_precision_loss)] // periods are far below 2^53
pub fn cci_source_into(source: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    let len = output.len();
    if source.len() < len {
        return Err(TaError::InvalidParameter {
            name: "source".to_string(),
            constraint: "must have at least the output length".to_string(),
        });
    }
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    output.fill(f64::NAN);

    for i in (period - 1)..len {
        let window_start = i + 1 - period;
        let mean = (window_start..=i).map(|j| source[j]).sum::<f64>() / period as f64;
        let mean_dev = (window_start..=i)
            .map(|j| (source[j] - mean).abs())
            .sum::<f64>()
            / period as f64;
        if mean_dev > crate::utils::TA_IS_ZERO_BANDWIDTH {
            output[i] = (source[i] - mean) / (0.015 * mean_dev);
        }
    }
    Ok(())
}

/// Commodity Channel Index (CCI).
#[expect(clippy::uninit_vec)]
pub fn cci(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period)?;

    let mut output = Vec::with_capacity(close.len());
    unsafe { output.set_len(close.len()) };
    if period == 14 {
        cci_period14_into(high, low, close, &mut output);
    } else {
        cci_generic_into(high, low, close, period, &mut output);
    }
    Ok(Array1::from_vec(output))
}

/// CCI zero-copy variant: writes result into pre-allocated slice.
pub fn cci_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<()> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if output.len() != high.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    validate_input(high.len(), period)?;

    if period == 14 {
        cci_period14_into(high, low, close, output);
    } else {
        cci_generic_into(high, low, close, period, output);
    }
    Ok(())
}
