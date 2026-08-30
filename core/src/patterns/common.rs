//! Common helpers for K-line and chart pattern detection.
//!
//! This module provides shared utilities used by all pattern detectors in
//! [`super::candlestick`], [`super::chart`], and the A-share extensions
//! ([`super::astock_kline`], [`super::astock_ma`], [`super::harmonic`],
//! [`super::classic_ext`]).
//!
//! # Provided utilities
//!
//! - [`Signal`] — type alias matching TA-Lib's `i32` convention
//!   (100 = bullish, -100 = bearish, 0 = no pattern).
//! - [`validate_ohlcv`] — uniform OHLCV length + minimum-size check.
//! - [`avg_true_range`] / [`precompute_atr`] — ATR family (simple mean +
//!   Wilder smoothing).
//! - [`is_uptrend`] / [`is_downtrend`] — trend context detection.
//! - [`body`], [`upper_shadow`], [`lower_shadow`], [`is_bullish`],
//!   [`is_bearish`] — micro-helpers used by every candle pattern.
//! - [`precompute_sma`] / [`precompute_volume_ratio`] — pre-computed
//!   moving-average helpers shared across many pattern detectors.

use crate::error::{Result, TaError};
use crate::utils::validate_input;
use ndarray::Array1;

/// Pattern signal — TA-Lib compatible.
///
/// * `100`  — bullish pattern
/// * `-100` — bearish pattern
/// * `0`    — no pattern
pub type Signal = i32;

/// Validate that all OHLCV arrays share the same length and that the length
/// meets `min_len`.
///
/// # Errors
///
/// * [`TaError::InvalidParameter`] — arrays have different lengths
/// * [`TaError::EmptyInput`] / [`TaError::InsufficientData`] — too few bars
pub fn validate_ohlcv(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    min_len: usize,
) -> Result<()> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "OHLC".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), min_len)
}

/// Validate that `volume` matches `close` length and that the input meets
/// `min_len`.
///
/// # Errors
///
/// * [`TaError::InvalidParameter`] — `close.len() != volume.len()`
/// * [`TaError::EmptyInput`] / [`TaError::InsufficientData`] — too few bars
pub fn validate_ohlcv_with_volume(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    min_len: usize,
) -> Result<()> {
    validate_ohlcv(open, high, low, close, min_len)?;
    if close.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "volume".to_string(),
            constraint: "must have the same length as OHLC".to_string(),
        });
    }
    Ok(())
}

/// Simple mean of the high-low range / true range over `[idx-period+1, idx]`.
///
/// This is the original formula used by [`super::candlestick`] (mean of TR
/// over the lookback window). For Wilder-smoothed ATR, use
/// [`precompute_atr`] instead.
pub fn avg_true_range(high: &[f64], low: &[f64], close: &[f64], period: usize, idx: usize) -> f64 {
    let start = idx.saturating_sub(period - 1);
    let mut sum = 0.0;
    for i in start..=idx {
        let tr = if i == 0 {
            high[i] - low[i]
        } else {
            let prev_close = close[i - 1];
            (high[i] - low[i])
                .max((high[i] - prev_close).abs())
                .max((low[i] - prev_close).abs())
        };
        sum += tr;
    }
    sum / (idx - start + 1) as f64
}

/// Pre-compute a Wilder-smoothed ATR array for the entire series.
///
/// This is the single biggest performance win for batch pattern detection:
/// the legacy code recomputes TR × period work for every (pattern, bar)
/// pair, leading to `O(N × patterns × period)` total work. With this helper,
/// the cost drops to `O(N × period) + O(N × patterns)` (one ATR pass + a
/// constant-time lookup per pattern/bar).
///
/// # Returns
///
/// A `Vec<f64>` of length `high.len()`. Entries before `period-1` are `0.0`.
pub fn precompute_atr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<f64> {
    let n = high.len();
    if n == 0 {
        return Vec::new();
    }
    if period == 0 || period > n {
        return vec![0.0; n];
    }
    let mut atr = vec![0.0; n];
    // Seed ATR with simple mean of the first `period` true ranges
    atr[period - 1] = avg_true_range(high, low, close, period, period - 1);
    for i in period..n {
        let prev_close = close[i - 1];
        let tr = (high[i] - low[i])
            .max((high[i] - prev_close).abs())
            .max((low[i] - prev_close).abs());
        atr[i] = (atr[i - 1] * (period - 1) as f64 + tr) / period as f64;
    }
    atr
}

/// Pre-compute a simple moving average (SMA) array.
///
/// `out[i] = NaN` for `i < period - 1`, else the mean of `input[i-period+1..=i]`.
pub fn precompute_sma(input: &[f64], period: usize) -> Vec<f64> {
    let n = input.len();
    let mut out = vec![f64::NAN; n];
    if period == 0 || period > n {
        return out;
    }
    let mut sum: f64 = input[..period].iter().sum();
    out[period - 1] = sum / period as f64;
    for i in period..n {
        sum += input[i] - input[i - period];
        out[i] = sum / period as f64;
    }
    out
}

/// Pre-compute volume / volume-MA ratio array.
///
/// `out[i] = volume[i] / sma(volume, period)[i]`. Entries before the SMA
/// warmup are `NaN`.
pub fn precompute_volume_ratio(volume: &[f64], period: usize) -> Vec<f64> {
    let n = volume.len();
    let mut out = vec![f64::NAN; n];
    if period == 0 || period > n {
        return out;
    }
    let ma = precompute_sma(volume, period);
    for i in period - 1..n {
        if ma[i] > 0.0 {
            out[i] = volume[i] / ma[i];
        }
    }
    out
}

/// Detect whether the price has been in an uptrend over the last `lookback`
/// bars (close gained at least `pct` from the start of the window).
pub fn is_uptrend(close: &[f64], lookback: usize, idx: usize, pct: f64) -> bool {
    if idx + 1 < lookback {
        return false;
    }
    let start_close = close[idx + 1 - lookback];
    if start_close <= 0.0 {
        return false;
    }
    close[idx] >= start_close * (1.0 + pct)
}

/// Detect whether the price has been in a downtrend (symmetric of
/// [`is_uptrend`]).
pub fn is_downtrend(close: &[f64], lookback: usize, idx: usize, pct: f64) -> bool {
    if idx + 1 < lookback {
        return false;
    }
    let start_close = close[idx + 1 - lookback];
    if start_close <= 0.0 {
        return false;
    }
    close[idx] <= start_close * (1.0 - pct)
}

/// Detect whether the current bar is at a relative high (close near the
/// recent max — useful for "in overbought zone" pattern gates).
pub fn at_recent_high(close: &[f64], lookback: usize, idx: usize, threshold: f64) -> bool {
    if idx + 1 < lookback {
        return false;
    }
    let start = idx + 1 - lookback;
    let max = close[start..=idx]
        .iter()
        .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    if max <= 0.0 {
        return false;
    }
    close[idx] >= max * (1.0 - threshold)
}

/// Detect whether the current bar is at a relative low (symmetric).
pub fn at_recent_low(close: &[f64], lookback: usize, idx: usize, threshold: f64) -> bool {
    if idx + 1 < lookback {
        return false;
    }
    let start = idx + 1 - lookback;
    let min = close[start..=idx]
        .iter()
        .fold(f64::INFINITY, |a, &b| a.min(b));
    if min <= 0.0 {
        return false;
    }
    close[idx] <= min * (1.0 + threshold)
}

/// Body size (|close - open|).
#[inline]
pub fn body(o: f64, c: f64) -> f64 {
    (c - o).abs()
}

/// Upper shadow (high - max(open, close)).
#[inline]
pub fn upper_shadow(h: f64, o: f64, c: f64) -> f64 {
    h - o.max(c)
}

/// Lower shadow (min(open, close) - low).
#[inline]
pub fn lower_shadow(l: f64, o: f64, c: f64) -> f64 {
    o.min(c) - l
}

/// Total range (high - low).
#[inline]
pub fn total_range(h: f64, l: f64) -> f64 {
    h - l
}

/// True the candle closed above its open.
#[inline]
pub fn is_bullish(o: f64, c: f64) -> bool {
    c > o
}

/// True the candle closed below its open.
#[inline]
pub fn is_bearish(o: f64, c: f64) -> bool {
    c < o
}

/// True the candle is a doji (body is `≤ body_pct × total_range`).
#[inline]
pub fn is_doji(o: f64, h: f64, l: f64, c: f64, body_pct: f64) -> bool {
    let r = total_range(h, l);
    if r <= 0.0 {
        return false;
    }
    body(o, c) <= r * body_pct
}

/// Number of consecutive bullish bars ending at `idx` (0 if `close[idx] <= open[idx]`).
pub fn consecutive_bull_count(open: &[f64], close: &[f64], idx: usize) -> usize {
    if idx >= open.len() {
        return 0;
    }
    if !is_bullish(open[idx], close[idx]) {
        return 0;
    }
    let mut n = 1;
    let mut i = idx;
    while i > 0 {
        i -= 1;
        if !is_bullish(open[i], close[i]) {
            break;
        }
        n += 1;
    }
    n
}

/// Number of consecutive bearish bars ending at `idx`.
pub fn consecutive_bear_count(open: &[f64], close: &[f64], idx: usize) -> usize {
    if idx >= open.len() {
        return 0;
    }
    if !is_bearish(open[idx], close[idx]) {
        return 0;
    }
    let mut n = 1;
    let mut i = idx;
    while i > 0 {
        i -= 1;
        if !is_bearish(open[i], close[i]) {
            break;
        }
        n += 1;
    }
    n
}

/// True the `idx`-th bar has a gap up from `idx-1` (low > prev high).
#[inline]
pub fn gap_up(low: &[f64], high: &[f64], idx: usize) -> bool {
    idx > 0 && low[idx] > high[idx - 1]
}

/// True the `idx`-th bar has a gap down from `idx-1` (high < prev low).
#[inline]
pub fn gap_down(low: &[f64], high: &[f64], idx: usize) -> bool {
    idx > 0 && high[idx] < low[idx - 1]
}

/// Initialize a zero-filled pattern-signal output array.
#[inline]
pub fn init_signal(n: usize) -> Array1<i32> {
    Array1::zeros(n)
}

// ============================================================================
// Peak / Trough Detection
// ============================================================================
//
// `detect_peaks` / `detect_troughs` mirror the semantics of
// `scipy.signal.find_peaks` so Python users can re-use the same intuition.
// They are designed for **offline** analysis (full history available) and
// return *all* confirmed local extrema in `data`.
//
// # Future-data safety
//
// These helpers are not "no look-ahead" by themselves — a local maximum at
// index `i` is only known to be a maximum once `data[i+1]` is observed.
// For real-time use, callers should:
// 1. Run `detect_peaks` on the full historical window, then
// 2. Emit a "peak confirmed" signal **at index `i+1`** (or later) instead
//    of at `i`.
//
// The functions never modify `data`; they only inspect it. They never
// reference index `j > i` to *predict* the peak at `i`; they only label
// `i` as a peak because of its geometric relationship to `i-1` and `i+1`.
// In a streaming pipeline, the streaming detector handles the confirmation
// delay automatically.

/// Detects local maxima in a 1-D signal (scipy.signal.find_peaks semantics).
///
/// A point `i` is a peak when:
/// - `data[i-1] < data[i]` (strictly higher than the left neighbour), and
/// - `data[i] >= data[i+1]` (higher or equal to the right neighbour).
///
/// The right-hand `>=` lets flat-top plateaus (e.g. `[1, 3, 3, 1]`) emit the
/// leftmost plateau point as the peak — matching scipy's default behaviour.
/// NaN entries are skipped (treated as missing data; a NaN neighbour simply
/// disqualifies the point from being a peak on that side).
///
/// # Arguments
/// * `data`        - 1-D signal
/// * `distance`    - minimal horizontal distance (`>=1`) between neighbouring
///                    peaks. When two candidates are closer than `distance`,
///                    the higher one is kept.
/// * `prominence`  - minimal vertical prominence (`>=0`). Prominence is the
///                    height of the peak above the highest of its left/right
///                    neighbouring valleys. Set to `0.0` to disable.
/// * `width`       - optional minimal width (`>=1`, in samples) of the peak
///                    measured as the number of consecutive samples within
///                    half-prominence of the peak value.
///
/// # Returns
/// Indices where `data[i]` is a peak satisfying all constraints, in ascending
/// order.
pub fn detect_peaks(
    data: &[f64],
    distance: usize,
    prominence: f64,
    width: Option<usize>,
) -> Vec<usize> {
    if data.len() < 3 || distance == 0 {
        return Vec::new();
    }
    // Phase 1: candidate peaks = strict local maxima or plateau starts.
    //   left  : data[i-1] < data[i]  (NaN disqualifies the left side)
    //   right : data[i] >= data[i+1] (NaN disqualifies the right side)
    let mut candidates: Vec<usize> = Vec::new();
    for i in 1..data.len() - 1 {
        if data[i].is_nan() {
            continue;
        }
        let left = data[i - 1];
        let right = data[i + 1];
        if left.is_nan() || right.is_nan() {
            continue;
        }
        if data[i] > left && data[i] >= right {
            candidates.push(i);
        }
    }
    if candidates.is_empty() {
        return Vec::new();
    }
    // Phase 2: enforce `distance`.
    candidates = enforce_distance(candidates, data, distance, true);
    // Phase 3: prominence.
    if prominence > 0.0 {
        candidates.retain(|&i| peak_prominence(data, i) >= prominence);
    }
    // Phase 4: width (optional).
    if let Some(min_w) = width {
        if min_w > 0 {
            candidates.retain(|&i| peak_width(data, i) >= min_w);
        }
    }
    candidates
}

/// Mirror of [`detect_peaks`] for local minima.
pub fn detect_troughs(
    data: &[f64],
    distance: usize,
    prominence: f64,
    width: Option<usize>,
) -> Vec<usize> {
    if data.len() < 3 || distance == 0 {
        return Vec::new();
    }
    let mut candidates: Vec<usize> = Vec::new();
    for i in 1..data.len() - 1 {
        if data[i].is_nan() {
            continue;
        }
        let left = data[i - 1];
        let right = data[i + 1];
        if left.is_nan() || right.is_nan() {
            continue;
        }
        if data[i] < left && data[i] <= right {
            candidates.push(i);
        }
    }
    if candidates.is_empty() {
        return Vec::new();
    }
    candidates = enforce_distance(candidates, data, distance, false);
    if prominence > 0.0 {
        candidates.retain(|&i| trough_prominence(data, i) >= prominence);
    }
    if let Some(min_w) = width {
        if min_w > 0 {
            candidates.retain(|&i| trough_width(data, i) >= min_w);
        }
    }
    candidates
}

/// Greedy `distance` enforcement: walk candidates in order, drop any candidate
/// within `distance` of the last kept one if the kept one is better. When
/// equal, the earlier index wins.
fn enforce_distance(
    mut candidates: Vec<usize>,
    data: &[f64],
    distance: usize,
    is_peak: bool,
) -> Vec<usize> {
    if distance <= 1 {
        return candidates;
    }
    let mut out: Vec<usize> = Vec::with_capacity(candidates.len());
    for c in candidates.drain(..) {
        match out.last() {
            None => out.push(c),
            Some(&last) => {
                if c.saturating_sub(last) < distance {
                    let better = if is_peak {
                        data[c] > data[last] || (data[c] == data[last] && c < last)
                    } else {
                        data[c] < data[last] || (data[c] == data[last] && c < last)
                    };
                    if better {
                        out.pop();
                        out.push(c);
                    }
                } else {
                    out.push(c);
                }
            }
        }
    }
    out
}

/// Prominence of a peak at index `i`: vertical distance from the peak to the
/// highest of its left and right neighbouring valleys (searching outward until
/// the boundary or a higher peak).
fn peak_prominence(data: &[f64], i: usize) -> f64 {
    let peak = data[i];
    let mut left_min = f64::INFINITY;
    let mut k = i;
    while k > 0 {
        k -= 1;
        if data[k].is_nan() {
            continue;
        }
        if data[k] >= peak {
            break;
        }
        if data[k] < left_min {
            left_min = data[k];
        }
    }
    let mut right_min = f64::INFINITY;
    let mut k = i;
    while k + 1 < data.len() {
        k += 1;
        if data[k].is_nan() {
            continue;
        }
        if data[k] >= peak {
            break;
        }
        if data[k] < right_min {
            right_min = data[k];
        }
    }
    let ref_level = left_min.max(right_min);
    if ref_level.is_finite() {
        peak - ref_level
    } else {
        0.0
    }
}

/// Mirror for troughs: vertical distance from the trough to the lowest of its
/// left and right neighbouring peaks.
fn trough_prominence(data: &[f64], i: usize) -> f64 {
    let trough = data[i];
    let mut left_max = f64::NEG_INFINITY;
    let mut k = i;
    while k > 0 {
        k -= 1;
        if data[k].is_nan() {
            continue;
        }
        if data[k] <= trough {
            break;
        }
        if data[k] > left_max {
            left_max = data[k];
        }
    }
    let mut right_max = f64::NEG_INFINITY;
    let mut k = i;
    while k + 1 < data.len() {
        k += 1;
        if data[k].is_nan() {
            continue;
        }
        if data[k] <= trough {
            break;
        }
        if data[k] > right_max {
            right_max = data[k];
        }
    }
    let ref_level = left_max.min(right_max);
    if ref_level.is_finite() {
        ref_level - trough
    } else {
        0.0
    }
}

/// Approximate peak width: number of consecutive samples within
/// `(peak - prominence/2)` of the peak.
fn peak_width(data: &[f64], i: usize) -> usize {
    let prom = peak_prominence(data, i);
    if prom <= 0.0 {
        return 1;
    }
    let threshold = data[i] - prom / 2.0;
    let mut left = i;
    while left > 0 && !data[left - 1].is_nan() && data[left - 1] >= threshold {
        left -= 1;
    }
    let mut right = i;
    while right + 1 < data.len() && !data[right + 1].is_nan() && data[right + 1] >= threshold {
        right += 1;
    }
    right - left + 1
}

/// Approximate trough width: number of consecutive samples within
/// `(trough + prominence/2)` of the trough.
fn trough_width(data: &[f64], i: usize) -> usize {
    let prom = trough_prominence(data, i);
    if prom <= 0.0 {
        return 1;
    }
    let threshold = data[i] + prom / 2.0;
    let mut left = i;
    while left > 0 && !data[left - 1].is_nan() && data[left - 1] <= threshold {
        left -= 1;
    }
    let mut right = i;
    while right + 1 < data.len() && !data[right + 1].is_nan() && data[right + 1] <= threshold {
        right += 1;
    }
    right - left + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_ohlcv_length_mismatch() {
        let o = vec![1.0, 2.0, 3.0];
        let h = vec![1.0, 2.0];
        let l = vec![1.0, 2.0, 3.0];
        let c = vec![1.0, 2.0, 3.0];
        assert!(validate_ohlcv(&o, &h, &l, &c, 1).is_err());
    }

    #[test]
    fn test_validate_ohlcv_too_short() {
        let o = vec![1.0];
        let h = vec![1.0];
        let l = vec![1.0];
        let c = vec![1.0];
        assert!(validate_ohlcv(&o, &h, &l, &c, 5).is_err());
    }

    #[test]
    fn test_avg_true_range_simple() {
        // Two bars: 1→2 (gap-up) and 2→1 (gap-down)
        let h = vec![2.0, 2.0];
        let l = vec![1.0, 1.0];
        let c = vec![1.0, 2.0];
        // idx=0: tr = 1.0 (h-l); idx=1: tr = max(1, 1, 1) = 1
        let atr0 = avg_true_range(&h, &l, &c, 2, 0);
        let atr1 = avg_true_range(&h, &l, &c, 2, 1);
        assert!((atr0 - 1.0).abs() < 1e-10);
        assert!((atr1 - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_precompute_atr_wilder() {
        // 5 bars with constant TR = 1.0 (close flat at 5.0, h-l range = 1.0,
        // so TR = max(h-l, |h-prev_c|, |l-prev_c|) = max(1, 0.5, 0.5) = 1).
        // ATR stabilises at 1.0.
        let h = vec![5.5; 5];
        let l = vec![4.5; 5];
        let c = vec![5.0; 5];
        let atr = precompute_atr(&h, &l, &c, 3);
        assert_eq!(atr.len(), 5);
        // idx 2: simple mean of first 3 TRs = 1.0
        assert!((atr[2] - 1.0).abs() < 1e-10);
        // idx 3+: Wilder = (prev*2 + tr)/3 = 1.0
        assert!((atr[3] - 1.0).abs() < 1e-10);
        assert!((atr[4] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_precompute_sma() {
        let v = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let s = precompute_sma(&v, 3);
        assert!(s[0].is_nan());
        assert!(s[1].is_nan());
        assert!((s[2] - 2.0).abs() < 1e-10);
        assert!((s[3] - 3.0).abs() < 1e-10);
        assert!((s[4] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_is_up_downtrend() {
        let c = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        assert!(is_uptrend(&c, 5, 4, 0.05));
        assert!(!is_downtrend(&c, 5, 4, 0.05));
        let c2 = vec![14.0, 13.0, 12.0, 11.0, 10.0];
        assert!(is_downtrend(&c2, 5, 4, 0.05));
        assert!(!is_uptrend(&c2, 5, 4, 0.05));
    }

    #[test]
    fn test_at_recent_high_low() {
        let c = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        assert!(at_recent_high(&c, 5, 4, 0.01));
        assert!(!at_recent_low(&c, 5, 4, 0.01));
        let c2 = vec![14.0, 13.0, 12.0, 11.0, 10.0];
        assert!(at_recent_low(&c2, 5, 4, 0.01));
        assert!(!at_recent_high(&c2, 5, 4, 0.01));
    }

    #[test]
    fn test_candle_helpers() {
        let (o, h, l, c) = (10.0, 12.0, 8.0, 11.0);
        assert!((body(o, c) - 1.0).abs() < 1e-10);
        assert!((upper_shadow(h, o, c) - 1.0).abs() < 1e-10);
        assert!((lower_shadow(l, o, c) - 2.0).abs() < 1e-10);
        assert!(is_bullish(o, c));
        assert!(!is_bearish(o, c));
    }

    #[test]
    fn test_is_doji() {
        // open=close=10, range=2 → doji
        assert!(is_doji(10.0, 11.0, 9.0, 10.0, 0.1));
        // open=9, close=11, range=2 → not doji
        assert!(!is_doji(9.0, 11.0, 9.0, 11.0, 0.1));
    }

    #[test]
    fn test_consecutive_bull_bear() {
        // 4 bullish bars followed by 1 bearish
        let o = vec![10.0, 10.0, 10.0, 10.0, 12.0];
        let c = vec![11.0, 11.0, 11.0, 11.0, 10.0];
        assert_eq!(consecutive_bull_count(&o, &c, 0), 1);
        assert_eq!(consecutive_bull_count(&o, &c, 3), 4);
        assert_eq!(consecutive_bear_count(&o, &c, 4), 1);
    }

    #[test]
    fn test_gap_up_down() {
        let h = vec![10.0, 12.0, 11.0];
        let l = vec![9.0, 11.0, 8.0];
        // bar 1: low=11 > high=10 → gap up
        assert!(gap_up(&l, &h, 1));
        // bar 2: high=11 < low=11? No. high=11 == low=11 → no gap
        assert!(!gap_down(&l, &h, 2));
    }

    #[test]
    fn test_precompute_volume_ratio() {
        let v = vec![100.0, 200.0, 300.0, 400.0, 500.0];
        let r = precompute_volume_ratio(&v, 3);
        assert!(r[0].is_nan());
        assert!(r[1].is_nan());
        // idx 2: ma=200, ratio=300/200=1.5
        assert!((r[2] - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_init_signal() {
        let s = init_signal(5);
        assert_eq!(s.len(), 5);
        assert!(s.iter().all(|&v| v == 0));
    }

    // -------- detect_peaks / detect_troughs --------

    #[test]
    fn test_detect_peaks_basic() {
        // 1 2 5 2 1 2 4 1 — peak at index 2 (value 5), peak at index 6 (value 4)
        let data = vec![1.0, 2.0, 5.0, 2.0, 1.0, 2.0, 4.0, 1.0];
        let peaks = detect_peaks(&data, 1, 0.0, None);
        assert_eq!(peaks, vec![2, 6]);
    }

    #[test]
    fn test_detect_troughs_basic() {
        let data = vec![5.0, 3.0, 1.0, 3.0, 5.0, 4.0, 2.0, 4.0];
        let troughs = detect_troughs(&data, 1, 0.0, None);
        assert_eq!(troughs, vec![2, 6]);
    }

    #[test]
    fn test_detect_peaks_monotonic_no_peak() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let peaks = detect_peaks(&data, 1, 0.0, None);
        assert!(peaks.is_empty());
    }

    #[test]
    fn test_detect_peaks_distance_filter() {
        // Multiple close peaks, distance=3 should keep only the highest
        let data = vec![1.0, 3.0, 1.0, 4.0, 1.0, 2.0, 1.0];
        let peaks = detect_peaks(&data, 3, 0.0, None);
        // candidates: idx=1 (v=3), idx=3 (v=4), idx=5 (v=2)
        // distance=3: idx=1 and idx=3 are 2 apart → keep idx=3 (higher)
        //             idx=3 and idx=5 are 2 apart → keep idx=3
        assert_eq!(peaks, vec![3]);
    }

    #[test]
    fn test_detect_peaks_prominence_filter() {
        // data = [10, 11, 9, 8, 9.5, 11.5, 9, 7]
        // idx=1 (val=11): left valley = 10 (boundary), right valley walks 9,8,9.5 then hits 11.5 ≥ 11 → ref_level = max(10, 8) = 10, prom = 1.
        // idx=5 (val=11.5): left valley walks 9.5,8,9,11 (still < 11.5), then 10 → left_min = 8; right valley = 7; ref_level = max(8, 7) = 8, prom = 3.5.
        let data = vec![10.0, 11.0, 9.0, 8.0, 9.5, 11.5, 9.0, 7.0];
        // prom=0: both candidates
        let all = detect_peaks(&data, 1, 0.0, None);
        assert_eq!(all, vec![1, 5]);
        // prom=1.5: idx=1 (prom=1) filtered, idx=5 (prom=3.5) kept
        let mid = detect_peaks(&data, 1, 1.5, None);
        assert_eq!(mid, vec![5]);
        // prom=4: neither survives
        let strict = detect_peaks(&data, 1, 4.0, None);
        assert!(strict.is_empty());
    }

    #[test]
    fn test_detect_peaks_nan_skipped() {
        // NaN at idx=2 disqualifies both neighbours (idx=1, idx=3) from being peaks.
        // idx=4 (val=3, neighbours 2/1) and idx=6 (val=3, neighbours 1/1) form clean peaks.
        let data = vec![3.0, 1.0, f64::NAN, 2.0, 3.0, 1.0, 3.0, 1.0];
        let peaks = detect_peaks(&data, 1, 0.0, None);
        assert_eq!(peaks, vec![4, 6], "idx=4 and idx=6 (unaffected by NaN) should be emitted");
    }

    #[test]
    fn test_detect_peaks_too_short() {
        let data = vec![1.0, 2.0];
        let peaks = detect_peaks(&data, 1, 0.0, None);
        assert!(peaks.is_empty());
    }

    #[test]
    fn test_detect_peaks_width_filter() {
        // Triangular peak with width >= 3: [1, 2, 3, 2, 1]
        // Prominence = 2, half-prom threshold = 2 → 3 samples (idx=1, 2, 3) ≥ 2.
        let data = vec![1.0, 2.0, 3.0, 2.0, 1.0];
        let peaks_wide = detect_peaks(&data, 1, 0.0, Some(3));
        assert_eq!(peaks_wide, vec![2]);
        // Sharper peak with same height but no shoulders — width 1.
        let data_sharp = vec![1.0, 1.0, 3.0, 1.0, 1.0];
        let peaks_strict = detect_peaks(&data_sharp, 1, 0.0, Some(3));
        assert!(peaks_strict.is_empty());
    }

    #[test]
    fn test_detect_peaks_plateau() {
        // Plateau [1, 3, 3, 3, 1] — all three middle points are plateau peaks
        // but only one should be emitted (leftmost by default).
        let data = vec![1.0, 3.0, 3.0, 3.0, 1.0];
        let peaks = detect_peaks(&data, 1, 0.0, None);
        // Scipy behaviour: only leftmost (idx=1) is emitted.
        assert_eq!(peaks, vec![1]);
    }
}
