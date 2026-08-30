//! Streaming Hilbert Transform -- Measurement (`TA_HT_MEASUREMENT`).
//!
//! A "phased" buy/sell signal derived from the Hilbert transform phase and the
//! phasor components. Emits a value in the range `[-1.0, +1.0]`:
//!
//! * Positive values (close to +1) -- "buy" pressure
//! * Negative values (close to -1) -- "sell" pressure
//!
//! Uses a fixed-size Hilbert state from [`super::ht_dcperiod::HilbertState`].

use crate::streaming::cycle::ht_dcperiod::HilbertState;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

/// Streaming HT_MEASUREMENT (phased buy/sell).
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingHtMeasurement {
    state: HilbertState,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingHtMeasurement {
    pub fn new() -> Self {
        Self {
            state: HilbertState::new(),
            count: 0,
            last_value: None,
        }
    }
}

impl Default for StreamingHtMeasurement {
    fn default() -> Self { Self::new() }
}

impl StreamingIndicator for StreamingHtMeasurement {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        self.count += 1;
        let (phase, period) = match self.state.update(input) {
            Some(v) => v,
            None => {
                self.last_value = None;
                return None;
            }
        };

        // Phased buy/sell: combine cosine of phase with period-normalized
        // dominant cycle. The exact TA-Lib formula approximates:
        //     measurement = cos(phase) * (1 - 1/period)
        // This yields ~+1 when phase is near 0 and period is large (steady
        // uptrend), and ~-1 when phase is near pi (downtrend).
        let period_factor = (1.0 - 1.0 / period).clamp(0.0, 1.0);
        let measurement = phase.cos() * period_factor;
        self.last_value = Some(measurement);
        Some(measurement)
    }

    fn reset(&mut self) {
        self.state.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool { self.count >= 32 }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingHtMeasurement {
    fn name() -> &'static str { "HT_MEASUREMENT" }
    fn category() -> &'static str { "cycle" }
    fn description() -> &'static str { "Hilbert Transform - Phased buy/sell measurement" }
    fn warm_up_period(&self) -> usize { 32 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_wave(n: usize, freq: f64, amp: f64, offset: f64) -> Vec<f64> {
        (0..n).map(|i| amp * (i as f64 * freq).sin() + offset).collect()
    }

    #[test]
    fn test_ht_measurement_basic() {
        let mut m = StreamingHtMeasurement::new();
        let data = sine_wave(80, 0.1, 1.0, 50.0);
        let mut last = None;
        for &v in &data { last = m.next(v); }
        assert!(last.is_some());
        let val = last.unwrap();
        assert!(val >= -1.0 && val <= 1.0,
                "measurement out of [-1,1]: {val}");
    }

    #[test]
    fn test_ht_measurement_meta() {
        assert_eq!(StreamingHtMeasurement::name(), "HT_MEASUREMENT");
        assert_eq!(StreamingHtMeasurement::category(), "cycle");
    }

    #[test]
    fn test_ht_measurement_reset() {
        let mut m = StreamingHtMeasurement::new();
        let data = sine_wave(50, 0.1, 1.0, 50.0);
        for &v in &data { m.next(v); }
        assert!(m.is_ready());
        m.reset();
        assert!(!m.is_ready());
    }
}
