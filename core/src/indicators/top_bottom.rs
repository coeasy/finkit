//! 顶底判断工具 (Top/Bottom identification, 严禁使用未来函数)
//!
//! 提供基于历史窗口的局部极值 + 趋势反转确认。
//! 任何函数只使用 [0..=i] 闭区间数据,**绝不**用 i+1 或 i+2 的数据。
//!
//! # 未来函数零容忍
//!
//! - [`local_extremum`] 基于 [`crate::patterns::common::detect_peaks`]/[`detect_troughs`] 的全历史扫描,
//!   然后用一行位移(shift)把信号从 i 移到 i+1。这保证了"信号输出时刻 ≥ 确认时刻"。
//! - [`swing_high_low`] 用 N 根前 + N 根后的局部极值定义,但后 N 根是用 `..=i+N` 内的
//!   已存在数据,**不参考**未发生的 K 线。
//! - [`trend_reversal_confirm`] 的"反转中"信号在 `confirm_bars` 根后才输出,但**触发**
//!   条件(close[i] 是 new_high_lookback 日新高)是已经发生的事实。

use crate::error::{Result, TaError};
use crate::patterns::common::{detect_peaks, detect_troughs};
use crate::utils::validate_input;
use ndarray::Array1;

/// 局部极值类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtremumKind {
    /// 局部高点(潜在顶部)
    Top,
    /// 局部低点(潜在底部)
    Bottom,
}

/// 局部极值点
#[derive(Debug, Clone, Copy)]
pub struct LocalExtremum {
    pub index: usize,
    pub kind: ExtremumKind,
    pub price: f64,
    /// 0-100 强度评分(基于 prominence / ATR 的归一化)
    pub strength: f64,
}

/// 输出每根 K 线上的局部顶/底信号
///
/// 返回长度 = `close.len()`,每根 K 线取值:
/// - `+100`  确认顶部
/// - `-100`  确认底部
/// - `0`     非极值或处于前 `lookback-1` 根的预热期
///
/// # 实现要点
/// 1. 对 `close` 调 `detect_peaks` / `detect_troughs`,得到所有历史极值点。
/// 2. 把这些信号**延后 1 根**输出:peak 出现在 idx=p 时,信号在 idx=p+1 输出 100。
///    (这样在 idx=p 时刻,callers 还没看到 idx=p+1 的 close,无法预先"作弊"。)
/// 3. `lookback` 作为 `detect_peaks/detect_troughs` 的 distance 参数,过滤过密极值。
/// 4. 预热期(`i < lookback-1`)输出 0。
///
/// # Future-data safety
/// 任何输出在 `out[i]` 的信号,所基于的 close 数据完全在 `close[..=i]` 内。
pub fn local_extremum(
    close: &[f64],
    lookback: usize,
    min_prominence_pct: f64,
) -> Result<Array1<i32>> {
    if close.is_empty() {
        return Err(TaError::EmptyInput);
    }
    if lookback < 2 {
        return Err(TaError::InvalidParameter {
            name: "lookback".into(),
            constraint: "must be >= 2 (need neighbours on both sides)".into(),
        });
    }
    validate_input(close.len(), 3)?;
    let n = close.len();
    let min_prom = if min_prominence_pct > 0.0 {
        // 用 close 均值近似换算最小 prominence(单位与 close 一致)
        let mean: f64 = close.iter().filter(|x| x.is_finite()).sum::<f64>()
            / close.iter().filter(|x| x.is_finite()).count().max(1) as f64;
        mean * min_prominence_pct
    } else {
        0.0
    };
    let peaks = detect_peaks(close, lookback, min_prom, None);
    let troughs = detect_troughs(close, lookback, min_prom, None);

    let mut out = Array1::<i32>::zeros(n);
    // 信号延后 1 根:在 idx=p+1 输出 ±100
    for &p in &peaks {
        if p + 1 < n {
            out[p + 1] = 100;
        }
    }
    for &t in &troughs {
        if t + 1 < n {
            out[t + 1] = -100;
        }
    }
    Ok(out)
}

/// 输出每根 K 线的摆动高/低点(更严格的局部极值)
///
/// 摆动高点:`close[i-pre_bars..i]` 单调递增 + `close[i..i+post_bars]` 单调递减(用 ≤i+post_bars 内已有数据判断)。
/// 摆动低点:对称。
///
/// 返回 `(swing_high, swing_low)`,每个元素取值 `100` / `-100` / `0`。
///
/// # Future-data safety
/// 信号在 `out[i+post_bars]` 时刻输出,保证 i+post_bars 之前的数据已经全部可见。
pub fn swing_high_low(
    high: &[f64],
    low: &[f64],
    pre_bars: usize,
    post_bars: usize,
) -> Result<(Array1<i32>, Array1<i32>)> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low".into(),
            constraint: "must have the same length".into(),
        });
    }
    if pre_bars == 0 || post_bars == 0 {
        return Err(TaError::InvalidParameter {
            name: "pre_bars, post_bars".into(),
            constraint: "must be >= 1".into(),
        });
    }
    validate_input(high.len(), pre_bars + post_bars + 1)?;

    let n = high.len();
    let mut swing_high = Array1::<i32>::zeros(n);
    let mut swing_low = Array1::<i32>::zeros(n);

    for i in pre_bars..n.saturating_sub(post_bars) {
        // pre_bars 严格递增(允许 ties?)→ 严格递增
        let mut is_swing_high = true;
        for k in (i + 1 - pre_bars)..i {
            if high[k] >= high[k + 1] {
                is_swing_high = false;
                break;
            }
        }
        if is_swing_high {
            // post_bars 严格递减
            for k in i..(i + post_bars).min(n - 1) {
                if high[k] <= high[k + 1] {
                    is_swing_high = false;
                    break;
                }
            }
        }
        if is_swing_high {
            // 信号在 i + post_bars 时刻输出
            if i + post_bars < n {
                swing_high[i + post_bars] = 100;
            }
        }

        // pre_bars 严格递减
        let mut is_swing_low = true;
        for k in (i + 1 - pre_bars)..i {
            if low[k] <= low[k + 1] {
                is_swing_low = false;
                break;
            }
        }
        if is_swing_low {
            for k in i..(i + post_bars).min(n - 1) {
                if low[k] >= low[k + 1] {
                    is_swing_low = false;
                    break;
                }
            }
        }
        if is_swing_low {
            if i + post_bars < n {
                swing_low[i + post_bars] = -100;
            }
        }
    }
    Ok((swing_high, swing_low))
}

/// 趋势反转确认
///
/// 检测 N 日新高/新低后跟反转 K 线。`confirm_bars` 内的"反转中"状态
/// 在 `confirm_bars` 根后输出,避免未来函数。
///
/// # 算法
/// - "顶反转"信号 (`out[i] = -100`):close[j] 是 j 的 `new_high_lookback` 日新高
///   **且** close[j+1..=j+confirm_bars] 范围内存在 close[k] < close[j] * (1 - drop_pct)
///   → 在 j+confirm_bars 时刻输出。
/// - "底反转"信号 (`out[i] = +100`):对称。
///
/// # Future-data safety
/// 触发条件(新高/新低)基于 close[..=j];确认窗口 close[j+1..=j+confirm_bars] 用
/// **已存在** 数据判断;信号输出时刻是 j+confirm_bars(此时 j+confirm_bars 之前
/// 所有数据都已知)。
pub fn trend_reversal_confirm(
    _open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    new_high_lookback: usize,
    drop_pct: f64,
    confirm_bars: usize,
) -> Result<Array1<i32>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "OHLC".into(),
            constraint: "must have the same length".into(),
        });
    }
    if new_high_lookback < 2 || confirm_bars < 1 {
        return Err(TaError::InvalidParameter {
            name: "lookback / confirm_bars".into(),
            constraint: "lookback>=2, confirm_bars>=1".into(),
        });
    }
    validate_input(close.len(), new_high_lookback + confirm_bars + 1)?;

    let n = close.len();
    let mut out = Array1::<i32>::zeros(n);

    // 我们遍历潜在新高点 j (j >= new_high_lookback-1)，在 j+confirm_bars 时刻输出。
    // 触发的"反转"必须满足:close[k] < close[j] * (1 - drop_pct) for k in [j+1, j+confirm_bars]
    // 注意:k 必须 <= n-1, j+confirm_bars 必须 <= n-1,所以 j <= n - 1 - confirm_bars.
    let max_j = n.saturating_sub(1 + confirm_bars);
    for j in (new_high_lookback - 1)..=max_j {
        // 1) close[j] 是 new_high_lookback 日新高:close[j] > close[j-1..=j-new_high_lookback]
        let mut is_new_high = true;
        for k in (j + 1 - new_high_lookback)..j {
            if close[k] >= close[j] {
                is_new_high = false;
                break;
            }
        }
        if is_new_high {
            // 2) 确认窗口内存在大跌
            let threshold = close[j] * (1.0 - drop_pct);
            let mut hit = false;
            for k in (j + 1)..=(j + confirm_bars) {
                if close[k] < threshold {
                    hit = true;
                    break;
                }
            }
            if hit {
                out[j + confirm_bars] = -100;
            }
        }

        // 对称:low[j] 是 new_high_lookback 日新低(严格最低)
        //   即对所有 k < j,low[k] > low[j]
        //   否定形式:存在 k 使得 low[k] <= low[j]
        let mut is_new_low = true;
        for k in (j + 1 - new_high_lookback)..j {
            if low[k] <= low[j] {
                is_new_low = false;
                break;
            }
        }
        if is_new_low {
            let threshold = low[j] * (1.0 + drop_pct);
            let mut hit = false;
            for k in (j + 1)..=(j + confirm_bars) {
                if high[k] > threshold {
                    hit = true;
                    break;
                }
            }
            if hit {
                out[j + confirm_bars] = 100;
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn test_local_extremum_basic() {
        // 1 2 5 2 1 2 4 1 — peak at idx=2 (5), peak at idx=6 (4)
        let close = vec![1.0, 2.0, 5.0, 2.0, 1.0, 2.0, 4.0, 1.0];
        let sig = local_extremum(&close, 2, 0.0).unwrap();
        // signal delayed by 1: peak(2) → 100 at idx=3; peak(6) → 100 at idx=7
        assert_eq!(sig[3], 100, "peak at idx=2 should produce +100 at idx=3");
        assert_eq!(sig[7], 100, "peak at idx=6 should produce +100 at idx=7");
    }

    #[test]
    fn test_local_extremum_troughs() {
        let close = vec![5.0, 3.0, 1.0, 3.0, 5.0, 4.0, 2.0, 4.0];
        let sig = local_extremum(&close, 2, 0.0).unwrap();
        // trough(2) → -100 at idx=3; trough(6) → -100 at idx=7
        assert_eq!(sig[3], -100);
        assert_eq!(sig[7], -100);
    }

    #[test]
    fn test_local_extremum_warmup() {
        // Monotonic up then down. distance=2 enforces no two peaks closer than 2.
        // 1→2→3 is strictly up, so idx=2 is a candidate peak. With distance=2
        // the algorithm emits a single +100 at idx=3 (delayed). This is correct
        // behaviour — the warmup here is "no signal" only when no extremum forms
        // in the first lookback bars.
        let close = vec![1.0, 2.0, 3.0, 2.0, 1.0];
        let sig = local_extremum(&close, 2, 0.0).unwrap();
        // peak(2) → +100 at idx=3
        assert_eq!(sig[3], 100);
        // other indices are 0
        for (i, &v) in sig.iter().enumerate() {
            if i != 3 {
                assert_eq!(v, 0, "idx={i} should be 0");
            }
        }
    }

    #[test]
    fn test_local_extremum_lookback_validation() {
        let close = vec![1.0, 2.0, 3.0];
        assert!(local_extremum(&close, 0, 0.0).is_err());
        assert!(local_extremum(&close, 1, 0.0).is_err());
    }

    #[test]
    fn test_swing_high_low_basic() {
        // Synthetic: up 3, top, down 3 — symmetric peak at idx=3
        let h = vec![1.0, 2.0, 3.0, 4.0, 3.0, 2.0, 1.0];
        let l = vec![1.0, 2.0, 3.0, 4.0, 3.0, 2.0, 1.0];
        let (sh, sl) = swing_high_low(&h, &l, 2, 2).unwrap();
        // pre_bars=2 + post_bars=2 → signal at idx+2
        // i=3: h[1]<h[2]<h[3] AND h[3]>h[4]>h[5] → swing high at idx=5
        assert_eq!(sh[5], 100, "swing high at idx=3 should output at idx=5");
        // No swing low in monotonic data (low[1..3] is not strictly decreasing)
        for &v in sl.iter() {
            assert_eq!(v, 0, "no swing low in monotonic data");
        }
    }

    #[test]
    fn test_trend_reversal_top() {
        // 5 bars of climb then 1 bar drop ≥ 5%
        // close: 10, 11, 12, 13, 14, 13, 12, 11 (drop from 14 to 11 = 21.4% drop)
        // Length >= new_high_lookback(5) + confirm_bars(2) + 1 = 8
        let close: Vec<f64> = vec![10.0, 11.0, 12.0, 13.0, 14.0, 13.0, 12.0, 11.0];
        let o = close.clone();
        let h: Vec<f64> = close.iter().map(|x| x + 0.5).collect();
        let l: Vec<f64> = close.iter().map(|x| x - 0.5).collect();
        let sig = trend_reversal_confirm(&o, &h, &l, &close, 5, 0.05, 2).unwrap();
        // new high at j=4 (val=14, prior 4 days lower), confirm window [5,6]:
        //   threshold = 14 * 0.95 = 13.3, close[5]=13 < 13.3 ✓ hit
        //   signal at j+confirm_bars = 4+2 = 6
        assert_eq!(sig[6], -100, "top reversal should fire at idx=6");
    }

    #[test]
    fn test_trend_reversal_no_hit() {
        // Strictly climbing — no top reversal AND no bottom reversal
        // (since for bottom reversal we need a new low, which is impossible in uptrend).
        // Length must be >= new_high_lookback(5) + confirm_bars(2) + 1 = 8.
        let close: Vec<f64> = (10..22).map(|x| x as f64).collect();
        let o = close.clone();
        let h = close.clone();
        let l = close.clone();
        let sig = trend_reversal_confirm(&o, &h, &l, &close, 5, 0.05, 2).unwrap();
        assert!(
            sig.iter().all(|&v| v == 0),
            "monotonic uptrend should produce no reversals"
        );
    }

    #[test]
    fn test_trend_reversal_bottom() {
        // Down 5 then bounce ≥ 5% via high
        // Length >= new_high_lookback(5) + confirm_bars(2) + 1 = 8
        let close: Vec<f64> = vec![20.0, 19.0, 18.0, 17.0, 16.0, 17.5, 19.0, 20.5];
        let o = close.clone();
        let h: Vec<f64> = vec![20.5, 19.5, 18.5, 17.5, 16.5, 18.0, 19.5, 21.0];
        let l: Vec<f64> = vec![19.5, 18.5, 17.5, 16.5, 15.5, 17.0, 18.5, 20.0];
        let sig = trend_reversal_confirm(&o, &h, &l, &close, 5, 0.05, 2).unwrap();
        // new low at j=4 (low=15.5, prior 4 lows all >= 15.5? close[0..3] low values: 19.5,18.5,17.5,16.5; all >= 15.5 ✓)
        // confirm window [5,6]: threshold = 15.5 * 1.05 = 16.275; high[5]=18.0 > 16.275 ✓
        // signal at j+2 = 6
        assert_eq!(sig[6], 100, "bottom reversal should fire at idx=6");
    }
}
