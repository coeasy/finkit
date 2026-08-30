use crate::streaming::cycle::ht_dcperiod::HilbertState;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming Hilbert Transform - Dominant Cycle Phase (degrees)
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingHtDcPhase {
    state: HilbertState,
    last_value: Option<f64>,
}

impl StreamingHtDcPhase {
    pub fn new() -> Self {
        Self { state: HilbertState::new(), last_value: None }
    }
}

impl Default for StreamingHtDcPhase {
    fn default() -> Self { Self::new() }
}

impl StreamingIndicator for StreamingHtDcPhase {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        let result = self.state.update(input).map(|(phase, _)| {
            phase * 180.0 / std::f64::consts::PI
        });
        self.last_value = result;
        result
    }

    fn reset(&mut self) { self.state.reset(); self.last_value = None; }
    fn is_ready(&self) -> bool { self.state.count >= 32 }
    fn count(&self) -> usize { self.state.count }
    fn value(&self) -> Option<f64> { self.last_value }
}

impl IndicatorMeta for StreamingHtDcPhase {
    fn name() -> &'static str { "HT_DCPHASE" }
    fn category() -> &'static str { "cycle" }
    fn description() -> &'static str { "Hilbert Transform - Dominant Cycle Phase" }
    fn warm_up_period(&self) -> usize { 32 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_wave(n: usize, freq: f64, amp: f64, offset: f64) -> Vec<f64> {
        (0..n).map(|i| amp * (i as f64 * freq).sin() + offset).collect()
    }

    #[test]
    fn test_streaming_ht_dcphase_basic() {
        let mut ht = StreamingHtDcPhase::new();
        let data = sine_wave(100, 0.1, 1.0, 50.0);
        let mut last = None;
        for &v in &data { last = ht.next(v); }
        assert!(last.is_some());
    }

    #[test]
    fn test_streaming_ht_dcphase_meta() {
        assert_eq!(StreamingHtDcPhase::name(), "HT_DCPHASE");
        assert_eq!(StreamingHtDcPhase::category(), "cycle");
    }

    #[test]
    fn test_streaming_ht_dcphase_reset() {
        let mut ht = StreamingHtDcPhase::new();
        for i in 0..50 { ht.next(i as f64); }
        assert!(ht.is_ready());
        ht.reset();
        assert!(!ht.is_ready());
    }
}
