use crate::streaming::traits::StreamingIndicator;
use crate::{impl_indicator_meta, impl_standard_methods};

/// Streaming Jurik Moving Average (JMA).
///
/// A low-lag, low-noise adaptive moving average using three-stage filtering.
/// Input: single `f64` price per bar.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingJma {
    period: usize,
    phase_ratio: f64,
    alpha: f64,
    beta: f64,
    e0: f64,
    e1: f64,
    e2: f64,
    jma_val: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingJma {
    pub fn new(period: usize, phase: f64, power: f64) -> Self {
        let phase_ratio = if phase < -100.0 {
            0.5
        } else if phase > 100.0 {
            2.5
        } else {
            phase / 100.0 + 1.5
        };

        let beta = 0.45 * (period as f64 - 1.0) / (0.45 * (period as f64 - 1.0) + 2.0);
        let alpha = beta.powf(power);

        Self {
            period,
            phase_ratio,
            alpha,
            beta,
            e0: 0.0,
            e1: 0.0,
            e2: 0.0,
            jma_val: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<f64, f64> for StreamingJma {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;

        if self.count == 1 {
            // Match batch: output[0] = input[0], e0/e1/e2 start at 0
            self.jma_val = input;
            self.last_value = Some(input);
            return self.last_value;
        }

        self.e0 = (1.0 - self.alpha) * input + self.alpha * self.e0;
        self.e1 = (input - self.e0) * (1.0 - self.beta) + self.beta * self.e1;
        self.e2 = (self.e0 + self.phase_ratio * self.e1 - self.jma_val)
            * (1.0 - self.alpha).powi(2)
            + self.alpha.powi(2) * self.e2;
        self.jma_val += self.e2;

        self.last_value = Some(self.jma_val);
        self.last_value
    }

    fn reset(&mut self) {
        self.e0 = 0.0;
        self.e1 = 0.0;
        self.e2 = 0.0;
        self.jma_val = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= 1
    }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingJma, "JMA", "overlap", "Jurik Moving Average: low-lag adaptive moving average with three-stage filtering");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;

    #[test]
    fn test_streaming_jma_basic() {
        let mut jma = StreamingJma::new(7, 0.0, 2.0);
        let data: Vec<f64> = (0..30).map(|i| 100.0 + (i as f64 * 0.3).sin() * 5.0).collect();

        let mut results = Vec::new();
        for &val in &data {
            if let Some(out) = jma.next(val) {
                results.push(out);
            }
        }
        assert_eq!(results.len(), 30);
        assert!((results[0] - data[0]).abs() < 1e-10);
        for v in &results {
            assert!(v.is_finite());
        }
    }

    #[test]
    fn test_streaming_jma_reset() {
        let mut jma = StreamingJma::new(7, 0.0, 2.0);
        jma.next(100.0);
        jma.next(101.0);
        assert!(jma.is_ready());

        jma.reset();
        assert!(!jma.is_ready());
        assert_eq!(jma.value(), None);
    }

    #[test]
    fn test_streaming_jma_meta() {
        let jma = StreamingJma::new(14, 0.0, 2.0);
        assert_eq!(StreamingJma::name(), "JMA");
        assert_eq!(StreamingJma::category(), "overlap");
        assert_eq!(jma.warm_up_period(), 14);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64 * 0.2).sin() * 5.0).collect();
        let period = 7;
        let phase = 0.0;
        let power = 2.0;

        let batch = crate::indicators::overlap::jma(&data, period, phase, power).unwrap();

        let mut streaming = StreamingJma::new(period, phase, power);
        for i in 0..data.len() {
            let result = streaming.next(data[i]);
            if let Some(val) = result {
                assert!(
                    (val - batch[i]).abs() < 1e-10,
                    "Mismatch at {i}: streaming={val}, batch={}",
                    batch[i]
                );
            }
        }
    }
}
