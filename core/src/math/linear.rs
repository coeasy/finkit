use crate::error::{Result, TaError};
use crate::utils::{init_output, validate_input};
use ndarray::Array1;

/// Linear Regression Result
#[derive(Debug, Clone)]
pub struct LinRegResult {
    /// Slope of the regression line
    pub slope: f64,
    /// Intercept of the regression line
    pub intercept: f64,
    /// R-squared value (coefficient of determination)
    pub r_squared: f64,
    /// Predicted values
    pub predicted: Array1<f64>,
}

/// Perform simple linear regression using least squares method
///
/// # Arguments
/// * `x` - Independent variable
/// * `y` - Dependent variable
///
/// # Returns
/// LinRegResult containing slope, intercept, r_squared, and predicted values
///
/// # Examples
///
/// ```
/// use finkit::math::linear;
///
/// let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let y = vec![2.0, 4.0, 5.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = linear::linear_regression(&x, &y).unwrap();
/// assert_eq!(result.predicted.len(), 10);
/// ```
pub fn linear_regression(x: &[f64], y: &[f64]) -> Result<LinRegResult> {
    if x.len() != y.len() {
        return Err(TaError::InvalidParameter {
            name: "x and y".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if x.len() < 2 {
        return Err(TaError::InsufficientData {
            length: x.len(),
            required: 2,
        });
    }

    let n = x.len() as f64;

    // Single-pass: accumulate all sums simultaneously
    let mut sum_x: f64 = 0.0;
    let mut sum_y: f64 = 0.0;
    let mut sum_xy: f64 = 0.0;
    let mut sum_x2: f64 = 0.0;
    let mut sum_y2: f64 = 0.0;
    for (xi, yi) in x.iter().zip(y.iter()) {
        sum_x += xi;
        sum_y += yi;
        sum_xy += xi * yi;
        sum_x2 += xi * xi;
        sum_y2 += yi * yi;
    }

    let denom = n * sum_x2 - sum_x * sum_x;
    if denom.abs() < 1e-15 {
        return Err(TaError::ComputationError {
            message: "Cannot compute regression: all x values are identical".to_string(),
        });
    }

    let slope = (n * sum_xy - sum_x * sum_y) / denom;
    let intercept = (sum_y - slope * sum_x) / n;

    // Calculate R-squared using sum_y2 from the single pass
    let ss_tot = sum_y2 - sum_y * sum_y / n;
    let predicted: Array1<f64> = x.iter().map(|xi| slope * xi + intercept).collect();
    let ss_res: f64 = y
        .iter()
        .zip(predicted.iter())
        .map(|(yi, pi)| (yi - pi).powi(2))
        .sum();

    let r_squared = if ss_tot.abs() < 1e-15 {
        1.0
    } else {
        1.0 - ss_res / ss_tot
    };

    Ok(LinRegResult {
        slope,
        intercept,
        r_squared,
        predicted,
    })
}

/// Calculate Linear Regression Slope over a rolling window
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
///
/// # Returns
/// Array of slope values (first `period - 1` values are NaN)
///
/// # Examples
///
/// ```
/// use finkit::math::linear;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = linear::linreg_slope(&data, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn linreg_slope(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period < 2 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let len = input.len();
    let mut output = init_output(len);
    let p = period as f64;
    // For x = 0..period-1: sum_x = p*(p-1)/2, sum_x2 = p*(p-1)*(2p-1)/6
    let sum_x = p * (p - 1.0) / 2.0;
    let sum_x2 = p * (p - 1.0) * (2.0 * p - 1.0) / 6.0;
    let denom = p * sum_x2 - sum_x * sum_x;

    let mut sum_y = 0.0;
    let mut sum_xy = 0.0;
    for (j, &val) in input[..period].iter().enumerate() {
        sum_y += val;
        sum_xy += j as f64 * val;
    }
    output[period - 1] = (p * sum_xy - sum_x * sum_y) / denom;

    for i in period..len {
        let old_val = input[i - period];
        let new_val = input[i];
        sum_xy += (period - 1) as f64 * new_val - (sum_y - old_val);
        sum_y += new_val - old_val;
        output[i] = (p * sum_xy - sum_x * sum_y) / denom;
    }

    Ok(output)
}

/// Calculate Linear Regression Intercept over a rolling window
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
///
/// # Returns
/// Array of intercept values
///
/// # Examples
///
/// ```
/// use finkit::math::linear;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = linear::linreg_intercept(&data, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn linreg_intercept(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period < 2 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let len = input.len();
    let mut output = init_output(len);
    let p = period as f64;
    let sum_x = p * (p - 1.0) / 2.0;
    let sum_x2 = p * (p - 1.0) * (2.0 * p - 1.0) / 6.0;
    let denom = p * sum_x2 - sum_x * sum_x;

    let mut sum_y = 0.0;
    let mut sum_xy = 0.0;
    for (j, &val) in input[..period].iter().enumerate() {
        sum_y += val;
        sum_xy += j as f64 * val;
    }
    let slope = (p * sum_xy - sum_x * sum_y) / denom;
    output[period - 1] = (sum_y - slope * sum_x) / p;

    for i in period..len {
        let old_val = input[i - period];
        let new_val = input[i];
        sum_xy += (period - 1) as f64 * new_val - (sum_y - old_val);
        sum_y += new_val - old_val;
        let slope = (p * sum_xy - sum_x * sum_y) / denom;
        output[i] = (sum_y - slope * sum_x) / p;
    }

    Ok(output)
}

/// Calculate Linear Regression predicted value over a rolling window
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
///
/// # Returns
/// Array of predicted values for the last point in each window
///
/// # Examples
///
/// ```
/// use finkit::math::linear;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = linear::linreg(&data, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn linreg(input: &[f64], period: usize) -> Result<Array1<f64>> {
    if period < 2 {
        return Err(TaError::InvalidParameter {
            name: "period".to_string(),
            constraint: "at least 2".to_string(),
        });
    }
    validate_input(input.len(), period)?;

    let len = input.len();
    let mut output = init_output(len);
    let p = period as f64;
    let sum_x = p * (p - 1.0) / 2.0;
    let sum_x2 = p * (p - 1.0) * (2.0 * p - 1.0) / 6.0;
    let denom = p * sum_x2 - sum_x * sum_x;
    let last_x = (period - 1) as f64;

    let mut sum_y = 0.0;
    let mut sum_xy = 0.0;
    for (j, &val) in input[..period].iter().enumerate() {
        sum_y += val;
        sum_xy += j as f64 * val;
    }
    let slope = (p * sum_xy - sum_x * sum_y) / denom;
    let intercept = (sum_y - slope * sum_x) / p;
    output[period - 1] = slope * last_x + intercept;

    for i in period..len {
        let old_val = input[i - period];
        let new_val = input[i];
        sum_xy += last_x * new_val - (sum_y - old_val);
        sum_y += new_val - old_val;
        let slope = (p * sum_xy - sum_x * sum_y) / denom;
        let intercept = (sum_y - slope * sum_x) / p;
        output[i] = slope * last_x + intercept;
    }

    Ok(output)
}

/// Calculate the angle of the linear regression line in degrees
///
/// # Arguments
/// * `input` - Input data series
/// * `period` - Lookback period
///
/// # Returns
/// Array of angle values in degrees
///
/// # Examples
///
/// ```
/// use finkit::math::linear;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = linear::linreg_angle(&data, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn linreg_angle(input: &[f64], period: usize) -> Result<Array1<f64>> {
    let slope = linreg_slope(input, period)?;
    let len = input.len();
    let mut output = init_output(len);

    for i in 0..len {
        if !slope[i].is_nan() {
            output[i] = slope[i].atan() * 180.0 / std::f64::consts::PI;
        }
    }

    Ok(output)
}

/// Quantile Regression Result
#[derive(Debug, Clone)]
pub struct QuantileRegResult {
    /// Slope of the quantile regression line
    pub slope: f64,
    /// Intercept of the quantile regression line
    pub intercept: f64,
}

/// Perform quantile regression using IRLS (Iteratively Reweighted Least Squares).
///
/// Fits a linear model y = slope*x + intercept at the given quantile tau.
///
/// # Arguments
/// * `x` - Independent variable
/// * `y` - Dependent variable
/// * `tau` - Quantile level in (0, 1), e.g. 0.5 for median regression
///
/// # Returns
/// QuantileRegResult with slope and intercept
pub fn quantile_regression(x: &[f64], y: &[f64], tau: f64) -> Result<QuantileRegResult> {
    if x.len() != y.len() {
        return Err(TaError::InvalidParameter {
            name: "x and y".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    let n = x.len();
    if n < 2 {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= 2".to_string(),
        });
    }
    if tau <= 0.0 || tau >= 1.0 {
        return Err(TaError::InvalidParameter {
            name: "tau".to_string(),
            constraint: "must be in (0, 1)".to_string(),
        });
    }

    // Initialize with OLS solution
    let mut slope;
    let mut intercept;
    {
        let mean_x: f64 = x.iter().sum::<f64>() / n as f64;
        let mean_y: f64 = y.iter().sum::<f64>() / n as f64;
        let mut sxy = 0.0;
        let mut sxx = 0.0;
        for i in 0..n {
            let dx = x[i] - mean_x;
            sxy += dx * (y[i] - mean_y);
            sxx += dx * dx;
        }
        slope = if sxx.abs() > 1e-15 { sxy / sxx } else { 0.0 };
        intercept = mean_y - slope * mean_x;
    }

    // IRLS iterations
    let max_iter = 50;
    let eps = 1e-6;

    for _ in 0..max_iter {
        let mut sw = 0.0;
        let mut swx = 0.0;
        let mut swy = 0.0;
        let mut swxx = 0.0;
        let mut swxy = 0.0;

        for i in 0..n {
            let residual = y[i] - (slope * x[i] + intercept);
            let w = if residual.abs() < eps {
                1.0 / eps
            } else if residual > 0.0 {
                tau / residual.abs()
            } else {
                (1.0 - tau) / residual.abs()
            };

            sw += w;
            swx += w * x[i];
            swy += w * y[i];
            swxx += w * x[i] * x[i];
            swxy += w * x[i] * y[i];
        }

        let denom = sw * swxx - swx * swx;
        if denom.abs() < 1e-15 {
            break;
        }

        let new_slope = (sw * swxy - swx * swy) / denom;
        let new_intercept = (swy - new_slope * swx) / sw;

        let d_slope = (new_slope - slope).abs();
        let d_intercept = (new_intercept - intercept).abs();
        slope = new_slope;
        intercept = new_intercept;

        if d_slope < eps && d_intercept < eps {
            break;
        }
    }

    Ok(QuantileRegResult { slope, intercept })
}

/// Theil-Sen Estimator Result
#[derive(Debug, Clone)]
pub struct TheilSenResult {
    /// Median slope
    pub slope: f64,
    /// Intercept (median of y_i - slope * x_i)
    pub intercept: f64,
}

/// Theil-Sen robust slope estimator.
///
/// Computes the median of slopes between all pairs of points.
/// Robust to outliers (up to 29.3% breakdown point).
///
/// # Arguments
/// * `x` - Independent variable
/// * `y` - Dependent variable (same length as x)
///
/// # Returns
/// TheilSenResult with median slope and intercept
pub fn theil_sen(x: &[f64], y: &[f64]) -> Result<TheilSenResult> {
    if x.len() != y.len() {
        return Err(TaError::InvalidParameter {
            name: "x and y".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    let n = x.len();
    if n < 2 {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= 2".to_string(),
        });
    }

    let mut slopes = Vec::with_capacity(n * (n - 1) / 2);
    for i in 0..n - 1 {
        for j in (i + 1)..n {
            let dx = x[j] - x[i];
            if dx.abs() > 1e-15 {
                slopes.push((y[j] - y[i]) / dx);
            }
        }
    }

    if slopes.is_empty() {
        return Ok(TheilSenResult {
            slope: 0.0,
            intercept: y.iter().sum::<f64>() / n as f64,
        });
    }

    slopes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let slope = median_sorted(&slopes);

    let mut intercepts: Vec<f64> = (0..n).map(|i| y[i] - slope * x[i]).collect();
    intercepts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let intercept = median_sorted(&intercepts);

    Ok(TheilSenResult { slope, intercept })
}

/// Compute median of a sorted slice.
fn median_sorted(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if n.is_multiple_of(2) {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    } else {
        sorted[n / 2]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_linear_regression() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 5.0, 4.0, 5.0];
        let result = linear_regression(&x, &y).unwrap();

        assert_relative_eq!(result.slope, 0.6, epsilon = 1e-10);
        assert_relative_eq!(result.intercept, 2.2, epsilon = 1e-10);
        assert!(result.r_squared >= 0.0 && result.r_squared <= 1.0);
    }

    #[test]
    fn test_linreg_slope() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = linreg_slope(&input, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result[4], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_linreg_intercept() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = linreg_intercept(&input, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        // For [1.0, 2.0, 3.0] with x=[0,1,2]: y = 1.0 + 1.0*x, intercept = 1.0
        assert_relative_eq!(result[2], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_linreg() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = linreg(&input, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 3.0, epsilon = 1e-10);
    }

    #[test]
    fn test_linreg_angle() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = linreg_angle(&input, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        let expected = 1.0_f64.atan() * 180.0 / std::f64::consts::PI;
        assert_relative_eq!(result[2], expected, epsilon = 1e-10);
    }

    #[test]
    fn test_invalid_input() {
        assert!(linear_regression(&[], &[]).is_err());
        assert!(linear_regression(&[1.0], &[2.0]).is_err());
        assert!(linreg_slope(&[1.0], 1).is_err());
    }

    #[test]
    fn test_quantile_regression_median() {
        // For perfectly linear data, median regression should be close to OLS
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let y: Vec<f64> = x.iter().map(|&v| 2.0 * v + 1.0).collect();
        let result = quantile_regression(&x, &y, 0.5).unwrap();
        assert_relative_eq!(result.slope, 2.0, epsilon = 0.1);
        assert_relative_eq!(result.intercept, 1.0, epsilon = 0.2);
    }

    #[test]
    fn test_quantile_regression_various_tau() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let y = vec![2.5, 5.1, 6.8, 8.2, 10.5, 13.1, 14.8, 16.2, 18.5, 21.1];
        let r25 = quantile_regression(&x, &y, 0.25).unwrap();
        let r50 = quantile_regression(&x, &y, 0.5).unwrap();
        let r75 = quantile_regression(&x, &y, 0.75).unwrap();
        // Higher tau -> higher intercept for same slope data
        assert!(r25.intercept < r75.intercept || r25.slope < r75.slope);
        assert!(r50.slope.is_finite());
    }

    #[test]
    fn test_quantile_regression_invalid() {
        let x = vec![1.0, 2.0];
        let y = vec![1.0, 2.0];
        assert!(quantile_regression(&x, &y, 0.0).is_err());
        assert!(quantile_regression(&x, &y, 1.0).is_err());
        assert!(quantile_regression(&[1.0], &[1.0], 0.5).is_err());
        assert!(quantile_regression(&[1.0, 2.0], &[1.0], 0.5).is_err());
    }

    #[test]
    fn test_theil_sen_perfect_linear() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let y: Vec<f64> = x.iter().map(|&v| 3.0 * v + 2.0).collect();
        let result = theil_sen(&x, &y).unwrap();
        assert_relative_eq!(result.slope, 3.0, epsilon = 1e-10);
        assert_relative_eq!(result.intercept, 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_theil_sen_with_outlier() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let mut y: Vec<f64> = x.iter().map(|&v| 2.0 * v + 1.0).collect();
        // Add extreme outlier
        y[5] = 1000.0;
        let result = theil_sen(&x, &y).unwrap();
        // Theil-Sen should be robust and still give slope ~2
        assert!((result.slope - 2.0).abs() < 0.5);
    }

    #[test]
    fn test_theil_sen_invalid() {
        assert!(theil_sen(&[1.0], &[2.0]).is_err());
        assert!(theil_sen(&[1.0, 2.0], &[1.0]).is_err());
    }
}
