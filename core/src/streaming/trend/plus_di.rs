use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::utils::true_range;

/// Streaming Plus Directional Indicator (+DI).
///
/// `+DI = smoothed(+DM) / smoothed(TR) * 100`, where *smoothed* is **Wilder's**
/// smoothing (`s - s/p + x`, seeded by a plain sum).
///
/// # Convergence
///
/// This mirrors `indicators::momentum::compute_single_di::<true>`, which backs
/// the batch `plus_di`. Wilder's smoothing is not an exponential moving
/// average, and the difference is visible from the first emitted bar onward, so
/// the `streaming == batch` gate in `core/tests/runtime_convergence.rs` pins
/// this kernel to the batch one.
///
/// Warm-up: `period - 1` seeding bars, then the first Wilder update on bar
/// `period` — the first row the batch kernel writes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingPlusDi {
    period: usize,
    smooth_dm: f64,
    smooth_tr: f64,
    prev_high: f64,
    prev_low: f64,
    prev_close: f64,
    count: usize,
    last_value: Option<f64>,
}

impl StreamingPlusDi {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            smooth_dm: 0.0,
            smooth_tr: 0.0,
            prev_high: f64::NAN,
            prev_low: f64::NAN,
            prev_close: f64::NAN,
            count: 0,
            last_value: None,
        }
    }
}

impl StreamingIndicator<(f64, f64, f64)> for StreamingPlusDi {
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

        let up_move = high - self.prev_high;
        let down_move = self.prev_low - low;
        let plus_dm = if up_move > down_move && up_move > 0.0 {
            up_move
        } else {
            0.0
        };
        let tr = true_range(high, low, self.prev_close);

        self.prev_high = high;
        self.prev_low = low;
        self.prev_close = close;

        let p = self.period as f64;

        if self.count <= self.period {
            self.smooth_dm += plus_dm;
            self.smooth_tr += tr;
            self.last_value = None;
            return None;
        }

        self.smooth_dm = self.smooth_dm - self.smooth_dm / p + plus_dm;
        self.smooth_tr = self.smooth_tr - self.smooth_tr / p + tr;

        let value = if !crate::utils::is_zero(self.smooth_tr) {
            self.smooth_dm / self.smooth_tr * 100.0
        } else {
            0.0
        };
        self.last_value = Some(value);
        Some(value)
    }

    fn reset(&mut self) {
        self.smooth_dm = 0.0;
        self.smooth_tr = 0.0;
        self.prev_high = f64::NAN;
        self.prev_low = f64::NAN;
        self.prev_close = f64::NAN;
        self.count = 0;
        self.last_value = None;
    }

    fn is_ready(&self) -> bool {
        self.last_value.is_some()
    }

    impl_standard_methods!();
}

impl IndicatorMeta for StreamingPlusDi {
    fn name() -> &'static str {
        "PLUS_DI"
    }
    fn category() -> &'static str {
        "momentum"
    }
    fn description() -> &'static str {
        "Plus Directional Indicator"
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
    fn test_streaming_plus_di_basic() {
        let mut ind = StreamingPlusDi::new(14);
        let data = gen_data(50);
        let mut last = None;
        for &d in &data {
            last = ind.next(d);
        }
        let v = last.unwrap();
        assert!(v >= 0.0, "+DI should be >= 0, got {v}");
    }

    #[test]
    fn test_streaming_plus_di_uptrend() {
        let mut ind = StreamingPlusDi::new(14);
        let data: Vec<(f64, f64, f64)> = (0..60)
            .map(|i| {
                let base = 100.0 + i as f64 * 2.0;
                (base + 3.0, base - 1.0, base + 1.0)
            })
            .collect();
        let mut last = None;
        for &d in &data {
            last = ind.next(d);
        }
        let v = last.unwrap();
        assert!(v > 20.0, "+DI in uptrend should be high, got {v}");
    }

    #[test]
    fn test_streaming_plus_di_reset() {
        let mut ind = StreamingPlusDi::new(14);
        for &d in &gen_data(50) {
            ind.next(d);
        }
        assert!(ind.is_ready());
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.count(), 0);
    }

    #[test]
    fn test_streaming_plus_di_meta() {
        let ind = StreamingPlusDi::new(14);
        assert_eq!(StreamingPlusDi::name(), "PLUS_DI");
        assert_eq!(
            ind.warm_up_period(),
            crate::streaming::registry::by_id("PLUS_DI")
                .expect("PLUS_DI is registered")
                .convergence
        );
    }

    #[test]
    fn first_value_lands_on_the_first_batch_row() {
        let mut ind = StreamingPlusDi::new(14);
        let mut first = None;
        for (index, &bar) in gen_data(40).iter().enumerate() {
            if first.is_none() && ind.next(bar).is_some() {
                first = Some(index);
            }
        }
        assert_eq!(first, Some(14));
    }
}
