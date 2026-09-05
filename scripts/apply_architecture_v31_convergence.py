#!/usr/bin/env python3
"""Deterministic Architecture v3.1 convergence patch.

This script exists only for the migration commit and is deleted by the one-shot
workflow after validation. It patches large legacy files in-place while keeping
all semantic changes explicit and anchor-checked.
"""

from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text(encoding="utf-8")
    count = text.count(old)
    if count == 0 and new in text:
        return
    if count != 1:
        raise SystemExit(f"{path}: expected one anchor, found {count}")
    p.write_text(text.replace(old, new, 1), encoding="utf-8")


def patch_statistics() -> None:
    p = Path("core/src/math/statistics.rs")
    text = p.read_text(encoding="utf-8")
    if "pub(crate) fn rolling_minmax_visit" in text:
        return
    anchor = "/// Find maximum value in a rolling window\n"
    if anchor not in text:
        raise SystemExit("statistics.rs: rolling extrema anchor missing")
    implementation = '''/// Visit fused rolling maximum/minimum values without materializing extrema arrays.
///
/// Architecture v3.1 consumers use this kernel to share one O(n) rolling-window
/// traversal across MIDPOINT, MIDPRICE, WILLR and other extrema-family consumers.
#[inline]
pub(crate) fn rolling_minmax_visit(
    high: &[f64],
    low: &[f64],
    window: usize,
    mut emit: impl FnMut(usize, f64, f64),
) {
    debug_assert_eq!(high.len(), low.len());
    debug_assert!(window > 0);

    let mut max_deque = std::collections::VecDeque::with_capacity(window + 1);
    let mut min_deque = std::collections::VecDeque::with_capacity(window + 1);

    for i in 0..high.len() {
        while let Some(&back) = max_deque.back() {
            if high[back] <= high[i] {
                max_deque.pop_back();
            } else {
                break;
            }
        }
        max_deque.push_back(i);

        while let Some(&back) = min_deque.back() {
            if low[back] >= low[i] {
                min_deque.pop_back();
            } else {
                break;
            }
        }
        min_deque.push_back(i);

        while max_deque.front().is_some_and(|front| *front + window <= i) {
            max_deque.pop_front();
        }
        while min_deque.front().is_some_and(|front| *front + window <= i) {
            min_deque.pop_front();
        }

        if i + 1 >= window {
            let highest = high[*max_deque.front().expect("rolling max is non-empty")];
            let lowest = low[*min_deque.front().expect("rolling min is non-empty")];
            emit(i, highest, lowest);
        }
    }
}

'''
    p.write_text(text.replace(anchor, implementation + anchor, 1), encoding="utf-8")


def patch_overlap() -> None:
    p = Path("core/src/indicators/overlap.rs")
    text = p.read_text(encoding="utf-8")
    text = text.replace(
        "use crate::math::statistics::{rolling_max, rolling_min};",
        "use crate::math::statistics::rolling_minmax_visit;",
        1,
    )

    old_midpoint = '''    let max = rolling_max(input, period)?;
    let min = rolling_min(input, period)?;

    let len = input.len();
    let mut output = init_output(len);

    for i in 0..len {
        if !max[i].is_nan() && !min[i].is_nan() {
            output[i] = (max[i] + min[i]) / 2.0;
        }
    }

    Ok(output)'''
    new_midpoint = '''    let mut output = init_output(input.len());
    rolling_minmax_visit(input, input, period, |i, highest, lowest| {
        output[i] = (highest + lowest) * 0.5;
    });
    Ok(output)'''
    if old_midpoint in text:
        text = text.replace(old_midpoint, new_midpoint, 1)
    elif "rolling_minmax_visit(input, input, period" not in text:
        raise SystemExit("overlap.rs: midpoint anchor missing")

    old_midprice = '''    let max = rolling_max(high, period)?;
    let min = rolling_min(low, period)?;

    let len = high.len();
    let mut output = init_output(len);

    for i in 0..len {
        if !max[i].is_nan() && !min[i].is_nan() {
            output[i] = (max[i] + min[i]) / 2.0;
        }
    }

    Ok(output)'''
    new_midprice = '''    let mut output = init_output(high.len());
    rolling_minmax_visit(high, low, period, |i, highest, lowest| {
        output[i] = (highest + lowest) * 0.5;
    });
    Ok(output)'''
    if old_midprice in text:
        text = text.replace(old_midprice, new_midprice, 1)
    elif "rolling_minmax_visit(high, low, period" not in text:
        raise SystemExit("overlap.rs: midprice anchor missing")

    start_token = "pub fn sar(high: &[f64], low: &[f64], acceleration: f64, maximum: f64) -> Result<SarResult> {"
    end_token = "\n}\n\n/// Parabolic SAR Extended"
    start = text.find(start_token)
    if start < 0:
        raise SystemExit("overlap.rs: SAR function anchor missing")
    end = text.find(end_token, start)
    if end < 0:
        raise SystemExit("overlap.rs: SAREXT boundary missing")
    current = text[start : end + 2]
    canonical = '''pub fn sar(high: &[f64], low: &[f64], acceleration: f64, maximum: f64) -> Result<SarResult> {
    let (sar, af) = crate::math::sar::sar_with_af(high, low, acceleration, maximum)?;
    Ok(SarResult {
        sar: Array1::from_vec(sar),
        af: Array1::from_vec(af),
    })
}'''
    if current != canonical:
        text = text[:start] + canonical + text[end + 2 :]

    test_start = text.find("    fn test_sar() {")
    test_end = text.find("    fn test_sarext() {", test_start)
    if test_start < 0 or test_end < 0:
        raise SystemExit("overlap.rs: SAR test boundary missing")
    test_body = text[test_start:test_end]
    old_assert = "        assert!(!result.sar[0].is_nan());\n"
    new_assert = (
        "        assert!(result.sar[0].is_nan());\n"
        "        assert!(!result.sar[1].is_nan());\n"
    )
    if old_assert in test_body:
        test_body = test_body.replace(old_assert, new_assert, 1)
        text = text[:test_start] + test_body + text[test_end:]
    elif new_assert not in test_body:
        raise SystemExit("overlap.rs: SAR warm-up assertion anchor missing")

    p.write_text(text, encoding="utf-8")


def patch_momentum() -> None:
    p = Path("core/src/indicators/momentum.rs")
    text = p.read_text(encoding="utf-8")
    import_anchor = "use crate::math::simd_ops;\n"
    if "use crate::math::statistics::rolling_minmax_visit;" not in text:
        if import_anchor not in text:
            raise SystemExit("momentum.rs: import anchor missing")
        text = text.replace(
            import_anchor,
            import_anchor + "use crate::math::statistics::rolling_minmax_visit;\n",
            1,
        )

    mask = '''
    // Public TA-Lib contract: MACD, signal and histogram share one lookback.
    // Earlier MACD values are internal signal-seed intermediates, not outputs.
    macd_line[macd_start..signal_start].fill(f64::NAN);
'''
    anchor = '''        for i in signal_start..len {
            hist[i] = macd_line[i] - signal[i];
        }
    }

    Ok(MacdResult {'''
    replacement = '''        for i in signal_start..len {
            hist[i] = macd_line[i] - signal[i];
        }
    }
''' + mask + '''
    Ok(MacdResult {'''
    if mask.strip() not in text:
        if anchor not in text:
            raise SystemExit("momentum.rs: MACD warm-up anchor missing")
        text = text.replace(anchor, replacement, 1)

    into_start = text.find("pub fn macd_into(")
    into_end = text.find("\npub fn ", into_start + len("pub fn macd_into("))
    if into_start < 0 or into_end < 0:
        raise SystemExit("momentum.rs: macd_into structural boundary missing")
    into = text[into_start:into_end]
    into_mask = '''
    // Same TA-Lib public lookback as macd(): pre-signal values are seed state.
    macd_line[macd_start..signal_start].fill(f64::NAN);
'''
    if into_mask.strip() not in into:
        return_anchor = "\n    Ok(())\n"
        if into.count(return_anchor) != 1:
            raise SystemExit("momentum.rs: macd_into return anchor changed")
        into = into.replace(return_anchor, into_mask + return_anchor, 1)
        text = text[:into_start] + into + text[into_end:]

    start_token = "pub fn willr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {"
    end_token = "\n}\n\n/// Elder-Ray Indicator Result"
    start = text.find(start_token)
    if start < 0:
        raise SystemExit("momentum.rs: WILLR function anchor missing")
    end = text.find(end_token, start)
    if end < 0:
        raise SystemExit("momentum.rs: WILLR boundary missing")
    current = text[start : end + 2]
    canonical = '''pub fn willr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period)?;

    let mut output = init_output(close.len());
    rolling_minmax_visit(high, low, period, |i, highest, lowest| {
        let range = highest - lowest;
        output[i] = if range > 1e-15 {
            (highest - close[i]) / range * -100.0
        } else {
            0.0
        };
    });
    Ok(output)
}'''
    if "rolling_minmax_visit(high, low, period, |i, highest, lowest|" not in current:
        text = text[:start] + canonical + text[end + 2 :]

    p.write_text(text, encoding="utf-8")


def patch_buffer_arena() -> None:
    p = Path("core/src/buffer_arena.rs")
    text = p.read_text(encoding="utf-8")

    start = text.find("    fn pop_cached(&mut self, len: usize) -> Option<Vec<f64>> {")
    end = text.find("    /// Checkout a zero-filled buffer", start)
    if start < 0 or end < 0:
        raise SystemExit("buffer_arena.rs: pop_cached boundary missing")
    current = text[start:end]
    replacement = '''    fn pop_cached(&mut self, len: usize) -> Option<Vec<f64>> {
        // Prefer an exact logical-length bucket, otherwise reuse the smallest
        // larger allocation. execute_range/eval_last frequently shrink the
        // logical extent while the physical allocation is still reusable.
        let key = if self.free.contains_key(&len) {
            len
        } else {
            self.free.range(len..).next().map(|(&key, _)| key)?
        };

        let mut remove_bucket = false;
        let cached = self.free.get_mut(&key).and_then(|bucket| {
            let buffer = bucket.pop();
            remove_bucket = bucket.is_empty();
            buffer
        });
        if remove_bucket {
            self.free.remove(&key);
        }
        if let Some(buffer) = &cached {
            self.cached_bytes = self
                .cached_bytes
                .saturating_sub(allocation_bytes(buffer.capacity()));
            self.cache_hits = self.cache_hits.saturating_add(1);
        }
        cached
    }

'''
    if "self.free.range(len..).next()" not in current:
        text = text[:start] + replacement + text[end:]

    old = '''    pub fn take_overwrite(&mut self, len: usize) -> Vec<f64> {
        if let Some(buffer) = self.pop_cached(len) {
            return buffer;
        }
        self.cache_misses = self.cache_misses.saturating_add(1);
        vec![0.0; len]
    }
'''
    new = '''    pub fn take_overwrite(&mut self, len: usize) -> Vec<f64> {
        if let Some(mut buffer) = self.pop_cached(len) {
            buffer.truncate(len);
            return buffer;
        }
        self.cache_misses = self.cache_misses.saturating_add(1);
        vec![0.0; len]
    }
'''
    if old in text:
        text = text.replace(old, new, 1)
    elif "buffer.truncate(len);" not in text:
        raise SystemExit("buffer_arena.rs: take_overwrite anchor missing")

    p.write_text(text, encoding="utf-8")


def patch_python_batch_zero_copy() -> None:
    p = Path("ffi/python-binding/src/lib.rs")
    text = p.read_text(encoding="utf-8")
    old = '''    let open_vec: Option<Vec<f64>> = open.as_ref().map(|arr| arr.as_array().to_vec());
    let high_vec: Option<Vec<f64>> = high.as_ref().map(|arr| arr.as_array().to_vec());
    let low_vec: Option<Vec<f64>> = low.as_ref().map(|arr| arr.as_array().to_vec());
    let volume_vec: Option<Vec<f64>> = volume.as_ref().map(|arr| arr.as_array().to_vec());
    let secondary_vec: Option<Vec<f64>> = secondary.as_ref().map(|arr| arr.as_array().to_vec());
'''
    new = '''    let array_error = |e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e));
    let open_slice = open.as_ref().map(|arr| arr.as_slice()).transpose().map_err(array_error)?;
    let high_slice = high.as_ref().map(|arr| arr.as_slice()).transpose().map_err(array_error)?;
    let low_slice = low.as_ref().map(|arr| arr.as_slice()).transpose().map_err(array_error)?;
    let volume_slice = volume.as_ref().map(|arr| arr.as_slice()).transpose().map_err(array_error)?;
    let secondary_slice = secondary.as_ref().map(|arr| arr.as_slice()).transpose().map_err(array_error)?;
'''
    if old in text:
        text = text.replace(old, new, 1)
    elif "let open_slice = open.as_ref().map(|arr| arr.as_slice())" not in text:
        raise SystemExit("python lib.rs: batch zero-copy anchor missing")

    text = text.replace("open_vec.as_deref(),", "open_slice,", 1)
    text = text.replace("high_vec.as_deref(),", "high_slice,", 1)
    text = text.replace("low_vec.as_deref(),", "low_slice,", 1)
    text = text.replace("volume_vec.as_deref(),", "volume_slice,", 1)
    text = text.replace("secondary_vec.as_deref(),", "secondary_slice,", 1)
    p.write_text(text, encoding="utf-8")


def patch_sync_bindings() -> None:
    p = Path("scripts/sync_bindings.py")
    text = p.read_text(encoding="utf-8")
    marker = "from pathlib import Path\n"
    import_line = "from optimize_python_bindings import optimize_source as optimize_python_source\n"
    if import_line not in text:
        if marker not in text:
            raise SystemExit("sync_bindings.py: import anchor missing")
        text = text.replace(marker, marker + import_line, 1)

    old_emit = '    return header + "\\n".join(bodies) + "\\n"\n'
    new_emit = (
        '    text = header + "\\n".join(bodies) + "\\n"\n'
        '    if lang == "python":\n'
        '        text, _ = optimize_python_source(text)\n'
        '    return text\n'
    )
    if old_emit in text:
        text = text.replace(old_emit, new_emit, 1)
    elif new_emit not in text:
        raise SystemExit("sync_bindings.py: emit anchor missing")

    check_anchor = '''    for lang in langs:
        cfg = LANG_CFG[lang]
        src = (ROOT / cfg["lib"]).read_text(encoding="utf-8")
'''
    check_replacement = '''    for lang in langs:
        cfg = LANG_CFG[lang]
        if lang == "python":
            gen_path = ROOT / cfg["gen"]
            expected = emit_generated(lang, inds)
            actual = gen_path.read_text(encoding="utf-8") if gen_path.exists() else ""
            if actual != expected:
                print(f"[check/{lang}] generated binding drift")
                rc = 1
            else:
                print(f"[check/{lang}] NumPy-direct generated binding OK")
            continue
        src = (ROOT / cfg["lib"]).read_text(encoding="utf-8")
'''
    if check_anchor in text:
        text = text.replace(check_anchor, check_replacement, 1)
    elif "NumPy-direct generated binding OK" not in text:
        raise SystemExit("sync_bindings.py: check anchor missing")
    p.write_text(text, encoding="utf-8")


def patch_formula_canonical() -> None:
    p = Path("ffi/python-binding/src/formula_plan.rs")
    if not p.exists():
        return
    text = p.read_text(encoding="utf-8")
    old_enum = '''enum CanonicalFormula {
    Atr { period: usize },
    Std { period: usize },
    Boll { period: usize, nbdev: f64 },
    Roc { period: usize },
    Mom { period: usize },
}'''
    new_enum = '''enum CanonicalFormula {
    Sma { period: usize },
    Ema { period: usize },
    Rsi { period: usize },
    Atr { period: usize },
    Std { period: usize },
    Boll { period: usize, nbdev: f64 },
    Roc { period: usize },
    Mom { period: usize },
}'''
    if old_enum in text:
        text = text.replace(old_enum, new_enum, 1)
    if "CanonicalFormula::Sma" not in text and "enum CanonicalFormula" in text:
        raise SystemExit("formula_plan.rs: canonical SMA/EMA/RSI lowering anchor changed")
    p.write_text(text, encoding="utf-8")


def main() -> None:
    patch_statistics()
    patch_overlap()
    patch_momentum()
    patch_buffer_arena()
    patch_python_batch_zero_copy()
    patch_sync_bindings()
    print("Architecture v3.1 convergence patches applied")


if __name__ == "__main__":
    main()
