use super::Transform;

/// A composable data transformation pipeline.
///
/// Chains multiple transforms sequentially: the output of each
/// transform becomes the input of the next.
///
/// # Example
///
/// ```
/// use finkit::transforms::{Pipeline, LogReturn, ZScore, Transform};
///
/// let data = vec![100.0, 105.0, 103.0, 108.0, 110.0, 107.0, 112.0, 115.0, 113.0, 118.0];
/// let result = Pipeline::new()
///     .add(LogReturn)
///     .add(ZScore)
///     .transform(&data);
/// ```
pub struct Pipeline {
    steps: Vec<Box<dyn Transform>>,
}

impl Pipeline {
    /// Create an empty pipeline.
    pub fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Append a transform step.
    #[allow(clippy::should_implement_trait)]
    pub fn add<T: Transform + 'static>(mut self, transform: T) -> Self {
        self.steps.push(Box::new(transform));
        self
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Transform for Pipeline {
    fn transform(&self, input: &[f64]) -> Vec<f64> {
        let mut data = input.to_vec();
        for step in &self.steps {
            data = step.transform(&data);
        }
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transforms::{LogReturn, ZScore, PctChange, MinMaxScaler, StandardScaler};

    #[test]
    fn test_pipeline_log_return_zscore() {
        let data = vec![100.0, 105.0, 103.0, 108.0, 110.0, 107.0, 112.0, 115.0, 113.0, 118.0];
        let result = Pipeline::new()
            .add(LogReturn)
            .add(ZScore)
            .transform(&data);
        assert_eq!(result.len(), 9);
        let sum: f64 = result.iter().sum();
        assert!(sum.abs() < 1e-10);
    }

    #[test]
    fn test_pipeline_single_step() {
        let data = vec![1.0, 2.0, 3.0];
        let result = Pipeline::new().add(PctChange).transform(&data);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_pipeline_empty() {
        let data = vec![1.0, 2.0, 3.0];
        let result = Pipeline::new().transform(&data);
        assert_eq!(result, data);
    }

    #[test]
    fn test_pipeline_minmax_after_standard() {
        let data = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = Pipeline::new()
            .add(StandardScaler)
            .add(MinMaxScaler)
            .transform(&data);
        assert!((result.iter().copied().fold(f64::INFINITY, f64::min)).abs() < 1e-10);
        assert!((result.iter().copied().fold(f64::NEG_INFINITY, f64::max) - 1.0).abs() < 1e-10);
    }
}
