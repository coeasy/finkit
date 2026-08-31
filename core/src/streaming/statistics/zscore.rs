use crate::impl_indicator_meta;
use crate::impl_standard_methods;
use crate::streaming::traits::StreamingIndicator;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingZscore {
    period: usize,
    buf: Vec<f64>,
    head: usize,
    len: usize,
    sum: f64,
    sum_sq: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingZscore {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buf: vec![0.0; period],
            head: 0,
            len: 0,
            sum: 0.0,
            sum_sq: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingZscore {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let cap = self.period;

        self.sum += input;
        self.sum_sq += input * input;

        if self.len < cap {
            self.buf[(self.head + self.len) % cap] = input;
            self.len += 1;
        } else {
            let old = self.buf[self.head];
            self.sum -= old;
            self.sum_sq -= old * old;
            self.buf[self.head] = input;
            self.head = (self.head + 1) % cap;
        }

        if self.len < self.period {
            self.last_value = None;
            return None;
        }

        let n = self.period as f64;
        let mean = self.sum / n;
        let variance = (self.sum_sq / n - mean * mean) * n / (n - 1.0);
        if variance <= 1e-30 {
            self.last_value = None;
            return None;
        }
        let std_dev = variance.sqrt();
        let result = Some((input - mean) / std_dev);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.sum = 0.0;
        self.sum_sq = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.len >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingZscore, "ZSCORE", "statistics", "Rolling Z-Score");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;

    #[test]
    fn test_streaming_zscore_basic() {
        let mut zscore = StreamingZscore::new(3);
        assert_eq!(zscore.next(1.0), None);
        assert_eq!(zscore.next(2.0), None);
        let v = zscore.next(3.0).unwrap();
        assert!(v.is_finite());
    }

    #[test]
    fn test_streaming_zscore_reset() {
        let mut zscore = StreamingZscore::new(5);
        for i in 0..10 {
            zscore.next(i as f64 + 1.0);
        }
        assert!(zscore.is_ready());
        zscore.reset();
        assert!(!zscore.is_ready());
        assert_eq!(zscore.count(), 0);
    }

    #[test]
    fn test_streaming_zscore_meta() {
        let zscore = StreamingZscore::new(20);
        assert_eq!(StreamingZscore::name(), "ZSCORE");
        assert_eq!(StreamingZscore::category(), "statistics");
        assert_eq!(zscore.warm_up_period(), 20);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..50)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 10.0)
            .collect();
        let period = 5;
        let batch = crate::indicators::zscore(&data, period).unwrap();
        let mut streaming = StreamingZscore::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                assert!((s - batch[i]).abs() < 1e-10, "Mismatch at {i}");
            }
        }
    }
}
