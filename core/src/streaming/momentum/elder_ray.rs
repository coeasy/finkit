use crate::impl_standard_methods;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

/// Elder Ray output: bull power and bear power.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ElderRayOutput {
    pub bull_power: f64,
    pub bear_power: f64,
}

/// Streaming Elder Ray Index.
///
/// Bull Power = High - EMA(Close), Bear Power = Low - EMA(Close).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingElderRay {
    period: usize,
    ema: StreamingEma,
    count: usize,
    last_value: Option<ElderRayOutput>,
}

impl StreamingElderRay {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            ema: StreamingEma::new(period),
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64, f64), ElderRayOutput> for StreamingElderRay {
    #[inline]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<ElderRayOutput> {
        let (high, low, close) = input;
        self.count += 1;

        let ema_val = self.ema.next(close);
        let Some(ema) = ema_val else {
            self.last_value = None;
            return None;
        };

        let result = ElderRayOutput {
            bull_power: high - ema,
            bear_power: low - ema,
        };
        self.last_value = Some(result);
        Some(result)
    }

    fn reset(&mut self) {
        self.ema.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.ema.is_ready()
    }

    impl_standard_methods!(output = ElderRayOutput);
}

impl IndicatorMeta for StreamingElderRay {
    fn name() -> &'static str {
        "ElderRay"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Elder Ray Index"
    }
    fn warm_up_period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_elder_ray_basic() {
        let mut er = StreamingElderRay::new(13);
        let data: Vec<(f64, f64, f64)> = (0..30)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.3).sin() * 10.0;
                (h, h - 4.0, h - 2.0)
            })
            .collect();
        let mut last = None;
        for &d in &data {
            last = er.next(d);
        }
        let v = last.unwrap();
        assert!(!v.bull_power.is_nan());
        assert!(!v.bear_power.is_nan());
    }

    #[test]
    fn test_streaming_elder_ray_uptrend() {
        let mut er = StreamingElderRay::new(13);
        let data: Vec<(f64, f64, f64)> = (0..30)
            .map(|i| {
                let base = 100.0 + i as f64 * 2.0;
                (base + 3.0, base - 1.0, base + 1.0)
            })
            .collect();
        let mut last = None;
        for &d in &data {
            last = er.next(d);
        }
        let v = last.unwrap();
        assert!(
            v.bull_power > 0.0,
            "Bull power should be positive in uptrend, got {}",
            v.bull_power
        );
    }

    #[test]
    fn test_streaming_elder_ray_reset() {
        let mut er = StreamingElderRay::new(13);
        for i in 0..30 {
            let h = 50.0 + i as f64;
            er.next((h, h - 4.0, h - 2.0));
        }
        assert!(er.is_ready());
        er.reset();
        assert!(!er.is_ready());
        assert_eq!(er.count(), 0);
    }

    #[test]
    fn test_streaming_elder_ray_meta() {
        let er = StreamingElderRay::new(13);
        assert_eq!(StreamingElderRay::name(), "ElderRay");
        assert_eq!(er.warm_up_period(), 13);
    }
}
