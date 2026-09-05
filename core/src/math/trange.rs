//! Single-write TA-Lib-compatible True Range kernel.
//!
//! TRANGE is a tiny serial formula. The legacy allocating wrapper first
//! zero-initialised the entire output array and then overwrote every element,
//! while the caller-owned path also paid the generic SIMD-dispatch cost.  This
//! hot kernel writes each output slot exactly once and keeps TA-Lib's row-zero
//! warm-up contract unchanged.

use crate::error::{Result, TaError};
use crate::utils::validate_input;
use ndarray::Array1;

/// True Range with TA-Lib-compatible first-row NaN warm-up.
pub fn trange(high: &[f64], low: &[f64], close: &[f64]) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;

    let len = high.len();
    let mut output = Vec::with_capacity(len);
    output.push(f64::NAN);

    unsafe {
        let high_ptr = high.as_ptr();
        let low_ptr = low.as_ptr();
        let close_ptr = close.as_ptr();
        for index in 1..len {
            let h = *high_ptr.add(index);
            let l = *low_ptr.add(index);
            let previous_close = *close_ptr.add(index - 1);
            output.push(
                (h - l)
                    .max((h - previous_close).abs())
                    .max((l - previous_close).abs()),
            );
        }
    }

    debug_assert_eq!(output.len(), len);
    Ok(Array1::from_vec(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_talib_row_zero_warmup() {
        let high = [10.0, 12.0, 14.0];
        let low = [8.0, 10.0, 12.0];
        let close = [9.0, 11.0, 13.0];
        let output = trange(&high, &low, &close).unwrap();
        assert!(output[0].is_nan());
        assert_eq!(output[1], 3.0);
        assert_eq!(output[2], 3.0);
    }
}
