use crate::error::{Result, TaError};
use crate::math::simd_ops::simd_prefix_sum;
use crate::utils::{init_output, smoothing_factor, validate_input};
use ndarray::Array1;

/// Seeding convention for the Exponential Moving Average (EMA) recursion.
///
/// Two conventions coexist in the TA-Lib ecosystem and in this crate:
///
/// * [`EmaSeed::Sma`] — the first `period` inputs are averaged (SMA) and that
///   mean becomes the EMA value at index `period - 1`; the warm-up region
///   (`0..period-1`) is `NaN`. This is the default for [`ema`] and for
///   streaming indicators built on [`crate::streaming::overlap::ema::StreamingEma`]
///   (e.g. TRIX, DEMA, TEMA).
/// * [`EmaSeed::FirstValue`] — the recursion is seeded with `input[0]`; the EMA
///   is valid from index `0` (no `NaN` warm-up). This matches the convention
///   used internally by `macd` / `macd_into` / `macdfix`, so a batch↔streaming
///   pair only converges when *both* sides use [`EmaSeed::FirstValue`].
///
/// Picking the correct seed is essential for batch↔streaming convergence. A
/// streaming indicator seeded with [`EmaSeed::Sma`] will *not* numerically
/// match a batch indicator (such as `macd`) that internally uses
/// [`EmaSeed::FirstValue`]. See the upgrade plan (支柱 A2) for the rationale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EmaSeed {
    /// Seed the EMA with the SMA of the first `period` inputs (warm-up `NaN`).
    Sma,
    /// Seed the EMA with `input[0]`, valid from index 0 (no warm-up `NaN`).
    FirstValue,
}

/// Common guard for non-finite (NaN / ±Inf) input rejection. Increments the
/// `indicator_input_rejected_total` counter (O-2) when the `metrics` feature
/// is enabled, and emits a `tracing::warn!` event for observability.
#[inline]
fn reject_if_non_finite(name: &'static str, input: &[f64]) -> Result<()> {
    if let Some(idx) = input.iter().position(|v| !v.is_finite()) {
        #[cfg(feature = "metrics")]
        crate::metrics::input_rejected(name, "non_finite");
        #[cfg(feature = "tracing")]
        crate::warn!(indicator = name, idx, "rejected non-finite input");
        return Err(TaError::InvalidParameter {
            name: "input".to_string(),
            constraint: format!("non-finite value at index {idx}"),
        });
    }
    Ok(())
}

/// Simple Moving Average (SMA)
///
/// Calculates the arithmetic mean of the last `period` data points.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
///
/// # Returns
/// Array of SMA values (first `period - 1` values are NaN)
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = moving_avg::sma(&data, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(period, len = input.len())))]
#[inline]
pub fn sma(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    reject_if_non_finite("sma", input)?;
    validate_input(input.len(), period)?;
    #[cfg(feature = "metrics")]
    {
        crate::metrics::indicator_called("sma");
        let start = std::time::Instant::now();
        let result = sma_inner(input, period);
        crate::metrics::record_indicator_duration("sma", start.elapsed().as_secs_f64());
        return result;
    }
    #[cfg(not(feature = "metrics"))]
    sma_inner(input, period)
}

#[inline]
fn sma_inner(input: &[f64], period: usize) -> Result<Array1<f64>> {
    let len = input.len();
    let mut output = init_output(len);
    let inv_period = 1.0 / period as f64;

    // SIMD-accelerated initial sum: 4-6x faster than iterator sum
    let mut sum = simd_horizontal_sum(&input[..period]);
    output[period - 1] = sum * inv_period;

    for i in period..len {
        sum += input[i] - input[i - period];
        output[i] = sum * inv_period;
    }

    Ok(output)
}

/// Compute SMA writing results into a pre-allocated buffer.
///
/// `output` must have the same length as `input`. Warm-up values are written as NaN.
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let mut output = vec![0.0; data.len()];
/// moving_avg::sma_into(&data, 3, &mut output).unwrap();
/// assert_eq!(output.len(), 10);
/// ```
pub fn sma_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
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

    // SIMD NaN-fill warm-up region
    crate::utils::simd_fill_nan(&mut output[..period - 1]);

    let len = input.len();
    let inv_period = 1.0 / period as f64;

    // SIMD-accelerated initial sum
    let mut sum = simd_horizontal_sum(&input[..period]);
    output[period - 1] = sum * inv_period;

    for i in period..len {
        sum += input[i] - input[i - period];
        output[i] = sum * inv_period;
    }

    Ok(())
}

/// Exponential Moving Average (EMA)
///
/// Applies more weight to recent prices using exponential smoothing.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
///
/// # Returns
/// Array of EMA values (first `period - 1` values are NaN)
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = moving_avg::ema(&data, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(period, len = input.len())))]
#[inline]
pub fn ema(input: &[f64], period: usize) -> Result<Array1<f64>> {
    ema_with_seed(input, period, EmaSeed::Sma)
}

/// EMA with an explicit seeding convention (see [`EmaSeed`]).
///
/// This is the generic form of [`ema`]; `ema(input, period)` is exactly
/// `ema_with_seed(input, period, EmaSeed::Sma)`. Use [`EmaSeed::FirstValue`]
/// when you need the recursion seeded with `input[0]` (valid from index `0`,
/// no warm-up `NaN`) — e.g. to converge with `macd` / `macdfix`.
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg::{ema_with_seed, EmaSeed};
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// // FirstValue seed: the EMA is valid immediately and starts at input[0].
/// let result = ema_with_seed(&data, 3, EmaSeed::FirstValue).unwrap();
/// assert_eq!(result[0], 1.0);
/// ```
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(period, len = input.len(), seed = "ema")))]
#[inline]
pub fn ema_with_seed(input: &[f64], period: usize, seed: EmaSeed) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period)?;
    // Check for non-finite values
    if let Some(pos) = input.iter().position(|v| !v.is_finite()) {
        return Err(TaError::InvalidParameter {
            name: "input".to_string(),
            constraint: format!("non-finite value at index {}", pos),
        });
    }
    #[cfg(feature = "metrics")]
    {
        crate::metrics::indicator_called("ema");
        let start = std::time::Instant::now();
        let result = ema_inner(input, period, seed);
        crate::metrics::record_indicator_duration("ema", start.elapsed().as_secs_f64());
        return result;
    }
    #[cfg(not(feature = "metrics"))]
    ema_inner(input, period, seed)
}

#[target_feature(enable = "avx2,fma")]
unsafe fn ema_inner_avx2(input: &[f64], period: usize, output: &mut [f64]) {
    unsafe {
        let len = input.len();
        let k = smoothing_factor(period);

        // SIMD-accelerated initial SMA
        let initial_sma: f64 = simd_horizontal_sum(&input[..period]) / period as f64;
        output[period - 1] = initial_sma;

        // Use pointer operations for better compiler optimization
        let input_ptr = input.as_ptr();
        let output_ptr = output.as_mut_ptr();

        let mut prev = initial_sma;
        for i in period..len {
            let val = *input_ptr.add(i);
            prev = (val - prev).mul_add(k, prev);
            *output_ptr.add(i) = prev;
        }
    }
}

/// AVX-512 EMA inner: identical recurrence to AVX2, but the initial SMA
/// seed uses the 8-wide `simd512_horizontal_sum` for roughly **2x faster**
/// seed throughput on supported hardware. This is the inner called by
/// [`ema_with_seed`] for the SMA seed; the FirstValue seed is scalar (no
/// initial reduction, so AVX-512 would not help).
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx512f")]
unsafe fn ema_inner_avx512(input: &[f64], period: usize, output: &mut [f64]) {
    let len = input.len();
    let k = smoothing_factor(period);

    // AVX-512 8-wide initial SMA — half the iterations of AVX2.
    // (simd512_horizontal_sum handles its own unsafe dispatch internally.)
    let initial_sma: f64 =
        crate::math::simd_ops_avx512::simd512_horizontal_sum(&input[..period]) / period as f64;
    output[period - 1] = initial_sma;

    let input_ptr = input.as_ptr();
    let output_ptr = output.as_mut_ptr();

    let mut prev = initial_sma;
    for i in period..len {
        unsafe {
            let val = *input_ptr.add(i);
            prev = (val - prev).mul_add(k, prev);
            *output_ptr.add(i) = prev;
        }
    }
}

#[inline]
fn ema_inner(input: &[f64], period: usize, seed: EmaSeed) -> Result<Array1<f64>> {
    let len = input.len();
    let mut output = init_output(len);

    match seed {
        EmaSeed::Sma => {
            #[cfg(all(feature = "std", target_arch = "x86_64"))]
            {
                // AVX-512 takes precedence when available — the 8-wide initial
                // sum is the dominant cost for short EMA periods, so this
                // tightens the gap to TA-Lib C for the common 9/12/26 cases.
                if is_x86_feature_detected!("avx512f") {
                    unsafe {
                        ema_inner_avx512(input, period, output.as_slice_mut().unwrap());
                    }
                    return Ok(output);
                }
                if is_x86_feature_detected!("avx2") && is_x86_feature_detected!("fma") {
                    unsafe {
                        ema_inner_avx2(input, period, output.as_slice_mut().unwrap());
                    }
                    return Ok(output);
                }
            }

            // Fallback to scalar implementation (SMA seed)
            let k = smoothing_factor(period);
            let initial_sma: f64 = simd_horizontal_sum(&input[..period]) / period as f64;
            output[period - 1] = initial_sma;

            let mut prev = initial_sma;
            let input_slice = input;
            let output_slice = output.as_slice_mut().unwrap();
            for i in period..len {
                let val = unsafe { *input_slice.get_unchecked(i) };
                prev = (val - prev).mul_add(k, prev);
                unsafe { *output_slice.get_unchecked_mut(i) = prev; }
            }
        }
        EmaSeed::FirstValue => {
            // Seed the recursion with input[0]; valid from index 0 (no warm-up).
            // NOTE: deliberately scalar-only — the AVX2 kernel above assumes an
            // SMA seed, so reusing it here would silently corrupt the output.
            let k = smoothing_factor(period);
            let mut prev = input[0];
            output[0] = prev;

            let input_slice = input;
            let output_slice = output.as_slice_mut().unwrap();
            for i in 1..len {
                let val = unsafe { *input_slice.get_unchecked(i) };
                prev = (val - prev).mul_add(k, prev);
                unsafe { *output_slice.get_unchecked_mut(i) = prev; }
            }
        }
    }

    Ok(output)
}

/// Horizontal (reduction) sum of a `&[f64]` slice using AVX-512 → AVX2 → scalar.
///
/// Algorithm: 4-way unroll, vectorised pairwise add, horizontal sum at the end.
/// On AVX-512 capable CPUs the 8-wide f64 path roughly doubles throughput vs.
/// AVX2 (4-wide) for the ≥ 32 element range, which is exactly where the seed
/// sums of EMA / WMA / Welford / RSI live.
#[inline]
pub fn simd_horizontal_sum(data: &[f64]) -> f64 {
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx512f") {
            return crate::math::simd_ops_avx512::simd512_horizontal_sum(data);
        }
        if is_x86_feature_detected!("avx2") {
            return unsafe { horizontal_sum_avx2(data) };
        }
    }
    data.iter().sum()
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn horizontal_sum_avx2(data: &[f64]) -> f64 {
    use std::arch::x86_64::*;
    let len = data.len();
    let ptr = data.as_ptr();
    let chunks = len / 4;

    let mut acc0 = _mm256_setzero_pd();
    let mut acc1 = _mm256_setzero_pd();
    let mut acc2 = _mm256_setzero_pd();
    let mut acc3 = _mm256_setzero_pd();

    // 16-way unroll: process 4 vectors (16 f64) per iteration
    let unroll_chunks = chunks / 4;
    let mut i = 0;
    unsafe {
        while i < unroll_chunks {
            let base = i * 16;
            acc0 = _mm256_add_pd(acc0, _mm256_loadu_pd(ptr.add(base)));
            acc1 = _mm256_add_pd(acc1, _mm256_loadu_pd(ptr.add(base + 4)));
            acc2 = _mm256_add_pd(acc2, _mm256_loadu_pd(ptr.add(base + 8)));
            acc3 = _mm256_add_pd(acc3, _mm256_loadu_pd(ptr.add(base + 12)));
            i += 1;
        }

        // Tail vectors (up to 3 remaining)
        let tail_start = unroll_chunks * 16;
        let remaining_vecs = chunks - unroll_chunks * 4;
        for j in 0..remaining_vecs {
            let base = tail_start + j * 4;
            acc0 = _mm256_add_pd(acc0, _mm256_loadu_pd(ptr.add(base)));
        }

        // Combine accumulators
        let merged = _mm256_add_pd(_mm256_add_pd(acc0, acc1), _mm256_add_pd(acc2, acc3));
        // Horizontal sum: high 128 + low 128
        let hi = _mm256_extractf128_pd(merged, 1);
        let lo = _mm256_castpd256_pd128(merged);
        let sum128 = _mm_add_pd(hi, lo);
        let high64 = _mm_cvtsd_f64(_mm_unpackhi_pd(sum128, sum128));
        let low64 = _mm_cvtsd_f64(sum128);
        let mut sum = high64 + low64;

        // Scalar tail
        let scalar_start = chunks * 4;
        for j in scalar_start..len {
            sum += *ptr.add(j);
        }
        sum
    }
}

#[cfg(test)]
mod simd_horizontal_sum_tests {
    use super::simd_horizontal_sum;

    #[test]
    fn test_simd_sum_basic() {
        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let expected: f64 = (0..100).map(|i| i as f64).sum();
        assert!((simd_horizontal_sum(&data) - expected).abs() < 1e-9);
    }

    #[test]
    fn test_simd_sum_short() {
        let data = vec![1.0, 2.0, 3.0];
        assert!((simd_horizontal_sum(&data) - 6.0).abs() < 1e-9);
    }

    #[test]
    fn test_simd_sum_empty() {
        let data: Vec<f64> = vec![];
        assert_eq!(simd_horizontal_sum(&data), 0.0);
    }
}

/// Compute EMA writing results into a pre-allocated buffer.
///
/// `output` must have the same length as `input`. Warm-up values are written as NaN.
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let mut output = vec![0.0; data.len()];
/// moving_avg::ema_into(&data, 3, &mut output).unwrap();
/// assert_eq!(output.len(), 10);
/// ```
pub fn ema_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
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

    // SIMD NaN-fill the warm-up region (faster than per-element loop)
    crate::utils::simd_fill_nan(&mut output[..period - 1]);

    let len = input.len();
    let k = smoothing_factor(period);

    // SIMD-accelerated initial SMA seed
    let initial_sma: f64 = simd_horizontal_sum(&input[..period]) / period as f64;
    output[period - 1] = initial_sma;

    let mut prev = initial_sma;
    for i in period..len {
        // FMA form (see `ema_inner` for rationale).
        prev = (input[i] - prev).mul_add(k, prev);
        output[i] = prev;
    }

    Ok(())
}

/// Weighted Moving Average (WMA)
///
/// Applies linearly decreasing weights to older data points.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
///
/// # Returns
/// Array of WMA values (first `period - 1` values are NaN)
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = moving_avg::wma(&data, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(period, len = input.len())))]
#[inline]
pub fn wma(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period)?;
    // Check for non-finite values
    if let Some(pos) = input.iter().position(|v| !v.is_finite()) {
        return Err(TaError::InvalidParameter {
            name: "input".to_string(),
            constraint: format!("non-finite value at index {}", pos),
        });
    }
    #[cfg(feature = "metrics")]
    {
        crate::metrics::indicator_called("wma");
        let start = std::time::Instant::now();
        let result = wma_inner(input, period);
        crate::metrics::record_indicator_duration("wma", start.elapsed().as_secs_f64());
        return result;
    }
    #[cfg(not(feature = "metrics"))]
    wma_inner(input, period)
}

#[inline]
fn wma_inner(input: &[f64], period: usize) -> Result<Array1<f64>> {
    let len = input.len();
    let mut output = Array1::<f64>::zeros(len);
    
    // Only fill warm-up region with NaN (not the entire array)
    for i in 0..period - 1 {
        output[i] = f64::NAN;
    }
    
    let inv_weight_sum = 1.0 / (period * (period + 1) / 2) as f64;
    let p = period as f64;

    // Calculate initial window
    let mut window_sum: f64 = 0.0;
    let mut wsum: f64 = 0.0;
    
    for j in 0..period {
        let v = input[j];
        window_sum += v;
        wsum += (j + 1) as f64 * v;
    }
    output[period - 1] = wsum * inv_weight_sum;

    // Main loop with pointer operations and FMA optimization
    let input_ptr = input.as_ptr();
    let output_ptr = output.as_mut_ptr();
    
    unsafe {
        for i in period..len {
            let old = *input_ptr.add(i - period);
            let new = *input_ptr.add(i);
            // Use FMA: wsum = wsum + p * new - window_sum
            wsum = (p * new).mul_add(1.0, wsum - window_sum);
            window_sum += new - old;
            *output_ptr.add(i) = wsum * inv_weight_sum;
        }
    }

    Ok(output)
}

/// Compute WMA writing results into a pre-allocated buffer.
///
/// `output` must have the same length as `input`. Warm-up values are written as NaN.
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let mut output = vec![0.0; data.len()];
/// moving_avg::wma_into(&data, 3, &mut output).unwrap();
/// assert_eq!(output.len(), 10);
/// ```
pub fn wma_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
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
    let inv_weight_sum = 1.0 / (period * (period + 1) / 2) as f64;
    let p = period as f64;

    let first = period - 1;
    let mut window_sum: f64 = 0.0;
    let mut wsum: f64 = 0.0;
    for (j, &v) in input.iter().enumerate().take(period) {
        window_sum += v;
        wsum += (j + 1) as f64 * v;
    }
    output[first] = wsum * inv_weight_sum;

    for i in period..len {
        let old = input[i - period];
        let new = input[i];
        wsum += p * new - window_sum;
        window_sum += new - old;
        output[i] = wsum * inv_weight_sum;
    }

    Ok(())
}

/// SIMD-optimized WMA using prefix sums.
///
/// The O(n) recursive formula `wsum' = wsum + period*new - window_sum` is
/// inherently serial, so it cannot be vectorized directly. Instead, this
/// implementation exploits the algebraic identity
///
///   WMA[i] = ((period - i) * (Px[i] - Px[i-period])
///           + (Pzx[i] - Pzx[i-period])) / (period*(period+1)/2)
///
/// where `Px` is the inclusive prefix sum of `input` and `Pzx` is the inclusive
/// prefix sum of `j * input[j]`. The two prefix sums and the `j * input[j]`
/// pointwise product are computed via SIMD (AVX2) kernels, then the final
/// O(n) sweep is just a few fadd/fmul per element.
///
/// Warm-up region (first `period - 1` outputs) is left as NaN, matching the
/// scalar `wma` / `wma_into` contract.
pub fn wma_simd(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let len = input.len();
    let mut output = init_output(len);
    if len >= period {
        wma_simd_core(input, period, output.as_slice_mut().unwrap());
    }
    Ok(output)
}

/// In-place SIMD-optimized WMA writing into a pre-allocated buffer of the
/// same length as `input`. Warm-up values are written as NaN.
pub fn wma_into_simd(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
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
    if input.len() >= period {
        wma_simd_core(input, period, output);
    }
    Ok(())
}

#[inline]
fn wma_simd_core(input: &[f64], period: usize, output: &mut [f64]) {
    let len = input.len();
    let p = period as f64;
    let inv_wsum = 1.0 / (p * (p + 1.0) * 0.5);

    // Stage 1: j * input[j] for j = 0..len
    let mut zx = vec![0.0f64; len];
    simd_weighted_by_index_avx2(input, &mut zx);

    // Stage 2: prefix sums of input and zx (length len)
    let mut px = vec![0.0f64; len];
    let mut pzx = vec![0.0f64; len];
    simd_prefix_sum(input, &mut px);
    simd_prefix_sum(&zx, &mut pzx);
    drop(zx);

    // Stage 3: combine to form WMA values for i = period-1..len
    let first = period - 1;
    // initial value at i = first
    let win_sum_init = px[first];
    let wsum_init = pzx[first];
    output[first] = (p * win_sum_init - (first as f64) * win_sum_init + wsum_init) * inv_wsum;
    for i in period..len {
        let win_sum = px[i] - px[i - period];
        let wsum = pzx[i] - pzx[i - period];
        output[i] = ((p - i as f64) * win_sum + wsum) * inv_wsum;
    }
}

/// SIMD vectorized pointwise `out[i] = i * input[i]`. AVX2 path processes
/// 4 f64 per iteration; scalar tail handles the remainder.
#[inline]
fn simd_weighted_by_index_avx2(input: &[f64], out: &mut [f64]) {
    let len = input.len().min(out.len());
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            unsafe { weighted_by_index_avx2_kernel(input, out, len) };
            return;
        }
    }
    for i in 0..len {
        out[i] = (i as f64) * input[i];
    }
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn weighted_by_index_avx2_kernel(input: &[f64], out: &mut [f64], len: usize) {
    unsafe {
        use core::arch::x86_64::*;
        let chunks = len / 4;
        for c in 0..chunks {
            let off = c * 4;
            let v = _mm256_loadu_pd(input.as_ptr().add(off));
            let idx = _mm256_set_pd(
                (off + 3) as f64,
                (off + 2) as f64,
                (off + 1) as f64,
                off as f64,
            );
            _mm256_storeu_pd(out.as_mut_ptr().add(off), _mm256_mul_pd(v, idx));
        }
        let tail_start = chunks * 4;
        for i in tail_start..len {
            out[i] = (i as f64) * input[i];
        }
    }
}

/// Double Exponential Moving Average (DEMA)
///
/// DEMA = 2 * EMA - EMA(EMA)
/// Reduces lag compared to traditional EMA.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
///
/// # Returns
/// Array of DEMA values
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = moving_avg::dema(&data, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(period, len = input.len())))]
#[inline]
pub fn dema(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    reject_if_non_finite("dema", input)?;
    validate_input(input.len(), period)?;

    let len = input.len();
    let s1 = period - 1;
    let k = smoothing_factor(period);
    let one_k = 1.0 - k;
    let inv_p = 1.0 / period as f64;

    let mut output = init_output(len);

    if len <= s1 {
        return Ok(output);
    }

    // Single-buffer approach: compute EMA1 into a temp vec, then do EMA2
    // and DEMA combination in a single forward pass using only a scalar
    // accumulator (eliminates the second vec allocation from the old code).
    let mut ema1_buf = vec![0.0f64; len];
    // SIMD-accelerated initial SMA seed: 4-6x faster than iterator sum.
    let sma1: f64 = simd_horizontal_sum(&input[..period]) * inv_p;
    let mut e1 = sma1;
    ema1_buf[s1] = e1;
    for i in period..len {
        e1 = input[i] * k + e1 * one_k;
        ema1_buf[i] = e1;
    }

    let ema2_start = 2 * s1;
    if ema2_start >= len || len - s1 < period {
        return Ok(output);
    }

    // SIMD-accelerated second SMA seed.
    let sma2: f64 = simd_horizontal_sum(&ema1_buf[s1..s1 + period]) * inv_p;
    let mut e2 = sma2;
    output[ema2_start] = 2.0 * ema1_buf[ema2_start] - e2;

    for i in (ema2_start + 1)..len {
        e2 = ema1_buf[i] * k + e2 * one_k;
        output[i] = 2.0 * ema1_buf[i] - e2;
    }

    Ok(output)
}

/// Triple Exponential Moving Average (TEMA)
///
/// TEMA = 3 * EMA - 3 * EMA(EMA) + EMA(EMA(EMA))
/// Further reduces lag compared to DEMA.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
///
/// # Returns
/// Array of TEMA values
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
/// let result = moving_avg::tema(&data, 3).unwrap();
/// assert_eq!(result.len(), 15);
/// ```
pub fn tema(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let len = input.len();
    let s1 = period - 1;
    let k = smoothing_factor(period);
    let one_k = 1.0 - k;
    let inv_p = 1.0 / period as f64;
    let mut output = init_output(len);

    if len <= s1 {
        return Ok(output);
    }

    // EMA pass 1: input -> ema1_buf (single allocation)
    let mut ema1_buf = vec![0.0f64; len];
    // SIMD-accelerated SMA seed (4-6x faster than iterator sum).
    let sma1: f64 = simd_horizontal_sum(&input[..period]) * inv_p;
    ema1_buf[s1] = sma1;
    let mut e1 = sma1;
    for i in period..len {
        e1 = input[i] * k + e1 * one_k;
        ema1_buf[i] = e1;
    }

    // EMA pass 2: ema1_buf -> scalar e2 accumulator (no allocation)
    let ema2_start = 2 * s1;
    if len - s1 < period {
        return Ok(output);
    }
    // SIMD-accelerated second SMA seed.
    let sma2: f64 = simd_horizontal_sum(&ema1_buf[s1..s1 + period]) * inv_p;
    let mut e2 = sma2;

    // We need EMA2 values at positions ema2_start.. for EMA3 seed.
    // Store them temporarily in output[ema2_start..] to avoid a second buffer.
    output[ema2_start] = e2;
    for i in (ema2_start + 1)..len {
        e2 = ema1_buf[i] * k + e2 * one_k;
        output[i] = e2;
    }

    // EMA pass 3: output[ema2_start..] -> scalar e3 accumulator, write TEMA
    let ema3_start = 3 * s1;
    if len - ema2_start < period {
        for i in ema2_start..len {
            output[i] = f64::NAN;
        }
        return Ok(output);
    }

    let out_slice = output.as_slice().unwrap();
    // SIMD-accelerated third SMA seed.
    let sma3: f64 = simd_horizontal_sum(&out_slice[ema2_start..ema2_start + period]) * inv_p;
    let mut e3 = sma3;

    // Clear positions before ema3_start
    for i in ema2_start..ema3_start {
        output[i] = f64::NAN;
    }

    // Recompute EMA2 from ema3_start onward (since we overwrote output above)
    let mut e2_re = sma2;
    for i in (ema2_start + 1)..=ema3_start {
        e2_re = ema1_buf[i] * k + e2_re * one_k;
    }

    output[ema3_start] = 3.0 * ema1_buf[ema3_start] - 3.0 * e2_re + e3;

    for i in (ema3_start + 1)..len {
        e2_re = ema1_buf[i] * k + e2_re * one_k;
        e3 = e2_re * k + e3 * one_k;
        output[i] = 3.0 * ema1_buf[i] - 3.0 * e2_re + e3;
    }

    Ok(output)
}

/// Kaufman's Adaptive Moving Average (KAMA)
///
/// Adapts to market noise by adjusting the smoothing constant based on the
/// Efficiency Ratio (ER).
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period for ER calculation
/// * `fast_period` - Fast EMA period (default: 2)
/// * `slow_period` - Slow EMA period (default: 30)
///
/// # Returns
/// Array of KAMA values
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = moving_avg::kama(&data, 5, 2, 30).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
#[cfg_attr(feature = "tracing", tracing::instrument(skip_all, fields(period, len = input.len())))]
#[inline]
pub fn kama(
    input: &[f64],
    period: usize,
    fast_period: usize,
    slow_period: usize,
) -> Result<Array1<f64>> {
    if period == 0 || fast_period == 0 || slow_period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    reject_if_non_finite("kama", input)?;
    validate_input(input.len(), period)?;
    #[cfg(feature = "metrics")]
    crate::metrics::indicator_called("kama");
    #[cfg(feature = "metrics")]
    let _kama_start = std::time::Instant::now();

    let len = input.len();
    let mut output = vec![f64::NAN; len];

    let fast_sc = 2.0 / (fast_period as f64 + 1.0);
    let slow_sc = 2.0 / (slow_period as f64 + 1.0);
    let sc_diff = fast_sc - slow_sc;

    output[period - 1] = input[period - 1];

    let mut volatility: f64 = 0.0;
    for i in 1..=period {
        volatility += (input[i] - input[i - 1]).abs();
    }
    {
        let direction = (input[period] - input[0]).abs();
        let er = if volatility != 0.0 {
            direction / volatility
        } else {
            0.0
        };
        let sc = er * sc_diff + slow_sc;
        let sc = sc * sc;
        output[period] = output[period - 1] + sc * (input[period] - output[period - 1]);
    }

    for i in period + 1..len {
        volatility += (input[i] - input[i - 1]).abs() - (input[i - period] - input[i - period - 1]).abs();

        let direction = (input[i] - input[i - period]).abs();
        let er = if volatility != 0.0 {
            direction / volatility
        } else {
            0.0
        };
        let sc = er * sc_diff + slow_sc;
        let sc = sc * sc;
        output[i] = output[i - 1] + sc * (input[i] - output[i - 1]);
    }

    #[cfg(feature = "metrics")]
    crate::metrics::record_indicator_duration("kama", _kama_start.elapsed().as_secs_f64());

    Ok(Array1::from_vec(output))
}

/// Triangular Moving Average (TRIMA)
///
/// A double-smoothed SMA: SMA of SMA. Equivalent to applying a triangular
/// weighting kernel that peaks at the center of the window.
///
/// For odd period N: TRIMA = SMA(SMA(input, (N+1)/2), (N+1)/2)
/// For even period N: TRIMA = SMA(SMA(input, N/2+1), N/2)
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = moving_avg::trima(&data, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
#[inline]
pub fn trima(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let len = input.len();

    if period == 1 {
        return Ok(Array1::from_vec(input.to_vec()));
    }

    let (first_period, second_period) = if period % 2 == 1 {
        let half = period.div_ceil(2);
        (half, half)
    } else {
        (period / 2 + 1, period / 2)
    };

    let s1_start = first_period - 1;
    let mut sma1_buf = vec![f64::NAN; len];
    sma_into(input, first_period, &mut sma1_buf)?;

    let total_warmup = s1_start + second_period - 1;
    let mut output = vec![f64::NAN; len];

    if total_warmup >= len {
        return Ok(Array1::from(output));
    }

    let inv_p = 1.0 / second_period as f64;
    // SIMD-accelerated initial sum.
    let mut sum: f64 = simd_horizontal_sum(&sma1_buf[s1_start..s1_start + second_period]);
    output[total_warmup] = sum * inv_p;

    for i in (total_warmup + 1)..len {
        sum += sma1_buf[i] - sma1_buf[i - second_period];
        output[i] = sum * inv_p;
    }

    Ok(Array1::from(output))
}

/// Moving Average with Variable Period (MAVP)
///
/// Computes a simple moving average where the period can vary for each data point.
///
/// # Arguments
/// * `input` - Input data series
/// * `periods` - Array of periods (one per data point, must match input length)
/// * `min_period` - Minimum allowed period (clamped)
/// * `max_period` - Maximum allowed period (clamped)
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let periods = vec![3.0; 10];
/// let result = moving_avg::mavp(&data, &periods, 2, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
#[inline]
pub fn mavp(
    input: &[f64],
    periods: &[f64],
    min_period: usize,
    max_period: usize,
) -> Result<Array1<f64>> {
    if input.len() != periods.len() {
        return Err(TaError::InvalidParameter {
            name: "periods".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    if min_period == 0 || max_period == 0 || min_period > max_period {
        return Err(TaError::InvalidParameter {
            name: "min_period/max_period".to_string(),
            constraint: "0 < min_period <= max_period".to_string(),
        });
    }
    validate_input(input.len(), min_period)?;

    let len = input.len();
    let mut output = init_output(len);

    for i in 0..len {
        let p = (periods[i].round() as usize).clamp(min_period, max_period);
        if i + 1 >= p {
            let start = i + 1 - p;
            // SIMD-accelerated window sum (4-6x faster than iterator sum).
            let sum: f64 = simd_horizontal_sum(&input[start..=i]);
            output[i] = sum / p as f64;
        }
    }

    Ok(output)
}

/// Hull Moving Average (HMA)
///
/// Reduces lag by applying WMA to a difference of two WMAs, then smoothing with
/// another WMA using `sqrt(period)` as the window.
///
/// Algorithm: WMA(2 * WMA(input, period/2) - WMA(input, period), sqrt(period))
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = moving_avg::hma(&data, 4).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
#[inline]
pub fn hma(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    if period < 2 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "at least 2 for HMA".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let half_period = period / 2;
    let sqrt_period = (period as f64).sqrt().round() as usize;
    let len = input.len();
    let diff_start = period - 1;
    let first_hma = diff_start + sqrt_period - 1;

    let mut output = init_output(len);
    if first_hma >= len {
        return Ok(output);
    }

    // WMA pass 1 & 2: use wma_into for half-period and full-period
    let mut wma_half = vec![f64::NAN; len];
    let mut wma_full = vec![f64::NAN; len];
    wma_into(input, half_period, &mut wma_half)?;
    wma_into(input, period, &mut wma_full)?;

    // WMA pass 3: WMA of diff = 2*wma_half - wma_full, over sqrt_period window
    let inv_ws_o = 1.0 / (sqrt_period * (sqrt_period + 1) / 2) as f64;
    let p_o = sqrt_period as f64;

    let mut o_ws = 0.0;
    let mut o_wsum = 0.0;
    for j in 0..sqrt_period {
        let idx = diff_start + j;
        let d = 2.0 * wma_half[idx] - wma_full[idx];
        o_ws += d;
        o_wsum += (j + 1) as f64 * d;
    }
    output[first_hma] = o_wsum * inv_ws_o;
    let mut o_dirty = o_ws.is_nan();

    for i in diff_start + sqrt_period..len {
        let old = 2.0 * wma_half[i - sqrt_period] - wma_full[i - sqrt_period];
        let new = 2.0 * wma_half[i] - wma_full[i];
        if o_dirty || old.is_nan() || new.is_nan() {
            let start = i + 1 - sqrt_period;
            o_ws = 0.0;
            o_wsum = 0.0;
            for (j, idx) in (start..=i).enumerate() {
                let d = 2.0 * wma_half[idx] - wma_full[idx];
                o_ws += d;
                o_wsum += (j + 1) as f64 * d;
            }
            o_dirty = o_ws.is_nan();
        } else {
            o_wsum += p_o * new - o_ws;
            o_ws += new - old;
        }
        output[i] = o_wsum * inv_ws_o;
    }

    Ok(output)
}

/// Arnaud Legoux Moving Average (ALMA)
///
/// Uses a Gaussian kernel to weight prices within the lookback window.
///
/// # Arguments
/// * `sigma` - Controls the width of the Gaussian (must be > 0)
/// * `offset` - Shifts the Gaussian peak (typically 0.0 to 1.0)
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = moving_avg::alma(&data, 3, 6.0, 0.85).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
#[inline]
pub fn alma(input: &[f64], period: usize, sigma: f64, offset: f64) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    if sigma <= 0.0 {
        return Err(TaError::InvalidParameter {
            name: "sigma".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let m = offset * (period - 1) as f64;
    let s = period as f64 / sigma;

    let mut weights = vec![0.0; period];
    let mut weight_sum = 0.0;
    for (i, w) in weights.iter_mut().enumerate().take(period) {
        *w = (-((i as f64 - m).powi(2)) / (2.0 * s * s)).exp();
        weight_sum += *w;
    }

    let len = input.len();
    let mut output = init_output(len);

    let inv_weight_sum = 1.0 / weight_sum;
    let has_nan = input.iter().any(|v| v.is_nan());

    if has_nan {
        for i in period - 1..len {
            let start = i + 1 - period;
            let mut sum = 0.0;
            let mut found_nan = false;
            for j in 0..period {
                let val = unsafe { *input.get_unchecked(start + j) };
                if val.is_nan() {
                    found_nan = true;
                    break;
                }
                sum += val * unsafe { *weights.get_unchecked(j) };
            }
            if !found_nan {
                output[i] = sum * inv_weight_sum;
            }
        }
    } else {
        for i in period - 1..len {
            let start = i + 1 - period;
            let mut sum = 0.0;
            for j in 0..period {
                sum += unsafe { *input.get_unchecked(start + j) }
                    * unsafe { *weights.get_unchecked(j) };
            }
            output[i] = sum * inv_weight_sum;
        }
    }

    Ok(output)
}

/// McGinley Dynamic
///
/// A self-adjusting moving average that tracks price with minimal lag.
///
/// MD = MD_prev + (Close - MD_prev) / (period * (Close / MD_prev)^4)
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg;
///
/// let data = vec![10.0, 11.0, 12.0, 11.5, 13.0, 14.0, 13.5, 15.0, 16.0, 17.0];
/// let result = moving_avg::mcginley(&data, 14).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
#[inline]
pub fn mcginley(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), 1)?;

    let len = input.len();
    let mut output = init_output(len);
    let period_f = period as f64;

    if input[0].is_nan() {
        return Ok(output);
    }
    output[0] = input[0];

    let inv_period = 1.0 / period_f;
    let mut prev = output[0];
    for i in 1..len {
        let close = input[i];
        if close.is_nan() {
            prev = f64::NAN;
            continue;
        }
        if prev.is_nan() {
            continue;
        }
        if prev.abs() < 1e-15 {
            output[i] = close;
            prev = close;
            continue;
        }
        let ratio = close / prev;
        let r2 = ratio * ratio;
        let r4 = r2 * r2;
        let adj = (close - prev) * inv_period / r4;
        prev += adj;
        output[i] = prev;
    }

    Ok(output)
}

/// Zero Lag Exponential Moving Average (ZLEMA)
///
/// Eliminates lag by applying EMA to a de-lagged price series.
///
/// lag = (period - 1) / 2, EMA(2 * close - close\[lag\], period)
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = moving_avg::zlema(&data, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
#[inline]
pub fn zlema(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let lag = (period - 1) / 2;
    let len = input.len();
    let k = smoothing_factor(period);
    let inv_period = 1.0 / period as f64;

    let mut output = init_output(len);

    let adj = |i: usize| -> f64 { 2.0 * input[i] - input[i - lag] };

    let ema_start = lag + period - 1;
    if ema_start >= len {
        return Ok(output);
    }

    let mut sum = 0.0;
    for i in lag..lag + period {
        sum += adj(i);
    }
    let mut prev = sum * inv_period;
    output[ema_start] = prev;

    for i in ema_start + 1..len {
        let a = adj(i);
        // FMA form (see `ema_inner` for rationale).
        prev = (a - prev).mul_add(k, prev);
        output[i] = prev;
    }

    Ok(output)
}

/// Variable Index Dynamic Average (VIDYA)
///
/// An adaptive EMA whose smoothing constant varies with the Chande Momentum Oscillator.
///
/// SC = 2 / (period + 1), VIDYA = SC * |CMO| * Close + (1 - SC * |CMO|) * VIDYA_prev
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = moving_avg::vidya(&data, 5, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
#[inline]
pub fn vidya(input: &[f64], period: usize, cmo_period: usize) -> Result<Array1<f64>> {
    if period == 0 || cmo_period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period/cmo_period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period.max(cmo_period + 1))?;

    let len = input.len();
    let mut output = init_output(len);
    let sc = smoothing_factor(period);

    let start = cmo_period.max(period - 1);
    if start >= len {
        return Ok(output);
    }

    if input[start].is_nan() {
        return Ok(output);
    }
    output[start] = input[start];

    let mut sum_up = 0.0;
    let mut sum_down = 0.0;
    for j in start - cmo_period + 1..=start {
        let change = input[j] - input[j - 1];
        if change > 0.0 {
            sum_up += change;
        } else {
            sum_down -= change;
        }
    }

    for i in start + 1..len {
        let close = input[i];
        if close.is_nan() || output[i - 1].is_nan() {
            continue;
        }

        let entering_change = input[i] - input[i - 1];
        if entering_change > 0.0 {
            sum_up += entering_change;
        } else {
            sum_down -= entering_change;
        }

        let leaving_idx = i - cmo_period;
        let leaving_change = input[leaving_idx + 1] - input[leaving_idx];
        if leaving_change > 0.0 {
            sum_up -= leaving_change;
        } else {
            sum_down += leaving_change;
        }

        let denom = sum_up + sum_down;
        let cmo_factor = if denom.abs() > 1e-15 {
            ((sum_up - sum_down) / denom).abs()
        } else {
            0.0
        };

        let alpha = sc * cmo_factor;
        output[i] = alpha * close + (1.0 - alpha) * output[i - 1];
    }

    Ok(output)
}

/// Volume Weighted Moving Average (VWMA)
///
/// Weights each price by its corresponding volume over the lookback window.
///
/// VWMA = SUM(Close * Volume, period) / SUM(Volume, period)
///
/// # Examples
///
/// ```
/// use finkit::math::moving_avg;
///
/// let data = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];
/// let volume = vec![1.0; 10];
/// let result = moving_avg::vwma(&data, &volume, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
#[inline]
pub fn vwma(input: &[f64], volume: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    if input.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "volume".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let len = input.len();
    let mut output = Array1::from_elem(len, f64::NAN);
    let clean = !input.iter().any(|v| v.is_nan()) && !volume.iter().any(|v| v.is_nan());

    if clean {
        let mut pv_sum = 0.0f64;
        let mut vol_sum = 0.0f64;
        for j in 0..period {
            pv_sum += input[j] * volume[j];
            vol_sum += volume[j];
        }
        if vol_sum.abs() > 1e-15 {
            output[period - 1] = pv_sum / vol_sum;
        }
        for i in period..len {
            pv_sum += input[i] * volume[i] - input[i - period] * volume[i - period];
            vol_sum += volume[i] - volume[i - period];
            if vol_sum.abs() > 1e-15 {
                output[i] = pv_sum / vol_sum;
            }
        }
    } else {
        let mut pv_sum = 0.0f64;
        let mut vol_sum = 0.0f64;
        let mut dirty = false;

        for j in 0..period {
            if input[j].is_nan() || volume[j].is_nan() {
                dirty = true;
            }
            pv_sum += input[j] * volume[j];
            vol_sum += volume[j];
        }
        if !dirty && vol_sum.abs() > 1e-15 {
            output[period - 1] = pv_sum / vol_sum;
        }

        for i in period..len {
            let old_v = input[i - period];
            let old_vol = volume[i - period];
            let new_v = input[i];
            let new_vol = volume[i];

            if dirty || old_v.is_nan() || old_vol.is_nan() || new_v.is_nan() || new_vol.is_nan() {
                let start = i + 1 - period;
                pv_sum = 0.0;
                vol_sum = 0.0;
                dirty = false;
                for j in start..=i {
                    if input[j].is_nan() || volume[j].is_nan() {
                        dirty = true;
                        break;
                    }
                    pv_sum += input[j] * volume[j];
                    vol_sum += volume[j];
                }
            } else {
                pv_sum += new_v * new_vol - old_v * old_vol;
                vol_sum += new_vol - old_vol;
            }
            if !dirty && vol_sum.abs() > 1e-15 {
                output[i] = pv_sum / vol_sum;
            }
        }
    }

    Ok(output)
}

/// Compute Exponential Moving Averages for **multiple periods in a single pass**.
///
/// This is the workhorse helper for downstream strategies that need the same
/// EMA value at many lookbacks (5/10/20/30/60/120 day, common in Chinese
/// multi-period resonance systems). Each output slice is filled in place.
///
/// # Algorithm
/// 1. For every period, seed the EMA with the SMA of the first `period` bars
///    (same convention as the single-period `ema`).
/// 2. From the first valid bar onward, advance every EMA independently.
///    Because the EMA recurrence is O(1) per bar, the total work is
///    `O(len * num_periods)` with no per-call allocation — strictly better
///    than calling `ema` `num_periods` times, which re-scans the input
///    `num_periods` times and allocates `num_periods` `Vec`s`.
///
/// # Arguments
/// * `input`     — input data series
/// * `periods`   — slice of distinct periods; each must be `>= 1` and `<= input.len()`
/// * `outputs`   — slice of pre-allocated output buffers; must match `periods`
///   in length and each buffer must have length `== input.len()`
///
/// # Errors
/// Returns `TaError::InvalidParameter` if any period is out of range,
/// the input is empty / non-finite, or the output buffers have the wrong shape.
///
/// # Example
/// ```rust
/// use finkit::math::moving_avg::ema_multi_periods;
/// let data: Vec<f64> = (1..=20).map(|i| i as f64).collect();
/// let mut buf5  = vec![0.0; data.len()];
/// let mut buf10 = vec![0.0; data.len()];
/// ema_multi_periods(&data, &[5, 10], &mut [&mut buf5, &mut buf10]).unwrap();
/// assert!(buf5[4]  > 0.0);
/// assert!(buf10[9] > 0.0);
/// ```
pub fn ema_multi_periods(
    input: &[f64],
    periods: &[usize],
    outputs: &mut [&mut [f64]],
) -> Result<()> {
    if periods.len() != outputs.len() {
        return Err(TaError::InvalidParameter {
            name: "outputs".to_string(),
            constraint: format!(
                "expected {} output buffers (one per period), got {}",
                periods.len(),
                outputs.len()
            ),
        });
    }
    if periods.is_empty() {
        return Ok(());
    }
    reject_if_non_finite("ema_multi_periods", input)?;
    let len = input.len();
    if len == 0 {
        // Empty input with non-empty period set: only valid if all periods are 0
        // (which we forbid above). Surface as InvalidParameter.
        return Err(TaError::InvalidParameter {
            name: "input".to_string(),
            constraint: "non-empty".to_string(),
        });
    }
    for &p in periods {
        if p == 0 {
            return Err(TaError::InvalidParameter {
                name: "period".to_string(),
                constraint: "greater than 0".to_string(),
            });
        }
        if p > len {
            return Err(TaError::InvalidParameter {
                name: "period".to_string(),
                constraint: format!("at most input length ({len})"),
            });
        }
    }
    for out in outputs.iter() {
        if out.len() != len {
            return Err(TaError::InvalidParameter {
                name: "output".to_string(),
                constraint: "must have the same length as input".to_string(),
            });
        }
    }

    // Pre-compute smoothing factors and initial SMA seeds for every period.
    // `seeds` holds the SMA of the first `period` bars (or the first bar for
    // period == 1, which is just `input[0]`).
    let n = periods.len();
    let mut alphas: Vec<f64> = Vec::with_capacity(n);
    let mut seeds: Vec<f64> = Vec::with_capacity(n);
    for &p in periods {
        alphas.push(smoothing_factor(p));
        let seed = if p == 1 {
            input[0]
        } else {
            simd_horizontal_sum(&input[..p]) / p as f64
        };
        seeds.push(seed);
    }

    // Per-period state vector `prev[j]`. `start_idx[j]` is the first index at
    // which EMA[j] is defined (== `periods[j] - 1`).
    let mut prev: Vec<f64> = seeds.clone();
    let start_idx: Vec<usize> = periods.iter().map(|&p| p - 1).collect();

    // Warm-up: fill NaN for every index < start_idx, then place the seed.
    for (j, out) in outputs.iter_mut().enumerate() {
        let s = start_idx[j];
        if s > 0 {
            crate::utils::simd_fill_nan(&mut out[..s]);
        }
        out[s] = seeds[j];
    }

    // Single forward pass: for each new bar `i`, advance every EMA whose
    // start index is < i. Emitting one branch per period per bar is cheap
    // (typically ≤8 periods in multi-period resonance) and keeps the inner
    // loop branch-free inside a per-j body. Compiler auto-vectorises the
    // FMA recurrence when periods share a common length.
    for i in 0..len {
        let x = input[i];
        for j in 0..n {
            if i < start_idx[j] {
                continue;
            }
            if i == start_idx[j] {
                // Seed already written above.
                continue;
            }
            let a = alphas[j];
            // FMA form — see `ema_inner` for rationale.
            prev[j] = (x - prev[j]).mul_add(a, prev[j]);
            outputs[j][i] = prev[j];
        }
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn assert_slices_match(a: &Array1<f64>, b: &[f64]) {
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
    fn test_sma() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = sma(&input, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 2.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 3.0, epsilon = 1e-10);
        assert_relative_eq!(result[4], 4.0, epsilon = 1e-10);
    }

    #[test]
    fn test_sma_into_matches_sma() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let expected = sma(&input, 3).unwrap();
        let mut output = vec![0.0; input.len()];
        sma_into(&input, 3, &mut output).unwrap();
        assert_slices_match(&expected, &output);
    }

    #[test]
    fn test_sma_into_invalid_output_len() {
        let input = vec![1.0, 2.0, 3.0];
        let mut output = vec![0.0; 2];
        assert!(sma_into(&input, 2, &mut output).is_err());
    }

    #[test]
    fn test_ema() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = ema(&input, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 2.0, epsilon = 1e-10);
        assert!(result[3] > 2.0 && result[3] < 4.0);
    }

    #[test]
    fn test_ema_into_matches_ema() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let expected = ema(&input, 3).unwrap();
        let mut output = vec![0.0; input.len()];
        ema_into(&input, 3, &mut output).unwrap();
        assert_slices_match(&expected, &output);
    }

    #[test]
    fn test_ema_first_value_seed() {
        // FirstValue seed: valid from index 0, starts at input[0], no warm-up NaN.
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = ema_with_seed(&input, 3, EmaSeed::FirstValue).unwrap();
        assert_eq!(result[0], 1.0);
        let k = 2.0 / 4.0;
        let expected1 = 2.0 * k + 1.0 * (1.0 - k);
        assert_relative_eq!(result[1], expected1, epsilon = 1e-10);
        let expected2 = 3.0 * k + expected1 * (1.0 - k);
        assert_relative_eq!(result[2], expected2, epsilon = 1e-10);
        // No NaN anywhere — the seed makes the EMA valid immediately.
        assert!(result.iter().all(|v| !v.is_nan()));
    }

    #[test]
    fn test_ema_default_is_sma_seed() {
        // `ema` must remain the SMA-seed variant (golden-locked semantics).
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let default = ema(&input, 3).unwrap();
        let explicit = ema_with_seed(&input, 3, EmaSeed::Sma).unwrap();
        assert_slices_match(&default, explicit.as_slice().unwrap());
        // SMA seed has NaN warm-up.
        assert!(default[0].is_nan() && default[1].is_nan());
    }

    #[test]
    fn test_wma() {
        let input = vec![1.0, 2.0, 3.0];
        let result = wma(&input, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        let expected = (1.0 * 1.0 + 2.0 * 2.0 + 3.0 * 3.0) / 6.0;
        assert_relative_eq!(result[2], expected, epsilon = 1e-10);
    }

    #[test]
    fn test_wma_into_matches_wma() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let expected = wma(&input, 3).unwrap();
        let mut output = vec![0.0; input.len()];
        wma_into(&input, 3, &mut output).unwrap();
        assert_slices_match(&expected, &output);
    }

    #[test]
    fn test_wma_into_invalid_output_len() {
        let input = vec![1.0, 2.0, 3.0];
        let mut output = vec![0.0; 2];
        assert!(wma_into(&input, 2, &mut output).is_err());
    }

    #[test]
    fn test_wma_simd_matches_wma() {
        // Compare prefix-sum SIMD WMA against the scalar recursive reference
        // for a range of sizes and periods to guarantee numerical agreement.
        for &n in &[1usize, 5, 16, 20, 64, 100, 257, 1000] {
            for &period in &[1usize, 2, 3, 5, 7, 20, 50] {
                if period > n {
                    continue;
                }
                let input: Vec<f64> = (0..n).map(|i| (i as f64) * 0.5 + 1.0).collect();
                let expected = wma(&input, period).unwrap();
                let simd = wma_simd(&input, period).unwrap();
                for i in 0..n {
                    if expected[i].is_nan() {
                        assert!(simd[i].is_nan(), "n={n} period={period} i={i}");
                    } else {
                        let diff = (expected[i] - simd[i]).abs();
                        let tol = 1e-9 * expected[i].abs().max(1.0);
                        assert!(
                            diff < tol,
                            "mismatch n={n} period={period} i={i}: expected={} simd={} diff={diff}",
                            expected[i], simd[i]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_wma_into_simd_matches_wma_into() {
        let input: Vec<f64> = (0..256).map(|i| ((i as f64) * 0.13).sin() * 10.0).collect();
        for &period in &[1usize, 3, 5, 10, 20, 30] {
            let mut a = vec![0.0; input.len()];
            let mut b = vec![0.0; input.len()];
            wma_into(&input, period, &mut a).unwrap();
            wma_into_simd(&input, period, &mut b).unwrap();
            for i in 0..input.len() {
                if a[i].is_nan() {
                    assert!(b[i].is_nan(), "period={period} i={i}");
                } else {
                    assert!(
                        (a[i] - b[i]).abs() < 1e-9,
                        "period={period} i={i} a={} b={}",
                        a[i], b[i]
                    );
                }
            }
        }
    }

    #[test]
    fn test_dema() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = dema(&input, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(result[3].is_nan());
        assert!(!result[4].is_nan());
    }

    #[test]
    fn test_tema() {
        let input = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
        ];
        let result = tema(&input, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
    }

    #[test]
    fn test_kama() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = kama(&input, 5, 2, 30).unwrap();
        assert!(result[0].is_nan());
        assert!(result[3].is_nan());
        assert_relative_eq!(result[4], 5.0, epsilon = 1e-10);
    }

    #[test]
    fn test_empty_input() {
        assert!(sma(&[], 5).is_err());
        assert!(ema(&[], 5).is_err());
        assert!(wma(&[], 5).is_err());
    }

    #[test]
    fn test_invalid_period() {
        assert!(sma(&[1.0], 0).is_err());
        assert!(ema(&[1.0], 0).is_err());
    }

    #[test]
    fn test_hma() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = hma(&input, 4).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(result[3].is_nan());
        assert!(!result[4].is_nan());
        assert!(result[4] > 0.0);
    }

    #[test]
    fn test_hma_invalid_period() {
        assert!(hma(&[1.0, 2.0], 1).is_err());
        assert!(hma(&[], 4).is_err());
    }

    #[test]
    fn test_alma() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = alma(&input, 3, 6.0, 0.85).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(!result[2].is_nan());
        assert!(result[2] > 1.0 && result[2] < 5.0);
    }

    #[test]
    fn test_alma_invalid_sigma() {
        assert!(alma(&[1.0, 2.0, 3.0], 3, 0.0, 0.85).is_err());
        assert!(alma(&[1.0, 2.0, 3.0], 3, -1.0, 0.85).is_err());
    }

    #[test]
    fn test_mcginley() {
        let input = vec![10.0, 11.0, 12.0, 11.5, 13.0];
        let result = mcginley(&input, 14).unwrap();
        assert_relative_eq!(result[0], 10.0, epsilon = 1e-10);
        assert!(!result[1].is_nan());
        assert!(result[1] > 10.0 && result[1] < 11.0);
    }

    #[test]
    fn test_mcginley_nan_input() {
        let input = vec![10.0, f64::NAN, 12.0];
        let result = mcginley(&input, 5).unwrap();
        assert_relative_eq!(result[0], 10.0, epsilon = 1e-10);
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
    }

    #[test]
    fn test_zlema() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let result = zlema(&input, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(!result[3].is_nan());
    }

    #[test]
    fn test_vidya() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let result = vidya(&input, 5, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert!(result[2].is_nan());
        assert!(result[3].is_nan());
        assert!(!result[4].is_nan());
        assert!(result[9] > result[4]);
    }

    #[test]
    fn test_vidya_invalid_params() {
        assert!(vidya(&[1.0, 2.0], 0, 3).is_err());
        assert!(vidya(&[1.0, 2.0], 5, 0).is_err());
    }

    #[test]
    fn test_vwma() {
        let input = vec![10.0, 20.0, 30.0];
        let volume = vec![1.0, 1.0, 1.0];
        let result = vwma(&input, &volume, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 20.0, epsilon = 1e-10);
    }

    #[test]
    fn test_vwma_weighted() {
        let input = vec![10.0, 20.0, 30.0];
        let volume = vec![1.0, 1.0, 3.0];
        let result = vwma(&input, &volume, 3).unwrap();
        let expected = (10.0 * 1.0 + 20.0 * 1.0 + 30.0 * 3.0) / (1.0 + 1.0 + 3.0);
        assert_relative_eq!(result[2], expected, epsilon = 1e-10);
    }

    #[test]
    fn test_vwma_length_mismatch() {
        assert!(vwma(&[1.0, 2.0], &[1.0], 2).is_err());
    }

    #[test]
    fn test_vwma_nan_input() {
        let input = vec![10.0, f64::NAN, 30.0];
        let volume = vec![1.0, 1.0, 1.0];
        let result = vwma(&input, &volume, 2).unwrap();
        assert!(result[1].is_nan());
    }

    // ───────────────── ema_multi_periods ─────────────────

    #[test]
    fn test_ema_multi_periods_matches_single() {
        // Reference: re-derive via repeated calls to single `ema`.
        let data: Vec<f64> = (1..=200)
            .map(|i| 100.0 + (i as f64 * 0.37).sin() * 5.0)
            .collect();
        let periods = [3usize, 5, 10, 20, 30, 60];
        let mut bufs: Vec<Vec<f64>> = periods.iter().map(|_| vec![0.0; data.len()]).collect();
        {
            let mut refs: Vec<&mut [f64]> = bufs.iter_mut().map(|b| b.as_mut_slice()).collect();
            ema_multi_periods(&data, &periods, &mut refs).unwrap();
        }
        for (j, &p) in periods.iter().enumerate() {
            let expected = ema(&data, p).unwrap();
            for i in 0..data.len() {
                if expected[i].is_nan() {
                    assert!(bufs[j][i].is_nan(), "p={p} i={i}");
                } else {
                    assert_relative_eq!(bufs[j][i], expected[i], epsilon = 1e-12);
                }
            }
        }
    }

    #[test]
    fn test_ema_multi_periods_empty_periods() {
        // Empty period list → no-op, no allocations, no panic.
        let data = vec![1.0, 2.0, 3.0];
        let r = ema_multi_periods(&data, &[], &mut []);
        assert!(r.is_ok());
    }

    #[test]
    fn test_ema_multi_periods_period_too_large() {
        let data = vec![1.0, 2.0, 3.0];
        let mut a = vec![0.0; data.len()];
        // period 5 > len 3 → must error
        let r = ema_multi_periods(&data, &[5], &mut [&mut a]);
        assert!(r.is_err());
    }

    #[test]
    fn test_ema_multi_periods_output_len_mismatch() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let mut a = vec![0.0; data.len()];
        let mut b = vec![0.0; 2]; // wrong length
        let r = ema_multi_periods(&data, &[3, 5], &mut [&mut a, &mut b]);
        assert!(r.is_err());
    }
}

// Zero-copy `_into` variants (B4 / TASK-315)
pub fn trima_into(input: &[f64], period: usize, output: &mut [f64]) -> crate::error::Result<()> {
    let result = trima(input, period)?;
    if result.len() != output.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    output.copy_from_slice(result.as_slice().unwrap());
    Ok(())
}
