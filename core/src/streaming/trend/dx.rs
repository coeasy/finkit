use crate::impl_standard_methods;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::utils::true_range;

/// Streaming Directional Movement Index (DX).
///
/// DX = |+DI - -DI| / (+DI + -DI) * 100
/// Unlike ADX, DX is not smoothed.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingDx {
    period: usize,
    plus_dm_ema: StreamingEma,
    minus_dm_ema: StreamingEma,
    tr_ema: StreamingEma,
    prev_high: f64,
    prev_low: f64,
    prev_close: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingDx {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            plus_dm_ema: StreamingEma::new(period),
            minus_dm_ema: StreamingEma::new(period),
            tr_ema: StreamingEma::new(period),
            prev_high: f64::NAN,
            prev_low: f64::NAN,
            prev_close: f64::NAN,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64, f64)> for StreamingDx {
    #[inline]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<f64> {
        let (high, low, close) = input;
        self.count += 1;

        if self.count == 1 {
            self.prev_high = high;
            self.prev_low = low;
            self.prev_close = close;
            self.last_value = None;
            return None;
        }

        let tr = true_range(high, low, self.prev_close);
        let up_move = high - self.prev_high;
        let down_move = self.prev_low - low;

        let plus_dm = if up_move > 0.0 && up_move > down_move {
            up_move
        } else {
            0.0
        };
        let minus_dm = if down_move > 0.0 && down_move > up_move {
            down_move
        } else {
            0.0
        };

        self.prev_high = high;
        self.prev_low = low;
        self.prev_close = close;

        let smoothed_tr = self.tr_ema.next(tr);
        let smoothed_plus_dm = self.plus_dm_ema.next(plus_dm);
        let smoothed_minus_dm = self.minus_dm_ema.next(minus_dm);

        let (Some(smoothed_tr), Some(sp), Some(sm)) =
            (smoothed_tr, smoothed_plus_dm, smoothed_minus_dm)
        else {
            self.last_value = None;
            return None;
        };

        if smoothed_tr.abs() < 1e-15 {
            self.last_value = None;
            return None;
        }

        let plus_di = (sp / smoothed_tr) * 100.0;
        let minus_di = (sm / smoothed_tr) * 100.0;

        let di_sum = plus_di + minus_di;
        let dx = if di_sum.abs() > 1e-15 {
            (plus_di - minus_di).abs() / di_sum * 100.0
        } else {
            0.0
        };

        self.last_value = Some(dx);
        Some(dx)
    }

    fn reset(&mut self) {
        self.plus_dm_ema.reset();
        self.minus_dm_ema.reset();
        self.tr_ema.reset();
        self.prev_high = f64::NAN;
        self.prev_low = f64::NAN;
        self.prev_close = f64::NAN;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.tr_ema.is_ready()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingDx {
    fn name() -> &'static str {
        "DX"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Directional Movement Index"
    }
    fn warm_up_period(&self) -> usize {
        self.period + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen_data(n: usize) -> Vec<(f64, f64, f64)> {
        (0..n)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.3).sin() * 10.0;
                (h, h - 4.0, h - 2.0)
            })
            .collect()
    }

    #[test]
    fn test_streaming_dx_basic() {
        let mut dx = StreamingDx::new(14);
        let data = gen_data(50);
        let mut last = None;
        for &d in &data {
            last = dx.next(d);
        }
        let v = last.unwrap();
        assert!((0.0..=100.0).contains(&v), "DX should be 0-100, got {v}");
    }

    #[test]
    fn test_streaming_dx_reset() {
        let mut dx = StreamingDx::new(14);
        for &d in &gen_data(50) {
            dx.next(d);
        }
        assert!(dx.is_ready());
        dx.reset();
        assert!(!dx.is_ready());
        assert_eq!(dx.count(), 0);
    }

    #[test]
    fn test_streaming_dx_meta() {
        let dx = StreamingDx::new(14);
        assert_eq!(StreamingDx::name(), "DX");
        assert_eq!(StreamingDx::category(), "momentum");
        assert_eq!(dx.warm_up_period(), 15);
    }
}
