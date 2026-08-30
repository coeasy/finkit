use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};
use crate::impl_standard_methods;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMedPrice {
    count: usize,
    last_value: Option<f64>,
}

impl Default for StreamingMedPrice {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingMedPrice {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingMedPrice {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.count += 1;
        let result = Some((bar.high() + bar.low()) / 2.0);
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

impl IndicatorMeta for StreamingMedPrice {
    fn name() -> &'static str {
        "MEDPRICE"
    }

    fn category() -> &'static str {
        "price_transform"
    }

    fn description() -> &'static str {
        "Median Price"
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
    fn test_streaming_medprice_basic() {
        let mut med = StreamingMedPrice::new();
        let v = med.next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0)).unwrap();
        assert!((v - 10.5).abs() < 1e-10);
        assert!(med.is_ready());
    }

    #[test]
    fn test_streaming_medprice_meta() {
        assert_eq!(StreamingMedPrice::name(), "MEDPRICE");
        assert_eq!(StreamingMedPrice::category(), "price_transform");
    }

    #[test]
    fn test_streaming_medprice_reset() {
        let mut med = StreamingMedPrice::new();
        med.next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0));
        assert!(med.is_ready());
        med.reset();
        assert!(!med.is_ready());
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let high = vec![12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0];
        let batch = crate::indicators::medprice(&high, &low).unwrap();

        let mut streaming = StreamingMedPrice::new();
        for i in 0..3 {
            let bar = OhlcvBar::new(0.0, high[i], low[i], 0.0, 0.0);
            let s = streaming.next(&bar).unwrap();
            assert!((s - batch[i]).abs() < 1e-10);
        }
    }
}
