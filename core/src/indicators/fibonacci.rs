use crate::error::{Result, TaError};

/// Fibonacci Level entry
#[derive(Debug, Clone, Copy)]
pub struct FibLevel {
    pub ratio: f64,
    pub price: f64,
}

/// Fibonacci Retracement Result
#[derive(Debug, Clone)]
pub struct FibonacciResult {
    pub levels: Vec<FibLevel>,
    pub trend: i32,
    pub high_price: f64,
    pub low_price: f64,
    pub high_index: usize,
    pub low_index: usize,
}

const FIB_RETRACEMENTS: [f64; 7] = [0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];
const FIB_EXTENSIONS: [f64; 2] = [1.272, 1.618];

/// Calculate Fibonacci Retracement levels
///
/// Automatically detects the highest and lowest prices in the specified range,
/// determines trend direction, and calculates all standard Fibonacci retracement
/// and extension levels.
///
/// # Arguments
/// * `high` - High prices series
/// * `low` - Low prices series
/// * `start_index` - Start index of the range (inclusive)
/// * `end_index` - End index of the range (inclusive)
///
/// # Returns
/// FibonacciResult containing:
/// - `levels`: Vec of FibLevel entries with ratio and price
/// - `trend`: 1 for uptrend, -1 for downtrend
/// - `high_price`: Highest price in range
/// - `low_price`: Lowest price in range
/// - `high_index`: Index of highest price
/// - `low_index`: Index of lowest price
///
/// # Trend Detection
/// - **Uptrend** (trend = 1): Low point occurs before high point
///   - Retracement calculated from low to high
///   - Extensions above the high
///
/// - **Downtrend** (trend = -1): High point occurs before low point
///   - Retracement calculated from high to low
///   - Extensions below the low
///
/// # Examples
///
/// ```
/// use finkit::indicators::fibonacci_retracement;
///
/// let high = vec![10.0, 11.0, 12.0, 13.0, 14.0, 13.5, 13.0];
/// let low = vec![9.0, 9.5, 10.0, 11.0, 12.0, 11.5, 11.0];
/// let result = fibonacci_retracement(&high, &low, 0, 6).unwrap();
///
/// assert_eq!(result.trend, 1);
/// assert!(result.levels.iter().any(|l| l.ratio == 0.618));
/// assert!(result.levels.iter().any(|l| l.ratio == 1.618));
/// ```
pub fn fibonacci_retracement(
    high: &[f64],
    low: &[f64],
    start_index: usize,
    end_index: usize,
) -> Result<FibonacciResult> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }

    if start_index > end_index {
        return Err(TaError::InvalidParameter {
            name: "start_index".to_string(),
            constraint: "must be <= end_index".to_string(),
        });
    }

    if end_index >= high.len() {
        return Err(TaError::InsufficientData {
            length: high.len(),
            required: end_index + 1,
        });
    }

    if start_index == end_index {
        return Err(TaError::InvalidParameter {
            name: "range".to_string(),
            constraint: "start_index and end_index must define a range of at least 2 bars"
                .to_string(),
        });
    }

    let mut high_price = f64::NEG_INFINITY;
    let mut low_price = f64::INFINITY;
    let mut high_index = start_index;
    let mut low_index = start_index;

    for i in start_index..=end_index {
        if high[i] > high_price {
            high_price = high[i];
            high_index = i;
        }
        if low[i] < low_price {
            low_price = low[i];
            low_index = i;
        }
    }

    let trend = if low_index <= high_index { 1 } else { -1 };

    let range = high_price - low_price;

    let mut levels: Vec<FibLevel> =
        Vec::with_capacity(FIB_RETRACEMENTS.len() + FIB_EXTENSIONS.len());

    for &ratio in &FIB_RETRACEMENTS {
        let price = if trend == 1 {
            high_price - (range * ratio)
        } else {
            low_price + (range * ratio)
        };
        levels.push(FibLevel { ratio, price });
    }

    for &ratio in &FIB_EXTENSIONS {
        let price = if trend == 1 {
            high_price + (range * (ratio - 1.0))
        } else {
            low_price - (range * (ratio - 1.0))
        };
        levels.push(FibLevel { ratio, price });
    }

    Ok(FibonacciResult {
        levels,
        trend,
        high_price,
        low_price,
        high_index,
        low_index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn get_price(result: &FibonacciResult, ratio: f64) -> f64 {
        result
            .levels
            .iter()
            .find(|l| l.ratio == ratio)
            .unwrap()
            .price
    }

    #[test]
    fn test_uptrend_basic() {
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0, 13.5, 13.0];
        let low = vec![9.0, 9.5, 10.0, 11.0, 12.0, 11.5, 11.0];
        let result = fibonacci_retracement(&high, &low, 0, 6).unwrap();

        assert_eq!(result.trend, 1);
        assert_eq!(result.high_price, 14.0);
        assert_eq!(result.low_price, 9.0);
        assert_eq!(result.high_index, 4);
        assert_eq!(result.low_index, 0);
    }

    #[test]
    fn test_uptrend_fib_levels() {
        let high = vec![10.0, 12.0, 15.0, 18.0, 20.0];
        let low = vec![8.0, 10.0, 13.0, 16.0, 18.0];
        let result = fibonacci_retracement(&high, &low, 0, 4).unwrap();

        assert_eq!(result.trend, 1);
        assert_eq!(result.high_price, 20.0);
        assert_eq!(result.low_price, 8.0);

        let range = 12.0;
        assert_relative_eq!(get_price(&result, 0.0), 20.0, epsilon = 1e-10);
        assert_relative_eq!(
            get_price(&result, 0.236),
            20.0 - range * 0.236,
            epsilon = 1e-10
        );
        assert_relative_eq!(
            get_price(&result, 0.382),
            20.0 - range * 0.382,
            epsilon = 1e-10
        );
        assert_relative_eq!(get_price(&result, 0.5), 14.0, epsilon = 1e-10);
        assert_relative_eq!(
            get_price(&result, 0.618),
            20.0 - range * 0.618,
            epsilon = 1e-10
        );
        assert_relative_eq!(
            get_price(&result, 0.786),
            20.0 - range * 0.786,
            epsilon = 1e-10
        );
        assert_relative_eq!(get_price(&result, 1.0), 8.0, epsilon = 1e-10);
    }

    #[test]
    fn test_uptrend_extensions() {
        let high = vec![100.0, 110.0, 120.0];
        let low = vec![90.0, 100.0, 110.0];
        let result = fibonacci_retracement(&high, &low, 0, 2).unwrap();

        assert_eq!(result.trend, 1);

        let range = 120.0 - 90.0;
        let expected_1272 = 120.0 + (range * 0.272);
        let expected_1618 = 120.0 + (range * 0.618);

        assert_relative_eq!(get_price(&result, 1.272), expected_1272, epsilon = 1e-10);
        assert_relative_eq!(get_price(&result, 1.618), expected_1618, epsilon = 1e-10);
    }

    #[test]
    fn test_downtrend_basic() {
        let high = vec![20.0, 19.0, 18.0, 17.0, 16.0, 16.5, 17.0];
        let low = vec![18.0, 17.0, 16.0, 15.0, 14.0, 14.5, 15.0];
        let result = fibonacci_retracement(&high, &low, 0, 6).unwrap();

        assert_eq!(result.trend, -1);
        assert_eq!(result.high_price, 20.0);
        assert_eq!(result.low_price, 14.0);
        assert_eq!(result.high_index, 0);
        assert_eq!(result.low_index, 4);
    }

    #[test]
    fn test_downtrend_fib_levels() {
        let high = vec![20.0, 18.0, 15.0, 12.0, 10.0];
        let low = vec![18.0, 16.0, 13.0, 10.0, 8.0];
        let result = fibonacci_retracement(&high, &low, 0, 4).unwrap();

        assert_eq!(result.trend, -1);
        assert_eq!(result.high_price, 20.0);
        assert_eq!(result.low_price, 8.0);

        let range = 12.0;
        assert_relative_eq!(get_price(&result, 0.0), 8.0, epsilon = 1e-10);
        assert_relative_eq!(
            get_price(&result, 0.236),
            8.0 + range * 0.236,
            epsilon = 1e-10
        );
        assert_relative_eq!(
            get_price(&result, 0.382),
            8.0 + range * 0.382,
            epsilon = 1e-10
        );
        assert_relative_eq!(get_price(&result, 0.5), 14.0, epsilon = 1e-10);
        assert_relative_eq!(
            get_price(&result, 0.618),
            8.0 + range * 0.618,
            epsilon = 1e-10
        );
        assert_relative_eq!(
            get_price(&result, 0.786),
            8.0 + range * 0.786,
            epsilon = 1e-10
        );
        assert_relative_eq!(get_price(&result, 1.0), 20.0, epsilon = 1e-10);
    }

    #[test]
    fn test_downtrend_extensions() {
        let high = vec![120.0, 110.0, 100.0];
        let low = vec![110.0, 100.0, 90.0];
        let result = fibonacci_retracement(&high, &low, 0, 2).unwrap();

        assert_eq!(result.trend, -1);

        let range = 120.0 - 90.0;
        let expected_1272 = 90.0 - (range * 0.272);
        let expected_1618 = 90.0 - (range * 0.618);

        assert_relative_eq!(get_price(&result, 1.272), expected_1272, epsilon = 1e-10);
        assert_relative_eq!(get_price(&result, 1.618), expected_1618, epsilon = 1e-10);
    }

    #[test]
    fn test_all_levels_present() {
        let high = vec![10.0, 12.0, 15.0, 18.0, 20.0];
        let low = vec![8.0, 10.0, 13.0, 16.0, 18.0];
        let result = fibonacci_retracement(&high, &low, 0, 4).unwrap();

        let expected_ratios: Vec<f64> =
            vec![0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0, 1.272, 1.618];
        for &ratio in &expected_ratios {
            assert!(
                result.levels.iter().any(|l| l.ratio == ratio),
                "Missing ratio: {}",
                ratio
            );
        }
        assert_eq!(result.levels.len(), 9);
    }

    #[test]
    fn test_high_low_same_index() {
        let high = vec![10.0, 15.0, 20.0];
        let low = vec![5.0, 10.0, 15.0];
        let result = fibonacci_retracement(&high, &low, 1, 2).unwrap();

        assert_eq!(result.high_price, 20.0);
        assert_eq!(result.low_price, 10.0);
        assert_eq!(result.high_index, 2);
        assert_eq!(result.low_index, 1);
        assert_eq!(result.trend, 1);
    }

    #[test]
    fn test_invalid_parameters() {
        let high = vec![10.0, 11.0, 12.0];
        let low = vec![9.0, 10.0, 11.0];

        assert!(fibonacci_retracement(&high, &low, 2, 1).is_err());
        assert!(fibonacci_retracement(&high, &low, 0, 10).is_err());
        assert!(fibonacci_retracement(&high, &low, 1, 1).is_err());
    }

    #[test]
    fn test_mismatched_lengths() {
        let high = vec![10.0, 11.0, 12.0];
        let low = vec![9.0, 10.0];

        assert!(fibonacci_retracement(&high, &low, 0, 1).is_err());
    }

    #[test]
    fn test_uptrend_with_zero_range() {
        let high = vec![10.0, 10.0, 10.0];
        let low = vec![10.0, 10.0, 10.0];
        let result = fibonacci_retracement(&high, &low, 0, 2).unwrap();

        assert_eq!(result.trend, 1);
        assert_eq!(result.high_price, 10.0);
        assert_eq!(result.low_price, 10.0);

        for level in &result.levels {
            assert_relative_eq!(level.price, 10.0, epsilon = 1e-10);
        }
    }

    #[test]
    fn test_downtrend_with_zero_range() {
        let high = vec![10.0, 10.0, 10.0];
        let low = vec![10.0, 10.0, 10.0];
        let result = fibonacci_retracement(&high, &low, 0, 2).unwrap();

        assert_eq!(result.trend, 1);
        assert_relative_eq!(result.high_price, 10.0, epsilon = 1e-10);
        assert_relative_eq!(result.low_price, 10.0, epsilon = 1e-10);
    }
}
