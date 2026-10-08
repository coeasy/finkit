//! MACD, MACDEXT and MACDFIX.

use super::prelude::*;

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
pub(crate) fn macd_inner(
    input: &[f64],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> Result<MacdResult> {
    let len = input.len();
    // All three outputs are written completely below. Keep them uninitialized
    // until each slot receives its final value so the public owned-array path
    // does not pay three full NaN-fill passes before the MACD recurrences.
    let mut macd_line = Vec::with_capacity(len);
    let mut signal = Vec::with_capacity(len);
    let mut hist = Vec::with_capacity(len);
    unsafe {
        macd_line.set_len(len);
        signal.set_len(len);
        hist.set_len(len);
    }

    if len == 0 {
        return Ok(MacdResult {
            macd: Array1::from_vec(macd_line),
            signal: Array1::from_vec(signal),
            hist: Array1::from_vec(hist),
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

    for value in &mut macd_line[..macd_start.min(len)] {
        *value = f64::NAN;
    }
    let signal_start = macd_start + signal_period - 1;
    for value in &mut signal[..signal_start.min(len)] {
        *value = f64::NAN;
    }
    for value in &mut hist[..signal_start.min(len)] {
        *value = f64::NAN;
    }

    // 种子点处的 MACD 值
    let mut macd_val = prev_fast - prev_slow;
    macd_line[macd_start] = macd_val;

    // EMA 递推：使用 FMA 精确匹配 TA-Lib 的浮点舍入路径
    for i in slow_period..len {
        let val = input[i];
        prev_fast = (val - prev_fast) * fast_k + prev_fast;
        prev_slow = (val - prev_slow) * slow_k + prev_slow;
        macd_val = prev_fast - prev_slow;
        macd_line[i] = macd_val;
    }

    // Signal line：SMA 种子 + FMA 递推
    if len > signal_start {
        let mut sig_sum: f64 = 0.0;
        for i in macd_start..=signal_start {
            sig_sum += macd_line[i];
        }
        let mut prev_signal = sig_sum / signal_period as f64;
        signal[signal_start] = prev_signal;
        hist[signal_start] = macd_line[signal_start] - prev_signal;

        for i in (signal_start + 1)..len {
            let m = macd_line[i];
            prev_signal = (m - prev_signal) * signal_k + prev_signal;
            signal[i] = prev_signal;
            hist[i] = m - prev_signal;
        }
    }

    // Public TA-Lib contract: MACD, signal and histogram share one lookback.
    // Earlier MACD values are internal signal-seed intermediates, not outputs.
    for index in macd_start..signal_start.min(len) {
        macd_line[index] = f64::NAN;
    }

    Ok(MacdResult {
        macd: Array1::from_vec(macd_line),
        signal: Array1::from_vec(signal),
        hist: Array1::from_vec(hist),
    })
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
pub(crate) fn macdext_sma(
    input: &[f64],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
) -> Result<MacdResult> {
    if fast_period == 0 || slow_period == 0 || signal_period == 0 {
        return Err(TaError::InvalidParameter {
            name: "fast_period/slow_period/signal_period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    let lookback = slow_period + signal_period - 1;
    validate_input(input.len(), lookback)?;

    let len = input.len();
    let mut macd_line = vec![f64::NAN; len];
    let mut fast_sum = input[..fast_period].iter().sum::<f64>();
    let mut slow_sum = input[..slow_period].iter().sum::<f64>();
    let fast_start = fast_period - 1;
    let slow_start = slow_period - 1;

    for i in 0..len {
        if i >= fast_period {
            fast_sum += input[i] - input[i - fast_period];
        }
        if i >= slow_period {
            slow_sum += input[i] - input[i - slow_period];
        }
        if i >= slow_start {
            let fast = if i == fast_start || i > fast_start {
                fast_sum / fast_period as f64
            } else {
                f64::NAN
            };
            if !fast.is_nan() {
                macd_line[i] = fast - slow_sum / slow_period as f64;
            }
        }
    }

    let signal_start = slow_start + signal_period - 1;
    let mut signal = vec![f64::NAN; len];
    let mut hist = vec![f64::NAN; len];
    let mut signal_sum = macd_line[slow_start..=signal_start].iter().sum::<f64>();
    signal[signal_start] = signal_sum / signal_period as f64;
    hist[signal_start] = macd_line[signal_start] - signal[signal_start];
    for i in signal_start + 1..len {
        signal_sum += macd_line[i] - macd_line[i - signal_period];
        signal[i] = signal_sum / signal_period as f64;
        hist[i] = macd_line[i] - signal[i];
    }
    macd_line[..signal_start].fill(f64::NAN);

    Ok(MacdResult {
        macd: Array1::from_vec(macd_line),
        signal: Array1::from_vec(signal),
        hist: Array1::from_vec(hist),
    })
}

pub fn macdext(
    input: &[f64],
    fast_period: usize,
    fast_ma_type: MaType,
    slow_period: usize,
    slow_ma_type: MaType,
    signal_period: usize,
    signal_ma_type: MaType,
) -> Result<MacdResult> {
    if fast_ma_type == MaType::Sma && slow_ma_type == MaType::Sma && signal_ma_type == MaType::Sma {
        return macdext_sma(input, fast_period, slow_period, signal_period);
    }
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
    macdfix_with_signal(input, 9)
}

/// MACDFIX with an explicit signal period, matching TA-Lib's optional
/// `signalperiod` argument while keeping `macdfix`'s historical default.
pub fn macdfix_with_signal(input: &[f64], signal_period: usize) -> Result<MacdResult> {
    if signal_period == 0 {
        return Err(TaError::InvalidParameter {
            name: "signal_period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), 26 + signal_period - 1)?;
    let len = input.len();
    let mut macd_line = vec![f64::NAN; len];
    let mut signal = vec![f64::NAN; len];
    let mut hist = vec![f64::NAN; len];

    // MACDFIX uses the fixed TA-Lib smoothing constants 0.15 and 0.075,
    // rather than recomputing 2/(period+1) as the general MACD path does.
    // The distinction is small but accumulates enough to fail parity.
    let fast_period = 12;
    let slow_period = 26;
    let fast_k = 0.15;
    let slow_k = 0.075;
    let signal_k = 2.0 / (signal_period as f64 + 1.0);
    let mut slow_sum = 0.0;
    for &value in &input[..slow_period] {
        slow_sum += value;
    }
    let mut fast_sum = 0.0;
    for &value in &input[slow_period - fast_period..slow_period] {
        fast_sum += value;
    }
    let mut fast = fast_sum / fast_period as f64;
    let mut slow = slow_sum / slow_period as f64;
    let first_macd = slow_period - 1;
    let mut macd_value = fast - slow;
    macd_line[first_macd] = macd_value;
    for (i, &value) in input.iter().enumerate().skip(slow_period) {
        fast = (value - fast) * fast_k + fast;
        slow = (value - slow) * slow_k + slow;
        macd_value = fast - slow;
        macd_line[i] = macd_value;
    }

    let first_output = first_macd + signal_period - 1;
    let mut signal_value =
        macd_line[first_macd..=first_output].iter().sum::<f64>() / signal_period as f64;
    signal[first_output] = signal_value;
    hist[first_output] = macd_line[first_output] - signal_value;
    for i in first_output + 1..len {
        signal_value = (macd_line[i] - signal_value) * signal_k + signal_value;
        signal[i] = signal_value;
        hist[i] = macd_line[i] - signal_value;
    }
    // TA-Lib only exposes the stable signal zone for MACDFIX.
    macd_line[..first_output].fill(f64::NAN);
    Ok(MacdResult {
        macd: Array1::from_vec(macd_line),
        signal: Array1::from_vec(signal),
        hist: Array1::from_vec(hist),
    })
}

/// Zero-copy MACDFIX kernel for bindings that already own the three output
/// buffers.  The recurrence is kept separate from the allocating API so the
/// compatibility layer does not materialize an intermediate `MacdResult`.
pub fn macdfix_into(
    input: &[f64],
    signal_period: usize,
    macd_line: &mut [f64],
    signal: &mut [f64],
    hist: &mut [f64],
) -> Result<()> {
    if signal_period == 0 {
        return Err(TaError::InvalidParameter {
            name: "signal_period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), 26 + signal_period - 1)?;
    if macd_line.len() != input.len() || signal.len() != input.len() || hist.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output slices".to_string(),
            constraint: "must each have the same length as input".to_string(),
        });
    }
    let len = input.len();

    let mut slow_sum = 0.0;
    for &value in &input[..26] {
        slow_sum += value;
    }
    let mut fast_sum = 0.0;
    for &value in &input[14..26] {
        fast_sum += value;
    }
    let mut fast = fast_sum / 12.0;
    let mut slow = slow_sum / 26.0;
    let first_macd = 25;
    let mut macd_value = fast - slow;
    macd_line[first_macd] = macd_value;
    for (i, &value) in input.iter().enumerate().skip(26) {
        fast = (value - fast) * 0.15 + fast;
        slow = (value - slow) * 0.075 + slow;
        macd_value = fast - slow;
        macd_line[i] = macd_value;
    }

    let first_output = first_macd + signal_period - 1;
    signal[..first_output].fill(f64::NAN);
    hist[..first_output].fill(f64::NAN);
    let mut signal_value =
        macd_line[first_macd..=first_output].iter().sum::<f64>() / signal_period as f64;
    signal[first_output] = signal_value;
    hist[first_output] = macd_line[first_output] - signal_value;
    let signal_k = 2.0 / (signal_period as f64 + 1.0);
    for i in first_output + 1..len {
        signal_value = (macd_line[i] - signal_value) * signal_k + signal_value;
        signal[i] = signal_value;
        hist[i] = macd_line[i] - signal_value;
    }
    // TA-Lib only exposes the stable signal zone for MACDFIX; the earlier
    // MACD state is used internally to seed the signal but remains NaN.
    macd_line[..first_output].fill(f64::NAN);
    Ok(())
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
    if fast_period == 0 || slow_period == 0 || signal_period == 0 || fast_period >= slow_period {
        return Err(TaError::InvalidParameter {
            name: "periods".to_string(),
            constraint: "fast/slow/signal periods must be greater than 0 and fast < slow"
                .to_string(),
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
        prev_fast = (val - prev_fast) * fast_k + prev_fast;
        prev_slow = (val - prev_slow) * slow_k + prev_slow;
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
        histogram[signal_start] = macd_line[signal_start] - prev_signal;

        for i in (signal_start + 1)..len {
            let m = macd_line[i];
            prev_signal = (m - prev_signal) * signal_k + prev_signal;
            signal[i] = prev_signal;
            histogram[i] = m - prev_signal;
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

    // Same TA-Lib public lookback as macd(): pre-signal values are seed state.
    macd_line[macd_start..signal_start].fill(f64::NAN);

    Ok(())
}

/// Public-boundary MACD kernel using the AVX2/FMA four-sample EMA block.
///
/// The regular [`macd_into`] path remains the numerically stable scalar/FMA
/// implementation used by formulas and Rust callers. Python's owned-array
/// fast path can use this equivalent recurrence for the two MACD EMAs and
/// the signal EMA, avoiding three long scalar dependency chains.
pub fn macd_fast_into(
    input: &[f64],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    macd_line: &mut [f64],
    signal: &mut [f64],
    histogram: &mut [f64],
) -> Result<()> {
    if fast_period == 0 || slow_period == 0 || signal_period == 0 || fast_period >= slow_period {
        return Err(TaError::InvalidParameter {
            name: "periods".to_string(),
            constraint: "fast/slow/signal periods must be greater than 0 and fast < slow"
                .to_string(),
        });
    }
    if let Some(idx) = input.iter().position(|v| !v.is_finite()) {
        return Err(TaError::InvalidParameter {
            name: "input".to_string(),
            constraint: format!("non-finite value at index {idx}"),
        });
    }
    validate_input(input.len(), slow_period)?;
    if macd_line.len() != input.len()
        || signal.len() != input.len()
        || histogram.len() != input.len()
    {
        return Err(TaError::InvalidParameter {
            name: "output slices".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }

    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    if crate::math::simd_kernels::avx2_fma_available()
        && input.len() >= slow_period.saturating_add(8)
    {
        // SAFETY: the runtime dispatch above checks the target features, and
        // all slices have equal lengths and satisfy the period preconditions.
        unsafe {
            return macd_fast_avx2_impl(
                input,
                fast_period,
                slow_period,
                signal_period,
                macd_line,
                signal,
                histogram,
            );
        }
    }

    macd_into(
        input,
        fast_period,
        slow_period,
        signal_period,
        macd_line,
        signal,
        histogram,
    )
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2,fma")]
pub(crate) unsafe fn macd_fast_avx2_impl(
    input: &[f64],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    macd_line: &mut [f64],
    signal: &mut [f64],
    histogram: &mut [f64],
) -> Result<()> {
    let len = input.len();
    macd_line.fill(f64::NAN);
    signal.fill(f64::NAN);
    histogram.fill(f64::NAN);

    let fast_k = 2.0 / (fast_period as f64 + 1.0);
    let slow_k = 2.0 / (slow_period as f64 + 1.0);
    let signal_k = 2.0 / (signal_period as f64 + 1.0);

    let offset = slow_period - fast_period;
    let mut slow_sum = 0.0;
    for &value in &input[..offset] {
        slow_sum += value;
    }
    let mut fast_sum = 0.0;
    for &value in &input[offset..slow_period] {
        fast_sum += value;
        slow_sum += value;
    }

    let macd_start = slow_period - 1;
    let mut previous_fast = fast_sum / fast_period as f64;
    let mut previous_slow = slow_sum / slow_period as f64;
    macd_line[macd_start] = previous_fast - previous_slow;

    let mut index = slow_period;
    while index + 4 <= len {
        let fast_values = unsafe {
            crate::math::simd_kernels::ema_block4_avx2(
                input.as_ptr().add(index),
                previous_fast,
                fast_k,
            )
        };
        let slow_values = unsafe {
            crate::math::simd_kernels::ema_block4_avx2(
                input.as_ptr().add(index),
                previous_slow,
                slow_k,
            )
        };
        for lane in 0..4 {
            macd_line[index + lane] = fast_values[lane] - slow_values[lane];
        }
        previous_fast = fast_values[3];
        previous_slow = slow_values[3];
        index += 4;
    }
    while index < len {
        let value = input[index];
        previous_fast = (value - previous_fast).mul_add(fast_k, previous_fast);
        previous_slow = (value - previous_slow).mul_add(slow_k, previous_slow);
        macd_line[index] = previous_fast - previous_slow;
        index += 1;
    }

    let signal_start = macd_start + signal_period - 1;
    if signal_start < len {
        let mut signal_sum = 0.0;
        for &value in &macd_line[macd_start..=signal_start] {
            signal_sum += value;
        }
        let mut previous_signal = signal_sum / signal_period as f64;
        signal[signal_start] = previous_signal;
        histogram[signal_start] = macd_line[signal_start] - previous_signal;

        let mut signal_index = signal_start + 1;
        while signal_index + 4 <= len {
            let values = unsafe {
                crate::math::simd_kernels::ema_block4_avx2(
                    macd_line.as_ptr().add(signal_index),
                    previous_signal,
                    signal_k,
                )
            };
            for lane in 0..4 {
                signal[signal_index + lane] = values[lane];
                histogram[signal_index + lane] = macd_line[signal_index + lane] - values[lane];
            }
            previous_signal = values[3];
            signal_index += 4;
        }
        while signal_index < len {
            previous_signal =
                (macd_line[signal_index] - previous_signal).mul_add(signal_k, previous_signal);
            signal[signal_index] = previous_signal;
            histogram[signal_index] = macd_line[signal_index] - previous_signal;
            signal_index += 1;
        }
    }

    macd_line[macd_start..signal_start.min(len)].fill(f64::NAN);
    Ok(())
}

/// MACD line-only zero-copy variant used by scalar formula projections.
///
/// Formula `MACD` returns the DIF/MACD line, not the signal or histogram. This
/// kernel therefore avoids allocating two unused companion series while
/// preserving the same TA-Lib seed and public lookback as [`macd`].
pub fn macd_line_into(
    input: &[f64],
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    output: &mut [f64],
) -> Result<()> {
    if fast_period == 0 || slow_period == 0 || signal_period == 0 || fast_period >= slow_period {
        return Err(TaError::InvalidParameter {
            name: "periods".to_string(),
            constraint: "fast/slow/signal periods must be greater than 0 and fast < slow"
                .to_string(),
        });
    }
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    if let Some(idx) = input.iter().position(|v| !v.is_finite()) {
        return Err(TaError::InvalidParameter {
            name: "input".to_string(),
            constraint: format!("non-finite value at index {idx}"),
        });
    }
    let lookback = slow_period
        .checked_add(signal_period)
        .and_then(|value| value.checked_sub(2))
        .ok_or_else(|| TaError::InvalidParameter {
            name: "periods".to_string(),
            constraint: "periods must not overflow".to_string(),
        })?;
    validate_input(input.len(), lookback + 1)?;

    output.fill(f64::NAN);
    let fast_k = 2.0 / (fast_period as f64 + 1.0);
    let slow_k = 2.0 / (slow_period as f64 + 1.0);
    let offset = slow_period - fast_period;
    let mut slow_sum = 0.0;
    for i in 0..offset {
        slow_sum += input[i];
    }
    let mut fast_sum = 0.0;
    for i in offset..slow_period {
        fast_sum += input[i];
        slow_sum += input[i];
    }

    let macd_start = slow_period - 1;
    let mut prev_slow = slow_sum / slow_period as f64;
    let mut prev_fast = fast_sum / fast_period as f64;
    output[macd_start] = prev_fast - prev_slow;
    for i in slow_period..input.len() {
        let value = input[i];
        prev_fast = (value - prev_fast) * fast_k + prev_fast;
        prev_slow = (value - prev_slow) * slow_k + prev_slow;
        output[i] = prev_fast - prev_slow;
    }

    // The MACD values used to seed the signal line are internal state and are
    // not part of the public output contract.
    let signal_start = macd_start + signal_period - 1;
    output[macd_start..signal_start].fill(f64::NAN);
    Ok(())
}
