//! proptest 不变量断言 helper（Phase 9）
//!
//! 本模块为 `core/tests/property_tests.rs` 与
//! `core/tests/property_volatility_ext.rs` 提供可复用的不变量断言函数，
//! 避免在每个 proptest 块内重复同样的断言循环。
//!
//! 设计原则：
//! - 每个 helper 接收原始输出数组与（可选的）参数描述
//! - 内部使用 `proptest::assert!` 风格的 `bool` 返回，便于嵌入 `proptest! { ... }` 块
//! - 所有断言允许容差（默认 1e-9 绝对值 + 1e-12 相对值）
//!
//! 命名规范：`assert_<indicator>_<invariant>`。
//! 例如 `assert_bollinger_envelope` 验证 lower ≤ middle ≤ upper。

use crate::common::golden_loader::DEFAULT_TOLERANCE;

/// 数值容差：与浮点比较相同。
pub const INVARIANT_TOLERANCE: f64 = 1e-9;

/// 验证 Bollinger 包络不变量：lower ≤ middle ≤ upper。
///
/// 跳过 NaN 元素（`init_output` 在 warm-up 期间填充 NaN）。
///
/// # Panics
/// - 当任一索引违反包络关系时 panic
pub fn assert_bollinger_envelope(
    upper: &[f64],
    middle: &[f64],
    lower: &[f64],
) -> Result<(), String> {
    assert_eq!(upper.len(), middle.len(), "upper/middle length mismatch");
    assert_eq!(middle.len(), lower.len(), "middle/lower length mismatch");
    for i in 0..upper.len() {
        if middle[i].is_nan() || upper[i].is_nan() || lower[i].is_nan() {
            continue;
        }
        if !(lower[i] <= middle[i] + INVARIANT_TOLERANCE) {
            return Err(format!(
                "bollinger envelope violated at i={i}: lower={} > middle={}",
                lower[i], middle[i]
            ));
        }
        if !(middle[i] <= upper[i] + INVARIANT_TOLERANCE) {
            return Err(format!(
                "bollinger envelope violated at i={i}: middle={} > upper={}",
                middle[i], upper[i]
            ));
        }
    }
    Ok(())
}

/// 验证 RSI 范围不变量：0 ≤ rsi ≤ 100。
///
/// 跳过 NaN 元素。
pub fn assert_rsi_bounded(rsi: &[f64]) -> Result<(), String> {
    for (i, &v) in rsi.iter().enumerate() {
        if v.is_nan() {
            continue;
        }
        if !(-INVARIANT_TOLERANCE..=100.0 + INVARIANT_TOLERANCE).contains(&v) {
            return Err(format!("rsi out of [0, 100] at i={i}: {v}"));
        }
    }
    Ok(())
}

/// 验证 ATR 非负不变量：atr ≥ 0。
pub fn assert_atr_nonneg(atr: &[f64]) -> Result<(), String> {
    for (i, &v) in atr.iter().enumerate() {
        if v.is_nan() {
            continue;
        }
        if v < -INVARIANT_TOLERANCE {
            return Err(format!("atr negative at i={i}: {v}"));
        }
    }
    Ok(())
}

/// 验证 SMA 单调性不变量：输入递增时输出非递减。
///
/// `sma` 是固定窗口的简单平均，对于严格递增的输入其输出也必须
/// 满足 `sma[i+1] >= sma[i] - tol`。
pub fn assert_sma_monotonic(sma: &[f64]) -> Result<(), String> {
    for i in 0..sma.len().saturating_sub(1) {
        if sma[i].is_nan() || sma[i + 1].is_nan() {
            continue;
        }
        if sma[i + 1] < sma[i] - INVARIANT_TOLERANCE {
            return Err(format!(
                "sma not monotonic: sma[{i}]={} > sma[{}]={}",
                sma[i],
                i + 1,
                sma[i + 1]
            ));
        }
    }
    Ok(())
}

/// 验证 EMA 收敛性不变量：足够多步之后输出为有限值（不应该是 NaN）。
///
/// EMA 的"收敛窗口"通常为 `3 * (period + 1)`。该 helper 检查从
/// `warmup_offset` 起所有值都是有限的。
pub fn assert_ema_converges(ema: &[f64], warmup_offset: usize) -> Result<(), String> {
    for (i, &v) in ema.iter().enumerate().skip(warmup_offset) {
        if !v.is_finite() {
            return Err(format!("ema did not converge at i={i}: {v}"));
        }
    }
    Ok(())
}

/// 验证 MACD signal 内部一致性：histogram = macd - signal。
///
/// 跳过任一端为 NaN 的位置。
pub fn assert_macd_invariant(macd: &[f64], signal: &[f64], hist: &[f64]) -> Result<(), String> {
    assert_eq!(macd.len(), signal.len(), "macd/signal length mismatch");
    assert_eq!(signal.len(), hist.len(), "signal/hist length mismatch");
    for i in 0..macd.len() {
        if macd[i].is_nan() || signal[i].is_nan() || hist[i].is_nan() {
            continue;
        }
        let expected = macd[i] - signal[i];
        if (hist[i] - expected).abs() > INVARIANT_TOLERANCE {
            return Err(format!(
                "macd histogram invariant violated at i={i}: hist={} vs macd-signal={}",
                hist[i], expected
            ));
        }
    }
    Ok(())
}

/// 验证两个长度相等的浮点数组在指定容差内近似相等。
pub fn assert_approx_eq_slice(a: &[f64], b: &[f64], tol: f64) -> Result<(), String> {
    assert_eq!(a.len(), b.len(), "slice length mismatch");
    for (i, (&x, &y)) in a.iter().zip(b.iter()).enumerate() {
        if x.is_nan() && y.is_nan() {
            continue;
        }
        if (x - y).abs() > tol {
            return Err(format!(
                "slice mismatch at i={i}: {x} vs {y} (tol={tol})"
            ));
        }
    }
    Ok(())
}

/// 默认容差的近似比较便捷版本。
pub fn assert_approx_eq_default(a: &[f64], b: &[f64]) -> Result<(), String> {
    assert_approx_eq_slice(a, b, DEFAULT_TOLERANCE)
}
