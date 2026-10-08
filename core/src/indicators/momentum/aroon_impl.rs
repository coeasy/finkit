//! Aroon, Aroon oscillator and their extreme tracker.

use super::prelude::*;

/// Aroon Indicator Result
#[derive(Debug, Clone)]
pub struct AroonResult {
    /// Aroon Up
    pub aroon_up: Array1<f64>,
    /// Aroon Down
    pub aroon_down: Array1<f64>,
}

/// Aroon Indicator (AROON)
///
/// Identifies trend changes and the strength of the trend.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `period` - Lookback period
///
/// # Returns
/// AroonResult containing Aroon Up and Aroon Down
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let result = indicators::aroon(&high, &low, 5).unwrap();
/// assert_eq!(result.aroon_up.len(), 10);
/// ```
/// Cached-index arg-extreme of the trailing window, shared by `AROON`,
/// `AROON_UP`/`AROON_DOWN` and `AROONOSC`.
///
/// `idx` is the absolute index of the window's extreme. The window always ends
/// at the current bar, so `i - idx` is the "age" `AROON` scales and, at the
/// same time, the lane offset — the two are the same number, and that is what
/// lets one recurrence serve all three outputs.
///
/// This is the cached-index strategy: one comparison per bar while `idx` stays
/// inside `[i - period, i]`, and a rescan of at most `period + 1` bars when it
/// leaves. It is the form that won the head-to-head measurements — a monotonic
/// deque pays a mispredicted pop-back branch on every noisy bar, and a fused
/// Van Herk–Gil–Werman block scan costs more in dynamically indexed tables than
/// the rescans it removes (V4 plan §36).
///
/// `AROON`, its caller-owned form and `AROONOSC` used to carry three separate
/// copies of this recurrence with three different `NaN` behaviours; there is
/// now one, and it is the above — the state is a trait-free struct so the
/// compiler keeps it in registers rather than re-deriving it per call site.
#[derive(Clone, Copy)]
pub(crate) struct ExtremeTracker<const WANT_MAX: bool> {
    extreme: f64,
    idx: usize,
    found: bool,
}

impl<const WANT_MAX: bool> ExtremeTracker<WANT_MAX> {
    #[inline(always)]
    fn new() -> Self {
        Self {
            extreme: if WANT_MAX {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            },
            idx: 0,
            found: false,
        }
    }

    /// Fold bar `i` in, evicting an extreme that has left `[i - period, i]`.
    ///
    /// The current value is passed in rather than re-read as `values[i]`, so
    /// the caller's `zip` walk is the only place the bar is loaded and there is
    /// no bounds check on the hot path.
    ///
    /// The comparison is against an infinite seed, so `NaN` — which fails both
    /// `>=` and `<=` — is skipped for free with no explicit test; that is the
    /// property all three previous copies relied on. `>=` / `<=` rather than
    /// `>` / `<` keep the *newest* tied bar, which matters here because the
    /// index, not only the value, is what gets reported.
    #[inline(always)]
    fn advance(&mut self, values: &[f64], i: usize, value: f64, period: usize) {
        let wins = if WANT_MAX {
            value >= self.extreme
        } else {
            value <= self.extreme
        };
        if wins {
            self.extreme = value;
            self.idx = i;
            self.found = true;
        } else if self.found && self.idx + period < i {
            let (extreme, idx, found) =
                crate::math::statistics::rescan_extreme_window::<WANT_MAX>(values, i - period, i);
            self.extreme = extreme;
            self.idx = idx;
            self.found = found;
        }
    }

    /// `100 * (period - age) / period`, or `NaN` when the window holds no
    /// finite bar.
    #[inline(always)]
    fn aroon(&self, i: usize, period: usize, scale: f64) -> f64 {
        if self.found {
            (period - (i - self.idx)) as f64 * scale
        } else {
            f64::NAN
        }
    }
}

/// AROON
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let result = indicators::aroon(&high, &low, 5).unwrap();
/// assert_eq!(result.aroon_up.len(), 10);
/// ```
pub fn aroon(high: &[f64], low: &[f64], period: usize) -> Result<AroonResult> {
    let len = high.len();
    // Every slot is written by the kernel below, warm-up `NaN` prefix included,
    // so these two buffers need no seeding.
    let mut aroon_up = crate::utils::uninit_output(len);
    let mut aroon_down = crate::utils::uninit_output(len);
    aroon_into(high, low, period, &mut aroon_up, &mut aroon_down)?;
    Ok(AroonResult {
        aroon_up: Array1::from(aroon_up),
        aroon_down: Array1::from(aroon_down),
    })
}

/// Caller-owned AROON kernel.
///
/// Writes `AROON_UP` into `aroon_up` and `AROON_DOWN` into `aroon_down`; both
/// must be as long as `high`, and every slot is written (`NaN` where the
/// contract has no value yet).
///
/// Both legs advance in **one** loop. A per-leg helper run twice — one pass
/// over `high`, then one over `low` — measured 21% slower than this shape on
/// the 10,000-bar series even though the two forms execute the same number of
/// comparisons: the second pass re-walks the index arithmetic and the branch
/// skeleton for no second result.
///
/// What is *shared* between [`aroon`], this kernel and [`aroonosc`] is the
/// recurrence — the private `ExtremeTracker` type — not the loop around it.
/// Sharing the type is the point: three hand-copied recurrences with three
/// different `NaN` behaviours cannot be kept in step, whereas these three
/// differ only in which outputs they pick out of the same state.
#[inline]
pub fn aroon_into(
    high: &[f64],
    low: &[f64],
    period: usize,
    aroon_up: &mut [f64],
    aroon_down: &mut [f64],
) -> Result<()> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if aroon_up.len() != high.len() || aroon_down.len() != high.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as high".to_string(),
        });
    }
    validate_input(high.len(), period + 1)?;

    let scale = 100.0 / period as f64;
    let mut highs = ExtremeTracker::<true>::new();
    let mut lows = ExtremeTracker::<false>::new();
    // The `zip` walk over all four slices keeps `high[i]`, `low[i]` and the two
    // output slots free of per-bar bounds checks. That is a property of this
    // loop, not an explanation of the gap to C: §43.5 tested the bounds-check
    // hypothesis directly on `LINREG_SLOPE` (index arithmetic against pointer
    // cursors, same seeding, same allocation) and measured it at exactly
    // 1.000x — LLVM had already elided the checks. Whatever still separates the
    // `AROON`/`ADX`/`MAX`/`MIN` band from C, it is not the checks.
    for (i, (((&high_bar, &low_bar), up_slot), down_slot)) in high
        .iter()
        .zip(low.iter())
        .zip(aroon_up.iter_mut())
        .zip(aroon_down.iter_mut())
        .enumerate()
    {
        highs.advance(high, i, high_bar, period);
        lows.advance(low, i, low_bar, period);
        let (up, down) = if i >= period {
            (highs.aroon(i, period, scale), lows.aroon(i, period, scale))
        } else {
            (f64::NAN, f64::NAN)
        };
        *up_slot = up;
        *down_slot = down;
    }
    Ok(())
}

/// Aroon Oscillator (AROONOSC)
///
/// AROONOSC = Aroon Up - Aroon Down
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high = vec![45.0, 45.5, 46.0, 45.5, 46.5, 46.0, 45.5, 45.0, 45.5, 46.0];
/// let low = vec![43.0, 43.5, 44.0, 43.0, 44.0, 43.5, 43.0, 42.5, 43.0, 43.5];
/// let result = indicators::aroonosc(&high, &low, 5).unwrap();
/// assert_eq!(result.len(), 10);
/// ```
pub fn aroonosc(high: &[f64], low: &[f64], period: usize) -> Result<Array1<f64>> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), period + 1)?;
    let len = high.len();
    let scale = 100.0 / period as f64;
    let mut output = crate::utils::uninit_output(len);

    // Both legs are tracked in this one pass rather than as two `AROON` columns
    // that are subtracted afterwards: the `_into` form would need two
    // intermediate 80 KB buffers per call. `AROONOSC = AROON_UP - AROON_DOWN`,
    // and with `age = i - idx` the two `period` offsets cancel, so the result
    // is `(idx_high - idx_low) * 100 / period` — a single scaled subtraction
    // per bar over the same recurrence. The cast happens before the
    // subtraction because the argmax may sit below the argmin; a window with no
    // finite bar on either side reports `NaN`, matching `AROON`.
    //
    // Every slot is written, warm-up prefix included, which is what lets
    // `uninit_output` stand in for a full `NaN` fill — see its contract.
    let mut highs = ExtremeTracker::<true>::new();
    let mut lows = ExtremeTracker::<false>::new();
    for (i, ((&high_bar, &low_bar), slot)) in high
        .iter()
        .zip(low.iter())
        .zip(output.iter_mut())
        .enumerate()
    {
        highs.advance(high, i, high_bar, period);
        lows.advance(low, i, low_bar, period);
        *slot = if i >= period && highs.found && lows.found {
            (highs.idx as f64 - lows.idx as f64) * scale
        } else {
            f64::NAN
        };
    }
    Ok(Array1::from(output))
}
