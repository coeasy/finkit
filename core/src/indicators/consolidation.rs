//! 横盘 / 蓄势 / 突破指标 (Consolidation & Breakout)
//!
//! Detect sideway phases (横盘) and the subsequent breakout (突破).
//!
//! A-share swing trading relies on these signals heavily — a tight consolidation
//! after a trend often produces a high-conviction breakout in the same direction.
//!
//! # Functions
//!
//! - [`consolidation_score`] — 综合评分 0~100（越高越横）
//! - [`is_sideways`]        — 二值横盘判断
//! - [`bottom_breakout`]    — 横盘后向上突破（+100）
//! - [`top_breakdown`]      — 横盘后向下跌破（-100）
//! - [`sideways_duration`]  — 当前连续横盘天数

use crate::error::{Result, TaError};
use crate::math::moving_avg::sma;
use crate::math::statistics::{rolling_max, rolling_min, rolling_std_dev};
use crate::utils::{init_output, validate_input};
use ndarray::Array1;

// ============================================================================
// 横盘综合评分
// ============================================================================

/// 横盘综合评分 `0~100`：分越高越横
///
/// 综合四个维度（满分 25 分）：
/// * 布林带宽度 `(UB-LB)/MA` < 0.10
/// * ADX（粗略：以 stdev/mean 近似） < 0.20
/// * ATR/Price < 0.025
/// * 振幅 `(high-low)/close` 的 `lookback` 日均值 < 0.04
///
/// # Arguments
/// * `high`, `low`, `close` - OHLC
/// * `lookback`     - 评估窗口
/// * `atr_period`   - ATR 周期
///
/// # Returns
/// `Array1<f64>` 长度 = `close.len()`，前 `lookback` 个 bar 为 NaN
pub fn consolidation_score(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
    atr_period: usize,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "OHLC".into(),
            constraint: "must have the same length".into(),
        });
    }
    validate_input(close.len(), lookback.max(atr_period) + 1)?;
    let n = close.len();
    let mut out = init_output(n);

    // ATR via rolling mean of true range
    let atr = rolling_atr(high, low, close, atr_period)?;
    // BB width via rolling stddev
    let std = rolling_std_dev(close, lookback)?;
    let mean = sma(close, lookback)?;

    for t in lookback..n {
        if mean[t] <= 0.0 || !mean[t].is_finite() || !std[t].is_finite() || !atr[t].is_finite() {
            continue;
        }
        let bb_width = 4.0 * std[t] / mean[t]; // 2-sigma band, normalized
        let cv = std[t] / mean[t]; // coefficient of variation
        let atr_pct = atr[t] / close[t];

        // amplitude over lookback
        let hh = high[t - lookback + 1..=t]
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let ll = low[t - lookback + 1..=t]
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let amp = if close[t] > 0.0 { (hh - ll) / close[t] } else { 0.0 };

        let mut s = 0.0;
        if bb_width < 0.10 {
            s += 25.0;
        } else if bb_width < 0.15 {
            s += 12.5;
        }
        if cv < 0.02 {
            s += 25.0;
        } else if cv < 0.04 {
            s += 12.5;
        }
        if atr_pct < 0.025 {
            s += 25.0;
        } else if atr_pct < 0.04 {
            s += 12.5;
        }
        if amp < 0.05 {
            s += 25.0;
        } else if amp < 0.10 {
            s += 12.5;
        }
        out[t] = s;
    }
    Ok(out)
}

// ============================================================================
// 横盘指数 (二值)
// ============================================================================

/// 横盘判断：true 表示该 bar 处于横盘状态
///
/// 综合三条规则（全部满足才为 true）：
/// * BB 宽度 < `bb_width`（默认 0.10）
/// * 振幅 `(max-min)/close` < 0.05
/// * `close` 在 `lookback` 窗口内的最大值-最小值 < 5%
pub fn is_sideways(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
    bb_period: usize,
    bb_width: f64,
) -> Result<Array1<bool>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "OHLC".into(),
            constraint: "must have the same length".into(),
        });
    }
    validate_input(close.len(), lookback.max(bb_period) + 1)?;
    let n = close.len();
    let mut out = Array1::from_elem(n, false);

    let std = rolling_std_dev(close, bb_period)?;
    let mean = sma(close, bb_period)?;
    let hh = rolling_max(high, lookback)?;
    let ll = rolling_min(low, lookback)?;

    for t in lookback.max(bb_period)..n {
        if mean[t] <= 0.0 || !std[t].is_finite() || close[t] <= 0.0 {
            continue;
        }
        let bw = 4.0 * std[t] / mean[t];
        let range_pct = if close[t] > 0.0 {
            (hh[t] - ll[t]) / close[t]
        } else {
            0.0
        };
        out[t] = bw < bb_width && range_pct < 0.05;
    }
    Ok(out)
}

// ============================================================================
// 底部突破
// ============================================================================

/// 底部突破信号：横盘后向上突破
///
/// 条件（全部满足）：
/// * 前 `consolidation_lookback` 日横盘（用 [`is_sideways`] 规则）
/// * 当日 `close` 突破箱体上沿 = `rolling_max(high, consolidation_lookback)`
/// * 当日放量 ≥ `vol_ma5 * 1.5`
/// * 当日 `close` > 5MA
pub fn bottom_breakout(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    consolidation_lookback: usize,
    bb_period: usize,
    bb_width: f64,
) -> Result<Array1<i32>> {
    if high.len() != low.len() || high.len() != close.len() || close.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "OHLCV".into(),
            constraint: "must have the same length".into(),
        });
    }
    validate_input(close.len(), consolidation_lookback.max(bb_period) + 6)?;
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);

    let sideways = is_sideways(high, low, close, consolidation_lookback, bb_period, bb_width)?;
    let box_top = rolling_max(high, consolidation_lookback)?;
    let vol_ma5 = sma(volume, 5)?;
    let ma5 = sma(close, 5)?;

    for t in consolidation_lookback.max(bb_period) + 5..n {
        // Need at least one sideways bar in the lookback window
        let had_sideways = (t - consolidation_lookback..t).any(|k| sideways[k]);
        if !had_sideways {
            continue;
        }
        let box_val = box_top[t - 1];
        if !box_val.is_finite() || !vol_ma5[t].is_finite() || !ma5[t].is_finite() {
            continue;
        }
        // Bullish breakout: close > previous box top, with volume surge, above 5MA
        if close[t] > box_val
            && open[t] < close[t]
            && volume[t] > vol_ma5[t] * 1.5
            && close[t] > ma5[t]
        {
            out[t] = 100;
        }
    }
    Ok(out)
}

// ============================================================================
// 顶部跌破
// ============================================================================

/// 顶部跌破信号：横盘后向下突破
///
/// 条件：前 N 日横盘 + 当日 `close` 跌破箱体下沿 + 放量 + 收盘 < 5MA
pub fn top_breakdown(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    consolidation_lookback: usize,
    bb_period: usize,
    bb_width: f64,
) -> Result<Array1<i32>> {
    if high.len() != low.len() || high.len() != close.len() || close.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "OHLCV".into(),
            constraint: "must have the same length".into(),
        });
    }
    validate_input(close.len(), consolidation_lookback.max(bb_period) + 6)?;
    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);

    let sideways = is_sideways(high, low, close, consolidation_lookback, bb_period, bb_width)?;
    let box_bot = rolling_min(low, consolidation_lookback)?;
    let vol_ma5 = sma(volume, 5)?;
    let ma5 = sma(close, 5)?;

    for t in consolidation_lookback.max(bb_period) + 5..n {
        let had_sideways = (t - consolidation_lookback..t).any(|k| sideways[k]);
        if !had_sideways {
            continue;
        }
        let bot_val = box_bot[t - 1];
        if !bot_val.is_finite() || !vol_ma5[t].is_finite() || !ma5[t].is_finite() {
            continue;
        }
        // Bearish breakdown
        if close[t] < bot_val
            && open[t] > close[t]
            && volume[t] > vol_ma5[t] * 1.5
            && close[t] < ma5[t]
        {
            out[t] = -100;
        }
    }
    Ok(out)
}

// ============================================================================
// 横盘持续天数
// ============================================================================

/// 当前连续横盘天数（在尾部窗口内）
///
/// # Arguments
/// * `is_sideways` - [`is_sideways`] 的输出
///
/// # Returns
/// `Array1<usize>`，每个 bar 表示"从该 bar 向前看，连续横盘的次数（含自身）"
pub fn sideways_duration(is_sideways: &[bool]) -> Array1<usize> {
    let n = is_sideways.len();
    let mut out = Array1::<usize>::zeros(n);
    let mut cur = 0usize;
    for i in 0..n {
        if is_sideways[i] {
            cur += 1;
        } else {
            cur = 0;
        }
        out[i] = cur;
    }
    out
}

// ============================================================================
// 横盘紧密度 + 倾斜度
// ============================================================================

/// 横盘紧密度评分 (0-100)
///
/// 高分 = 价格紧密围绕中枢波动（好横盘，适合做突破蓄势）
/// 低分 = 宽幅震荡或趋势明显
///
/// 计算维度（每维 25 分）：
/// 1. 区间/均值 < 5%  → 25 分
/// 2. 区间/均值 < 10% → 15 分；< 15% → 8 分
/// 3. 5MA 斜率 < 0.1%/bar → 25 分
/// 4. ATR(14) / 收盘价 < 2% → 25 分
///
/// # Future-data safety
/// 完全使用 `close[..=i]` 的闭区间数据,无未来函数。
pub fn sideways_quality(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "OHLC".into(),
            constraint: "must have the same length".into(),
        });
    }
    validate_input(close.len(), lookback + 1)?;
    let n = close.len();
    let mut out = Array1::<f64>::from(vec![0.0; n]);
    let ma5 = sma(close, 5_usize.min(lookback))?;
    let atr14 = rolling_atr(high, low, close, 14_usize.min(lookback))?;

    for t in lookback..n {
        let start = t + 1 - lookback;
        let mean_c = close[start..=t].iter().sum::<f64>() / lookback as f64;
        if mean_c <= 0.0 {
            continue;
        }
        let hi = high[start..=t].iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let lo = low[start..=t].iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let range = (hi - lo) / mean_c;

        let mut score: f64 = 0.0;
        // 维度 1: 区间宽度
        if range < 0.05 {
            score += 25.0;
        } else if range < 0.10 {
            score += 15.0;
        } else if range < 0.15 {
            score += 8.0;
        }
        // 维度 2: 5MA 斜率(用 lookback 起点 vs 终点近似)
        if t > start && ma5[t].is_finite() && ma5[start].is_finite() {
            let slope = ((ma5[t] - ma5[start]) / ma5[start] / lookback as f64).abs();
            if slope < 0.001 {
                score += 25.0;
            } else if slope < 0.003 {
                score += 15.0;
            }
        }
        // 维度 3: ATR%
        if atr14[t].is_finite() {
            let atr_pct = atr14[t] / close[t];
            if atr_pct < 0.02 {
                score += 25.0;
            } else if atr_pct < 0.04 {
                score += 15.0;
            }
        }
        // 维度 4: close 相对均值的偏移(对称性)
        let close_dev = ((close[t] - mean_c) / mean_c).abs();
        if close_dev < 0.01 {
            score += 25.0;
        } else if close_dev < 0.03 {
            score += 15.0;
        }
        out[t] = score.min(100.0);
    }
    Ok(out)
}

/// 横盘倾斜度（-100 ~ +100）
///
/// 0 = 完美水平横盘；正 = 看多偏好（横盘期间重心逐步上移）；负 = 看空偏好。
///
/// 算法：横盘区间内最高点 vs 最低点的位置和幅度结合，给出方向偏好。
/// 若 `close` 在横盘区间前半段低于后半段 → 倾向看多。
///
/// # Future-data safety
/// 使用 `close[..=i]` 数据,无未来函数。
pub fn sideways_tilt(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    lookback: usize,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "OHLC".into(),
            constraint: "must have the same length".into(),
        });
    }
    validate_input(close.len(), lookback + 1)?;
    let n = close.len();
    let mut out = Array1::<f64>::zeros(n);
    for t in lookback..n {
        let start = t + 1 - lookback;
        let mean_c = close[start..=t].iter().sum::<f64>() / lookback as f64;
        if mean_c <= 0.0 {
            continue;
        }
        // 前半段均值 vs 后半段均值
        let mid = start + lookback / 2;
        let first_mean = close[start..mid].iter().sum::<f64>() / (mid - start) as f64;
        let second_mean = close[mid..=t].iter().sum::<f64>() / (t - mid + 1) as f64;
        // 归一化差值
        let diff_pct = (second_mean - first_mean) / mean_c;
        // 映射到 -100..+100（5% 涨跌幅对应 ±100）
        out[t] = (diff_pct * 2000.0).clamp(-100.0, 100.0);
    }
    Ok(out)
}

// ============================================================================
// 突破评分（0-100 连续分数）
// ============================================================================

/// 底部突破评分（0-100）：横盘天数 + 突破幅度 + 量比 + MA5 偏离
///
/// # 评分构成
/// * 横盘天数分 (0-25): 横盘天数 / 10 × 25, 截断 25
/// * 突破幅度分 (0-30): 突破幅度 / 0.05 × 30, 截断 30
/// * 量比分 (0-20): vol_ratio / 1.5 × 20, 截断 20
/// * MA5 偏离分 (0-25): (close - MA5) / MA5 / 0.02 × 25, 截断 25
///
/// 综合输出 0-100。
pub fn bottom_breakout_score(
    _open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    consolidation_lookback: usize,
    bb_period: usize,
    bb_width: f64,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() || close.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "OHLCV".into(),
            constraint: "must have the same length".into(),
        });
    }
    validate_input(close.len(), consolidation_lookback.max(bb_period) + 6)?;
    let n = close.len();
    let mut out = Array1::<f64>::zeros(n);

    let sideways = is_sideways(high, low, close, consolidation_lookback, bb_period, bb_width)?;
    let box_top = rolling_max(high, consolidation_lookback)?;
    let vol_ma5 = sma(volume, 5)?;
    let ma5 = sma(close, 5)?;
    let side_dur = sideways_duration(sideways.as_slice().expect("contiguous bool array"));

    for t in consolidation_lookback.max(bb_period) + 5..n {
        let had_sideways = (t - consolidation_lookback..t).any(|k| sideways[k]);
        if !had_sideways {
            continue;
        }
        let box_val = box_top[t - 1];
        if !box_val.is_finite() || !vol_ma5[t].is_finite() || !ma5[t].is_finite() {
            continue;
        }
        // 必须放量
        if volume[t] <= vol_ma5[t] * 1.0 {
            continue;
        }
        let dur = side_dur[t] as f64;
        let s_dur = 25.0 * (dur / 10.0).clamp(0.0, 1.0);
        let breakout_pct = if box_val > 0.0 { (close[t] - box_val) / box_val } else { 0.0 };
        let s_break = 30.0 * (breakout_pct / 0.05).clamp(0.0, 1.0);
        let vol_ratio = if vol_ma5[t] > 0.0 { volume[t] / vol_ma5[t] } else { 0.0 };
        let s_vol = 20.0 * (vol_ratio / 1.5).clamp(0.0, 1.0);
        let ma5_dev = if ma5[t] > 0.0 { (close[t] - ma5[t]) / ma5[t] } else { 0.0 };
        let s_ma = 25.0 * (ma5_dev / 0.02).clamp(0.0, 1.0);
        out[t] = (s_dur + s_break + s_vol + s_ma).min(100.0);
    }
    Ok(out)
}

/// 顶部跌破评分（0-100，对称）
pub fn top_breakdown_score(
    _open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    consolidation_lookback: usize,
    bb_period: usize,
    bb_width: f64,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() || close.len() != volume.len() {
        return Err(TaError::InvalidParameter {
            name: "OHLCV".into(),
            constraint: "must have the same length".into(),
        });
    }
    validate_input(close.len(), consolidation_lookback.max(bb_period) + 6)?;
    let n = close.len();
    let mut out = Array1::<f64>::zeros(n);

    let sideways = is_sideways(high, low, close, consolidation_lookback, bb_period, bb_width)?;
    let box_bot = rolling_min(low, consolidation_lookback)?;
    let vol_ma5 = sma(volume, 5)?;
    let ma5 = sma(close, 5)?;
    let side_dur = sideways_duration(sideways.as_slice().expect("contiguous bool array"));

    for t in consolidation_lookback.max(bb_period) + 5..n {
        let had_sideways = (t - consolidation_lookback..t).any(|k| sideways[k]);
        if !had_sideways {
            continue;
        }
        let bot_val = box_bot[t - 1];
        if !bot_val.is_finite() || !vol_ma5[t].is_finite() || !ma5[t].is_finite() {
            continue;
        }
        if volume[t] <= vol_ma5[t] * 1.0 {
            continue;
        }
        let dur = side_dur[t] as f64;
        let s_dur = 25.0 * (dur / 10.0).clamp(0.0, 1.0);
        let breakdown_pct = if bot_val > 0.0 { (bot_val - close[t]) / bot_val } else { 0.0 };
        let s_break = 30.0 * (breakdown_pct / 0.05).clamp(0.0, 1.0);
        let vol_ratio = if vol_ma5[t] > 0.0 { volume[t] / vol_ma5[t] } else { 0.0 };
        let s_vol = 20.0 * (vol_ratio / 1.5).clamp(0.0, 1.0);
        let ma5_dev = if ma5[t] > 0.0 { (ma5[t] - close[t]) / ma5[t] } else { 0.0 };
        let s_ma = 25.0 * (ma5_dev / 0.02).clamp(0.0, 1.0);
        out[t] = (s_dur + s_break + s_vol + s_ma).min(100.0);
    }
    Ok(out)
}

// ============================================================================
// Helpers
// ============================================================================

/// 简单 ATR：rolling mean of true range
fn rolling_atr(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
) -> Result<Array1<f64>> {
    let n = close.len();
    let mut tr = vec![0.0_f64; n];
    for i in 0..n {
        if i == 0 {
            tr[i] = high[i] - low[i];
        } else {
            let hl = high[i] - low[i];
            let hpc = (high[i] - close[i - 1]).abs();
            let lpc = (low[i] - close[i - 1]).abs();
            tr[i] = hl.max(hpc).max(lpc);
        }
    }
    sma(&tr, period)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn synth_sideways_then_breakout() -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
        let n = 60;
        let mut o = vec![0.0; n];
        let mut h = vec![0.0; n];
        let mut l = vec![0.0; n];
        let mut c = vec![0.0; n];
        let mut v = vec![100.0; n];
        // bars 0..29: sideways at ~10 ± 0.2
        for i in 0..30 {
            o[i] = 10.0 + (i as f64 % 2.0) * 0.05 - 0.025;
            c[i] = 10.0 - (i as f64 % 2.0) * 0.05 + 0.025;
            h[i] = 10.20;
            l[i] = 9.80;
        }
        // bars 30..44: continued sideways
        for i in 30..45 {
            o[i] = 10.0 + ((i - 30) as f64 % 3.0) * 0.04 - 0.06;
            c[i] = 10.0 - ((i - 30) as f64 % 3.0) * 0.04 + 0.06;
            h[i] = 10.18;
            l[i] = 9.82;
        }
        // bar 45: strong breakout
        o[45] = 10.10;
        c[45] = 10.60;
        h[45] = 10.70;
        l[45] = 10.05;
        v[45] = 300.0;
        // bars 46..59: continued uptrend
        for i in 46..n {
            o[i] = 10.50 + (i - 46) as f64 * 0.05;
            c[i] = o[i] + 0.10;
            h[i] = c[i] + 0.05;
            l[i] = o[i] - 0.05;
            v[i] = 150.0;
        }
        (o, h, l, c, v)
    }

    #[test]
    fn test_is_sideways_basic() {
        let (_o, h, l, c, _v) = synth_sideways_then_breakout();
        let s = is_sideways(&h, &l, &c, 20, 20, 0.10).unwrap();
        // Bars 25-44 should be sideways
        let n_sideways = (20..45).filter(|&i| s[i]).count();
        assert!(n_sideways > 10, "expected many sideways bars, got {}", n_sideways);
    }

    #[test]
    fn test_consolidation_score() {
        let (_o, h, l, c, _v) = synth_sideways_then_breakout();
        let s = consolidation_score(&h, &l, &c, 20, 14).unwrap();
        assert_eq!(s.len(), c.len());
        // Score at bar 30 should be high
        assert!(s[35] > 50.0, "expected high consolidation score at 35, got {}", s[35]);
    }

    #[test]
    fn test_bottom_breakout() {
        let (o, h, l, c, v) = synth_sideways_then_breakout();
        let sig = bottom_breakout(&o, &h, &l, &c, &v, 20, 20, 0.10).unwrap();
        // Bar 45 should fire
        assert_eq!(sig[45], 100, "expected bottom breakout at 45");
    }

    #[test]
    fn test_top_breakdown() {
        // Build: uptrend → sideways → strong down day
        let n = 60;
        let mut o = vec![0.0; n];
        let mut h = vec![0.0; n];
        let mut l = vec![0.0; n];
        let mut c = vec![0.0; n];
        let mut v = vec![100.0; n];
        for i in 0..30 {
            o[i] = 10.0 + (i as f64) * 0.05;
            c[i] = o[i] + 0.10;
            h[i] = c[i] + 0.05;
            l[i] = o[i] - 0.05;
        }
        for i in 30..45 {
            o[i] = 11.5 + (i as f64 % 3.0) * 0.05;
            c[i] = 11.5 - (i as f64 % 3.0) * 0.05;
            h[i] = 11.7;
            l[i] = 11.3;
        }
        o[45] = 11.4;
        c[45] = 10.9;
        h[45] = 11.5;
        l[45] = 10.85;
        v[45] = 300.0;
        for i in 46..n {
            o[i] = 11.0 - (i - 46) as f64 * 0.05;
            c[i] = o[i] - 0.10;
            h[i] = o[i];
            l[i] = c[i] - 0.05;
        }
        let sig = top_breakdown(&o, &h, &l, &c, &v, 20, 20, 0.10).unwrap();
        assert_eq!(sig[45], -100, "expected top breakdown at 45");
    }

    #[test]
    fn test_sideways_duration() {
        let flags = vec![true, true, false, true, true, true, false, true];
        let dur = sideways_duration(&flags);
        let expected = vec![1, 2, 0, 1, 2, 3, 0, 1];
        assert_eq!(dur.to_vec(), expected);
    }

    #[test]
    fn test_input_validation() {
        let empty: Vec<f64> = vec![];
        assert!(consolidation_score(&empty, &empty, &empty, 5, 5).is_err());
        let o = vec![1.0, 2.0];
        let h = vec![1.0];
        assert!(is_sideways(&h, &o, &o, 5, 5, 0.1).is_err());
    }

    #[test]
    fn test_sideways_quality_tight_box() {
        // 30 bars of perfect sideways at 10 ± 0.05
        let n = 30;
        let h: Vec<f64> = (0..n).map(|_| 10.05).collect();
        let l: Vec<f64> = (0..n).map(|_| 9.95).collect();
        let c: Vec<f64> = (0..n).map(|_| 10.0).collect();
        let score = sideways_quality(&h, &l, &c, 20).unwrap();
        // Score at the end of the box should be high
        assert!(score[25] > 60.0, "expected high tightness, got {}", score[25]);
    }

    #[test]
    fn test_sideways_quality_wide_range() {
        // 30 bars of 50% range — should score low
        let n = 30;
        let h: Vec<f64> = (0..n).map(|i| 10.0 + (i as f64) * 0.5).collect();
        let l: Vec<f64> = (0..n).map(|i| 5.0 + (i as f64) * 0.5).collect();
        let c: Vec<f64> = (0..n).map(|i| 7.5 + (i as f64) * 0.5).collect();
        let score = sideways_quality(&h, &l, &c, 20).unwrap();
        assert!(score[25] < 30.0, "expected low score for trending data, got {}", score[25]);
    }

    #[test]
    fn test_sideways_tilt_up() {
        // First half lower, second half higher → positive tilt
        let n = 30;
        let c: Vec<f64> = (0..n).map(|i| 10.0 + (i as f64) * 0.01).collect();
        let h: Vec<f64> = c.iter().map(|x| x + 0.05).collect();
        let l: Vec<f64> = c.iter().map(|x| x - 0.05).collect();
        let tilt = sideways_tilt(&h, &l, &c, 20).unwrap();
        assert!(tilt[25] > 0.0, "upward trend should have positive tilt, got {}", tilt[25]);
    }

    #[test]
    fn test_sideways_tilt_down() {
        let n = 30;
        let c: Vec<f64> = (0..n).map(|i| 10.0 - (i as f64) * 0.01).collect();
        let h: Vec<f64> = c.iter().map(|x| x + 0.05).collect();
        let l: Vec<f64> = c.iter().map(|x| x - 0.05).collect();
        let tilt = sideways_tilt(&h, &l, &c, 20).unwrap();
        assert!(tilt[25] < 0.0, "downward trend should have negative tilt, got {}", tilt[25]);
    }

    // ====================== Score variants ======================

    #[test]
    fn test_bottom_breakout_score() {
        let (o, h, l, c, v) = synth_sideways_then_breakout();
        let s = bottom_breakout_score(&o, &h, &l, &c, &v, 20, 20, 0.10).unwrap();
        // Bar 45 should have a non-zero score
        assert!(s[45] > 0.0, "expected positive breakout score at 45, got {}", s[45]);
    }

    #[test]
    fn test_top_breakdown_score() {
        // Reuse synth pattern: build uptrend→sideways→breakdown manually
        let n = 60;
        let mut o = vec![0.0; n];
        let mut h = vec![0.0; n];
        let mut l = vec![0.0; n];
        let mut c = vec![0.0; n];
        let mut v = vec![100.0; n];
        for i in 0..30 {
            o[i] = 10.0 + (i as f64) * 0.05;
            c[i] = o[i] + 0.10;
            h[i] = c[i] + 0.05;
            l[i] = o[i] - 0.05;
        }
        for i in 30..45 {
            o[i] = 11.5 + (i as f64 % 3.0) * 0.05;
            c[i] = 11.5 - (i as f64 % 3.0) * 0.05;
            h[i] = 11.7;
            l[i] = 11.3;
        }
        o[45] = 11.4;
        c[45] = 10.9;
        h[45] = 11.5;
        l[45] = 10.85;
        v[45] = 300.0;
        for i in 46..n {
            o[i] = 11.0 - (i - 46) as f64 * 0.05;
            c[i] = o[i] - 0.10;
            h[i] = o[i];
            l[i] = c[i] - 0.05;
        }
        let s = top_breakdown_score(&o, &h, &l, &c, &v, 20, 20, 0.10).unwrap();
        assert!(s[45] > 0.0, "expected positive breakdown score at 45, got {}", s[45]);
    }
}
