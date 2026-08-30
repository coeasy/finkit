use crate::error::{Result, TaError};
use crate::math::statistics::{rolling_max, rolling_min};
use crate::utils::{init_output, validate_input};
use ndarray::Array1;

/// Donchian Channel Result
///
/// Contains all components of the Donchian Channel indicator.
#[derive(Debug, Clone)]
pub struct DonchianResult {
    /// Upper Band - N-period highest high
    pub upper: Array1<f64>,
    /// Lower Band - N-period lowest low
    pub lower: Array1<f64>,
    /// Middle Band - (Upper + Lower) / 2
    pub middle: Array1<f64>,
    /// Width - Upper - Lower
    pub width: Array1<f64>,
}

/// Donchian Channel (DONCHIAN)
///
/// A trend-following indicator that displays the highest and lowest prices over a given period.
/// The upper band represents the highest high, the lower band represents the lowest low,
/// and the middle band is their average. The width shows the volatility range.
///
/// # Formula
/// - Upper Band = Highest High over N periods
/// - Lower Band = Lowest Low over N periods
/// - Middle Band = (Upper Band + Lower Band) / 2
/// - Width = Upper Band - Lower Band
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `period` - Lookback period (must be >= 1)
///
/// # Returns
/// DonchianResult containing upper, lower, middle bands and width
///
/// # Example
/// ```rust
/// use alpha_ta_core::indicators::donchian;
///
/// let high = vec![10.0, 12.0, 14.0, 13.0, 15.0, 16.0];
/// let low = vec![8.0, 10.0, 11.0, 10.0, 12.0, 13.0];
/// let result = donchian(&high, &low, 5).unwrap();
///
/// // result.upper[4] is the highest high in periods 0-4
/// // result.lower[4] is the lowest low in periods 0-4
/// // result.middle[4] is (upper[4] + lower[4]) / 2
/// // result.width[4] is upper[4] - lower[4]
/// ```
pub fn donchian(high: &[f64], low: &[f64], period: usize) -> Result<DonchianResult> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if period == 0 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }
    validate_input(high.len(), period)?;

    let upper = rolling_max(high, period)?;
    let lower = rolling_min(low, period)?;

    let len = high.len();
    let mut middle = init_output(len);
    let mut width = init_output(len);

    for i in 0..len {
        if !upper[i].is_nan() && !lower[i].is_nan() {
            middle[i] = (upper[i] + lower[i]) / 2.0;
            width[i] = upper[i] - lower[i];
        }
    }

    Ok(DonchianResult {
        upper,
        lower,
        middle,
        width,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_donchian_basic() {
        let high = vec![10.0, 12.0, 14.0, 13.0, 15.0, 16.0];
        let low = vec![8.0, 10.0, 11.0, 10.0, 12.0, 13.0];
        let result = donchian(&high, &low, 5).unwrap();

        // First 4 values should be NaN (need 5 periods)
        assert!(result.upper[0].is_nan());
        assert!(result.upper[1].is_nan());
        assert!(result.upper[2].is_nan());
        assert!(result.upper[3].is_nan());

        // Period 4: highest high in [10, 12, 14, 13, 15] = 15
        assert_relative_eq!(result.upper[4], 15.0, epsilon = 1e-10);
        // Period 4: lowest low in [8, 10, 11, 10, 12] = 8
        assert_relative_eq!(result.lower[4], 8.0, epsilon = 1e-10);
        // Middle = (15 + 8) / 2 = 11.5
        assert_relative_eq!(result.middle[4], 11.5, epsilon = 1e-10);
        // Width = 15 - 8 = 7
        assert_relative_eq!(result.width[4], 7.0, epsilon = 1e-10);
    }

    #[test]
    fn test_donchian_period_1() {
        let high = vec![10.0, 12.0, 14.0, 13.0, 15.0];
        let low = vec![8.0, 10.0, 11.0, 10.0, 12.0];
        let result = donchian(&high, &low, 1).unwrap();

        // With period 1, all values should be valid
        for i in 0..high.len() {
            assert_relative_eq!(result.upper[i], high[i], epsilon = 1e-10);
            assert_relative_eq!(result.lower[i], low[i], epsilon = 1e-10);
            assert_relative_eq!(result.middle[i], (high[i] + low[i]) / 2.0, epsilon = 1e-10);
            assert_relative_eq!(result.width[i], high[i] - low[i], epsilon = 1e-10);
        }
    }

    #[test]
    fn test_donchian_full_period() {
        let high = vec![10.0, 12.0, 14.0, 13.0, 15.0, 16.0, 17.0];
        let low = vec![8.0, 10.0, 11.0, 10.0, 12.0, 13.0, 14.0];
        let result = donchian(&high, &low, 7).unwrap();

        // Only the last value should be valid
        assert!(result.upper[5].is_nan());
        assert!(!result.upper[6].is_nan());

        // Period 6: highest high in all 7 values = 17
        assert_relative_eq!(result.upper[6], 17.0, epsilon = 1e-10);
        // Period 6: lowest low in all 7 values = 8
        assert_relative_eq!(result.lower[6], 8.0, epsilon = 1e-10);
    }

    #[test]
    fn test_donchian_constant_prices() {
        let high = vec![10.0; 10];
        let low = vec![8.0; 10];
        let result = donchian(&high, &low, 5).unwrap();

        // All valid values should be the same
        assert_relative_eq!(result.upper[4], 10.0, epsilon = 1e-10);
        assert_relative_eq!(result.lower[4], 8.0, epsilon = 1e-10);
        assert_relative_eq!(result.middle[4], 9.0, epsilon = 1e-10);
        assert_relative_eq!(result.width[4], 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_donchian_sliding_window() {
        let high = vec![10.0, 15.0, 12.0, 18.0, 14.0, 20.0];
        let low = vec![8.0, 10.0, 7.0, 12.0, 9.0, 15.0];
        let result = donchian(&high, &low, 3).unwrap();

        // Period 2: window [10, 15, 12] -> max=15, window [8, 10, 7] -> min=7
        assert_relative_eq!(result.upper[2], 15.0, epsilon = 1e-10);
        assert_relative_eq!(result.lower[2], 7.0, epsilon = 1e-10);

        // Period 3: window [15, 12, 18] -> max=18, window [10, 7, 12] -> min=7
        assert_relative_eq!(result.upper[3], 18.0, epsilon = 1e-10);
        assert_relative_eq!(result.lower[3], 7.0, epsilon = 1e-10);

        // Period 4: window [12, 18, 14] -> max=18, window [7, 12, 9] -> min=7
        assert_relative_eq!(result.upper[4], 18.0, epsilon = 1e-10);
        assert_relative_eq!(result.lower[4], 7.0, epsilon = 1e-10);

        // Period 5: window [18, 14, 20] -> max=20, window [12, 9, 15] -> min=9
        assert_relative_eq!(result.upper[5], 20.0, epsilon = 1e-10);
        assert_relative_eq!(result.lower[5], 9.0, epsilon = 1e-10);
    }

    #[test]
    fn test_donchian_unequal_lengths() {
        let high = vec![10.0, 12.0, 14.0];
        let low = vec![8.0, 10.0];
        let result = donchian(&high, &low, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_donchian_insufficient_data() {
        let high = vec![10.0, 12.0];
        let low = vec![8.0, 10.0];
        let result = donchian(&high, &low, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_donchian_empty_input() {
        let high: Vec<f64> = vec![];
        let low: Vec<f64> = vec![];
        let result = donchian(&high, &low, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_donchian_zero_period() {
        let high = vec![10.0, 12.0, 14.0];
        let low = vec![8.0, 10.0, 11.0];
        let result = donchian(&high, &low, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_donchian_result_clone() {
        let high = vec![10.0, 12.0, 14.0, 13.0, 15.0];
        let low = vec![8.0, 10.0, 11.0, 10.0, 12.0];
        let result = donchian(&high, &low, 3).unwrap();

        let cloned = result.clone();
        assert_eq!(cloned.upper.len(), result.upper.len());
        assert_eq!(cloned.lower.len(), result.lower.len());
        assert_eq!(cloned.middle.len(), result.middle.len());
        assert_eq!(cloned.width.len(), result.width.len());
    }
}
