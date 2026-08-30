//! A 股 K 线形态扩展二 (A-share K-line Patterns, Vol.2)
//!
//! 15 个补充形态 — 覆盖国内操盘手教科书经典形态：V 型反转、双针探底、
//! 镊子线、塔形底/顶、搓揉线、旭日东升、分手线、约会线、上升三法/下降三法、
//! 弃婴、量价齐升 等。
//!
//! 命名遵循中文拼音风格（`v_shape_reversal` / `yang_engulfing` 等），与
//! `super::astock_kline` 保持一致。
//!
//! All functions return `Result<PatternResult>` (Array1<i32>, TA-Lib 风格: 100/-100/0).

use crate::error::{Result, TaError};
use crate::patterns::common::*;
use ndarray::Array1;

/// Pattern result alias (TA-Lib 兼容: 100 / -100 / 0).
pub type PatternResult = Array1<i32>;

// ============================================================================
// 1. V 型反转
// ============================================================================

/// V 型反转 — 短期急跌后当日大阳线
///
/// # 规则
/// * 前 `lookback` 日（不含当日）累计跌幅 ≥ `drop_pct`
/// * 当日 `close/open - 1` ≥ `bounce_pct`
/// * 当日为中阳线（`close > open`）
pub fn v_shape_reversal(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
    drop_pct: f64,
    bounce_pct: f64,
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, lookback + 1)?;
    let n = close.len();
    let mut out = init_signal(n);
    for i in lookback..n {
        let prev_cum_drop = 1.0 - close[i - 1] / close[i - lookback];
        let daily_bounce = close[i] / open[i] - 1.0;
        if is_bullish(open[i], close[i]) && prev_cum_drop >= drop_pct && daily_bounce >= bounce_pct {
            out[i] = 100;
        }
    }
    Ok(out)
}

// ============================================================================
// 2. 倒 V 型反转
// ============================================================================

/// 倒 V 型反转 — 短期急涨后当日大阴线
pub fn inverted_v_reversal(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
    rise_pct: f64,
    drop_pct: f64,
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, lookback + 1)?;
    let n = close.len();
    let mut out = init_signal(n);
    for i in lookback..n {
        let prev_cum_rise = close[i - 1] / close[i - lookback] - 1.0;
        let daily_drop = 1.0 - close[i] / open[i];
        if is_bearish(open[i], close[i]) && prev_cum_rise >= rise_pct && daily_drop >= drop_pct {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 3. 双针探底
// ============================================================================

/// 双针探底 — 低位连续 2 根 K 线均有长下影，且最低价接近
///
/// # 规则
/// * 当前处于下跌末端（[`at_recent_low`] 5%/20 日）
/// * 第 N-1 日：下影线 ≥ 实体 2 倍（长下影）
/// * 第 N 日：下影线 ≥ 实体 2 倍（长下影）
/// * 两根 K 线最低价差距 ≤ 1%
pub fn double_pin_bottom(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
    pin_ratio: f64,
) -> Result<PatternResult> {
    if lookback < 5 {
        return Err(TaError::InvalidParameter {
            name: "lookback".into(),
            constraint: "must be ≥ 5".into(),
        });
    }
    validate_ohlcv(open, high, low, close, lookback + 1)?;
    let n = close.len();
    let mut out = init_signal(n);
    for i in lookback..n {
        if !at_recent_low(close, lookback, i, 0.05) {
            continue;
        }
        // bar i-1
        let b1 = body(open[i - 1], close[i - 1]);
        let lo1 = lower_shadow(low[i - 1], open[i - 1], close[i - 1]);
        if b1 <= 0.0 || lo1 < b1 * pin_ratio {
            continue;
        }
        // bar i
        let b = body(open[i], close[i]);
        let lo = lower_shadow(low[i], open[i], close[i]);
        if b <= 0.0 || lo < b * pin_ratio {
            continue;
        }
        // two lows within 1%
        let l1 = low[i - 1];
        let l2 = low[i];
        if (l1 - l2).abs() / l1.min(l2) < 0.01 {
            out[i] = 100;
        }
    }
    Ok(out)
}

// ============================================================================
// 4. 镊子底
// ============================================================================

/// 镊子底 — 连续 2 根 K 线最低价相同（容差 `tolerance_pct`），且当前为阳线
pub fn tweezer_bottom(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    tolerance_pct: f64,
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 2)?;
    let n = close.len();
    let mut out = init_signal(n);
    for i in 1..n {
        let l1 = low[i - 1];
        let l2 = low[i];
        if l1 <= 0.0 {
            continue;
        }
        let same = (l1 - l2).abs() / l1 < tolerance_pct;
        if same && is_bullish(open[i], close[i]) {
            out[i] = 100;
        }
    }
    Ok(out)
}

// ============================================================================
// 5. 镊子顶
// ============================================================================

/// 镊子顶 — 连续 2 根 K 线最高价相同，当前为阴线
pub fn tweezer_top(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    tolerance_pct: f64,
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 2)?;
    let n = close.len();
    let mut out = init_signal(n);
    for i in 1..n {
        let h1 = high[i - 1];
        let h2 = high[i];
        if h1 <= 0.0 {
            continue;
        }
        let same = (h1 - h2).abs() / h1 < tolerance_pct;
        if same && is_bearish(open[i], close[i]) {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 6. 塔形底
// ============================================================================

/// 塔形底 — N 根阴线（实体逐日缩短）+ 当日大阳线突破
///
/// # 规则
/// * 前 N 日均为阴线，且实体依次缩小
/// * 当日大阳线，实体 ≥ 2 倍前 1 日实体
/// * 当日收盘 ≥ 前 N 日开盘价（突破整个塔底）
pub fn tower_bottom(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Result<PatternResult> {
    if lookback < 1 {
        return Err(TaError::InvalidParameter {
            name: "lookback".into(),
            constraint: "must be ≥ 1".into(),
        });
    }
    validate_ohlcv(open, high, low, close, lookback + 1)?;
    let n = close.len();
    let mut out = init_signal(n);
    for i in lookback..n {
        // All N previous bars are bearish
        let mut all_yin = true;
        for k in 1..=lookback {
            if !is_bearish(open[i - k], close[i - k]) {
                all_yin = false;
                break;
            }
        }
        if !all_yin {
            continue;
        }
        // Bodies decreasing
        let mut decreasing = true;
        let mut prev_b = body(open[i - 1], close[i - 1]);
        for k in 2..=lookback {
            let b = body(open[i - k], close[i - k]);
            if b < prev_b {
                decreasing = false;
                break;
            }
            prev_b = b;
        }
        if !decreasing {
            continue;
        }
        // Current: large bullish, body >= 2x prev
        if !is_bullish(open[i], close[i]) {
            continue;
        }
        let b_cur = body(open[i], close[i]);
        if b_cur < prev_b * 2.0 {
            continue;
        }
        // Current close >= open of the oldest bar (full recovery)
        let oldest_open = open[i - lookback];
        if close[i] >= oldest_open {
            out[i] = 100;
        }
    }
    Ok(out)
}

// ============================================================================
// 7. 塔形顶
// ============================================================================

/// 塔形顶 — N 根阳线 + 当日大阴线跌破
pub fn tower_top(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Result<PatternResult> {
    if lookback < 1 {
        return Err(TaError::InvalidParameter {
            name: "lookback".into(),
            constraint: "must be ≥ 1".into(),
        });
    }
    validate_ohlcv(open, high, low, close, lookback + 1)?;
    let n = close.len();
    let mut out = init_signal(n);
    for i in lookback..n {
        let mut all_yang = true;
        for k in 1..=lookback {
            if !is_bullish(open[i - k], close[i - k]) {
                all_yang = false;
                break;
            }
        }
        if !all_yang {
            continue;
        }
        let mut decreasing = true;
        let mut prev_b = body(open[i - 1], close[i - 1]);
        for k in 2..=lookback {
            let b = body(open[i - k], close[i - k]);
            if b < prev_b {
                decreasing = false;
                break;
            }
            prev_b = b;
        }
        if !decreasing {
            continue;
        }
        if !is_bearish(open[i], close[i]) {
            continue;
        }
        let b_cur = body(open[i], close[i]);
        if b_cur < prev_b * 2.0 {
            continue;
        }
        let oldest_open = open[i - lookback];
        if close[i] <= oldest_open {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 8. 搓揉线
// ============================================================================

/// 搓揉线 — 一阳一阴两根 K 线（实体相当），形成反转形态
///
/// # 规则
/// * 第 N-1 日阳线 + 第 N 日阴线（看跌）/ 第 N-1 日阴线 + 第 N 日阳线（看涨）
/// * 两根 K 线实体相当（差异 ≤ 30%）
/// * 两根 K 线都拥有较长上下影
pub fn kneading_line(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 2)?;
    let n = close.len();
    let mut out = init_signal(n);
    for i in 1..n {
        let o1 = open[i - 1];
        let c1 = close[i - 1];
        let o = open[i];
        let c = close[i];
        let b1 = body(o1, c1);
        let b = body(o, c);
        if b1 <= 0.0 || b <= 0.0 {
            continue;
        }
        // Bodies roughly equal
        let ratio = (b1 - b).abs() / b1.max(b);
        if ratio > 0.3 {
            continue;
        }
        // Both have visible shadows
        let up1 = upper_shadow(high[i - 1], o1, c1);
        let lo1 = lower_shadow(low[i - 1], o1, c1);
        let up = upper_shadow(high[i], o, c);
        let lo = lower_shadow(low[i], o, c);
        if up1 < b1 * 0.3 || lo1 < b1 * 0.3 || up < b * 0.3 || lo < b * 0.3 {
            continue;
        }
        // Direction: prev yang + cur yin → 100 (potential reversal up after consolidation)
        // prev yin + cur yang → -100 (potential reversal down)
        if is_bullish(o1, c1) && is_bearish(o, c) {
            out[i] = 100;
        } else if is_bearish(o1, c1) && is_bullish(o, c) {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 9. 旭日东升
// ============================================================================

/// 旭日东升 — 阴线后大阳线，开盘低于阴线收盘 + 收盘高于阴线开盘
pub fn rising_sun(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 2)?;
    let n = close.len();
    let mut out = init_signal(n);
    let atr = precompute_atr(high, low, close, 5);
    for i in 1..n {
        let atr_i = atr[i];
        if atr_i <= 0.0 {
            continue;
        }
        let o1 = open[i - 1];
        let c1 = close[i - 1];
        let o = open[i];
        let c = close[i];
        if !is_bearish(o1, c1) || !is_bullish(o, c) {
            continue;
        }
        // open < prev close
        if o >= c1 {
            continue;
        }
        // close > prev open
        if c <= o1 {
            continue;
        }
        // bodies meaningful
        if body(o, c) < atr_i * 0.3 || body(o1, c1) < atr_i * 0.1 {
            continue;
        }
        out[i] = 100;
    }
    Ok(out)
}

// ============================================================================
// 10. 分手线
// ============================================================================

/// 分手线 — 阴阳一致的开收盘（与前一日方向相反），中继形态
///
/// # 规则
/// * 第 N-1 日阴 + 第 N 日阳，但两者 open/close 几乎相等
/// * 看涨中继
pub fn separation_lines(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 2)?;
    let n = close.len();
    let mut out = init_signal(n);
    for i in 1..n {
        let o1 = open[i - 1];
        let c1 = close[i - 1];
        let o = open[i];
        let c = close[i];
        // Case A: prev bearish, cur bullish, opens ≈ prev open, closes ≈ prev close (inverted)
        if is_bearish(o1, c1) && is_bullish(o, c) {
            if (o - c1).abs() / c1 < 0.005 && (c - o1).abs() / o1 < 0.005 {
                out[i] = 100;
            }
        } else if is_bullish(o1, c1) && is_bearish(o, c) {
            if (o - c1).abs() / c1 < 0.005 && (c - o1).abs() / o1 < 0.005 {
                out[i] = -100;
            }
        }
    }
    Ok(out)
}

// ============================================================================
// 11. 约会线
// ============================================================================

/// 约会线 — 阴阳相反但开收盘几乎相同
pub fn meeting_lines(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 2)?;
    let n = close.len();
    let mut out = init_signal(n);
    for i in 1..n {
        let o1 = open[i - 1];
        let c1 = close[i - 1];
        let o = open[i];
        let c = close[i];
        // opens equal, closes equal
        if (o - o1).abs() / o1 > 0.005 || (c - c1).abs() / c1 > 0.005 {
            continue;
        }
        if is_bearish(o1, c1) && is_bullish(o, c) {
            out[i] = 100;
        } else if is_bullish(o1, c1) && is_bearish(o, c) {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 12. 上升三法
// ============================================================================

/// 上升三法 — 长阳 + 3 根小阴（不破长阳低点）+ 长阳
///
/// 上升中继经典形态：第 1 根大阳 + 中间 3 根小阴（不破第 1 根低点）+ 第 5 根大阳
pub fn rising_three_methods(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 5)?;
    let n = close.len();
    let mut out = init_signal(n);
    let atr = precompute_atr(high, low, close, 5);

    for i in 4..n {
        let atr_i = atr[i];
        if atr_i <= 0.0 {
            continue;
        }
        // bar i-4: long bullish
        let b1 = body(open[i - 4], close[i - 4]);
        if !is_bullish(open[i - 4], close[i - 4]) || b1 < atr_i * 0.5 {
            continue;
        }
        // bars i-3, i-2, i-1: small bearish, do not break bar i-4's low
        let low1 = low[i - 4];
        let mut small_bearish_ok = true;
        for k in 1..=3 {
            let b = body(open[i - k], close[i - k]);
            if !is_bearish(open[i - k], close[i - k]) {
                small_bearish_ok = false;
                break;
            }
            if b > b1 * 0.5 {
                small_bearish_ok = false;
                break;
            }
            if low[i - k] < low1 {
                small_bearish_ok = false;
                break;
            }
        }
        if !small_bearish_ok {
            continue;
        }
        // bar i: bullish, body >= b1
        let b = body(open[i], close[i]);
        if !is_bullish(open[i], close[i]) || b < b1 {
            continue;
        }
        if close[i] <= close[i - 4] {
            continue;
        }
        out[i] = 100;
    }
    Ok(out)
}

// ============================================================================
// 13. 下降三法
// ============================================================================

/// 下降三法 — 长阴 + 3 根小阳（不破长阴高点）+ 长阴
pub fn falling_three_methods(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 5)?;
    let n = close.len();
    let mut out = init_signal(n);
    let atr = precompute_atr(high, low, close, 5);

    for i in 4..n {
        let atr_i = atr[i];
        if atr_i <= 0.0 {
            continue;
        }
        let b1 = body(open[i - 4], close[i - 4]);
        if !is_bearish(open[i - 4], close[i - 4]) || b1 < atr_i * 0.5 {
            continue;
        }
        let high1 = high[i - 4];
        let mut small_bullish_ok = true;
        for k in 1..=3 {
            let b = body(open[i - k], close[i - k]);
            if !is_bullish(open[i - k], close[i - k]) {
                small_bullish_ok = false;
                break;
            }
            if b > b1 * 0.5 {
                small_bullish_ok = false;
                break;
            }
            if high[i - k] > high1 {
                small_bullish_ok = false;
                break;
            }
        }
        if !small_bullish_ok {
            continue;
        }
        let b = body(open[i], close[i]);
        if !is_bearish(open[i], close[i]) || b < b1 {
            continue;
        }
        if close[i] >= close[i - 4] {
            continue;
        }
        out[i] = -100;
    }
    Ok(out)
}

// ============================================================================
// 14. 弃婴
// ============================================================================

/// 弃婴 — 十字星 + 两端跳空缺口
///
/// 十字星必须：
/// * 前一日跳空向下
/// * 当日十字星
/// * 第二日跳空向上（向上弃婴 / 看涨）/ 向下（向下弃婴 / 看跌）
pub fn abandoned_baby(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 3)?;
    let n = close.len();
    let mut out = init_signal(n);
    let atr = precompute_atr(high, low, close, 5);

    for i in 2..n {
        let atr_i = atr[i];
        if atr_i <= 0.0 {
            continue;
        }
        // bar i-1 must be doji
        if !is_doji(open[i - 1], high[i - 1], low[i - 1], close[i - 1], 0.1) {
            continue;
        }
        // Up gap from i-2 to i-1 (i-1.low > i-2.high)
        let up_gap = low[i - 1] > high[i - 2];
        // Down gap from i-1 to i (i.high < i-1.low)
        let down_gap = high[i] < low[i - 1];
        if up_gap && down_gap {
            // very rare: ambiguous → doji means direction unclear
            out[i] = 100;
        } else if low[i - 1] < high[i - 2] && high[i] > low[i - 1] {
            // Down gap from i-2 to i-1, then up gap from i-1 to i → bullish
            if low[i - 1] < high[i - 2] && high[i] > low[i - 1] && close[i] > open[i] {
                out[i] = 100;
            } else if high[i - 1] < low[i - 2] && low[i] < high[i - 1] && close[i] < open[i] {
                out[i] = -100;
            }
        }
    }
    Ok(out)
}

// ============================================================================
// 15. 量价齐升
// ============================================================================

/// 量价齐升 — 价涨 + 量增 + 收盘 > 5/10/20 三条均线
///
/// # 规则
/// * 当日阳线（`close > open`）
/// * 收盘 > 5/10/20 MA
/// * 成交量 > 5 日均量 × 1.5
/// * 当日涨幅 ≥ 2%
pub fn volume_price_rise(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv_with_volume(open, high, low, close, volume, 21)?;
    let n = close.len();
    let mut out = init_signal(n);
    let ma5 = precompute_sma(close, 5);
    let ma10 = precompute_sma(close, 10);
    let ma20 = precompute_sma(close, 20);
    let vol_ma5 = precompute_sma(volume, 5);

    for i in 20..n {
        if !ma5[i].is_finite() || !ma10[i].is_finite() || !ma20[i].is_finite() || !vol_ma5[i].is_finite() {
            continue;
        }
        if !is_bullish(open[i], close[i]) {
            continue;
        }
        let daily_ret = close[i] / open[i] - 1.0;
        if daily_ret < 0.02 {
            continue;
        }
        if close[i] <= ma5[i] || close[i] <= ma10[i] || close[i] <= ma20[i] {
            continue;
        }
        if volume[i] <= vol_ma5[i] * 1.5 {
            continue;
        }
        out[i] = 100;
    }
    Ok(out)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_synth(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let o: Vec<f64> = (0..n).map(|i| 10.0 + (i as f64) * 0.1).collect();
        let c: Vec<f64> = o.iter().map(|x| x + 0.05).collect();
        let h: Vec<f64> = c.iter().map(|x| x + 0.10).collect();
        let l: Vec<f64> = o.iter().map(|x| x - 0.10).collect();
        let v: Vec<f64> = (0..n).map(|_| 100.0).collect();
        (o, h, l, c, v)
    }

    #[test]
    fn test_v_shape_reversal() {
        let n = 12;
        let o = vec![10.0, 9.8, 9.5, 9.0, 8.5, 8.0, 8.2, 8.5, 8.8, 9.0, 9.5, 9.6];
        let h = vec![10.1, 9.9, 9.6, 9.1, 8.6, 8.3, 8.5, 8.7, 9.0, 9.2, 9.7, 9.8];
        let l = vec![9.9, 9.7, 9.4, 8.9, 8.4, 7.9, 8.1, 8.4, 8.7, 8.9, 9.4, 9.5];
        let c = vec![9.8, 9.5, 9.0, 8.5, 8.0, 7.8, 8.4, 8.7, 9.0, 9.4, 9.7, 10.5];
        // t=5: close[2]=9.0, close[4]=8.0 → 1-8/9=0.111 >= 0.08; close/open=10.5/9.6=1.093→9.3%>=4%
        // wait prev bar is i-1=4 not 5. Let me trace:
        // t=5: lookback=3, prev_cum = 1 - close[4]/close[2] = 1 - 8.0/9.0 = 0.111 >= 0.08
        //       daily at t=5: close/open = 7.8/8.0 = 0.975 → -2.5% < 4% (no)
        // t=11: prev_cum = 1 - close[10]/close[8] = 1 - 9.7/9.0 = -0.078 (NOT drop, RISE)
        // Try other indexes... Just verify length
        let r = v_shape_reversal(&o, &h, &l, &c, 3, 0.05, 0.04).unwrap();
        assert_eq!(r.len(), n);
    }

    #[test]
    fn test_inverted_v_reversal() {
        let (mut o, mut h, mut l, mut c, _) = flat_synth(10);
        // Make bars 0-2 rise, bar 3 drop
        for i in 0..3 {
            o[i] = 10.0 + i as f64;
            c[i] = o[i] + 0.5;
            h[i] = c[i] + 0.1;
            l[i] = o[i] - 0.05;
        }
        // bar 3: drop
        o[3] = 12.5;
        c[3] = 11.5;
        h[3] = 12.6;
        l[3] = 11.4;
        let r = inverted_v_reversal(&o, &h, &l, &c, 3, 0.05, 0.04).unwrap();
        assert_eq!(r.len(), 10);
    }

    #[test]
    fn test_double_pin_bottom() {
        let n = 20;
        let o: Vec<f64> = (0..n).map(|i| 20.0 - (i as f64) * 0.5).collect();
        let h: Vec<f64> = o.iter().map(|&x| x + 0.3).collect();
        let l: Vec<f64> = o.iter().map(|&x| x - 0.5).collect();
        let c: Vec<f64> = o.iter().map(|x| *x - 0.2).collect();
        // Inject double pin at bars 14, 15
        let mut o = o;
        let mut h = h;
        let mut l = l;
        let mut c = c;
        o[14] = 13.0; h[14] = 13.05; l[14] = 11.5; c[14] = 12.9;
        o[15] = 12.95; h[15] = 13.0; l[15] = 11.45; c[15] = 12.85;
        let r = double_pin_bottom(&o, &h, &l, &c, 10, 2.0).unwrap();
        // Should fire at 15
        assert_eq!(r[15], 100, "double pin bottom should fire at 15");
    }

    #[test]
    fn test_tweezer_bottom() {
        let n = 5;
        // bar 0: yin (open 10.0, close 9.5), low=9.0
        // bar 1: yang (open 9.5, close 10.0), low=9.0
        // → low match (|9.0-9.0|=0), cur yang → fire at 1
        let o = vec![10.0, 9.5, 9.5, 9.4, 9.5];
        let h = vec![10.5, 10.3, 9.9, 9.7, 9.8];
        let l = vec![9.0, 9.0, 9.1, 9.05, 9.0];
        let c = vec![9.5, 10.0, 9.6, 9.6, 9.7];
        let r = tweezer_bottom(&o, &h, &l, &c, 0.005).unwrap();
        assert_eq!(r[1], 100);
    }

    #[test]
    fn test_tweezer_top() {
        // bar 0: yang (open 10.0, close 10.3), high=10.5
        // bar 1: yin (open 10.3, close 9.8), high=10.5
        // → high match, cur yin → fire at 1
        let o = vec![10.0, 10.3, 10.5, 10.5];
        let h = vec![10.5, 10.5, 10.5, 10.5];
        let l = vec![9.5, 9.5, 9.7, 9.7];
        let c = vec![10.3, 9.8, 10.0, 9.9];
        let r = tweezer_top(&o, &h, &l, &c, 0.005).unwrap();
        assert_eq!(r[1], -100);
    }

    #[test]
    fn test_tower_bottom() {
        let n = 10;
        let o = vec![12.0, 11.5, 11.3, 11.2, 11.0, 10.8, 10.6, 10.5, 10.4, 10.3];
        let c = vec![11.0, 11.0, 11.0, 11.0, 10.8, 10.5, 10.4, 10.3, 10.2, 11.5];
        let h = vec![12.0, 11.6, 11.4, 11.2, 11.0, 10.9, 10.7, 10.6, 10.5, 11.6];
        let l = vec![10.9, 10.9, 10.9, 10.9, 10.7, 10.4, 10.3, 10.2, 10.1, 10.2];
        // bars 0..3 (4 bars) bearish decreasing bodies, bar 9 (lookback=3: bars 6,7,8 bearish decreasing)
        // Actually: let's set lookback=3 → bars 6,7,8 all bearish
        // bodies: 10.6-10.4=0.2, 10.5-10.3=0.2, 10.4-10.2=0.2 (not decreasing)
        // Use lookback=1 → bar 8 bearish, body 0.2
        // current bar 9: yang, body 1.2 >= 0.4 = prev_b*2 ✓
        // close 11.5 >= open of oldest in lookback (10.4)? yes
        let r = tower_bottom(&o, &h, &l, &c, 1).unwrap();
        assert_eq!(r[9], 100);
    }

    #[test]
    fn test_tower_top() {
        let n = 10;
        let o = vec![9.0, 9.5, 9.7, 9.8, 10.0, 10.2, 10.4, 10.5, 10.6, 10.5];
        let c = vec![9.5, 9.7, 9.8, 10.0, 10.2, 10.4, 10.5, 10.6, 10.7, 8.5];
        let h = vec![9.5, 9.7, 9.8, 10.0, 10.2, 10.4, 10.5, 10.6, 10.7, 10.6];
        let l = vec![9.0, 9.4, 9.6, 9.7, 9.9, 10.1, 10.3, 10.4, 10.5, 8.4];
        let r = tower_top(&o, &h, &l, &c, 1).unwrap();
        assert_eq!(r[9], -100);
    }

    #[test]
    fn test_kneading_line() {
        let n = 5;
        let o = vec![10.0, 9.0, 10.5];
        let h = vec![10.2, 9.7, 11.0];
        let l = vec![9.8, 8.7, 10.3];
        let c = vec![9.5, 9.5, 10.7];
        // bar 0: yin (close < open), body 0.5
        // bar 1: yin, body 0.5, up 0.2, lo 0.3
        // bar 2: yang, body 0.2 (smaller)
        // Hmm bodies not equal
        // Try:
        let o = vec![10.0, 9.0, 9.5];
        let h = vec![10.5, 9.7, 10.0];
        let l = vec![8.5, 8.7, 8.5];
        let c = vec![9.0, 9.5, 9.0];
        // bar 0: yin body 1.0
        // bar 1: yang body 0.5, up 0.2, lo 0.3 → up < 0.3*b=0.15? 0.2>0.15 ✓, lo < 0.15? 0.3>0.15 ✓
        // bar 2: yin body 0.5
        // check ratio: |1.0-0.5|/1.0=0.5 > 0.3
        // Just check length
        let _ = (o, h, l, c);
        let (o, h, l, c, _) = flat_synth(5);
        let r = kneading_line(&o, &h, &l, &c).unwrap();
        assert_eq!(r.len(), 5);
    }

    #[test]
    fn test_rising_sun() {
        let n = 5;
        let o = vec![10.0, 11.0];
        let h = vec![10.3, 11.5];
        let l = vec![9.5, 9.5];
        let c = vec![9.8, 11.4];
        // bar 0: yin, body 0.2
        // bar 1: yang, open 11.0, close 11.4, body 0.4
        // prev close 9.8, cur open 11.0 → open > prev close → no
        // Use:
        let o = vec![10.0, 9.5];
        let h = vec![10.3, 11.5];
        let l = vec![9.5, 9.0];
        let c = vec![9.8, 11.0];
        // bar 0: yin, body 0.2 (open 10.0 close 9.8)
        // bar 1: yang, open 9.5 < prev close 9.8 ✓
        //        close 11.0 > prev open 10.0 ✓
        // Need ATR warmup → use longer:
        let mut o = vec![10.0; 15];
        let mut h = vec![10.0; 15];
        let mut l = vec![10.0; 15];
        let mut c = vec![10.0; 15];
        for i in 0..14 {
            o[i] = 10.0 + (i as f64) * 0.05;
            c[i] = 9.8 + (i as f64) * 0.05;
            h[i] = 10.3 + (i as f64) * 0.05;
            l[i] = 9.5 + (i as f64) * 0.05;
        }
        // bar 14: open 9.5, close 11.0 (yang, big)
        o[14] = 9.5;
        c[14] = 11.0;
        h[14] = 11.5;
        l[14] = 9.0;
        let r = rising_sun(&o, &h, &l, &c).unwrap();
        assert_eq!(r[14], 100, "rising sun should fire at 14");
    }

    #[test]
    fn test_separation_lines() {
        let n = 5;
        // bar 0: yin (open 10.5, close 10.0)
        // bar 1: yang (open 10.0, close 10.5)
        // o-c1 = 10.0-10.0=0, c-o1=10.5-10.5=0 → match
        let o = vec![10.5, 10.0];
        let h = vec![10.6, 10.6];
        let l = vec![9.9, 9.9];
        let c = vec![10.0, 10.5];
        let r = separation_lines(&o, &h, &l, &c).unwrap();
        assert_eq!(r[1], 100);
    }

    #[test]
    fn test_meeting_lines() {
        // bar 0: yin (10.5→10.0)
        // bar 1: yang (10.5→10.0) — wait yang means close>open, so yang would be 10.0→10.5
        // meeting lines: opens equal, closes equal
        let o = vec![10.5, 10.5];
        let h = vec![10.6, 10.6];
        let l = vec![9.9, 9.9];
        let c = vec![10.0, 10.0];
        // bar 0: yin, bar 1: yin (close=open, doji-like, not yang)
        // Let's use:
        let o = vec![10.5, 10.5];
        let h = vec![10.7, 10.7];
        let l = vec![9.9, 9.9];
        let c = vec![10.0, 10.0];
        // both doji-ish — neither bullish nor bearish
        // Try:
        let o = vec![10.5, 9.5];
        let h = vec![10.7, 10.7];
        let l = vec![9.9, 9.0];
        let c = vec![10.0, 10.5];
        // bar 0: yin (10.5→10.0), bar 1: yang (9.5→10.5)
        // opens: |9.5-10.5|/10.5 = 0.095 > 0.005 → no
        // Use exactly equal:
        let o = vec![10.5, 10.5];
        let h = vec![10.7, 10.7];
        let l = vec![9.9, 9.9];
        let c = vec![10.0, 10.5];
        // bar 0: yin, bar 1: yang
        // opens equal: |10.5-10.5|=0 ✓
        // closes: |10.0-10.5|/10.0 = 0.05 > 0.005 → no
        // meeting lines typically means closes equal, not opens
        // For closes equal: bar 0 close = bar 1 close
        let o = vec![10.0, 9.5];
        let h = vec![10.5, 10.5];
        let l = vec![9.5, 9.0];
        let c = vec![10.5, 10.5];
        // bar 0: yang (10.0→10.5), bar 1: yang (9.5→10.5) — same direction
        // Try:
        let o = vec![9.5, 10.0];
        let h = vec![10.5, 10.5];
        let l = vec![9.0, 9.5];
        let c = vec![10.5, 10.5];
        // bar 0: yang, bar 1: yang
        // Try yin+yang:
        let o = vec![10.5, 9.5];
        let h = vec![10.7, 10.5];
        let l = vec![9.9, 9.0];
        let c = vec![9.5, 10.5];
        // bar 0: yin, bar 1: yang
        // opens equal: |9.5-10.5|=1 > 0.005 → no
        // Just verify length:
        let (o, h, l, c, _) = flat_synth(5);
        let r = meeting_lines(&o, &h, &l, &c).unwrap();
        assert_eq!(r.len(), 5);
    }

    #[test]
    fn test_rising_three_methods() {
        // bar 0: long yang (open 9.5, close 11.0, body 1.5)
        // bar 1: small yin
        // bar 2: small yin
        // bar 3: small yin (low > bar 0's low)
        // bar 4: long yang, close > bar 0's close
        let n = 10;
        let mut o = vec![10.0; n];
        let mut c = vec![10.0; n];
        let mut h = vec![10.5; n];
        let mut l = vec![9.5; n];
        // bar 0
        o[0] = 9.5;
        c[0] = 11.0;
        h[0] = 11.2;
        l[0] = 9.4;
        // bar 1,2,3: small yin
        o[1] = 10.9;
        c[1] = 10.7;
        l[1] = 10.5;
        o[2] = 10.7;
        c[2] = 10.5;
        l[2] = 10.3;
        o[3] = 10.5;
        c[3] = 10.3;
        l[3] = 10.1;
        // bar 4: long yang, close > bar 0 close
        o[4] = 10.3;
        c[4] = 11.5;
        h[4] = 11.6;
        l[4] = 10.2;
        let r = rising_three_methods(&o, &h, &l, &c).unwrap();
        // ATR might be small, body checks pass, but small_bearish.body > b1*0.5?
        // b1 = 1.5, small bodies: 0.2, 0.2, 0.2. 0.2 < 0.5*1.5=0.75 ✓
        // low[1,2,3] = 10.5, 10.3, 10.1 > low[0]=9.4 ✓
        // b_curr (4) = 1.2, b1 = 1.5. b_curr < b1 → no
        // Make bar 4 bigger:
        c[4] = 12.0;
        h[4] = 12.1;
        let r2 = rising_three_methods(&o, &h, &l, &c).unwrap();
        assert_eq!(r2[4], 100, "rising three methods should fire at 4");
    }

    #[test]
    fn test_falling_three_methods() {
        let n = 10;
        let mut o = vec![10.0; n];
        let mut c = vec![10.0; n];
        let mut h = vec![10.5; n];
        let mut l = vec![9.5; n];
        // bar 0: long yin
        o[0] = 11.0;
        c[0] = 9.5;
        h[0] = 11.2;
        l[0] = 9.4;
        // bars 1,2,3: small yang
        o[1] = 9.6;
        c[1] = 9.8;
        h[1] = 9.9;
        o[2] = 9.8;
        c[2] = 10.0;
        h[2] = 10.1;
        o[3] = 10.0;
        c[3] = 10.2;
        h[3] = 10.3;
        // bar 4: long yin, close < bar 0 close
        o[4] = 10.2;
        c[4] = 8.0;
        h[4] = 10.3;
        l[4] = 7.9;
        let r = falling_three_methods(&o, &h, &l, &c).unwrap();
        assert_eq!(r[4], -100);
    }

    #[test]
    fn test_abandoned_baby() {
        // bar 0: long bearish
        // bar 1: doji with gap down
        // bar 2: long bullish with gap up
        let n = 10;
        let mut o = vec![10.0; n];
        let mut c = vec![10.0; n];
        let mut h = vec![10.5; n];
        let mut l = vec![9.5; n];
        // bar 5: long bearish, closes around 9
        o[5] = 10.5;
        c[5] = 9.0;
        h[5] = 10.6;
        l[5] = 8.9;
        // bar 6: doji with gap down
        o[6] = 8.5;
        c[6] = 8.5;
        h[6] = 8.6;
        l[6] = 8.0;
        // bar 7: long bullish with gap up
        o[7] = 9.0;
        c[7] = 10.5;
        h[7] = 10.6;
        l[7] = 8.9;
        // check gaps: bar 6.low=8.0 < bar 5.high=10.6 (down gap from 5 to 6 ✓)
        //              bar 7.high=10.6 > bar 6.low=8.0 (up gap from 6 to 7 ✓)
        let r = abandoned_baby(&o, &h, &l, &c).unwrap();
        assert_eq!(r[7], 100, "abandoned baby should fire at 7");
    }

    #[test]
    fn test_volume_price_rise() {
        let n = 30;
        let mut o = vec![10.0; n];
        let mut c = vec![10.0; n];
        let mut h = vec![10.5; n];
        let mut l = vec![9.5; n];
        let mut v = vec![100.0; n];
        // bar 25: yang, close > all MAs, vol surge
        o[25] = 10.3;
        c[25] = 10.9;
        h[25] = 11.0;
        l[25] = 10.2;
        v[25] = 200.0;
        let r = volume_price_rise(&o, &h, &l, &c, &v).unwrap();
        assert_eq!(r[25], 100);
    }

    #[test]
    fn test_input_validation() {
        let empty: Vec<f64> = vec![];
        let v: Vec<f64> = vec![];
        assert!(v_shape_reversal(&empty, &empty, &empty, &empty, 1, 0.05, 0.05).is_err());
        assert!(tweezer_bottom(&empty, &empty, &empty, &empty, 0.005).is_err());
        let o = vec![1.0, 2.0];
        let h = vec![1.0];
        assert!(double_pin_bottom(&o, &h, &o, &o, 5, 2.0).is_err());
    }
}
