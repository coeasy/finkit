use crate::error::{Result, TaError};
use ndarray::Array1;

/// Calculate arithmetic mean
///
/// # Arguments
/// * `data` - Input data series
///
/// # Returns
/// The mean value
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::mean(&data).unwrap();
/// assert_eq!(result, 5.5);
/// ```
pub fn mean(data: &[f64]) -> Result<f64> {
    if data.is_empty() {
        return Err(TaError::EmptyInput);
    }
    Ok(data.iter().sum::<f64>() / data.len() as f64)
}

/// Calculate variance (sample variance with Bessel's correction)
///
/// # Arguments
/// * `data` - Input data series
///
/// # Returns
/// The variance value
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::variance(&data).unwrap();
/// assert!(result > 0.0);
/// ```
pub fn variance(data: &[f64]) -> Result<f64> {
    if data.len() < 2 {
        return Err(TaError::InsufficientData {
            length: data.len(),
            required: 2,
        });
    }
    let m = mean(data)?;
    let sum_sq: f64 = data.iter().map(|x| (x - m).powi(2)).sum();
    Ok(sum_sq / (data.len() - 1) as f64)
}

/// Calculate standard deviation (sample)
///
/// # Arguments
/// * `data` - Input data series
///
/// # Returns
/// The standard deviation value
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::std_dev(&data).unwrap();
/// assert!(result > 0.0);
/// ```
pub fn std_dev(data: &[f64]) -> Result<f64> {
    variance(data).map(|v| v.sqrt())
}

/// Calculate covariance between two series (sample)
///
/// # Arguments
/// * `x` - First data series
/// * `y` - Second data series
///
/// # Returns
/// The covariance value
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let y = vec![2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
/// let result = statistics::covariance(&x, &y).unwrap();
/// assert!(result > 0.0);
/// ```
pub fn covariance(x: &[f64], y: &[f64]) -> Result<f64> {
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

    // Single-pass: cov = (sum_xy - sum_x*sum_y/n) / (n-1)
    let mut sum_x: f64 = 0.0;
    let mut sum_y: f64 = 0.0;
    let mut sum_xy: f64 = 0.0;
    for (xi, yi) in x.iter().zip(y.iter()) {
        sum_x += xi;
        sum_y += yi;
        sum_xy += xi * yi;
    }

    Ok((sum_xy - sum_x * sum_y / n) / (n - 1.0))
}

/// Calculate Pearson correlation coefficient
///
/// # Arguments
/// * `x` - First data series
/// * `y` - Second data series
///
/// # Returns
/// Correlation coefficient in range [-1, 1]
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let x = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let y = vec![2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
/// let result = statistics::correlation(&x, &y).unwrap();
/// assert!((result - 1.0).abs() < 1e-10);
/// ```
pub fn correlation(x: &[f64], y: &[f64]) -> Result<f64> {
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

    let var_x = sum_x2 - sum_x * sum_x / n;
    let var_y = sum_y2 - sum_y * sum_y / n;

    if var_x.abs() < 1e-15 || var_y.abs() < 1e-15 {
        return Err(TaError::ComputationError {
            message: "Standard deviation is zero for one or both series".to_string(),
        });
    }

    let cov = sum_xy - sum_x * sum_y / n;
    Ok(cov / (var_x * var_y).sqrt())
}

/// Calculate rolling mean over a window
///
/// # Arguments
/// * `data` - Input data series
/// * `window` - Window size
///
/// # Returns
/// Array of rolling mean values
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::rolling_mean(&data, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn rolling_mean(data: &[f64], window: usize) -> Result<Array1<f64>> {
    if data.is_empty() {
        return Err(TaError::EmptyInput);
    }
    if window == 0 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }

    let len = data.len();
    let mut output = Array1::from_elem(len, f64::NAN);

    if window > len {
        return Ok(output);
    }

    // Compute initial window sum
    let mut sum: f64 = data[..window].iter().sum();
    output[window - 1] = sum / window as f64;

    // Incremental sliding window: O(n) instead of O(n*window)
    for i in window..len {
        sum += data[i] - data[i - window];
        output[i] = sum / window as f64;
    }

    Ok(output)
}

/// Calculate rolling variance over a window
///
/// # Arguments
/// * `data` - Input data series
/// * `window` - Window size
///
/// # Returns
/// Array of rolling variance values
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::rolling_variance(&data, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn rolling_variance(data: &[f64], window: usize) -> Result<Array1<f64>> {
    if data.is_empty() {
        return Err(TaError::EmptyInput);
    }
    if window < 2 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "at least 2".to_string(),
        });
    }

    let len = data.len();
    let mut output = Array1::from_elem(len, f64::NAN);

    if window > len {
        return Ok(output);
    }

    let inv_w = 1.0 / window as f64;
    let inv_w_minus_1 = 1.0 / (window as f64 - 1.0);

    let mut sum: f64 = data[..window].iter().sum();
    let mut sum_sq: f64 = data[..window].iter().map(|x| x * x).sum();
    let mean = sum * inv_w;
    output[window - 1] = (sum_sq - sum * mean) * inv_w_minus_1;

    for i in window..len {
        let old = data[i - window];
        let new = data[i];
        sum += new - old;
        sum_sq += new * new - old * old;
        let m = sum * inv_w;
        let var = (sum_sq - sum * m) * inv_w_minus_1;
        output[i] = var.max(0.0);
    }

    Ok(output)
}

/// Calculate rolling standard deviation over a window
///
/// # Arguments
/// * `data` - Input data series
/// * `window` - Window size
///
/// # Returns
/// Array of rolling standard deviation values
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::rolling_std_dev(&data, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn rolling_std_dev(data: &[f64], window: usize) -> Result<Array1<f64>> {
    rolling_variance(data, window).map(|v| v.map(|x| if x.is_nan() { f64::NAN } else { x.sqrt() }))
}

/// Calculate skewness of a data series
///
/// # Arguments
/// * `data` - Input data series
///
/// # Returns
/// Skewness value
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::skewness(&data).unwrap();
/// assert!(result.is_finite());
/// ```
pub fn skewness(data: &[f64]) -> Result<f64> {
    if data.len() < 3 {
        return Err(TaError::InsufficientData {
            length: data.len(),
            required: 3,
        });
    }

    let n = data.len() as f64;
    let m = mean(data)?;
    let s = std_dev(data)?;

    if s.abs() < 1e-15 {
        return Err(TaError::ComputationError {
            message: "Standard deviation is zero".to_string(),
        });
    }

    let sum_cubed: f64 = data.iter().map(|x| ((x - m) / s).powi(3)).sum();

    Ok((n / ((n - 1.0) * (n - 2.0))) * sum_cubed)
}

/// Calculate kurtosis of a data series (excess kurtosis)
///
/// # Arguments
/// * `data` - Input data series
///
/// # Returns
/// Kurtosis value
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::kurtosis(&data).unwrap();
/// assert!(result.is_finite());
/// ```
pub fn kurtosis(data: &[f64]) -> Result<f64> {
    if data.len() < 4 {
        return Err(TaError::InsufficientData {
            length: data.len(),
            required: 4,
        });
    }

    let n = data.len() as f64;
    let m = mean(data)?;
    let s = std_dev(data)?;

    if s.abs() < 1e-15 {
        return Err(TaError::ComputationError {
            message: "Standard deviation is zero".to_string(),
        });
    }

    let sum_fourth: f64 = data.iter().map(|x| ((x - m) / s).powi(4)).sum();

    let k = ((n * (n + 1.0)) / ((n - 1.0) * (n - 2.0) * (n - 3.0))) * sum_fourth;
    let correction = (3.0 * (n - 1.0).powi(2)) / ((n - 2.0) * (n - 3.0));

    Ok(k - correction)
}

/// Find maximum value in a rolling window
///
/// # Arguments
/// * `data` - Input data series
/// * `window` - Window size
///
/// # Returns
/// Array of rolling maximum values
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::rolling_max(&data, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn rolling_max(data: &[f64], window: usize) -> Result<Array1<f64>> {
    if data.is_empty() {
        return Err(TaError::EmptyInput);
    }
    if window == 0 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }

    let len = data.len();
    let mut output = Array1::from_elem(len, f64::NAN);
    let mut deque: std::collections::VecDeque<usize> =
        std::collections::VecDeque::with_capacity(window);

    for i in 0..len {
        while let Some(&back) = deque.back() {
            if data[back] <= data[i] {
                deque.pop_back();
            } else {
                break;
            }
        }
        deque.push_back(i);

        if let Some(&front) = deque.front() {
            if front + window <= i {
                deque.pop_front();
            }
        }

        if i >= window - 1 {
            output[i] = data[*deque.front().unwrap()];
        }
    }

    Ok(output)
}

/// Find minimum value in a rolling window
///
/// # Arguments
/// * `data` - Input data series
/// * `window` - Window size
///
/// # Returns
/// Array of rolling minimum values
///
/// # Examples
///
/// ```
/// use finkit::math::statistics;
///
/// let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
/// let result = statistics::rolling_min(&data, 3).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn rolling_min(data: &[f64], window: usize) -> Result<Array1<f64>> {
    if data.is_empty() {
        return Err(TaError::EmptyInput);
    }
    if window == 0 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "greater than 0".to_string(),
        });
    }

    let len = data.len();
    let mut output = Array1::from_elem(len, f64::NAN);
    let mut deque: std::collections::VecDeque<usize> =
        std::collections::VecDeque::with_capacity(window);

    for i in 0..len {
        while let Some(&back) = deque.back() {
            if data[back] >= data[i] {
                deque.pop_back();
            } else {
                break;
            }
        }
        deque.push_back(i);

        if let Some(&front) = deque.front() {
            if front + window <= i {
                deque.pop_front();
            }
        }

        if i >= window - 1 {
            output[i] = data[*deque.front().unwrap()];
        }
    }

    Ok(output)
}

/// Compute Kendall Tau rank correlation coefficient between two series.
///
/// Uses the O(n²) pairwise comparison algorithm. For each pair (i, j) with i < j,
/// count concordant (+1) vs discordant (-1) pairs.
/// tau = (concordant - discordant) / (n * (n-1) / 2)
///
/// # Arguments
/// * `x` - First data series
/// * `y` - Second data series (same length as x)
///
/// # Returns
/// Kendall Tau coefficient in [-1, 1]
pub fn kendall_tau(x: &[f64], y: &[f64]) -> Result<f64> {
    if x.len() != y.len() {
        return Err(TaError::InvalidParameter {
            name: "x, y".to_string(),
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

    let mut concordant: i64 = 0;
    let mut discordant: i64 = 0;

    for i in 0..n - 1 {
        for j in (i + 1)..n {
            let x_diff = x[j] - x[i];
            let y_diff = y[j] - y[i];
            let product = x_diff * y_diff;
            if product > 0.0 {
                concordant += 1;
            } else if product < 0.0 {
                discordant += 1;
            }
        }
    }

    let pairs = (n * (n - 1)) as f64 / 2.0;
    if pairs == 0.0 {
        return Ok(0.0);
    }
    Ok((concordant - discordant) as f64 / pairs)
}

/// Compute Spearman rank correlation coefficient between two series.
///
/// Assigns fractional ranks to each series, then computes Pearson correlation
/// on the ranks.
///
/// # Arguments
/// * `x` - First data series
/// * `y` - Second data series (same length as x)
///
/// # Returns
/// Spearman rho coefficient in [-1, 1]
pub fn spearman_rank(x: &[f64], y: &[f64]) -> Result<f64> {
    if x.len() != y.len() {
        return Err(TaError::InvalidParameter {
            name: "x, y".to_string(),
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

    let rank_x = fractional_ranks(x);
    let rank_y = fractional_ranks(y);

    // Pearson correlation on ranks
    let mean_rx: f64 = rank_x.iter().sum::<f64>() / n as f64;
    let mean_ry: f64 = rank_y.iter().sum::<f64>() / n as f64;

    let mut cov = 0.0;
    let mut var_x = 0.0;
    let mut var_y = 0.0;

    for i in 0..n {
        let dx = rank_x[i] - mean_rx;
        let dy = rank_y[i] - mean_ry;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let denom = (var_x * var_y).sqrt();
    if denom < 1e-15 {
        return Ok(0.0);
    }
    Ok(cov / denom)
}

/// Assign fractional ranks to data (handles ties by averaging ranks).
fn fractional_ranks(data: &[f64]) -> Vec<f64> {
    let n = data.len();
    let mut indices: Vec<usize> = (0..n).collect();
    indices.sort_by(|&a, &b| {
        data[a]
            .partial_cmp(&data[b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut ranks = vec![0.0; n];
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j < n - 1 && (data[indices[j + 1]] - data[indices[j]]).abs() < 1e-15 {
            j += 1;
        }
        let avg_rank = (i + j) as f64 / 2.0 + 1.0;
        for k in i..=j {
            ranks[indices[k]] = avg_rank;
        }
        i = j + 1;
    }
    ranks
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_mean() {
        assert_relative_eq!(
            mean(&[1.0, 2.0, 3.0, 4.0, 5.0]).unwrap(),
            3.0,
            epsilon = 1e-10
        );
        assert!(mean(&[]).is_err());
    }

    #[test]
    fn test_variance() {
        let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let var = variance(&data).unwrap();
        assert_relative_eq!(var, 4.571428571428571, epsilon = 1e-10);
        assert!(variance(&[1.0]).is_err());
    }

    #[test]
    fn test_std_dev() {
        let data = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let sd = std_dev(&data).unwrap();
        assert_relative_eq!(sd, 4.571428571428571_f64.sqrt(), epsilon = 1e-10);
    }

    #[test]
    fn test_covariance() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let cov = covariance(&x, &y).unwrap();
        assert_relative_eq!(cov, 5.0, epsilon = 1e-10);
    }

    #[test]
    fn test_correlation() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let corr = correlation(&x, &y).unwrap();
        assert_relative_eq!(corr, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_rolling_mean() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = rolling_mean(&data, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 2.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 3.0, epsilon = 1e-10);
        assert_relative_eq!(result[4], 4.0, epsilon = 1e-10);
    }

    #[test]
    fn test_skewness() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let skew = skewness(&data).unwrap();
        assert_relative_eq!(skew, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_kurtosis() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let kurt = kurtosis(&data).unwrap();
        // For uniform distribution, excess kurtosis should be negative
        assert!(kurt < 0.0);
    }

    #[test]
    fn test_rolling_max() {
        let data = vec![1.0, 3.0, 2.0, 5.0, 4.0];
        let result = rolling_max(&data, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 3.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 5.0, epsilon = 1e-10);
        assert_relative_eq!(result[4], 5.0, epsilon = 1e-10);
    }

    #[test]
    fn test_rolling_min() {
        let data = vec![5.0, 3.0, 4.0, 1.0, 2.0];
        let result = rolling_min(&data, 3).unwrap();
        assert!(result[0].is_nan());
        assert!(result[1].is_nan());
        assert_relative_eq!(result[2], 3.0, epsilon = 1e-10);
        assert_relative_eq!(result[3], 1.0, epsilon = 1e-10);
        assert_relative_eq!(result[4], 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_kendall_tau_perfect_concordance() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let tau = kendall_tau(&x, &y).unwrap();
        assert_relative_eq!(tau, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_kendall_tau_perfect_discordance() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 8.0, 6.0, 4.0, 2.0];
        let tau = kendall_tau(&x, &y).unwrap();
        assert_relative_eq!(tau, -1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_kendall_tau_invalid() {
        assert!(kendall_tau(&[1.0], &[2.0]).is_err());
        assert!(kendall_tau(&[1.0, 2.0], &[1.0]).is_err());
    }

    #[test]
    fn test_spearman_rank_perfect() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let rho = spearman_rank(&x, &y).unwrap();
        assert_relative_eq!(rho, 1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_spearman_rank_perfect_inverse() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![50.0, 40.0, 30.0, 20.0, 10.0];
        let rho = spearman_rank(&x, &y).unwrap();
        assert_relative_eq!(rho, -1.0, epsilon = 1e-10);
    }

    #[test]
    fn test_spearman_rank_invalid() {
        assert!(spearman_rank(&[1.0], &[2.0]).is_err());
        assert!(spearman_rank(&[1.0, 2.0], &[1.0]).is_err());
    }
}
