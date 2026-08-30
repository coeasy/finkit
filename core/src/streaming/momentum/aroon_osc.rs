use crate::streaming::momentum::aroon::StreamingAroon;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

/// Streaming Aroon Oscillator.
///
/// AROONOSC = Aroon_Up - Aroon_Down
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingAroonOsc {
    period: usize,
    aroon: StreamingAroon,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingAroonOsc {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            aroon: StreamingAroon::new(period),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64)> for StreamingAroonOsc {
    #[inline]
    fn next(&mut self, input: (f64, f64)) -> Option<f64> {
        self.count += 1;

        let aroon_out = self.aroon.next(input);

        let Some(out) = aroon_out else {
            self.last_value = None;
            return None;
        };

        let result = out.aroon_up - out.aroon_down;
        self.last_value = Some(result);
        Some(result)
    }

    fn reset(&mut self) {
        self.aroon.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.aroon.is_ready()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingAroonOsc {
    fn name() -> &'static str { "AROONOSC" }
    fn category() -> &'static str { "momentum" }
    fn description() -> &'static str { "Aroon Oscillator" }
    fn warm_up_period(&self) -> usize { self.period + 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_aroon_osc_basic() {
        let mut osc = StreamingAroonOsc::new(5);
        let data: Vec<(f64, f64)> = (0..15)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.5).sin() * 10.0;
                (h, h - 3.0)
            })
            .collect();
        let mut last = None;
        for &d in &data {
            last = osc.next(d);
        }
        let v = last.unwrap();
        assert!((-100.0..=100.0).contains(&v), "AROONOSC should be -100..100, got {v}");
    }

    #[test]
    fn test_streaming_aroon_osc_uptrend() {
        let mut osc = StreamingAroonOsc::new(5);
        for i in 0..15 {
            let h = 100.0 + i as f64 * 5.0;
            let out = osc.next((h, h - 2.0));
            if osc.is_ready() {
                let v = out.unwrap();
                assert!(v > 0.0, "In uptrend, AROONOSC should be positive, got {v}");
            }
        }
    }

    #[test]
    fn test_streaming_aroon_osc_downtrend() {
        let mut osc = StreamingAroonOsc::new(5);
        for i in 0..15 {
            let h = 100.0 - i as f64 * 5.0;
            let out = osc.next((h, h - 2.0));
            if osc.is_ready() {
                let v = out.unwrap();
                assert!(v < 0.0, "In downtrend, AROONOSC should be negative, got {v}");
            }
        }
    }

    #[test]
    fn test_streaming_aroon_osc_reset() {
        let mut osc = StreamingAroonOsc::new(5);
        for i in 0..15 {
            osc.next((50.0 + i as f64, 45.0 + i as f64));
        }
        assert!(osc.is_ready());
        osc.reset();
        assert!(!osc.is_ready());
        assert_eq!(osc.count(), 0);
    }

    #[test]
    fn test_streaming_aroon_osc_meta() {
        let osc = StreamingAroonOsc::new(25);
        assert_eq!(StreamingAroonOsc::name(), "AROONOSC");
        assert_eq!(osc.warm_up_period(), 26);
    }
}
