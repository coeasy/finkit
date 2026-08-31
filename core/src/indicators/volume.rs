use crate::error::{Result, TaError};
use crate::math::statistics::rolling_std_dev;
use crate::utils::{init_output, validate_input};
use ndarray::Array1;

/// VWAP Bands Result
#[derive(Debug, Clone)]
pub struct VwapBandsResult {
    /// VWAP line (center band)
    pub vwap: Array1<f64>,
    /// Upper band (VWAP + nb_dev * std_dev)
    pub upper: Array1<f64>,
    /// Lower band (VWAP - nb_dev * std_dev)
    pub lower: Array1<f64>,
}

/// Accumulation/Distribution Line (AD)
///
/// A cumulative indicator that uses volume and price to assess whether an asset is being accumulated or distributed.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
///
/// # Returns
/// Array of AD values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let volume = vec![100.0, 110.0, 120.0, 115.0, 130.0, 125.0, 105.0, 95.0, 110.0, 115.0];
/// let result = indicators::ad(&high, &low, &close, &volume).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn ad(high: &[f64], low: &[f64], close: &[f64], volume: &[f64]) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;

    let len = high.len();
    // 直接分配 Array1 并写入，避免中间 Vec 分配
    let mut output = Array1::<f64>::zeros(len);
    // SIMD-accelerated money-flow vectorisation + cumulative sum.
    // Replaces the per-bar scalar loop with an AVX2 block-wise kernel
    // (typically 1.5-2x speedup on 10K+ bars).
    crate::math::simd_ops::simd_ad_line(high, low, close, volume, output.as_slice_mut().unwrap());

    Ok(output)
}

/// AD zero-copy variant: writes result into pre-allocated slice.
///
/// Same semantics as [`ad`] but writes directly into the caller-provided buffer.
pub fn ad_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    output: &mut [f64],
) -> Result<()> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;
    if output.len() != high.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as high".to_string(),
        });
    }
    crate::math::simd_ops::simd_ad_line(high, low, close, volume, output);
    Ok(())
}

/// Chaikin A/D Oscillator (ADOSC)
///
/// Measures the momentum of the Accumulation/Distribution Line using two EMAs.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `fast_period` - Fast EMA period
/// * `slow_period` - Slow EMA period
///
/// # Returns
/// Array of ADOSC values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let volume = vec![100.0, 110.0, 120.0, 115.0, 130.0, 125.0, 105.0, 95.0, 110.0, 115.0];
/// let result = indicators::adosc(&high, &low, &close, &volume, 3, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn adosc(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    fast_period: usize,
    slow_period: usize,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), slow_period)?;

    let len = high.len();
    let mut output = Array1::<f64>::zeros(len);
    let fast_k = 2.0 / (fast_period as f64 + 1.0);
    let fast_one_k = 1.0 - fast_k;
    let slow_k = 2.0 / (slow_period as f64 + 1.0);
    let slow_one_k = 1.0 - slow_k;

    // Build the full AD line in a scratch buffer via the SIMD-accelerated
    // helper, then run the EMA pass on top. This is faster than the per-bar
    // scalar loop because the AVX2 money-flow + prefix-sum kernels replace
    // the inner division and accumulate with block-level parallelism.
    let mut cumulative = vec![0.0f64; len];
    crate::math::simd_ops::simd_ad_line(high, low, close, volume, &mut cumulative);

    let output_slice = output.as_slice_mut().unwrap();
    let mut fast_ema = 0.0;
    let mut slow_ema = 0.0;
    for i in 0..len {
        let c = cumulative[i];
        if i == 0 {
            fast_ema = c;
            slow_ema = c;
        } else {
            fast_ema = c * fast_k + fast_ema * fast_one_k;
            slow_ema = c * slow_k + slow_ema * slow_one_k;
        }
        if i >= slow_period - 1 {
            output_slice[i] = fast_ema - slow_ema;
        }
    }

    Ok(output)
}

/// On Balance Volume (OBV)
///
/// A cumulative indicator that uses volume flow to predict price changes.
///
/// # Arguments
/// * `close` - Close prices
/// * `volume` - Volume data
///
/// # Returns
/// Array of OBV values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let volume = vec![100.0, 110.0, 120.0, 115.0, 130.0, 125.0, 105.0, 95.0, 110.0, 115.0];
/// let result = indicators::obv(&close, &volume).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn obv(close: &[f64], volume: &[f64]) -> Result<Array1<f64>> {
    if close.len() != volume.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "close and volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(close.len(), 1)?;

    let len = close.len();
    let mut out = vec![0.0_f64; len];
    // SIMD-accelerated OBV: AVX2 kernel vectorises the diff/signum/mul chain.
    // Scalar fallback path is identical to the legacy implementation.
    crate::math::simd_ops::simd_obv(close, volume, &mut out);

    Ok(Array1::from(out))
}

/// Volume Profile 结果结构体
///
/// 包含 Volume Profile 分析的关键数据
#[derive(Debug, Clone)]
pub struct VolumeProfileResult {
    /// Point of Control - 成交量最大的价格水平
    pub poc: f64,
    /// Value Area High - 70%价值区域上界
    pub vah: f64,
    /// Value Area Low - 70%价值区域下界
    pub val: f64,
    /// 各bin的成交量分布
    pub profile: Vec<f64>,
    /// 各bin的中心价格
    pub bin_prices: Vec<f64>,
}

/// Volume Profile（成交量分布图）
///
/// 将价格范围分成多个bin，统计每个bin的成交量，用于识别关键的价格水平。
///
/// # Arguments
/// * `high` - 最高价数组
/// * `low` - 最低价数组
/// * `close` - 收盘价数组
/// * `volume` - 成交量数组
/// * `num_bins` - 价格分bin数量
///
/// # Returns
/// VolumeProfileResult 包含 POC、VAH、VAL 和完整的分布数据
///
/// # Errors
/// * 当输入数组长度不一致时返回错误
/// * 当 num_bins 为0时返回错误
/// * 当输入数据为空时返回错误
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let volume = vec![100.0, 110.0, 120.0, 115.0, 130.0, 125.0, 105.0, 95.0, 110.0, 115.0];
/// let result = indicators::volume_profile(&high, &low, &close, &volume, 5).unwrap();
/// assert_eq!(result.profile.len(), 5);
/// ```
pub fn volume_profile(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    num_bins: usize,
) -> Result<VolumeProfileResult> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;

    if num_bins == 0 {
        return Err(crate::error::TaError::InvalidParameter {
            name: "num_bins".to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }

    let len = high.len();
    let mut min_price = f64::INFINITY;
    let mut max_price = f64::NEG_INFINITY;

    for i in 0..len {
        min_price = min_price.min(low[i]);
        max_price = max_price.max(high[i]);
    }

    if (max_price - min_price).abs() < 1e-15 {
        max_price = min_price + 1.0;
    }

    let price_range = max_price - min_price;
    let bin_size = price_range / num_bins as f64;

    let mut profile = vec![0.0; num_bins];
    let mut bin_prices = Vec::with_capacity(num_bins);

    for i in 0..num_bins {
        let bin_center = min_price + (i as f64 + 0.5) * bin_size;
        bin_prices.push(bin_center);
    }

    for i in 0..len {
        let typical_price = (high[i] + low[i] + close[i]) / 3.0;
        let bin_index = ((typical_price - min_price) / bin_size) as usize;
        let bin_index = bin_index.min(num_bins - 1);
        profile[bin_index] += volume[i];
    }

    let total_volume: f64 = profile.iter().sum();
    let mut poc_index = 0;
    let mut max_volume = f64::NEG_INFINITY;

    for (i, &vol) in profile.iter().enumerate() {
        if vol > max_volume {
            max_volume = vol;
            poc_index = i;
        }
    }

    let poc = bin_prices[poc_index];

    let value_area_threshold = total_volume * 0.70;

    let mut vah_index = poc_index;
    let mut val_index = poc_index;
    let mut accumulated_volume = profile[poc_index];
    let mut upper_idx = poc_index;
    let mut lower_idx = poc_index;

    while accumulated_volume < value_area_threshold && (upper_idx > 0 || lower_idx < num_bins - 1) {
        let upper_volume = if upper_idx > 0 {
            profile[upper_idx - 1]
        } else {
            f64::NEG_INFINITY
        };
        let lower_volume = if lower_idx < num_bins - 1 {
            profile[lower_idx + 1]
        } else {
            f64::NEG_INFINITY
        };

        if upper_volume >= lower_volume {
            if upper_idx > 0 {
                upper_idx -= 1;
                accumulated_volume += profile[upper_idx];
            } else if lower_idx < num_bins - 1 {
                lower_idx += 1;
                accumulated_volume += profile[lower_idx];
            }
        } else {
            if lower_idx < num_bins - 1 {
                lower_idx += 1;
                accumulated_volume += profile[lower_idx];
            } else if upper_idx > 0 {
                upper_idx -= 1;
                accumulated_volume += profile[upper_idx];
            }
        }

        vah_index = upper_idx;
        val_index = lower_idx;
    }

    vah_index = std::cmp::min(vah_index, num_bins - 1);
    val_index = std::cmp::min(val_index, num_bins - 1);

    let vah = bin_prices[vah_index];
    let val = bin_prices[val_index];

    Ok(VolumeProfileResult {
        poc,
        vah,
        val,
        profile,
        bin_prices,
    })
}

/// Volume Weighted Average Price (VWAP)
///
/// A trading benchmark that represents the average price a security has traded at throughout the day,
/// based on both volume and price. It provides traders with insight into both the trend and value of a security.
///
/// # Formula
/// VWAP = Σ(Typical Price × Volume) / Σ(Volume)
/// where Typical Price = (High + Low + Close) / 3
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
///
/// # Returns
/// Array of VWAP values
///
/// # Example
/// ```rust
/// use finkit::indicators::vwap;
///
/// let high = vec![10.0, 11.0, 12.0, 13.0, 14.0];
/// let low = vec![9.0, 10.0, 11.0, 12.0, 13.0];
/// let close = vec![9.5, 10.5, 11.5, 12.5, 13.5];
/// let volume = vec![100.0, 200.0, 150.0, 300.0, 250.0];
/// let result = vwap(&high, &low, &close, &volume).unwrap();
/// ```
pub fn vwap(high: &[f64], low: &[f64], close: &[f64], volume: &[f64]) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;

    let len = high.len();
    let mut output = Array1::zeros(len);
    let mut cum_tp_vol = 0.0;
    let mut cum_volume = 0.0;

    for i in 0..len {
        let typical_price = (high[i] + low[i] + close[i]) / 3.0;
        cum_tp_vol += typical_price * volume[i];
        cum_volume += volume[i];

        if cum_volume.abs() > 1e-15 {
            output[i] = cum_tp_vol / cum_volume;
        }
    }

    Ok(output)
}

/// Anchored Volume Weighted Average Price (Anchored VWAP)
///
/// Similar to VWAP, but allows traders to specify a starting point (anchor) from which
/// the calculation begins. This is useful for measuring average price from significant
/// events like earnings releases, Fed announcements, or trend changes.
///
/// # Formula
/// Anchored VWAP = Σ(Typical Price × Volume) / Σ(Volume)
/// where the summation starts from start_index
/// Typical Price = (High + Low + Close) / 3
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `start_index` - The index from which to start calculating VWAP
///
/// # Returns
/// Array of Anchored VWAP values (NaN for indices before start_index)
///
/// # Example
/// ```rust
/// use finkit::indicators::anchored_vwap;
///
/// let high = vec![10.0, 11.0, 12.0, 13.0, 14.0];
/// let low = vec![9.0, 10.0, 11.0, 12.0, 13.0];
/// let close = vec![9.5, 10.5, 11.5, 12.5, 13.5];
/// let volume = vec![100.0, 200.0, 150.0, 300.0, 250.0];
/// let result = anchored_vwap(&high, &low, &close, &volume, 2).unwrap();
/// ```
pub fn anchored_vwap(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    start_index: usize,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if start_index >= high.len() {
        return Err(TaError::InvalidParameter {
            name: "start_index".to_string(),
            constraint: "must be less than input length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;

    let len = high.len();
    let mut output = init_output(len);
    let mut cum_tp_vol = 0.0;
    let mut cum_volume = 0.0;

    for i in start_index..len {
        let typical_price = (high[i] + low[i] + close[i]) / 3.0;
        cum_tp_vol += typical_price * volume[i];
        cum_volume += volume[i];

        if cum_volume.abs() > 1e-15 {
            output[i] = cum_tp_vol / cum_volume;
        }
    }

    Ok(output)
}

/// Volume Weighted Average Price Bands (VWAP Bands)
///
/// VWAP Bands consist of the VWAP line with upper and lower bands based on standard
/// deviation. These bands help identify overbought and oversold levels relative to
/// the volume-weighted average price.
///
/// # Formula
/// VWAP = Σ(Typical Price × Volume) / Σ(Volume)
/// Upper Band = VWAP + (nb_dev × Rolling StdDev of Typical Price)
/// Lower Band = VWAP - (nb_dev × Rolling StdDev of Typical Price)
/// where Typical Price = (High + Low + Close) / 3
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `timeperiod` - Lookback period for standard deviation calculation
/// * `nb_dev` - Number of standard deviations for the bands
///
/// # Returns
/// VwapBandsResult containing vwap, upper, and lower bands
///
/// # Example
/// ```rust
/// use finkit::indicators::vwap_bands;
///
/// let high = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
/// let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
/// let close = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];
/// let volume = vec![100.0, 200.0, 150.0, 300.0, 250.0, 180.0, 220.0, 280.0, 190.0, 310.0];
/// let result = vwap_bands(&high, &low, &close, &volume, 5, 2.0).unwrap();
/// ```
pub fn vwap_bands(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    timeperiod: usize,
    nb_dev: f64,
) -> Result<VwapBandsResult> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if timeperiod < 2 {
        return Err(TaError::InvalidParameter {
            name: "timeperiod".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    validate_input(high.len(), timeperiod)?;

    let len = high.len();

    let vwap_values = vwap(high, low, close, volume)?;

    let typical_prices: Vec<f64> = (0..len)
        .map(|i| (high[i] + low[i] + close[i]) / 3.0)
        .collect();

    let std_dev = rolling_std_dev(&typical_prices, timeperiod)?;

    let mut upper = init_output(len);
    let mut lower = init_output(len);

    for i in 0..len {
        if !vwap_values[i].is_nan() && !std_dev[i].is_nan() {
            upper[i] = vwap_values[i] + std_dev[i] * nb_dev;
            lower[i] = vwap_values[i] - std_dev[i] * nb_dev;
        }
    }

    Ok(VwapBandsResult {
        vwap: vwap_values,
        upper,
        lower,
    })
}

/// Multi-timeframe Volume Weighted Average Price (VWAP MTF)
///
/// Extends standard VWAP with session-based resets. At each bar where
/// `session_start[i]` is `true`, the cumulative sums are reset, effectively
/// anchoring VWAP to the session boundary (e.g. day start, week start, month start).
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `session_start` - Boolean array; `true` at each bar that begins a new session
///
/// # Returns
/// Array of VWAP values that reset at each session boundary
pub fn vwap_mtf(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    session_start: &[bool],
) -> Result<Array1<f64>> {
    let len = high.len();
    if len != low.len() || len != close.len() || len != volume.len() || len != session_start.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close, volume, session_start".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(len, 1)?;

    let mut output = Array1::zeros(len);
    let mut cum_tp_vol = 0.0;
    let mut cum_volume = 0.0;

    for i in 0..len {
        if session_start[i] {
            cum_tp_vol = 0.0;
            cum_volume = 0.0;
        }

        let typical_price = (high[i] + low[i] + close[i]) / 3.0;
        cum_tp_vol += typical_price * volume[i];
        cum_volume += volume[i];

        if cum_volume.abs() > 1e-15 {
            output[i] = cum_tp_vol / cum_volume;
        }
    }

    Ok(output)
}

/// OBV zero-copy variant: writes result into pre-allocated slice.
pub fn obv_into(close: &[f64], volume: &[f64], output: &mut [f64]) -> Result<()> {
    let result = obv(close, volume)?;
    if output.len() != close.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    output.copy_from_slice(result.as_slice().unwrap());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_ad() {
        let high = vec![10.0, 12.0, 14.0];
        let low = vec![8.0, 10.0, 12.0];
        let close = vec![9.0, 11.0, 13.0];
        let volume = vec![100.0, 200.0, 300.0];
        let result = ad(&high, &low, &close, &volume).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_adosc() {
        let high = vec![10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0];
        let low = vec![8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0];
        let close = vec![9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0, 23.0];
        let volume = vec![100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0];
        let result = adosc(&high, &low, &close, &volume, 3, 6).unwrap();
        assert!(result[0].is_nan() || result[7].is_finite());
    }

    #[test]
    fn test_ad_into_matches_ad() {
        let high = vec![10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
        let low = vec![8.0, 10.0, 12.0, 14.0, 16.0, 18.0];
        let close = vec![9.0, 11.0, 13.0, 15.0, 17.0, 19.0];
        let volume = vec![100.0, 200.0, 300.0, 400.0, 500.0, 600.0];
        let expected = ad(&high, &low, &close, &volume).unwrap();
        let mut output = vec![0.0; high.len()];
        ad_into(&high, &low, &close, &volume, &mut output).unwrap();
        for i in 0..high.len() {
            assert!((expected[i] - output[i]).abs() < 1e-9, "mismatch at {i}");
        }
    }

    #[test]
    fn test_obv() {
        let close = vec![10.0, 11.0, 10.0, 12.0];
        let volume = vec![100.0, 200.0, 150.0, 300.0];
        let result = obv(&close, &volume).unwrap();
        assert_relative_eq!(result[0], 100.0, epsilon = 1e-10);
        assert_relative_eq!(result[1], 300.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], 150.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 450.0, epsilon = 1e-10);
    }

    #[test]
    fn test_volume_profile_basic() {
        let high = vec![10.0, 12.0, 14.0, 12.0, 10.0];
        let low = vec![8.0, 10.0, 12.0, 10.0, 8.0];
        let close = vec![9.0, 11.0, 13.0, 11.0, 9.0];
        let volume = vec![100.0, 200.0, 300.0, 200.0, 100.0];
        let result = volume_profile(&high, &low, &close, &volume, 10).unwrap();
        assert!(result.poc.is_finite());
        assert!(result.vah.is_finite());
        assert!(result.val.is_finite());
        assert_eq!(result.profile.len(), 10);
        assert_eq!(result.bin_prices.len(), 10);
    }

    #[test]
    fn test_volume_profile_poc_identification() {
        let high = vec![15.0, 15.0, 15.0, 15.0, 15.0];
        let low = vec![5.0, 5.0, 5.0, 5.0, 5.0];
        let close = vec![10.0, 10.0, 10.0, 10.0, 10.0];
        let volume = vec![100.0, 200.0, 500.0, 200.0, 100.0];
        let result = volume_profile(&high, &low, &close, &volume, 5).unwrap();
        let poc_idx = result
            .profile
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap()
            .0;
        assert_relative_eq!(result.poc, result.bin_prices[poc_idx], epsilon = 1e-10);
    }

    #[test]
    fn test_volume_profile_single_price_level() {
        let high = vec![10.0, 10.0, 10.0];
        let low = vec![10.0, 10.0, 10.0];
        let close = vec![10.0, 10.0, 10.0];
        let volume = vec![100.0, 200.0, 300.0];
        let result = volume_profile(&high, &low, &close, &volume, 5).unwrap();
        assert!(result.poc.is_finite());
        assert!(result.profile.iter().sum::<f64>() > 0.0);
    }

    #[test]
    fn test_volume_profile_value_area() {
        let high = vec![20.0, 20.0, 20.0, 20.0, 20.0];
        let low = vec![10.0, 10.0, 10.0, 10.0, 10.0];
        let close = vec![15.0, 15.0, 15.0, 15.0, 15.0];
        let volume = vec![50.0, 100.0, 500.0, 100.0, 50.0];
        let result = volume_profile(&high, &low, &close, &volume, 10).unwrap();
        assert!(result.val <= result.vah);
        let total_volume: f64 = result.profile.iter().sum();
        let mut va_volume = 0.0;
        for (i, &vol) in result.profile.iter().enumerate() {
            let price = result.bin_prices[i];
            if price >= result.val && price <= result.vah {
                va_volume += vol;
            }
        }
        assert!(
            va_volume >= total_volume * 0.65,
            "VA volume should be around 70%"
        );
    }

    #[test]
    fn test_volume_profile_invalid_parameters() {
        let high = vec![10.0, 12.0];
        let low = vec![8.0, 10.0];
        let close = vec![9.0, 11.0];
        let volume = vec![100.0, 200.0];
        assert!(volume_profile(&high, &low, &close, &volume, 0).is_err());
        let short_volume = vec![100.0];
        assert!(volume_profile(&high, &low, &close, &short_volume, 5).is_err());
    }

    #[test]
    fn test_volume_profile_bin_prices_ordered() {
        let high = vec![100.0, 100.0, 100.0];
        let low = vec![0.0, 0.0, 0.0];
        let close = vec![50.0, 50.0, 50.0];
        let volume = vec![100.0, 200.0, 300.0];
        let result = volume_profile(&high, &low, &close, &volume, 10).unwrap();
        for i in 1..result.bin_prices.len() {
            assert!(result.bin_prices[i] > result.bin_prices[i - 1]);
        }
    }

    #[test]
    fn test_vwap_basic() {
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5];
        let volume = vec![100.0, 200.0, 150.0, 300.0, 250.0];
        let result = vwap(&high, &low, &close, &volume).unwrap();
        assert_eq!(result.len(), 5);
        assert!(result[0].is_finite());
        assert!(result[4].is_finite());
    }

    #[test]
    fn test_vwap_monotonic_price() {
        let high = vec![11.0, 12.0, 13.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![10.0, 11.0, 12.0];
        let volume = vec![100.0, 100.0, 100.0];
        let result = vwap(&high, &low, &close, &volume).unwrap();
        assert!(result[0] > 9.0 && result[0] < 11.0);
        assert!(result[1] > result[0]);
        assert!(result[2] > result[1]);
    }

    #[test]
    fn test_vwap_constant_price() {
        let high = vec![10.0, 10.0, 10.0];
        let low = vec![10.0, 10.0, 10.0];
        let close = vec![10.0, 10.0, 10.0];
        let volume = vec![100.0, 200.0, 300.0];
        let result = vwap(&high, &low, &close, &volume).unwrap();
        assert_relative_eq!(result[0], 10.0, epsilon = 1e-10);
        assert_relative_eq!(result[1], 10.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], 10.0, epsilon = 1e-10);
    }

    #[test]
    fn test_anchored_vwap_basic() {
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5];
        let volume = vec![100.0, 200.0, 150.0, 300.0, 250.0];
        let result = anchored_vwap(&high, &low, &close, &volume, 2).unwrap();
        assert_eq!(result.len(), 5);
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_finite());
        assert!(result[4].is_finite());
    }

    #[test]
    fn test_anchored_vwap_from_zero() {
        let high = vec![10.0, 11.0, 12.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![9.5, 10.5, 11.5];
        let volume = vec![100.0, 200.0, 150.0];
        let result = anchored_vwap(&high, &low, &close, &volume, 0).unwrap();
        let vwap_result = vwap(&high, &low, &close, &volume).unwrap();
        for i in 0..result.len() {
            assert_relative_eq!(result[i], vwap_result[i], epsilon = 1e-10);
        }
    }

    #[test]
    fn test_vwap_bands_basic() {
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];
        let volume = vec![
            100.0, 200.0, 150.0, 300.0, 250.0, 180.0, 220.0, 280.0, 190.0, 310.0,
        ];
        let result = vwap_bands(&high, &low, &close, &volume, 5, 2.0).unwrap();
        assert_eq!(result.vwap.len(), 10);
        assert_eq!(result.upper.len(), 10);
        assert_eq!(result.lower.len(), 10);
    }

    #[test]
    fn test_vwap_bands_structure() {
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];
        let volume = vec![
            100.0, 200.0, 150.0, 300.0, 250.0, 180.0, 220.0, 280.0, 190.0, 310.0,
        ];
        let result = vwap_bands(&high, &low, &close, &volume, 5, 2.0).unwrap();
        for i in 0..result.vwap.len() {
            if !result.upper[i].is_nan() {
                assert!(result.upper[i] > result.vwap[i]);
                assert!(result.lower[i] < result.vwap[i]);
                assert!(result.upper[i] > result.lower[i]);
            }
        }
    }

    #[test]
    fn test_vwap_invalid_inputs() {
        let high = vec![10.0, 11.0];
        let low = vec![9.0, 10.0];
        let close = vec![9.5, 10.5];
        let volume = vec![100.0, 200.0];
        let short_volume = vec![100.0];
        assert!(vwap(&high, &low, &close, &short_volume).is_err());
        assert!(anchored_vwap(&high, &low, &close, &volume, 5).is_err());
        assert!(vwap_bands(&high, &low, &close, &volume, 1, 2.0).is_err());
    }

    #[test]
    fn test_vwap_bands_invalid_period() {
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5, 15.5, 16.5, 17.5, 18.5];
        let volume = vec![
            100.0, 200.0, 150.0, 300.0, 250.0, 180.0, 220.0, 280.0, 190.0, 310.0,
        ];
        assert!(vwap_bands(&high, &low, &close, &volume, 0, 2.0).is_err());
        assert!(vwap_bands(&high, &low, &close, &volume, 1, 2.0).is_err());
    }

    #[test]
    fn test_vwap_large_dataset() {
        let n = 100;
        let high: Vec<f64> = (0..n).map(|i| 100.0 + i as f64 + 1.0).collect();
        let low: Vec<f64> = (0..n).map(|i| 100.0 + i as f64 - 1.0).collect();
        let close: Vec<f64> = (0..n).map(|i| 100.0 + i as f64).collect();
        let volume: Vec<f64> = (0..n).map(|_| 1000.0).collect();
        let result = vwap(&high, &low, &close, &volume).unwrap();
        assert_eq!(result.len(), n);
        for i in 0..n {
            assert!(result[i].is_finite());
            let expected = (0..=i).map(|j| 100.0 + j as f64).sum::<f64>() / (i + 1) as f64;
            assert_relative_eq!(result[i], expected, epsilon = 1e-6);
        }
    }

    #[test]
    fn test_vwap_mtf_basic() {
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5];
        let volume = vec![100.0, 200.0, 150.0, 300.0, 250.0];
        // Session starts at index 0 and 3
        let session_start = vec![true, false, false, true, false];
        let result = vwap_mtf(&high, &low, &close, &volume, &session_start).unwrap();
        assert_eq!(result.len(), 5);

        // First session: indices 0..3
        let vwap_full = vwap(&high, &low, &close, &volume).unwrap();
        // First 3 bars should match normal VWAP from start
        for i in 0..3 {
            assert_relative_eq!(result[i], vwap_full[i], epsilon = 1e-10);
        }

        // Second session starts at index 3, so result[3] should be tp[3]
        let tp3 = (high[3] + low[3] + close[3]) / 3.0;
        assert_relative_eq!(result[3], tp3, epsilon = 1e-10);
    }

    #[test]
    fn test_vwap_mtf_no_reset() {
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5];
        let volume = vec![100.0, 200.0, 150.0, 300.0, 250.0];
        // Only one session (all bars same session)
        let session_start = vec![true, false, false, false, false];
        let result = vwap_mtf(&high, &low, &close, &volume, &session_start).unwrap();
        let vwap_full = vwap(&high, &low, &close, &volume).unwrap();
        for i in 0..5 {
            assert_relative_eq!(result[i], vwap_full[i], epsilon = 1e-10);
        }
    }

    #[test]
    fn test_vwap_mtf_invalid_length() {
        let high = vec![10.0, 11.0];
        let low = vec![9.0, 10.0];
        let close = vec![9.5, 10.5];
        let volume = vec![100.0, 200.0];
        let session_start = vec![true]; // mismatched length
        assert!(vwap_mtf(&high, &low, &close, &volume, &session_start).is_err());
    }
}
