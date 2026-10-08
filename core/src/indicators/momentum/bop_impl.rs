//! Balance of power.

use super::prelude::*;

/// Balance of Power (BOP)
///
/// Measures the strength of buyers vs sellers in the market.
///
/// # Arguments
/// * `open` - Open prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
///
/// # Returns
/// Array of BOP values
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
/// let result = indicators::bop(&open, &high, &low, &close).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn bop(open: &[f64], high: &[f64], low: &[f64], close: &[f64]) -> Result<Array1<f64>> {
    if open.len() != high.len() || open.len() != low.len() || open.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "open, high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(open.len(), 1)?;

    let len = open.len();
    let mut buf = vec![0.0f64; len];
    simd_ops::simd_bop(open, high, low, close, &mut buf);

    Ok(Array1::from_vec(buf))
}
