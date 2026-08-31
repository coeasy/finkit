use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::streaming::Ohlcv;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingNvi {
    prev_close: f64,
    prev_volume: f64,
    current_nvi: f64,
    count: usize,
    last_value: Option<f64>,
}

impl Default for StreamingNvi {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingNvi {
    pub fn new() -> Self {
        Self {
            prev_close: f64::NAN,
            prev_volume: f64::NAN,
            current_nvi: 1000.0,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<&dyn Ohlcv, f64> for StreamingNvi {
    #[inline]
    fn next(&mut self, bar: &dyn Ohlcv) -> Option<f64> {
        self.count += 1;
        let close = bar.close();
        let volume = bar.volume();

        if self.count == 1 {
            self.prev_close = close;
            self.prev_volume = volume;
            let result = Some(self.current_nvi);
            self.last_value = result;
            return result;
        }

        if close.is_nan()
            || self.prev_close.is_nan()
            || volume.is_nan()
            || self.prev_volume.is_nan()
        {
            self.last_value = None;
            return None;
        }

        if volume < self.prev_volume && self.prev_close.abs() > 1e-15 {
            self.current_nvi *= 1.0 + (close - self.prev_close) / self.prev_close;
        }

        self.prev_close = close;
        self.prev_volume = volume;
        let result = Some(self.current_nvi);
        self.last_value = result;
        result
    }

    fn reset(&mut self) {
        self.prev_close = f64::NAN;
        self.prev_volume = f64::NAN;
        self.current_nvi = 1000.0;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.count >= 1
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingNvi {
    fn name() -> &'static str {
        "NVI"
    }
    fn category() -> &'static str {
        "volume"
    }
    fn description() -> &'static str {
        "Negative Volume Index"
    }
    fn warm_up_period(&self) -> usize {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::OhlcvBar;
    use approx::assert_relative_eq;

    #[test]
    fn test_streaming_nvi_basic() {
        let mut nvi = StreamingNvi::new();
        let v0 = nvi
            .next(&OhlcvBar::new(10.0, 12.0, 9.0, 10.0, 1000.0))
            .unwrap();
        assert_relative_eq!(v0, 1000.0, epsilon = 1e-10);
        let v1 = nvi
            .next(&OhlcvBar::new(11.0, 13.0, 10.0, 11.0, 900.0))
            .unwrap();
        assert_relative_eq!(v1, 1100.0, epsilon = 1e-10);
        let v2 = nvi
            .next(&OhlcvBar::new(10.5, 12.5, 9.5, 10.5, 1100.0))
            .unwrap();
        assert_relative_eq!(v2, 1100.0, epsilon = 1e-10);
    }

    #[test]
    fn test_streaming_nvi_meta() {
        assert_eq!(StreamingNvi::name(), "NVI");
        assert_eq!(StreamingNvi::category(), "volume");
        assert_eq!(StreamingNvi::new().warm_up_period(), 1);
    }

    #[test]
    fn test_streaming_nvi_reset() {
        let mut nvi = StreamingNvi::new();
        nvi.next(&OhlcvBar::new(10.0, 12.0, 9.0, 11.0, 100.0));
        assert!(nvi.is_ready());
        nvi.reset();
        assert!(!nvi.is_ready());
        assert_eq!(nvi.count(), 0);
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

        let batch = crate::indicators::volume_ext::nvi(&close, &volume).unwrap();
        let mut streaming = StreamingNvi::new();

        for (i, bar) in bars.iter().enumerate() {
            if let (Some(s), false) = (streaming.next(bar as &dyn Ohlcv), batch[i].is_nan()) {
                assert_relative_eq!(s, batch[i], epsilon = 1e-10);
            }
        }
    }
}
