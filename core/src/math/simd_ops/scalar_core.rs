//! Portable scalar fallbacks for the basic vector primitives.

use super::prelude::*;

pub(crate) fn sma_scalar(input: &[f64], period: usize, output: &mut [f64]) {
    let len = input.len().min(output.len());
    if period == 0 || len < period {
        return;
    }
    for o in output.iter_mut().take(period - 1) {
        *o = f64::NAN;
    }
    let inv_period = 1.0 / period as f64;
    let mut sum = 0.0f64;
    for &v in input.iter().take(period) {
        sum += v;
    }
    output[period - 1] = sum * inv_period;
    for i in period..len {
        sum += input[i] - input[i - period];
        output[i] = sum * inv_period;
    }
}

pub(crate) fn wma_scalar(input: &[f64], period: usize, output: &mut [f64]) {
    let len = input.len().min(output.len());
    if period == 0 || len < period {
        return;
    }
    let inv_weight_sum = 1.0 / (period * (period + 1) / 2) as f64;
    let p = period as f64;

    for o in output.iter_mut().take(period - 1) {
        *o = f64::NAN;
    }
    let mut window_sum = 0.0f64;
    let mut wsum = 0.0f64;
    for (j, &v) in input.iter().enumerate().take(period) {
        window_sum += v;
        wsum += (j + 1) as f64 * v;
    }
    output[period - 1] = wsum * inv_weight_sum;
    for i in period..len {
        let old = input[i - period];
        let new = input[i];
        wsum += p * new - window_sum;
        window_sum += new - old;
        output[i] = wsum * inv_weight_sum;
    }
}

// The AVX2 tier of `simd_zscore`. This is compact scalar code compiled with
// AVX2 enabled rather than a hand-written intrinsic kernel, so LLVM can
// auto-vectorize the rolling sums; `dispatch::simd_zscore` selects it when the
// CPU reports AVX2 and otherwise calls `zscore_scalar` directly.
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn zscore_fallback(data: &[f64], period: usize, result: &mut [f64]) {
    let len = data.len().min(result.len());
    if period == 0 || len == 0 {
        return;
    }
    let mut sum = 0.0;
    let mut sum_sq = 0.0;
    for i in 0..len {
        sum += data[i];
        sum_sq += data[i] * data[i];
        if i + 1 >= period {
            if i + 1 > period {
                let old = data[i - period];
                sum -= old;
                sum_sq -= old * old;
            }
            let mean = sum / period as f64;
            let variance = (sum_sq / period as f64) - (mean * mean);
            let std = variance.max(0.0).sqrt();
            result[i] = if std.abs() < crate::utils::NUMERIC_EPSILON {
                0.0
            } else {
                (data[i] - mean) / std
            };
        } else {
            result[i] = f64::NAN;
        }
    }
}

// The AVX2 tier of `simd_shift`; see `zscore_fallback` for why this lives in
// the scalar module rather than behind hand-written intrinsics.
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
pub(crate) unsafe fn shift_fallback(data: &[f64], n: isize, fill: f64, result: &mut [f64]) {
    let len = data.len().min(result.len());
    if n >= 0 {
        let n = n as usize;
        for r in result.iter_mut().take(n.min(len)) {
            *r = fill;
        }
        if n < len {
            result[n..len].copy_from_slice(&data[..(len - n)]);
        }
    } else {
        let n = (-n) as usize;
        let valid = len.saturating_sub(n);
        if valid > 0 {
            result[..valid].copy_from_slice(&data[n..n + valid]);
        }
        for r in result.iter_mut().take(len).skip(valid) {
            *r = fill;
        }
    }
}

pub(crate) fn prefix_sum_scalar(data: &[f64], result: &mut [f64]) {
    let len = data.len().min(result.len());
    let mut acc = 0.0;
    for i in 0..len {
        acc += data[i];
        result[i] = acc;
    }
}

pub(crate) fn diff_scalar(data: &[f64], result: &mut [f64]) {
    let len = data.len().min(result.len());
    if len == 0 {
        return;
    }
    result[0] = f64::NAN;
    for i in 1..len {
        result[i] = data[i] - data[i - 1];
    }
}

pub(crate) fn scale_scalar(data: &[f64], factor: f64, result: &mut [f64]) {
    let len = data.len().min(result.len());
    for i in 0..len {
        result[i] = data[i] * factor;
    }
}

pub(crate) fn pct_change_scalar(data: &[f64], result: &mut [f64]) {
    let len = data.len().min(result.len());
    if len == 0 {
        return;
    }
    result[0] = f64::NAN;
    for i in 1..len {
        result[i] = if data[i - 1].abs() < crate::utils::NUMERIC_EPSILON {
            f64::NAN
        } else {
            (data[i] - data[i - 1]) / data[i - 1] * 100.0
        };
    }
}

pub(crate) fn clamp_scalar(data: &[f64], lo: f64, hi: f64, result: &mut [f64]) {
    let len = data.len().min(result.len());
    for i in 0..len {
        result[i] = data[i].max(lo).min(hi);
    }
}

pub(crate) fn weighted_sum_scalar(data: &[f64], weights: &[f64], result: &mut [f64]) {
    let len = data.len().min(weights.len()).min(result.len());
    for i in 0..len {
        result[i] = data[i] * weights[i];
    }
}

pub(crate) fn true_range_scalar(high: &[f64], low: &[f64], prev_close: &[f64], result: &mut [f64]) {
    let len = high
        .len()
        .min(low.len())
        .min(prev_close.len())
        .min(result.len());
    for i in 0..len {
        let hl = high[i] - low[i];
        let hpc = (high[i] - prev_close[i]).abs();
        let lpc = (low[i] - prev_close[i]).abs();
        result[i] = hl.max(hpc).max(lpc);
    }
}

pub(crate) fn typical_price_scalar(high: &[f64], low: &[f64], close: &[f64], result: &mut [f64]) {
    let len = high.len().min(low.len()).min(close.len()).min(result.len());
    for i in 0..len {
        result[i] = (high[i] + low[i] + close[i]) / 3.0;
    }
}

pub(crate) fn median_price_scalar(high: &[f64], low: &[f64], result: &mut [f64]) {
    let len = high.len().min(low.len()).min(result.len());
    for i in 0..len {
        result[i] = (high[i] + low[i]) / 2.0;
    }
}

pub(crate) fn log_return_scalar(data: &[f64], result: &mut [f64]) {
    let len = data.len().min(result.len());
    if len == 0 {
        return;
    }
    result[0] = f64::NAN;
    for i in 1..len {
        result[i] = if data[i - 1] > 0.0 && data[i] > 0.0 {
            f64_ln(data[i] / data[i - 1])
        } else {
            f64::NAN
        };
    }
}

pub(crate) fn zscore_scalar(data: &[f64], period: usize, result: &mut [f64]) {
    zscore_optimized_scalar(data, period, result)
}

pub(crate) fn cumsum_scalar(data: &[f64], result: &mut [f64]) {
    let len = data.len().min(result.len());
    let mut acc = 0.0;
    for i in 0..len {
        acc += data[i];
        result[i] = acc;
    }
}

pub(crate) fn shift_scalar(data: &[f64], n: isize, fill: f64, result: &mut [f64]) {
    let len = data.len().min(result.len());
    if n >= 0 {
        let n = n as usize;
        for r in result.iter_mut().take(n.min(len)) {
            *r = fill;
        }
        if n < len {
            result[n..len].copy_from_slice(&data[..(len - n)]);
        }
    } else {
        let n = (-n) as usize;
        let valid = len.saturating_sub(n);
        if valid > 0 {
            result[..valid].copy_from_slice(&data[n..n + valid]);
        }
        for r in result.iter_mut().take(len).skip(valid) {
            *r = fill;
        }
    }
}

pub(crate) fn obv_core_scalar(close: &[f64], volume: &[f64], result: &mut [f64]) {
    let len = close.len().min(volume.len()).min(result.len());
    if len == 0 {
        return;
    }
    result[0] = volume[0];
    for i in 1..len {
        if close[i] > close[i - 1] {
            result[i] = result[i - 1] + volume[i];
        } else if close[i] < close[i - 1] {
            result[i] = result[i - 1] - volume[i];
        } else {
            result[i] = result[i - 1];
        }
    }
}

pub(crate) fn ad_line_scalar(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    result: &mut [f64],
) {
    let len = high
        .len()
        .min(low.len())
        .min(close.len())
        .min(volume.len())
        .min(result.len());
    if len == 0 {
        return;
    }
    let mut acc = 0.0;
    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();
    let close_ptr = close.as_ptr();
    let volume_ptr = volume.as_ptr();
    let result_ptr = result.as_mut_ptr();
    for i in 0..len {
        unsafe {
            let h = *high_ptr.add(i);
            let l = *low_ptr.add(i);
            let c = *close_ptr.add(i);
            let hl = h - l;
            let mfm = if hl > 0.0 {
                ((c - l) - (h - c)) / hl
            } else {
                0.0
            };
            acc += mfm * *volume_ptr.add(i);
            *result_ptr.add(i) = acc;
        }
    }
}

pub(crate) fn roc_scalar(data: &[f64], period: usize, result: &mut [f64]) {
    let len = data.len().min(result.len());
    for r in result.iter_mut().take(period.min(len)) {
        *r = f64::NAN;
    }
    for i in period..len {
        result[i] = if data[i - period].abs() < crate::utils::NUMERIC_EPSILON {
            f64::NAN
        } else {
            (data[i] - data[i - period]) / data[i - period] * 100.0
        };
    }
}

pub(crate) fn floor_scalar(data: &[f64], result: &mut [f64]) {
    for (out, &value) in result.iter_mut().zip(data.iter()) {
        *out = f64_floor(value);
    }
}

pub(crate) fn ceil_scalar(data: &[f64], result: &mut [f64]) {
    for (out, &value) in result.iter_mut().zip(data.iter()) {
        *out = f64_ceil(value);
    }
}

pub(crate) fn first_non_finite_scalar(data: &[f64]) -> Option<usize> {
    data.iter().position(|value| !value.is_finite())
}

pub(crate) fn count_less_scalar(data: &[f64], cutoff: f64) -> usize {
    data.iter().filter(|&&value| value < cutoff).count()
}

#[inline]
pub(crate) fn simd_sin_cos_scalar(input: &[f64], sin_out: &mut [f64], cos_out: &mut [f64]) {
    let n = input.len().min(sin_out.len()).min(cos_out.len());
    for i in 0..n {
        let (s, c) = f64_sin_cos(input[i]);
        sin_out[i] = s;
        cos_out[i] = c;
    }
}

#[inline]
pub(crate) fn simd_sqrt_scalar(input: &[f64], output: &mut [f64]) {
    for (source, destination) in input.iter().zip(output.iter_mut()) {
        *destination = f64_sqrt(*source);
    }
}

pub(crate) fn simd_sqrt_checked_scalar(input: &[f64], output: &mut [f64]) -> Option<usize> {
    for (i, (source, destination)) in input.iter().zip(output.iter_mut()).enumerate() {
        if !source.is_finite() || *source < 0.0 {
            return Some(i);
        }
        *destination = f64_sqrt(*source);
    }
    None
}

#[inline]
pub(crate) fn simd_bp_tr_scalar(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    bp: &mut [f64],
    tr: &mut [f64],
    len: usize,
) {
    if len == 0 {
        return;
    }
    bp[0] = 0.0;
    tr[0] = 0.0;
    for i in 1..len {
        let prev_close = close[i - 1];
        let tl = low[i].min(prev_close);
        bp[i] = close[i] - tl;
        tr[i] = high[i].max(prev_close) - tl;
    }
}
