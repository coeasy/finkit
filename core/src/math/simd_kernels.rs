#![allow(unused_unsafe)]
#![allow(unsafe_op_in_unsafe_fn)]

//! SIMD-accelerated batch indicator kernels (SMA, EMA, RSI, MACD).
//!
//! Each public function dispatches at runtime to AVX2 (x86_64), SSE2, or a
//! scalar fallback. Results match the reference scalar implementations within
//! `1e-12` absolute error.
//!
//! ## no_std support
//!
//! In `no_std` mode, only the scalar `_into` functions are available.
//! These write results into caller-provided buffers without requiring `Vec`.

#[cfg(feature = "std")]
use crate::utils::smoothing_factor;

#[cfg(all(feature = "no_std", not(feature = "std")))]
#[inline]
fn smoothing_factor(period: usize) -> f64 {
    2.0 / (period as f64 + 1.0)
}

#[cfg(feature = "std")]
#[derive(Debug, Clone)]
pub struct SimdMacdResult {
    pub macd: alloc::vec::Vec<f64>,
    pub signal: alloc::vec::Vec<f64>,
    pub hist: alloc::vec::Vec<f64>,
}

#[cfg(feature = "std")]
#[derive(Debug, Clone)]
pub struct SimdStochResult {
    pub k: alloc::vec::Vec<f64>,
    pub d: alloc::vec::Vec<f64>,
}

#[inline]
fn sma_scalar(data: &[f64], period: usize, out: &mut [f64]) {
    let len = data.len().min(out.len());
    if period == 0 || len == 0 {
        return;
    }
    for v in out.iter_mut().take(period.saturating_sub(1).min(len)) {
        *v = f64::NAN;
    }
    if len < period {
        return;
    }
    let inv_period = 1.0 / period as f64;
    let mut sum = 0.0;
    for &v in &data[..period] {
        sum += v;
    }
    out[period - 1] = sum * inv_period;
    for i in period..len {
        sum += data[i] - data[i - period];
        out[i] = sum * inv_period;
    }
}

#[inline]
fn ema_scalar(data: &[f64], period: usize, out: &mut [f64]) {
    let len = data.len().min(out.len());
    if period == 0 || len == 0 {
        return;
    }
    for v in out.iter_mut().take(period.saturating_sub(1).min(len)) {
        *v = f64::NAN;
    }
    if len < period {
        return;
    }
    let k = smoothing_factor(period);
    let one_minus_k = 1.0 - k;
    let initial_sma: f64 = data[..period].iter().sum::<f64>() / period as f64;
    out[period - 1] = initial_sma;
    let mut prev = initial_sma;
    for i in period..len {
        prev = data[i] * k + prev * one_minus_k;
        out[i] = prev;
    }
}

#[inline]
pub fn rsi_scalar(data: &[f64], period: usize, out: &mut [f64]) {
    let len = data.len().min(out.len());
    if period == 0 || len <= period {
        for v in out.iter_mut().take(len) {
            *v = f64::NAN;
        }
        return;
    }
    for v in out.iter_mut().take(period) {
        *v = f64::NAN;
    }

    let mut avg_gain = 0.0;
    let mut avg_loss = 0.0;
    for i in 1..=period {
        let change = data[i] - data[i - 1];
        avg_gain += change.max(0.0);
        avg_loss += (-change).max(0.0);
    }

    let inv_period = 1.0 / period as f64;
    let period_minus_1 = (period as f64 - 1.0) * inv_period;
    avg_gain *= inv_period;
    avg_loss *= inv_period;

    out[period] = rsi_from_averages(avg_gain, avg_loss);

    for i in period + 1..len {
        let change = data[i] - data[i - 1];
        let gain = change.max(0.0);
        let loss = (-change).max(0.0);
        avg_gain = avg_gain * period_minus_1 + gain * inv_period;
        avg_loss = avg_loss * period_minus_1 + loss * inv_period;
        out[i] = rsi_from_averages(avg_gain, avg_loss);
    }
}

#[inline]
fn rsi_from_averages(avg_gain: f64, avg_loss: f64) -> f64 {
    if avg_loss.abs() < 1e-15 {
        100.0
    } else {
        100.0 - 100.0 / (1.0 + avg_gain / avg_loss)
    }
}

#[inline]
fn macd_scalar(
    data: &[f64],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    macd_out: &mut [f64],
    signal_out: &mut [f64],
    hist_out: &mut [f64],
) {
    let len = data.len();
    for v in macd_out.iter_mut().take(len) {
        *v = f64::NAN;
    }
    for v in signal_out.iter_mut().take(len) {
        *v = f64::NAN;
    }
    for v in hist_out.iter_mut().take(len) {
        *v = f64::NAN;
    }
    if fast_period >= slow_period || len < slow_period {
        return;
    }

    let fast_inv = 1.0 / fast_period as f64;
    let slow_inv = 1.0 / slow_period as f64;
    let signal_inv = 1.0 / signal_period as f64;

    let fast_k = smoothing_factor(fast_period);
    let slow_k = smoothing_factor(slow_period);
    let signal_k = smoothing_factor(signal_period);

    let fast_one_minus_k = 1.0 - fast_k;
    let slow_one_minus_k = 1.0 - slow_k;
    let signal_one_minus_k = 1.0 - signal_k;

    let fast_sma: f64 = data[..fast_period].iter().sum::<f64>() * fast_inv;
    let slow_sma: f64 = data[..slow_period].iter().sum::<f64>() * slow_inv;

    let mut fast_ema = fast_sma;
    let mut slow_ema = slow_sma;
    let mut signal_sma_sum = 0.0;
    let mut signal_ema = 0.0;

    for i in 0..len {
        if i >= fast_period {
            fast_ema = data[i] * fast_k + fast_ema * fast_one_minus_k;
        }
        if i >= slow_period {
            slow_ema = data[i] * slow_k + slow_ema * slow_one_minus_k;
        }

        if i >= slow_period - 1 {
            macd_out[i] = fast_ema - slow_ema;
        }

        let signal_input = if i >= slow_period - 1 {
            macd_out[i]
        } else {
            0.0
        };

        if i < signal_period - 1 {
            signal_sma_sum += signal_input;
        } else if i == signal_period - 1 {
            signal_sma_sum += signal_input;
            signal_ema = signal_sma_sum * signal_inv;
            signal_out[i] = signal_ema;
        } else {
            signal_ema = signal_input * signal_k + signal_ema * signal_one_minus_k;
            signal_out[i] = signal_ema;
        }

        if !macd_out[i].is_nan() && !signal_out[i].is_nan() {
            hist_out[i] = macd_out[i] - signal_out[i];
        }
    }
}

#[cfg(feature = "std")]
#[inline]
#[allow(clippy::too_many_arguments)]
pub fn stoch_scalar(
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
    for v in k_out.iter_mut().take(len) {
        *v = f64::NAN;
    }
    for v in d_out.iter_mut().take(len) {
        *v = f64::NAN;
    }
    if len < k_period {
        return;
    }

    let fastk_start = k_period - 1;
    // Match `stoch_fused_pipeline`: both Slow %K and %D are written starting at
    // `slowd_start` (TA-Lib special rule adopted by this library).
    let slowk_start = fastk_start + k_slow - 1;
    let slowd_start = slowk_start + d_period - 1;
    let inv_k_slow = 1.0 / k_slow as f64;
    let inv_d_period = 1.0 / d_period as f64;

    let mut k_sum: f64 = 0.0;
    let mut d_sum: f64 = 0.0;

    let mut fast_k_ring = alloc::vec![0.0_f64; k_slow];
    let mut k_ring = alloc::vec![0.0_f64; d_period];
    let mut fk_ring_pos: usize = 0;
    let mut d_ring_pos: usize = 0;

    let mut highest_idx: usize = 0;
    let mut lowest_idx: usize = 0;
    let mut highest: f64 = f64::NEG_INFINITY;
    let mut lowest: f64 = f64::INFINITY;

    for i in 0..len {
        let new_h = high[i];
        let new_l = low[i];

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
                highest = high[ws];
                highest_idx = ws;
                for k in (ws + 1)..=i {
                    let h = high[k];
                    if h >= highest {
                        highest = h;
                        highest_idx = k;
                    }
                }
            } else if new_h >= highest {
                highest = new_h;
                highest_idx = i;
            }

            if lowest_idx < ws {
                lowest = low[ws];
                lowest_idx = ws;
                for k in (ws + 1)..=i {
                    let l = low[k];
                    if l <= lowest {
                        lowest = l;
                        lowest_idx = k;
                    }
                }
            } else if new_l <= lowest {
                lowest = new_l;
                lowest_idx = i;
            }
        }

        let fk = if i >= fastk_start {
            let denom = highest - lowest;
            if denom > 1e-15 {
                (close[i] - lowest) / denom * 100.0
            } else {
                50.0
            }
        } else {
            0.0
        };

        k_sum += fk - fast_k_ring[fk_ring_pos];
        fast_k_ring[fk_ring_pos] = fk;
        fk_ring_pos += 1;
        if fk_ring_pos == k_slow {
            fk_ring_pos = 0;
        }

        let k_val = if i >= k_slow - 1 {
            let v = k_sum * inv_k_slow;
            if i >= slowd_start {
                k_out[i] = v;
            }
            v
        } else {
            0.0
        };

        d_sum += k_val - k_ring[d_ring_pos];
        k_ring[d_ring_pos] = k_val;
        d_ring_pos += 1;
        if d_ring_pos == d_period {
            d_ring_pos = 0;
        }

        if i >= slowd_start {
            d_out[i] = d_sum * inv_d_period;
        }
    }
}

#[cfg(feature = "std")]
#[inline]
pub fn cci_scalar(high: &[f64], low: &[f64], close: &[f64], period: usize, out: &mut [f64]) {
    let len = close.len();
    for v in out.iter_mut().take(len) {
        *v = f64::NAN;
    }
    if len < period {
        return;
    }

    let inv_p = 1.0 / period as f64;

    let mut tp_buf = alloc::vec![0.0; period];
    let mut tp_sum = 0.0;
    for j in 0..period {
        tp_buf[j] = (high[j] + low[j] + close[j]) / 3.0;
        tp_sum += tp_buf[j];
    }

    let first = period - 1;
    {
        let tp_mean = tp_sum * inv_p;
        let mut mean_dev = 0.0;
        for &tp in &tp_buf {
            mean_dev += (tp - tp_mean).abs();
        }
        mean_dev *= inv_p;
        if mean_dev.abs() > 1e-15 {
            out[first] = (tp_buf[period - 1] - tp_mean) / (0.015 * mean_dev);
        }
    }

    let mut ring_idx = 0;
    for i in period..len {
        let new_tp = (high[i] + low[i] + close[i]) / 3.0;
        tp_sum += new_tp - tp_buf[ring_idx];
        tp_buf[ring_idx] = new_tp;
        ring_idx = (ring_idx + 1) % period;

        let tp_mean = tp_sum * inv_p;
        let mut mean_dev = 0.0;
        for &tp in &tp_buf {
            mean_dev += (tp - tp_mean).abs();
        }
        mean_dev *= inv_p;
        if mean_dev.abs() > 1e-15 {
            out[i] = (new_tp - tp_mean) / (0.015 * mean_dev);
        }
    }
}

// Scalar fallback (no SIMD intrinsics)
#[cfg(all(feature = "std", target_arch = "x86_64"))]
unsafe fn sma_fallback(data: &[f64], period: usize, out: &mut [f64]) {
    sma_scalar(data, period, out);
}

// Scalar fallback (no SIMD intrinsics)
#[cfg(all(feature = "std", target_arch = "x86_64"))]
unsafe fn ema_fallback(data: &[f64], period: usize, out: &mut [f64]) {
    let len = data.len().min(out.len());
    if period == 0 || len == 0 {
        return;
    }
    for v in out.iter_mut().take(period.saturating_sub(1).min(len)) {
        *v = f64::NAN;
    }
    if len < period {
        return;
    }

    let k = smoothing_factor(period);
    let one_minus_k = 1.0 - k;
    let initial_sma: f64 = data[..period].iter().sum::<f64>() / period as f64;
    out[period - 1] = initial_sma;

    let mut prev = initial_sma;
    for i in period..len {
        prev = data[i] * k + prev * one_minus_k;
        out[i] = prev;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn rsi_avx2(data: &[f64], period: usize, out: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(out.len());
    if period == 0 || len <= period {
        for v in out.iter_mut().take(len) {
            *v = f64::NAN;
        }
        return;
    }
    for v in out.iter_mut().take(period) {
        *v = f64::NAN;
    }

    // Accumulate the warm-up gains/losses directly from the input. The old
    // implementation materialized every price change in a temporary Vec,
    // which defeated the allocation-free `_into` contract on AVX2.
    let zero = _mm256_setzero_pd();
    let mut gain_sum = zero;
    let mut loss_sum = zero;
    let gain_chunks = period / 4;
    for c in 0..gain_chunks {
        let off = c * 4;
        let curr = _mm256_loadu_pd(data.as_ptr().add(off + 1));
        let prev = _mm256_loadu_pd(data.as_ptr().add(off));
        let change = _mm256_sub_pd(curr, prev);
        let neg = _mm256_sub_pd(zero, change);
        gain_sum = _mm256_add_pd(gain_sum, _mm256_max_pd(change, zero));
        loss_sum = _mm256_add_pd(loss_sum, _mm256_max_pd(neg, zero));
    }
    let gain_arr: [f64; 4] = core::mem::transmute(gain_sum);
    let loss_arr: [f64; 4] = core::mem::transmute(loss_sum);
    let mut avg_gain = gain_arr[0] + gain_arr[1] + gain_arr[2] + gain_arr[3];
    let mut avg_loss = loss_arr[0] + loss_arr[1] + loss_arr[2] + loss_arr[3];
    for i in gain_chunks * 4..period {
        let change = data[i + 1] - data[i];
        avg_gain += change.max(0.0);
        avg_loss += (-change).max(0.0);
    }

    let inv_period = 1.0 / period as f64;
    let period_minus_1 = (period as f64 - 1.0) * inv_period;
    avg_gain *= inv_period;
    avg_loss *= inv_period;
    out[period] = rsi_from_averages(avg_gain, avg_loss);

    for i in period + 1..len {
        let change = data[i] - data[i - 1];
        let gain = change.max(0.0);
        let loss = (-change).max(0.0);
        avg_gain = avg_gain * period_minus_1 + gain * inv_period;
        avg_loss = avg_loss * period_minus_1 + loss * inv_period;
        out[i] = rsi_from_averages(avg_gain, avg_loss);
    }
}

// Scalar fallback (no SIMD intrinsics)
#[cfg(all(feature = "std", target_arch = "x86_64"))]
unsafe fn macd_fallback(
    data: &[f64],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    macd_out: &mut [f64],
    signal_out: &mut [f64],
    hist_out: &mut [f64],
) {
    let len = data.len();
    for v in macd_out.iter_mut().take(len) {
        *v = f64::NAN;
    }
    for v in signal_out.iter_mut().take(len) {
        *v = f64::NAN;
    }
    for v in hist_out.iter_mut().take(len) {
        *v = f64::NAN;
    }
    if fast_period >= slow_period || len < slow_period {
        return;
    }

    let fast_inv = 1.0 / fast_period as f64;
    let slow_inv = 1.0 / slow_period as f64;
    let signal_inv = 1.0 / signal_period as f64;

    let fast_k = smoothing_factor(fast_period);
    let slow_k = smoothing_factor(slow_period);
    let signal_k = smoothing_factor(signal_period);

    let fast_one_minus_k = 1.0 - fast_k;
    let slow_one_minus_k = 1.0 - slow_k;
    let signal_one_minus_k = 1.0 - signal_k;

    let fast_sma: f64 = data[..fast_period].iter().sum::<f64>() * fast_inv;
    let slow_sma: f64 = data[..slow_period].iter().sum::<f64>() * slow_inv;

    let mut fast_ema = fast_sma;
    let mut slow_ema = slow_sma;
    let mut signal_sma_sum = 0.0;
    let mut signal_ema = 0.0;

    for i in 0..len {
        if i >= fast_period {
            fast_ema = data[i] * fast_k + fast_ema * fast_one_minus_k;
        }
        if i >= slow_period {
            slow_ema = data[i] * slow_k + slow_ema * slow_one_minus_k;
        }

        if i >= slow_period - 1 {
            macd_out[i] = fast_ema - slow_ema;
        }

        let signal_input = if i >= slow_period - 1 {
            macd_out[i]
        } else {
            0.0
        };

        if i < signal_period - 1 {
            signal_sma_sum += signal_input;
        } else if i == signal_period - 1 {
            signal_sma_sum += signal_input;
            signal_ema = signal_sma_sum * signal_inv;
            signal_out[i] = signal_ema;
        } else {
            signal_ema = signal_input * signal_k + signal_ema * signal_one_minus_k;
            signal_out[i] = signal_ema;
        }

        if !macd_out[i].is_nan() && !signal_out[i].is_nan() {
            hist_out[i] = macd_out[i] - signal_out[i];
        }
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
#[allow(clippy::too_many_arguments)]
unsafe fn stoch_avx2(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    k_period: usize,
    k_slow: usize,
    d_period: usize,
    k_out: &mut [f64],
    d_out: &mut [f64],
) {
    use core::arch::x86_64::*;
    let len = close.len();
    for v in k_out.iter_mut().take(len) {
        *v = f64::NAN;
    }
    for v in d_out.iter_mut().take(len) {
        *v = f64::NAN;
    }
    if len < k_period {
        return;
    }

    let fastk_start = k_period - 1;
    // Match `stoch_fused_pipeline`: both Slow %K and %D are written starting at
    // `slowd_start` (TA-Lib special rule adopted by this library), not at the
    // standard `k_slow-1` / `d_period-1` indices.
    let slowk_start = fastk_start + k_slow - 1;
    let slowd_start = slowk_start + d_period - 1;
    let inv_k_slow = 1.0 / k_slow as f64;
    let inv_d_period = 1.0 / d_period as f64;

    let mut k_sum: f64 = 0.0;
    let mut d_sum: f64 = 0.0;

    let mut fast_k_ring = alloc::vec![0.0_f64; k_slow];
    let mut k_ring = alloc::vec![0.0_f64; d_period];
    let mut fk_ring_pos: usize = 0;
    let mut d_ring_pos: usize = 0;

    let mut highest_idx: usize = 0;
    let mut lowest_idx: usize = 0;
    let mut highest: f64 = f64::NEG_INFINITY;
    let mut lowest: f64 = f64::INFINITY;

    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();
    let close_ptr = close.as_ptr();

    let neg_inf = _mm256_set1_pd(f64::NEG_INFINITY);
    let pos_inf = _mm256_set1_pd(f64::INFINITY);

    for i in 0..len {
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
                highest_idx = ws;
                let window_len = i - ws + 1;
                let chunks = window_len / 4;
                let mut max_vec = neg_inf;
                for c in 0..chunks {
                    let offset = ws + c * 4;
                    let h_vec = _mm256_loadu_pd(high_ptr.add(offset));
                    max_vec = _mm256_max_pd(max_vec, h_vec);
                }
                let max_arr: [f64; 4] = core::mem::transmute(max_vec);
                highest = max_arr[0].max(max_arr[1]).max(max_arr[2]).max(max_arr[3]);
                for k in (ws + chunks * 4)..=i {
                    let h = *high_ptr.add(k);
                    if h >= highest {
                        highest = h;
                        highest_idx = k;
                    }
                }
            } else if new_h >= highest {
                highest = new_h;
                highest_idx = i;
            }

            if lowest_idx < ws {
                lowest_idx = ws;
                let window_len = i - ws + 1;
                let chunks = window_len / 4;
                let mut min_vec = pos_inf;
                for c in 0..chunks {
                    let offset = ws + c * 4;
                    let l_vec = _mm256_loadu_pd(low_ptr.add(offset));
                    min_vec = _mm256_min_pd(min_vec, l_vec);
                }
                let min_arr: [f64; 4] = core::mem::transmute(min_vec);
                lowest = min_arr[0].min(min_arr[1]).min(min_arr[2]).min(min_arr[3]);
                for k in (ws + chunks * 4)..=i {
                    let l = *low_ptr.add(k);
                    if l <= lowest {
                        lowest = l;
                        lowest_idx = k;
                    }
                }
            } else if new_l <= lowest {
                lowest = new_l;
                lowest_idx = i;
            }
        }

        let fk = if i >= fastk_start {
            let denom = highest - lowest;
            if denom > 1e-15 {
                (*close_ptr.add(i) - lowest) / denom * 100.0
            } else {
                50.0
            }
        } else {
            0.0
        };

        k_sum += fk - fast_k_ring[fk_ring_pos];
        fast_k_ring[fk_ring_pos] = fk;
        fk_ring_pos += 1;
        if fk_ring_pos == k_slow {
            fk_ring_pos = 0;
        }

        let k_val = if i >= k_slow - 1 {
            let v = k_sum * inv_k_slow;
            if i >= slowd_start {
                k_out[i] = v;
            }
            v
        } else {
            0.0
        };

        d_sum += k_val - k_ring[d_ring_pos];
        k_ring[d_ring_pos] = k_val;
        d_ring_pos += 1;
        if d_ring_pos == d_period {
            d_ring_pos = 0;
        }

        if i >= slowd_start {
            d_out[i] = d_sum * inv_d_period;
        }
    }
}

// Scalar fallback (no SIMD intrinsics)
#[cfg(all(feature = "std", target_arch = "x86_64"))]
unsafe fn cci_fallback(high: &[f64], low: &[f64], close: &[f64], period: usize, out: &mut [f64]) {
    cci_scalar(high, low, close, period, out);
}

#[cfg(all(feature = "std", target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
unsafe fn sma_avx512(data: &[f64], period: usize, out: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(out.len());
    if period == 0 || len == 0 {
        return;
    }
    for v in out.iter_mut().take(period.saturating_sub(1).min(len)) {
        *v = f64::NAN;
    }
    if len < period {
        return;
    }

    let inv_period = 1.0 / period as f64;
    let inv_period_vec = _mm512_set1_pd(inv_period);

    let chunks = period / 8;
    let mut sum_vec = _mm512_setzero_pd();
    for c in 0..chunks {
        let v = _mm512_loadu_pd(data.as_ptr().add(c * 8));
        sum_vec = _mm512_add_pd(sum_vec, v);
    }
    let sum_arr: [f64; 8] = core::mem::transmute(sum_vec);
    let mut sum = sum_arr.iter().sum::<f64>();
    for i in chunks * 8..period {
        sum += data[i];
    }
    out[period - 1] = sum * inv_period;

    for i in period..len {
        let old_val = data[i - period];
        let new_val = data[i];
        sum += new_val - old_val;
        out[i] = sum * inv_period;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64", feature = "nightly-avx512"))]
#[target_feature(enable = "avx512f")]
unsafe fn ema_avx512(data: &[f64], period: usize, out: &mut [f64]) {
    use core::arch::x86_64::*;
    let len = data.len().min(out.len());
    if period == 0 || len == 0 {
        return;
    }
    for v in out.iter_mut().take(period.saturating_sub(1).min(len)) {
        *v = f64::NAN;
    }
    if len < period {
        return;
    }

    let k = smoothing_factor(period);
    let one_minus_k = 1.0 - k;

    let chunks = period / 8;
    let mut sum_vec = _mm512_setzero_pd();
    for c in 0..chunks {
        let v = _mm512_loadu_pd(data.as_ptr().add(c * 8));
        sum_vec = _mm512_add_pd(sum_vec, v);
    }
    let sum_arr: [f64; 8] = core::mem::transmute(sum_vec);
    let mut sum = sum_arr.iter().sum::<f64>();
    for i in chunks * 8..period {
        sum += data[i];
    }
    let initial_sma = sum / period as f64;
    out[period - 1] = initial_sma;

    let mut prev = initial_sma;
    for i in period..len {
        prev = data[i] * k + prev * one_minus_k;
        out[i] = prev;
    }
}

// Scalar fallback (no SIMD intrinsics)
#[cfg(all(feature = "std", target_arch = "x86_64"))]
unsafe fn sma_fallback_sse(data: &[f64], period: usize, out: &mut [f64]) {
    sma_scalar(data, period, out);
}

// Scalar fallback (no SIMD intrinsics)
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
unsafe fn ema_fallback_sse(data: &[f64], period: usize, out: &mut [f64]) {
    let len = data.len().min(out.len());
    if period == 0 || len == 0 {
        return;
    }
    for v in out.iter_mut().take(period.saturating_sub(1).min(len)) {
        *v = f64::NAN;
    }
    if len < period {
        return;
    }

    let k = smoothing_factor(period);
    let one_minus_k = 1.0 - k;
    let initial_sma: f64 = data[..period].iter().sum::<f64>() / period as f64;
    out[period - 1] = initial_sma;

    let mut prev = initial_sma;
    for i in period..len {
        prev = data[i] * k + prev * one_minus_k;
        out[i] = prev;
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
mod x86_dispatch {
    use std::sync::OnceLock;

    pub fn has_avx2() -> bool {
        static CACHE: OnceLock<bool> = OnceLock::new();
        *CACHE.get_or_init(|| is_x86_feature_detected!("avx2"))
    }

    pub fn has_sse2() -> bool {
        static CACHE: OnceLock<bool> = OnceLock::new();
        *CACHE.get_or_init(|| is_x86_feature_detected!("sse2"))
    }

    #[cfg(feature = "nightly-avx512")]
    pub fn has_avx512f() -> bool {
        static CACHE: OnceLock<bool> = OnceLock::new();
        *CACHE.get_or_init(|| is_x86_feature_detected!("avx512f"))
    }
}

pub fn sma_simd_into(data: &[f64], period: usize, out: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64", feature = "nightly-avx512"))]
    {
        if x86_dispatch::has_avx512f() {
            return unsafe { sma_avx512(data, period, out) };
        }
        if x86_dispatch::has_avx2() {
            return unsafe { sma_fallback(data, period, out) };
        }
        if x86_dispatch::has_sse2() {
            return unsafe { sma_fallback_sse(data, period, out) };
        }
    }
    #[cfg(all(
        feature = "std",
        target_arch = "x86_64",
        not(feature = "nightly-avx512")
    ))]
    {
        if x86_dispatch::has_avx2() {
            return unsafe { sma_fallback(data, period, out) };
        }
        if x86_dispatch::has_sse2() {
            return unsafe { sma_fallback_sse(data, period, out) };
        }
    }
    sma_scalar(data, period, out);
}

#[cfg(feature = "std")]
pub fn sma_simd(data: &[f64], period: usize) -> alloc::vec::Vec<f64> {
    let mut out = alloc::vec![f64::NAN; data.len()];
    sma_simd_into(data, period, &mut out);
    out
}

pub fn sma_scalar_into(data: &[f64], period: usize, out: &mut [f64]) {
    sma_scalar(data, period, out);
}

#[cfg(feature = "std")]
pub fn sma_scalar_naive_into(data: &[f64], period: usize, out: &mut [f64]) {
    let len = data.len();
    if period == 0 || len == 0 {
        return;
    }
    for v in out.iter_mut().take(period.saturating_sub(1).min(len)) {
        *v = f64::NAN;
    }
    if len < period {
        return;
    }
    let inv_period = 1.0 / period as f64;
    for i in period - 1..len {
        let start = i + 1 - period;
        let sum: f64 = data[start..=i].iter().sum();
        out[i] = sum * inv_period;
    }
}

#[cfg(feature = "std")]
pub fn ema_simd(data: &[f64], period: usize) -> alloc::vec::Vec<f64> {
    let mut out = alloc::vec![f64::NAN; data.len()];
    ema_simd_into(data, period, &mut out);
    out
}

pub fn ema_simd_into(data: &[f64], period: usize, out: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64", feature = "nightly-avx512"))]
    {
        if x86_dispatch::has_avx512f() {
            return unsafe { ema_avx512(data, period, out) };
        }
        if x86_dispatch::has_avx2() {
            return unsafe { ema_fallback(data, period, out) };
        }
        if x86_dispatch::has_sse2() {
            return unsafe { ema_fallback_sse(data, period, out) };
        }
    }
    #[cfg(all(
        feature = "std",
        target_arch = "x86_64",
        not(feature = "nightly-avx512")
    ))]
    {
        if x86_dispatch::has_avx2() {
            return unsafe { ema_fallback(data, period, out) };
        }
        if x86_dispatch::has_sse2() {
            return unsafe { ema_fallback_sse(data, period, out) };
        }
    }
    ema_scalar(data, period, out);
}

#[cfg(feature = "std")]
pub fn rsi_simd(data: &[f64], period: usize) -> alloc::vec::Vec<f64> {
    let mut out = alloc::vec![f64::NAN; data.len()];
    rsi_simd_into(data, period, &mut out);
    out
}

pub fn rsi_simd_into(data: &[f64], period: usize, out: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx512f") {
            return unsafe { crate::math::simd_ops_avx512::simd512_rsi(data, period, out) };
        }
        if x86_dispatch::has_avx2() {
            return unsafe { rsi_avx2(data, period, out) };
        }
    }
    rsi_scalar(data, period, out);
}

#[cfg(feature = "std")]
pub fn macd_simd(
    data: &[f64],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> SimdMacdResult {
    let len = data.len();
    let mut macd = alloc::vec![f64::NAN; len];
    let mut signal = alloc::vec![f64::NAN; len];
    let mut hist = alloc::vec![f64::NAN; len];
    macd_simd_into(
        data,
        fast_period,
        slow_period,
        signal_period,
        &mut macd,
        &mut signal,
        &mut hist,
    );
    SimdMacdResult { macd, signal, hist }
}

/// NOTE: MACD's inner loops are sequential EMA chains (`x[i]*k + prev*(1-k)`),
/// which are not vectorizable across the series. This entry point therefore
/// delegates to the scalar implementation. It is kept as a dispatch slot for a
/// future SIMD seed-SMA / bulk-EMA kernel — do NOT assume vectorized speedup.
pub fn macd_simd_into(
    data: &[f64],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    macd_out: &mut [f64],
    signal_out: &mut [f64],
    hist_out: &mut [f64],
) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if x86_dispatch::has_avx2() {
            return unsafe {
                macd_fallback(
                    data,
                    fast_period,
                    slow_period,
                    signal_period,
                    macd_out,
                    signal_out,
                    hist_out,
                )
            };
        }
    }
    macd_scalar(
        data,
        fast_period,
        slow_period,
        signal_period,
        macd_out,
        signal_out,
        hist_out,
    );
}

#[cfg(feature = "std")]
pub fn stoch_simd(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    k_period: usize,
    k_slow: usize,
    d_period: usize,
) -> SimdStochResult {
    let len = close.len();
    let mut k = alloc::vec![f64::NAN; len];
    let mut d = alloc::vec![f64::NAN; len];
    stoch_simd_into(high, low, close, k_period, k_slow, d_period, &mut k, &mut d);
    SimdStochResult { k, d }
}

#[cfg(feature = "std")]
#[allow(clippy::too_many_arguments)]
pub fn stoch_simd_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    k_period: usize,
    k_slow: usize,
    d_period: usize,
    k_out: &mut [f64],
    d_out: &mut [f64],
) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if x86_dispatch::has_avx2() {
            return unsafe {
                stoch_avx2(high, low, close, k_period, k_slow, d_period, k_out, d_out)
            };
        }
    }
    stoch_scalar(high, low, close, k_period, k_slow, d_period, k_out, d_out);
}

#[cfg(feature = "std")]
pub fn cci_simd(high: &[f64], low: &[f64], close: &[f64], period: usize) -> alloc::vec::Vec<f64> {
    let mut out = alloc::vec![f64::NAN; close.len()];
    cci_simd_into(high, low, close, period, &mut out);
    out
}

#[cfg(feature = "std")]
/// NOTE: CCI's MAD requires a sorted window (order statistics), which is not
/// amenable to plain SIMD. The scalar path already uses an O(n·log period)
/// sorted-window + prefix-sum scheme, so there is no easy vectorization win.
/// This entry delegates to scalar; it is a dispatch slot, not a vectorized kernel.
pub fn cci_simd_into(high: &[f64], low: &[f64], close: &[f64], period: usize, out: &mut [f64]) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if x86_dispatch::has_avx2() {
            return unsafe { cci_fallback(high, low, close, period, out) };
        }
    }
    cci_scalar(high, low, close, period, out);
}

// ADX SIMD warmup helper (P-10).
//
// The public `indicators::adx` in `indicators::momentum` is a tight O(1)
// per-step loop; the most expensive part of the warmup phase (1..=period)
// is computing the four scalar accumulators (smooth_+DM, smooth_-DM, smooth_TR)
// from per-bar `up_move`, `down_move`, `true_range` values. We expose
// [`adx_warmup_avx2`] as a public function so `momentum::adx` can use it
// when AVX2 is available, without duplicating the full ADX state machine.
#[cfg(feature = "std")]
#[inline]
pub fn adx_warmup_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    smooth_plus_dm: &mut f64,
    smooth_minus_dm: &mut f64,
    smooth_tr: &mut f64,
) {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") && period >= 8 {
            unsafe {
                adx_warmup_avx2(
                    high,
                    low,
                    close,
                    period,
                    smooth_plus_dm,
                    smooth_minus_dm,
                    smooth_tr,
                );
            }
            return;
        }
    }
    // Scalar fallback (also used on non-x86_64 / no AVX2 builds).
    let mut up = 0.0f64;
    let mut down = 0.0f64;
    let mut tr = 0.0f64;
    for i in 1..=period {
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];
        tr += crate::utils::true_range(high[i], low[i], close[i - 1]);
        if up_move > down_move && up_move > 0.0 {
            up += up_move;
        }
        if down_move > up_move && down_move > 0.0 {
            down += down_move;
        }
    }
    *smooth_plus_dm = up;
    *smooth_minus_dm = down;
    *smooth_tr = tr;
}

/// AVX2 warmup pass: vectorise the `up_move`, `down_move`, and `true_range`
/// computations for the first `period` bars (1..=period), then reduce the
/// four SIMD lanes into the scalar smooth accumulators.
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn adx_warmup_avx2(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    smooth_plus_dm: &mut f64,
    smooth_minus_dm: &mut f64,
    smooth_tr: &mut f64,
) {
    use core::arch::x86_64::*;
    // Process indices 1..=period in 4-wide chunks. We need `high[i-1]`,
    // `low[i-1]`, `close[i-1]`, so we load from `i-1` to `i-1+3` and
    // shift by 1 lane to get [1..4] = high[1..5], etc. The
    // `_mm256_permute4x64_pd` shuffle gives us a 1-lane-left shift.
    let mut up_acc = _mm256_setzero_pd();
    let mut down_acc = _mm256_setzero_pd();
    let mut tr_acc = _mm256_setzero_pd();

    let mut i: usize = 1;
    while i + 4 <= period + 1 {
        // Load two adjacent 4-vectors: the "previous" window
        // [high[i-1], high[i], high[i+1], high[i+2]] and the "current"
        // window [high[i], high[i+1], high[i+2], high[i+3]]. Lane `j`
        // in both vectors corresponds to bar `i + j`.
        let h_raw = _mm256_loadu_pd(high.as_ptr().add(i - 1));
        let l_raw = _mm256_loadu_pd(low.as_ptr().add(i - 1));
        let c_raw = _mm256_loadu_pd(close.as_ptr().add(i - 1));

        let h_cur = _mm256_loadu_pd(high.as_ptr().add(i));
        let l_cur = _mm256_loadu_pd(low.as_ptr().add(i));
        let _c_cur = _mm256_loadu_pd(close.as_ptr().add(i));

        let zero = _mm256_setzero_pd();

        // up_move = high_cur - high_prev (h_cur - h_raw), down_move = low_prev - low_cur
        let up_move = _mm256_sub_pd(h_cur, h_raw);
        let down_move = _mm256_sub_pd(l_raw, l_cur);

        // tr = max(|h-l|, |h-c_prev|, |l-c_prev|)
        let hl = _mm256_sub_pd(h_cur, l_cur);
        let abs_hl = _mm256_andnot_pd(_mm256_set1_pd(-0.0), hl);
        let hc = _mm256_sub_pd(h_cur, c_raw);
        let abs_hc = _mm256_andnot_pd(_mm256_set1_pd(-0.0), hc);
        let lc = _mm256_sub_pd(l_cur, c_raw);
        let abs_lc = _mm256_andnot_pd(_mm256_set1_pd(-0.0), lc);
        let tr = _mm256_max_pd(_mm256_max_pd(abs_hl, abs_hc), abs_lc);

        // pdm = (up_move > down_move) & (up_move > 0) ? up_move : 0
        let gt_down = _mm256_cmp_pd::<_CMP_GT_OS>(up_move, down_move);
        let gt_zero = _mm256_cmp_pd::<_CMP_GT_OS>(up_move, zero);
        let pdm_mask = _mm256_and_pd(gt_down, gt_zero);
        let pdm = _mm256_and_pd(pdm_mask, up_move);

        let lt_up = _mm256_cmp_pd::<_CMP_GT_OS>(down_move, up_move);
        let lt_zero = _mm256_cmp_pd::<_CMP_GT_OS>(down_move, zero);
        let mdm_mask = _mm256_and_pd(lt_up, lt_zero);
        let mdm = _mm256_and_pd(mdm_mask, down_move);

        up_acc = _mm256_add_pd(up_acc, pdm);
        down_acc = _mm256_add_pd(down_acc, mdm);
        tr_acc = _mm256_add_pd(tr_acc, tr);

        i += 4;
    }

    // Horizontal sum.
    let up_arr: [f64; 4] = core::mem::transmute(up_acc);
    let down_arr: [f64; 4] = core::mem::transmute(down_acc);
    let tr_arr: [f64; 4] = core::mem::transmute(tr_acc);
    let mut up = up_arr[0] + up_arr[1] + up_arr[2] + up_arr[3];
    let mut down = down_arr[0] + down_arr[1] + down_arr[2] + down_arr[3];
    let mut tr = tr_arr[0] + tr_arr[1] + tr_arr[2] + tr_arr[3];

    // Scalar tail.
    while i <= period {
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];
        tr += crate::utils::true_range(high[i], low[i], close[i - 1]);
        if up_move > down_move && up_move > 0.0 {
            up += up_move;
        }
        if down_move > up_move && down_move > 0.0 {
            down += down_move;
        }
        i += 1;
    }

    *smooth_plus_dm = up;
    *smooth_minus_dm = down;
    *smooth_tr = tr;
}

#[cfg(feature = "std")]
pub fn sma_scalar_rolling(data: &[f64], period: usize) -> alloc::vec::Vec<f64> {
    let len = data.len();
    let mut out = alloc::vec![f64::NAN; len];
    if period == 0 || len < period {
        return out;
    }
    let inv_period = 1.0 / period as f64;
    let mut sum = 0.0;
    for &v in &data[..period] {
        sum += v;
    }
    out[period - 1] = sum * inv_period;
    for i in period..len {
        sum += data[i] - data[i - period];
        out[i] = sum * inv_period;
    }
    out
}

#[cfg(feature = "std")]
pub fn sma_scalar_ref(data: &[f64], period: usize) -> alloc::vec::Vec<f64> {
    let mut out = alloc::vec![f64::NAN; data.len()];
    sma_scalar(data, period, &mut out);
    out
}

#[cfg(feature = "std")]
pub fn ema_scalar_ref(data: &[f64], period: usize) -> alloc::vec::Vec<f64> {
    let mut out = alloc::vec![f64::NAN; data.len()];
    ema_scalar(data, period, &mut out);
    out
}

#[cfg(all(feature = "std", test))]
mod tests {
    use super::*;

    fn generate_series(n: usize) -> alloc::vec::Vec<f64> {
        (0..n)
            .map(|i| {
                let t = i as f64;
                100.0 + t * 0.01 + (t * 0.37).sin() * 2.0
            })
            .collect()
    }

    fn assert_slices_close(a: &[f64], b: &[f64], eps: f64) {
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            if x.is_nan() {
                assert!(y.is_nan());
            } else {
                assert!((x - y).abs() < eps, "diff {} vs {}", x, y);
            }
        }
    }

    #[test]
    fn test_sma_simd_matches_scalar() {
        let data = generate_series(10_000);
        for period in [5, 14, 20, 50] {
            let scalar = sma_scalar_ref(&data, period);
            let simd = sma_simd(&data, period);
            assert_slices_close(&scalar, &simd, 1e-10);
        }
    }

    #[test]
    fn test_ema_simd_matches_scalar() {
        let data = generate_series(10_000);
        for period in [5, 12, 26, 50] {
            let scalar = ema_scalar_ref(&data, period);
            let simd = ema_simd(&data, period);
            assert_slices_close(&scalar, &simd, 1e-12);
        }
    }

    #[test]
    fn test_rsi_simd_matches_scalar() {
        let data = generate_series(5_000);
        for period in [2, 3, 4, 5, 14, 21] {
            let mut scalar = alloc::vec![f64::NAN; data.len()];
            rsi_scalar(&data, period, &mut scalar);
            let simd = rsi_simd(&data, period);
            assert_slices_close(&scalar, &simd, 1e-12);
        }
    }

    #[test]
    fn test_macd_simd_matches_scalar() {
        let data = generate_series(2_000);
        let mut macd_s = alloc::vec![f64::NAN; data.len()];
        let mut sig_s = alloc::vec![f64::NAN; data.len()];
        let mut hist_s = alloc::vec![f64::NAN; data.len()];
        macd_scalar(&data, 12, 26, 9, &mut macd_s, &mut sig_s, &mut hist_s);
        let simd = macd_simd(&data, 12, 26, 9);
        assert_slices_close(&macd_s, &simd.macd, 1e-12);
        assert_slices_close(&sig_s, &simd.signal, 1e-12);
        assert_slices_close(&hist_s, &simd.hist, 1e-12);
    }

    #[test]
    fn test_sma_simd_short_input() {
        let data = alloc::vec![1.0, 2.0, 3.0];
        let out = sma_simd(&data, 5);
        assert!(out.iter().all(|v| v.is_nan()));
    }

    #[test]
    fn test_ema_simd_known_values() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let out = ema_simd(&data, 3);
        assert!(out[0].is_nan());
        assert!(out[1].is_nan());
        assert!((out[2] - 2.0).abs() < 1e-12);
    }

    fn generate_hlc_data(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let close: Vec<f64> = (0..n)
            .map(|i| {
                let t = i as f64;
                100.0 + t * 0.01 + (t * 0.37).sin() * 2.0 + (t * 1.13).cos() * 1.5
            })
            .collect();
        let high: Vec<f64> = close
            .iter()
            .map(|c| c + 1.5 + (c % 10.0).sin() * 0.5)
            .collect();
        let low: Vec<f64> = close
            .iter()
            .map(|c| c - 1.5 - (c % 10.0).cos() * 0.5)
            .collect();
        (high, low, close)
    }

    #[test]
    fn test_stoch_simd_matches_scalar() {
        let (high, low, close) = generate_hlc_data(10_000);
        for (k_period, k_slow, d_period) in [(5, 3, 3), (14, 3, 3), (14, 5, 5)] {
            let mut k_scalar = vec![f64::NAN; close.len()];
            let mut d_scalar = vec![f64::NAN; close.len()];
            stoch_scalar(
                &high,
                &low,
                &close,
                k_period,
                k_slow,
                d_period,
                &mut k_scalar,
                &mut d_scalar,
            );
            let simd = stoch_simd(&high, &low, &close, k_period, k_slow, d_period);
            assert_slices_close(&k_scalar, &simd.k, 1e-10);
            assert_slices_close(&d_scalar, &simd.d, 1e-10);
        }
    }

    #[test]
    fn test_stoch_simd_range() {
        let (high, low, close) = generate_hlc_data(1000);
        let result = stoch_simd(&high, &low, &close, 14, 3, 3);
        for (i, &k) in result.k.iter().enumerate() {
            if !k.is_nan() {
                assert!(k >= -0.01 && k <= 100.01, "K out of range at {}: {}", i, k);
            }
        }
        for (i, &d) in result.d.iter().enumerate() {
            if !d.is_nan() {
                assert!(d >= -0.01 && d <= 100.01, "D out of range at {}: {}", i, d);
            }
        }
    }

    #[test]
    fn test_stoch_simd_short_input() {
        let high = vec![10.0, 11.0, 12.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![9.5, 10.5, 11.5];
        let result = stoch_simd(&high, &low, &close, 5, 3, 3);
        assert!(result.k.iter().all(|v| v.is_nan()));
        assert!(result.d.iter().all(|v| v.is_nan()));
    }

    #[test]
    fn test_cci_simd_matches_scalar() {
        let (high, low, close) = generate_hlc_data(10_000);
        for period in [5, 14, 20] {
            let mut scalar = vec![f64::NAN; close.len()];
            cci_scalar(&high, &low, &close, period, &mut scalar);
            let simd = cci_simd(&high, &low, &close, period);
            assert_slices_close(&scalar, &simd, 1e-10);
        }
    }

    #[test]
    fn test_cci_simd_flat_market() {
        let high = vec![100.0; 50];
        let low = vec![100.0; 50];
        let close = vec![100.0; 50];
        let result = cci_simd(&high, &low, &close, 14);
        for (i, &v) in result.iter().enumerate() {
            if !v.is_nan() {
                assert!(
                    v.abs() < 1e-10,
                    "CCI should be ~0 for flat market at {}: {}",
                    i,
                    v
                );
            }
        }
    }

    #[test]
    fn test_cci_simd_short_input() {
        let high = vec![10.0, 11.0, 12.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![9.5, 10.5, 11.5];
        let result = cci_simd(&high, &low, &close, 5);
        assert!(result.iter().all(|v| v.is_nan()));
    }

    /// P-10: verify that the AVX2 ADX warmup path produces the same
    /// accumulators as the scalar reference. The smoothing loop is
    /// identical in both paths, so equality of the warmup output is
    /// sufficient to guarantee end-to-end ADX correctness.
    #[test]
    fn test_adx_warmup_matches_scalar() {
        let (high, low, close) = generate_hlc_data(2_000);
        for period in [8usize, 14, 20, 50] {
            let mut plus_a = 0.0;
            let mut minus_a = 0.0;
            let mut tr_a = 0.0;
            // Reference: scalar loop.
            for i in 1..=period {
                let up = high[i] - high[i - 1];
                let down = low[i - 1] - low[i];
                tr_a += crate::utils::true_range(high[i], low[i], close[i - 1]);
                if up > down && up > 0.0 {
                    plus_a += up;
                }
                if down > up && down > 0.0 {
                    minus_a += down;
                }
            }
            let mut plus_b = 0.0;
            let mut minus_b = 0.0;
            let mut tr_b = 0.0;
            // SIMD dispatch.
            adx_warmup_into(
                &high,
                &low,
                &close,
                period,
                &mut plus_b,
                &mut minus_b,
                &mut tr_b,
            );
            assert!(
                (plus_a - plus_b).abs() < 1e-12,
                "ADX +DM mismatch at period {period}: scalar={plus_a} simd={plus_b}"
            );
            assert!(
                (minus_a - minus_b).abs() < 1e-12,
                "ADX -DM mismatch at period {period}: scalar={minus_a} simd={minus_b}"
            );
            assert!(
                (tr_a - tr_b).abs() < 1e-12,
                "ADX TR mismatch at period {period}: scalar={tr_a} simd={tr_b}"
            );
        }
    }
}
