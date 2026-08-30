use super::Transform;

/// Z-score normalization: (x - mean) / std.
///
/// Output has the same length as input.
/// Returns all zeros for constant or single-element input.
pub struct ZScore;

impl Transform for ZScore {
    fn transform(&self, input: &[f64]) -> Vec<f64> {
        if input.is_empty() {
            return vec![];
        }
        let n = input.len() as f64;
        let mean = input.iter().sum::<f64>() / n;
        let variance = input.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n;
        let std = variance.sqrt();
        if std < 1e-15 {
            return vec![0.0; input.len()];
        }
        input.iter().map(|x| (x - mean) / std).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = ZScore.transform(&data);
        assert_eq!(result.len(), 5);
        let sum: f64 = result.iter().sum();
        assert!(sum.abs() < 1e-10);
    }

    #[test]
    fn test_empty_input() {
        assert!(ZScore.transform(&[]).is_empty());
    }

    #[test]
    fn test_constant_input() {
        let data = vec![5.0, 5.0, 5.0];
        let result = ZScore.transform(&data);
        for v in &result {
            assert!(v.abs() < 1e-10);
        }
    }
}
