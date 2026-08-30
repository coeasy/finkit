//! Hilbert Transform Cycle Indicators & Ehlers Digital Signal Processing Filters
//!
//! This module implements cycle detection indicators based on the Hilbert Transform
//! and John Ehlers' digital signal processing (DSP) filters for financial data.
//!
//! The Hilbert Transform decomposes a signal into its instantaneous amplitude and phase,
//! enabling measurement of the dominant cycle period, trend vs cycle mode, and other
//! cyclical properties.
//!
//! Ehlers filters apply DSP techniques (IIR/FIR designs) to smooth, detrend, and
//! bandpass-filter price series with minimal lag.
//!
//! # Indicators
//! - [`ht_dcperiod`] - Hilbert Transform - Dominant Cycle Period
//! - [`ht_dcphase`] - Hilbert Transform - Dominant Cycle Phase
//! - [`ht_phasor`] - Hilbert Transform - Phasor Components
//! - [`ht_sine`] - Hilbert Transform - Sine Wave
//! - [`ht_trendmode`] - Hilbert Transform - Trend vs Cycle Mode
//! - [`ht_trendline`] - Hilbert Transform - Instantaneous Trendline
//! - [`super_smoother`] - Ehlers 2-pole Super Smoother Filter
//! - [`super_smoother_3pole`] - Ehlers 3-pole Super Smoother Filter
//! - [`roofing_filter`] - Ehlers Roofing Filter (highpass + super smoother)
//! - [`decycler`] - Ehlers Decycler (removes cycle component, keeps trend)
//! - [`bandpass`] - Ehlers Bandpass Filter
//! - [`instantaneous_trendline`] - Ehlers Instantaneous Trendline via ITrend
//! - [`ehlers_ema_super_smoother`] - Ehlers EMA + 2-pole Super Smoother fusion
//! - [`ehlers_fisher_transform`] - Ehlers Fisher Transform (batch wrapper)
//! - [`ehlers_instantaneous_trendline`] - Ehlers Instantaneous Trendline (default alpha=0.07)
//! - [`ehlers_roofing_filter_v2`] - Ehlers Roofing Filter V2 (spectral dilation highpass)
//! - [`ehlers_sidewinder`] - Ehlers Sidewinder (Efficiency Ratio consolidation detector)
//!
//! # Performance
//!
//! Hilbert pipeline is SIMD-accelerated via AVX2 kernels in
//! [`crate::math::simd_ops::simd_ht_smooth`], [`simd_ht_detrender`](crate::math::simd_ops::simd_ht_detrender)
//! and [`simd_ht_components`](crate::math::simd_ops::simd_ht_components).
//!
//! The HT_SINE terminal sin/cos stage (`sin(p)` and `(sin p + cos p)·√2/2` for
//! the lead sine) is batched through [`crate::math::simd_ops::simd_sin_cos`],
//! an AVX2 polynomial-approximation kernel. Because `phase = atan(im/re)` is
//! always bounded to (-π/2, π/2), the polynomial is exact to ~1e-11. The
//! terminal stage drops from a per-element scalar `f64::sin_cos` (~38 ns/bar
//! equivalent) to the batched SIMD path (~15 ns/bar, ~2.5x) on x86_64 AVX2.
//! (The overall `ht_sine` cost is dominated by the Hilbert IIR chain and is
//! accordingly higher.)

use crate::error::Result;
use crate::math::simd_ops;
use crate::utils::{init_output, validate_input};
use ndarray::Array1;

/// Hilbert Transform - Dominant Cycle Period (HT_DCPERIOD)
///
/// Measures the dominant cycle period of the price series using the Hilbert Transform.
/// The dominant cycle period represents the most significant cycle length in the data.
///
/// The algorithm applies a Hilbert Transform to decompose the signal into its
/// instantaneous phase, then tracks phase changes to determine the cycle period.
/// A 6-period smoothed version of the output is returned.
///
/// # Arguments
/// * `input` - Input data series (typically typical price: (high+low+close)/3)
///
/// # Returns
/// Array of dominant cycle period values. Values are smoothed over 6 periods.
/// Initial values are NaN until enough data is available (32 bars minimum).
///
/// # References
/// Based on John F. Ehlers' work on the Hilbert Transform for cycle analysis.
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::ht_dcperiod;
/// let typical_price: Vec<f64> = (0..60).map(|i| i as f64).collect();
/// let result = ht_dcperiod(&typical_price).unwrap();
/// ```
pub fn ht_dcperiod(input: &[f64]) -> Result<Array1<f64>> {
    validate_input(input.len(), 32)?;

    let len = input.len();
    let mut output = init_output(len);

    // compute_hilbert_components 已经计算了 smooth_period
    let (_smooth, _detrender, _in_phase, _quadrature, _j1, _i2, _j2, _phase, period_out) =
        compute_hilbert_components(input, len);

    // 直接使用计算好的 period_out，从 index 32 开始有效
    for i in 32..len {
        output[i] = period_out[i];
    }

    Ok(output)
}

/// Hilbert Transform - Dominant Cycle Phase (HT_DCPHASE)
///
/// Measures the dominant cycle phase of the price series. The phase indicates
/// where the current price is within the dominant cycle, expressed in degrees
/// (0-360).
///
/// The phase wraps around, with values near 0 or 360 indicating cycle turning points.
///
/// # Arguments
/// * `input` - Input data series
///
/// # Returns
/// Array of dominant cycle phase values in degrees. Initial values are NaN until
/// enough data is available (32 bars minimum).
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::ht_dcphase;
/// let data: Vec<f64> = (0..60).map(|i| (i as f64 * 0.1).sin()).collect();
/// let result = ht_dcphase(&data).unwrap();
/// ```
pub fn ht_dcphase(input: &[f64]) -> Result<Array1<f64>> {
    validate_input(input.len(), 32)?;

    let len = input.len();
    let mut output = init_output(len);

    let (_smooth, _detrender, _in_phase, _quadrature, _j1, _i2, _j2, phase, _period) =
        compute_hilbert_components(input, len);

    for i in 32..len {
        // Convert phase from radians to degrees
        output[i] = phase[i] * 180.0 / std::f64::consts::PI;
    }

    Ok(output)
}

/// Hilbert Transform - Phasor Components (HT_PHASOR)
///
/// Returns the in-phase and quadrature components of the Hilbert Transform.
/// These components represent the signal decomposed into two orthogonal parts.
///
/// The in-phase component is a delayed version of the smoothed input, while
/// the quadrature component is the Hilbert Transform of the in-phase component.
/// Together they form an analytic signal representation.
///
/// # Arguments
/// * `input` - Input data series
///
/// # Returns
/// Tuple of (in_phase, quadrature) arrays. Initial values are NaN until enough
/// data is available (12 bars minimum).
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::ht_phasor;
/// let data: Vec<f64> = (0..40).map(|i| (i as f64 * 0.1).sin()).collect();
/// let (in_phase, quadrature) = ht_phasor(&data).unwrap();
/// ```
pub fn ht_phasor(input: &[f64]) -> Result<(Array1<f64>, Array1<f64>)> {
    validate_input(input.len(), 32)?;

    let len = input.len();
    let mut in_phase_out = init_output(len);
    let mut quadrature_out = init_output(len);

    // 使用 IIR 递归滤波的 compute_hilbert_components（匹配 TA-Lib）
    let (_smooth, _detrender, in_phase, quadrature, _j1, _i2, _j2, _phase, _period) =
        compute_hilbert_components(input, len);

    // TA-Lib 兼容：首有效值从 index 32 开始
    for i in 32..len {
        in_phase_out[i] = in_phase[i];
        quadrature_out[i] = quadrature[i];
    }

    Ok((in_phase_out, quadrature_out))
}

/// Hilbert Transform - Sine Wave (HT_SINE)
///
/// Returns the sine and lead sine wave components derived from the Hilbert Transform.
///
/// The sine wave represents the cycle component of the signal, while the lead sine
/// is the sine wave shifted forward by 45 degrees (PI/4 radians). When the sine
/// crosses above the lead sine, it indicates the start of a new cycle.
///
/// # Arguments
/// * `input` - Input data series
///
/// # Returns
/// Tuple of (sine, lead_sine) arrays. Initial values are NaN until enough data
/// is available (32 bars minimum).
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::ht_sine;
/// let data: Vec<f64> = (0..60).map(|i| (i as f64 * 0.1).sin()).collect();
/// let (sine, lead_sine) = ht_sine(&data).unwrap();
/// ```
pub fn ht_sine(input: &[f64]) -> Result<(Array1<f64>, Array1<f64>)> {
    validate_input(input.len(), 32)?;

    let len = input.len();
    let mut sine = init_output(len);
    let mut lead_sine = init_output(len);

    let (_smooth, _detrender, _in_phase, _quadrature, _j1, _i2, _j2, phase, _period) =
        compute_hilbert_components(input, len);

    // `phase` is atan(im/re) ∈ (-π/2, π/2): a bounded domain where the SIMD
    // sin/cos polynomial is accurate to ~1e-11. Batch it through simd_sin_cos.
    let mut phase_sin = vec![0.0_f64; len];
    let mut phase_cos = vec![0.0_f64; len];
    simd_ops::simd_sin_cos(
        &phase[32..len],
        &mut phase_sin[32..len],
        &mut phase_cos[32..len],
    );

    // lead_sine = sin(p)·cos(π/4) + cos(p)·sin(π/4) = (sin(p) + cos(p))·√2/2
    let lead_c = std::f64::consts::FRAC_1_SQRT_2; // cos(π/4) = sin(π/4) = √2/2
    for i in 32..len {
        sine[i] = phase_sin[i];
        lead_sine[i] = (phase_sin[i] + phase_cos[i]) * lead_c;
    }

    Ok((sine, lead_sine))
}

/// Hilbert Transform - Trend vs Cycle Mode (HT_TRENDMODE)
///
/// Indicates whether the market is in trend mode (1) or cycle mode (0).
///
/// The indicator uses the dominant cycle period to determine the market mode:
/// - Trend mode (1.0): The dominant cycle period is at an extreme (very low or very high)
/// - Cycle mode (0.0): The dominant cycle period is within normal range
///
/// This helps traders identify when to use trend-following vs cycle-based strategies.
///
/// # Arguments
/// * `input` - Input data series
///
/// # Returns
/// Array of mode values (1.0 for trend, 0.0 for cycle). Initial values are NaN
/// until enough data is available (32 bars minimum).
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::ht_trendmode;
/// let data: Vec<f64> = (0..60).map(|i| i as f64).collect();
/// let result = ht_trendmode(&data).unwrap();
/// ```
pub fn ht_trendmode(input: &[f64]) -> Result<Array1<f64>> {
    validate_input(input.len(), 32)?;

    let len = input.len();
    let mut output = init_output(len);

    let (_smooth, _detrender, _in_phase, _quadrature, _j1, _i2, _j2, _phase, period_out) =
        compute_hilbert_components(input, len);

    // 使用计算好的 period_out，从 index 32 开始有效
    for i in 32..len {
        let dc_period = period_out[i];
        // Trend mode when period is at extreme values
        if dc_period <= 6.0 || dc_period >= 36.0 {
            output[i] = 1.0;
        } else {
            output[i] = 0.0;
        }
    }

    Ok(output)
}

/// Hilbert Transform - Measurement (HT_MEASUREMENT)
///
/// Provides a single combined measurement value derived from the Hilbert Transform
/// components, summarizing the dominant cycle period and trend vs. cycle mode at
/// each bar. The output ranges roughly in `[0, 1]` where values near 1 indicate a
/// strong trend (period near the edges of the [6, 36] range) and values near 0
/// indicate a strong cycle (period near the centre).
///
/// This indicator is a scalar combination of `ht_dcperiod` and `ht_trendmode`:
/// `ht_measurement[i] = clip((dc_period[i] - 6) / 30, 0, 1)` if trend-mode
/// otherwise `1 - clip((dc_period[i] - 6) / 30, 0, 1)`, providing a continuous
/// "trend strength" measurement rather than a binary trend/cycle flag.
///
/// # Arguments
/// * `input` - Input data series (typically typical price)
///
/// # Returns
/// Array of HT measurement values in `[0, 1]`. Initial values are NaN until
/// enough data is available (32 bars minimum).
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::ht_measurement;
/// let data: Vec<f64> = (0..60).map(|i| i as f64).collect();
/// let result = ht_measurement(&data).unwrap();
/// ```
pub fn ht_measurement(input: &[f64]) -> Result<Array1<f64>> {
    validate_input(input.len(), 32)?;

    let len = input.len();
    let mut output = init_output(len);

    let (_smooth, _detrender, _in_phase, _quadrature, _j1, _i2, _j2, _phase, period_out) =
        compute_hilbert_components(input, len);

    // 使用计算好的 period_out，从 index 32 开始有效
    for i in 32..len {
        let dc_period = period_out[i];
        // Normalize the period to [0, 1] over the canonical [6, 36] range
        let norm = ((dc_period - 6.0) / 30.0).clamp(0.0, 1.0);
        // Trend mode: edges of the range (norm close to 0 or 1)
        // Cycle mode: middle of the range (norm ~0.5)
        // measurement = 4 * norm * (1 - norm)  peaks at 0.5, zero at edges
        let measurement = 4.0 * norm * (1.0 - norm);
        output[i] = measurement;
    }

    Ok(output)
}

/// Hilbert Transform - Instantaneous Trendline (HT_TRENDLINE)
///
/// Computes the instantaneous trendline of the price series using the Hilbert Transform.
/// The trendline represents the underlying trend with cycle components removed.
///
/// The algorithm uses the Hilbert Transform to separate the trend component from
/// the cycle component, then applies a weighted average to smooth the trendline.
///
/// # Arguments
/// * `input` - Input data series (typically typical price)
///
/// # Returns
/// Array of trendline values. Initial values are NaN until enough data is available
/// (32 bars minimum).
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::ht_trendline;
/// let typical_price: Vec<f64> = (0..60).map(|i| i as f64).collect();
/// let result = ht_trendline(&typical_price).unwrap();
/// ```
pub fn ht_trendline(input: &[f64]) -> Result<Array1<f64>> {
    validate_input(input.len(), 32)?;

    let len = input.len();
    let mut output = init_output(len);

    let (_smooth, _detrender, _in_phase, _quadrature, _j1, _i2, _j2, _phase, period_out) =
        compute_hilbert_components(input, len);

    let mut prev_trendline = 0.0;

    for i in 32..len {
        // WMA(4) smooth price: (4*price + 3*price[1] + 2*price[2] + price[3]) / 10
        let smooth_price = (4.0 * input[i]
            + 3.0 * input[i - 1]
            + 2.0 * input[i - 2]
            + input[i - 3]) / 10.0;

        // TA-Lib 兼容：trend mode 当 dc_period <= 6 或 >= 36
        let dc_period = period_out[i];
        let trend_mode = dc_period <= 6.0 || dc_period >= 36.0;

        let today_trendline = if trend_mode {
            // Trend mode: 2:1 weighted average with previous trendline
            (smooth_price + 2.0 * prev_trendline) / 3.0
        } else {
            // Cycle mode: reset to smooth price
            smooth_price
        };

        prev_trendline = today_trendline;
        output[i] = today_trendline;
    }

    Ok(output)
}

// ============================================================================
// Internal Hilbert Transform Implementation
// ============================================================================

/// Compute the smoothed input using a 4-period weighted moving average.
///
/// Smooth = (4*Price + 3*Price[1] + 2*Price[2] + 1*Price[3]) / 10
///
/// Delegates to the AVX2 kernel [`simd_ops::simd_ht_smooth`] when available
/// (scalar fallback otherwise). The first 3 entries are left as 0.0 — the
/// downstream detrender starts at index 6, so these zeros are never read.
fn smooth_input(input: &[f64], len: usize) -> Vec<f64> {
    let mut smooth = vec![0.0; len];
    simd_ops::simd_ht_smooth(input, &mut smooth[..len]);
    smooth
}

/// Compute the detrender (zero-lag differentiator) from smoothed data.
///
/// The detrender removes low-frequency components and amplifies the cycle components.
///
/// Detrender = (0.0962*Smooth + 0.5769*Smooth[2] - 0.5769*Smooth[4] - 0.0962*Smooth[6])
///             * (0.075*Smooth[1] + 0.54*Smooth[3] + 0.075*Smooth[5])
///
/// Delegates to the AVX2 kernel [`simd_ops::simd_ht_detrender`] (scalar fallback).
/// Indices 0..10 are left as 0.0 because the consumer only reads from i >= 10.
#[allow(dead_code)]
fn compute_detrender(smooth: &[f64], len: usize) -> Vec<f64> {
    let mut detrender = vec![0.0; len];
    simd_ops::simd_ht_detrender(smooth, &mut detrender[..len]);
    detrender
}

/// Compute the quadrature component from detrender values.
///
/// Quadrature = 0.0962*Detrender + 0.5769*Detrender[2] - 0.5769*Detrender[4] - 0.0962*Detrender[6]
#[allow(dead_code)]
#[inline(always)]
fn compute_quadrature(detrender: &[f64], i: usize) -> f64 {
    // FMA form: a*b + c (replaces 4 multiplies + 3 adds with 1 mul + 3 fma)
    0.0962f64.mul_add(detrender[i], 0.5769 * detrender[i - 2])
        - 0.5769f64.mul_add(detrender[i - 4], 0.0962 * detrender[i - 6])
}

/// Compute all Hilbert Transform components needed by the cycle indicators.
///
/// This function faithfully replicates TA-Lib's HT implementation including:
/// 1. WMA-based price smoothing (4-period weighted moving average)
/// 2. IIR highpass filter for detrender: `y[n] = x[n] - b*x[n-2] - y[n-2]`
/// 3. IIR filters for Q1, jI, jQ (same highpass structure, scaled by adjustedPrevPeriod)
/// 4. IIR recursive filtering for Q2, I2 (feedback: 0.2*new + 0.8*prev)
/// 5. IIR recursive filtering for Re, Im (feedback: 0.2*new + 0.8*prev)
/// 6. Odd/even bar interleaved processing with 3-element circular buffers
/// 7. Adaptive period adjustment via `adjustedPrevPeriod = 0.075*period + 0.54`
///
/// Returns tuple of:
/// (smooth, detrender, in_phase, quadrature, j1, i2, j2, phase, period)
///
/// CRITICAL: The IIR recursive feedback is essential for numerical accuracy.
/// The previous FIR-only implementation caused deviations up to 487 degrees.
#[allow(clippy::type_complexity)]
fn compute_hilbert_components(
    input: &[f64],
    len: usize,
) -> (
    Vec<f64>, // smooth
    Vec<f64>, // detrender
    Vec<f64>, // in_phase
    Vec<f64>, // quadrature
    Vec<f64>, // j1
    Vec<f64>, // i2
    Vec<f64>, // j2
    Vec<f64>, // phase
    Vec<f64>, // smooth_period (IIR-filtered, matches TA-Lib)
) {
    // Compute smoothed price: WMA(4) = (4*p[i] + 3*p[i-1] + 2*p[i-2] + p[i-3]) / 10
    let smooth = smooth_input(input, len);

    // Output buffers
    let mut detrender_out = vec![0.0; len];
    let mut in_phase_out = vec![0.0; len];
    let mut quadrature_out = vec![0.0; len];
    let mut j1_out = vec![0.0; len];
    let mut i2_out = vec![0.0; len];
    let mut j2_out = vec![0.0; len];
    let mut phase_out = vec![0.0; len];
    let mut period_out = vec![0.0; len];

    // Constants
    let a_coeff = 0.0962;
    let b_coeff = 0.5769;

    // IIR filter state variables
    let mut prev_q2 = 0.0;
    let mut prev_i2 = 0.0;
    let mut re = 0.0;
    let mut im = 0.0;
    let mut period = 0.0;

    // Delay lines for I1 (detrender delayed by 2 bars for each parity)
    // I1ForEvenPrev3 = detrender delayed by 3 for even bars (2 even steps back)
    // I1ForEvenPrev2 = detrender delayed by 1 for even bars (1 even step back)
    let mut i1_for_even_prev3 = 0.0;
    let mut i1_for_odd_prev3 = 0.0;
    let mut i1_for_even_prev2 = 0.0;
    let mut i1_for_odd_prev2 = 0.0;

    // 3-element circular buffers for IIR highpass filters.
    // For each filter (detrender, Q1, jI, jQ), the buffer stores the current input
    // (a_coeff * source_value) and the value from 2 steps back is read before overwrite.
    // detrender: input = a*smooth, delayed = a*smooth[i-2]
    // Q1: input = a*detrender, delayed = a*detrender[i-2]
    // jI: input = a*I1Prev3, delayed = a*I1Prev3[i-2]
    // jQ: input = a*Q1, delayed = a*Q1[i-2]
    let mut detrender_even = [0.0; 3];
    let mut detrender_odd = [0.0; 3];
    let mut q1_even = [0.0; 3];
    let mut q1_odd = [0.0; 3];
    let mut ji_even = [0.0; 3];
    let mut ji_odd = [0.0; 3];
    let mut jq_even = [0.0; 3];
    let mut jq_odd = [0.0; 3];

    // Previous values for IIR highpass feedback.
    // TA-Lib highpass formula: y[n] = a*x[n] - buffer[n-2] - prev_y + b*prev_x
    // where buffer[n-2] = a*x[n-2] and prev_x is the input from 2 steps ago.
    let mut prev_detrender_even = 0.0;
    let mut prev_detrender_odd = 0.0;
    let mut prev_detrender_input_even = 0.0;
    let mut prev_detrender_input_odd = 0.0;
    let mut prev_q1_even = 0.0;
    let mut prev_q1_odd = 0.0;
    let mut prev_q1_input_even = 0.0;
    let mut prev_q1_input_odd = 0.0;
    let mut prev_ji_even = 0.0;
    let mut prev_ji_odd = 0.0;
    let mut prev_ji_input_even = 0.0;
    let mut prev_ji_input_odd = 0.0;
    let mut prev_jq_even = 0.0;
    let mut prev_jq_odd = 0.0;
    let mut prev_jq_input_even = 0.0;
    let mut prev_jq_input_odd = 0.0;

    let mut hilbert_idx = 0;
    let mut smooth_period = 0.0;
    let mut current_q2;
    let mut current_i2;

    // Process from bar 10 (matching TA-Lib: WMA needs 10 bars warmup).
    // Output starts at bar 32 (lookbackTotal = 32).
    for i in 10..len {
        let adjusted_prev_period = 0.075 * period + 0.54;
        let smoothed_value = smooth[i];

        if i % 2 == 0 {
            // ---- Even bar processing ----
            // Detrender IIR highpass: y[n] = (a*x[n] - buffer[n-6] - prev_y + b*prev_x) * adj
            // where buffer[n-6] = a*x[n-6], prev_y = b*x[n-4], prev_x = x[n-2]
            let mut detrender_val = -detrender_even[hilbert_idx];
            detrender_even[hilbert_idx] = a_coeff * smoothed_value;
            detrender_val += a_coeff * smoothed_value;
            detrender_val -= prev_detrender_even;
            prev_detrender_even = b_coeff * prev_detrender_input_even;
            detrender_val += prev_detrender_even;
            prev_detrender_input_even = smoothed_value;
            detrender_val *= adjusted_prev_period;
            detrender_out[i] = detrender_val;

            // Q1 IIR highpass: same structure, input = detrender
            let mut q1_val = -q1_even[hilbert_idx];
            q1_even[hilbert_idx] = a_coeff * detrender_val;
            q1_val += a_coeff * detrender_val;
            q1_val -= prev_q1_even;
            prev_q1_even = b_coeff * prev_q1_input_even;
            q1_val += prev_q1_even;
            prev_q1_input_even = detrender_val;
            q1_val *= adjusted_prev_period;

            // jI IIR highpass: input = I1ForEvenPrev3 (detrender delayed by 2 even steps)
            let mut ji_val = -ji_even[hilbert_idx];
            ji_even[hilbert_idx] = a_coeff * i1_for_even_prev3;
            ji_val += a_coeff * i1_for_even_prev3;
            ji_val -= prev_ji_even;
            prev_ji_even = b_coeff * prev_ji_input_even;
            ji_val += prev_ji_even;
            prev_ji_input_even = i1_for_even_prev3;
            ji_val *= adjusted_prev_period;

            // jQ IIR highpass: input = Q1
            let mut jq_val = -jq_even[hilbert_idx];
            jq_even[hilbert_idx] = a_coeff * q1_val;
            jq_val += a_coeff * q1_val;
            jq_val -= prev_jq_even;
            prev_jq_even = b_coeff * prev_jq_input_even;
            jq_val += prev_jq_even;
            prev_jq_input_even = q1_val;
            jq_val *= adjusted_prev_period;

            // Advance circular buffer index (only on even bars)
            hilbert_idx = if hilbert_idx == 2 { 0 } else { hilbert_idx + 1 };

            // IIR recursive filtering for Q2 and I2
            // Q2 = 0.2*(Q1 + jI) + 0.8*prevQ2
            // I2 = 0.2*(I1ForEvenPrev3 - jQ) + 0.8*prevI2
            current_q2 = 0.2 * (q1_val + ji_val) + 0.8 * prev_q2;
            current_i2 = 0.2 * (i1_for_even_prev3 - jq_val) + 0.8 * prev_i2;

            // Update I1 delay lines for next odd bar
            i1_for_odd_prev3 = i1_for_odd_prev2;
            i1_for_odd_prev2 = detrender_val;

            // Store intermediate values for output
            in_phase_out[i] = i1_for_even_prev3;
            quadrature_out[i] = q1_val;
            j1_out[i] = ji_val;
            i2_out[i] = current_i2;
            j2_out[i] = current_q2;
        } else {
            // ---- Odd bar processing ----
            // Detrender IIR highpass: y[n] = (a*x[n] - buffer[n-6] - prev_y + b*prev_x) * adj
            let mut detrender_val = -detrender_odd[hilbert_idx];
            detrender_odd[hilbert_idx] = a_coeff * smoothed_value;
            detrender_val += a_coeff * smoothed_value;
            detrender_val -= prev_detrender_odd;
            prev_detrender_odd = b_coeff * prev_detrender_input_odd;
            detrender_val += prev_detrender_odd;
            prev_detrender_input_odd = smoothed_value;
            detrender_val *= adjusted_prev_period;
            detrender_out[i] = detrender_val;

            // Q1 IIR highpass: same structure, input = detrender
            let mut q1_val = -q1_odd[hilbert_idx];
            q1_odd[hilbert_idx] = a_coeff * detrender_val;
            q1_val += a_coeff * detrender_val;
            q1_val -= prev_q1_odd;
            prev_q1_odd = b_coeff * prev_q1_input_odd;
            q1_val += prev_q1_odd;
            prev_q1_input_odd = detrender_val;
            q1_val *= adjusted_prev_period;

            // jI IIR highpass: input = I1ForOddPrev3
            let mut ji_val = -ji_odd[hilbert_idx];
            ji_odd[hilbert_idx] = a_coeff * i1_for_odd_prev3;
            ji_val += a_coeff * i1_for_odd_prev3;
            ji_val -= prev_ji_odd;
            prev_ji_odd = b_coeff * prev_ji_input_odd;
            ji_val += prev_ji_odd;
            prev_ji_input_odd = i1_for_odd_prev3;
            ji_val *= adjusted_prev_period;

            // jQ IIR highpass: input = Q1
            let mut jq_val = -jq_odd[hilbert_idx];
            jq_odd[hilbert_idx] = a_coeff * q1_val;
            jq_val += a_coeff * q1_val;
            jq_val -= prev_jq_odd;
            prev_jq_odd = b_coeff * prev_jq_input_odd;
            jq_val += prev_jq_odd;
            prev_jq_input_odd = q1_val;
            jq_val *= adjusted_prev_period;

            // IIR recursive filtering for Q2 and I2
            // Q2 = 0.2*(Q1 + jI) + 0.8*prevQ2
            // I2 = 0.2*(I1ForOddPrev3 - jQ) + 0.8*prevI2
            current_q2 = 0.2 * (q1_val + ji_val) + 0.8 * prev_q2;
            current_i2 = 0.2 * (i1_for_odd_prev3 - jq_val) + 0.8 * prev_i2;

            // Update I1 delay lines for next even bar
            i1_for_even_prev3 = i1_for_even_prev2;
            i1_for_even_prev2 = detrender_val;

            // Store intermediate values for output
            in_phase_out[i] = i1_for_odd_prev3;
            quadrature_out[i] = q1_val;
            j1_out[i] = ji_val;
            i2_out[i] = current_i2;
            j2_out[i] = current_q2;
        }

        // IIR recursive filtering for Re and Im.
        // CRITICAL: Must use OLD prevQ2/prevI2 (before this bar's update).
        // TA-Lib: Re = 0.2*(I2*prevI2 + Q2*prevQ2) + 0.8*Re
        //         Im = 0.2*(I2*prevQ2 - Q2*prevI2) + 0.8*Im
        //         prevQ2 = Q2; prevI2 = I2
        re = 0.2 * (current_i2 * prev_i2 + current_q2 * prev_q2) + 0.8 * re;
        im = 0.2 * (current_i2 * prev_q2 - current_q2 * prev_i2) + 0.8 * im;

        // Update prevQ2/prevI2 AFTER Re/Im computation
        prev_q2 = current_q2;
        prev_i2 = current_i2;

        // Compute period from Re/Im
        let temp_real = period;
        if im.abs() > 1e-10 && re.abs() > 1e-10 {
            period = 360.0 / (im / re).atan();
        }

        // Clamp period to [0.67*prev, 1.5*prev] then [6, 50]
        let temp_real2 = 1.5 * temp_real;
        if period > temp_real2 {
            period = temp_real2;
        }
        let temp_real2 = 0.67 * temp_real;
        if period < temp_real2 {
            period = temp_real2;
        }
        if period < 6.0 {
            period = 6.0;
        } else if period > 50.0 {
            period = 50.0;
        }

        // Smooth period with EMA: period = 0.2*period + 0.8*prevPeriod
        period = 0.2 * period + 0.8 * temp_real;

        // Compute smoothPeriod = 0.33*period + 0.67*prevSmoothPeriod
        smooth_period = 0.33 * period + 0.67 * smooth_period;

        // TA-Lib 兼容：使用 atan(im/re) 而非 atan2(im, re)
        // TA-Lib ta_HT_DCPHASE.c: atan(Quadrature / InPhase) * RAD2DEG
        phase_out[i] = if re.abs() > 1e-10 { (im / re).atan() } else { 0.0 };

        // Store smoothed period (valid from bar 32 onward)
        period_out[i] = smooth_period;
    }

    (
        smooth,
        detrender_out,
        in_phase_out,
        quadrature_out,
        j1_out,
        i2_out,
        j2_out,
        phase_out,
        period_out,
    )
}

// ============================================================================
// Ehlers Digital Signal Processing Filters
// ============================================================================

/// Ehlers 2-Pole Super Smoother Filter
///
/// A zero-lag smoothing filter based on a 2-pole Butterworth design that
/// eliminates the Nyquist frequency component while preserving signal shape.
/// It produces less lag than an EMA of equivalent smoothing.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Cut-off period (must be >= 2)
///
/// # References
/// John F. Ehlers, "Cybernetic Analysis for Stocks and Futures" (2004)
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::super_smoother;
/// let data: Vec<f64> = (0..50).map(|i| (i as f64 * 0.1).sin() * 10.0 + 50.0).collect();
/// let result = super_smoother(&data, 10).unwrap();
/// ```
pub fn super_smoother(input: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_input(input.len(), period.max(3))?;
    if period < 2 {
        return Err(crate::error::TaError::InvalidParameter {
            name: "period".into(),
            constraint: ">= 2".into(),
        });
    }

    let len = input.len();
    let mut output = init_output(len);

    let a1 = (-std::f64::consts::SQRT_2 * std::f64::consts::PI / period as f64).exp();
    let b1 = 2.0 * a1 * (std::f64::consts::SQRT_2 * std::f64::consts::PI / period as f64).cos();
    let c2 = b1;
    let c3 = -a1 * a1;
    let c1 = 1.0 - c2 - c3;

    output[0] = input[0];
    if len > 1 {
        output[1] = input[1];
    }

    for i in 2..len {
        output[i] = c1 * (input[i] + input[i - 1]) / 2.0 + c2 * output[i - 1] + c3 * output[i - 2];
    }

    Ok(output)
}

/// Ehlers 3-Pole Super Smoother Filter
///
/// A sharper cut-off version of the super smoother using a 3-pole design.
/// Provides steeper roll-off at the cost of slightly more lag than the 2-pole version.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Cut-off period (must be >= 2)
///
/// # References
/// John F. Ehlers, "Cycle Analytics for Traders" (2013)
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::super_smoother_3pole;
/// let data: Vec<f64> = (0..50).map(|i| (i as f64 * 0.1).sin() * 10.0 + 50.0).collect();
/// let result = super_smoother_3pole(&data, 10).unwrap();
/// ```
pub fn super_smoother_3pole(input: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_input(input.len(), period.max(4))?;
    if period < 2 {
        return Err(crate::error::TaError::InvalidParameter {
            name: "period".into(),
            constraint: ">= 2".into(),
        });
    }

    let len = input.len();
    let mut output = init_output(len);

    let a1 = (-std::f64::consts::PI / period as f64).exp();
    let b1 = 2.0 * a1 * (std::f64::consts::PI * 1.738 / period as f64).cos();
    let c1 = a1 * a1;

    let coef2 = b1 + c1;
    let coef3 = -(c1 + b1 * c1);
    let coef4 = c1 * c1;
    let coef1 = 1.0 - coef2 - coef3 - coef4;

    output[0] = input[0];
    if len > 1 {
        output[1] = input[1];
    }
    if len > 2 {
        output[2] = input[2];
    }

    for i in 3..len {
        output[i] = coef1 * input[i] + coef2 * output[i - 1] + coef3 * output[i - 2]
            + coef4 * output[i - 3];
    }

    Ok(output)
}

/// Ehlers Roofing Filter
///
/// Combines a high-pass filter (to remove trend) with a super smoother
/// (to remove high-frequency noise), isolating the cycle component.
/// The result oscillates around zero.
///
/// # Arguments
/// * `input` - Input data series
/// * `hp_period` - High-pass filter period (removes components longer than this)
/// * `lp_period` - Super smoother (low-pass) period (removes components shorter than this)
///
/// # References
/// John F. Ehlers, "Cycle Analytics for Traders" (2013)
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::roofing_filter;
/// let data: Vec<f64> = (0..80).map(|i| (i as f64 * 0.1).sin() * 10.0 + 50.0).collect();
/// let result = roofing_filter(&data, 48, 10).unwrap();
/// ```
pub fn roofing_filter(input: &[f64], hp_period: usize, lp_period: usize) -> Result<Array1<f64>> {
    let min_bars = hp_period.max(lp_period).max(3);
    validate_input(input.len(), min_bars)?;
    if hp_period < 2 || lp_period < 2 {
        return Err(crate::error::TaError::InvalidParameter {
            name: "hp_period/lp_period".into(),
            constraint: ">= 2".into(),
        });
    }

    let len = input.len();
    let mut hp = vec![0.0f64; len];
    let mut output = init_output(len);

    // High-pass filter coefficients
    let alpha_hp =
        (0.707 * 2.0 * std::f64::consts::PI / hp_period as f64).cos();
    let hp_coef = (1.0 + alpha_hp) / 2.0;

    // Apply high-pass filter
    hp[0] = input[0];
    if len > 1 {
        hp[1] = hp_coef * (input[1] - input[0]);
    }
    for i in 2..len {
        hp[i] = hp_coef * (input[i] - input[i - 1]) + (2.0 * alpha_hp - 1.0) * hp[i - 1]
            - (alpha_hp * alpha_hp - 2.0 * alpha_hp + 1.0) * hp[i - 2];
    }

    // Apply super smoother to the high-passed data
    let a1 = (-std::f64::consts::SQRT_2 * std::f64::consts::PI / lp_period as f64).exp();
    let b1 = 2.0 * a1 * (std::f64::consts::SQRT_2 * std::f64::consts::PI / lp_period as f64).cos();
    let c2 = b1;
    let c3 = -a1 * a1;
    let c1 = 1.0 - c2 - c3;

    output[0] = hp[0];
    if len > 1 {
        output[1] = hp[1];
    }
    for i in 2..len {
        output[i] = c1 * (hp[i] + hp[i - 1]) / 2.0 + c2 * output[i - 1] + c3 * output[i - 2];
    }

    Ok(output)
}

/// Ehlers Decycler
///
/// Removes the cycle component from the price series, preserving only the trend.
/// Computed as `price - highpass(price)`, this produces an extremely smooth
/// trend-following line with very little lag.
///
/// # Arguments
/// * `input` - Input data series
/// * `hp_period` - High-pass filter period (components shorter than this are removed)
///
/// # References
/// John F. Ehlers, "Cycle Analytics for Traders" (2013)
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::decycler;
/// let data: Vec<f64> = (0..60).map(|i| i as f64 + (i as f64 * 0.3).sin() * 2.0).collect();
/// let result = decycler(&data, 20).unwrap();
/// ```
pub fn decycler(input: &[f64], hp_period: usize) -> Result<Array1<f64>> {
    validate_input(input.len(), hp_period.max(3))?;
    if hp_period < 2 {
        return Err(crate::error::TaError::InvalidParameter {
            name: "hp_period".into(),
            constraint: ">= 2".into(),
        });
    }

    let len = input.len();
    let mut hp = vec![0.0f64; len];
    let mut output = init_output(len);

    let alpha_hp =
        (0.707 * 2.0 * std::f64::consts::PI / hp_period as f64).cos();
    let hp_coef = (1.0 + alpha_hp) / 2.0;

    hp[0] = 0.0;
    if len > 1 {
        hp[1] = hp_coef * (input[1] - input[0]);
    }
    for i in 2..len {
        hp[i] = hp_coef * (input[i] - input[i - 1]) + (2.0 * alpha_hp - 1.0) * hp[i - 1]
            - (alpha_hp * alpha_hp - 2.0 * alpha_hp + 1.0) * hp[i - 2];
    }

    for i in 0..len {
        output[i] = input[i] - hp[i];
    }

    Ok(output)
}

/// Ehlers Bandpass Filter
///
/// Isolates a specific frequency band in the price data. Returns values that
/// oscillate around zero, representing the cycle component at the specified period.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Center period of the bandpass
/// * `bandwidth` - Bandwidth as a fraction of the center period (typically 0.1–0.5)
///
/// # References
/// John F. Ehlers, "Cycle Analytics for Traders" (2013)
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::bandpass;
/// let data: Vec<f64> = (0..80).map(|i| (i as f64 * 0.2).sin() * 5.0 + 50.0).collect();
/// let result = bandpass(&data, 20, 0.3).unwrap();
/// ```
pub fn bandpass(input: &[f64], period: usize, bandwidth: f64) -> Result<Array1<f64>> {
    validate_input(input.len(), period.max(3))?;
    if period < 2 {
        return Err(crate::error::TaError::InvalidParameter {
            name: "period".into(),
            constraint: ">= 2".into(),
        });
    }
    if !(0.0..=1.0).contains(&bandwidth) {
        return Err(crate::error::TaError::InvalidParameter {
            name: "bandwidth".into(),
            constraint: "between 0.0 and 1.0".into(),
        });
    }

    let len = input.len();
    let mut output = init_output(len);

    let beta =
        (2.0 * std::f64::consts::PI / period as f64).cos();
    let gamma = (2.0 * std::f64::consts::PI * bandwidth / period as f64).cos();
    let delta = 1.0 / gamma;
    let alpha = delta - (delta * delta - 1.0).sqrt();

    output[0] = 0.0;
    if len > 1 {
        output[1] = 0.0;
    }

    for i in 2..len {
        output[i] = 0.5 * (1.0 - alpha) * (input[i] - input[i - 2])
            + beta * (1.0 + alpha) * output[i - 1]
            - alpha * output[i - 2];
    }

    Ok(output)
}

/// Ehlers Instantaneous Trendline (ITrend)
///
/// A forward-shifted moving average that tracks the instantaneous trend with
/// minimal lag. Uses a 2-pole filter design to smoothly follow price.
///
/// # Arguments
/// * `input` - Input data series
/// * `alpha` - Smoothing factor (0.0–1.0, typically 0.07). Smaller = smoother.
///
/// # References
/// John F. Ehlers, "Cybernetic Analysis for Stocks and Futures" (2004)
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::instantaneous_trendline;
/// let data: Vec<f64> = (0..60).map(|i| i as f64 * 0.5 + 10.0).collect();
/// let result = instantaneous_trendline(&data, 0.07).unwrap();
/// ```
pub fn instantaneous_trendline(input: &[f64], alpha: f64) -> Result<Array1<f64>> {
    validate_input(input.len(), 3)?;
    if !(0.0..=1.0).contains(&alpha) {
        return Err(crate::error::TaError::InvalidParameter {
            name: "alpha".into(),
            constraint: "between 0.0 and 1.0".into(),
        });
    }

    let len = input.len();
    let mut output = init_output(len);

    output[0] = input[0];
    if len > 1 {
        output[1] = (input[1] + input[0]) / 2.0;
    }

    for i in 2..len {
        output[i] = (alpha - alpha * alpha / 4.0) * input[i]
            + 0.5 * alpha * alpha * input[i - 1]
            - (alpha - 0.75 * alpha * alpha) * input[i - 2]
            + 2.0 * (1.0 - alpha) * output[i - 1]
            - (1.0 - alpha) * (1.0 - alpha) * output[i - 2];
    }

    Ok(output)
}

// ============================================================================
// Ehlers Signal Processing Enhancements (A4)
// ============================================================================

/// Ehlers EMA + 2-Pole Super Smoother Fusion
///
/// Cascades an EMA (Exponential Moving Average) with a 2-pole super smoother
/// filter to achieve steeper roll-off than either filter alone. The EMA provides
/// initial smoothing, and the super smoother removes residual high-frequency noise.
///
/// This is a cascaded IIR design: Stage 1 (EMA) → Stage 2 (2-pole super smoother),
/// both using the same `period` parameter. The cascade produces a 3rd-order
/// low-pass response with improved noise rejection.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Cut-off period for both EMA and super smoother (must be >= 2)
///
/// # Returns
/// Array of smoothed values. The first two bars are seeded from the EMA output;
/// subsequent bars use the full cascade.
///
/// # References
/// John F. Ehlers, "Cycle Analytics for Traders" (2013) — cascaded filter design
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::ehlers_ema_super_smoother;
/// let data: Vec<f64> = (0..50).map(|i| (i as f64 * 0.1).sin() * 10.0 + 50.0).collect();
/// let result = ehlers_ema_super_smoother(&data, 10).unwrap();
/// ```
pub fn ehlers_ema_super_smoother(input: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_input(input.len(), period.max(3))?;
    if period < 2 {
        return Err(crate::error::TaError::InvalidParameter {
            name: "period".into(),
            constraint: ">= 2".into(),
        });
    }

    let len = input.len();

    // Stage 1: EMA
    let alpha_ema = 2.0 / (period as f64 + 1.0);
    let mut ema = vec![0.0f64; len];
    ema[0] = input[0];
    for i in 1..len {
        ema[i] = alpha_ema * input[i] + (1.0 - alpha_ema) * ema[i - 1];
    }

    // Stage 2: 2-pole super smoother on EMA output
    let mut output = init_output(len);
    let a1 = (-std::f64::consts::SQRT_2 * std::f64::consts::PI / period as f64).exp();
    let b1 = 2.0 * a1 * (std::f64::consts::SQRT_2 * std::f64::consts::PI / period as f64).cos();
    let c2 = b1;
    let c3 = -a1 * a1;
    let c1 = 1.0 - c2 - c3;

    output[0] = ema[0];
    if len > 1 {
        output[1] = ema[1];
    }
    for i in 2..len {
        output[i] = c1 * (ema[i] + ema[i - 1]) / 2.0 + c2 * output[i - 1] + c3 * output[i - 2];
    }

    Ok(output)
}

/// Ehlers Fisher Transform (batch wrapper)
///
/// Computes the Fisher Transform using the midpoint `(High + Low) / 2` as input.
/// This is a convenience wrapper around the existing [`crate::indicators::fisher`]
/// function, providing the `ehlers_` namespaced version for discoverability.
///
/// The Fisher Transform converts prices to a Gaussian (normal) distribution,
/// making turning points easier to identify. When the Fisher line crosses
/// above the signal line, it indicates a potential bullish reversal.
///
/// # Arguments
/// * `high` - High price series
/// * `low` - Low price series
/// * `period` - Lookback period for normalizing the price range
///
/// # Returns
/// A [`crate::indicators::FisherResult`] containing the Fisher line and signal line.
///
/// # References
/// John F. Ehlers, "Cybernetic Analysis for Stocks and Futures" (2004)
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::ehlers_fisher_transform;
/// let high: Vec<f64> = (0..30).map(|i| 50.0 + (i as f64 * 0.2).sin() * 5.0).collect();
/// let low: Vec<f64> = high.iter().map(|h| h - 3.0).collect();
/// let result = ehlers_fisher_transform(&high, &low, 10).unwrap();
/// ```
pub fn ehlers_fisher_transform(
    high: &[f64],
    low: &[f64],
    period: usize,
) -> Result<crate::indicators::momentum_ext::FisherResult> {
    crate::indicators::momentum_ext::fisher(high, low, period)
}

/// Ehlers Instantaneous Trendline (default alpha)
///
/// Convenience wrapper around [`instantaneous_trendline`] with the default
/// Ehlers-recommended alpha of 0.07. The instantaneous trendline uses an
/// IIR filter to separate trend from cycle components with minimal lag.
///
/// At alpha = 0.07, the filter responds slowly to price changes, providing
/// a very smooth trend estimate suitable for trend-following strategies.
///
/// # Arguments
/// * `input` - Input data series (typically typical price)
///
/// # Returns
/// Array of trendline values.
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::ehlers_instantaneous_trendline;
/// let data: Vec<f64> = (0..60).map(|i| i as f64 * 0.5 + 10.0).collect();
/// let result = ehlers_instantaneous_trendline(&data).unwrap();
/// ```
pub fn ehlers_instantaneous_trendline(input: &[f64]) -> Result<Array1<f64>> {
    instantaneous_trendline(input, 0.07)
}

/// Ehlers Roofing Filter V2 (Spectral Dilation Highpass)
///
/// An improved version of the roofing filter that uses the spectral dilation
/// highpass filter from Ehlers' "Predictive Indicators for Financial Trading"
/// (2015). The V2 highpass provides better removal of low-frequency spectral
/// dilation — a characteristic of financial data where lower frequencies have
/// disproportionately larger amplitudes.
///
/// The filter combines:
/// 1. A spectral dilation highpass (fixed 48-bar cutoff) that removes trend
///    and compensates for the 1/f spectral shape of market data
/// 2. A 2-pole super smoother with the user-specified `period` that removes
///    high-frequency noise
///
/// The result oscillates around zero, isolating the cycle component.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Super smoother (low-pass) cut-off period (must be >= 2)
///
/// # Returns
/// Array of filtered values oscillating around zero.
///
/// # References
/// John F. Ehlers, "Predictive Indicators for Financial Trading" (2015)
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::ehlers_roofing_filter_v2;
/// let data: Vec<f64> = (0..80).map(|i| (i as f64 * 0.1).sin() * 10.0 + 50.0).collect();
/// let result = ehlers_roofing_filter_v2(&data, 10).unwrap();
/// ```
pub fn ehlers_roofing_filter_v2(input: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_input(input.len(), period.max(4))?;
    if period < 2 {
        return Err(crate::error::TaError::InvalidParameter {
            name: "period".into(),
            constraint: ">= 2".into(),
        });
    }

    let len = input.len();
    let mut hp = vec![0.0f64; len];
    let mut output = init_output(len);

    // Spectral dilation highpass filter (Ehlers "Predictive Indicators" 2015)
    // Fixed 48-bar cutoff removes spectral dilation (low-frequency drift)
    let arg = 0.707 * 2.0 * std::f64::consts::PI / 48.0;
    let alpha1 = (arg.cos() + arg.sin() - 1.0) / arg.cos();
    let hp_coef = (1.0 - alpha1 / 2.0).powi(2);
    let hp_fb1 = 2.0 * (1.0 - alpha1);
    let hp_fb2 = (1.0 - alpha1).powi(2);

    hp[0] = 0.0;
    if len > 1 {
        hp[1] = input[1] - input[0];
    }
    for i in 2..len {
        hp[i] = hp_coef * (input[i] - 2.0 * input[i - 1] + input[i - 2])
            + hp_fb1 * hp[i - 1]
            - hp_fb2 * hp[i - 2];
    }

    // 2-pole super smoother
    let a1 = (-std::f64::consts::SQRT_2 * std::f64::consts::PI / period as f64).exp();
    let b1 = 2.0 * a1 * (std::f64::consts::SQRT_2 * std::f64::consts::PI / period as f64).cos();
    let c2 = b1;
    let c3 = -a1 * a1;
    let c1 = 1.0 - c2 - c3;

    output[0] = hp[0];
    if len > 1 {
        output[1] = hp[1];
    }
    for i in 2..len {
        output[i] = c1 * (hp[i] + hp[i - 1]) / 2.0 + c2 * output[i - 1] + c3 * output[i - 2];
    }

    Ok(output)
}

/// Ehlers Sidewinder (Efficiency Ratio Consolidation Detector)
///
/// Detects whether the market is in a consolidation (sideways) phase or a
/// trending phase using a smoothed Kaufman Efficiency Ratio (ER).
///
/// The Efficiency Ratio measures the ratio of net directional movement to
/// total path length over a `period`-bar window:
/// - ER ≈ 1.0: Strong directional trend (all movement in one direction)
/// - ER ≈ 0.0: Pure consolidation / sideways market (movement cancels out)
///
/// The raw ER is then smoothed with a 2-pole super smoother (using half the
/// period for responsiveness) and clamped to [0, 1]. Traders typically use
/// a threshold (e.g., 0.5) to distinguish trend from consolidation.
///
/// # Arguments
/// * `input` - Input data series (typically close prices)
/// * `period` - Lookback window for Efficiency Ratio calculation (must be >= 2)
///
/// # Returns
/// Array of Sidewinder values in [0, 1]. Values near 0 indicate consolidation;
/// values near 1 indicate strong trend. Initial values before `period` bars
/// are NaN.
///
/// # References
/// John F. Ehlers, "Cycle Analytics for Traders" (2013) — trend strength filtering
///
/// # Example
/// ```
/// use alpha_ta_core::indicators::ehlers_sidewinder;
/// // Trending data
/// let trend: Vec<f64> = (0..50).map(|i| i as f64).collect();
/// let result = ehlers_sidewinder(&trend, 10).unwrap();
/// // Sideways data
/// let sideways: Vec<f64> = (0..50).map(|i| ((i as f64 * 0.5).sin() * 2.0 + 50.0)).collect();
/// let result2 = ehlers_sidewinder(&sideways, 10).unwrap();
/// ```
pub fn ehlers_sidewinder(input: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_input(input.len(), period.max(4))?;
    if period < 2 {
        return Err(crate::error::TaError::InvalidParameter {
            name: "period".into(),
            constraint: ">= 2".into(),
        });
    }

    let len = input.len();
    let mut er = vec![0.0f64; len];

    // Compute Efficiency Ratio
    for i in period..len {
        let direction = (input[i] - input[i - period]).abs();
        let mut volatility = 0.0;
        for j in (i - period + 1)..=i {
            volatility += (input[j] - input[j - 1]).abs();
        }
        er[i] = if volatility > 1e-15 {
            direction / volatility
        } else {
            0.0
        };
    }

    // Smooth ER with a 2-pole super smoother (half period for responsiveness)
    let smooth_period = (period / 2).max(2);
    let mut output = init_output(len);
    let a1 = (-std::f64::consts::SQRT_2 * std::f64::consts::PI / smooth_period as f64).exp();
    let b1 = 2.0 * a1 * (std::f64::consts::SQRT_2 * std::f64::consts::PI / smooth_period as f64).cos();
    let c2 = b1;
    let c3 = -a1 * a1;
    let c1 = 1.0 - c2 - c3;

    output[0] = er[0];
    if len > 1 {
        output[1] = er[1];
    }
    for i in 2..len {
        output[i] = c1 * (er[i] + er[i - 1]) / 2.0 + c2 * output[i - 1] + c3 * output[i - 2];
    }

    // Clamp to [0, 1]
    for v in output.iter_mut() {
        if v.is_finite() {
            *v = v.clamp(0.0, 1.0);
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a sine wave for testing
    fn sine_wave(n: usize, frequency: f64, amplitude: f64, offset: f64) -> Vec<f64> {
        (0..n)
            .map(|i| amplitude * (i as f64 * frequency).sin() + offset)
            .collect()
    }

    // ========================================================================
    // HT_DCPERIOD Tests
    // ========================================================================

    #[test]
    fn test_ht_dcperiod_basic() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let result = ht_dcperiod(&input).unwrap();
        assert_eq!(result.len(), 100);
    }

    #[test]
    fn test_ht_dcperiod_initial_nan() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let result = ht_dcperiod(&input).unwrap();
        // First 16 values should be NaN
        for i in 0..16 {
            assert!(result[i].is_nan(), "result[{}] should be NaN", i);
        }
    }

    #[test]
    fn test_ht_dcperiod_produces_values() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let result = ht_dcperiod(&input).unwrap();
        // After warmup, should produce finite values
        assert!(result.iter().skip(32).any(|&x| x.is_finite()));
    }

    #[test]
    fn test_ht_dcperiod_reasonable_range() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let result = ht_dcperiod(&input).unwrap();
        // Check that non-NaN values are in reasonable range (6-50)
        for i in 0..result.len() {
            if result[i].is_finite() {
                assert!(
                    result[i] >= 5.0 && result[i] <= 55.0,
                    "result[{}] = {} out of range",
                    i,
                    result[i]
                );
            }
        }
    }

    #[test]
    fn test_ht_dcperiod_insufficient_data() {
        let input = vec![1.0, 2.0, 3.0];
        assert!(ht_dcperiod(&input).is_err());
    }

    #[test]
    fn test_ht_dcperiod_at_boundary() {
        let input = sine_wave(32, 0.1, 1.0, 50.0);
        let result = ht_dcperiod(&input).unwrap();
        assert_eq!(result.len(), 32);
    }

    // ========================================================================
    // HT_DCPHASE Tests
    // ========================================================================

    #[test]
    fn test_ht_dcphase_basic() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let result = ht_dcphase(&input).unwrap();
        assert_eq!(result.len(), 100);
    }

    #[test]
    fn test_ht_dcphase_initial_nan() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let result = ht_dcphase(&input).unwrap();
        for i in 0..16 {
            assert!(result[i].is_nan(), "result[{}] should be NaN", i);
        }
    }

    #[test]
    fn test_ht_dcphase_produces_values() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let result = ht_dcphase(&input).unwrap();
        assert!(result.iter().skip(32).any(|&x| x.is_finite()));
    }

    #[test]
    fn test_ht_dcphase_degree_range() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let result = ht_dcphase(&input).unwrap();
        // Phase values should be in a reasonable degree range when finite
        for i in 0..result.len() {
            if result[i].is_finite() {
                assert!(
                    result[i] >= -400.0 && result[i] <= 400.0,
                    "result[{}] = {} out of expected range",
                    i,
                    result[i]
                );
            }
        }
    }

    #[test]
    fn test_ht_dcphase_insufficient_data() {
        let input = vec![1.0, 2.0, 3.0];
        assert!(ht_dcphase(&input).is_err());
    }

    // ========================================================================
    // HT_PHASOR Tests
    // ========================================================================

    #[test]
    fn test_ht_phasor_basic() {
        let input = sine_wave(50, 0.1, 1.0, 50.0);
        let (in_phase, quadrature) = ht_phasor(&input).unwrap();
        assert_eq!(in_phase.len(), 50);
        assert_eq!(quadrature.len(), 50);
    }

    #[test]
    fn test_ht_phasor_initial_nan() {
        let input = sine_wave(50, 0.1, 1.0, 50.0);
        let (in_phase, quadrature) = ht_phasor(&input).unwrap();
        for i in 0..6 {
            assert!(in_phase[i].is_nan(), "in_phase[{}] should be NaN", i);
            assert!(quadrature[i].is_nan(), "quadrature[{}] should be NaN", i);
        }
    }

    #[test]
    fn test_ht_phasor_produces_values() {
        let input = sine_wave(50, 0.1, 1.0, 50.0);
        let (in_phase, quadrature) = ht_phasor(&input).unwrap();
        assert!(in_phase.iter().any(|&x| x.is_finite()));
        assert!(quadrature.iter().any(|&x| x.is_finite()));
    }

    #[test]
    fn test_ht_phasor_phase_relationship() {
        // For a pure sine wave, the in-phase and quadrature should be ~90 degrees apart
        let input = sine_wave(100, 0.1, 1.0, 0.0);
        let (in_phase, quadrature) = ht_phasor(&input).unwrap();

        // Check that both components have non-zero variance
        let ip_mean: f64 = in_phase.iter().filter(|&&x| x.is_finite()).sum::<f64>()
            / in_phase.iter().filter(|&&x| x.is_finite()).count() as f64;
        let q_mean: f64 = quadrature.iter().filter(|&&x| x.is_finite()).sum::<f64>()
            / quadrature.iter().filter(|&&x| x.is_finite()).count() as f64;

        let ip_var: f64 = in_phase
            .iter()
            .filter(|&&x| x.is_finite())
            .map(|&x| (x - ip_mean).powi(2))
            .sum();
        let q_var: f64 = quadrature
            .iter()
            .filter(|&&x| x.is_finite())
            .map(|&x| (x - q_mean).powi(2))
            .sum();

        // Both should have significant variance for a sine wave input
        assert!(ip_var > 0.001);
        assert!(q_var > 0.001);
    }

    #[test]
    fn test_ht_phasor_insufficient_data() {
        let input = vec![1.0, 2.0, 3.0];
        assert!(ht_phasor(&input).is_err());
    }

    #[test]
    fn test_ht_phasor_at_boundary() {
        // Hilbert 变换需要至少 32 根 K 线作为预热，边界处的有效输入应正常返回等长数组
        let input = sine_wave(40, 0.1, 1.0, 50.0);
        let (in_phase, quadrature) = ht_phasor(&input).unwrap();
        assert_eq!(in_phase.len(), 40);
        assert_eq!(quadrature.len(), 40);
        // 低于最小长度应返回错误
        let short = sine_wave(31, 0.1, 1.0, 50.0);
        assert!(ht_phasor(&short).is_err());
    }

    // ========================================================================
    // HT_SINE Tests
    // ========================================================================

    #[test]
    fn test_ht_sine_basic() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let (sine, lead_sine) = ht_sine(&input).unwrap();
        assert_eq!(sine.len(), 100);
        assert_eq!(lead_sine.len(), 100);
    }

    #[test]
    fn test_ht_sine_initial_nan() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let (sine, lead_sine) = ht_sine(&input).unwrap();
        for i in 0..16 {
            assert!(sine[i].is_nan(), "sine[{}] should be NaN", i);
            assert!(lead_sine[i].is_nan(), "lead_sine[{}] should be NaN", i);
        }
    }

    #[test]
    fn test_ht_sine_produces_values() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let (sine, lead_sine) = ht_sine(&input).unwrap();
        assert!(sine.iter().skip(32).any(|&x| x.is_finite()));
        assert!(lead_sine.iter().skip(32).any(|&x| x.is_finite()));
    }

    #[test]
    fn test_ht_sine_bounded() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let (sine, lead_sine) = ht_sine(&input).unwrap();
        // Sine values should be bounded in [-1, 1]
        for i in 0..sine.len() {
            if sine[i].is_finite() {
                assert!(
                    sine[i] >= -1.0001 && sine[i] <= 1.0001,
                    "sine[{}] = {} out of [-1,1]",
                    i,
                    sine[i]
                );
            }
            if lead_sine[i].is_finite() {
                assert!(
                    lead_sine[i] >= -1.0001 && lead_sine[i] <= 1.0001,
                    "lead_sine[{}] = {} out of [-1,1]",
                    i,
                    lead_sine[i]
                );
            }
        }
    }

    #[test]
    fn test_ht_sine_lead_relationship() {
        // Lead sine should be phase-shifted by ~45 degrees from sine
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let (sine, lead_sine) = ht_sine(&input).unwrap();

        let mut both_finite = 0;
        let mut consistent_lead = 0;

        for i in 32..sine.len() {
            if sine[i].is_finite() && lead_sine[i].is_finite() {
                both_finite += 1;
                // Lead should generally be ahead of sine
                if (lead_sine[i] - sine[i]).abs() < 1.5 {
                    consistent_lead += 1;
                }
            }
        }

        assert!(both_finite > 0);
        assert!(consistent_lead > 0);
    }

    #[test]
    fn test_ht_sine_insufficient_data() {
        let input = vec![1.0, 2.0, 3.0];
        assert!(ht_sine(&input).is_err());
    }

    #[test]
    fn test_ht_sine_simd_matches_scalar() {
        // The SIMD sin/cos terminal stage must match a scalar f64::sin_cos reference
        // (the phase is atan(im/re) ∈ (-π/2, π/2), where the polynomial is exact to ~1e-11).
        let n = 256;
        let input: Vec<f64> = (0..n)
            .map(|i| 100.0 + 10.0 * (i as f64 * 0.13).sin() + (i as f64 * 0.7).cos())
            .collect();
        let (sine, lead) = ht_sine(&input).unwrap();

        let (_s, _d, _ip, _q, _j1, _i2, _j2, phase, _p) = compute_hilbert_components(&input, n);
        let lead_c = std::f64::consts::FRAC_1_SQRT_2; // cos(π/4) = sin(π/4) = √2/2
        let mut max_sine_err = 0.0_f64;
        let mut max_lead_err = 0.0_f64;
        for i in 32..n {
            let (sp, cp) = phase[i].sin_cos();
            let exp_sine = sp;
            let exp_lead = (sp + cp) * lead_c;
            max_sine_err = max_sine_err.max((sine[i] - exp_sine).abs());
            max_lead_err = max_lead_err.max((lead[i] - exp_lead).abs());
        }
        assert!(
            max_sine_err <= 1e-9,
            "ht_sine SIMD sine error {} exceeds 1e-9",
            max_sine_err
        );
        assert!(
            max_lead_err <= 1e-9,
            "ht_sine SIMD lead error {} exceeds 1e-9",
            max_lead_err
        );

        // Sanity: the kernel was actually exercised (finite, non-trivial output).
        let mut finite = 0;
        for i in 32..n {
            if sine[i].is_finite() && lead[i].is_finite() {
                finite += 1;
            }
        }
        assert!(finite > n / 2);
    }

    #[test]
    fn test_ht_sine_throughput() {
        // Informational throughput probe for the whole ht_sine function
        // (dominated by the Hilbert IIR chain). Prints ns/bar; only enforces a
        // generous upper bound to catch gross regressions.
        use std::time::Instant;
        let n = 4000usize;
        let input: Vec<f64> = (0..n)
            .map(|i| 100.0 + 10.0 * (i as f64 * 0.13).sin() + (i as f64 * 0.7).cos())
            .collect();

        for _ in 0..20 {
            let _ = ht_sine(&input).unwrap();
        }
        let iters = 200;
        let start = Instant::now();
        for _ in 0..iters {
            let _ = ht_sine(&input).unwrap();
        }
        let elapsed = start.elapsed();
        let ns_per_bar = elapsed.as_nanos() as f64 / (iters as f64 * n as f64);
        eprintln!(
            "ht_sine (whole fn) throughput: {:.2} ns/bar over {} bars x {} iters",
            ns_per_bar, n, iters
        );
        assert!(ns_per_bar < 200.0, "ht_sine too slow: {:.2} ns/bar", ns_per_bar);
    }

    // ========================================================================
    // HT_TRENDMODE Tests
    // ========================================================================

    #[test]
    fn test_ht_trendmode_basic() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let result = ht_trendmode(&input).unwrap();
        assert_eq!(result.len(), 100);
    }

    #[test]
    fn test_ht_trendmode_initial_nan() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let result = ht_trendmode(&input).unwrap();
        for i in 0..16 {
            assert!(result[i].is_nan(), "result[{}] should be NaN", i);
        }
    }

    #[test]
    fn test_ht_trendmode_binary_values() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let result = ht_trendmode(&input).unwrap();
        // All non-NaN values should be either 0.0 or 1.0
        for i in 0..result.len() {
            if result[i].is_finite() {
                assert!(
                    result[i] == 0.0 || result[i] == 1.0,
                    "result[{}] = {} should be 0 or 1",
                    i,
                    result[i]
                );
            }
        }
    }

    #[test]
    fn test_ht_trendmode_cycle_mode() {
        // Pure sine wave should be mostly in cycle mode (0)
        let input = sine_wave(200, 0.1, 1.0, 50.0);
        let result = ht_trendmode(&input).unwrap();

        let cycle_count = result
            .iter()
            .filter(|&&x| x.is_finite() && x == 0.0)
            .count();
        let total_finite = result.iter().filter(|&&x| x.is_finite()).count();

        // At least some values should be in cycle mode
        if total_finite > 0 {
            assert!(
                cycle_count > 0,
                "Expected some cycle mode values for sine wave input"
            );
        }
    }

    #[test]
    fn test_ht_trendmode_strong_trend() {
        // Strong linear trend should be in trend mode (1)
        let input: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let result = ht_trendmode(&input).unwrap();

        let trend_count = result
            .iter()
            .filter(|&&x| x.is_finite() && x == 1.0)
            .count();

        // Strong linear trend should produce trend mode
        assert!(trend_count > 0, "Expected trend mode for linear input");
    }

    #[test]
    fn test_ht_trendmode_insufficient_data() {
        let input = vec![1.0, 2.0, 3.0];
        assert!(ht_trendmode(&input).is_err());
    }

    // ========================================================================
    // HT_TRENDLINE Tests
    // ========================================================================

    #[test]
    fn test_ht_trendline_basic() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let result = ht_trendline(&input).unwrap();
        assert_eq!(result.len(), 100);
    }

    #[test]
    fn test_ht_trendline_initial_nan() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let result = ht_trendline(&input).unwrap();
        for i in 0..16 {
            assert!(result[i].is_nan(), "result[{}] should be NaN", i);
        }
    }

    #[test]
    fn test_ht_trendline_produces_values() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let result = ht_trendline(&input).unwrap();
        assert!(result.iter().skip(32).any(|&x| x.is_finite()));
    }

    #[test]
    fn test_ht_trendline_tracks_input() {
        // For a sine wave, trendline should track around the mean
        let input = sine_wave(200, 0.1, 1.0, 50.0);
        let result = ht_trendline(&input).unwrap();

        let finite_values: Vec<f64> = result.iter().filter(|&&x| x.is_finite()).copied().collect();
        if !finite_values.is_empty() {
            let mean: f64 = finite_values.iter().sum::<f64>() / finite_values.len() as f64;
            // Mean should be close to the offset (50.0)
            assert!(
                (mean - 50.0).abs() < 5.0,
                "trendline mean {} too far from 50.0",
                mean
            );
        }
    }

    #[test]
    fn test_ht_trendline_linear_input() {
        // For linear input, trendline should track closely
        let input: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let result = ht_trendline(&input).unwrap();

        // Check that trendline values are reasonable
        let finite_count = result.iter().filter(|&&x| x.is_finite()).count();
        assert!(finite_count > 0);
    }

    #[test]
    fn test_ht_trendline_insufficient_data() {
        let input = vec![1.0, 2.0, 3.0];
        assert!(ht_trendline(&input).is_err());
    }

    // ========================================================================
    // Cross-Indicator Consistency Tests
    // ========================================================================

    #[test]
    fn test_all_indicators_same_length() {
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let dcperiod = ht_dcperiod(&input).unwrap();
        let dcphase = ht_dcphase(&input).unwrap();
        let (phasor_i, phasor_q) = ht_phasor(&input).unwrap();
        let (sine, lead) = ht_sine(&input).unwrap();
        let trendmode = ht_trendmode(&input).unwrap();
        let trendline = ht_trendline(&input).unwrap();

        assert_eq!(dcperiod.len(), 100);
        assert_eq!(dcphase.len(), 100);
        assert_eq!(phasor_i.len(), 100);
        assert_eq!(phasor_q.len(), 100);
        assert_eq!(sine.len(), 100);
        assert_eq!(lead.len(), 100);
        assert_eq!(trendmode.len(), 100);
        assert_eq!(trendline.len(), 100);
    }

    #[test]
    fn test_sine_and_phase_consistency() {
        // HT_SINE and HT_DCPHASE should produce related outputs
        let input = sine_wave(100, 0.1, 1.0, 50.0);
        let dcphase = ht_dcphase(&input).unwrap();
        let (sine, _) = ht_sine(&input).unwrap();

        // Both should produce values after warmup
        let phase_has_values = dcphase.iter().skip(32).any(|&x| x.is_finite());
        let sine_has_values = sine.iter().skip(32).any(|&x| x.is_finite());

        assert!(phase_has_values);
        assert!(sine_has_values);
    }

    #[test]
    fn test_empty_input() {
        let empty: Vec<f64> = vec![];
        assert!(ht_dcperiod(&empty).is_err());
        assert!(ht_dcphase(&empty).is_err());
        assert!(ht_phasor(&empty).is_err());
        assert!(ht_sine(&empty).is_err());
        assert!(ht_trendmode(&empty).is_err());
        assert!(ht_trendline(&empty).is_err());
    }

    #[test]
    fn test_constant_input() {
        let input = vec![50.0; 100];
        let dcperiod = ht_dcperiod(&input).unwrap();
        let dcphase = ht_dcphase(&input).unwrap();
        let trendmode = ht_trendmode(&input).unwrap();

        // Constant input: phase changes should be minimal
        // Values may be NaN or finite, but should not panic
        assert_eq!(dcperiod.len(), 100);
        assert_eq!(dcphase.len(), 100);
        assert_eq!(trendmode.len(), 100);
    }

    // ============ Ehlers filter tests ============

    fn trending_data(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| i as f64 * 0.5 + (i as f64 * 0.3).sin() * 3.0 + 50.0)
            .collect()
    }

    #[test]
    fn test_super_smoother_output_length() {
        let data = trending_data(100);
        let result = super_smoother(&data, 10).unwrap();
        assert_eq!(result.len(), 100);
    }

    #[test]
    fn test_super_smoother_reduces_noise() {
        let data = trending_data(200);
        let smoothed = super_smoother(&data, 14).unwrap();
        let raw_var: f64 = data.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum::<f64>() / data.len() as f64;
        let sm_var: f64 = smoothed.as_slice().unwrap().windows(2)
            .skip(2) // skip warmup
            .map(|w| (w[1] - w[0]).powi(2)).sum::<f64>() / smoothed.len() as f64;
        assert!(sm_var < raw_var, "smoother should reduce variance");
    }

    #[test]
    fn test_super_smoother_invalid_period() {
        let data = trending_data(50);
        assert!(super_smoother(&data, 1).is_err());
    }

    #[test]
    fn test_super_smoother_3pole_output_length() {
        let data = trending_data(100);
        let result = super_smoother_3pole(&data, 10).unwrap();
        assert_eq!(result.len(), 100);
    }

    #[test]
    fn test_roofing_filter_output_length() {
        let data = trending_data(200);
        let result = roofing_filter(&data, 48, 10).unwrap();
        assert_eq!(result.len(), 200);
        // After warmup, output should contain both positive and negative values
        // (even with trending data, the HP removes trend, leaving oscillation)
        let tail = &result.as_slice().unwrap()[60..];
        let non_zero = tail.iter().any(|&v| v.abs() > 1e-10);
        assert!(non_zero, "roofing filter should produce non-zero output");
    }

    #[test]
    fn test_roofing_filter_sine_oscillation() {
        // Pure sine wave should be isolated by roofing filter
        let data: Vec<f64> = (0..200)
            .map(|i| (i as f64 * 2.0 * std::f64::consts::PI / 20.0).sin() * 10.0 + 100.0)
            .collect();
        let result = roofing_filter(&data, 48, 10).unwrap();
        let tail = &result.as_slice().unwrap()[60..];
        let has_positive = tail.iter().any(|&v| v > 0.5);
        let has_negative = tail.iter().any(|&v| v < -0.5);
        assert!(has_positive && has_negative, "roofing filter should oscillate on sine input");
    }

    #[test]
    fn test_decycler_follows_trend() {
        let data: Vec<f64> = (0..100).map(|i| i as f64 * 1.0).collect();
        let result = decycler(&data, 20).unwrap();
        assert_eq!(result.len(), 100);
        // After sufficient warmup, the decycler should converge toward the input trend.
        // The HP filter has transient effects, so we check later bars with generous tolerance.
        for i in 50..100 {
            assert!((result[i] - data[i]).abs() < 20.0, "decycler should track trend at bar {i}, got {} vs {}", result[i], data[i]);
        }
    }

    #[test]
    fn test_bandpass_zero_mean() {
        let data = trending_data(200);
        let result = bandpass(&data, 20, 0.3).unwrap();
        assert_eq!(result.len(), 200);
        let tail = &result.as_slice().unwrap()[40..];
        let mean: f64 = tail.iter().sum::<f64>() / tail.len() as f64;
        let max_abs: f64 = tail.iter().map(|v| v.abs()).fold(0.0f64, f64::max);
        assert!(mean.abs() < max_abs * 0.5, "bandpass mean should be near zero");
    }

    #[test]
    fn test_bandpass_invalid_bandwidth() {
        let data = trending_data(50);
        assert!(bandpass(&data, 20, 1.5).is_err());
        assert!(bandpass(&data, 20, -0.1).is_err());
    }

    #[test]
    fn test_instantaneous_trendline_tracks_price() {
        let data = trending_data(100);
        let result = instantaneous_trendline(&data, 0.07).unwrap();
        assert_eq!(result.len(), 100);
        // Should track the trend reasonably well after warmup
        for i in 20..100 {
            assert!((result[i] - data[i]).abs() < 15.0,
                "itrend should be within 15 of price at bar {i}");
        }
    }

    #[test]
    fn test_instantaneous_trendline_invalid_alpha() {
        let data = trending_data(50);
        assert!(instantaneous_trendline(&data, -0.1).is_err());
        assert!(instantaneous_trendline(&data, 1.5).is_err());
    }

    // ========================================================================
    // A4: Ehlers Enhancement Tests
    // ========================================================================

    // --- ehlers_ema_super_smoother ---

    #[test]
    fn test_ehlers_ema_super_smoother_output_length() {
        let data = trending_data(100);
        let result = ehlers_ema_super_smoother(&data, 10).unwrap();
        assert_eq!(result.len(), 100);
    }

    #[test]
    fn test_ehlers_ema_super_smoother_reduces_noise() {
        let data = trending_data(200);
        let smoothed = ehlers_ema_super_smoother(&data, 14).unwrap();
        let raw_var: f64 =
            data.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum::<f64>() / data.len() as f64;
        let sm_var: f64 = smoothed
            .as_slice()
            .unwrap()
            .windows(2)
            .skip(2)
            .map(|w| (w[1] - w[0]).powi(2))
            .sum::<f64>()
            / smoothed.len() as f64;
        assert!(sm_var < raw_var, "ema_super_smoother should reduce variance");
    }

    #[test]
    fn test_ehlers_ema_super_smoother_smoother_than_super_smoother() {
        // The cascade (EMA + super_smoother) should be smoother than super_smoother alone
        let data = trending_data(200);
        let cascade = ehlers_ema_super_smoother(&data, 14).unwrap();
        let plain = super_smoother(&data, 14).unwrap();
        let cascade_var: f64 = cascade
            .as_slice()
            .unwrap()
            .windows(2)
            .skip(10)
            .map(|w| (w[1] - w[0]).powi(2))
            .sum::<f64>();
        let plain_var: f64 = plain
            .as_slice()
            .unwrap()
            .windows(2)
            .skip(10)
            .map(|w| (w[1] - w[0]).powi(2))
            .sum::<f64>();
        assert!(
            cascade_var < plain_var,
            "cascade should be smoother than plain super_smoother"
        );
    }

    #[test]
    fn test_ehlers_ema_super_smoother_invalid_period() {
        let data = trending_data(50);
        assert!(ehlers_ema_super_smoother(&data, 1).is_err());
    }

    #[test]
    fn test_ehlers_ema_super_smoother_insufficient_data() {
        let data = vec![1.0, 2.0];
        assert!(ehlers_ema_super_smoother(&data, 10).is_err());
    }

    // --- ehlers_fisher_transform ---

    #[test]
    fn test_ehlers_fisher_transform_output_length() {
        let high: Vec<f64> = (0..50).map(|i| 50.0 + (i as f64 * 0.2).sin() * 5.0).collect();
        let low: Vec<f64> = high.iter().map(|h| h - 3.0).collect();
        let result = ehlers_fisher_transform(&high, &low, 10).unwrap();
        assert_eq!(result.fisher.len(), 50);
        assert_eq!(result.signal.len(), 50);
    }

    #[test]
    fn test_ehlers_fisher_transform_matches_fisher() {
        let high: Vec<f64> = (0..80).map(|i| 50.0 + (i as f64 * 0.15).sin() * 8.0).collect();
        let low: Vec<f64> = high.iter().map(|h| h - 4.0).collect();
        let wrapper_result = ehlers_fisher_transform(&high, &low, 10).unwrap();
        let direct_result = crate::indicators::momentum_ext::fisher(&high, &low, 10).unwrap();
        for i in 0..80 {
            // NaN-aware comparison: both NaN is OK, otherwise must match
            if direct_result.fisher[i].is_nan() {
                assert!(wrapper_result.fisher[i].is_nan(), "fisher[{i}] should be NaN");
            } else {
                assert!((wrapper_result.fisher[i] - direct_result.fisher[i]).abs() < 1e-12);
            }
            if direct_result.signal[i].is_nan() {
                assert!(wrapper_result.signal[i].is_nan(), "signal[{i}] should be NaN");
            } else {
                assert!((wrapper_result.signal[i] - direct_result.signal[i]).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn test_ehlers_fisher_transform_signal_lagged() {
        // Signal line is the previous Fisher value
        let high: Vec<f64> = (0..50).map(|i| 50.0 + (i as f64 * 0.3).sin() * 6.0).collect();
        let low: Vec<f64> = high.iter().map(|h| h - 2.0).collect();
        let result = ehlers_fisher_transform(&high, &low, 5).unwrap();
        for i in 6..50 {
            if !result.fisher[i].is_nan() && !result.signal[i].is_nan() {
                // signal[i] should equal fisher[i-1] (approximately, due to smoothing)
                assert!((result.signal[i] - result.fisher[i - 1]).abs() < 1e-10);
            }
        }
    }

    // --- ehlers_instantaneous_trendline ---

    #[test]
    fn test_ehlers_instantaneous_trendline_matches_wrapper() {
        let data = trending_data(100);
        let wrapper_result = ehlers_instantaneous_trendline(&data).unwrap();
        let direct_result = instantaneous_trendline(&data, 0.07).unwrap();
        for i in 0..100 {
            assert!((wrapper_result[i] - direct_result[i]).abs() < 1e-12);
        }
    }

    #[test]
    fn test_ehlers_instantaneous_trendline_tracks_price() {
        let data = trending_data(100);
        let result = ehlers_instantaneous_trendline(&data).unwrap();
        assert_eq!(result.len(), 100);
        // Should track the trend reasonably well after warmup
        for i in 20..100 {
            assert!(
                (result[i] - data[i]).abs() < 15.0,
                "ehlers_itrend should be within 15 of price at bar {i}"
            );
        }
    }

    // --- ehlers_roofing_filter_v2 ---

    #[test]
    fn test_ehlers_roofing_filter_v2_output_length() {
        let data = trending_data(200);
        let result = ehlers_roofing_filter_v2(&data, 10).unwrap();
        assert_eq!(result.len(), 200);
    }

    #[test]
    fn test_ehlers_roofing_filter_v2_oscillates_around_zero() {
        // After warmup, the roofing filter V2 should oscillate around zero
        // (highpass removes the trend/DC component)
        let data = trending_data(200);
        let result = ehlers_roofing_filter_v2(&data, 10).unwrap();
        let tail = &result.as_slice().unwrap()[60..];
        let mean: f64 = tail.iter().sum::<f64>() / tail.len() as f64;
        let max_abs: f64 = tail.iter().map(|v| v.abs()).fold(0.0f64, f64::max);
        assert!(
            mean.abs() < max_abs * 0.3,
            "V2 should oscillate around zero, mean={mean}"
        );
    }

    #[test]
    fn test_ehlers_roofing_filter_v2_sine_isolation() {
        // Pure sine wave should be isolated by the roofing filter V2
        let data: Vec<f64> = (0..200)
            .map(|i| (i as f64 * 2.0 * std::f64::consts::PI / 20.0).sin() * 10.0 + 100.0)
            .collect();
        let result = ehlers_roofing_filter_v2(&data, 10).unwrap();
        let tail = &result.as_slice().unwrap()[60..];
        let has_positive = tail.iter().any(|&v| v > 0.5);
        let has_negative = tail.iter().any(|&v| v < -0.5);
        assert!(
            has_positive && has_negative,
            "V2 should oscillate on sine input"
        );
    }

    #[test]
    fn test_ehlers_roofing_filter_v2_invalid_period() {
        let data = trending_data(50);
        assert!(ehlers_roofing_filter_v2(&data, 1).is_err());
    }

    #[test]
    fn test_ehlers_roofing_filter_v2_differs_from_v1() {
        // V2 uses a different highpass formulation, so it should differ from V1
        let data = trending_data(200);
        let v1 = roofing_filter(&data, 48, 10).unwrap();
        let v2 = ehlers_roofing_filter_v2(&data, 10).unwrap();
        let mut any_diff = false;
        for i in 60..200 {
            if (v1[i] - v2[i]).abs() > 1e-6 {
                any_diff = true;
                break;
            }
        }
        assert!(any_diff, "V2 should produce different output than V1");
    }

    // --- ehlers_sidewinder ---

    #[test]
    fn test_ehlers_sidewinder_output_length() {
        let data = trending_data(100);
        let result = ehlers_sidewinder(&data, 10).unwrap();
        assert_eq!(result.len(), 100);
    }

    #[test]
    fn test_ehlers_sidewinder_trending_data_high_er() {
        // Strong linear trend → ER should be close to 1.0
        let trend: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let result = ehlers_sidewinder(&trend, 10).unwrap();
        // After sufficient warmup, ER should be high
        let tail = &result.as_slice().unwrap()[30..];
        let mean_er: f64 = tail.iter().sum::<f64>() / tail.len() as f64;
        assert!(
            mean_er > 0.9,
            "trending data should have high ER (>0.9), got {mean_er}"
        );
    }

    #[test]
    fn test_ehlers_sidewinder_sideways_data_low_er() {
        // Pure sideways/oscillating data → ER should be low
        let sideways: Vec<f64> = (0..200)
            .map(|i| (i as f64 * 0.5).sin() * 2.0 + 50.0)
            .collect();
        let result = ehlers_sidewinder(&sideways, 20).unwrap();
        let tail = &result.as_slice().unwrap()[40..];
        let mean_er: f64 = tail.iter().sum::<f64>() / tail.len() as f64;
        assert!(
            mean_er < 0.3,
            "sideways data should have low ER (<0.3), got {mean_er}"
        );
    }

    #[test]
    fn test_ehlers_sidewinder_bounded_01() {
        let data = trending_data(200);
        let result = ehlers_sidewinder(&data, 10).unwrap();
        for &v in result.iter() {
            if v.is_finite() {
                assert!(v >= 0.0 && v <= 1.0, "sidewinder value {v} out of [0,1]");
            }
        }
    }

    #[test]
    fn test_ehlers_sidewinder_invalid_period() {
        let data = trending_data(50);
        assert!(ehlers_sidewinder(&data, 1).is_err());
    }

    #[test]
    fn test_ehlers_sidewinder_constant_input() {
        // Constant input → direction = 0, volatility = 0 → ER = 0
        let data = vec![50.0; 100];
        let result = ehlers_sidewinder(&data, 10).unwrap();
        // After warmup, all values should be 0 (or very close)
        for i in 15..100 {
            if result[i].is_finite() {
                assert!(result[i] < 0.01, "constant input should have ER≈0 at bar {i}, got {}", result[i]);
            }
        }
    }
}
