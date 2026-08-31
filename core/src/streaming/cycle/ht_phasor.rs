use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming Hilbert Transform - Phasor Components.
///
/// Returns (InPhase, Quadrature) pair from Hilbert Transform decomposition.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingHtPhasor {
    smooth_buf: [f64; 8],
    det_buf: [f64; 8],
    input_buf: [f64; 8],
    count: usize,
    last_value: Option<(f64, f64)>,
}

impl StreamingHtPhasor {
    pub fn new() -> Self {
        Self {
            smooth_buf: [0.0; 8],
            det_buf: [0.0; 8],
            input_buf: [0.0; 8],
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    fn shift(buf: &mut [f64; 8], val: f64) {
        buf.copy_within(1.., 0);
        buf[7] = val;
    }
}

impl Default for StreamingHtPhasor {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingIndicator<f64, (f64, f64)> for StreamingHtPhasor {
    #[inline]
    fn next(&mut self, input: f64) -> Option<(f64, f64)> {
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

        let in_phase = self.det_buf[1];
        let quadrature = 0.0962 * self.det_buf[7] + 0.5769 * self.det_buf[5]
            - 0.5769 * self.det_buf[3]
            - 0.0962 * self.det_buf[1];

        self.last_value = Some((in_phase, quadrature));
        Some((in_phase, quadrature))
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn is_ready(&self) -> bool {
        self.count >= 16
    }

    impl_standard_methods!(output = (f64, f64));
}

impl IndicatorMeta for StreamingHtPhasor {
    fn name() -> &'static str {
        "HT_PHASOR"
    }
    fn category() -> &'static str {
        "cycle"
    }
    fn description() -> &'static str {
        "Hilbert Transform - Phasor Components"
    }
    fn warm_up_period(&self) -> usize {
        16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_ht_phasor_warmup() {
        let mut phasor = StreamingHtPhasor::new();
        for i in 0..15 {
            assert_eq!(phasor.next(100.0 + (i as f64).sin() * 5.0), None);
        }
        // Should produce output after 16th value
        let val = phasor.next(102.0);
        assert!(val.is_some());
    }

    #[test]
    fn test_streaming_ht_phasor_reset() {
        let mut phasor = StreamingHtPhasor::new();
        for i in 0..20 {
            phasor.next(100.0 + (i as f64 * 0.5).sin() * 10.0);
        }
        assert!(phasor.is_ready());
        phasor.reset();
        assert!(!phasor.is_ready());
        assert_eq!(phasor.count(), 0);
    }
}
