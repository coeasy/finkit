use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingAr {
    period: usize,
    buffer: Vec<(f64, f64)>,
    head: usize,
    len: usize,
    sum_ho: f64,
    sum_ol: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingAr {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: vec![(0.0, 0.0); period],
            head: 0,
            len: 0,
            sum_ho: 0.0,
            sum_ol: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingAr {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.count += 1;

        let ho = bar.high() - bar.open();
        let ol = bar.open() - bar.low();

        self.sum_ho += ho;
        self.sum_ol += ol;

        if self.len == self.period {
            let (leave_ho, leave_ol) = self.buffer[self.head];
            self.sum_ho -= leave_ho;
            self.sum_ol -= leave_ol;
            self.buffer[self.head] = (ho, ol);
            self.head = (self.head + 1) % self.period;
        } else {
            let idx = (self.head + self.len) % self.period;
            self.buffer[idx] = (ho, ol);
            self.len += 1;
        }

        let result = if self.count >= self.period {
            if self.sum_ol.abs() <= 1e-15 {
                None
            } else {
                Some(self.sum_ho / self.sum_ol * 100.0)
            }
        } else {
            None
        };
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.sum_ho = 0.0;
        self.sum_ol = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingAr {
    fn name() -> &'static str {
        "AR"
    }

    fn category() -> &'static str {
        "momentum"
    }

    fn description() -> &'static str {
        "Activity Ratio (人气指标)"
    }

    fn warm_up_period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::types::OhlcvBar;

    #[test]
    fn test_streaming_ar_basic() {
        let mut ar = StreamingAr::new(3);
        let bars = [
            OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0),
            OhlcvBar::new(11.0, 13.0, 10.0, 12.0, 150.0),
            OhlcvBar::new(12.0, 14.0, 11.0, 13.0, 200.0),
        ];
        for bar in &bars[..2] {
            assert_eq!(ar.next(bar), None);
        }
        let v = ar.next(&bars[2]).unwrap();
        assert!(v > 0.0);
    }

    #[test]
    fn test_streaming_ar_reset() {
        let mut ar = StreamingAr::new(3);
        for i in 0..10 {
            ar.next(&OhlcvBar::new(
                i as f64,
                i as f64 + 2.0,
                i as f64 - 1.0,
                i as f64 + 1.0,
                100.0,
            ));
        }
        assert!(ar.is_ready());
        ar.reset();
        assert!(!ar.is_ready());
        assert_eq!(ar.count(), 0);
    }

    #[test]
    fn test_streaming_ar_meta() {
        let ar = StreamingAr::new(26);
        assert_eq!(StreamingAr::name(), "AR");
        assert_eq!(StreamingAr::category(), "momentum");
        assert_eq!(ar.warm_up_period(), 26);
    }
}
