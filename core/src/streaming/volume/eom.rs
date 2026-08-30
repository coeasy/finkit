use crate::streaming::overlap::sma::StreamingSma;
use crate::streaming::traits::{StreamingIndicator};
use crate::impl_standard_methods;
use crate::{impl_indicator_meta};
use crate::streaming::Ohlcv;

const EOM_DIVISOR: f64 = 100_000_000.0;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingEom {
    period: usize,
    sma: StreamingSma,
    prev_high: f64,
    prev_low: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingEom {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            sma: StreamingSma::new(period),
            prev_high: f64::NAN,
            prev_low: f64::NAN,
            count: 0,
            last_value: None,
        }
    }

    #[inline]
    fn raw_eom(high: f64, low: f64, prev_high: f64, prev_low: f64, volume: f64) -> f64 {
        if high.is_nan() || low.is_nan() || prev_high.is_nan() || prev_low.is_nan() || volume.is_nan() {
            return f64::NAN;
        }

        let distance = (high + low) / 2.0 - (prev_high + prev_low) / 2.0;
        let range = high - low;

        if range.abs() <= 1e-15 || volume.abs() <= 1e-15 {
            0.0
        } else {
            let box_ratio = (volume / EOM_DIVISOR) / range;
            if box_ratio.abs() <= 1e-15 {
                0.0
            } else {
                distance / box_ratio
            }
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv, f64> for StreamingEom {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.count += 1;
        let high = bar.high();
        let low = bar.low();
        let volume = bar.volume();

        let raw = if self.count == 1 {
            0.0
        } else {
            Self::raw_eom(high, low, self.prev_high, self.prev_low, volume)
        };

        self.prev_high = high;
        self.prev_low = low;

        let result = self.sma.next(raw);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.sma.reset();
        self.prev_high = f64::NAN;
        self.prev_low = f64::NAN;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.sma.is_ready()
    }

    impl_standard_methods!();
}

impl_indicator_meta!(StreamingEom, "EOM", "volume", "Ease of Movement");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::traits::IndicatorMeta;
    use crate::streaming::OhlcvBar;
    use approx::assert_relative_eq;

    #[test]
    fn test_streaming_eom_basic() {
        let mut eom = StreamingEom::new(3);
        let bars = [
            OhlcvBar::new(10.0, 12.0, 10.0, 11.0, 1_000_000.0),
            OhlcvBar::new(11.0, 13.0, 11.0, 12.0, 1_200_000.0),
            OhlcvBar::new(12.0, 14.0, 12.0, 13.0, 900_000.0),
        ];
        assert_eq!(eom.next(&bars[0]), None);
        assert_eq!(eom.next(&bars[1]), None);
        let val = eom.next(&bars[2]).unwrap();
        assert!(val.is_finite());
    }

    #[test]
    fn test_streaming_eom_meta() {
        let eom = StreamingEom::new(14);
        assert_eq!(StreamingEom::name(), "EOM");
        assert_eq!(StreamingEom::category(), "volume");
        assert_eq!(eom.warm_up_period(), 14);
    }

    #[test]
    fn test_streaming_eom_reset() {
        let mut eom = StreamingEom::new(3);
        for i in 0..10 {
            eom.next(&OhlcvBar::new(
                i as f64,
                12.0 + i as f64,
                10.0 + i as f64,
                11.0 + i as f64,
                1_000_000.0,
            ));
        }
        assert!(eom.is_ready());
        eom.reset();
        assert!(!eom.is_ready());
        assert_eq!(eom.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 100;
        let bars: Vec<OhlcvBar> = (0..n)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.2).sin() * 10.0;
                let l = h - 3.0;
                let c = (h + l) / 2.0;
                let v = 1000.0 + (i as f64 * 0.5).cos() * 500.0;
                OhlcvBar::new(c - 0.5, h, l, c, v)
            })
            .collect();
        let high: Vec<f64> = bars.iter().map(|b| b.high()).collect();
        let low: Vec<f64> = bars.iter().map(|b| b.low()).collect();
        let volume: Vec<f64> = bars.iter().map(|b| b.volume()).collect();
        let period = 14;

        let batch = crate::indicators::volume_ext::eom(&high, &low, &volume, period).unwrap();
        let mut streaming = StreamingEom::new(period);

        for (i, bar) in bars.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(bar as &dyn Ohlcv), batch[i].is_nan()) {
                assert_relative_eq!(s, batch[i], epsilon = 1e-6);
            }
        }
    }
}
