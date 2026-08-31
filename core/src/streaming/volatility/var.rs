use std::collections::VecDeque;

use crate::impl_indicator_meta;
use crate::impl_standard_methods;
use crate::streaming::traits::StreamingIndicator;

/// Streaming Variance using Welford's online algorithm with rolling window.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingVar {
    period: usize,
    window: VecDeque<f64>,
    count: usize,
    mean: f64,
    m2: f64,
    last_value: Option<f64>,
}

impl StreamingVar {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            window: VecDeque::with_capacity(period),
            count: 0,
            mean: 0.0,
            m2: 0.0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingVar {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;

        if self.window.len() == self.period {
            let old = self.window.pop_front().unwrap();
            let old_mean = self.mean;
            self.mean += (input - old) / self.period as f64;
            self.m2 +=
                (input - self.mean) * (input - old_mean) - (old - self.mean) * (old - old_mean);
            if self.m2 < 0.0 {
                self.m2 = 0.0;
            }
        } else {
            let n = self.window.len() as f64 + 1.0;
            let delta = input - self.mean;
            self.mean += delta / n;
            self.m2 += delta * (input - self.mean);
        }
        self.window.push_back(input);

        let result = if self.window.len() == self.period {
            let n = self.period as f64;
            Some((self.m2 / (n - 1.0)).max(0.0))
        } else {
            None
        };
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.window.clear();
        self.count = 0;
        self.mean = 0.0;
        self.m2 = 0.0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.window.len() >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(
    StreamingVar,
    "VAR",
    "statistic",
    "Rolling Variance (Welford)"
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;

    #[test]
    fn test_streaming_var_basic() {
        let mut v = StreamingVar::new(3);
        assert_eq!(v.next(2.0), None);
        assert_eq!(v.next(4.0), None);
        let val = v.next(6.0).unwrap();
        assert!((val - 4.0).abs() < 1e-10); // var([2,4,6]) = 4.0
    }

    #[test]
    fn test_streaming_var_meta() {
        assert_eq!(StreamingVar::name(), "VAR");
        assert_eq!(StreamingVar::category(), "statistic");
    }

    #[test]
    fn test_streaming_var_reset() {
        let mut v = StreamingVar::new(3);
        v.next(1.0);
        v.next(2.0);
        v.next(3.0);
        assert!(v.is_ready());
        v.reset();
        assert!(!v.is_ready());
    }

    #[test]
    fn test_streaming_var_welford_stability() {
        let mut v = StreamingVar::new(100);
        let base = 1e6;
        for i in 0..200 {
            let val = base + (i as f64 * 0.01).sin() * 0.001;
            if let Some(s) = v.next(val) {
                assert!(s.is_finite(), "NaN/Inf at i={i}");
                assert!(s >= 0.0, "Negative variance at i={i}");
            }
        }
    }
}
