//! 行业 / 板块轮动 (Sector Rotation)
//!
//! 申万一级 31 个行业指数 + 板块强度排序 + 轮动信号。
//!
//! Use cases:
//!
//! - 个股 vs 板块 / 板块 vs 大盘 的相对强弱排序
//! - 行业指数的动量轮动（短/长期动量对比）
//! - 板块持仓轮动（top K 持有 / bottom K 卖出）
//!
//! Data format: `Array2<f64>` shaped `[n_sectors, n_bars]`.

use crate::error::{Result, TaError};
use ndarray::{Array1, Array2};
use std::cmp::Ordering;

// ============================================================================
// 数据结构
// ============================================================================

/// 板块面板：n_sectors × n_bars
///
/// 申万一级 31 个行业指数 / 概念板块 / 自定义行业分组。
///
/// # Example
/// ```
/// use ndarray::Array2;
/// use finkit::sector::SectorPanel;
/// let close = Array2::<f64>::zeros((31, 100));
/// let panel = SectorPanel::new(
///     (0..31).map(|i| format!("8010{:02}0", i + 10)).collect(),
///     (0..31).map(|i| format!("行业{}", i + 1)).collect(),
///     close,
/// ).unwrap();
/// assert_eq!(panel.n_sectors(), 31);
/// assert_eq!(panel.n_bars(), 100);
/// ```
#[derive(Debug, Clone)]
pub struct SectorPanel {
    /// 行业代码（如 "801010"）
    pub codes: Vec<String>,
    /// 行业名称（如 "农林牧渔"）
    pub names: Vec<String>,
    /// 收盘价矩阵 `[n_sectors, n_bars]`
    pub close: Array2<f64>,
}

impl SectorPanel {
    /// 构造板块面板
    pub fn new(codes: Vec<String>, names: Vec<String>, close: Array2<f64>) -> Result<Self> {
        if codes.len() != names.len() {
            return Err(TaError::InvalidParameter {
                name: "codes, names".into(),
                constraint: "must have the same length".into(),
            });
        }
        if codes.is_empty() {
            return Err(TaError::EmptyInput);
        }
        let n_sectors = codes.len();
        if close.shape()[0] != n_sectors {
            return Err(TaError::InvalidParameter {
                name: "close".into(),
                constraint: format!("first dim must be n_sectors={}", n_sectors),
            });
        }
        Ok(Self { codes, names, close })
    }

    /// 板块数
    pub fn n_sectors(&self) -> usize {
        self.codes.len()
    }

    /// 行情 bar 数
    pub fn n_bars(&self) -> usize {
        self.close.shape()[1]
    }

    /// 获取某板块的收盘价序列
    pub fn sector_close(&self, idx: usize) -> Array1<f64> {
        self.close.row(idx).to_owned()
    }
}

// ============================================================================
// 相对强弱
// ============================================================================

/// 板块 vs 大盘基准的相对强弱
///
/// `rs[i, t] = return_sector[i, t] - return_benchmark[t]`
/// 其中 `return_x = close_x[t] / close_x[t-lookback] - 1`。
///
/// # Arguments
/// * `panel`     - 板块面板
/// * `benchmark` - 大盘基准收盘价（长度 = `n_bars`）
/// * `lookback`  - 回看窗口（≥ 1）
pub fn sector_relative_strength(
    panel: &SectorPanel,
    benchmark: &[f64],
    lookback: usize,
) -> Array2<f64> {
    let n_sectors = panel.n_sectors();
    let n_bars = panel.n_bars();
    let mut out = Array2::<f64>::zeros((n_sectors, n_bars));
    if lookback == 0 || lookback >= n_bars {
        return out;
    }
    for i in 0..n_sectors {
        let row = panel.close.row(i);
        for t in lookback..n_bars {
            let b0 = benchmark[t - lookback];
            let b1 = benchmark[t];
            let s0 = row[t - lookback];
            let s1 = row[t];
            if b0 <= 0.0 || s0 <= 0.0 {
                continue;
            }
            let rs_bench = b1 / b0 - 1.0;
            let rs_sec = s1 / s0 - 1.0;
            out[[i, t]] = rs_sec - rs_bench;
        }
    }
    out
}

// ============================================================================
// 板块强度排序
// ============================================================================

/// 板块强度排序：从强到弱的 `(name, rs)` 列表
///
/// 强度 = 板块在 `lookback` 日内的累计收益率（不依赖外部基准）。
///
/// # Arguments
/// * `panel`    - 板块面板
/// * `lookback` - 回看窗口（≥ 1）
pub fn sector_rank(panel: &SectorPanel, lookback: usize) -> Vec<(String, f64)> {
    let n_sectors = panel.n_sectors();
    let n_bars = panel.n_bars();
    if lookback == 0 || lookback >= n_bars {
        return Vec::new();
    }
    let mut scored: Vec<(String, f64)> = Vec::with_capacity(n_sectors);
    for i in 0..n_sectors {
        let row = panel.close.row(i);
        let p0 = row[n_bars - lookback];
        let p1 = row[n_bars - 1];
        let rs = if p0 > 0.0 { p1 / p0 - 1.0 } else { 0.0 };
        scored.push((panel.names[i].clone(), rs));
    }
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    scored
}

// ============================================================================
// 板块轮动信号
// ============================================================================

/// 板块轮动信号：`Array2<i32>` `[n_sectors, n_bars]`
///
/// 规则：
/// * 在 `t` 日，按 `lookback` 窗口内的累计收益排序
/// * 前 `top_k` 名 → +1（持有）
/// * 后 `bottom_k` 名 → -1（卖出 / 做空）
/// * 中间名次 → 0（中性）
///
/// # Arguments
/// * `panel`    - 板块面板
/// * `lookback` - 排序窗口
/// * `top_k`    - 持有数量（≥ 0）
/// * `bottom_k` - 卖出数量（≥ 0），`top_k + bottom_k ≤ n_sectors`
pub fn sector_rotation_signal(
    panel: &SectorPanel,
    lookback: usize,
    top_k: usize,
    bottom_k: usize,
) -> Array2<i32> {
    let n_sectors = panel.n_sectors();
    let n_bars = panel.n_bars();
    let mut out = Array2::<i32>::zeros((n_sectors, n_bars));
    if lookback == 0 || lookback >= n_bars || top_k + bottom_k > n_sectors {
        return out;
    }
    for t in lookback..n_bars {
        // Compute strength for each sector at bar t
        let mut scored: Vec<(usize, f64)> = (0..n_sectors)
            .map(|i| {
                let row = panel.close.row(i);
                let p0 = row[t - lookback];
                let p1 = row[t];
                let rs = if p0 > 0.0 { p1 / p0 - 1.0 } else { 0.0 };
                (i, rs)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        for (rank, &(i, _)) in scored.iter().enumerate() {
            if rank < top_k {
                out[[i, t]] = 1;
            } else if rank >= n_sectors - bottom_k {
                out[[i, t]] = -1;
            }
        }
    }
    out
}

// ============================================================================
// 动量轮动
// ============================================================================

/// 行业资金流入近似：动量轮动
///
/// 用"短期动量 - 长期动量"作为资金流入代理，返回从流入强到流入弱的
/// `(name, momentum)` 列表。
///
/// `momentum = return(short) - return(long)`
///
/// # Arguments
/// * `panel` - 板块面板
/// * `short` - 短期窗口（如 5 日）
/// * `long`  - 长期窗口（如 20 日）
pub fn sector_momentum_rotation(
    panel: &SectorPanel,
    short: usize,
    long: usize,
) -> Vec<(String, f64)> {
    let n_sectors = panel.n_sectors();
    let n_bars = panel.n_bars();
    if short == 0 || long == 0 || short >= long || long >= n_bars {
        return Vec::new();
    }
    let mut scored: Vec<(String, f64)> = Vec::with_capacity(n_sectors);
    for i in 0..n_sectors {
        let row = panel.close.row(i);
        let p_now = row[n_bars - 1];
        let p_short = row[n_bars - short];
        let p_long = row[n_bars - long];
        let r_short = if p_short > 0.0 { p_now / p_short - 1.0 } else { 0.0 };
        let r_long = if p_long > 0.0 { p_now / p_long - 1.0 } else { 0.0 };
        scored.push((panel.names[i].clone(), r_short - r_long));
    }
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    scored
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array2;

    fn build_panel(n_sectors: usize, n_bars: usize) -> SectorPanel {
        let mut close = Array2::<f64>::zeros((n_sectors, n_bars));
        for i in 0..n_sectors {
            for t in 0..n_bars {
                close[[i, t]] = 10.0 + (i as f64) * 0.1 + (t as f64) * 0.01 * (i as f64 + 1.0) * 0.1;
            }
        }
        let codes = (0..n_sectors).map(|i| format!("8010{:02}0", i + 10)).collect();
        let names = (0..n_sectors).map(|i| format!("行业{}", i + 1)).collect();
        SectorPanel::new(codes, names, close).unwrap()
    }

    #[test]
    fn test_sector_panel_basic() {
        let panel = build_panel(31, 100);
        assert_eq!(panel.n_sectors(), 31);
        assert_eq!(panel.n_bars(), 100);
        assert_eq!(panel.sector_close(0).len(), 100);
    }

    #[test]
    fn test_sector_panel_construction_errors() {
        let codes = vec!["A".to_string()];
        let names: Vec<String> = vec![];
        let close = Array2::<f64>::zeros((1, 5));
        assert!(SectorPanel::new(codes, names, close).is_err());
    }

    #[test]
    fn test_sector_relative_strength() {
        let panel = build_panel(5, 50);
        let benchmark: Vec<f64> = (0..50).map(|t| 100.0 + t as f64 * 0.1).collect();
        let rs = sector_relative_strength(&panel, &benchmark, 10);
        assert_eq!(rs.shape(), &[5, 50]);
        // The first 10 bars should be 0 (warmup period)
        for i in 0..5 {
            assert_eq!(rs[[i, 5]], 0.0, "warmup bars should be zero");
        }
        // Later bars should be non-zero
        assert!(rs[[4, 49]].abs() > 0.0, "warmup-over bars should have non-zero RS");
    }

    #[test]
    fn test_sector_rank() {
        let panel = build_panel(5, 50);
        let ranked = sector_rank(&panel, 20);
        assert_eq!(ranked.len(), 5);
        // The strongest sector should be sector 4 (highest slope)
        assert_eq!(ranked[0].0, "行业5");
        // Verify monotonic descending order
        for w in ranked.windows(2) {
            assert!(w[0].1 >= w[1].1);
        }
    }

    #[test]
    fn test_sector_rotation_signal() {
        let panel = build_panel(5, 50);
        let sig = sector_rotation_signal(&panel, 20, 2, 1);
        assert_eq!(sig.shape(), &[5, 50]);
        // Verify that at any t after warmup, exactly 2 are +1 and 1 is -1
        for t in 20..50 {
            let n_pos = (0..5).filter(|&i| sig[[i, t]] == 1).count();
            let n_neg = (0..5).filter(|&i| sig[[i, t]] == -1).count();
            assert_eq!(n_pos, 2, "expected 2 longs at t={}", t);
            assert_eq!(n_neg, 1, "expected 1 short at t={}", t);
        }
    }

    #[test]
    fn test_sector_momentum_rotation() {
        let panel = build_panel(5, 50);
        let mom = sector_momentum_rotation(&panel, 5, 20);
        assert_eq!(mom.len(), 5);
        // Sectors with steeper short-term slope should be ranked higher
        for w in mom.windows(2) {
            assert!(w[0].1 >= w[1].1);
        }
    }

    #[test]
    fn test_sector_rank_empty_when_lookback_too_large() {
        let panel = build_panel(5, 50);
        let ranked = sector_rank(&panel, 100);
        assert!(ranked.is_empty());
    }

    #[test]
    fn test_sector_rotation_signal_zero_k() {
        let panel = build_panel(5, 50);
        let sig = sector_rotation_signal(&panel, 20, 0, 0);
        // All zeros
        for t in 20..50 {
            for i in 0..5 {
                assert_eq!(sig[[i, t]], 0);
            }
        }
    }
}
