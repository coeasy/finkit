use crate::error::{Result, TaError};
use crate::utils::validate_input;
use ndarray::Array1;

#[inline]
fn validate_ohlc(high: &[f64], low: &[f64], close: &[f64]) -> Result<()> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    Ok(())
}

#[inline]
fn validate_output(input_len: usize, output_len: usize) -> Result<()> {
    if input_len != output_len {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    Ok(())
}

#[inline]
fn true_range_at(high: &[f64], low: &[f64], close: &[f64], index: usize) -> f64 {
    let h = high[index];
    let l = low[index];
    let previous_close = close[index - 1];
    (h - l)
        .max((h - previous_close).abs())
        .max((l - previous_close).abs())
}

/// Average True Range (ATR), TA-Lib-compatible Wilder semantics.
pub fn atr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_ohlc(high, low, close)?;
    validate_input(high.len(), period + 1)?;
    let mut output = Array1::<f64>::zeros(high.len());
    atr_into(
        high,
        low,
        close,
        period,
        output.as_slice_mut().expect("owned Array1 is contiguous"),
    )?;
    Ok(output)
}

/// Compute ATR directly into caller-owned output.
///
/// The first valid value is at `period`; rows before it are NaN. The seed is
/// `SMA(TR[1..=period])`, matching TA-Lib's previous-close requirement.
pub fn atr_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<()> {
    validate_ohlc(high, low, close)?;
    validate_input(high.len(), period + 1)?;
    validate_output(high.len(), output.len())?;

    output[..period].fill(f64::NAN);

    let mut tr_sum = 0.0;
    for index in 1..=period {
        tr_sum += true_range_at(high, low, close, index);
    }
    let mut previous_atr = tr_sum / period as f64;
    output[period] = previous_atr;

    let inverse_period = 1.0 / period as f64;
    for index in (period + 1)..high.len() {
        let true_range = true_range_at(high, low, close, index);
        previous_atr += (true_range - previous_atr) * inverse_period;
        output[index] = previous_atr;
    }
    Ok(())
}

/// Normalized Average True Range (NATR), expressed as a percentage of close.
pub fn natr(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Result<Array1<f64>> {
    validate_ohlc(high, low, close)?;
    validate_input(high.len(), period + 1)?;
    let mut output = Array1::<f64>::zeros(high.len());
    natr_into(
        high,
        low,
        close,
        period,
        output.as_slice_mut().expect("owned Array1 is contiguous"),
    )?;
    Ok(output)
}

/// Compute NATR directly into caller-owned output without allocating an ATR array.
pub fn natr_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<()> {
    validate_ohlc(high, low, close)?;
    validate_input(high.len(), period + 1)?;
    validate_output(high.len(), output.len())?;

    output[..period].fill(f64::NAN);

    let mut tr_sum = 0.0;
    for index in 1..=period {
        tr_sum += true_range_at(high, low, close, index);
    }
    let mut previous_atr = tr_sum / period as f64;

    let seed_close = close[period];
    output[period] = if seed_close.abs() > 1e-15 {
        previous_atr / seed_close * 100.0
    } else {
        0.0
    };

    let inverse_period = 1.0 / period as f64;
    for index in (period + 1)..high.len() {
        let true_range = true_range_at(high, low, close, index);
        previous_atr += (true_range - previous_atr) * inverse_period;
        let current_close = close[index];
        output[index] = if current_close.abs() > 1e-15 {
            previous_atr / current_close * 100.0
        } else {
            0.0
        };
    }
    Ok(())
}

/// True Range (TRANGE) with TA-Lib-compatible first-row warm-up.
pub fn trange(high: &[f64], low: &[f64], close: &[f64]) -> Result<Array1<f64>> {
    validate_ohlc(high, low, close)?;
    validate_input(high.len(), 1)?;
    let mut output = Array1::<f64>::zeros(high.len());
    trange_into(
        high,
        low,
        close,
        output.as_slice_mut().expect("owned Array1 is contiguous"),
    )?;
    Ok(output)
}

/// Compute True Range directly into caller-owned output.
///
/// TA-Lib has no previous close for row zero, so `output[0]` is NaN. Remaining
/// values are written by the SIMD true-range primitive in one pass.
pub fn trange_into(high: &[f64], low: &[f64], close: &[f64], output: &mut [f64]) -> Result<()> {
    validate_ohlc(high, low, close)?;
    validate_input(high.len(), 1)?;
    validate_output(high.len(), output.len())?;

    output[0] = f64::NAN;
    if high.len() > 1 {
        crate::math::simd_ops::simd_true_range(
            &high[1..],
            &low[1..],
            &close[..close.len() - 1],
            &mut output[1..],
        );
    }
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
    fn trange_has_talib_warmup_and_into_matches() {
        let high = vec![10.0, 12.0, 14.0];
        let low = vec![8.0, 10.0, 12.0];
        let close = vec![9.0, 11.0, 13.0];
        let result = trange(&high, &low, &close).unwrap();
        assert!(result[0].is_nan());
        assert_relative_eq!(result[1], 3.0, epsilon = 1e-10);

        let mut output = vec![0.0; high.len()];
        trange_into(&high, &low, &close, &mut output).unwrap();
        assert_array_matches_slice(&result, &output);
    }

    #[test]
    fn atr_into_matches_allocating_api() {
        let high = vec![10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0];
        let low = vec![8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
        let close = vec![9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0];
        let expected = atr(&high, &low, &close, 5).unwrap();
        let mut output = vec![0.0; high.len()];
        atr_into(&high, &low, &close, 5, &mut output).unwrap();
        assert_array_matches_slice(&expected, &output);
        assert!(output[..5].iter().all(|value| value.is_nan()));
        assert!(output[5].is_finite());
    }

    #[test]
    fn natr_into_matches_allocating_api() {
        let high = vec![10.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0];
        let low = vec![8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
        let close = vec![9.0, 11.0, 13.0, 15.0, 17.0, 19.0, 21.0];
        let expected = natr(&high, &low, &close, 5).unwrap();
        let mut output = vec![0.0; high.len()];
        natr_into(&high, &low, &close, 5, &mut output).unwrap();
        assert_array_matches_slice(&expected, &output);
        assert!(output[..5].iter().all(|value| value.is_nan()));
        assert!(output[5].is_finite());
    }
}
