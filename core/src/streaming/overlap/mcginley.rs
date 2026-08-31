use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMcGinley {
    period: usize,
    prev_md: f64,
    initialized: bool,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingMcGinley {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            prev_md: f64::NAN,
            initialized: false,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingMcGinley {
    #[inline]
    fn next(&mut self, close: f64) -> Option<f64> {
        self.count += 1;

        if !self.initialized {
            self.prev_md = close;
            self.initialized = true;
            let result = Some(close);
            self.last_value = result;
            return result;
        }

        let prev = self.prev_md;
        let result = if prev.abs() <= 1e-15 {
            Some(close)
        } else {
            let ratio = close / prev;
            let r4 = ratio * ratio * ratio * ratio;
            let denom = self.period as f64 * r4;
            Some(if denom.abs() <= 1e-15 {
                close
            } else {
                prev + (close - prev) / denom
            })
        };

        self.prev_md = result.unwrap();
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.prev_md = f64::NAN;
        self.initialized = false;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.initialized
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingMcGinley {
    fn name() -> &'static str {
        "McGinley"
    }

    fn category() -> &'static str {
        "overlap"
    }

    fn description() -> &'static str {
        "McGinley Dynamic"
    }

    fn warm_up_period(&self) -> usize {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_mcginley_basic() {
        let mut md = StreamingMcGinley::new(14);
        let first = md.next(100.0).unwrap();
        assert!((first - 100.0).abs() < 1e-10);
        let second = md.next(105.0).unwrap();
        assert!(second > 100.0 && second < 105.0);
    }

    #[test]
    fn test_streaming_mcginley_meta() {
        let md = StreamingMcGinley::new(14);
        assert_eq!(StreamingMcGinley::name(), "McGinley");
        assert_eq!(StreamingMcGinley::category(), "overlap");
        assert_eq!(md.warm_up_period(), 1);
    }

    #[test]
    fn test_streaming_mcginley_reset() {
        let mut md = StreamingMcGinley::new(10);
        for i in 0..20 {
            md.next(i as f64 + 100.0);
        }
        assert!(md.is_ready());
        md.reset();
        assert!(!md.is_ready());
        assert_eq!(md.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 100;
        let data: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let period = 14;

        let batch = crate::math::moving_avg::mcginley(&data, period).unwrap();
        let mut streaming = StreamingMcGinley::new(period);

        for (i, &val) in data.iter().enumerate() {
            if let Some(s) = streaming.next(val) {
                if !batch[i].is_nan() {
                    assert!(
                        (s - batch[i]).abs() < 1e-10,
                        "Mismatch at {i}: streaming={s}, batch={}",
                        batch[i]
                    );
                }
            }
        }
    }
}
