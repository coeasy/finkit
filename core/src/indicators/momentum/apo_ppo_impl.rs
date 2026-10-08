//! Absolute and percentage price oscillator.

use super::prelude::*;

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

/// Absolute Price Oscillator with an explicit moving-average type.
///
/// This is the TA-Lib profile form of [`apo`].  The convenience function
/// keeps its SMA implementation and this selector preserves that path for
/// `MaType::Sma`, while allowing the public compatibility dispatcher to honor
/// TA-Lib's `matype` parameter for the other supported selectors.
pub fn apo_with_ma_type(
    input: &[f64],
    fast_period: usize,
    slow_period: usize,
    ma_type: MaType,
) -> Result<Array1<f64>> {
    if ma_type == MaType::Sma {
        return apo(input, fast_period, slow_period);
    }
    if fast_period == 0 || slow_period == 0 || fast_period >= slow_period {
        return Err(TaError::InvalidParameter {
            name: "fast_period, slow_period".to_string(),
            constraint: "must be positive and fast_period < slow_period".to_string(),
        });
    }
    validate_input(input.len(), slow_period)?;

    let fast = crate::indicators::overlap::ma(input, fast_period, ma_type)?;
    let slow = crate::indicators::overlap::ma(input, slow_period, ma_type)?;
    let mut output = init_output(input.len());
    for i in slow_period - 1..input.len() {
        if fast[i].is_finite() && slow[i].is_finite() {
            output[i] = fast[i] - slow[i];
        }
    }
    Ok(output)
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
    ppo_with_ma_type(input, fast_period, slow_period, MaType::Ema)
}

/// Percentage Price Oscillator with an explicit TA-Lib moving-average type.
///
/// TA-Lib's PPO defaults to `matype=0` (SMA), while the formula-oriented
/// [`ppo`] convenience API historically used EMA semantics.  Keeping the
/// selector explicit lets the compatibility operation match TA-Lib exactly
/// and preserves the useful EMA shorthand for formula users.
pub fn ppo_with_ma_type(
    input: &[f64],
    fast_period: usize,
    slow_period: usize,
    ma_type: MaType,
) -> Result<Array1<f64>> {
    if fast_period == 0 || slow_period == 0 || fast_period >= slow_period {
        return Err(TaError::InvalidParameter {
            name: "fast_period, slow_period".to_string(),
            constraint: "must be positive and fast_period < slow_period".to_string(),
        });
    }
    validate_input(input.len(), slow_period)?;

    // SMA and EMA are the two selectors that actually get called: `ppo` is the
    // formula shorthand (`PPO:=(EMA(CLOSE,SHORT)-EMA(CLOSE,LONG))/EMA(CLOSE,LONG)*100`)
    // and TA-Lib's default profile is `matype=0`/SMA. Both are pure recurrences,
    // so they fuse into one pass that never materializes either moving average.
    // That removes two full-length allocations and two full-length store passes
    // from a kernel whose own arithmetic is one division per element — which is
    // what put PPO at 0.79x while the structurally identical APO sat at 2.59x.
    match ma_type {
        MaType::Sma => ppo_fused_sma(input, fast_period, slow_period),
        MaType::Ema => ppo_fused_ema(input, fast_period, slow_period),
        other => {
            let fast_ma = crate::indicators::overlap::ma(input, fast_period, other)?;
            let slow_ma = crate::indicators::overlap::ma(input, slow_period, other)?;
            let mut output = init_output(input.len());
            for i in slow_period - 1..input.len() {
                if fast_ma[i].is_finite() && slow_ma[i].is_finite() {
                    output[i] = ppo_ratio(fast_ma[i], slow_ma[i]);
                }
            }
            Ok(output)
        }
    }
}

/// `(fast - slow) / slow * 100`, with the near-zero denominator collapsed to `0`.
///
/// Shared by every PPO path so the fused kernels and the generic `ma()` path
/// cannot drift apart on the guard.
#[inline]
pub(crate) fn ppo_ratio(fast: f64, slow: f64) -> f64 {
    if !crate::utils::is_zero(slow) {
        (fast - slow) / slow * 100.0
    } else {
        0.0
    }
}

/// Fused PPO over two rolling sums (`MaType::Sma`).
///
/// Bit-identical to `sma(fast) - sma(slow)` fed through [`ppo_ratio`]: the
/// seed uses `simd_horizontal_sum` and the sliding order is
/// `sum += input[i] - input[i - period]`, exactly as `sma_inner` does.
// The warm-up prefix is filled right below and the loop writes every slot from
// `first` on, so no slot is ever read uninitialized. Kept as `expect` (not
// `allow`): CI denies unfulfilled lint expectations.
#[expect(clippy::uninit_vec)]
pub(crate) fn ppo_fused_sma(
    input: &[f64],
    fast_period: usize,
    slow_period: usize,
) -> Result<Array1<f64>> {
    let len = input.len();
    // A leading run is an upstream rolling indicator's warm-up prefix, not bad
    // data: slide both windows past it the way `sma` does. Anything non-finite
    // after the series starts is a hard error there and stays one here.
    let start = crate::math::leading_warmup(input);

    let mut output = Vec::<f64>::with_capacity(len);
    // SAFETY: the fill below covers `[..first]` and the loops cover `[first..]`.
    unsafe {
        output.set_len(len);
    }
    let first = start + slow_period - 1;
    output[..first.min(len)].fill(f64::NAN);
    if first >= len {
        return Ok(Array1::from(output));
    }
    for (offset, &value) in input[start..start + slow_period].iter().enumerate() {
        if !value.is_finite() {
            return Err(non_finite_at(start + offset));
        }
    }

    let fast_inv = 1.0 / fast_period as f64;
    let slow_inv = 1.0 / slow_period as f64;
    let mut fast_sum = simd_horizontal_sum(&input[start..start + fast_period]);
    let mut slow_sum = simd_horizontal_sum(&input[start..start + slow_period]);

    // Advance the fast window to the slow window's first valid slot; same
    // sliding order as `sma_inner`, so the accumulation is bit-identical.
    for i in start + fast_period..start + slow_period {
        fast_sum += input[i] - input[i - fast_period];
    }
    let slow_val = slow_sum * slow_inv;
    output[first] = ppo_ratio(fast_sum * fast_inv, slow_val);

    for i in start + slow_period..len {
        let value = input[i];
        if !value.is_finite() {
            return Err(non_finite_at(i));
        }
        fast_sum += value - input[i - fast_period];
        slow_sum += value - input[i - slow_period];
        output[i] = ppo_ratio(fast_sum * fast_inv, slow_sum * slow_inv);
    }

    Ok(Array1::from(output))
}

/// Fused PPO over two EMA recurrences (`MaType::Ema`).
///
/// Bit-identical to `ema(fast)`/`ema(slow)` fed through [`ppo_ratio`]: SMA seed
/// via `simd_horizontal_sum / period`, then `prev = (val - prev) * k + prev`
/// with `k = 2 / (period + 1)` — the recurrence `ema_inner` runs.
// See [`ppo_fused_sma`].
#[expect(clippy::uninit_vec)]
pub(crate) fn ppo_fused_ema(
    input: &[f64],
    fast_period: usize,
    slow_period: usize,
) -> Result<Array1<f64>> {
    let len = input.len();
    let start = crate::math::leading_warmup(input);

    let mut output = Vec::<f64>::with_capacity(len);
    // SAFETY: the fill below covers `[..first]` and the loops cover `[first..]`.
    unsafe {
        output.set_len(len);
    }
    let first = start + slow_period - 1;
    output[..first.min(len)].fill(f64::NAN);
    if first >= len {
        return Ok(Array1::from(output));
    }
    for (offset, &value) in input[start..start + slow_period].iter().enumerate() {
        if !value.is_finite() {
            return Err(non_finite_at(start + offset));
        }
    }

    let k_fast = smoothing_factor(fast_period);
    let k_slow = smoothing_factor(slow_period);
    // `ema_inner` seeds with `sum / period as f64`, not `sum * (1/period)`;
    // keep the division so the seed is bit-identical.
    let mut fast_prev =
        simd_horizontal_sum(&input[start..start + fast_period]) / fast_period as f64;
    let mut slow_prev =
        simd_horizontal_sum(&input[start..start + slow_period]) / slow_period as f64;

    // `f64::mul_add` outside an FMA-target-feature function lowers to a libm
    // `fma` call — measured at ~1.1 ns apiece here, i.e. ~22 µs over a 10k
    // series for the two recurrences below, which is more than the whole
    // fusion saves. `ema_inner` avoids that by running its recurrence inside
    // `#[target_feature(enable = "avx2,fma")]`; the fused kernel has to do the
    // same. Both forms are correctly-rounded FMA, so the bits match either way.
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if std::arch::is_x86_feature_detected!("fma") {
            // SAFETY: `fma` was just detected on this CPU.
            if let Some(index) = unsafe {
                ppo_ema_fma_tail(
                    input,
                    start,
                    fast_period,
                    slow_period,
                    &mut fast_prev,
                    &mut slow_prev,
                    &mut output,
                )
            } {
                return Err(non_finite_at(index));
            }
            return Ok(Array1::from(output));
        }
    }

    for i in start + fast_period..start + slow_period {
        fast_prev = (input[i] - fast_prev) * k_fast + fast_prev;
    }
    output[first] = ppo_ratio(fast_prev, slow_prev);

    for i in start + slow_period..len {
        let value = input[i];
        if !value.is_finite() {
            return Err(non_finite_at(i));
        }
        fast_prev = (value - fast_prev) * k_fast + fast_prev;
        slow_prev = (value - slow_prev) * k_slow + slow_prev;
        output[i] = ppo_ratio(fast_prev, slow_prev);
    }

    Ok(Array1::from(output))
}

/// Fused PPO EMA recurrences, compiled with hardware FMA.
///
/// Returns `Some(index)` at the first non-finite input value; the caller turns
/// that into the same error `ema` would produce. The caller guarantees
/// `start + slow_period <= len` and that `output` has `input.len()` slots, with
/// everything from `start + slow_period - 1` on writable.
#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "fma")]
pub(crate) unsafe fn ppo_ema_fma_tail(
    input: &[f64],
    start: usize,
    fast_period: usize,
    slow_period: usize,
    fast_prev: &mut f64,
    slow_prev: &mut f64,
    output: &mut [f64],
) -> Option<usize> {
    let len = input.len();
    let k_fast = smoothing_factor(fast_period);
    let k_slow = smoothing_factor(slow_period);
    let mut fast = *fast_prev;
    let mut slow = *slow_prev;

    // Bring the fast EMA up to the slow EMA's first valid slot.
    for i in start + fast_period..start + slow_period {
        let value = input[i];
        if !value.is_finite() {
            return Some(i);
        }
        fast = (value - fast).mul_add(k_fast, fast);
    }
    output[start + slow_period - 1] = ppo_ratio(fast, slow);

    for i in start + slow_period..len {
        let value = input[i];
        if !value.is_finite() {
            return Some(i);
        }
        fast = (value - fast).mul_add(k_fast, fast);
        slow = (value - slow).mul_add(k_slow, slow);
        output[i] = ppo_ratio(fast, slow);
    }

    *fast_prev = fast;
    *slow_prev = slow;
    None
}
