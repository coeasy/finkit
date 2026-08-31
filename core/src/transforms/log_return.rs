use super::Transform;

/// Log return transformer: ln(x\[i\] / x\[i-1\]).
///
/// Output has length `input.len() - 1`.
/// Returns empty vec for inputs with fewer than 2 elements.
pub struct LogReturn;

impl Transform for LogReturn {
    fn transform(&self, input: &[f64]) -> Vec<f64> {
        if input.len() < 2 {
            return vec![];
        }
        input.windows(2).map(|w| (w[1] / w[0]).ln()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let data = vec![100.0, 110.0, 105.0];
        let result = LogReturn.transform(&data);
        assert_eq!(result.len(), 2);
        assert!((result[0] - (110.0_f64 / 100.0).ln()).abs() < 1e-10);
    }

    #[test]
    fn test_empty_input() {
        assert!(LogReturn.transform(&[]).is_empty());
        assert!(LogReturn.transform(&[1.0]).is_empty());
    }

    #[test]
    fn test_constant_prices() {
        let data = vec![50.0, 50.0, 50.0, 50.0];
        let result = LogReturn.transform(&data);
        for v in &result {
            assert!(v.abs() < 1e-10);
        }
    }
}
