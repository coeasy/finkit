use crate::impl_standard_methods;
use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::utils::true_range;

/// Streaming Average Directional Index (ADX).
///
/// Computes +DI, -DI, DX, then smooths DX via EMA to produce ADX.
/// Input: (high, low, close) tuple per bar.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingAdx {
    period: usize,
    plus_dm_ema: StreamingEma,
    minus_dm_ema: StreamingEma,
    tr_ema: StreamingEma,
    dx_ema: StreamingEma,
    prev_high: f64,
    prev_low: f64,
    prev_close: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingAdx {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            plus_dm_ema: StreamingEma::new(period),
            minus_dm_ema: StreamingEma::new(period),
            tr_ema: StreamingEma::new(period),
            dx_ema: StreamingEma::new(period),
            prev_high: f64::NAN,
            prev_low: f64::NAN,
            prev_close: f64::NAN,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64, f64)> for StreamingAdx {
    #[inline]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip(self, input))
    )]
    fn next(&mut self, input: (f64, f64, f64)) -> Option<f64> {
        crate::streaming_measure!("adx", self.count, {
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

            let (Some(smoothed_tr), Some(_), Some(_)) =
                (smoothed_tr, smoothed_plus_dm, smoothed_minus_dm)
            else {
                self.last_value = None;
                return None;
            };

            if smoothed_tr.abs() < 1e-15 {
                self.last_value = None;
                return None;
            }

            let plus_di = (smoothed_plus_dm.unwrap() / smoothed_tr) * 100.0;
            let minus_di = (smoothed_minus_dm.unwrap() / smoothed_tr) * 100.0;

            let di_sum = plus_di + minus_di;
            let dx = if di_sum.abs() > 1e-15 {
                (plus_di - minus_di).abs() / di_sum * 100.0
            } else {
                0.0
            };

            let result = self.dx_ema.next(dx);
            self.last_value = result;
            result
        })
    }

    fn reset(&mut self) {
        self.plus_dm_ema.reset();
        self.minus_dm_ema.reset();
        self.tr_ema.reset();
        self.dx_ema.reset();
        self.prev_high = f64::NAN;
        self.prev_low = f64::NAN;
        self.prev_close = f64::NAN;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.dx_ema.is_ready()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingAdx {
    fn name() -> &'static str {
        "ADX"
    }

    fn category() -> &'static str {
        "momentum"
    }

    fn description() -> &'static str {
        "Average Directional Index"
    }

    fn warm_up_period(&self) -> usize {
        self.period * 2 + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_adx_basic() {
        let mut adx = StreamingAdx::new(14);
        let data: Vec<(f64, f64, f64)> = (0..50)
            .map(|i| {
                let h = 50.0 + (i as f64 * 0.3).sin() * 10.0;
                let l = h - 4.0;
                let c = h - 2.0;
                (h, l, c)
            })
            .collect();

        let mut last = None;
        for &d in &data {
            last = adx.next(d);
        }
        let last = last.unwrap();
        assert!((0.0..=100.0).contains(&last));
    }

    #[test]
    fn test_streaming_adx_trending_market() {
        let mut adx = StreamingAdx::new(14);
        let data: Vec<(f64, f64, f64)> = (0..60)
            .map(|i| {
                let base = 100.0 + i as f64 * 2.0;
                (base + 3.0, base - 1.0, base + 1.0)
            })
            .collect();

        let mut last = None;
        for &d in &data {
            last = adx.next(d);
        }
        let last = last.unwrap();
        assert!(
            last > 20.0,
            "ADX in trending market should be > 20, got {last}"
        );
    }

    #[test]
    fn test_streaming_adx_reset() {
        let mut adx = StreamingAdx::new(14);
        for i in 0..50 {
            adx.next((50.0 + i as f64, 45.0 + i as f64, 47.0 + i as f64));
        }
        assert!(adx.is_ready());
        adx.reset();
        assert!(!adx.is_ready());
        assert_eq!(adx.count(), 0);
    }

    #[test]
    fn test_streaming_adx_meta() {
        let adx = StreamingAdx::new(14);
        assert_eq!(StreamingAdx::name(), "ADX");
        assert_eq!(StreamingAdx::category(), "momentum");
        assert_eq!(adx.warm_up_period(), 29);
    }
}
