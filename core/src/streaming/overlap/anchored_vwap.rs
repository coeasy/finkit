use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};
use crate::impl_standard_methods;

/// Streaming Anchored VWAP
///
/// Like VWAP but can be reset at an anchor point.
/// Accumulates typical_price * volume / cumulative_volume from the anchor.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingAnchoredVwap {
    cumulative_tp_vol: f64,
    cumulative_vol: f64,
    count: usize,
    last_value: Option<f64>,
}

impl Default for StreamingAnchoredVwap {
    fn default() -> Self { Self::new() }
}

impl StreamingAnchoredVwap {
    pub fn new() -> Self {
        Self {
            cumulative_tp_vol: 0.0,
            cumulative_vol: 0.0,
            count: 0,
            last_value: None,
        }
    }

    /// Reset accumulation to start a new anchor point
    pub fn anchor(&mut self) {
        self.cumulative_tp_vol = 0.0;
        self.cumulative_vol = 0.0;
        self.count = 0;
        self.last_value = None;
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingAnchoredVwap {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
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
    }

    fn reset(&mut self) {
        self.anchor();
    }

    fn is_ready(&self) -> bool { self.count >= 1 }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingAnchoredVwap {
    fn name() -> &'static str { "AnchoredVWAP" }
    fn category() -> &'static str { "volume" }
    fn description() -> &'static str { "Anchored Volume Weighted Average Price" }
    fn warm_up_period(&self) -> usize { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::OhlcvBar;

    #[test]
    fn test_streaming_anchored_vwap_basic() {
        let mut avwap = StreamingAnchoredVwap::new();
        let bar1 = OhlcvBar::new(0.0, 12.0, 8.0, 10.0, 100.0);
        let v1 = avwap.next(&bar1 as &dyn Ohlcv).unwrap();
        // tp = (12+8+10)/3 = 10.0
        assert!((v1 - 10.0).abs() < 1e-10);

        let bar2 = OhlcvBar::new(0.0, 14.0, 10.0, 12.0, 200.0);
        let v2 = avwap.next(&bar2 as &dyn Ohlcv).unwrap();
        // tp = (14+10+12)/3 = 12.0
        // avwap = (10*100 + 12*200) / (100+200) = 3400/300 = 11.333..
        assert!((v2 - 3400.0 / 300.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_anchored_vwap_anchor() {
        let mut avwap = StreamingAnchoredVwap::new();
        let bar = OhlcvBar::new(0.0, 12.0, 8.0, 10.0, 100.0);
        avwap.next(&bar as &dyn Ohlcv);
        avwap.next(&bar as &dyn Ohlcv);
        assert_eq!(avwap.count(), 2);

        avwap.anchor();
        assert_eq!(avwap.count(), 0);
        assert!(!avwap.is_ready());

        let v = avwap.next(&bar as &dyn Ohlcv).unwrap();
        assert!((v - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_anchored_vwap_meta() {
        assert_eq!(StreamingAnchoredVwap::name(), "AnchoredVWAP");
        assert_eq!(StreamingAnchoredVwap::category(), "volume");
    }

    #[test]
    fn test_streaming_anchored_vwap_reset() {
        let mut avwap = StreamingAnchoredVwap::new();
        let bar = OhlcvBar::new(0.0, 12.0, 8.0, 10.0, 100.0);
        avwap.next(&bar as &dyn Ohlcv);
        assert!(avwap.is_ready());
        avwap.reset();
        assert!(!avwap.is_ready());
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 50;
        let highs: Vec<f64> = (0..n).map(|i| 50.0 + (i as f64 * 0.1).sin() * 5.0 + 2.0).collect();
        let lows: Vec<f64> = (0..n).map(|i| 50.0 + (i as f64 * 0.1).sin() * 5.0 - 2.0).collect();
        let closes: Vec<f64> = (0..n).map(|i| 50.0 + (i as f64 * 0.1).sin() * 5.0).collect();
        let volumes: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64 * 0.2).cos() * 50.0).collect();

        let batch = crate::indicators::anchored_vwap(&highs, &lows, &closes, &volumes, 0).unwrap();

        let mut streaming = StreamingAnchoredVwap::new();
        for i in 0..n {
            let bar = OhlcvBar::new(0.0, highs[i], lows[i], closes[i], volumes[i]);
            if let (Some(s), false) = (streaming.next(&bar as &dyn Ohlcv), batch[i].is_nan()) {
                assert!(
                    (s - batch[i]).abs() < 1e-8,
                    "Mismatch at {i}: streaming={s}, batch={}",
                    batch[i]
                );
            }
        }
    }
}
