//! Streaming Triangular Moving Average (TRIMA).
//!
//! TRIMA is the average of an SMA over a second SMA. For an even period, the
//! two SMA windows are `(period/2 + 1)` and `(period/2)`. For an odd period,
//! both windows are `period.div_ceil(2)`.
//!
//! Implementation: feed each value into an inner `StreamingSma` of period
//! `first_period`; whenever that SMA emits a value, feed it into an outer
//! `StreamingSma` of period `second_period`. The outer SMA's output is the
//! TRIMA. Each individual SMA is O(1) per call, so the composed indicator is
//! also O(1) per input.

use crate::streaming::overlap::sma::StreamingSma;
use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};
use crate::impl_standard_methods;

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingTrima {
    period: usize,
    first_period: usize,
    second_period: usize,
    inner: StreamingSma,
    outer: StreamingSma,
    count: usize,
    last_value: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip))]
    snapshot: Option<SnapshotState>,
    last_open_time: i64,
}

#[derive(Clone, Copy)]
struct SnapshotState {
    inner: super::sma::SmaSnapshot,
    outer: super::sma::SmaSnapshot,
    count: usize,
    last_value: Option<f64>,
    last_open_time: i64,
}

impl StreamingTrima {
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "TRIMA period must be > 0");
        let (first_period, second_period) = if period % 2 == 1 {
            let half = period.div_ceil(2);
            (half, half)
        } else {
            (period / 2 + 1, period / 2)
        };
        Self {
            period,
            first_period,
            second_period,
            inner: StreamingSma::new(first_period),
            outer: StreamingSma::new(second_period),
            count: 0,
            last_value: None,
            snapshot: None,
            last_open_time: 0,
        }
    }

    pub fn compute_bar(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        let t = bar.open_time();
        if t != 0 && t == self.last_open_time {
            if let Some(snap) = self.snapshot.take() {
                self.inner.restore(snap.inner);
                self.outer.restore(snap.outer);
                self.count = snap.count;
                self.last_value = snap.last_value;
                self.last_open_time = snap.last_open_time;
            }
        }
        self.snapshot = Some(SnapshotState {
            inner: self.inner.snapshot(),
            outer: self.outer.snapshot(),
            count: self.count,
            last_value: self.last_value,
            last_open_time: self.last_open_time,
        });
        self.last_open_time = t;
        self.next(bar.close())
    }
}

impl StreamingIndicator for StreamingTrima {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let inner_val = self.inner.next(input);
        let result = match inner_val {
            Some(v) => self.outer.next(v),
            None => {
                // Inner not ready, outer should not advance
                None
            }
        };
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.inner.reset();
        self.outer.reset();
        self.count = 0;
        self.last_value = None;
        self.snapshot = None;
        self.last_open_time = 0;
    }

    fn is_ready(&self) -> bool {
        self.outer.is_ready()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingTrima {
    fn name() -> &'static str {
        "TRIMA"
    }
    fn category() -> &'static str {
        "overlap"
    }
    fn description() -> &'static str {
        "Triangular Moving Average"
    }
    fn warm_up_period(&self) -> usize {
        self.first_period + self.second_period - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_trima_basic() {
        let mut trima = StreamingTrima::new(5);
        for i in 1..=10 {
            let v = trima.next(i as f64);
            if trima.is_ready() {
                assert!(v.is_some());
                let val = v.unwrap();
                assert!(!val.is_nan());
            }
        }
    }

    #[test]
    fn test_streaming_trima_meta() {
        let t = StreamingTrima::new(5);
        assert_eq!(StreamingTrima::name(), "TRIMA");
        assert_eq!(StreamingTrima::category(), "overlap");
        // period 5 odd �?first=3, second=3 �?warmup = 3+3-1 = 5
        assert_eq!(t.warm_up_period(), 5);
    }

    #[test]
    fn test_streaming_trima_reset() {
        let mut trima = StreamingTrima::new(4);
        for i in 0..20 {
            trima.next(i as f64);
        }
        assert!(trima.is_ready());
        trima.reset();
        assert!(!trima.is_ready());
        assert_eq!(trima.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..120)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let period = 7;
        let batch_result = crate::math::moving_avg::trima(&data, period).unwrap();

        let mut streaming = StreamingTrima::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch_result[i].is_nan()) {
                assert!(
                    (s - batch_result[i]).abs() < 1e-9,
                    "TRIMA mismatch at index {i}: streaming={s}, batch={}",
                    batch_result[i]
                );
            }
        }
    }
}
