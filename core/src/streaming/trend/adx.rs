use crate::impl_standard_methods;
use crate::streaming::traits::{IndicatorMeta, StreamingIndicator};
use crate::utils::true_range;

/// Streaming Average Directional Index (ADX).
///
/// Computes +DM/-DM/TR, smooths all three with Wilder's smoothing, forms DX,
/// then smooths DX the same way.
///
/// # Convergence
///
/// The smoothing here is **Wilder's** (`s - s/p + x`, seeded by a plain sum),
/// not an exponential moving average. The two look similar and are not the
/// same: an EMA seed of `EMA(period)` does not reproduce TA-Lib's ADX, and
/// using one makes the incremental path a *different indicator* from
/// `indicators::momentum::adx` rather than an incremental form of it. The
/// `streaming == batch` gate in `core/tests/runtime_convergence.rs` pins the
/// two together; this kernel is written to mirror `adx_into` step for step.
///
/// Warm-up is exactly `2 * period` bars, matching the registry's
/// `convergence` figure for `ADX`: `period - 1` bars to seed the smoothed
/// sums, then `period` DX values to seed the ADX average.
///
/// Input: `(high, low, close)` per bar.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StreamingAdx {
    period: usize,
    /// Wilder-smoothed +DM, −DM and true range.
    smooth_plus_dm: f64,
    smooth_minus_dm: f64,
    smooth_tr: f64,
    /// Running sum of DX over the seeding window, then the ADX itself.
    dx_sum: f64,
    adx_value: f64,
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
            smooth_plus_dm: 0.0,
            smooth_minus_dm: 0.0,
            smooth_tr: 0.0,
            dx_sum: 0.0,
            adx_value: 0.0,
            prev_high: f64::NAN,
            prev_low: f64::NAN,
            prev_close: f64::NAN,
            count: 0,
            last_value: None,
        }
    }
}

/// Directional index from the three smoothed inputs.
///
/// Byte-for-byte the arithmetic in `indicators::momentum::adx_into::dx`,
/// including the degenerate-input guards, so the two paths cannot drift on the
/// edge cases either.
#[inline]
fn directional_index(smooth_plus_dm: f64, smooth_minus_dm: f64, smooth_tr: f64) -> f64 {
    if smooth_tr.abs() <= 1e-15 {
        return 0.0;
    }
    let plus_di = smooth_plus_dm / smooth_tr * 100.0;
    let minus_di = smooth_minus_dm / smooth_tr * 100.0;
    let sum = plus_di + minus_di;
    if sum.abs() > 1e-15 {
        (plus_di - minus_di).abs() / sum * 100.0
    } else {
        0.0
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

            // Bar 0 has no predecessor, so it only establishes the deltas the
            // first real bar needs. `adx_into` starts its accumulation at index
            // 1 for the same reason.
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

            let period = self.period;
            let p = period as f64;

            if self.count <= period {
                // Seeding window: `adx_into` sums indices `1..period`, which is
                // the same `period - 1` bars this branch covers.
                self.smooth_plus_dm += plus_dm;
                self.smooth_minus_dm += minus_dm;
                self.smooth_tr += tr;
                self.last_value = None;
                return None;
            }

            self.smooth_plus_dm = self.smooth_plus_dm - self.smooth_plus_dm / p + plus_dm;
            self.smooth_minus_dm = self.smooth_minus_dm - self.smooth_minus_dm / p + minus_dm;
            self.smooth_tr = self.smooth_tr - self.smooth_tr / p + tr;
            let current_dx =
                directional_index(self.smooth_plus_dm, self.smooth_minus_dm, self.smooth_tr);

            if self.count <= period * 2 {
                // DX seeding window: `adx_into` averages indices
                // `period..2*period`, so the first ADX lands on the bar whose
                // index is `2 * period - 1` — the last bar of this branch.
                self.dx_sum += current_dx;
                if self.count == period * 2 {
                    self.adx_value = self.dx_sum / p;
                    self.last_value = Some(self.adx_value);
                    return self.last_value;
                }
                self.last_value = None;
                return None;
            }

            self.adx_value = (self.adx_value * (p - 1.0) + current_dx) / p;
            self.last_value = Some(self.adx_value);
            self.last_value
        })
    }

    fn reset(&mut self) {
        self.smooth_plus_dm = 0.0;
        self.smooth_minus_dm = 0.0;
        self.smooth_tr = 0.0;
        self.dx_sum = 0.0;
        self.adx_value = 0.0;
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

    /// `2 * period`: `period - 1` bars of smoothing seed plus `period` DX
    /// values. Matches `registry::by_id("ADX").convergence` at the default
    /// period of 14, and the `streaming == batch` gate covers the general case.
    fn warm_up_period(&self) -> usize {
        self.period * 2
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
        // 2 * period, matching registry::by_id("ADX").convergence.
        assert_eq!(adx.warm_up_period(), 28);
        assert_eq!(
            adx.warm_up_period(),
            crate::streaming::registry::by_id("ADX")
                .expect("ADX is registered")
                .convergence
        );
    }

    #[test]
    fn first_value_lands_on_the_registry_convergence_bar() {
        let mut adx = StreamingAdx::new(14);
        let mut first = None;
        for i in 0..60 {
            let base = 100.0 + i as f64;
            let value = adx.next((base + 2.0, base - 2.0, base));
            if first.is_none() && value.is_some() {
                first = Some(i);
            }
        }
        assert_eq!(
            first,
            Some(27),
            "ADX must emit from index 2*period - 1, one bar before the \
             convergence count"
        );
    }
}
