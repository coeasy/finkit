use crate::streaming::traits::{Ohlcv, StreamingIndicator};
use crate::streaming::volatility::atr::StreamingAtr;
use crate::{impl_indicator_meta, impl_standard_methods};

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

impl_indicator_meta!(
    StreamingNatr,
    "NATR",
    "volatility",
    "Normalized Average True Range"
);

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
        for bar in &bars[..2] {
            assert_eq!(natr.next(bar), None);
        }
        let v = natr.next(&bars[2]).unwrap();
        assert!(v > 0.0);
    }

    #[test]
    fn test_streaming_natr_meta() {
        assert_eq!(StreamingNatr::name(), "NATR");
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
