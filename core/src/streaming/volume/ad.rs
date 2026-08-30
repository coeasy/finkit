use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};
use crate::impl_standard_methods;

/// Streaming Accumulation/Distribution Line
///
/// AD = cumulative sum of CLV * Volume
/// where CLV = ((Close - Low) - (High - Close)) / (High - Low)
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingAd {
    cumulative: f64,
    count: usize,
    last_value: Option<f64>,
}

impl Default for StreamingAd {
    fn default() -> Self { Self::new() }
}

impl StreamingAd {
    pub fn new() -> Self {
        Self {
            cumulative: 0.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingAd {
    #[inline]
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "trace", skip(self, bar)))]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        crate::streaming_measure!("ad", self.count, {
            self.count += 1;
            let range = bar.high() - bar.low();
            if range.abs() > 1e-15 {
                let clv = ((bar.close() - bar.low()) - (bar.high() - bar.close())) / range;
                self.cumulative += clv * bar.volume();
            }
            let result = Some(self.cumulative);
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.cumulative = 0.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool { self.count >= 1 }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingAd {
    fn name() -> &'static str { "AD" }
    fn category() -> &'static str { "volume" }
    fn description() -> &'static str { "Accumulation/Distribution Line" }
    fn warm_up_period(&self) -> usize { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::OhlcvBar;

    #[test]
    fn test_streaming_ad_basic() {
        let mut ad = StreamingAd::new();
        let bar1 = OhlcvBar::new(0.0, 10.0, 8.0, 9.0, 100.0);
        let v1 = ad.next(&bar1 as &dyn Ohlcv).unwrap();
        // CLV = ((9-8) - (10-9)) / (10-8) = 0/2 = 0
        assert!((v1 - 0.0).abs() < 1e-10);

        let bar2 = OhlcvBar::new(0.0, 12.0, 10.0, 11.0, 200.0);
        let v2 = ad.next(&bar2 as &dyn Ohlcv).unwrap();
        // CLV = ((11-10) - (12-11)) / (12-10) = 0/2 = 0
        assert!((v2 - 0.0).abs() < 1e-10);

        let bar3 = OhlcvBar::new(0.0, 14.0, 12.0, 14.0, 300.0);
        let v3 = ad.next(&bar3 as &dyn Ohlcv).unwrap();
        // CLV = ((14-12) - (14-14)) / (14-12) = 2/2 = 1.0, AD += 300
        assert!((v3 - 300.0).abs() < 1e-10);
    }

    #[test]
    fn test_streaming_ad_meta() {
        assert_eq!(StreamingAd::name(), "AD");
        assert_eq!(StreamingAd::category(), "volume");
    }

    #[test]
    fn test_streaming_ad_reset() {
        let mut ad = StreamingAd::new();
        let bar = OhlcvBar::new(0.0, 10.0, 8.0, 10.0, 100.0);
        ad.next(&bar as &dyn Ohlcv);
        assert!(ad.is_ready());
        ad.reset();
        assert!(!ad.is_ready());
        assert_eq!(ad.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 100;
        let highs: Vec<f64> = (0..n).map(|i| 50.0 + (i as f64 * 0.1).sin() * 5.0 + 2.0).collect();
        let lows: Vec<f64> = (0..n).map(|i| 50.0 + (i as f64 * 0.1).sin() * 5.0 - 2.0).collect();
        let closes: Vec<f64> = (0..n).map(|i| 50.0 + (i as f64 * 0.1).sin() * 5.0).collect();
        let volumes: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64 * 0.2).cos() * 50.0).collect();

        let batch = crate::indicators::ad(&highs, &lows, &closes, &volumes).unwrap();

        let mut streaming = StreamingAd::new();
        for i in 0..n {
            let bar = OhlcvBar::new(0.0, highs[i], lows[i], closes[i], volumes[i]);
            if let Some(s) = streaming.next(&bar as &dyn Ohlcv) {
                assert!(
                    (s - batch[i]).abs() < 1e-8,
                    "Mismatch at {i}: streaming={s}, batch={}",
                    batch[i]
                );
            }
        }
    }
}
