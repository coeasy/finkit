use crate::impl_standard_methods;
use crate::streaming::traits::{Ohlcv, StreamingIndicator};

/// Streaming (incremental) Weighted Close Price.
///
/// Weighted Close = `(High + Low + 2 * Close) / 4`. This is a stateless,
/// per-bar transform and is therefore ready from the very first bar.
///
/// Demonstrates how trivially a new indicator plugs into the unified
/// [`StreamingIndicator`] framework (B2: shared `next`/`reset`/`is_ready`
/// abstraction). See `crate::indicators::wclprice` for the batch equivalent.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingWclPrice {
    count: usize,
    last_value: Option<f64>,
}

impl Default for StreamingWclPrice {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingWclPrice {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingWclPrice {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.count += 1;
        let v = (bar.high() + bar.low() + 2.0 * bar.close()) / 4.0;
        let result = Some(v);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= 1
    }

    impl_standard_methods!();
}

impl crate::streaming::IndicatorMeta for StreamingWclPrice {
    fn name() -> &'static str {
        "WCLPRICE"
    }
    fn category() -> &'static str {
        "price_transform"
    }
    fn description() -> &'static str {
        "Weighted Close Price"
    }
    fn warm_up_period(&self) -> usize {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::wclprice;
    use crate::streaming::traits::IndicatorMeta;
    use crate::streaming::types::OhlcvBar;

    #[test]
    fn test_streaming_wclprice_basic() {
        let mut w = StreamingWclPrice::new();
        let bar = OhlcvBar::new(10.0, 12.0, 8.0, 9.0, 100.0);
        let v = w.next(&bar).unwrap();
        assert!((v - (12.0 + 8.0 + 2.0 * 9.0) / 4.0).abs() < 1e-12);
        assert_eq!(StreamingWclPrice::name(), "WCLPRICE");
    }

    #[test]
    fn test_streaming_wclprice_converges_to_batch() {
        let highs = vec![10.0, 12.0, 14.0, 16.0, 18.0];
        let lows = vec![8.0, 10.0, 12.0, 14.0, 16.0];
        let closes = vec![9.0, 11.0, 13.0, 15.0, 17.0];
        let expected = wclprice(&highs, &lows, &closes).unwrap();
        let mut w = StreamingWclPrice::new();
        for i in 0..highs.len() {
            let bar = OhlcvBar::new(0.0, highs[i], lows[i], closes[i], 0.0);
            let v = w.next(&bar).unwrap();
            assert!(
                (v - expected[i]).abs() < 1e-12,
                "mismatch at {i}: {} vs {}",
                v,
                expected[i]
            );
        }
    }

    #[test]
    fn test_streaming_wclprice_reset() {
        let mut w = StreamingWclPrice::new();
        let bar = OhlcvBar::new(0.0, 12.0, 8.0, 9.0, 0.0);
        assert!(w.next(&bar).is_some());
        assert!(w.is_ready());
        w.reset();
        assert!(!w.is_ready());
    }
}
