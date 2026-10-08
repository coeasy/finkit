//! Directional movement: ADX/ADXR/+DI/-DI/DX/+DM/-DM.

use super::prelude::*;

/// Average Directional Index from pre-computed `+DI` / `-DI` series.
///
/// This is the five-argument `ADX(srcHigh, srcLow, srcClose, diLength,
/// adxSmoothing)` contract lifted out of the formula engine so the
/// tree-walking executor and the compiled-plan kernel share one
/// implementation. `diLength` and `adxSmoothing` stay distinct: DX is smoothed
/// with Wilder/RMA whose seed is the arithmetic mean of the first `adx_n`
/// valid DX values.
///
/// `output` is fully written: NaN where the contract has no value yet.
///
/// # Errors
///
/// Returns `TaError::InvalidParameter` if the `+DI` / `-DI` inputs are shorter
/// than `output`, or if `adx_n` is zero.
#[allow(clippy::cast_precision_loss)] // smoothing lengths are far below 2^53
pub fn adx_from_di_into(
    plus_di: &[f64],
    minus_di: &[f64],
    adx_n: usize,
    output: &mut [f64],
) -> Result<()> {
    let len = output.len();
    if plus_di.len() < len || minus_di.len() < len {
        return Err(TaError::InvalidParameter {
            name: "plus_di, minus_di".to_string(),
            constraint: "must have at least the output length".to_string(),
        });
    }
    if adx_n == 0 {
        return Err(TaError::InvalidParameter {
            name: "adx_n".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    output.fill(f64::NAN);

    let mut dx = vec![f64::NAN; len];
    for i in 0..len {
        let plus = plus_di[i];
        let minus = minus_di[i];
        if plus.is_finite() && minus.is_finite() {
            let sum = plus + minus;
            dx[i] = if !crate::utils::is_zero(sum) {
                (plus - minus).abs() / sum * 100.0
            } else {
                0.0
            };
        }
    }

    let Some(first_valid) = dx.iter().position(|value| value.is_finite()) else {
        return Ok(());
    };
    let Some(seed_end) = first_valid.checked_add(adx_n - 1) else {
        return Ok(());
    };
    if seed_end >= len || (first_valid..=seed_end).any(|i| !dx[i].is_finite()) {
        return Ok(());
    }

    let seed = (first_valid..=seed_end).map(|i| dx[i]).sum::<f64>() / adx_n as f64;
    output[seed_end] = seed;
    let mut previous = seed;
    for i in (seed_end + 1)..len {
        let value = dx[i];
        if value.is_finite() {
            previous = (value + (adx_n as f64 - 1.0) * previous) / adx_n as f64;
            output[i] = previous;
        }
    }
    Ok(())
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
    // `ADX` needs the smoothed DX chain and nothing else. `adx_into` is the
    // raw-pointer recurrence that writes the ADX column straight into the
    // caller's buffer: one allocation, one write stream, no bounds checks. It
    // replaces the `compute_adx_only` path, which ran the same arithmetic
    // through indexed access into a fresh `Vec` — the fix that took `adxr` from
    // 0.91x to 1.09x (see `adxr`). `dx()` remains the one public entry point
    // that genuinely wants the `±DI` pair, and it still calls the family.
    let mut output = Array1::from(crate::utils::uninit_output(high.len()));
    adx_into(
        high,
        low,
        close,
        period,
        output.as_slice_mut().expect("owned Array1 is contiguous"),
    )?;
    Ok(output)
}

/// The `+DI` / `-DI` pair for one Wilder-smoothed DM/TR state.
///
/// The single source of the `tr ≈ 0` guard: the callers used to carry separate
/// hand-copied versions of it, which is exactly the kind of duplication that
/// lets one copy drift from the other.
#[inline(always)]
pub(crate) fn di_pair_from_state(plus_dm: f64, minus_dm: f64, tr: f64) -> (f64, f64) {
    if crate::utils::is_zero(tr) {
        return (0.0, 0.0);
    }
    (plus_dm / tr * 100.0, minus_dm / tr * 100.0)
}

/// The `+DI` / `-DI` pair, computed in one pass over the bars.
///
/// Produced by [`compute_di_pair`] and consumed by [`dx`], which needs both
/// columns to rebuild `DX` from their ratio. Nothing else wants a pair: `adx`
/// and `adxr` want the DX chain alone and go through [`adx_into`], while
/// `plus_di` and `minus_di` each want a single column and go through
/// [`compute_single_di`].
///
/// This struct used to carry a third `adx` column as well, and its producer
/// (then named `compute_adx_family`) wrote it on every call. Once `adx` stopped
/// routing through the family, that column had no reader left — it was a
/// full-length allocation, a full-length write stream, and the whole ADX
/// smoothing recurrence, all discarded on return by the one remaining caller.
pub(crate) struct DiPair {
    plus_di: Vec<f64>,
    minus_di: Vec<f64>,
}

/// Single-pass computation of `+DM`, `-DM`, `TR`, `+DI` and `-DI`.
///
/// The DI family shares the same True Range and Directional Movement values, so
/// computing both columns in one scan avoids a redundant TR/DM pass that
/// [`compute_single_di`] pays once per column.
pub(crate) fn compute_di_pair(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
) -> Result<DiPair> {
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

    // Only the warm-up prefix needs seeding. `validate_input` above already
    // guaranteed `len >= 2*period`, so the `period..dx_start` loop and the
    // `dx_start..len` loop between them write every `±DI` slot from `period`
    // on. Two full-length NaN fills used to precede them — 160 KB of stores on
    // a 10k-bar series that the loops overwrote immediately, and 16 MB on a
    // million-bar one. A third fill covered the `adx` column, which this
    // function no longer produces.
    let dx_start = 2 * period;
    // Every slot from `period` on is written by the two loops below; this fill
    // is the warm-up prefix. See `utils::uninit_output`.
    let mut plus_di_out = crate::utils::uninit_output(len);
    let mut minus_di_out = crate::utils::uninit_output(len);
    plus_di_out[..period].fill(f64::NAN);
    minus_di_out[..period].fill(f64::NAN);

    // TA-Lib Phase 2: Wilder 平滑 DM/TR 迭代 `period` 次，把状态坐实，
    // 之后交给稳态循环。每次：先 `prevDM -= prevDM/period; prevDM += newDM`，
    // 再取 DI 两列。
    for i in period..dx_start.min(len) {
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

        let (pdi, mdi) = di_pair_from_state(smooth_plus_dm, smooth_minus_dm, smooth_tr);
        plus_di_out[i] = pdi;
        minus_di_out[i] = mdi;
    }

    for i in dx_start..len {
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

        let (pdi, mdi) = di_pair_from_state(smooth_plus_dm, smooth_minus_dm, smooth_tr);
        plus_di_out[i] = pdi;
        minus_di_out[i] = mdi;
    }

    Ok(DiPair {
        plus_di: plus_di_out,
        minus_di: minus_di_out,
    })
}
/// Compute one directional indicator without ADX smoothing.
///
/// `PLUS` is a const parameter so the hot loop contains no per-row direction
/// branch.  The public APIs request one projection at a time, therefore keeping
/// the other DM state and output vector alive is pure overhead.
pub(crate) fn compute_single_di<const PLUS: bool>(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
) -> Result<Vec<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period * 2)?;

    let len = close.len();
    let p = period as f64;

    let mut smooth_dm = 0.0f64;
    let mut smooth_tr = 0.0f64;

    // TA-Lib 兼容：累积 period-1 个 DM/TR 值，随后 Wilder 平滑处理第 period 个 bar。
    if period > 1 {
        let mut unused_dm = 0.0f64;
        #[cfg(feature = "std")]
        {
            if PLUS {
                crate::math::simd_kernels::adx_warmup_into(
                    high,
                    low,
                    close,
                    period - 1,
                    &mut smooth_dm,
                    &mut unused_dm,
                    &mut smooth_tr,
                );
            } else {
                crate::math::simd_kernels::adx_warmup_into(
                    high,
                    low,
                    close,
                    period - 1,
                    &mut unused_dm,
                    &mut smooth_dm,
                    &mut smooth_tr,
                );
            }
        }
        #[cfg(not(feature = "std"))]
        {
            for i in 1..period {
                let up_move = high[i] - high[i - 1];
                let down_move = low[i - 1] - low[i];
                smooth_tr += crate::utils::true_range(high[i], low[i], close[i - 1]);
                if PLUS {
                    if up_move > down_move && up_move > 0.0 {
                        smooth_dm += up_move;
                    }
                } else if down_move > up_move && down_move > 0.0 {
                    smooth_dm += down_move;
                }
            }
        }
    }

    let mut output = vec![f64::NAN; len];

    #[inline(always)]
    fn calc_di(s_dm: f64, s_tr: f64) -> f64 {
        if !crate::utils::is_zero(s_tr) {
            s_dm / s_tr * 100.0
        } else {
            0.0
        }
    }

    // TA-Lib: 第 period 个 bar 先 Wilder 平滑再计算首个 DI
    // 之后继续 Wilder 平滑（无 ADX 计算）
    for i in period..len {
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];
        let tr = crate::utils::true_range(high[i], low[i], close[i - 1]);
        let dm = if PLUS {
            if up_move > down_move && up_move > 0.0 {
                up_move
            } else {
                0.0
            }
        } else if down_move > up_move && down_move > 0.0 {
            down_move
        } else {
            0.0
        };
        smooth_dm = smooth_dm - smooth_dm / p + dm;
        smooth_tr = smooth_tr - smooth_tr / p + tr;

        output[i] = calc_di(smooth_dm, smooth_tr);
    }

    Ok(output)
}

/// Caller-owned directional-indicator kernel for the Python boundary.
///
/// This keeps the standalone DI recurrence allocation-free and uses a raw
/// pointer walk in the long tail. The recurrence and operation order match
/// [`compute_single_di`] so the public fast path remains bit-stable.
#[inline]
pub(crate) fn directional_di_into<const PLUS: bool, const INITIALIZE: bool>(
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
    if output.len() != high.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as high".to_string(),
        });
    }
    validate_input(high.len(), period * 2)?;

    if INITIALIZE {
        output.fill(f64::NAN);
    } else {
        output[..period].fill(f64::NAN);
    }
    let len = close.len();
    let p = period as f64;
    let inv_period = 1.0 / p;
    let mut smooth_dm = 0.0f64;
    let mut smooth_tr = 0.0f64;

    for i in 1..period {
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];
        smooth_tr += crate::utils::true_range(high[i], low[i], close[i - 1]);
        if PLUS {
            if up_move > down_move && up_move > 0.0 {
                smooth_dm += up_move;
            }
        } else if down_move > up_move && down_move > 0.0 {
            smooth_dm += down_move;
        }
    }

    unsafe {
        let high_ptr = high.as_ptr();
        let low_ptr = low.as_ptr();
        let close_ptr = close.as_ptr();
        let output_ptr = output.as_mut_ptr();
        for i in period..len {
            let h = *high_ptr.add(i);
            let l = *low_ptr.add(i);
            let previous_h = *high_ptr.add(i - 1);
            let previous_l = *low_ptr.add(i - 1);
            let previous_c = *close_ptr.add(i - 1);
            let up_move = h - previous_h;
            let down_move = previous_l - l;
            let mut tr = h - l;
            let high_gap = (h - previous_c).abs();
            if high_gap > tr {
                tr = high_gap;
            }
            let low_gap = (l - previous_c).abs();
            if low_gap > tr {
                tr = low_gap;
            }
            let dm = if PLUS {
                if up_move > down_move && up_move > 0.0 {
                    up_move
                } else {
                    0.0
                }
            } else if down_move > up_move && down_move > 0.0 {
                down_move
            } else {
                0.0
            };
            smooth_dm = smooth_dm - smooth_dm * inv_period + dm;
            smooth_tr = smooth_tr - smooth_tr * inv_period + tr;
            *output_ptr.add(i) = if smooth_tr > 0.0 {
                100.0 * (smooth_dm / smooth_tr)
            } else {
                0.0
            };
        }
    }
    Ok(())
}

/// Caller-owned PLUS_DI kernel.
pub fn plus_di_fast_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<()> {
    directional_di_into::<true, false>(high, low, close, period, output)
}

/// Caller-owned MINUS_DI kernel.
pub fn minus_di_fast_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<()> {
    directional_di_into::<false, false>(high, low, close, period, output)
}

pub(crate) fn di(
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

    if !crate::utils::is_zero(smooth_tr) {
        di_values[period - 1] = smooth_dm / smooth_tr * 100.0;
    }

    for i in period..len {
        smooth_dm = smooth_dm - smooth_dm * inv_p + dm[i];
        smooth_tr = smooth_tr - smooth_tr * inv_p + tr_values[i];
        if !crate::utils::is_zero(smooth_tr) {
            di_values[i] = smooth_dm / smooth_tr * 100.0;
        }
    }

    Ok(Array1::from_vec(di_values))
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
        let pair = compute_di_pair(high, low, close, period)?;
        let len = close.len();
        let mut dx_vals = init_output(len);
        for i in 0..len {
            let pdi = pair.plus_di[i];
            let mdi = pair.minus_di[i];
            if !pdi.is_nan() && !mdi.is_nan() {
                let sum = pdi + mdi;
                if !crate::utils::is_zero(sum) {
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
            if !crate::utils::is_zero(sum) {
                dx_vals[i] = (plus_di_vals[i] - minus_di_vals[i]).abs() / sum * 100.0;
            }
        }
    }

    Ok(dx_vals)
}

/// Caller-owned DX kernel that avoids materializing the complete ADX family
/// when only DX is requested at the Python boundary.
pub fn dx_into(
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
    if output.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as close".to_string(),
        });
    }
    validate_input(close.len(), period * 2)?;
    output.fill(f64::NAN);

    let p = period as f64;
    let mut smooth_plus_dm = 0.0;
    let mut smooth_minus_dm = 0.0;
    let mut smooth_tr = 0.0;
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

    for i in period..close.len() {
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
        if !crate::utils::is_zero(smooth_tr) {
            let pdi = smooth_plus_dm / smooth_tr * 100.0;
            let mdi = smooth_minus_dm / smooth_tr * 100.0;
            let sum = pdi + mdi;
            output[i] = if !crate::utils::is_zero(sum) {
                (pdi - mdi).abs() / sum * 100.0
            } else {
                0.0
            };
        } else {
            output[i] = 0.0;
        }
    }
    Ok(())
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
    // Compute only the requested projection; the companion +DI state is not
    // needed when MINUS_DI is called as a standalone indicator.
    Ok(Array1::from_vec(compute_single_di::<false>(
        high, low, close, period,
    )?))
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

/// TA-Lib-compatible smoothed Minus Directional Movement (MINUS_DM).
///
/// The two-argument [`minus_dm`] function remains the raw one-bar movement
/// primitive used by formula expressions.  TA-Lib's public MINUS_DM
/// operation takes a period and returns a Wilder-smoothed series; this
/// explicit variant keeps both semantics available without hiding the
/// difference in a binding-specific adapter.
pub fn minus_dm_with_period(high: &[f64], low: &[f64], period: usize) -> Result<Array1<f64>> {
    directional_movement_with_period::<false>(high, low, period)
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
    // Compute only the requested projection; the companion -DI state is not
    // needed when PLUS_DI is called as a standalone indicator.
    Ok(Array1::from_vec(compute_single_di::<true>(
        high, low, close, period,
    )?))
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

/// TA-Lib-compatible smoothed Plus Directional Movement (PLUS_DM).
pub fn plus_dm_with_period(high: &[f64], low: &[f64], period: usize) -> Result<Array1<f64>> {
    directional_movement_with_period::<true>(high, low, period)
}

pub(crate) fn directional_movement_with_period<const PLUS: bool>(
    high: &[f64],
    low: &[f64],
    period: usize,
) -> Result<Array1<f64>> {
    if period < 2 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period)?;

    let len = high.len();
    let mut output = init_output(len);
    let lookback = period - 1;
    let mut smoothed = 0.0;

    // TA-Lib seeds the Wilder accumulator with the period-1 movement values
    // at bars 1..=period-1 and publishes the first result at period-1.
    for i in 1..=lookback {
        smoothed += directional_movement_value::<PLUS>(high, low, i);
    }
    output[lookback] = smoothed;

    let p = period as f64;
    for i in period..len {
        let movement = directional_movement_value::<PLUS>(high, low, i);
        smoothed = smoothed - smoothed / p + movement;
        output[i] = smoothed;
    }

    Ok(output)
}

#[inline]
pub(crate) fn directional_movement_value<const PLUS: bool>(
    high: &[f64],
    low: &[f64],
    index: usize,
) -> f64 {
    let up_move = high[index] - high[index - 1];
    let down_move = low[index - 1] - low[index];
    if PLUS {
        if up_move > 0.0 && up_move > down_move {
            up_move
        } else {
            0.0
        }
    } else if down_move > 0.0 && down_move > up_move {
        down_move
    } else {
        0.0
    }
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
    // Single buffer end to end: `adxr_into` materializes ADX in place and
    // then walks backwards, so the previous public path's extra full-length
    // `vec![NAN; len]` allocation (on top of the temp Vec the old indexed
    // recurrence produced)
    // bought nothing.
    let mut output = Array1::from(crate::utils::uninit_output(high.len()));
    adxr_into(
        high,
        low,
        close,
        period,
        output.as_slice_mut().expect("owned Array1 is contiguous"),
    )?;
    Ok(output)
}

/// Write ADXR directly into a caller-owned buffer.
///
/// ADXR still needs the internal ADX history, but avoiding a second result
/// allocation and copy matters for the public NumPy hot path.
pub fn adxr_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<()> {
    // First materialize ADX in the caller-owned buffer, then walk backwards.
    // Descending order keeps the lower-index ADX history intact until it has
    // been consumed, so ADXR needs no second full-length scratch vector.
    adx_into(high, low, close, period, output)?;
    for i in (period..output.len()).rev() {
        let cur = output[i];
        let prev = output[i + 1 - period];
        if !cur.is_nan() && !prev.is_nan() {
            output[i] = (cur + prev) * 0.5;
        } else {
            output[i] = f64::NAN;
        }
    }
    Ok(())
}

/// ADX zero-copy variant: writes result into pre-allocated slice.
///
/// # Single canonical kernel (V5 Batch 1 ①)
///
/// This used to be a second, hand-copied ADX: its own Wilder smoothing, its
/// own `true_range_fast` (comparison-style, which *propagates* NaN) and its
/// own `1e-15` zero guards — while `math::kernels::volatility::AdxState` (the
/// TA-Lib-compat path) used `f64::max` true range (which *absorbs* NaN,
/// matching TA-Lib's `fmax`). On OHLC containing NaN the two gave different
/// answers — the exact "one policy, two implementations" drift this batch
/// removes. The public kernel now drives the canonical `AdxState` directly:
/// one kernel, batch == streaming == TA-Lib-compat path by construction.
///
/// Output layout is unchanged: the first value lands at index `2*period-1`
/// (`AdxState` seeds `period-1` DMI movements from index 1, emits its first
/// DX at index `period`, and needs `period` DX values before the first ADX —
/// exactly the public lookback `2*period-1`).
pub fn adx_into(
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
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "must be greater than 0".to_string(),
        });
    }
    validate_input(high.len(), period * 2)?;
    if output.len() != high.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }

    output.fill(f64::NAN);
    let mut state = crate::math::kernels::AdxState::new(period);
    for index in 0..close.len() {
        if let Some(family) = state.update(high[index], low[index], close[index]) {
            if let Some(adx) = family.adx {
                output[index] = adx;
            }
        }
    }
    Ok(())
}
