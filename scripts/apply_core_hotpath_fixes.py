#!/usr/bin/env python3
"""Close the remaining P1 core hot-path gaps from the TA-Lib plan.

Only confirmed gaps are changed:
- TRANGE/NATR/OBV `_into` functions must not allocate an Array1/Vec and copy it;
- WILLR and STOCH must use true monotonic deques rather than rescanning a full
  window whenever the previous extreme expires.

Rolling max/min, AROON, ADX-family sharing, and rolling statistics are already
O(n) / O(1)-state in the current core and are intentionally left untouched.
"""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def span(text: str, marker: str) -> tuple[int, int]:
    start = text.find(marker)
    if start < 0:
        raise RuntimeError(f"marker not found: {marker}")
    brace = text.find("{", start)
    if brace < 0:
        raise RuntimeError(f"brace not found: {marker}")
    depth = 0
    in_string = False
    escaped = False
    for i in range(brace, len(text)):
        ch = text[i]
        if in_string:
            if escaped:
                escaped = False
            elif ch == "\\":
                escaped = True
            elif ch == '"':
                in_string = False
            continue
        if ch == '"':
            in_string = True
        elif ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return start, i + 1
    raise RuntimeError(f"unbalanced function: {marker}")


def replace_function(path: Path, marker: str, replacement: str) -> None:
    text = path.read_text(encoding="utf-8")
    if replacement.strip() in text:
        print(f"{path.name}:{marker}: already optimized")
        return
    start, end = span(text, marker)
    path.write_text(text[:start] + replacement.rstrip() + text[end:], encoding="utf-8")
    print(f"{path.name}:{marker}: optimized")


def optimize_trange_natr_into() -> None:
    path = ROOT / "core/src/indicators/volatility.rs"
    replace_function(
        path,
        "pub fn natr_into(",
        r'''pub fn natr_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<()> {
    if output.len() != high.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    // Reuse the allocation-free ATR state machine, then normalize in-place.
    atr_into(high, low, close, period, output)?;
    for i in period..output.len() {
        let c = close[i];
        if c.abs() > 1e-15 {
            output[i] = output[i] / c * 100.0;
        } else {
            output[i] = f64::NAN;
        }
    }
    Ok(())
}''',
    )
    replace_function(
        path,
        "pub fn trange_into(",
        r'''pub fn trange_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    output: &mut [f64],
) -> crate::error::Result<()> {
    if high.len() != low.len() || high.len() != close.len() || output.len() != high.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "high, low, close, output".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;
    output[0] = f64::NAN;
    if high.len() > 1 {
        crate::math::simd_ops::simd_true_range(
            &high[1..],
            &low[1..],
            &close[..close.len() - 1],
            &mut output[1..],
        );
    }
    Ok(())
}''',
    )


def optimize_obv_into() -> None:
    path = ROOT / "core/src/indicators/volume.rs"
    replace_function(
        path,
        "pub fn obv_into(",
        r'''pub fn obv_into(close: &[f64], volume: &[f64], output: &mut [f64]) -> Result<()> {
    if close.len() != volume.len() || output.len() != close.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "close, volume, output".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(close.len(), 1)?;
    crate::math::simd_ops::simd_obv(close, volume, output);
    Ok(())
}''',
    )


def optimize_willr() -> None:
    path = ROOT / "core/src/indicators/momentum.rs"
    replace_function(
        path,
        "pub fn willr(",
        r'''pub fn willr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period)?;

    let len = close.len();
    let mut output = init_output(len);
    let mut max_dq: VecDeque<usize> = VecDeque::with_capacity(period + 1);
    let mut min_dq: VecDeque<usize> = VecDeque::with_capacity(period + 1);

    for i in 0..len {
        while let Some(&back) = max_dq.back() {
            if high[back] <= high[i] {
                max_dq.pop_back();
            } else {
                break;
            }
        }
        max_dq.push_back(i);
        while let Some(&back) = min_dq.back() {
            if low[back] >= low[i] {
                min_dq.pop_back();
            } else {
                break;
            }
        }
        min_dq.push_back(i);

        let window_start = i.saturating_add(1).saturating_sub(period);
        while max_dq.front().is_some_and(|front| *front < window_start) {
            max_dq.pop_front();
        }
        while min_dq.front().is_some_and(|front| *front < window_start) {
            min_dq.pop_front();
        }

        if i + 1 >= period {
            let highest = high[*max_dq.front().expect("non-empty max deque")];
            let lowest = low[*min_dq.front().expect("non-empty min deque")];
            let denom = highest - lowest;
            output[i] = if denom > 1e-15 {
                (highest - close[i]) / denom * -100.0
            } else {
                0.0
            };
        }
    }

    Ok(output)
}''',
    )


def optimize_stoch() -> None:
    path = ROOT / "core/src/math/simd_kernels.rs"
    replacement = r'''fn stoch_scalar(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    k_period: usize,
    k_slow: usize,
    d_period: usize,
    k_out: &mut [f64],
    d_out: &mut [f64],
) {
    let len = close.len();
    for value in k_out.iter_mut().take(len) {
        *value = f64::NAN;
    }
    for value in d_out.iter_mut().take(len) {
        *value = f64::NAN;
    }
    if k_period == 0 || k_slow == 0 || d_period == 0 || len < k_period {
        return;
    }

    let fastk_start = k_period - 1;
    let slowk_start = fastk_start + k_slow - 1;
    let slowd_start = slowk_start + d_period - 1;
    if slowd_start >= len {
        return;
    }

    let mut max_dq: std::collections::VecDeque<usize> =
        std::collections::VecDeque::with_capacity(k_period + 1);
    let mut min_dq: std::collections::VecDeque<usize> =
        std::collections::VecDeque::with_capacity(k_period + 1);
    let mut fast_k_ring = alloc::vec![0.0_f64; k_slow];
    let mut slow_k_ring = alloc::vec![0.0_f64; d_period];
    let mut fast_ring_pos = 0usize;
    let mut slow_ring_pos = 0usize;
    let mut fast_sum = 0.0;
    let mut slow_sum = 0.0;
    let inv_k = 1.0 / k_slow as f64;
    let inv_d = 1.0 / d_period as f64;

    for i in 0..len {
        while let Some(&back) = max_dq.back() {
            if high[back] <= high[i] {
                max_dq.pop_back();
            } else {
                break;
            }
        }
        max_dq.push_back(i);
        while let Some(&back) = min_dq.back() {
            if low[back] >= low[i] {
                min_dq.pop_back();
            } else {
                break;
            }
        }
        min_dq.push_back(i);

        let window_start = i.saturating_add(1).saturating_sub(k_period);
        while max_dq.front().is_some_and(|front| *front < window_start) {
            max_dq.pop_front();
        }
        while min_dq.front().is_some_and(|front| *front < window_start) {
            min_dq.pop_front();
        }
        if i < fastk_start {
            continue;
        }

        let highest = high[*max_dq.front().expect("non-empty max deque")];
        let lowest = low[*min_dq.front().expect("non-empty min deque")];
        let denom = highest - lowest;
        let fast_k = if denom > 1e-15 {
            (close[i] - lowest) / denom * 100.0
        } else {
            50.0
        };

        fast_sum += fast_k - fast_k_ring[fast_ring_pos];
        fast_k_ring[fast_ring_pos] = fast_k;
        fast_ring_pos += 1;
        if fast_ring_pos == k_slow {
            fast_ring_pos = 0;
        }
        let slow_k = fast_sum * inv_k;

        slow_sum += slow_k - slow_k_ring[slow_ring_pos];
        slow_k_ring[slow_ring_pos] = slow_k;
        slow_ring_pos += 1;
        if slow_ring_pos == d_period {
            slow_ring_pos = 0;
        }

        if i >= slowd_start {
            k_out[i] = slow_k;
            d_out[i] = slow_sum * inv_d;
        }
    }
}'''
    replace_function(path, "fn stoch_scalar(", replacement)

    # The previous AVX2 implementation vectorized only the occasional full
    # rescan, so its worst-case complexity remained O(n*period).  The branchy
    # deque state machine is not lane-vectorizable; delegate to the O(n) kernel.
    replacement_avx = r'''unsafe fn stoch_avx2(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    k_period: usize,
    k_slow: usize,
    d_period: usize,
    k_out: &mut [f64],
    d_out: &mut [f64],
) {
    stoch_scalar(high, low, close, k_period, k_slow, d_period, k_out, d_out);
}'''
    replace_function(path, "unsafe fn stoch_avx2(", replacement_avx)


def main() -> int:
    optimize_trange_natr_into()
    optimize_obv_into()
    optimize_willr()
    optimize_stoch()
    print("core hot-path P1 fixes applied")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
