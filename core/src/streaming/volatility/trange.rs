use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Streaming True Range.
///
/// # Convergence
///
/// Bar zero has no predecessor, so it has no `previous close` to measure a gap
/// against. The shipped convention — stated in `volatility::trange_into` and
/// relied on by ATR's warm-up — is `NaN` on row zero, not `high - low`. This
/// kernel follows it: the first emitted value is bar 1, and
/// `registry::by_id("TRANGE").convergence` is 2 for the same reason.
///
/// Emitting `high - low` on bar 0 instead would look harmless and would make
/// the incremental path disagree with the batch path on exactly one row — the
/// kind of difference the `streaming == batch` gate exists to catch.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingTrange {
    prev_close: f64,
    count: usize,
    last_value: Option<f64>,
}

impl Default for StreamingTrange {
    fn default() -> Self {
        Self::new()
    }
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
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip(self, input))
    )]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<f64> {
        crate::streaming_measure!("trange", self.count, {
            let (high, low, close) = input;
            self.count += 1;

            // TA-Lib row zero: no previous close, so no true range.
            if self.count == 1 {
                self.prev_close = close;
                self.last_value = None;
                return None;
            }

            let hl = high - low;
            let hc = (high - self.prev_close).abs();
            let lc = (low - self.prev_close).abs();

            self.prev_close = close;
            let result = Some(hl.max(hc).max(lc));
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.prev_close = f64::NAN;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.last_value.is_some()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingTrange {
    fn name() -> &'static str {
        "TRANGE"
    }
    fn category() -> &'static str {
        "volatility"
    }
    fn description() -> &'static str {
        "True Range"
    }
    /// Bar 0 is a warm-up row, so two bars are needed for the first value.
    fn warm_up_period(&self) -> usize {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_trange_basic() {
        let mut tr = StreamingTrange::new();
        assert_eq!(
            tr.next((12.0, 9.0, 11.0)),
            None,
            "bar zero has no previous close, so it must stay NaN"
        );
        let v2 = tr.next((14.0, 10.0, 13.0)).unwrap();
        assert!((v2 - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_trange_meta() {
        assert_eq!(StreamingTrange::name(), "TRANGE");
        assert_eq!(
            StreamingTrange::new().warm_up_period(),
            crate::streaming::registry::by_id("TRANGE")
                .expect("TRANGE is registered")
                .convergence
        );
    }

    #[test]
    fn test_streaming_trange_reset() {
        let mut tr = StreamingTrange::new();
        tr.next((12.0, 9.0, 11.0));
        tr.next((14.0, 10.0, 13.0));
        assert!(tr.is_ready());
        tr.reset();
        assert!(!tr.is_ready());
        assert_eq!(tr.next((12.0, 9.0, 11.0)), None);
    }
}
