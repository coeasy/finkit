use crate::streaming::traits::{StreamingIndicator};
use crate::impl_standard_methods;
use crate::{impl_indicator_meta};

/// Streaming Time Series Forecast
///
/// TSF = intercept + slope * period  (one step ahead of linreg)
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingTsf {
    period: usize,
    buf: Vec<f64>,
    head: usize,
    len: usize,
    count: usize,
    last_value: Option<f64>,
    sum_y: f64,
    sum_xy: f64,
    sum_x: f64,
    sum_x2: f64,
}

impl StreamingTsf {
    pub fn new(period: usize) -> Self {
        let n = period as f64;
        let sum_x = n * (n - 1.0) / 2.0;
        let sum_x2 = n * (n - 1.0) * (2.0 * n - 1.0) / 6.0;
        Self {
            period,
            buf: vec![0.0; period],
            head: 0,
            len: 0,
            count: 0,
            last_value: None,
            sum_y: 0.0,
            sum_xy: 0.0,
            sum_x,
            sum_x2,
        }
    }

    fn compute(&self) -> Option<f64> {
        let n = self.period as f64;
        let denom = n * self.sum_x2 - self.sum_x * self.sum_x;
        if denom.abs() < 1e-15 { return None; }

        let slope = (n * self.sum_xy - self.sum_x * self.sum_y) / denom;
        let intercept = (self.sum_y - slope * self.sum_x) / n;
        Some(intercept + slope * n)
    }
}

impl StreamingIndicator for StreamingTsf {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let cap = self.period;

        if self.len < cap {
            self.sum_y += input;
            self.sum_xy += self.len as f64 * input;
            self.buf[(self.head + self.len) % cap] = input;
            self.len += 1;
        } else {
            let old_y = self.buf[self.head];
            let n = cap as f64;
            self.sum_xy = self.sum_xy - self.sum_y + old_y + (n - 1.0) * input;
            self.sum_y = self.sum_y - old_y + input;
            self.buf[self.head] = input;
            self.head = (self.head + 1) % cap;
        }

        let result = if self.len == self.period { self.compute() } else { None };
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.buf.fill(0.0);
        self.head = 0;
        self.len = 0;
        self.count = 0;
        self.last_value = None;
        self.sum_y = 0.0;
        self.sum_xy = 0.0;
    }

    fn is_ready(&self) -> bool { self.len >= self.period }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingTsf, "TSF", "statistic", "Time Series Forecast");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;

    #[test]
    fn test_streaming_tsf_basic() {
        let mut tsf = StreamingTsf::new(3);
        assert_eq!(tsf.next(1.0), None);
        assert_eq!(tsf.next(2.0), None);
        let v = tsf.next(3.0).unwrap();
        // Perfect linear: slope=1, intercept=1, tsf = 1 + 1*3 = 4
        assert!((v - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_tsf_meta() {
        assert_eq!(StreamingTsf::name(), "TSF");
        assert_eq!(StreamingTsf::category(), "statistic");
    }

    #[test]
    fn test_streaming_tsf_reset() {
        let mut tsf = StreamingTsf::new(3);
        tsf.next(1.0); tsf.next(2.0); tsf.next(3.0);
        assert!(tsf.is_ready());
        tsf.reset();
        assert!(!tsf.is_ready());
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..100).map(|i| 50.0 + (i as f64 * 0.1).sin() * 10.0).collect();
        let period = 14;
        let batch = crate::indicators::tsf(&data, period).unwrap();

        let mut streaming = StreamingTsf::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                assert!(
                    (s - batch[i]).abs() < 1e-8,
                    "Mismatch at {i}: streaming={s}, batch={}",
                    batch[i]
                );
            }
        }
    }
}
