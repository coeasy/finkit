use crate::impl_indicator_meta;
use crate::impl_standard_methods;
use crate::streaming::traits::StreamingIndicator;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingLinReg {
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

impl StreamingLinReg {
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

    fn compute_regression(&self) -> Option<f64> {
        let n = self.period as f64;
        let denom = n * self.sum_x2 - self.sum_x * self.sum_x;
        if denom.abs() < 1e-15 {
            return None;
        }

        let slope = (n * self.sum_xy - self.sum_x * self.sum_y) / denom;
        let intercept = (self.sum_y - slope * self.sum_x) / n;
        Some(intercept + slope * (n - 1.0))
    }
}

impl StreamingIndicator for StreamingLinReg {
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

        let result = if self.len == self.period {
            self.compute_regression()
        } else {
            None
        };
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.count = 0;
        self.last_value = None;
        self.sum_y = 0.0;
        self.sum_xy = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.len >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(
    StreamingLinReg,
    "LINEARREG",
    "statistics",
    "Rolling Linear Regression"
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;

    #[test]
    fn test_streaming_linreg_basic() {
        let mut linreg = StreamingLinReg::new(3);
        assert_eq!(linreg.next(1.0), None);
        assert_eq!(linreg.next(2.0), None);
        let v = linreg.next(3.0).unwrap();
        assert!(v.is_finite());
    }

    #[test]
    fn test_streaming_linreg_reset() {
        let mut linreg = StreamingLinReg::new(3);
        for i in 0..10 {
            linreg.next(i as f64);
        }
        assert!(linreg.is_ready());
        linreg.reset();
        assert!(!linreg.is_ready());
        assert_eq!(linreg.count(), 0);
    }

    #[test]
    fn test_streaming_linreg_meta() {
        let linreg = StreamingLinReg::new(14);
        assert_eq!(StreamingLinReg::name(), "LINEARREG");
        assert_eq!(StreamingLinReg::category(), "statistics");
        assert_eq!(linreg.warm_up_period(), 14);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let data: Vec<f64> = (0..50)
            .map(|i| 50.0 + (i as f64 * 0.2).sin() * 10.0)
            .collect();
        let period = 5;
        let batch = crate::math::linear::linreg(&data, period).unwrap();
        let mut streaming = StreamingLinReg::new(period);
        for (i, &val) in data.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(val), batch[i].is_nan()) {
                assert!((s - batch[i]).abs() < 1e-10, "Mismatch at {i}");
            }
        }
    }
}
