use crate::streaming::rolling_minmax::RollingMax;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingUlcerIndex {
    period: usize,
    rolling_max: RollingMax,
    /// Ring buffer of close prices in the current window.
    close_buf: Vec<f64>,
    close_head: usize,
    close_len: usize,
    /// Sum of squared percentage drawdowns in the current window.
    dd_sum: f64,
    prev_max: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingUlcerIndex {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            rolling_max: RollingMax::new(),
            close_buf: vec![0.0; period],
            close_head: 0,
            close_len: 0,
            dd_sum: 0.0,
            prev_max: 0.0,
            count: 0,
            last_value: None,
        }
    }

    /// Recompute `dd_sum` from the current `close_buf` using `max` as the
    /// rolling maximum. Called whenever the rolling maximum changes so that
    /// previously-stored drawdowns are re-evaluated against the new max.
    fn recompute_dd_sum(&mut self, max: f64) {
        self.dd_sum = 0.0;
        for k in 0..self.close_len {
            let close = self.close_buf[(self.close_head + k) % self.period];
            let pct = 100.0 * (close - max) / max;
            self.dd_sum += pct * pct;
        }
    }
}

impl StreamingIndicator for StreamingUlcerIndex {
    #[inline]
    fn next(&mut self, close: f64) -> Option<f64> {
        self.count += 1;

        // Push new close to rolling max
        self.rolling_max.push(self.count, close);

        // Pop expired entries (keep only last `period` bars)
        if self.count > self.period {
            self.rolling_max.pop(self.count - self.period);
        }

        // Only compute drawdown when close window is full
        if self.count < self.period {
            self.last_value = None;
            return None;
        }

        let max = match self.rolling_max.current() {
            Some(v) if v > 0.0 && v.is_finite() => v,
            _ => {
                self.last_value = None;
                return None;
            }
        };

        if self.close_len < self.period {
            // First fill phase: append the close, recompute dd_sum from
            // scratch (the rolling max may have changed during the fill).
            self.close_buf[(self.close_head + self.close_len) % self.period] = close;
            self.close_len += 1;
            self.recompute_dd_sum(max);
        } else {
            let old_close = self.close_buf[self.close_head];
            self.close_head = (self.close_head + 1) % self.period;
            let new_slot = (self.close_head + self.period - 1) % self.period;
            self.close_buf[new_slot] = close;

            if max == self.prev_max {
                let old_pct = 100.0 * (old_close - max) / max;
                let new_pct = 100.0 * (close - max) / max;
                self.dd_sum = self.dd_sum - old_pct * old_pct + new_pct * new_pct;
            } else {
                self.recompute_dd_sum(max);
            }
            self.prev_max = max;
        }

        let result = if self.close_len == self.period {
            Some((self.dd_sum / self.period as f64).sqrt())
        } else {
            None
        };
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.rolling_max.reset();
        self.close_head = 0;
        self.close_len = 0;
        self.dd_sum = 0.0;
        self.prev_max = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.close_len >= self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingUlcerIndex {
    fn name() -> &'static str {
        "Ulcer Index"
    }

    fn category() -> &'static str {
        "volatility"
    }

    fn description() -> &'static str {
        "Ulcer Index"
    }

    fn warm_up_period(&self) -> usize {
        2 * self.period - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_ulcer_index_basic() {
        let mut ui = StreamingUlcerIndex::new(5);
        let data: Vec<f64> = (0..30).map(|i| 100.0 + (i as f64 * 0.3).sin() * 5.0).collect();
        let mut last = None;
        for &val in &data {
            last = ui.next(val);
        }
        assert!(last.is_some());
        assert!(last.unwrap() >= 0.0);
    }

    #[test]
    fn test_streaming_ulcer_index_meta() {
        let ui = StreamingUlcerIndex::new(14);
        assert_eq!(StreamingUlcerIndex::name(), "Ulcer Index");
        assert_eq!(StreamingUlcerIndex::category(), "volatility");
        assert_eq!(ui.warm_up_period(), 27);
    }

    #[test]
    fn test_streaming_ulcer_index_reset() {
        let mut ui = StreamingUlcerIndex::new(5);
        for i in 0..20 {
            ui.next(i as f64 + 100.0);
        }
        assert!(ui.is_ready());
        ui.reset();
        assert!(!ui.is_ready());
        assert_eq!(ui.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 100;
        let data: Vec<f64> = (0..n)
            .map(|i| 100.0 + (i as f64 * 0.15).sin() * 10.0 + i as f64 * 0.02)
            .collect();
        let period = 14;

        let batch = crate::indicators::volatility_ext::ulcer_index(&data, period).unwrap();
        let mut streaming = StreamingUlcerIndex::new(period);

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
