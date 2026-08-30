use crate::streaming::volatility::atr::StreamingAtr;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct KeltnerOutput {
    pub upper: f64,
    pub middle: f64,
    pub lower: f64,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingKeltner {
    ema: StreamingEma,
    atr: StreamingAtr,
    multiplier: f64,
    count: usize,
    last_value: Option<KeltnerOutput>,
}

impl StreamingKeltner {
    pub fn new(ema_period: usize, atr_period: usize, multiplier: f64) -> Self {
        Self {
            ema: StreamingEma::new(ema_period),
            atr: StreamingAtr::new(atr_period),
            multiplier,
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    pub fn next(&mut self, bar: &dyn Ohlcv) -> Option<KeltnerOutput> {
        self.count += 1;
        let mid = self.ema.next(bar.close());
        let atr_val = self.atr.next((bar.high(), bar.low(), bar.close()));
        let (Some(mid), Some(atr_val)) = (mid, atr_val) else {
            self.last_value = None;
            return None;
        };

        let result = Some(KeltnerOutput {
            upper: mid + self.multiplier * atr_val,
            middle: mid,
            lower: mid - self.multiplier * atr_val,
        });
        self.last_value = result;
        result
    }

    pub fn reset(&mut self) {
        self.ema.reset();
        self.atr.reset();
        self.count = 0;
        self.last_value = None;
    }

    pub fn is_ready(&self) -> bool { self.ema.is_ready() && self.atr.is_ready() }
    pub fn count(&self) -> usize { self.count }

    pub fn value(&self) -> Option<KeltnerOutput> {
        self.last_value
    }
}

impl IndicatorMeta for StreamingKeltner {
    fn name() -> &'static str { "Keltner" }
    fn category() -> &'static str { "volatility" }
    fn description() -> &'static str { "Keltner Channels" }
    fn warm_up_period(&self) -> usize {
        self.ema.warm_up_period().max(self.atr.warm_up_period())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::types::OhlcvBar;

    #[test]
    fn test_streaming_keltner_basic() {
        let mut k = StreamingKeltner::new(5, 5, 2.0);
        for i in 0..10 {
            let bar = OhlcvBar::new(10.0 + i as f64, 12.0 + i as f64, 9.0 + i as f64, 11.0 + i as f64, 100.0);
            if let Some(out) = k.next(&bar) {
                assert!(out.upper > out.middle);
                assert!(out.middle > out.lower);
            }
        }
    }

    #[test]
    fn test_streaming_keltner_default() {
        let k = StreamingKeltner::new(20, 10, 2.0);
        assert_eq!(StreamingKeltner::name(), "Keltner");
        assert_eq!(k.warm_up_period(), 20);
    }

    #[test]
    fn test_streaming_keltner_meta() {
        assert_eq!(StreamingKeltner::name(), "Keltner");
    }

    #[test]
    fn test_streaming_keltner_reset() {
        let mut k = StreamingKeltner::new(3, 3, 2.0);
        for i in 0..10 {
            k.next(&OhlcvBar::new(10.0 + i as f64, 12.0 + i as f64, 9.0 + i as f64, 11.0 + i as f64, 100.0));
        }
        assert!(k.is_ready());
        k.reset();
        assert!(!k.is_ready());
    }

    #[test]
    fn test_streaming_keltner_formula() {
        let mut k = StreamingKeltner::new(3, 3, 1.5);
        let bars = vec![
            OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0),
            OhlcvBar::new(11.0, 13.0, 10.0, 12.0, 100.0),
            OhlcvBar::new(12.0, 14.0, 11.0, 13.0, 100.0),
            OhlcvBar::new(13.0, 15.0, 12.0, 14.0, 100.0),
        ];
        let mut out = None;
        for bar in &bars { out = k.next(bar); }
        assert!(out.is_some());
    }
}
