use super::Transform;

/// First-order difference: x\[i\] - x\[i-1\].
///
/// Output has length `input.len() - 1`.
pub struct Diff;

impl Transform for Diff {
    fn transform(&self, input: &[f64]) -> Vec<f64> {
        if input.len() < 2 {
            return vec![];
        }
        input.windows(2).map(|w| w[1] - w[0]).collect()
    }
}

/// N-th order difference transform.
///
/// Applies first-order differencing `order` times.
/// Output length is `input.len() - order`.
pub struct DiffN {
    pub order: usize,
}

impl Transform for DiffN {
    fn transform(&self, input: &[f64]) -> Vec<f64> {
        let mut data = input.to_vec();
        for _ in 0..self.order {
            if data.len() < 2 {
                return vec![];
            }
            data = data.windows(2).map(|w| w[1] - w[0]).collect();
        }
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_basic() {
        let data = vec![1.0, 3.0, 6.0, 10.0, 15.0];
        let result = Diff.transform(&data);
        assert_eq!(result, vec![2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_diff_empty() {
        assert!(Diff.transform(&[]).is_empty());
        assert!(Diff.transform(&[1.0]).is_empty());
    }

    #[test]
    fn test_diff_negative() {
        let data = vec![10.0, 7.0, 3.0];
        let result = Diff.transform(&data);
        assert_eq!(result, vec![-3.0, -4.0]);
    }

    #[test]
    fn test_diff_n_order2() {
        let data = vec![1.0, 3.0, 6.0, 10.0, 15.0];
        let result = DiffN { order: 2 }.transform(&data);
        assert_eq!(result, vec![1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_diff_n_order0() {
        let data = vec![1.0, 2.0, 3.0];
        let result = DiffN { order: 0 }.transform(&data);
        assert_eq!(result, data);
    }

    #[test]
    fn test_diff_n_too_many_orders() {
        let data = vec![1.0, 2.0, 3.0];
        let result = DiffN { order: 5 }.transform(&data);
        assert!(result.is_empty());
    }
}
