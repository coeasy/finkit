use crate::impl_standard_methods;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingZlema {
    period: usize,
    lag: usize,
    close_buf: Vec<f64>,
    close_head: usize,
    close_len: usize,
    ema: StreamingEma,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingZlema {
    pub fn new(period: usize) -> Self {
        let lag = (period - 1) / 2;
        Self {
            period,
            lag,
            close_buf: vec![0.0; lag + 1],
            close_head: 0,
            close_len: 0,
            ema: StreamingEma::new(period),
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    fn ring_close(&self, i: usize) -> f64 {
        self.close_buf[(self.close_head + i) % (self.lag + 1)]
    }

    #[inline]
    fn push_close(&mut self, close: f64) {
        let cap = self.lag + 1;
        if self.close_len < cap {
            self.close_buf[(self.close_head + self.close_len) % cap] = close;
            self.close_len += 1;
        } else {
            self.close_buf[self.close_head] = close;
            self.close_head = (self.close_head + 1) % cap;
        }
    }
}

impl StreamingIndicator for StreamingZlema {
    #[inline]
    fn next(&mut self, close: f64) -> Option<f64> {
        self.count += 1;
        self.push_close(close);

        if self.close_len <= self.lag {
            self.last_value = None;
            return None;
        }

        let lagged = self.ring_close(0);
        let adjusted = 2.0 * close - lagged;
        let result = self.ema.next(adjusted);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.close_head = 0;
        self.close_len = 0;
        self.ema.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.ema.is_ready()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingZlema {
    fn name() -> &'static str {
        "ZLEMA"
    }

    fn category() -> &'static str {
        "overlap"
    }

    fn description() -> &'static str {
        "Zero Lag EMA"
    }

    fn warm_up_period(&self) -> usize {
        self.period + self.lag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_zlema_basic() {
        let mut zlema = StreamingZlema::new(10);
        let data: Vec<f64> = (0..30).map(|i| 100.0 + i as f64).collect();
        let mut last = None;
        for &val in &data {
            last = zlema.next(val);
        }
        assert!(last.is_some());
    }

    #[test]
    fn test_streaming_zlema_meta() {
        let zlema = StreamingZlema::new(10);
        assert_eq!(StreamingZlema::name(), "ZLEMA");
        assert_eq!(StreamingZlema::category(), "overlap");
        assert_eq!(zlema.warm_up_period(), 14);
    }

    #[test]
    fn test_streaming_zlema_reset() {
        let mut zlema = StreamingZlema::new(10);
        for i in 0..20 {
            zlema.next(i as f64 + 100.0);
        }
        assert!(zlema.is_ready());
        zlema.reset();
        assert!(!zlema.is_ready());
        assert_eq!(zlema.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 100;
        let data: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let period = 10;

        let batch = crate::math::moving_avg::zlema(&data, period).unwrap();
        let mut streaming = StreamingZlema::new(period);

        for (i, &val) in data.iter().enumerate() {
            if let Some(s) = streaming.next(val) {
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
