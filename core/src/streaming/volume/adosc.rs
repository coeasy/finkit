use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};
use crate::impl_standard_methods;

/// Streaming Chaikin A/D Oscillator (ADOSC)
///
/// ADOSC = EMA(AD, fast_period) - EMA(AD, slow_period)
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingAdosc {
    fast_period: usize,
    slow_period: usize,
    ad_cumulative: f64,
    fast_ema: StreamingEma,
    slow_ema: StreamingEma,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingAdosc {
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self {
            fast_period,
            slow_period,
            ad_cumulative: 0.0,
            fast_ema: StreamingEma::new(fast_period),
            slow_ema: StreamingEma::new(slow_period),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingAdosc {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.count += 1;

        let range = bar.high() - bar.low();
        if range.abs() > 1e-15 {
            let clv = ((bar.close() - bar.low()) - (bar.high() - bar.close())) / range;
            self.ad_cumulative += clv * bar.volume();
        }

        let fast_val = self.fast_ema.next(self.ad_cumulative);
        let slow_val = self.slow_ema.next(self.ad_cumulative);

        let result = match (fast_val, slow_val) {
            (Some(f), Some(s)) => Some(f - s),
            _ => None,
        };
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.ad_cumulative = 0.0;
        self.fast_ema.reset();
        self.slow_ema.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool { self.slow_ema.is_ready() }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingAdosc {
    fn name() -> &'static str { "ADOSC" }
    fn category() -> &'static str { "volume" }
    fn description() -> &'static str { "Chaikin A/D Oscillator" }
    fn warm_up_period(&self) -> usize { self.slow_period }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::OhlcvBar;

    #[test]
    fn test_streaming_adosc_basic() {
        let mut adosc = StreamingAdosc::new(3, 5);
        let bars: Vec<OhlcvBar> = (0..20)
            .map(|i| {
                let base = 50.0 + (i as f64 * 0.3).sin() * 5.0;
                OhlcvBar::new(0.0, base + 2.0, base - 2.0, base + 1.0, 100.0 + i as f64 * 10.0)
            })
            .collect();

        let mut last_val = None;
        for bar in &bars {
            last_val = adosc.next(bar as &dyn Ohlcv);
        }
        assert!(last_val.is_some());
    }

    #[test]
    fn test_streaming_adosc_meta() {
        assert_eq!(StreamingAdosc::name(), "ADOSC");
        assert_eq!(StreamingAdosc::category(), "volume");
    }

    #[test]
    fn test_streaming_adosc_reset() {
        let mut adosc = StreamingAdosc::new(3, 5);
        let bar = OhlcvBar::new(0.0, 12.0, 8.0, 10.0, 100.0);
        for _ in 0..10 {
            adosc.next(&bar as &dyn Ohlcv);
        }
        assert!(adosc.is_ready());
        adosc.reset();
        assert!(!adosc.is_ready());
        assert_eq!(adosc.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 100;
        let highs: Vec<f64> = (0..n).map(|i| 50.0 + (i as f64 * 0.1).sin() * 5.0 + 2.0).collect();
        let lows: Vec<f64> = (0..n).map(|i| 50.0 + (i as f64 * 0.1).sin() * 5.0 - 2.0).collect();
        let closes: Vec<f64> = (0..n).map(|i| 50.0 + (i as f64 * 0.1).sin() * 5.0).collect();
        let volumes: Vec<f64> = (0..n).map(|i| 100.0 + (i as f64 * 0.2).cos() * 50.0).collect();

        let batch = crate::indicators::adosc(&highs, &lows, &closes, &volumes, 3, 10).unwrap();

        let mut streaming = StreamingAdosc::new(3, 10);
        for i in 0..n {
            let bar = OhlcvBar::new(0.0, highs[i], lows[i], closes[i], volumes[i]);
            if let (Some(s), false) = (streaming.next(&bar as &dyn Ohlcv), batch[i].is_nan()) {
                assert!(
                    (s - batch[i]).abs() < 1e-8,
                    "Mismatch at {i}: streaming={s}, batch={}",
                    batch[i]
                );
            }
        }
    }
}
