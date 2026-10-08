//! Stochastic oscillator, full and fast variants.

use super::prelude::*;

/// Stochastic Oscillator (STOCH) Result
#[derive(Debug, Clone)]
pub struct StochResult {
    /// %K line (fast)
    pub k: Array1<f64>,
    /// %D line (slow, SMA of %K)
    pub d: Array1<f64>,
}

/// Stochastic Oscillator (STOCH)
///
/// Compares a security's closing price to its price range over a given period.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `k_period` - %K lookback period
/// * `k_slow` - %K slowing period
/// * `d_period` - %D period
///
/// # Returns
/// StochResult containing %K and %D values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::stoch(&high, &low, &close, 5, 3, 3).unwrap();
/// assert_eq!(result.k.len(), 10);
/// ```
pub fn stoch(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    k_period: usize,
    k_slow: usize,
    d_period: usize,
) -> Result<StochResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), k_period)?;

    let len = close.len();
    let mut k_out = vec![0.0_f64; len];
    let mut d_out = vec![0.0_f64; len];

    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    crate::math::simd_kernels::stoch_simd_into(
        high, low, close, k_period, k_slow, d_period, &mut k_out, &mut d_out,
    );
    #[cfg(not(all(feature = "std", target_arch = "x86_64")))]
    stoch_fused_pipeline(
        high, low, close, k_period, k_slow, d_period, &mut k_out, &mut d_out,
    );

    Ok(StochResult {
        k: Array1::from(k_out),
        d: Array1::from(d_out),
    })
}

/// TA-Lib stochastic oscillator with explicit slow-K and slow-D MA types.
///
/// The existing [`stoch`] function remains the optimized SMA/SMA convenience
/// path. This entry point follows TA-Lib's parameter order and applies the
/// selected moving-average kernels to the fast-K series.
#[allow(clippy::too_many_arguments)]
pub fn stoch_with_ma_types(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    fastk_period: usize,
    slowk_period: usize,
    slowk_ma_type: MaType,
    slowd_period: usize,
    slowd_ma_type: MaType,
) -> Result<StochResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if slowk_period == 0 || slowd_period == 0 {
        return Err(TaError::InvalidParameter {
            name: "slowk_period, slowd_period".to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    validate_input(high.len(), fastk_period)?;

    let mut fastk = vec![f64::NAN; close.len()];
    rolling_minmax_visit(high, low, fastk_period, |i, highest, lowest| {
        let range = highest - lowest;
        fastk[i] = if range > crate::utils::TA_IS_ZERO_BANDWIDTH {
            (close[i] - lowest) / range * 100.0
        } else {
            50.0
        };
    });

    let fastk_start = fastk_period - 1;
    let slowk_values =
        crate::indicators::overlap::ma(&fastk[fastk_start..], slowk_period, slowk_ma_type)?;
    let slowk_slice = slowk_values.as_slice().unwrap();
    let mut slowk = vec![f64::NAN; close.len()];
    for (offset, value) in slowk_slice.iter().enumerate() {
        slowk[fastk_start + offset] = *value;
    }

    let slowk_start = slowk_period - 1;
    let slowd_values =
        crate::indicators::overlap::ma(&slowk_slice[slowk_start..], slowd_period, slowd_ma_type)?;
    let slowd_slice = slowd_values.as_slice().unwrap();
    let slowd_base = fastk_start + slowk_start;
    let mut slowd = vec![f64::NAN; close.len()];
    for (offset, value) in slowd_slice.iter().enumerate() {
        slowd[slowd_base + offset] = *value;
    }

    // TA-Lib exposes SlowK only once SlowD's lookback has elapsed.
    let output_start = slowd_base + slowd_period - 1;
    for i in fastk_start..output_start.min(close.len()) {
        slowk[i] = f64::NAN;
    }
    Ok(StochResult {
        k: Array1::from(slowk),
        d: Array1::from(slowd),
    })
}

/// Single-pass fused pipeline: computes fast %K via incremental max/min tracking, then applies
/// WMA for %K (slow) and %D simultaneously without intermediate allocation.
/// Matches TA-Lib C behavior where NaN fast_k values are treated as 0.
/// TA-Lib uses WMA (Weighted Moving Average) instead of SMA for smoothing.
#[inline]
#[allow(clippy::too_many_arguments)]
#[allow(dead_code)] // used only on non-(std+x86_64) builds; SIMD path covers std+x86_64
pub(crate) fn stoch_fused_pipeline(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    k_period: usize,
    k_slow: usize,
    d_period: usize,
    k_out: &mut [f64],
    d_out: &mut [f64],
) {
    let len = close.len();
    let fastk_start = k_period - 1;
    // TA-Lib: slow_k first valid at (k_period - 1) + (k_slow - 1)
    let slowk_start = fastk_start + k_slow - 1;
    // TA-Lib: %D first valid at slowk_start + (d_period - 1)
    let slowd_start = slowk_start + d_period - 1;

    for value in k_out.iter_mut().take(slowd_start.min(len)) {
        *value = f64::NAN;
    }
    for value in d_out.iter_mut().take(slowd_start.min(len)) {
        *value = f64::NAN;
    }

    let inv_k_slow = 1.0 / k_slow as f64;
    let inv_d_period = 1.0 / d_period as f64;

    let mut fast_k_ring = vec![0.0_f64; k_slow];
    let mut k_ring = vec![0.0_f64; d_period];
    let mut fk_ring_pos: usize = 0;
    let mut d_ring_pos: usize = 0;

    let mut k_sum: f64 = 0.0;
    let mut d_sum: f64 = 0.0;

    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();
    let close_ptr = close.as_ptr();

    let mut highest_idx: usize = 0;
    let mut lowest_idx: usize = 0;
    let mut highest: f64 = f64::NEG_INFINITY;
    let mut lowest: f64 = f64::INFINITY;

    for i in 0..len {
        unsafe {
            let new_h = *high_ptr.add(i);
            let new_l = *low_ptr.add(i);

            if i < k_period {
                if new_h >= highest {
                    highest = new_h;
                    highest_idx = i;
                }
                if new_l <= lowest {
                    lowest = new_l;
                    lowest_idx = i;
                }
            } else {
                let ws = i + 1 - k_period;
                if highest_idx < ws {
                    highest = *high_ptr.add(ws);
                    highest_idx = ws;
                    let mut k = ws + 1;
                    while k <= i {
                        let h = *high_ptr.add(k);
                        if h >= highest {
                            highest = h;
                            highest_idx = k;
                        }
                        k += 1;
                    }
                } else if new_h >= highest {
                    highest = new_h;
                    highest_idx = i;
                }

                if lowest_idx < ws {
                    lowest = *low_ptr.add(ws);
                    lowest_idx = ws;
                    let mut k = ws + 1;
                    while k <= i {
                        let l = *low_ptr.add(k);
                        if l <= lowest {
                            lowest = l;
                            lowest_idx = k;
                        }
                        k += 1;
                    }
                } else if new_l <= lowest {
                    lowest = new_l;
                    lowest_idx = i;
                }
            }

            // Only compute fast_k and accumulate after warm-up
            if i >= fastk_start {
                let denom = highest - lowest;
                let fk = if denom > crate::utils::TA_IS_ZERO_BANDWIDTH {
                    (*close_ptr.add(i) - lowest) / denom * 100.0
                } else {
                    50.0
                };

                // SMA update for slow %K
                let old_fk = *fast_k_ring.get_unchecked(fk_ring_pos);
                k_sum += fk - old_fk;
                *fast_k_ring.get_unchecked_mut(fk_ring_pos) = fk;
                fk_ring_pos += 1;
                if fk_ring_pos == k_slow {
                    fk_ring_pos = 0;
                }

                // Compute slow %K value
                let k_val = k_sum * inv_k_slow;

                // SMA update for %D
                let old_k = *k_ring.get_unchecked(d_ring_pos);
                d_sum += k_val - old_k;
                *k_ring.get_unchecked_mut(d_ring_pos) = k_val;
                d_ring_pos += 1;
                if d_ring_pos == d_period {
                    d_ring_pos = 0;
                }

                // TA-Lib special rule: both Slow %K and %D start from slowd_start
                if i >= slowd_start {
                    *k_out.get_unchecked_mut(i) = k_val;
                    *d_out.get_unchecked_mut(i) = d_sum * inv_d_period;
                }
            }
        }
    }
}

/// Compute Stochastic Oscillator writing results into pre-allocated buffers.
///
/// `k_out` and `d_out` must have the same length as `close`. Warm-up values are NaN.
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let mut k_out = vec![0.0; close.len()];
/// let mut d_out = vec![0.0; close.len()];
/// indicators::stoch_into(&high, &low, &close, 5, 3, 3, &mut k_out, &mut d_out).unwrap();
/// assert_eq!(k_out.len(), 10);
/// ```
#[allow(clippy::too_many_arguments)]
pub fn stoch_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    k_period: usize,
    k_slow: usize,
    d_period: usize,
    k_out: &mut [f64],
    d_out: &mut [f64],
) -> Result<()> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), k_period)?;
    if k_out.len() != close.len() || d_out.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "k_out/d_out".to_string(),
            constraint: "must have the same length as close".to_string(),
        });
    }

    if k_period == 5 && k_slow == 3 && d_period == 3 {
        stoch_default_5_3_3_into(high, low, close, k_out, d_out);
        return Ok(());
    }

    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    crate::math::simd_kernels::stoch_simd_into(
        high, low, close, k_period, k_slow, d_period, k_out, d_out,
    );
    #[cfg(not(all(feature = "std", target_arch = "x86_64")))]
    stoch_fused_pipeline(high, low, close, k_period, k_slow, d_period, k_out, d_out);

    Ok(())
}

/// Fixed-period STOCH kernel for the TA-Lib default configuration. Keeping
/// the two monotonic queues and both smoothing rings on the stack removes the
/// small dynamic-indexing costs from the generic SIMD dispatcher.
#[inline(always)]
pub(crate) fn stoch_default_5_3_3_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    k_out: &mut [f64],
    d_out: &mut [f64],
) {
    const LOOKBACK: usize = 8;
    let warmup = LOOKBACK.min(k_out.len());
    k_out[..warmup].fill(f64::NAN);
    d_out[..warmup].fill(f64::NAN);

    let mut fast_k_prev1 = 0.0;
    let mut fast_k_prev2 = 0.0;
    let mut slow_k_prev1 = 0.0;
    let mut slow_k_prev2 = 0.0;

    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();
    let close_ptr = close.as_ptr();
    let k_out_ptr = k_out.as_mut_ptr();
    let d_out_ptr = d_out.as_mut_ptr();
    let len = close.len();

    // The public default is a fixed five-bar window.  Scanning those five
    // values directly is cheaper than maintaining two monotonic queues for
    // this hot path, while retaining the same smoothing state and warm-up
    // contract as TA-Lib.
    unsafe {
        let mut high_cursor = high_ptr;
        let mut low_cursor = low_ptr;
        let mut close_cursor = close_ptr;
        let mut k_out_cursor = k_out_ptr;
        let mut d_out_cursor = d_out_ptr;
        for i in 0..len {
            let new_high = *high_cursor;
            let new_low = *low_cursor;
            let fast_k = if i >= 4 {
                let mut highest = *high_ptr.add(i - 4);
                let mut lowest = *low_ptr.add(i - 4);
                let high_1 = *high_ptr.add(i - 3);
                let low_1 = *low_ptr.add(i - 3);
                let high_2 = *high_ptr.add(i - 2);
                let low_2 = *low_ptr.add(i - 2);
                let high_3 = *high_ptr.add(i - 1);
                let low_3 = *low_ptr.add(i - 1);
                if high_1 > highest {
                    highest = high_1;
                }
                if high_2 > highest {
                    highest = high_2;
                }
                if high_3 > highest {
                    highest = high_3;
                }
                if new_high > highest {
                    highest = new_high;
                }
                if low_1 < lowest {
                    lowest = low_1;
                }
                if low_2 < lowest {
                    lowest = low_2;
                }
                if low_3 < lowest {
                    lowest = low_3;
                }
                if new_low < lowest {
                    lowest = new_low;
                }
                let denom = highest - lowest;
                if denom > crate::utils::TA_IS_ZERO_BANDWIDTH {
                    (*close_cursor - lowest) / denom * 100.0
                } else {
                    50.0
                }
            } else {
                0.0
            };
            let slow_k = if i >= 2 {
                (fast_k + fast_k_prev1 + fast_k_prev2) / 3.0
            } else {
                0.0
            };
            let d_sum = slow_k + slow_k_prev1 + slow_k_prev2;
            if i >= LOOKBACK {
                *k_out_cursor = slow_k;
                *d_out_cursor = d_sum / 3.0;
            }
            fast_k_prev2 = fast_k_prev1;
            fast_k_prev1 = fast_k;
            slow_k_prev2 = slow_k_prev1;
            slow_k_prev1 = slow_k;
            high_cursor = high_cursor.add(1);
            low_cursor = low_cursor.add(1);
            close_cursor = close_cursor.add(1);
            k_out_cursor = k_out_cursor.add(1);
            d_out_cursor = d_out_cursor.add(1);
        }
    }
}
