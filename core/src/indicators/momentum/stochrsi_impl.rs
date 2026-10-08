//! `StochRSI`: RSI fed back through the stochastic transform.

use super::prelude::*;

/// Stochastic RSI (STOCHRSI)
///
/// Applies Stochastic formula to RSI values instead of price.
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64 * 0.1).sin() * 10.0).collect();
/// let result = indicators::stochrsi(&close, 14, 14, 3, 3).unwrap();
/// assert_eq!(result.k.len(), 50);
/// ```
pub fn stochrsi(
    input: &[f64],
    rsi_period: usize,
    stoch_period: usize,
    fastk_period: usize,
    fastd_period: usize,
) -> Result<StochResult> {
    // The three-kernel shape this replaces (RSI array, then a raw-%K array,
    // then two SMA arrays) cost four full-length buffers and four passes. The
    // zero-copy path below is bit-identical: it parks RSI in the %D buffer,
    // rewrites %K in place, and finishes with the fused %K/%D ring pass.
    let len = input.len();
    let mut out_k = init_output(len);
    let mut out_d = init_output(len);
    {
        let k_slice = out_k.as_slice_mut().expect("owned Array1 is contiguous");
        let d_slice = out_d.as_slice_mut().expect("owned Array1 is contiguous");
        stochrsi_into(
            input,
            rsi_period,
            stoch_period,
            fastk_period,
            fastd_period,
            k_slice,
            d_slice,
        )?;
    }
    Ok(StochResult { k: out_k, d: out_d })
}

/// TA-Lib STOCHRSI with the official parameter contract.
///
/// `fastk_period` is the stochastic lookback over RSI values and
/// `fastd_period` smooths the resulting Fast-K series with the selected MA.
/// TA-Lib exposes both outputs only after the Fast-D lookback has elapsed.
pub fn stochrsi_with_ma_type(
    input: &[f64],
    timeperiod: usize,
    fastk_period: usize,
    fastd_period: usize,
    fastd_ma_type: MaType,
) -> Result<StochResult> {
    if fastk_period == 0 || fastd_period == 0 {
        return Err(TaError::InvalidParameter {
            name: "fastk_period, fastd_period".to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    let rsi_values = rsi(input, timeperiod)?;
    let rsi_slice = rsi_values.as_slice().unwrap();
    let rsi_start = timeperiod;
    if rsi_start >= rsi_slice.len() {
        return Err(TaError::InsufficientData {
            length: input.len(),
            required: timeperiod + fastk_period + fastd_period - 1,
        });
    }

    let valid_rsi = &rsi_slice[rsi_start..];
    validate_input(valid_rsi.len(), fastk_period)?;
    let mut raw_fastk = vec![f64::NAN; valid_rsi.len()];
    rolling_minmax_visit(valid_rsi, valid_rsi, fastk_period, |i, highest, lowest| {
        let range = highest - lowest;
        raw_fastk[i] = if range > crate::utils::TA_IS_ZERO_BANDWIDTH {
            (valid_rsi[i] - lowest) / range * 100.0
        } else {
            0.0
        };
    });

    let raw_start = rsi_start + fastk_period - 1;
    let fastd_values = crate::indicators::overlap::ma(
        &raw_fastk[fastk_period - 1..],
        fastd_period,
        fastd_ma_type,
    )?;
    let fastd_slice = fastd_values.as_slice().unwrap();
    let output_start = raw_start + fastd_period - 1;
    let mut fastk = vec![f64::NAN; input.len()];
    let mut fastd = vec![f64::NAN; input.len()];
    for i in output_start..input.len() {
        fastk[i] = raw_fastk[i - rsi_start];
    }
    for (offset, value) in fastd_slice.iter().enumerate() {
        fastd[raw_start + offset] = *value;
    }
    Ok(StochResult {
        k: Array1::from(fastk),
        d: Array1::from(fastd),
    })
}

/// Zero-copy STOCHRSI path.  RSI remains a small scratch series, while the
/// raw %K and public %K/%D outputs share the caller-owned buffers.
pub fn stochrsi_into(
    input: &[f64],
    rsi_period: usize,
    stoch_period: usize,
    fastk_period: usize,
    fastd_period: usize,
    out_k: &mut [f64],
    out_d: &mut [f64],
) -> Result<()> {
    if out_k.len() != input.len() || out_d.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output slices".to_string(),
            constraint: "must each have the same length as input".to_string(),
        });
    }
    if stoch_period == 0 || fastk_period == 0 || fastd_period == 0 {
        return Err(TaError::InvalidParameter {
            name: "stochastic periods".to_string(),
            constraint: "all periods must be greater than 0".to_string(),
        });
    }
    // RSI parks in the caller-owned %D buffer: pass two reads the series from
    // there while pass three overwrites it with %D, so the extra full-length
    // raw-%K scratch array this used to need is gone.
    rsi_into(input, rsi_period, out_d)?;
    let len = input.len();
    let window = stoch_period;
    let raw_start = rsi_period + window - 1;
    let k_start = raw_start + fastk_period - 1;
    let d_start = k_start + fastd_period - 1;

    // Cached-index arg-extremes over the RSI series — the same kernel `stoch`
    // and `stochf` use, and the one that beats the two `VecDeque`s it replaces
    // here: one comparison per leg per bar, a rescan only when the cached
    // extreme leaves the window, and no per-bar ring traffic. Ties keep the
    // newest position, matching the deques' `<=` / `>=` pop rules.
    let mut highest = f64::NEG_INFINITY;
    let mut highest_idx = 0usize;
    let mut lowest = f64::INFINITY;
    let mut lowest_idx = 0usize;
    {
        let rsi_series: &[f64] = out_d;
        for i in rsi_period..len {
            let value = rsi_series[i];
            if value >= highest {
                highest = value;
                highest_idx = i;
            } else if highest_idx + window <= i {
                let (rescanned, position, _found) = crate::math::statistics::rescan_extreme_window::<
                    true,
                >(rsi_series, i + 1 - window, i);
                highest = rescanned;
                highest_idx = position;
            }
            if value <= lowest {
                lowest = value;
                lowest_idx = i;
            } else if lowest_idx + window <= i {
                let (rescanned, position, _found) = crate::math::statistics::rescan_extreme_window::<
                    false,
                >(rsi_series, i + 1 - window, i);
                lowest = rescanned;
                lowest_idx = position;
            }
            if i >= raw_start {
                let range = highest - lowest;
                out_k[i] = if range > crate::utils::TA_IS_ZERO_BANDWIDTH {
                    (value - lowest) / range * 100.0
                } else {
                    0.0
                };
            }
        }
    }

    let mut k_sum = 0.0;
    let mut raw_ring = vec![0.0; fastk_period];
    let mut raw_pos = 0usize;
    let mut d_sum = 0.0;
    let mut d_ring = vec![0.0; fastd_period];
    let mut d_pos = 0usize;
    for i in raw_start..len {
        let raw = out_k[i];
        k_sum += raw - raw_ring[raw_pos];
        raw_ring[raw_pos] = raw;
        raw_pos += 1;
        if raw_pos == fastk_period {
            raw_pos = 0;
        }
        if i >= k_start {
            let smoothed_k = k_sum / fastk_period as f64;
            out_k[i] = smoothed_k;
            d_sum += smoothed_k - d_ring[d_pos];
            d_ring[d_pos] = smoothed_k;
            d_pos += 1;
            if d_pos == fastd_period {
                d_pos = 0;
            }
        }
        if i >= d_start {
            out_d[i] = d_sum / fastd_period as f64;
        }
    }
    // The first `fastk_period - 1` raw values seed %K internally but are not
    // exposed as public %K values by TA-Lib.
    out_k[..k_start.min(len)].fill(f64::NAN);
    // %D's buffer doubled as the RSI scratch series above, so its own warm-up
    // prefix still holds RSI values rather than the NaN the contract promises.
    out_d[..d_start.min(len)].fill(f64::NAN);
    Ok(())
}
