//! Fast stochastic (`STOCHF`) family.

use super::prelude::*;

/// Stochastic Fast (STOCHF)
///
/// Like STOCH but %K is unsmoothed and %D uses a simple MA of %K.
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::stochf(&high, &low, &close, 5, 3).unwrap();
/// assert_eq!(result.k.len(), 10);
/// ```
pub fn stochf(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    fastk_period: usize,
    fastd_period: usize,
) -> Result<StochResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), fastk_period)?;
    if fastd_period == 0 {
        return Err(TaError::InvalidParameter {
            name: "fastd_period".to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }

    if fastk_period == 5 && fastd_period == 3 {
        return stochf_5_3(high, low, close);
    }

    let len = high.len();
    let mut fastk = vec![f64::NAN; len];
    let mut fastd = vec![f64::NAN; len];

    let fastk_start = fastk_period - 1;
    let d_start = fastk_start + fastd_period - 1;
    let inv_d = 1.0 / fastd_period as f64;

    let mut d_ring = vec![0.0_f64; fastd_period];
    let mut d_sum: f64 = 0.0;
    // `d_ring_pos` grows by exactly one per emitted bar, so the ring position is
    // a wrapping counter — a `%` here would be a division on the hot path.
    let mut d_ring_pos = 0usize;

    // Cached-index arg-extremes — the kernel the sibling `stoch` uses for the
    // same lookback, and the one that beats a monotonic deque here: one
    // comparison per leg per bar, rescanning only when the cached extreme
    // leaves the window, and no per-bar allocation at all.
    //
    // The two `Vec<usize>` deques this replaces only advanced a logical head
    // and never compacted, so they grew to the full series length and
    // reallocated ~log2(len / period) times per call — on a 10k-bar series that
    // is ~160 KiB of churn and a `memcpy` per reallocation, which is what put
    // STOCHF behind TA-Lib while STOCH (same shape, cached kernel) stayed ahead.
    // Ties keep the newest position, matching the deque's `<=` pop rule.
    let mut highest = f64::NEG_INFINITY;
    let mut highest_idx = 0usize;
    let mut lowest = f64::INFINITY;
    let mut lowest_idx = 0usize;

    for i in 0..len {
        let new_high = high[i];
        let new_low = low[i];

        if new_high >= highest {
            highest = new_high;
            highest_idx = i;
        } else if highest_idx + fastk_period <= i {
            let (rescanned, position, _found) = crate::math::statistics::rescan_extreme_window::<
                true,
            >(high, i + 1 - fastk_period, i);
            highest = rescanned;
            highest_idx = position;
        }

        if new_low <= lowest {
            lowest = new_low;
            lowest_idx = i;
        } else if lowest_idx + fastk_period <= i {
            let (rescanned, position, _found) = crate::math::statistics::rescan_extreme_window::<
                false,
            >(low, i + 1 - fastk_period, i);
            lowest = rescanned;
            lowest_idx = position;
        }

        if i + 1 < fastk_period {
            continue;
        }

        let denom = highest - lowest;
        let fk = if denom > crate::utils::TA_IS_ZERO_BANDWIDTH {
            (close[i] - lowest) / denom * 100.0
        } else {
            0.0
        };

        d_sum += fk - d_ring[d_ring_pos];
        d_ring[d_ring_pos] = fk;
        d_ring_pos += 1;
        if d_ring_pos == fastd_period {
            d_ring_pos = 0;
        }

        if i >= d_start {
            fastk[i] = fk;
            fastd[i] = d_sum * inv_d;
        }
    }

    Ok(StochResult {
        k: Array1::from(fastk),
        d: Array1::from(fastd),
    })
}

/// TA-Lib stochastic fast oscillator with an explicit Fast-D MA type.
#[allow(clippy::too_many_arguments)]
pub fn stochf_with_ma_type(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    fastk_period: usize,
    fastd_period: usize,
    fastd_ma_type: MaType,
) -> Result<StochResult> {
    if fastd_ma_type == MaType::Sma {
        return stochf(high, low, close, fastk_period, fastd_period);
    }
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if fastd_period == 0 {
        return Err(TaError::InvalidParameter {
            name: "fastd_period".to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    validate_input(high.len(), fastk_period)?;

    let mut fastk_raw = vec![f64::NAN; close.len()];
    rolling_minmax_visit(high, low, fastk_period, |i, highest, lowest| {
        let range = highest - lowest;
        fastk_raw[i] = if range > crate::utils::TA_IS_ZERO_BANDWIDTH {
            (close[i] - lowest) / range * 100.0
        } else {
            0.0
        };
    });
    let fastk_start = fastk_period - 1;
    let fastd_values =
        crate::indicators::overlap::ma(&fastk_raw[fastk_start..], fastd_period, fastd_ma_type)?;
    let fastd_slice = fastd_values.as_slice().unwrap();
    let fastd_start = fastk_start + fastd_period - 1;
    let mut fastk = vec![f64::NAN; close.len()];
    let mut fastd = vec![f64::NAN; close.len()];
    for i in fastd_start..close.len() {
        fastk[i] = fastk_raw[i];
    }
    for (offset, value) in fastd_slice.iter().enumerate() {
        fastd[fastk_start + offset] = *value;
    }
    Ok(StochResult {
        k: Array1::from(fastk),
        d: Array1::from(fastd),
    })
}

/// Zero-copy STOCHF variant used by the Python compatibility layer.
pub fn stochf_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    fastk_period: usize,
    fastd_period: usize,
    out_k: &mut [f64],
    out_d: &mut [f64],
) -> Result<()> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if out_k.len() != high.len() || out_d.len() != high.len() {
        return Err(TaError::InvalidParameter {
            name: "output slices".to_string(),
            constraint: "must each have the same length as input".to_string(),
        });
    }
    validate_input(high.len(), fastk_period)?;
    if fastk_period == 5 && fastd_period == 3 {
        return stochf_5_3_into(high, low, close, out_k, out_d);
    }

    let result = stochf(high, low, close, fastk_period, fastd_period)?;
    out_k.copy_from_slice(result.k.as_slice().unwrap());
    out_d.copy_from_slice(result.d.as_slice().unwrap());
    Ok(())
}

#[inline]
pub(crate) fn stochf_5_3(high: &[f64], low: &[f64], close: &[f64]) -> Result<StochResult> {
    validate_input(high.len(), 5)?;
    let len = high.len();
    let mut fastk = vec![f64::NAN; len];
    let mut fastd = vec![f64::NAN; len];
    stochf_5_3_into(high, low, close, &mut fastk, &mut fastd)?;
    Ok(StochResult {
        k: Array1::from(fastk),
        d: Array1::from(fastd),
    })
}

#[inline(always)]
pub(crate) fn stochf_5_3_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    fastk: &mut [f64],
    fastd: &mut [f64],
) -> Result<()> {
    validate_input(high.len(), 5)?;
    let len = high.len();
    fastk.fill(f64::NAN);
    fastd.fill(f64::NAN);
    let mut highest_idx = usize::MAX;
    let mut lowest_idx = usize::MAX;
    let mut highest = 0.0;
    let mut lowest = 0.0;
    let mut d_ring = [0.0; 3];
    let mut d_sum = 0.0;
    let mut ring_pos = 0usize;

    unsafe {
        let high_ptr = high.as_ptr();
        let low_ptr = low.as_ptr();
        let close_ptr = close.as_ptr();
        let fastk_ptr = fastk.as_mut_ptr();
        let fastd_ptr = fastd.as_mut_ptr();
        for today in 4..len {
            let trailing = today - 4;
            if highest_idx == usize::MAX || highest_idx < trailing {
                highest_idx = trailing;
                highest = *high_ptr.add(trailing);
                let mut i = trailing + 1;
                while i <= today {
                    let value = *high_ptr.add(i);
                    if value > highest {
                        highest_idx = i;
                        highest = value;
                    }
                    i += 1;
                }
            } else if *high_ptr.add(today) >= highest {
                highest_idx = today;
                highest = *high_ptr.add(today);
            }
            if lowest_idx == usize::MAX || lowest_idx < trailing {
                lowest_idx = trailing;
                lowest = *low_ptr.add(trailing);
                let mut i = trailing + 1;
                while i <= today {
                    let value = *low_ptr.add(i);
                    if value < lowest {
                        lowest_idx = i;
                        lowest = value;
                    }
                    i += 1;
                }
            } else if *low_ptr.add(today) <= lowest {
                lowest_idx = today;
                lowest = *low_ptr.add(today);
            }
            let range = highest - lowest;
            let value = if range > crate::utils::TA_IS_ZERO_BANDWIDTH {
                (*close_ptr.add(today) - lowest) / range * 100.0
            } else {
                0.0
            };
            if today >= 6 {
                *fastk_ptr.add(today) = value;
            }
            d_sum += value - d_ring[ring_pos];
            d_ring[ring_pos] = value;
            if today >= 6 {
                *fastd_ptr.add(today) = d_sum / 3.0;
            }
            ring_pos += 1;
            if ring_pos == 3 {
                ring_pos = 0;
            }
        }
    }

    Ok(())
}
