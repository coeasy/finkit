//! `WorldQuant101` gate: the classification must be complete, and honest.
//!
//! There is no numeric reference for these alphas — the only available
//! implementation is cross-sectional, so it cannot produce a single-instrument
//! value to compare against. What this gate checks instead is that the
//! *classification* is sound, which is the part that can be checked:
//!
//! 1. Every formula the source defines is either computed or annotated, and the
//!    two sets partition the corpus with no overlap and nothing dropped.
//! 2. Every annotated alpha names a blocker that **actually appears in that
//!    alpha's formula**, cross-checked against the committed
//!    `tests/golden/worldquant101/formulas_v1.json` rather than against the
//!    module's own tables. An annotation that is not backed by the formula text
//!    fails here, which is what stops "unavailable" from becoming a place to put
//!    things that were merely inconvenient.
//! 3. Every computed alpha compiles into a plan and evaluates to at least one
//!    finite value on the shared market.
//!
//! The reserved-number check is what makes the corpus size auditable: the paper
//! defines 71 formulas and reserves 30 numbers, so `71 + 30 == 101` is asserted
//! rather than assumed.

use finkit::factors::builtin::{factor_library, worldquant101};
use finkit::formula::FormulaContext;
use ndarray::Array1;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Bars in the shared market. Enough for the longest window any alpha uses
/// (`Alpha24` reaches back 200 bars through `REF(SUM(CLOSE,100)/100, 100)`).
const BARS: usize = 260;

/// Operators that need a cross-section of instruments on one date.
const CROSS_SECTIONAL: &[&str] = &["rank", "scale", "indneutralize"];

#[derive(Deserialize)]
struct FormulaContract {
    count: usize,
    formulas: BTreeMap<String, String>,
    reference: Reference,
}

#[derive(Deserialize)]
struct Reference {
    reserved_numbers: Vec<usize>,
}

fn formulas() -> FormulaContract {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tests")
        .join("golden")
        .join("worldquant101")
        .join("formulas_v1.json");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    serde_json::from_str(&raw).expect("parse the WorldQuant101 formula contract")
}

/// Function names called in a source formula.
fn operators(expression: &str) -> BTreeSet<&str> {
    let bytes = expression.as_bytes();
    let mut found = BTreeSet::new();
    let mut start = 0usize;
    while start < bytes.len() {
        if bytes[start].is_ascii_alphabetic() || bytes[start] == b'_' {
            let mut end = start;
            while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                end += 1;
            }
            // Only a call, not a bare identifier.
            let mut cursor = end;
            while cursor < bytes.len() && bytes[cursor] == b' ' {
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor] == b'(' {
                found.insert(&expression[start..end]);
            }
            start = end;
        } else {
            start += 1;
        }
    }
    found
}

/// Index-to-`f64` for the synthetic series.
///
/// `cast_precision_loss` is expected and harmless: `BARS` is a few hundred, so
/// every index is exactly representable. The project's convention for this is a
/// local `allow` carrying the reason rather than a bare cast.
#[allow(clippy::cast_precision_loss)]
fn as_f64(index: usize) -> f64 {
    index as f64
}

/// A deterministic market, matching the shape the Alpha158 gate uses.
fn market() -> FormulaContext {
    let mut state = 0x2545_F491_u64;
    let mut next_unit = move || {
        state = (state.wrapping_mul(1_103_515_245).wrapping_add(12_345)) & 0x7FFF_FFFF;
        // `state` is masked to 31 bits above, so it is exactly representable in
        // `f64` and the division is exact to the last bit.
        #[allow(clippy::cast_precision_loss)]
        let numerator = state as f64;
        numerator / 2_147_483_647.0
    };

    let (mut opens, mut highs, mut lows, mut closes, mut volumes) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut close = 100.0_f64;
    for bar in 0..BARS {
        let step = (next_unit() - 0.48) * 2.4;
        let open = close * (1.0 + (next_unit() - 0.5) * 0.004);
        close += step;
        let high = open.max(close) * (1.0 + next_unit() * 0.006);
        let low = open.min(close) * (1.0 - next_unit() * 0.006);
        let volume = 1.0e6 * (1.0 + 0.35 * (as_f64(bar) * 0.21).sin()) * (0.8 + 0.4 * next_unit());
        opens.push(open);
        closes.push(close);
        highs.push(high);
        lows.push(low);
        volumes.push(volume);
    }

    let typical: Vec<f64> = (0..BARS)
        .map(|index| (highs[index] + lows[index] + closes[index]) / 3.0)
        .collect();
    let (mut cumulative, mut cumulative_volume) = (0.0_f64, 0.0_f64);
    let vwap: Vec<f64> = (0..BARS)
        .map(|index| {
            cumulative += typical[index] * volumes[index];
            cumulative_volume += volumes[index];
            cumulative / cumulative_volume
        })
        .collect();

    let array = Array1::from_vec;
    let mut ctx = FormulaContext::new(
        array(opens),
        array(highs),
        array(lows),
        array(closes),
        array(volumes),
        None,
    );
    ctx.set_variable("VWAP".to_string(), array(vwap));
    ctx
}

#[test]
fn the_corpus_is_71_formulas_and_30_reserved_numbers() {
    let contract = formulas();
    assert_eq!(
        contract.formulas.len(),
        contract.count,
        "the committed formula count disagrees with the table"
    );
    assert_eq!(contract.count, worldquant101::FORMULA_COUNT);
    assert_eq!(
        contract.reference.reserved_numbers.len(),
        worldquant101::RESERVED_NUMBERS.len(),
        "the reserved-number lists disagree"
    );
    assert_eq!(
        contract.count + contract.reference.reserved_numbers.len(),
        101,
        "the paper numbers 1..101, so the defined and reserved sets must add to 101"
    );

    // Every number 1..=101 is either defined or reserved, exactly once.
    let defined: BTreeSet<usize> = contract
        .formulas
        .keys()
        .map(|name| {
            name.trim_start_matches("Alpha")
                .parse::<usize>()
                .unwrap_or_else(|_| panic!("unparseable alpha name {name}"))
        })
        .collect();
    let reserved: BTreeSet<usize> = contract
        .reference
        .reserved_numbers
        .iter()
        .copied()
        .collect();
    assert!(
        defined.is_disjoint(&reserved),
        "a number is both defined and reserved"
    );
    assert_eq!(
        defined.union(&reserved).count(),
        101,
        "some number in 1..=101 is neither defined nor reserved"
    );
}

#[test]
fn every_formula_is_computed_or_annotated() {
    let contract = formulas();
    let computed: BTreeSet<&str> = worldquant101::EXPRESSIONS.iter().map(|(n, _)| *n).collect();
    let annotated: BTreeSet<&str> = worldquant101::UNAVAILABLE.iter().map(|(n, _)| *n).collect();

    let missing: Vec<&str> = contract
        .formulas
        .keys()
        .map(String::as_str)
        .filter(|name| !computed.contains(name) && !annotated.contains(name))
        .collect();
    assert!(
        missing.is_empty(),
        "these formulas are in neither table, so they were silently dropped: {missing:?}"
    );
    let invented: Vec<&str> = computed
        .union(&annotated)
        .copied()
        .filter(|name| !contract.formulas.contains_key(*name))
        .collect();
    assert!(
        invented.is_empty(),
        "these names are in a table but not in the corpus: {invented:?}"
    );
    assert!(
        computed.is_disjoint(&annotated),
        "an alpha cannot be both computed and annotated"
    );
    assert_eq!(computed.len(), worldquant101::COMPUTABLE_COUNT);
}

/// The anti-invention check: an annotation has to be earned by the formula text.
#[test]
fn every_annotated_blocker_appears_in_its_formula() {
    let contract = formulas();
    for (name, blockers) in worldquant101::UNAVAILABLE {
        let formula = contract
            .formulas
            .get(*name)
            .unwrap_or_else(|| panic!("{name} is annotated but not in the corpus"));
        let present = operators(formula);
        assert!(
            !blockers.is_empty(),
            "{name} is annotated as unavailable but names no blocker"
        );
        for blocker in *blockers {
            assert!(
                CROSS_SECTIONAL.contains(blocker),
                "{name} is annotated as blocked by {blocker:?}, which is not a \
                 known cross-sectional operator"
            );
            assert!(
                present.contains(blocker),
                "{name} is annotated as blocked by {blocker:?}, but its formula \
                 never calls it: {formula}"
            );
        }
    }
}

/// The converse: a formula with no cross-sectional operator must not be parked
/// in the unavailable table, because "unavailable" would then be a place to put
/// anything inconvenient.
#[test]
fn no_computable_formula_is_parked_in_the_unavailable_table() {
    let contract = formulas();
    for (name, blockers) in worldquant101::UNAVAILABLE {
        let formula = &contract.formulas[*name];
        let present = operators(formula);
        let hit: Vec<&str> = present
            .iter()
            .copied()
            .filter(|operator| CROSS_SECTIONAL.contains(operator))
            .collect();
        assert_eq!(
            hit.len(),
            blockers.len(),
            "{name} names {} blocker(s) but its formula contains {hit:?}",
            blockers.len()
        );
    }
}

#[test]
fn every_computable_alpha_compiles_and_evaluates() {
    let ctx = market();
    let library = factor_library("worldquant101").expect("the worldquant101 library builds");
    assert_eq!(library.len(), worldquant101::COMPUTABLE_COUNT);

    for (name, expression) in worldquant101::EXPRESSIONS {
        let values = library
            .evaluate(name, &ctx)
            .unwrap_or_else(|err| panic!("evaluate {name} ({expression}): {err}"));
        assert_eq!(values.len(), BARS, "{name} produced {} bars", values.len());
        assert!(
            values.iter().any(|value| value.is_finite()),
            "{name} is all-NaN on the shared market: {expression}"
        );
    }
}

#[test]
fn the_annotated_alphas_are_absent_from_the_library() {
    let library = factor_library("worldquant101").expect("the worldquant101 library builds");
    for (name, blockers) in worldquant101::UNAVAILABLE {
        assert!(
            library.get(name).is_none(),
            "{name} is annotated as unavailable (blocked by {blockers:?}) but the \
             library exposes it"
        );
    }
}
