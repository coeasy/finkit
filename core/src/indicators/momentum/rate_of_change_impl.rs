//! Momentum and rate-of-change family (MOM/ROC/ROCP/ROCR).

use super::prelude::*;

/// Momentum (MOM)
///
/// Measures the change in price over a given period.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
///
/// # Returns
/// Array of momentum values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::mom(&close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn mom(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period + 1)?;

    let len = input.len();
    // 直接分配 Array1 并写入，避免中间 Vec 分配
    let mut output = Array1::<f64>::zeros(len);
    simd_ops::simd_mom(input, period, output.as_slice_mut().unwrap());

    Ok(output)
}

/// Rate of Change (ROC)
///
/// Measures the percentage change in price over a given period.
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
///
/// # Returns
/// Array of ROC values (in percentage)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::roc(&close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn roc(input: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_input(input.len(), period + 1)?;

    let len = input.len();
    let mut buf = vec![0.0f64; len];
    simd_ops::simd_roc(input, period, &mut buf);

    Ok(Array1::from_vec(buf))
}
/// Rate of Change Percentage (ROCP)
///
/// ROCP = (close - close_n) / close_n
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::rocp(&close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn rocp(input: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_input(input.len(), period + 1)?;
    let len = input.len();
    let mut output = init_output(len);

    for i in period..len {
        if !crate::utils::is_zero(input[i - period]) {
            output[i] = (input[i] - input[i - period]) / input[i - period];
        }
    }

    Ok(output)
}

/// Rate of Change Ratio (ROCR)
///
/// ROCR = close / close_n
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::rocr(&close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn rocr(input: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_input(input.len(), period + 1)?;
    let len = input.len();
    let mut output = init_output(len);

    for i in period..len {
        if !crate::utils::is_zero(input[i - period]) {
            output[i] = input[i] / input[i - period];
        }
    }

    Ok(output)
}

/// Rate of Change Ratio scaled to 100 (ROCR100)
///
/// ROCR100 = (close / close_n) * 100
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::rocr100(&close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn rocr100(input: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_input(input.len(), period + 1)?;
    let len = input.len();
    let mut output = init_output(len);

    for i in period..len {
        if !crate::utils::is_zero(input[i - period]) {
            output[i] = (input[i] / input[i - period]) * 100.0;
        }
    }

    Ok(output)
}

/// Momentum zero-copy variant: writes result into pre-allocated slice.
pub fn mom_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(input.len(), period + 1)?;
    // Keep the formula zero-copy path allocation-free.  The public `mom`
    // wrapper uses the same kernel, so this is bit-for-bit equivalent while
    // avoiding an intermediate Array1 and a full-length copy.
    simd_ops::simd_mom(input, period, output);
    Ok(())
}

/// Rate of Change zero-copy variant: writes result into pre-allocated slice.
pub fn roc_into(input: &[f64], period: usize, output: &mut [f64]) -> Result<()> {
    if output.len() != input.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    validate_input(input.len(), period + 1)?;
    // ROC is already implemented as a caller-owned SIMD kernel.  Route the
    // formula executor directly to it instead of allocating an Array1 and
    // copying the result back into the caller's buffer.
    simd_ops::simd_roc(input, period, output);
    Ok(())
}
