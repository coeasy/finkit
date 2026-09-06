#!/usr/bin/env python3
"""Apply Architecture v3 round 6 extrema-kernel convergence.

This migration is intentionally idempotent. It moves MIDPOINT, MIDPRICE and
WILLR onto caller-owned `*_into` kernels, replaces the cached-extrema rescan
hot loop with a stack-backed monotonic ring for technical-analysis windows,
and removes the duplicate Python-binding extrema implementation.
"""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(path: str, pattern: str, replacement: str, *, flags: int = 0) -> None:
    target = ROOT / path
    text = target.read_text()
    updated, count = re.subn(pattern, replacement, text, count=1, flags=flags)
    if count == 0:
        if replacement.strip() in text:
            return
        raise RuntimeError(f"pattern not found in {path}: {pattern[:80]!r}")
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
            while high_head < high_tail
                && high_queue[high_head & RING_MASK].saturating_add(window) <= i
            {
                high_head += 1;
            }

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
            while low_head < low_tail
                && low_queue[low_head & RING_MASK].saturating_add(window) <= i
            {
                low_head += 1;
            }

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

    // Large-window compatibility fallback: keep the previous cached-index
    // algorithm to avoid a window-sized heap allocation in the generic path.
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

replace_once(
    "core/src/math/statistics.rs",
    r"#\[inline\]\npub\(crate\) fn rolling_minmax_visit\(.*?\n\}\n(?=\n/// Find maximum value in a rolling window)",
    STATISTICS_KERNEL,
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
replace_once(
    "core/src/indicators/overlap.rs",
    r"pub fn midpoint\(input: &\[f64\], period: usize\) -> Result<Array1<f64>> \{.*?\n\}",
    MIDPOINT,
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
replace_once(
    "core/src/indicators/overlap.rs",
    r"pub fn midprice\(high: &\[f64\], low: &\[f64\], period: usize\) -> Result<Array1<f64>> \{.*?\n\}",
    MIDPRICE,
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
replace_once(
    "core/src/indicators/momentum.rs",
    r"pub fn willr\(high: &\[f64\], low: &\[f64\], close: &\[f64\], period: usize\) -> Result<Array1<f64>> \{.*?\n\}",
    WILLR,
    flags=re.S,
)

# The pre-round6 file also exposes a later legacy `willr_into` in the generic
# zero-copy section. Remove that exact block after installing the canonical
# implementation above so the migration is truly idempotent and cannot
# reintroduce E0428 duplicate-symbol failures on subsequent runs.
replace_once(
    "core/src/indicators/momentum.rs",
    r"\n/// Williams %R zero-copy variant: writes result into pre-allocated slice\.\npub fn willr_into\(.*?\n\}\n(?=\n/// Momentum zero-copy variant:)",
    "\n",
    flags=re.S,
)

# Remove the binding-local extrema implementation. The language boundary now
# allocates exactly one output Vec and delegates to the core caller-owned API.
replace_once(
    "ffi/python-binding/src/native_fast_path.rs",
    r"/// Sliding extrema with TA-Lib-style cached extreme indexes\..*?(?=#\[inline\]\nfn mom_vec)",
    "",
    flags=re.S,
)

replace_once(
    "ffi/python-binding/src/native_fast_path.rs",
    r'''"midpoint" => \{\n\s*validate_period\(close\.len\(\), timeperiod\)\?;\n\s*py\.detach\(\|\| midpoint_vec\(close, close, timeperiod\)\)\n\s*\}''',
    '''"midpoint" => {
            let mut output = vec![0.0; close.len()];
            py.detach(|| indicators::midpoint_into(close, timeperiod, &mut output))
                .map_err(value_error)?;
            output
        }''',
)
replace_once(
    "ffi/python-binding/src/native_fast_path.rs",
    r'''"midprice" => \{\n\s*validate_period\(input_a\.len\(\), timeperiod\)\?;\n\s*py\.detach\(\|\| midpoint_vec\(input_a, input_b, timeperiod\)\)\n\s*\}''',
    '''"midprice" => {
            let mut output = vec![0.0; input_a.len()];
            py.detach(|| indicators::midprice_into(input_a, input_b, timeperiod, &mut output))
                .map_err(value_error)?;
            output
        }''',
)
replace_once(
    "ffi/python-binding/src/native_fast_path.rs",
    r'''"willr" => \{\n\s*validate_period\(high\.len\(\), timeperiod\)\?;\n\s*py\.detach\(\|\| willr_vec\(high, low, close, timeperiod\)\)\n\s*\}''',
    '''"willr" => {
            let mut output = vec![0.0; high.len()];
            py.detach(|| indicators::willr_into(high, low, close, timeperiod, &mut output))
                .map_err(value_error)?;
            output
        }''',
)

print("Architecture v3 round 6 extrema convergence applied")
