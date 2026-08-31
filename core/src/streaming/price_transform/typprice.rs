use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingTypPrice {
    count: usize,
    last_value: Option<f64>,
}

impl Default for StreamingTypPrice {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingTypPrice {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingTypPrice {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.count += 1;
        let result = Some((bar.high() + bar.low() + bar.close()) / 3.0);
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

impl IndicatorMeta for StreamingTypPrice {
    fn name() -> &'static str {
        "TYPPRICE"
    }

    fn category() -> &'static str {
        "price_transform"
    }

    fn description() -> &'static str {
        "Typical Price"
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
    fn test_streaming_typprice_basic() {
        let mut typ = StreamingTypPrice::new();
        let v = typ
            .next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0))
            .unwrap();
        assert!((v - (12.0 + 9.0 + 11.0) / 3.0).abs() < 1e-10);
        assert!(typ.is_ready());
    }

    #[test]
    fn test_streaming_typprice_meta() {
        assert_eq!(StreamingTypPrice::name(), "TYPPRICE");
        assert_eq!(StreamingTypPrice::category(), "price_transform");
    }

    #[test]
    fn test_streaming_typprice_reset() {
        let mut typ = StreamingTypPrice::new();
        typ.next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0));
        assert!(typ.is_ready());
        typ.reset();
        assert!(!typ.is_ready());
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let high = vec![12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![11.0, 12.0, 13.0];
        let batch = crate::indicators::typprice(&high, &low, &close).unwrap();

        let mut streaming = StreamingTypPrice::new();
        for i in 0..3 {
            let bar = OhlcvBar::new(0.0, high[i], low[i], close[i], 0.0);
            let s = streaming.next(&bar).unwrap();
            assert!((s - batch[i]).abs() < 1e-10);
        }
    }
}
