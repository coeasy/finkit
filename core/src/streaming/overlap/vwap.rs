use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingVwap {
    cumulative_tp_vol: f64,
    cumulative_vol: f64,
    count: usize,
    last_value: Option<f64>,
}

impl Default for StreamingVwap {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingVwap {
    pub fn new() -> Self {
        Self {
            cumulative_tp_vol: 0.0,
            cumulative_vol: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingVwap {
    #[inline]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip(self, bar))
    )]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        crate::streaming_measure!("vwap", self.count, {
            self.count += 1;
            let tp = (bar.high() + bar.low() + bar.close()) / 3.0;
            self.cumulative_tp_vol += tp * bar.volume();
            self.cumulative_vol += bar.volume();

            let result = if self.cumulative_vol.abs() > 1e-15 {
                Some(self.cumulative_tp_vol / self.cumulative_vol)
            } else {
                None
            };
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.cumulative_tp_vol = 0.0;
        self.cumulative_vol = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= 1
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingVwap {
    fn name() -> &'static str {
        "VWAP"
    }
    fn category() -> &'static str {
        "volume"
    }
    fn description() -> &'static str {
        "Volume Weighted Average Price"
    }
    fn warm_up_period(&self) -> usize {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::types::OhlcvBar;

    #[test]
    fn test_streaming_vwap_basic() {
        let mut vwap = StreamingVwap::new();
        let v1 = vwap
            .next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0))
            .unwrap();
        let tp1 = (12.0 + 9.0 + 11.0) / 3.0;
        assert!((v1 - tp1).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_vwap_meta() {
        assert_eq!(StreamingVwap::name(), "VWAP");
    }

    #[test]
    fn test_streaming_vwap_reset() {
        let mut vwap = StreamingVwap::new();
        vwap.next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0));
        assert!(vwap.is_ready());
        vwap.reset();
        assert!(!vwap.is_ready());
    }
}
