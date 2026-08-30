//! Streaming (incremental) Math Operator 指标。
//!
//! 为 TA-Lib C 库的 9 个 Math Operator 函数（除 `maxindex`/`minindex` 索引函数外）
//! 提供 O(1) 摊销复杂度的 streaming 实现。
//!
//! ## 双输入算术函数（无状态）
//!
//! 下列函数接受两个等长输入数组并逐元素运算，streaming 版本通过
//! [`StreamingAdd::next_pair`] / [`StreamingSub::next_pair`] /
//! [`StreamingMult::next_pair`] / [`StreamingDiv::next_pair`] 方法输入。
//!
//! 由于 [`StreamingIndicator::next`] 签名只接受单输入，这些指标的 `next` 方法
//! 是**占位实现**——它仅返回缓存的最近值，不会更新状态。**请使用 `next_pair`**
//! 来获取实际结果。
//!
//! - [`StreamingAdd`]: 逐元素相加 `a + b`
//! - [`StreamingSub`]: 逐元素相减 `a - b`
//! - [`StreamingMult`]: 逐元素相乘 `a * b`
//! - [`StreamingDiv`]: 逐元素相除 `a / b`（除零返回 `None`）
//!
//! ## 周期差分（stateful, 单输入）
//!
//! - [`StreamingMinus`]: `data[i] - data[i - period]`，前 `period` 个值返回 `None`
//!
//! ## 窗口统计函数（stateful, 单输入）
//!
//! - [`StreamingMax`]: 滚动窗口最大值
//! - [`StreamingMin`]: 滚动窗口最小值
//! - [`StreamingSum`]: 滚动窗口求和
//!
//! 所有窗口函数在累积满 `period` 个数据点之前返回 `None`。

use std::collections::VecDeque;

use crate::{impl_indicator_meta, impl_standard_methods};
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

// =====================================================================
// 双输入算术函数（无状态）
// =====================================================================

/// 逐元素相加 ADD（streaming 版本）。
///
/// 计算 `a + b`。无状态：每次 `next_pair` 调用立刻返回结果。
///
/// **用法**：本指标的主入口是 [`StreamingAdd::next_pair`]。
/// [`StreamingAdd`] 实现的 [`StreamingIndicator::next`] 是占位方法——它不会处理
/// 双输入，仅返回缓存的最近值。
///
/// # 示例
/// ```rust
/// use alpha_ta_core::streaming::indicators::StreamingAdd;
/// let mut add = StreamingAdd::new();
/// assert_eq!(add.next_pair(1.0, 2.0), Some(3.0));
/// assert_eq!(add.next_pair(10.0, 20.0), Some(30.0));
/// ```
#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingAdd {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingAdd {
    pub fn new() -> Self {
        Self::default()
    }

    /// 接收两个输入并返回 `a + b`。
    #[inline]
    pub fn next_pair(&mut self, a: f64, b: f64) -> Option<f64> {
        self.count += 1;
        let v = a + b;
        self.last_value = Some(v);
        Some(v)
    }
}

impl StreamingIndicator for StreamingAdd {
    #[inline]
    fn next(&mut self, _input: f64) -> Option<f64> {
        // 双输入指标：请使用 next_pair(a, b)。此处仅返回缓存值。
        self.last_value
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count > 0
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingAdd {
    fn name() -> &'static str { "ADD" }
    fn category() -> &'static str { "math_operators" }
    fn description() -> &'static str { "Vector add: a + b" }
    fn warm_up_period(&self) -> usize { 0 }
}

/// 逐元素相减 SUB（streaming 版本）。
///
/// 计算 `a - b`。无状态：每次 `next_pair` 调用立刻返回结果。
///
/// # 示例
/// ```rust
/// use alpha_ta_core::streaming::indicators::StreamingSub;
/// let mut sub = StreamingSub::new();
/// assert_eq!(sub.next_pair(10.0, 3.0), Some(7.0));
/// assert_eq!(sub.next_pair(5.0, 8.0), Some(-3.0));
/// ```
#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingSub {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingSub {
    pub fn new() -> Self {
        Self::default()
    }

    /// 接收两个输入并返回 `a - b`。
    #[inline]
    pub fn next_pair(&mut self, a: f64, b: f64) -> Option<f64> {
        self.count += 1;
        let v = a - b;
        self.last_value = Some(v);
        Some(v)
    }
}

impl StreamingIndicator for StreamingSub {
    #[inline]
    fn next(&mut self, _input: f64) -> Option<f64> {
        // 双输入指标：请使用 next_pair(a, b)。此处仅返回缓存值。
        self.last_value
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count > 0
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingSub {
    fn name() -> &'static str { "SUB" }
    fn category() -> &'static str { "math_operators" }
    fn description() -> &'static str { "Vector subtract: a - b" }
    fn warm_up_period(&self) -> usize { 0 }
}

/// 逐元素相乘 MULT（streaming 版本）。
///
/// 计算 `a * b`。无状态：每次 `next_pair` 调用立刻返回结果。
///
/// # 示例
/// ```rust
/// use alpha_ta_core::streaming::indicators::StreamingMult;
/// let mut mult = StreamingMult::new();
/// assert_eq!(mult.next_pair(3.0, 4.0), Some(12.0));
/// assert_eq!(mult.next_pair(-2.0, 5.0), Some(-10.0));
/// ```
#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMult {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingMult {
    pub fn new() -> Self {
        Self::default()
    }

    /// 接收两个输入并返回 `a * b`。
    #[inline]
    pub fn next_pair(&mut self, a: f64, b: f64) -> Option<f64> {
        self.count += 1;
        let v = a * b;
        self.last_value = Some(v);
        Some(v)
    }
}

impl StreamingIndicator for StreamingMult {
    #[inline]
    fn next(&mut self, _input: f64) -> Option<f64> {
        // 双输入指标：请使用 next_pair(a, b)。此处仅返回缓存值。
        self.last_value
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count > 0
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingMult {
    fn name() -> &'static str { "MULT" }
    fn category() -> &'static str { "math_operators" }
    fn description() -> &'static str { "Vector multiply: a * b" }
    fn warm_up_period(&self) -> usize { 0 }
}

/// 逐元素相除 DIV（streaming 版本）。
///
/// 计算 `a / b`。无状态：每次 `next_pair` 调用立刻返回结果。
///
/// **除零行为**：当 `b == 0.0` 时返回 `None`（与 batch 版本返回 `NaN` 略有不同——
/// streaming 接口用 `None` 表示结果无效，保持与其它 streaming 指标一致的语义）。
///
/// # 示例
/// ```rust
/// use alpha_ta_core::streaming::indicators::StreamingDiv;
/// let mut div = StreamingDiv::new();
/// assert_eq!(div.next_pair(10.0, 2.0), Some(5.0));
/// assert_eq!(div.next_pair(1.0, 0.0), None); // 除零
/// ```
#[derive(Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingDiv {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingDiv {
    pub fn new() -> Self {
        Self::default()
    }

    /// 接收两个输入并返回 `a / b`；除零返回 `None`。
    #[inline]
    pub fn next_pair(&mut self, a: f64, b: f64) -> Option<f64> {
        self.count += 1;
        if b == 0.0 {
            self.last_value = None;
            return None;
        }
        let v = a / b;
        self.last_value = Some(v);
        Some(v)
    }
}

impl StreamingIndicator for StreamingDiv {
    #[inline]
    fn next(&mut self, _input: f64) -> Option<f64> {
        // 双输入指标：请使用 next_pair(a, b)。此处仅返回缓存值。
        self.last_value
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count > 0
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingDiv {
    fn name() -> &'static str { "DIV" }
    fn category() -> &'static str { "math_operators" }
    fn description() -> &'static str { "Vector divide: a / b (None on divide by zero)" }
    fn warm_up_period(&self) -> usize { 0 }
}

// =====================================================================
// 周期差分 MINUS（stateful, 单输入）
// =====================================================================

/// 周期差分 MINUS（streaming 版本）。
///
/// 计算 `data[i] - data[i - period]`。维护一个长度为 `period` 的循环延迟队列：
/// 在累积满 `period` 个数据点之前返回 `None`；之后返回当前值与 `period` 步前值的差。
///
/// # 示例
/// ```rust
/// use alpha_ta_core::streaming::{StreamingIndicator, indicators::StreamingMinus};
/// let mut m = StreamingMinus::new(2);
/// assert_eq!(m.next(1.0), None);  // 累积中
/// assert_eq!(m.next(2.0), None);  // 累积中
/// assert_eq!(m.next(4.0), Some(3.0));  // 4 - 1
/// assert_eq!(m.next(7.0), Some(5.0));  // 7 - 2
/// ```
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMinus {
    period: usize,
    buffer: Vec<f64>,
    head: usize,
    len: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingMinus {
    /// 创建一个周期为 `period` 的差分指标。
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: vec![0.0; period],
            head: 0,
            len: 0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingMinus {
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let result = if self.len < self.period {
            // 累积阶段：写入 (head + len) % period
            self.buffer[(self.head + self.len) % self.period] = input;
            self.len += 1;
            None
        } else {
            // 已就绪：覆盖最旧元素并推进 head
            let old = self.buffer[self.head];
            self.buffer[self.head] = input;
            self.head = (self.head + 1) % self.period;
            Some(input - old)
        };
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.len >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingMinus, "MINUS", "math_operators", "Periodic difference: data[i] - data[i - period]");

// =====================================================================
// 窗口统计函数（stateful, 单输入）
// =====================================================================

/// 滚动窗口最大值 MAX（streaming 版本）。
///
/// 维护最近 `period` 个输入值，返回其中最大值。O(period) 简单扫描实现。
///
/// # 示例
/// ```rust
/// use alpha_ta_core::streaming::{StreamingIndicator, indicators::StreamingMax};
/// let mut m = StreamingMax::new(3);
/// assert_eq!(m.next(3.0), None);
/// assert_eq!(m.next(1.0), None);
/// assert_eq!(m.next(4.0), Some(4.0));  // max(3,1,4)
/// assert_eq!(m.next(1.0), Some(4.0));  // max(1,4,1)
/// assert_eq!(m.next(5.0), Some(5.0));  // max(4,1,5)
/// ```
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMax {
    period: usize,
    deque: VecDeque<(usize, f64)>,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingMax {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            deque: VecDeque::with_capacity(period),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingMax {
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;

        while self.deque.back().map_or(false, |&(_, v)| v <= input) {
            self.deque.pop_back();
        }

        self.deque.push_back((self.count - 1, input));

        while self.deque.front().map_or(false, |&(pos, _)| pos + self.period < self.count) {
            self.deque.pop_front();
        }

        if self.count < self.period {
            self.last_value = None;
            return None;
        }

        let result = self.deque.front().map(|&(_, v)| v);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.deque.clear();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingMax, "MAX", "math_operators", "Rolling window maximum");

/// 滚动窗口最小值 MIN（streaming 版本）。
///
/// 维护最近 `period` 个输入值，返回其中最小值。O(period) 简单扫描实现。
///
/// # 示例
/// ```rust
/// use alpha_ta_core::streaming::{StreamingIndicator, indicators::StreamingMin};
/// let mut m = StreamingMin::new(3);
/// assert_eq!(m.next(3.0), None);
/// assert_eq!(m.next(1.0), None);
/// assert_eq!(m.next(4.0), Some(1.0));  // min(3,1,4)
/// assert_eq!(m.next(1.0), Some(1.0));  // min(1,4,1)
/// ```
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMin {
    period: usize,
    deque: VecDeque<(usize, f64)>,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingMin {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            deque: VecDeque::with_capacity(period),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingMin {
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;

        while self.deque.back().map_or(false, |&(_, v)| v >= input) {
            self.deque.pop_back();
        }

        self.deque.push_back((self.count - 1, input));

        while self.deque.front().map_or(false, |&(pos, _)| pos + self.period < self.count) {
            self.deque.pop_front();
        }

        if self.count < self.period {
            self.last_value = None;
            return None;
        }

        let result = self.deque.front().map(|&(_, v)| v);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.deque.clear();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingMin, "MIN", "math_operators", "Rolling window minimum");

/// 滚动窗口求和 SUM（streaming 版本）。
///
/// 维护最近 `period` 个输入值的求和。O(1) 摊销增量更新：每步减去最旧值、加上最新值。
///
/// # 示例
/// ```rust
/// use alpha_ta_core::streaming::{StreamingIndicator, indicators::StreamingSum};
/// let mut s = StreamingSum::new(3);
/// assert_eq!(s.next(1.0), None);
/// assert_eq!(s.next(2.0), None);
/// assert_eq!(s.next(3.0), Some(6.0));   // 1+2+3
/// assert_eq!(s.next(4.0), Some(9.0));   // 2+3+4
/// assert_eq!(s.next(5.0), Some(12.0));  // 3+4+5
/// ```
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingSum {
    period: usize,
    buffer: Vec<f64>,
    head: usize,
    len: usize,
    sum: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingSum {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: vec![0.0; period],
            head: 0,
            len: 0,
            sum: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingSum {
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        if self.len < self.period {
            self.buffer[(self.head + self.len) % self.period] = input;
            self.len += 1;
            self.sum += input;
        } else {
            let old = self.buffer[self.head];
            self.buffer[self.head] = input;
            self.head = (self.head + 1) % self.period;
            self.sum += input - old;
        }
        let result = if self.is_ready() {
            Some(self.sum)
        } else {
            None
        };
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.sum = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.len >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingSum, "SUM", "math_operators", "Rolling window sum");

// =====================================================================
// 单元测试
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------- 双输入算术函数 -------------------------

    #[test]
    fn test_streaming_add() {
        let mut add = StreamingAdd::new();
        assert_eq!(add.next_pair(1.0, 2.0), Some(3.0));
        assert_eq!(add.next_pair(-5.0, 5.0), Some(0.0));
        assert_eq!(add.next_pair(0.0, 0.0), Some(0.0));
        assert_eq!(add.value(), Some(0.0));
        assert_eq!(add.count(), 3);
        assert!(add.is_ready());
    }

    #[test]
    fn test_streaming_add_placeholder_next() {
        // `next` 是占位：调用它不会更新状态，仅返回缓存值。
        let mut add = StreamingAdd::new();
        assert_eq!(add.next(42.0), None);
        assert_eq!(add.count(), 0);
        assert!(!add.is_ready());
        add.next_pair(1.0, 1.0);
        // 之后 `next` 返回缓存值
        assert_eq!(add.next(999.0), Some(2.0));
        assert_eq!(add.count(), 1); // count 不变
    }

    #[test]
    fn test_streaming_add_reset() {
        let mut add = StreamingAdd::new();
        add.next_pair(1.0, 2.0);
        add.reset();
        assert_eq!(add.value(), None);
        assert_eq!(add.count(), 0);
        assert!(!add.is_ready());
    }

    #[test]
    fn test_streaming_add_vs_batch() {
        let a: Vec<f64> = (0..50).map(|i| i as f64 * 0.5).collect();
        let b: Vec<f64> = (0..50).map(|i| i as f64 * 0.25).collect();
        let batch = crate::indicators::math_operators::add(&a, &b).unwrap();
        let mut s = StreamingAdd::new();
        for i in 0..a.len() {
            let v = s.next_pair(a[i], b[i]).unwrap();
            assert!((v - batch[i]).abs() < 1e-12);
        }
    }

    #[test]
    fn test_streaming_sub() {
        let mut sub = StreamingSub::new();
        assert_eq!(sub.next_pair(10.0, 3.0), Some(7.0));
        assert_eq!(sub.next_pair(0.0, 5.0), Some(-5.0));
        assert_eq!(sub.next_pair(2.0, 2.0), Some(0.0));
        assert_eq!(sub.count(), 3);
    }

    #[test]
    fn test_streaming_sub_vs_batch() {
        let a: Vec<f64> = (0..30).map(|i| i as f64).collect();
        let b: Vec<f64> = (0..30).map(|i| (i as f64).sin()).collect();
        let batch = crate::indicators::math_operators::sub(&a, &b).unwrap();
        let mut s = StreamingSub::new();
        for i in 0..a.len() {
            let v = s.next_pair(a[i], b[i]).unwrap();
            assert!((v - batch[i]).abs() < 1e-12);
        }
    }

    #[test]
    fn test_streaming_mult() {
        let mut mult = StreamingMult::new();
        assert_eq!(mult.next_pair(3.0, 4.0), Some(12.0));
        assert_eq!(mult.next_pair(-2.0, 5.0), Some(-10.0));
        assert_eq!(mult.next_pair(0.0, 100.0), Some(0.0));
    }

    #[test]
    fn test_streaming_mult_vs_batch() {
        let a: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        let b: Vec<f64> = (1..=20).map(|i| (i as f64) * 0.1).collect();
        let batch = crate::indicators::math_operators::mult(&a, &b).unwrap();
        let mut s = StreamingMult::new();
        for i in 0..a.len() {
            let v = s.next_pair(a[i], b[i]).unwrap();
            assert!((v - batch[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn test_streaming_div() {
        let mut div = StreamingDiv::new();
        assert_eq!(div.next_pair(10.0, 2.0), Some(5.0));
        assert_eq!(div.next_pair(1.0, 4.0), Some(0.25));
        // 除零返回 None
        assert_eq!(div.next_pair(1.0, 0.0), None);
        // 除零之后，缓存值变为 None
        assert_eq!(div.value(), None);
        // 恢复有效运算
        assert_eq!(div.next_pair(6.0, 3.0), Some(2.0));
        assert_eq!(div.value(), Some(2.0));
    }

    #[test]
    fn test_streaming_div_vs_batch() {
        let a: Vec<f64> = (1..=20).map(|i| (i as f64) * 2.0).collect();
        let b: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        let batch = crate::indicators::math_operators::div(&a, &b).unwrap();
        let mut s = StreamingDiv::new();
        for i in 0..a.len() {
            let v = s.next_pair(a[i], b[i]);
            if batch[i].is_nan() {
                assert!(v.is_none());
            } else {
                let v = v.unwrap();
                assert!((v - batch[i]).abs() < 1e-10, "mismatch at {i}: {v} vs {}", batch[i]);
            }
        }
    }

    // ------------------------- StreamingMinus -------------------------

    #[test]
    fn test_streaming_minus_basic() {
        let mut m = StreamingMinus::new(2);
        // 累积中
        assert_eq!(m.next(1.0), None);
        assert_eq!(m.next(2.0), None);
        // 已就绪
        assert_eq!(m.next(4.0), Some(3.0));  // 4 - 1
        assert_eq!(m.next(7.0), Some(5.0));  // 7 - 2
        assert_eq!(m.next(11.0), Some(7.0)); // 11 - 4
    }

    #[test]
    fn test_streaming_minus_period_one() {
        let mut m = StreamingMinus::new(1);
        assert_eq!(m.next(10.0), None);
        assert_eq!(m.next(13.0), Some(3.0));  // 13 - 10
        assert_eq!(m.next(15.0), Some(2.0));  // 15 - 13
    }

    #[test]
    fn test_streaming_minus_reset() {
        let mut m = StreamingMinus::new(3);
        m.next(1.0);
        m.next(2.0);
        m.next(3.0);
        let v = m.next(10.0);
        assert!(v.is_some());
        m.reset();
        assert!(!m.is_ready());
        assert_eq!(m.count(), 0);
        assert_eq!(m.value(), None);
        // 重置后从头开始
        assert_eq!(m.next(5.0), None);
        assert_eq!(m.next(7.0), None);
        assert_eq!(m.next(9.0), None);
        assert_eq!(m.next(20.0), Some(15.0)); // 20 - 5
    }

    #[test]
    fn test_streaming_minus_vs_batch() {
        let data: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        let period = 3;
        let batch = crate::indicators::math_operators::minus(&data, period).unwrap();
        let mut s = StreamingMinus::new(period);
        for i in 0..data.len() {
            let v = s.next(data[i]);
            if batch[i].is_nan() {
                assert!(v.is_none());
            } else {
                let v = v.unwrap();
                assert!((v - batch[i]).abs() < 1e-10, "mismatch at {i}: {v} vs {}", batch[i]);
            }
        }
    }

    // ------------------------- StreamingMax -------------------------

    #[test]
    fn test_streaming_max_basic() {
        let data = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        let mut m = StreamingMax::new(3);
        let mut last = None;
        for v in &data {
            last = m.next(*v);
        }
        // 最后窗口 [9, 2, 6] -> 9
        assert_eq!(last, Some(9.0));
    }

    #[test]
    fn test_streaming_max_warmup() {
        let mut m = StreamingMax::new(3);
        assert_eq!(m.next(3.0), None);
        assert_eq!(m.next(1.0), None);
        assert_eq!(m.next(4.0), Some(4.0));
        assert_eq!(m.next(1.0), Some(4.0));
        assert_eq!(m.next(5.0), Some(5.0));
    }

    #[test]
    fn test_streaming_max_period_one() {
        let mut m = StreamingMax::new(1);
        assert_eq!(m.next(5.0), Some(5.0));
        assert_eq!(m.next(2.0), Some(2.0));
        assert_eq!(m.next(7.0), Some(7.0));
    }

    #[test]
    fn test_streaming_max_reset() {
        let mut m = StreamingMax::new(2);
        m.next(5.0);
        m.next(10.0);
        assert!(m.is_ready());
        m.reset();
        assert!(!m.is_ready());
        assert_eq!(m.count(), 0);
        assert_eq!(m.value(), None);
    }

    #[test]
    fn test_streaming_max_vs_batch() {
        let data: Vec<f64> = (0..50).map(|i| 50.0 + (i as f64 * 0.3).sin() * 10.0).collect();
        let period = 7;
        let batch = crate::indicators::math_operators::max(&data, period).unwrap();
        let mut s = StreamingMax::new(period);
        for i in 0..data.len() {
            let v = s.next(data[i]);
            if batch[i].is_nan() {
                assert!(v.is_none());
            } else {
                let v = v.unwrap();
                assert!((v - batch[i]).abs() < 1e-10, "mismatch at {i}: {v} vs {}", batch[i]);
            }
        }
    }

    // ------------------------- StreamingMin -------------------------

    #[test]
    fn test_streaming_min_basic() {
        let data = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        let mut m = StreamingMin::new(3);
        let mut last = None;
        for v in &data {
            last = m.next(*v);
        }
        // 最后窗口 [9, 2, 6] -> 2
        assert_eq!(last, Some(2.0));
    }

    #[test]
    fn test_streaming_min_warmup() {
        let mut m = StreamingMin::new(3);
        assert_eq!(m.next(3.0), None);
        assert_eq!(m.next(1.0), None);
        assert_eq!(m.next(4.0), Some(1.0));
        assert_eq!(m.next(1.0), Some(1.0));
    }

    #[test]
    fn test_streaming_min_period_one() {
        let mut m = StreamingMin::new(1);
        assert_eq!(m.next(5.0), Some(5.0));
        assert_eq!(m.next(2.0), Some(2.0));
    }

    #[test]
    fn test_streaming_min_reset() {
        let mut m = StreamingMin::new(2);
        m.next(5.0);
        m.next(10.0);
        assert!(m.is_ready());
        m.reset();
        assert!(!m.is_ready());
        assert_eq!(m.count(), 0);
    }

    #[test]
    fn test_streaming_min_vs_batch() {
        let data: Vec<f64> = (0..50).map(|i| 50.0 + (i as f64 * 0.3).sin() * 10.0).collect();
        let period = 5;
        let batch = crate::indicators::math_operators::min(&data, period).unwrap();
        let mut s = StreamingMin::new(period);
        for i in 0..data.len() {
            let v = s.next(data[i]);
            if batch[i].is_nan() {
                assert!(v.is_none());
            } else {
                let v = v.unwrap();
                assert!((v - batch[i]).abs() < 1e-10, "mismatch at {i}: {v} vs {}", batch[i]);
            }
        }
    }

    // ------------------------- StreamingSum -------------------------

    #[test]
    fn test_streaming_sum_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let mut s = StreamingSum::new(3);
        let results: Vec<Option<f64>> = data.iter().map(|v| s.next(*v)).collect();
        assert_eq!(results, vec![None, None, Some(6.0), Some(9.0), Some(12.0)]);
    }

    #[test]
    fn test_streaming_sum_negative() {
        let mut s = StreamingSum::new(2);
        assert_eq!(s.next(-1.0), None);
        assert_eq!(s.next(-2.0), Some(-3.0));
        assert_eq!(s.next(-3.0), Some(-5.0));
    }

    #[test]
    fn test_streaming_sum_period_one() {
        let mut s = StreamingSum::new(1);
        assert_eq!(s.next(5.0), Some(5.0));
        assert_eq!(s.next(2.0), Some(2.0));
        assert_eq!(s.next(7.0), Some(7.0));
    }

    #[test]
    fn test_streaming_sum_reset() {
        let mut s = StreamingSum::new(2);
        s.next(1.0);
        s.next(2.0);
        assert!(s.is_ready());
        s.reset();
        assert!(!s.is_ready());
        assert_eq!(s.count(), 0);
        assert_eq!(s.value(), None);
        // 重置后从干净状态开始
        assert_eq!(s.next(10.0), None);
        assert_eq!(s.next(20.0), Some(30.0));
    }

    #[test]
    fn test_streaming_sum_vs_batch() {
        let data: Vec<f64> = (1..=30).map(|i| (i as f64) * 0.5).collect();
        let period = 4;
        let batch = crate::indicators::math_operators::sum(&data, period).unwrap();
        let mut s = StreamingSum::new(period);
        for i in 0..data.len() {
            let v = s.next(data[i]);
            if batch[i].is_nan() {
                assert!(v.is_none());
            } else {
                let v = v.unwrap();
                assert!((v - batch[i]).abs() < 1e-10, "mismatch at {i}: {v} vs {}", batch[i]);
            }
        }
    }

    // ------------------------- Meta -------------------------

    #[test]
    fn test_meta_values() {
        assert_eq!(StreamingAdd::name(), "ADD");
        assert_eq!(StreamingAdd::category(), "math_operators");
        assert_eq!(StreamingAdd::new().warm_up_period(), 0);

        assert_eq!(StreamingSub::name(), "SUB");
        assert_eq!(StreamingSub::category(), "math_operators");
        assert_eq!(StreamingSub::new().warm_up_period(), 0);

        assert_eq!(StreamingMult::name(), "MULT");
        assert_eq!(StreamingMult::category(), "math_operators");
        assert_eq!(StreamingMult::new().warm_up_period(), 0);

        assert_eq!(StreamingDiv::name(), "DIV");
        assert_eq!(StreamingDiv::category(), "math_operators");
        assert_eq!(StreamingDiv::new().warm_up_period(), 0);

        assert_eq!(StreamingMinus::name(), "MINUS");
        assert_eq!(StreamingMinus::category(), "math_operators");
        assert_eq!(StreamingMinus::new(5).warm_up_period(), 5);

        assert_eq!(StreamingMax::name(), "MAX");
        assert_eq!(StreamingMax::category(), "math_operators");
        assert_eq!(StreamingMax::new(3).warm_up_period(), 3);

        assert_eq!(StreamingMin::name(), "MIN");
        assert_eq!(StreamingMin::category(), "math_operators");
        assert_eq!(StreamingMin::new(3).warm_up_period(), 3);

        assert_eq!(StreamingSum::name(), "SUM");
        assert_eq!(StreamingSum::category(), "math_operators");
        assert_eq!(StreamingSum::new(3).warm_up_period(), 3);
    }
}
