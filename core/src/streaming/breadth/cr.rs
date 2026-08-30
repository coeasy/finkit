use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};
use crate::impl_standard_methods;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingCr {
    period: usize,
    buffer: Vec<(f64, f64)>,
    head: usize,
    len: usize,
    prev_high: f64,
    prev_low: f64,
    prev_close: f64,
    sum_up: f64,
    sum_down: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingCr {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: vec![(0.0, 0.0); period],
            head: 0,
            len: 0,
            prev_high: f64::NAN,
            prev_low: f64::NAN,
            prev_close: f64::NAN,
            sum_up: 0.0,
            sum_down: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingCr {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.count += 1;

        if self.count == 1 {
            self.prev_high = bar.high();
            self.prev_low = bar.low();
            self.prev_close = bar.close();
            self.last_value = None;
            return None;
        }

        let mid = (self.prev_high + self.prev_low + self.prev_close) / 3.0;
        let up_val = (bar.high() - mid).max(0.0);
        let down_val = (mid - bar.low()).max(0.0);

        self.sum_up += up_val;
        self.sum_down += down_val;

        if self.len == self.period {
            let (leave_up, leave_down) = self.buffer[self.head];
            self.sum_up -= leave_up;
            self.sum_down -= leave_down;
            self.buffer[self.head] = (up_val, down_val);
            self.head = (self.head + 1) % self.period;
        } else {
            let idx = (self.head + self.len) % self.period;
            self.buffer[idx] = (up_val, down_val);
            self.len += 1;
        }

        self.prev_high = bar.high();
        self.prev_low = bar.low();
        self.prev_close = bar.close();

        let result = if self.count > self.period {
            if self.sum_down.abs() <= 1e-15 {
                None
            } else {
                Some(self.sum_up / self.sum_down * 100.0)
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
        self.prev_high = f64::NAN;
        self.prev_low = f64::NAN;
        self.prev_close = f64::NAN;
        self.sum_up = 0.0;
        self.sum_down = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count > self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingCr {
    fn name() -> &'static str {
        "CR"
    }

    fn category() -> &'static str {
        "momentum"
    }

    fn description() -> &'static str {
        "Energy Indicator (能量指标)"
    }

    fn warm_up_period(&self) -> usize {
        self.period + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::types::OhlcvBar;

    #[test]
    fn test_streaming_cr_basic() {
        let mut cr = StreamingCr::new(3);
        let bars = [
            OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0),
            OhlcvBar::new(11.0, 13.0, 10.0, 12.0, 150.0),
            OhlcvBar::new(12.0, 14.0, 11.0, 13.0, 200.0),
            OhlcvBar::new(13.0, 15.0, 12.0, 14.0, 180.0),
        ];
        for bar in &bars[..3] {
            assert_eq!(cr.next(bar), None);
        }
        let v = cr.next(&bars[3]).unwrap();
        assert!(v > 0.0);
    }

    #[test]
    fn test_streaming_cr_reset() {
        let mut cr = StreamingCr::new(3);
        for i in 0..10 {
            cr.next(&OhlcvBar::new(
                i as f64,
                i as f64 + 2.0,
                i as f64 - 1.0,
                i as f64 + 1.0,
                100.0,
            ));
        }
        assert!(cr.is_ready());
        cr.reset();
        assert!(!cr.is_ready());
        assert_eq!(cr.count(), 0);
    }

    #[test]
    fn test_streaming_cr_meta() {
        let cr = StreamingCr::new(26);
        assert_eq!(StreamingCr::name(), "CR");
        assert_eq!(StreamingCr::category(), "momentum");
        assert_eq!(cr.warm_up_period(), 27);
    }
}
