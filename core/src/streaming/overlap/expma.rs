use crate::impl_standard_methods;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExpmaOutput {
    pub ema_short: f64,
    pub ema_long: f64,
}

/// Streaming EXPMA (Exponential Moving Average Group 指数平滑均线).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingExpma {
    short_period: usize,
    long_period: usize,
    ema_short: StreamingEma,
    ema_long: StreamingEma,
    count: usize,
    last_value: Option<ExpmaOutput>,
}

impl StreamingExpma {
    pub fn new(short_period: usize, long_period: usize) -> Self {
        Self {
            short_period,
            long_period,
            ema_short: StreamingEma::new(short_period),
            ema_long: StreamingEma::new(long_period),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<f64, ExpmaOutput> for StreamingExpma {
    #[inline]
    fn next(&mut self, input: f64) -> Option<ExpmaOutput> {
        self.count += 1;

        let short = self.ema_short.next(input);
        let long = self.ema_long.next(input);
        let (Some(ema_short), Some(ema_long)) = (short, long) else {
            self.last_value = None;
            return None;
        };

        let result = Some(ExpmaOutput {
            ema_short,
            ema_long,
        });
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.ema_short.reset();
        self.ema_long.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.ema_short.is_ready() && self.ema_long.is_ready()
    }

    impl_standard_methods!(output = ExpmaOutput);
}

impl IndicatorMeta for StreamingExpma {
    fn name() -> &'static str {
        "EXPMA"
    }

    fn category() -> &'static str {
        "overlap"
    }

    fn description() -> &'static str {
        "Exponential Moving Average Group (指数平滑均线)"
    }

    fn warm_up_period(&self) -> usize {
        self.short_period.max(self.long_period)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_expma_basic() {
        let mut expma = StreamingExpma::new(3, 5);
        for i in 1..=5 {
            expma.next(i as f64 * 10.0);
        }
        assert!(expma.is_ready());
        let out = expma.value().unwrap();
        assert!(out.ema_short.is_finite());
        assert!(out.ema_long.is_finite());
    }

    #[test]
    fn test_streaming_expma_reset() {
        let mut expma = StreamingExpma::new(3, 5);
        for i in 0..20 {
            expma.next(i as f64 + 1.0);
        }
        assert!(expma.is_ready());
        expma.reset();
        assert!(!expma.is_ready());
        assert_eq!(expma.count(), 0);
    }

    #[test]
    fn test_streaming_expma_meta() {
        let expma = StreamingExpma::new(12, 50);
        assert_eq!(StreamingExpma::name(), "EXPMA");
        assert_eq!(StreamingExpma::category(), "overlap");
        assert_eq!(expma.warm_up_period(), 50);
    }
}
