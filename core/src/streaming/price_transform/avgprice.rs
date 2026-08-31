use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingAvgPrice {
    count: usize,
    last_value: Option<f64>,
}

impl Default for StreamingAvgPrice {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingAvgPrice {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingAvgPrice {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.count += 1;
        let result = Some((bar.open() + bar.high() + bar.low() + bar.close()) / 4.0);
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

impl IndicatorMeta for StreamingAvgPrice {
    fn name() -> &'static str {
        "AVGPRICE"
    }

    fn category() -> &'static str {
        "price_transform"
    }

    fn description() -> &'static str {
        "Average Price"
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
    fn test_streaming_avgprice_basic() {
        let mut avg = StreamingAvgPrice::new();
        let v = avg
            .next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0))
            .unwrap();
        assert!((v - 10.5).abs() < 1e-10);
        assert!(avg.is_ready());
    }

    #[test]
    fn test_streaming_avgprice_meta() {
        assert_eq!(StreamingAvgPrice::name(), "AVGPRICE");
        assert_eq!(StreamingAvgPrice::category(), "price_transform");
    }

    #[test]
    fn test_streaming_avgprice_reset() {
        let mut avg = StreamingAvgPrice::new();
        avg.next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0));
        assert!(avg.is_ready());
        avg.reset();
        assert!(!avg.is_ready());
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let open = vec![10.0, 11.0, 12.0];
        let high = vec![12.0, 13.0, 14.0];
        let low = vec![9.0, 10.0, 11.0];
        let close = vec![11.0, 12.0, 13.0];
        let batch = crate::indicators::avgprice(&open, &high, &low, &close).unwrap();

        let mut streaming = StreamingAvgPrice::new();
        for i in 0..3 {
            let bar = OhlcvBar::new(open[i], high[i], low[i], close[i], 0.0);
            let s = streaming.next(&bar).unwrap();
            assert!((s - batch[i]).abs() < 1e-10);
        }
    }
}
