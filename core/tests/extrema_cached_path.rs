//! Lock the cached-index rolling-extrema fast path.
//!
//! `math::statistics` serves the whole extrema family — `MAX`/`MIN`,
//! `MAXINDEX`/`MININDEX`, `MIDPOINT`, `MIDPRICE`, `WILLR`, `STOCH`, `AROON` —
//! through two strategies selected only by [`EXTREMA_CACHE_LIMIT`]: the cached
//! index for small windows and the monotonic ring for oversized ones. Both are
//! NaN-transparent, so the invariant that actually needs guarding is
//! *agreement*: each strategy must reproduce the other and a naive oracle, with
//! and without missing bars.
//!
//! These tests therefore check three things independently:
//!
//! 1. the public batch surface equals an O(n·window) oracle on clean data across
//!    both sides of the cache limit,
//! 2. the fused high/low kernel behind `MIDPOINT`/`MIDPRICE`/`WILLR` does too,
//! 3. series with holes are answered by the same dropna rule on both sides of
//!    the cache limit, and no finite bar is blanked by a missing neighbour.

use finkit::indicators::math_operators::{max, maxindex, min, minindex};
use finkit::indicators::momentum::willr;
use finkit::indicators::overlap::{midpoint, midprice};
use finkit::math::statistics::{rolling_max, rolling_min};

/// Deterministic xorshift walk: reproducible across runs and platforms, and
/// free of missing bars unless a test injects them.
fn walk(len: usize, seed: u64, step: f64) -> Vec<f64> {
    let mut state = seed | 1;
    let mut level = 100.0f64;
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let unit = ((state >> 11) as f64) / ((1u64 << 53) as f64);
        level += (unit - 0.5) * 2.0 * step;
        out.push(level);
    }
    out
}

fn naive_extreme(data: &[f64], window: usize, want_max: bool) -> Vec<f64> {
    let mut out = vec![f64::NAN; data.len()];
    if window == 0 || window > data.len() {
        return out;
    }
    for i in window - 1..data.len() {
        let slice = &data[i + 1 - window..=i];
        out[i] = if want_max {
            slice.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        } else {
            slice.iter().copied().fold(f64::INFINITY, f64::min)
        };
    }
    out
}

/// Naive oracle with the two rules the extrema family documents: a missing bar
/// inside a full window is skipped, and a window of all missing bars reports a
/// missing result. A leading run of missing bars does not fill the window, so
/// the first report lands `window - 1` bars after the first finite bar — the
/// same anchor the batch kernels use.
fn naive_extreme_dropna(data: &[f64], window: usize, want_max: bool) -> Vec<f64> {
    let mut out = vec![f64::NAN; data.len()];
    if window == 0 || window > data.len() {
        return out;
    }
    let start = data
        .iter()
        .position(|value| !value.is_nan())
        .unwrap_or(data.len());
    for i in start.saturating_add(window - 1)..data.len() {
        let slice = &data[i + 1 - window..=i];
        let mut best = if want_max {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
        let mut found = false;
        for value in slice {
            if value.is_nan() {
                continue;
            }
            if !found {
                best = *value;
                found = true;
            } else if want_max && *value >= best || !want_max && *value <= best {
                best = *value;
            }
        }
        out[i] = if found { best } else { f64::NAN };
    }
    out
}

fn assert_same_series(actual: &[f64], expected: &[f64], context: &str) {
    assert_eq!(actual.len(), expected.len(), "{context}: length");
    for (index, (got, want)) in actual.iter().zip(expected.iter()).enumerate() {
        match (got.is_nan(), want.is_nan()) {
            (true, true) => {}
            (false, false) => assert_eq!(
                got.to_bits(),
                want.to_bits(),
                "{context}: index {index} differs ({got} vs {want})"
            ),
            _ => panic!("{context}: index {index} NaN mismatch ({got} vs {want})"),
        }
    }
}

/// Windows straddle `EXTREMA_CACHE_LIMIT = 512` and every TA-Lib period class:
/// 1, the short 2-5 band, the 14-30 band, the 200-plus moving-average band, the
/// cache limit itself, and windows far past it.
const WINDOWS: [usize; 15] = [
    1, 2, 3, 5, 14, 30, 64, 128, 200, 255, 256, 511, 512, 513, 1024,
];

#[test]
fn batch_extrema_match_the_naive_oracle_across_the_cache_limit() {
    let data = walk(2048, 0x9E37_79B9_7F4A_7C15, 1.0);
    for window in WINDOWS {
        let expected_max = naive_extreme(&data, window, true);
        let expected_min = naive_extreme(&data, window, false);

        let by_max = max(&data, window).expect("max accepts a valid window");
        let by_min = min(&data, window).expect("min accepts a valid window");
        assert_same_series(by_max.as_slice().unwrap(), &expected_max, "max");
        assert_same_series(by_min.as_slice().unwrap(), &expected_min, "min");

        let by_rolling_max =
            rolling_max(&data, window).expect("rolling_max accepts a valid window");
        let by_rolling_min =
            rolling_min(&data, window).expect("rolling_min accepts a valid window");
        assert_same_series(
            by_rolling_max.as_slice().unwrap(),
            &expected_max,
            "rolling_max",
        );
        assert_same_series(
            by_rolling_min.as_slice().unwrap(),
            &expected_min,
            "rolling_min",
        );
    }
}

#[test]
fn warm_up_slots_stay_missing_and_are_never_back_filled() {
    for window in WINDOWS {
        // Long enough that every window in `WINDOWS`, including those past the
        // cache limit, is accepted by the indicator validators.
        let data = walk(2048, 0x2545_F491_4F6C_DD1D, 0.5);
        let out_max = max(&data, window).expect("valid window");
        let out_min = min(&data, window).expect("valid window");
        for index in 0..window - 1 {
            assert!(
                out_max[index].is_nan() && out_min[index].is_nan(),
                "window {window}: warm-up slot {index} must stay NaN"
            );
        }
        assert!(
            out_max[window - 1].is_finite() && out_min[window - 1].is_finite(),
            "window {window}: the first full window must be reported"
        );
    }
}

#[test]
fn fused_high_low_kernel_matches_the_naive_oracle() {
    let high = walk(1024, 0xDEAD_BEEF_CAFE_1234, 1.5);
    let low: Vec<f64> = high.iter().map(|value| value - 2.0).collect();
    let close: Vec<f64> = high
        .iter()
        .zip(low.iter())
        .map(|(h, l)| l + (h - l) * 0.4)
        .collect();

    for window in [1usize, 2, 3, 14, 30, 255, 511, 512, 513] {
        // `MIDPOINT` takes both extremes from one series; `MIDPRICE` and `WILLR`
        // take the high extreme from `high` and the low extreme from `low`.
        let one_series_high = naive_extreme(&high, window, true);
        let one_series_low = naive_extreme(&high, window, false);
        let span_high = naive_extreme(&high, window, true);
        let span_low = naive_extreme(&low, window, false);

        let mid = midpoint(&high, window).expect("midpoint accepts a valid window");
        for index in 0..high.len() {
            if one_series_high[index].is_nan() {
                assert!(
                    mid[index].is_nan(),
                    "midpoint window {window} index {index}"
                );
            } else {
                let expected = (one_series_high[index] + one_series_low[index]) / 2.0;
                assert_eq!(
                    mid[index].to_bits(),
                    expected.to_bits(),
                    "midpoint window {window} index {index}"
                );
            }
        }

        let mid_price = midprice(&high, &low, window).expect("midprice accepts a valid window");
        for index in 0..high.len() {
            if span_high[index].is_nan() {
                assert!(
                    mid_price[index].is_nan(),
                    "midprice window {window} index {index}"
                );
            } else {
                let expected = (span_high[index] + span_low[index]) / 2.0;
                assert_eq!(
                    mid_price[index].to_bits(),
                    expected.to_bits(),
                    "midprice window {window} index {index}"
                );
            }
        }

        let williams = willr(&high, &low, &close, window).expect("willr accepts a valid window");
        for index in 0..high.len() {
            if span_high[index].is_nan() {
                assert!(
                    williams[index].is_nan(),
                    "willr window {window} index {index}"
                );
            } else {
                let range = span_high[index] - span_low[index];
                let expected = -100.0 * (span_high[index] - close[index]) / range;
                assert!(
                    (williams[index] - expected).abs() < 1e-9,
                    "willr window {window} index {index}: {} vs {expected}",
                    williams[index]
                );
            }
        }
    }
}

/// `MAXINDEX`/`MININDEX` publish a position, so ties are observable there in a
/// way they are not for `MAX`/`MIN`: TA-Lib reports the earliest occurrence,
/// which the deque reaches by discarding only *strictly* worse tails.
#[test]
fn index_extremes_report_the_earliest_of_equal_values() {
    let data = [3.0, 7.0, 7.0, 7.0, 2.0, 9.0, 9.0, 1.0];
    // Offsets are relative to the window start, so a window whose extreme sits
    // at its oldest bar reports 0 even though the window itself has advanced.
    let expected_max = [-1i64, -1, 1, 0, 0, 2, 1, 0];
    let expected_min = [-1i64, -1, 0, 0, 2, 1, 0, 2];

    let got_max = maxindex(&data, 3).expect("maxindex accepts a valid window");
    let got_min = minindex(&data, 3).expect("minindex accepts a valid window");
    assert_eq!(got_max.to_vec(), expected_max.to_vec());
    assert_eq!(got_min.to_vec(), expected_min.to_vec());
}

/// A missing bar used to blank every window it sat in: `f64` comparisons against
/// `NaN` are `false`, so a `NaN` neither won nor evicted its neighbours, and
/// once it reached the queue front it masked a window holding a perfectly finite
/// value. The extrema family now drops missing bars and reports a missing result
/// only when nothing finite is left.
#[test]
fn missing_bars_are_dropped_not_poisoned() {
    let data = [5.0, f64::NAN, 3.0, 4.0, 6.0, f64::NAN, 2.0, 8.0];

    let got_max = max(&data, 3).expect("max accepts a valid window");
    let got_min = min(&data, 3).expect("min accepts a valid window");

    assert_same_series(
        got_max.as_slice().unwrap(),
        &naive_extreme_dropna(&data, 3, true),
        "max dropna",
    );
    assert_same_series(
        got_min.as_slice().unwrap(),
        &naive_extreme_dropna(&data, 3, false),
        "min dropna",
    );

    // The windows that used to read `NaN`: the hole no longer masks a finite bar.
    assert_eq!(got_max[3], 4.0, "[NaN, 3, 4] must report 4, not NaN");
    assert_eq!(got_min[3], 3.0, "[NaN, 3, 4] must report 3");
    assert_eq!(got_max[7], 8.0, "a trailing hole must not mask 8");
    assert_eq!(got_min[6], 2.0, "[6, NaN, 2] must report 2");
}

/// A window whose bars are all missing has no extreme to report.
#[test]
fn all_missing_windows_report_missing() {
    let data = [1.0, 2.0, 3.0, f64::NAN, f64::NAN, f64::NAN, 4.0, 5.0];

    let got_max = max(&data, 3).expect("max accepts a valid window");
    let got_min = min(&data, 3).expect("min accepts a valid window");
    let max_slice = got_max.as_slice().unwrap();
    let min_slice = got_min.as_slice().unwrap();

    assert_eq!(max_slice[2], 3.0, "a finite window is unaffected");

    // Bars 1..2 are still inside the windows at index 3 and 4, so the gap does
    // not make them missing — it only stops contributing a new extreme.
    assert_eq!(max_slice[3], 3.0, "[2, 3, NaN] reports 3");
    assert_eq!(min_slice[3], 2.0, "[2, 3, NaN] reports 2");
    assert_eq!(max_slice[4], 3.0, "[3, NaN, NaN] reports 3");
    assert_eq!(min_slice[4], 3.0, "[3, NaN, NaN] reports 3");

    // The gap is three bars long, so index 5 is the first window with nothing
    // finite left in it.
    assert!(max_slice[5].is_nan(), "[NaN, NaN, NaN] has no maximum");
    assert!(min_slice[5].is_nan(), "[NaN, NaN, NaN] has no minimum");

    assert_eq!(
        max_slice[6], 4.0,
        "the first finite bar after the gap is reported"
    );
    assert_eq!(min_slice[6], 4.0, "that same bar is also the minimum");
    assert_eq!(max_slice[7], 5.0, "the gap does not poison later windows");
    assert_eq!(
        min_slice[7], 4.0,
        "the bar before it is still inside the window"
    );

    assert_same_series(
        max_slice,
        &naive_extreme_dropna(&data, 3, true),
        "max after a gap",
    );
    assert_same_series(
        min_slice,
        &naive_extreme_dropna(&data, 3, false),
        "min after a gap",
    );
}

/// A leading run of missing bars must be skipped rather than emitted as
/// partial-window extremes: the first report sits at `first_finite + window - 1`,
/// which is the same rule `MA(X, 9)` and `SUM(X, 9)` obey. Without it a warm-up
/// prefix would leak maxima computed over 1..N-1 bars.
#[test]
fn a_leading_missing_run_delays_the_first_report_by_its_length() {
    let data = [f64::NAN, f64::NAN, 10.0, 8.0, 12.0, 6.0, 14.0];
    let period = 3;

    let got_max = max(&data, period).expect("max accepts a valid window");
    let got_min = min(&data, period).expect("min accepts a valid window");
    let max_slice = got_max.as_slice().unwrap();
    let min_slice = got_min.as_slice().unwrap();

    // The run is two bars long, so the first report is at 2 + 3 - 1 = 4.
    for index in 0..period {
        assert!(
            max_slice[index].is_nan() && min_slice[index].is_nan(),
            "index {index} must stay missing while the window is incomplete"
        );
    }
    assert_eq!(max_slice[4], 12.0, "the first full window reports 12");
    assert_eq!(min_slice[4], 8.0, "the first full window reports 8");
    assert_eq!(max_slice[5], 12.0, "[8, 12, 6]");
    assert_eq!(min_slice[5], 6.0, "[8, 12, 6]");
    assert_eq!(max_slice[6], 14.0, "[12, 6, 14]");
    assert_eq!(min_slice[6], 6.0, "[12, 6, 14]");

    // The shift is exactly the run's length: the tail agrees with a series that
    // starts with the same bars and no warm-up prefix.
    let tail = &data[2..];
    let tail_max = max(tail, period).expect("max accepts a valid window");
    assert_same_series(&max_slice[2..], tail_max.as_slice().unwrap(), "shifted max");
    assert_same_series(
        &min_slice[2..],
        min(tail, period).unwrap().as_slice().unwrap(),
        "shifted min",
    );
}

/// Removing the missing-value gate means both strategies can see a hole, so both
/// must answer it the same way.
#[test]
fn both_strategies_agree_when_the_series_has_holes() {
    let base = walk(2048, 0x5F37_59DF_8A5B_C3E7, 1.0);
    // Roughly one hole in nine bars, plus occasional back-to-back runs.
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let data: Vec<f64> = base
        .iter()
        .enumerate()
        .map(|(index, value)| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            if index % 9 == 0 || (index % 23 == 0 && state % 5 == 0) {
                f64::NAN
            } else {
                *value
            }
        })
        .collect();

    assert!(
        data.iter().any(|value| value.is_nan()),
        "the fixture must contain holes"
    );

    for (window, strategy) in [(30usize, "cached"), (513usize, "ring")] {
        for want_max in [true, false] {
            let got = if want_max {
                max(&data, window).expect("max accepts a valid window")
            } else {
                min(&data, window).expect("min accepts a valid window")
            };
            assert_same_series(
                got.as_slice().unwrap(),
                &naive_extreme_dropna(&data, window, want_max),
                &format!("{strategy} dropna"),
            );
        }
    }
}

/// The fused high/low kernel behind `MIDPRICE`/`WILLR` drops missing bars
/// independently on each leg.
#[test]
fn fused_kernel_drops_missing_bars_per_leg() {
    let high = [10.0, f64::NAN, 12.0, 14.0, 9.0, 16.0];
    let low = [8.0, 6.0, f64::NAN, 10.0, 5.0, 12.0];

    let got = midprice(&high, &low, 3).expect("midprice accepts a valid window");
    let slice = got.as_slice().unwrap();
    assert!(slice[0].is_nan() && slice[1].is_nan(), "warm-up stays NaN");

    assert_eq!(slice[2], 9.0, "high 12 / low 6");
    assert_eq!(slice[3], 10.0, "high 14 / low 6");
    assert_eq!(slice[4], 9.5, "high 14 / low 5");
    assert_eq!(slice[5], 10.5, "high 16 / low 5");
}

/// The cached index never allocates, so the fast path must not change with the
/// input length; this guards the `best_index + window <= i` arithmetic that
/// would otherwise wrap or drift on long series.
#[test]
fn cached_index_survives_long_series_at_the_cache_limit() {
    let data = walk(20_000, 0x0123_4567_89AB_CDEF, 0.2);
    for window in [30usize, 512] {
        let expected = naive_extreme(&data, window, true);
        let got = max(&data, window).expect("max accepts a valid window");
        assert_same_series(got.as_slice().unwrap(), &expected, "long series");
    }
}
