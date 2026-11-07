//! Aroon, Aroon oscillator and their extreme tracker.

use super::prelude::*;

const AROON_DEQUE_CAPACITY: usize = 32;
const AROON_DEQUE_MIN_PERIOD: usize = 7;

#[derive(Clone, Copy)]
struct MonotonicIndexDeque {
    indices: [usize; AROON_DEQUE_CAPACITY],
    head: usize,
    len: usize,
}

impl MonotonicIndexDeque {
    #[inline(always)]
    fn new() -> Self {
        Self {
            indices: [0; AROON_DEQUE_CAPACITY],
            head: 0,
            len: 0,
        }
    }

    #[inline(always)]
    fn front(&self) -> Option<usize> {
        (self.len != 0).then(|| self.indices[self.head])
    }

    #[inline(always)]
    fn back(&self) -> Option<usize> {
        (self.len != 0).then(|| self.indices[(self.head + self.len - 1) % AROON_DEQUE_CAPACITY])
    }

    #[inline(always)]
    fn pop_front(&mut self) {
        self.head = (self.head + 1) % AROON_DEQUE_CAPACITY;
        self.len -= 1;
    }

    #[inline(always)]
    fn pop_back(&mut self) {
        self.len -= 1;
    }

    #[inline(always)]
    fn push_back(&mut self, index: usize) {
        debug_assert!(self.len < AROON_DEQUE_CAPACITY);
        self.indices[(self.head + self.len) % AROON_DEQUE_CAPACITY] = index;
        self.len += 1;
    }
}

/// Maintain the newest maximum/minimum index in the active AROON window.
/// NaNs are ignored, infinities remain valid, and popping equal values keeps
/// the newest tie just like `ExtremeTracker`.
#[inline(always)]
fn advance_monotonic_deque<const WANT_MAX: bool>(
    values: &[f64],
    index: usize,
    first_valid: usize,
    deque: &mut MonotonicIndexDeque,
) {
    while deque.front().is_some_and(|old| old < first_valid) {
        deque.pop_front();
    }
    let value = values[index];
    if value.is_nan() {
        return;
    }
    while deque.back().is_some_and(|old| {
        if WANT_MAX {
            values[old] <= value
        } else {
            values[old] >= value
        }
    }) {
        deque.pop_back();
    }
    deque.push_back(index);
}

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
/// Cached-age arg-extreme of the trailing window, shared by `AROON`,
/// `AROON_UP`/`AROON_DOWN` and `AROONOSC`.
///
/// The tracker stores the extreme's age rather than its absolute index. The
/// window always ends at the current bar, so that age is exactly the offset
/// `AROON` scales; for `AROONOSC`, the current index cancels and only the two
/// ages' difference is needed.
///
/// This cached-extreme strategy does one comparison per bar while the cached
/// extreme stays inside `[i - period, i]`, then rescans at most `period + 1`
/// bars when it leaves. The public two-leg and oscillator APIs use this path.
/// Single-leg formula calls use a fixed-capacity monotone deque for common
/// periods; its extra queue maintenance lost on the public OHLCV benchmark.
#[derive(Clone, Copy)]
pub(crate) struct ExtremeTracker<const WANT_MAX: bool> {
    extreme: f64,
    age: usize,
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
            age: 0,
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
    /// `>` / `<` keep the *newest* tied bar, which resets its age and delays
    /// the next rescan.
    #[inline(always)]
    fn advance(&mut self, values: &[f64], i: usize, value: f64, period: usize) {
        if self.found {
            self.age += 1;
        }
        let wins = if WANT_MAX {
            value >= self.extreme
        } else {
            value <= self.extreme
        };
        if wins {
            self.extreme = value;
            self.age = 0;
            self.found = true;
        } else if self.found && self.age > period {
            let (extreme, idx, found) =
                crate::math::statistics::rescan_extreme_window::<WANT_MAX>(values, i - period, i);
            self.extreme = extreme;
            self.age = if found { i - idx } else { 0 };
            self.found = found;
        }
    }

    /// `100 * (period - age) / period`, or `NaN` when the window holds no
    /// finite bar.
    #[inline(always)]
    fn aroon(&self, period: usize, scale: f64) -> f64 {
        if self.found {
            debug_assert!(self.age <= period);
            (period - self.age) as f64 * scale
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
/// The cached-age path serves both legs in one walk. Single-leg formula calls
/// have their own measured deque path in `aroon_leg_into`.
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
            (highs.aroon(period, scale), lows.aroon(period, scale))
        } else {
            (f64::NAN, f64::NAN)
        };
        *up_slot = up;
        *down_slot = down;
    }
    Ok(())
}

/// Caller-owned single-leg AROON kernel used when a formula requests only one
/// of the two columns. This keeps the same inclusive window and tie / NaN
/// rules as [`aroon_into`] while avoiding work and storage for the unused leg.
#[inline]
pub(crate) fn aroon_leg_into<const WANT_MAX: bool>(
    values: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<()> {
    if output.len() != values.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    validate_input(values.len(), period + 1)?;

    let scale = 100.0 / period as f64;
    if (AROON_DEQUE_MIN_PERIOD..AROON_DEQUE_CAPACITY).contains(&period) {
        let mut extremes = MonotonicIndexDeque::new();
        for (index, slot) in output.iter_mut().enumerate() {
            let first_valid = index.saturating_sub(period);
            advance_monotonic_deque::<WANT_MAX>(values, index, first_valid, &mut extremes);
            *slot = if index >= period {
                extremes.front().map_or(f64::NAN, |extreme| {
                    (period - (index - extreme)) as f64 * scale
                })
            } else {
                f64::NAN
            };
        }
        return Ok(());
    }

    let mut tracker = ExtremeTracker::<WANT_MAX>::new();
    for (i, (&value, slot)) in values.iter().zip(output.iter_mut()).enumerate() {
        tracker.advance(values, i, value, period);
        *slot = if i >= period {
            tracker.aroon(period, scale)
        } else {
            f64::NAN
        };
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
    // The current index cancels from the two ages, so the result is
    // `(age_low - age_high) * 100 / period` — a single scaled subtraction
    // per bar over the same recurrence. A signed integer difference handles
    // either ordering with one conversion to `f64`; a window with no
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
            (lows.age as isize - highs.age as isize) as f64 * scale
        } else {
            f64::NAN
        };
    }
    Ok(Array1::from(output))
}
