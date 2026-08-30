use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

/// Streaming Put/Call Ratio.
///
/// PCR = put_volume / call_volume
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingPutCallRatio {
    count: usize,
    last_value: Option<f64>,
}

impl StreamingPutCallRatio {
    pub fn new() -> Self {
        Self {
            count: 0,
            last_value: None,
        }
    }
}

impl Default for StreamingPutCallRatio {
    fn default() -> Self { Self::new() }
}

/// Input: (put_volume, call_volume)
impl StreamingIndicator<(f64, f64)> for StreamingPutCallRatio {
    #[inline]
    fn next(&mut self, input: (f64, f64)) -> Option<f64> {
        let (put_vol, call_vol) = input;
        self.count += 1;

        let val = if call_vol.abs() > 1e-15 {
            put_vol / call_vol
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

impl IndicatorMeta for StreamingPutCallRatio {
    fn name() -> &'static str { "PUT_CALL_RATIO" }
    fn category() -> &'static str { "sentiment" }
    fn description() -> &'static str { "Put/Call Ratio" }
    fn warm_up_period(&self) -> usize { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_put_call_ratio() {
        let mut pcr = StreamingPutCallRatio::new();
        // 5000 puts / 10000 calls = 0.5
        assert_eq!(pcr.next((5000.0, 10000.0)), Some(0.5));
        // 15000 / 10000 = 1.5
        assert_eq!(pcr.next((15000.0, 10000.0)), Some(1.5));
    }

    #[test]
    fn test_streaming_put_call_ratio_zero_calls() {
        let mut pcr = StreamingPutCallRatio::new();
        assert_eq!(pcr.next((5000.0, 0.0)), Some(0.0));
    }

    #[test]
    fn test_streaming_put_call_ratio_reset() {
        let mut pcr = StreamingPutCallRatio::new();
        pcr.next((5000.0, 10000.0));
        pcr.reset();
        assert!(!pcr.is_ready());
    }
}
