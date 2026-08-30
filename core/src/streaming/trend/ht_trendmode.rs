use crate::streaming::cycle::ht_dcperiod::HilbertState;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming Hilbert Transform - Trend vs Cycle Mode
///
/// Returns 1.0 for trend mode, 0.0 for cycle mode.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingHtTrendMode {
    state: HilbertState,
    last_value: Option<f64>,
}

impl StreamingHtTrendMode {
    pub fn new() -> Self {
        Self { state: HilbertState::new(), last_value: None }
    }
}

impl Default for StreamingHtTrendMode {
    fn default() -> Self { Self::new() }
}

impl StreamingIndicator for StreamingHtTrendMode {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        let result = self.state.update(input).map(|(_, dc_period)| {
            if dc_period <= 6.0 || dc_period >= 36.0 { 1.0 } else { 0.0 }
        });
        self.last_value = result;
        result
    }

    fn reset(&mut self) { self.state.reset(); self.last_value = None; }
    fn is_ready(&self) -> bool { self.state.count >= 32 }
    fn count(&self) -> usize { self.state.count }
    fn value(&self) -> Option<f64> { self.last_value }
}

impl IndicatorMeta for StreamingHtTrendMode {
    fn name() -> &'static str { "HT_TRENDMODE" }
    fn category() -> &'static str { "cycle" }
    fn description() -> &'static str { "Hilbert Transform - Trend vs Cycle Mode" }
    fn warm_up_period(&self) -> usize { 32 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_wave(n: usize, freq: f64, amp: f64, offset: f64) -> Vec<f64> {
        (0..n).map(|i| amp * (i as f64 * freq).sin() + offset).collect()
    }

    #[test]
    fn test_streaming_ht_trendmode_basic() {
        let mut ht = StreamingHtTrendMode::new();
        let data = sine_wave(100, 0.1, 1.0, 50.0);
        let mut last = None;
        for &v in &data { last = ht.next(v); }
        assert!(last.is_some());
        let val = last.unwrap();
        assert!(val == 0.0 || val == 1.0);
    }

    #[test]
    fn test_streaming_ht_trendmode_meta() {
        assert_eq!(StreamingHtTrendMode::name(), "HT_TRENDMODE");
        assert_eq!(StreamingHtTrendMode::category(), "cycle");
    }

    #[test]
    fn test_streaming_ht_trendmode_reset() {
        let mut ht = StreamingHtTrendMode::new();
        for i in 0..50 { ht.next(i as f64); }
        assert!(ht.is_ready());
        ht.reset();
        assert!(!ht.is_ready());
    }

    #[test]
    fn test_streaming_ht_trendmode_binary() {
        let mut ht = StreamingHtTrendMode::new();
        let data = sine_wave(200, 0.1, 1.0, 50.0);
        for &v in &data {
            if let Some(val) = ht.next(v) {
                assert!(val == 0.0 || val == 1.0, "Expected 0 or 1, got {val}");
            }
        }
    }
}
