use crate::impl_standard_methods;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMassIndex {
    period: usize,
    ema_period: usize,
    ema1: StreamingEma,
    ema2: StreamingEma,
    ratio_buf: Vec<f64>,
    ratio_head: usize,
    ratio_len: usize,
    ratio_sum: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingMassIndex {
    pub fn new(period: usize, ema_period: usize) -> Self {
        Self {
            period,
            ema_period,
            ema1: StreamingEma::new(ema_period),
            ema2: StreamingEma::new(ema_period),
            ratio_buf: vec![0.0; period],
            ratio_head: 0,
            ratio_len: 0,
            ratio_sum: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingMassIndex {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.count += 1;
        let hl = bar.high() - bar.low();

        let Some(ema1_val) = self.ema1.next(hl) else {
            self.last_value = None;
            return None;
        };

        let Some(ema2_val) = self.ema2.next(ema1_val) else {
            self.last_value = None;
            return None;
        };

        if ema2_val.abs() <= 1e-15 {
            self.last_value = None;
            return None;
        }

        let ratio = ema1_val / ema2_val;
        let cap = self.period;

        self.ratio_sum += ratio;
        if self.ratio_len < cap {
            self.ratio_buf[(self.ratio_head + self.ratio_len) % cap] = ratio;
            self.ratio_len += 1;
        } else {
            let old = self.ratio_buf[self.ratio_head];
            self.ratio_sum -= old;
            self.ratio_buf[self.ratio_head] = ratio;
            self.ratio_head = (self.ratio_head + 1) % cap;
        }

        let result = if self.ratio_len == self.period {
            Some(self.ratio_sum)
        } else {
            None
        };
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.ema1.reset();
        self.ema2.reset();
        self.ratio_head = 0;
        self.ratio_len = 0;
        self.ratio_sum = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.ratio_len >= self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingMassIndex {
    fn name() -> &'static str {
        "Mass Index"
    }

    fn category() -> &'static str {
        "volatility"
    }

    fn description() -> &'static str {
        "Mass Index"
    }

    fn warm_up_period(&self) -> usize {
        2 * self.ema_period + self.period - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::OhlcvBar;

    #[test]
    fn test_streaming_mass_index_basic() {
        let mut mi = StreamingMassIndex::new(25, 9);
        let bars: Vec<OhlcvBar> = (0..60)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.2).sin() * 10.0;
                let l = h - 3.0;
                let c = (h + l) / 2.0;
                OhlcvBar::new(c - 0.5, h, l, c, 1000.0)
            })
            .collect();
        let mut last = None;
        for bar in &bars {
            last = mi.next(bar);
        }
        assert!(last.is_some());
        assert!(last.unwrap() > 0.0);
    }

    #[test]
    fn test_streaming_mass_index_meta() {
        let mi = StreamingMassIndex::new(25, 9);
        assert_eq!(StreamingMassIndex::name(), "Mass Index");
        assert_eq!(StreamingMassIndex::category(), "volatility");
        assert_eq!(mi.warm_up_period(), 42);
    }

    #[test]
    fn test_streaming_mass_index_reset() {
        let mut mi = StreamingMassIndex::new(5, 3);
        let bars: Vec<OhlcvBar> = (0..30)
            .map(|i| OhlcvBar::new(i as f64, i as f64 + 2.0, i as f64, i as f64 + 1.0, 100.0))
            .collect();
        for bar in &bars {
            mi.next(bar);
        }
        assert!(mi.is_ready());
        mi.reset();
        assert!(!mi.is_ready());
        assert_eq!(mi.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 120;
        let bars: Vec<OhlcvBar> = (0..n)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.2).sin() * 10.0;
                let l = h - 3.0;
                let c = (h + l) / 2.0;
                let v = 1000.0 + (i as f64 * 0.5).cos() * 500.0;
                OhlcvBar::new(c - 0.5, h, l, c, v)
            })
            .collect();
        let high: Vec<f64> = bars.iter().map(|b| b.high()).collect();
        let low: Vec<f64> = bars.iter().map(|b| b.low()).collect();
        let period = 25;
        let ema_period = 9;

        let batch =
            crate::indicators::volatility_ext::mass_index(&high, &low, period, ema_period).unwrap();
        let mut streaming = StreamingMassIndex::new(period, ema_period);

        for (i, bar) in bars.iter().enumerate() {
            if let Some(s) = streaming.next(bar) {
                if !batch[i].is_nan() {
                    assert!(
                        (s - batch[i]).abs() < 1e-10,
                        "Mismatch at {i}: streaming={s}, batch={}",
                        batch[i]
                    );
                }
            }
        }
    }
}
