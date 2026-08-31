use crate::impl_indicator_meta;
use crate::impl_standard_methods;
use crate::streaming::rolling_minmax::{RollingMax, RollingMin};
use crate::streaming::traits::StreamingIndicator;
use crate::utils::true_range;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingChop {
    period: usize,
    log_period: f64,
    tr_buf: Vec<f64>,
    tr_head: usize,
    tr_len: usize,
    sum_tr: f64,
    highest: RollingMax,
    lowest: RollingMin,
    prev_close: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingChop {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            log_period: (period as f64).log10(),
            tr_buf: vec![0.0; period],
            tr_head: 0,
            tr_len: 0,
            sum_tr: 0.0,
            highest: RollingMax::new(),
            lowest: RollingMin::new(),
            prev_close: f64::NAN,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64, f64)> for StreamingChop {
    #[inline]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<f64> {
        let (high, low, close) = input;
        self.count += 1;
        let idx = self.count - 1;

        let tr = if self.count == 1 {
            high - low
        } else {
            true_range(high, low, self.prev_close)
        };
        self.prev_close = close;

        let cap = self.period;
        self.sum_tr += tr;
        if self.tr_len < cap {
            self.tr_buf[(self.tr_head + self.tr_len) % cap] = tr;
            self.tr_len += 1;
        } else {
            let old_tr = self.tr_buf[self.tr_head];
            self.sum_tr -= old_tr;
            self.tr_buf[self.tr_head] = tr;
            self.tr_head = (self.tr_head + 1) % cap;
        }

        self.highest.push(idx, high);
        self.lowest.push(idx, low);

        if idx >= self.period {
            let expired = idx - self.period;
            self.highest.pop(expired);
            self.lowest.pop(expired);
        }

        if self.count < self.period {
            self.last_value = None;
            return None;
        }

        let highest = self.highest.current().unwrap();
        let lowest = self.lowest.current().unwrap();

        let range = highest - lowest;
        let result = if range.abs() > 1e-15 && self.sum_tr > 0.0 && self.log_period.abs() > 1e-15 {
            Some(100.0 * (self.sum_tr / range).log10() / self.log_period)
        } else {
            None
        };
        self.last_value = result;
        result
    }

    #[inline]
    fn reset(&mut self) {
        self.tr_head = 0;
        self.tr_len = 0;
        self.sum_tr = 0.0;
        self.highest.reset();
        self.lowest.reset();
        self.prev_close = f64::NAN;
        self.count = 0;
        self.last_value = None;
    }

    #[inline]
    fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingChop, "CHOP", "volatility", "Choppiness Index");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;
    use approx::assert_relative_eq;

    #[test]
    fn test_streaming_chop_basic() {
        let mut chop = StreamingChop::new(3);
        assert_eq!(chop.next((12.0, 10.0, 11.0)), None);
        assert_eq!(chop.next((13.0, 11.0, 12.0)), None);
        let val = chop.next((14.0, 12.0, 13.0)).unwrap();
        assert!(!val.is_nan());
        assert!((0.0..=100.0).contains(&val));
    }

    #[test]
    fn test_streaming_chop_meta() {
        let chop = StreamingChop::new(14);
        assert_eq!(StreamingChop::name(), "CHOP");
        assert_eq!(StreamingChop::category(), "volatility");
        assert_eq!(chop.warm_up_period(), 14);
    }

    #[test]
    fn test_streaming_chop_reset() {
        let mut chop = StreamingChop::new(3);
        for i in 0..5 {
            chop.next((10.0 + i as f64, 8.0 + i as f64, 9.0 + i as f64));
        }
        assert!(chop.is_ready());
        chop.reset();
        assert!(!chop.is_ready());
        assert_eq!(chop.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 100;
        let high: Vec<f64> = (0..n)
            .map(|i| 55.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 2.0).collect();
        let close: Vec<f64> = high
            .iter()
            .zip(low.iter())
            .map(|(h, l)| (h + l) / 2.0)
            .collect();
        let period = 14;

        let batch = crate::indicators::momentum_ext::chop(&high, &low, &close, period).unwrap();
        let mut streaming = StreamingChop::new(period);

        for i in 0..n {
            if let Some(s) = streaming.next((high[i], low[i], close[i])) {
                if !batch[i].is_nan() {
                    assert_relative_eq!(s, batch[i], epsilon = 1e-10);
                }
            }
        }
    }
}
