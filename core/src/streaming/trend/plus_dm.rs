use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

/// Streaming Plus Directional Movement (+DM).
///
/// +DM = high - prev_high (if up_move > down_move and up_move > 0, else 0)
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingPlusDm {
    prev_high: f64,
    prev_low: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingPlusDm {
    pub fn new() -> Self {
        Self {
            prev_high: f64::NAN,
            prev_low: f64::NAN,
            count: 0,
            last_value: None,
        }
    }
}

impl Default for StreamingPlusDm {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingIndicator<(f64, f64)> for StreamingPlusDm {
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

        let up_move = high - self.prev_high;
        let down_move = self.prev_low - low;

        let val = if up_move > 0.0 && up_move > down_move {
            up_move
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

impl IndicatorMeta for StreamingPlusDm {
    fn name() -> &'static str { "PLUS_DM" }
    fn category() -> &'static str { "momentum" }
    fn description() -> &'static str { "Plus Directional Movement" }
    fn warm_up_period(&self) -> usize { 2 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_plus_dm() {
        let mut dm = StreamingPlusDm::new();
        assert_eq!(dm.next((100.0, 90.0)), None);
        // up_move = 115 - 100 = 15, down_move = 90 - 88 = 2, 15 > 2 => +DM = 15
        let val = dm.next((115.0, 88.0));
        assert_eq!(val, Some(15.0));
    }

    #[test]
    fn test_streaming_plus_dm_down_dominates() {
        let mut dm = StreamingPlusDm::new();
        dm.next((110.0, 100.0));
        // up_move = 112 - 110 = 2, down_move = 100 - 95 = 5, down > up => 0
        let val = dm.next((112.0, 95.0));
        assert_eq!(val, Some(0.0));
    }

    #[test]
    fn test_streaming_plus_dm_reset() {
        let mut dm = StreamingPlusDm::new();
        dm.next((100.0, 90.0));
        dm.next((115.0, 88.0));
        dm.reset();
        assert!(!dm.is_ready());
        assert_eq!(dm.count(), 0);
    }
}
