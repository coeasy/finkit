use crate::math::sar::SarState;
use crate::streaming::traits::{IndicatorMeta, Ohlcv};

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SarOutput {
    pub sar: f64,
    pub direction: i32,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingSar {
    state: SarState,
    last_value: Option<SarOutput>,
}

impl StreamingSar {
    pub fn new(acceleration: f64, maximum: f64) -> Self {
        Self {
            state: SarState::try_new(acceleration, maximum)
                .expect("StreamingSar requires non-negative acceleration and maximum"),
            last_value: None,
        }
    }

    #[inline]
    pub fn next(&mut self, bar: &dyn Ohlcv) -> Option<SarOutput> {
        let point = self.state.next(bar.high(), bar.low());
        let result = Some(SarOutput {
            sar: point.sar,
            direction: point.direction,
        });
        self.last_value = result;
        result
    }

    pub fn reset(&mut self) {
        self.state.reset();
        self.last_value = None;
    }

    pub fn is_ready(&self) -> bool {
        self.state.is_ready()
    }

    pub fn count(&self) -> usize {
        self.state.len()
    }

    pub fn value(&self) -> Option<SarOutput> {
        self.last_value
    }
}

impl IndicatorMeta for StreamingSar {
    fn name() -> &'static str {
        "SAR"
    }

    fn category() -> &'static str {
        "overlap"
    }

    fn description() -> &'static str {
        "Parabolic SAR"
    }

    fn warm_up_period(&self) -> usize {
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::types::OhlcvBar;

    #[test]
    fn test_streaming_sar_first_bar_is_talib_warmup() {
        let mut sar = StreamingSar::new(0.02, 0.2);
        let out = sar
            .next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0))
            .unwrap();
        assert!(out.sar.is_nan());
        assert_eq!(out.direction, 0);
        assert!(!sar.is_ready());

        let second = sar
            .next(&OhlcvBar::new(11.0, 13.0, 10.0, 12.0, 100.0))
            .unwrap();
        assert!(second.sar.is_finite());
        assert!(sar.is_ready());
    }

    #[test]
    fn test_streaming_sar_meta() {
        let sar = StreamingSar::new(0.02, 0.2);
        assert_eq!(StreamingSar::name(), "SAR");
        assert_eq!(StreamingSar::category(), "overlap");
        assert_eq!(sar.warm_up_period(), 2);
    }

    #[test]
    fn test_streaming_sar_reset() {
        let mut sar = StreamingSar::new(0.02, 0.2);
        for i in 0..5 {
            sar.next(&OhlcvBar::new(
                10.0 + i as f64,
                12.0 + i as f64,
                9.0 + i as f64,
                11.0 + i as f64,
                100.0,
            ));
        }
        assert!(sar.is_ready());
        sar.reset();
        assert!(!sar.is_ready());
        assert_eq!(sar.count(), 0);
    }

    #[test]
    fn test_streaming_vs_canonical_batch_exactly() {
        let high: Vec<f64> = (0..30)
            .map(|i| 55.0 + (i as f64 * 0.3).sin() * 5.0)
            .collect();
        let low: Vec<f64> = high.iter().map(|h| h - 2.0).collect();
        let batch = crate::math::sar::sar(&high, &low, 0.02, 0.2).unwrap();

        let mut streaming = StreamingSar::new(0.02, 0.2);
        for i in 0..30 {
            let bar = OhlcvBar::new(0.0, high[i], low[i], 0.0, 0.0);
            let s = streaming.next(&bar).unwrap();
            if batch[i].is_nan() {
                assert!(s.sar.is_nan());
            } else {
                assert_eq!(s.sar, batch[i], "SAR mismatch at {i}");
            }
        }
    }
}
