use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};
use crate::impl_standard_methods;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingBr {
    period: usize,
    buffer: Vec<(f64, f64)>,
    head: usize,
    len: usize,
    prev_close: f64,
    sum_up: f64,
    sum_down: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingBr {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: vec![(0.0, 0.0); period],
            head: 0,
            len: 0,
            prev_close: f64::NAN,
            sum_up: 0.0,
            sum_down: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingBr {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.count += 1;

        if self.count == 1 {
            self.prev_close = bar.close();
            self.last_value = None;
            return None;
        }

        let up_val = (bar.high() - self.prev_close).max(0.0);
        let down_val = (self.prev_close - bar.low()).max(0.0);

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

impl IndicatorMeta for StreamingBr {
    fn name() -> &'static str {
        "BR"
    }

    fn category() -> &'static str {
        "momentum"
    }

    fn description() -> &'static str {
        "Bias Ratio (意愿指标)"
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
    fn test_streaming_br_basic() {
        let mut br = StreamingBr::new(3);
        let bars = [
            OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0),
            OhlcvBar::new(11.0, 13.0, 10.0, 12.0, 150.0),
            OhlcvBar::new(12.0, 14.0, 11.0, 13.0, 200.0),
            OhlcvBar::new(13.0, 15.0, 12.0, 14.0, 180.0),
        ];
        for bar in &bars[..3] {
            assert_eq!(br.next(bar), None);
        }
        let v = br.next(&bars[3]).unwrap();
        assert!(v > 0.0);
    }

    #[test]
    fn test_streaming_br_reset() {
        let mut br = StreamingBr::new(3);
        for i in 0..10 {
            br.next(&OhlcvBar::new(
                i as f64,
                i as f64 + 2.0,
                i as f64 - 1.0,
                i as f64 + 1.0,
                100.0,
            ));
        }
        assert!(br.is_ready());
        br.reset();
        assert!(!br.is_ready());
        assert_eq!(br.count(), 0);
    }

    #[test]
    fn test_streaming_br_meta() {
        let br = StreamingBr::new(26);
        assert_eq!(StreamingBr::name(), "BR");
        assert_eq!(StreamingBr::category(), "momentum");
        assert_eq!(br.warm_up_period(), 27);
    }
}
