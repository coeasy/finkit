use crate::streaming::rolling_minmax::{RollingMax, RollingMin};
use crate::streaming::traits::{StreamingIndicator};
use crate::impl_standard_methods;
use crate::{impl_indicator_meta};

/// Streaming Midpoint Price over period.
///
/// MIDPRICE = (highest_high + lowest_low) / 2 over `period` bars.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMidprice {
    period: usize,
    high_max: RollingMax,
    low_min: RollingMin,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingMidprice {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            high_max: RollingMax::new(),
            low_min: RollingMin::new(),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64)> for StreamingMidprice {
    #[inline]
    fn next(&mut self, input: (f64, f64)) -> Option<f64> {
        let (high, low) = input;
        self.count += 1;
        let idx = self.count - 1;

        self.high_max.push(idx, high);
        self.low_min.push(idx, low);

        if self.count > self.period {
            let expire_idx = self.count - self.period - 1;
            self.high_max.pop(expire_idx);
            self.low_min.pop(expire_idx);
        }

        if self.count < self.period {
            self.last_value = None;
            return None;
        }

        let max = self.high_max.current().unwrap();
        let min = self.low_min.current().unwrap();
        let val = (max + min) / 2.0;
        self.last_value = Some(val);
        Some(val)
    }

    fn reset(&mut self) {
        self.high_max.reset();
        self.low_min.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingMidprice, "MIDPRICE", "overlap", "Midpoint Price over period");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_midprice() {
        let mut mp = StreamingMidprice::new(3);
        assert_eq!(mp.next((110.0, 90.0)), None);
        assert_eq!(mp.next((120.0, 95.0)), None);
        // highest_high=120, lowest_low=90, mid=105
        assert_eq!(mp.next((115.0, 92.0)), Some(105.0));
    }

    #[test]
    fn test_streaming_midprice_reset() {
        let mut mp = StreamingMidprice::new(3);
        mp.next((110.0, 90.0));
        mp.next((120.0, 95.0));
        mp.next((115.0, 92.0));
        mp.reset();
        assert!(!mp.is_ready());
    }
}
