use crate::impl_standard_methods;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::streaming::Ohlcv;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingForceIndex {
    period: usize,
    ema: StreamingEma,
    prev_close: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingForceIndex {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            ema: StreamingEma::new(period),
            prev_close: f64::NAN,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv, f64> for StreamingForceIndex {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.count += 1;
        let close = bar.close();
        let volume = bar.volume();

        if self.count == 1 {
            self.prev_close = close;
            self.last_value = None;
            return None;
        }

        let raw_force = if close.is_nan() || self.prev_close.is_nan() || volume.is_nan() {
            f64::NAN
        } else {
            (close - self.prev_close) * volume
        };
        self.prev_close = close;

        let result = self.ema.next(raw_force);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.ema.reset();
        self.prev_close = f64::NAN;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.ema.is_ready()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingForceIndex {
    fn name() -> &'static str {
        "Force Index"
    }
    fn category() -> &'static str {
        "volume"
    }
    fn description() -> &'static str {
        "Force Index"
    }
    fn warm_up_period(&self) -> usize {
        self.period + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::OhlcvBar;
    use approx::assert_relative_eq;

    #[test]
    fn test_streaming_force_index_basic() {
        let mut fi = StreamingForceIndex::new(3);
        let bars = [
            OhlcvBar::new(10.0, 12.0, 9.0, 10.0, 100.0),
            OhlcvBar::new(11.0, 13.0, 10.0, 11.0, 200.0),
            OhlcvBar::new(10.5, 12.5, 9.5, 10.0, 150.0),
            OhlcvBar::new(12.0, 14.0, 11.0, 12.0, 300.0),
        ];
        assert_eq!(fi.next(&bars[0]), None);
        assert_eq!(fi.next(&bars[1]), None);
        assert_eq!(fi.next(&bars[2]), None);
        let val = fi.next(&bars[3]).unwrap();
        assert!(val.is_finite());
    }

    #[test]
    fn test_streaming_force_index_meta() {
        let fi = StreamingForceIndex::new(13);
        assert_eq!(StreamingForceIndex::name(), "Force Index");
        assert_eq!(StreamingForceIndex::category(), "volume");
        assert_eq!(fi.warm_up_period(), 14);
    }

    #[test]
    fn test_streaming_force_index_reset() {
        let mut fi = StreamingForceIndex::new(3);
        for i in 0..10 {
            fi.next(&OhlcvBar::new(
                i as f64,
                i as f64 + 2.0,
                i as f64 - 1.0,
                i as f64 + 1.0,
                100.0 + i as f64 * 10.0,
            ));
        }
        assert!(fi.is_ready());
        fi.reset();
        assert!(!fi.is_ready());
        assert_eq!(fi.count(), 0);
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
        let close: Vec<f64> = bars.iter().map(|b| b.close()).collect();
        let volume: Vec<f64> = bars.iter().map(|b| b.volume()).collect();
        let period = 13;

        let batch = crate::indicators::volume_ext::force_index(&close, &volume, period).unwrap();
        let mut streaming = StreamingForceIndex::new(period);

        for (i, bar) in bars.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(bar as &dyn Ohlcv), batch[i].is_nan()) {
                assert_relative_eq!(s, batch[i], epsilon = 1e-10);
            }
        }
    }
}
