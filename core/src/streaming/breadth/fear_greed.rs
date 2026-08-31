use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming Fear & Greed Index (simplified composite).
///
/// Composite score from volatility, momentum, and breadth components.
/// Score = (norm_vol + norm_mom + norm_breadth) / 3 * 100, clamped to [0, 100].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingFearGreedIndex {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingFearGreedIndex {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl Default for StreamingFearGreedIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Input: (volatility_score, momentum_score, breadth_score) each in [0, 1]
impl StreamingIndicator<(f64, f64, f64)> for StreamingFearGreedIndex {
    #[inline]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<f64> {
        let (vol, mom, breadth) = input;
        self.count += 1;

        let composite = ((vol + mom + breadth) / 3.0 * 100.0).clamp(0.0, 100.0);
        self.last_value = Some(composite);
        Some(composite)
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= 1
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingFearGreedIndex {
    fn name() -> &'static str {
        "FEAR_GREED_INDEX"
    }
    fn category() -> &'static str {
        "sentiment"
    }
    fn description() -> &'static str {
        "Fear & Greed Index"
    }
    fn warm_up_period(&self) -> usize {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_fear_greed() {
        let mut fg = StreamingFearGreedIndex::new();
        // All at 0.5 => 50.0
        assert_eq!(fg.next((0.5, 0.5, 0.5)), Some(50.0));
        // All at 1.0 => 100.0
        assert_eq!(fg.next((1.0, 1.0, 1.0)), Some(100.0));
        // All at 0.0 => 0.0
        assert_eq!(fg.next((0.0, 0.0, 0.0)), Some(0.0));
    }

    #[test]
    fn test_streaming_fear_greed_clamp() {
        let mut fg = StreamingFearGreedIndex::new();
        // Values > 1.0 get clamped
        let val = fg.next((1.5, 1.5, 1.5));
        assert_eq!(val, Some(100.0));
    }

    #[test]
    fn test_streaming_fear_greed_reset() {
        let mut fg = StreamingFearGreedIndex::new();
        fg.next((0.5, 0.5, 0.5));
        fg.reset();
        assert!(!fg.is_ready());
    }
}
