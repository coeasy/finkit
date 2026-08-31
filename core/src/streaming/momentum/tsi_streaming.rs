use crate::impl_standard_methods;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingTsi {
    long_period: usize,
    short_period: usize,
    ema_long_mom: StreamingEma,
    ema_short_mom: StreamingEma,
    ema_long_abs: StreamingEma,
    ema_short_abs: StreamingEma,
    prev_close: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingTsi {
    pub fn new(long_period: usize, short_period: usize) -> Self {
        Self {
            long_period,
            short_period,
            ema_long_mom: StreamingEma::new(long_period),
            ema_short_mom: StreamingEma::new(short_period),
            ema_long_abs: StreamingEma::new(long_period),
            ema_short_abs: StreamingEma::new(short_period),
            prev_close: f64::NAN,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator for StreamingTsi {
    #[inline]
    fn next(&mut self, close: f64) -> Option<f64> {
        self.count += 1;

        if self.count == 1 {
            self.prev_close = close;
            self.last_value = None;
            return None;
        }

        let mom = close - self.prev_close;
        self.prev_close = close;
        let abs_mom = mom.abs();

        let ema_long_mom = self.ema_long_mom.next(mom);
        let ema_long_abs = self.ema_long_abs.next(abs_mom);

        let smooth_mom = ema_long_mom.and_then(|v| self.ema_short_mom.next(v));
        let smooth_abs = ema_long_abs.and_then(|v| self.ema_short_abs.next(v));

        let (Some(smooth_mom), Some(smooth_abs)) = (smooth_mom, smooth_abs) else {
            self.last_value = None;
            return None;
        };

        let result = if smooth_abs.abs() > 1e-15 {
            Some(100.0 * smooth_mom / smooth_abs)
        } else {
            None
        };
        self.last_value = result;
        result
    }

    #[inline]
    fn reset(&mut self) {
        self.ema_long_mom.reset();
        self.ema_short_mom.reset();
        self.ema_long_abs.reset();
        self.ema_short_abs.reset();
        self.prev_close = f64::NAN;
        self.count = 0;
        self.last_value = None;
    }

    #[inline]
    fn is_ready(&self) -> bool {
        self.count > self.long_period + self.short_period - 2
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingTsi {
    fn name() -> &'static str {
        "TSI"
    }

    fn category() -> &'static str {
        "momentum"
    }

    fn description() -> &'static str {
        "True Strength Index"
    }

    fn warm_up_period(&self) -> usize {
        self.long_period + self.short_period - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_tsi_basic() {
        let mut tsi = StreamingTsi::new(25, 13);
        let data: Vec<f64> = (0..40).map(|i| 100.0 + i as f64).collect();
        let mut last = None;
        for &val in &data {
            last = tsi.next(val);
        }
        let last = last.unwrap();
        assert!(last > 0.0);
    }

    #[test]
    fn test_streaming_tsi_meta() {
        let tsi = StreamingTsi::new(25, 13);
        assert_eq!(StreamingTsi::name(), "TSI");
        assert_eq!(StreamingTsi::category(), "momentum");
        assert_eq!(tsi.warm_up_period(), 37);
    }

    #[test]
    fn test_streaming_tsi_reset() {
        let mut tsi = StreamingTsi::new(5, 3);
        for i in 0..20 {
            tsi.next(i as f64 + 100.0);
        }
        assert!(tsi.is_ready());
        tsi.reset();
        assert!(!tsi.is_ready());
        assert_eq!(tsi.count(), 0);
    }

    #[test]
    fn test_streaming_vs_batch_convergence() {
        let n = 200;
        let data: Vec<f64> = (0..n)
            .map(|i| 100.0 + (i as f64 * 0.2).sin() * 15.0 + i as f64 * 0.05)
            .collect();
        let long_period = 25;
        let short_period = 13;

        let batch = crate::indicators::momentum_ext::tsi(&data, long_period, short_period).unwrap();
        let mut streaming = StreamingTsi::new(long_period, short_period);

        let converge_after = long_period + short_period + 20;
        for (i, &val) in data.iter().enumerate() {
            if let Some(s) = streaming.next(val) {
                if i >= converge_after && !batch[i].is_nan() {
                    assert!(
                        (s - batch[i]).abs() < 1e-6,
                        "Mismatch at {i}: streaming={s}, batch={}",
                        batch[i]
                    );
                }
            }
        }
    }
}
