use crate::streaming::overlap::ema::StreamingEma;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::impl_standard_methods;
use crate::utils::true_range;

/// Streaming Minus Directional Indicator (-DI).
///
/// -DI = EMA(-DM) / EMA(TR) * 100
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingMinusDi {
    period: usize,
    minus_dm_ema: StreamingEma,
    tr_ema: StreamingEma,
    prev_high: f64,
    prev_low: f64,
    prev_close: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingMinusDi {
    pub fn new(period: usize) -> Self {
        Self {
            period,
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

impl StreamingIndicator<(f64, f64, f64)> for StreamingMinusDi {
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

        let minus_dm = if down_move > 0.0 && down_move > up_move { down_move } else { 0.0 };

        self.prev_high = high;
        self.prev_low = low;
        self.prev_close = close;

        let smoothed_tr = self.tr_ema.next(tr);
        let smoothed_minus_dm = self.minus_dm_ema.next(minus_dm);

        let (Some(str_val), Some(sm)) = (smoothed_tr, smoothed_minus_dm) else {
            self.last_value = None;
            return None;
        };

        if str_val.abs() < 1e-15 {
            self.last_value = Some(0.0);
            return Some(0.0);
        }

        let result = (sm / str_val) * 100.0;
        self.last_value = Some(result);
        Some(result)
    }

    fn reset(&mut self) {
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

impl IndicatorMeta for StreamingMinusDi {
    fn name() -> &'static str { "MINUS_DI" }
    fn category() -> &'static str { "momentum" }
    fn description() -> &'static str { "Minus Directional Indicator" }
    fn warm_up_period(&self) -> usize { self.period + 1 }
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
    fn test_streaming_minus_di_basic() {
        let mut ind = StreamingMinusDi::new(14);
        let data = gen_data(50);
        let mut last = None;
        for &d in &data {
            last = ind.next(d);
        }
        let v = last.unwrap();
        assert!(v >= 0.0, "-DI should be >= 0, got {v}");
    }

    #[test]
    fn test_streaming_minus_di_downtrend() {
        let mut ind = StreamingMinusDi::new(14);
        let data: Vec<(f64, f64, f64)> = (0..60)
            .map(|i| {
                let base = 200.0 - i as f64 * 2.0;
                (base + 1.0, base - 3.0, base - 1.0)
            })
            .collect();
        let mut last = None;
        for &d in &data {
            last = ind.next(d);
        }
        let v = last.unwrap();
        assert!(v > 20.0, "-DI in downtrend should be high, got {v}");
    }

    #[test]
    fn test_streaming_minus_di_reset() {
        let mut ind = StreamingMinusDi::new(14);
        for &d in &gen_data(50) {
            ind.next(d);
        }
        assert!(ind.is_ready());
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.count(), 0);
    }

    #[test]
    fn test_streaming_minus_di_meta() {
        let ind = StreamingMinusDi::new(14);
        assert_eq!(StreamingMinusDi::name(), "MINUS_DI");
        assert_eq!(ind.warm_up_period(), 15);
    }
}
