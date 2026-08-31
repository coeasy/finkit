use crate::error::{Result, TaError};
use crate::utils::{init_output, validate_input};
use ndarray::Array1;

/// Average True Range (ATR)
///
/// A volatility indicator that measures the average price range over a given period.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - Lookback period
///
/// # Returns
/// Array of ATR values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::atr(&high, &low, &close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn atr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period + 1)?;

    let len = high.len();
    let mut atr_values = Array1::<f64>::zeros(len);

    // TA-Lib 兼容：warm-up 区域为 NaN（前 period 个值，首有效值在 period 位置）
    for i in 0..period {
        atr_values[i] = f64::NAN;
    }

    // TA-Lib 兼容：首个 ATR = TR[1..=period] 的 SMA（不含 TR[0]）
    let mut atr_sum = 0.0;
    for i in 1..=period {
        let h = high[i];
        let l = low[i];
        let pc = close[i - 1];
        let tr_i = (h - l).max((h - pc).abs()).max((l - pc).abs());
        atr_sum += tr_i;
    }
    let mut prev_atr = atr_sum / period as f64;
    atr_values[period] = prev_atr;

    // Wilder's RMA: ATR[i] = (ATR[i-1] * (period-1) + TR[i]) / period
    let inv_period = 1.0 / period as f64;

    // 内联 true_range 计算，避免函数调用开销
    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();
    let close_ptr = close.as_ptr();
    let atr_ptr = atr_values.as_mut_ptr();

    unsafe {
        for i in (period + 1)..len {
            let h = *high_ptr.add(i);
            let l = *low_ptr.add(i);
            let pc = *close_ptr.add(i - 1);
            let tr_i = (h - l).max((h - pc).abs()).max((l - pc).abs());
            prev_atr = prev_atr + (tr_i - prev_atr) * inv_period;
            *atr_ptr.add(i) = prev_atr;
        }
    }

    Ok(atr_values)
}

/// Compute ATR writing results into a pre-allocated buffer.
///
/// `output` must have the same length as `high`. Warm-up values are written as NaN.
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let mut output = vec![0.0; high.len()];
/// indicators::atr_into(&high, &low, &close, 5, &mut output).unwrap();
/// assert_eq!(output.len(), 10);
/// ```
pub fn atr_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<()> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period + 1)?;
    if output.len() != high.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as high".to_string(),
        });
    }

    let len = high.len();

    // TA-Lib 兼容：warm-up region 为 NaN（前 period 个值，首有效值在 period 位置）
    for i in 0..period {
        output[i] = f64::NAN;
    }

    // TA-Lib 兼容：首个 ATR = TR[1..=period] 的 SMA（不含 TR[0]）
    let mut tr_sum = 0.0;
    for i in 1..=period {
        let h = high[i];
        let l = low[i];
        let pc = close[i - 1];
        let tr_i = (h - l).max((h - pc).abs()).max((l - pc).abs());
        tr_sum += tr_i;
    }
    let mut prev_atr = tr_sum / period as f64;
    output[period] = prev_atr;

    // Wilder's RMA: ATR[i] = ATR[i-1] + (TR[i] - ATR[i-1]) / period
    let inv_period = 1.0 / period as f64;

    let high_ptr = high.as_ptr();
    let low_ptr = low.as_ptr();
    let close_ptr = close.as_ptr();
    let out_ptr = output.as_mut_ptr();

    unsafe {
        for i in (period + 1)..len {
            let h = *high_ptr.add(i);
            let l = *low_ptr.add(i);
            let pc = *close_ptr.add(i - 1);
            let tr_i = (h - l).max((h - pc).abs()).max((l - pc).abs());
            prev_atr += (tr_i - prev_atr) * inv_period;
            *out_ptr.add(i) = prev_atr;
        }
    }

    Ok(())
}

/// Normalized Average True Range (NATR)
///
/// The ATR expressed as a percentage of the close price.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - Lookback period
///
/// # Returns
/// Array of NATR values (in percentage)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::natr(&high, &low, &close, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn natr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period + 1)?;

    let len = high.len();
    let mut output = init_output(len);

    // TA-Lib 兼容：warm-up region 为 NaN（前 period 个值，首有效值在 period 位置）
    // TA-Lib NATR 与 ATR 共享相同的 ATR 计算，只是多了归一化。
    // 种子 = SMA(TR[1..=period])，不含 TR[0]（因为 TR 需要 prev close）。

    // 计算种子：TR[1] 到 TR[period] 的 SMA
    let mut tr_sum = 0.0;
    for i in 1..=period {
        let h = high[i];
        let l = low[i];
        let pc = close[i - 1];
        let tr_i = (h - l).max((h - pc).abs()).max((l - pc).abs());
        tr_sum += tr_i;
    }
    let mut prev_atr = tr_sum / period as f64;

    // 归一化首个输出：NATR = ATR / close * 100
    let c = close[period];
    if c.abs() > 1e-15 {
        output[period] = prev_atr / c * 100.0;
    }

    // Wilder's RMA: ATR[i] = ATR[i-1] + (TR[i] - ATR[i-1]) / period
    let inv_period = 1.0 / period as f64;

    for i in (period + 1)..len {
        let h = high[i];
        let l = low[i];
        let pc = close[i - 1];
        let tr_i = (h - l).max((h - pc).abs()).max((l - pc).abs());
        prev_atr += (tr_i - prev_atr) * inv_period;
        let c = close[i];
        if c.abs() > 1e-15 {
            output[i] = prev_atr / c * 100.0;
        }
    }

    Ok(output)
}

/// True Range (TRANGE)
///
/// The greatest of the following:
/// - High - Low
/// - |High - Previous Close|
/// - |Low - Previous Close|
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
///
/// # Returns
/// Array of True Range values
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let result = indicators::trange(&high, &low, &close).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn trange(high: &[f64], low: &[f64], close: &[f64]) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;

    let len = high.len();
    let mut output = Array1::zeros(len);

    output[0] = high[0] - low[0];

    // SIMD-accelerated true range calculation
    // Create prev_close slice (close[0..len-1]) for SIMD kernel
    let prev_close = &close[..len - 1];
    let high_slice = &high[1..];
    let low_slice = &low[1..];
    let output_slice = &mut output.as_slice_mut().unwrap()[1..];

    crate::math::simd_ops::simd_true_range(high_slice, low_slice, prev_close, output_slice);

    Ok(output)
}

/// NATR zero-copy variant: writes result into pre-allocated slice.
pub fn natr_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<()> {
    let result = natr(high, low, close, period)?;
    if output.len() != high.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    output.copy_from_slice(result.as_slice().unwrap());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn assert_array_matches_slice(a: &Array1<f64>, b: &[f64]) {
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            if x.is_nan() {
                assert!(y.is_nan());
            } else {
                assert_relative_eq!(*x, *y, epsilon = 1e-15);
            }
        }
    }

    #[test]
    fn test_trange() {
        let high = vec![10.0, 12.0, 14.0];
        let low = vec![8.0, 10.0, 12.0];
        let close = vec![9.0, 11.0, 13.0];
        let result = trange(&high, &low, &close).unwrap();
        assert_relative_eq!(result[0], 2.0, epsilon = 1e-10);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_atr() {
        let high = vec![10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0];
        let low = vec![8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
        let close = vec![9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0];
        let result = atr(&high, &low, &close, 5).unwrap();
        // ATR 首有效值位于索引 period（与 TA-Lib 兼容：SMA(TR[1..=period])）
        assert!(!result[5].is_nan());
    }

    #[test]
    fn test_atr_into_matches_atr() {
        let high = vec![10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0];
        let low = vec![8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
        let close = vec![9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0];
        let expected = atr(&high, &low, &close, 5).unwrap();
        let mut output = vec![0.0; high.len()];
        atr_into(&high, &low, &close, 5, &mut output).unwrap();
        assert_array_matches_slice(&expected, &output);
    }

    #[test]
    fn test_natr() {
        let high = vec![10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0];
        let low = vec![8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
        let close = vec![9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0];
        let result = natr(&high, &low, &close, 5).unwrap();
        // TA-Lib 兼容：首有效值在 period 位置（SMA(TR[1..=period]) / close * 100）
        assert!(!result[5].is_nan());
        assert!(result[4].is_nan());
    }
}

// Zero-copy `_into` variant (B4 / TASK-315)
pub fn trange_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    output: &mut [f64],
) -> crate::error::Result<()> {
    let result = trange(high, low, close)?;
    if result.len() != output.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    output.copy_from_slice(result.as_slice().unwrap());
    Ok(())
}
