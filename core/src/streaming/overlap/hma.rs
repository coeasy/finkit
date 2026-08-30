use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::streaming::overlap::wma::StreamingWma;

/// Streaming Hull Moving Average (HMA).
///
/// HMA = WMA(2 * WMA(input, period/2) - WMA(input, period), sqrt(period))
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingHma {
    period: usize,
    sqrt_period: usize,
    wma_half: StreamingWma,
    wma_full: StreamingWma,
    wma_final: StreamingWma,
    last_value: Option<f64>,
}

impl StreamingHma {
    pub fn new(period: usize) -> Self {
        let half_period = period / 2;
        let sqrt_period = (period as f64).sqrt().round() as usize;
        Self {
            period,
            sqrt_period,
            wma_half: StreamingWma::new(half_period),
            wma_full: StreamingWma::new(period),
            wma_final: StreamingWma::new(sqrt_period),
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingHma {
    #[inline]
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip(self, input)))]
    fn next(&mut self, input: f64) -> Option<f64> {
        crate::streaming_measure!("hma", self.wma_full.count(), {
            let half = self.wma_half.next(input);
            let full = self.wma_full.next(input);
            let result = match (half, full) {
                (Some(h), Some(f)) => {
                    let diff = 2.0 * h - f;
                    self.wma_final.next(diff)
                }
                _ => None,
            };
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.wma_half.reset();
        self.wma_full.reset();
        self.wma_final.reset();
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.wma_full.is_ready()
            && self.wma_final.is_ready()
            && self.wma_final.count() >= self.sqrt_period
    }

    fn count(&self) -> usize {
        self.wma_full.count()
    }

    fn value(&self) -> Option<f64> {
        self.last_value
    }
}

impl IndicatorMeta for StreamingHma {
    fn name() -> &'static str {
        "HMA"
    }

    fn category() -> &'static str {
        "overlap"
    }

    fn description() -> &'static str {
        "Hull Moving Average"
    }

    fn warm_up_period(&self) -> usize {
        self.period - 1 + self.sqrt_period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_hma_basic() {
        let mut hma = StreamingHma::new(16);
        let data: Vec<f64> = (0..50).map(|i| 50.0 + i as f64).collect();
        let mut last = None;
        for &v in &data {
            last = hma.next(v);
        }
        assert!(last.is_some());
    }

    #[test]
    fn test_streaming_hma_reset() {
        let mut hma = StreamingHma::new(16);
        for i in 0..30 {
            hma.next(i as f64 + 1.0);
        }
        assert!(hma.is_ready());
        hma.reset();
        assert!(!hma.is_ready());
        assert_eq!(hma.count(), 0);
    }

    #[test]
    fn test_streaming_hma_meta() {
        let hma = StreamingHma::new(16);
        assert_eq!(StreamingHma::name(), "HMA");
        assert_eq!(StreamingHma::category(), "overlap");
        assert_eq!(hma.warm_up_period(), 15 + 4);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let period = 16;

        let batch = crate::math::moving_avg::hma(&data, period).unwrap();

        let mut streaming = StreamingHma::new(period);
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
