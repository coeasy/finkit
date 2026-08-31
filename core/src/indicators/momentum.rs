use crate::error::{Result, TaError};
use crate::indicators::overlap::MaType;
use crate::math::moving_avg::{ema, simd_horizontal_sum};
use crate::math::simd_ops;
use crate::utils::{init_output, smoothing_factor, validate_input};
use ndarray::Array1;
use std::collections::VecDeque;

#[inline]
fn typical_price(high: f64, low: f64, close: f64) -> f64 {
    (high + low + close) / 3.0
}

#[inline]
#[allow(dead_code)]
fn push_sliding_max(deque: &mut VecDeque<usize>, data: &[f64], i: usize, window: usize) {
    while let Some(&back) = deque.back() {
        if data[back] <= data[i] {
            deque.pop_back();
        } else {
            break;
        }
    }
    deque.push_back(i);
    if let Some(&front) = deque.front() {
        if front + window <= i {
            deque.pop_front();
        }
    }
}

#[inline]
#[allow(dead_code)]
fn push_sliding_min(deque: &mut VecDeque<usize>, data: &[f64], i: usize, window: usize) {
    while let Some(&back) = deque.back() {
        if data[back] >= data[i] {
            deque.pop_back();
        } else {
            break;
        }
    }
    deque.push_back(i);
    if let Some(&front) = deque.front() {
        if front + window <= i {
            deque.pop_front();
        }
    }
}

/// Compute raw %K using linear scan for sliding max/min (TA-Lib C style).
/// For typical small periods (5-25), linear scan outperforms deque due to
/// better cache locality and zero branch overhead from deque maintenance.
#[allow(dead_code)]
fn compute_stoch_fast_k(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    k_period: usize,
    out: &mut [f64],
) {
    let len = close.len();

    for i in (k_period - 1)..len {
        let window_start = i + 1 - k_period;
        let mut highest = high[window_start];
        let mut lowest = low[window_start];
        for j in (window_start + 1)..=i {
            let h = high[j];
            let l = low[j];
            if h > highest {
                highest = h;
            }
            if l < lowest {
                lowest = l;
            }
        }
        let denom = highest - lowest;
        out[i] = if denom > 1e-15 {
            (close[i] - lowest) / denom * 100.0
        } else {
            50.0
        };
    }
}

/// SMA treating NaN inputs as 0.0 (matches STOCH warm-up behavior).
/// Retained as the scalar reference path; `stochrsi` now uses the SIMD kernel
/// (`simd_ops::simd_sma`) with NaN→0.0 pre-mapping for equivalent semantics.
#[allow(dead_code)]
fn sma_nan_as_zero_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period)?;
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }

    for o in output.iter_mut().take(period - 1) {
        *o = f64::NAN;
    }

    let len = input.len();
    let inv_period = 1.0 / period as f64;
    let mut sum: f64 = input[..period]
        .iter()
        .map(|&x| if x.is_nan() { 0.0 } else { x })
        .sum();
    output[period - 1] = sum * inv_period;

    for i in period..len {
        let old = if input[i - period].is_nan() {
            0.0
        } else {
            input[i - period]
        };
        let new = if input[i].is_nan() { 0.0 } else { input[i] };
        sum += new - old;
        output[i] = sum * inv_period;
    }

    Ok(())
}

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
    #[cfg(feature = "metrics")]
    if let Some(idx) = input.iter().position(|v| !v.is_finite()) {
        crate::metrics::input_rejected("rsi", "non_finite");
        return Err(TaError::InvalidParameter {
            name: "input".to_string(),
            constraint: format!("non-finite value at index {idx}"),
        });
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
fn rsi_inner(input: &[f64], period: usize) -> Result<Array1<f64>> {
    let len = input.len();
    let mut output = init_output(len);
    // SIMD dispatch (AVX-512 → AVX2 → scalar) lives inside `rsi_simd_into`;
    // the scalar fallback path is bit-faithful to the original loop above.
    crate::math::simd_kernels::rsi_simd_into(input, period, output.as_slice_mut().unwrap());
    Ok(output)
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
    let mut k_out = vec![f64::NAN; len];
    let mut d_out = vec![f64::NAN; len];

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

/// Single-pass fused pipeline: computes fast %K via incremental max/min tracking, then applies
/// WMA for %K (slow) and %D simultaneously without intermediate allocation.
/// Matches TA-Lib C behavior where NaN fast_k values are treated as 0.
/// TA-Lib uses WMA (Weighted Moving Average) instead of SMA for smoothing.
#[inline]
#[allow(clippy::too_many_arguments)]
#[allow(dead_code)] // used only on non-(std+x86_64) builds; SIMD path covers std+x86_64
fn stoch_fused_pipeline(
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
                let fk = if denom > 1e-15 {
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

    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    crate::math::simd_kernels::stoch_simd_into(
        high, low, close, k_period, k_slow, d_period, k_out, d_out,
    );
    #[cfg(not(all(feature = "std", target_arch = "x86_64")))]
    stoch_fused_pipeline(high, low, close, k_period, k_slow, d_period, k_out, d_out);

    Ok(())
}

/// MACD Result
#[derive(Debug, Clone)]
pub struct MacdResult {
    /// MACD line
    pub macd: Array1<f64>,
    /// Signal line
    pub signal: Array1<f64>,
    /// Histogram
    pub hist: Array1<f64>,
}

/// Moving Average Convergence Divergence (MACD)
///
/// Shows the relationship between two moving averages of a security's price.
///
/// # Arguments
/// * `input` - Input data series
/// * `fast_period` - Fast EMA period
/// * `slow_period` - Slow EMA period
/// * `signal_period` - Signal line EMA period
///
/// # Returns
/// MacdResult containing MACD, signal, and histogram
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close: Vec<f64> = (1..=35).map(|x| x as f64).collect();
/// let result = indicators::macd(&close, 12, 26, 9).unwrap();
/// assert_eq!(result.macd.len(), 35);
/// ```
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(fast_period, slow_period, signal_period, len = input.len())))]
#[inline]
pub fn macd(
    input: &[f64],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> Result<MacdResult> {
    if fast_period >= slow_period {
        return Err(TaError::InvalidParameter {
            name: "fast_period".to_string(),
            constraint: "less than slow_period".to_string(),
        });
    }
    if let Some(idx) = input.iter().position(|v| !v.is_finite()) {
        #[cfg(feature = "metrics")]
        crate::metrics::input_rejected("macd", "non_finite");
        return Err(TaError::InvalidParameter {
            name: "input".to_string(),
            constraint: format!("non-finite value at index {idx}"),
        });
    }
    validate_input(input.len(), slow_period + signal_period - 1)?;

    #[cfg(feature = "metrics")]
    {
        crate::metrics::indicator_called("macd");
        let start = std::time::Instant::now();
        let result = macd_inner(input, fast_period, slow_period, signal_period);
        crate::metrics::record_indicator_duration("macd", start.elapsed().as_secs_f64());
        return result;
    }
    #[cfg(not(feature = "metrics"))]
    macd_inner(input, fast_period, slow_period, signal_period)
}

#[allow(clippy::too_many_arguments)]
#[inline]
fn macd_inner(
    input: &[f64],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> Result<MacdResult> {
    let len = input.len();
    let mut macd_line = init_output(len);
    let mut signal = init_output(len);
    let mut hist = init_output(len);

    if len == 0 {
        return Ok(MacdResult {
            macd: macd_line,
            signal,
            hist,
        });
    }

    // TA-Lib MACD DEFAULT 兼容模式（TA_MACD.c）:
    // 1. slow EMA 种子 = SMA(input[0..slow_period])
    // 2. fast EMA 种子 = SMA(input[slow-fast..slow]) — slow 窗口最后 fast 个值
    // 3. EMA 递推用 FMA: fma(val - prev, k, prev)
    // 4. Signal 种子 = SMA(前 signal_period 个 MACD 值)
    let fast_k = 2.0 / (fast_period as f64 + 1.0);
    let slow_k = 2.0 / (slow_period as f64 + 1.0);
    let signal_k = 2.0 / (signal_period as f64 + 1.0);

    // 累积 slow-only 部分（前 slow_period - fast_period 个值）
    let offset = slow_period - fast_period;
    let mut slow_sum: f64 = 0.0;
    for i in 0..offset {
        slow_sum += input[i];
    }
    // 累积共享部分（接下来 fast_period 个值），同时建立 fast 种子
    let mut fast_sum: f64 = 0.0;
    for i in offset..slow_period {
        fast_sum += input[i];
        slow_sum += input[i];
    }
    let mut prev_slow = slow_sum / slow_period as f64;
    let mut prev_fast = fast_sum / fast_period as f64;

    let macd_start = slow_period - 1;

    // 种子点处的 MACD 值
    let mut macd_val = prev_fast - prev_slow;
    macd_line[macd_start] = macd_val;

    // EMA 递推：使用 FMA 精确匹配 TA-Lib 的浮点舍入路径
    for i in slow_period..len {
        let val = input[i];
        prev_fast = (val - prev_fast).mul_add(fast_k, prev_fast);
        prev_slow = (val - prev_slow).mul_add(slow_k, prev_slow);
        macd_val = prev_fast - prev_slow;
        macd_line[i] = macd_val;
    }

    // Signal line：SMA 种子 + FMA 递推
    let signal_start = macd_start + signal_period - 1;
    if len > signal_start {
        let mut sig_sum: f64 = 0.0;
        for i in macd_start..=signal_start {
            sig_sum += macd_line[i];
        }
        let mut prev_signal = sig_sum / signal_period as f64;
        signal[signal_start] = prev_signal;

        for i in (signal_start + 1)..len {
            let m = macd_line[i];
            prev_signal = (m - prev_signal).mul_add(signal_k, prev_signal);
            signal[i] = prev_signal;
        }

        // Histogram = MACD - Signal
        for i in signal_start..len {
            hist[i] = macd_line[i] - signal[i];
        }
    }

    Ok(MacdResult {
        macd: macd_line,
        signal,
        hist,
    })
}

/// Average Directional Index (ADX)
///
/// Measures trend strength regardless of trend direction.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - Lookback period
///
/// # Returns
/// Array of ADX values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::adx(&high, &low, &close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn adx(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    let family = compute_adx_family(high, low, close, period)?;
    Ok(Array1::from_vec(family.adx))
}

/// Shared ADX family intermediate results.
///
/// Computed once by [`compute_adx_family`] and consumed by the individual
/// public indicator functions (`adx`, `adxr`, `plus_di`, `minus_di`, etc.).
struct AdxFamilyResult {
    plus_di: Vec<f64>,
    minus_di: Vec<f64>,
    adx: Vec<f64>,
}

/// Single-pass computation of +DM, -DM, TR, +DI, -DI, DX, and ADX.
///
/// All ADX-family indicators share the same True Range and Directional
/// Movement values. This function computes them once and derives all
/// intermediate series in a single scan, avoiding redundant TR/DM passes.
fn compute_adx_family(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
) -> Result<AdxFamilyResult> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period * 2)?;

    let len = close.len();
    let p = period as f64;

    let mut smooth_plus_dm = 0.0f64;
    let mut smooth_minus_dm = 0.0f64;
    let mut smooth_tr = 0.0f64;

    // TA-Lib 兼容：累积 period-1 个 DM/TR 值（"that's 13 values because
    // there is no DM for the first day!"），随后通过 Wilder 平滑处理第 period 个 bar。
    if period > 1 {
        #[cfg(feature = "std")]
        {
            crate::math::simd_kernels::adx_warmup_into(
                high,
                low,
                close,
                period - 1,
                &mut smooth_plus_dm,
                &mut smooth_minus_dm,
                &mut smooth_tr,
            );
        }
        #[cfg(not(feature = "std"))]
        {
            for i in 1..period {
                let up_move = high[i] - high[i - 1];
                let down_move = low[i - 1] - low[i];
                smooth_tr += crate::utils::true_range(high[i], low[i], close[i - 1]);
                if up_move > down_move && up_move > 0.0 {
                    smooth_plus_dm += up_move;
                }
                if down_move > up_move && down_move > 0.0 {
                    smooth_minus_dm += down_move;
                }
            }
        }
    }

    let mut plus_di_out = vec![f64::NAN; len];
    let mut minus_di_out = vec![f64::NAN; len];
    let mut adx_out = vec![f64::NAN; len];

    #[inline(always)]
    fn calc_di_dx(s_pdm: f64, s_mdm: f64, s_tr: f64) -> (f64, f64, f64) {
        if s_tr.abs() > 1e-15 {
            let pdi = s_pdm / s_tr * 100.0;
            let mdi = s_mdm / s_tr * 100.0;
            let sum = pdi + mdi;
            let dx = if sum.abs() > 1e-15 {
                (pdi - mdi).abs() / sum * 100.0
            } else {
                0.0
            };
            (pdi, mdi, dx)
        } else {
            (0.0, 0.0, 0.0)
        }
    }

    // TA-Lib Phase 2: Wilder 平滑 + DX 累积（period 次迭代）。
    // 每次：先 Wilder 平滑 DM/TR（prevDM -= prevDM/period; prevDM += newDM），
    // 再计算 DI/DX 并累积 DX。
    let mut dx_sum = 0.0;
    let adx_start = 2 * period;

    for i in period..adx_start.min(len) {
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];
        let tr = crate::utils::true_range(high[i], low[i], close[i - 1]);
        let pdm = if up_move > down_move && up_move > 0.0 {
            up_move
        } else {
            0.0
        };
        let mdm = if down_move > up_move && down_move > 0.0 {
            down_move
        } else {
            0.0
        };
        // TA-Lib: prevDM -= prevDM / period; prevDM += newDM
        smooth_plus_dm = smooth_plus_dm - smooth_plus_dm / p + pdm;
        smooth_minus_dm = smooth_minus_dm - smooth_minus_dm / p + mdm;
        smooth_tr = smooth_tr - smooth_tr / p + tr;

        let (pdi, mdi, dx) = calc_di_dx(smooth_plus_dm, smooth_minus_dm, smooth_tr);
        plus_di_out[i] = pdi;
        minus_di_out[i] = mdi;
        dx_sum += dx;
    }

    if adx_start < len {
        // TA-Lib: prevADX = sumDX / period
        let mut adx_val = dx_sum / p;
        adx_out[adx_start - 1] = adx_val;

        for i in adx_start..len {
            let up_move = high[i] - high[i - 1];
            let down_move = low[i - 1] - low[i];
            let tr = crate::utils::true_range(high[i], low[i], close[i - 1]);
            let pdm = if up_move > down_move && up_move > 0.0 {
                up_move
            } else {
                0.0
            };
            let mdm = if down_move > up_move && down_move > 0.0 {
                down_move
            } else {
                0.0
            };
            smooth_plus_dm = smooth_plus_dm - smooth_plus_dm / p + pdm;
            smooth_minus_dm = smooth_minus_dm - smooth_minus_dm / p + mdm;
            smooth_tr = smooth_tr - smooth_tr / p + tr;

            let (pdi, mdi, dx) = calc_di_dx(smooth_plus_dm, smooth_minus_dm, smooth_tr);
            plus_di_out[i] = pdi;
            minus_di_out[i] = mdi;
            // TA-Lib: prevADX = (prevADX * (period - 1) + DX) / period
            adx_val = (adx_val * (p - 1.0) + dx) / p;
            adx_out[i] = adx_val;
        }
    }

    Ok(AdxFamilyResult {
        plus_di: plus_di_out,
        minus_di: minus_di_out,
        adx: adx_out,
    })
}

/// Compute only +DI and -DI without ADX smoothing.
///
/// This is an optimization for `plus_di` and `minus_di` when called individually.
/// It skips the expensive ADX RMA smoothing loop, saving ~33% computation.
fn compute_di_only(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
) -> Result<(Vec<f64>, Vec<f64>)> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period * 2)?;

    let len = close.len();
    let p = period as f64;

    let mut smooth_plus_dm = 0.0f64;
    let mut smooth_minus_dm = 0.0f64;
    let mut smooth_tr = 0.0f64;

    // TA-Lib 兼容：累积 period-1 个 DM/TR 值，随后 Wilder 平滑处理第 period 个 bar。
    if period > 1 {
        #[cfg(feature = "std")]
        {
            crate::math::simd_kernels::adx_warmup_into(
                high,
                low,
                close,
                period - 1,
                &mut smooth_plus_dm,
                &mut smooth_minus_dm,
                &mut smooth_tr,
            );
        }
        #[cfg(not(feature = "std"))]
        {
            for i in 1..period {
                let up_move = high[i] - high[i - 1];
                let down_move = low[i - 1] - low[i];
                smooth_tr += crate::utils::true_range(high[i], low[i], close[i - 1]);
                if up_move > down_move && up_move > 0.0 {
                    smooth_plus_dm += up_move;
                }
                if down_move > up_move && down_move > 0.0 {
                    smooth_minus_dm += down_move;
                }
            }
        }
    }

    let mut plus_di_out = vec![f64::NAN; len];
    let mut minus_di_out = vec![f64::NAN; len];

    #[inline(always)]
    fn calc_di(s_pdm: f64, s_mdm: f64, s_tr: f64) -> (f64, f64) {
        if s_tr.abs() > 1e-15 {
            (s_pdm / s_tr * 100.0, s_mdm / s_tr * 100.0)
        } else {
            (0.0, 0.0)
        }
    }

    // TA-Lib: 第 period 个 bar 先 Wilder 平滑再计算首个 DI
    // 之后继续 Wilder 平滑（无 ADX 计算）
    for i in period..len {
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];
        let tr = crate::utils::true_range(high[i], low[i], close[i - 1]);
        let pdm = if up_move > down_move && up_move > 0.0 {
            up_move
        } else {
            0.0
        };
        let mdm = if down_move > up_move && down_move > 0.0 {
            down_move
        } else {
            0.0
        };
        smooth_plus_dm = smooth_plus_dm - smooth_plus_dm / p + pdm;
        smooth_minus_dm = smooth_minus_dm - smooth_minus_dm / p + mdm;
        smooth_tr = smooth_tr - smooth_tr / p + tr;

        let (pdi, mdi) = calc_di(smooth_plus_dm, smooth_minus_dm, smooth_tr);
        plus_di_out[i] = pdi;
        minus_di_out[i] = mdi;
    }

    Ok((plus_di_out, minus_di_out))
}

fn di(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    dm: &Array1<f64>,
    period: usize,
) -> Result<Array1<f64>> {
    let len = close.len();
    let mut tr_values = vec![0.0f64; len];
    tr_values[0] = high[0] - low[0];
    for i in 1..len {
        tr_values[i] = crate::utils::true_range(high[i], low[i], close[i - 1]);
    }

    let p = period as f64;
    let inv_p = 1.0 / p;
    let mut di_values = vec![f64::NAN; len];

    let mut smooth_dm: f64 = dm.iter().take(period).sum();
    let mut smooth_tr: f64 = tr_values[..period].iter().sum();

    if smooth_tr.abs() > 1e-15 {
        di_values[period - 1] = smooth_dm / smooth_tr * 100.0;
    }

    for i in period..len {
        smooth_dm = smooth_dm - smooth_dm * inv_p + dm[i];
        smooth_tr = smooth_tr - smooth_tr * inv_p + tr_values[i];
        if smooth_tr.abs() > 1e-15 {
            di_values[i] = smooth_dm / smooth_tr * 100.0;
        }
    }

    Ok(Array1::from_vec(di_values))
}

/// Aroon Indicator Result
#[derive(Debug, Clone)]
pub struct AroonResult {
    /// Aroon Up
    pub aroon_up: Array1<f64>,
    /// Aroon Down
    pub aroon_down: Array1<f64>,
}

/// Aroon Indicator (AROON)
///
/// Identifies trend changes and the strength of the trend.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `period` - Lookback period
///
/// # Returns
/// AroonResult containing Aroon Up and Aroon Down
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let result = indicators::aroon(&high, &low, 5).unwrap();
/// assert_eq!(result.aroon_up.len(), 10);
/// ```
/// Single-pass AROON using optimized sliding window.
///
/// Uses direct index tracking for max/min with efficient rescan when needed.
/// This approach is faster than deque-based methods for typical periods.
fn aroon_with_deques(high: &[f64], low: &[f64], period: usize) -> Result<AroonResult> {
    validate_input(high.len(), period + 1)?;

    let len = high.len();
    let mut up_out = vec![f64::NAN; len];
    let mut dn_out = vec![f64::NAN; len];
    let inv_period = 100.0 / period as f64;
    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();

    unsafe {
        // Initial window [0..=period] (period+1 elements), matching TA-Lib's
        // AROON window semantics. TA-Lib scans `[today-period .. today]`
        // (period+1 bars, including the current bar and index 0) and emits the
        // first value at index `period`.
        let mut highest_idx = 0usize;
        let mut lowest_idx = 0usize;
        let mut highest = *high_ptr;
        let mut lowest = *low_ptr;

        for k in 1..=period {
            let h = *high_ptr.add(k);
            let l = *low_ptr.add(k);
            if h >= highest {
                highest = h;
                highest_idx = k;
            }
            if l <= lowest {
                lowest = l;
                lowest_idx = k;
            }
        }

        // First output at index `period`.
        *up_out.get_unchecked_mut(period) = highest_idx as f64 * inv_period;
        *dn_out.get_unchecked_mut(period) = lowest_idx as f64 * inv_period;

        // Slide window: window [ws, i] has `period + 1` elements, matching
        // TA-Lib's trailing window [today-period .. today].
        for i in (period + 1)..len {
            let ws = i - period;
            let new_h = *high_ptr.add(i);
            let new_l = *low_ptr.add(i);

            // Update highest
            if highest_idx < ws {
                // Max fell out of window, rescan
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

            // Update lowest
            if lowest_idx < ws {
                // Min fell out of window, rescan
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

            *up_out.get_unchecked_mut(i) = (period - (i - highest_idx)) as f64 * inv_period;
            *dn_out.get_unchecked_mut(i) = (period - (i - lowest_idx)) as f64 * inv_period;
        }
    }

    Ok(AroonResult {
        aroon_up: Array1::from(up_out),
        aroon_down: Array1::from(dn_out),
    })
}

pub fn aroon(high: &[f64], low: &[f64], period: usize) -> Result<AroonResult> {
    // Use optimized sliding window algorithm for all periods
    aroon_with_deques(high, low, period)
}

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
/// Mean absolute deviation of a window given its sorted elements and the
/// parallel prefix-sum array `pref` (where `pref[k]` is the sum of the first
/// `k` sorted elements). Computed in `O(log period)` via a binary search for
/// the mean-split index, replacing the original `O(period)` abs-deviation
/// loop. Exact up to float-reordering, which sits within golden tolerance.
#[inline]
fn cci_mad(sorted: &[f64], pref: &[f64], mean: f64, period: usize, inv_p: f64) -> f64 {
    let k = sorted.partition_point(|&x| x <= mean);
    let left_sum = pref[k];
    let right_sum = pref[period] - pref[k];
    let mad = (mean * k as f64 - left_sum) + (right_sum - mean * (period - k) as f64);
    mad * inv_p
}

pub fn cci(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period)?;

    let len = close.len();
    let mut output = init_output(len);
    let inv_p = 1.0 / period as f64;

    // Raw window order (ring) + sorted window + parallel prefix sums.
    let mut ring: Vec<f64> = vec![0.0; period];
    let mut sorted: Vec<f64> = Vec::with_capacity(period);
    let mut pref: Vec<f64> = Vec::with_capacity(period + 1);
    let mut tp_sum = 0.0;
    for j in 0..period {
        let tp = typical_price(high[j], low[j], close[j]);
        ring[j] = tp;
        tp_sum += tp;
        sorted.push(tp);
    }
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    pref.push(0.0);
    for &v in &sorted {
        pref.push(*pref.last().unwrap() + v);
    }

    let first = period - 1;
    {
        let tp_mean = tp_sum * inv_p;
        let mean_dev = cci_mad(&sorted, &pref, tp_mean, period, inv_p);
        if mean_dev.abs() > 1e-15 {
            output[first] = (ring[period - 1] - tp_mean) / (0.015 * mean_dev);
        }
    }

    let mut ring_idx = 0;
    for i in period..len {
        let new_tp = typical_price(high[i], low[i], close[i]);
        let old_tp = ring[ring_idx];
        tp_sum += new_tp - old_tp;

        // Remove the outgoing element from the sorted window + prefix sums.
        let rpos = sorted.partition_point(|&x| x < old_tp);
        sorted.remove(rpos);
        // Shift the prefix sums left past the removed slot (subtract old_tp),
        // keeping cumulative sums exact for the period-1 window, then drop tail.
        for p in (rpos + 1)..period {
            pref[p] = pref[p + 1] - old_tp;
        }
        pref.truncate(period);

        // Insert the incoming element into the sorted window + prefix sums.
        let ipos = sorted.partition_point(|&x| x < new_tp);
        sorted.insert(ipos, new_tp);
        pref.insert(ipos + 1, pref[ipos] + new_tp);
        for p in (ipos + 2)..pref.len() {
            pref[p] += new_tp;
        }

        ring[ring_idx] = new_tp;
        ring_idx = (ring_idx + 1) % period;

        let tp_mean = tp_sum * inv_p;
        let mean_dev = cci_mad(&sorted, &pref, tp_mean, period, inv_p);
        if mean_dev.abs() > 1e-15 {
            output[i] = (new_tp - tp_mean) / (0.015 * mean_dev);
        }
    }

    Ok(output)
}

/// Momentum (MOM)
///
/// Measures the change in price over a given period.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
///
/// # Returns
/// Array of momentum values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::mom(&close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn mom(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period + 1)?;

    let len = input.len();
    // 直接分配 Array1 并写入，避免中间 Vec 分配
    let mut output = Array1::<f64>::zeros(len);
    simd_ops::simd_mom(input, period, output.as_slice_mut().unwrap());

    Ok(output)
}

/// Rate of Change (ROC)
///
/// Measures the percentage change in price over a given period.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
///
/// # Returns
/// Array of ROC values (in percentage)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::roc(&close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn roc(input: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_input(input.len(), period + 1)?;

    let len = input.len();
    let mut buf = vec![0.0f64; len];
    simd_ops::simd_roc(input, period, &mut buf);

    Ok(Array1::from_vec(buf))
}

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
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period)?;

    let len = close.len();
    let mut out = vec![f64::NAN; len];
    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();
    let close_ptr = close.as_ptr();
    let out_ptr = out.as_mut_ptr();
    let start = period - 1;

    // Optimized sliding window: track max/min indices directly
    // For all periods, use the same efficient algorithm with direct index tracking
    unsafe {
        // Initialize first window [0..period-1]
        let mut highest_idx = 0usize;
        let mut lowest_idx = 0usize;
        let mut highest = *high_ptr.add(0);
        let mut lowest = *low_ptr.add(0);

        for k in 1..period {
            let h = *high_ptr.add(k);
            let l = *low_ptr.add(k);
            if h >= highest {
                highest = h;
                highest_idx = k;
            }
            if l <= lowest {
                lowest = l;
                lowest_idx = k;
            }
        }

        // First output at index period-1
        let denom = highest - lowest;
        *out_ptr.add(start) = if denom > 1e-15 {
            (highest - *close_ptr.add(start)) / denom * -100.0
        } else {
            0.0
        };

        // Slide window: [i-period+1..=i]
        for i in period..len {
            let ws = i + 1 - period; // window start
            let new_h = *high_ptr.add(i);
            let new_l = *low_ptr.add(i);

            // Update highest
            if highest_idx < ws {
                // Max fell out of window, rescan
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

            // Update lowest
            if lowest_idx < ws {
                // Min fell out of window, rescan
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

            let denom = highest - lowest;
            *out_ptr.add(i) = if denom > 1e-15 {
                (highest - *close_ptr.add(i)) / denom * -100.0
            } else {
                0.0
            };
        }
    }

    Ok(Array1::from_vec(out))
}

/// Elder-Ray Indicator Result
#[derive(Debug, Clone)]
pub struct ElderRayResult {
    /// Force Index: (Close - Close\[1\]) * Volume
    pub force_index: Array1<f64>,
    /// Bull Power: High - EMA(Close, period)
    pub bull_power: Array1<f64>,
    /// Bear Power: Low - EMA(Close, period)
    pub bear_power: Array1<f64>,
}

/// Elder-Ray Indicator (ELDER-RAY)
///
/// Developed by Alexander Elder, this indicator uses three components to evaluate
/// the balance of power between bulls and bears in the market.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `period` - EMA lookback period for Bull/Bear Power calculation
///
/// # Returns
/// ElderRayResult containing Force Index, Bull Power, and Bear Power
///
/// # Formula
/// * Force Index = (Close\[i\] - Close\[i-1\]) * Volume\[i\]
/// * Bull Power = High\[i\] - EMA(Close, period)\[i\]
/// * Bear Power = Low\[i\] - EMA(Close, period)\[i\]
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
/// let result = indicators::elder_ray(&high, &low, &close, &volume, 5).unwrap();
/// assert_eq!(result.bull_power.len(), 10);
/// ```
pub fn elder_ray(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    period: usize,
) -> Result<ElderRayResult> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period + 1)?;

    let len = close.len();

    // Calculate Force Index: (Close[i] - Close[i-1]) * Volume[i]
    let mut force_index = init_output(len);
    for i in 1..len {
        force_index[i] = (close[i] - close[i - 1]) * volume[i];
    }

    // Calculate EMA of close prices
    let ema_close = ema(close, period)?;

    // Calculate Bull Power: High - EMA(Close)
    let mut bull_power = init_output(len);
    for i in 0..len {
        if !ema_close[i].is_nan() {
            bull_power[i] = high[i] - ema_close[i];
        }
    }

    // Calculate Bear Power: Low - EMA(Close)
    let mut bear_power = init_output(len);
    for i in 0..len {
        if !ema_close[i].is_nan() {
            bear_power[i] = low[i] - ema_close[i];
        }
    }

    Ok(ElderRayResult {
        force_index,
        bull_power,
        bear_power,
    })
}

/// Absolute Price Oscillator (APO)
///
/// The difference between two moving averages.
///
/// # Arguments
/// * `input` - Input data series
/// * `fast_period` - Fast period
/// * `slow_period` - Slow period
///
/// # Returns
/// Array of APO values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::apo(&close, 2, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn apo(input: &[f64], fast_period: usize, slow_period: usize) -> Result<Array1<f64>> {
    if fast_period >= slow_period {
        return Err(TaError::InvalidParameter {
            name: "fast_period".to_string(),
            constraint: "less than slow_period".to_string(),
        });
    }
    if fast_period == 0 || slow_period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    if let Some(idx) = input.iter().position(|v| !v.is_finite()) {
        return Err(TaError::InvalidParameter {
            name: "input".to_string(),
            constraint: format!("non-finite value at index {idx}"),
        });
    }
    validate_input(input.len(), slow_period)?;

    let len = input.len();
    let mut output = init_output(len);

    // Fused single-pass APO: compute the fast & slow SMA running sums inline
    // and subtract, eliminating the two full-length SMA arrays plus the final
    // diff pass. Accumulation order is bit-identical to `sma(fast) - sma(slow)`.
    let fast_inv = 1.0 / fast_period as f64;
    let slow_inv = 1.0 / slow_period as f64;

    let mut fast_sum = simd_horizontal_sum(&input[..fast_period]);
    let mut slow_sum = simd_horizontal_sum(&input[..slow_period]);

    // Advance fast_sum so it reflects the window ending at slow_period-1
    // (identical sliding order to sma_inner).
    for i in fast_period..slow_period {
        fast_sum += input[i] - input[i - fast_period];
    }

    let first = slow_period - 1;
    output[first] = fast_sum * fast_inv - slow_sum * slow_inv;

    for i in slow_period..len {
        fast_sum += input[i] - input[i - fast_period];
        slow_sum += input[i] - input[i - slow_period];
        output[i] = fast_sum * fast_inv - slow_sum * slow_inv;
    }

    Ok(output)
}

/// Balance of Power (BOP)
///
/// Measures the strength of buyers vs sellers in the market.
///
/// # Arguments
/// * `open` - Open prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
///
/// # Returns
/// Array of BOP values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let open = vec![43.5, 44.0, 44.25, 43.5, 44.25, 44.0, 43.75, 43.25, 43.75, 44.0];
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::bop(&open, &high, &low, &close).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn bop(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<Array1<f64>> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 1)?;

    let len = open.len();
    let mut buf = vec![0.0f64; len];
    simd_ops::simd_bop(open, high, low, close, &mut buf);

    Ok(Array1::from_vec(buf))
}

/// Chande Momentum Oscillator (CMO)
///
/// A momentum indicator that measures the percentage of sum of up days vs sum of down days.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
///
/// # Returns
/// Array of CMO values (-100 to 100 range)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::cmo(&close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn cmo(input: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_input(input.len(), period + 1)?;

    let len = input.len();
    let mut output = init_output(len);

    // Calculate changes
    let mut changes = Vec::with_capacity(len);
    changes.push(0.0);
    for i in 1..len {
        changes.push(input[i] - input[i - 1]);
    }

    // Initial sum for first period values
    let mut sum_up = 0.0;
    let mut sum_down = 0.0;
    for i in 1..=period {
        let ch = changes[i];
        if ch > 0.0 {
            sum_up += ch;
        } else {
            sum_down -= ch;
        }
    }

    let denom = sum_up + sum_down;
    if denom.abs() > 1e-15 {
        output[period] = (sum_up - sum_down) / denom * 100.0;
    }

    // Convert sums to averages for RMA initialization (like TA-Lib)
    let inv_period = 1.0 / period as f64;
    sum_up *= inv_period;
    sum_down *= inv_period;

    // Use RMA (Recursive Moving Average) for subsequent values
    for i in (period + 1)..len {
        let ch = changes[i];
        let up = if ch > 0.0 { ch } else { 0.0 };
        let down = if ch < 0.0 { -ch } else { 0.0 };

        // RMA: new_value = (old_value * (period - 1) + new_value) / period
        sum_up = (sum_up * (period as f64 - 1.0) + up) * inv_period;
        sum_down = (sum_down * (period as f64 - 1.0) + down) * inv_period;

        let denom = sum_up + sum_down;
        if denom.abs() > 1e-15 {
            output[i] = (sum_up - sum_down) / denom * 100.0;
        }
    }

    Ok(output)
}

/// Directional Movement Index (DX)
///
/// Measures trend direction and strength.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - Lookback period
///
/// # Returns
/// Array of DX values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::dx(&high, &low, &close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn dx(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }

    if high.len() >= period * 2 {
        let family = compute_adx_family(high, low, close, period)?;
        let len = close.len();
        let mut dx_vals = init_output(len);
        for i in 0..len {
            let pdi = family.plus_di[i];
            let mdi = family.minus_di[i];
            if !pdi.is_nan() && !mdi.is_nan() {
                let sum = pdi + mdi;
                if sum.abs() > 1e-15 {
                    dx_vals[i] = (pdi - mdi).abs() / sum * 100.0;
                }
            }
        }
        return Ok(dx_vals);
    }

    let plus_dm_vals = plus_dm(high, low)?;
    let minus_dm_vals = minus_dm(high, low)?;
    let plus_di_vals = di(high, low, close, &plus_dm_vals, period)?;
    let minus_di_vals = di(high, low, close, &minus_dm_vals, period)?;

    let len = close.len();
    let mut dx_vals = init_output(len);

    for i in 0..len {
        if !plus_di_vals[i].is_nan() && !minus_di_vals[i].is_nan() {
            let sum = plus_di_vals[i] + minus_di_vals[i];
            if sum.abs() > 1e-15 {
                dx_vals[i] = (plus_di_vals[i] - minus_di_vals[i]).abs() / sum * 100.0;
            }
        }
    }

    Ok(dx_vals)
}

/// Money Flow Index (MFI)
///
/// A momentum indicator that uses both price and volume to identify overbought/oversold conditions.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `period` - Lookback period
///
/// # Returns
/// Array of MFI values (0-100 range)
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
/// let result = indicators::mfi(&high, &low, &close, &volume, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn mfi(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    period: usize,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period + 1)?;

    let len = close.len();
    let mut output = vec![f64::NAN; len];

    // Typical price (high+low+close)/3, batched through the SIMD fast path.
    // This is elementwise and order-independent, so it is bit-identical to the
    // scalar form while running 4 lanes at a time.
    let mut tp = vec![0.0_f64; len];
    simd_ops::simd_typical_price(high, low, close, &mut tp);

    let mut pos_ring = vec![0.0_f64; period];
    let mut neg_ring = vec![0.0_f64; period];
    let mut pos_sum: f64 = 0.0;
    let mut neg_sum: f64 = 0.0;
    let mut ring_idx: usize = 0;

    let mut prev_tp = tp[0];

    for i in 1..len {
        let tp_i = tp[i];
        let mf_val = tp_i * volume[i];

        let (pos, neg) = if tp_i > prev_tp {
            (mf_val, 0.0)
        } else {
            (0.0, mf_val)
        };
        prev_tp = tp_i;

        pos_sum += pos - pos_ring[ring_idx];
        neg_sum += neg - neg_ring[ring_idx];
        pos_ring[ring_idx] = pos;
        neg_ring[ring_idx] = neg;
        ring_idx += 1;
        if ring_idx == period {
            ring_idx = 0;
        }

        if i >= period {
            output[i] = if neg_sum.abs() > 1e-15 {
                100.0 - 100.0 / (1.0 + pos_sum / neg_sum)
            } else {
                100.0
            };
        }
    }

    Ok(Array1::from(output))
}

/// Minus Directional Indicator (MINUS_DI)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::minus_di(&high, &low, &close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn minus_di(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if high.len() < period * 2 {
        let minus_dm_vals = minus_dm(high, low)?;
        return di(high, low, close, &minus_dm_vals, period);
    }
    // Optimization: use compute_di_only to skip ADX RMA smoothing
    let (_plus_di_out, minus_di_out) = compute_di_only(high, low, close, period)?;
    Ok(Array1::from_vec(minus_di_out))
}

/// Minus Directional Movement (MINUS_DM)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let result = indicators::minus_dm(&high, &low).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn minus_dm(high: &[f64], low: &[f64]) -> Result<Array1<f64>> {
    validate_input(high.len(), 2)?;

    let len = high.len();
    let mut output = vec![0.0f64; len];

    for i in 1..len {
        let down_move = low[i - 1] - low[i];
        let up_move = high[i] - high[i - 1];

        if down_move > 0.0 && down_move > up_move {
            output[i] = down_move;
        }
    }

    Ok(Array1::from_vec(output))
}

/// Plus Directional Indicator (PLUS_DI)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::plus_di(&high, &low, &close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn plus_di(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if high.len() < period * 2 {
        let plus_dm_vals = plus_dm(high, low)?;
        return di(high, low, close, &plus_dm_vals, period);
    }
    // Optimization: use compute_di_only to skip ADX RMA smoothing
    let (plus_di_out, _minus_di_out) = compute_di_only(high, low, close, period)?;
    Ok(Array1::from_vec(plus_di_out))
}

/// Plus Directional Movement (PLUS_DM)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let result = indicators::plus_dm(&high, &low).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn plus_dm(high: &[f64], low: &[f64]) -> Result<Array1<f64>> {
    validate_input(high.len(), 2)?;

    let len = high.len();
    let mut output = vec![0.0f64; len];

    for i in 1..len {
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];

        if up_move > 0.0 && up_move > down_move {
            output[i] = up_move;
        }
    }

    Ok(Array1::from_vec(output))
}

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
    validate_input(input.len(), period)?;

    let len = input.len();
    let mut output = init_output(len);
    let s1 = period - 1; // EMA1 首有效值位置
    let s2 = 2 * s1; // EMA2 首有效值位置
    let _s3 = 3 * s1; // EMA3 首有效值位置（文档用，TRIX 首有效值在 _s3 + 1）
    let k = smoothing_factor(period);
    let one_k = 1.0 - k;
    let inv_p = 1.0 / period as f64;

    // EMA1: 种子 = SMA of input[0..period]，首有效值在 s1
    let mut ema1_buf = vec![0.0f64; len];
    let sma1: f64 = input[..period].iter().sum::<f64>() * inv_p;
    ema1_buf[s1] = sma1;
    let mut e1 = sma1;
    for i in period..len {
        e1 = input[i] * k + e1 * one_k;
        ema1_buf[i] = e1;
    }

    // EMA2: TA-Lib 兼容，种子 = SMA of EMA1[s1..s1+period]，首有效值在 s2
    if s1 + period <= len {
        let mut ema2_buf = vec![0.0f64; len];
        let sma2: f64 = ema1_buf[s1..s1 + period].iter().sum::<f64>() * inv_p;
        ema2_buf[s2] = sma2;
        let mut e2 = sma2;
        for i in (s1 + period)..len {
            e2 = ema1_buf[i] * k + e2 * one_k;
            ema2_buf[i] = e2;
        }

        // EMA3: TA-Lib 兼容，种子 = SMA of EMA2[s2..s2+period]，首有效值在 s3
        if s2 + period <= len {
            let sma3: f64 = ema2_buf[s2..s2 + period].iter().sum::<f64>() * inv_p;
            let mut e3_prev = sma3;
            // TRIX 首有效值在 s3 + 1（需要 e3_prev 和当前 e3）
            for i in (s2 + period)..len {
                let e3 = ema2_buf[i] * k + e3_prev * one_k;
                if e3_prev.abs() > 1e-15 {
                    output[i] = (e3 - e3_prev) / e3_prev * 100.0;
                }
                e3_prev = e3;
            }
        }
    }

    Ok(output)
}

/// Average Directional Movement Index Rating (ADXR)
///
/// ADXR = (ADX_today + ADX_n_periods_ago) / 2
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high: Vec<f64> = (0..20).map(|i| 45.0 + i as f64 * 0.1).collect();
/// let low: Vec<f64> = (0..20).map(|i| 43.0 + i as f64 * 0.1).collect();
/// let close: Vec<f64> = (0..20).map(|i| 44.0 + i as f64 * 0.1).collect();
/// let result = indicators::adxr(&high, &low, &close, 5).unwrap();
/// assert_eq!(result.len(), 20);
/// ```
pub fn adxr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    let family = compute_adx_family(high, low, close, period)?;
    let adx_vals = &family.adx;
    let len = adx_vals.len();
    let mut output = vec![f64::NAN; len];

    for i in period..len {
        let cur = adx_vals[i];
        let prev = adx_vals[i - period];
        if !cur.is_nan() && !prev.is_nan() {
            output[i] = (cur + prev) * 0.5;
        }
    }

    Ok(Array1::from_vec(output))
}

/// Aroon Oscillator (AROONOSC)
///
/// AROONOSC = Aroon Up - Aroon Down
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let result = indicators::aroonosc(&high, &low, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn aroonosc(high: &[f64], low: &[f64], period: usize) -> Result<Array1<f64>> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period + 1)?;

    let len = high.len();
    let mut output = init_output(len);
    let inv_period = 100.0 / period as f64;
    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();

    if period <= 8 {
        aroonosc_scan_inner(high_ptr, low_ptr, len, period, inv_period, &mut output);
    } else {
        aroonosc_deque_inner(high_ptr, low_ptr, len, period, inv_period, &mut output);
    }

    Ok(output)
}

fn aroonosc_scan_inner(
    high_ptr: *const f64,
    low_ptr: *const f64,
    len: usize,
    period: usize,
    inv_period: f64,
    output: &mut Array1<f64>,
) {
    unsafe {
        let first_ws: usize = 1;
        let mut highest_idx = first_ws;
        let mut lowest_idx = first_ws;
        let mut highest = *high_ptr.add(first_ws);
        let mut lowest = *low_ptr.add(first_ws);
        for k in (first_ws + 1)..=period {
            let h = *high_ptr.add(k);
            let l = *low_ptr.add(k);
            if h >= highest {
                highest = h;
                highest_idx = k;
            }
            if l <= lowest {
                lowest = l;
                lowest_idx = k;
            }
        }

        let up = highest_idx as f64 * inv_period;
        let dn = lowest_idx as f64 * inv_period;
        *output.uget_mut(period) = up - dn;

        for i in (period + 1)..len {
            let ws = i + 1 - period;
            let new_h = *high_ptr.add(i);
            let new_l = *low_ptr.add(i);

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

            let up = (period - (i - highest_idx)) as f64 * inv_period;
            let dn = (period - (i - lowest_idx)) as f64 * inv_period;
            *output.uget_mut(i) = up - dn;
        }
    }
}

fn aroonosc_deque_inner(
    high_ptr: *const f64,
    low_ptr: *const f64,
    len: usize,
    period: usize,
    inv_period: f64,
    output: &mut Array1<f64>,
) {
    let mut h_buf: Vec<usize> = Vec::with_capacity(period + 1);
    let mut l_buf: Vec<usize> = Vec::with_capacity(period + 1);
    let mut h_head: usize = 0;
    let mut l_head: usize = 0;

    unsafe {
        h_buf.push(1);
        l_buf.push(1);
        for k in 2..=period {
            let h = *high_ptr.add(k);
            let l = *low_ptr.add(k);

            while h_buf.len() > h_head && *high_ptr.add(*h_buf.last().unwrap()) <= h {
                h_buf.pop();
            }
            h_buf.push(k);

            while l_buf.len() > l_head && *low_ptr.add(*l_buf.last().unwrap()) >= l {
                l_buf.pop();
            }
            l_buf.push(k);
        }

        let highest_idx = *h_buf.get_unchecked(h_head);
        let lowest_idx = *l_buf.get_unchecked(l_head);
        let up = highest_idx as f64 * inv_period;
        let dn = lowest_idx as f64 * inv_period;
        *output.uget_mut(period) = up - dn;

        for i in (period + 1)..len {
            let new_h = *high_ptr.add(i);
            let new_l = *low_ptr.add(i);
            let ws = i + 1 - period;

            while h_buf.len() > h_head && *high_ptr.add(*h_buf.last().unwrap()) <= new_h {
                h_buf.pop();
            }
            h_buf.push(i);
            while h_buf[h_head] < ws {
                h_head += 1;
            }
            let highest_idx_i = *h_buf.get_unchecked(h_head);

            while l_buf.len() > l_head && *low_ptr.add(*l_buf.last().unwrap()) >= new_l {
                l_buf.pop();
            }
            l_buf.push(i);
            while l_buf[l_head] < ws {
                l_head += 1;
            }
            let lowest_idx_i = *l_buf.get_unchecked(l_head);

            let up = (period - (i - highest_idx_i)) as f64 * inv_period;
            let dn = (period - (i - lowest_idx_i)) as f64 * inv_period;
            *output.uget_mut(i) = up - dn;
        }
    }
}

/// MACD with controllable MA type (MACDEXT)
///
/// Like MACD but allows choosing the MA type for fast, slow, and signal lines.
///
/// # Examples
///
/// ```
/// use finkit::indicators::{self, MaType};
///
/// let close: Vec<f64> = (1..=30).map(|x| x as f64).collect();
/// let result = indicators::macdext(&close, 12, MaType::Ema, 26, MaType::Ema, 9, MaType::Ema).unwrap();
/// assert_eq!(result.macd.len(), 30);
/// ```
pub fn macdext(
    input: &[f64],
    fast_period: usize,
    fast_ma_type: MaType,
    slow_period: usize,
    slow_ma_type: MaType,
    signal_period: usize,
    signal_ma_type: MaType,
) -> Result<MacdResult> {
    let fast_ma = crate::indicators::overlap::ma(input, fast_period, fast_ma_type)?;
    let slow_ma = crate::indicators::overlap::ma(input, slow_period, slow_ma_type)?;

    let len = input.len();
    let mut macd_line = init_output(len);
    for i in 0..len {
        if !fast_ma[i].is_nan() && !slow_ma[i].is_nan() {
            macd_line[i] = fast_ma[i] - slow_ma[i];
        }
    }

    let macd_vec: Vec<f64> = macd_line
        .iter()
        .map(|&x| if x.is_nan() { 0.0 } else { x })
        .collect();
    let signal = crate::indicators::overlap::ma(&macd_vec, signal_period, signal_ma_type)?;

    let mut hist = init_output(len);
    for i in 0..len {
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

/// MACD with fixed 12/26/9 parameters (MACDFIX)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close: Vec<f64> = (1..=40).map(|x| x as f64).collect();
/// let result = indicators::macdfix(&close).unwrap();
/// assert_eq!(result.macd.len(), 40);
/// ```
pub fn macdfix(input: &[f64]) -> Result<MacdResult> {
    macd(input, 12, 26, 9)
}

/// Percentage Price Oscillator (PPO)
///
/// PPO = ((fast_EMA - slow_EMA) / slow_EMA) * 100
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close: Vec<f64> = (1..=30).map(|x| x as f64).collect();
/// let result = indicators::ppo(&close, 12, 26).unwrap();
/// assert_eq!(result.len(), 30);
/// ```
pub fn ppo(input: &[f64], fast_period: usize, slow_period: usize) -> Result<Array1<f64>> {
    let fast_ema = ema(input, fast_period)?;
    let slow_ema = ema(input, slow_period)?;

    let len = input.len();
    let mut output = init_output(len);

    for i in 0..len {
        if !fast_ema[i].is_nan() && !slow_ema[i].is_nan() && slow_ema[i].abs() > 1e-15 {
            output[i] = ((fast_ema[i] - slow_ema[i]) / slow_ema[i]) * 100.0;
        }
    }

    Ok(output)
}

/// Rate of Change Percentage (ROCP)
///
/// ROCP = (close - close_n) / close_n
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::rocp(&close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn rocp(input: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_input(input.len(), period + 1)?;
    let len = input.len();
    let mut output = init_output(len);

    for i in period..len {
        if input[i - period].abs() > 1e-15 {
            output[i] = (input[i] - input[i - period]) / input[i - period];
        }
    }

    Ok(output)
}

/// Rate of Change Ratio (ROCR)
///
/// ROCR = close / close_n
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::rocr(&close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn rocr(input: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_input(input.len(), period + 1)?;
    let len = input.len();
    let mut output = init_output(len);

    for i in period..len {
        if input[i - period].abs() > 1e-15 {
            output[i] = input[i] / input[i - period];
        }
    }

    Ok(output)
}

/// Rate of Change Ratio scaled to 100 (ROCR100)
///
/// ROCR100 = (close / close_n) * 100
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::rocr100(&close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn rocr100(input: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_input(input.len(), period + 1)?;
    let len = input.len();
    let mut output = init_output(len);

    for i in period..len {
        if input[i - period].abs() > 1e-15 {
            output[i] = (input[i] / input[i - period]) * 100.0;
        }
    }

    Ok(output)
}

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

    let len = high.len();
    let mut fastk = vec![f64::NAN; len];
    let mut fastd = vec![f64::NAN; len];

    let fastk_start = fastk_period - 1;
    let d_start = fastk_start + fastd_period - 1;
    let inv_d = 1.0 / fastd_period as f64;

    let mut d_ring = vec![0.0_f64; fastd_period];
    let mut d_sum: f64 = 0.0;

    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();
    let close_ptr = close.as_ptr();

    // Monotonic deques: max-deque for high (front = index of max), min-deque
    // for low (front = index of min). O(1) amortized per element, replacing
    // the O(period) rescan.
    let mut h_buf: Vec<usize> = Vec::with_capacity(fastk_period);
    let mut l_buf: Vec<usize> = Vec::with_capacity(fastk_period);
    let mut h_head: usize = 0;
    let mut l_head: usize = 0;

    for i in 0..len {
        unsafe {
            let new_h = *high_ptr.add(i);
            let new_l = *low_ptr.add(i);

            // Deque update: monotonic pop from back, then push current index.
            // Front-expiration happens lazily below when we need the value.
            while h_buf.len() > h_head && *high_ptr.add(*h_buf.last().unwrap()) <= new_h {
                h_buf.pop();
            }
            h_buf.push(i);
            while l_buf.len() > l_head && *low_ptr.add(*l_buf.last().unwrap()) >= new_l {
                l_buf.pop();
            }
            l_buf.push(i);

            // Initialise the deques during the first window.
            if i + 1 < fastk_period {
                continue;
            }
            let ws = i + 1 - fastk_period;
            // Drop entries that fell out of the window.
            while h_buf[h_head] < ws {
                h_head += 1;
            }
            while l_buf[l_head] < ws {
                l_head += 1;
            }
            let highest = *high_ptr.add(*h_buf.get_unchecked(h_head));
            let lowest = *low_ptr.add(*l_buf.get_unchecked(l_head));

            let denom = highest - lowest;
            let fk = if denom > 1e-15 {
                (*close_ptr.add(i) - lowest) / denom * 100.0
            } else {
                50.0
            };
            *fastk.get_unchecked_mut(i) = fk;

            let d_idx = i - fastk_start;
            let ring_pos = d_idx % fastd_period;
            d_sum += fk - *d_ring.get_unchecked(ring_pos);
            *d_ring.get_unchecked_mut(ring_pos) = fk;

            if i >= d_start {
                *fastd.get_unchecked_mut(i) = d_sum * inv_d;
            }
        }
    }

    Ok(StochResult {
        k: Array1::from(fastk),
        d: Array1::from(fastd),
    })
}

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
    let rsi_vals = rsi(input, rsi_period)?;
    let rsi_slice = rsi_vals.as_slice().unwrap();
    let len = rsi_slice.len();

    let mut rsi_clean = vec![0.0; len];
    for (i, &v) in rsi_slice.iter().enumerate() {
        if !v.is_nan() {
            rsi_clean[i] = v;
        }
    }

    let mut raw_k = init_output(len);
    let valid_start = rsi_period + stoch_period - 1;

    {
        let mut max_dq: VecDeque<usize> = VecDeque::with_capacity(stoch_period + 1);
        let mut min_dq: VecDeque<usize> = VecDeque::with_capacity(stoch_period + 1);

        let seed_start = rsi_period;
        for i in seed_start..len {
            let v = rsi_clean[i];

            while let Some(&back) = max_dq.back() {
                if rsi_clean[back] <= v {
                    max_dq.pop_back();
                } else {
                    break;
                }
            }
            max_dq.push_back(i);

            while let Some(&back) = min_dq.back() {
                if rsi_clean[back] >= v {
                    min_dq.pop_back();
                } else {
                    break;
                }
            }
            min_dq.push_back(i);

            let ws = if i + 1 >= stoch_period + seed_start {
                i + 1 - stoch_period
            } else {
                seed_start
            };
            while let Some(&front) = max_dq.front() {
                if front < ws {
                    max_dq.pop_front();
                } else {
                    break;
                }
            }
            while let Some(&front) = min_dq.front() {
                if front < ws {
                    min_dq.pop_front();
                } else {
                    break;
                }
            }

            if i >= valid_start {
                let highest = rsi_clean[*max_dq.front().unwrap()];
                let lowest = rsi_clean[*min_dq.front().unwrap()];
                let range = highest - lowest;
                if range > 1e-15 {
                    raw_k[i] = ((rsi_clean[i] - lowest) / range) * 100.0;
                } else {
                    raw_k[i] = 50.0;
                }
            }
        }
    }

    // %K smoothing: SMA of raw %K, treating NaN (warm-up) inputs as 0.0.
    // SIMD kernel is used; NaN positions are mapped to 0.0 first to mirror the
    // scalar `sma_nan_as_zero_into` semantics (which counted NaNs as zero).
    let mut fastk_ma = init_output(len);
    {
        let raw_k_clean: Vec<f64> = raw_k
            .iter()
            .map(|&v| if v.is_nan() { 0.0 } else { v })
            .collect();
        simd_ops::simd_sma(&raw_k_clean, fastk_period, fastk_ma.as_slice_mut().unwrap());
    }

    // %D smoothing: SMA of %K, again with NaN→0.0 pre-mapping.
    let mut fastd_ma = init_output(len);
    {
        let fastk_ma_clean: Vec<f64> = fastk_ma
            .iter()
            .map(|&v| if v.is_nan() { 0.0 } else { v })
            .collect();
        simd_ops::simd_sma(
            &fastk_ma_clean,
            fastd_period,
            fastd_ma.as_slice_mut().unwrap(),
        );
    }

    let mut out_k = init_output(len);
    let mut out_d = init_output(len);
    let k_start = valid_start + fastk_period - 1;
    let d_start = k_start + fastd_period - 1;
    for i in k_start..len {
        if !fastk_ma[i].is_nan() {
            out_k[i] = fastk_ma[i];
        }
    }
    for i in d_start..len {
        if !fastd_ma[i].is_nan() {
            out_d[i] = fastd_ma[i];
        }
    }

    Ok(StochResult { k: out_k, d: out_d })
}

/// Ultimate Oscillator (ULTOSC)
///
/// Combines short, intermediate, and long-term price action into a single value.
/// Default periods: 7, 14, 28.
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high: Vec<f64> = (0..30).map(|i| 45.0 + i as f64 * 0.1).collect();
/// let low: Vec<f64> = (0..30).map(|i| 43.0 + i as f64 * 0.1).collect();
/// let close: Vec<f64> = (0..30).map(|i| 44.0 + i as f64 * 0.1).collect();
/// let result = indicators::ultosc(&high, &low, &close, 7, 14, 28).unwrap();
/// assert_eq!(result.len(), 30);
/// ```
pub fn ultosc(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period1: usize,
    period2: usize,
    period3: usize,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    let max_period = period1.max(period2).max(period3);
    validate_input(high.len(), max_period + 1)?;

    let len = high.len();
    let mut bp = vec![0.0; len];
    let mut tr = vec![0.0; len];

    // Buying pressure / true range pre-pass, batched through the SIMD fast path.
    simd_ops::simd_bp_tr(high, low, close, &mut bp, &mut tr);

    let mut output = init_output(len);

    let mut bp1_sum: f64 = bp[max_period + 1 - period1..=max_period].iter().sum();
    let mut tr1_sum: f64 = tr[max_period + 1 - period1..=max_period].iter().sum();
    let mut bp2_sum: f64 = bp[max_period + 1 - period2..=max_period].iter().sum();
    let mut tr2_sum: f64 = tr[max_period + 1 - period2..=max_period].iter().sum();
    let mut bp3_sum: f64 = bp[max_period + 1 - period3..=max_period].iter().sum();
    let mut tr3_sum: f64 = tr[max_period + 1 - period3..=max_period].iter().sum();

    let avg1 = if tr1_sum.abs() > 1e-15 {
        bp1_sum / tr1_sum
    } else {
        0.0
    };
    let avg2 = if tr2_sum.abs() > 1e-15 {
        bp2_sum / tr2_sum
    } else {
        0.0
    };
    let avg3 = if tr3_sum.abs() > 1e-15 {
        bp3_sum / tr3_sum
    } else {
        0.0
    };
    output[max_period] = 100.0 * (4.0 * avg1 + 2.0 * avg2 + avg3) / 7.0;

    for i in max_period + 1..len {
        bp1_sum += bp[i] - bp[i - period1];
        tr1_sum += tr[i] - tr[i - period1];
        bp2_sum += bp[i] - bp[i - period2];
        tr2_sum += tr[i] - tr[i - period2];
        bp3_sum += bp[i] - bp[i - period3];
        tr3_sum += tr[i] - tr[i - period3];

        let avg1 = if tr1_sum.abs() > 1e-15 {
            bp1_sum / tr1_sum
        } else {
            0.0
        };
        let avg2 = if tr2_sum.abs() > 1e-15 {
            bp2_sum / tr2_sum
        } else {
            0.0
        };
        let avg3 = if tr3_sum.abs() > 1e-15 {
            bp3_sum / tr3_sum
        } else {
            0.0
        };

        output[i] = 100.0 * (4.0 * avg1 + 2.0 * avg2 + avg3) / 7.0;
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// _into zero-copy API variants
// ---------------------------------------------------------------------------

/// MACD zero-copy variant: writes (macd_line, signal, histogram) into pre-allocated slices.
///
/// This is a re-implementation of `macd()` that writes directly into the
/// caller-provided buffers, avoiding the three `Array1` allocations that
/// the array-returning version requires. Use this in hot loops
/// (walk-forward / live trading) to eliminate per-call allocation overhead.
pub fn macd_into(
    input: &[f64],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    macd_line: &mut [f64],
    signal: &mut [f64],
    histogram: &mut [f64],
) -> Result<()> {
    if fast_period >= slow_period {
        return Err(TaError::InvalidParameter {
            name: "fast_period".to_string(),
            constraint: "less than slow_period".to_string(),
        });
    }
    if let Some(idx) = input.iter().position(|v| !v.is_finite()) {
        return Err(TaError::InvalidParameter {
            name: "input".to_string(),
            constraint: format!("non-finite value at index {idx}"),
        });
    }
    validate_input(input.len(), slow_period)?;
    let len = input.len();
    if macd_line.len() != len || signal.len() != len || histogram.len() != len {
        return Err(TaError::InvalidParameter {
            name: "output slices".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }

    // TA-Lib MACD DEFAULT 兼容模式 — 与 macd_inner() 逐位一致。
    // 1. slow EMA 种子 = SMA(input[0..slow_period])
    // 2. fast EMA 种子 = SMA(input[slow-fast..slow])
    // 3. EMA 递推用 FMA: fma(val - prev, k, prev)
    // 4. Signal 种子 = SMA(前 signal_period 个 MACD 值)
    let fast_k = 2.0 / (fast_period as f64 + 1.0);
    let slow_k = 2.0 / (slow_period as f64 + 1.0);
    let signal_k = 2.0 / (signal_period as f64 + 1.0);

    // 累积 slow-only 部分
    let offset = slow_period - fast_period;
    let mut slow_sum: f64 = 0.0;
    for i in 0..offset {
        slow_sum += input[i];
    }
    // 累积共享部分，同时建立 fast 种子
    let mut fast_sum: f64 = 0.0;
    for i in offset..slow_period {
        fast_sum += input[i];
        slow_sum += input[i];
    }
    let mut prev_slow = slow_sum / slow_period as f64;
    let mut prev_fast = fast_sum / fast_period as f64;

    let macd_start = slow_period - 1;

    // 种子点处的 MACD 值
    let mut macd_val = prev_fast - prev_slow;
    macd_line[macd_start] = macd_val;

    // EMA 递推：FMA
    for i in slow_period..len {
        let val = input[i];
        prev_fast = (val - prev_fast).mul_add(fast_k, prev_fast);
        prev_slow = (val - prev_slow).mul_add(slow_k, prev_slow);
        macd_val = prev_fast - prev_slow;
        macd_line[i] = macd_val;
    }

    // Signal line：SMA 种子 + FMA 递推
    let signal_start = macd_start + signal_period - 1;
    if len > signal_start {
        let mut sig_sum: f64 = 0.0;
        for i in macd_start..=signal_start {
            sig_sum += macd_line[i];
        }
        let mut prev_signal = sig_sum / signal_period as f64;
        signal[signal_start] = prev_signal;

        for i in (signal_start + 1)..len {
            let m = macd_line[i];
            prev_signal = (m - prev_signal).mul_add(signal_k, prev_signal);
            signal[i] = prev_signal;
        }

        // Histogram
        for i in signal_start..len {
            histogram[i] = macd_line[i] - signal[i];
        }
    }

    // 预热区填 NaN
    for i in 0..macd_start.min(len) {
        macd_line[i] = f64::NAN;
    }
    for i in 0..signal_start.min(len) {
        signal[i] = f64::NAN;
        histogram[i] = f64::NAN;
    }

    Ok(())
}

/// ADX zero-copy variant: writes result into pre-allocated slice.
pub fn adx_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<()> {
    let result = adx(high, low, close, period)?;
    if output.len() != high.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    output.copy_from_slice(result.as_slice().unwrap());
    Ok(())
}

/// CCI zero-copy variant: writes result into pre-allocated slice.
pub fn cci_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<()> {
    let result = cci(high, low, close, period)?;
    if output.len() != high.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    output.copy_from_slice(result.as_slice().unwrap());
    Ok(())
}

/// Williams %R zero-copy variant: writes result into pre-allocated slice.
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
    validate_input(high.len(), period)?;
    if output.len() != high.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }

    let len = close.len();
    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();
    let close_ptr = close.as_ptr();
    let out_ptr = output.as_mut_ptr();
    let start = period - 1;

    // Initialize output with NaN
    for i in 0..start {
        unsafe {
            *out_ptr.add(i) = f64::NAN;
        }
    }

    // Optimized sliding window with direct index tracking
    unsafe {
        // Initialize first window [0..period-1]
        let mut highest_idx = 0usize;
        let mut lowest_idx = 0usize;
        let mut highest = *high_ptr.add(0);
        let mut lowest = *low_ptr.add(0);

        for k in 1..period {
            let h = *high_ptr.add(k);
            let l = *low_ptr.add(k);
            if h >= highest {
                highest = h;
                highest_idx = k;
            }
            if l <= lowest {
                lowest = l;
                lowest_idx = k;
            }
        }

        // First output at index period-1
        let denom = highest - lowest;
        *out_ptr.add(start) = if denom > 1e-15 {
            (highest - *close_ptr.add(start)) / denom * -100.0
        } else {
            0.0
        };

        // Slide window: [i-period+1..=i]
        for i in period..len {
            let ws = i + 1 - period;
            let new_h = *high_ptr.add(i);
            let new_l = *low_ptr.add(i);

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

            let denom = highest - lowest;
            *out_ptr.add(i) = if denom > 1e-15 {
                (highest - *close_ptr.add(i)) / denom * -100.0
            } else {
                0.0
            };
        }
    }

    Ok(())
}

/// Momentum zero-copy variant: writes result into pre-allocated slice.
pub fn mom_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    let result = mom(input, period)?;
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    output.copy_from_slice(result.as_slice().unwrap());
    Ok(())
}

/// Rate of Change zero-copy variant: writes result into pre-allocated slice.
pub fn roc_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    let result = roc(input, period)?;
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
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

    fn assert_array_matches_slice(a: &Array1<f64>, b: &[f64]) {
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            if x.is_nan() {
                assert!(y.is_nan());
            } else {
                assert_relative_eq!(*x, *y, epsilon = 1e-15);
            }
        }
    }

    #[test]
    fn test_rsi() {
        let input = vec![
            44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 45.5, 45.5, 45.5, 46.0, 45.75, 46.25, 45.5,
            45.25, 46.0, 46.25, 47.0, 47.0, 47.25, 48.25,
        ];
        let result = rsi(&input, 14).unwrap();
        assert!(result[14] > 0.0 && result[14] <= 100.0);
    }

    #[test]
    fn test_rsi_into_matches_rsi() {
        let input = vec![
            44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 45.5, 45.5, 45.5, 46.0, 45.75, 46.25, 45.5,
            45.25, 46.0, 46.25, 47.0, 47.0, 47.25, 48.25,
        ];
        let expected = rsi(&input, 14).unwrap();
        let mut output = vec![0.0; input.len()];
        rsi_into(&input, 14, &mut output).unwrap();
        assert_array_matches_slice(&expected, &output);
    }

    #[test]
    fn test_stoch() {
        let high = vec![10.0, 12.0, 14.0, 16.0, 18.0, 17.0, 16.0, 15.0];
        let low = vec![8.0, 10.0, 12.0, 14.0, 16.0, 15.0, 14.0, 13.0];
        let close = vec![9.0, 11.0, 13.0, 15.0, 17.0, 16.0, 15.0, 14.0];
        let result = stoch(&high, &low, &close, 5, 1, 3).unwrap();
        // TA-Lib lookback = (k_period-1)+(k_slow-1)+(d_period-1) = 6，故首有效索引为 6
        assert!(!result.k[6].is_nan());
        assert!(!result.d[6].is_nan());
    }

    #[test]
    fn test_stoch_into_matches_stoch() {
        let high = vec![10.0, 12.0, 14.0, 16.0, 18.0, 17.0, 16.0, 15.0];
        let low = vec![8.0, 10.0, 12.0, 14.0, 16.0, 15.0, 14.0, 13.0];
        let close = vec![9.0, 11.0, 13.0, 15.0, 17.0, 16.0, 15.0, 14.0];
        let expected = stoch(&high, &low, &close, 5, 1, 3).unwrap();
        let mut k_out = vec![0.0; close.len()];
        let mut d_out = vec![0.0; close.len()];
        stoch_into(&high, &low, &close, 5, 1, 3, &mut k_out, &mut d_out).unwrap();
        assert_array_matches_slice(&expected.k, &k_out);
        assert_array_matches_slice(&expected.d, &d_out);
    }

    #[test]
    fn test_macd() {
        let input: Vec<f64> = (1..=35).map(|x| x as f64).collect();
        let result = macd(&input, 12, 26, 9).unwrap();
        assert!(!result.macd[25].is_nan());
    }

    #[test]
    fn test_mom() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = mom(&input, 2).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 2.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 2.0, epsilon = 1e-10);
        assert_relative_eq!(result[4], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_roc() {
        let input = vec![10.0, 12.0, 15.0];
        let result = roc(&input, 1).unwrap();
        assert!(result[0].is_nan());
        assert_relative_eq!(result[1], 20.0, epsilon = 1e-10);
    }

    #[test]
    fn test_willr() {
        let high = vec![10.0, 12.0, 14.0, 16.0, 18.0];
        let low = vec![8.0, 10.0, 12.0, 14.0, 16.0];
        let close = vec![9.0, 11.0, 13.0, 15.0, 17.0];
        let result = willr(&high, &low, &close, 3).unwrap();
        assert!(!result[2].is_nan());
    }

    #[test]
    fn test_cmo() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0];
        let result = cmo(&input, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[3] > 0.0);
    }

    #[test]
    fn test_trix() {
        let input = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
        ];
        let result = trix(&input, 5).unwrap();
        assert!(result[0].is_nan());
    }

    #[test]
    fn test_apo() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let result = apo(&input, 2, 4).unwrap();
        assert!(result[0].is_nan());
    }

    #[test]
    fn test_apo_fused_matches_sma_diff() {
        // Fused single-pass APO must be bit-identical to sma(fast) - sma(slow).
        let n = 10_000;
        let input: Vec<f64> = (0..n)
            .map(|i| 100.0 + (i as f64 * 0.37).sin() * 2.0 + i as f64 * 0.01)
            .collect();
        for (fast, slow) in [(2usize, 5usize), (12, 26), (1, 3), (30, 60)] {
            let fused = apo(&input, fast, slow).unwrap();
            let f = crate::math::moving_avg::sma(&input, fast).unwrap();
            let s = crate::math::moving_avg::sma(&input, slow).unwrap();
            for i in 0..n {
                let expect = if f[i].is_nan() || s[i].is_nan() {
                    f64::NAN
                } else {
                    f[i] - s[i]
                };
                if expect.is_nan() {
                    assert!(fused[i].is_nan(), "mismatch NaN at {i}");
                } else {
                    assert_eq!(fused[i], expect, "mismatch at {i} fast={fast} slow={slow}");
                }
            }
        }
    }

    #[test]
    fn test_bop() {
        let open = vec![10.0, 11.0, 12.0];
        let high = vec![12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![11.0, 12.0, 13.0];
        let result = bop(&open, &high, &low, &close).unwrap();
        assert_relative_eq!(result[0], 0.333333, epsilon = 1e-4);
    }

    #[test]
    fn test_elder_ray_basic() {
        let high = vec![10.0, 12.0, 14.0, 16.0, 18.0, 17.0, 16.0, 15.0, 14.0, 13.0];
        let low = vec![8.0, 10.0, 12.0, 14.0, 16.0, 15.0, 14.0, 13.0, 12.0, 11.0];
        let close = vec![9.0, 11.0, 13.0, 15.0, 17.0, 16.0, 15.0, 14.0, 13.0, 12.0];
        let volume = vec![
            1000.0, 1200.0, 1400.0, 1600.0, 1800.0, 1500.0, 1300.0, 1100.0, 900.0, 800.0,
        ];
        let result = elder_ray(&high, &low, &close, &volume, 5).unwrap();
        assert!(!result.force_index[1].is_nan());
        assert!(!result.bull_power[4].is_nan());
        assert!(!result.bear_power[4].is_nan());
    }

    #[test]
    fn test_elder_ray_force_index_calculation() {
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5, 14.5];
        let volume = vec![100.0, 200.0, 300.0, 400.0, 500.0, 600.0];
        let result = elder_ray(&high, &low, &close, &volume, 3).unwrap();
        assert_relative_eq!(result.force_index[1], (10.5 - 9.5) * 200.0, epsilon = 1e-10);
        assert_relative_eq!(
            result.force_index[2],
            (11.5 - 10.5) * 300.0,
            epsilon = 1e-10
        );
        assert_relative_eq!(
            result.force_index[5],
            (14.5 - 13.5) * 600.0,
            epsilon = 1e-10
        );
    }

    #[test]
    fn test_elder_ray_bull_bear_power() {
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5];
        let volume = vec![100.0, 200.0, 300.0, 400.0, 500.0];
        let period = 3;
        let result = elder_ray(&high, &low, &close, &volume, period).unwrap();
        for i in 0..(period - 1) {
            assert!(result.bull_power[i].is_nan());
            assert!(result.bear_power[i].is_nan());
        }
        for i in (period - 1)..result.bull_power.len() {
            assert!(!result.bull_power[i].is_nan());
            assert!(!result.bear_power[i].is_nan());
            assert!(result.bull_power[i] > result.bear_power[i]);
        }
    }

    #[test]
    fn test_elder_ray_invalid_input_length() {
        let high = vec![10.0, 11.0, 12.0];
        let low = vec![9.0, 10.0];
        let close = vec![9.5, 10.5, 11.5];
        let volume = vec![100.0, 200.0, 300.0];
        let result = elder_ray(&high, &low, &close, &volume, 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_elder_ray_insufficient_data() {
        let high = vec![10.0, 11.0];
        let low = vec![9.0, 10.0];
        let close = vec![9.5, 10.5];
        let volume = vec![100.0, 200.0];
        let result = elder_ray(&high, &low, &close, &volume, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_elder_ray_zero_volume() {
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0, 12.0, 13.0];
        let close = vec![9.5, 10.5, 11.5, 12.5, 13.5];
        let volume = vec![0.0, 0.0, 0.0, 0.0, 0.0];
        let result = elder_ray(&high, &low, &close, &volume, 3).unwrap();
        assert_relative_eq!(result.force_index[1], 0.0, epsilon = 1e-10);
        assert_relative_eq!(result.force_index[2], 0.0, epsilon = 1e-10);
        assert_relative_eq!(result.force_index[4], 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_elder_ray_negative_force_index() {
        let high = vec![10.0, 11.0, 10.0, 9.0, 8.0];
        let low = vec![9.0, 10.0, 9.0, 8.0, 7.0];
        let close = vec![9.5, 10.5, 9.5, 8.5, 7.5];
        let volume = vec![100.0, 200.0, 300.0, 400.0, 500.0];
        let result = elder_ray(&high, &low, &close, &volume, 3).unwrap();
        assert!(result.force_index[2] < 0.0);
        assert!(result.force_index[3] < 0.0);
        assert!(result.force_index[4] < 0.0);
    }

    #[test]
    fn test_adxr() {
        let high: Vec<f64> = (0..60).map(|i| 100.0 + (i as f64) * 0.5).collect();
        let low: Vec<f64> = (0..60).map(|i| 98.0 + (i as f64) * 0.5).collect();
        let close: Vec<f64> = (0..60).map(|i| 99.0 + (i as f64) * 0.5).collect();
        let result = adxr(&high, &low, &close, 14).unwrap();
        assert_eq!(result.len(), 60);
        assert!(result.iter().any(|&x| !x.is_nan()));
    }

    #[test]
    fn test_aroonosc() {
        let high = vec![10.0, 12.0, 14.0, 13.0, 15.0, 11.0, 16.0, 17.0, 14.0, 13.0];
        let low = vec![8.0, 10.0, 12.0, 11.0, 13.0, 9.0, 14.0, 15.0, 12.0, 11.0];
        let result = aroonosc(&high, &low, 5).unwrap();
        assert_eq!(result.len(), 10);
        assert!(result.iter().skip(4).any(|&x| !x.is_nan()));
    }

    #[test]
    fn test_macdext() {
        let input: Vec<f64> = (1..=30).map(|x| x as f64).collect();
        let result = macdext(&input, 12, MaType::Ema, 26, MaType::Ema, 9, MaType::Ema).unwrap();
        assert_eq!(result.macd.len(), 30);
    }

    #[test]
    fn test_macdfix() {
        let input: Vec<f64> = (1..=40).map(|x| x as f64).collect();
        let result = macdfix(&input).unwrap();
        assert_eq!(result.macd.len(), 40);
    }

    #[test]
    fn test_ppo() {
        let input: Vec<f64> = (1..=30).map(|x| x as f64).collect();
        let result = ppo(&input, 12, 26).unwrap();
        assert_eq!(result.len(), 30);
        assert!(result.iter().skip(25).any(|&x| !x.is_nan()));
    }

    #[test]
    fn test_rocp() {
        let input = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let result = rocp(&input, 1).unwrap();
        assert_relative_eq!(result[1], 0.1, epsilon = 1e-10);
        assert_relative_eq!(result[2], 1.0 / 11.0, epsilon = 1e-10);
    }

    #[test]
    fn test_rocr() {
        let input = vec![10.0, 12.0, 15.0];
        let result = rocr(&input, 1).unwrap();
        assert_relative_eq!(result[1], 1.2, epsilon = 1e-10);
        assert_relative_eq!(result[2], 15.0 / 12.0, epsilon = 1e-10);
    }

    #[test]
    fn test_rocr100() {
        let input = vec![10.0, 12.0, 15.0];
        let result = rocr100(&input, 1).unwrap();
        assert_relative_eq!(result[1], 120.0, epsilon = 1e-10);
    }

    #[test]
    fn test_stochf() {
        let high = vec![10.0, 12.0, 14.0, 13.0, 15.0, 11.0, 16.0, 17.0, 14.0, 13.0];
        let low = vec![8.0, 10.0, 12.0, 11.0, 13.0, 9.0, 14.0, 15.0, 12.0, 11.0];
        let close = vec![9.0, 11.0, 13.0, 12.0, 14.0, 10.0, 15.0, 16.0, 13.0, 12.0];
        let result = stochf(&high, &low, &close, 5, 3).unwrap();
        assert_eq!(result.k.len(), 10);
    }

    #[test]
    fn test_stochrsi() {
        let input: Vec<f64> = (0..50)
            .map(|i| 100.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let result = stochrsi(&input, 14, 14, 3, 3).unwrap();
        assert_eq!(result.k.len(), 50);
    }

    #[test]
    fn test_ultosc() {
        let high: Vec<f64> = (0..40).map(|i| 100.0 + (i as f64) * 0.5).collect();
        let low: Vec<f64> = (0..40).map(|i| 98.0 + (i as f64) * 0.5).collect();
        let close: Vec<f64> = (0..40).map(|i| 99.0 + (i as f64) * 0.5).collect();
        let result = ultosc(&high, &low, &close, 7, 14, 28).unwrap();
        assert_eq!(result.len(), 40);
        assert!(result.iter().skip(28).any(|&x| !x.is_nan()));
    }

    #[test]
    fn test_macd_into_matches_macd() {
        let input: Vec<f64> = (1..=50).map(|x| x as f64).collect();
        let expected = macd(&input, 12, 26, 9).unwrap();
        let mut macd_out = vec![0.0; input.len()];
        let mut signal_out = vec![0.0; input.len()];
        let mut hist_out = vec![0.0; input.len()];
        macd_into(
            &input,
            12,
            26,
            9,
            &mut macd_out,
            &mut signal_out,
            &mut hist_out,
        )
        .unwrap();
        for i in 0..input.len() {
            let em = expected.macd[i];
            let am = macd_out[i];
            if em.is_nan() {
                assert!(am.is_nan());
            } else {
                assert!((em - am).abs() < 1e-9, "macd mismatch at {i}: {em} vs {am}");
            }
            let es = expected.signal[i];
            let as_ = signal_out[i];
            if es.is_nan() {
                assert!(as_.is_nan());
            } else {
                assert!(
                    (es - as_).abs() < 1e-9,
                    "signal mismatch at {i}: {es} vs {as_}"
                );
            }
        }
    }

    // ===========================================================================
    // Zero-copy `_into` variants (B4 / TASK-315)
    //
    // Each `_into` function computes the indicator directly into a caller-owned
    // `&mut [f64]` buffer (zero per-call allocation from the caller's perspective)
    // by delegating to the canonical allocating batch implementation and copying
    // the result. This mirrors the existing `bbands_into`/`dema_into` convention
    // and guarantees numerical parity with the batch API.
    // ===========================================================================

    macro_rules! impl_into_delegate {
    ($name:ident, $batch:path, ($($arg:ident: $t:ty),* $(,)?)) => {
        pub fn $name($($arg: $t,)* output: &mut [f64]) -> Result<()> {
            let result = $batch($($arg),*)?;
            if result.len() != output.len() {
                return Err(TaError::InvalidParameter {
                    name: "output".to_string(),
                    constraint: "must have the same length as the input series".to_string(),
                });
            }
            output.copy_from_slice(result.as_slice().unwrap());
            Ok(())
        }
    };
}

    impl_into_delegate!(apo_into, apo, (input: &[f64], fast_period: usize, slow_period: usize));
    impl_into_delegate!(bop_into, bop, (open: &[f64], high: &[f64], low: &[f64], close: &[f64]));
    impl_into_delegate!(cmo_into, cmo, (input: &[f64], period: usize));
    impl_into_delegate!(dx_into, dx, (high: &[f64], low: &[f64], close: &[f64], period: usize));
    impl_into_delegate!(minus_di_into, minus_di, (high: &[f64], low: &[f64], close: &[f64], period: usize));
    impl_into_delegate!(minus_dm_into, minus_dm, (high: &[f64], low: &[f64]));
    impl_into_delegate!(plus_di_into, plus_di, (high: &[f64], low: &[f64], close: &[f64], period: usize));
    impl_into_delegate!(plus_dm_into, plus_dm, (high: &[f64], low: &[f64]));
    impl_into_delegate!(trix_into, trix, (input: &[f64], period: usize));
    impl_into_delegate!(adxr_into, adxr, (high: &[f64], low: &[f64], close: &[f64], period: usize));
    impl_into_delegate!(aroonosc_into, aroonosc, (high: &[f64], low: &[f64], period: usize));
    impl_into_delegate!(ppo_into, ppo, (input: &[f64], fast_period: usize, slow_period: usize));
    impl_into_delegate!(rocp_into, rocp, (input: &[f64], period: usize));
    impl_into_delegate!(rocr_into, rocr, (input: &[f64], period: usize));
    impl_into_delegate!(rocr100_into, rocr100, (input: &[f64], period: usize));

    #[cfg(test)]
    mod into_tests {
        use super::*;

        fn check_eq(a: &[f64], b: &[f64]) {
            assert_eq!(a.len(), b.len(), "length mismatch");
            for i in 0..a.len() {
                if a[i].is_nan() {
                    assert!(b[i].is_nan(), "nan mismatch at {i}");
                } else {
                    assert!(
                        (a[i] - b[i]).abs() < 1e-12,
                        "value mismatch at {i}: {} vs {}",
                        a[i],
                        b[i]
                    );
                }
            }
        }

        #[test]
        fn test_momentum_into_parity() {
            let input = vec![
                1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0,
            ];
            let high = vec![
                2.0, 3.0, 4.0, 5.0, 6.0, 5.0, 4.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0,
            ];
            let low = vec![
                0.0, 1.0, 2.0, 3.0, 4.0, 3.0, 2.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,
            ];
            let open = vec![
                1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0,
            ];
            let close = input.clone();
            let n = input.len();

            let e = apo(&input, 3, 6).unwrap();
            let mut o = vec![0.0; n];
            apo_into(&input, 3, 6, &mut o).unwrap();
            check_eq(e.as_slice().unwrap(), &o);
            let e = bop(&open, &high, &low, &close).unwrap();
            let mut o = vec![0.0; n];
            bop_into(&open, &high, &low, &close, &mut o).unwrap();
            check_eq(e.as_slice().unwrap(), &o);
            let e = cmo(&input, 5).unwrap();
            let mut o = vec![0.0; n];
            cmo_into(&input, 5, &mut o).unwrap();
            check_eq(e.as_slice().unwrap(), &o);
            let e = dx(&high, &low, &close, 5).unwrap();
            let mut o = vec![0.0; n];
            dx_into(&high, &low, &close, 5, &mut o).unwrap();
            check_eq(e.as_slice().unwrap(), &o);
            let e = minus_di(&high, &low, &close, 5).unwrap();
            let mut o = vec![0.0; n];
            minus_di_into(&high, &low, &close, 5, &mut o).unwrap();
            check_eq(e.as_slice().unwrap(), &o);
            let e = plus_di(&high, &low, &close, 5).unwrap();
            let mut o = vec![0.0; n];
            plus_di_into(&high, &low, &close, 5, &mut o).unwrap();
            check_eq(e.as_slice().unwrap(), &o);
            let e = minus_dm(&high, &low).unwrap();
            let mut o = vec![0.0; n];
            minus_dm_into(&high, &low, &mut o).unwrap();
            check_eq(e.as_slice().unwrap(), &o);
            let e = plus_dm(&high, &low).unwrap();
            let mut o = vec![0.0; n];
            plus_dm_into(&high, &low, &mut o).unwrap();
            check_eq(e.as_slice().unwrap(), &o);
            let e = trix(&input, 5).unwrap();
            let mut o = vec![0.0; n];
            trix_into(&input, 5, &mut o).unwrap();
            check_eq(e.as_slice().unwrap(), &o);
            let e = adxr(&high, &low, &close, 5).unwrap();
            let mut o = vec![0.0; n];
            adxr_into(&high, &low, &close, 5, &mut o).unwrap();
            check_eq(e.as_slice().unwrap(), &o);
            let e = aroonosc(&high, &low, 5).unwrap();
            let mut o = vec![0.0; n];
            aroonosc_into(&high, &low, 5, &mut o).unwrap();
            check_eq(e.as_slice().unwrap(), &o);
            let e = ppo(&input, 3, 6).unwrap();
            let mut o = vec![0.0; n];
            ppo_into(&input, 3, 6, &mut o).unwrap();
            check_eq(e.as_slice().unwrap(), &o);
            let e = rocp(&input, 5).unwrap();
            let mut o = vec![0.0; n];
            rocp_into(&input, 5, &mut o).unwrap();
            check_eq(e.as_slice().unwrap(), &o);
            let e = rocr(&input, 5).unwrap();
            let mut o = vec![0.0; n];
            rocr_into(&input, 5, &mut o).unwrap();
            check_eq(e.as_slice().unwrap(), &o);
            let e = rocr100(&input, 5).unwrap();
            let mut o = vec![0.0; n];
            rocr100_into(&input, 5, &mut o).unwrap();
            check_eq(e.as_slice().unwrap(), &o);
        }
    }
}
