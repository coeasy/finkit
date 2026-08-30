use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingApo {
    slow_period: usize,
    fast_ema: StreamingEma,
    slow_ema: StreamingEma,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingApo {
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self {
            slow_period,
            fast_ema: StreamingEma::new(fast_period),
            slow_ema: StreamingEma::new(slow_period),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingApo {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let fast = self.fast_ema.next(input);
        let slow = self.slow_ema.next(input);
        let result = match (fast, slow) {
            (Some(f), Some(s)) => Some(f - s),
            _ => None,
        };
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.fast_ema.reset();
        self.slow_ema.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.fast_ema.is_ready() && self.slow_ema.is_ready()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingApo {
    fn name() -> &'static str {
        "APO"
    }

    fn category() -> &'static str {
        "momentum"
    }

    fn description() -> &'static str {
        "Absolute Price Oscillator"
    }

    fn warm_up_period(&self) -> usize {
        self.slow_period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_apo_basic() {
        let mut apo = StreamingApo::new(2, 4);
        assert_eq!(apo.next(10.0), None);
        assert_eq!(apo.next(12.0), None);
        assert_eq!(apo.next(14.0), None);
        let v = apo.next(16.0).unwrap();
        assert!(v.is_finite());
    }

    #[test]
    fn test_streaming_apo_reset() {
        let mut apo = StreamingApo::new(3, 5);
        for i in 0..10 {
            apo.next(i as f64 + 1.0);
        }
        assert!(apo.is_ready());
        apo.reset();
        assert!(!apo.is_ready());
        assert_eq!(apo.count(), 0);
    }

    #[test]
    fn test_streaming_apo_meta() {
        let apo = StreamingApo::new(12, 26);
        assert_eq!(StreamingApo::name(), "APO");
        assert_eq!(StreamingApo::category(), "momentum");
        assert_eq!(apo.warm_up_period(), 26);
    }

    #[test]
    fn test_streaming_apo_ema_difference() {
        let mut apo = StreamingApo::new(2, 3);
        apo.next(10.0);
        apo.next(20.0);
        apo.next(30.0);
        let result = apo.next(40.0);
        assert!(result.is_some());
    }
}
