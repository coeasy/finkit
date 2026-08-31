use crate::impl_standard_methods;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingTema {
    ema1: StreamingEma,
    ema2: StreamingEma,
    ema3: StreamingEma,
    period: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingTema {
    pub fn new(period: usize) -> Self {
        Self {
            ema1: StreamingEma::new(period),
            ema2: StreamingEma::new(period),
            ema3: StreamingEma::new(period),
            period,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingTema {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let e1 = self.ema1.next(input)?;
        let e2 = self.ema2.next(e1)?;
        let e3 = self.ema3.next(e2)?;
        let result = Some(3.0 * e1 - 3.0 * e2 + e3);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.ema1.reset();
        self.ema2.reset();
        self.ema3.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.ema3.is_ready()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingTema {
    fn name() -> &'static str {
        "TEMA"
    }
    fn category() -> &'static str {
        "overlap"
    }
    fn description() -> &'static str {
        "Triple Exponential Moving Average"
    }
    fn warm_up_period(&self) -> usize {
        self.period * 3 - 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_tema_basic() {
        let mut tema = StreamingTema::new(3);
        for i in 0..10 {
            tema.next(i as f64 + 1.0);
        }
        assert!(tema.is_ready());
    }

    #[test]
    fn test_streaming_tema_meta() {
        assert_eq!(StreamingTema::name(), "TEMA");
    }

    #[test]
    fn test_streaming_tema_reset() {
        let mut tema = StreamingTema::new(3);
        for i in 0..10 {
            tema.next(i as f64);
        }
        assert!(tema.is_ready());
        tema.reset();
        assert!(!tema.is_ready());
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let period = 5;
        let batch = crate::math::moving_avg::tema(&data, period).unwrap();
        let mut streaming = StreamingTema::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                assert!((s - batch[i]).abs() < 1e-10, "Mismatch at {i}");
            }
        }
    }
}
