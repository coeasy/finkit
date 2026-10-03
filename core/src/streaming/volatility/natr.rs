use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, Ohlcv, StreamingIndicator};
use crate::streaming::volatility::atr::StreamingAtr;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingNatr {
    atr: StreamingAtr,
    period: usize,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingNatr {
    pub fn new(period: usize) -> Self {
        Self {
            atr: StreamingAtr::new(period),
            period,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv> for StreamingNatr {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.count += 1;
        let atr_val = self.atr.next((bar.high(), bar.low(), bar.close()))?;
        let close = bar.close();

        if close.abs() < 1e-15 {
            self.last_value = None;
            return None;
        }

        let result = Some((atr_val / close) * 100.0);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.atr.reset();
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.atr.is_ready()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingNatr {
    fn name() -> &'static str {
        "NATR"
    }
    fn category() -> &'static str {
        "volatility"
    }
    fn description() -> &'static str {
        "Normalized Average True Range"
    }
    /// NATR is ATR rescaled by close, so it inherits ATR's warm-up exactly.
    fn warm_up_period(&self) -> usize {
        self.atr.warm_up_period()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;
    use crate::streaming::types::OhlcvBar;

    #[test]
    fn test_streaming_natr_basic() {
        let mut natr = StreamingNatr::new(3);
        let bars = [
            OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0),
            OhlcvBar::new(11.0, 13.0, 10.0, 12.0, 100.0),
            OhlcvBar::new(12.0, 14.0, 11.0, 13.0, 100.0),
            OhlcvBar::new(13.0, 15.0, 12.0, 14.0, 100.0),
        ];
        for bar in &bars[..3] {
            assert_eq!(natr.next(bar), None);
        }
        let v = natr.next(&bars[3]).unwrap();
        assert!(v > 0.0);
    }

    #[test]
    fn test_streaming_natr_meta() {
        assert_eq!(StreamingNatr::name(), "NATR");
        assert_eq!(
            StreamingNatr::new(14).warm_up_period(),
            crate::streaming::registry::by_id("NATR")
                .expect("NATR is registered")
                .convergence
        );
    }

    #[test]
    fn test_streaming_natr_reset() {
        let mut natr = StreamingNatr::new(3);
        for i in 0..5 {
            natr.next(&OhlcvBar::new(
                i as f64,
                i as f64 + 2.0,
                i as f64 - 1.0,
                i as f64 + 1.0,
                100.0,
            ));
        }
        assert!(natr.is_ready());
        natr.reset();
        assert!(!natr.is_ready());
    }
}
