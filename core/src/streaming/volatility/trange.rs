use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingTrange {
    prev_close: f64,
    count: usize,
    last_value: Option<f64>,
}

impl Default for StreamingTrange {
    fn default() -> Self { Self::new() }
}

impl StreamingTrange {
    pub fn new() -> Self {
        Self {
            prev_close: f64::NAN,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64, f64)> for StreamingTrange {
    #[inline]
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip(self, input)))]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<f64> {
        crate::streaming_measure!("trange", self.count, {
            let (high, low, close) = input;
            self.count += 1;

            let tr = if self.prev_close.is_nan() {
                high - low
            } else {
                let hl = high - low;
                let hc = (high - self.prev_close).abs();
                let lc = (low - self.prev_close).abs();
                hl.max(hc).max(lc)
            };

            self.prev_close = close;
            let result = Some(tr);
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.prev_close = f64::NAN;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool { self.count >= 1 }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingTrange {
    fn name() -> &'static str { "TRANGE" }
    fn category() -> &'static str { "volatility" }
    fn description() -> &'static str { "True Range" }
    fn warm_up_period(&self) -> usize { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_trange_basic() {
        let mut tr = StreamingTrange::new();
        let v1 = tr.next((12.0, 9.0, 11.0)).unwrap();
        assert!((v1 - 3.0).abs() < 1e-10);
        let v2 = tr.next((14.0, 10.0, 13.0)).unwrap();
        assert!((v2 - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_trange_meta() {
        assert_eq!(StreamingTrange::name(), "TRANGE");
    }

    #[test]
    fn test_streaming_trange_reset() {
        let mut tr = StreamingTrange::new();
        tr.next((12.0, 9.0, 11.0));
        assert!(tr.is_ready());
        tr.reset();
        assert!(!tr.is_ready());
    }
}
