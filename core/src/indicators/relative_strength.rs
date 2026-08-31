//! 相对强弱评级 (Relative Strength Rating, A 股个股 vs 大盘/板块)
//!
//! 借鉴 IBD (Investor's Business Daily) RS Rating 的核心思路：用 1/3/6/9/12 月
//! 相对收益率的加权百分位合成 1-99 分。分值越高表示个股相对基准的强度越高。
//!
//! # 关键特性
//!
//! - **单股评分** [`rs_rating`]：输出 1-99 整数（50 = 中位, 99 = 全市场最强, 1 = 最弱）
//! - **RS 动量** [`rs_slope`]：每根 K 线的 RS 评分（与 N 日前比较），-99 ~ +99
//! - **横截面排名** [`relative_strength_rank`]：对一组个股 vs 共同基准做排名
//!
//! # Future-data safety
//!
//! 所有函数严格使用 `data[..=i]` 闭区间数据,严禁用未来函数。

use crate::error::{Result, TaError};
use crate::utils::validate_input;
use ndarray::Array1;

// ============================================================================
// 常量：5 个评估期 + 权重
// ============================================================================

/// 5 个评估期（按交易日折算 1/3/6/9/12 月）
const RS_PERIODS: [usize; 5] = [21, 63, 126, 189, 252];

/// 5 期权重（IBD 经典权重：近期权重最高）
const RS_WEIGHTS: [f64; 5] = [0.4, 0.2, 0.2, 0.1, 0.1];

// ============================================================================
// 单股 RS 评分
// ============================================================================

/// 个股 vs 基准的 RS 评分（1-99）
///
/// 综合 1/3/6/9/12 月相对收益率（个股收益率 - 基准收益率）的加权百分位。
///
/// # Arguments
/// * `symbol`    - 个股收盘价序列
/// * `benchmark` - 基准收盘价序列（沪深300 / 行业指数 / 板块等）
/// * `as_of`     - 评估日索引（0-based）
///
/// # Returns
/// `u8` 1-99 整数：
/// - 99 = 全市场最强
/// - 50 = 中位
/// - 1  = 最弱
///
/// # Future-data safety
/// 仅用 `symbol[..=as_of]` 与 `benchmark[..=as_of]` 数据,无未来函数。
///
/// # Errors
/// - `InvalidParameter`: `symbol.len() != benchmark.len()` 或 `as_of` 越界
/// - `InsufficientData`: 数据长度不足以计算任何一期收益
pub fn rs_rating(symbol: &[f64], benchmark: &[f64], as_of: usize) -> Result<u8> {
    if symbol.len() != benchmark.len() {
        return Err(TaError::InvalidParameter {
            name: "symbol, benchmark".into(),
            constraint: "must have the same length".into(),
        });
    }
    if as_of >= symbol.len() {
        return Err(TaError::InvalidParameter {
            name: "as_of".into(),
            constraint: format!("must be < {}", symbol.len()),
        });
    }
    if as_of + 1 < RS_PERIODS[0] {
        return Err(TaError::InsufficientData {
            length: as_of + 1,
            required: RS_PERIODS[0],
        });
    }

    let mut weighted_pct = 0.0_f64;
    let mut total_weight = 0.0_f64;
    for (i, &period) in RS_PERIODS.iter().enumerate() {
        if as_of + 1 < period {
            continue;
        }
        let s_end = symbol[as_of];
        let s_start = symbol[as_of + 1 - period];
        let b_end = benchmark[as_of];
        let b_start = benchmark[as_of + 1 - period];
        if s_start <= 0.0 || b_start <= 0.0 {
            continue;
        }
        let s_ret = s_end / s_start - 1.0;
        let b_ret = b_end / b_start - 1.0;
        weighted_pct += (s_ret - b_ret) * RS_WEIGHTS[i];
        total_weight += RS_WEIGHTS[i];
    }

    if total_weight <= 0.0 {
        return Err(TaError::InsufficientData {
            length: as_of + 1,
            required: RS_PERIODS[0],
        });
    }
    // 把累计相对收益 [-1, +1] 映射到 [1, 99]
    // 每 100% 相对超额收益对应 +98 分（1 → 99 极差）
    let pct = weighted_pct.clamp(-0.99, 0.99);
    let score = ((pct + 1.0) * 49.0 + 1.0).round() as i32;
    Ok((score.clamp(1, 99)) as u8)
}

// ============================================================================
// RS 动量
// ============================================================================

/// RS 动量：每根 K 线的 RS 评分
///
/// 输出每根 K 线上的 RS 评分（1-99），便于时序上观察个股相对强度变化。
///
/// # Arguments
/// * `symbol`    - 个股收盘价序列
/// * `benchmark` - 基准收盘价序列
///
/// # Returns
/// `Array1<u8>` 长度 = `symbol.len()`,前 `RS_PERIODS[0]-1` 个 bar 为 0(预热期)
///
/// # Future-data safety
/// 每根 K 线的输出仅用 `symbol[..=t]` 与 `benchmark[..=t]` 数据。
pub fn rs_slope(symbol: &[f64], benchmark: &[f64]) -> Result<Array1<u8>> {
    if symbol.len() != benchmark.len() {
        return Err(TaError::InvalidParameter {
            name: "symbol, benchmark".into(),
            constraint: "must have the same length".into(),
        });
    }
    let n = symbol.len();
    validate_input(n, RS_PERIODS[0])?;
    let mut out = Array1::<u8>::zeros(n);
    for t in (RS_PERIODS[0] - 1)..n {
        // rs_rating 不返回 Err 时,我们也用 ? 透传错误
        out[t] = rs_rating(symbol, benchmark, t)?;
    }
    Ok(out)
}

/// RS 评分变化：每根 K 线相对 N 日前的 RS 差值（-99 ~ +99）
///
/// # Future-data safety
/// 每根 K 线 t 的输出仅用 `symbol[..=t]` 与 `benchmark[..=t]` 数据。
pub fn rs_momentum(symbol: &[f64], benchmark: &[f64], lookback: usize) -> Result<Array1<i16>> {
    if symbol.len() != benchmark.len() {
        return Err(TaError::InvalidParameter {
            name: "symbol, benchmark".into(),
            constraint: "must have the same length".into(),
        });
    }
    if lookback == 0 {
        return Err(TaError::InvalidParameter {
            name: "lookback".into(),
            constraint: "must be >= 1".into(),
        });
    }
    let n = symbol.len();
    validate_input(n, RS_PERIODS[0] + lookback)?;
    let mut out = Array1::<i16>::zeros(n);
    for t in (RS_PERIODS[0] - 1 + lookback)..n {
        let cur = rs_rating(symbol, benchmark, t)? as i16;
        let prev = rs_rating(symbol, benchmark, t - lookback)? as i16;
        out[t] = cur - prev;
    }
    Ok(out)
}

// ============================================================================
// 横截面 RS 排名
// ============================================================================

/// 横截面 RS 排名：对一组个股 vs 同一基准做排名
///
/// # Arguments
/// * `symbols`   - 一组个股的收盘价序列
/// * `benchmark` - 共同基准
/// * `as_of`     - 评估日索引
///
/// # Returns
/// `Vec<u8>` 每个元素的 1-99 排名（1 = 最弱, 99 = 最强）。
/// 注意：返回的索引顺序与 `symbols` 输入顺序一致。
///
/// # Future-data safety
/// 仅用 `data[..=as_of]` 数据,无未来函数。
///
/// # Errors
/// - `InsufficientData`: 数据长度不足
/// - `InvalidParameter`: 任一 `symbol.len() != benchmark.len()`
pub fn relative_strength_rank(
    symbols: &[&[f64]],
    benchmark: &[f64],
    as_of: usize,
) -> Result<Vec<u8>> {
    if symbols.is_empty() {
        return Ok(Vec::new());
    }
    let n = benchmark.len();
    if as_of >= n {
        return Err(TaError::InvalidParameter {
            name: "as_of".into(),
            constraint: format!("must be < {}", n),
        });
    }
    if as_of + 1 < RS_PERIODS[0] {
        return Err(TaError::InsufficientData {
            length: as_of + 1,
            required: RS_PERIODS[0],
        });
    }
    // 计算每个 symbol 在 as_of 的加权相对收益
    let mut returns: Vec<(usize, f64)> = Vec::with_capacity(symbols.len());
    for (idx, sym) in symbols.iter().enumerate() {
        if sym.len() != n {
            return Err(TaError::InvalidParameter {
                name: "symbol, benchmark".into(),
                constraint: "all symbols must match benchmark length".into(),
            });
        }
        let mut weighted_pct = 0.0;
        let mut total_weight = 0.0;
        for (i, &period) in RS_PERIODS.iter().enumerate() {
            if as_of + 1 < period {
                continue;
            }
            let s_start = sym[as_of + 1 - period];
            let b_start = benchmark[as_of + 1 - period];
            if s_start <= 0.0 || b_start <= 0.0 {
                continue;
            }
            let s_ret = sym[as_of] / s_start - 1.0;
            let b_ret = benchmark[as_of] / b_start - 1.0;
            weighted_pct += (s_ret - b_ret) * RS_WEIGHTS[i];
            total_weight += RS_WEIGHTS[i];
        }
        if total_weight > 0.0 {
            returns.push((idx, weighted_pct / total_weight));
        } else {
            returns.push((idx, 0.0));
        }
    }
    // 排序：收益从低到高
    returns.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    // 把最低收益的 symbol 标为 1,最高标为 99
    let m = returns.len();
    let mut ranks: Vec<u8> = vec![0; m];
    for (rank, &(orig_idx, _)) in returns.iter().enumerate() {
        // rank 0..m-1 → 1..99
        let score = if m <= 1 {
            50
        } else {
            ((rank as f64 / (m - 1) as f64) * 98.0 + 1.0).round() as i32
        };
        ranks[orig_idx] = score.clamp(1, 99) as u8;
    }
    Ok(ranks)
}

// ============================================================================
// 强势/弱势判断
// ============================================================================

/// 强势判断：当前 RS 评分是否超过给定阈值（默认 80）
///
/// 返回 `Array1<bool>`,`true` = 强势区域。
///
/// # Future-data safety
/// 每根 K 线 t 的输出仅用 `data[..=t]` 数据。
pub fn is_strong(symbol: &[f64], benchmark: &[f64], threshold: u8) -> Result<Array1<bool>> {
    let slope = rs_slope(symbol, benchmark)?;
    Ok(slope.mapv(|v| v >= threshold))
}

/// 弱势判断：当前 RS 评分是否低于给定阈值（默认 20）
///
/// 返回 `Array1<bool>`,`true` = 弱势区域。
///
/// # Future-data safety
/// 每根 K 线 t 的输出仅用 `data[..=t]` 数据。
pub fn is_weak(symbol: &[f64], benchmark: &[f64], threshold: u8) -> Result<Array1<bool>> {
    let slope = rs_slope(symbol, benchmark)?;
    Ok(slope.mapv(|v| v > 0 && v <= threshold))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn build_uptrend(start: f64, n: usize, pct: f64) -> Vec<f64> {
        // 几何级数: start * (1+pct)^i
        (0..n).map(|i| start * (1.0 + pct).powi(i as i32)).collect()
    }

    fn build_downtrend(start: f64, n: usize, pct: f64) -> Vec<f64> {
        (0..n).map(|i| start * (1.0 - pct).powi(i as i32)).collect()
    }

    #[test]
    fn test_rs_rating_strong_above_benchmark() {
        // symbol 涨 50%, benchmark 涨 10% → RS 高
        // 公式映射: 0% excess→50, 99% excess→99, 中等超额对应 70-85
        let n = 300;
        let symbol = build_uptrend(10.0, n, 0.005); // 强
        let benchmark = build_uptrend(10.0, n, 0.001); // 弱基准
        let rating = rs_rating(&symbol, &benchmark, n - 1).unwrap();
        assert!(
            rating >= 70,
            "strong symbol should have high RS, got {}",
            rating
        );
    }

    #[test]
    fn test_rs_rating_weak_below_benchmark() {
        // symbol 跌 30%, benchmark 涨 10% → RS 低
        let n = 300;
        let symbol = build_downtrend(10.0, n, 0.002);
        let benchmark = build_uptrend(10.0, n, 0.001);
        let rating = rs_rating(&symbol, &benchmark, n - 1).unwrap();
        assert!(
            rating <= 40,
            "weak symbol should have low RS, got {}",
            rating
        );
    }

    #[test]
    fn test_rs_rating_equal_to_benchmark() {
        // symbol 与 benchmark 走势一致 → RS 50 附近
        let n = 300;
        let symbol = build_uptrend(10.0, n, 0.001);
        let benchmark = build_uptrend(10.0, n, 0.001);
        let rating = rs_rating(&symbol, &benchmark, n - 1).unwrap();
        // 应该非常接近 50（50 ± 5）
        assert!(
            (rating as i32 - 50).abs() <= 5,
            "equal series should be near 50, got {}",
            rating
        );
    }

    #[test]
    fn test_rs_rating_as_of_bounds() {
        let data = vec![10.0; 50];
        assert!(rs_rating(&data, &data, 50).is_err()); // out of bounds
        assert!(rs_rating(&data, &data, 0).is_err()); // not enough data
    }

    #[test]
    fn test_rs_rating_length_mismatch() {
        let s = vec![10.0; 100];
        let b = vec![10.0; 50];
        assert!(rs_rating(&s, &b, 60).is_err());
    }

    #[test]
    fn test_rs_slope_basic() {
        let n = 300;
        let symbol = build_uptrend(10.0, n, 0.003);
        let benchmark = build_uptrend(10.0, n, 0.001);
        let slope = rs_slope(&symbol, &benchmark).unwrap();
        assert_eq!(slope.len(), n);
        // 前 RS_PERIODS[0]-1 个 bar 是 0（预热期）
        for i in 0..(RS_PERIODS[0] - 1) {
            assert_eq!(slope[i], 0, "warmup bar {} should be 0", i);
        }
        // 后面的 bar 应该 > 50
        assert!(
            slope[n - 1] > 50,
            "ending bar should be strong, got {}",
            slope[n - 1]
        );
    }

    #[test]
    fn test_rs_momentum_increasing() {
        // 早期与基准同步,后期大幅走强 → rs_momentum > 0
        let n = 400;
        let mut symbol = build_uptrend(10.0, 200, 0.001);
        symbol.extend(build_uptrend(symbol[199], 200, 0.005));
        let benchmark = build_uptrend(10.0, n, 0.001);
        let mom = rs_momentum(&symbol, &benchmark, 100).unwrap();
        // 最后一段应该有正动量
        assert!(
            mom[n - 1] > 0,
            "rs_momentum should be positive at end, got {}",
            mom[n - 1]
        );
    }

    #[test]
    fn test_relative_strength_rank_basic() {
        let n = 300;
        let strong1 = build_uptrend(10.0, n, 0.005);
        let strong2 = build_uptrend(10.0, n, 0.004);
        let neutral = build_uptrend(10.0, n, 0.001);
        let weak1 = build_downtrend(10.0, n, 0.001);
        let weak2 = build_downtrend(10.0, n, 0.002);
        let benchmark = build_uptrend(10.0, n, 0.001);
        let symbols: Vec<&[f64]> = vec![&strong1, &strong2, &neutral, &weak1, &weak2];
        let ranks = relative_strength_rank(&symbols, &benchmark, n - 1).unwrap();
        assert_eq!(ranks.len(), 5);
        // strong1 应该排最高（接近 99）
        assert!(ranks[0] >= 80, "strong1 should rank high, got {}", ranks[0]);
        // strong2 第二高
        assert!(ranks[1] > ranks[2], "strong2 should outrank neutral");
        // neutral 中位
        assert!(
            (ranks[2] as i32 - 50).abs() <= 10,
            "neutral should be middle, got {}",
            ranks[2]
        );
        // weak1 较低
        assert!(ranks[3] <= 30, "weak1 should rank low, got {}", ranks[3]);
        // weak2 最低
        assert!(ranks[4] < ranks[3], "weak2 should be lowest");
    }

    #[test]
    fn test_relative_strength_rank_empty() {
        let benchmark = vec![10.0; 100];
        let ranks = relative_strength_rank(&[], &benchmark, 50).unwrap();
        assert!(ranks.is_empty());
    }

    #[test]
    fn test_relative_strength_rank_bounds() {
        let benchmark = vec![10.0; 50];
        let symbol = vec![10.0; 50];
        let symbols: Vec<&[f64]> = vec![&symbol];
        // 长度不足
        assert!(relative_strength_rank(&symbols, &benchmark, 50).is_err());
        // as_of 越界
        assert!(relative_strength_rank(&symbols, &benchmark, 100).is_err());
    }

    #[test]
    fn test_is_strong_weak() {
        let n = 300;
        let strong = build_uptrend(10.0, n, 0.005);
        let weak = build_downtrend(10.0, n, 0.002);
        let benchmark = build_uptrend(10.0, n, 0.001);
        let is_s = is_strong(&strong, &benchmark, 70).unwrap();
        let is_w = is_weak(&weak, &benchmark, 40).unwrap();
        // 末尾 bar: strong=true, weak=true
        assert!(is_s[n - 1], "strong symbol end bar should be is_strong");
        assert!(is_w[n - 1], "weak symbol end bar should be is_weak");
    }
}
