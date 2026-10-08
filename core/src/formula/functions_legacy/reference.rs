//! TDX sequence/reference operators (REF/CROSS/COUNT/HHV/SUM/...).

use super::prelude::*;

pub(crate) fn fn_hhv(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HHV", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "HHV")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_stat::rolling_max(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_llv(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LLV", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "LLV")?;

    let data_len = ctx.data_len;
    let values = input.as_slice().unwrap();
    match lib_stat::rolling_min(values, n) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}

pub(crate) fn fn_hhvbars(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("HHVBARS", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "HHVBARS")?;

    let data_len = ctx.data_len;
    let mut output = nan_vec(data_len);

    // Monotonic deque over bar indices: amortized O(1) per bar instead of
    // rescanning the window. Ties keep the earliest bar (`<=` pops the equal
    // tail), matching the per-window rescan this replaces. Missing bars are
    // never queued; a window with no finite bar reports nothing.
    let mut deque = ArgExtremeDeque::<true>::default();
    for i in 0..data_len {
        let window_start = (i + 1).saturating_sub(n);
        deque.offer(i, input[i]);
        deque.expire_before(window_start);
        if let Some(index) = deque.extreme_index() {
            output[i] = (i - index) as f64;
        }
    }

    Ok(output)
}

pub(crate) fn fn_llvbars(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LLVBARS", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "LLVBARS")?;

    let data_len = ctx.data_len;
    let mut output = nan_vec(data_len);

    // Same deque as `fn_hhvbars`, mirrored for the minimum; see
    // [`ArgExtremeDeque`] for the tie and missing-bar rules.
    let mut deque = ArgExtremeDeque::<false>::default();
    for i in 0..data_len {
        let window_start = (i + 1).saturating_sub(n);
        deque.offer(i, input[i]);
        deque.expire_before(window_start);
        if let Some(index) = deque.extreme_index() {
            output[i] = (i - index) as f64;
        }
    }

    Ok(output)
}

pub(crate) fn fn_ref(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("REF", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "REF")?;

    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);
    for i in 0..data_len {
        if i >= n {
            result[i] = input[i - n];
        } else {
            result[i] = f64::NAN;
        }
    }
    Ok(result)
}

pub(crate) fn fn_cross(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CROSS", args, 2)?;
    let a = &args[0];
    let b = &args[1];
    let len = a.len().min(b.len());
    let mut result = Array1::zeros(len);
    if len > 0 {
        result[0] = 0.0;
    }
    for i in 1..len {
        if a[i - 1] <= b[i - 1] && a[i] > b[i] {
            result[i] = 1.0;
        }
    }
    Ok(result)
}

pub(crate) fn fn_crossbelow(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CROSSBELOW", args, 2)?;
    let a = &args[0];
    let b = &args[1];
    let len = a.len().min(b.len());
    let mut result = Array1::zeros(len);
    if len > 0 {
        result[0] = 0.0;
    }
    for i in 1..len {
        if a[i - 1] >= b[i - 1] && a[i] < b[i] {
            result[i] = 1.0;
        }
    }
    Ok(result)
}

pub(crate) fn fn_longcross(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LONGCROSS", args, 3)?;
    let a = &args[0];
    let b = &args[1];
    let n = args[2][0] as usize;
    let len = a.len().min(b.len());
    let mut result = Array1::zeros(len);

    // "a stayed at or below b for the previous n bars" is a sliding all-false
    // test over the window `(i - n)..i`, so a counter of bars violating
    // `a[j] <= b[j]` — i.e. `a[j] > b[j]` — turns the O(len · n) rescan into
    // one add and one subtract per bar. `NaN` fails `>`, so a missing bar
    // never counts as a violation, matching the old inner loop.
    let mut violations = 0usize;
    for i in 1..len {
        // Bar `i - 1` enters the window for this bar.
        if a[i - 1] > b[i - 1] {
            violations += 1;
        }
        // Bars below `i - n` have left it again.
        if i > n && a[i - n - 1] > b[i - n - 1] {
            violations -= 1;
        }
        if a[i] > b[i] && i >= n && violations == 0 && a[i - 1] <= b[i - 1] {
            result[i] = 1.0;
        }
    }

    Ok(result)
}

pub(crate) fn fn_if(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("IF", args, 3)?;
    let cond = &args[0];
    let then_val = &args[1];
    let else_val = &args[2];

    // A single code path for every length. The previous short-series fallback
    // tested `c > 0.0` while the vectorised path tested `c != 0.0`, so
    // `IF(-1, a, b)` returned different results depending on series length.
    // The rule now lives in `formula::truth::is_true`, which `SimdOps::select`
    // also uses.
    Ok(SimdOps::simd_select_arrays(cond, then_val, else_val))
}

pub(crate) fn fn_count(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("COUNT", args, 2)?;
    let cond = &args[0];
    let n = extract_n(args, 1, "COUNT")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    // Sliding true-count over the trailing window: O(1) per bar. `> 0.0`
    // treats a missing bar as false, same as the per-window rescan.
    let mut true_count = 0usize;
    for i in 0..data_len {
        if cond[i] > 0.0 {
            true_count += 1;
        }
        if i >= n && cond[i - n] > 0.0 {
            true_count -= 1;
        }
        if i + 1 >= n {
            result[i] = true_count as f64;
        }
    }

    Ok(result)
}

pub(crate) fn fn_sum(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SUM", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "SUM")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    // `SUM` has no math-layer counterpart, so it carries the same warm-up rule
    // locally: seed the running total at the first finite value instead of
    // letting a leading `NaN` run (an upstream indicator's warm-up prefix)
    // absorb the accumulator for the whole series.
    let start = crate::math::leading_warmup(input.as_slice().unwrap_or_default());
    let mut running_sum = 0.0;
    for i in start..data_len {
        running_sum += input[i];
        if i >= start + n {
            running_sum -= input[i - n];
            result[i] = running_sum;
        } else if i == start + n - 1 {
            result[i] = running_sum;
        }
    }

    Ok(result)
}

pub(crate) fn fn_every(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("EVERY", args, 2)?;
    let cond = &args[0];
    let n = extract_n(args, 1, "EVERY")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    // Sliding count of bars that fail the predicate: `EVERY` is true when the
    // window holds no false bar. The predicate is `<= 0.0`, so a missing bar
    // does *not* fail it — counting failures (not successes) keeps that rule.
    let mut false_count = 0usize;
    for i in 0..data_len {
        if cond[i] <= 0.0 {
            false_count += 1;
        }
        if i >= n && cond[i - n] <= 0.0 {
            false_count -= 1;
        }
        if i + 1 >= n {
            result[i] = if false_count == 0 { 1.0 } else { 0.0 };
        }
    }

    Ok(result)
}

pub(crate) fn fn_exist(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("EXIST", args, 2)?;
    let cond = &args[0];
    let n = extract_n(args, 1, "EXIST")?;

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);

    // Sliding true-count: `EXIST` is true when at least one bar is true.
    let mut true_count = 0usize;
    for i in 0..data_len {
        if cond[i] > 0.0 {
            true_count += 1;
        }
        if i >= n && cond[i - n] > 0.0 {
            true_count -= 1;
        }
        if i + 1 >= n {
            result[i] = if true_count > 0 { 1.0 } else { 0.0 };
        }
    }

    Ok(result)
}

pub(crate) fn fn_filter(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("FILTER", args, 2)?;
    let cond = &args[0];
    let n = extract_n(args, 1, "FILTER")?;

    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);
    let mut last_signal: Option<usize> = None;

    for i in 0..data_len {
        if cond[i] > 0.0 {
            if let Some(last) = last_signal {
                if i - last >= n {
                    result[i] = 1.0;
                    last_signal = Some(i);
                }
            } else {
                result[i] = 1.0;
                last_signal = Some(i);
            }
        }
    }

    Ok(result)
}

pub(crate) fn fn_barslast(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BARSLAST", args, 1)?;
    let cond = &args[0];

    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);
    let mut last_true: Option<usize> = None;

    for i in 0..data_len {
        if cond[i] > 0.0 {
            result[i] = 0.0;
            last_true = Some(i);
        } else if let Some(last) = last_true {
            result[i] = (i - last) as f64;
        }
    }

    Ok(result)
}

pub(crate) fn fn_backset(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BACKSET", args, 2)?;
    let cond = &args[0];
    let n = extract_n(args, 1, "BACKSET")?;

    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    // `BACKSET` sets the *previous* `n - 1` bars too, so bar `j` is on iff some
    // trigger fires in `[j, j + n - 1]` — a forward-looking condition. Walking
    // the series backwards with a countdown answers it in one pass: the trigger
    // at `i` arms `n` bars including itself, and the counter decays as `i`
    // moves back. The old inner loop rewrote up to `n` slots per trigger,
    // O(len · n) with a write storm on every trigger bar.
    let mut remaining = 0usize;
    for i in (0..data_len).rev() {
        if cond[i] > 0.0 {
            remaining = n;
        }
        if remaining > 0 {
            result[i] = 1.0;
            remaining -= 1;
        }
    }

    Ok(result)
}

pub(crate) fn fn_between(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BETWEEN", args, 3)?;
    let x = &args[0];
    let a = &args[1];
    let b = &args[2];
    let len = x.len().min(a.len()).min(b.len());
    let mut result = Array1::zeros(len);
    for i in 0..len {
        let lo = a[i].min(b[i]);
        let hi = a[i].max(b[i]);
        result[i] = if x[i] >= lo && x[i] <= hi { 1.0 } else { 0.0 };
    }
    Ok(result)
}

pub(crate) fn fn_currbarscount(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let len = ctx.data_len;
    Ok(Array1::from_vec(
        (0..len).map(|i| (len - i) as f64).collect(),
    ))
}

pub(crate) fn fn_totalbarscount(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    Ok(Array1::from_elem(ctx.data_len, ctx.data_len as f64))
}

pub(crate) fn fn_barssince(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BARSSINCE", args, 1)?;
    let cond = &args[0];
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let mut last_true: Option<usize> = None;
    for i in 0..len {
        if cond[i] != 0.0 && !cond[i].is_nan() {
            last_true = Some(i);
        }
        if let Some(lt) = last_true {
            out[i] = (i - lt) as f64;
        }
    }
    Ok(out)
}

pub(crate) fn fn_barssincen(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BARSSINCEN", args, 2)?;
    let cond = &args[0];
    let n = extract_n(args, 1, "BARSSINCEN")?;
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let mut count = 0usize;
    let mut last_bar: Option<usize> = None;
    for i in 0..len {
        if cond[i] != 0.0 && !cond[i].is_nan() {
            count += 1;
            if count >= n {
                last_bar = Some(i);
            }
        }
        if let Some(lb) = last_bar {
            out[i] = (i - lb) as f64;
        }
    }
    Ok(out)
}

pub(crate) fn fn_barscount(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BARSCOUNT", args, 1)?;
    let data = &args[0];
    let len = ctx.data_len;
    let mut out = Array1::zeros(len);
    let mut valid_count = 0.0f64;
    for i in 0..len {
        if !data[i].is_nan() {
            valid_count += 1.0;
        }
        out[i] = valid_count;
    }
    Ok(out)
}

pub(crate) fn fn_barstatus(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let len = ctx.data_len;
    let mut out = Array1::from_elem(len, 0.0);
    if len > 0 {
        out[0] = 1.0; // first bar
        out[len - 1] = 2.0; // last bar
    }
    Ok(out)
}

pub(crate) fn fn_islastbar(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    let len = ctx.data_len;
    let mut out = Array1::zeros(len);
    if len > 0 {
        out[len - 1] = 1.0;
    }
    Ok(out)
}

pub(crate) fn fn_fromopen(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    Ok(datetime_component(ctx, |ts| {
        let (_, _, _, h, m, _) = ts_to_date_parts(ts);
        let minutes_since_930 = (h as i32 - 9) * 60 + m as i32 - 30;
        minutes_since_930.max(0) as f64
    }))
}

pub(crate) fn fn_sumbars(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("SUMBARS", args, 2)?;
    let input = &args[0];
    let target = &args[1];
    let len = ctx.data_len;
    let mut out = nan_vec(len);
    let values = input.as_slice().unwrap();

    // Fast path: with non-negative values the prefix sums are monotone, so the
    // "smallest window ending at `i` whose sum reaches the threshold" is a
    // binary search over the prefix table instead of a walk that restarts at
    // bar `i` every time — O(len²) when the threshold is rarely reached (the
    // usual `SUMBARS(VOL, CAPITAL)` shape). Negative or missing values break
    // the monotonicity (one more bar can lower the sum), so they keep the
    // rescan; a per-bar threshold is fine because each search is independent.
    let monotone =
        values.iter().all(|v| v.is_finite() && *v >= 0.0) && target.iter().all(|t| t.is_finite());
    if monotone {
        let mut prefix = Vec::with_capacity(len + 1);
        prefix.push(0.0f64);
        let mut acc = 0.0f64;
        for &value in values.iter() {
            acc += value;
            prefix.push(acc);
        }
        for i in 0..len {
            let need = prefix[i + 1] - target[i];
            // Largest `j <= i` with `prefix[j] <= need`.
            let first_above = prefix[..=i].partition_point(|&p| p <= need);
            out[i] = if first_above == 0 {
                // Even the empty prefix exceeds `need`: every window ending at
                // `i` falls short, which the rescan reported as `i + 1` bars.
                (i + 1) as f64
            } else {
                (i - (first_above - 1) + 1) as f64
            };
        }
        return Ok(out);
    }

    for i in 0..len {
        let threshold = target[i];
        let mut cumsum = 0.0;
        let mut bars = 0.0;
        for j in (0..=i).rev() {
            cumsum += values[j];
            bars += 1.0;
            if cumsum >= threshold {
                break;
            }
        }
        out[i] = bars;
    }
    Ok(out)
}

pub(crate) fn fn_valuewhen(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("VALUEWHEN", args, 2)?;
    let cond = &args[0];
    let x = &args[1];
    let data_len = ctx.data_len;
    let mut result = nan_vec(data_len);
    let mut last_value = f64::NAN;

    for i in 0..data_len {
        if cond[i] > 0.0 && !cond[i].is_nan() {
            last_value = x[i];
        }
        result[i] = last_value;
    }

    Ok(result)
}

pub(crate) fn fn_last(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("LAST", args, 3)?;
    let cond = &args[0];
    let a = args[1][0] as usize;
    let b = args[2][0] as usize;
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    if a < b {
        return Err(FormulaError::InvalidParameter(
            "LAST: A must be >= B".to_string(),
        ));
    }

    if a >= data_len {
        // Every bar is inside the cold zone (`i < a`), so nothing can be set.
        return Ok(result);
    }

    // "cond held for every bar in [i - a, i - b]" is a fixed-width sliding
    // all-true test, so a counter of violating bars turns the O(len · (a-b))
    // rescan into one add and one subtract per bar. `NaN` violates, exactly as
    // in the original loop.
    let violates = |j: usize| -> usize {
        if cond[j] <= 0.0 || cond[j].is_nan() {
            1
        } else {
            0
        }
    };
    let mut violations = 0usize;
    for j in 0..=(a - b) {
        violations += violates(j);
    }
    for i in a..data_len {
        if i > a {
            // Window moves from `[i-1-a, i-1-b]` to `[i-a, i-b]`.
            violations -= violates(i - 1 - a);
            violations += violates(i - b);
        }
        result[i] = if violations == 0 { 1.0 } else { 0.0 };
    }

    Ok(result)
}

pub(crate) fn fn_barslastcount(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("BARSLASTCOUNT", args, 1)?;
    let cond = &args[0];
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);

    for i in 0..data_len {
        if cond[i] > 0.0 && !cond[i].is_nan() {
            if i > 0 {
                result[i] = result[i - 1] + 1.0;
            } else {
                result[i] = 1.0;
            }
        }
    }

    Ok(result)
}

pub(crate) fn fn_drawnull(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    Ok(nan_vec(ctx.data_len))
}

pub(crate) fn fn_ceiling(
    _ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CEILING", args, 2)?;
    let input = &args[0];
    let precision = args[1][0];
    let len = input.len();
    let mut result = Array1::zeros(len);

    if precision.abs() < f64::EPSILON {
        for i in 0..len {
            result[i] = input[i].ceil();
        }
    } else {
        for i in 0..len {
            if input[i].is_nan() {
                result[i] = f64::NAN;
            } else {
                result[i] = (input[i] / precision).ceil() * precision;
            }
        }
    }

    Ok(result)
}

// === Cumulative / Sequence operations ===

/// CUMSUM / CUM: Cumulative sum
pub(crate) fn fn_cumsum(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CUMSUM", args, 1)?;
    let input = &args[0];
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);
    let mut sum = 0.0;
    for i in 0..data_len {
        if !input[i].is_nan() {
            sum += input[i];
        }
        result[i] = sum;
    }
    Ok(result)
}

/// CUMMAX: Cumulative maximum
pub(crate) fn fn_cummax(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CUMMAX", args, 1)?;
    let input = &args[0];
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);
    let mut max = f64::NEG_INFINITY;
    for i in 0..data_len {
        if !input[i].is_nan() && input[i] > max {
            max = input[i];
        }
        result[i] = if max == f64::NEG_INFINITY {
            f64::NAN
        } else {
            max
        };
    }
    Ok(result)
}

/// CUMMIN: Cumulative minimum
pub(crate) fn fn_cummin(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("CUMMIN", args, 1)?;
    let input = &args[0];
    let data_len = ctx.data_len;
    let mut result = Array1::zeros(data_len);
    let mut min = f64::INFINITY;
    for i in 0..data_len {
        if !input[i].is_nan() && input[i] < min {
            min = input[i];
        }
        result[i] = if min == f64::INFINITY { f64::NAN } else { min };
    }
    Ok(result)
}

// === Multi-period / cross-timeframe functions ===

/// PERIODTYPE(): Returns current period type (0=daily, 1=weekly, 2=monthly, 3=minute)
pub(crate) fn fn_periodtype(
    ctx: &FormulaContext,
    _args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    Ok(Array1::from_elem(ctx.data_len, ctx.period_type as f64))
}

/// REFDATE(X, DATE): Reference value of X at a specific date (bar index).
/// If DATE is an index, returns constant series of X[DATE].
pub(crate) fn fn_refdate(
    ctx: &FormulaContext,
    args: &[Array1<f64>],
) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("REFDATE", args, 2)?;
    let input = &args[0];
    let date_idx = args[1][0] as usize;
    let data_len = ctx.data_len;
    let val = if date_idx < data_len {
        input[date_idx]
    } else {
        f64::NAN
    };
    Ok(Array1::from_elem(data_len, val))
}
