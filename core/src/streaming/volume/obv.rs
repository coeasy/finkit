use crate::impl_standard_methods;
use crate::streaming::traits::{Ohlcv, StreamingIndicator};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingObv {
    prev_close: f64,
    obv: f64,
    count: usize,
    last_value: Option<f64>,
}

impl Default for StreamingObv {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingObv {
    pub fn new() -> Self {
        Self {
            prev_close: f64::NAN,
            obv: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingObv {
    #[inline]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip(self, bar))
    )]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        crate::streaming_measure!("obv", self.count, {
            self.count += 1;
            let close = bar.close();
            let volume = bar.volume();

            if self.count == 1 {
                self.prev_close = close;
                self.obv = volume;
                let result = Some(self.obv);
                self.last_value = result;
                return result;
            }

            // Canonical OBV: a flat bar (close unchanged) contributes nothing.
            // `f64::signum(0.0)` is `+1.0`, so `diff.signum() * volume` would
            // *add* `volume` on a flat bar and silently diverge from the batch
            // `obv` / `simd_obv` contract (and TA-Lib). Match that contract with
            // an explicit three-way step so the two implementations cannot
            // disagree again.
            let step = if close > self.prev_close {
                volume
            } else if close < self.prev_close {
                -volume
            } else {
                0.0
            };
            self.obv += step;
            self.prev_close = close;
            let result = Some(self.obv);
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.prev_close = f64::NAN;
        self.obv = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= 1
    }
    impl_standard_methods!();
}

impl crate::streaming::IndicatorMeta for StreamingObv {
    fn name() -> &'static str {
        "OBV"
    }
    fn category() -> &'static str {
        "volume"
    }
    fn description() -> &'static str {
        "On Balance Volume"
    }
    fn warm_up_period(&self) -> usize {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicators::obv as batch_obv;
    use crate::streaming::traits::IndicatorMeta;
    use crate::streaming::types::OhlcvBar;

    /// The streaming OBV must agree with the batch `obv`/`simd_obv` contract on
    /// the exact same bars — including a flat bar, which a naive
    /// `diff.signum() * volume` would push the wrong way. This is the regression
    /// gate that pins the two implementations together (see the fix note above).
    #[test]
    fn test_streaming_obv_matches_batch() {
        let close = vec![10.0, 11.0, 11.0, 10.5, 12.0, 12.0, 9.0];
        let volume = vec![100.0, 150.0, 200.0, 80.0, 130.0, 60.0, 95.0];

        let batch = batch_obv(&close, &volume).unwrap();

        let mut stream = StreamingObv::new();
        let mut streamed = Vec::with_capacity(close.len());
        for i in 0..close.len() {
            let bar = OhlcvBar::new(
                close[i],
                close[i] + 1.0,
                close[i] - 1.0,
                close[i],
                volume[i],
            );
            streamed.push(stream.next(&bar).unwrap());
        }

        assert_eq!(batch.len(), streamed.len());
        for (i, (b, s)) in batch.iter().zip(&streamed).enumerate() {
            assert!(
                (b - s).abs() < 1e-9,
                "OBV diverges at bar {i}: batch={b} stream={s}"
            );
        }
    }

    #[test]
    fn test_streaming_obv_basic() {
        let mut obv = StreamingObv::new();
        let v1 = obv
            .next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0))
            .unwrap();
        assert!((v1 - 100.0).abs() < 1e-10);
        let v2 = obv
            .next(&OhlcvBar::new(11.0, 13.0, 10.0, 12.0, 150.0))
            .unwrap();
        assert!((v2 - 250.0).abs() < 1e-10);
        let v3 = obv
            .next(&OhlcvBar::new(12.0, 14.0, 11.0, 10.0, 200.0))
            .unwrap();
        assert!((v3 - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_obv_flat_bar_unchanged() {
        // A bar with an unchanged close must not move OBV — matches the batch
        // `obv`/`simd_obv` contract. `f64::signum(0.0)` is `+1.0`, so a naive
        // `diff.signum() * volume` would wrongly add `volume` here.
        let mut obv = StreamingObv::new();
        let _ = obv
            .next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0))
            .unwrap(); // close=11, obv=100
        let v2 = obv
            .next(&OhlcvBar::new(11.0, 13.0, 10.0, 12.0, 150.0))
            .unwrap(); // close=12 > 11 -> +150 -> 250
        let v3 = obv
            .next(&OhlcvBar::new(11.0, 13.0, 10.0, 12.0, 200.0))
            .unwrap(); // close=12 == 12 (flat) -> unchanged -> 250
        assert!((v2 - 250.0).abs() < 1e-10);
        assert!(
            (v3 - v2).abs() < 1e-10,
            "flat bar must leave OBV unchanged: v2={v2} v3={v3}"
        );
    }

    #[test]
    fn test_streaming_obv_meta() {
        assert_eq!(StreamingObv::name(), "OBV");
    }

    #[test]
    fn test_streaming_obv_reset() {
        let mut obv = StreamingObv::new();
        obv.next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0));
        assert!(obv.is_ready());
        obv.reset();
        assert!(!obv.is_ready());
    }
}
