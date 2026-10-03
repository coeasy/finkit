//! §18 convergence gates — one canonical kernel contract, five verified edges.
//!
//! ## Why this file exists
//!
//! The V4 plan (§18) requires that batch, streaming and range execution be
//! *three projections of one contract*, not three independent implementations
//! of the same mathematics. That property cannot be asserted by inspection: it
//! only holds while something keeps re-checking it. This file is that
//! something, and it covers the edges that had no systematic gate before:
//!
//! ```text
//! batch == streaming     <- `streaming_agrees_with_batch` (+ registry coverage)
//! full  == range         <- `range_execution_equals_full_for_stateless_plans`
//! allocating == into     <- `into_kernel_equals_allocating_kernel`
//! tree  == plan          <- provided by formula_plan_differential.rs (rostered)
//! Rust  == FFI           <- provided by ffi-common / binding suites (rostered)
//! ```
//!
//! The last two already had gates; deleting one silently was the remaining
//! risk, so `every_convergence_edge_has_an_artifact` rosters them instead of
//! duplicating their work.
//!
//! ## Non-rotting by construction
//!
//! The `batch == streaming` table is checked against
//! [`finkit::streaming::registry`] in both directions:
//!
//! * every case must name a registered indicator that is marked `streaming`;
//! * every registered streaming indicator's *category* must be represented in
//!   the table, or listed in `UNCOVERED_CATEGORIES` with a reason.
//!
//! So the table cannot silently drift out of date: adding a streaming indicator
//! in an uncovered category fails, renaming one fails, and removing a category
//! from `UNCOVERED_CATEGORIES` fails because the allowlist entries must still be
//! needed. The same pattern is used for the plan-kernel allowlist in
//! `formula_plan_differential.rs`.

use finkit::compute::{
    ComputeCapabilities, ComputeEffect, ComputeNode, ComputeNodeId, ComputePlan, DependencyShape,
    FactorPlan, LookbackRequirement,
};
use finkit::execution_plan::{HotExecutionPlan, KernelId};
use finkit::factors::{
    BorrowedFactorContext, FactorDefinition, FactorDirection, FactorEngine, FactorKind,
    FactorRegistry,
};
use finkit::indicators::{
    math_operators::add,
    math_transform::{exp, ln},
    momentum::{adx, cci, dx, minus_di, mom, plus_di, roc, rsi, trix, willr},
    price_transform::avgprice,
    statistics::zscore,
    volatility::{atr, natr, trange},
    volume::ad,
};
use finkit::math::moving_avg::{dema, ema, sma, wma};
use finkit::math::rolling_stats::stddev;
use finkit::math::rolling_stats::variance;
use finkit::runtime_context::RuntimeContext;
use finkit::semantic_graph::{NodeKind, SemanticGraph};
use finkit::state_arena::StateArena;
use finkit::streaming::indicators::{
    StreamingAd, StreamingAdd, StreamingAdx, StreamingAtr, StreamingAvgPrice, StreamingCci,
    StreamingDema, StreamingDx, StreamingEma, StreamingExp, StreamingLn, StreamingMinusDi,
    StreamingMom, StreamingNatr, StreamingObv, StreamingPlusDi, StreamingPsy, StreamingRoc,
    StreamingRsi, StreamingSma, StreamingTrange, StreamingTrix, StreamingWillR, StreamingWma,
    StreamingZscore,
};
use finkit::streaming::{registry, Ohlcv, StreamingIndicator};
use finkit::unified_executor::{
    ExecuteError, KernelCall, KernelDispatchError, KernelDispatcher, UnifiedExecutor,
};
use finkit::unified_runtime::{DirtyRange, GraphOptimization, UnifiedRuntime};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Relative tolerance used for every "these two paths agree" comparison.
///
/// The paths are expected to compute the *same* expression, so the tolerance
/// exists only to absorb the associativity differences an incremental update
/// inevitably introduces (rolling sum vs. a fresh window sum). A real
/// semantic divergence produces a much larger error than this.
const TOLERANCE: f64 = 1e-9;

/// Number of bars in the shared synthetic series.
const BARS: usize = 256;

/// Deterministic synthetic OHLCV input.
///
/// The gate must be reproducible without external fixtures, so the series comes
/// from an LCG. It is a *random walk*, not noise: a first-difference series
/// makes MOM/ROC/TRIX degenerate and would let a broken implementation pass.
struct Series {
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
}

impl Series {
    fn synthetic(len: usize) -> Self {
        // Numerical Recipes LCG constants; identical on every platform.
        let mut state: u64 = 0x2545_F491_4F6C_DD1D;
        let mut next_unit = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((state >> 11) as f64) / ((1u64 << 53) as f64)
        };

        let mut open = Vec::with_capacity(len);
        let mut high = Vec::with_capacity(len);
        let mut low = Vec::with_capacity(len);
        let mut close = Vec::with_capacity(len);
        let mut volume = Vec::with_capacity(len);

        let mut price = 100.0f64;
        let mut previous = price;
        for _ in 0..len {
            open.push(previous);
            // Drift plus a persistent component, so trends last long enough for
            // the longer exponential chains (TRIX) to have something to track.
            price += (next_unit() - 0.5) * 2.0 + 0.02;
            let body_high = open[open.len() - 1].max(price);
            let body_low = open[open.len() - 1].min(price);
            high.push(body_high + next_unit());
            low.push(body_low - next_unit());
            close.push(price);
            volume.push(1_000.0 + next_unit() * 500.0);
            previous = price;
        }

        Self {
            open,
            high,
            low,
            close,
            volume,
        }
    }

    fn len(&self) -> usize {
        self.close.len()
    }
}

/// Whether two values are the same result for convergence purposes.
///
/// NaN is only equal to NaN: a warm-up hole on one side and a number on the
/// other is a divergence, not a rounding difference. Infinities must match on
/// sign — `+inf` against `-inf` is a divergence too, and a tolerance test would
/// quietly call it equal.
fn agrees(reference: f64, candidate: f64) -> bool {
    if reference.is_nan() || candidate.is_nan() {
        return reference.is_nan() && candidate.is_nan();
    }
    if !reference.is_finite() || !candidate.is_finite() {
        return reference.is_infinite() == candidate.is_infinite()
            && reference.is_sign_positive() == candidate.is_sign_positive();
    }
    (reference - candidate).abs() <= TOLERANCE * reference.abs().max(1.0)
}

/// Flatten a batch kernel result into plain samples.
///
/// Batched kernels return `ndarray::Array1<f64>`, `into` kernels fill a
/// `Vec<f64>`; the gate only cares about the numbers, so it goes through
/// `IntoIterator` rather than naming either container.
fn to_vec<I: IntoIterator<Item = f64>>(result: finkit::Result<I>) -> Vec<f64> {
    result.map_or_else(
        |error| panic!("batch kernel failed: {error}"),
        |values| values.into_iter().collect(),
    )
}

// ───────────────────────── batch == streaming ─────────────────────────

/// Feed a single-series streaming indicator and collect its per-bar output.
fn drain<I>(mut indicator: I, input: &[f64]) -> Vec<Option<f64>>
where
    I: StreamingIndicator<f64, f64>,
{
    input.iter().map(|value| indicator.next(*value)).collect()
}

/// Feed a high/low/close streaming indicator and collect its per-bar output.
fn drain_hlc<I>(mut indicator: I, series: &Series) -> Vec<Option<f64>>
where
    I: StreamingIndicator<(f64, f64, f64), f64>,
{
    (0..series.len())
        .map(|i| indicator.next((series.high[i], series.low[i], series.close[i])))
        .collect()
}

/// Feed a full-bar streaming indicator and collect its per-bar output.
fn drain_ohlcv<I>(mut indicator: I, series: &Series) -> Vec<Option<f64>>
where
    I: for<'a> StreamingIndicator<&'a dyn Ohlcv, f64>,
{
    (0..series.len())
        .map(|i| {
            let bar = (
                series.open[i],
                series.high[i],
                series.low[i],
                series.close[i],
                series.volume[i],
            );
            indicator.next(&bar as &dyn Ohlcv)
        })
        .collect()
}

/// One `batch == streaming` case.
struct StreamingCase {
    /// Registered indicator name, resolved against the streaming registry.
    name: &'static str,
    /// Registry category, used for the coverage assertion.
    category: &'static str,
    /// Latest bar index at which the streaming path may still return `None`.
    ///
    /// Pinned deliberately: it is the number that would move if someone
    /// "optimised" a warm-up away, which is exactly the change the gate is here
    /// to catch.
    max_warmup: usize,
    /// Reference (batch) path.
    batch: fn(&Series) -> Vec<f64>,
    /// Incremental (streaming) path.
    stream: fn(&Series) -> Vec<Option<f64>>,
}

/// Categories with no case in `STREAMING_CASES`, and why.
///
/// Non-rotting: an entry that stops being needed fails
/// `streaming_categories_are_covered`, so this list can only shrink by
/// deliberate edit.
const UNCOVERED_CATEGORIES: &[(&str, &str)] = &[
    (
        "cycle",
        "Hilbert-transform family needs ~100 bars of phase history and is \
         precision-sensitive; it is gated by golden_talib_tests.rs instead",
    ),
    (
        "astock",
        "MONEY_FLOW on the streaming surface is a rolling typical-price*volume \
         sum, a different product from indicators::astock::money_flow, so there \
         is no batch counterpart to compare against",
    ),
];

// Per-case adapters. Each one pins its own period so the table stays a plain
// data structure rather than a closure zoo.

fn batch_sma(s: &Series) -> Vec<f64> {
    to_vec(sma(&s.close, 14))
}
fn stream_sma(s: &Series) -> Vec<Option<f64>> {
    drain(StreamingSma::new(14), &s.close)
}

fn batch_ema(s: &Series) -> Vec<f64> {
    to_vec(ema(&s.close, 14))
}
fn stream_ema(s: &Series) -> Vec<Option<f64>> {
    drain(StreamingEma::new(14), &s.close)
}

fn batch_wma(s: &Series) -> Vec<f64> {
    to_vec(wma(&s.close, 14))
}
fn stream_wma(s: &Series) -> Vec<Option<f64>> {
    drain(StreamingWma::new(14), &s.close)
}

fn batch_dema(s: &Series) -> Vec<f64> {
    to_vec(dema(&s.close, 14))
}
fn stream_dema(s: &Series) -> Vec<Option<f64>> {
    drain(StreamingDema::new(14), &s.close)
}

fn batch_rsi(s: &Series) -> Vec<f64> {
    to_vec(rsi(&s.close, 14))
}
fn stream_rsi(s: &Series) -> Vec<Option<f64>> {
    drain(StreamingRsi::new(14), &s.close)
}

fn batch_mom(s: &Series) -> Vec<f64> {
    to_vec(mom(&s.close, 10))
}
fn stream_mom(s: &Series) -> Vec<Option<f64>> {
    drain(StreamingMom::new(10), &s.close)
}

fn batch_roc(s: &Series) -> Vec<f64> {
    to_vec(roc(&s.close, 10))
}
fn stream_roc(s: &Series) -> Vec<Option<f64>> {
    drain(StreamingRoc::new(10), &s.close)
}

fn batch_trix(s: &Series) -> Vec<f64> {
    to_vec(trix(&s.close, 9))
}
fn stream_trix(s: &Series) -> Vec<Option<f64>> {
    drain(StreamingTrix::new(9), &s.close)
}

fn batch_zscore(s: &Series) -> Vec<f64> {
    to_vec(zscore(&s.close, 20))
}
fn stream_zscore(s: &Series) -> Vec<Option<f64>> {
    drain(StreamingZscore::new(20), &s.close)
}

fn batch_exp(s: &Series) -> Vec<f64> {
    to_vec(exp(&s.close))
}
fn stream_exp(s: &Series) -> Vec<Option<f64>> {
    drain(StreamingExp::new(), &s.close)
}

fn batch_ln(s: &Series) -> Vec<f64> {
    to_vec(ln(&s.close))
}
fn stream_ln(s: &Series) -> Vec<Option<f64>> {
    drain(StreamingLn::new(), &s.close)
}

fn batch_psy(s: &Series) -> Vec<f64> {
    to_vec(finkit::indicators::china::psy(&s.close, 12))
}
fn stream_psy(s: &Series) -> Vec<Option<f64>> {
    drain(StreamingPsy::new(12), &s.close)
}

fn batch_cci(s: &Series) -> Vec<f64> {
    to_vec(cci(&s.high, &s.low, &s.close, 20))
}
fn stream_cci(s: &Series) -> Vec<Option<f64>> {
    drain_hlc(StreamingCci::new(20), s)
}

fn batch_willr(s: &Series) -> Vec<f64> {
    to_vec(willr(&s.high, &s.low, &s.close, 14))
}
fn stream_willr(s: &Series) -> Vec<Option<f64>> {
    drain_ohlcv(StreamingWillR::new(14), s)
}

fn batch_adx(s: &Series) -> Vec<f64> {
    to_vec(adx(&s.high, &s.low, &s.close, 14))
}
fn stream_adx(s: &Series) -> Vec<Option<f64>> {
    drain_hlc(StreamingAdx::new(14), s)
}

fn batch_dx(s: &Series) -> Vec<f64> {
    to_vec(dx(&s.high, &s.low, &s.close, 14))
}
fn stream_dx(s: &Series) -> Vec<Option<f64>> {
    drain_hlc(StreamingDx::new(14), s)
}

fn batch_plus_di(s: &Series) -> Vec<f64> {
    to_vec(plus_di(&s.high, &s.low, &s.close, 14))
}
fn stream_plus_di(s: &Series) -> Vec<Option<f64>> {
    drain_hlc(StreamingPlusDi::new(14), s)
}

fn batch_minus_di(s: &Series) -> Vec<f64> {
    to_vec(minus_di(&s.high, &s.low, &s.close, 14))
}
fn stream_minus_di(s: &Series) -> Vec<Option<f64>> {
    drain_hlc(StreamingMinusDi::new(14), s)
}

fn batch_trange(s: &Series) -> Vec<f64> {
    to_vec(trange(&s.high, &s.low, &s.close))
}
fn stream_trange(s: &Series) -> Vec<Option<f64>> {
    drain_hlc(StreamingTrange::new(), s)
}

fn batch_atr(s: &Series) -> Vec<f64> {
    to_vec(atr(&s.high, &s.low, &s.close, 14))
}
fn stream_atr(s: &Series) -> Vec<Option<f64>> {
    drain_hlc(StreamingAtr::new(14), s)
}

fn batch_natr(s: &Series) -> Vec<f64> {
    to_vec(natr(&s.high, &s.low, &s.close, 14))
}
fn stream_natr(s: &Series) -> Vec<Option<f64>> {
    drain_ohlcv(StreamingNatr::new(14), s)
}

fn batch_obv(s: &Series) -> Vec<f64> {
    to_vec(finkit::indicators::volume::obv(&s.close, &s.volume))
}
fn stream_obv(s: &Series) -> Vec<Option<f64>> {
    drain_ohlcv(StreamingObv::new(), s)
}

fn batch_ad(s: &Series) -> Vec<f64> {
    to_vec(ad(&s.high, &s.low, &s.close, &s.volume))
}
fn stream_ad(s: &Series) -> Vec<Option<f64>> {
    drain_ohlcv(StreamingAd::new(), s)
}

fn batch_avgprice(s: &Series) -> Vec<f64> {
    to_vec(avgprice(&s.open, &s.high, &s.low, &s.close))
}
fn stream_avgprice(s: &Series) -> Vec<Option<f64>> {
    drain_ohlcv(StreamingAvgPrice::new(), s)
}

fn batch_add(s: &Series) -> Vec<f64> {
    to_vec(add(&s.close, &s.volume))
}
fn stream_add(s: &Series) -> Vec<Option<f64>> {
    // `ADD` is a two-input operator, so it uses the pair API rather than the
    // single-value `next`.
    let mut indicator = StreamingAdd::new();
    s.close
        .iter()
        .zip(s.volume.iter())
        .map(|(a, b)| indicator.next_pair(*a, *b))
        .collect()
}

const STREAMING_CASES: &[StreamingCase] = &[
    StreamingCase {
        name: "SMA",
        category: "overlap",
        max_warmup: 14,
        batch: batch_sma,
        stream: stream_sma,
    },
    StreamingCase {
        name: "EMA",
        category: "overlap",
        max_warmup: 14,
        batch: batch_ema,
        stream: stream_ema,
    },
    StreamingCase {
        name: "WMA",
        category: "overlap",
        max_warmup: 14,
        batch: batch_wma,
        stream: stream_wma,
    },
    StreamingCase {
        name: "DEMA",
        category: "overlap",
        // Two cascaded period-14 EMAs: the second cannot start until the first
        // has emitted, so ~2*period bars of warm-up is structural, not slack.
        max_warmup: 28,
        batch: batch_dema,
        stream: stream_dema,
    },
    StreamingCase {
        name: "RSI",
        category: "momentum",
        max_warmup: 15,
        batch: batch_rsi,
        stream: stream_rsi,
    },
    StreamingCase {
        name: "MOM",
        category: "momentum",
        max_warmup: 10,
        batch: batch_mom,
        stream: stream_mom,
    },
    StreamingCase {
        name: "ROC",
        category: "momentum",
        max_warmup: 10,
        batch: batch_roc,
        stream: stream_roc,
    },
    StreamingCase {
        name: "TRIX",
        category: "momentum",
        max_warmup: 30,
        batch: batch_trix,
        stream: stream_trix,
    },
    StreamingCase {
        name: "CCI",
        category: "momentum",
        max_warmup: 20,
        batch: batch_cci,
        stream: stream_cci,
    },
    StreamingCase {
        name: "Williams %R",
        category: "momentum",
        max_warmup: 14,
        batch: batch_willr,
        stream: stream_willr,
    },
    StreamingCase {
        name: "ADX",
        category: "momentum",
        // Wilder ADX needs `2 * period` bars: the first value lands on index
        // `2*period - 1`, which is also what `registry::by_id("ADX").convergence`
        // reports.
        max_warmup: 27,
        batch: batch_adx,
        stream: stream_adx,
    },
    StreamingCase {
        name: "DX",
        category: "momentum",
        // Wilder: first value on index `period`.
        max_warmup: 14,
        batch: batch_dx,
        stream: stream_dx,
    },
    StreamingCase {
        name: "PLUS_DI",
        category: "momentum",
        max_warmup: 14,
        batch: batch_plus_di,
        stream: stream_plus_di,
    },
    StreamingCase {
        name: "MINUS_DI",
        category: "momentum",
        max_warmup: 14,
        batch: batch_minus_di,
        stream: stream_minus_di,
    },
    StreamingCase {
        name: "ZSCORE",
        category: "statistics",
        max_warmup: 20,
        batch: batch_zscore,
        stream: stream_zscore,
    },
    StreamingCase {
        name: "TRANGE",
        category: "volatility",
        max_warmup: 1,
        batch: batch_trange,
        stream: stream_trange,
    },
    StreamingCase {
        name: "ATR",
        category: "volatility",
        max_warmup: 15,
        batch: batch_atr,
        stream: stream_atr,
    },
    StreamingCase {
        name: "NATR",
        category: "volatility",
        max_warmup: 15,
        batch: batch_natr,
        stream: stream_natr,
    },
    StreamingCase {
        name: "OBV",
        category: "volume",
        max_warmup: 1,
        batch: batch_obv,
        stream: stream_obv,
    },
    StreamingCase {
        name: "AD",
        category: "volume",
        max_warmup: 1,
        batch: batch_ad,
        stream: stream_ad,
    },
    StreamingCase {
        name: "EXP",
        category: "math_transform",
        max_warmup: 1,
        batch: batch_exp,
        stream: stream_exp,
    },
    StreamingCase {
        name: "LN",
        category: "math_transform",
        max_warmup: 1,
        batch: batch_ln,
        stream: stream_ln,
    },
    StreamingCase {
        name: "ADD",
        category: "math_operators",
        max_warmup: 1,
        batch: batch_add,
        stream: stream_add,
    },
    StreamingCase {
        name: "AVGPRICE",
        category: "price_transform",
        max_warmup: 1,
        batch: batch_avgprice,
        stream: stream_avgprice,
    },
    StreamingCase {
        name: "PSY",
        category: "sentiment",
        max_warmup: 13,
        batch: batch_psy,
        stream: stream_psy,
    },
];

#[test]
fn streaming_agrees_with_batch() {
    let series = Series::synthetic(BARS);

    for case in STREAMING_CASES {
        let batch = (case.batch)(&series);
        let stream = (case.stream)(&series);

        assert_eq!(
            batch.len(),
            series.len(),
            "{}: batch path returned {} values for {} bars",
            case.name,
            batch.len(),
            series.len()
        );
        assert_eq!(
            stream.len(),
            series.len(),
            "{}: streaming path emitted {} values for {} bars",
            case.name,
            stream.len(),
            series.len()
        );

        let first = stream.iter().position(Option::is_some).unwrap_or_else(|| {
            panic!(
                "{}: streaming path never converged over {} bars",
                case.name,
                series.len()
            )
        });
        assert!(
            first <= case.max_warmup,
            "{}: streaming converged at bar {first}, later than the pinned \
             warm-up of {} — either the kernel regressed or the pin is stale",
            case.name,
            case.max_warmup
        );

        let mut compared = 0usize;
        for (index, value) in stream.iter().enumerate() {
            match value {
                Some(value) => {
                    assert!(
                        agrees(batch[index], *value),
                        "{}: batch={} streaming={} at bar {index}",
                        case.name,
                        batch[index],
                        value
                    );
                    compared += 1;
                }
                None => assert!(
                    index < first,
                    "{}: streaming returned None at bar {index} after converging at bar {first}",
                    case.name
                ),
            }
        }
        assert!(
            compared >= series.len() - case.max_warmup,
            "{}: only {compared} bars were actually compared; a converging case must \
             yield at least {} values",
            case.name,
            series.len() - case.max_warmup
        );
    }
}

#[test]
fn streaming_cases_resolve_in_the_registry() {
    for case in STREAMING_CASES {
        let entry = registry::by_id(case.name).unwrap_or_else(|| {
            panic!(
                "{}: not a registered indicator — the table has drifted from the SSOT",
                case.name
            )
        });
        assert!(
            entry.streaming,
            "{}: listed in the convergence table but the registry says streaming = false",
            case.name
        );
        assert_eq!(
            entry.category, case.category,
            "{}: registry category changed, so the coverage assertion below would \
             silently stop covering it",
            case.name
        );
    }
}

#[test]
fn streaming_categories_are_covered() {
    let covered: BTreeSet<&str> = STREAMING_CASES.iter().map(|case| case.category).collect();
    let allowed: BTreeSet<&str> = UNCOVERED_CATEGORIES
        .iter()
        .map(|(category, _)| *category)
        .collect();

    let mut uncovered: BTreeSet<&str> = BTreeSet::new();
    for indicator in registry::all_indicators() {
        if indicator.streaming && !covered.contains(indicator.category) {
            uncovered.insert(indicator.category);
        }
    }

    for category in &uncovered {
        assert!(
            allowed.contains(category),
            "convergence table has no case in streaming category {category:?} \
             and it is not allowlisted; add a case or record why it cannot have one"
        );
    }
    for category in &allowed {
        assert!(
            uncovered.contains(category),
            "allowlisted category {category:?} is now covered (or no longer streamable), \
             so its entry in UNCOVERED_CATEGORIES is stale and must be removed"
        );
    }
}

// ───────────────────────── allocating == into ─────────────────────────

/// Run one allocating kernel and one caller-owned-output kernel over the same
/// input and return `(allocating, into)`.
type IntoPair = fn(&Series) -> (Vec<f64>, Vec<f64>);

struct IntoCase {
    name: &'static str,
    run: IntoPair,
}

/// Scratch buffer sized from the series; `_into` kernels validate the length.
fn blank(series: &Series) -> Vec<f64> {
    vec![0.0; series.len()]
}

fn into_sma(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::math::moving_avg::sma_into(&s.close, 14, &mut out).expect("sma_into");
    (to_vec(sma(&s.close, 14)), out)
}

fn into_ema(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::math::moving_avg::ema_into(&s.close, 14, &mut out).expect("ema_into");
    (to_vec(ema(&s.close, 14)), out)
}

fn into_wma(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::math::moving_avg::wma_into(&s.close, 14, &mut out).expect("wma_into");
    (to_vec(wma(&s.close, 14)), out)
}

fn into_dema(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::math::moving_avg::dema_into(&s.close, 14, &mut out).expect("dema_into");
    (to_vec(dema(&s.close, 14)), out)
}

fn into_trima(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::math::moving_avg::trima_into(&s.close, 14, &mut out).expect("trima_into");
    (to_vec(finkit::math::moving_avg::trima(&s.close, 14)), out)
}

fn into_variance(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::math::rolling_stats::variance_into(&s.close, 20, &mut out).expect("variance_into");
    (to_vec(variance(&s.close, 20)), out)
}

fn into_stddev(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::math::rolling_stats::stddev_into(&s.close, 20, 1.0, &mut out).expect("stddev_into");
    (to_vec(stddev(&s.close, 20, 1.0)), out)
}

fn into_rsi(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::indicators::momentum::rsi_into(&s.close, 14, &mut out).expect("rsi_into");
    (to_vec(rsi(&s.close, 14)), out)
}

fn into_mom(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::indicators::momentum::mom_into(&s.close, 10, &mut out).expect("mom_into");
    (to_vec(mom(&s.close, 10)), out)
}

fn into_roc(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::indicators::momentum::roc_into(&s.close, 10, &mut out).expect("roc_into");
    (to_vec(roc(&s.close, 10)), out)
}

fn into_cci(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::indicators::momentum::cci_into(&s.high, &s.low, &s.close, 20, &mut out)
        .expect("cci_into");
    (to_vec(cci(&s.high, &s.low, &s.close, 20)), out)
}

fn into_willr(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::indicators::momentum::willr_into(&s.high, &s.low, &s.close, 14, &mut out)
        .expect("willr_into");
    (to_vec(willr(&s.high, &s.low, &s.close, 14)), out)
}

fn into_adx(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::indicators::momentum::adx_into(&s.high, &s.low, &s.close, 14, &mut out)
        .expect("adx_into");
    (to_vec(adx(&s.high, &s.low, &s.close, 14)), out)
}

fn into_trix(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::indicators::momentum::trix_into(&s.close, 9, &mut out).expect("trix_into");
    (to_vec(trix(&s.close, 9)), out)
}

fn into_midpoint(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::indicators::overlap::midpoint_into(&s.close, 14, &mut out).expect("midpoint_into");
    (
        to_vec(finkit::indicators::overlap::midpoint(&s.close, 14)),
        out,
    )
}

fn into_midprice(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::indicators::overlap::midprice_into(&s.high, &s.low, 14, &mut out)
        .expect("midprice_into");
    (
        to_vec(finkit::indicators::overlap::midprice(&s.high, &s.low, 14)),
        out,
    )
}

fn into_t3(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::indicators::overlap::t3_into(&s.close, 5, 0.7, &mut out).expect("t3_into");
    (
        to_vec(finkit::indicators::overlap::t3(&s.close, 5, 0.7)),
        out,
    )
}

fn into_atr(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::indicators::volatility::atr_into(&s.high, &s.low, &s.close, 14, &mut out)
        .expect("atr_into");
    (to_vec(atr(&s.high, &s.low, &s.close, 14)), out)
}

fn into_natr(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::indicators::volatility::natr_into(&s.high, &s.low, &s.close, 14, &mut out)
        .expect("natr_into");
    (to_vec(natr(&s.high, &s.low, &s.close, 14)), out)
}

fn into_trange(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::indicators::volatility::trange_into(&s.high, &s.low, &s.close, &mut out)
        .expect("trange_into");
    (to_vec(trange(&s.high, &s.low, &s.close)), out)
}

fn into_ad(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::indicators::volume::ad_into(&s.high, &s.low, &s.close, &s.volume, &mut out)
        .expect("ad_into");
    (to_vec(ad(&s.high, &s.low, &s.close, &s.volume)), out)
}

fn into_zscore(s: &Series) -> (Vec<f64>, Vec<f64>) {
    let mut out = blank(s);
    finkit::indicators::statistics::zscore_into(&s.close, 20, &mut out).expect("zscore_into");
    (to_vec(zscore(&s.close, 20)), out)
}

/// `_into` kernels that are genuinely separate implementations from their
/// allocating siblings.
///
/// Wrappers of the form `fn x_into(..) { let v = x(..)?; output.copy_from_slice(..) }`
/// are deliberately excluded: they cannot disagree with `x`, so listing them
/// would inflate the coverage claim without testing anything. The gate over the
/// *hot* kernels is the one that matters, and those are exactly the ones below.
const INTO_CASES: &[IntoCase] = &[
    IntoCase {
        name: "sma",
        run: into_sma,
    },
    IntoCase {
        name: "ema",
        run: into_ema,
    },
    IntoCase {
        name: "wma",
        run: into_wma,
    },
    IntoCase {
        name: "dema",
        run: into_dema,
    },
    IntoCase {
        name: "trima",
        run: into_trima,
    },
    IntoCase {
        name: "variance",
        run: into_variance,
    },
    IntoCase {
        name: "stddev",
        run: into_stddev,
    },
    IntoCase {
        name: "rsi",
        run: into_rsi,
    },
    IntoCase {
        name: "mom",
        run: into_mom,
    },
    IntoCase {
        name: "roc",
        run: into_roc,
    },
    IntoCase {
        name: "cci",
        run: into_cci,
    },
    IntoCase {
        name: "willr",
        run: into_willr,
    },
    IntoCase {
        name: "adx",
        run: into_adx,
    },
    IntoCase {
        name: "trix",
        run: into_trix,
    },
    IntoCase {
        name: "midpoint",
        run: into_midpoint,
    },
    IntoCase {
        name: "midprice",
        run: into_midprice,
    },
    IntoCase {
        name: "t3",
        run: into_t3,
    },
    IntoCase {
        name: "atr",
        run: into_atr,
    },
    IntoCase {
        name: "natr",
        run: into_natr,
    },
    IntoCase {
        name: "trange",
        run: into_trange,
    },
    IntoCase {
        name: "ad",
        run: into_ad,
    },
    IntoCase {
        name: "zscore",
        run: into_zscore,
    },
];

#[test]
fn into_kernel_equals_allocating_kernel() {
    let series = Series::synthetic(BARS);
    for case in INTO_CASES {
        let (allocating, into) = (case.run)(&series);
        assert_eq!(
            allocating.len(),
            into.len(),
            "{}: allocating returned {} values, into returned {}",
            case.name,
            allocating.len(),
            into.len()
        );
        for (index, (reference, candidate)) in allocating.iter().zip(into.iter()).enumerate() {
            assert!(
                agrees(*reference, *candidate),
                "{}: allocating={reference} into={candidate} at bar {index}",
                case.name
            );
        }
    }
    assert_eq!(INTO_CASES.len(), 22, "pinned case count changed");
}

// ───────────────────────── full == range ─────────────────────────

const PLUS_ONE: KernelId = KernelId::from_static("CONVERGENCE_PLUS_ONE");
const DOUBLE: KernelId = KernelId::from_static("CONVERGENCE_DOUBLE");
const SUM: KernelId = KernelId::from_static("CONVERGENCE_SUM");

/// Stateless test kernels. Statelessness is the point: `execute_range` is only
/// sound for a plan whose nodes need a bounded lookback, which is precisely the
/// `DependencyShape::FixedLookback(0)` case these nodes declare.
struct ArithmeticDispatcher;

impl KernelDispatcher for ArithmeticDispatcher {
    fn dispatch(
        &mut self,
        call: KernelCall<'_>,
        buffers: &mut [Vec<f64>],
        _states: &mut StateArena,
    ) -> Result<(), KernelDispatchError> {
        let output = call.output.0;
        // Clone the inputs so this test dispatcher never has to reason about
        // slot aliasing; it is measuring the executor, not the kernels.
        let inputs: Vec<Vec<f64>> = call
            .inputs
            .iter()
            .map(|slot| buffers[slot.0].clone())
            .collect();

        match call.kernel {
            PLUS_ONE if inputs.len() == 1 => {
                for (dst, src) in buffers[output].iter_mut().zip(inputs[0].iter()) {
                    *dst = *src + 1.0;
                }
            }
            DOUBLE if inputs.len() == 1 => {
                for (dst, src) in buffers[output].iter_mut().zip(inputs[0].iter()) {
                    *dst = *src * 2.0;
                }
            }
            SUM if inputs.len() == 2 => {
                for (index, dst) in buffers[output].iter_mut().enumerate() {
                    *dst = inputs[0][index] + inputs[1][index];
                }
            }
            _ => return Err(KernelDispatchError::new(1)),
        }
        Ok(())
    }
}

fn stateless() -> ComputeCapabilities {
    ComputeCapabilities {
        deterministic: true,
        streaming: false,
        stateful: false,
        lookback: LookbackRequirement::None,
        dependency: DependencyShape::FixedLookback(0),
        effect: ComputeEffect::Pure,
    }
}

/// `CLOSE -> (+1)`, `CLOSE -> (*2)`, then their sum. A diamond, so the plan has
/// a shared input, two live intermediate buffers and one retained output.
fn diamond_plan() -> HotExecutionPlan {
    let semantic = ComputePlan::compile([
        ComputeNode::new(ComputeNodeId(0), "VARIABLE:CLOSE", vec![], stateless()),
        ComputeNode::new(
            ComputeNodeId(1),
            "CONVERGENCE_PLUS_ONE",
            vec![ComputeNodeId(0)],
            stateless(),
        ),
        ComputeNode::new(
            ComputeNodeId(2),
            "CONVERGENCE_DOUBLE",
            vec![ComputeNodeId(0)],
            stateless(),
        ),
        ComputeNode::new(
            ComputeNodeId(3),
            "CONVERGENCE_SUM",
            vec![ComputeNodeId(1), ComputeNodeId(2)],
            stateless(),
        ),
    ])
    .expect("diamond plan compiles");
    HotExecutionPlan::compile(&semantic, [ComputeNodeId(3)])
        .expect("diamond plan is hot-compilable")
}

#[test]
fn range_execution_equals_full_for_stateless_plans() {
    let close: Vec<f64> = (0..64).map(|i| 100.0 + (i as f64) * 0.5).collect();
    let mut executor = UnifiedExecutor::new(diamond_plan(), ArithmeticDispatcher);

    let full = executor.execute(&[&close]).expect("full execution");
    assert_eq!(full.len(), 1);
    // 3x + 1 for the diamond above.
    for (index, value) in full.values[0].iter().enumerate() {
        let expected = close[index] * 3.0 + 1.0;
        assert!(
            agrees(expected, *value),
            "full execution wrong at bar {index}: {value} != {expected}"
        );
    }

    // Dirty windows of every alignment: single row, odd start, trailing rows.
    for window in [4..5usize, 7..23, 30..63, 63..64] {
        let ranged = executor
            .execute_range(&[&close], window.clone())
            .expect("range execution");
        assert_eq!(ranged.len(), 1);
        assert_eq!(
            ranged.values[0].len(),
            window.len(),
            "range execution must return exactly the requested window"
        );
        for (offset, value) in ranged.values[0].iter().enumerate() {
            let index = window.start + offset;
            assert!(
                agrees(full.values[0][index], *value),
                "range and full disagree at bar {index}: {value} != {}",
                full.values[0][index]
            );
        }
    }
}

#[test]
fn context_separates_full_and_range_executions() {
    let close: Vec<f64> = (0..32).map(|i| 50.0 + i as f64).collect();
    let mut executor = UnifiedExecutor::new(diamond_plan(), ArithmeticDispatcher);

    let full = executor.execute(&[&close]).expect("full execution");
    let mut spliced = full.values[0].clone();
    let ranged = executor.execute_range(&[&close], 10..20).expect("range");
    spliced[10..20].copy_from_slice(&ranged.values[0]);

    for (index, (reference, candidate)) in full.values[0].iter().zip(spliced.iter()).enumerate() {
        assert!(
            agrees(*reference, *candidate),
            "splicing a range execution changed bar {index}"
        );
    }

    let metrics = executor.context().metrics();
    assert_eq!(metrics.full_executions, 1);
    assert_eq!(metrics.range_executions, 1);
    assert_eq!(metrics.buffers_taken, metrics.buffers_recycled);
}

#[test]
fn factor_dirty_range_splice_equals_full_execution() {
    let mut registry = FactorRegistry::new();
    registry
        .register(FactorDefinition::new(
            "momentum",
            ["close"],
            FactorKind::TimeSeries,
            FactorDirection::HigherBetter,
            // A 3-bar trailing window: exactly the shape a range execution must
            // be able to seed from history.
            Arc::new(|inputs| {
                let close = inputs.get("close")?;
                Ok(close
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        if index < 2 {
                            f64::NAN
                        } else {
                            value * 2.0 - close[index - 2]
                        }
                    })
                    .collect())
            }),
        ))
        .unwrap();
    let plan = FactorPlan::compile(&registry, &["momentum"]).unwrap();
    let engine = FactorEngine::new(registry);

    let original = Series::synthetic(48);
    let context = BorrowedFactorContext::new()
        .with_series("close", &original.close)
        .unwrap();
    let full = UnifiedRuntime::execute_factor_plan_borrowed(&plan, &engine, &context).unwrap();

    // Mutate one interior row and recompute only what it can affect.
    let mut mutated = original.close.clone();
    mutated[20] += 7.0;
    let mutated_context = BorrowedFactorContext::new()
        .with_series("close", &mutated)
        .unwrap();
    let mut runtime = RuntimeContext::new();
    let mut spliced = full.output.clone();
    let trace = UnifiedRuntime::execute_factor_plan_range_into_borrowed_with_context(
        &plan,
        &engine,
        &mutated_context,
        &mut spliced,
        DirtyRange::new(20, 21),
        DependencyShape::FixedLookback(2),
        &mut runtime,
    )
    .unwrap();
    assert_eq!(runtime.metrics().executions(), 1);
    assert_eq!(runtime.metrics().range_executions, 1);

    // §20, priority 4: the range path has to do *strictly less work* than a
    // full recompute, or it is an API curiosity with a correctness cost.
    assert_eq!(trace.rows, original.len());
    assert_eq!(trace.executed_nodes, 1);
    assert!(
        trace.recomputed_rows < trace.rows,
        "dirty-range execution recomputed {} of {} rows",
        trace.recomputed_rows,
        trace.rows
    );
    assert!(
        trace.recomputed_rows <= 1 + 2 * 2,
        "one dirty row must not drag in more than the row itself plus the two \
         history rows needed to propagate forward and seed backward; got {}",
        trace.recomputed_rows
    );

    // The spliced result must be exactly what a full recompute produces.
    let reference = UnifiedRuntime::execute_factor_plan_borrowed(&plan, &engine, &mutated_context)
        .unwrap()
        .output;
    for (name, values) in &reference {
        let spliced_values = spliced.get(name).expect("retained output");
        for (index, expected) in values.iter().enumerate() {
            assert!(
                agrees(*expected, spliced_values[index]),
                "{name}: dirty-range execution diverged from a full recompute at bar {index} \
                 (recomputed {} of {} rows)",
                trace.recomputed_rows,
                original.len()
            );
        }
    }
    // Rows outside the affected window must be byte-identical to the original
    // run, which is what makes the range path worth having.
    for (index, (reference, candidate)) in full.output["momentum"]
        .iter()
        .zip(spliced["momentum"].iter())
        .take(20)
        .enumerate()
    {
        assert!(
            agrees(*reference, *candidate),
            "row {index} was churned by an unrelated dirty row"
        );
    }
}

// ───────────────────────── CSE == no CSE ─────────────────────────

/// `((CLOSE + 1) + (CLOSE + 1)) * 2` — the same pure sub-expression declared
/// twice, which is exactly the shape §20's DAG CSE exists to fold.
///
/// Built as a [`SemanticGraph`] rather than a raw [`ComputePlan`] so the test
/// exercises the path a real frontend takes: graph, then optimization, then
/// lowering, then execution.
fn duplicated_subexpression_graph() -> SemanticGraph {
    let mut builder = SemanticGraph::builder();
    let close = builder.push_leaf(NodeKind::Input, "VARIABLE:CLOSE");
    let first = builder.push(
        NodeKind::Formula,
        "CONVERGENCE_PLUS_ONE",
        vec![close],
        stateless(),
    );
    let second = builder.push(
        NodeKind::Formula,
        "CONVERGENCE_PLUS_ONE",
        vec![close],
        stateless(),
    );
    let sum = builder.push(
        NodeKind::Formula,
        "CONVERGENCE_SUM",
        vec![first, second],
        stateless(),
    );
    builder.target(sum);
    builder
        .build()
        .expect("the duplicated-subexpression graph is a DAG")
}

/// Lower a semantic graph and hot-compile it against its single declared target.
fn hot_plan_for(graph: &SemanticGraph) -> HotExecutionPlan {
    let plan = graph.lower().expect("a validated graph lowers");
    let target = graph
        .targets()
        .first()
        .copied()
        .expect("the graph declares a target");
    HotExecutionPlan::compile(&plan, [ComputeNodeId(target.0)])
        .expect("the lowered plan is hot-compilable")
}

/// §20, priority 1: removing a duplicated pure sub-expression must not change
/// a single output value.
///
/// A CSE that is merely *smaller* is worthless if it is also wrong, and the
/// plan differential gates cannot see this: both the folded and the unfolded
/// plan are internally consistent, they just compute different things. Only an
/// execution-level comparison pins it down.
#[test]
fn cse_preserves_every_value_it_folds_away() {
    let close: Vec<f64> = (0..96).map(|index| 20.0 + (index as f64) * 0.25).collect();
    let graph = duplicated_subexpression_graph();

    let mut unfolded = UnifiedExecutor::new(hot_plan_for(&graph), ArithmeticDispatcher);
    let baseline = unfolded.execute(&[&close]).expect("uncse'd execution");

    let outcome = graph.eliminate_common_subexpressions();
    assert_eq!(outcome.report.nodes_before, 4, "CLOSE, +1, +1, SUM");
    assert_eq!(
        outcome.report.nodes_after, 3,
        "the two identical +1 nodes collapse to one"
    );
    assert_eq!(outcome.report.merged, 1);
    assert_eq!(outcome.report.refused_impure, 0);
    assert_eq!(outcome.report.refused_stateful, 0);

    let mut folded = UnifiedExecutor::new(hot_plan_for(&outcome.graph), ArithmeticDispatcher);
    let after = folded.execute(&[&close]).expect("cse'd execution");

    assert_eq!(after.len(), baseline.len());
    assert_eq!(after.values[0].len(), baseline.values[0].len());
    for (index, (source, (uncse, folded_value))) in close
        .iter()
        .zip(baseline.values[0].iter().zip(after.values[0].iter()))
        .enumerate()
    {
        // (C + 1) + (C + 1) = 2C + 2, computed either way.
        let expected = source * 2.0 + 2.0;
        assert!(
            agrees(expected, *uncse),
            "uncse'd execution wrong at bar {index}"
        );
        assert!(
            agrees(*uncse, *folded_value),
            "CSE changed bar {index}: {uncse} -> {folded_value}"
        );
    }

    // The folded plan must genuinely execute fewer kernels — otherwise the pass
    // is "equivalent" but pointless and this gate would pass on a no-op.
    assert!(
        folded.context().metrics().kernel_calls < unfolded.context().metrics().kernel_calls,
        "the folded plan ran {} kernel(s), the unfolded plan ran {} — CSE saved nothing",
        folded.context().metrics().kernel_calls,
        unfolded.context().metrics().kernel_calls
    );
}

/// The §16 graph layer must be reachable from production, not only from tests.
///
/// `check_orphan_modules.py` reported `semantic_graph` as `test-only` before
/// this entry point existed: the graph, the CSE pass and the scheduler all
/// worked, but no production module could ask for them, so §20's first
/// performance priority was reachable from a benchmark and from nobody else.
/// This drives the production route end to end and demands the same guarantees
/// the direct route already gets, so the wiring is covered rather than merely
/// present.
#[test]
fn the_production_graph_entry_point_matches_the_direct_route() {
    let close: Vec<f64> = (0..96).map(|index| 20.0 + (index as f64) * 0.25).collect();
    let graph = duplicated_subexpression_graph();

    let mut as_declared = UnifiedRuntime::compile_semantic_graph(
        &graph,
        ArithmeticDispatcher,
        GraphOptimization::None,
    )
    .expect("the graph compiles as declared");
    assert!(
        as_declared.cse().is_none(),
        "no pass was requested, so there is no report — and `None` must stay distinct from \
         a report that ran and merged nothing"
    );

    let mut optimized = UnifiedRuntime::compile_semantic_graph(
        &graph,
        ArithmeticDispatcher,
        GraphOptimization::CommonSubexpressions,
    )
    .expect("the graph compiles with CSE");
    assert_eq!(
        optimized.cse().expect("CSE was requested").merged,
        1,
        "the duplicated +1 is folded"
    );

    let before = as_declared
        .executor_mut()
        .execute(&[&close])
        .expect("as-declared execution");
    let after = optimized
        .executor_mut()
        .execute(&[&close])
        .expect("optimized execution");

    assert_eq!(before.len(), after.len());
    for (index, (left, right)) in before.values[0]
        .iter()
        .zip(after.values[0].iter())
        .enumerate()
    {
        assert!(agrees(*left, *right), "bar {index}: {left} -> {right}");
    }

    // The pass has to be applied, not merely reported.
    assert!(
        optimized.executor_mut().context().metrics().kernel_calls
            < as_declared.executor_mut().context().metrics().kernel_calls,
        "the production route reported a fold but executed just as many kernels"
    );

    // Folding a duplicate must not change the dependency contract: both graphs
    // are pure fixed-lookback chains, so both stay range-eligible. A `Dynamic`
    // here would cost a caller dirty-range reuse for a graph that can prove it
    // is safe.
    assert!(optimized.can_execute_range());
    assert_eq!(optimized.dependency(), as_declared.dependency());
}

/// CSE and the scheduler must agree with each other, and neither may disturb
/// the graph's deterministic order.
#[test]
fn cse_and_scheduling_keep_a_deterministic_order() {
    let graph = duplicated_subexpression_graph();

    let scheduled = graph.scheduled_order();
    assert_eq!(scheduled.len(), graph.len(), "every node is scheduled once");

    let outcome = graph.eliminate_common_subexpressions();
    let folded = outcome.graph.scheduled_order();
    assert_eq!(folded.len(), outcome.graph.len());
    assert_eq!(
        folded.len(),
        scheduled.len() - 1,
        "the schedule shrinks by exactly the node CSE removed"
    );

    // Re-running both passes changes nothing.
    let again = outcome.graph.eliminate_common_subexpressions();
    assert!(!again.report.changed());
    assert_eq!(again.graph.scheduled_order(), folded);
    assert_eq!(again.graph.content_hash(), outcome.graph.content_hash());
}

/// §20, priority 2: repeated execution must reuse buffers, not re-allocate.
///
/// The plan's second performance priority is "avoid duplicate allocation",
/// delivered by the `_into` kernel family, `BufferArena`, and persistent
/// output. `into_kernel_equals_allocating_kernel` covers the first of those.
/// The arena is the one that decides whether a scan allocates per symbol or
/// not, and the arena's own counters measure it exactly and deterministically —
/// unlike a process-wide allocation count, which `cargo test` running tests in
/// parallel threads would make flaky.
///
/// The two halves below pin both API shapes. Writing this test is what showed
/// that plain `execute` re-allocates one buffer per execution: it moves the
/// result buffers out of the arena for the caller to own, leaving zero-length
/// `Vec`s behind, and `BufferArena::recycle` deliberately refuses those. That
/// is a real per-symbol allocation in a scan loop, so `execute_into` was added
/// to close it — and this test keeps it closed.
#[test]
fn repeated_execution_reuses_the_arena_instead_of_allocating() {
    let close: Vec<f64> = (0..128).map(|index| 100.0 + index as f64).collect();
    let mut executor = UnifiedExecutor::new(diamond_plan(), ArithmeticDispatcher);

    // Shape 1: the owning API. Exactly the retained outputs are re-allocated,
    // because the caller takes ownership of them.
    let _ = executor.execute(&[&close]).expect("first execution");
    let warm = executor.buffer_stats();
    assert!(
        warm.cache_misses > 0,
        "the first execution must allocate the plan's working set"
    );

    for _ in 0..32 {
        let _ = executor.execute(&[&close]).expect("repeated execution");
    }
    let owned = executor.buffer_stats();
    assert_eq!(
        owned.cache_misses - warm.cache_misses,
        32,
        "expected exactly one re-allocation per execution (the retained output \
         that ownership moved out of the arena); more than that means the \
         intermediate buffers are not coming back"
    );
    assert!(
        owned.cache_hits > warm.cache_hits,
        "32 further executions produced no arena cache hits, so the counters \
         are not measuring what this test thinks they measure"
    );

    // Shape 2: the persistent-output API. Nothing is left to re-allocate.
    let mut destination = vec![0.0; close.len()];
    let mut destinations: [&mut [f64]; 1] = [destination.as_mut_slice()];
    executor
        .execute_into(&[&close], &mut destinations)
        .expect("first persistent-output execution");
    let into_warm = executor.buffer_stats();

    for _ in 0..32 {
        let mut destinations: [&mut [f64]; 1] = [destination.as_mut_slice()];
        executor
            .execute_into(&[&close], &mut destinations)
            .expect("repeated persistent-output execution");
    }
    let into_steady = executor.buffer_stats();
    assert_eq!(
        into_steady.cache_misses,
        into_warm.cache_misses,
        "32 further persistent-output executions allocated {} more buffer(s); \
         with a caller-owned destination the whole working set should be \
         recyclable after warm-up",
        into_steady.cache_misses - into_warm.cache_misses
    );
    assert!(
        into_steady.cache_hits > into_warm.cache_hits,
        "the persistent-output path returned nothing to the arena"
    );

    // Copying out must not change the answer: this is the diamond's `3x + 1`.
    for (index, value) in destination.iter().enumerate() {
        assert!(
            agrees(close[index] * 3.0 + 1.0, *value),
            "persistent-output execution wrong at bar {index}: {value}"
        );
    }

    assert_eq!(
        executor.context().metrics().buffers_taken,
        executor.context().metrics().buffers_recycled,
        "every buffer taken from the arena must be handed back"
    );
}

#[test]
fn execute_into_rejects_mismatched_destinations() {
    let close: Vec<f64> = (0..64).map(|index| 100.0 + index as f64).collect();
    let mut executor = UnifiedExecutor::new(diamond_plan(), ArithmeticDispatcher);

    // Wrong number of destinations: the plan retains exactly one output.
    let mut none: [&mut [f64]; 0] = [];
    assert!(matches!(
        executor
            .execute_into(&[&close], &mut none)
            .expect_err("zero destinations cannot accept one retained output"),
        ExecuteError::OutputCount {
            expected: 1,
            actual: 0
        }
    ));

    // Right count, wrong length.
    let mut short = vec![0.0; close.len() - 1];
    let mut destinations: [&mut [f64]; 1] = [short.as_mut_slice()];
    assert!(matches!(
        executor
            .execute_into(&[&close], &mut destinations)
            .expect_err("a short destination must be rejected"),
        ExecuteError::OutputLength { index: 0, .. }
    ));

    // The rejection must not have written anything.
    assert!(
        short.iter().all(|value| *value == 0.0),
        "a rejected destination was partially written"
    );

    // And a correct call still works afterwards, so the failure left no state
    // behind.
    let mut destination = vec![0.0; close.len()];
    let mut destinations: [&mut [f64]; 1] = [destination.as_mut_slice()];
    executor
        .execute_into(&[&close], &mut destinations)
        .expect("a valid call after a rejected one still succeeds");
    assert!(agrees(close[0] * 3.0 + 1.0, destination[0]));
}

// ───────────────────────── convergence roster ─────────────────────────

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("core/ always has a parent")
        .to_path_buf()
}

/// Every §18 edge and the artifact that provides it.
///
/// The point is not to re-run the other gates but to make their removal a test
/// failure. A deleted gate is invisible otherwise: the build stays green and the
/// coverage claim quietly becomes false.
///
/// The third element is a **symbol**, not a caption. Validating that the file
/// merely exists would leave the roster half-rotting: renaming or deleting
/// `streaming_agrees_with_batch` keeps the file present, so the edge would go
/// ungated while this test still certified it. Checking the symbol is what
/// makes the roster bidirectional in the §18 sense — an entry that stops being
/// needed, and an entry that stops being real, both fail here.
const CONVERGENCE_ROSTER: &[(&str, &str, &str)] = &[
    (
        "batch == streaming",
        "core/tests/runtime_convergence.rs",
        "streaming_agrees_with_batch",
    ),
    (
        "full == range",
        "core/tests/runtime_convergence.rs",
        "range_execution_equals_full_for_stateless_plans",
    ),
    (
        "allocating == into",
        "core/tests/runtime_convergence.rs",
        "into_kernel_equals_allocating_kernel",
    ),
    (
        "tree == plan",
        "core/tests/formula_plan_differential.rs",
        "domestic_corpus_plan_matches_ast_reference",
    ),
    (
        "Rust == FFI",
        "ffi/ffi-common/tests/talib_numeric_contract.rs",
        "talib_numeric_contract_201_vectors_execute_natively",
    ),
];

#[test]
fn every_convergence_edge_has_an_artifact() {
    let root = repo_root();
    for (edge, relative, symbol) in CONVERGENCE_ROSTER {
        let path = root.join(relative);
        assert!(
            path.is_file(),
            "{edge}: convergence artifact {relative} is missing — the edge ({symbol}) is \
             ungated, so the §18 claim no longer holds"
        );
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        assert!(
            text.len() > 512,
            "{edge}: {relative} exists but is empty or truncated"
        );
        assert!(
            text.contains(&format!("fn {symbol}(")),
            "{edge}: {relative} no longer defines `fn {symbol}(...)`, so the edge is not \
             actually gated by the symbol this roster certifies"
        );
    }
    assert_eq!(
        CONVERGENCE_ROSTER.len(),
        5,
        "all five §18 edges must be rostered"
    );
}

// ---- §20, priority 4: derive once, reuse across the universe ----------------

/// §20 ranks "stop re-deriving what has already been derived" above every
/// micro-optimization, and a factor scan is the case it was written for: one
/// declaration compiled, then run for every symbol. `RuntimeContext` owns the
/// cache for that, and until now nothing wrote to it — an `ArtifactCache` that
/// is never consulted is indistinguishable from one that works, so this asserts
/// the hit counter rather than merely that both calls succeeded.
#[test]
fn recompiling_the_same_declaration_is_served_from_the_artifact_cache() {
    let graph = duplicated_subexpression_graph();
    let mut runtime = RuntimeContext::new();
    assert!(
        runtime.cache().stats().capacity > 0,
        "the default runtime context must enable the artifact cache, or this gate proves nothing"
    );

    let first = UnifiedRuntime::compile_semantic_graph_cached::<ArithmeticDispatcher>(
        &graph,
        ArithmeticDispatcher,
        GraphOptimization::CommonSubexpressions,
        &mut runtime,
    )
    .expect("the duplicated-subexpression graph compiles");
    assert_eq!(
        runtime.cache().stats().hits,
        0,
        "the first compile of a declaration cannot be a cache hit"
    );

    let second = UnifiedRuntime::compile_semantic_graph_cached::<ArithmeticDispatcher>(
        &graph,
        ArithmeticDispatcher,
        GraphOptimization::CommonSubexpressions,
        &mut runtime,
    )
    .expect("the same declaration compiles again");

    let stats = runtime.cache().stats();
    assert_eq!(
        (stats.hits, stats.misses, stats.entries),
        (1, 1, 1),
        "the second compile must be a hit on the single entry the first one wrote"
    );

    // A hit must describe the same computation as the build it replaced.
    assert_eq!(
        first.cse().map(|report| report.merged),
        second.cse().map(|report| report.merged),
        "a cache hit must not change what CSE reported"
    );
    assert_eq!(first.dependency(), second.dependency());
    assert_eq!(first.can_execute_range(), second.can_execute_range());
}

/// The key has to separate the two settings that produce different plans from
/// one declaration. Sharing a namespace would let an as-declared request be
/// answered by an optimized compile, which is a silently wrong plan.
#[test]
fn the_plan_cache_separates_optimization_settings() {
    let graph = duplicated_subexpression_graph();
    let mut runtime = RuntimeContext::new();

    let settings = [
        GraphOptimization::None,
        GraphOptimization::CommonSubexpressions,
    ];
    for optimization in settings {
        UnifiedRuntime::compile_semantic_graph_cached::<ArithmeticDispatcher>(
            &graph,
            ArithmeticDispatcher,
            optimization,
            &mut runtime,
        )
        .unwrap_or_else(|error| panic!("{} must compile: {error}", optimization.label()));
    }
    assert_eq!(
        runtime.cache().stats().entries,
        2,
        "as-declared and CSE plans occupy different cache namespaces"
    );
    assert_eq!(
        runtime.cache().stats().hits,
        0,
        "neither setting may be served by the other's entry"
    );

    // Repeating the first setting is a hit, not a third entry.
    UnifiedRuntime::compile_semantic_graph_cached::<ArithmeticDispatcher>(
        &graph,
        ArithmeticDispatcher,
        GraphOptimization::None,
        &mut runtime,
    )
    .expect("the as-declared plan compiles again");
    assert_eq!(runtime.cache().stats().hits, 1);
    assert_eq!(runtime.cache().stats().entries, 2);
}

/// Caching a *failed* build would poison the entry: the corrected graph would
/// then be served the error's absence. The failure path must leave no trace.
#[test]
fn a_failed_compile_leaves_the_cache_untouched() {
    let mut builder = SemanticGraph::builder();
    let close = builder.push_leaf(NodeKind::Input, "VARIABLE:CLOSE");
    // Built and never claimed as an output: a frontend bug, not an empty graph.
    let _ = builder.push(
        NodeKind::Formula,
        "CONVERGENCE_PLUS_ONE",
        vec![close],
        stateless(),
    );
    let graph = builder
        .build()
        .expect("a target-less graph is still a valid DAG");

    let mut runtime = RuntimeContext::new();
    let error = UnifiedRuntime::compile_semantic_graph_cached::<ArithmeticDispatcher>(
        &graph,
        ArithmeticDispatcher,
        GraphOptimization::None,
        &mut runtime,
    )
    .expect_err("a graph with no target cannot compile");
    assert_eq!(error, finkit::unified_runtime::GraphPlanError::NoTarget);
    assert_eq!(
        runtime.cache().stats().entries,
        0,
        "a failed build must not be cached, or a corrected graph would resolve to it"
    );
}

/// A rejected `execute_into` must not have run the plan.
///
/// This is not a stylistic preference. `execute` drives persistent kernel state,
/// so a length check placed *after* the run would leave a rejected call one step
/// ahead of its caller: a retry with a corrected destination would compute the
/// second sample's value and hand it back as the first. `RuntimeMetrics` makes
/// the difference observable — its counters are only advanced on the success
/// path of `run`.
#[test]
fn a_rejected_execute_into_leaves_the_executor_untouched() {
    let graph = duplicated_subexpression_graph();
    let close: Vec<f64> = (0..64).map(|index| 20.0 + index as f64).collect();
    let inputs: [&[f64]; 1] = [&close];

    let mut executor = UnifiedExecutor::new(hot_plan_for(&graph), ArithmeticDispatcher);
    assert_eq!(
        executor.plan().output_layout().len(),
        1,
        "the fixture declares a single target"
    );

    let mut short = vec![0.0; close.len() - 1];
    let error = executor
        .execute_into(&inputs, &mut [&mut short[..]])
        .expect_err("a destination of the wrong length must be rejected");
    assert!(
        matches!(error, ExecuteError::OutputLength { .. }),
        "expected OutputLength, got {error:?}"
    );

    let metrics = executor.context().metrics();
    assert_eq!(metrics.executions(), 0, "the plan must not have run at all");
    assert_eq!(metrics.kernel_calls, 0, "no kernel may have been invoked");

    // The corrected call is therefore the *first* execution, not the second.
    let mut destination = vec![0.0; close.len()];
    executor
        .execute_into(&inputs, &mut [&mut destination[..]])
        .expect("a correctly sized destination is accepted");
    for (index, value) in close.iter().enumerate() {
        let expected = value * 2.0 + 2.0;
        assert!(
            agrees(destination[index], expected),
            "(C + 1) + (C + 1) = 2C + 2, but bar {index} is {}",
            destination[index]
        );
    }
    assert_eq!(executor.context().metrics().executions(), 1);

    // An output-count mismatch is rejected just as early.
    let mut extra = vec![0.0; close.len()];
    let error = executor
        .execute_into(&inputs, &mut [&mut destination[..], &mut extra[..]])
        .expect_err("too many destinations must be rejected");
    assert!(
        matches!(error, ExecuteError::OutputCount { .. }),
        "expected OutputCount, got {error:?}"
    );
    assert_eq!(
        executor.context().metrics().executions(),
        1,
        "a rejected call must not have executed anything"
    );
}
