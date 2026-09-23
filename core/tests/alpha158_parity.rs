//! Alpha158 parity gate: finkit's factor library against Qlib's own output.
//!
//! The contract is `tests/golden/alpha158/reference_v1.json`, produced by
//! `scripts/gen_alpha158_reference.py`, which evaluates Qlib's 158 expressions
//! on a deterministic market using a transcription of Qlib's own operators
//! running on pandas — the substrate `qlib/data/ops.py` delegates to.
//!
//! # What is asserted, and why it is more than one claim
//!
//! "The two agree" is not a single statement here, because finkit and Qlib make
//! three different choices about rolling windows and degenerate arithmetic:
//!
//! * Qlib builds every window as `series.rolling(N, min_periods=1)`, so its
//!   first `N - 1` bars carry a *partial-window* value. finkit reports `NaN`
//!   until the window is full.
//! * Qlib NaNs any window whose rolling standard deviation is within
//!   `atol=2e-05` of zero. finkit NaNs a window whose variance is numerically
//!   absent, which is scale-free where Qlib's threshold is not.
//! * finkit's `/` operator NaNs a divisor below `1e-15` in magnitude. That is a
//!   deliberate engine-wide guard, and it interacts with Qlib's `+1e-12`
//!   idiom — see `the_division_guard_explains_the_wvma_family`.
//!
//! Collapsing those into "agreement to 1e-8" would require one side to pretend,
//! so the gate asserts four separately falsifiable things instead:
//!
//! 1. **Arithmetic.** On the common support — every bar where both sides have a
//!    value — they agree to [`TOLERANCE`]. This is the headline claim, and the
//!    M0-3 acceptance criterion.
//! 2. **No unexplained gaps.** finkit never withholds a value that Qlib has
//!    beyond its own warm-up, *except* inside the flat run, where the division
//!    guard is the cause — and that cause is pinned causally by
//!    [`the_division_guard_explains_the_wvma_family`], not assumed.
//! 3. **The probe is where the policies can differ.** Every bar where finkit has
//!    a value and Qlib does not must lie in the window-expanded span of the
//!    deliberately flat run. Outside it, the two masks agree exactly.
//! 4. **The probe is not dead code.** The reference must have `NaN` bars inside
//!    the probe for the guarded operators, or claims 1–3 could pass because the
//!    guard branch never fired on either side.
//!
//! Claims 2 and 3 are *structural*: they are stated against a region recorded in
//! the contract, not against pinned counts, so they name the offending bar when
//! they fail instead of just changing a number.

use finkit::factors::builtin::{factor_library, CompiledFactor};
use finkit::factors::FactorDirection;
use finkit::formula::FormulaContext;
use ndarray::Array1;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Absolute tolerance for the numeric comparison, from the M0-3 acceptance
/// criterion.
const TOLERANCE: f64 = 1e-8;

/// The operators whose degenerate-window guard the probe is meant to exercise.
const GUARDED_OPERATORS: &[&str] = &["RSQR", "CORR", "CORD"];

#[derive(Deserialize)]
struct Probe {
    start: usize,
    end: usize,
    widest_window: usize,
}

#[derive(Deserialize)]
struct Contract {
    bars: usize,
    probe: Probe,
    market: BTreeMap<String, Vec<f64>>,
    factors: BTreeMap<String, Vec<Option<f64>>>,
}

fn contract() -> Contract {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("golden")
        .join("alpha158")
        .join("reference_v1.json");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    serde_json::from_str(&raw).expect("parse the Alpha158 reference contract")
}

fn context(market: &BTreeMap<String, Vec<f64>>) -> FormulaContext {
    let array = |name: &str| Array1::from_vec(market[name].clone());
    let mut ctx = FormulaContext::new(
        array("open"),
        array("high"),
        array("low"),
        array("close"),
        array("volume"),
        None,
    );
    // `$vwap` is a data-layer field in Qlib, not an operator, so the market
    // supplies it directly.
    ctx.set_variable("VWAP".to_string(), array("vwap"));
    ctx
}

/// Half-open span of bars whose window can reach into the flat run.
///
/// A window of length `W` at bar `i` covers `[i - W + 1, i]`, so it touches the
/// probe `[start, end)` exactly when `i >= start` and `i - W + 1 < end`. No bar
/// before `start` can reach forward into the probe.
fn probe_span(probe: &Probe) -> std::ops::RangeInclusive<usize> {
    probe.start..=(probe.end - 1 + probe.widest_window - 1)
}

#[test]
fn every_factor_builds_and_evaluates() {
    let contract = contract();
    let library = factor_library("alpha158").expect("the alpha158 library builds");
    let ctx = context(&contract.market);

    assert_eq!(
        library.len(),
        contract.factors.len(),
        "the library and the reference must describe the same number of factors"
    );

    // The names must match as sets, in both directions: a factor the reference
    // has and the library does not is an omission, and the reverse is an
    // invention that no gate would otherwise notice.
    let library_names: std::collections::BTreeSet<&str> = library.names().collect();
    let reference_names: std::collections::BTreeSet<&str> =
        contract.factors.keys().map(String::as_str).collect();
    assert_eq!(library_names, reference_names, "factor name sets differ");

    for name in &reference_names {
        let values = library
            .evaluate(name, &ctx)
            .unwrap_or_else(|err| panic!("evaluate {name}: {err}"));
        assert_eq!(
            values.len(),
            contract.bars,
            "{name} produced {} bars, want {}",
            values.len(),
            contract.bars
        );
        assert!(
            values.iter().any(|value| value.is_finite()),
            "{name} is all-NaN on the reference market"
        );
    }
}

#[test]
fn agrees_with_the_qlib_reference_to_1e_8() {
    let contract = contract();
    let library = factor_library("alpha158").expect("the alpha158 library builds");
    let ctx = context(&contract.market);

    let mut worst = 0.0_f64;
    let mut worst_name = String::new();
    let mut compared_total = 0usize;
    let mut violations: Vec<String> = Vec::new();

    for (name, reference) in &contract.factors {
        let actual = library
            .evaluate(name, &ctx)
            .unwrap_or_else(|err| panic!("evaluate {name}: {err}"));
        assert_eq!(actual.len(), reference.len(), "{name}: length mismatch");

        for (index, expected) in reference.iter().enumerate() {
            // Bars where finkit is legitimately in warm-up are the subject of
            // `finkit_withholds_no_steady_state_value`, not of this test: the
            // common support is exactly "both sides have a value".
            let Some(expected) = expected else {
                continue;
            };
            let got = actual[index];
            if !got.is_finite() {
                continue;
            }
            compared_total += 1;
            let difference = (got - expected).abs();
            if difference > worst {
                worst = difference;
                worst_name.clone_from(name);
            }
            if difference > TOLERANCE && violations.len() < 20 {
                violations.push(format!(
                    "{name} bar {index}: finkit {got} vs Qlib {expected} \
                     (|diff| = {difference:.3e})"
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "{} factors disagree with the Qlib reference beyond {TOLERANCE:.0e}:\n{}",
        violations.len(),
        violations.join("\n")
    );
    assert!(
        compared_total > 30_000,
        "only {compared_total} bars were compared, which means the reference is \
         mostly NaN and this gate is close to vacuous"
    );
    println!(
        "{compared_total} bars compared across {} factors; worst |diff| = \
         {worst:.3e} ({worst_name})",
        contract.factors.len()
    );
}

#[test]
fn finkit_withholds_no_steady_state_value() {
    let contract = contract();
    let library = factor_library("alpha158").expect("the alpha158 library builds");
    let ctx = context(&contract.market);
    let widest = contract.probe.widest_window;
    let probe = contract.probe.start..contract.probe.end;

    let mut violations: Vec<String> = Vec::new();
    let mut withheld = 0usize;
    for (name, reference) in &contract.factors {
        let actual = library
            .evaluate(name, &ctx)
            .unwrap_or_else(|err| panic!("evaluate {name}: {err}"));
        for (index, expected) in reference.iter().enumerate() {
            if expected.is_none() || actual[index].is_finite() || index < widest {
                continue;
            }
            withheld += 1;
            // Past the warm-up, the only reason finkit may withhold a value is
            // the `/` operator's epsilon guard, and that can only fire on a
            // window lying entirely inside the flat run. Anything else is an
            // unexplained gap. `the_division_guard_explains_the_wvma_family`
            // pins the causal mechanism; this pins the location for all 158.
            if !probe.contains(&index) && violations.len() < 20 {
                violations.push(format!(
                    "{name} bar {index}: finkit NaN, Qlib {}",
                    expected.unwrap_or(f64::NAN)
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "finkit withholds a value past the widest warm-up ({widest}) outside the \
         flat run, where neither the partial-window convention nor the division \
         guard can explain it:\n{}",
        violations.join("\n")
    );
    println!("{withheld} steady-state values are withheld, all inside the probe {probe:?}");
}

/// The mechanism behind the withheld values, tested directly rather than inferred.
///
/// `WVMA{d}` is `Std(X, d) / (Mean(X, d) + 1e-12)` where
/// `X = |close / Ref(close, 1) - 1| * volume`. On a window lying entirely inside
/// the flat run, `X` is exactly zero, so the true denominator is `1e-12` — Qlib
/// divides by it and gets `0`.
///
/// finkit's rolling mean accumulates and removes the large pre-probe values, so
/// on an all-zero window it returns a cancellation residue of about
/// `-1.0e-12` rather than exactly `0`. The residue is harmless in itself
/// (`1e-17` relative at this data's scale), but it pushes the denominator to
/// `-4.4e-16`, which is below the `/` operator's `|rhs| < 1e-15` epsilon guard,
/// and the guard returns `NaN`.
///
/// This test asserts that mechanism rather than assuming it: the bars where the
/// denominator drops below the guard must be exactly the bars where `WVMA` is
/// `NaN`. If the mean's residue were fixed, or the guard changed, this fails and
/// says which half moved.
///
/// The coincidence is a knife edge, which is worth knowing: the denominator is
/// `residue + 1e-12`, so it only drops below the `1e-15` guard when the residue
/// is slightly *more negative* than `-1e-12`. On this market that happens for
/// `WVMA30` and for no other window, so the test requires the mechanism to fire
/// somewhere in the family rather than at every window.
#[test]
fn the_division_guard_explains_the_wvma_family() {
    let contract = contract();
    let library = factor_library("alpha158").expect("the alpha158 library builds");
    let ctx = context(&contract.market);

    let mut total_guard_bars = 0usize;
    let mut total_withheld_bars = 0usize;

    for window in [5usize, 10, 20, 30, 60] {
        let expression = format!("MA(ABS(CLOSE/REF(CLOSE,1)-1)*VOLUME, {window})+1e-12");
        let denominator =
            CompiledFactor::compile("denominator", expression.clone(), FactorDirection::Neutral)
                .unwrap_or_else(|err| panic!("compile {expression}: {err}"))
                .evaluate(&ctx)
                .unwrap_or_else(|err| panic!("evaluate {expression}: {err}"));

        let name = format!("WVMA{window}");
        let actual = library
            .evaluate(&name, &ctx)
            .unwrap_or_else(|err| panic!("evaluate {name}: {err}"));

        let mut guard_bars = 0usize;
        let mut withheld_bars = 0usize;
        for index in 0..contract.bars {
            let below_guard = denominator[index].is_finite() && denominator[index].abs() < 1e-15;
            if below_guard {
                guard_bars += 1;
                assert!(
                    !actual[index].is_finite(),
                    "{name} bar {index}: the denominator {:.6e} is below the \
                     division guard, so finkit should withhold a value, but it \
                     returned {}",
                    denominator[index],
                    actual[index]
                );
            }
            // Warm-up is the only other source of NaN, so the coincidence is
            // asserted on the steady-state bars only.
            if !actual[index].is_finite() && index >= window {
                withheld_bars += 1;
                assert!(
                    below_guard,
                    "{name} bar {index}: finkit is NaN but the denominator \
                     {:.6e} is above the division guard and the bar is past the \
                     warm-up",
                    denominator[index]
                );
            }
        }

        assert_eq!(
            guard_bars, withheld_bars,
            "{name}: {guard_bars} bars are below the division guard but \
             {withheld_bars} bars are withheld; the two sets must coincide"
        );
        total_guard_bars += guard_bars;
        total_withheld_bars += withheld_bars;
    }

    assert!(
        total_guard_bars > 0,
        "the denominator never drops below the division guard for any window, so \
         this test is not exercising the mechanism it describes"
    );
    println!(
        "{total_guard_bars} of {total_withheld_bars} withheld values are explained \
         by the division guard"
    );
}

#[test]
fn extra_finite_bars_are_confined_to_the_probe() {
    let contract = contract();
    let library = factor_library("alpha158").expect("the alpha158 library builds");
    let ctx = context(&contract.market);
    let span = probe_span(&contract.probe);

    let mut extra_total = 0usize;
    for (name, reference) in &contract.factors {
        let actual = library
            .evaluate(name, &ctx)
            .unwrap_or_else(|err| panic!("evaluate {name}: {err}"));
        for (index, expected) in reference.iter().enumerate() {
            if expected.is_none() && actual[index].is_finite() {
                extra_total += 1;
                assert!(
                    span.contains(&index),
                    "{name} bar {index}: finkit returns {} where Qlib returns NaN, \
                     but bar {index} is outside the probe's window span \
                     ({span:?}). Outside a window that touches the flat run, the \
                     two degeneracy policies cannot differ.",
                    actual[index]
                );
            }
        }
    }
    println!(
        "{extra_total} bars are finite in finkit and NaN in Qlib, all inside the \
         probe span {span:?}"
    );
}

#[test]
fn the_probe_actually_exercises_the_degenerate_guard() {
    let contract = contract();
    let probe = &contract.probe;

    for operator in GUARDED_OPERATORS {
        for window in [5usize, 10, 20, 30, 60] {
            let name = format!("{operator}{window}");
            let series = contract
                .factors
                .get(&name)
                .unwrap_or_else(|| panic!("{name} is in the reference"));
            let inside = probe.start..probe.end;
            let guarded = series[inside.clone()]
                .iter()
                .filter(|value| value.is_none())
                .count();
            assert!(
                guarded > 0,
                "{name} has no NaN bars inside the probe {inside:?}, so Qlib's \
                 degenerate-std guard never fired and the probe is not testing \
                 what it was built to test"
            );
        }
    }
}

/// The sample-standard-deviation convention, checked against the population one.
///
/// This is the measurement that justified adding `STDDEV_SAMPLE` instead of
/// reusing `STD`: the two differ by exactly `sqrt((n - 1) / n)`, which is a 10.6%
/// scale error at `n = 5` — far too large for a tolerance to absorb. Pinning the
/// identity here means a future change to either kernel has to keep it.
#[test]
#[allow(clippy::cast_precision_loss)] // windows are 5..=60: exact in f64.
fn the_sample_std_is_the_population_std_scaled() {
    let contract = contract();
    let ctx = context(&contract.market);

    for window in [5usize, 10, 20, 30, 60] {
        let evaluate = |expression: &str| {
            let factor =
                CompiledFactor::compile("probe", expression.to_string(), FactorDirection::Neutral)
                    .unwrap_or_else(|err| panic!("compile {expression}: {err}"));
            factor
                .evaluate(&ctx)
                .unwrap_or_else(|err| panic!("evaluate {expression}: {err}"))
        };
        let sample = evaluate(&format!("STDDEV_SAMPLE(CLOSE, {window})"));
        let population = evaluate(&format!("STD(CLOSE, {window})"));

        let correction = (window as f64 / (window - 1) as f64).sqrt();
        let mut checked = 0usize;
        for index in 0..contract.bars {
            if !sample[index].is_finite() || !population[index].is_finite() {
                continue;
            }
            checked += 1;
            let expected = population[index] * correction;
            assert!(
                (sample[index] - expected).abs() <= 1e-12 * expected.abs().max(1.0),
                "window {window} bar {index}: STDDEV_SAMPLE {} vs \
                 STD * sqrt(n/(n-1)) = {}",
                sample[index],
                expected
            );
        }
        assert!(
            checked > contract.bars / 2,
            "window {window}: only {checked} bars were comparable"
        );
    }
}
