use crate::streaming::traits::{StreamingIndicator};
use crate::impl_standard_methods;
use crate::{impl_indicator_meta};

/// Streaming Linear Regression Intercept
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingLinRegIntercept {
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

impl StreamingLinRegIntercept {
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

    fn compute_intercept(&self) -> Option<f64> {
        let n = self.period as f64;
        let denom = n * self.sum_x2 - self.sum_x * self.sum_x;
        if denom.abs() < 1e-15 { return None; }

        let slope = (n * self.sum_xy - self.sum_x * self.sum_y) / denom;
        Some((self.sum_y - slope * self.sum_x) / n)
    }
}

impl StreamingIndicator for StreamingLinRegIntercept {
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

        let result = if self.len == self.period { self.compute_intercept() } else { None };
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

impl_indicator_meta!(StreamingLinRegIntercept, "LinRegIntercept", "statistic", "Linear Regression Intercept");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;

    #[test]
    fn test_streaming_linreg_intercept_basic() {
        let mut s = StreamingLinRegIntercept::new(3);
        assert_eq!(s.next(1.0), None);
        assert_eq!(s.next(2.0), None);
        let v = s.next(3.0).unwrap();
        assert!((v - 1.0).abs() < 1e-10); // intercept of [1,2,3] with x=[0,1,2] = 1.0
    }

    #[test]
    fn test_streaming_linreg_intercept_meta() {
        assert_eq!(StreamingLinRegIntercept::name(), "LinRegIntercept");
    }

    #[test]
    fn test_streaming_linreg_intercept_reset() {
        let mut s = StreamingLinRegIntercept::new(3);
        s.next(1.0); s.next(2.0); s.next(3.0);
        assert!(s.is_ready());
        s.reset();
        assert!(!s.is_ready());
    }

    #[test]
    fn test_streaming_linreg_intercept_constant() {
        let mut s = StreamingLinRegIntercept::new(5);
        for _ in 0..10 {
            if let Some(v) = s.next(42.0) {
                assert!((v - 42.0).abs() < 1e-10);
            }
        }
    }
}
