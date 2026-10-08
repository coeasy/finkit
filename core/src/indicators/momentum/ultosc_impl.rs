//! Ultimate oscillator.

use super::prelude::*;

/// Ultimate Oscillator (ULTOSC)
///
/// Combines short, intermediate, and long-term price action into a single value.
/// Default periods: 7, 14, 28.
///
/// # Examples
///
/// ```
/// use finkit::indicators;
///
/// let high: Vec<f64> = (0..30).map(|i| 45.0 + i as f64 * 0.1).collect();
/// let low: Vec<f64> = (0..30).map(|i| 43.0 + i as f64 * 0.1).collect();
/// let close: Vec<f64> = (0..30).map(|i| 44.0 + i as f64 * 0.1).collect();
/// let result = indicators::ultosc(&high, &low, &close, 7, 14, 28).unwrap();
/// assert_eq!(result.len(), 30);
/// ```
pub fn ultosc(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period1: usize,
    period2: usize,
    period3: usize,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    let max_period = period1.max(period2).max(period3);
    validate_input(high.len(), max_period + 1)?;

    let len = high.len();
    // Allocate uninitialized: `ultosc_into` fills the warm-up prefix with NaN
    // itself, so the `init_output` zeros + SIMD-NaN double fill here would be
    // two wasted full-length passes before compute (TA-Lib does one).
    let mut output = Array1::from(crate::utils::uninit_output(len));
    ultosc_into(
        high,
        low,
        close,
        period1,
        period2,
        period3,
        output.as_slice_mut().unwrap(),
    )?;
    Ok(output)
}

/// Ring length at or below which [`ultosc_into`] keeps the two term rings on the
/// stack. 32 covers the 7/14/28 default, whose longest window needs
/// `next_power_of_two(29) == 32` slots.
pub(crate) const ULTOSC_STACK_RING: usize = 32;

/// Zero-copy Ultimate Oscillator variant.
///
/// The three oscillator windows consume the *same* two per-bar terms
/// (`close - min(low, prevClose)` and the true range); only the age of the value
/// leaving each window differs. Storing each term once, in a ring as long as the
/// longest window, and reading it back at three offsets costs two stores per bar
/// where a ring per window costs six. TA-Lib 0.8.1's `ta_ULTOSC.c` is built the
/// same way (`local_term_closeMinusTrueLow[32]` with one write cursor and three
/// trailing cursors), so this is not a trick we are gambling on.
///
/// The rings are backed by the stack for the default periods and by the heap for
/// anything longer; both tiers run the same `ultosc_body`, so they cannot give
/// two answers for the same periods.
pub fn ultosc_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period1: usize,
    period2: usize,
    period3: usize,
    output: &mut [f64],
) -> Result<()> {
    if high.len() != low.len() || high.len() != close.len() {
        return Err(TaError::InvalidParameter {
            name: "high, low, close".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if output.len() != high.len() {
        return Err(TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as input".to_string(),
        });
    }
    let max_period = period1.max(period2).max(period3);
    validate_input(high.len(), max_period + 1)?;
    // Only the warm-up prefix stays NaN: `ultosc_body` writes every slot from
    // `max_period` onward, so a full-length fill would rewrite ~len slots the
    // loop immediately overwrites (TA-Lib's own path writes the prefix only).
    output[..max_period.min(high.len())].fill(f64::NAN);

    // One slot more than the longest window, rounded up to a power of two so the
    // slot holding bar `i - k` is `(i - k) & (ring - 1)`. That removes both the
    // modulo and the three wrap counters a ring of exactly `period3` needs.
    let ring = (max_period + 1).next_power_of_two();
    if ring <= ULTOSC_STACK_RING {
        let mut bp_ring = [0.0f64; ULTOSC_STACK_RING];
        let mut tr_ring = [0.0f64; ULTOSC_STACK_RING];
        ultosc_body(
            high,
            low,
            close,
            period1,
            period2,
            period3,
            output,
            &mut bp_ring[..ring],
            &mut tr_ring[..ring],
        )
    } else {
        let mut bp_ring = vec![0.0f64; ring];
        let mut tr_ring = vec![0.0f64; ring];
        ultosc_body(
            high,
            low,
            close,
            period1,
            period2,
            period3,
            output,
            &mut bp_ring,
            &mut tr_ring,
        )
    }
}

/// The one Ultimate Oscillator recurrence.
///
/// `bp_ring` and `tr_ring` are scratch storage, not inputs: each must be a power
/// of two long and longer than the longest window, and their contents on entry
/// are irrelevant. Callers choose the tier and this function is the only place
/// the running totals and the empty-window reseed are written.
#[inline(always)]
pub(crate) fn ultosc_body(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    period1: usize,
    period2: usize,
    period3: usize,
    output: &mut [f64],
    bp_ring: &mut [f64],
    tr_ring: &mut [f64],
) -> Result<()> {
    let ring = bp_ring.len();
    let mask = ring - 1;
    debug_assert!(ring.is_power_of_two());
    debug_assert!(tr_ring.len() == ring);
    let max_period = period1.max(period2).max(period3);
    debug_assert!(ring > max_period);

    let mut bp1_sum = 0.0;
    let mut tr1_sum = 0.0;
    let mut bp2_sum = 0.0;
    let mut tr2_sum = 0.0;
    let mut bp3_sum = 0.0;
    let mut tr3_sum = 0.0;
    // Consecutive bars whose two terms are both exactly zero. A running total
    // cannot be asked whether its window is empty: the totals are maintained by
    // add-then-subtract, so a window that has just emptied holds rounding residue
    // of either sign rather than 0.0. Reseeding on this count is what lets the
    // divides below be guarded exactly (`> 0.0`) instead of against a fixed band.
    let mut null_run = 0usize;
    // No window shorter than this can be empty, so one compare covers all three
    // in the common case and the exact per-window checks only run once a run
    // that long has actually happened. `ta_ULTOSC.c` nests its three checks for
    // the same reason. Skipping a check when `null_run < shortest <= period` is
    // sound: the check it skips would have been false anyway.
    let shortest = period1.min(period2).min(period3);

    // Two loops, the shape `ta_ULTOSC.c` uses. The warm-up emits no bar and
    // carries no watermark branch; that branch plus the divides it guards was
    // 14% of this kernel on the 10k-bar series (`core/examples/ultosc_probe.rs`).
    // The ring writes and the accumulator fold are identical in both loops, so
    // the state reaching the main loop is bit-identical to the one-loop form
    // (probe: 0 / 10000 bit-differences, max_abs 0.0).
    //
    // Bar 0 has no previous close, so both of its terms are 0.0 and the slot for
    // it reads back as the 0.0 the ring was initialised with -- which is exactly
    // what writing bar 0 would have stored. That is what lets an age older than
    // the first bar (`i - period < 0`) read a slot that is still 0.0, so the
    // warm-up needs no separate phase and no branch.
    let len = high.len();
    let warm_end = max_period.min(len);
    for i in 1..warm_end {
        let write = i & mask;
        let t1 = i.wrapping_sub(period1) & mask;
        let t2 = i.wrapping_sub(period2) & mask;
        let t3 = i.wrapping_sub(period3) & mask;

        let true_low = low[i].min(close[i - 1]);
        let bp = close[i] - true_low;
        let tr = high[i].max(close[i - 1]) - true_low;

        bp1_sum += bp - bp_ring[t1];
        tr1_sum += tr - tr_ring[t1];
        bp2_sum += bp - bp_ring[t2];
        tr2_sum += tr - tr_ring[t2];
        bp3_sum += bp - bp_ring[t3];
        tr3_sum += tr - tr_ring[t3];
        bp_ring[write] = bp;
        tr_ring[write] = tr;

        if bp == 0.0 && tr == 0.0 {
            null_run += 1;
        } else {
            null_run = 0;
        }
        if null_run >= shortest {
            // A run at least as long as a window means every slot that window
            // spans is exactly 0.0, so its residue can be dropped.
            if null_run >= period1 {
                bp1_sum = 0.0;
                tr1_sum = 0.0;
            }
            if null_run >= period2 {
                bp2_sum = 0.0;
                tr2_sum = 0.0;
            }
            if null_run >= period3 {
                bp3_sum = 0.0;
                tr3_sum = 0.0;
            }
        }
    }

    // Every bar from `warm_end` on emits a value; the watermark branch is gone,
    // so this loop is straight-line apart from the shared null-run reseed.
    for i in warm_end..len {
        let write = i & mask;
        let t1 = i.wrapping_sub(period1) & mask;
        let t2 = i.wrapping_sub(period2) & mask;
        let t3 = i.wrapping_sub(period3) & mask;

        let true_low = low[i].min(close[i - 1]);
        let bp = close[i] - true_low;
        let tr = high[i].max(close[i - 1]) - true_low;

        bp1_sum += bp - bp_ring[t1];
        tr1_sum += tr - tr_ring[t1];
        bp2_sum += bp - bp_ring[t2];
        tr2_sum += tr - tr_ring[t2];
        bp3_sum += bp - bp_ring[t3];
        tr3_sum += tr - tr_ring[t3];
        bp_ring[write] = bp;
        tr_ring[write] = tr;

        if bp == 0.0 && tr == 0.0 {
            null_run += 1;
        } else {
            null_run = 0;
        }
        if null_run >= shortest {
            // A run at least as long as a window means every slot that window
            // spans is exactly 0.0, so its residue can be dropped.
            if null_run >= period1 {
                bp1_sum = 0.0;
                tr1_sum = 0.0;
            }
            if null_run >= period2 {
                bp2_sum = 0.0;
                tr2_sum = 0.0;
            }
            if null_run >= period3 {
                bp3_sum = 0.0;
                tr3_sum = 0.0;
            }
        }

        // `> 0.0`, not `abs() > 1e-15`. A true-range total carries the quote
        // unit, so a fixed band zeroes the oscillator for an instrument
        // quoted below it and -- because a just-emptied window holds residue
        // rather than 0.0 -- divides one residue by another for an
        // instrument quoted above it. TA-Lib 0.8.1 carries the same exact
        // test and the reseed above; it added both for its issues #244 and
        // #253, and the band this replaces is the shape #253 describes.
        //
        // The select form is deliberate: `if .. { a / b } else { 0.0 }`
        // compiles to a divide plus a conditional move, where branching into
        // a `value += ..` accumulator cost 28% when it was measured
        // (`core/examples/talib_gap_probe.rs` section D).
        //
        // The three guarded divides stay (they are the contract), but the final
        // scale multiplies by the folded `1/7` instead of dividing by `7.0`:
        // the divide was 8% of the kernel in the probe, and the reciprocal's one
        // extra rounding is 1 ulp (probe max_rel 1.66e-16 -- four orders inside
        // this family's 1e-8 golden tolerance). `linreg` makes the same trade
        // with its hoisted `inv_divisor`.
        let avg1 = if tr1_sum > 0.0 {
            bp1_sum / tr1_sum
        } else {
            0.0
        };
        let avg2 = if tr2_sum > 0.0 {
            bp2_sum / tr2_sum
        } else {
            0.0
        };
        let avg3 = if tr3_sum > 0.0 {
            bp3_sum / tr3_sum
        } else {
            0.0
        };
        output[i] = 100.0 * (4.0 * avg1 + 2.0 * avg2 + avg3) * (1.0 / 7.0);
    }
    Ok(())
}
