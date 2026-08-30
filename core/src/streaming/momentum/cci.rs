use crate::streaming::traits::{StreamingIndicator};
use crate::impl_standard_methods;
use crate::{impl_indicator_meta};

/// Streaming Commodity Channel Index (CCI).
///
/// CCI = (TP - SMA(TP, period)) / (0.015 * Mean Deviation)
/// where TP = (High + Low + Close) / 3
///
/// Input: (high, low, close) tuple per bar.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingCci {
    period: usize,
    tp_buf: Vec<f64>,
    head: usize,
    len: usize,
    sum: f64,
    mean_dev_sum: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingCci {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            tp_buf: vec![0.0; period],
            head: 0,
            len: 0,
            sum: 0.0,
            mean_dev_sum: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64, f64)> for StreamingCci {
    #[inline]
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip(self, input)))]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<f64> {
        crate::streaming_measure!("cci", self.count, {
            let (high, low, close) = input;
            self.count += 1;

            let tp = (high + low + close) / 3.0;
            let cap = self.period;

            if self.len < cap {
                // Window not yet full —just accumulate
                self.tp_buf[(self.head + self.len) % cap] = tp;
                self.sum += tp;
                self.len += 1;

                if self.len < self.period {
                    self.last_value = None;
                    return None;
                }

                // Window just became full —compute mean_dev_sum from scratch
                let tp_mean = self.sum / self.period as f64;
                self.mean_dev_sum = 0.0;
                for &v in &self.tp_buf {
                    self.mean_dev_sum += (v - tp_mean).abs();
                }
            } else {
                // Window is full —incremental update with exact mean-shift correction
                let old_mean = self.sum / self.period as f64;
                let old = self.tp_buf[self.head];

                // 1. Remove evicted element's deviation from old mean
                self.mean_dev_sum -= (old - old_mean).abs();

                // 2. Update sum and buffer
                self.sum += tp - old;
                let old_head = self.head;
                self.tp_buf[old_head] = tp;
                self.head = (self.head + 1) % cap;

                let new_mean = self.sum / self.period as f64;

                // 3. Correct remaining elements for mean shift.
                //    For each remaining element, its deviation changes from
                //    |v - old_mean| to |v - new_mean|. We compute the exact
                //    difference for each element.
                for i in 0..cap {
                    if i == old_head {
                        continue; // skip the new element (handled in step 4)
                    }
                    let v = self.tp_buf[i];
                    self.mean_dev_sum += (v - new_mean).abs() - (v - old_mean).abs();
                }

                // 4. Add new element's deviation from new mean
                self.mean_dev_sum += (tp - new_mean).abs();
            }

            let tp_mean = self.sum / self.period as f64;
            let mean_dev = self.mean_dev_sum / self.period as f64;

            let result = if mean_dev.abs() < 1e-15 {
                Some(0.0)
            } else {
                Some((tp - tp_mean) / (0.015 * mean_dev))
            };
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.sum = 0.0;
        self.mean_dev_sum = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.len >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingCci, "CCI", "momentum", "Commodity Channel Index");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;

    #[test]
    fn test_streaming_cci_basic() {
        let mut cci = StreamingCci::new(14);
        let data: Vec<(f64, f64, f64)> = (0..30)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.3).sin() * 10.0;
                let l = h - 3.0;
                let c = h - 1.5;
                (h, l, c)
            })
            .collect();

        let mut last = None;
        for &d in &data {
            last = cci.next(d);
        }
        assert!(last.is_some());
    }

    #[test]
    fn test_streaming_cci_flat_market() {
        let mut cci = StreamingCci::new(5);
        for _ in 0..10 {
            let val = cci.next((100.0, 100.0, 100.0));
            if cci.is_ready() {
                assert_eq!(val, Some(0.0));
            }
        }
    }

    #[test]
    fn test_streaming_cci_reset() {
        let mut cci = StreamingCci::new(5);
        for i in 0..10 {
            cci.next((50.0 + i as f64, 45.0 + i as f64, 47.0 + i as f64));
        }
        assert!(cci.is_ready());
        cci.reset();
        assert!(!cci.is_ready());
        assert_eq!(cci.count(), 0);
    }

    #[test]
    fn test_streaming_cci_meta() {
        let cci = StreamingCci::new(20);
        assert_eq!(StreamingCci::name(), "CCI");
        assert_eq!(StreamingCci::category(), "momentum");
        assert_eq!(cci.warm_up_period(), 20);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 100;
        let high: Vec<f64> = (0..n)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 10.0)
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 3.0).collect();
        let close: Vec<f64> = high.iter().zip(low.iter()).map(|(h, l)| (h + l) / 2.0).collect();
        let period = 14;

        let batch = crate::indicators::momentum::cci(&high, &low, &close, period).unwrap();

        let mut streaming = StreamingCci::new(period);
        for i in 0..n {
            if let (Some(s), false) =
                (streaming.next((high[i], low[i], close[i])), batch[i].is_nan())
            {
                assert!(
                    (s - batch[i]).abs() < 1e-10,
                    "CCI mismatch at {i}: streaming={s}, batch={}",
                    batch[i]
                );
            }
        }
    }
}
