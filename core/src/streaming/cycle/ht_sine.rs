use crate::streaming::cycle::ht_dcperiod::HilbertState;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use std::f64::consts::PI;

/// Output for HT_SINE indicator
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HtSineOutput {
    pub sine: f64,
    pub lead_sine: f64,
}

/// Streaming Hilbert Transform - Sine Wave
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingHtSine {
    state: HilbertState,
    last_value: Option<f64>,
}

impl StreamingHtSine {
    pub fn new() -> Self {
        Self { state: HilbertState::new(), last_value: None }
    }

    /// Returns (sine, lead_sine) tuple
    pub fn next_sine(&mut self, input: f64) -> Option<HtSineOutput> {
        self.state.update(input).map(|(phase, _)| {
            let sine = phase.sin();
            let lead_sine = (phase + PI / 4.0).sin();
            self.last_value = Some(sine);
            HtSineOutput { sine, lead_sine }
        })
    }
}

impl Default for StreamingHtSine {
    fn default() -> Self { Self::new() }
}

impl StreamingIndicator for StreamingHtSine {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.next_sine(input).map(|o| o.sine)
    }

    fn reset(&mut self) { self.state.reset(); self.last_value = None; }
    fn is_ready(&self) -> bool { self.state.count >= 32 }
    fn count(&self) -> usize { self.state.count }
    fn value(&self) -> Option<f64> { self.last_value }
}

impl IndicatorMeta for StreamingHtSine {
    fn name() -> &'static str { "HT_SINE" }
    fn category() -> &'static str { "cycle" }
    fn description() -> &'static str { "Hilbert Transform - Sine Wave" }
    fn warm_up_period(&self) -> usize { 32 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_wave(n: usize, freq: f64, amp: f64, offset: f64) -> Vec<f64> {
        (0..n).map(|i| amp * (i as f64 * freq).sin() + offset).collect()
    }

    #[test]
    fn test_streaming_ht_sine_basic() {
        let mut ht = StreamingHtSine::new();
        let data = sine_wave(100, 0.1, 1.0, 50.0);
        let mut last = None;
        for &v in &data { last = ht.next_sine(v); }
        let out = last.unwrap();
        assert!(out.sine >= -1.0 && out.sine <= 1.0);
        assert!(out.lead_sine >= -1.0 && out.lead_sine <= 1.0);
    }

    #[test]
    fn test_streaming_ht_sine_meta() {
        assert_eq!(StreamingHtSine::name(), "HT_SINE");
        assert_eq!(StreamingHtSine::category(), "cycle");
    }

    #[test]
    fn test_streaming_ht_sine_reset() {
        let mut ht = StreamingHtSine::new();
        for i in 0..50 { ht.next(i as f64); }
        assert!(ht.is_ready());
        ht.reset();
        assert!(!ht.is_ready());
    }

    #[test]
    fn test_streaming_ht_sine_bounded() {
        let mut ht = StreamingHtSine::new();
        let data = sine_wave(200, 0.1, 1.0, 50.0);
        for &v in &data {
            if let Some(out) = ht.next_sine(v) {
                assert!(out.sine >= -1.001 && out.sine <= 1.001);
                assert!(out.lead_sine >= -1.001 && out.lead_sine <= 1.001);
            }
        }
    }
}
