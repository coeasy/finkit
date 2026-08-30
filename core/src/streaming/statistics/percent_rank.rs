//! Streaming rolling `PERCENTRANK` (TA-Lib `TA_PERCENTRANK`).
//!
//! O(n) per-bar -- the simple batch algorithm scales as O(period) per step
//! because we need to count how many values in the window are strictly less
//! than the current value. For typical periods (<= 100) this is fast enough.
//! A true O(1) streaming version would require a balanced BST keyed by value,
//! which is not worth the constant factor for these small periods.

use crate::streaming::traits::{StreamingIndicator};
use crate::impl_standard_methods;
use crate::{impl_indicator_meta};
use std::collections::VecDeque;

/// Streaming rolling `PERCENTRANK` (0..=100 scale).
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingPercentRank {
    period: usize,
    buf: VecDeque<f64>,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingPercentRank {
    pub fn new(period: usize) -> Self {
        assert!(period >= 1, "period must be >= 1");
        Self {
            period,
            buf: VecDeque::with_capacity(period),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingPercentRank {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        if self.buf.len() == self.period {
            self.buf.pop_front();
        }
        self.buf.push_back(input);

        if self.count < self.period {
            self.last_value = None;
            return None;
        }
        // Count values strictly less than `input` in the window
        let less = self.buf.iter().filter(|&&v| v < input).count() as f64;
        // TA-Lib formula: (less / (period - 1)) * 100
        let pct = if self.period > 1 {
            (less / (self.period as f64 - 1.0)) * 100.0
        } else {
            0.0
        };
        self.last_value = Some(pct);
        Some(pct)
    }

    fn reset(&mut self) {
        self.buf.clear();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool { self.count >= self.period }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingPercentRank, "PERCENTRANK", "statistics", "Percent rank (0-100) of current value in rolling window");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;

    #[test]
    fn test_percent_rank_basic() {
        let mut p = StreamingPercentRank::new(5);
        // window = [1, 2, 3, 4, 5] -> 5 is the max -> (4 / 4) * 100 = 100
        for &v in &[1.0, 2.0, 3.0, 4.0] { p.next(v); }
        assert_eq!(p.next(5.0), Some(100.0));
        // window = [2, 3, 4, 5, 1] -> 1 is the min -> 0
        assert_eq!(p.next(1.0), Some(0.0));
    }

    #[test]
    fn test_percent_rank_median() {
        let mut p = StreamingPercentRank::new(3);
        assert_eq!(p.next(1.0), None);
        assert_eq!(p.next(2.0), None);
        // [1, 2, 3] -> 3 is max -> 100
        assert_eq!(p.next(3.0), Some(100.0));
        // [2, 3, 2] -> 2 has 0 values less than it in [2,3,2] -> 0
        // (count strict-less; 0 of 2 strictly less than 2)
        assert_eq!(p.next(2.0), Some(0.0));
    }

    #[test]
    fn test_percent_rank_reset() {
        let mut p = StreamingPercentRank::new(3);
        p.next(1.0);
        p.next(2.0);
        p.next(3.0);
        assert!(p.is_ready());
        p.reset();
        assert!(!p.is_ready());
    }

    #[test]
    fn test_meta() {
        assert_eq!(StreamingPercentRank::name(), "PERCENTRANK");
        assert_eq!(StreamingPercentRank::category(), "statistics");
    }
}
