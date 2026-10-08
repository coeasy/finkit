//! Money flow index.

use super::prelude::*;

/// Money Flow Index (MFI)
///
/// A momentum indicator that uses both price and volume to identify overbought/oversold conditions.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `period` - Lookback period
///
/// # Returns
/// Array of MFI values (0-100 range)
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let close = vec![44.0, 44.25, 44.5, 43.75, 44.5, 44.25, 44.0, 43.5, 44.0, 44.25];
/// let volume = vec![100.0, 110.0, 120.0, 115.0, 130.0, 125.0, 105.0, 95.0, 110.0, 115.0];
/// let result = indicators::mfi(&high, &low, &close, &volume, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn mfi(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    period: usize,
) -> Result<Array1<f64>> {
    crate::math::mfi::mfi(high, low, close, volume, period)
}
