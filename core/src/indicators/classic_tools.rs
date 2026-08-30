//! Classic technical analysis tools including Andrews Pitchfork, Gann Angles, and other drawing tools.

use crate::error::{Result, TaError};
use crate::utils::{init_output, validate_input, validate_param};
use ndarray::Array1;

/// Andrews Pitchfork Result
///
/// Contains all components of the Andrews Pitchfork indicator.
#[derive(Debug, Clone)]
pub struct AndrewsPitchforkResult {
    /// Upper median line (from pivot A through pivot B)
    pub upper_line: Array1<f64>,
    /// Middle median line (from pivot A through pivot C)
    pub middle_line: Array1<f64>,
    /// Lower median line (from pivot A through pivot C reflected)
    pub lower_line: Array1<f64>,
    /// Upper warning line (parallel to upper line)
    pub upper_warning: Array1<f64>,
    /// Lower warning line (parallel to lower line)
    pub lower_warning: Array1<f64>,
}

/// Andrews Pitchfork
///
/// A technical analysis tool that uses three pivot points to create a channel
/// that helps identify potential support and resistance levels.
///
/// # Pivot Points
/// - Pivot A: Starting point (usually a significant high or low)
/// - Pivot B: First reaction point after A
/// - Pivot C: Second reaction point after B
///
/// # Formula
/// - Middle Line: Line from A through midpoint of B and C
/// - Upper Line: Line from A through B
/// - Lower Line: Line from A through C
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `pivot_a_idx` - Index of pivot A (starting point)
/// * `pivot_b_idx` - Index of pivot B (first reaction)
/// * `pivot_c_idx` - Index of pivot C (second reaction)
/// * `use_warning_lines` - Whether to calculate warning lines
///
/// # Returns
/// AndrewsPitchforkResult containing all median and warning lines
///
/// # Example
/// ```rust
/// use finkit::indicators::andrews_pitchfork;
///
/// let high = vec![10.0, 12.0, 15.0, 14.0, 16.0, 18.0, 17.0, 19.0, 21.0];
/// let low = vec![8.0, 10.0, 12.0, 11.0, 13.0, 15.0, 14.0, 16.0, 18.0];
/// let result = andrews_pitchfork(&high, &low, 0, 2, 4, true).unwrap();
/// ```
pub fn andrews_pitchfork(
    high: &[f64],
    low: &[f64],
    pivot_a_idx: usize,
    pivot_b_idx: usize,
    pivot_c_idx: usize,
    use_warning_lines: bool,
) -> Result<AndrewsPitchforkResult> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }

    // Validate pivot indices
    if pivot_a_idx >= pivot_b_idx || pivot_b_idx >= pivot_c_idx {
        return Err(TaError::InvalidParameter {
            name: "pivot indices".to_string(),
            constraint: "must be in ascending order (A < B < C)".to_string(),
        });
    }

    if pivot_c_idx >= high.len() {
        return Err(TaError::InvalidParameter {
            name: "pivot_c_idx".to_string(),
            constraint: "must be less than data length".to_string(),
        });
    }

    let len = high.len();

    // Determine pivot values (use high for peaks, low for troughs)
    let pivot_a = (high[pivot_a_idx] + low[pivot_a_idx]) / 2.0;
    let pivot_b = (high[pivot_b_idx] + low[pivot_b_idx]) / 2.0;
    let pivot_c = (high[pivot_c_idx] + low[pivot_c_idx]) / 2.0;

    // Calculate midpoint of B and C
    let bc_midpoint = (pivot_b + pivot_c) / 2.0;

    // Calculate slopes
    let middle_slope = if pivot_c_idx > pivot_a_idx {
        (bc_midpoint - pivot_a) / (pivot_c_idx - pivot_a_idx) as f64
    } else {
        0.0
    };

    let upper_slope = if pivot_b_idx > pivot_a_idx {
        (pivot_b - pivot_a) / (pivot_b_idx - pivot_a_idx) as f64
    } else {
        0.0
    };

    let lower_slope = if pivot_c_idx > pivot_a_idx {
        (pivot_c - pivot_a) / (pivot_c_idx - pivot_a_idx) as f64
    } else {
        0.0
    };

    // Calculate lines from pivot A onwards
    let mut middle_line = init_output(len);
    let mut upper_line = init_output(len);
    let mut lower_line = init_output(len);
    let mut upper_warning = init_output(len);
    let mut lower_warning = init_output(len);

    for i in pivot_a_idx..len {
        let distance = (i - pivot_a_idx) as f64;
        middle_line[i] = pivot_a + middle_slope * distance;
        upper_line[i] = pivot_a + upper_slope * distance;
        lower_line[i] = pivot_a + lower_slope * distance;

        if use_warning_lines {
            // Warning lines are parallel to median lines at the distance of B and C from middle
            let upper_offset = upper_line[pivot_c_idx] - middle_line[pivot_c_idx];
            let lower_offset = lower_line[pivot_c_idx] - middle_line[pivot_c_idx];
            upper_warning[i] = middle_line[i] + upper_offset;
            lower_warning[i] = middle_line[i] + lower_offset;
        }
    }

    Ok(AndrewsPitchforkResult {
        upper_line,
        middle_line,
        lower_line,
        upper_warning,
        lower_warning,
    })
}

/// Gann Angles Result
///
/// Contains Gann angle lines from a pivot point.
#[derive(Debug, Clone)]
pub struct GannAnglesResult {
    /// 1x1 angle (45 degrees) - most important
    pub angle_1x1: Array1<f64>,
    /// 1x2 angle (26.25 degrees)
    pub angle_1x2: Array1<f64>,
    /// 2x1 angle (63.75 degrees)
    pub angle_2x1: Array1<f64>,
    /// 1x4 angle (15 degrees)
    pub angle_1x4: Array1<f64>,
    /// 4x1 angle (75 degrees)
    pub angle_4x1: Array1<f64>,
    /// 1x8 angle (7.5 degrees)
    pub angle_1x8: Array1<f64>,
    /// 8x1 angle (82.5 degrees)
    pub angle_8x1: Array1<f64>,
}

/// Gann Angles (Gann Fan)
///
/// A set of trend lines drawn from a significant pivot point at specific angles.
/// The most important angle is 1x1 (45 degrees), which represents one unit of
/// price for one unit of time.
///
/// # Angles
/// - 1x8: 82.5 degrees (steepest)
/// - 1x4: 75 degrees
/// - 1x2: 63.75 degrees
/// - 1x1: 45 degrees (most important)
/// - 2x1: 26.25 degrees
/// - 4x1: 15 degrees
/// - 8x1: 7.5 degrees (flattest)
///
/// # Arguments
/// * `price` - Price data (typically close prices)
/// * `pivot_idx` - Index of the pivot point
/// * `pivot_price` - Price at the pivot point
/// * `price_unit` - Price unit for angle calculation
/// * `time_unit` - Time unit for angle calculation
///
/// # Returns
/// GannAnglesResult containing all Gann angle lines
///
/// # Example
/// ```rust
/// use finkit::indicators::gann_angles;
///
/// let close = vec![100.0, 102.0, 105.0, 108.0, 110.0, 112.0, 115.0];
/// let result = gann_angles(&close, 0, 100.0, 1.0, 1.0).unwrap();
/// ```
pub fn gann_angles(
    price: &[f64],
    pivot_idx: usize,
    pivot_price: f64,
    price_unit: f64,
    time_unit: f64,
) -> Result<GannAnglesResult> {
    validate_param("pivot_idx", "less than data length", || pivot_idx < price.len())?;
    validate_param("price_unit", "greater than 0", || price_unit > 0.0)?;
    validate_param("time_unit", "greater than 0", || time_unit > 0.0)?;

    let len = price.len();

    // Calculate Gann slopes (price/time ratios)
    // 1x1 = 1 unit price / 1 unit time
    let base_slope = price_unit / time_unit;

    let angle_1x1_slope = base_slope;
    let angle_1x2_slope = base_slope / 2.0;
    let angle_2x1_slope = base_slope * 2.0;
    let angle_1x4_slope = base_slope / 4.0;
    let angle_4x1_slope = base_slope * 4.0;
    let angle_1x8_slope = base_slope / 8.0;
    let angle_8x1_slope = base_slope * 8.0;

    // Calculate lines from pivot point
    let mut angle_1x1 = init_output(len);
    let mut angle_1x2 = init_output(len);
    let mut angle_2x1 = init_output(len);
    let mut angle_1x4 = init_output(len);
    let mut angle_4x1 = init_output(len);
    let mut angle_1x8 = init_output(len);
    let mut angle_8x1 = init_output(len);

    for i in pivot_idx..len {
        let time_distance = (i - pivot_idx) as f64 * time_unit;

        angle_1x1[i] = pivot_price + angle_1x1_slope * time_distance;
        angle_1x2[i] = pivot_price + angle_1x2_slope * time_distance;
        angle_2x1[i] = pivot_price + angle_2x1_slope * time_distance;
        angle_1x4[i] = pivot_price + angle_1x4_slope * time_distance;
        angle_4x1[i] = pivot_price + angle_4x1_slope * time_distance;
        angle_1x8[i] = pivot_price + angle_1x8_slope * time_distance;
        angle_8x1[i] = pivot_price + angle_8x1_slope * time_distance;
    }

    Ok(GannAnglesResult {
        angle_1x1,
        angle_1x2,
        angle_2x1,
        angle_1x4,
        angle_4x1,
        angle_1x8,
        angle_8x1,
    })
}

/// Speed Resistance Lines Result
///
/// Contains speed resistance lines for trend analysis.
#[derive(Debug, Clone)]
pub struct SpeedResistanceResult {
    /// 1/3 speed line (33.3%)
    pub line_1_3: Array1<f64>,
    /// 2/3 speed line (66.6%)
    pub line_2_3: Array1<f64>,
    /// Full speed line (100%)
    pub line_full: Array1<f64>,
}

/// Speed Resistance Lines
///
/// Similar to Fibonacci retracement but uses 1/3 and 2/3 levels instead.
/// Drawn from a significant high or low to identify potential support/resistance.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `start_idx` - Starting pivot index
/// * `end_idx` - Ending pivot index
/// * `is_uptrend` - True for uptrend (start is low), false for downtrend (start is high)
///
/// # Returns
/// SpeedResistanceResult containing 1/3, 2/3, and full speed lines
///
/// # Example
/// ```rust
/// use finkit::indicators::speed_resistance_lines;
///
/// let high = vec![10.0, 12.0, 15.0, 14.0, 16.0, 18.0, 17.0];
/// let low = vec![8.0, 10.0, 12.0, 11.0, 13.0, 15.0, 14.0];
/// let result = speed_resistance_lines(&high, &low, 0, 5, true).unwrap();
/// ```
pub fn speed_resistance_lines(
    high: &[f64],
    low: &[f64],
    start_idx: usize,
    end_idx: usize,
    is_uptrend: bool,
) -> Result<SpeedResistanceResult> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }

    if start_idx >= end_idx || end_idx >= high.len() {
        return Err(TaError::InvalidParameter {
            name: "start_idx, end_idx".to_string(),
            constraint: "start_idx < end_idx < length".to_string(),
        });
    }

    let len = high.len();

    // Determine start and end prices
    let start_price = if is_uptrend { low[start_idx] } else { high[start_idx] };
    let end_price = if is_uptrend { high[end_idx] } else { low[end_idx] };

    let price_range = end_price - start_price;
    let time_range = (end_idx - start_idx) as f64;

    // Calculate slopes
    let full_slope = if time_range > 0.0 { price_range / time_range } else { 0.0 };
    let slope_2_3 = full_slope * 2.0 / 3.0;
    let slope_1_3 = full_slope / 3.0;

    let mut line_full = init_output(len);
    let mut line_2_3 = init_output(len);
    let mut line_1_3 = init_output(len);

    for i in start_idx..len {
        let time_distance = (i - start_idx) as f64;
        line_full[i] = start_price + full_slope * time_distance;
        line_2_3[i] = start_price + slope_2_3 * time_distance;
        line_1_3[i] = start_price + slope_1_3 * time_distance;
    }

    Ok(SpeedResistanceResult {
        line_1_3,
        line_2_3,
        line_full,
    })
}

/// Median Price
///
/// Simple average of high and low prices for each period.
///
/// # Formula
/// Median Price = (High + Low) / 2
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
///
/// # Returns
/// Array of median price values
pub fn median_price(high: &[f64], low: &[f64]) -> Result<Array1<f64>> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;

    let len = high.len();
    let mut output = Array1::zeros(len);

    for i in 0..len {
        output[i] = (high[i] + low[i]) / 2.0;
    }

    Ok(output)
}

/// Weighted Close
///
/// A price calculation that gives more weight to the closing price.
///
/// # Formula
/// Weighted Close = (High + Low + 2 × Close) / 4
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
///
/// # Returns
/// Array of weighted close values
pub fn weighted_close(high: &[f64], low: &[f64], close: &[f64]) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;

    let len = high.len();
    let mut output = Array1::zeros(len);

    for i in 0..len {
        output[i] = (high[i] + low[i] + 2.0 * close[i]) / 4.0;
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_andrews_pitchfork_basic() {
        let high = vec![10.0, 12.0, 15.0, 14.0, 16.0, 18.0, 17.0, 19.0, 21.0];
        let low = vec![8.0, 10.0, 12.0, 11.0, 13.0, 15.0, 14.0, 16.0, 18.0];

        let result = andrews_pitchfork(&high, &low, 0, 2, 4, false).unwrap();

        assert_eq!(result.upper_line.len(), 9);
        assert_eq!(result.middle_line.len(), 9);
        assert_eq!(result.lower_line.len(), 9);

        // Check that lines start from pivot A
        let pivot_a = (high[0] + low[0]) / 2.0;
        assert_relative_eq!(result.middle_line[0], pivot_a, epsilon = 1e-10);
        assert_relative_eq!(result.upper_line[0], pivot_a, epsilon = 1e-10);
        assert_relative_eq!(result.lower_line[0], pivot_a, epsilon = 1e-10);
    }

    #[test]
    fn test_andrews_pitchfork_with_warning_lines() {
        let high = vec![10.0, 12.0, 15.0, 14.0, 16.0, 18.0, 17.0, 19.0, 21.0];
        let low = vec![8.0, 10.0, 12.0, 11.0, 13.0, 15.0, 14.0, 16.0, 18.0];

        let result = andrews_pitchfork(&high, &low, 0, 2, 4, true).unwrap();

        assert!(!result.upper_warning[4].is_nan());
        assert!(!result.lower_warning[4].is_nan());
    }

    #[test]
    fn test_andrews_pitchfork_invalid_indices() {
        let high = vec![10.0, 12.0, 15.0];
        let low = vec![8.0, 10.0, 12.0];

        // Indices not in ascending order
        assert!(andrews_pitchfork(&high, &low, 2, 1, 0, false).is_err());
        // Index out of bounds
        assert!(andrews_pitchfork(&high, &low, 0, 1, 5, false).is_err());
    }

    #[test]
    fn test_gann_angles_basic() {
        let close = vec![100.0, 101.0, 102.0, 103.0, 104.0, 105.0];
        let result = gann_angles(&close, 0, 100.0, 1.0, 1.0).unwrap();

        assert_eq!(result.angle_1x1.len(), 6);
        assert_relative_eq!(result.angle_1x1[0], 100.0, epsilon = 1e-10);

        // 1x1 angle: 100 + 1 * time_distance
        assert_relative_eq!(result.angle_1x1[5], 105.0, epsilon = 1e-10);
    }

    #[test]
    fn test_gann_angles_different_slopes() {
        let close = vec![100.0, 101.0, 102.0, 103.0, 104.0];
        let result = gann_angles(&close, 0, 100.0, 1.0, 1.0).unwrap();

        // 1x2 should be half of 1x1
        assert_relative_eq!(result.angle_1x2[4], 102.0, epsilon = 1e-10);
        // 2x1 should be double of 1x1
        assert_relative_eq!(result.angle_2x1[4], 108.0, epsilon = 1e-10);
    }

    #[test]
    fn test_gann_angles_invalid_params() {
        let close = vec![100.0, 101.0];
        assert!(gann_angles(&close, 5, 100.0, 1.0, 1.0).is_err());
        assert!(gann_angles(&close, 0, 100.0, 0.0, 1.0).is_err());
    }

    #[test]
    fn test_speed_resistance_lines_uptrend() {
        let high = vec![10.0, 12.0, 15.0, 14.0, 16.0, 18.0, 17.0];
        let low = vec![8.0, 10.0, 12.0, 11.0, 13.0, 15.0, 14.0];

        let result = speed_resistance_lines(&high, &low, 0, 5, true).unwrap();

        assert_eq!(result.line_full.len(), 7);
        assert_relative_eq!(result.line_full[0], low[0], epsilon = 1e-10);
        assert_relative_eq!(result.line_full[5], high[5], epsilon = 1e-10);
    }

    #[test]
    fn test_speed_resistance_lines_downtrend() {
        let high = vec![18.0, 16.0, 15.0, 14.0, 12.0, 10.0, 11.0];
        let low = vec![15.0, 13.0, 12.0, 11.0, 10.0, 8.0, 9.0];

        let result = speed_resistance_lines(&high, &low, 0, 5, false).unwrap();

        assert_relative_eq!(result.line_full[0], high[0], epsilon = 1e-10);
        assert_relative_eq!(result.line_full[5], low[5], epsilon = 1e-10);
    }

    #[test]
    fn test_median_price_basic() {
        let high = vec![10.0, 12.0, 14.0, 16.0];
        let low = vec![8.0, 10.0, 12.0, 14.0];

        let result = median_price(&high, &low).unwrap();

        assert_eq!(result.len(), 4);
        assert_relative_eq!(result[0], 9.0, epsilon = 1e-10);
        assert_relative_eq!(result[1], 11.0, epsilon = 1e-10);
        assert_relative_eq!(result[2], 13.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 15.0, epsilon = 1e-10);
    }

    #[test]
    fn test_weighted_close_basic() {
        let high = vec![10.0, 12.0, 14.0];
        let low = vec![8.0, 10.0, 12.0];
        let close = vec![9.0, 11.0, 13.0];

        let result = weighted_close(&high, &low, &close).unwrap();

        assert_eq!(result.len(), 3);
        // (10 + 8 + 2*9) / 4 = 36 / 4 = 9.0
        assert_relative_eq!(result[0], 9.0, epsilon = 1e-10);
        // (12 + 10 + 2*11) / 4 = 44 / 4 = 11.0
        assert_relative_eq!(result[1], 11.0, epsilon = 1e-10);
        // (14 + 12 + 2*13) / 4 = 52 / 4 = 13.0
        assert_relative_eq!(result[2], 13.0, epsilon = 1e-10);
    }

    #[test]
    fn test_weighted_close_gives_more_weight_to_close() {
        let high = vec![10.0];
        let low = vec![8.0];
        let close = vec![9.5];

        let result = weighted_close(&high, &low, &close).unwrap();

        // Weighted close should be closer to close than median price
        let median = (high[0] + low[0]) / 2.0;
        assert!(result[0] > median); // 9.375 > 9.0
    }
}