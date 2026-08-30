use crate::error::{Result, TaError};
use crate::indicators::momentum::MacdResult;
use crate::math::moving_avg::{ema, vwma};
use crate::utils::{init_output, smoothing_factor, validate_input};
use ndarray::Array1;

const EOM_DIVISOR: f64 = 100_000_000.0;
const NVI_PVI_INITIAL: f64 = 1000.0;

fn validate_same_length(name: &str, lengths: &[usize]) -> Result<()> {
    if lengths.is_empty() {
        return Ok(());
    }
    if lengths.iter().any(|&len| len != lengths[0]) {
        return Err(TaError::InvalidParameter {
            name: name.to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    Ok(())
}

fn validate_period(name: &str, period: usize) -> Result<()> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: name.to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    Ok(())
}

/// Chaikin Money Flow (CMF)
///
/// Measures the amount of money flow volume over a given period.
///
/// # Formula
/// MFM = ((Close - Low) - (High - Close)) / (High - Low)
/// MFV = MFM × Volume
/// CMF = SUM(MFV, period) / SUM(Volume, period)
pub fn cmf(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    period: usize,
) -> Result<Array1<f64>> {
    validate_same_length("high, low, close, volume", &[high.len(), low.len(), close.len(), volume.len()])?;
    validate_period("period", period)?;
    validate_input(high.len(), period)?;

    let len = high.len();
    let mut output = init_output(len);

    if len < period {
        return Ok(output);
    }

    #[inline(always)]
    fn compute_mfv(h: f64, l: f64, c: f64, v: f64) -> (f64, f64, bool) {
        let range = h - l;
        if range.abs() > 1e-15
            && !h.is_nan() && !l.is_nan()
            && !c.is_nan() && !v.is_nan()
        {
            let mfv = ((c - l) - (h - c)) / range * v;
            (mfv, v, true)
        } else {
            (0.0, 0.0, false)
        }
    }

    let mut mfv_ring = vec![0.0f64; period];
    let mut vol_ring = vec![0.0f64; period];
    let mut valid_ring = vec![false; period];
    let mut sum_mfv = 0.0;
    let mut sum_vol = 0.0;
    let mut valid_count = 0usize;

    for i in 0..period {
        let (mfv, vol, ok) = compute_mfv(high[i], low[i], close[i], volume[i]);
        mfv_ring[i] = mfv;
        vol_ring[i] = vol;
        valid_ring[i] = ok;
        if ok {
            sum_mfv += mfv;
            sum_vol += vol;
            valid_count += 1;
        }
    }

    if valid_count == period && sum_vol.abs() > 1e-15 {
        output[period - 1] = sum_mfv / sum_vol;
    }

    for i in period..len {
        let slot = i % period;
        if valid_ring[slot] {
            sum_mfv -= mfv_ring[slot];
            sum_vol -= vol_ring[slot];
            valid_count -= 1;
        }
        let (mfv, vol, ok) = compute_mfv(high[i], low[i], close[i], volume[i]);
        mfv_ring[slot] = mfv;
        vol_ring[slot] = vol;
        valid_ring[slot] = ok;
        if ok {
            sum_mfv += mfv;
            sum_vol += vol;
            valid_count += 1;
        }
        if valid_count == period && sum_vol.abs() > 1e-15 {
            output[i] = sum_mfv / sum_vol;
        }
    }

    Ok(output)
}

/// Force Index
///
/// Measures the force behind a price move using price change and volume.
///
/// # Formula
/// Force(1) = (Close - Close_prev) × Volume
/// Force Index = EMA(Force(1), period)
pub fn force_index(close: &[f64], volume: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_same_length("close, volume", &[close.len(), volume.len()])?;
    validate_period("period", period)?;
    validate_input(close.len(), period + 1)?;

    let len = close.len();
    let mut output = init_output(len);

    if len <= period {
        return Ok(output);
    }

    let k = smoothing_factor(period);
    let one_minus_k = 1.0 - k;

    let raw_force_at = |i: usize| -> f64 {
        if close[i].is_nan() || close[i - 1].is_nan() || volume[i].is_nan() {
            f64::NAN
        } else {
            (close[i] - close[i - 1]) * volume[i]
        }
    };

    let mut sum = 0.0;
    for i in 1..=period {
        sum += raw_force_at(i);
    }
    let mut prev = sum / period as f64;
    output[period] = prev;

    for i in period + 1..len {
        let f = raw_force_at(i);
        prev = f * k + prev * one_minus_k;
        output[i] = prev;
    }

    Ok(output)
}

/// Ease of Movement (EOM)
///
/// Relates price change to volume and facilitates comparison between instruments.
///
/// # Formula
/// Distance = ((High + Low) / 2) - ((High_prev + Low_prev) / 2)
/// Box Ratio = (Volume / divisor) / (High - Low)
/// EOM = SMA(Distance / Box Ratio, period)
pub fn eom(high: &[f64], low: &[f64], volume: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_same_length("high, low, volume", &[high.len(), low.len(), volume.len()])?;
    validate_period("period", period)?;
    validate_input(high.len(), period + 1)?;

    let len = high.len();
    let mut output = init_output(len);
    let inv_period = 1.0 / period as f64;

    let mut raw = vec![0.0f64; len];
    for i in 1..len {
        let distance = (high[i] + low[i]) - (high[i - 1] + low[i - 1]);
        let range = high[i] - low[i];
        if range.abs() > 1e-15 && volume[i].abs() > 1e-15 {
            raw[i] = distance * 0.5 * range * EOM_DIVISOR / volume[i];
        }
    }

    let first = period - 1;
    let mut sum: f64 = raw[..period].iter().sum();
    output[first] = sum * inv_period;

    for i in period..len {
        sum += raw[i] - raw[i - period];
        output[i] = sum * inv_period;
    }

    Ok(output)
}

/// Klinger Volume Oscillator (KVO) result
#[derive(Debug, Clone)]
pub struct KvoResult {
    pub kvo: Array1<f64>,
    pub signal: Array1<f64>,
}

/// Klinger Volume Oscillator (KVO)
///
/// Combines price and volume to identify long-term money flow trends.
///
/// # Formula
/// dm = High - Low
/// cm accumulates dm in the direction of trend
/// VF = Volume × |2 × dm / cm - 1| × sign(trend)
/// KVO = EMA(VF, fast) - EMA(VF, slow)
/// Signal = EMA(KVO, signal_period)
pub fn kvo(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> Result<KvoResult> {
    validate_same_length(
        "high, low, close, volume",
        &[high.len(), low.len(), close.len(), volume.len()],
    )?;
    validate_period("fast_period", fast_period)?;
    validate_period("slow_period", slow_period)?;
    validate_period("signal_period", signal_period)?;
    validate_input(high.len(), slow_period.max(fast_period) + 1)?;

    let len = high.len();
    let max_period = fast_period.max(slow_period);
    let kvo_start = max_period - 1;

    let mut kvo = init_output(len);
    let mut signal = init_output(len);

    // Incremental state for dm, cm, trend (avoid storing complete arrays)
    let mut prev_dm: f64 = 0.0;
    let mut prev_cm: f64 = 0.0;
    let mut prev_trend: i32 = 0;

    // Incremental EMA state for fast and slow
    let fast_k = smoothing_factor(fast_period);
    let fast_one_minus_k = 1.0 - fast_k;
    let mut fast_sum = 0.0;
    let mut fast_count = 0usize;
    let mut fast_ema = f64::NAN;

    let slow_k = smoothing_factor(slow_period);
    let slow_one_minus_k = 1.0 - slow_k;
    let mut slow_sum = 0.0;
    let mut slow_count = 0usize;
    let mut slow_ema = f64::NAN;

    // Incremental EMA state for signal
    let signal_k = smoothing_factor(signal_period);
    let signal_one_minus_k = 1.0 - signal_k;
    let mut signal_sum = 0.0;
    let mut signal_count = 0usize;
    let mut signal_ema_val = f64::NAN;

    for i in 0..len {
        // Compute dm[i]
        let dm_i = if high[i].is_nan() || low[i].is_nan() {
            f64::NAN
        } else {
            high[i] - low[i]
        };

        // Compute trend[i] and cm[i]
        let (trend_i, cm_i) = if i == 0 {
            let t = if !close[i].is_nan() { 1 } else { 0 };
            let c = if !dm_i.is_nan() { dm_i } else { f64::NAN };
            (t, c)
        } else if close[i].is_nan()
            || close[i - 1].is_nan()
            || high[i].is_nan()
            || low[i].is_nan()
            || high[i - 1].is_nan()
            || low[i - 1].is_nan()
        {
            (prev_trend, f64::NAN)
        } else {
            let today = high[i] + low[i] + close[i];
            let prev_val = high[i - 1] + low[i - 1] + close[i - 1];
            let t = if today > prev_val { 1 } else { -1 };
            let c = if dm_i.is_nan() || prev_dm.is_nan() {
                f64::NAN
            } else if t == prev_trend {
                prev_cm + dm_i
            } else {
                prev_dm + dm_i
            };
            (t, c)
        };

        // Compute vf[i]
        let vf_i =
            if volume[i].is_nan() || dm_i.is_nan() || cm_i.is_nan() || cm_i.abs() <= 1e-15 {
                0.0
            } else {
                volume[i] * (2.0 * dm_i / cm_i - 1.0).abs() * trend_i as f64
            };

        // Update fast EMA
        if fast_count < fast_period {
            fast_sum += vf_i;
            fast_count += 1;
            if fast_count == fast_period {
                fast_ema = fast_sum / fast_period as f64;
            }
        } else {
            fast_ema = vf_i * fast_k + fast_ema * fast_one_minus_k;
        }

        // Update slow EMA
        if slow_count < slow_period {
            slow_sum += vf_i;
            slow_count += 1;
            if slow_count == slow_period {
                slow_ema = slow_sum / slow_period as f64;
            }
        } else {
            slow_ema = vf_i * slow_k + slow_ema * slow_one_minus_k;
        }

        // Compute KVO
        if i >= kvo_start && !fast_ema.is_nan() && !slow_ema.is_nan() {
            kvo[i] = fast_ema - slow_ema;
        }

        // Update signal EMA
        if i >= kvo_start && !kvo[i].is_nan() {
            if signal_count < signal_period {
                signal_sum += kvo[i];
                signal_count += 1;
                if signal_count == signal_period {
                    signal_ema_val = signal_sum / signal_period as f64;
                    signal[i] = signal_ema_val;
                }
            } else {
                signal_ema_val = kvo[i] * signal_k + signal_ema_val * signal_one_minus_k;
                signal[i] = signal_ema_val;
            }
        }

        // Update prev state
        prev_dm = dm_i;
        prev_cm = cm_i;
        prev_trend = trend_i;
    }

    Ok(KvoResult { kvo, signal })
}

/// Negative Volume Index (NVI)
///
/// Cumulative index that only changes on days when volume decreases.
///
/// # Formula
/// Initial NVI = 1000
/// When volume < volume_prev: NVI = NVI_prev × (1 + (Close - Close_prev) / Close_prev)
/// Otherwise: NVI = NVI_prev
#[inline]
pub fn nvi(close: &[f64], volume: &[f64]) -> Result<Array1<f64>> {
    validate_same_length("close, volume", &[close.len(), volume.len()])?;
    validate_input(close.len(), 1)?;

    let len = close.len();
    let has_any_nan = close.iter().any(|v| v.is_nan()) || volume.iter().any(|v| v.is_nan());

    if !has_any_nan {
        let mut output = vec![NVI_PVI_INITIAL; len];
        let mut prev = NVI_PVI_INITIAL;
        for i in 1..len {
            if volume[i] < volume[i - 1] {
                let inv_cp = 1.0 / close[i - 1];
                prev *= 1.0 + (close[i] - close[i - 1]) * inv_cp;
            }
            output[i] = prev;
        }
        Ok(Array1::from_vec(output))
    } else {
        let mut output = vec![NVI_PVI_INITIAL; len];
        let mut prev = NVI_PVI_INITIAL;
        for i in 1..len {
            let c = close[i];
            let cp = close[i - 1];
            if volume[i] < volume[i - 1] && cp.abs() > 1e-15 && !c.is_nan() && !cp.is_nan()
                && !volume[i].is_nan() && !volume[i - 1].is_nan()
            {
                prev += prev * (c - cp) / cp;
                output[i] = prev;
            } else if c.is_nan() || cp.is_nan() || volume[i].is_nan() || volume[i - 1].is_nan() {
                output[i] = f64::NAN;
                prev = f64::NAN;
            } else {
                output[i] = prev;
            }
        }
        Ok(Array1::from_vec(output))
    }
}

/// Positive Volume Index (PVI)
///
/// Cumulative index that only changes on days when volume increases.
///
/// # Formula
/// Initial PVI = 1000
/// When volume > volume_prev: PVI = PVI_prev × (1 + (Close - Close_prev) / Close_prev)
/// Otherwise: PVI = PVI_prev
#[inline]
pub fn pvi(close: &[f64], volume: &[f64]) -> Result<Array1<f64>> {
    validate_same_length("close, volume", &[close.len(), volume.len()])?;
    validate_input(close.len(), 1)?;

    let len = close.len();
    let has_any_nan = close.iter().any(|v| v.is_nan()) || volume.iter().any(|v| v.is_nan());

    if !has_any_nan {
        let mut output = vec![NVI_PVI_INITIAL; len];
        let mut prev = NVI_PVI_INITIAL;
        for i in 1..len {
            if volume[i] > volume[i - 1] {
                let inv_cp = 1.0 / close[i - 1];
                prev *= 1.0 + (close[i] - close[i - 1]) * inv_cp;
            }
            output[i] = prev;
        }
        Ok(Array1::from_vec(output))
    } else {
        let mut output = vec![NVI_PVI_INITIAL; len];
        let mut prev = NVI_PVI_INITIAL;
        for i in 1..len {
            let c = close[i];
            let cp = close[i - 1];
            if volume[i] > volume[i - 1] && cp.abs() > 1e-15 && !c.is_nan() && !cp.is_nan()
                && !volume[i].is_nan() && !volume[i - 1].is_nan()
            {
                prev += prev * (c - cp) / cp;
                output[i] = prev;
            } else if c.is_nan() || cp.is_nan() || volume[i].is_nan() || volume[i - 1].is_nan() {
                output[i] = f64::NAN;
                prev = f64::NAN;
            } else {
                output[i] = prev;
            }
        }
        Ok(Array1::from_vec(output))
    }
}

/// Volume Weighted MACD (VWMACD)
///
/// Same structure as MACD, but uses VWMA instead of EMA for the fast and slow lines.
///
/// # Formula
/// Fast Line = VWMA(Close, Volume, fast_period)
/// Slow Line = VWMA(Close, Volume, slow_period)
/// VWMACD = Fast Line - Slow Line
/// Signal = EMA(VWMACD, signal_period)
/// Histogram = VWMACD - Signal
pub fn vwmacd(
    close: &[f64],
    volume: &[f64],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> Result<MacdResult> {
    validate_same_length("close, volume", &[close.len(), volume.len()])?;
    validate_period("fast_period", fast_period)?;
    validate_period("slow_period", slow_period)?;
    validate_period("signal_period", signal_period)?;
    if fast_period >= slow_period {
        return Err(TaError::InvalidParameter {
            name: "fast_period".to_string(),
            constraint: "less than slow_period".to_string(),
        });
    }
    validate_input(close.len(), slow_period)?;

    let len = close.len();
    let fast_vwma = vwma(close, volume, fast_period)?;
    let slow_vwma = vwma(close, volume, slow_period)?;

    let mut macd_line = init_output(len);
    let mut signal = init_output(len);
    let mut hist = init_output(len);

    let signal_k = smoothing_factor(signal_period);
    let signal_one_minus_k = 1.0 - signal_k;
    let signal_inv = 1.0 / signal_period as f64;

    let mut signal_sma_sum = 0.0;
    let mut signal_ema = 0.0;
    let slow_start = slow_period - 1;

    for i in 0..len {
        if i >= slow_start && !fast_vwma[i].is_nan() && !slow_vwma[i].is_nan() {
            macd_line[i] = fast_vwma[i] - slow_vwma[i];
        }

        let signal_input = if i >= slow_start { macd_line[i] } else { 0.0 };

        if i < signal_period - 1 {
            signal_sma_sum += signal_input;
        } else if i == signal_period - 1 {
            signal_sma_sum += signal_input;
            signal_ema = signal_sma_sum * signal_inv;
            signal[i] = signal_ema;
        } else {
            signal_ema = signal_input * signal_k + signal_ema * signal_one_minus_k;
            signal[i] = signal_ema;
        }

        if !macd_line[i].is_nan() && !signal[i].is_nan() {
            hist[i] = macd_line[i] - signal[i];
        }
    }

    Ok(MacdResult {
        macd: macd_line,
        signal,
        hist,
    })
}

/// Price Volume Trend (PVT)
///
/// Cumulative indicator that adds a percentage change in price multiplied by volume.
///
/// # Formula
/// PVT = PVT_prev + Volume × (Close - Close_prev) / Close_prev
#[inline]
pub fn pvt(close: &[f64], volume: &[f64]) -> Result<Array1<f64>> {
    validate_same_length("close, volume", &[close.len(), volume.len()])?;
    validate_input(close.len(), 1)?;

    let len = close.len();
    let has_any_nan = close.iter().any(|v| v.is_nan()) || volume.iter().any(|v| v.is_nan());

    if !has_any_nan {
        let mut output = vec![0.0; len];
        let mut acc = 0.0;
        for i in 1..len {
            let inv_cp = 1.0 / close[i - 1];
            acc += volume[i] * (close[i] - close[i - 1]) * inv_cp;
            output[i] = acc;
        }
        Ok(Array1::from_vec(output))
    } else {
        let mut output = vec![0.0; len];
        let mut acc = 0.0;
        for i in 1..len {
            let c = close[i];
            let cp = close[i - 1];
            if c.is_nan() || cp.is_nan() || volume[i].is_nan() {
                output[i] = f64::NAN;
                acc = f64::NAN;
            } else if cp.abs() <= 1e-15 {
                output[i] = acc;
            } else {
                acc += volume[i] * (c - cp) / cp;
                output[i] = acc;
            }
        }
        Ok(Array1::from_vec(output))
    }
}

/// Money Flow Index (MFI) - Extended version with divergence detection
///
/// Uses both price and volume to identify buying and selling pressure.
///
/// # Formula
/// Typical Price = (High + Low + Close) / 3
/// Raw Money Flow = Typical Price × Volume
/// Positive MF = Sum of Raw MF when TP > TP_prev
/// Negative MF = Sum of Raw MF when TP < TP_prev
/// MFI = 100 - 100 / (1 + Positive MF / Negative MF)
pub fn mfi_ext(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    period: usize,
) -> Result<Array1<f64>> {
    validate_same_length("high, low, close, volume", &[high.len(), low.len(), close.len(), volume.len()])?;
    validate_period("period", period)?;
    validate_input(high.len(), period + 1)?;

    let len = close.len();
    let mut output = init_output(len);

    // Calculate typical price
    let tp: Vec<f64> = close
        .iter()
        .zip(high.iter())
        .zip(low.iter())
        .map(|((&c, &h), &l)| (h + l + c) / 3.0)
        .collect();

    // Calculate raw money flow
    let rmf: Vec<f64> = tp
        .iter()
        .zip(volume.iter())
        .map(|(&t, &v)| t * v)
        .collect();

    // Rolling sum of positive and negative money flow
    let mut pos_mf_ring = vec![0.0f64; period];
    let mut neg_mf_ring = vec![0.0f64; period];
    let mut sum_pos = 0.0;
    let mut sum_neg = 0.0;

    for i in 1..len {
        let pos_mf = if tp[i] > tp[i - 1] && !rmf[i].is_nan() { rmf[i] } else { 0.0 };
        let neg_mf = if tp[i] < tp[i - 1] && !rmf[i].is_nan() { rmf[i] } else { 0.0 };

        if i <= period {
            pos_mf_ring[i - 1] = pos_mf;
            neg_mf_ring[i - 1] = neg_mf;
            sum_pos += pos_mf;
            sum_neg += neg_mf;
        } else {
            let slot = (i - 1) % period;
            sum_pos += pos_mf - pos_mf_ring[slot];
            sum_neg += neg_mf - neg_mf_ring[slot];
            pos_mf_ring[slot] = pos_mf;
            neg_mf_ring[slot] = neg_mf;
        }

        if i >= period {
            let mf_ratio = if sum_neg.abs() > 1e-15 { sum_pos / sum_neg } else { 0.0 };
            output[i] = 100.0 - 100.0 / (1.0 + mf_ratio);
        }
    }

    Ok(output)
}

/// Volume Oscillator
///
/// Measures the difference between two volume moving averages.
///
/// # Formula
/// Fast MA = SMA(Volume, fast_period)
/// Slow MA = SMA(Volume, slow_period)
/// Volume Oscillator = Fast MA - Slow MA
/// Percentage = (Fast MA - Slow MA) / Slow MA × 100
pub fn volume_oscillator(
    volume: &[f64],
    fast_period: usize,
    slow_period: usize,
) -> Result<Array1<f64>> {
    validate_period("fast_period", fast_period)?;
    validate_period("slow_period", slow_period)?;
    if fast_period >= slow_period {
        return Err(TaError::InvalidParameter {
            name: "fast_period".to_string(),
            constraint: "less than slow_period".to_string(),
        });
    }
    validate_input(volume.len(), slow_period)?;

    let len = volume.len();
    let mut output = init_output(len);

    // Calculate fast SMA
    let mut fast_sum = 0.0;
    let mut fast_sma = vec![f64::NAN; len];
    for i in 0..len {
        fast_sum += volume[i];
        if i >= fast_period - 1 {
            if i >= fast_period {
                fast_sum -= volume[i - fast_period];
            }
            fast_sma[i] = fast_sum / fast_period as f64;
        }
    }

    // Calculate slow SMA
    let mut slow_sum = 0.0;
    let mut slow_sma = vec![f64::NAN; len];
    for i in 0..len {
        slow_sum += volume[i];
        if i >= slow_period - 1 {
            if i >= slow_period {
                slow_sum -= volume[i - slow_period];
            }
            slow_sma[i] = slow_sum / slow_period as f64;
        }
    }

    // Calculate oscillator
    for i in slow_period - 1..len {
        if !fast_sma[i].is_nan() && !slow_sma[i].is_nan() && slow_sma[i].abs() > 1e-15 {
            output[i] = (fast_sma[i] - slow_sma[i]) / slow_sma[i] * 100.0;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_cmf_basic() {
        let high = vec![12.0, 13.0, 14.0, 15.0, 16.0];
        let low = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let close = vec![11.0, 12.0, 13.0, 14.0, 15.0];
        let volume = vec![100.0, 200.0, 150.0, 300.0, 250.0];
        let result = cmf(&high, &low, &close, &volume, 3).unwrap();
        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_finite());
        assert!(result[4].is_finite());
    }

    #[test]
    fn test_cmf_known_value() {
        let high = vec![10.0, 12.0, 14.0];
        let low = vec![8.0, 10.0, 12.0];
        let close = vec![9.0, 11.0, 13.0];
        let volume = vec![100.0, 100.0, 100.0];
        let result = cmf(&high, &low, &close, &volume, 2).unwrap();
        // MFM all = 0 (close at midpoint), so CMF = 0
        assert_relative_eq!(result[1], 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_cmf_zero_range() {
        let high = vec![10.0, 10.0, 10.0];
        let low = vec![10.0, 10.0, 10.0];
        let close = vec![10.0, 10.0, 10.0];
        let volume = vec![100.0, 200.0, 300.0];
        let result = cmf(&high, &low, &close, &volume, 2).unwrap();
        assert!(result[1].is_nan() || result[1].is_finite());
    }

    #[test]
    fn test_cmf_invalid_params() {
        let high = vec![10.0, 11.0];
        let low = vec![9.0, 10.0];
        let close = vec![9.5, 10.5];
        let volume = vec![100.0];
        assert!(cmf(&high, &low, &close, &volume, 0).is_err());
        assert!(cmf(&high, &low, &close, &volume, 2).is_err());
    }

    #[test]
    fn test_force_index_basic() {
        let close = vec![10.0, 11.0, 10.5, 12.0, 11.5, 13.0];
        let volume = vec![100.0, 200.0, 150.0, 300.0, 250.0, 180.0];
        let result = force_index(&close, &volume, 3).unwrap();
        assert_eq!(result.len(), 6);
        assert!(result[0].is_nan() || result[1].is_nan());
        assert!(result[5].is_finite());
    }

    #[test]
    fn test_force_index_raw_calculation() {
        let close = vec![10.0, 11.0, 10.0];
        let volume = vec![100.0, 200.0, 150.0];
        let result = force_index(&close, &volume, 2).unwrap();
        // raw_force = [(11-10)*200=200, (10-11)*150=-150], length=2
        // EMA(2): initial SMA at index 1 = (200 + (-150))/2 = 25
        // maps to orig_idx = 1 + 1 = 2
        assert!(!result[2].is_nan());
        assert_relative_eq!(result[2], 25.0, epsilon = 1e-10);
    }

    #[test]
    fn test_eom_basic() {
        let high = vec![12.0, 13.0, 14.0, 15.0, 16.0, 17.0];
        let low = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let volume = vec![1_000_000.0, 1_200_000.0, 900_000.0, 1_100_000.0, 1_300_000.0, 1_000_000.0];
        let result = eom(&high, &low, &volume, 3).unwrap();
        assert_eq!(result.len(), 6);
        assert!(result[0].is_nan());
        assert!(result[2].is_finite());
    }

    #[test]
    fn test_eom_invalid_params() {
        let high = vec![12.0, 13.0];
        let low = vec![10.0, 11.0];
        let volume = vec![1_000_000.0, 1_200_000.0];
        assert!(eom(&high, &low, &volume, 0).is_err());
        assert!(eom(&high, &low, &volume, 5).is_err());
    }

    #[test]
    fn test_kvo_basic() {
        let high = vec![12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let low = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0];
        let close = vec![11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let volume = vec![100.0, 200.0, 150.0, 300.0, 250.0, 180.0, 220.0, 280.0];
        let result = kvo(&high, &low, &close, &volume, 3, 5, 2).unwrap();
        assert_eq!(result.kvo.len(), 8);
        assert_eq!(result.signal.len(), 8);
        assert!(result.kvo[7].is_finite());
    }

    #[test]
    fn test_kvo_default_periods() {
        let n = 80;
        let high: Vec<f64> = (0..n).map(|i| 10.0 + i as f64 + 1.0).collect();
        let low: Vec<f64> = (0..n).map(|i| 10.0 + i as f64 - 1.0).collect();
        let close: Vec<f64> = (0..n).map(|i| 10.0 + i as f64).collect();
        let volume: Vec<f64> = (0..n).map(|_| 1000.0).collect();
        let result = kvo(&high, &low, &close, &volume, 34, 55, 13).unwrap();
        assert!(result.kvo[n - 1].is_finite());
        assert!(result.signal[n - 1].is_finite());
    }

    #[test]
    fn test_nvi_basic() {
        let close = vec![10.0, 11.0, 10.5, 12.0, 11.0];
        let volume = vec![1000.0, 900.0, 1100.0, 800.0, 950.0];
        let result = nvi(&close, &volume).unwrap();
        assert_relative_eq!(result[0], 1000.0, epsilon = 1e-10);
        // volume[1]=900 < volume[0]=1000, close up 10%
        assert_relative_eq!(result[1], 1100.0, epsilon = 1e-10);
        // volume[2]=1100 > volume[1]=900, unchanged
        assert_relative_eq!(result[2], 1100.0, epsilon = 1e-10);
        // volume[3]=800 < volume[2]=1100, close 10.5->12 (+14.29%)
        assert_relative_eq!(result[3], 1257.142857, epsilon = 1e-5);
    }

    #[test]
    fn test_pvi_basic() {
        let close = vec![10.0, 11.0, 10.5, 12.0, 11.0];
        let volume = vec![1000.0, 1100.0, 900.0, 1200.0, 950.0];
        let result = pvi(&close, &volume).unwrap();
        assert_relative_eq!(result[0], 1000.0, epsilon = 1e-10);
        // volume[1]=1100 > volume[0]=1000, close up 10%
        assert_relative_eq!(result[1], 1100.0, epsilon = 1e-10);
        // volume[2]=900 < volume[1]=1100, unchanged
        assert_relative_eq!(result[2], 1100.0, epsilon = 1e-10);
        // volume[3]=1200 > volume[2]=900, close up ~14.29%
        assert_relative_eq!(result[3], 1257.142857, epsilon = 1e-5);
    }

    #[test]
    fn test_pvt_basic() {
        let close = vec![10.0, 11.0, 10.0, 12.0];
        let volume = vec![100.0, 200.0, 150.0, 300.0];
        let result = pvt(&close, &volume).unwrap();
        assert_relative_eq!(result[0], 0.0, epsilon = 1e-10);
        // +200 * (11-10)/10 = 20
        assert_relative_eq!(result[1], 20.0, epsilon = 1e-10);
        // 20 + 150 * (10-11)/11 = 20 - 13.636...
        assert_relative_eq!(result[2], 6.363636, epsilon = 1e-5);
        // 6.3636 + 300 * (12-10)/10 = 66.3636
        assert_relative_eq!(result[3], 66.363636, epsilon = 1e-5);
    }

    #[test]
    fn test_nan_propagation() {
        let close = vec![10.0, f64::NAN, 12.0];
        let volume = vec![100.0, 200.0, 300.0];
        let nvi_result = nvi(&close, &volume).unwrap();
        assert!(nvi_result[1].is_nan());

        let pvi_result = pvi(&close, &volume).unwrap();
        assert!(pvi_result[1].is_nan());

        let pvt_result = pvt(&close, &volume).unwrap();
        assert!(pvt_result[1].is_nan());
    }

    #[test]
    fn test_vwmacd_basic() {
        let close: Vec<f64> = (1..=40).map(|x| x as f64).collect();
        let volume: Vec<f64> = (1..=40).map(|x| 100.0 + x as f64).collect();
        let result = vwmacd(&close, &volume, 12, 26, 9).unwrap();
        assert_eq!(result.macd.len(), 40);
        assert_eq!(result.signal.len(), 40);
        assert_eq!(result.hist.len(), 40);
        assert!(result.macd[0].is_nan());
        assert!(result.macd[25].is_finite());
        assert!(result.macd[39].is_finite());
    }

    #[test]
    fn test_vwmacd_constant_price() {
        let close = vec![10.0; 30];
        let volume: Vec<f64> = (1..=30).map(|x| x as f64 * 100.0).collect();
        let result = vwmacd(&close, &volume, 5, 10, 3).unwrap();
        // Constant price => VWMA lines equal => MACD ~ 0
        assert_relative_eq!(result.macd[29], 0.0, epsilon = 1e-10);
        assert_relative_eq!(result.hist[29], 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_vwmacd_invalid_params() {
        let close = vec![10.0, 11.0, 12.0];
        let volume = vec![100.0, 200.0, 150.0];
        assert!(vwmacd(&close, &volume, 0, 2, 2).is_err());
        assert!(vwmacd(&close, &volume, 3, 2, 2).is_err());
        assert!(vwmacd(&close, &volume, 2, 5, 2).is_err());
    }

    #[test]
    fn test_vwmacd_length_mismatch() {
        let close = vec![10.0, 11.0, 12.0];
        let volume = vec![100.0, 200.0];
        assert!(vwmacd(&close, &volume, 2, 3, 2).is_err());
    }

    #[test]
    fn test_nvi_pvi_invalid_length() {
        let close = vec![10.0, 11.0];
        let volume = vec![100.0];
        assert!(nvi(&close, &volume).is_err());
        assert!(pvi(&close, &volume).is_err());
        assert!(pvt(&close, &volume).is_err());
    }
}

// ============================================================================
// Twiggs Money Flow
// ============================================================================

/// Twiggs Money Flow (TMF)
///
/// A variation of Accumulation/Distribution that uses EMA smoothing and
/// True Range instead of simple High-Low range.
///
/// TMF = EMA(Volume × (2×Close - TrueHigh - TrueLow) / (TrueHigh - TrueLow)) / EMA(Volume)
///
/// Where:
/// - TrueHigh = max(High, PrevClose)
/// - TrueLow = min(Low, PrevClose)
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `period` - EMA smoothing period
///
/// # Returns
/// TMF values (-1 to 1 range). First value is NaN.
pub fn twiggs_money_flow(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    period: usize,
) -> Result<Array1<f64>> {
    let len = high.len();
    validate_same_length("high, low, close, volume", &[len, low.len(), close.len(), volume.len()])?;
    if period < 1 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "at least 1".to_string(),
        });
    }
    validate_input(len, period + 1)?;

    let mut output = vec![f64::NAN; len];
    let alpha = smoothing_factor(period);

    let mut ema_ad = 0.0_f64;
    let mut ema_vol = 0.0_f64;
    let mut initialized = false;

    for i in 1..len {
        let true_high = high[i].max(close[i - 1]);
        let true_low = low[i].min(close[i - 1]);
        let tr = true_high - true_low;

        let ad_val = if tr > 1e-15 {
            volume[i] * (2.0 * close[i] - true_high - true_low) / tr
        } else {
            0.0
        };

        if !initialized {
            ema_ad = ad_val;
            ema_vol = volume[i];
            initialized = true;
        } else {
            ema_ad = alpha * ad_val + (1.0 - alpha) * ema_ad;
            ema_vol = alpha * volume[i] + (1.0 - alpha) * ema_vol;
        }

        if i >= period && ema_vol.abs() > 1e-15 {
            output[i] = ema_ad / ema_vol;
        }
    }

    Ok(Array1::from_vec(output))
}

#[cfg(test)]
mod twiggs_mf_tests {
    use super::*;

    #[test]
    fn test_twiggs_mf_basic() {
        let n = 30;
        let high: Vec<f64> = (0..n).map(|i| 105.0 + i as f64 * 0.5).collect();
        let low: Vec<f64> = (0..n).map(|i| 95.0 + i as f64 * 0.5).collect();
        let close: Vec<f64> = (0..n).map(|i| 100.0 + i as f64 * 0.5).collect();
        let volume: Vec<f64> = (0..n).map(|i| 1000.0 + (i as f64 * 0.7).sin() * 200.0).collect();

        let result = twiggs_money_flow(&high, &low, &close, &volume, 14).unwrap();
        assert_eq!(result.len(), n);
        assert!(result[0].is_nan());
        for i in 14..n {
            assert!(result[i].is_finite(), "NaN at index {i}");
            assert!(result[i] >= -1.1 && result[i] <= 1.1);
        }
    }

    #[test]
    fn test_twiggs_mf_invalid() {
        let h = vec![2.0; 5];
        let l = vec![1.0; 5];
        let c = vec![1.5; 5];
        let v = vec![100.0; 3]; // mismatched length
        assert!(twiggs_money_flow(&h, &l, &c, &v, 3).is_err());
    }

    #[test]
    fn test_twiggs_mf_insufficient_data() {
        let h = vec![2.0; 3];
        let l = vec![1.0; 3];
        let c = vec![1.5; 3];
        let v = vec![100.0; 3];
        assert!(twiggs_money_flow(&h, &l, &c, &v, 10).is_err());
    }
}

// ============================================================================
// Volume Zone Oscillator (VZO)
// ============================================================================

/// Volume Zone Oscillator (VZO)
///
/// VZO = EMA(VP, period) / EMA(TV, period) × 100
/// VP (Volume Pressure) = volume if close > prev_close, -volume if close < prev_close, 0 if equal
/// TV = volume (always positive)
///
/// # Arguments
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `period` - EMA smoothing period (typically 14)
///
/// # Returns
/// VZO values (-100 to 100). First value is NaN.
pub fn vzo(close: &[f64], volume: &[f64], period: usize) -> Result<Array1<f64>> {
    let len = close.len();
    validate_same_length("close, volume", &[len, volume.len()])?;
    if period < 1 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "at least 1".to_string(),
        });
    }
    validate_input(len, period + 1)?;

    let mut vp = vec![0.0_f64; len];
    for i in 1..len {
        if close[i] > close[i - 1] {
            vp[i] = volume[i];
        } else if close[i] < close[i - 1] {
            vp[i] = -volume[i];
        }
    }

    let ema_vp = ema(&vp, period)?;
    let vol_f: Vec<f64> = volume.to_vec();
    let ema_tv = ema(&vol_f, period)?;

    let mut output = init_output(len);
    for i in 0..len {
        if !ema_vp[i].is_nan() && !ema_tv[i].is_nan() && ema_tv[i].abs() > 1e-15 {
            output[i] = (ema_vp[i] / ema_tv[i]) * 100.0;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod vzo_tests {
    use super::*;

    #[test]
    fn test_vzo_basic() {
        let n = 30;
        let close: Vec<f64> = (0..n).map(|i| 100.0 + i as f64 * 0.5).collect();
        let volume: Vec<f64> = (0..n).map(|i| 1000.0 + (i as f64 * 0.7).sin() * 200.0).collect();

        let result = vzo(&close, &volume, 14).unwrap();
        assert_eq!(result.len(), n);
        // In an uptrend, VZO should be positive
        for i in 14..n {
            if result[i].is_finite() {
                assert!(result[i] > 0.0, "Expected positive VZO in uptrend at {i}");
            }
        }
    }

    #[test]
    fn test_vzo_invalid() {
        let c = vec![1.0; 5];
        let v = vec![100.0; 3]; // mismatched
        assert!(vzo(&c, &v, 14).is_err());
    }

    #[test]
    fn test_vzo_insufficient_data() {
        let c = vec![1.0; 5];
        let v = vec![100.0; 5];
        assert!(vzo(&c, &v, 14).is_err());
    }
}

/// Volume Momentum = Volume - SMA(Volume, period)
///
/// Measures how current volume deviates from its moving average.
///
/// # Arguments
/// * `volume` - Volume data
/// * `period` - SMA lookback period
///
/// # Returns
/// Array of volume momentum values (NaN for warm-up period)
pub fn volume_momentum(volume: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_period("period", period)?;
    validate_input(volume.len(), period)?;

    let len = volume.len();
    let mut output = init_output(len);

    let mut sum: f64 = volume[..period].iter().sum();
    output[period - 1] = volume[period - 1] - sum / period as f64;

    for i in period..len {
        sum += volume[i] - volume[i - period];
        output[i] = volume[i] - sum / period as f64;
    }

    Ok(output)
}

/// Volume Rate of Change = (Volume - Volume[n]) / Volume[n] * 100
///
/// # Arguments
/// * `volume` - Volume data
/// * `period` - Lookback period
///
/// # Returns
/// Array of volume ROC values (NaN for warm-up period)
pub fn volume_roc(volume: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_period("period", period)?;
    validate_input(volume.len(), period + 1)?;

    let len = volume.len();
    let mut output = init_output(len);

    for i in period..len {
        let prev = volume[i - period];
        if prev.abs() > 1e-15 {
            output[i] = (volume[i] - prev) / prev * 100.0;
        } else {
            output[i] = 0.0;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod volume_momentum_tests {
    use super::*;

    #[test]
    fn test_volume_momentum_basic() {
        let volume = vec![100.0, 120.0, 110.0, 130.0, 140.0, 150.0, 160.0];
        let result = volume_momentum(&volume, 3).unwrap();
        assert_eq!(result.len(), 7);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        // period-1 = index 2: vol[2] - mean(vol[0..3])
        let sma2 = (100.0 + 120.0 + 110.0) / 3.0;
        assert!((result[2] - (110.0 - sma2)).abs() < 1e-10);
    }

    #[test]
    fn test_volume_momentum_invalid() {
        assert!(volume_momentum(&[1.0; 2], 5).is_err());
        assert!(volume_momentum(&[1.0; 5], 0).is_err());
    }

    #[test]
    fn test_volume_roc_basic() {
        let volume = vec![100.0, 200.0, 150.0, 300.0, 250.0];
        let result = volume_roc(&volume, 2).unwrap();
        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        // index 2: (150 - 100) / 100 * 100 = 50
        assert!((result[2] - 50.0).abs() < 1e-10);
        // index 3: (300 - 200) / 200 * 100 = 50
        assert!((result[3] - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_volume_roc_invalid() {
        assert!(volume_roc(&[1.0; 2], 5).is_err());
        assert!(volume_roc(&[1.0; 5], 0).is_err());
    }
}
