use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;
use std::collections::VecDeque;

/// Streaming Chande Momentum Oscillator (CMO).
///
/// CMO = (sum_up - sum_down) / (sum_up + sum_down) * 100
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingCmo {
    period: usize,
    changes: VecDeque<f64>,
    sum_up: f64,
    sum_down: f64,
    prev: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingCmo {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            changes: VecDeque::with_capacity(period),
            sum_up: 0.0,
            sum_down: 0.0,
            prev: f64::NAN,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingCmo {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;

        if self.count == 1 {
            self.prev = input;
            self.last_value = None;
            return None;
        }

        let change = input - self.prev;
        self.prev = input;

        if change > 0.0 {
            self.sum_up += change;
        } else {
            self.sum_down -= change;
        }

        self.changes.push_back(change);

        if self.changes.len() > self.period {
            let old = self.changes.pop_front().unwrap();
            if old > 0.0 {
                self.sum_up -= old;
            } else {
                self.sum_down += old;
            }
        }

        if self.changes.len() < self.period {
            self.last_value = None;
            return None;
        }

        let denom = self.sum_up + self.sum_down;
        let result = if denom.abs() > 1e-15 {
            (self.sum_up - self.sum_down) / denom * 100.0
        } else {
            0.0
        };
        self.last_value = Some(result);
        Some(result)
    }

    fn reset(&mut self) {
        self.changes.clear();
        self.sum_up = 0.0;
        self.sum_down = 0.0;
        self.prev = f64::NAN;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.changes.len() >= self.period
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingCmo {
    fn name() -> &'static str { "CMO" }
    fn category() -> &'static str { "momentum" }
    fn description() -> &'static str { "Chande Momentum Oscillator" }
    fn warm_up_period(&self) -> usize { self.period + 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_cmo_basic() {
        let mut cmo = StreamingCmo::new(14);
        let data: Vec<f64> = (0..30).map(|i| 50.0 + (i as f64 * 0.3).sin() * 10.0).collect();
        let mut last = None;
        for &d in &data {
            last = cmo.next(d);
        }
        let v = last.unwrap();
        assert!((-100.0..=100.0).contains(&v), "CMO should be -100..100, got {v}");
    }

    #[test]
    fn test_streaming_cmo_uptrend() {
        let mut cmo = StreamingCmo::new(14);
        let data: Vec<f64> = (0..30).map(|i| 100.0 + i as f64).collect();
        let mut last = None;
        for &d in &data {
            last = cmo.next(d);
        }
        let v = last.unwrap();
        assert_eq!(v, 100.0, "CMO in pure uptrend should be 100");
    }

    #[test]
    fn test_streaming_cmo_reset() {
        let mut cmo = StreamingCmo::new(14);
        for i in 0..30 {
            cmo.next(50.0 + i as f64);
        }
        assert!(cmo.is_ready());
        cmo.reset();
        assert!(!cmo.is_ready());
        assert_eq!(cmo.count(), 0);
    }

    #[test]
    fn test_streaming_cmo_meta() {
        let cmo = StreamingCmo::new(14);
        assert_eq!(StreamingCmo::name(), "CMO");
        assert_eq!(cmo.warm_up_period(), 15);
    }
}
