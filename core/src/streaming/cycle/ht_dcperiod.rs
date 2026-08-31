use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use std::f64::consts::PI;

/// Shared Hilbert Transform state for streaming indicators.
/// Uses fixed-size arrays (no heap allocation) per AC3.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct HilbertState {
    pub input_buf: [f64; 8],
    pub smooth_buf: [f64; 8],
    pub det_buf: [f64; 8],
    pub ip_buf: [f64; 8],
    pub q_buf: [f64; 8],
    pub j1_buf: [f64; 8],
    pub count: usize,
    pub phase: f64,
    pub prev_phase: f64,
    pub smooth_period: f64,
}

impl HilbertState {
    pub fn new() -> Self {
        Self {
            input_buf: [0.0; 8],
            smooth_buf: [0.0; 8],
            det_buf: [0.0; 8],
            ip_buf: [0.0; 8],
            q_buf: [0.0; 8],
            j1_buf: [0.0; 8],
            count: 0,
            phase: 0.0,
            prev_phase: 0.0,
            smooth_period: 15.0,
        }
    }

    #[inline]
    fn shift(buf: &mut [f64; 8], val: f64) {
        buf.copy_within(1.., 0);
        buf[7] = val;
    }

    /// Update the Hilbert state with a new input value.
    /// Returns (phase, smooth_dcperiod) once warmed up (count >= 32).
    pub fn update(&mut self, input: f64) -> Option<(f64, f64)> {
        self.count += 1;
        Self::shift(&mut self.input_buf, input);

        if self.count < 4 {
            return None;
        }

        let smooth = (4.0 * self.input_buf[7]
            + 3.0 * self.input_buf[6]
            + 2.0 * self.input_buf[5]
            + self.input_buf[4])
            / 10.0;
        Self::shift(&mut self.smooth_buf, smooth);

        if self.count < 10 {
            return None;
        }

        let det = (0.0962 * self.smooth_buf[7] + 0.5769 * self.smooth_buf[5]
            - 0.5769 * self.smooth_buf[3]
            - 0.0962 * self.smooth_buf[1])
            * (0.075 * self.smooth_buf[6] + 0.54 * self.smooth_buf[4] + 0.075 * self.smooth_buf[2]);
        Self::shift(&mut self.det_buf, det);

        if self.count < 16 {
            return None;
        }

        let in_phase = self.det_buf[1]; // det[i-6]
        Self::shift(&mut self.ip_buf, in_phase);

        let quadrature = 0.0962 * self.det_buf[7] + 0.5769 * self.det_buf[5]
            - 0.5769 * self.det_buf[3]
            - 0.0962 * self.det_buf[1];
        Self::shift(&mut self.q_buf, quadrature);

        let j1 = 0.0962 * self.ip_buf[7] + 0.5769 * self.ip_buf[5]
            - 0.5769 * self.ip_buf[3]
            - 0.0962 * self.ip_buf[1];
        Self::shift(&mut self.j1_buf, j1);

        let i2 = in_phase - j1;
        let j2 = quadrature + in_phase;

        let re = i2 * in_phase + j2 * quadrature;
        let im = i2 * quadrature - j2 * in_phase;

        self.phase = if re.abs() > 1e-10 { im.atan2(re) } else { 0.0 };

        let delta_phase = self.prev_phase - self.phase;
        self.prev_phase = self.phase;

        let period = if delta_phase.abs() > 1e-10 {
            (2.0 * PI / delta_phase.abs()).clamp(6.0, 50.0)
        } else {
            self.smooth_period
        };

        self.smooth_period = 0.33 * period + 0.67 * self.smooth_period;

        Some((self.phase, self.smooth_period))
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

/// Streaming Hilbert Transform - Dominant Cycle Period
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingHtDcPeriod {
    state: HilbertState,
    last_value: Option<f64>,
}

impl StreamingHtDcPeriod {
    pub fn new() -> Self {
        Self {
            state: HilbertState::new(),
            last_value: None,
        }
    }
}

impl Default for StreamingHtDcPeriod {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingIndicator for StreamingHtDcPeriod {
    #[inline]
    fn next(&mut self, input: f64) -> Option<f64> {
        let result = self.state.update(input).map(|(_, period)| period);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.state.reset();
        self.last_value = None;
    }
    fn is_ready(&self) -> bool {
        self.state.count >= 32
    }
    fn count(&self) -> usize {
        self.state.count
    }
    fn value(&self) -> Option<f64> {
        self.last_value
    }
}

impl IndicatorMeta for StreamingHtDcPeriod {
    fn name() -> &'static str {
        "HT_DCPERIOD"
    }
    fn category() -> &'static str {
        "cycle"
    }
    fn description() -> &'static str {
        "Hilbert Transform - Dominant Cycle Period"
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
    fn test_streaming_ht_dcperiod_basic() {
        let mut ht = StreamingHtDcPeriod::new();
        let data = sine_wave(100, 0.1, 1.0, 50.0);
        let mut last = None;
        for &v in &data {
            last = ht.next(v);
        }
        assert!(last.is_some());
        let val = last.unwrap();
        assert!((6.0..=50.0).contains(&val));
    }

    #[test]
    fn test_streaming_ht_dcperiod_meta() {
        assert_eq!(StreamingHtDcPeriod::name(), "HT_DCPERIOD");
        assert_eq!(StreamingHtDcPeriod::category(), "cycle");
    }

    #[test]
    fn test_streaming_ht_dcperiod_reset() {
        let mut ht = StreamingHtDcPeriod::new();
        for i in 0..50 {
            ht.next(i as f64);
        }
        assert!(ht.is_ready());
        ht.reset();
        assert!(!ht.is_ready());
        assert_eq!(ht.count(), 0);
    }

    #[test]
    fn test_streaming_ht_dcperiod_no_heap() {
        let ht = StreamingHtDcPeriod::new();
        assert_eq!(std::mem::size_of_val(&ht.state.input_buf), 64);
    }
}
