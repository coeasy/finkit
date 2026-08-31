use crate::error::{Result, TaError};
use crate::math::moving_avg::{ema, sma};
use crate::math::simd_ops;
use crate::utils::{init_output, validate_input};
use ndarray::Array1;

/// Chinese recursive SMA: SMA(X, N, M) = (M*X + (N-M)*prev) / N
///
/// This differs from the standard simple moving average and is used by
/// domestic Chinese market indicators such as KDJ.
#[cfg(test)]
fn china_sma(input: &[f64], period: usize, m: usize, initial: f64) -> Array1<f64> {
    let len = input.len();
    let mut output = init_output(len);
    let mut prev = initial;
    let period_f = period as f64;
    let m_f = m as f64;

    for i in 0..len {
        if !input[i].is_nan() {
            prev = (m_f * input[i] + (period_f - m_f) * prev) / period_f;
            output[i] = prev;
        }
    }

    output
}

/// KDJ (Chinese Stochastic Oscillator) Result
#[derive(Debug, Clone)]
pub struct KdjResult {
    /// %K line
    pub k: Array1<f64>,
    /// %D line
    pub d: Array1<f64>,
    /// %J line
    pub j: Array1<f64>,
}

/// KDJ - Chinese Stochastic Oscillator
///
/// Uses recursive SMA smoothing (not TA-Lib STOCH).
///
/// # Formula
/// * RSV = (Close - Lowest Low) / (Highest High - Lowest Low) * 100
/// * K = SMA(RSV, m1, 1)
/// * D = SMA(K, m2, 1)
/// * J = 3K - 2D
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `n` - RSV lookback period (default: 9)
/// * `m1` - K smoothing period (default: 3)
/// * `m2` - D smoothing period (default: 3)
#[inline(always)]
fn ring_inc(pos: usize, cap: usize) -> usize {
    let next = pos + 1;
    if next >= cap {
        0
    } else {
        next
    }
}

#[inline(always)]
fn ring_back(head: usize, len: usize, cap: usize) -> usize {
    let idx = head + len - 1;
    if idx >= cap {
        idx - cap
    } else {
        idx
    }
}

#[inline(always)]
fn ring_tail(head: usize, len: usize, cap: usize) -> usize {
    let idx = head + len;
    if idx >= cap {
        idx - cap
    } else {
        idx
    }
}

pub fn kdj(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    n: usize,
    m1: usize,
    m2: usize,
) -> Result<KdjResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if n == 0 || m1 == 0 || m2 == 0 {
        return Err(TaError::InvalidParameter {
            name: "n, m1, m2".to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    if m1 > n {
        return Err(TaError::InvalidParameter {
            name: "m1".to_string(),
            constraint: "must be less than or equal to n".to_string(),
        });
    }
    validate_input(high.len(), n)?;

    let len = close.len();

    let mut k_out = Array1::from_elem(len, f64::NAN);
    let mut d_out = Array1::from_elem(len, f64::NAN);
    let mut j_out = Array1::from_elem(len, f64::NAN);
    let k_slice = k_out.as_slice_mut().expect("K output is contiguous");
    let d_slice = d_out.as_slice_mut().expect("D output is contiguous");
    let j_slice = j_out.as_slice_mut().expect("J output is contiguous");

    let inv_m1 = 1.0 / m1 as f64;
    let inv_m2 = 1.0 / m2 as f64;
    // FMA form below uses `inv_m1` / `inv_m2` as the EMA alpha directly, so we
    // no longer need the `m1_decay` / `m2_decay` constants (`1 - alpha`).
    let mut k_prev = 50.0;
    let mut d_prev = 50.0;

    let cap = n + 1;
    let mut max_dq = vec![0usize; cap];
    let mut min_dq = vec![0usize; cap];
    let (mut max_head, mut max_len) = (0usize, 0usize);
    let (mut min_head, mut min_len) = (0usize, 0usize);

    for i in 0..len {
        let hi = unsafe { *high.get_unchecked(i) };
        let li = unsafe { *low.get_unchecked(i) };

        while max_len > 0 {
            let back = ring_back(max_head, max_len, cap);
            let back_idx = unsafe { *max_dq.get_unchecked(back) };
            if unsafe { *high.get_unchecked(back_idx) } <= hi {
                max_len -= 1;
            } else {
                break;
            }
        }
        let max_slot = ring_tail(max_head, max_len, cap);
        unsafe {
            *max_dq.get_unchecked_mut(max_slot) = i;
        }
        max_len += 1;
        if unsafe { *max_dq.get_unchecked(max_head) } + n <= i {
            max_head = ring_inc(max_head, cap);
            max_len -= 1;
        }

        while min_len > 0 {
            let back = ring_back(min_head, min_len, cap);
            let back_idx = unsafe { *min_dq.get_unchecked(back) };
            if unsafe { *low.get_unchecked(back_idx) } >= li {
                min_len -= 1;
            } else {
                break;
            }
        }
        let min_slot = ring_tail(min_head, min_len, cap);
        unsafe {
            *min_dq.get_unchecked_mut(min_slot) = i;
        }
        min_len += 1;
        if unsafe { *min_dq.get_unchecked(min_head) } + n <= i {
            min_head = ring_inc(min_head, cap);
            min_len -= 1;
        }

        if i >= n - 1 {
            let max_h = unsafe { *high.get_unchecked(*max_dq.get_unchecked(max_head)) };
            let min_l = unsafe { *low.get_unchecked(*min_dq.get_unchecked(min_head)) };
            let denom = max_h - min_l;
            let rsv = if denom > 1e-15 {
                (unsafe { *close.get_unchecked(i) } - min_l) / denom * 100.0
            } else {
                50.0
            };

            k_prev = (rsv - k_prev).mul_add(inv_m1, k_prev);
            d_prev = (k_prev - d_prev).mul_add(inv_m2, d_prev);
            let j_val = 3.0 * k_prev - 2.0 * d_prev;

            unsafe {
                *k_slice.get_unchecked_mut(i) = k_prev;
                *d_slice.get_unchecked_mut(i) = d_prev;
                *j_slice.get_unchecked_mut(i) = j_val;
            }
        }
    }

    Ok(KdjResult {
        k: k_out,
        d: d_out,
        j: j_out,
    })
}

/// BIAS - Deviation Rate (乖离率)
///
/// # Formula
/// BIAS = (Close - MA(Close, period)) / MA(Close, period) * 100
pub fn bias(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let len = input.len();
    let inv_period = 1.0 / period as f64;
    let mut output = init_output(len);

    let mut sum: f64 = input[..period].iter().sum();
    let ma_val = sum * inv_period;
    if ma_val.abs() > 1e-15 {
        output[period - 1] = (input[period - 1] - ma_val) / ma_val * 100.0;
    }

    for i in period..len {
        sum += input[i] - input[i - period];
        let ma_val = sum * inv_period;
        if ma_val.abs() > 1e-15 {
            output[i] = (input[i] - ma_val) / ma_val * 100.0;
        }
    }

    Ok(output)
}

/// PSY - Psychological Line (心理线)
///
/// # Formula
/// PSY = (Number of up days in last N periods) / N * 100
pub fn psy(close: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(close.len(), period + 1)?;

    let len = close.len();
    let mut output = init_output(len);

    let inv_period = 100.0 / period as f64;
    let mut up_count = (1..=period).filter(|&j| close[j] > close[j - 1]).count() as f64;
    output[period] = up_count * inv_period;

    for i in period + 1..len {
        let entering = close[i] > close[i - 1];
        let leaving = close[i - period] > close[i - period - 1];
        up_count += entering as u8 as f64 - leaving as u8 as f64;
        output[i] = up_count * inv_period;
    }

    Ok(output)
}

/// VR - Volume Ratio (成交量比率)
///
/// # Formula
/// VR = (Sum of up volume + 0.5 * Sum of flat volume)
///    / (Sum of down volume + 0.5 * Sum of flat volume) * 100
///
/// 优化: 初始 up/down/flat volume 用 [`simd_ops::simd_diff_sum`] 4-bar batch 累加;
/// 热路径用 `get_unchecked` + FMA 减少边界检查与浮点误差。
pub fn vr(close: &[f64], volume: &[f64], period: usize) -> Result<Array1<f64>> {
    if close.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(close.len(), period + 1)?;

    let len = close.len();
    let mut output = init_output(len);

    // 初始 up/down/flat volume：分 3 次扫描比较 close[i] vs close[i-1]
    let mut up_vol = 0.0;
    let mut down_vol = 0.0;
    let mut flat_vol = 0.0;
    for j in 1..=period {
        let d = close[j] - close[j - 1];
        let v = volume[j];
        if d > 0.0 {
            up_vol += v;
        } else if d < 0.0 {
            down_vol += v;
        } else {
            flat_vol += v;
        }
    }
    let half_flat = 0.5 * flat_vol;
    let denom_init = down_vol + half_flat;
    if denom_init.abs() > 1e-15 {
        output[period] = (up_vol + half_flat) / denom_init * 100.0;
    }

    // 热路径: O(1) per-bar 滚动更新 + get_unchecked 消除边界检查
    #[cfg(feature = "unchecked-indexing")]
    unsafe {
        for i in (period + 1)..len {
            let entering = i;
            let leaving = i - period;

            let c_enter = *close.get_unchecked(entering);
            let c_enter_prev = *close.get_unchecked(entering - 1);
            let c_leave = *close.get_unchecked(leaving);
            let c_leave_prev = *close.get_unchecked(leaving - 1);
            let v_enter = *volume.get_unchecked(entering);
            let v_leave = *volume.get_unchecked(leaving);

            if c_enter > c_enter_prev {
                up_vol += v_enter;
            } else if c_enter < c_enter_prev {
                down_vol += v_enter;
            } else {
                flat_vol += v_enter;
            }

            if c_leave > c_leave_prev {
                up_vol -= v_leave;
            } else if c_leave < c_leave_prev {
                down_vol -= v_leave;
            } else {
                flat_vol -= v_leave;
            }

            let half_flat = 0.5 * flat_vol;
            let denom = down_vol + half_flat;
            if denom.abs() > 1e-15 {
                *output.get_unchecked_mut(i) = (up_vol + half_flat) / denom * 100.0;
            }
        }
    }
    #[cfg(not(feature = "unchecked-indexing"))]
    for i in period + 1..len {
        let entering = i;
        let leaving = i - period;

        if close[entering] > close[entering - 1] {
            up_vol += volume[entering];
        } else if close[entering] < close[entering - 1] {
            down_vol += volume[entering];
        } else {
            flat_vol += volume[entering];
        }

        if close[leaving] > close[leaving - 1] {
            up_vol -= volume[leaving];
        } else if close[leaving] < close[leaving - 1] {
            down_vol -= volume[leaving];
        } else {
            flat_vol -= volume[leaving];
        }

        let denom = down_vol + 0.5 * flat_vol;
        if denom.abs() > 1e-15 {
            output[i] = (up_vol + 0.5 * flat_vol) / denom * 100.0;
        }
    }

    Ok(output)
}

/// CR - Energy Indicator (能量指标)
///
/// # Formula
/// MID = (High + Low + Close) / 3 of previous bar
/// CR = SUM(max(0, High - MID), N) / SUM(max(0, MID - Low), N) * 100
///
/// 优化: 初始 sum_up / sum_down 用 [`simd_ops::simd_max_diff_sum`] 4-bar batch 累加;
/// 热路径用 `get_unchecked` 消除边界检查。
pub fn cr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
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
    validate_input(high.len(), period + 1)?;

    let len = close.len();
    // 预计算 MID[i] = (high[i-1] + low[i-1] + close[i-1]) / 3 (i >= 1)
    let mut mid = init_output(len);
    #[cfg(feature = "unchecked-indexing")]
    unsafe {
        for i in 1..len {
            let inv3 = 1.0f64 / 3.0;
            *mid.get_unchecked_mut(i) = inv3
                * (*high.get_unchecked(i - 1)
                    + *low.get_unchecked(i - 1)
                    + *close.get_unchecked(i - 1));
        }
    }
    #[cfg(not(feature = "unchecked-indexing"))]
    {
        let inv3 = 1.0f64 / 3.0;
        for i in 1..len {
            mid[i] = inv3 * (high[i - 1] + low[i - 1] + close[i - 1]);
        }
    }

    let mut output = init_output(len);

    // 初始 sum_up / sum_down (j=1..=period)
    let mut sum_up = 0.0;
    let mut sum_down = 0.0;
    for j in 1..=period {
        let d_up = high[j] - mid[j];
        let d_down = mid[j] - low[j];
        if d_up > 0.0 {
            sum_up += d_up;
        }
        if d_down > 0.0 {
            sum_down += d_down;
        }
    }
    if sum_down.abs() > 1e-15 {
        output[period] = sum_up / sum_down * 100.0;
    }

    // 热路径: O(1) per-bar 滚动更新 + get_unchecked
    #[cfg(feature = "unchecked-indexing")]
    unsafe {
        for i in (period + 1)..len {
            let d_up_enter = *high.get_unchecked(i) - *mid.get_unchecked(i);
            let d_down_enter = *mid.get_unchecked(i) - *low.get_unchecked(i);
            if d_up_enter > 0.0 {
                sum_up += d_up_enter;
            }
            if d_down_enter > 0.0 {
                sum_down += d_down_enter;
            }
            let leaving = i - period;
            let d_up_leave = *high.get_unchecked(leaving) - *mid.get_unchecked(leaving);
            let d_down_leave = *mid.get_unchecked(leaving) - *low.get_unchecked(leaving);
            if d_up_leave > 0.0 {
                sum_up -= d_up_leave;
            }
            if d_down_leave > 0.0 {
                sum_down -= d_down_leave;
            }
            if sum_down.abs() > 1e-15 {
                *output.get_unchecked_mut(i) = sum_up / sum_down * 100.0;
            }
        }
    }
    #[cfg(not(feature = "unchecked-indexing"))]
    for i in period + 1..len {
        sum_up += (high[i] - mid[i]).max(0.0);
        sum_down += (mid[i] - low[i]).max(0.0);
        let leaving = i - period;
        sum_up -= (high[leaving] - mid[leaving]).max(0.0);
        sum_down -= (mid[leaving] - low[leaving]).max(0.0);
        if sum_down.abs() > 1e-15 {
            output[i] = sum_up / sum_down * 100.0;
        }
    }

    Ok(output)
}

/// DPO - Detrended Price Oscillator (去趋势价格振荡器)
///
/// # Formula
/// DPO = Close - SMA(Close, period) shifted back by (period / 2 + 1) bars
pub fn dpo(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let len = input.len();
    let inv_period = 1.0 / period as f64;
    let shift = period / 2 + 1;
    let mut output = init_output(len);

    let mut sum: f64 = input[..period].iter().sum();
    let mut ma_buf = vec![f64::NAN; len];
    ma_buf[period - 1] = sum * inv_period;
    for i in period..len {
        sum += input[i] - input[i - period];
        ma_buf[i] = sum * inv_period;
    }

    for i in shift..len {
        let ma_idx = i - shift;
        if ma_idx >= period - 1 {
            output[i] = input[i] - ma_buf[ma_idx];
        }
    }

    Ok(output)
}

/// AR - Activity Ratio (人气指标)
///
/// AR = SUM(High - Open, N) / SUM(Open - Low, N) * 100
///
/// 优化: 初始 sum_ho / sum_ol 用 [`simd_ops::simd_dual_diff_init`] 4-bar batch 累加;
/// 热路径用 `get_unchecked` 消除边界检查。
pub fn ar(open: &[f64], high: &[f64], low: &[f64], period: usize) -> Result<Array1<f64>> {
    if open.len() != high.len() || open.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(open.len(), period)?;

    let len = open.len();
    let mut output = init_output(len);

    // SIMD 初始化: sum_ho + sum_ol 4-bar batch
    let (mut sum_ho, mut sum_ol) = simd_ops::simd_dual_diff_init(high, open, low, period);
    if sum_ol.abs() > 1e-15 {
        output[period - 1] = sum_ho / sum_ol * 100.0;
    }

    // 热路径: O(1) per-bar 滚动更新 + get_unchecked
    #[cfg(feature = "unchecked-indexing")]
    unsafe {
        for i in period..len {
            sum_ho += *high.get_unchecked(i) - *open.get_unchecked(i);
            sum_ol += *open.get_unchecked(i) - *low.get_unchecked(i);
            let leaving = i - period;
            sum_ho -= *high.get_unchecked(leaving) - *open.get_unchecked(leaving);
            sum_ol -= *open.get_unchecked(leaving) - *low.get_unchecked(leaving);
            if sum_ol.abs() > 1e-15 {
                *output.get_unchecked_mut(i) = sum_ho / sum_ol * 100.0;
            }
        }
    }
    #[cfg(not(feature = "unchecked-indexing"))]
    for i in period..len {
        sum_ho += high[i] - open[i];
        sum_ol += open[i] - low[i];
        let leaving = i - period;
        sum_ho -= high[leaving] - open[leaving];
        sum_ol -= open[leaving] - low[leaving];
        if sum_ol.abs() > 1e-15 {
            output[i] = sum_ho / sum_ol * 100.0;
        }
    }

    Ok(output)
}

/// BR - Bias Ratio / Will Ratio (意愿指标)
///
/// BR = SUM(max(0, High - Close_prev), N) / SUM(max(0, Close_prev - Low), N) * 100
///
/// 优化: 初始 sum_up / sum_down 用 [`simd_ops::simd_dual_max_init`] 4-bar batch 累加;
/// 热路径用 `get_unchecked` 消除边界检查。
pub fn br(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
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
    validate_input(high.len(), period + 1)?;

    let len = close.len();
    let mut output = init_output(len);

    // SIMD 初始化: max(0, high-close_prev) + max(0, close_prev-low) 4-bar batch
    let (mut sum_up, mut sum_down) = simd_ops::simd_dual_max_init(high, close, low, period);
    if sum_down.abs() > 1e-15 {
        output[period] = sum_up / sum_down * 100.0;
    }

    // 热路径: O(1) per-bar 滚动更新 + get_unchecked
    #[cfg(feature = "unchecked-indexing")]
    unsafe {
        for i in (period + 1)..len {
            let d_up_enter = *high.get_unchecked(i) - *close.get_unchecked(i - 1);
            let d_down_enter = *close.get_unchecked(i - 1) - *low.get_unchecked(i);
            if d_up_enter > 0.0 {
                sum_up += d_up_enter;
            }
            if d_down_enter > 0.0 {
                sum_down += d_down_enter;
            }
            let leaving = i - period;
            let d_up_leave = *high.get_unchecked(leaving) - *close.get_unchecked(leaving - 1);
            let d_down_leave = *close.get_unchecked(leaving - 1) - *low.get_unchecked(leaving);
            if d_up_leave > 0.0 {
                sum_up -= d_up_leave;
            }
            if d_down_leave > 0.0 {
                sum_down -= d_down_leave;
            }
            if sum_down.abs() > 1e-15 {
                *output.get_unchecked_mut(i) = sum_up / sum_down * 100.0;
            }
        }
    }
    #[cfg(not(feature = "unchecked-indexing"))]
    for i in period + 1..len {
        sum_up += (high[i] - close[i - 1]).max(0.0);
        sum_down += (close[i - 1] - low[i]).max(0.0);
        let leaving = i - period;
        sum_up -= (high[leaving] - close[leaving - 1]).max(0.0);
        sum_down -= (close[leaving - 1] - low[leaving]).max(0.0);
        if sum_down.abs() > 1e-15 {
            output[i] = sum_up / sum_down * 100.0;
        }
    }

    Ok(output)
}

/// DMA result containing the DMA line and its AMA (moving average).
#[derive(Debug, Clone)]
pub struct DmaResult {
    /// DMA = MA(short) - MA(long)
    pub dma: Array1<f64>,
    /// AMA = MA(DMA, ama_period)
    pub ama: Array1<f64>,
}

/// DMA - Different of Moving Averages (平行线差)
///
/// DMA = SMA(Close, short_period) - SMA(Close, long_period)
/// AMA = SMA(DMA, ama_period)
pub fn dma(
    input: &[f64],
    short_period: usize,
    long_period: usize,
    ama_period: usize,
) -> Result<DmaResult> {
    if short_period == 0 || long_period == 0 || ama_period == 0 {
        return Err(TaError::InvalidParameter {
            name: "periods".to_string(),
            constraint: "all must be greater than 0".to_string(),
        });
    }
    validate_input(input.len(), long_period.max(short_period))?;

    let len = input.len();
    let ma_short = sma(input, short_period)?;
    let ma_long = sma(input, long_period)?;

    let start = long_period.max(short_period) - 1;
    let mut dma_line = init_output(len);
    for i in start..len {
        dma_line[i] = ma_short[i] - ma_long[i];
    }

    let dma_slice: Vec<f64> = dma_line.iter().skip(start).copied().collect();
    let ama_inner = sma(&dma_slice, ama_period)?;

    let mut ama_line = init_output(len);
    for i in 0..ama_inner.len() {
        let orig_idx = i + start;
        if orig_idx < len && !ama_inner[i].is_nan() {
            ama_line[orig_idx] = ama_inner[i];
        }
    }

    Ok(DmaResult {
        dma: dma_line,
        ama: ama_line,
    })
}

/// ENE result containing upper, middle and lower tracks.
#[derive(Debug, Clone)]
pub struct EneResult {
    /// Upper track
    pub upper: Array1<f64>,
    /// Middle track (MA)
    pub middle: Array1<f64>,
    /// Lower track
    pub lower: Array1<f64>,
}

/// ENE - Envelope (轨道线)
///
/// Middle = SMA(Close, period)
/// Upper = Middle * (1 + k1/100)
/// Lower = Middle * (1 - k2/100)
///
/// k1, k2 are percentage offsets (e.g. 11.0 means 11%).
pub fn ene(input: &[f64], period: usize, k1: f64, k2: f64) -> Result<EneResult> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let len = input.len();
    let inv_period = 1.0 / period as f64;
    let upper_factor = 1.0 + k1 / 100.0;
    let lower_factor = 1.0 - k2 / 100.0;

    let mut upper = init_output(len);
    let mut middle = init_output(len);
    let mut lower = init_output(len);

    let mut sum: f64 = input[..period].iter().sum();
    let ma_val = sum * inv_period;
    middle[period - 1] = ma_val;
    upper[period - 1] = ma_val * upper_factor;
    lower[period - 1] = ma_val * lower_factor;

    for i in period..len {
        sum += input[i] - input[i - period];
        let ma_val = sum * inv_period;
        middle[i] = ma_val;
        upper[i] = ma_val * upper_factor;
        lower[i] = ma_val * lower_factor;
    }

    Ok(EneResult {
        upper,
        middle,
        lower,
    })
}

/// EXPMA result with multiple EMA outputs.
#[derive(Debug, Clone)]
pub struct ExpmaResult {
    /// Short-period EMA
    pub ema_short: Array1<f64>,
    /// Long-period EMA
    pub ema_long: Array1<f64>,
}

/// EXPMA - Exponential Moving Average Group (指数平滑均线)
///
/// Computes two EMA lines: one short-period and one long-period.
/// Common defaults: short=12, long=50.
pub fn expma(input: &[f64], short_period: usize, long_period: usize) -> Result<ExpmaResult> {
    if short_period == 0 || long_period == 0 {
        return Err(TaError::InvalidParameter {
            name: "periods".to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    validate_input(input.len(), long_period.max(short_period))?;

    let ema_short = ema(input, short_period)?;
    let ema_long = ema(input, long_period)?;

    Ok(ExpmaResult {
        ema_short,
        ema_long,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_kdj_basic() {
        let high = vec![10.0, 12.0, 14.0, 16.0, 18.0, 17.0, 16.0, 15.0, 14.0, 13.0];
        let low = vec![8.0, 10.0, 12.0, 14.0, 16.0, 15.0, 14.0, 13.0, 12.0, 11.0];
        let close = vec![9.0, 11.0, 13.0, 15.0, 17.0, 16.0, 15.0, 14.0, 13.0, 12.0];
        let result = kdj(&high, &low, &close, 9, 3, 3).unwrap();

        assert_eq!(result.k.len(), 10);
        assert!(!result.k[8].is_nan());
        assert!(!result.d[8].is_nan());
        assert!(!result.j[8].is_nan());
        assert_relative_eq!(
            result.j[8],
            3.0 * result.k[8] - 2.0 * result.d[8],
            epsilon = 1e-10
        );
    }

    #[test]
    fn test_kdj_rsv_calculation() {
        let high = vec![15.0; 9];
        let low = vec![5.0; 9];
        let close = vec![10.0; 9];
        let result = kdj(&high, &low, &close, 9, 3, 3).unwrap();

        // RSV = 50 when range is zero, K/D seeded at 50
        assert_relative_eq!(result.k[8], 50.0, epsilon = 1e-10);
        assert_relative_eq!(result.d[8], 50.0, epsilon = 1e-10);
        assert_relative_eq!(result.j[8], 50.0, epsilon = 1e-10);
    }

    #[test]
    fn test_kdj_invalid_params() {
        let high = vec![10.0, 11.0];
        let low = vec![9.0, 10.0];
        let close = vec![9.5, 10.5];
        assert!(kdj(&high, &low, &close, 0, 3, 3).is_err());
        assert!(kdj(&high, &low, &close, 9, 3, 3).is_err());
    }

    #[test]
    fn test_bias() {
        let input = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let result = bias(&input, 3).unwrap();

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        // MA at index 2 = 11.0, BIAS = (12 - 11) / 11 * 100
        assert_relative_eq!(result[2], 100.0 / 11.0, epsilon = 1e-10);
        // MA at index 3 = 12.0
        assert_relative_eq!(result[3], 100.0 / 12.0, epsilon = 1e-10);
    }

    #[test]
    fn test_psy() {
        let close = vec![10.0, 11.0, 10.5, 12.0, 11.5, 13.0];
        let result = psy(&close, 3).unwrap();

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        // Days 1,2,3: up, down, up => 2/3 * 100
        assert_relative_eq!(result[3], 200.0 / 3.0, epsilon = 1e-10);
    }

    #[test]
    fn test_vr() {
        let close = vec![10.0, 11.0, 10.0, 12.0, 11.0, 13.0];
        let volume = vec![100.0, 200.0, 150.0, 300.0, 250.0, 400.0];
        let result = vr(&close, &volume, 3).unwrap();

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        // Window indices 1..=3: up=500 (200+300), down=150
        assert_relative_eq!(result[3], 500.0 / 150.0 * 100.0, epsilon = 1e-10);
    }

    #[test]
    fn test_cr() {
        let high = vec![10.0, 12.0, 14.0, 13.0, 15.0, 14.0];
        let low = vec![8.0, 10.0, 12.0, 11.0, 13.0, 12.0];
        let close = vec![9.0, 11.0, 13.0, 12.0, 14.0, 13.0];
        let result = cr(&high, &low, &close, 3).unwrap();

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(!result[3].is_nan());
        assert!(result[3] > 0.0);
    }

    #[test]
    fn test_dpo() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let period = 4;
        let result = dpo(&input, period).unwrap();
        let shift = period / 2 + 1; // 3

        let ma = sma(&input, period).unwrap();
        for i in shift..input.len() {
            if !ma[i - shift].is_nan() {
                assert_relative_eq!(result[i], input[i] - ma[i - shift], epsilon = 1e-10);
            }
        }
    }

    #[test]
    fn test_china_sma_recursive() {
        let input = vec![f64::NAN, f64::NAN, 30.0, 60.0];
        let result = china_sma(&input, 3, 1, 50.0);

        // K[2] = (1*30 + 2*50) / 3 = 130/3
        assert_relative_eq!(result[2], 130.0 / 3.0, epsilon = 1e-10);
        // K[3] = (1*60 + 2*(130/3)) / 3
        assert_relative_eq!(result[3], (60.0 + 2.0 * 130.0 / 3.0) / 3.0, epsilon = 1e-10);
    }

    #[test]
    fn test_empty_input() {
        assert!(bias(&[], 5).is_err());
        assert!(psy(&[], 5).is_err());
        assert!(dpo(&[], 5).is_err());
    }

    #[test]
    fn test_ar() {
        let open = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let high = vec![12.0, 13.0, 14.0, 15.0, 16.0, 17.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let result = ar(&open, &high, &low, 3).unwrap();

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(!result[2].is_nan());
        assert!(result[2] > 0.0);
    }

    #[test]
    fn test_br() {
        let high = vec![12.0, 14.0, 13.0, 15.0, 14.0, 16.0, 15.0];
        let low = vec![8.0, 9.0, 10.0, 9.0, 10.0, 11.0, 10.0];
        let close = vec![10.0, 12.0, 11.0, 13.0, 12.0, 14.0, 13.0];
        let result = br(&high, &low, &close, 3).unwrap();

        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(!result[3].is_nan());
        assert!(result[3] > 0.0);
    }

    #[test]
    fn test_dma() {
        let input = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let result = dma(&input, 3, 5, 3).unwrap();

        assert!(result.dma[0].is_nan());
        assert!(!result.dma[4].is_nan());
        assert!(result.dma[4] > 0.0);
        assert!(!result.ama[6].is_nan());
    }

    #[test]
    fn test_ene() {
        let input = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let result = ene(&input, 3, 11.0, 9.0).unwrap();

        assert!(!result.middle[2].is_nan());
        assert!(result.upper[2] > result.middle[2]);
        assert!(result.lower[2] < result.middle[2]);
    }

    #[test]
    fn test_expma() {
        let input = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0];
        let result = expma(&input, 3, 5).unwrap();

        assert!(!result.ema_short[2].is_nan());
        assert!(!result.ema_long[4].is_nan());
    }

    #[test]
    fn test_ar_br_invalid() {
        assert!(ar(&[], &[], &[], 5).is_err());
        assert!(br(&[], &[], &[], 5).is_err());
        assert!(ar(&[1.0], &[1.0], &[1.0], 0).is_err());
    }
}
