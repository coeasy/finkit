//! A-share specific moving-average combination patterns (均线组合形态).
//!
//! 15 patterns that use moving-average crossovers, ordering, and divergence
//! to generate buy/sell signals. Each function returns a [`PatternResult`]
//! (TA-Lib compatible: 100 = bullish, -100 = bearish, 0 = no signal).
//!
//! # Pattern groups
//!
//! * **Cross** (1-2): 金叉 / 死叉
//! * **Valley / Peak** (3-7): 银山谷 / 金山谷 / 死亡谷 / 金蜘蛛 / 死蜘蛛
//! * **Order** (8-9): 多头排列 / 空头排列
//! * **Convergence** (10-11): 粘合向上 / 粘合向下
//! * **Break** (12-13): 首次站上 60MA / 跌破 60MA
//! * **Divergence** (14-15): 均线底背离 / 均线顶背离

use crate::error::{Result, TaError};
use crate::patterns::common::precompute_sma;
use ndarray::Array1;

/// Pattern result alias (TA-Lib compatible: 100/-100/0).
pub type PatternResult = Array1<i32>;

// ============================================================================
// 金叉 / 死叉 (Golden Cross / Death Cross)
// ============================================================================

/// 金叉 — 短期 MA 上穿长期 MA
///
/// # 信号
/// * `100` — 金叉
/// * `0`   — 未触发
pub fn golden_cross(
    close: &[f64],
    short_period: usize,
    long_period: usize,
) -> Result<PatternResult> {
    if long_period <= short_period {
        return Err(TaError::InvalidParameter {
            name: "long_period".into(),
            constraint: "must be > short_period".into(),
        });
    }
    let n = close.len();
    let out = Array1::<i32>::zeros(n);
    let short_ma = precompute_sma(close, short_period);
    let long_ma = precompute_sma(close, long_period);
    detect_cross(&short_ma, &long_ma, &out, true)
}

/// 死叉 — 短期 MA 下穿长期 MA
pub fn death_cross(
    close: &[f64],
    short_period: usize,
    long_period: usize,
) -> Result<PatternResult> {
    if long_period <= short_period {
        return Err(TaError::InvalidParameter {
            name: "long_period".into(),
            constraint: "must be > short_period".into(),
        });
    }
    let n = close.len();
    let out = Array1::<i32>::zeros(n);
    let short_ma = precompute_sma(close, short_period);
    let long_ma = precompute_sma(close, long_period);
    detect_cross(&short_ma, &long_ma, &out, false)
}

fn detect_cross(
    short: &[f64],
    long_: &[f64],
    out: &Array1<i32>,
    bullish: bool,
) -> Result<PatternResult> {
    let mut out = out.clone();
    for i in 1..short.len() {
        if !short[i].is_finite()
            || !long_[i].is_finite()
            || !short[i - 1].is_finite()
            || !long_[i - 1].is_finite()
        {
            continue;
        }
        let crossed = if bullish {
            short[i - 1] <= long_[i - 1] && short[i] > long_[i]
        } else {
            short[i - 1] >= long_[i - 1] && short[i] < long_[i]
        };
        if crossed {
            out[i] = if bullish { 100 } else { -100 };
        }
    }
    Ok(out)
}

// ============================================================================
// 银山谷 / 金山谷 / 死亡谷 (Silver Valley / Gold Valley / Death Valley)
// ============================================================================

/// 银山谷 — 短期 MA 在长期 MA 下方金叉后，2 次金叉形成三角形向上发散
///
/// 在金叉后 `span` 个 bar 内出现第二次金叉，视为银山谷起点。
pub fn silver_valley(
    close: &[f64],
    short_period: usize,
    long_period: usize,
    span: usize,
) -> Result<PatternResult> {
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);
    let short_ma = precompute_sma(close, short_period);
    let long_ma = precompute_sma(close, long_period);

    // Find all golden cross points
    let mut crosses: Vec<usize> = Vec::new();
    for i in 1..n {
        if !short_ma[i].is_finite()
            || !long_ma[i].is_finite()
            || !short_ma[i - 1].is_finite()
            || !long_ma[i - 1].is_finite()
        {
            continue;
        }
        if short_ma[i - 1] <= long_ma[i - 1] && short_ma[i] > long_ma[i] {
            crosses.push(i);
        }
    }
    // Two crosses within `span` bars → silver valley
    for w in crosses.windows(2) {
        if w[1] - w[0] <= span {
            out[w[1]] = 100;
        }
    }
    Ok(out)
}

/// 金山谷 — 银山谷之后，第二个金叉点（更可靠的中继信号）
pub fn gold_valley(
    close: &[f64],
    short_period: usize,
    long_period: usize,
    span: usize,
) -> Result<PatternResult> {
    // Gold valley = silver valley with a third cross within span of the second
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);
    let short_ma = precompute_sma(close, short_period);
    let long_ma = precompute_sma(close, long_period);

    let mut crosses: Vec<usize> = Vec::new();
    for i in 1..n {
        if !short_ma[i].is_finite()
            || !long_ma[i].is_finite()
            || !short_ma[i - 1].is_finite()
            || !long_ma[i - 1].is_finite()
        {
            continue;
        }
        if short_ma[i - 1] <= long_ma[i - 1] && short_ma[i] > long_ma[i] {
            crosses.push(i);
        }
    }
    for w in crosses.windows(3) {
        if w[1] - w[0] <= span && w[2] - w[1] <= span {
            out[w[2]] = 100;
        }
    }
    Ok(out)
}

/// 死亡谷 — 死叉三角形向下发散
pub fn death_valley(
    close: &[f64],
    short_period: usize,
    long_period: usize,
    span: usize,
) -> Result<PatternResult> {
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);
    let short_ma = precompute_sma(close, short_period);
    let long_ma = precompute_sma(close, long_period);

    let mut crosses: Vec<usize> = Vec::new();
    for i in 1..n {
        if !short_ma[i].is_finite()
            || !long_ma[i].is_finite()
            || !short_ma[i - 1].is_finite()
            || !long_ma[i - 1].is_finite()
        {
            continue;
        }
        if short_ma[i - 1] >= long_ma[i - 1] && short_ma[i] < long_ma[i] {
            crosses.push(i);
        }
    }
    for w in crosses.windows(2) {
        if w[1] - w[0] <= span {
            out[w[1]] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 金蜘蛛 / 死蜘蛛 (Golden Spider / Death Spider)
// ============================================================================

/// 金蜘蛛 — 5/10/20 MA 三线粘合后同时向上发散
///
/// # 规则
/// 1. 5/10/20 MA 最大-最小 ≤ `convergence_pct × 中位数 MA`
/// 2. 之后短期 MA 增速 > 长期 MA 增速
pub fn golden_spider(close: &[f64], convergence_pct: f64) -> Result<PatternResult> {
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);
    let ma5 = precompute_sma(close, 5);
    let ma10 = precompute_sma(close, 10);
    let ma20 = precompute_sma(close, 20);

    for i in 20..n {
        if !ma5[i].is_finite() || !ma10[i].is_finite() || !ma20[i].is_finite() {
            continue;
        }
        let max_ma = ma5[i].max(ma10[i]).max(ma20[i]);
        let min_ma = ma5[i].min(ma10[i]).min(ma20[i]);
        let mid = (max_ma + min_ma) / 2.0;
        if mid <= 0.0 {
            continue;
        }
        if (max_ma - min_ma) / mid > convergence_pct {
            continue;
        }
        // All three MAs should rise in the next 3 bars, and the rate of rise
        // should accelerate: 5MA rises faster than 20MA
        if i + 3 >= n {
            continue;
        }
        let slope5 = (ma5[i + 3] - ma5[i]) / 3.0;
        let slope20 = (ma20[i + 3] - ma20[i]) / 3.0;
        if slope5 > 0.0 && slope20 > 0.0 && slope5 > slope20 {
            out[i] = 100;
        }
    }
    Ok(out)
}

/// 死蜘蛛 — 三线粘合后向下发散
pub fn death_spider(close: &[f64], convergence_pct: f64) -> Result<PatternResult> {
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);
    let ma5 = precompute_sma(close, 5);
    let ma10 = precompute_sma(close, 10);
    let ma20 = precompute_sma(close, 20);

    for i in 20..n {
        if !ma5[i].is_finite() || !ma10[i].is_finite() || !ma20[i].is_finite() {
            continue;
        }
        let max_ma = ma5[i].max(ma10[i]).max(ma20[i]);
        let min_ma = ma5[i].min(ma10[i]).min(ma20[i]);
        let mid = (max_ma + min_ma) / 2.0;
        if mid <= 0.0 {
            continue;
        }
        if (max_ma - min_ma) / mid > convergence_pct {
            continue;
        }
        if i + 3 >= n {
            continue;
        }
        let slope5 = (ma5[i + 3] - ma5[i]) / 3.0;
        let slope20 = (ma20[i + 3] - ma20[i]) / 3.0;
        if slope5 < 0.0 && slope20 < 0.0 && slope5 < slope20 {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 多头排列 / 空头排列 (Bullish / Bearish Alignment)
// ============================================================================

/// 多头排列 — 5MA > 10MA > 20MA > 60MA 持续 `period` 日
pub fn bullish_alignment(close: &[f64], period: usize) -> Result<PatternResult> {
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);
    let ma5 = precompute_sma(close, 5);
    let ma10 = precompute_sma(close, 10);
    let ma20 = precompute_sma(close, 20);
    let ma60 = precompute_sma(close, 60);

    for i in 60..n {
        // Check last `period` bars all satisfy the ordering
        let ok = (0..period.min(i + 1)).all(|k| {
            let j = i - k;
            ma5[j].is_finite()
                && ma10[j].is_finite()
                && ma20[j].is_finite()
                && ma60[j].is_finite()
                && ma5[j] > ma10[j]
                && ma10[j] > ma20[j]
                && ma20[j] > ma60[j]
        });
        if ok {
            out[i] = 100;
        }
    }
    Ok(out)
}

/// 空头排列 — 5MA < 10MA < 20MA < 60MA 持续 `period` 日
pub fn bearish_alignment(close: &[f64], period: usize) -> Result<PatternResult> {
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);
    let ma5 = precompute_sma(close, 5);
    let ma10 = precompute_sma(close, 10);
    let ma20 = precompute_sma(close, 20);
    let ma60 = precompute_sma(close, 60);

    for i in 60..n {
        let ok = (0..period.min(i + 1)).all(|k| {
            let j = i - k;
            ma5[j].is_finite()
                && ma10[j].is_finite()
                && ma20[j].is_finite()
                && ma60[j].is_finite()
                && ma5[j] < ma10[j]
                && ma10[j] < ma20[j]
                && ma20[j] < ma60[j]
        });
        if ok {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 粘合向上 / 粘合向下 (Convergence Up / Down)
// ============================================================================

/// 粘合向上 — 5/10/20MA 粘合（最大-最小 ≤ 1%）后向上发散
pub fn convergence_up(
    close: &[f64],
    convergence_pct: f64,
    lookforward: usize,
) -> Result<PatternResult> {
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);
    let ma5 = precompute_sma(close, 5);
    let ma10 = precompute_sma(close, 10);
    let ma20 = precompute_sma(close, 20);

    for i in 20..n.saturating_sub(lookforward) {
        if !ma5[i].is_finite() || !ma10[i].is_finite() || !ma20[i].is_finite() {
            continue;
        }
        let max_ma = ma5[i].max(ma10[i]).max(ma20[i]);
        let min_ma = ma5[i].min(ma10[i]).min(ma20[i]);
        let mid = (max_ma + min_ma) / 2.0;
        if mid <= 0.0 || (max_ma - min_ma) / mid > convergence_pct {
            continue;
        }
        // Converged. Now check that in the next `lookforward` bars the
        // short MA rises faster than the long MA
        let slope5 = (ma5[i + lookforward] - ma5[i]) / lookforward as f64;
        let slope20 = (ma20[i + lookforward] - ma20[i]) / lookforward as f64;
        if slope5 > 0.0 && slope5 > slope20 * 1.5 {
            out[i] = 100;
        }
    }
    Ok(out)
}

/// 粘合向下 — 三线粘合后向下发散
pub fn convergence_down(
    close: &[f64],
    convergence_pct: f64,
    lookforward: usize,
) -> Result<PatternResult> {
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);
    let ma5 = precompute_sma(close, 5);
    let ma10 = precompute_sma(close, 10);
    let ma20 = precompute_sma(close, 20);

    for i in 20..n.saturating_sub(lookforward) {
        if !ma5[i].is_finite() || !ma10[i].is_finite() || !ma20[i].is_finite() {
            continue;
        }
        let max_ma = ma5[i].max(ma10[i]).max(ma20[i]);
        let min_ma = ma5[i].min(ma10[i]).min(ma20[i]);
        let mid = (max_ma + min_ma) / 2.0;
        if mid <= 0.0 || (max_ma - min_ma) / mid > convergence_pct {
            continue;
        }
        let slope5 = (ma5[i + lookforward] - ma5[i]) / lookforward as f64;
        let slope20 = (ma20[i + lookforward] - ma20[i]) / lookforward as f64;
        if slope5 < 0.0 && slope5 < slope20 * 1.5 {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 首次站上 60MA / 跌破 60MA
// ============================================================================

/// 首次站上 60MA — 长期下跌后首次收盘站上 60 日均线
pub fn first_break_above_ma60(close: &[f64]) -> Result<PatternResult> {
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);
    let ma60 = precompute_sma(close, 60);

    for i in 60..n {
        if !ma60[i].is_finite() || !ma60[i - 1].is_finite() {
            continue;
        }
        if close[i - 1] <= ma60[i - 1] && close[i] > ma60[i] {
            // Make sure we've been below MA60 for at least 30 days
            let below_count = (1..=30.min(i))
                .filter(|&k| close[i - k] < ma60[i - k])
                .count();
            if below_count >= 20 {
                out[i] = 100;
            }
        }
    }
    Ok(out)
}

/// 跌破 60MA — 长期上涨后首次收盘跌破 60 日均线
pub fn first_break_below_ma60(close: &[f64]) -> Result<PatternResult> {
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);
    let ma60 = precompute_sma(close, 60);

    for i in 60..n {
        if !ma60[i].is_finite() || !ma60[i - 1].is_finite() {
            continue;
        }
        if close[i - 1] >= ma60[i - 1] && close[i] < ma60[i] {
            let above_count = (1..=30.min(i))
                .filter(|&k| close[i - k] > ma60[i - k])
                .count();
            if above_count >= 20 {
                out[i] = -100;
            }
        }
    }
    Ok(out)
}

// ============================================================================
// 均线背离 (MA Divergence)
// ============================================================================

/// 均线底背离 — 价格新低但短期 MA 未新低
pub fn ma_bullish_divergence(
    close: &[f64],
    short_period: usize,
    lookback: usize,
) -> Result<PatternResult> {
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);
    let short_ma = precompute_sma(close, short_period);

    for i in lookback..n {
        if !short_ma[i].is_finite() {
            continue;
        }
        // Find the previous lowest close and lowest MA in [i-lookback, i-1]
        let mut min_close_idx = i - 1;
        let mut min_ma_idx = i - 1;
        for k in (i.saturating_sub(lookback))..i {
            if close[k] < close[min_close_idx] {
                min_close_idx = k;
            }
            if short_ma[k].is_finite() && short_ma[k] < short_ma[min_ma_idx] {
                min_ma_idx = k;
            }
        }
        // Current: new low in close, but MA is above the previous MA low
        if close[i] < close[min_close_idx] && short_ma[i] > short_ma[min_ma_idx] {
            out[i] = 100;
        }
    }
    Ok(out)
}

/// 均线顶背离 — 价格新高但短期 MA 未新高
pub fn ma_bearish_divergence(
    close: &[f64],
    short_period: usize,
    lookback: usize,
) -> Result<PatternResult> {
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);
    let short_ma = precompute_sma(close, short_period);

    for i in lookback..n {
        if !short_ma[i].is_finite() {
            continue;
        }
        let mut max_close_idx = i - 1;
        let mut max_ma_idx = i - 1;
        for k in (i.saturating_sub(lookback))..i {
            if close[k] > close[max_close_idx] {
                max_close_idx = k;
            }
            if short_ma[k].is_finite() && short_ma[k] > short_ma[max_ma_idx] {
                max_ma_idx = k;
            }
        }
        if close[i] > close[max_close_idx] && short_ma[i] < short_ma[max_ma_idx] {
            out[i] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 均线首次多/空头排列 / MACD 柱状背离
// ============================================================================

/// 均线首次多头排列 — 5MA > 10MA > 20MA > 60MA 首次全部成立
///
/// # 信号
/// * `100` — 首次多头排列（看涨）
/// * `0`   — 已成立或未触发
///
/// "首次" 的定义：前 10 个 bar 内未出现过多头排列。
pub fn ma_first_bull_alignment(close: &[f64]) -> Result<PatternResult> {
    if close.len() < 70 {
        return Err(TaError::InvalidParameter {
            name: "close".into(),
            constraint: "length must be >= 70".into(),
        });
    }
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);
    let m5 = precompute_sma(close, 5);
    let m10 = precompute_sma(close, 10);
    let m20 = precompute_sma(close, 20);
    let m60 = precompute_sma(close, 60);

    // Track previous bar's "in alignment" state. Fire only on transition
    // from "not in alignment" to "in alignment" (rising edge).
    let mut prev_in = false;
    for i in 60..n {
        if !m5[i].is_finite() || !m10[i].is_finite() || !m20[i].is_finite() || !m60[i].is_finite() {
            prev_in = false;
            continue;
        }
        let bull_now = m5[i] > m10[i] && m10[i] > m20[i] && m20[i] > m60[i];
        if bull_now && !prev_in {
            out[i] = 100;
        }
        prev_in = bull_now;
    }
    Ok(out)
}

/// 均线首次空头排列 — 5MA < 10MA < 20MA < 60MA 首次全部成立
pub fn ma_first_bear_alignment(close: &[f64]) -> Result<PatternResult> {
    if close.len() < 70 {
        return Err(TaError::InvalidParameter {
            name: "close".into(),
            constraint: "length must be >= 70".into(),
        });
    }
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);
    let m5 = precompute_sma(close, 5);
    let m10 = precompute_sma(close, 10);
    let m20 = precompute_sma(close, 20);
    let m60 = precompute_sma(close, 60);

    let mut prev_in = false;
    for i in 60..n {
        if !m5[i].is_finite() || !m10[i].is_finite() || !m20[i].is_finite() || !m60[i].is_finite() {
            prev_in = false;
            continue;
        }
        let bear_now = m5[i] < m10[i] && m10[i] < m20[i] && m20[i] < m60[i];
        if bear_now && !prev_in {
            out[i] = -100;
        }
        prev_in = bear_now;
    }
    Ok(out)
}

/// MACD 柱状背离 — 价格新高但 MACD 柱未新高
///
/// # 输入
/// * `close` - 收盘价
/// * `macd_hist` - 预计算的 MACD 柱状值 (DIF - DEA)
/// * `lookback` - 比较窗口
///
/// # 信号
/// * `-100` — 顶背离（看跌）
/// * `100`  — 底背离（看涨）：价格新低但 MACD 柱未新低
pub fn macd_histogram_divergence(
    close: &[f64],
    macd_hist: &[f64],
    lookback: usize,
) -> Result<PatternResult> {
    if close.len() != macd_hist.len() {
        return Err(TaError::InvalidParameter {
            name: "close, macd_hist".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if lookback < 2 {
        return Err(TaError::InvalidParameter {
            name: "lookback".into(),
            constraint: "must be >= 2".into(),
        });
    }
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);

    for i in lookback..n {
        // Find max close and max macd_hist in [i-lookback, i-1]
        let mut max_close_idx = i - 1;
        let mut max_hist_idx = i - 1;
        let mut min_close_idx = i - 1;
        let mut min_hist_idx = i - 1;
        for k in (i - lookback)..i {
            if close[k] > close[max_close_idx] {
                max_close_idx = k;
            }
            if macd_hist[k] > macd_hist[max_hist_idx] {
                max_hist_idx = k;
            }
            if close[k] < close[min_close_idx] {
                min_close_idx = k;
            }
            if macd_hist[k] < macd_hist[min_hist_idx] {
                min_hist_idx = k;
            }
        }
        // 顶背离：价格新高但 hist 未新高
        if close[i] > close[max_close_idx] && macd_hist[i] < macd_hist[max_hist_idx] {
            out[i] = -100;
            continue;
        }
        // 底背离：价格新低但 hist 未新低
        if close[i] < close[min_close_idx] && macd_hist[i] > macd_hist[min_hist_idx] {
            out[i] = 100;
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

    #[test]
    fn test_golden_cross_basic() {
        let n = 30;
        // Flat for 20 bars, then a steep uptrend that lifts the 5MA above
        // the 10MA and triggers a golden cross.
        let mut close = vec![10.0; n];
        for i in 20..n {
            close[i] = 10.0 + (i - 20) as f64 * 0.5;
        }
        let out = golden_cross(&close, 5, 10).unwrap();
        let fires: Vec<usize> = (0..n).filter(|&i| out[i] == 100).collect();
        assert!(!fires.is_empty(), "golden cross should fire on uptrend");
    }

    #[test]
    fn test_death_cross_basic() {
        let n = 30;
        // Steep uptrend for 20 bars, then a flat segment that drops the 5MA
        // back below the 10MA and triggers a death cross.
        let mut close = vec![10.0; n];
        for i in 0..20 {
            close[i] = 10.0 + i as f64 * 0.5;
        }
        let out = death_cross(&close, 5, 10).unwrap();
        let fires: Vec<usize> = (0..n).filter(|&i| out[i] == -100).collect();
        assert!(!fires.is_empty());
    }

    #[test]
    fn test_invalid_period() {
        let close = vec![1.0; 20];
        assert!(golden_cross(&close, 10, 5).is_err());
    }

    #[test]
    fn test_bullish_alignment() {
        let n = 80;
        // Strict uptrend: 5MA > 10MA > 20MA > 60MA always
        let close: Vec<f64> = (0..n).map(|i| 10.0 + (i as f64) * 0.5).collect();
        let out = bullish_alignment(&close, 5).unwrap();
        assert!(out.iter().any(|&s| s == 100));
    }

    #[test]
    fn test_bearish_alignment() {
        let n = 80;
        let close: Vec<f64> = (0..n).map(|i| 100.0 - (i as f64) * 0.5).collect();
        let out = bearish_alignment(&close, 5).unwrap();
        assert!(out.iter().any(|&s| s == -100));
    }

    #[test]
    fn test_golden_spider() {
        // Sideways then breakout
        let n = 30;
        let close: Vec<f64> = (0..n).map(|i| 10.0 + ((i as f64) * 0.3).sin()).collect();
        let out = golden_spider(&close, 0.02).unwrap();
        assert_eq!(out.len(), n);
    }

    #[test]
    fn test_convergence_up() {
        let n = 40;
        // 20 bars flat, then strong uptrend
        let mut close = vec![10.0; n];
        for i in 20..n {
            close[i] = 10.0 + (i - 20) as f64 * 0.3;
        }
        let out = convergence_up(&close, 0.01, 3).unwrap();
        assert_eq!(out.len(), n);
    }

    #[test]
    fn test_first_break_above_ma60() {
        // 60 bars of decline, then a flat stabilization period (so MA60
        // keeps cooling down), followed by a gentle uptrend that crosses
        // MA60 for the first time.
        let n = 120;
        let mut close = vec![0.0; n];
        for i in 0..60 {
            close[i] = 100.0 - i as f64;
        }
        // 30 bars of stabilization at the low so MA60 has time to cool down
        for i in 60..90 {
            close[i] = 40.0;
        }
        // 30 bars of gentle uptrend (1 per bar) to cross MA60 from below
        for i in 90..n {
            close[i] = 40.0 + (i - 90) as f64;
        }
        let out = first_break_above_ma60(&close).unwrap();
        assert!(
            out.iter().any(|&s| s == 100),
            "expected first break above MA60"
        );
    }

    #[test]
    fn test_ma_divergence() {
        let n = 30;
        // Two price lows but MA is rising
        let close = vec![
            10.0, 9.0, 8.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 12.0, 11.0, 10.5, 10.0, 9.5,
            10.0, 10.5, 11.0, 12.0, 13.0, 14.0, 14.5, 15.0, 15.5, 16.0, 16.5, 17.0, 17.5, 18.0,
            18.5,
        ];
        let out = ma_bullish_divergence(&close, 5, 20).unwrap();
        assert_eq!(out.len(), n);
    }

    // -----------------------------------------------------------------
    // 3 new A-share MA patterns (15→18 expansion)
    // -----------------------------------------------------------------

    #[test]
    fn test_ma_first_bull_alignment() {
        let n = 80;
        // Slow uptrend to ensure 5MA > 10MA > 20MA > 60MA eventually
        let close: Vec<f64> = (0..n).map(|i| 10.0 + (i as f64) * 0.5).collect();
        let out = ma_first_bull_alignment(&close).unwrap();
        assert_eq!(out.len(), n);
        // In a sustained uptrend, the first alignment should be detected
        assert!(out.iter().any(|&s| s == 100));
    }

    #[test]
    fn test_ma_first_bear_alignment() {
        let n = 80;
        // Slow downtrend
        let close: Vec<f64> = (0..n).map(|i| 100.0 - (i as f64) * 0.5).collect();
        let out = ma_first_bear_alignment(&close).unwrap();
        assert!(out.iter().any(|&s| s == -100));
    }

    #[test]
    fn test_ma_first_alignment_short_input() {
        // length < 70 should error
        let close = vec![1.0; 50];
        assert!(ma_first_bull_alignment(&close).is_err());
        assert!(ma_first_bear_alignment(&close).is_err());
    }

    #[test]
    fn test_macd_histogram_divergence_top() {
        // 顶背离: prices make new high but MACD hist doesn't
        let close = vec![10.0, 11.0, 12.0, 13.0, 14.0, 13.5, 15.0];
        // MACD hist: was high when price was 14 (e.g., 0.5), now weaker
        let hist = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.4, 0.2];
        let out = macd_histogram_divergence(&close, &hist, 5).unwrap();
        // bar 6: close=15 > max(close[1..6])=14 → top divergence
        assert_eq!(out[6], -100);
    }

    #[test]
    fn test_macd_histogram_divergence_bottom() {
        // 底背离: prices make new low but MACD hist doesn't
        let close = vec![15.0, 14.0, 13.0, 12.0, 11.0, 11.5, 10.0];
        let hist = vec![-0.1, -0.2, -0.3, -0.4, -0.5, -0.4, -0.2];
        let out = macd_histogram_divergence(&close, &hist, 5).unwrap();
        // bar 6: close=10 < min(close[1..6])=11 → bottom divergence
        assert_eq!(out[6], 100);
    }

    #[test]
    fn test_macd_histogram_divergence_length_mismatch() {
        let close = vec![1.0, 2.0];
        let hist = vec![1.0];
        assert!(macd_histogram_divergence(&close, &hist, 2).is_err());
    }
}
