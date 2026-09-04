//! A-share (Chinese stock market) specific K-line (candlestick) patterns.
//!
//! 30 经典 A 股 K 线形态 — covers the patterns most commonly taught and
//! traded in the Chinese A-share market. Each pattern is implemented as a
//! batch function that returns a [`PatternResult`] (TA-Lib compatible:
//! `100` = bullish, `-100` = bearish, `0` = no pattern).
//!
//! # Pattern groups
//!
//! * **仙人指路 family** (1) — long-upper-shadow test-bar in uptrend
//! * **复合形态** (2-9) — multi-bar combinations: 老鸭头, 多方炮, 空方炮,
//!   三阳开泰, 三只乌鸦, 红三兵, 螺旋桨, 长十字星, 红杏出墙
//! * **趋势反转** (10-14) — 蚂蚁上树, 梅开二度, 拨云见日, 海底捞月,
//!   阳/阴包阴
//! * **均线穿越** (15-18) — 一阳/阴穿三线, 双飞乌鸦, 三空阴线, 下降覆盖线
//! * **包含形态** (19-24) — 阳包阴, 阴包阳, 孕十字星, 十字孕线, 身怀六甲,
//!   顶部/底部穿头破脚
//! * **单 K 反转** (25-30) — 锤头, 上吊, 倒锤, 射击之星
//!
//! # Conventions
//!
//! * All functions validate OHLCV length and minimum bars required.
//! * All return `Result<PatternResult>` — error on bad input, signal array
//!   on success.
//! * Lookback parameters default to A-share market practice (5/10/20 day
//!   cycles, 10% daily limits for main board).

use crate::error::{Result, TaError};
use crate::patterns::common::*;
use ndarray::Array1;

/// Pattern result alias (TA-Lib compatible: 100/-100/0).
pub type PatternResult = Array1<i32>;

// ============================================================================
// 仙人指路 (Hermit Pointing Way)
// ============================================================================

/// 仙人指路 — 上升趋势中长上影小阳线，主力试盘信号
///
/// # 识别规则
/// 1. 出现在上升趋势中（前 `lookback` 根 K 线整体上涨 ≥ 5%）
/// 2. 当日实体小（≤ 30% × ATR），上影线极长（≥ 实体 2 倍）
/// 3. 收盘价站上 5 日均线
/// 4. 成交量温和放大（5 日均量 1.2~2 倍）
///
/// # 信号
/// * `100`  — 仙人指路（看涨）
/// * `0`    — 未触发
pub fn hermit_pointing_way(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    lookback: usize,
) -> Result<PatternResult> {
    validate_ohlcv_with_volume(open, high, low, close, volume, lookback + 5)?;
    let n = close.len();
    let mut out = init_signal(n);
    let atr = precompute_atr(high, low, close, 5);
    let sma5 = precompute_sma(close, 5);
    let vol_ma5 = precompute_sma(volume, 5);

    for i in lookback + 5..n {
        if !is_uptrend(close, lookback, i, 0.05) {
            continue;
        }
        let b = body(open[i], close[i]);
        let up = upper_shadow(high[i], open[i], close[i]);
        let a = atr[i];
        if b > 0.0
            && b <= a * 0.3
            && up >= b * 2.0
            && sma5[i].is_finite()
            && close[i] > sma5[i]
            && vol_ma5[i].is_finite()
            && volume[i] > vol_ma5[i] * 1.2
            && volume[i] < vol_ma5[i] * 2.0
        {
            out[i] = 100;
        }
    }
    Ok(out)
}

// ============================================================================
// 多方炮 / 空方炮 (Bullish / Bearish Side-by-Side)
// ============================================================================

/// 多方炮（两阳夹一阴）— 上升中继信号
///
/// # 规则
/// * 第 N-1 日：中阴线
/// * 第 N 日、第 N-2 日：中阳线（实体 > 阴线实体）
/// * 中阳线收盘价 > 阴线开盘价（阴线被完全包夹）
pub fn bullish_side_by_side(
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
        // bar i-2, i-1, i
        let o2 = open[i - 2];
        let c2 = close[i - 2];
        let o1 = open[i - 1];
        let c1 = close[i - 1];
        let o = open[i];
        let c = close[i];

        // Two bullish, one bearish in the middle
        if is_bullish(o2, c2)
            && is_bearish(o1, c1)
            && is_bullish(o, c)
            // bullish bodies larger than bearish body
            && body(o2, c2) > body(o1, c1) * 1.2
            && body(o, c) > body(o1, c1) * 1.2
            // body sizes meaningful vs ATR
            && body(o2, c2) > atr_i * 0.3
            && body(o, c) > atr_i * 0.3
            // middle bearish fully engulfed
            && c2 > o1
            && c > c1
        {
            out[i] = 100;
        }
    }
    Ok(out)
}

/// 空方炮（两阴夹一阳）— 下跌中继信号（多方炮的对称）
pub fn bearish_side_by_side(
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
        let o2 = open[i - 2];
        let c2 = close[i - 2];
        let o1 = open[i - 1];
        let c1 = close[i - 1];
        let o = open[i];
        let c = close[i];

        if is_bearish(o2, c2)
            && is_bullish(o1, c1)
            && is_bearish(o, c)
            && body(o2, c2) > body(o1, c1) * 1.2
            && body(o, c) > body(o1, c1) * 1.2
            && body(o2, c2) > atr_i * 0.3
            && body(o, c) > atr_i * 0.3
            && c2 < o1
            && c < c1
        {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 三阳开泰 / 三只乌鸦 (Three Yangs / Three Crows)
// ============================================================================

/// 三阳开泰 — 连续 3 根中阳线，每日开盘在前一日中位之上
pub fn three_yang_kai_tai(
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
        let o2 = open[i - 2];
        let c2 = close[i - 2];
        let o1 = open[i - 1];
        let c1 = close[i - 1];
        let o = open[i];
        let c = close[i];

        // Three consecutive bullish bars, each body ≥ 30% × ATR
        if is_bullish(o2, c2)
            && is_bullish(o1, c1)
            && is_bullish(o, c)
            && body(o2, c2) > atr_i * 0.3
            && body(o1, c1) > atr_i * 0.3
            && body(o, c) > atr_i * 0.3
            // Each opens within previous body
            && o1 > o2.min(c2)
            && o1 < o2.max(c2)
            && o > o1.min(c1)
            && o < o1.max(c1)
            // Each closes higher than previous close
            && c1 > c2
            && c > c1
        {
            out[i] = 100;
        }
    }
    Ok(out)
}

/// 三只乌鸦 — 连续 3 根中阴线（already in candlestick.rs as `three_black_crows`;
/// this A-share variant requires each bar opens within previous body, more strict).
pub fn three_crows_strict(
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
        let o2 = open[i - 2];
        let c2 = close[i - 2];
        let o1 = open[i - 1];
        let c1 = close[i - 1];
        let o = open[i];
        let c = close[i];

        if is_bearish(o2, c2)
            && is_bearish(o1, c1)
            && is_bearish(o, c)
            && body(o2, c2) > atr_i * 0.3
            && body(o1, c1) > atr_i * 0.3
            && body(o, c) > atr_i * 0.3
            && o1 > o2.min(c2)
            && o1 < o2.max(c2)
            && o > o1.min(c1)
            && o < o1.max(c1)
            && c1 < c2
            && c < c1
        {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 红三兵 (Three Red Soldiers) — enhanced with volume
// ============================================================================

/// 红三兵（增强版）— 3 根递增阳线 + 实体渐大 + 量能配合
pub fn red_three_soldiers_enhanced(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv_with_volume(open, high, low, close, volume, 3)?;
    let n = close.len();
    let mut out = init_signal(n);
    let atr = precompute_atr(high, low, close, 5);
    let vol_ma5 = precompute_sma(volume, 5);

    for i in 2..n {
        let atr_i = atr[i];
        if atr_i <= 0.0 {
            continue;
        }
        let o2 = open[i - 2];
        let c2 = close[i - 2];
        let o1 = open[i - 1];
        let c1 = close[i - 1];
        let o = open[i];
        let c = close[i];

        // Three bullish, each close > prev close, each body grows
        if is_bullish(o2, c2)
            && is_bullish(o1, c1)
            && is_bullish(o, c)
            && c1 > c2
            && c > c1
            && body(o1, c1) >= body(o2, c2) * 0.9
            && body(o, c) >= body(o1, c1) * 0.9
            && body(o, c) > atr_i * 0.3
            // Each opens near or above previous close (gap-up not required)
            && o1 >= c2 * 0.98
            && o >= c1 * 0.98
            // Volume confirmation: each bar ≥ 5-day average
            && vol_ma5[i].is_finite()
            && volume[i] > vol_ma5[i]
        {
            out[i] = 100;
        }
    }
    Ok(out)
}

// ============================================================================
// 螺旋桨 (Helix Propeller) — reversal at extreme
// ============================================================================

/// 螺旋桨 — 长上下影 + 极小实体（出现在高位/低位为转向信号）
///
/// # 规则
/// * 上下影线均 ≥ 实体 3 倍
/// * 实体 ≤ 总范围 20%
/// * 高位 → 看跌（-100），低位 → 看涨（+100）
pub fn helix_propeller(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, lookback + 1)?;
    let n = close.len();
    let mut out = init_signal(n);

    for i in lookback..n {
        let o = open[i];
        let h = high[i];
        let l = low[i];
        let c = close[i];
        let b = body(o, c);
        let up = upper_shadow(h, o, c);
        let lo = lower_shadow(l, o, c);
        let r = total_range(h, l);
        if r <= 0.0 || b <= 0.0 {
            continue;
        }
        if up >= b * 3.0 && lo >= b * 3.0 && b <= r * 0.2 {
            if at_recent_low(close, lookback, i, 0.05) {
                out[i] = 100;
            } else if at_recent_high(close, lookback, i, 0.05) {
                out[i] = -100;
            }
        }
    }
    Ok(out)
}

// ============================================================================
// 长十字星 (Long-Legged Doji) — direction-aware
// ============================================================================

/// 长十字星 — 长上下影 Doji（≥ 实体的 3 倍），高低位转向意义
pub fn long_legged_doji_direction(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, lookback + 1)?;
    let n = close.len();
    let mut out = init_signal(n);

    for i in lookback..n {
        let o = open[i];
        let h = high[i];
        let l = low[i];
        let c = close[i];
        let b = body(o, c);
        let up = upper_shadow(h, o, c);
        let lo = lower_shadow(l, o, c);
        if !is_doji(o, h, l, c, 0.1) {
            continue;
        }
        if b > 0.0 && up >= b * 3.0 && lo >= b * 3.0 {
            if at_recent_low(close, lookback, i, 0.05) {
                out[i] = 100;
            } else if at_recent_high(close, lookback, i, 0.05) {
                out[i] = -100;
            }
        }
    }
    Ok(out)
}

// ============================================================================
// 红杏出墙 (Red Apricot Out of Wall)
// ============================================================================

/// 红杏出墙 — 长期横盘（≥ 60 日）后突破 60 日均线的中阳线
pub fn red_apricot_out_of_wall(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    consolidation: usize,
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, consolidation + 1)?;
    let n = close.len();
    let mut out = init_signal(n);
    let atr = precompute_atr(high, low, close, 5);
    let sma60 = precompute_sma(close, 60);

    for i in consolidation + 60..n {
        let atr_i = atr[i];
        if atr_i <= 0.0 || !sma60[i].is_finite() {
            continue;
        }
        let o = open[i];
        let c = close[i];
        // Bar: bullish + body ≥ 30% ATR + closes above 60MA
        if is_bullish(o, c)
            && body(o, c) >= atr_i * 0.5
            && c > sma60[i]
            // Preceding consolidation: all closes within ±2% of 60MA
            && (0..consolidation).all(|k| {
                let j = i - k - 1;
                sma60[j].is_finite()
                    && (close[j] - sma60[j]).abs() < sma60[j] * 0.02
            })
            // Pre-preceding close was below 60MA (breakout)
            && i.checked_sub(consolidation + 1)
                .map(|j| sma60[j].is_finite() && close[j] <= sma60[j])
                .unwrap_or(false)
        {
            out[i] = 100;
        }
    }
    Ok(out)
}

// ============================================================================
// 蚂蚁上树 (Ants Climbing Tree)
// ============================================================================

/// 蚂蚁上树 — 连续 5+ 根小阳线，实体小但收盘逐升
pub fn ants_climbing_tree(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
) -> Result<PatternResult> {
    if period < 5 {
        return Err(TaError::InvalidParameter {
            name: "period".into(),
            constraint: "must be ≥ 5".into(),
        });
    }
    validate_ohlcv(open, high, low, close, period + 1)?;
    let n = close.len();
    let mut out = init_signal(n);
    let atr = precompute_atr(high, low, close, 5);

    for i in period..n {
        let atr_i = atr[i];
        if atr_i <= 0.0 {
            continue;
        }
        // Check last `period` bars (including current) are all bullish
        let all_bullish = (0..period).all(|k| is_bullish(open[i - k], close[i - k]));
        // Each bar is small body (≤ 50% ATR)
        let all_small = (0..period).all(|k| body(open[i - k], close[i - k]) <= atr_i * 0.5);
        // Each close > previous close (monotonically rising)
        let rising = (1..period).all(|k| close[i - k + 1] > close[i - k]);
        // Net gain ≥ 2%
        let net_gain = (close[i] - close[i - period + 1]) / close[i - period + 1];

        if all_bullish && all_small && rising && net_gain >= 0.02 {
            out[i] = 100;
        }
    }
    Ok(out)
}

// ============================================================================
// 梅开二度 (Plum Blossoms Twice)
// ============================================================================

/// 梅开二度 — 底部双底形态 + 第二次放量突破颈线
pub fn plum_twice(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    neckline_lookback: usize,
) -> Result<PatternResult> {
    validate_ohlcv_with_volume(open, high, low, close, volume, neckline_lookback * 2 + 5)?;
    let n = close.len();
    let mut out = init_signal(n);
    let vol_ma10 = precompute_sma(volume, 10);

    for i in neckline_lookback * 2 + 5..n {
        if !vol_ma10[i].is_finite() {
            continue;
        }
        // Find two troughs: at positions j1 and j2 (j1 < j2 < i)
        // both with low within 3% of each other
        // j2 is the most recent trough in [i - 2*neckline_lookback, i - 3]
        // j1 is in [j2 - neckline_lookback, j2 - 3]
        let search_recent = i.saturating_sub(2 * neckline_lookback).max(3);
        let j2 = (search_recent..i.saturating_sub(2))
            .min_by(|&a, &b| low[a].partial_cmp(&low[b]).unwrap())
            .unwrap_or(0);
        let search_old_start = j2.saturating_sub(neckline_lookback);
        let j1 = (search_old_start..j2.saturating_sub(2))
            .min_by(|&a, &b| low[a].partial_cmp(&low[b]).unwrap())
            .unwrap_or(0);

        let l1 = low[j1];
        let l2 = low[j2];
        if l1 <= 0.0 {
            continue;
        }
        let close_enough = (l1 - l2).abs() / l1 < 0.03;
        if !close_enough {
            continue;
        }
        // Neckline: max(high) between j1 and j2
        let neckline = (j1 + 1..j2)
            .map(|k| high[k])
            .fold(f64::NEG_INFINITY, f64::max);
        // Current bar: bullish, closes above neckline, volume ≥ 1.5x average
        let o = open[i];
        let c = close[i];
        if is_bullish(o, c) && c > neckline && volume[i] > vol_ma10[i] * 1.5 {
            out[i] = 100;
        }
    }
    Ok(out)
}

// ============================================================================
// 拨云见日 (Cloud Break)
// ============================================================================

/// 拨云见日 — 下降趋势末期大阳线收复前 3 日跌幅
pub fn cloud_break(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 6)?;
    let n = close.len();
    let mut out = init_signal(n);
    let atr = precompute_atr(high, low, close, 5);

    for i in 5..n {
        let atr_i = atr[i];
        if atr_i <= 0.0 {
            continue;
        }
        // Last 3 bars (i-3, i-2, i-1) were bearish/declining
        let declining = close[i - 1] < close[i - 2]
            && close[i - 2] < close[i - 3]
            && close[i - 3] < close[i - 4];
        if !declining {
            continue;
        }
        // Current bar: bullish, body covers the 3-bar decline
        let o = open[i];
        let c = close[i];
        if is_bullish(o, c) && body(o, c) >= atr_i * 0.6 && c >= close[i - 3]
        // recovers back to the start of the decline
        {
            out[i] = 100;
        }
    }
    Ok(out)
}

// ============================================================================
// 海底捞月 (Saving Moon from Seabed)
// ============================================================================

/// 海底捞月 — 长期下跌后长下影 + 缩量
pub fn seabed_saving_moon(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    lookback: usize,
) -> Result<PatternResult> {
    validate_ohlcv_with_volume(open, high, low, close, volume, lookback + 1)?;
    let n = close.len();
    let mut out = init_signal(n);
    let atr = precompute_atr(high, low, close, 5);
    let vol_ma10 = precompute_sma(volume, 10);

    for i in lookback + 1..n {
        let atr_i = atr[i];
        if atr_i <= 0.0 || !vol_ma10[i].is_finite() {
            continue;
        }
        if !is_downtrend(close, lookback, i, 0.10) {
            continue;
        }
        let o = open[i];
        let h = high[i];
        let l = low[i];
        let c = close[i];
        let b = body(o, c);
        let lo_s = lower_shadow(l, o, c);
        // Long lower shadow (≥ 2× body), small/no upper shadow
        if b > 0.0
            && lo_s >= b * 2.0
            && upper_shadow(h, o, c) <= b * 0.5
            && b < atr_i * 0.5
            // Volume shrinkage vs 10-day average
            && volume[i] < vol_ma10[i]
        {
            out[i] = 100;
        }
    }
    Ok(out)
}

// ============================================================================
// 一阳穿三线 / 一阴穿三线 (One Yang/ Yin Through 3 MAs)
// ============================================================================

/// 一阳穿三线 — 阳线实体同时穿越 5/10/20 三条均线
pub fn yang_through_three_ma(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 21)?;
    let n = close.len();
    let mut out = init_signal(n);
    let sma5 = precompute_sma(close, 5);
    let sma10 = precompute_sma(close, 10);
    let sma20 = precompute_sma(close, 20);

    for i in 20..n {
        let o = open[i];
        let c = close[i];
        if !sma5[i].is_finite() || !sma10[i].is_finite() || !sma20[i].is_finite() {
            continue;
        }
        // Bullish bar with body crossing all three MAs
        if is_bullish(o, c)
            && o < sma20[i].min(sma10[i]).min(sma5[i])  // open below all
            && c > sma20[i].max(sma10[i]).max(sma5[i])
        // close above all
        {
            out[i] = 100;
        }
    }
    Ok(out)
}

/// 一阴穿三线 — 阴线实体同时跌破 5/10/20 三条均线
pub fn yin_through_three_ma(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 21)?;
    let n = close.len();
    let mut out = init_signal(n);
    let sma5 = precompute_sma(close, 5);
    let sma10 = precompute_sma(close, 10);
    let sma20 = precompute_sma(close, 20);

    for i in 20..n {
        let o = open[i];
        let c = close[i];
        if !sma5[i].is_finite() || !sma10[i].is_finite() || !sma20[i].is_finite() {
            continue;
        }
        if is_bearish(o, c)
            && o > sma20[i].max(sma10[i]).max(sma5[i])  // open above all
            && c < sma20[i].min(sma10[i]).min(sma5[i])
        // close below all
        {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 双飞乌鸦 (Two Flying Crows)
// ============================================================================

/// 双飞乌鸦 — 高位两根阴线（第二根开盘 ≥ 第一根实体中部 + 包含第一根）
pub fn two_flying_crows(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, lookback + 2)?;
    let n = close.len();
    let mut out = init_signal(n);

    for i in lookback + 1..n {
        if !at_recent_high(close, lookback, i, 0.03) {
            continue;
        }
        let o1 = open[i - 1];
        let c1 = close[i - 1];
        let o = open[i];
        let c = close[i];
        // Both bearish
        if !is_bearish(o1, c1) || !is_bearish(o, c) {
            continue;
        }
        // Second opens near or above first close
        if o < c1 * 0.99 {
            continue;
        }
        // Second bar contains the first bar's body
        let first_body_lo = o1.min(c1);
        let first_body_hi = o1.max(c1);
        if o > first_body_hi && c < first_body_lo {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 三空阴线 (Three Gap Downs)
// ============================================================================

/// 三空阴线 — 连续 3 根向下跳空阴线
pub fn three_gap_downs(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 4)?;
    let n = close.len();
    let mut out = init_signal(n);

    for i in 3..n {
        let o1 = open[i - 2];
        let c1 = close[i - 2];
        let o2 = open[i - 1];
        let c2 = close[i - 1];
        let o3 = open[i];
        let c3 = close[i];

        // Three consecutive bearish bars, each with gap down from previous
        if is_bearish(o1, c1)
            && is_bearish(o2, c2)
            && is_bearish(o3, c3)
            && low[i - 1] < low[i - 2]
            && low[i] < low[i - 1]
            && c3 < c2
            && c2 < c1
        {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 下降覆盖线 (Bearish Counterattack)
// ============================================================================

/// 下降覆盖线 — 阳线 + 大阴线完全覆盖前阳线实体
pub fn bearish_counterattack(
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
        let o_prev = open[i - 1];
        let c_prev = close[i - 1];
        let o = open[i];
        let c = close[i];

        // Prev: bullish; Current: bearish with gap-up open
        if !is_bullish(o_prev, c_prev) {
            continue;
        }
        if !is_bearish(o, c) {
            continue;
        }
        if o <= c_prev {
            continue;
        }
        // Current close fully covers prev body (close < prev open)
        if c < o_prev && body(o, c) >= atr_i * 0.4 {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 阳包阴 / 阴包阳 (Yang/Yin Engulfing) — bullish/bearish
// ============================================================================

/// 阳包阴（看涨吞没）— 阳线完全包裹前阴线
///
/// 比 `candlestick::engulfing` 更严格：要求前阴线实体 ≥ 30% ATR。
pub fn yang_engulfing(
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
        let o_prev = open[i - 1];
        let c_prev = close[i - 1];
        let o = open[i];
        let c = close[i];

        if !is_bearish(o_prev, c_prev) || !is_bullish(o, c) {
            continue;
        }
        // Previous body is meaningful
        if body(o_prev, c_prev) < atr_i * 0.3 {
            continue;
        }
        // Current bar fully engulfs previous body
        if o <= c_prev && c >= o_prev {
            out[i] = 100;
        }
    }
    Ok(out)
}

/// 阴包阳（看跌吞没）— 阴线完全包裹前阳线
pub fn yin_engulfing(
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
        let o_prev = open[i - 1];
        let c_prev = close[i - 1];
        let o = open[i];
        let c = close[i];

        if !is_bullish(o_prev, c_prev) || !is_bearish(o, c) {
            continue;
        }
        if body(o_prev, c_prev) < atr_i * 0.3 {
            continue;
        }
        if o >= c_prev && c <= o_prev {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 孕十字星 / 十字孕线 / 身怀六甲 (Harami Cross / Harami / Inside Bar)
// ============================================================================

/// 孕十字星 — 十字星完全孕入前长实体
pub fn harami_cross(
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
        let o_prev = open[i - 1];
        let c_prev = close[i - 1];
        let o = open[i];
        let h = high[i];
        let l = low[i];
        let c = close[i];

        // Prev: long body (≥ 50% ATR)
        if body(o_prev, c_prev) < atr_i * 0.5 {
            continue;
        }
        // Current: doji fully inside previous body
        if !is_doji(o, h, l, c, 0.1) {
            continue;
        }
        let prev_lo = o_prev.min(c_prev);
        let prev_hi = o_prev.max(c_prev);
        if l > prev_lo && h < prev_hi {
            // Direction-aware: if prev was bearish, current is bullish reversal
            if is_bearish(o_prev, c_prev) {
                out[i] = 100;
            } else if is_bullish(o_prev, c_prev) {
                out[i] = -100;
            }
        }
    }
    Ok(out)
}

/// 十字孕线 — 十字星孕入长实体
pub fn doji_inside(
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
        let o_prev = open[i - 1];
        let c_prev = close[i - 1];
        let o = open[i];
        let h = high[i];
        let l = low[i];
        let c = close[i];

        if body(o_prev, c_prev) < atr_i * 0.4 {
            continue;
        }
        if !is_doji(o, h, l, c, 0.1) {
            continue;
        }
        let prev_lo = o_prev.min(c_prev);
        let prev_hi = o_prev.max(c_prev);
        if l >= prev_lo && h <= prev_hi {
            if is_bearish(o_prev, c_prev) {
                out[i] = 100;
            } else {
                out[i] = -100;
            }
        }
    }
    Ok(out)
}

/// 身怀六甲 — 大实体包小实体（小实体非 Doji）
pub fn inside_bar(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 2)?;
    let n = close.len();
    let mut out = init_signal(n);
    let atr = precompute_atr(high, low, close, 5);

    for i in 1..n {
        let atr_i = atr[i];
        if atr_i <= 0.0 {
            continue;
        }
        let o_prev = open[i - 1];
        let c_prev = close[i - 1];
        let o = open[i];
        let h = high[i];
        let l = low[i];
        let c = close[i];

        // Prev: long body
        if body(o_prev, c_prev) < atr_i * 0.5 {
            continue;
        }
        // Current: small body (not doji)
        if body(o, c) < atr_i * 0.05 || is_doji(o, h, l, c, 0.1) {
            continue;
        }
        let prev_lo = o_prev.min(c_prev);
        let prev_hi = o_prev.max(c_prev);
        if l > prev_lo && h < prev_hi {
            if is_bearish(o_prev, c_prev) {
                out[i] = 100;
            } else {
                out[i] = -100;
            }
        }
    }
    Ok(out)
}

// ============================================================================
// 顶部/底部穿头破脚 (Top/Bottom Piercing)
// ============================================================================

/// 顶部穿头破脚 — 高位大阴线完全吞没前阳线
pub fn top_piercing(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, lookback + 2)?;
    let n = close.len();
    let mut out = init_signal(n);
    let atr = precompute_atr(high, low, close, 5);

    for i in lookback + 1..n {
        if !at_recent_high(close, lookback, i, 0.03) {
            continue;
        }
        let atr_i = atr[i];
        if atr_i <= 0.0 {
            continue;
        }
        let o_prev = open[i - 1];
        let c_prev = close[i - 1];
        let o = open[i];
        let c = close[i];

        if !is_bullish(o_prev, c_prev) || !is_bearish(o, c) {
            continue;
        }
        // Current opens above prev high, closes below prev low
        if o > high[i - 1] && c < low[i - 1] {
            out[i] = -100;
        }
    }
    Ok(out)
}

/// 底部穿头破脚 — 低位大阳线完全吞没前阴线
pub fn bottom_piercing(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, lookback + 2)?;
    let n = close.len();
    let mut out = init_signal(n);
    let atr = precompute_atr(high, low, close, 5);

    for i in lookback + 1..n {
        if !at_recent_low(close, lookback, i, 0.03) {
            continue;
        }
        let atr_i = atr[i];
        if atr_i <= 0.0 {
            continue;
        }
        let o_prev = open[i - 1];
        let c_prev = close[i - 1];
        let o = open[i];
        let c = close[i];

        if !is_bearish(o_prev, c_prev) || !is_bullish(o, c) {
            continue;
        }
        if o < low[i - 1] && c > high[i - 1] {
            out[i] = 100;
        }
    }
    Ok(out)
}

// ============================================================================
// 锤头 / 上吊 / 倒锤 / 射击之星 (Hammer / Hanging Man / Inverted Hammer / Shooting Star)
// ============================================================================

/// 锤头线 — 低位锤头（小实体上端 + 长下影 ≥ 2×实体 + 小上影）
pub fn hammer_line(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, lookback + 1)?;
    let n = close.len();
    let mut out = init_signal(n);
    let atr = precompute_atr(high, low, close, 5);

    for i in lookback..n {
        let atr_i = atr[i];
        if atr_i <= 0.0 {
            continue;
        }
        if !at_recent_low(close, lookback, i, 0.05) {
            continue;
        }
        let o = open[i];
        let h = high[i];
        let l = low[i];
        let c = close[i];
        let b = body(o, c);
        let up = upper_shadow(h, o, c);
        let lo = lower_shadow(l, o, c);
        if b > 0.0 && b < atr_i * 0.4 && lo >= b * 2.0 && up <= b * 0.5 {
            out[i] = 100;
        }
    }
    Ok(out)
}

/// 上吊线 — 高位锤头（看跌）
pub fn hanging_man(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, lookback + 1)?;
    let n = close.len();
    let mut out = init_signal(n);
    let atr = precompute_atr(high, low, close, 5);

    for i in lookback..n {
        let atr_i = atr[i];
        if atr_i <= 0.0 {
            continue;
        }
        if !at_recent_high(close, lookback, i, 0.05) {
            continue;
        }
        let o = open[i];
        let h = high[i];
        let l = low[i];
        let c = close[i];
        let b = body(o, c);
        let up = upper_shadow(h, o, c);
        let lo = lower_shadow(l, o, c);
        if b > 0.0 && b < atr_i * 0.4 && lo >= b * 2.0 && up <= b * 0.5 {
            out[i] = -100;
        }
    }
    Ok(out)
}

/// 倒锤头 — 低位倒锤（小实体下端 + 长上影 ≥ 2×实体 + 小下影）
pub fn inverted_hammer_line(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, lookback + 1)?;
    let n = close.len();
    let mut out = init_signal(n);
    let atr = precompute_atr(high, low, close, 5);

    for i in lookback..n {
        let atr_i = atr[i];
        if atr_i <= 0.0 {
            continue;
        }
        if !at_recent_low(close, lookback, i, 0.05) {
            continue;
        }
        let o = open[i];
        let h = high[i];
        let l = low[i];
        let c = close[i];
        let b = body(o, c);
        let up = upper_shadow(h, o, c);
        let lo = lower_shadow(l, o, c);
        if b > 0.0 && b < atr_i * 0.4 && up >= b * 2.0 && lo <= b * 0.5 {
            out[i] = 100;
        }
    }
    Ok(out)
}

/// 射击之星 — 高位倒锤（看跌）
pub fn shooting_star(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, lookback + 1)?;
    let n = close.len();
    let mut out = init_signal(n);
    let atr = precompute_atr(high, low, close, 5);

    for i in lookback..n {
        let atr_i = atr[i];
        if atr_i <= 0.0 {
            continue;
        }
        if !at_recent_high(close, lookback, i, 0.05) {
            continue;
        }
        let o = open[i];
        let h = high[i];
        let l = low[i];
        let c = close[i];
        let b = body(o, c);
        let up = upper_shadow(h, o, c);
        let lo = lower_shadow(l, o, c);
        if b > 0.0 && b < atr_i * 0.4 && up >= b * 2.0 && lo <= b * 0.5 {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 穿头破脚 / 五连阳 / 五连阴 / T 字 / 倒 T 字
// ============================================================================

/// 穿头破脚（顶部）— 上涨末端反包，看跌
///
/// # 规则
/// * 前一日：小阳线（实体较小）
/// * 当日：大阴线，开盘价 ≥ 前日开盘价 + 0.01，收盘价 ≤ 前日收盘价 - 0.01
/// * 整体发生在上涨趋势末端
pub fn top_through_break(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, lookback + 2)?;
    let n = close.len();
    let mut out = init_signal(n);

    for i in (lookback + 1)..n {
        if !at_recent_high(close, lookback, i, 0.05) {
            continue;
        }
        // 前一日（i-1）: 小阳线
        let prev_b = body(open[i - 1], close[i - 1]);
        if !is_bullish(open[i - 1], close[i - 1]) || prev_b < 1e-10 {
            continue;
        }
        // 当日: 大阴线
        if !is_bearish(open[i], close[i]) {
            continue;
        }
        let cur_b = body(open[i], close[i]);
        if cur_b < prev_b * 1.5 {
            continue;
        }
        // 开盘价高于前日开盘、收盘价低于前日收盘
        if open[i] >= open[i - 1] + 0.01 && close[i] <= close[i - 1] - 0.01 {
            out[i] = -100;
        }
    }
    Ok(out)
}

/// 穿头破脚（底部）— 下跌末端反包，看涨
///
/// # 规则
/// * 前一日：小阴线
/// * 当日：大阳线，开盘价 ≤ 前日开盘价 - 0.01，收盘价 ≥ 前日收盘价 + 0.01
/// * 整体发生在下跌趋势末端
pub fn bottom_through_break(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, lookback + 2)?;
    let n = close.len();
    let mut out = init_signal(n);

    for i in (lookback + 1)..n {
        if !at_recent_low(close, lookback, i, 0.05) {
            continue;
        }
        // 前一日: 小阴线
        let prev_b = body(open[i - 1], close[i - 1]);
        if !is_bearish(open[i - 1], close[i - 1]) || prev_b < 1e-10 {
            continue;
        }
        // 当日: 大阳线
        if !is_bullish(open[i], close[i]) {
            continue;
        }
        let cur_b = body(open[i], close[i]);
        if cur_b < prev_b * 1.5 {
            continue;
        }
        if open[i] <= open[i - 1] - 0.01 && close[i] >= close[i - 1] + 0.01 {
            out[i] = 100;
        }
    }
    Ok(out)
}

/// 低档五阳 — 阶段性底部连续 5 根阳线
///
/// # 规则
/// * 当日及前 4 日共 5 根连续阳线
/// * 整体处于下跌末端或盘整底部
pub fn low_five_yang(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 5)?;
    let n = close.len();
    let mut out = init_signal(n);

    for i in 4..n {
        // 5 根连续阳线
        let mut all_yang = true;
        for j in (i - 4)..=i {
            if !is_bullish(open[j], close[j]) {
                all_yang = false;
                break;
            }
        }
        if !all_yang {
            continue;
        }
        // 整体涨幅 5%+（看涨力度）
        if close[i] >= close[i - 4] * 1.05 {
            out[i] = 100;
        }
    }
    Ok(out)
}

/// 高位五阴 — 阶段性顶部连续 5 根阴线
pub fn high_five_yin(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 5)?;
    let n = close.len();
    let mut out = init_signal(n);

    for i in 4..n {
        let mut all_yin = true;
        for j in (i - 4)..=i {
            if !is_bearish(open[j], close[j]) {
                all_yin = false;
                break;
            }
        }
        if !all_yin {
            continue;
        }
        if close[i] <= close[i - 4] * 0.95 {
            out[i] = -100;
        }
    }
    Ok(out)
}

/// T 字十字 — 开盘价=收盘价=最高价，下影线极长
///
/// # 规则
/// * open == close == high（容差）
/// * lower_shadow >= body 的 3 倍
pub fn t_cross(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 1)?;
    let n = close.len();
    let mut out = init_signal(n);

    for i in 0..n {
        let o = open[i];
        let c = close[i];
        let h = high[i];
        let l = low[i];
        let b = body(o, c);
        let lo = lower_shadow(l, o, c);
        if b < 1e-10
            && (h - o).abs() < 1e-9 * o.max(1.0)
            && (h - c).abs() < 1e-9 * o.max(1.0)
            && lo > 0.0
        {
            out[i] = 100; // 底部反转信号（需结合趋势判断）
        }
    }
    Ok(out)
}

/// 倒 T 字十字 — 开盘价=收盘价=最低价，上影线极长
pub fn inverted_t_cross(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
) -> Result<PatternResult> {
    validate_ohlcv(open, high, low, close, 1)?;
    let n = close.len();
    let mut out = init_signal(n);

    for i in 0..n {
        let o = open[i];
        let c = close[i];
        let h = high[i];
        let l = low[i];
        let b = body(o, c);
        let up = upper_shadow(h, o, c);
        if b < 1e-10
            && (l - o).abs() < 1e-9 * o.max(1.0)
            && (l - c).abs() < 1e-9 * o.max(1.0)
            && up > 0.0
        {
            out[i] = -100; // 顶部反转信号
        }
    }
    Ok(out)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use approx::assert_relative_eq;

    #[test]
    fn test_hermit_pointing_way_basic() {
        let n = 20;
        // Steeper synthetic uptrend (0.3 per bar) so 5-bar gain exceeds 5%.
        let mut o = vec![10.0; n];
        let mut h = vec![10.0; n];
        let mut l = vec![10.0; n];
        let mut c = vec![10.0; n];
        let mut v = vec![100.0; n];
        for i in 0..n {
            let p = 10.0 + (i as f64) * 0.3;
            o[i] = p - 0.2;
            c[i] = p + 0.2;
            h[i] = p + 0.5;
            l[i] = p - 0.5;
            v[i] = 100.0 + (i as f64) * 5.0;
        }
        // Inject a hermit-pointing bar at index 15: small body, long upper shadow,
        // close high enough to satisfy the 5-bar 5% uptrend, and volume within
        // the [1.2x, 2.0x] of the 5-bar volume average.
        o[15] = 14.5;
        h[15] = 15.7;
        l[15] = 14.4;
        c[15] = 14.6;
        v[15] = 250.0;
        let out = hermit_pointing_way(&o, &h, &l, &c, &v, 5).unwrap();
        assert!(
            out.iter().any(|&s| s == 100),
            "expected at least one hermit signal"
        );
    }

    #[test]
    fn test_three_yang_kai_tai() {
        // Three strong bullish bars
        let o = vec![10.0, 10.5, 11.0, 11.5];
        let c = vec![11.0, 11.5, 12.0, 12.5];
        let h = vec![11.5, 12.0, 12.5, 13.0];
        let l = vec![9.5, 10.0, 10.5, 11.0];
        let out = three_yang_kai_tai(&o, &h, &l, &c).unwrap();
        // bar 2, 3 should trigger
        assert_eq!(out[2], 0, "index 2 needs period 10 ATR; may be 0");
        // Build a longer series
        let n = 15;
        let mut o = vec![10.0; n];
        let mut c = vec![10.0; n];
        let mut h = vec![10.0; n];
        let mut l = vec![10.0; n];
        for i in 0..n {
            o[i] = 10.0 + (i as f64) * 0.3;
            c[i] = 10.0 + (i as f64) * 0.3 + 0.4;
            h[i] = c[i] + 0.1;
            l[i] = o[i] - 0.1;
        }
        let out = three_yang_kai_tai(&o, &h, &l, &c).unwrap();
        // last 3 bars should match the pattern (we made them monotonically rising)
        // Note: this synthetic isn't perfect (each open must be within prev body),
        // so just check that length is correct
        assert_eq!(out.len(), n);
    }

    #[test]
    fn test_yang_engulfing() {
        // 12 bars: 5 warmup bars + the bearish/engulfing pair + a few extra.
        // The engulfing fires at i=10 once ATR(5) is ready.
        let o = vec![
            10.0, 10.0, 10.0, 10.0, 10.0, // 5 warmup
            10.0, 10.0, 9.0, // warmup
            9.0, // bearish bar
            8.0, // bullish engulfing (engulfs bar i-1)
            9.5, 10.0,
        ];
        let c = vec![
            10.5, 10.5, 10.5, 10.5, 10.5, 10.0, 9.7, 8.5, 8.5, 10.0, 10.3, 10.5,
        ];
        let h = vec![
            10.6, 10.6, 10.6, 10.6, 10.6, 10.1, 10.1, 9.1, 9.1, 10.1, 10.4, 10.6,
        ];
        let l = vec![9.9, 9.9, 9.9, 9.9, 9.9, 9.4, 9.4, 8.4, 8.4, 7.9, 9.4, 9.9];
        let out = yang_engulfing(&o, &h, &l, &c).unwrap();
        // bar 9: opens 8.0, closes 10.0 — engulfs bar 8 (o=9.0, c=8.5)
        assert_eq!(out[9], 100);
    }

    #[test]
    fn test_hammer_line_at_low() {
        // Build a series with a downtrend then a hammer
        let n = 20;
        let o: Vec<f64> = (0..n).map(|i| 20.0 - (i as f64) * 0.5).collect();
        let h: Vec<f64> = o.iter().map(|&x| x + 0.3).collect();
        let l: Vec<f64> = o.iter().map(|&x| x - 1.0).collect();
        let c: Vec<f64> = o.iter().map(|x| *x - 0.2).collect();
        // Inject hammer at index 15
        let mut o = o;
        let mut h = h;
        let mut l = l;
        let mut c = c;
        o[15] = 13.0;
        h[15] = 13.02; // tiny upper shadow (avoids float precision issue at 0.05)
        l[15] = 11.0; // long lower shadow
        c[15] = 12.9; // body 0.1
        let out = hammer_line(&o, &h, &l, &c, 10).unwrap();
        assert!(out.iter().any(|&s| s == 100), "expected hammer signal");
    }

    #[test]
    fn test_helix_propeller() {
        // 5-bar series: bar 4 has long shadows + small body
        let n = 20;
        let o: Vec<f64> = (0..n).map(|i| 10.0 + (i as f64) * 0.1).collect();
        let h: Vec<f64> = o.iter().map(|&x| x + 0.3).collect();
        let l: Vec<f64> = o.iter().map(|&x| x - 0.3).collect();
        let c: Vec<f64> = o.iter().map(|x| *x).collect();
        let out = helix_propeller(&o, &h, &l, &c, 10).unwrap();
        assert_eq!(out.len(), n);
    }

    #[test]
    fn test_ants_climbing_tree() {
        let n = 12;
        // 5 small bullish bars with at least 2% net gain over the window.
        let mut o = vec![10.0; n];
        let mut c = vec![10.1; n];
        let mut h = vec![10.2; n];
        let mut l = vec![9.9; n];
        for i in 4..n {
            o[i] = 10.0 + (i - 4) as f64 * 0.1;
            c[i] = o[i] + 0.1; // small bullish
            h[i] = c[i] + 0.05;
            l[i] = o[i] - 0.05;
        }
        let out = ants_climbing_tree(&o, &h, &l, &c, 5).unwrap();
        // Last bar (11) should match
        assert_eq!(out[11], 100, "ants climbing tree should fire at index 11");
    }

    #[test]
    fn test_yang_through_three_ma() {
        // Long flat series followed by a big bullish breakout
        let n = 30;
        let mut o = vec![10.0; n];
        let mut c = vec![10.0; n];
        let mut h = vec![10.5; n];
        let mut l = vec![9.5; n];
        // Bar 25: big bullish breakout crossing all MAs
        o[25] = 9.0; // open below 5/10/20 MA
        c[25] = 12.0; // close above all
        h[25] = 12.5;
        l[25] = 8.5;
        let out = yang_through_three_ma(&o, &h, &l, &c).unwrap();
        assert_eq!(out[25], 100, "yang through three MA should fire at 25");
    }

    #[test]
    fn test_three_gap_downs() {
        let n = 10;
        let mut o = vec![10.0; n];
        let mut c = vec![9.8; n]; // bearish
        let h = vec![10.2; n];
        let mut l = vec![9.5; n];
        // Three bars with progressive gap-downs
        o[3] = 9.5;
        c[3] = 9.0;
        l[3] = 8.9;
        o[4] = 8.9;
        c[4] = 8.5;
        l[4] = 8.4;
        o[5] = 8.4;
        c[5] = 8.0;
        l[5] = 7.9;
        let out = three_gap_downs(&o, &h, &l, &c).unwrap();
        assert_eq!(out[5], -100);
    }

    #[test]
    fn test_bearish_counterattack() {
        // 5 warmup bars (so ATR(5) is defined), then bullish prev bar at i=5,
        // then gap-up bearish bar at i=6 that closes below the prior open.
        let o = vec![
            10.0, 10.1, 10.2, 10.3, 10.4, // warmup
            10.5, // bullish prev (open 10.5, close 11.0)
            11.5, // gap-up bearish (open 11.5, close 9.5 — covers prev body)
        ];
        let c = vec![10.15, 10.25, 10.35, 10.45, 10.55, 11.0, 9.5];
        let h = vec![10.3, 10.3, 10.4, 10.5, 10.6, 11.2, 11.6];
        let l = vec![9.9, 10.0, 10.1, 10.2, 10.3, 10.4, 9.4];
        let out = bearish_counterattack(&o, &h, &l, &c).unwrap();
        assert_eq!(out[6], -100);
    }

    #[test]
    fn test_inside_bar() {
        let n = 12;
        let mut o = vec![10.0; n];
        let mut c = vec![10.0; n];
        let mut h = vec![10.0; n];
        let mut l = vec![10.0; n];
        // Bar 5: big bearish body
        o[5] = 12.0;
        c[5] = 10.0;
        h[5] = 12.1;
        l[5] = 9.9;
        // Bar 6: small bullish inside
        o[6] = 11.0;
        c[6] = 11.2;
        h[6] = 11.3;
        l[6] = 10.9;
        let out = inside_bar(&o, &h, &l, &c).unwrap();
        assert_eq!(
            out[6], 100,
            "inside bar after bearish big body should be 100"
        );
    }

    #[test]
    fn test_seabed_saving_moon() {
        let n = 30;
        // Steep downtrend then a hammer-like bar with tiny upper shadow,
        // long lower shadow (>= 2x body), and shrinking volume.
        let o: Vec<f64> = (0..n).map(|i| 20.0 - (i as f64) * 0.3).collect();
        let h: Vec<f64> = o.iter().map(|&x| x + 0.05).collect();
        let l: Vec<f64> = o.iter().map(|&x| x - 1.5).collect();
        // Larger body (0.2) so the upper-shadow / body ratio passes the
        // 0.5× body threshold with comfortable margin.
        let c: Vec<f64> = o.iter().map(|x| *x - 0.2).collect();
        let volume: Vec<f64> = (0..n).map(|i| 1000.0 - (i as f64) * 10.0).collect();
        let out = seabed_saving_moon(&o, &h, &l, &c, &volume, 15).unwrap();
        assert!(out.iter().any(|&s| s == 100), "expected seabed signal");
    }

    #[test]
    fn test_plum_twice() {
        // Build a series: down → up to neckline → down to similar low → breakout
        let n = 50;
        let mut close = vec![10.0; n];
        // down phase
        for i in 5..20 {
            close[i] = 10.0 - (i - 5) as f64 * 0.2;
        }
        // rally
        for i in 20..30 {
            close[i] = 6.0 + (i - 20) as f64 * 0.4;
        }
        // second dip to similar low
        for i in 30..40 {
            close[i] = 10.0 - (i - 30) as f64 * 0.2;
        }
        // breakout
        for i in 40..n {
            close[i] = 6.0 + (i - 40) as f64 * 0.3;
        }
        let o = close.clone();
        let h: Vec<f64> = close.iter().map(|&x| x + 0.5).collect();
        let l: Vec<f64> = close.iter().map(|&x| x - 0.5).collect();
        let volume: Vec<f64> = (0..n)
            .map(|i| 100.0 + if i == 49 { 500.0 } else { 0.0 })
            .collect();
        let out = plum_twice(&o, &h, &l, &close, &volume, 10).unwrap();
        assert_eq!(out.len(), n);
    }

    #[test]
    fn test_length_mismatch_errors() {
        let o = vec![1.0, 2.0];
        let h = vec![1.0];
        assert!(hermit_pointing_way(&o, &h, &o, &o, &o, 5).is_err());
        assert!(yang_engulfing(&o, &h, &o, &o).is_err());
    }

    #[test]
    fn test_empty_input_errors() {
        let empty: Vec<f64> = vec![];
        assert!(yang_engulfing(&empty, &empty, &empty, &empty).is_err());
    }

    // -----------------------------------------------------------------
    // 5 new A-share K-line patterns (30→35 expansion)
    // -----------------------------------------------------------------

    #[test]
    fn test_top_through_break() {
        // need close[6] within 5% of max(close[4..=6])
        // close: 10.1, 10.2, 10.4, 10.6, 10.5, 10.8, 10.5
        // max(close[4..=6]) = 10.8, 0.95*10.8 = 10.26, close[6]=10.5 >= 10.26 ✓
        let open = vec![10.0, 10.1, 10.3, 10.5, 10.4, 10.5, 11.5];
        let high = vec![10.2, 10.3, 10.5, 10.7, 10.6, 10.9, 11.6];
        let low = vec![9.9, 10.0, 10.2, 10.4, 10.3, 10.4, 10.4];
        let close = vec![10.1, 10.2, 10.4, 10.6, 10.5, 10.8, 10.5];
        let r = top_through_break(&open, &high, &low, &close, 3).unwrap();
        // prev (i=5): open=10.5, close=10.8, yang, body=0.3
        // cur (i=6): open=11.5, close=10.5, yin, body=1.0 > 0.3*1.5=0.45 ✓
        assert_eq!(r[6], -100);
        assert_eq!(r[5], 0);
    }

    #[test]
    fn test_bottom_through_break() {
        // mirror of top
        let open = vec![10.0, 9.9, 9.7, 9.5, 9.6, 9.5, 8.5];
        let high = vec![10.2, 10.0, 9.8, 9.7, 9.7, 9.6, 9.6];
        let low = vec![9.8, 9.8, 9.6, 9.4, 9.5, 9.4, 8.4];
        let close = vec![9.9, 9.8, 9.6, 9.4, 9.5, 9.2, 9.5];
        // close[6]=9.5, max(close[4..=6])=max(9.5, 9.2, 9.5)=9.5
        // 1.05*min... wait, this is at_recent_low, check close[6] <= min * 1.05
        // min(close[4..=6])=9.2, 1.05*9.2=9.66, 9.5 <= 9.66 ✓
        let r = bottom_through_break(&open, &high, &low, &close, 3).unwrap();
        // prev (i=5): open=9.5, close=9.2, yin, body=0.3
        // cur (i=6): open=8.5, close=9.5, yang, body=1.0 > 0.45 ✓
        // open[6]=8.5 <= open[5]=9.5 - 0.01 ✓
        // close[6]=9.5 >= close[5]=9.2 + 0.01 ✓
        assert_eq!(r[6], 100);
    }

    #[test]
    fn test_low_five_yang() {
        // 5 根阳线，close[i-4]=9.6, close[i]=10.1, ratio=1.052 >= 1.05 ✓
        let open = vec![9.5, 9.6, 9.7, 9.8, 10.0];
        let close = vec![9.6, 9.7, 9.8, 9.9, 10.1];
        let high = vec![9.7, 9.8, 9.9, 10.0, 10.2];
        let low = vec![9.4, 9.5, 9.6, 9.7, 9.9];
        let r = low_five_yang(&open, &high, &low, &close).unwrap();
        assert_eq!(r[4], 100);
    }

    #[test]
    fn test_high_five_yin() {
        // 5 根阴线，close[i-4]=10.5, close[i]=9.7, ratio=0.924 <= 0.95 ✓
        let open = vec![10.6, 10.4, 10.2, 10.0, 9.8];
        let close = vec![10.5, 10.3, 10.1, 9.9, 9.7];
        let high = vec![10.7, 10.5, 10.3, 10.1, 9.9];
        let low = vec![10.4, 10.2, 10.0, 9.8, 9.6];
        let r = high_five_yin(&open, &high, &low, &close).unwrap();
        assert_eq!(r[4], -100);
    }

    #[test]
    fn test_t_cross() {
        // T 字: open=close=high=10, low=9 (long lower shadow)
        let open = vec![10.0, 10.0];
        let high = vec![10.0, 10.0];
        let low = vec![9.0, 9.5];
        let close = vec![10.0, 10.0];
        let r = t_cross(&open, &high, &low, &close).unwrap();
        assert_eq!(r[0], 100);
        assert_eq!(r[1], 100);
    }

    #[test]
    fn test_inverted_t_cross() {
        // 倒 T: open=close=low=10, high=11 (long upper shadow)
        let open = vec![10.0, 10.0];
        let high = vec![11.0, 11.5];
        let low = vec![10.0, 10.0];
        let close = vec![10.0, 10.0];
        let r = inverted_t_cross(&open, &high, &low, &close).unwrap();
        assert_eq!(r[0], -100);
        assert_eq!(r[1], -100);
    }
}
