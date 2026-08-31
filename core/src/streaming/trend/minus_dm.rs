use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming Minus Directional Movement (-DM).
///
/// -DM = prev_low - low (if down_move > up_move and down_move > 0, else 0)
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMinusDm {
    prev_high: f64,
    prev_low: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingMinusDm {
    pub fn new() -> Self {
        Self {
            prev_high: f64::NAN,
            prev_low: f64::NAN,
            count: 0,
            last_value: None,
        }
    }
}

impl Default for StreamingMinusDm {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingIndicator<(f64, f64)> for StreamingMinusDm {
    #[inline]
    fn next(&mut self, input: (f64, f64)) -> Option<f64> {
        let (high, low) = input;
        self.count += 1;

        if self.count == 1 {
            self.prev_high = high;
            self.prev_low = low;
            self.last_value = None;
            return None;
        }

        let down_move = self.prev_low - low;
        let up_move = high - self.prev_high;

        let val = if down_move > 0.0 && down_move > up_move {
            down_move
        } else {
            0.0
        };

        self.prev_high = high;
        self.prev_low = low;
        self.last_value = Some(val);
        Some(val)
    }

    fn reset(&mut self) {
        self.prev_high = f64::NAN;
        self.prev_low = f64::NAN;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= 2
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingMinusDm {
    fn name() -> &'static str {
        "MINUS_DM"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Minus Directional Movement"
    }
    fn warm_up_period(&self) -> usize {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_minus_dm() {
        let mut dm = StreamingMinusDm::new();
        assert_eq!(dm.next((110.0, 100.0)), None);
        // down_move = 100 - 95 = 5, up_move = 112 - 110 = 2, 5 > 2 => -DM = 5
        let val = dm.next((112.0, 95.0));
        assert_eq!(val, Some(5.0));
    }

    #[test]
    fn test_streaming_minus_dm_up_dominates() {
        let mut dm = StreamingMinusDm::new();
        dm.next((100.0, 90.0));
        // up_move = 115 - 100 = 15, down_move = 90 - 88 = 2, up > down => 0
        let val = dm.next((115.0, 88.0));
        assert_eq!(val, Some(0.0));
    }

    #[test]
    fn test_streaming_minus_dm_reset() {
        let mut dm = StreamingMinusDm::new();
        dm.next((100.0, 90.0));
        dm.next((105.0, 85.0));
        dm.reset();
        assert!(!dm.is_ready());
        assert_eq!(dm.count(), 0);
    }
}
