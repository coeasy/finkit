use crate::impl_standard_methods;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingTrix {
    ema1: StreamingEma,
    ema2: StreamingEma,
    ema3: StreamingEma,
    period: usize,
    prev_triple_ema: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingTrix {
    pub fn new(period: usize) -> Self {
        Self {
            ema1: StreamingEma::new(period),
            ema2: StreamingEma::new(period),
            ema3: StreamingEma::new(period),
            period,
            prev_triple_ema: f64::NAN,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingTrix {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        // 与 batch trix() 完全一致：EMA 链预热期级联 None，
        // 禁止喂 0.0（会污染 EMA2/EMA3 的 SMA 种子）。
        let Some(e1) = self.ema1.next(input) else {
            self.last_value = None;
            return None;
        };
        let Some(e2) = self.ema2.next(e1) else {
            self.last_value = None;
            return None;
        };
        let e3 = self.ema3.next(e2)?;

        let result = if self.prev_triple_ema.is_nan() {
            None
        } else if self.prev_triple_ema.abs() > 1e-15 {
            Some((e3 - self.prev_triple_ema) / self.prev_triple_ema * 100.0)
        } else {
            Some(0.0)
        };

        self.prev_triple_ema = e3;
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.ema1.reset();
        self.ema2.reset();
        self.ema3.reset();
        self.prev_triple_ema = f64::NAN;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.ema3.is_ready() && !self.prev_triple_ema.is_nan()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingTrix {
    fn name() -> &'static str {
        "TRIX"
    }

    fn category() -> &'static str {
        "momentum"
    }

    fn description() -> &'static str {
        "TRIX Triple Exponential Smoothed ROC"
    }

    fn warm_up_period(&self) -> usize {
        self.period * 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_trix_basic() {
        let mut trix = StreamingTrix::new(3);
        for i in 0..10 {
            trix.next(i as f64 + 1.0);
        }
        assert!(trix.is_ready());
    }

    #[test]
    fn test_streaming_trix_reset() {
        let mut trix = StreamingTrix::new(3);
        for i in 0..10 {
            trix.next(i as f64);
        }
        assert!(trix.is_ready());
        trix.reset();
        assert!(!trix.is_ready());
        assert_eq!(trix.count(), 0);
    }

    #[test]
    fn test_streaming_trix_meta() {
        let trix = StreamingTrix::new(14);
        assert_eq!(StreamingTrix::name(), "TRIX");
        assert_eq!(StreamingTrix::category(), "momentum");
        assert_eq!(trix.warm_up_period(), 42);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..50)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 10.0)
            .collect();
        let period = 5;
        let batch = crate::indicators::momentum::trix(&data, period).unwrap();
        let mut streaming = StreamingTrix::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                assert!((s - batch[i]).abs() < 1e-10, "Mismatch at {i}");
            }
        }
    }
}
