use super::Transform;

/// Standard scaler: (x - mean) / std, using sample standard deviation (N-1).
///
/// Similar to ZScore but uses Bessel-corrected (N-1) denominator
/// for sample standard deviation, matching scikit-learn's StandardScaler.
pub struct StandardScaler;

impl Transform for StandardScaler {
    fn transform(&self, input: &[f64]) -> Vec<f64> {
        if input.len() < 2 {
            return input.to_vec();
        }
        let n = input.len() as f64;
        let mean = input.iter().sum::<f64>() / n;
        let variance = input.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let std = variance.sqrt();
        if std < 1e-15 {
            return vec![0.0; input.len()];
        }
        input.iter().map(|x| (x - mean) / std).collect()
    }
}

/// Min-max scaler: (x - min) / (max - min), scales to [0, 1].
pub struct MinMaxScaler;

impl Transform for MinMaxScaler {
    fn transform(&self, input: &[f64]) -> Vec<f64> {
        if input.is_empty() {
            return vec![];
        }
        let min = input.iter().copied().fold(f64::INFINITY, f64::min);
        let max = input.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let range = max - min;
        if range < 1e-15 {
            return vec![0.0; input.len()];
        }
        input.iter().map(|x| (x - min) / range).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_scaler_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = StandardScaler.transform(&data);
        assert_eq!(result.len(), 5);
        let sum: f64 = result.iter().sum();
        assert!(sum.abs() < 1e-10);
    }

    #[test]
    fn test_standard_scaler_constant() {
        let data = vec![3.0, 3.0, 3.0];
        let result = StandardScaler.transform(&data);
        for v in &result {
            assert!(v.abs() < 1e-10);
        }
    }

    #[test]
    fn test_standard_scaler_single() {
        let data = vec![42.0];
        let result = StandardScaler.transform(&data);
        assert_eq!(result, vec![42.0]);
    }

    #[test]
    fn test_minmax_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = MinMaxScaler.transform(&data);
        assert!((result[0] - 0.0).abs() < 1e-10);
        assert!((result[4] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_minmax_empty() {
        assert!(MinMaxScaler.transform(&[]).is_empty());
    }

    #[test]
    fn test_minmax_constant() {
        let data = vec![7.0, 7.0, 7.0];
        let result = MinMaxScaler.transform(&data);
        for v in &result {
            assert!(v.abs() < 1e-10);
        }
    }
}
