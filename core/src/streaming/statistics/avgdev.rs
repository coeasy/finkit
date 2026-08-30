//! Streaming Mean Absolute Deviation (AVGDEV).
//!
//! AVGDEV_i = mean(|x_{i-k} - mean(window)|) for k in [0, period)
//!
//! Complexity: O(1) amortised for the rolling-sum update of the window mean,
//! but O(period) for the per-step `sum |x - mean|` term. For typical
//! `period` values (<= 200) this is fast enough on every bar; a strict
//! O(1) version is non-trivial because the abs-deviation sum cannot be
//! maintained incrementally with only the rolling mean.

use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};
use crate::impl_standard_methods;

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingAvgdev {
    period: usize,
    inv_period: f64,
    buf: Vec<f64>,
    head: usize,
    len: usize,
    sum: f64,
    count: usize,
    last_value: Option<f64>,
    #[cfg_attr(feature = "serde", serde(skip))]
    snapshot: Option<SnapshotState>,
    last_open_time: i64,
}

#[derive(Clone, Copy)]
struct SnapshotState {
    head: usize,
    len: usize,
    sum: f64,
    count: usize,
    last_value: Option<f64>,
    last_open_time: i64,
    head_val: f64,
}

impl StreamingAvgdev {
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "AVGDEV period must be > 0");
        Self {
            period,
            inv_period: 1.0 / period as f64,
            buf: vec![0.0; period],
            head: 0,
            len: 0,
            sum: 0.0,
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
                self.head = snap.head;
                self.len = snap.len;
                self.sum = snap.sum;
                self.count = snap.count;
                self.last_value = snap.last_value;
                self.last_open_time = snap.last_open_time;
                self.buf[snap.head] = snap.head_val;
            }
        }
        self.snapshot = Some(SnapshotState {
            head: self.head,
            len: self.len,
            sum: self.sum,
            count: self.count,
            last_value: self.last_value,
            last_open_time: self.last_open_time,
            head_val: self.buf[self.head],
        });
        self.last_open_time = t;
        self.next(bar.close())
    }
}

impl StreamingIndicator for StreamingAvgdev {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        if self.period == 1 {
            self.last_value = Some(0.0);
            return Some(0.0);
        }

        if self.len == self.period {
            // Evict oldest
            self.sum -= self.buf[self.head];
        } else {
            self.len += 1;
        }
        self.sum += input;
        self.buf[self.head] = input;
        self.head += 1;
        if self.head == self.period {
            self.head = 0;
        }

        if self.len < self.period {
            self.last_value = None;
            return None;
        }

        let mean = self.sum * self.inv_period;
        // O(period) deviation sum. The buffer is already in logical order
        // thanks to the head-relative indexing; we walk all `period` slots.
        let mut dev_sum = 0.0;
        for i in 0..self.period {
            // logical position i maps to (head + i) % period once full
            let idx = if self.period == self.buf.len() {
                (self.head + i) % self.period
            } else {
                // defensive: if buffer length is period, head arithmetic is correct
                i
            };
            dev_sum += (self.buf[idx] - mean).abs();
        }
        let result = Some(dev_sum * self.inv_period);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.sum = 0.0;
        self.count = 0;
        self.last_value = None;
        self.snapshot = None;
        self.last_open_time = 0;
    }

    fn is_ready(&self) -> bool {
        self.period == 1 || self.len == self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingAvgdev {
    fn name() -> &'static str {
        "AVGDEV"
    }
    fn category() -> &'static str {
        "statistics"
    }
    fn description() -> &'static str {
        "Mean Absolute Deviation (O(period) per bar)"
    }
    fn warm_up_period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_avgdev_basic() {
        let mut a = StreamingAvgdev::new(5);
        // 1..=5: 1, 2, 3, 4, 5 -> mean=3, dev_sum=2+1+0+1+2=6, avgdev=1.2
        for i in 0..4 {
            assert_eq!(a.next(i as f64 + 1.0), None);
        }
        let v = a.next(5.0).unwrap();
        assert!((v - 1.2).abs() < 1e-10, "got {v}");
    }

    #[test]
    fn test_streaming_avgdev_meta() {
        let a = StreamingAvgdev::new(14);
        assert_eq!(StreamingAvgdev::name(), "AVGDEV");
        assert_eq!(StreamingAvgdev::category(), "statistics");
        assert_eq!(a.warm_up_period(), 14);
    }

    #[test]
    fn test_streaming_avgdev_period_one() {
        let mut a = StreamingAvgdev::new(1);
        for i in 1..=5 {
            assert_eq!(a.next(i as f64), Some(0.0));
        }
    }

    #[test]
    fn test_streaming_avgdev_rolling() {
        // The data [1, 2, 3, 4, 5, 6, 7] is linear, so every window
        // has the same mean and same mean absolute deviation (1.2 for size 5).
        let mut a = StreamingAvgdev::new(5);
        let data = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let mut out = Vec::new();
        for &v in &data {
            out.push(a.next(v));
        }
        // First 4 are None
        for i in 0..4 {
            assert!(out[i].is_none());
        }
        for i in 4..data.len() {
            let v = out[i].unwrap();
            assert!((v - 1.2).abs() < 1e-10, "mismatch at {i}: {v}");
        }
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 8.0)
            .collect();
        let period = 14;
        let batch = crate::indicators::statistics::avgdev(&data, period).unwrap();
        let mut streaming = StreamingAvgdev::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                assert!(
                    (s - batch[i]).abs() < 1e-10,
                    "AVGDEV mismatch at {i}: streaming={s}, batch={}",
                    batch[i]
                );
            }
        }
    }
}
