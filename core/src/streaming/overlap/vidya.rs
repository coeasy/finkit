use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingVidya {
    period: usize,
    cmo_period: usize,
    sc: f64,
    start: usize,
    close_buf: Vec<f64>,
    close_head: usize,
    close_len: usize,
    base_idx: usize,
    sum_up: f64,
    sum_down: f64,
    prev_close: f64,
    vidya: f64,
    initialized: bool,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingVidya {
    pub fn new(period: usize, cmo_period: usize) -> Self {
        Self {
            period,
            cmo_period,
            sc: 2.0 / (period as f64 + 1.0),
            start: cmo_period.max(period - 1),
            close_buf: vec![0.0; cmo_period + 2],
            close_head: 0,
            close_len: 0,
            base_idx: 0,
            sum_up: 0.0,
            sum_down: 0.0,
            prev_close: f64::NAN,
            vidya: f64::NAN,
            initialized: false,
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    fn cap(&self) -> usize {
        self.cmo_period + 2
    }

    #[inline]
    fn push_close(&mut self, close: f64) {
        let cap = self.cap();
        if self.close_len < cap {
            self.close_buf[(self.close_head + self.close_len) % cap] = close;
            self.close_len += 1;
        } else {
            self.close_buf[self.close_head] = close;
            self.close_head = (self.close_head + 1) % cap;
            self.base_idx += 1;
        }
    }

    #[inline]
    fn close_at(&self, idx: usize) -> f64 {
        self.close_buf[(idx - self.base_idx + self.close_head) % self.cap()]
    }

    #[inline]
    fn add_change(&mut self, change: f64) {
        if change > 0.0 {
            self.sum_up += change;
        } else {
            self.sum_down -= change;
        }
    }

    #[inline]
    fn remove_change(&mut self, change: f64) {
        if change > 0.0 {
            self.sum_up -= change;
        } else {
            self.sum_down += change;
        }
    }

    #[inline]
    fn init_cmo_sums(&mut self, start: usize) {
        self.sum_up = 0.0;
        self.sum_down = 0.0;
        for j in start - self.cmo_period + 1..=start {
            let change = self.close_at(j) - self.close_at(j - 1);
            self.add_change(change);
        }
    }

    #[inline]
    fn cmo_factor(&self) -> f64 {
        let denom = self.sum_up + self.sum_down;
        if denom.abs() <= 1e-15 {
            0.0
        } else {
            ((self.sum_up - self.sum_down) / denom).abs()
        }
    }
}

impl StreamingIndicator for StreamingVidya {
    #[inline]
    fn next(&mut self, close: f64) -> Option<f64> {
        self.count += 1;
        let idx = self.count - 1;
        self.push_close(close);

        if self.count == 1 {
            self.prev_close = close;
            self.last_value = None;
            return None;
        }

        if !self.initialized {
            if idx < self.start {
                self.prev_close = close;
                self.last_value = None;
                return None;
            }
            self.init_cmo_sums(idx);
            self.vidya = close;
            self.initialized = true;
            self.prev_close = close;
            let result = Some(close);
            self.last_value = result;
            return result;
        }

        let entering_change = close - self.prev_close;
        self.prev_close = close;
        self.add_change(entering_change);

        let leaving_idx = idx - self.cmo_period;
        let leaving_change = self.close_at(leaving_idx + 1) - self.close_at(leaving_idx);
        self.remove_change(leaving_change);

        let alpha = self.sc * self.cmo_factor();
        self.vidya = alpha * close + (1.0 - alpha) * self.vidya;
        let result = Some(self.vidya);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.close_head = 0;
        self.close_len = 0;
        self.base_idx = 0;
        self.sum_up = 0.0;
        self.sum_down = 0.0;
        self.prev_close = f64::NAN;
        self.vidya = f64::NAN;
        self.initialized = false;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.initialized
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingVidya {
    fn name() -> &'static str {
        "VIDYA"
    }

    fn category() -> &'static str {
        "overlap"
    }

    fn description() -> &'static str {
        "Variable Index Dynamic Average"
    }

    fn warm_up_period(&self) -> usize {
        self.period.max(self.cmo_period + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_vidya_basic() {
        let mut vidya = StreamingVidya::new(14, 9);
        let data: Vec<f64> = (0..40).map(|i| 100.0 + i as f64 * 0.5).collect();
        let mut last = None;
        for &val in &data {
            last = vidya.next(val);
        }
        assert!(last.is_some());
    }

    #[test]
    fn test_streaming_vidya_meta() {
        let vidya = StreamingVidya::new(14, 9);
        assert_eq!(StreamingVidya::name(), "VIDYA");
        assert_eq!(StreamingVidya::category(), "overlap");
        assert_eq!(vidya.warm_up_period(), 14);
    }

    #[test]
    fn test_streaming_vidya_reset() {
        let mut vidya = StreamingVidya::new(10, 5);
        for i in 0..30 {
            vidya.next(i as f64 + 100.0);
        }
        assert!(vidya.is_ready());
        vidya.reset();
        assert!(!vidya.is_ready());
        assert_eq!(vidya.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 100;
        let data: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let period = 14;
        let cmo_period = 9;

        let batch = crate::math::moving_avg::vidya(&data, period, cmo_period).unwrap();
        let mut streaming = StreamingVidya::new(period, cmo_period);

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
