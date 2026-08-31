use crate::streaming::rolling_minmax::{RollingMax, RollingMin};
use crate::streaming::traits::StreamingIndicator;
use crate::{impl_indicator_meta, impl_standard_methods};

/// Streaming MidPoint over period.
///
/// MIDPOINT = (highest + lowest) / 2 over `period` bars.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMidpoint {
    period: usize,
    rolling_max: RollingMax,
    rolling_min: RollingMin,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingMidpoint {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            rolling_max: RollingMax::new(),
            rolling_min: RollingMin::new(),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingMidpoint {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let idx = self.count - 1;

        self.rolling_max.push(idx, input);
        self.rolling_min.push(idx, input);

        if self.count > self.period {
            let expire_idx = self.count - self.period - 1;
            self.rolling_max.pop(expire_idx);
            self.rolling_min.pop(expire_idx);
        }

        if self.count < self.period {
            self.last_value = None;
            return None;
        }

        let max = self.rolling_max.current().unwrap();
        let min = self.rolling_min.current().unwrap();
        let val = (max + min) / 2.0;
        self.last_value = Some(val);
        Some(val)
    }

    fn reset(&mut self) {
        self.rolling_max.reset();
        self.rolling_min.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(
    StreamingMidpoint,
    "MIDPOINT",
    "overlap",
    "MidPoint over period"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_midpoint() {
        let mut mid = StreamingMidpoint::new(3);
        assert_eq!(mid.next(10.0), None);
        assert_eq!(mid.next(20.0), None);
        // window [10,20,15]: min=10, max=20, mid=15
        assert_eq!(mid.next(15.0), Some(15.0));
        // window [20,15,25]: min=15, max=25, mid=20
        assert_eq!(mid.next(25.0), Some(20.0));
    }

    #[test]
    fn test_streaming_midpoint_reset() {
        let mut mid = StreamingMidpoint::new(3);
        mid.next(10.0);
        mid.next(20.0);
        mid.next(15.0);
        mid.reset();
        assert!(!mid.is_ready());
        assert_eq!(mid.count(), 0);
    }
}
