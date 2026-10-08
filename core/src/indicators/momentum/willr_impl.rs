//! Williams %R.

use super::prelude::*;

/// Williams %R (WILLR)
///
/// A momentum indicator that measures overbought/oversold levels.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - Lookback period
///
/// # Returns
/// Array of Williams %R values (-100 to 0 range)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::willr(&high, &low, &close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn willr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    let mut output = Array1::<f64>::zeros(close.len());
    willr_into(high, low, close, period, output.as_slice_mut().unwrap())?;
    Ok(output)
}

/// Caller-owned Williams %R kernel sharing the canonical extrema lifecycle.
pub fn willr_into(
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
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(high.len(), period)?;
    if output.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }

    // The fixed-period block-scan kernel is bit-identical to the generic
    // visitor on NaN-free input, but the generic path also implements the
    // documented warm-up contract (skip a leading missing run), which the
    // block scan does not. Dispatch on cleanliness so both properties hold.
    if period == 14
        && high.iter().all(|value| !value.is_nan())
        && low.iter().all(|value| !value.is_nan())
    {
        return willr14_into(high, low, close, output);
    }

    crate::utils::simd_fill_nan(&mut output[..period - 1]);
    rolling_minmax_visit(high, low, period, |i, highest, lowest| {
        let range = highest - lowest;
        output[i] = if range > crate::utils::TA_IS_ZERO_BANDWIDTH {
            (highest - close[i]) / range * -100.0
        } else {
            0.0
        };
    });
    Ok(())
}

/// Fixed-period WILLR kernel for the benchmark-critical 14-bar path.
///
/// TA-Lib 0.8.x uses a Van Herk/Gil-Werman block scan for the batch API.  A
/// monotonic deque is asymptotically equivalent, but the block scan performs
/// two straight-line extrema passes per block and then combines the two
/// prefix/suffix tables.  That layout is substantially friendlier to the
/// optimizer for a small fixed window and, unlike a callback-based generic
/// visitor, keeps the hot path allocation-free.
#[inline]
pub fn willr14_into(high: &[f64], low: &[f64], close: &[f64], output: &mut [f64]) -> Result<()> {
    const PERIOD: usize = 14;

    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), PERIOD)?;
    if output.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }

    crate::utils::simd_fill_nan(&mut output[..PERIOD - 1]);
    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();
    let close_ptr = close.as_ptr();
    let output_ptr = output.as_mut_ptr();

    // Four small stack tables are enough for the fixed period.  The suffix
    // table belongs to the older block and the prefix table to the next block;
    // combining them produces all windows crossing the block boundary.
    let mut suffix_high = [0.0f64; PERIOD];
    let mut suffix_low = [0.0f64; PERIOD];
    let mut prefix_high = [0.0f64; PERIOD];
    let mut prefix_low = [0.0f64; PERIOD];

    macro_rules! emit {
        ($index:expr, $highest:expr, $lowest:expr) => {{
            let highest = $highest;
            let lowest = $lowest;
            let range = highest - lowest;
            let close_value = *close_ptr.add($index);
            *output_ptr.add($index) = if range > crate::utils::TA_IS_ZERO_BANDWIDTH {
                (highest - close_value) / range * -100.0
            } else {
                0.0
            };
        }};
    }

    unsafe {
        let mut block_start = 0usize;
        let mut today = PERIOD - 1;
        while today < close.len() {
            let block_end = block_start + PERIOD - 1;
            let mut highest = *high_ptr.add(block_end);
            let mut lowest = *low_ptr.add(block_end);
            suffix_high[PERIOD - 1] = highest;
            suffix_low[PERIOD - 1] = lowest;

            let mut offset = PERIOD - 1;
            while offset > 0 {
                offset -= 1;
                let index = block_start + offset;
                let high_value = *high_ptr.add(index);
                let low_value = *low_ptr.add(index);
                if high_value > highest {
                    highest = high_value;
                }
                if low_value < lowest {
                    lowest = low_value;
                }
                suffix_high[offset] = highest;
                suffix_low[offset] = lowest;
            }

            emit!(today, suffix_high[0], suffix_low[0]);

            let block_next = block_start + PERIOD;
            if block_next >= close.len() {
                break;
            }
            let n_available = (close.len() - block_next).min(PERIOD - 1);
            highest = *high_ptr.add(block_next);
            lowest = *low_ptr.add(block_next);
            prefix_high[0] = highest;
            prefix_low[0] = lowest;
            let mut prefix = 1usize;
            while prefix < n_available {
                let index = block_next + prefix;
                let high_value = *high_ptr.add(index);
                let low_value = *low_ptr.add(index);
                if high_value > highest {
                    highest = high_value;
                }
                if low_value < lowest {
                    lowest = low_value;
                }
                prefix_high[prefix] = highest;
                prefix_low[prefix] = lowest;
                prefix += 1;
            }

            let mut offset = 1usize;
            while offset <= n_available {
                let combined_high = if prefix_high[offset - 1] > suffix_high[offset] {
                    prefix_high[offset - 1]
                } else {
                    suffix_high[offset]
                };
                let combined_low = if prefix_low[offset - 1] < suffix_low[offset] {
                    prefix_low[offset - 1]
                } else {
                    suffix_low[offset]
                };
                emit!(today + offset, combined_high, combined_low);
                offset += 1;
            }

            block_start += PERIOD;
            today += n_available + 1;
        }
    }
    Ok(())
}
