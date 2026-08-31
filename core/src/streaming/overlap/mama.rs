//! Streaming MAMA (MESA Adaptive Moving Average).
//!
//! Implementation of Ehlers' MAMA/FAMA: the MAMA line adapts its smoothing
//! constant to the dominant cycle period estimated by the Hilbert transform.
//! Uses a fixed-size Hilbert state from [`super::ht_dcperiod::HilbertState`].
//!
//! Reference: John F. Ehlers, "Rocket Science for Traders" (2001), Chapter 9.

use crate::streaming::cycle::ht_dcperiod::HilbertState;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming MAMA/FAMA pair: a single Hilbert state feeding two adaptive EMAs.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMama {
    state: HilbertState,
    /// Fast limit for MAMA (default 0.5)
    fast_limit: f64,
    /// Slow limit for FAMA (default 0.05)
    slow_limit: f64,
    /// Previous input (for difference).
    prev_input: f64,
    /// Smoothed phase (alpha state for MAMA)
    phase_state: f64,
    /// MAMA value at previous step
    prev_mama: f64,
    /// FAMA value at previous step
    prev_fama: f64,
    /// Computed alpha
    alpha: f64,
    count: usize,
    last_mama: Option<f64>,
    last_fama: Option<f64>,
}

impl StreamingMama {
    /// Create with default limits fast=0.5, slow=0.05.
    pub fn new() -> Self {
        Self::with_limits(0.5, 0.05)
    }

    /// Create with custom fast/slow limits.
    pub fn with_limits(fast_limit: f64, slow_limit: f64) -> Self {
        Self {
            state: HilbertState::new(),
            fast_limit,
            slow_limit,
            prev_input: 0.0,
            phase_state: 0.0,
            prev_mama: 0.0,
            prev_fama: 0.0,
            alpha: fast_limit,
            count: 0,
            last_mama: None,
            last_fama: None,
        }
    }
}

impl Default for StreamingMama {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingIndicator for StreamingMama {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        if self.count == 1 {
            self.prev_input = input;
            self.prev_mama = input;
            self.prev_fama = input;
            self.last_mama = Some(input);
            self.last_fama = Some(input);
            // Still need to feed Hilbert state for warm-up
            let _ = self.state.update(input);
            return None;
        }

        // Update Hilbert transform to get the dominant period
        let (_, period) = match self.state.update(input) {
            Some(v) => v,
            None => {
                self.prev_input = input;
                self.last_mama = Some(self.prev_mama);
                self.last_fama = Some(self.prev_fama);
                return Some(self.prev_mama); // not ready yet, return prev
            }
        };

        // Phase rate of change (delta_phase is computed inside HilbertState.update)
        // Ehlers' adaptive alpha: alpha = fast_limit * delta_phase
        // We use period to derive delta_phase ≈ 2π/period
        let delta_phase = (2.0 * std::f64::consts::PI / period).clamp(0.0, 1.0);
        // Smooth phase state
        self.phase_state = 0.0; // not used beyond 1-step lag
        let alpha = (self.fast_limit * delta_phase).clamp(self.slow_limit, self.fast_limit);
        self.alpha = alpha;

        // MAMA = alpha * price + (1 - alpha) * prev_mama
        let mama = alpha * input + (1.0 - alpha) * self.prev_mama;
        // FAMA uses half the alpha
        let alpha_f = (alpha * 0.5).clamp(self.slow_limit, self.fast_limit);
        let fama = alpha_f * mama + (1.0 - alpha_f) * self.prev_fama;

        self.prev_input = input;
        self.prev_mama = mama;
        self.prev_fama = fama;
        self.last_mama = Some(mama);
        self.last_fama = Some(fama);

        Some(mama)
    }

    fn reset(&mut self) {
        self.state.reset();
        self.prev_input = 0.0;
        self.phase_state = 0.0;
        self.prev_mama = 0.0;
        self.prev_fama = 0.0;
        self.alpha = self.fast_limit;
        self.count = 0;
        self.last_mama = None;
        self.last_fama = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= 32
    }
    fn count(&self) -> usize {
        self.count
    }
    fn value(&self) -> Option<f64> {
        self.last_mama
    }
}

impl StreamingMama {
    /// FAMA value (follows MAMA with even slower adaptation).
    pub fn fama(&self) -> Option<f64> {
        self.last_fama
    }

    /// Adaptive smoothing constant (0..=fast_limit). Useful for diagnostics.
    pub fn alpha(&self) -> f64 {
        self.alpha
    }
}

impl IndicatorMeta for StreamingMama {
    fn name() -> &'static str {
        "MAMA"
    }
    fn category() -> &'static str {
        "overlap"
    }
    fn description() -> &'static str {
        "MESA Adaptive Moving Average"
    }
    fn warm_up_period(&self) -> usize {
        32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_wave(n: usize, freq: f64, amp: f64, offset: f64) -> Vec<f64> {
        (0..n)
            .map(|i| amp * (i as f64 * freq).sin() + offset)
            .collect()
    }

    #[test]
    fn test_mama_warmup() {
        let mut m = StreamingMama::new();
        let data = sine_wave(50, 0.1, 1.0, 50.0);
        let mut last = None;
        for &v in &data {
            last = m.next(v);
        }
        assert!(last.is_some());
        // MAMA should track the wave within bounds
        let v = last.unwrap();
        assert!(v > 48.0 && v < 52.0, "MAMA out of expected range: {v}");
    }

    #[test]
    fn test_mama_fama_relationship() {
        let mut m = StreamingMama::new();
        let data = sine_wave(60, 0.1, 5.0, 100.0);
        for &v in &data {
            m.next(v);
        }
        let mama = m.value().unwrap();
        let fama = m.fama().unwrap();
        // FAMA adapts slower, so MAMA and FAMA should be close but not identical
        assert!((mama - fama).abs() < 5.0);
    }

    #[test]
    fn test_mama_reset() {
        let mut m = StreamingMama::new();
        let data = sine_wave(50, 0.1, 1.0, 50.0);
        for &v in &data {
            m.next(v);
        }
        assert!(m.is_ready());
        m.reset();
        assert!(!m.is_ready());
        assert_eq!(m.value(), None);
    }

    #[test]
    fn test_mama_meta() {
        assert_eq!(StreamingMama::name(), "MAMA");
        assert_eq!(StreamingMama::category(), "overlap");
    }
}
