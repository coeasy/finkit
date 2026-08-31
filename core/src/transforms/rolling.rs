use super::Transform;

/// Rolling mean transform over a fixed window.
///
/// Output has the same length as input.
/// The first `window - 1` elements use a growing window.
pub struct RollingMean {
    pub window: usize,
}

impl Transform for RollingMean {
    fn transform(&self, input: &[f64]) -> Vec<f64> {
        if input.is_empty() || self.window == 0 {
            return vec![];
        }
        let w = self.window;
        let mut result = Vec::with_capacity(input.len());
        let mut sum = 0.0;
        for (i, &val) in input.iter().enumerate() {
            sum += val;
            if i >= w {
                sum -= input[i - w];
                result.push(sum / w as f64);
            } else {
                result.push(sum / (i + 1) as f64);
            }
        }
        result
    }
}

/// Rolling standard deviation transform over a fixed window.
///
/// Uses population std (N denominator). Uses numerically stable variance formula
/// based on Welford's online algorithm to avoid catastrophic cancellation.
/// The first `window - 1` elements use a growing window.
pub struct RollingStd {
    pub window: usize,
}

impl Transform for RollingStd {
    fn transform(&self, input: &[f64]) -> Vec<f64> {
        if input.is_empty() || self.window == 0 {
            return vec![];
        }
        let w = self.window;
        let mut result = Vec::with_capacity(input.len());
        let mut count: usize = 0;
        let mut mean = 0.0;
        let mut m2 = 0.0;
        for (i, &val) in input.iter().enumerate() {
            count += 1;
            let delta = val - mean;
            mean += delta / count as f64;
            let delta2 = val - mean;
            m2 += delta * delta2;
            if count > w {
                let old = input[i - w];
                count -= 1;
                if count == 0 {
                    mean = 0.0;
                    m2 = 0.0;
                } else {
                    let delta = old - mean;
                    mean -= delta / count as f64;
                    let delta2 = old - mean;
                    m2 -= delta * delta2;
                }
            }
            let var = m2 / count as f64;
            result.push(var.max(0.0).sqrt());
        }
        result
    }
}

/// Rolling sum transform over a fixed window.
///
/// Output has the same length as input.
/// The first `window - 1` elements use a growing window.
pub struct RollingSum {
    pub window: usize,
}

impl Transform for RollingSum {
    fn transform(&self, input: &[f64]) -> Vec<f64> {
        if input.is_empty() || self.window == 0 {
            return vec![];
        }
        let w = self.window;
        let mut result = Vec::with_capacity(input.len());
        let mut sum = 0.0;
        for (i, &val) in input.iter().enumerate() {
            sum += val;
            if i >= w {
                sum -= input[i - w];
            }
            result.push(sum);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rolling_mean_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = RollingMean { window: 3 }.transform(&data);
        assert_eq!(result.len(), 5);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 1.5).abs() < 1e-10);
        assert!((result[2] - 2.0).abs() < 1e-10);
        assert!((result[3] - 3.0).abs() < 1e-10);
        assert!((result[4] - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_rolling_mean_empty() {
        assert!(RollingMean { window: 3 }.transform(&[]).is_empty());
    }

    #[test]
    fn test_rolling_mean_window_zero() {
        assert!(RollingMean { window: 0 }.transform(&[1.0, 2.0]).is_empty());
    }

    #[test]
    fn test_rolling_mean_window_larger_than_input() {
        let data = vec![2.0, 4.0];
        let result = RollingMean { window: 5 }.transform(&data);
        assert!((result[0] - 2.0).abs() < 1e-10);
        assert!((result[1] - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_rolling_std_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = RollingStd { window: 3 }.transform(&data);
        assert_eq!(result.len(), 5);
        assert!((result[0] - 0.0).abs() < 1e-10);
        // std of [1,2]: mean=1.5, var=((0.25+0.25)/2)=0.25, std=0.5
        assert!((result[1] - 0.5).abs() < 1e-10);
        // std of [1,2,3]: mean=2, var=((1+0+1)/3)=0.6667, std≈0.8165
        let expected_std_3 = (2.0_f64 / 3.0).sqrt();
        assert!((result[2] - expected_std_3).abs() < 1e-10);
    }

    #[test]
    fn test_rolling_std_empty() {
        assert!(RollingStd { window: 3 }.transform(&[]).is_empty());
    }

    #[test]
    fn test_rolling_std_constant() {
        let data = vec![5.0, 5.0, 5.0, 5.0];
        let result = RollingStd { window: 3 }.transform(&data);
        for v in &result {
            assert!(v.abs() < 1e-10);
        }
    }

    #[test]
    fn test_rolling_std_large_values() {
        let base = 1e8;
        let data: Vec<f64> = (0..20)
            .map(|i| base + (i as f64 * 0.01).sin() * 0.001)
            .collect();
        let result = RollingStd { window: 10 }.transform(&data);
        for (i, &v) in result.iter().enumerate() {
            if i >= 9 {
                assert!(v.is_finite(), "Non-finite stddev at index {i}: {v}");
                assert!(v >= 0.0, "Negative stddev at index {i}: {v}");
                assert!(v < 1.0, "Unexpectedly large stddev at index {i}: {v}");
            }
        }
    }

    #[test]
    fn test_rolling_sum_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = RollingSum { window: 3 }.transform(&data);
        assert_eq!(result.len(), 5);
        assert!((result[0] - 1.0).abs() < 1e-10);
        assert!((result[1] - 3.0).abs() < 1e-10);
        assert!((result[2] - 6.0).abs() < 1e-10);
        assert!((result[3] - 9.0).abs() < 1e-10);
        assert!((result[4] - 12.0).abs() < 1e-10);
    }

    #[test]
    fn test_rolling_sum_empty() {
        assert!(RollingSum { window: 3 }.transform(&[]).is_empty());
    }
}
