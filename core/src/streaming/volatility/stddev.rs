use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::streaming::volatility::var::StreamingVar;

/// Streaming Standard Deviation using Welford's online algorithm with rolling window.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingStdDev {
    var: StreamingVar,
}

impl StreamingStdDev {
    pub fn new(period: usize) -> Self {
        Self {
            var: StreamingVar::new(period),
        }
    }
}

impl StreamingIndicator for StreamingStdDev {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.var.next(input).map(|v| v.max(0.0).sqrt())
    }

    fn reset(&mut self) {
        self.var.reset();
    }

    fn is_ready(&self) -> bool {
        self.var.is_ready()
    }
    fn count(&self) -> usize {
        self.var.count()
    }
    fn value(&self) -> Option<f64> {
        self.var.value().map(|v| v.max(0.0).sqrt())
    }
}

impl IndicatorMeta for StreamingStdDev {
    fn name() -> &'static str {
        "STDDEV"
    }
    fn category() -> &'static str {
        "statistic"
    }
    fn description() -> &'static str {
        "Rolling Standard Deviation (Welford)"
    }
    fn warm_up_period(&self) -> usize {
        self.var.warm_up_period()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_stddev_basic() {
        let mut sd = StreamingStdDev::new(3);
        assert_eq!(sd.next(2.0), None);
        assert_eq!(sd.next(4.0), None);
        let v = sd.next(6.0).unwrap();
        assert!((v - 2.0).abs() < 1e-10); // std([2,4,6]) = 2.0
    }

    #[test]
    fn test_streaming_stddev_meta() {
        assert_eq!(StreamingStdDev::name(), "STDDEV");
        assert_eq!(StreamingStdDev::category(), "statistic");
    }

    #[test]
    fn test_streaming_stddev_reset() {
        let mut sd = StreamingStdDev::new(3);
        sd.next(1.0);
        sd.next(2.0);
        sd.next(3.0);
        assert!(sd.is_ready());
        sd.reset();
        assert!(!sd.is_ready());
        assert_eq!(sd.count(), 0);
    }

    #[test]
    fn test_streaming_stddev_welford_stability() {
        let mut sd = StreamingStdDev::new(100);
        let base = 1e6;
        for i in 0..200 {
            let v = base + (i as f64 * 0.01).sin() * 0.001;
            if let Some(s) = sd.next(v) {
                assert!(s.is_finite(), "NaN/Inf at i={i}");
                assert!(s >= 0.0, "Negative stddev at i={i}");
            }
        }
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let period = 20;
        let batch = crate::math::statistics::rolling_std_dev(&data, period).unwrap();

        let mut streaming = StreamingStdDev::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                assert!(
                    (s - batch[i]).abs() < 1e-8,
                    "Mismatch at {i}: streaming={s}, batch={}",
                    batch[i]
                );
            }
        }
    }
}
