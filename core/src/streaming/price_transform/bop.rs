use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;
use crate::streaming::Ohlcv;

/// Streaming Balance of Power (BOP).
///
/// BOP = (Close - Open) / (High - Low)
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingBop {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingBop {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl Default for StreamingBop {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ohlcv> StreamingIndicator<T> for StreamingBop {
    #[inline]
    fn next(&mut self, input: T) -> Option<f64> {
        self.count += 1;
        let range = input.high() - input.low();
        let val = if range.abs() > 1e-15 {
            (input.close() - input.open()) / range
        } else {
            0.0
        };
        self.last_value = Some(val);
        Some(val)
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

impl IndicatorMeta for StreamingBop {
    fn name() -> &'static str { "BOP" }
    fn category() -> &'static str { "momentum" }
    fn description() -> &'static str { "Balance of Power" }
    fn warm_up_period(&self) -> usize { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::OhlcvBar;
    use crate::streaming::traits::StreamingIndicator;

    #[test]
    fn test_streaming_bop_basic() {
        let mut bop = StreamingBop::new();
        // Bullish bar: close > open
        let bar = OhlcvBar::new(100.0, 110.0, 95.0, 108.0, 1000.0);
        let val = bop.next(bar);
        // (108 - 100) / (110 - 95) = 8/15
        assert!((val.unwrap() - 8.0 / 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_bop_zero_range() {
        let mut bop = StreamingBop::new();
        let bar = OhlcvBar::new(100.0, 100.0, 100.0, 100.0, 1000.0);
        assert_eq!(bop.next(bar), Some(0.0));
    }

    #[test]
    fn test_streaming_bop_reset() {
        let mut bop = StreamingBop::new();
        let bar = OhlcvBar::new(100.0, 110.0, 95.0, 105.0, 1000.0);
        <StreamingBop as StreamingIndicator<OhlcvBar>>::next(&mut bop, bar);
        assert!(<StreamingBop as StreamingIndicator<OhlcvBar>>::is_ready(&bop));
        <StreamingBop as StreamingIndicator<OhlcvBar>>::reset(&mut bop);
        assert!(!<StreamingBop as StreamingIndicator<OhlcvBar>>::is_ready(&bop));
        assert_eq!(<StreamingBop as StreamingIndicator<OhlcvBar>>::value(&bop), None);
    }
}
