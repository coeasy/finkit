use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::utils::true_range;

/// Streaming Directional Movement Index (DX).
///
/// `DX = |+DI - -DI| / (+DI + -DI) * 100`, with all three smoothed inputs
/// produced by **Wilder's** smoothing (`s - s/p + x`, seeded by a plain sum).
///
/// # Convergence
///
/// This mirrors `indicators::momentum::dx_into` step for step. Wilder's
/// smoothing is *not* an exponential moving average: an `EMA(period)` seed does
/// not reproduce TA-Lib's DX, so an EMA-based incremental path is a different
/// indicator that happens to share a name. The `streaming == batch` gate in
/// `core/tests/runtime_convergence.rs` pins the two together.
///
/// Warm-up: `period - 1` bars to seed the smoothed sums, then the Wilder update
/// on bar `period`, which is the first row `dx_into` writes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingDx {
    period: usize,
    smooth_plus_dm: f64,
    smooth_minus_dm: f64,
    smooth_tr: f64,
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
            smooth_plus_dm: 0.0,
            smooth_minus_dm: 0.0,
            smooth_tr: 0.0,
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

        let up_move = high - self.prev_high;
        let down_move = self.prev_low - low;
        let plus_dm = if up_move > down_move && up_move > 0.0 {
            up_move
        } else {
            0.0
        };
        let minus_dm = if down_move > up_move && down_move > 0.0 {
            down_move
        } else {
            0.0
        };
        let tr = true_range(high, low, self.prev_close);

        self.prev_high = high;
        self.prev_low = low;
        self.prev_close = close;

        let p = self.period as f64;

        if self.count <= self.period {
            // `dx_into` seeds by summing indices `1..period`.
            self.smooth_plus_dm += plus_dm;
            self.smooth_minus_dm += minus_dm;
            self.smooth_tr += tr;
            self.last_value = None;
            return None;
        }

        self.smooth_plus_dm = self.smooth_plus_dm - self.smooth_plus_dm / p + plus_dm;
        self.smooth_minus_dm = self.smooth_minus_dm - self.smooth_minus_dm / p + minus_dm;
        self.smooth_tr = self.smooth_tr - self.smooth_tr / p + tr;

        // Degenerate window handling matches `dx_into` exactly: a flat TR yields
        // 0.0 (not NaN, not `None`) once the smoothing has started.
        let value = if !crate::utils::is_zero(self.smooth_tr) {
            let plus_di = self.smooth_plus_dm / self.smooth_tr * 100.0;
            let minus_di = self.smooth_minus_dm / self.smooth_tr * 100.0;
            let sum = plus_di + minus_di;
            if !crate::utils::is_zero(sum) {
                (plus_di - minus_di).abs() / sum * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        };

        self.last_value = Some(value);
        Some(value)
    }

    fn reset(&mut self) {
        self.smooth_plus_dm = 0.0;
        self.smooth_minus_dm = 0.0;
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
    /// `period - 1` seeding bars plus the bar that gets the first Wilder update.
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
        assert_eq!(
            dx.warm_up_period(),
            crate::streaming::registry::by_id("DX")
                .expect("DX is registered")
                .convergence
        );
    }

    #[test]
    fn first_value_lands_on_the_first_batch_row() {
        let mut dx = StreamingDx::new(14);
        let mut first = None;
        for (index, &bar) in gen_data(40).iter().enumerate() {
            if first.is_none() && dx.next(bar).is_some() {
                first = Some(index);
            }
        }
        // `dx_into` writes its first value at index `period`.
        assert_eq!(first, Some(14));
    }
}
