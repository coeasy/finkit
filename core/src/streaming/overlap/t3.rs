use crate::impl_standard_methods;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingT3 {
    emas: [StreamingEma; 6],
    c1: f64,
    c2: f64,
    c3: f64,
    c4: f64,
    period: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingT3 {
    pub fn new(period: usize) -> Self {
        Self::with_vfactor(period, 0.7)
    }

    pub fn with_vfactor(period: usize, v: f64) -> Self {
        let c1 = -(v * v * v);
        let c2 = 3.0 * v * v + 3.0 * v * v * v;
        let c3 = -6.0 * v * v - 3.0 * v - 3.0 * v * v * v;
        let c4 = 1.0 + 3.0 * v + v * v * v + 3.0 * v * v;
        Self {
            emas: std::array::from_fn(|_| StreamingEma::new(period)),
            c1,
            c2,
            c3,
            c4,
            period,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingT3 {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let e1 = self.emas[0].next(input)?;
        let e2 = self.emas[1].next(e1)?;
        let e3 = self.emas[2].next(e2)?;
        let e4 = self.emas[3].next(e3)?;
        let e5 = self.emas[4].next(e4)?;
        let e6 = self.emas[5].next(e5)?;
        let result = Some(self.c1 * e6 + self.c2 * e5 + self.c3 * e4 + self.c4 * e3);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        for ema in &mut self.emas {
            ema.reset();
        }
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.emas[5].is_ready()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingT3 {
    fn name() -> &'static str {
        "T3"
    }
    fn category() -> &'static str {
        "overlap"
    }
    fn description() -> &'static str {
        "Triple Exponential Moving Average T3"
    }
    fn warm_up_period(&self) -> usize {
        self.period * 6 - 5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_t3_basic() {
        let mut t3 = StreamingT3::new(3);
        for i in 0..20 {
            t3.next(i as f64 + 1.0);
        }
        assert!(t3.is_ready());
    }

    #[test]
    fn test_streaming_t3_nan_before_ready() {
        let mut t3 = StreamingT3::new(5);
        for _ in 0..10 {
            assert_eq!(t3.next(10.0), None);
        }
    }

    #[test]
    fn test_streaming_t3_meta() {
        assert_eq!(StreamingT3::name(), "T3");
    }

    #[test]
    fn test_streaming_t3_reset() {
        let mut t3 = StreamingT3::new(3);
        for i in 0..20 {
            t3.next(i as f64);
        }
        assert!(t3.is_ready());
        t3.reset();
        assert!(!t3.is_ready());
    }
}
