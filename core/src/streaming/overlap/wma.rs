use crate::streaming::traits::StreamingIndicator;
use crate::{impl_indicator_meta, impl_standard_methods};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingWma {
    period: usize,
    inv_denom: f64,
    buf: Vec<f64>,
    head: usize,
    len: usize,
    count: usize,
    weighted_sum: f64,
    window_sum: f64,
    last_value: Option<f64>,
}

impl StreamingWma {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            inv_denom: 2.0 / (period * (period + 1)) as f64,
            buf: vec![0.0; period],
            head: 0,
            len: 0,
            count: 0,
            weighted_sum: 0.0,
            window_sum: 0.0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingWma {
    #[inline]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip(self, input))
    )]
    fn next(&mut self, input: f64) -> Option<f64> {
        crate::streaming_measure!("wma", self.count, {
            self.count += 1;
            let cap = self.period;

            if self.len < cap {
                // Warm-up: window grows, newest value always carries the
                // current max weight (len + 1). O(1) incremental update.
                let new_weight = (self.len + 1) as f64;
                self.buf[(self.head + self.len) % cap] = input;
                self.len += 1;
                self.weighted_sum += input * new_weight;
                self.window_sum += input;
            } else {
                // Steady state: evict oldest, shift weights up by one, newest
                // carries weight `period`. Mirrors `wma_inner` batch update.
                let old = self.buf[self.head];
                self.weighted_sum = self.weighted_sum + (cap as f64) * input - self.window_sum;
                self.window_sum = self.window_sum + input - old;
                self.buf[self.head] = input;
                self.head = (self.head + 1) % cap;
            }

            if self.len < self.period {
                self.last_value = None;
                return None;
            }

            let result = Some(self.weighted_sum * self.inv_denom);
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.count = 0;
        self.weighted_sum = 0.0;
        self.window_sum = 0.0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.len >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingWma, "WMA", "overlap", "Weighted Moving Average");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;

    #[test]
    fn test_streaming_wma_basic() {
        let mut wma = StreamingWma::new(3);
        assert_eq!(wma.next(1.0), None);
        assert_eq!(wma.next(2.0), None);
        let v = wma.next(3.0).unwrap();
        // WMA = (1*1 + 2*2 + 3*3) / 6 = 14/6
        assert!((v - 14.0 / 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_wma_meta() {
        assert_eq!(StreamingWma::name(), "WMA");
        assert_eq!(StreamingWma::category(), "overlap");
    }

    #[test]
    fn test_streaming_wma_reset() {
        let mut wma = StreamingWma::new(3);
        for i in 0..5 {
            wma.next(i as f64);
        }
        assert!(wma.is_ready());
        wma.reset();
        assert!(!wma.is_ready());
        assert_eq!(wma.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..100)
            .map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0)
            .collect();
        let period = 10;
        let batch = crate::math::moving_avg::wma(&data, period).unwrap();
        let mut streaming = StreamingWma::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                assert!(
                    (s - batch[i]).abs() < 1e-10,
                    "Mismatch at {i}: s={s}, b={}",
                    batch[i]
                );
            }
        }
    }
}
