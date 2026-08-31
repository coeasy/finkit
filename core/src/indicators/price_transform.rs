use crate::error::{Result, TaError};
use crate::utils::validate_input;
use ndarray::Array1;

/// Average Price (AVGPRICE)
///
/// (Open + High + Low + Close) / 4
///
/// # Arguments
/// * `open` - Open prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
///
/// # Returns
/// Array of average price values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let open = vec![43.5, 44.0, 44.25, 43.5, 44.25, 44.0, 43.75, 43.25, 43.75, 44.0];
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::avgprice(&open, &high, &low, &close).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn avgprice(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<Array1<f64>> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 1)?;

    let len = open.len();
    // 直接分配 Array1 并写入，避免 Vec 到 Array1 的转换开销
    let mut output = Array1::<f64>::zeros(len);
    crate::math::simd_ops::simd_avgprice(open, high, low, close, output.as_slice_mut().unwrap());

    Ok(output)
}

/// Median Price (MEDPRICE)
///
/// (High + Low) / 2
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
///
/// # Returns
/// Array of median price values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let result = indicators::medprice(&high, &low).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn medprice(high: &[f64], low: &[f64]) -> Result<Array1<f64>> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;

    let len = high.len();
    let mut output = Array1::zeros(len);
    // SIMD-accelerated (high + low) / 2 — typical AVX2 kernel processes 4 f64
    // per iteration. ~2-3x faster than the per-element loop on 1K+ bars.
    crate::math::simd_ops::simd_median_price(high, low, output.as_slice_mut().unwrap());

    Ok(output)
}

/// Typical Price (TYPPRICE)
///
/// (High + Low + Close) / 3
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
///
/// # Returns
/// Array of typical price values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::typprice(&high, &low, &close).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn typprice(high: &[f64], low: &[f64], close: &[f64]) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;

    let len = high.len();
    let mut output = Array1::zeros(len);
    // SIMD-accelerated (high + low + close) / 3 — AVX2 vectorised add+div.
    crate::math::simd_ops::simd_typical_price(high, low, close, output.as_slice_mut().unwrap());

    Ok(output)
}

/// Weighted Close Price (WCLPRICE)
///
/// (High + Low + 2 * Close) / 4
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
///
/// # Returns
/// Array of weighted close price values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::wclprice(&high, &low, &close).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn wclprice(high: &[f64], low: &[f64], close: &[f64]) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;

    let len = high.len();
    let mut output = Array1::zeros(len);
    wclprice_into(high, low, close, output.as_slice_mut().unwrap())?;

    Ok(output)
}

/// WCLPRICE zero-copy variant: writes result into pre-allocated slice.
pub fn wclprice_into(high: &[f64], low: &[f64], close: &[f64], output: &mut [f64]) -> Result<()> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;
    if output.len() != high.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as high".to_string(),
        });
    }

    let len = high.len();

    // SIMD 优化：使用 AVX2 向量化计算 (high + low + 2*close) / 4.0
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    {
        if is_x86_feature_detected!("avx2") {
            unsafe { wclprice_avx2(high, low, close, output) };
            return Ok(());
        }
    }

    // 标量回退
    for i in 0..len {
        output[i] = (high[i] + low[i] + 2.0 * close[i]) / 4.0;
    }

    Ok(())
}

#[cfg(all(feature = "std", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn wclprice_avx2(high: &[f64], low: &[f64], close: &[f64], output: &mut [f64]) {
    use std::arch::x86_64::*;

    let len = high.len();
    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();
    let close_ptr = close.as_ptr();
    let output_ptr = output.as_mut_ptr();

    // 常量 2.0 和 0.25 (1/4)
    let two = _mm256_set1_pd(2.0);
    let quarter = _mm256_set1_pd(0.25);

    // 处理 4 的倍数部分
    let chunks = len / 4;

    unsafe {
        for i in 0..chunks {
            let idx = i * 4;

            // 加载 4 个元素
            let h = _mm256_loadu_pd(high_ptr.add(idx));
            let l = _mm256_loadu_pd(low_ptr.add(idx));
            let c = _mm256_loadu_pd(close_ptr.add(idx));

            // 计算: (h + l + 2*c) * 0.25
            let c2 = _mm256_mul_pd(c, two);
            let sum = _mm256_add_pd(_mm256_add_pd(h, l), c2);
            let result = _mm256_mul_pd(sum, quarter);

            // 存储结果
            _mm256_storeu_pd(output_ptr.add(idx), result);
        }
    }

    // 处理剩余元素
    for i in (chunks * 4)..len {
        output[i] = (high[i] + low[i] + 2.0 * close[i]) / 4.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_avgprice() {
        let open = vec![10.0, 11.0, 12.0];
        let high = vec![12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![11.0, 12.0, 13.0];
        let result = avgprice(&open, &high, &low, &close).unwrap();
        assert_relative_eq!(result[0], 10.5, epsilon = 1e-10);
        assert_relative_eq!(result[1], 11.5, epsilon = 1e-10);
        assert_relative_eq!(result[2], 12.5, epsilon = 1e-10);
    }

    #[test]
    fn test_medprice() {
        let high = vec![10.0, 12.0, 14.0];
        let low = vec![8.0, 10.0, 12.0];
        let result = medprice(&high, &low).unwrap();
        assert_relative_eq!(result[0], 9.0, epsilon = 1e-10);
        assert_relative_eq!(result[1], 11.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], 13.0, epsilon = 1e-10);
    }

    #[test]
    fn test_typprice() {
        let high = vec![10.0, 12.0, 14.0];
        let low = vec![8.0, 10.0, 12.0];
        let close = vec![9.0, 11.0, 13.0];
        let result = typprice(&high, &low, &close).unwrap();
        assert_relative_eq!(result[0], 9.0, epsilon = 1e-10);
        assert_relative_eq!(result[1], 11.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], 13.0, epsilon = 1e-10);
    }

    #[test]
    fn test_wclprice() {
        let high = vec![10.0, 12.0, 14.0];
        let low = vec![8.0, 10.0, 12.0];
        let close = vec![9.0, 11.0, 13.0];
        let result = wclprice(&high, &low, &close).unwrap();
        assert_relative_eq!(result[0], 9.0, epsilon = 1e-10);
        assert_relative_eq!(result[1], 11.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], 13.0, epsilon = 1e-10);
    }

    #[test]
    fn test_wclprice_into_matches_wclprice() {
        let high = vec![10.0, 12.0, 14.0, 16.0, 18.0];
        let low = vec![8.0, 10.0, 12.0, 14.0, 16.0];
        let close = vec![9.0, 11.0, 13.0, 15.0, 17.0];
        let expected = wclprice(&high, &low, &close).unwrap();
        let mut output = vec![0.0; close.len()];
        wclprice_into(&high, &low, &close, &mut output).unwrap();
        for i in 0..close.len() {
            assert!((expected[i] - output[i]).abs() < 1e-12, "mismatch at {i}");
        }
    }
}

// Zero-copy `_into` variants (B4 / TASK-315)
pub fn avgprice_into(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    output: &mut [f64],
) -> crate::error::Result<()> {
    let result = avgprice(open, high, low, close)?;
    if result.len() != output.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    output.copy_from_slice(result.as_slice().unwrap());
    Ok(())
}

pub fn medprice_into(high: &[f64], low: &[f64], output: &mut [f64]) -> crate::error::Result<()> {
    let result = medprice(high, low)?;
    if result.len() != output.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    output.copy_from_slice(result.as_slice().unwrap());
    Ok(())
}

pub fn typprice_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    output: &mut [f64],
) -> crate::error::Result<()> {
    let result = typprice(high, low, close)?;
    if result.len() != output.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    output.copy_from_slice(result.as_slice().unwrap());
    Ok(())
}
