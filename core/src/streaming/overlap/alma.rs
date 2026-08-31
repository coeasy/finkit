use crate::streaming::traits::StreamingIndicator;
use crate::{impl_indicator_meta, impl_standard_methods};

/// Streaming ALMA (Arnaud Legoux Moving Average).
///
/// Uses a precomputed Gaussian kernel over a ring buffer of prices.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingAlma {
    period: usize,
    buffer: Vec<f64>,
    head: usize,
    len: usize,
    weights: Vec<f64>,
    weight_sum: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingAlma {
    pub fn new(period: usize, sigma: f64, offset: f64) -> Self {
        let m = offset * (period - 1) as f64;
        let s = period as f64 / sigma;

        let mut weights = vec![0.0; period];
        let mut weight_sum = 0.0;
        for (i, w) in weights.iter_mut().enumerate() {
            *w = (-((i as f64 - m).powi(2)) / (2.0 * s * s)).exp();
            weight_sum += *w;
        }

        Self {
            period,
            buffer: vec![0.0; period],
            head: 0,
            len: 0,
            weights,
            weight_sum,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingAlma {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;

        if self.len == self.period {
            // ring full — overwrite oldest
        } else {
            self.len += 1;
        }

        self.buffer[self.head] = input;
        self.head += 1;
        if self.head == self.period {
            self.head = 0;
        }

        if self.len < self.period {
            self.last_value = None;
            return None;
        }

        let mut sum = 0.0;
        for j in 0..self.period {
            let idx = (self.head + j) % self.period;
            sum += self.buffer[idx] * self.weights[j];
        }
        let result = Some(sum / self.weight_sum);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.len >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(
    StreamingAlma,
    "ALMA",
    "overlap",
    "Arnaud Legoux Moving Average"
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;
    use crate::streaming::StreamingIndicator;

    #[test]
    fn test_streaming_alma_basic() {
        let mut alma = StreamingAlma::new(5, 6.0, 0.85);
        assert_eq!(alma.next(10.0), None);
        assert_eq!(alma.next(11.0), None);
        assert_eq!(alma.next(12.0), None);
        assert_eq!(alma.next(13.0), None);
        let v = alma.next(14.0).unwrap();
        assert!(v.is_finite());
    }

    #[test]
    fn test_streaming_alma_reset() {
        let mut alma = StreamingAlma::new(9, 6.0, 0.85);
        for i in 0..20 {
            alma.next(i as f64 + 1.0);
        }
        assert!(alma.is_ready());
        alma.reset();
        assert!(!alma.is_ready());
        assert_eq!(alma.count(), 0);
    }

    #[test]
    fn test_streaming_alma_meta() {
        let alma = StreamingAlma::new(9, 6.0, 0.85);
        assert_eq!(StreamingAlma::name(), "ALMA");
        assert_eq!(StreamingAlma::category(), "overlap");
        assert_eq!(alma.warm_up_period(), 9);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let period = 9;
        let sigma = 6.0;
        let offset = 0.85;

        let batch = crate::math::moving_avg::alma(&data, period, sigma, offset).unwrap();

        let mut streaming = StreamingAlma::new(period, sigma, offset);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                assert!(
                    (s - batch[i]).abs() < 1e-10,
                    "Mismatch at {i}: streaming={s}, batch={}",
                    batch[i]
                );
            }
        }
    }
}
