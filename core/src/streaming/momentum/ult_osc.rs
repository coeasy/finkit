use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;
use std::collections::VecDeque;

/// Streaming Ultimate Oscillator (ULTOSC).
///
/// Uses three time periods (7, 14, 28 by default) with weighted average.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingUltOsc {
    period1: usize,
    period2: usize,
    period3: usize,
    max_period: usize,
    bp_buf: VecDeque<f64>,
    tr_buf: VecDeque<f64>,
    prev_close: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingUltOsc {
    pub fn new(period1: usize, period2: usize, period3: usize) -> Self {
        let max_period = period1.max(period2).max(period3);
        Self {
            period1,
            period2,
            period3,
            max_period,
            bp_buf: VecDeque::with_capacity(max_period + 1),
            tr_buf: VecDeque::with_capacity(max_period + 1),
            prev_close: f64::NAN,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64, f64)> for StreamingUltOsc {
    #[inline]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<f64> {
        let (high, low, close) = input;
        self.count += 1;

        if self.count == 1 {
            self.prev_close = close;
            self.last_value = None;
            return None;
        }

        let true_low = low.min(self.prev_close);
        let true_high = high.max(self.prev_close);
        let bp = close - true_low;
        let tr = true_high - true_low;

        self.prev_close = close;

        self.bp_buf.push_back(bp);
        self.tr_buf.push_back(tr);

        if self.bp_buf.len() > self.max_period {
            self.bp_buf.pop_front();
            self.tr_buf.pop_front();
        }

        if self.bp_buf.len() < self.max_period {
            self.last_value = None;
            return None;
        }

        let len = self.bp_buf.len();
        let bp1: f64 = self.bp_buf.iter().skip(len - self.period1).sum();
        let tr1: f64 = self.tr_buf.iter().skip(len - self.period1).sum();
        let bp2: f64 = self.bp_buf.iter().skip(len - self.period2).sum();
        let tr2: f64 = self.tr_buf.iter().skip(len - self.period2).sum();
        let bp3: f64 = self.bp_buf.iter().skip(len - self.period3).sum();
        let tr3: f64 = self.tr_buf.iter().skip(len - self.period3).sum();

        let avg1 = if tr1.abs() > 1e-15 { bp1 / tr1 } else { 0.0 };
        let avg2 = if tr2.abs() > 1e-15 { bp2 / tr2 } else { 0.0 };
        let avg3 = if tr3.abs() > 1e-15 { bp3 / tr3 } else { 0.0 };

        let result = 100.0 * (4.0 * avg1 + 2.0 * avg2 + avg3) / 7.0;
        self.last_value = Some(result);
        Some(result)
    }

    fn reset(&mut self) {
        self.bp_buf.clear();
        self.tr_buf.clear();
        self.prev_close = f64::NAN;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.bp_buf.len() >= self.max_period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingUltOsc {
    fn name() -> &'static str { "ULTOSC" }
    fn category() -> &'static str { "momentum" }
    fn description() -> &'static str { "Ultimate Oscillator" }
    fn warm_up_period(&self) -> usize { self.max_period + 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_ult_osc_basic() {
        let mut uo = StreamingUltOsc::new(7, 14, 28);
        let data: Vec<(f64, f64, f64)> = (0..60)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.2).sin() * 10.0;
                (h, h - 3.0, h - 1.5)
            })
            .collect();
        let mut last = None;
        for &d in &data {
            if let Some(v) = uo.next(d) {
                last = Some(v);
            }
        }
        let v = last.unwrap();
        assert!((0.0..=100.0).contains(&v), "ULTOSC should be 0-100, got {v}");
    }

    #[test]
    fn test_streaming_ult_osc_reset() {
        let mut uo = StreamingUltOsc::new(7, 14, 28);
        for i in 0..60 {
            let h = 50.0 + i as f64;
            uo.next((h, h - 3.0, h - 1.5));
        }
        assert!(uo.is_ready());
        uo.reset();
        assert!(!uo.is_ready());
        assert_eq!(uo.count(), 0);
    }

    #[test]
    fn test_streaming_ult_osc_meta() {
        let uo = StreamingUltOsc::new(7, 14, 28);
        assert_eq!(StreamingUltOsc::name(), "ULTOSC");
        assert_eq!(uo.warm_up_period(), 29);
    }
}
