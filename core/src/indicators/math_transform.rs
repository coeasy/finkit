//! Math Transform 函数集合
//!
//! 本模块提供与 TA-Lib C 库 100% 兼容的 15 个 Math Transform 函数。
//! 所有函数均为 element-wise 逐元素数学变换，输入输出长度相同。
//!
//! # 函数列表
//! - 三角函数：`acos`/`asin`/`atan`/`cos`/`sin`/`tan`
//! - 双曲函数：`cosh`/`sinh`/`tanh`
//! - 指数对数：`exp`/`ln`/`log10`
//! - 取整函数：`ceil`/`floor`
//! - 幂函数：`sqrt`
//!
//! # 错误处理
//! - 输入为空时返回 [`TaError::EmptyInput`]
//! - 输入值超出函数定义域时返回 [`TaError::InvalidParameter`] 或 [`TaError::ComputationError`]

use crate::error::{Result, TaError};
use crate::utils::validate_input;
use ndarray::Array1;

/// 反余弦 (Vector Arc Cosine)
///
/// 计算每个元素的反余弦值（arccos），输入域 `[-1, 1]`，输出域 `[0, π]`。
///
/// # 参数
/// * `data` - 输入数据序列（每个值应在 `[-1, 1]`）
///
/// # 返回值
/// 反余弦值数组
///
/// # 错误
/// - 输入为空时返回 `EmptyInput`
/// - 任一元素超出 `[-1, 1]` 域时返回 `InvalidParameter`
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_transform::acos;
/// let data = vec![1.0, 0.0, -1.0];
/// let result = acos(&data).unwrap();
/// assert_eq!(result.len(), 3);
/// ```
pub fn acos(data: &[f64]) -> Result<Array1<f64>> {
    validate_input(data.len(), 1)?;
    for (i, &x) in data.iter().enumerate() {
        if !x.is_finite() || x < -1.0 || x > 1.0 {
            return Err(TaError::InvalidParameter {
                name: format!("data[{}]", i),
                constraint: "value in [-1, 1]".to_string(),
            });
        }
    }
    Ok(data.iter().map(|x| x.acos()).collect())
}

/// 反正弦 (Vector Arc Sine)
///
/// 计算每个元素的反正弦值（arcsin），输入域 `[-1, 1]`，输出域 `[-π/2, π/2]`。
///
/// # 参数
/// * `data` - 输入数据序列（每个值应在 `[-1, 1]`）
///
/// # 返回值
/// 反正弦值数组
///
/// # 错误
/// - 输入为空时返回 `EmptyInput`
/// - 任一元素超出 `[-1, 1]` 域时返回 `InvalidParameter`
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_transform::asin;
/// let data = vec![1.0, 0.0, -1.0];
/// let result = asin(&data).unwrap();
/// assert_eq!(result.len(), 3);
/// ```
pub fn asin(data: &[f64]) -> Result<Array1<f64>> {
    validate_input(data.len(), 1)?;
    for (i, &x) in data.iter().enumerate() {
        if !x.is_finite() || x < -1.0 || x > 1.0 {
            return Err(TaError::InvalidParameter {
                name: format!("data[{}]", i),
                constraint: "value in [-1, 1]".to_string(),
            });
        }
    }
    Ok(data.iter().map(|x| x.asin()).collect())
}

/// 反正切 (Vector Arc Tangent)
///
/// 计算每个元素的反正切值（arctan），输入域 `ℝ`，输出域 `(-π/2, π/2)`。
///
/// # 参数
/// * `data` - 输入数据序列
///
/// # 返回值
/// 反正切值数组
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_transform::atan;
/// let data = vec![0.0, 1.0, -1.0];
/// let result = atan(&data).unwrap();
/// assert_eq!(result.len(), 3);
/// ```
pub fn atan(data: &[f64]) -> Result<Array1<f64>> {
    validate_input(data.len(), 1)?;
    Ok(data.iter().map(|x| x.atan()).collect())
}

/// 向上取整 (Vector Ceiling)
///
/// 返回大于或等于每个元素的最小整数值。
///
/// # 参数
/// * `data` - 输入数据序列
///
/// # 返回值
/// 向上取整后的数组
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_transform::ceil;
/// let data = vec![1.2, 2.8, -1.7];
/// let result = ceil(&data).unwrap();
/// assert_eq!(result.len(), 3);
/// ```
pub fn ceil(data: &[f64]) -> Result<Array1<f64>> {
    validate_input(data.len(), 1)?;
    Ok(data.iter().map(|x| x.ceil()).collect())
}

/// 余弦 (Vector Cosine)
///
/// 计算每个元素的余弦值（输入弧度）。
///
/// # 参数
/// * `data` - 输入数据序列（弧度）
///
/// # 返回值
/// 余弦值数组，范围 `[-1, 1]`
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_transform::cos;
/// let data = vec![0.0, std::f64::consts::FRAC_PI_2];
/// let result = cos(&data).unwrap();
/// assert_eq!(result.len(), 2);
/// ```
pub fn cos(data: &[f64]) -> Result<Array1<f64>> {
    validate_input(data.len(), 1)?;
    Ok(data.iter().map(|x| x.cos()).collect())
}

/// 双曲余弦 (Vector Hyperbolic Cosine)
///
/// 计算每个元素的双曲余弦值。
///
/// # 公式
/// cosh(x) = (e^x + e^-x) / 2
///
/// # 参数
/// * `data` - 输入数据序列
///
/// # 返回值
/// 双曲余弦值数组
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_transform::cosh;
/// let data = vec![0.0, 1.0, -1.0];
/// let result = cosh(&data).unwrap();
/// assert_eq!(result.len(), 3);
/// ```
pub fn cosh(data: &[f64]) -> Result<Array1<f64>> {
    validate_input(data.len(), 1)?;
    Ok(data.iter().map(|x| x.cosh()).collect())
}

/// 指数函数 (Vector Exponential)
///
/// 计算每个元素的自然指数 e^x。
///
/// # 参数
/// * `data` - 输入数据序列
///
/// # 返回值
/// e^x 数组
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_transform::exp;
/// let data = vec![0.0, 1.0, 2.0];
/// let result = exp(&data).unwrap();
/// assert_eq!(result.len(), 3);
/// ```
pub fn exp(data: &[f64]) -> Result<Array1<f64>> {
    validate_input(data.len(), 1)?;
    Ok(data.iter().map(|x| x.exp()).collect())
}

/// 向下取整 (Vector Floor)
///
/// 返回小于或等于每个元素的最大整数值。
///
/// # 参数
/// * `data` - 输入数据序列
///
/// # 返回值
/// 向下取整后的数组
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_transform::floor;
/// let data = vec![1.2, 2.8, -1.7];
/// let result = floor(&data).unwrap();
/// assert_eq!(result.len(), 3);
/// ```
pub fn floor(data: &[f64]) -> Result<Array1<f64>> {
    validate_input(data.len(), 1)?;
    Ok(data.iter().map(|x| x.floor()).collect())
}

/// 自然对数 (Vector Natural Logarithm)
///
/// 计算每个元素的自然对数，输入必须为正数。
///
/// # 参数
/// * `data` - 输入数据序列（每个值应 > 0）
///
/// # 返回值
/// 自然对数值数组
///
/// # 错误
/// - 输入为空时返回 `EmptyInput`
/// - 任一元素 ≤ 0 时返回 `InvalidParameter`
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_transform::ln;
/// let data = vec![1.0, std::f64::consts::E];
/// let result = ln(&data).unwrap();
/// assert_eq!(result.len(), 2);
/// ```
pub fn ln(data: &[f64]) -> Result<Array1<f64>> {
    validate_input(data.len(), 1)?;
    for (i, &x) in data.iter().enumerate() {
        if !x.is_finite() || x <= 0.0 {
            return Err(TaError::InvalidParameter {
                name: format!("data[{}]", i),
                constraint: "value > 0".to_string(),
            });
        }
    }
    Ok(data.iter().map(|x| x.ln()).collect())
}

/// 常用对数 (Vector Base-10 Logarithm)
///
/// 计算每个元素的以 10 为底的对数，输入必须为正数。
///
/// # 参数
/// * `data` - 输入数据序列（每个值应 > 0）
///
/// # 返回值
/// 常用对数值数组
///
/// # 错误
/// - 输入为空时返回 `EmptyInput`
/// - 任一元素 ≤ 0 时返回 `InvalidParameter`
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_transform::log10;
/// let data = vec![1.0, 10.0, 100.0];
/// let result = log10(&data).unwrap();
/// assert_eq!(result.len(), 3);
/// ```
pub fn log10(data: &[f64]) -> Result<Array1<f64>> {
    validate_input(data.len(), 1)?;
    for (i, &x) in data.iter().enumerate() {
        if !x.is_finite() || x <= 0.0 {
            return Err(TaError::InvalidParameter {
                name: format!("data[{}]", i),
                constraint: "value > 0".to_string(),
            });
        }
    }
    Ok(data.iter().map(|x| x.log10()).collect())
}

/// 正弦 (Vector Sine)
///
/// 计算每个元素的正弦值（输入弧度）。
///
/// # 参数
/// * `data` - 输入数据序列（弧度）
///
/// # 返回值
/// 正弦值数组，范围 `[-1, 1]`
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_transform::sin;
/// let data = vec![0.0, std::f64::consts::FRAC_PI_2];
/// let result = sin(&data).unwrap();
/// assert_eq!(result.len(), 2);
/// ```
pub fn sin(data: &[f64]) -> Result<Array1<f64>> {
    validate_input(data.len(), 1)?;
    Ok(data.iter().map(|x| x.sin()).collect())
}

/// 双曲正弦 (Vector Hyperbolic Sine)
///
/// 计算每个元素的双曲正弦值。
///
/// # 公式
/// sinh(x) = (e^x - e^-x) / 2
///
/// # 参数
/// * `data` - 输入数据序列
///
/// # 返回值
/// 双曲正弦值数组
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_transform::sinh;
/// let data = vec![0.0, 1.0, -1.0];
/// let result = sinh(&data).unwrap();
/// assert_eq!(result.len(), 3);
/// ```
pub fn sinh(data: &[f64]) -> Result<Array1<f64>> {
    validate_input(data.len(), 1)?;
    Ok(data.iter().map(|x| x.sinh()).collect())
}

/// 平方根 (Vector Square Root)
///
/// 计算每个元素的平方根，输入必须为非负数。
///
/// # 参数
/// * `data` - 输入数据序列（每个值应 ≥ 0）
///
/// # 返回值
/// 平方根数组
///
/// # 错误
/// - 输入为空时返回 `EmptyInput`
/// - 任一元素 < 0 时返回 `InvalidParameter`
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_transform::sqrt;
/// let data = vec![4.0];
/// let result = sqrt(&data).unwrap();
/// assert_eq!(result[0], 2.0);
/// ```
pub fn sqrt(data: &[f64]) -> Result<Array1<f64>> {
    validate_input(data.len(), 1)?;
    for (i, &x) in data.iter().enumerate() {
        if !x.is_finite() || x < 0.0 {
            return Err(TaError::InvalidParameter {
                name: format!("data[{}]", i),
                constraint: "value >= 0".to_string(),
            });
        }
    }
    Ok(data.iter().map(|x| x.sqrt()).collect())
}

/// 正切 (Vector Tangent)
///
/// 计算每个元素的正切值（输入弧度）。
///
/// # 参数
/// * `data` - 输入数据序列（弧度）
///
/// # 返回值
/// 正切值数组
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_transform::tan;
/// let data = vec![0.0, std::f64::consts::FRAC_PI_4];
/// let result = tan(&data).unwrap();
/// assert_eq!(result.len(), 2);
/// ```
pub fn tan(data: &[f64]) -> Result<Array1<f64>> {
    validate_input(data.len(), 1)?;
    Ok(data.iter().map(|x| x.tan()).collect())
}

/// 双曲正切 (Vector Hyperbolic Tangent)
///
/// 计算每个元素的双曲正切值。
///
/// # 公式
/// tanh(x) = sinh(x) / cosh(x)
///
/// # 参数
/// * `data` - 输入数据序列
///
/// # 返回值
/// 双曲正切值数组，范围 `(-1, 1)`
///
/// # 示例
/// ```rust
/// use finkit::indicators::math_transform::tanh;
/// let data = vec![0.0, 1.0, -1.0];
/// let result = tanh(&data).unwrap();
/// assert_eq!(result.len(), 3);
/// ```
pub fn tanh(data: &[f64]) -> Result<Array1<f64>> {
    validate_input(data.len(), 1)?;
    Ok(data.iter().map(|x| x.tanh()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    const EPS: f64 = 1e-10;

    // ---------- acos ----------
    #[test]
    fn test_acos_basic() {
        let data = vec![1.0, 0.5, 0.0, -0.5, -1.0];
        let result = acos(&data).unwrap();
        assert_eq!(result.len(), 5);
        assert_relative_eq!(result[0], 0.0, epsilon = EPS);
        assert_relative_eq!(result[1], std::f64::consts::FRAC_PI_3, epsilon = 1e-10);
        assert_relative_eq!(result[2], std::f64::consts::FRAC_PI_2, epsilon = 1e-10);
        assert_relative_eq!(result[4], std::f64::consts::PI, epsilon = 1e-10);
    }

    #[test]
    fn test_acos_out_of_domain() {
        assert!(acos(&[2.0]).is_err());
        assert!(acos(&[-2.0]).is_err());
        assert!(acos(&[0.0, 1.5]).is_err());
    }

    #[test]
    fn test_acos_empty() {
        assert!(acos(&[]).is_err());
    }

    // ---------- asin ----------
    #[test]
    fn test_asin_basic() {
        let data = vec![0.0, 0.5, 1.0, -0.5, -1.0];
        let result = asin(&data).unwrap();
        assert_eq!(result.len(), 5);
        assert_relative_eq!(result[0], 0.0, epsilon = EPS);
        assert_relative_eq!(result[1], std::f64::consts::FRAC_PI_6, epsilon = 1e-10);
        assert_relative_eq!(result[2], std::f64::consts::FRAC_PI_2, epsilon = 1e-10);
        assert_relative_eq!(result[4], -std::f64::consts::FRAC_PI_2, epsilon = 1e-10);
    }

    #[test]
    fn test_asin_out_of_domain() {
        assert!(asin(&[2.0]).is_err());
        assert!(asin(&[-2.0]).is_err());
    }

    #[test]
    fn test_asin_empty() {
        assert!(asin(&[]).is_err());
    }

    // ---------- atan ----------
    #[test]
    fn test_atan_basic() {
        let data = vec![0.0, 1.0, -1.0];
        let result = atan(&data).unwrap();
        assert_eq!(result.len(), 3);
        assert_relative_eq!(result[0], 0.0, epsilon = EPS);
        assert_relative_eq!(result[1], std::f64::consts::FRAC_PI_4, epsilon = 1e-10);
        assert_relative_eq!(result[2], -std::f64::consts::FRAC_PI_4, epsilon = 1e-10);
    }

    #[test]
    fn test_atan_empty() {
        assert!(atan(&[]).is_err());
    }

    // ---------- ceil ----------
    #[test]
    fn test_ceil_basic() {
        let data = vec![1.2, 2.8, -1.7, 0.0, 3.0];
        let result = ceil(&data).unwrap();
        assert_eq!(result.len(), 5);
        assert_relative_eq!(result[0], 2.0, epsilon = EPS);
        assert_relative_eq!(result[1], 3.0, epsilon = EPS);
        assert_relative_eq!(result[2], -1.0, epsilon = EPS);
        assert_relative_eq!(result[3], 0.0, epsilon = EPS);
        assert_relative_eq!(result[4], 3.0, epsilon = EPS);
    }

    #[test]
    fn test_ceil_empty() {
        assert!(ceil(&[]).is_err());
    }

    // ---------- cos ----------
    #[test]
    fn test_cos_basic() {
        let data = vec![0.0, std::f64::consts::FRAC_PI_2, std::f64::consts::PI];
        let result = cos(&data).unwrap();
        assert_eq!(result.len(), 3);
        assert_relative_eq!(result[0], 1.0, epsilon = EPS);
        assert_relative_eq!(result[1], 0.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], -1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_cos_empty() {
        assert!(cos(&[]).is_err());
    }

    // ---------- cosh ----------
    #[test]
    fn test_cosh_basic() {
        let data = vec![0.0, 1.0, -1.0];
        let result = cosh(&data).unwrap();
        assert_eq!(result.len(), 3);
        assert_relative_eq!(result[0], 1.0, epsilon = EPS);
        // cosh(1) = (e + 1/e) / 2
        let expected_cosh1 = (std::f64::consts::E + 1.0 / std::f64::consts::E) / 2.0;
        assert_relative_eq!(result[1], expected_cosh1, epsilon = 1e-10);
        assert_relative_eq!(result[2], expected_cosh1, epsilon = 1e-10);
    }

    #[test]
    fn test_cosh_empty() {
        assert!(cosh(&[]).is_err());
    }

    // ---------- exp ----------
    #[test]
    fn test_exp_basic() {
        let data = vec![0.0, 1.0, 2.0];
        let result = exp(&data).unwrap();
        assert_eq!(result.len(), 3);
        assert_relative_eq!(result[0], 1.0, epsilon = EPS);
        assert_relative_eq!(result[1], std::f64::consts::E, epsilon = 1e-10);
        assert_relative_eq!(
            result[2],
            std::f64::consts::E * std::f64::consts::E,
            epsilon = 1e-10
        );
    }

    #[test]
    fn test_exp_empty() {
        assert!(exp(&[]).is_err());
    }

    // ---------- floor ----------
    #[test]
    fn test_floor_basic() {
        let data = vec![1.2, 2.8, -1.7, 0.0, 3.0];
        let result = floor(&data).unwrap();
        assert_eq!(result.len(), 5);
        assert_relative_eq!(result[0], 1.0, epsilon = EPS);
        assert_relative_eq!(result[1], 2.0, epsilon = EPS);
        assert_relative_eq!(result[2], -2.0, epsilon = EPS);
        assert_relative_eq!(result[3], 0.0, epsilon = EPS);
        assert_relative_eq!(result[4], 3.0, epsilon = EPS);
    }

    #[test]
    fn test_floor_empty() {
        assert!(floor(&[]).is_err());
    }

    // ---------- ln ----------
    #[test]
    fn test_ln_basic() {
        let data = vec![
            1.0,
            std::f64::consts::E,
            std::f64::consts::E * std::f64::consts::E,
        ];
        let result = ln(&data).unwrap();
        assert_eq!(result.len(), 3);
        assert_relative_eq!(result[0], 0.0, epsilon = EPS);
        assert_relative_eq!(result[1], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_ln_negative_or_zero() {
        assert!(ln(&[-1.0]).is_err());
        assert!(ln(&[0.0]).is_err());
        assert!(ln(&[0.0, 1.0]).is_err());
    }

    #[test]
    fn test_ln_empty() {
        assert!(ln(&[]).is_err());
    }

    // ---------- log10 ----------
    #[test]
    fn test_log10_basic() {
        let data = vec![1.0, 10.0, 100.0, 1000.0];
        let result = log10(&data).unwrap();
        assert_eq!(result.len(), 4);
        assert_relative_eq!(result[0], 0.0, epsilon = EPS);
        assert_relative_eq!(result[1], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], 2.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 3.0, epsilon = 1e-10);
    }

    #[test]
    fn test_log10_negative_or_zero() {
        assert!(log10(&[-1.0]).is_err());
        assert!(log10(&[0.0]).is_err());
    }

    #[test]
    fn test_log10_empty() {
        assert!(log10(&[]).is_err());
    }

    // ---------- sin ----------
    #[test]
    fn test_sin_basic() {
        let data = vec![0.0, std::f64::consts::FRAC_PI_2, std::f64::consts::PI];
        let result = sin(&data).unwrap();
        assert_eq!(result.len(), 3);
        assert_relative_eq!(result[0], 0.0, epsilon = EPS);
        assert_relative_eq!(result[1], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_sin_empty() {
        assert!(sin(&[]).is_err());
    }

    // ---------- sinh ----------
    #[test]
    fn test_sinh_basic() {
        let data = vec![0.0, 1.0, -1.0];
        let result = sinh(&data).unwrap();
        assert_eq!(result.len(), 3);
        assert_relative_eq!(result[0], 0.0, epsilon = EPS);
        // sinh(1) = (e - 1/e) / 2
        let expected = (std::f64::consts::E - 1.0 / std::f64::consts::E) / 2.0;
        assert_relative_eq!(result[1], expected, epsilon = 1e-10);
        assert_relative_eq!(result[2], -expected, epsilon = 1e-10);
    }

    #[test]
    fn test_sinh_empty() {
        assert!(sinh(&[]).is_err());
    }

    // ---------- sqrt ----------
    #[test]
    fn test_sqrt_basic() {
        let data = vec![4.0];
        let result = sqrt(&data).unwrap();
        assert_eq!(result.len(), 1);
        assert_relative_eq!(result[0], 2.0, epsilon = EPS);
    }

    #[test]
    fn test_sqrt_multiple() {
        let data = vec![0.0, 1.0, 4.0, 9.0, 16.0];
        let result = sqrt(&data).unwrap();
        assert_eq!(result.len(), 5);
        assert_relative_eq!(result[0], 0.0, epsilon = EPS);
        assert_relative_eq!(result[1], 1.0, epsilon = EPS);
        assert_relative_eq!(result[2], 2.0, epsilon = EPS);
        assert_relative_eq!(result[3], 3.0, epsilon = EPS);
        assert_relative_eq!(result[4], 4.0, epsilon = EPS);
    }

    #[test]
    fn test_sqrt_negative() {
        assert!(sqrt(&[-1.0]).is_err());
        assert!(sqrt(&[0.0, -4.0]).is_err());
    }

    #[test]
    fn test_sqrt_empty() {
        assert!(sqrt(&[]).is_err());
    }

    // ---------- tan ----------
    #[test]
    fn test_tan_basic() {
        let data = vec![0.0, std::f64::consts::FRAC_PI_4];
        let result = tan(&data).unwrap();
        assert_eq!(result.len(), 2);
        assert_relative_eq!(result[0], 0.0, epsilon = EPS);
        assert_relative_eq!(result[1], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_tan_empty() {
        assert!(tan(&[]).is_err());
    }

    // ---------- tanh ----------
    #[test]
    fn test_tanh_basic() {
        let data = vec![0.0, 1.0, -1.0];
        let result = tanh(&data).unwrap();
        assert_eq!(result.len(), 3);
        assert_relative_eq!(result[0], 0.0, epsilon = EPS);
        // tanh(1) = (e - 1/e) / (e + 1/e)
        let e = std::f64::consts::E;
        let pos = (e - 1.0 / e) / (e + 1.0 / e);
        assert_relative_eq!(result[1], pos, epsilon = 1e-10);
        assert_relative_eq!(result[2], -pos, epsilon = 1e-10);
    }

    #[test]
    fn test_tanh_range() {
        // 验证 tanh 输出在 (-1, 1) 内（极值趋近 ±1）
        let data = vec![-100.0, -1.0, 0.0, 1.0, 100.0];
        let result = tanh(&data).unwrap();
        for &v in result.iter() {
            assert!(v >= -1.0 && v <= 1.0);
        }
        // 极值应趋近 ±1
        assert_relative_eq!(result[0], -1.0, epsilon = 1e-10);
        assert_relative_eq!(result[4], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_tanh_empty() {
        assert!(tanh(&[]).is_err());
    }

    // ---------- 交叉验证：sin^2 + cos^2 = 1 ----------
    #[test]
    fn test_sin_cos_identity() {
        let data = vec![0.1, 0.5, 1.0, 1.5, 2.0];
        let s = sin(&data).unwrap();
        let c = cos(&data).unwrap();
        for i in 0..data.len() {
            let v = s[i] * s[i] + c[i] * c[i];
            assert_relative_eq!(v, 1.0, epsilon = 1e-10);
        }
    }

    // ---------- 交叉验证：cosh^2 - sinh^2 = 1 ----------
    #[test]
    fn test_cosh_sinh_identity() {
        let data = vec![0.1, 0.5, 1.0, 1.5, 2.0];
        let ch = cosh(&data).unwrap();
        let sh = sinh(&data).unwrap();
        for i in 0..data.len() {
            let v = ch[i] * ch[i] - sh[i] * sh[i];
            assert_relative_eq!(v, 1.0, epsilon = 1e-10);
        }
    }
}
