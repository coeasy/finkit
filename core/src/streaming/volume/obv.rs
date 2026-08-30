use crate::impl_standard_methods;
use crate::streaming::traits::{Ohlcv, StreamingIndicator};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingObv {
    prev_close: f64,
    obv: f64,
    count: usize,
    last_value: Option<f64>,
}

impl Default for StreamingObv {
    fn default() -> Self { Self::new() }
}

impl StreamingObv {
    pub fn new() -> Self {
        Self {
            prev_close: f64::NAN,
            obv: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingObv {
    #[inline]
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip(self, bar)))]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        crate::streaming_measure!("obv", self.count, {
            self.count += 1;
            let close = bar.close();
            let volume = bar.volume();

            if self.count == 1 {
                self.prev_close = close;
                self.obv = volume;
                let result = Some(self.obv);
                self.last_value = result;
                return result;
            }

            // Branchless: signum() returns -1.0 / 0.0 / 1.0 based on diff sign.
            // Matches TA-Lib OBV semantics (no change when close unchanged).
            let diff = close - self.prev_close;
            self.obv += diff.signum() * volume;
            self.prev_close = close;
            let result = Some(self.obv);
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.prev_close = f64::NAN;
        self.obv = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool { self.count >= 1 }
    impl_standard_methods!();
}

impl crate::streaming::IndicatorMeta for StreamingObv {
    fn name() -> &'static str { "OBV" }
    fn category() -> &'static str { "volume" }
    fn description() -> &'static str { "On Balance Volume" }
    fn warm_up_period(&self) -> usize { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;
    use crate::streaming::types::OhlcvBar;

    #[test]
    fn test_streaming_obv_basic() {
        let mut obv = StreamingObv::new();
        let v1 = obv.next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0)).unwrap();
        assert!((v1 - 100.0).abs() < 1e-10);
        let v2 = obv.next(&OhlcvBar::new(11.0, 13.0, 10.0, 12.0, 150.0)).unwrap();
        assert!((v2 - 250.0).abs() < 1e-10);
        let v3 = obv.next(&OhlcvBar::new(12.0, 14.0, 11.0, 10.0, 200.0)).unwrap();
        assert!((v3 - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_obv_meta() {
        assert_eq!(StreamingObv::name(), "OBV");
    }

    #[test]
    fn test_streaming_obv_reset() {
        let mut obv = StreamingObv::new();
        obv.next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0));
        assert!(obv.is_ready());
        obv.reset();
        assert!(!obv.is_ready());
    }
}
