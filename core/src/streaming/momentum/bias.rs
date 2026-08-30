use crate::streaming::overlap::sma::StreamingSma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming BIAS (乖离率) — deviation from moving average.
///
/// BIAS = (Close - SMA(Close, period)) / SMA(Close, period) * 100
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingBias {
    period: usize,
    sma: StreamingSma,
    last_value: Option<f64>,
}

impl StreamingBias {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            sma: StreamingSma::new(period),
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingBias {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        let ma = self.sma.next(input)?;
        if ma.abs() <= 1e-15 {
            self.last_value = None;
            return None;
        }
        let result = Some((input - ma) / ma * 100.0);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.sma.reset();
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.sma.is_ready()
    }

    fn count(&self) -> usize {
        self.sma.count()
    }

    fn value(&self) -> Option<f64> {
        self.last_value
    }
}

impl IndicatorMeta for StreamingBias {
    fn name() -> &'static str {
        "BIAS"
    }

    fn category() -> &'static str {
        "momentum"
    }

    fn description() -> &'static str {
        "Deviation Rate (乖离率)"
    }

    fn warm_up_period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_bias_basic() {
        let mut bias = StreamingBias::new(3);
        assert_eq!(bias.next(10.0), None);
        assert_eq!(bias.next(20.0), None);
        let v = bias.next(30.0).unwrap();
        // SMA = 20, BIAS = (30-20)/20*100 = 50
        assert!((v - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_bias_reset() {
        let mut bias = StreamingBias::new(5);
        for i in 0..10 {
            bias.next(i as f64 + 1.0);
        }
        assert!(bias.is_ready());
        bias.reset();
        assert!(!bias.is_ready());
        assert_eq!(bias.count(), 0);
    }

    #[test]
    fn test_streaming_bias_meta() {
        let bias = StreamingBias::new(6);
        assert_eq!(StreamingBias::name(), "BIAS");
        assert_eq!(StreamingBias::category(), "momentum");
        assert_eq!(bias.warm_up_period(), 6);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let period = 6;

        let batch = crate::indicators::china::bias(&data, period).unwrap();

        let mut streaming = StreamingBias::new(period);
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
