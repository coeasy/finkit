//! Math Operator 指标（TA-Lib Math Operators 兼容实现）
//!
//! 本模块提供与 TA-Lib C 库 100% 兼容的 11 个 Math Operator 函数：
//!
//! ## 双输入算术函数
//! - [`add`]: 逐元素相加 `a + b`
//! - [`sub`]: 逐元素相减 `a - b`
//! - [`mult`]: 逐元素相乘 `a * b`
//! - [`div`]: 逐元素相除 `a / b`（除零返回 NaN）
//! - [`minus`]: 周期差分 `data[i] - data[i - period]`
//!
//! ## 窗口统计函数
//! - [`max`]: 滚动窗口最大值（前 `period - 1` 个值为 NaN）
//! - [`min`]: 滚动窗口最小值（前 `period - 1` 个值为 NaN）
//! - [`sum`]: 滚动窗口求和（前 `period - 1` 个值为 NaN）
//!
//! ## 索引函数
//! - [`maxindex`]: 滚动窗口内最大值的索引（相对窗口起点）
//! - [`minindex`]: 滚动窗口内最小值的索引（相对窗口起点）

use crate::error::{Result, TaError};
use crate::utils::{init_output, validate_input};
use ndarray::Array1;
use std::collections::VecDeque;

/// 逐元素相加 ADD：返回 `a + b`
///
/// 与 TA-Lib C 库的 `TA_ADD` 函数完全等价。计算两个等长数组的逐元素加法。
///
/// # 参数
/// * `a` - 第一个输入数组
/// * `b` - 第二个输入数组
///
/// # 返回值
/// `Result<Array1<f64>>` - 长度为 `a.len()` 的数组，每个元素为 `a[i] + b[i]`。
///
/// # 错误
/// * 当 `a.len() != b.len()` 时返回 `TaError::InvalidParameter`
/// * 当任一输入长度为 0 时返回 `TaError::EmptyInput`
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_operators::add;
/// let a = vec![1.0, 2.0, 3.0];
/// let b = vec![10.0, 20.0, 30.0];
/// let r = add(&a, &b).unwrap();
/// assert_eq!(r.to_vec(), vec![11.0, 22.0, 33.0]);
/// ```
pub fn add(a: &[f64], b: &[f64]) -> Result<Array1<f64>> {
    validate_two_inputs(a, b)?;
    Ok(a.iter().zip(b.iter()).map(|(x, y)| x + y).collect())
}

/// 逐元素相减 SUB：返回 `a - b`
///
/// 与 TA-Lib C 库的 `TA_SUB` 函数完全等价。计算两个等长数组的逐元素减法。
///
/// # 参数
/// * `a` - 被减数数组
/// * `b` - 减数数组
///
/// # 返回值
/// `Result<Array1<f64>>` - 长度为 `a.len()` 的数组，每个元素为 `a[i] - b[i]`。
///
/// # 错误
/// * 当 `a.len() != b.len()` 时返回 `TaError::InvalidParameter`
/// * 当任一输入长度为 0 时返回 `TaError::EmptyInput`
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_operators::sub;
/// let a = vec![10.0, 20.0, 30.0];
/// let b = vec![1.0, 2.0, 3.0];
/// let r = sub(&a, &b).unwrap();
/// assert_eq!(r.to_vec(), vec![9.0, 18.0, 27.0]);
/// ```
pub fn sub(a: &[f64], b: &[f64]) -> Result<Array1<f64>> {
    validate_two_inputs(a, b)?;
    Ok(a.iter().zip(b.iter()).map(|(x, y)| x - y).collect())
}

/// 逐元素相乘 MULT：返回 `a * b`
///
/// 与 TA-Lib C 库的 `TA_MULT` 函数完全等价。计算两个等长数组的逐元素乘法。
///
/// # 参数
/// * `a` - 第一个输入数组
/// * `b` - 第二个输入数组
///
/// # 返回值
/// `Result<Array1<f64>>` - 长度为 `a.len()` 的数组，每个元素为 `a[i] * b[i]`。
///
/// # 错误
/// * 当 `a.len() != b.len()` 时返回 `TaError::InvalidParameter`
/// * 当任一输入长度为 0 时返回 `TaError::EmptyInput`
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_operators::mult;
/// let a = vec![2.0, 3.0, 4.0];
/// let b = vec![5.0, 6.0, 7.0];
/// let r = mult(&a, &b).unwrap();
/// assert_eq!(r.to_vec(), vec![10.0, 18.0, 28.0]);
/// ```
pub fn mult(a: &[f64], b: &[f64]) -> Result<Array1<f64>> {
    validate_two_inputs(a, b)?;
    Ok(a.iter().zip(b.iter()).map(|(x, y)| x * y).collect())
}

/// 逐元素相除 DIV：返回 `a / b`
///
/// 与 TA-Lib C 库的 `TA_DIV` 函数完全等价。计算两个等长数组的逐元素除法。
///
/// 当除数为 0 时，结果为 `f64::NAN`（与 IEEE 754 浮点语义一致，**不会**返回错误），
/// 与 TA-Lib C 库的行为保持兼容。
///
/// # 参数
/// * `a` - 被除数数组
/// * `b` - 除数数组
///
/// # 返回值
/// `Result<Array1<f64>>` - 长度为 `a.len()` 的数组，每个元素为 `a[i] / b[i]`。
///
/// # 错误
/// * 当 `a.len() != b.len()` 时返回 `TaError::InvalidParameter`
/// * 当任一输入长度为 0 时返回 `TaError::EmptyInput`
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_operators::div;
/// let a = vec![10.0, 20.0, 30.0];
/// let b = vec![2.0, 4.0, 5.0];
/// let r = div(&a, &b).unwrap();
/// assert_eq!(r.to_vec(), vec![5.0, 5.0, 6.0]);
/// ```
pub fn div(a: &[f64], b: &[f64]) -> Result<Array1<f64>> {
    validate_two_inputs(a, b)?;
    Ok(a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            if *y == 0.0 {
                f64::NAN
            } else {
                x / y
            }
        })
        .collect())
}

/// 周期差分 MINUS：返回 `data[i] - data[i - period]`
///
/// 与 TA-Lib C 库的 `TA_MINUS` 函数完全等价。计算一阶差分，但滞后量为 `period`
/// 而非 1。前 `period` 个值为 `NaN`（因为没有足够的历史数据）。
///
/// # 参数
/// * `data` - 输入数据序列
/// * `period` - 差分滞后期（必须 `>= 1`）
///
/// # 返回值
/// `Result<Array1<f64>>` - 长度与 `data` 相同的数组。
///
/// # 错误
/// * 当 `period == 0` 时返回 `TaError::InvalidParameter`
/// * 当 `period >= data.len()` 时返回 `TaError::InsufficientData`
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_operators::minus;
/// let data = vec![1.0, 2.0, 4.0, 7.0, 11.0];
/// let r = minus(&data, 2).unwrap();
/// // 前 2 个为 NaN，r[2] = 4 - 1 = 3, r[3] = 7 - 2 = 5, r[4] = 11 - 4 = 7
/// assert!(r[0].is_nan() && r[1].is_nan());
/// assert_eq!(r[2], 3.0);
/// assert_eq!(r[3], 5.0);
/// assert_eq!(r[4], 7.0);
/// ```
pub fn minus(data: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: ">= 1".to_string(),
        });
    }
    validate_input(data.len(), period + 1)?;

    let len = data.len();
    let mut output = init_output(len);
    for i in period..len {
        output[i] = data[i] - data[i - period];
    }
    Ok(output)
}

/// 滚动窗口最大值 MAX
///
/// 与 TA-Lib C 库的 `TA_MAX` 函数完全等价。返回长度为 `period` 的滑动窗口内的最大值。
/// 前 `period - 1` 个值为 `NaN`。
///
/// # 参数
/// * `data` - 输入数据序列
/// * `period` - 滚动窗口大小（必须 `>= 1`）
///
/// # 返回值
/// `Result<Array1<f64>>` - 长度与 `data` 相同的数组。
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_operators::max;
/// let data = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
/// let r = max(&data, 3).unwrap();
/// assert!(r[0].is_nan() && r[1].is_nan());
/// assert_eq!(r[2], 4.0); // max(3,1,4)
/// assert_eq!(r[3], 4.0); // max(1,4,1)
/// assert_eq!(r[4], 5.0); // max(4,1,5)
/// assert_eq!(r[7], 9.0); // max(9,2,6)
/// ```
pub fn max(data: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_window(data, period)?;

    let len = data.len();
    let mut output = init_output(len);
    if period == 1 {
        // 1 周期窗口就是自身
        for (i, v) in data.iter().enumerate() {
            output[i] = *v;
        }
        return Ok(output);
    }

    // 使用 deque 实现 O(n) 的滑动窗口最大算法
    let mut deque: VecDeque<usize> = VecDeque::with_capacity(period);
    for i in 0..len {
        // 移除超出窗口左侧的元素
        while let Some(&front) = deque.front() {
            if front + period <= i {
                deque.pop_front();
            } else {
                break;
            }
        }
        // 维护单调递减：移除所有 <= 当前值的队尾
        while let Some(&back) = deque.back() {
            if data[back] <= data[i] {
                deque.pop_back();
            } else {
                break;
            }
        }
        deque.push_back(i);

        if i + 1 >= period {
            output[i] = data[*deque.front().expect("deque non-empty after push")];
        }
    }
    Ok(output)
}

/// 滚动窗口最小值 MIN
///
/// 与 TA-Lib C 库的 `TA_MIN` 函数完全等价。返回长度为 `period` 的滑动窗口内的最小值。
/// 前 `period - 1` 个值为 `NaN`。
///
/// # 参数
/// * `data` - 输入数据序列
/// * `period` - 滚动窗口大小（必须 `>= 1`）
///
/// # 返回值
/// `Result<Array1<f64>>` - 长度与 `data` 相同的数组。
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_operators::min;
/// let data = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
/// let r = min(&data, 3).unwrap();
/// assert!(r[0].is_nan() && r[1].is_nan());
/// assert_eq!(r[2], 1.0); // min(3,1,4)
/// assert_eq!(r[3], 1.0); // min(1,4,1)
/// assert_eq!(r[4], 1.0); // min(4,1,5)
/// assert_eq!(r[7], 2.0); // min(9,2,6)
/// ```
pub fn min(data: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_window(data, period)?;

    let len = data.len();
    let mut output = init_output(len);
    if period == 1 {
        for (i, v) in data.iter().enumerate() {
            output[i] = *v;
        }
        return Ok(output);
    }

    // 使用 deque 实现 O(n) 的滑动窗口最小算法
    let mut deque: VecDeque<usize> = VecDeque::with_capacity(period);
    for i in 0..len {
        // 移除超出窗口左侧的元素
        while let Some(&front) = deque.front() {
            if front + period <= i {
                deque.pop_front();
            } else {
                break;
            }
        }
        // 维护单调递增：移除所有 >= 当前值的队尾
        while let Some(&back) = deque.back() {
            if data[back] >= data[i] {
                deque.pop_back();
            } else {
                break;
            }
        }
        deque.push_back(i);

        if i + 1 >= period {
            output[i] = data[*deque.front().expect("deque non-empty after push")];
        }
    }
    Ok(output)
}

/// 滚动窗口求和 SUM
///
/// 与 TA-Lib C 库的 `TA_SUM` 函数完全等价。返回长度为 `period` 的滑动窗口内的元素之和。
/// 前 `period - 1` 个值为 `NaN`。
///
/// 使用增量更新实现 O(1) 摊销复杂度：每个新窗口减去最早元素、加上最新元素。
///
/// # 参数
/// * `data` - 输入数据序列
/// * `period` - 滚动窗口大小（必须 `>= 1`）
///
/// # 返回值
/// `Result<Array1<f64>>` - 长度与 `data` 相同的数组。
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_operators::sum;
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let r = sum(&data, 3).unwrap();
/// assert!(r[0].is_nan() && r[1].is_nan());
/// assert_eq!(r[2], 6.0);  // 1+2+3
/// assert_eq!(r[3], 9.0);  // 2+3+4
/// assert_eq!(r[4], 12.0); // 3+4+5
/// ```
pub fn sum(data: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_window(data, period)?;

    let len = data.len();
    let mut output = init_output(len);
    if period == 1 {
        for (i, v) in data.iter().enumerate() {
            output[i] = *v;
        }
        return Ok(output);
    }

    // 初始化第一个窗口
    let mut acc: f64 = data[..period].iter().sum();
    output[period - 1] = acc;

    // 增量更新：每步减去离开元素、加上新进入元素
    for i in period..len {
        acc += data[i] - data[i - period];
        output[i] = acc;
    }
    Ok(output)
}

/// 滚动窗口内最大值的索引 MAXINDEX
///
/// 与 TA-Lib C 库的 `TA_MAXINDEX` 函数完全等价。返回长度为 `period` 的滑动窗口内
/// 最大值相对于窗口起点的偏移量。前 `period - 1` 个值为 `-1`。
///
/// 当窗口内出现多个相同的最大值时，TA-Lib 约定返回第一次出现的位置。
///
/// # 参数
/// * `data` - 输入数据序列
/// * `period` - 滚动窗口大小（必须 `>= 1`）
///
/// # 返回值
/// `Result<Array1<i64>>` - 长度与 `data` 相同的整数数组。
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_operators::maxindex;
/// let data = vec![3.0, 1.0, 4.0, 1.0, 5.0];
/// let r = maxindex(&data, 3).unwrap();
/// assert_eq!(r[0], -1);
/// assert_eq!(r[1], -1);
/// // 窗口 [3,1,4] 内最大值 4 出现在偏移 2
/// assert_eq!(r[2], 2);
/// // 窗口 [1,4,1] 内最大值 4 出现在偏移 1
/// assert_eq!(r[3], 1);
/// // 窗口 [4,1,5] 内最大值 5 出现在偏移 2
/// assert_eq!(r[4], 2);
/// ```
pub fn maxindex(data: &[f64], period: usize) -> Result<Array1<i64>> {
    validate_window(data, period)?;
    let len = data.len();
    let mut output = Array1::from_elem(len, -1_i64);

    if period == 1 {
        for i in 0..len {
            output[i] = 0;
        }
        return Ok(output);
    }

    for end in (period - 1)..len {
        let start = end + 1 - period;
        let mut best_off = 0_i64;
        let mut best_val = data[start];
        for k in 1..period {
            let v = data[start + k];
            if v > best_val {
                best_val = v;
                best_off = k as i64;
            }
        }
        output[end] = best_off;
    }
    Ok(output)
}

/// 滚动窗口内最小值的索引 MININDEX
///
/// 与 TA-Lib C 库的 `TA_MININDEX` 函数完全等价。返回长度为 `period` 的滑动窗口内
/// 最小值相对于窗口起点的偏移量。前 `period - 1` 个值为 `-1`。
///
/// 当窗口内出现多个相同的最小值时，TA-Lib 约定返回第一次出现的位置。
///
/// # 参数
/// * `data` - 输入数据序列
/// * `period` - 滚动窗口大小（必须 `>= 1`）
///
/// # 返回值
/// `Result<Array1<i64>>` - 长度与 `data` 相同的整数数组。
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_operators::minindex;
/// let data = vec![3.0, 1.0, 4.0, 1.0, 5.0];
/// let r = minindex(&data, 3).unwrap();
/// assert_eq!(r[0], -1);
/// assert_eq!(r[1], -1);
/// // 窗口 [3,1,4] 内最小值 1 出现在偏移 1
/// assert_eq!(r[2], 1);
/// // 窗口 [1,4,1] 内最小值 1 出现在偏移 0
/// assert_eq!(r[3], 0);
/// // 窗口 [4,1,5] 内最小值 1 出现在偏移 1
/// assert_eq!(r[4], 1);
/// ```
pub fn minindex(data: &[f64], period: usize) -> Result<Array1<i64>> {
    validate_window(data, period)?;
    let len = data.len();
    let mut output = Array1::from_elem(len, -1_i64);

    if period == 1 {
        for i in 0..len {
            output[i] = 0;
        }
        return Ok(output);
    }

    for end in (period - 1)..len {
        let start = end + 1 - period;
        let mut best_off = 0_i64;
        let mut best_val = data[start];
        for k in 1..period {
            let v = data[start + k];
            if v < best_val {
                best_val = v;
                best_off = k as i64;
            }
        }
        output[end] = best_off;
    }
    Ok(output)
}

/// 滚动窗口最小值与最大值 MINMAX
///
/// 与 TA-Lib C 库的 `TA_MINMAX` 函数完全等价。同时返回长度为 `period` 的滑动窗口
/// 内的最小值和最大值数组。前 `period - 1` 个值为 `NaN`。
///
/// # 参数
/// * `data` - 输入数据序列
/// * `period` - 滚动窗口大小（必须 `>= 1`）
///
/// # 返回值
/// `Result<(Array1<f64>, Array1<f64>)>` - `(min_array, max_array)`。
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_operators::minmax;
/// let data = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
/// let (mins, maxs) = minmax(&data, 3).unwrap();
/// assert_eq!(mins[2], 1.0);
/// assert_eq!(maxs[2], 4.0);
/// ```
pub fn minmax(data: &[f64], period: usize) -> Result<(Array1<f64>, Array1<f64>)> {
    let min_arr = min(data, period)?;
    let max_arr = max(data, period)?;
    Ok((min_arr, max_arr))
}

/// 滚动窗口最小值与最大值的索引 MINMAXINDEX
///
/// 与 TA-Lib C 库的 `TA_MINMAXINDEX` 函数完全等价。同时返回长度为 `period` 的
/// 滑动窗口内最小值和最大值相对于窗口起点的偏移量。前 `period - 1` 个值为 `-1`。
///
/// # 参数
/// * `data` - 输入数据序列
/// * `period` - 滚动窗口大小（必须 `>= 1`）
///
/// # 返回值
/// `Result<(Array1<i64>, Array1<i64>)>` - `(min_index_array, max_index_array)`。
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_operators::minmaxindex;
/// let data = vec![3.0, 1.0, 4.0, 1.0, 5.0];
/// let (min_idx, max_idx) = minmaxindex(&data, 3).unwrap();
/// assert_eq!(min_idx[2], 1); // [3,1,4] -> min 1 at offset 1
/// assert_eq!(max_idx[2], 2); // [3,1,4] -> max 4 at offset 2
/// ```
pub fn minmaxindex(data: &[f64], period: usize) -> Result<(Array1<i64>, Array1<i64>)> {
    let min_idx = minindex(data, period)?;
    let max_idx = maxindex(data, period)?;
    Ok((min_idx, max_idx))
}

// ---------------------------------------------------------------------------
// 内部工具
// ---------------------------------------------------------------------------

/// 校验双输入函数：长度相等且均非空
fn validate_two_inputs(a: &[f64], b: &[f64]) -> Result<()> {
    if a.len() != b.len() {
        return Err(TaError::InvalidParameter {
            name: "a and b".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(a.len(), 1)?;
    Ok(())
}

/// 校验窗口函数：周期 `>= 1` 且输入长度足够
fn validate_window(data: &[f64], period: usize) -> Result<()> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: ">= 1".to_string(),
        });
    }
    validate_input(data.len(), period)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn nans_at(slice: &Array1<f64>, idx: &[usize]) -> bool {
        idx.iter().all(|&i| slice[i].is_nan())
    }

    // ------------------------- 双输入算术函数 -------------------------

    #[test]
    fn test_add_basic() {
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![10.0, 20.0, 30.0, 40.0];
        let r = add(&a, &b).unwrap();
        assert_eq!(r.to_vec(), vec![11.0, 22.0, 33.0, 44.0]);
    }

    #[test]
    fn test_add_negative() {
        let a = vec![5.0, -2.0, 0.0];
        let b = vec![-1.0, 2.0, 0.5];
        let r = add(&a, &b).unwrap();
        assert_eq!(r.to_vec(), vec![4.0, 0.0, 0.5]);
    }

    #[test]
    fn test_add_length_mismatch() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        assert!(add(&a, &b).is_err());
    }

    #[test]
    fn test_add_empty() {
        let a: Vec<f64> = vec![];
        let b: Vec<f64> = vec![];
        assert!(add(&a, &b).is_err());
    }

    #[test]
    fn test_sub_basic() {
        let a = vec![10.0, 20.0, 30.0];
        let b = vec![1.0, 5.0, 100.0];
        let r = sub(&a, &b).unwrap();
        assert_eq!(r.to_vec(), vec![9.0, 15.0, -70.0]);
    }

    #[test]
    fn test_sub_length_mismatch() {
        let a = vec![1.0];
        let b = vec![1.0, 2.0];
        assert!(sub(&a, &b).is_err());
    }

    #[test]
    fn test_mult_basic() {
        let a = vec![2.0, 3.0, 4.0];
        let b = vec![5.0, 6.0, 7.0];
        let r = mult(&a, &b).unwrap();
        assert_eq!(r.to_vec(), vec![10.0, 18.0, 28.0]);
    }

    #[test]
    fn test_mult_zero() {
        let a = vec![1.0, 0.0, 5.0];
        let b = vec![3.0, 7.0, 0.0];
        let r = mult(&a, &b).unwrap();
        assert_eq!(r.to_vec(), vec![3.0, 0.0, 0.0]);
    }

    #[test]
    fn test_div_basic() {
        let a = vec![10.0, 20.0, 30.0];
        let b = vec![2.0, 4.0, 5.0];
        let r = div(&a, &b).unwrap();
        assert_eq!(r.to_vec(), vec![5.0, 5.0, 6.0]);
    }

    #[test]
    fn test_div_by_zero_returns_nan() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![0.0, 1.0, 0.0];
        let r = div(&a, &b).unwrap();
        assert!(r[0].is_nan());
        assert_relative_eq!(r[1], 2.0, epsilon = 1e-12);
        assert!(r[2].is_nan());
    }

    #[test]
    fn test_div_length_mismatch() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0];
        assert!(div(&a, &b).is_err());
    }

    // ------------------------- minus -------------------------

    #[test]
    fn test_minus_basic() {
        let data = vec![1.0, 2.0, 4.0, 7.0, 11.0];
        let r = minus(&data, 2).unwrap();
        assert!(nans_at(&r, &[0, 1]));
        assert_eq!(r[2], 3.0);
        assert_eq!(r[3], 5.0);
        assert_eq!(r[4], 7.0);
    }

    #[test]
    fn test_minus_period_one() {
        let data = vec![1.0, 3.0, 6.0, 10.0];
        let r = minus(&data, 1).unwrap();
        assert!(r[0].is_nan());
        assert_eq!(r[1], 2.0);
        assert_eq!(r[2], 3.0);
        assert_eq!(r[3], 4.0);
    }

    #[test]
    fn test_minus_zero_period() {
        let data = vec![1.0, 2.0, 3.0];
        assert!(minus(&data, 0).is_err());
    }

    #[test]
    fn test_minus_period_too_large() {
        let data = vec![1.0, 2.0];
        assert!(minus(&data, 5).is_err());
    }

    #[test]
    fn test_minus_empty() {
        let data: Vec<f64> = vec![];
        assert!(minus(&data, 1).is_err());
    }

    // ------------------------- max -------------------------

    #[test]
    fn test_max_basic() {
        let data = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        let r = max(&data, 3).unwrap();
        assert!(nans_at(&r, &[0, 1]));
        assert_eq!(r[2], 4.0);
        assert_eq!(r[3], 4.0);
        assert_eq!(r[4], 5.0);
        assert_eq!(r[5], 9.0);
        assert_eq!(r[6], 9.0);
        assert_eq!(r[7], 9.0);
    }

    #[test]
    fn test_max_period_one() {
        let data = vec![3.0, 1.0, 4.0];
        let r = max(&data, 1).unwrap();
        assert_eq!(r.to_vec(), vec![3.0, 1.0, 4.0]);
    }

    #[test]
    fn test_max_zero_period() {
        let data = vec![1.0, 2.0, 3.0];
        assert!(max(&data, 0).is_err());
    }

    #[test]
    fn test_max_insufficient_data() {
        let data = vec![1.0, 2.0];
        assert!(max(&data, 5).is_err());
    }

    #[test]
    fn test_max_constant() {
        let data = vec![5.0, 5.0, 5.0, 5.0];
        let r = max(&data, 2).unwrap();
        assert!(nans_at(&r, &[0]));
        assert_eq!(r[1], 5.0);
        assert_eq!(r[2], 5.0);
        assert_eq!(r[3], 5.0);
    }

    // ------------------------- min -------------------------

    #[test]
    fn test_min_basic() {
        let data = vec![3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        let r = min(&data, 3).unwrap();
        assert!(nans_at(&r, &[0, 1]));
        assert_eq!(r[2], 1.0);
        assert_eq!(r[3], 1.0);
        assert_eq!(r[4], 1.0);
        assert_eq!(r[5], 1.0);
        assert_eq!(r[6], 2.0);
        assert_eq!(r[7], 2.0);
    }

    #[test]
    fn test_min_period_one() {
        let data = vec![3.0, 1.0, 4.0];
        let r = min(&data, 1).unwrap();
        assert_eq!(r.to_vec(), vec![3.0, 1.0, 4.0]);
    }

    #[test]
    fn test_min_zero_period() {
        let data = vec![1.0, 2.0, 3.0];
        assert!(min(&data, 0).is_err());
    }

    #[test]
    fn test_min_insufficient_data() {
        let data: Vec<f64> = vec![];
        assert!(min(&data, 1).is_err());
    }

    // ------------------------- sum -------------------------

    #[test]
    fn test_sum_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let r = sum(&data, 3).unwrap();
        assert!(nans_at(&r, &[0, 1]));
        assert_eq!(r[2], 6.0);
        assert_eq!(r[3], 9.0);
        assert_eq!(r[4], 12.0);
    }

    #[test]
    fn test_sum_period_one() {
        let data = vec![1.0, 2.0, 3.0];
        let r = sum(&data, 1).unwrap();
        assert_eq!(r.to_vec(), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_sum_negative() {
        let data = vec![-1.0, -2.0, -3.0, -4.0];
        let r = sum(&data, 2).unwrap();
        assert!(r[0].is_nan());
        assert_eq!(r[1], -3.0);
        assert_eq!(r[2], -5.0);
        assert_eq!(r[3], -7.0);
    }

    #[test]
    fn test_sum_zero_period() {
        let data = vec![1.0, 2.0];
        assert!(sum(&data, 0).is_err());
    }

    #[test]
    fn test_sum_incremental_matches_simple() {
        // 增量更新版本应与朴素 O(n*period) 版本结果一致
        let data: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        let period = 5;
        let r = sum(&data, period).unwrap();
        // 朴素计算
        let mut expected = vec![f64::NAN; data.len()];
        for i in (period - 1)..data.len() {
            expected[i] = data[(i + 1 - period)..=i].iter().sum();
        }
        for i in 0..data.len() {
            if expected[i].is_nan() {
                assert!(r[i].is_nan());
            } else {
                assert_relative_eq!(r[i], expected[i], epsilon = 1e-9);
            }
        }
    }

    // ------------------------- maxindex -------------------------

    #[test]
    fn test_maxindex_basic() {
        let data = vec![3.0, 1.0, 4.0, 1.0, 5.0];
        let r = maxindex(&data, 3).unwrap();
        assert_eq!(r[0], -1);
        assert_eq!(r[1], -1);
        assert_eq!(r[2], 2); // [3,1,4] -> 4 at offset 2
        assert_eq!(r[3], 1); // [1,4,1] -> 4 at offset 1
        assert_eq!(r[4], 2); // [4,1,5] -> 5 at offset 2
    }

    #[test]
    fn test_maxindex_first_occurrence_on_ties() {
        // 当窗口内有相同最大值时，TA-Lib 约定返回首次出现的位置
        let data = vec![5.0, 5.0, 5.0];
        let r = maxindex(&data, 3).unwrap();
        assert_eq!(r[2], 0);
    }

    #[test]
    fn test_maxindex_period_one() {
        let data = vec![1.0, 5.0, 3.0];
        let r = maxindex(&data, 1).unwrap();
        assert_eq!(r.to_vec(), vec![0, 0, 0]);
    }

    #[test]
    fn test_maxindex_zero_period() {
        let data = vec![1.0, 2.0, 3.0];
        assert!(maxindex(&data, 0).is_err());
    }

    #[test]
    fn test_maxindex_incremental_max() {
        let data = vec![1.0, 5.0, 3.0, 8.0, 2.0, 6.0, 4.0];
        let r = maxindex(&data, 3).unwrap();
        // 手动核对：每个窗口内最大值相对窗口起点的偏移
        // window [1,5,3] -> 5 @ 1
        // window [5,3,8] -> 8 @ 2
        // window [3,8,2] -> 8 @ 1
        // window [8,2,6] -> 8 @ 0
        // window [2,6,4] -> 6 @ 1
        let expected = vec![-1_i64, -1, 1, 2, 1, 0, 1];
        assert_eq!(r.to_vec(), expected);
    }

    // ------------------------- minindex -------------------------

    #[test]
    fn test_minindex_basic() {
        let data = vec![3.0, 1.0, 4.0, 1.0, 5.0];
        let r = minindex(&data, 3).unwrap();
        assert_eq!(r[0], -1);
        assert_eq!(r[1], -1);
        assert_eq!(r[2], 1); // [3,1,4] -> 1 at offset 1
        assert_eq!(r[3], 0); // [1,4,1] -> 1 at offset 0
        assert_eq!(r[4], 1); // [4,1,5] -> 1 at offset 1
    }

    #[test]
    fn test_minindex_first_occurrence_on_ties() {
        let data = vec![2.0, 2.0, 2.0];
        let r = minindex(&data, 3).unwrap();
        assert_eq!(r[2], 0);
    }

    #[test]
    fn test_minindex_period_one() {
        let data = vec![1.0, 5.0, 3.0];
        let r = minindex(&data, 1).unwrap();
        assert_eq!(r.to_vec(), vec![0, 0, 0]);
    }

    #[test]
    fn test_minindex_zero_period() {
        let data = vec![1.0, 2.0, 3.0];
        assert!(minindex(&data, 0).is_err());
    }

    // ------------------------- 长度验证 -------------------------

    #[test]
    fn test_output_lengths_match_input() {
        let a: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let b: Vec<f64> = (0..100).map(|i| (i as f64) * 2.0).collect();
        assert_eq!(add(&a, &b).unwrap().len(), 100);
        assert_eq!(sub(&a, &b).unwrap().len(), 100);
        assert_eq!(mult(&a, &b).unwrap().len(), 100);
        assert_eq!(div(&a, &b).unwrap().len(), 100);
        assert_eq!(minus(&a, 5).unwrap().len(), 100);
        assert_eq!(max(&a, 7).unwrap().len(), 100);
        assert_eq!(min(&a, 7).unwrap().len(), 100);
        assert_eq!(sum(&a, 7).unwrap().len(), 100);
        assert_eq!(maxindex(&a, 7).unwrap().len(), 100);
        assert_eq!(minindex(&a, 7).unwrap().len(), 100);
    }
}
