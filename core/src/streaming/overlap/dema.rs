use crate::impl_standard_methods;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::StreamingIndicator;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingDema {
    ema1: StreamingEma,
    ema2: StreamingEma,
    period: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingDema {
    pub fn new(period: usize) -> Self {
        Self {
            ema1: StreamingEma::new(period),
            ema2: StreamingEma::new(period),
            period,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingDema {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let e1 = self.ema1.next(input)?;
        let e2 = self.ema2.next(e1)?;
        let result = Some(2.0 * e1 - e2);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.ema1.reset();
        self.ema2.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.ema2.is_ready()
    }

    impl_standard_methods!();
}

impl crate::streaming::IndicatorMeta for StreamingDema {
    fn name() -> &'static str {
        "DEMA"
    }
    fn category() -> &'static str {
        "overlap"
    }
    fn description() -> &'static str {
        "Double Exponential Moving Average"
    }
    fn warm_up_period(&self) -> usize {
        self.period * 2 - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;
    use crate::{test_streaming_reset, test_streaming_vs_batch};

    #[test]
    fn test_streaming_dema_basic() {
        let mut dema = StreamingDema::new(3);
        for i in 0..5 {
            dema.next(i as f64 + 1.0);
        }
        assert!(dema.is_ready());
    }

    #[test]
    fn test_streaming_dema_meta() {
        let dema = StreamingDema::new(10);
        assert_eq!(StreamingDema::name(), "DEMA");
        assert_eq!(StreamingDema::category(), "overlap");
        assert_eq!(dema.warm_up_period(), 19);
    }

    #[test]
    fn test_streaming_dema_reset() {
        test_streaming_reset!(StreamingDema, 3, 10, |ind: &mut StreamingDema, i| {
            ind.next(i);
        });
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        test_streaming_vs_batch!(StreamingDema, 10, |data, period| {
            crate::math::moving_avg::dema(data, period).unwrap()
        });
    }
}
