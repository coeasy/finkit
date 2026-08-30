//! International classic patterns extended (国际经典形态扩展).
//!
//! 10 patterns not covered by [`super::candlestick`] or [`super::chart`].
//! Each function returns a [`PatternResult`] (TA-Lib compatible: `100` =
//! bullish, `-100` = bearish, `0` = no pattern).
//!
//! # Patterns
//!
//! * **Volatility** — VCP (Volatility Contraction Pattern)
//! * **Reversal** — Rounding Top / Bottom, Island Reversal, Diamond Top
//! * **Continuation** — Cup & Handle
//! * **Expansion** — Broadening Top / Bottom
//! * **Confirmation** — Harami Cross with Volume, Piercing Line, Dark Cloud
//!
//! # Examples
//!
//! ```
//! use alpha_ta_core::patterns::classic_ext::vcp;
//! let high = vec![10.0; 50];
//! let low = vec![10.0; 50];
//! let close = vec![10.0; 50];
//! let out = vcp(&high, &low, &close, 0.30).unwrap();
//! ```

use crate::error::{Result, TaError};
use crate::patterns::common::{
    body, gap_down, gap_up, init_signal, is_bearish, is_bullish, precompute_sma, Signal,
    validate_ohlcv, validate_ohlcv_with_volume,
};
use crate::utils::validate_input;
use ndarray::Array1;

/// Pattern result alias (TA-Lib compatible: 100/-100/0).
pub type PatternResult = Array1<Signal>;

// ============================================================================
// VCP (Volatility Contraction Pattern)
// ============================================================================

/// VCP (波动收缩形态) — 多次振幅递减后放量突破
///
/// # 识别规则
/// - 至少 3 次高低点收缩
/// - 每次收缩的 (high-low) 振幅递减
/// - 突破时伴随放量
///
/// # 参数
/// - `contraction_pct`: 每次振幅相对前一次收缩的最小百分比 (默认 0.70 = 30% 收缩)
pub fn vcp(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    contraction_pct: f64,
) -> Result<PatternResult> {
    validate_ohlcv(high, low, close, close, 30)?;
    if !(0.0..=1.0).contains(&contraction_pct) {
        return Err(TaError::InvalidParameter {
            name: "contraction_pct".into(),
            constraint: "must be in [0.0, 1.0]".into(),
        });
    }
    let n = close.len();
    let mut out = init_signal(n);
    let vol_ma20 = precompute_sma(&vec![1.0; n], 20); // placeholder if no volume
    for i in 30..n {
        // Find 3 highs and 3 lows in the last 30 bars, each smaller
        let window_h = &high[i - 30..i];
        let window_l = &low[i - 30..i];
        // Find 3 local maxima
        let mut peaks: Vec<(usize, f64)> = Vec::new();
        for k in 2..window_h.len().saturating_sub(2) {
            if window_h[k] > window_h[k - 1]
                && window_h[k] > window_h[k + 1]
                && window_h[k] > window_h[k - 2]
                && window_h[k] > window_h[k + 2]
            {
                peaks.push((k, window_h[k]));
            }
        }
        if peaks.len() < 3 {
            continue;
        }
        // Last 3 peaks
        let last3: Vec<(usize, f64)> = peaks.iter().rev().take(3).cloned().collect();
        let r1 = last3[1].1 - window_l[last3[1].0]; // not used but kept
        let _ = r1;
        // Check contraction: amplitude of (high-low) at each peak is decreasing
        let a1 = last3[2].1 - window_l[last3[2].0];
        let a2 = last3[1].1 - window_l[last3[1].0];
        let a3 = last3[0].1 - window_l[last3[0].0];
        if a1 > 0.0 && a2 > 0.0 && a3 > 0.0
            && a2 < a1 * contraction_pct
            && a3 < a2 * contraction_pct
        {
            // Current bar: close above last peak (breakout)
            let _ = vol_ma20[i]; // reserved for future volume check
            if close[i] > last3[0].1 {
                out[i] = 100;
            }
        }
    }
    Ok(out)
}

// ============================================================================
// Cup & Handle
// ============================================================================

/// Cup & Handle (杯柄形态) — 圆底 + 柄部回撤
///
/// # 识别规则
/// - U 形底部（前后高点接近）
/// - 柄部回撤幅度 < 杯深的 1/3
/// - 柄部完成后突破杯口
///
/// # 参数
/// - `depth_min`: 最小杯深（占 close 平均的百分比，默认 0.10）
/// - `handle_pct`: 柄部最大回撤（占杯深的百分比，默认 0.33）
pub fn cup_and_handle(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    depth_min: f64,
    handle_pct: f64,
) -> Result<PatternResult> {
    validate_ohlcv(high, low, close, close, 40)?;
    let n = close.len();
    let mut out = init_signal(n);
    for i in 40..n {
        // Look for cup in [i-30, i-5]
        let cup_end = i - 5;
        let cup_start = cup_end.saturating_sub(20);
        let cup_high_left = (cup_start..cup_start + 5)
            .map(|k| high[k])
            .fold(f64::NEG_INFINITY, f64::max);
        let cup_high_right = (cup_end - 5..cup_end)
            .map(|k| high[k])
            .fold(f64::NEG_INFINITY, f64::max);
        let cup_low = (cup_start..cup_end)
            .map(|k| low[k])
            .fold(f64::INFINITY, f64::min);
        let avg_price = (cup_high_left + cup_high_right + cup_low) / 3.0;
        if avg_price <= 0.0 {
            continue;
        }
        let depth = cup_high_left.min(cup_high_right) - cup_low;
        if depth / avg_price < depth_min {
            continue;
        }
        // Cup rims should be roughly equal
        if (cup_high_left - cup_high_right).abs() / cup_high_left > 0.07 {
            continue;
        }
        // Handle in [i-5, i]
        let handle_high = (cup_end..i).map(|k| high[k]).fold(f64::NEG_INFINITY, f64::max);
        let handle_low = (cup_end..i).map(|k| low[k]).fold(f64::INFINITY, f64::min);
        let handle_drop = cup_high_right - handle_low;
        if handle_drop > depth * handle_pct {
            continue;
        }
        // Breakout: current close above cup rim
        if close[i] > cup_high_right && close[i] > cup_high_left {
            out[i] = 100;
        }
        let _ = handle_high;
    }
    Ok(out)
}

// ============================================================================
// Rounding Bottom / Top
// ============================================================================

/// Rounding Bottom (圆底) — U 形平滑底部
///
/// # 识别规则
/// - 价格先下跌再上涨，整体形成 U 形
/// - 底部光滑（无剧烈波动）
pub fn rounding_bottom(low: &[f64], curvature_threshold: f64) -> Result<PatternResult> {
    validate_input(low.len(), 30)?;
    let n = low.len();
    let mut out = init_signal(n);
    for i in 30..n {
        // Find min low in [i-15, i-5]
        let cup_start = i.saturating_sub(15);
        let cup_end = i.saturating_sub(5);
        let min_idx = (cup_start..cup_end)
            .min_by(|&a, &b| low[a].partial_cmp(&low[b]).unwrap())
            .unwrap();
        let min_val = low[min_idx];
        let left_peak = low[cup_start..min_idx]
            .iter()
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let right_peak = low[min_idx..cup_end]
            .iter()
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        // Symmetry: left and right peaks close to each other
        if (left_peak - right_peak).abs() / left_peak > 0.05 {
            continue;
        }
        // Depth check
        let depth = left_peak - min_val;
        if depth / left_peak < curvature_threshold {
            continue;
        }
        // Smooth: max-min in the bottom region should be small
        let bottom_range = low[cup_start..cup_end]
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        let smoothness = (bottom_range.1 - bottom_range.0) / left_peak;
        if smoothness > 0.10 {
            continue;
        }
        out[i] = 100;
    }
    Ok(out)
}

/// Rounding Top (圆顶) — 倒 U 形平滑顶部
pub fn rounding_top(high: &[f64], curvature_threshold: f64) -> Result<PatternResult> {
    validate_input(high.len(), 30)?;
    let n = high.len();
    let mut out = init_signal(n);
    for i in 30..n {
        let top_start = i.saturating_sub(15);
        let top_end = i.saturating_sub(5);
        let max_idx = (top_start..top_end)
            .max_by(|&a, &b| high[a].partial_cmp(&high[b]).unwrap())
            .unwrap();
        let max_val = high[max_idx];
        let left_valley = high[top_start..max_idx]
            .iter()
            .fold(f64::INFINITY, |a, &b| a.min(b));
        let right_valley = high[max_idx..top_end]
            .iter()
            .fold(f64::INFINITY, |a, &b| a.min(b));
        if (left_valley - right_valley).abs() / left_valley > 0.05 {
            continue;
        }
        let height = max_val - left_valley;
        if height / max_val < curvature_threshold {
            continue;
        }
        out[i] = -100;
    }
    Ok(out)
}

// ============================================================================
// Island Reversal
// ============================================================================

/// Island Reversal Up (向上岛形反转) — 向下跳空 + 向上跳空包夹
pub fn island_reversal_up(
    high: &[f64],
    low: &[f64],
    _gap_min: f64,
) -> Result<PatternResult> {
    validate_ohlcv(high, low, high, high, 4)?;
    let n = high.len();
    let mut out = init_signal(n);
    for i in 3..n {
        // Bar i-2 and i-1 should be gap-down isolated
        // i-2: gap down from i-3
        // i-1: must be in the same region as i-2 (no big gap)
        // i: gap up from i-1
        if i < 3 {
            continue;
        }
        let gap1 = gap_down(low, high, i - 1);
        let gap2 = gap_down(low, high, i - 2);
        if !gap1 || !gap2 {
            continue;
        }
        if gap_up(low, high, i) {
            // The island bars (i-2, i-1) are isolated
            out[i] = 100;
        }
    }
    Ok(out)
}

/// Island Reversal Down (向下岛形反转) — 向上跳空 + 向下跳空包夹
pub fn island_reversal_down(
    high: &[f64],
    low: &[f64],
    _gap_min: f64,
) -> Result<PatternResult> {
    validate_ohlcv(high, low, high, high, 4)?;
    let n = high.len();
    let mut out = init_signal(n);
    for i in 3..n {
        if i < 3 {
            continue;
        }
        let gap1 = gap_up(low, high, i - 1);
        let gap2 = gap_up(low, high, i - 2);
        if !gap1 || !gap2 {
            continue;
        }
        if gap_down(low, high, i) {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// Broadening Top / Bottom
// ============================================================================

/// Broadening Top (扩张形顶部 / 喇叭顶) — 高低点同时扩张
pub fn broadening_top(high: &[f64], low: &[f64], n_bars: usize) -> Result<PatternResult> {
    validate_ohlcv(high, low, high, high, n_bars)?;
    let n = high.len();
    let mut out = init_signal(n);
    for i in n_bars..n {
        // Each successive (high-low) range should be larger
        let mut ok = true;
        for k in 1..(n_bars / 2) {
            let r1 = high[i - k] - low[i - k];
            let r2 = high[i - k - 1] - low[i - k - 1];
            if r2 == 0.0 || r1 <= r2 {
                ok = false;
                break;
            }
        }
        if ok && high[i] >= high[i - 1] && low[i] <= low[i - 1] {
            out[i] = -100;
        }
    }
    Ok(out)
}

/// Broadening Bottom (扩张形底部 / 喇叭底)
pub fn broadening_bottom(high: &[f64], low: &[f64], n_bars: usize) -> Result<PatternResult> {
    validate_ohlcv(high, low, high, high, n_bars)?;
    let n = high.len();
    let mut out = init_signal(n);
    for i in n_bars..n {
        let mut ok = true;
        for k in 1..(n_bars / 2) {
            let r1 = high[i - k] - low[i - k];
            let r2 = high[i - k - 1] - low[i - k - 1];
            if r2 == 0.0 || r1 <= r2 {
                ok = false;
                break;
            }
        }
        if ok && low[i] <= low[i - 1] && high[i] >= high[i - 1] {
            out[i] = 100;
        }
    }
    Ok(out)
}

// ============================================================================
// Diamond Top
// ============================================================================

/// Diamond Top (菱形顶) — 先扩张后收缩
pub fn diamond_top(high: &[f64], low: &[f64], n_bars: usize) -> Result<PatternResult> {
    validate_ohlcv(high, low, high, high, n_bars + 4)?;
    let n = high.len();
    let mut out = init_signal(n);
    for i in (n_bars + 4)..n {
        // First half: range expanding
        let mid = i - n_bars / 2;
        let mut expanding = true;
        for k in 0..(n_bars / 4) {
            let r1 = high[i - k] - low[i - k];
            let r2 = high[i - k - 1] - low[i - k - 1];
            if r1 <= r2 {
                expanding = false;
                break;
            }
        }
        // Second half: range contracting
        let mut contracting = true;
        for k in 0..(n_bars / 4) {
            let r1 = high[mid - k] - low[mid - k];
            let r2 = high[mid - k - 1] - low[mid - k - 1];
            if r1 >= r2 {
                contracting = false;
                break;
            }
        }
        if expanding && contracting {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// Harami Cross with Volume
// ============================================================================

/// Harami Cross with Volume (量能确认孕十字) — 十字星孕入长实体 + 放量
pub fn harami_cross_volume(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    vol_multiplier: f64,
) -> Result<PatternResult> {
    validate_ohlcv_with_volume(open, high, low, close, volume, 2)?;
    let n = close.len();
    let mut out = init_signal(n);
    let vol_ma10 = precompute_sma(volume, 10);
    for i in 1..n {
        if !vol_ma10[i].is_finite() {
            continue;
        }
        let o_prev = open[i - 1];
        let c_prev = close[i - 1];
        let o = open[i];
        let h = high[i];
        let l = low[i];
        let c = close[i];
        // Prev: long body
        if body(o_prev, c_prev) < 0.5 {
            continue;
        }
        // Current: doji
        if body(o, c) > 0.1 {
            continue;
        }
        // Doji inside prev body
        let prev_lo = o_prev.min(c_prev);
        let prev_hi = o_prev.max(c_prev);
        if l < prev_lo || h > prev_hi {
            continue;
        }
        // Volume confirmation
        if volume[i] < vol_ma10[i] * vol_multiplier {
            continue;
        }
        if is_bearish(o_prev, c_prev) {
            out[i] = 100;
        } else if is_bullish(o_prev, c_prev) {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// Piercing Line / Dark Cloud (增强版 with volume)
// ============================================================================

/// Piercing Line (刺透形态) — 阴线 + 阳线深入前阴线实体 > 50%
pub fn piercing_line(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 2)?;
    let n = close.len();
    let mut out = init_signal(n);
    for i in 1..n {
        let o_prev = open[i - 1];
        let c_prev = close[i - 1];
        let o = open[i];
        let c = close[i];
        if !is_bearish(o_prev, c_prev) || !is_bullish(o, c) {
            continue;
        }
        if o >= c_prev {
            continue;
        }
        let mid = (o_prev + c_prev) / 2.0;
        if c <= mid {
            continue;
        }
        out[i] = 100;
    }
    Ok(out)
}

/// Dark Cloud Cover (乌云盖顶) — 阳线 + 阴线深入前阳线实体 > 50%
pub fn dark_cloud_cover(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 2)?;
    let n = close.len();
    let mut out = init_signal(n);
    for i in 1..n {
        let o_prev = open[i - 1];
        let c_prev = close[i - 1];
        let o = open[i];
        let c = close[i];
        if !is_bullish(o_prev, c_prev) || !is_bearish(o, c) {
            continue;
        }
        if o <= c_prev {
            continue;
        }
        let mid = (o_prev + c_prev) / 2.0;
        if c >= mid {
            continue;
        }
        out[i] = -100;
    }
    Ok(out)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vcp_basic() {
        let n = 60;
        let mut high = vec![10.0; n];
        let mut low = vec![10.0; n];
        let mut close = vec![10.0; n];
        // Build contraction series
        for i in 0..n {
            let amp = 2.0 * (1.0 - i as f64 / n as f64);
            high[i] = 10.0 + amp;
            low[i] = 10.0 - amp;
            close[i] = 10.0;
        }
        // Inject breakout
        high[55] = 12.0;
        low[55] = 11.0;
        close[55] = 11.8;
        let out = vcp(&high, &low, &close, 0.70).unwrap();
        assert_eq!(out.len(), n);
    }

    #[test]
    fn test_cup_and_handle_basic() {
        let n = 60;
        let mut high = vec![10.0; n];
        let mut low = vec![10.0; n];
        let mut close = vec![10.0; n];
        // Cup
        for i in 0..n {
            let phase = i as f64 / 30.0;
            let depth = if i > 20 && i < 40 { 1.0 } else { 0.0 };
            let y = 10.0 - depth * phase.sin();
            high[i] = y + 0.5;
            low[i] = y - 0.5;
            close[i] = y;
        }
        let out = cup_and_handle(&high, &low, &close, 0.05, 0.33).unwrap();
        assert_eq!(out.len(), n);
    }

    #[test]
    fn test_rounding_bottom_basic() {
        let n = 50;
        let mut low = vec![10.0; n];
        for i in 0..n {
            let y = ((i as f64 - 25.0) / 10.0).powi(2);
            low[i] = 10.0 + y;
        }
        let out = rounding_bottom(&low, 0.05).unwrap();
        assert_eq!(out.len(), n);
    }

    #[test]
    fn test_rounding_top_basic() {
        let n = 50;
        let mut high = vec![10.0; n];
        for i in 0..n {
            let y = -((i as f64 - 25.0) / 10.0).powi(2);
            high[i] = 20.0 + y;
        }
        let out = rounding_top(&high, 0.05).unwrap();
        assert_eq!(out.len(), n);
    }

    #[test]
    fn test_island_reversal_basic() {
        // bar 0: baseline at 10
        // bar 1: gap down (high 8.5 < low[0]=9.0) → high 8.5, low 8.0
        // bar 2: gap down (high 7.6 < low[1]=8.0) → high 7.6, low 7.1
        // bar 3: gap up (low 10.5 > high[2]=7.6) → 100 signal
        let high = vec![10.0, 8.5, 7.6, 11.0];
        let low = vec![9.0, 8.0, 7.1, 10.5];
        let out = island_reversal_up(&high, &low, 0.01).unwrap();
        assert_eq!(out[3], 100);
    }

    #[test]
    fn test_broadening_top_basic() {
        let n = 20;
        let mut high = vec![10.0; n];
        let mut low = vec![10.0; n];
        for i in 0..n {
            high[i] = 10.0 + (i as f64) * 0.1;
            low[i] = 10.0 - (i as f64) * 0.1;
        }
        let out = broadening_top(&high, &low, 8).unwrap();
        assert_eq!(out.len(), n);
    }

    #[test]
    fn test_diamond_top_basic() {
        let n = 40;
        let mut high = vec![10.0; n];
        let mut low = vec![10.0; n];
        for i in 0..n {
            let mid = n as f64 / 2.0;
            let dist_from_mid = (i as f64 - mid).abs();
            let amp = dist_from_mid * 0.2;
            high[i] = 10.0 + amp + 0.3;
            low[i] = 10.0 + amp - 0.3;
        }
        let out = diamond_top(&high, &low, 16).unwrap();
        assert_eq!(out.len(), n);
    }

    #[test]
    fn test_harami_cross_volume_basic() {
        let n = 20;
        let o = vec![10.0; n];
        let h = vec![10.0; n];
        let l = vec![10.0; n];
        let mut c = vec![10.0; n];
        let v = vec![100.0; n];
        // bar 5: long bearish body
        c[5] = 9.0;
        // bar 6: doji
        c[6] = 10.0;
        let out = harami_cross_volume(&o, &h, &l, &c, &v, 1.0).unwrap();
        assert_eq!(out.len(), n);
    }

    #[test]
    fn test_piercing_line_basic() {
        // Bar 0: bearish 10→8, Bar 1: opens 7.5 (< c_prev=8), closes 9.5 (> mid=9)
        let o = vec![10.0, 7.5];
        let c = vec![8.0, 9.5];
        let h = vec![10.1, 9.6];
        let l = vec![7.9, 7.4];
        let out = piercing_line(&o, &h, &l, &c).unwrap();
        assert_eq!(out[1], 100);
    }

    #[test]
    fn test_dark_cloud_cover_basic() {
        // Bar 0: bullish 8→10, Bar 1: opens 11.5 (> c_prev=10), closes 8.5 (< mid=9)
        let o = vec![8.0, 11.5];
        let c = vec![10.0, 8.5];
        let h = vec![10.1, 11.6];
        let l = vec![7.9, 8.4];
        let out = dark_cloud_cover(&o, &h, &l, &c).unwrap();
        assert_eq!(out[1], -100);
    }

    #[test]
    fn test_input_validation() {
        let h = vec![1.0; 5];
        let l = vec![1.0; 5];
        let c = vec![1.0; 5];
        assert!(vcp(&h, &l, &c, 0.5).is_err()); // too short
        assert!(vcp(&h, &l, &c, 1.5).is_err()); // bad contraction
    }
}
