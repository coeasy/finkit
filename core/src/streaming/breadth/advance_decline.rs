use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming Advance/Decline Line.
///
/// Cumulative sum of (advances - declines) each period.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingAdvanceDeclineLine {
    cumulative: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingAdvanceDeclineLine {
    pub fn new() -> Self {
        Self {
            cumulative: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl Default for StreamingAdvanceDeclineLine {
    fn default() -> Self {
        Self::new()
    }
}

/// Input: (advances, declines)
impl StreamingIndicator<(f64, f64)> for StreamingAdvanceDeclineLine {
    #[inline]
    fn next(&mut self, input: (f64, f64)) -> Option<f64> {
        let (advances, declines) = input;
        self.count += 1;
        self.cumulative += advances - declines;
        self.last_value = Some(self.cumulative);
        Some(self.cumulative)
    }

    fn reset(&mut self) {
        self.cumulative = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= 1
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingAdvanceDeclineLine {
    fn name() -> &'static str {
        "ADVANCE_DECLINE_LINE"
    }
    fn category() -> &'static str {
        "breadth"
    }
    fn description() -> &'static str {
        "Advance/Decline Line"
    }
    fn warm_up_period(&self) -> usize {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_ad_line() {
        let mut adl = StreamingAdvanceDeclineLine::new();
        assert_eq!(adl.next((200.0, 100.0)), Some(100.0));
        assert_eq!(adl.next((150.0, 180.0)), Some(70.0));
        assert_eq!(adl.next((100.0, 100.0)), Some(70.0));
    }

    #[test]
    fn test_streaming_ad_line_reset() {
        let mut adl = StreamingAdvanceDeclineLine::new();
        adl.next((200.0, 100.0));
        adl.reset();
        assert!(!adl.is_ready());
        assert_eq!(adl.value(), None);
    }
}
