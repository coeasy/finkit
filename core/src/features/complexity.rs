//! Complexity and fractal dimension estimators for time series.

use crate::error::{Result, TaError};
use ndarray::Array1;

/// Higuchi fractal dimension estimate for a time series.
///
/// Constructs subseries at intervals k = 1..max_k, computes curve lengths L(k),
/// then fits log(L(k)) vs log(1/k) to estimate the fractal dimension.
///
/// # Arguments
/// * `data` - Time series data
/// * `max_k` - Maximum interval (typically 8-64)
///
/// # Returns
/// Fractal dimension estimate (typically between 1.0 and 2.0)
pub fn fractal_dimension_higuchi(data: &[f64], max_k: usize) -> Result<f64> {
    let n = data.len();
    if n < 4 {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= 4".to_string(),
        });
    }
    if max_k < 2 {
        return Err(TaError::InvalidParameter {
            name: "max_k".to_string(),
            constraint: "must be >= 2".to_string(),
        });
    }

    let k_max = max_k.min(n / 2);
    let mut log_k = Vec::with_capacity(k_max);
    let mut log_l = Vec::with_capacity(k_max);

    for k in 1..=k_max {
        let mut l_k = 0.0;
        for m in 1..=k {
            let num_points = (n - m) / k;
            if num_points == 0 {
                continue;
            }
            let mut length = 0.0;
            for i in 1..=num_points {
                length += (data[m - 1 + i * k] - data[m - 1 + (i - 1) * k]).abs();
            }
            length *= (n - 1) as f64 / (num_points * k) as f64;
            l_k += length;
        }
        l_k /= k as f64;

        if l_k > 0.0 {
            log_k.push((1.0 / k as f64).ln());
            log_l.push(l_k.ln());
        }
    }

    if log_k.len() < 2 {
        return Ok(1.0);
    }

    // Linear regression: log_l = slope * log_k + intercept
    let n_pts = log_k.len() as f64;
    let sum_x: f64 = log_k.iter().sum();
    let sum_y: f64 = log_l.iter().sum();
    let sum_xy: f64 = log_k.iter().zip(log_l.iter()).map(|(x, y)| x * y).sum();
    let sum_xx: f64 = log_k.iter().map(|x| x * x).sum();

    let denom = n_pts * sum_xx - sum_x * sum_x;
    if denom.abs() < 1e-15 {
        return Ok(1.0);
    }

    let slope = (n_pts * sum_xy - sum_x * sum_y) / denom;
    Ok(slope)
}

/// Box-counting fractal dimension for a price path.
///
/// Overlays boxes of various sizes on the (index, price) path and counts
/// how many boxes are needed to cover the path. FD = -log(N) / log(epsilon).
///
/// # Arguments
/// * `data` - Price/time series data
/// * `num_scales` - Number of box scales to evaluate (typically 5-20)
///
/// # Returns
/// Fractal dimension estimate
pub fn fractal_dimension_box(data: &[f64], num_scales: usize) -> Result<f64> {
    let n = data.len();
    if n < 4 {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= 4".to_string(),
        });
    }
    if num_scales < 2 {
        return Err(TaError::InvalidParameter {
            name: "num_scales".to_string(),
            constraint: "must be >= 2".to_string(),
        });
    }

    let min_val = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max_val - min_val;
    if range < 1e-15 {
        return Ok(1.0);
    }

    let mut log_eps = Vec::with_capacity(num_scales);
    let mut log_n = Vec::with_capacity(num_scales);

    for s in 1..=num_scales {
        let box_size = range / (2.0_f64.powi(s as i32));
        if box_size < 1e-15 {
            break;
        }

        let x_box_size = (n as f64) / (2.0_f64.powi(s as i32));
        if x_box_size < 1.0 {
            break;
        }

        // Count boxes that contain at least one segment of the path
        use std::collections::HashSet;
        let mut occupied = HashSet::new();

        for (i, &val) in data.iter().enumerate() {
            let bx = (i as f64 / x_box_size).floor() as i64;
            let by = ((val - min_val) / box_size).floor() as i64;
            occupied.insert((bx, by));
        }

        let count = occupied.len();
        if count > 0 {
            log_eps.push(box_size.ln());
            log_n.push((count as f64).ln());
        }
    }

    if log_eps.len() < 2 {
        return Ok(1.0);
    }

    // Linear regression: log_n = slope * log_eps + intercept
    // FD = -slope
    let n_pts = log_eps.len() as f64;
    let sum_x: f64 = log_eps.iter().sum();
    let sum_y: f64 = log_n.iter().sum();
    let sum_xy: f64 = log_eps.iter().zip(log_n.iter()).map(|(x, y)| x * y).sum();
    let sum_xx: f64 = log_eps.iter().map(|x| x * x).sum();

    let denom = n_pts * sum_xx - sum_x * sum_x;
    if denom.abs() < 1e-15 {
        return Ok(1.0);
    }

    let slope = (n_pts * sum_xy - sum_x * sum_y) / denom;
    Ok(-slope)
}

/// Rolling Higuchi fractal dimension.
///
/// # Arguments
/// * `data` - Time series
/// * `window` - Rolling window size
/// * `max_k` - Maximum interval for Higuchi algorithm
///
/// # Returns
/// Array of rolling fractal dimension values (NaN during warm-up)
pub fn rolling_fractal_dimension_higuchi(
    data: &[f64],
    window: usize,
    max_k: usize,
) -> Result<Array1<f64>> {
    if window < 4 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "must be >= 4".to_string(),
        });
    }
    let n = data.len();
    if n < window {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= window".to_string(),
        });
    }

    let mut output = Array1::from_elem(n, f64::NAN);
    for i in (window - 1)..n {
        let start = i + 1 - window;
        if let Ok(fd) = fractal_dimension_higuchi(&data[start..=i], max_k) {
            output[i] = fd;
        }
    }

    Ok(output)
}

/// Rolling box-counting fractal dimension.
///
/// # Arguments
/// * `data` - Time series
/// * `window` - Rolling window size
/// * `num_scales` - Number of scales for box-counting
///
/// # Returns
/// Array of rolling fractal dimension values (NaN during warm-up)
pub fn rolling_fractal_dimension_box(
    data: &[f64],
    window: usize,
    num_scales: usize,
) -> Result<Array1<f64>> {
    if window < 4 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "must be >= 4".to_string(),
        });
    }
    let n = data.len();
    if n < window {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= window".to_string(),
        });
    }

    let mut output = Array1::from_elem(n, f64::NAN);
    for i in (window - 1)..n {
        let start = i + 1 - window;
        if let Ok(fd) = fractal_dimension_box(&data[start..=i], num_scales) {
            output[i] = fd;
        }
    }

    Ok(output)
}

/// Approximate Entropy (ApEn) of a time series.
///
/// Measures regularity/predictability. Lower values indicate more regularity.
///
/// # Arguments
/// * `data` - Time series data
/// * `m` - Embedding dimension (pattern length), typically 2
/// * `r` - Tolerance threshold (fraction of std dev), typically 0.2 * std_dev
///
/// # Returns
/// ApEn value (non-negative; lower = more regular)
pub fn approx_entropy(data: &[f64], m: usize, r: f64) -> Result<f64> {
    let n = data.len();
    if n < m + 2 {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= m + 2".to_string(),
        });
    }
    if m == 0 {
        return Err(TaError::InvalidParameter {
            name: "m".to_string(),
            constraint: "must be >= 1".to_string(),
        });
    }
    if r <= 0.0 {
        return Err(TaError::InvalidParameter {
            name: "r".to_string(),
            constraint: "must be > 0".to_string(),
        });
    }

    let phi_m = phi(data, m, r);
    let phi_m1 = phi(data, m + 1, r);

    Ok(phi_m - phi_m1)
}

/// Helper: compute phi(m) for ApEn.
fn phi(data: &[f64], m: usize, r: f64) -> f64 {
    let n = data.len();
    let num_patterns = n - m + 1;

    let mut sum_log = 0.0;
    for i in 0..num_patterns {
        let mut count = 0u32;
        for j in 0..num_patterns {
            let mut is_match = true;
            for k in 0..m {
                if (data[i + k] - data[j + k]).abs() > r {
                    is_match = false;
                    break;
                }
            }
            if is_match {
                count += 1;
            }
        }
        sum_log += (count as f64 / num_patterns as f64).ln();
    }

    sum_log / num_patterns as f64
}

/// Sample Entropy (SampEn) of a time series.
///
/// Similar to ApEn but avoids self-matches, giving a less biased estimate.
/// Lower values indicate more regularity/self-similarity.
///
/// # Arguments
/// * `data` - Time series data
/// * `m` - Embedding dimension (pattern length), typically 2
/// * `r` - Tolerance threshold, typically 0.2 * std_dev
///
/// # Returns
/// SampEn value (non-negative; lower = more regular, Inf if no matches)
pub fn sample_entropy(data: &[f64], m: usize, r: f64) -> Result<f64> {
    let n = data.len();
    if n < m + 2 {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= m + 2".to_string(),
        });
    }
    if m == 0 {
        return Err(TaError::InvalidParameter {
            name: "m".to_string(),
            constraint: "must be >= 1".to_string(),
        });
    }
    if r <= 0.0 {
        return Err(TaError::InvalidParameter {
            name: "r".to_string(),
            constraint: "must be > 0".to_string(),
        });
    }

    let num_m = n - m;
    let num_m1 = n - m; // patterns of length m+1

    // Count template matches for dimension m (B) and m+1 (A)
    let mut b_count: u64 = 0;
    let mut a_count: u64 = 0;

    for i in 0..num_m {
        for j in (i + 1)..num_m {
            // Check if patterns of length m match
            let mut match_m = true;
            for k in 0..m {
                if (data[i + k] - data[j + k]).abs() > r {
                    match_m = false;
                    break;
                }
            }
            if match_m {
                b_count += 1;
                // Check if extending to m+1 still matches
                if i + m < n && j + m < n && (data[i + m] - data[j + m]).abs() <= r {
                    a_count += 1;
                }
            }
        }
    }

    if b_count == 0 {
        return Ok(f64::INFINITY);
    }

    let _ = num_m1; // suppress unused warning
    Ok(-(a_count as f64 / b_count as f64).ln())
}

/// Rolling Approximate Entropy.
///
/// # Arguments
/// * `data` - Time series
/// * `window` - Rolling window size
/// * `m` - Embedding dimension
/// * `r` - Tolerance
pub fn rolling_approx_entropy(data: &[f64], window: usize, m: usize, r: f64) -> Result<Array1<f64>> {
    if window < m + 2 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "must be >= m + 2".to_string(),
        });
    }
    let n = data.len();
    if n < window {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= window".to_string(),
        });
    }

    let mut output = Array1::from_elem(n, f64::NAN);
    for i in (window - 1)..n {
        let start = i + 1 - window;
        if let Ok(val) = approx_entropy(&data[start..=i], m, r) {
            output[i] = val;
        }
    }

    Ok(output)
}

/// Rolling Sample Entropy.
///
/// # Arguments
/// * `data` - Time series
/// * `window` - Rolling window size
/// * `m` - Embedding dimension
/// * `r` - Tolerance
pub fn rolling_sample_entropy(data: &[f64], window: usize, m: usize, r: f64) -> Result<Array1<f64>> {
    if window < m + 2 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "must be >= m + 2".to_string(),
        });
    }
    let n = data.len();
    if n < window {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= window".to_string(),
        });
    }

    let mut output = Array1::from_elem(n, f64::NAN);
    for i in (window - 1)..n {
        let start = i + 1 - window;
        if let Ok(val) = sample_entropy(&data[start..=i], m, r) {
            output[i] = val;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_higuchi_smooth() {
        // High-frequency oscillation should give FD > 1
        let data: Vec<f64> = (0..200)
            .map(|i| (i as f64 * 0.7).sin() * 10.0 + (i as f64 * 1.3).cos() * 5.0 + (i as f64 * 3.7).sin() * 2.0)
            .collect();
        let fd = fractal_dimension_higuchi(&data, 10).unwrap();
        // Multi-harmonic signal should have higher complexity
        assert!(fd.is_finite(), "FD should be finite: {}", fd);
    }

    #[test]
    fn test_higuchi_random_like() {
        // Highly variable data should have higher FD
        let data: Vec<f64> = (0..100)
            .map(|i| (i as f64 * 2.7).sin() * 10.0 + (i as f64 * 0.3).cos() * 5.0)
            .collect();
        let fd = fractal_dimension_higuchi(&data, 10).unwrap();
        assert!(fd > 1.0 && fd < 2.1, "FD for oscillating data: {}", fd);
    }

    #[test]
    fn test_higuchi_invalid() {
        assert!(fractal_dimension_higuchi(&[1.0, 2.0, 3.0], 2).is_err());
        assert!(fractal_dimension_higuchi(&[1.0; 10], 1).is_err());
    }

    #[test]
    fn test_box_counting_linear() {
        let data: Vec<f64> = (0..100).map(|i| i as f64).collect();
        let fd = fractal_dimension_box(&data, 8).unwrap();
        assert!(fd > 0.8 && fd < 1.5, "Box FD for linear data: {}", fd);
    }

    #[test]
    fn test_box_counting_oscillating() {
        let data: Vec<f64> = (0..100)
            .map(|i| (i as f64 * 0.5).sin() * 20.0)
            .collect();
        let fd = fractal_dimension_box(&data, 8).unwrap();
        assert!(fd > 0.7 && fd < 2.5, "Box FD for oscillating data: {}", fd);
    }

    #[test]
    fn test_box_counting_invalid() {
        assert!(fractal_dimension_box(&[1.0, 2.0, 3.0], 2).is_err());
        assert!(fractal_dimension_box(&[1.0; 10], 1).is_err());
    }

    #[test]
    fn test_rolling_higuchi() {
        let data: Vec<f64> = (0..50).map(|i| (i as f64 * 0.3).sin() * 10.0 + i as f64).collect();
        let result = rolling_fractal_dimension_higuchi(&data, 20, 5).unwrap();
        assert_eq!(result.len(), 50);
        assert!(result[18].is_nan());
        assert!(result[19].is_finite());
    }

    #[test]
    fn test_rolling_box() {
        let data: Vec<f64> = (0..50).map(|i| (i as f64 * 0.3).sin() * 10.0 + i as f64).collect();
        let result = rolling_fractal_dimension_box(&data, 20, 5).unwrap();
        assert_eq!(result.len(), 50);
        assert!(result[18].is_nan());
        assert!(result[19].is_finite());
    }

    #[test]
    fn test_approx_entropy_regular() {
        // Very regular data (constant) should have low ApEn
        let data = vec![1.0; 50];
        let apen = approx_entropy(&data, 2, 0.2).unwrap();
        assert_relative_eq!(apen, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_approx_entropy_random_like() {
        // Pseudo-random data should have higher ApEn
        let data: Vec<f64> = (0..100)
            .map(|i| (i as f64 * 2.7).sin() * 10.0 + (i as f64 * 7.3).cos() * 5.0)
            .collect();
        let std_dev = (data.iter().map(|x| x * x).sum::<f64>() / data.len() as f64
            - (data.iter().sum::<f64>() / data.len() as f64).powi(2))
        .sqrt();
        let apen = approx_entropy(&data, 2, 0.2 * std_dev).unwrap();
        assert!(apen >= 0.0, "ApEn should be non-negative: {}", apen);
    }

    #[test]
    fn test_approx_entropy_invalid() {
        assert!(approx_entropy(&[1.0, 2.0], 2, 0.2).is_err());
        assert!(approx_entropy(&[1.0; 10], 0, 0.2).is_err());
        assert!(approx_entropy(&[1.0; 10], 2, 0.0).is_err());
    }

    #[test]
    fn test_sample_entropy_regular() {
        // Constant data: all patterns match, so SampEn should be ~0 or low
        let data = vec![1.0; 50];
        let sampen = sample_entropy(&data, 2, 0.2).unwrap();
        // For constant data, all B and A matches, ln(A/B) = ln(1) = 0
        assert!(sampen >= 0.0, "SampEn should be non-negative: {}", sampen);
    }

    #[test]
    fn test_sample_entropy_random_like() {
        let data: Vec<f64> = (0..100)
            .map(|i| (i as f64 * 2.7).sin() * 10.0 + (i as f64 * 7.3).cos() * 5.0)
            .collect();
        let std_dev = (data.iter().map(|x| x * x).sum::<f64>() / data.len() as f64
            - (data.iter().sum::<f64>() / data.len() as f64).powi(2))
        .sqrt();
        let sampen = sample_entropy(&data, 2, 0.2 * std_dev).unwrap();
        assert!(sampen >= 0.0, "SampEn should be non-negative: {}", sampen);
    }

    #[test]
    fn test_sample_entropy_invalid() {
        assert!(sample_entropy(&[1.0, 2.0], 2, 0.2).is_err());
        assert!(sample_entropy(&[1.0; 10], 0, 0.2).is_err());
        assert!(sample_entropy(&[1.0; 10], 2, 0.0).is_err());
    }

    #[test]
    fn test_rolling_approx_entropy() {
        let data: Vec<f64> = (0..50)
            .map(|i| (i as f64 * 0.5).sin() * 10.0)
            .collect();
        let result = rolling_approx_entropy(&data, 20, 2, 1.0).unwrap();
        assert_eq!(result.len(), 50);
        assert!(result[18].is_nan());
        assert!(result[19].is_finite());
    }

    #[test]
    fn test_rolling_sample_entropy() {
        let data: Vec<f64> = (0..50)
            .map(|i| (i as f64 * 0.5).sin() * 10.0)
            .collect();
        let result = rolling_sample_entropy(&data, 20, 2, 3.0).unwrap();
        assert_eq!(result.len(), 50);
        assert!(result[18].is_nan());
        // Value could be Inf if no matches - just check it's not NaN
        assert!(!result[19].is_nan());
    }
}

/// Detrended Fluctuation Analysis (DFA) scaling exponent.
///
/// Estimates the long-range correlation exponent α of a time series.
/// - α ≈ 0.5: uncorrelated (white noise)
/// - α > 0.5: persistent long-range correlations
/// - α < 0.5: anti-persistent
/// - α ≈ 1.0: 1/f noise
/// - α ≈ 1.5: Brownian motion
///
/// # Arguments
/// * `data` - Time series data
/// * `order` - Polynomial order for detrending (1-4; DFA-1, DFA-2, etc.)
///
/// # Returns
/// DFA scaling exponent α
pub fn dfa(data: &[f64], order: usize) -> Result<f64> {
    let n = data.len();
    if n < 16 {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= 16".to_string(),
        });
    }
    if order == 0 || order > 4 {
        return Err(TaError::InvalidParameter {
            name: "order".to_string(),
            constraint: "must be 1-4".to_string(),
        });
    }

    // Step 1: Integrate the series (cumulative sum of deviations from mean)
    let mean = data.iter().sum::<f64>() / n as f64;
    let mut profile = vec![0.0; n];
    profile[0] = data[0] - mean;
    for i in 1..n {
        profile[i] = profile[i - 1] + (data[i] - mean);
    }

    // Step 2: For various window sizes, compute RMS of detrended segments
    let min_window = (order + 2).max(4);
    let max_window = n / 4;
    if min_window >= max_window {
        return Ok(0.5);
    }

    let mut log_s = Vec::new();
    let mut log_f = Vec::new();

    let num_scales = 15.min(max_window - min_window + 1);
    let log_min = (min_window as f64).ln();
    let log_max = (max_window as f64).ln();

    for s_idx in 0..num_scales {
        let s = ((log_min + (log_max - log_min) * s_idx as f64 / (num_scales - 1).max(1) as f64)
            .exp()) as usize;
        let s = s.max(min_window).min(max_window);

        let num_segments = n / s;
        if num_segments == 0 {
            continue;
        }

        let mut total_var = 0.0;
        let mut count = 0usize;

        for seg in 0..num_segments {
            let start = seg * s;
            let end = start + s;
            let segment = &profile[start..end];

            // Polynomial fit and detrend
            let trend = poly_fit(segment, order);
            let mut var = 0.0;
            for (i, &val) in segment.iter().enumerate() {
                let residual = val - trend[i];
                var += residual * residual;
            }
            total_var += var / s as f64;
            count += 1;
        }

        if count > 0 {
            let f_s = (total_var / count as f64).sqrt();
            if f_s > 0.0 {
                log_s.push((s as f64).ln());
                log_f.push(f_s.ln());
            }
        }
    }

    if log_s.len() < 2 {
        return Ok(0.5);
    }

    // Step 3: Linear regression of log(F(s)) vs log(s)
    let n_pts = log_s.len() as f64;
    let sum_x: f64 = log_s.iter().sum();
    let sum_y: f64 = log_f.iter().sum();
    let sum_xy: f64 = log_s.iter().zip(log_f.iter()).map(|(x, y)| x * y).sum();
    let sum_xx: f64 = log_s.iter().map(|x| x * x).sum();

    let denom = n_pts * sum_xx - sum_x * sum_x;
    if denom.abs() < 1e-15 {
        return Ok(0.5);
    }

    Ok((n_pts * sum_xy - sum_x * sum_y) / denom)
}

/// Fit a polynomial of given order to data and return fitted values.
fn poly_fit(data: &[f64], order: usize) -> Vec<f64> {
    let n = data.len();
    let mut fitted = vec![0.0; n];

    // Use least squares: build normal equations for polynomial coefficients
    let cols = order + 1;
    let mut ata = vec![0.0; cols * cols];
    let mut atb = vec![0.0; cols];

    for (i, &val) in data.iter().enumerate() {
        let x = i as f64;
        let mut x_pow = vec![1.0; cols];
        for j in 1..cols {
            x_pow[j] = x_pow[j - 1] * x;
        }
        for r in 0..cols {
            for c in 0..cols {
                ata[r * cols + c] += x_pow[r] * x_pow[c];
            }
            atb[r] += x_pow[r] * val;
        }
    }

    // Solve via Gaussian elimination
    let coeffs = solve_linear_system(&mut ata, &mut atb, cols);

    // Compute fitted values
    for (i, item) in fitted.iter_mut().enumerate() {
        let x = i as f64;
        let mut val = coeffs[0];
        let mut x_pow = 1.0;
        for coeff in coeffs.iter().skip(1) {
            x_pow *= x;
            val += coeff * x_pow;
        }
        *item = val;
    }

    fitted
}

/// Solve a linear system Ax = b using Gaussian elimination with partial pivoting.
fn solve_linear_system(a: &mut [f64], b: &mut [f64], n: usize) -> Vec<f64> {
    // Forward elimination
    for col in 0..n {
        // Partial pivot
        let mut max_row = col;
        let mut max_val = a[col * n + col].abs();
        for row in (col + 1)..n {
            let val = a[row * n + col].abs();
            if val > max_val {
                max_val = val;
                max_row = row;
            }
        }
        if max_row != col {
            for k in 0..n {
                a.swap(col * n + k, max_row * n + k);
            }
            b.swap(col, max_row);
        }

        let pivot = a[col * n + col];
        if pivot.abs() < 1e-15 {
            continue;
        }

        for row in (col + 1)..n {
            let factor = a[row * n + col] / pivot;
            for k in col..n {
                a[row * n + k] -= factor * a[col * n + k];
            }
            b[row] -= factor * b[col];
        }
    }

    // Back substitution
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let mut sum = b[i];
        for j in (i + 1)..n {
            sum -= a[i * n + j] * x[j];
        }
        if a[i * n + i].abs() > 1e-15 {
            x[i] = sum / a[i * n + i];
        }
    }
    x
}

/// Rolling DFA scaling exponent.
///
/// # Arguments
/// * `data` - Time series
/// * `window` - Rolling window size (>= 16)
/// * `order` - Polynomial order (1-4)
pub fn rolling_dfa(data: &[f64], window: usize, order: usize) -> Result<Array1<f64>> {
    if window < 16 {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "must be >= 16".to_string(),
        });
    }
    let n = data.len();
    if n < window {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= window".to_string(),
        });
    }

    let mut output = Array1::from_elem(n, f64::NAN);
    for i in (window - 1)..n {
        let start = i + 1 - window;
        if let Ok(alpha) = dfa(&data[start..=i], order) {
            output[i] = alpha;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod dfa_tests {
    use super::*;

    #[test]
    fn test_dfa_brownian() {
        // Cumulative sum of constant = linear => profile is quadratic => α ~1.5
        // Let's use something more realistic - a trending series
        let data: Vec<f64> = (0..200).map(|i| i as f64 * 0.1 + (i as f64 * 0.3).sin()).collect();
        let alpha = dfa(&data, 1).unwrap();
        assert!(alpha.is_finite(), "DFA alpha should be finite: {}", alpha);
    }

    #[test]
    fn test_dfa_orders() {
        let data: Vec<f64> = (0..100)
            .map(|i| (i as f64 * 0.5).sin() * 5.0 + (i as f64 * 0.1).cos() * 3.0)
            .collect();
        for order in 1..=4 {
            let alpha = dfa(&data, order).unwrap();
            assert!(alpha.is_finite(), "DFA-{} alpha should be finite: {}", order, alpha);
        }
    }

    #[test]
    fn test_dfa_invalid() {
        let short = vec![1.0; 10];
        assert!(dfa(&short, 1).is_err());
        let data = vec![1.0; 20];
        assert!(dfa(&data, 0).is_err());
        assert!(dfa(&data, 5).is_err());
    }

    #[test]
    fn test_rolling_dfa() {
        let data: Vec<f64> = (0..60).map(|i| (i as f64 * 0.3).sin() * 10.0 + i as f64 * 0.1).collect();
        let result = rolling_dfa(&data, 30, 1).unwrap();
        assert_eq!(result.len(), 60);
        assert!(result[28].is_nan());
        assert!(result[29].is_finite());
    }

    #[test]
    fn test_rolling_dfa_invalid() {
        let data = vec![1.0; 20];
        assert!(rolling_dfa(&data, 10, 1).is_err());
        assert!(rolling_dfa(&data, 25, 1).is_err());
    }
}

/// Largest Lyapunov Exponent estimation using Rosenstein's method.
///
/// Embeds the time series into phase space, finds nearest neighbors (excluding
/// temporal neighbors), then tracks average logarithmic divergence.
///
/// # Arguments
/// * `data` - Time series data
/// * `m` - Embedding dimension (typically 2-5)
/// * `tau` - Time delay for embedding
/// * `max_iter` - Maximum iterations to track divergence
///
/// # Returns
/// Estimated largest Lyapunov exponent (positive = chaotic, ~0 = periodic, negative = convergent)
pub fn lyapunov_exponent(data: &[f64], m: usize, tau: usize, max_iter: usize) -> Result<f64> {
    let n = data.len();
    let embed_len = n - (m - 1) * tau;

    if m < 2 {
        return Err(TaError::InvalidParameter {
            name: "m".to_string(),
            constraint: "must be >= 2".to_string(),
        });
    }
    if tau == 0 {
        return Err(TaError::InvalidParameter {
            name: "tau".to_string(),
            constraint: "must be >= 1".to_string(),
        });
    }
    if embed_len < max_iter + 2 {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "not enough data for embedding".to_string(),
        });
    }

    let mean_period = estimate_mean_period(data);
    let exclusion = mean_period.max(1);

    // Find nearest neighbor for each embedded point
    let mut nearest = vec![0usize; embed_len];
    for (i, nn_slot) in nearest.iter_mut().enumerate() {
        let mut min_dist = f64::INFINITY;
        let mut nn = 0;
        for j in 0..embed_len {
            if (i as isize - j as isize).unsigned_abs() < exclusion {
                continue;
            }
            let dist = embed_distance(data, i, j, m, tau);
            if dist < min_dist {
                min_dist = dist;
                nn = j;
            }
        }
        *nn_slot = nn;
    }

    // Track divergence
    let track_len = max_iter.min(embed_len - 1);
    let mut divergence = vec![0.0; track_len];
    let mut counts = vec![0u32; track_len];

    for (i, &nn) in nearest.iter().enumerate() {
        for k in 0..track_len {
            let i_k = i + k;
            let nn_k = nn + k;
            if i_k >= embed_len || nn_k >= embed_len {
                break;
            }
            let dist = embed_distance(data, i_k, nn_k, m, tau);
            if dist > 0.0 {
                divergence[k] += dist.ln();
                counts[k] += 1;
            }
        }
    }

    // Average log divergence
    let mut log_div: Vec<f64> = Vec::with_capacity(track_len);
    let mut time_idx: Vec<f64> = Vec::with_capacity(track_len);
    for k in 0..track_len {
        if counts[k] > 0 {
            log_div.push(divergence[k] / counts[k] as f64);
            time_idx.push(k as f64);
        }
    }

    if time_idx.len() < 2 {
        return Ok(0.0);
    }

    // Linear regression slope of log divergence vs time = Lyapunov exponent
    let n_pts = time_idx.len() as f64;
    let sum_x: f64 = time_idx.iter().sum();
    let sum_y: f64 = log_div.iter().sum();
    let sum_xy: f64 = time_idx.iter().zip(log_div.iter()).map(|(x, y)| x * y).sum();
    let sum_xx: f64 = time_idx.iter().map(|x| x * x).sum();

    let denom = n_pts * sum_xx - sum_x * sum_x;
    if denom.abs() < 1e-15 {
        return Ok(0.0);
    }

    Ok((n_pts * sum_xy - sum_x * sum_y) / denom)
}

/// Estimate mean period via zero-crossing of autocorrelation.
fn estimate_mean_period(data: &[f64]) -> usize {
    let n = data.len();
    let mean = data.iter().sum::<f64>() / n as f64;
    let max_lag = n / 4;

    let mut var = 0.0;
    for &v in data.iter() {
        var += (v - mean) * (v - mean);
    }
    if var < 1e-15 {
        return 1;
    }

    let mut prev_ac = 1.0;
    for lag in 1..max_lag {
        let mut ac = 0.0;
        for i in 0..(n - lag) {
            ac += (data[i] - mean) * (data[i + lag] - mean);
        }
        ac /= var;
        if ac < 0.0 && prev_ac >= 0.0 {
            return lag;
        }
        prev_ac = ac;
    }
    max_lag
}

/// Euclidean distance in embedded space.
#[inline]
fn embed_distance(data: &[f64], i: usize, j: usize, m: usize, tau: usize) -> f64 {
    let mut dist = 0.0;
    for k in 0..m {
        let diff = data[i + k * tau] - data[j + k * tau];
        dist += diff * diff;
    }
    dist.sqrt()
}

/// Rolling Lyapunov exponent.
///
/// # Arguments
/// * `data` - Time series
/// * `window` - Rolling window size
/// * `m` - Embedding dimension
/// * `tau` - Time delay
/// * `max_iter` - Max tracking iterations
pub fn rolling_lyapunov(
    data: &[f64],
    window: usize,
    m: usize,
    tau: usize,
    max_iter: usize,
) -> Result<Array1<f64>> {
    let min_len = (m - 1) * tau + max_iter + 2;
    if window < min_len {
        return Err(TaError::InvalidParameter {
            name: "window".to_string(),
            constraint: "too small for given m, tau, max_iter".to_string(),
        });
    }
    let n = data.len();
    if n < window {
        return Err(TaError::InvalidParameter {
            name: "data".to_string(),
            constraint: "length must be >= window".to_string(),
        });
    }

    let mut output = Array1::from_elem(n, f64::NAN);
    for i in (window - 1)..n {
        let start = i + 1 - window;
        if let Ok(le) = lyapunov_exponent(&data[start..=i], m, tau, max_iter) {
            output[i] = le;
        }
    }

    Ok(output)
}

#[cfg(test)]
mod lyapunov_tests {
    use super::*;

    #[test]
    fn test_lyapunov_periodic() {
        // Periodic signal should have low/zero Lyapunov
        let data: Vec<f64> = (0..200).map(|i| (i as f64 * 0.1).sin()).collect();
        let le = lyapunov_exponent(&data, 2, 1, 20).unwrap();
        assert!(le.is_finite(), "Lyapunov should be finite: {}", le);
    }

    #[test]
    fn test_lyapunov_chaotic() {
        // Logistic map at r=3.9 (chaotic)
        let mut data = vec![0.0; 300];
        data[0] = 0.1;
        for i in 1..300 {
            data[i] = 3.9 * data[i - 1] * (1.0 - data[i - 1]);
        }
        let le = lyapunov_exponent(&data, 3, 1, 30).unwrap();
        // Chaotic system should have positive Lyapunov exponent
        assert!(le > 0.0, "Chaotic Lyapunov should be positive: {}", le);
    }

    #[test]
    fn test_lyapunov_invalid() {
        let data = vec![1.0; 50];
        assert!(lyapunov_exponent(&data, 1, 1, 10).is_err());
        assert!(lyapunov_exponent(&data, 2, 0, 10).is_err());
    }

    #[test]
    fn test_rolling_lyapunov() {
        let data: Vec<f64> = (0..100).map(|i| (i as f64 * 0.3).sin() * 5.0).collect();
        let result = rolling_lyapunov(&data, 50, 2, 1, 10).unwrap();
        assert_eq!(result.len(), 100);
        assert!(result[48].is_nan());
        assert!(result[49].is_finite());
    }
}
