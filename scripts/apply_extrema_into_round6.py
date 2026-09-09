#!/usr/bin/env python3
"""Apply Architecture v3 round 6 extrema-kernel convergence.

The migration is deliberately idempotent. MIDPOINT, MIDPRICE and WILLR share
caller-owned ``*_into`` paths and the fused rolling extrema visitor. Existing
canonical stages are detected by stable semantic markers rather than exact
rustfmt output so re-running the migration cannot duplicate functions.
"""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_if_missing(
    path: str,
    pattern: str,
    replacement: str,
    *,
    marker: str,
    flags: int = 0,
) -> None:
    target = ROOT / path
    text = target.read_text()
    if marker in text:
        return
    updated, count = re.subn(pattern, replacement, text, count=1, flags=flags)
    if count != 1:
        raise RuntimeError(f"pattern not found in {path}: {pattern[:80]!r}")
    target.write_text(updated)


def remove_if_present(path: str, pattern: str, *, flags: int = 0) -> None:
    target = ROOT / path
    text = target.read_text()
    updated, count = re.subn(pattern, "", text, count=1, flags=flags)
    if count:
        target.write_text(updated)


STATISTICS_KERNEL = r'''#[inline]
pub(crate) fn rolling_minmax_visit(
    high: &[f64],
    low: &[f64],
    window: usize,
    mut emit: impl FnMut(usize, f64, f64),
) {
    debug_assert_eq!(high.len(), low.len());
    debug_assert!(window > 0);

    if high.is_empty() || high.len() != low.len() || window == 0 {
        return;
    }

    // Most TA windows are small. Keep monotonic queues on the stack so every
    // bar is inserted/removed at most once without heap allocation or expiry
    // rescans. The power-of-two ring makes wrapping a single mask operation.
    const RING_CAPACITY: usize = 256;
    const RING_MASK: usize = RING_CAPACITY - 1;
    if window <= RING_CAPACITY {
        let mut high_queue = [0usize; RING_CAPACITY];
        let mut low_queue = [0usize; RING_CAPACITY];
        let mut high_head = 0usize;
        let mut high_tail = 0usize;
        let mut low_head = 0usize;
        let mut low_tail = 0usize;

        for i in 0..high.len() {
            // Expire stale fronts before insertion so a full 256-slot ring is
            // never overwritten before its oldest element has been removed.
            while high_head < high_tail
                && high_queue[high_head & RING_MASK].saturating_add(window) <= i
            {
                high_head += 1;
            }
            while low_head < low_tail
                && low_queue[low_head & RING_MASK].saturating_add(window) <= i
            {
                low_head += 1;
            }

            let new_high = high[i];
            while high_head < high_tail {
                let back = high_queue[(high_tail - 1) & RING_MASK];
                if high[back] <= new_high {
                    high_tail -= 1;
                } else {
                    break;
                }
            }
            high_queue[high_tail & RING_MASK] = i;
            high_tail += 1;

            let new_low = low[i];
            while low_head < low_tail {
                let back = low_queue[(low_tail - 1) & RING_MASK];
                if low[back] >= new_low {
                    low_tail -= 1;
                } else {
                    break;
                }
            }
            low_queue[low_tail & RING_MASK] = i;
            low_tail += 1;

            if i + 1 >= window {
                emit(
                    i,
                    high[high_queue[high_head & RING_MASK]],
                    low[low_queue[low_head & RING_MASK]],
                );
            }
        }
        return;
    }

    // Large-window compatibility fallback: retain the cached-index algorithm
    // without a window-sized heap allocation in the generic path.
    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();
    let mut highest_idx = 0usize;
    let mut lowest_idx = 0usize;
    let mut highest = f64::NEG_INFINITY;
    let mut lowest = f64::INFINITY;

    for i in 0..high.len() {
        unsafe {
            let new_high = *high_ptr.add(i);
            let new_low = *low_ptr.add(i);

            if i < window {
                if new_high >= highest {
                    highest = new_high;
                    highest_idx = i;
                }
                if new_low <= lowest {
                    lowest = new_low;
                    lowest_idx = i;
                }
            } else {
                let window_start = i + 1 - window;
                if highest_idx < window_start {
                    highest = *high_ptr.add(window_start);
                    highest_idx = window_start;
                    let mut scan = window_start + 1;
                    while scan <= i {
                        let candidate = *high_ptr.add(scan);
                        if candidate >= highest {
                            highest = candidate;
                            highest_idx = scan;
                        }
                        scan += 1;
                    }
                } else if new_high >= highest {
                    highest = new_high;
                    highest_idx = i;
                }

                if lowest_idx < window_start {
                    lowest = *low_ptr.add(window_start);
                    lowest_idx = window_start;
                    let mut scan = window_start + 1;
                    while scan <= i {
                        let candidate = *low_ptr.add(scan);
                        if candidate <= lowest {
                            lowest = candidate;
                            lowest_idx = scan;
                        }
                        scan += 1;
                    }
                } else if new_low <= lowest {
                    lowest = new_low;
                    lowest_idx = i;
                }
            }

            if i + 1 >= window {
                emit(i, highest, lowest);
            }
        }
    }
}'''

replace_if_missing(
    "core/src/math/statistics.rs",
    r"#\[inline\]\npub\(crate\) fn rolling_minmax_visit\(.*?\n\}\n(?=\s*/// Find maximum value in a rolling window)",
    STATISTICS_KERNEL,
    marker="Expire stale fronts before insertion",
    flags=re.S,
)

MIDPOINT = r'''pub fn midpoint(input: &[f64], period: usize) -> Result<Array1<f64>> {
    let mut output = Array1::<f64>::zeros(input.len());
    midpoint_into(input, period, output.as_slice_mut().unwrap())?;
    Ok(output)
}

/// Caller-owned MIDPOINT kernel used by runtime and language bindings.
pub fn midpoint_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
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

    crate::utils::simd_fill_nan(&mut output[..period - 1]);
    rolling_minmax_visit(input, input, period, |i, highest, lowest| {
        output[i] = (highest + lowest) * 0.5;
    });
    Ok(())
}'''
replace_if_missing(
    "core/src/indicators/overlap.rs",
    r"pub fn midpoint\(input: &\[f64\], period: usize\) -> Result<Array1<f64>> \{.*?\n\}",
    MIDPOINT,
    marker="pub fn midpoint_into(",
    flags=re.S,
)

MIDPRICE = r'''pub fn midprice(high: &[f64], low: &[f64], period: usize) -> Result<Array1<f64>> {
    let mut output = Array1::<f64>::zeros(high.len());
    midprice_into(high, low, period, output.as_slice_mut().unwrap())?;
    Ok(output)
}

/// Caller-owned MIDPRICE kernel used by runtime and language bindings.
pub fn midprice_into(high: &[f64], low: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(high.len(), period)?;
    if output.len() != high.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }

    crate::utils::simd_fill_nan(&mut output[..period - 1]);
    rolling_minmax_visit(high, low, period, |i, highest, lowest| {
        output[i] = (highest + lowest) * 0.5;
    });
    Ok(())
}'''
replace_if_missing(
    "core/src/indicators/overlap.rs",
    r"pub fn midprice\(high: &\[f64\], low: &\[f64\], period: usize\) -> Result<Array1<f64>> \{.*?\n\}",
    MIDPRICE,
    marker="pub fn midprice_into(",
    flags=re.S,
)

WILLR = r'''pub fn willr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    let mut output = Array1::<f64>::zeros(close.len());
    willr_into(high, low, close, period, output.as_slice_mut().unwrap())?;
    Ok(output)
}

/// Caller-owned Williams %R kernel sharing the canonical extrema lifecycle.
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
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(high.len(), period)?;
    if output.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }

    crate::utils::simd_fill_nan(&mut output[..period - 1]);
    rolling_minmax_visit(high, low, period, |i, highest, lowest| {
        let range = highest - lowest;
        output[i] = if range > 1e-15 {
            (highest - close[i]) / range * -100.0
        } else {
            0.0
        };
    });
    Ok(())
}'''
replace_if_missing(
    "core/src/indicators/momentum.rs",
    r"pub fn willr\(high: &\[f64\], low: &\[f64\], close: &\[f64\], period: usize\) -> Result<Array1<f64>> \{.*?\n\}",
    WILLR,
    marker="Caller-owned Williams %R kernel sharing the canonical extrema lifecycle.",
    flags=re.S,
)

# Remove the pre-round6 duplicate after installing the canonical function.
remove_if_present(
    "core/src/indicators/momentum.rs",
    r"\n/// Williams %R zero-copy variant: writes result into pre-allocated slice\.\npub fn willr_into\(.*?\n\}\n(?=\n/// Momentum zero-copy variant:)",
    flags=re.S,
)

# Remove binding-local extrema kernels when present; dispatches below delegate
# to the same core caller-owned APIs used by Rust/runtime execution.
remove_if_present(
    "ffi/python-binding/src/native_fast_path.rs",
    r"/// Sliding extrema with TA-Lib-style cached extreme indexes\..*?(?=#\[inline\]\nfn mom_vec)",
    flags=re.S,
)

replace_if_missing(
    "ffi/python-binding/src/native_fast_path.rs",
    r'''"midpoint" => \{\n\s*validate_period\(close\.len\(\), timeperiod\)\?;\n\s*py\.detach\(\|\| midpoint_vec\(close, close, timeperiod\)\)\n\s*\}''',
    '''"midpoint" => {
            let mut output = vec![0.0; close.len()];
            py.detach(|| indicators::midpoint_into(close, timeperiod, &mut output))
                .map_err(value_error)?;
            output
        }''',
    marker="indicators::midpoint_into(close, timeperiod, &mut output)",
)
replace_if_missing(
    "ffi/python-binding/src/native_fast_path.rs",
    r'''"midprice" => \{\n\s*validate_period\(input_a\.len\(\), timeperiod\)\?;\n\s*py\.detach\(\|\| midpoint_vec\(input_a, input_b, timeperiod\)\n\s*\}''',
    '''"midprice" => {
            let mut output = vec![0.0; input_a.len()];
            py.detach(|| indicators::midprice_into(input_a, input_b, timeperiod, &mut output))
                .map_err(value_error)?;
            output
        }''',
    marker="indicators::midprice_into(input_a, input_b, timeperiod, &mut output)",
)
replace_if_missing(
    "ffi/python-binding/src/native_fast_path.rs",
    r'''"willr" => \{\n\s*validate_period\(high\.len\(\), timeperiod\)\?;\n\s*py\.detach\(\|\| willr_vec\(high, low, close, timeperiod\)\)\n\s*\}''',
    '''"willr" => {
            let mut output = vec![0.0; high.len()];
            py.detach(|| indicators::willr_into(high, low, close, timeperiod, &mut output))
                .map_err(value_error)?;
            output
        }''',
    marker="indicators::willr_into(high, low, close, timeperiod, &mut output)",
)

print("Architecture v3 round 6 extrema convergence applied")
