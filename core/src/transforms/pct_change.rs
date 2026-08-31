use super::Transform;

/// Percentage change transformer: (x\[i\] - x\[i-1\]) / x\[i-1\].
///
/// Output has length `input.len() - 1`.
/// Returns NaN when x[i-1] is zero.
pub struct PctChange;

impl Transform for PctChange {
    fn transform(&self, input: &[f64]) -> Vec<f64> {
        if input.len() < 2 {
            return vec![];
        }
        input.windows(2).map(|w| (w[1] - w[0]) / w[0]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let data = vec![100.0, 110.0, 105.0];
        let result = PctChange.transform(&data);
        assert_eq!(result.len(), 2);
        assert!((result[0] - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_empty_input() {
        assert!(PctChange.transform(&[]).is_empty());
    }

    #[test]
    fn test_negative_change() {
        let data = vec![100.0, 90.0];
        let result = PctChange.transform(&data);
        assert!((result[0] - (-0.1)).abs() < 1e-10);
    }

    #[test]
    fn test_zero_division() {
        let data = vec![0.0, 10.0];
        let result = PctChange.transform(&data);
        assert!(result[0].is_infinite());
    }
}
