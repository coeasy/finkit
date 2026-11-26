//! `WorldQuant`'s **101 Alphas**, and an honest account of how many are reachable.
//!
//! `WorldQuant` is a company name rather than an identifier, so `doc_markdown`
//! would want it in backticks at every one of the dozens of mentions below.
//! That would turn the prose of a module *about* `WorldQuant` into code font
//! throughout, so the lint is silenced here instead — scoped to this file, where
//! the name is the subject rather than an incidental reference.
#![allow(clippy::doc_markdown)]
//!
//! # What the corpus actually is
//!
//! The plan asked for "at least 80 computable factors" here. Measurement says
//! that target rests on two premises the source does not support, and both are
//! recorded rather than quietly worked around:
//!
//! 1. **The checked-in transcription contains 71 formulas, not 101.** The paper's
//!    numbering is `1..101`, but 30 numbers are absent from this transcription.
//!    Those are catalog gaps, not evidence that the paper left them undefined.
//!
//! 2. **The partial built-in catalog is mostly cross-sectional.**
//!    `rank(x)` here means *rank `x` across the universe on that date*, not a
//!    rank over time, and `scale`/`indneutralize` are likewise universe-level.
//!    `rank` alone appears 129 times across the 71 formulas and is the sole
//!    panel requirement for 50 of them. The panel API now provides both axes,
//!    but these 54 expressions are not yet exposed as built-in factors.
//!
//! There is a trap worth naming: finkit *does* have `RANK_PCT`, but it is a
//! **rolling** rank. Substituting it for `rank` would compile, run, and produce
//! a different factor. This module still does not claim complete paper coverage:
//! the 30 absent numbers require verified source transcription and the user's
//! requested redistribution clearance before they can enter a public release.
//!
//! # The split
//!
//! * [`EXPRESSIONS`] — the [`COMPUTABLE_COUNT`] alphas translated into the
//!   single-series library.
//! * [`UNAVAILABLE`] — the remaining formulas in the checked-in transcription,
//!   annotated with the cross-sectional operations they require. The annotation is *derived* from the formula text
//!   by `scripts/gen_worldquant101.py`, and
//!   `tests/worldquant101_library.rs` asserts that every recorded blocker really
//!   does occur in the formula it is attached to, so an annotation cannot be
//!   invented.
//!
//! # Provenance and the limits of verification
//!
//! The formulas are transcribed from the formula comment above each class in
//! `microsoft/qlib`'s `alpha101_defs.py`, and committed verbatim as
//! `tests/golden/worldquant101/formulas_v1.json`.
//!
//! Unlike Alpha158, there is **no numeric reference** for these factors. The
//! only available implementation is cross-sectional — it ranks across assets via
//! `stats.rankdata` — so it cannot produce a single-instrument value to compare
//! against, and re-deriving the numbers from this module's own translations
//! would only check the translations against themselves. What is verified is
//! therefore weaker and stated as such: every expression compiles into a
//! `FactorPlan`, evaluates on the shared market without error, and produces at
//! least one finite value. The arithmetic is a faithful transliteration of the
//! published formula, not a numerically confirmed reproduction of anyone's
//! output.
//!
//! # Rewrites applied during translation
//!
//! All of them are exact, and all of them are visible in [`EXPRESSIONS`]:
//!
//! | WorldQuant        | finkit              | note                                  |
//! | ----------------- | ------------------- | ------------------------------------- |
//! | `delta(x, d)`     | `x-REF(x, d)`       | `delta` is not a finkit operator       |
//! | `delay(x, d)`     | `REF(x, d)`         | same operation, different name         |
//! | `adv20`           | `MA(VOLUME, 20)`    | the 20-day average volume              |
//! | `returns`         | `CLOSE/REF(CLOSE,1)-1` | the paper's `returns` field         |
//! | `(c ? a : b)`     | `IF(c, a, b)`       | `IF` treats non-zero as true           |
//! | `(a \|\| b)`      | `MAX(a, b)`         | comparisons yield `0.0`/`1.0`, for which `max` *is* logical or |
//! | `ts_rank(x, d)`   | `RANK_PCT(x, d)`    | both are the rank *within the window*  |
//! | `stddev(x, d)`    | `STDDEV_SAMPLE(x, d)` | the paper means the sample deviation |
//! | `decay_linear`    | `WMA`               | both weight the newest bar most        |
//!
//! # Direction
//!
//! Every factor is [`FactorDirection::Neutral`]. The paper publishes these as
//! signals with a sign baked into the formula, not as directional factors, so
//! inventing a direction here would be a claim the source does not make.

use crate::factors::builtin::FactorLibrary;
use crate::factors::{FactorDirection, FactorError};

/// Number of formulas present in the checked-in partial transcription.
pub const FORMULA_COUNT: usize = 71;

/// Numbered entries absent from the checked-in transcription. These are not
/// asserted to be reserved or undefined by the paper.
pub const CATALOG_GAP_NUMBERS: &[usize] = &[
    10, 25, 28, 33, 34, 48, 58, 59, 63, 67, 69, 70, 71, 73, 76, 77, 79, 80, 82, 84, 87, 88, 89, 90,
    91, 92, 93, 96, 97, 100,
];

/// Legacy name retained for source compatibility. The values are catalog gaps,
/// not paper-reserved alpha numbers.
#[deprecated(note = "use CATALOG_GAP_NUMBERS; these numbers are not paper-reserved")]
pub const RESERVED_NUMBERS: &[usize] = CATALOG_GAP_NUMBERS;

/// Alphas translated into the single-series library.
pub const COMPUTABLE_COUNT: usize = 17;

/// The computable factors, as `(name, finkit expression)` pairs.
///
/// Generated by `scripts/gen_worldquant101.py --write` from the committed
/// formulas; `--check` fails if this table drifts from the classification.
pub const EXPRESSIONS: &[(&str, &str)] = &[
    // --- BEGIN GENERATED: expressions
    ("Alpha6", "-1*CORREL(OPEN, VOLUME, 10)"),
    ("Alpha7", "IF(MA(VOLUME,20)<VOLUME, -1*RANK_PCT(ABS(CLOSE-REF(CLOSE,7)), 60)*SIGN(CLOSE-REF(CLOSE,7)), -1)"),
    ("Alpha9", "IF(0<LLV(CLOSE-REF(CLOSE,1), 5), CLOSE-REF(CLOSE,1), IF(HHV(CLOSE-REF(CLOSE,1), 5)<0, CLOSE-REF(CLOSE,1), -1*(CLOSE-REF(CLOSE,1))))"),
    ("Alpha12", "SIGN(VOLUME-REF(VOLUME,1))*(-1*(CLOSE-REF(CLOSE,1)))"),
    ("Alpha21", "IF((SUM(CLOSE,8)/8+STDDEV_SAMPLE(CLOSE,8))<(SUM(CLOSE,2)/2), -1, IF((SUM(CLOSE,2)/2)<(SUM(CLOSE,8)/8-STDDEV_SAMPLE(CLOSE,8)), 1, IF(MAX(1<VOLUME/MA(VOLUME,20), VOLUME/MA(VOLUME,20)==1), 1, -1)))"),
    ("Alpha23", "IF(SUM(HIGH,20)/20<HIGH, -1*(HIGH-REF(HIGH,2)), 0)"),
    ("Alpha24", "IF(MAX(((SUM(CLOSE,100)/100)-REF(SUM(CLOSE,100)/100,100))/REF(CLOSE,100)<0.05, ((SUM(CLOSE,100)/100)-REF(SUM(CLOSE,100)/100,100))/REF(CLOSE,100)==0.05), -1*(CLOSE-LLV(CLOSE,100)), -1*(CLOSE-REF(CLOSE,3)))"),
    ("Alpha26", "-1*HHV(CORREL(RANK_PCT(VOLUME,5), RANK_PCT(HIGH,5), 5), 3)"),
    ("Alpha35", "RANK_PCT(VOLUME,32)*(1-RANK_PCT((CLOSE+HIGH)-LOW,16))*(1-RANK_PCT(CLOSE/REF(CLOSE,1)-1,32))"),
    ("Alpha41", "(HIGH*LOW)^0.5-VWAP"),
    ("Alpha43", "RANK_PCT(VOLUME/MA(VOLUME,20), 20)*RANK_PCT(-1*(CLOSE-REF(CLOSE,7)), 8)"),
    ("Alpha46", "IF(0.25<(((REF(CLOSE,20)-REF(CLOSE,10))/10)-((REF(CLOSE,10)-CLOSE)/10)), -1, IF((((REF(CLOSE,20)-REF(CLOSE,10))/10)-((REF(CLOSE,10)-CLOSE)/10))<0, 1, -1*(CLOSE-REF(CLOSE,1))))"),
    ("Alpha49", "IF((((REF(CLOSE,20)-REF(CLOSE,10))/10)-((REF(CLOSE,10)-CLOSE)/10))<(-1*0.1), 1, -1*(CLOSE-REF(CLOSE,1)))"),
    ("Alpha51", "IF((((REF(CLOSE,20)-REF(CLOSE,10))/10)-((REF(CLOSE,10)-CLOSE)/10))<(-1*0.05), 1, -1*(CLOSE-REF(CLOSE,1)))"),
    ("Alpha53", "-1*(((CLOSE-LOW)-(HIGH-CLOSE))/(CLOSE-LOW)-REF(((CLOSE-LOW)-(HIGH-CLOSE))/(CLOSE-LOW),9))"),
    ("Alpha54", "(-1*((LOW-CLOSE)*OPEN^5))/((LOW-HIGH)*CLOSE^5)"),
    ("Alpha101", "(CLOSE-OPEN)/((HIGH-LOW)+0.001)"),
    // --- END GENERATED: expressions
];

/// Alphas not exposed in the built-in single-series library, as
/// `(name, required panel operations)`.
///
/// Each blocker is an operator that appears in that alpha's formula and that
/// for correct semantics, because it needs the universe on one date rather than
/// a series over time. The panel API supplies those axes, but this partial
/// catalog has not registered these expressions. See [`CATALOG_GAP_NUMBERS`]
/// for entries absent from the transcription, and the module docs for why
/// `RANK_PCT` is not a substitute for `rank`.
pub const UNAVAILABLE: &[(&str, &[&str])] = &[
    // --- BEGIN GENERATED: unavailable
    ("Alpha1", &["rank"]),
    ("Alpha2", &["rank"]),
    ("Alpha3", &["rank"]),
    ("Alpha4", &["rank"]),
    ("Alpha5", &["rank"]),
    ("Alpha8", &["rank"]),
    ("Alpha11", &["rank"]),
    ("Alpha13", &["rank"]),
    ("Alpha14", &["rank"]),
    ("Alpha15", &["rank"]),
    ("Alpha16", &["rank"]),
    ("Alpha17", &["rank"]),
    ("Alpha18", &["rank"]),
    ("Alpha19", &["rank"]),
    ("Alpha20", &["rank"]),
    ("Alpha22", &["rank"]),
    ("Alpha27", &["rank"]),
    ("Alpha29", &["rank", "scale"]),
    ("Alpha30", &["rank"]),
    ("Alpha31", &["rank", "scale"]),
    ("Alpha32", &["scale"]),
    ("Alpha36", &["rank"]),
    ("Alpha37", &["rank"]),
    ("Alpha38", &["rank"]),
    ("Alpha39", &["rank"]),
    ("Alpha40", &["rank"]),
    ("Alpha42", &["rank"]),
    ("Alpha44", &["rank"]),
    ("Alpha45", &["rank"]),
    ("Alpha47", &["rank"]),
    ("Alpha50", &["rank"]),
    ("Alpha52", &["rank"]),
    ("Alpha55", &["rank"]),
    ("Alpha56", &["rank"]),
    ("Alpha57", &["rank"]),
    ("Alpha60", &["rank", "scale"]),
    ("Alpha61", &["rank"]),
    ("Alpha62", &["rank"]),
    ("Alpha64", &["rank"]),
    ("Alpha65", &["rank"]),
    ("Alpha66", &["rank"]),
    ("Alpha68", &["rank"]),
    ("Alpha72", &["rank"]),
    ("Alpha74", &["rank"]),
    ("Alpha75", &["rank"]),
    ("Alpha78", &["rank"]),
    ("Alpha81", &["rank"]),
    ("Alpha83", &["rank"]),
    ("Alpha85", &["rank"]),
    ("Alpha86", &["rank"]),
    ("Alpha94", &["rank"]),
    ("Alpha95", &["rank"]),
    ("Alpha98", &["rank"]),
    ("Alpha99", &["rank"]),
    // --- END GENERATED: unavailable
];

/// Build the WorldQuant101 library.
///
/// Contains only the computable alphas: the annotated ones are deliberately
/// absent from [`EXPRESSIONS`] rather than present and NaN, so a caller can tell
/// "not available in this engine" from "available but undefined on this bar".
///
/// # Errors
///
/// [`FactorError::Compute`] naming the factor if any expression fails to compile.
pub fn library() -> Result<FactorLibrary, FactorError> {
    let entries: Vec<(&str, &str, FactorDirection)> = EXPRESSIONS
        .iter()
        .map(|(name, expression)| (*name, *expression, FactorDirection::Neutral))
        .collect();
    FactorLibrary::from_expressions("worldquant101", &entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checked_in_corpus_and_catalog_gaps_partition_the_numbering() {
        assert_eq!(
            FORMULA_COUNT + CATALOG_GAP_NUMBERS.len(),
            101,
            "the checked-in corpus and gaps must cover the paper numbering"
        );
        let mut sorted = CATALOG_GAP_NUMBERS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            CATALOG_GAP_NUMBERS.len(),
            "catalog gap numbers repeat"
        );
        assert!(
            CATALOG_GAP_NUMBERS
                .iter()
                .all(|number| (1..=101).contains(number)),
            "catalog gap numbers must lie in 1..=101"
        );
    }

    #[test]
    fn every_formula_is_either_computed_or_annotated() {
        assert_eq!(
            EXPRESSIONS.len() + UNAVAILABLE.len(),
            FORMULA_COUNT,
            "the two tables must partition the corpus; a formula that is in \
             neither is silently dropped, and one in both is double-counted"
        );
        assert_eq!(EXPRESSIONS.len(), COMPUTABLE_COUNT);
        let computed: std::collections::BTreeSet<&str> =
            EXPRESSIONS.iter().map(|(name, _)| *name).collect();
        let annotated: std::collections::BTreeSet<&str> =
            UNAVAILABLE.iter().map(|(name, _)| *name).collect();
        assert!(
            computed.is_disjoint(&annotated),
            "an alpha cannot be both computed and annotated"
        );
    }

    #[test]
    fn every_annotated_alpha_names_a_blocker() {
        for (name, blockers) in UNAVAILABLE {
            assert!(
                !blockers.is_empty(),
                "{name} is annotated as unavailable but names no blocker"
            );
        }
    }

    #[test]
    fn the_library_holds_exactly_the_computable_alphas() {
        let library = library().expect("the worldquant101 library builds");
        assert_eq!(library.len(), COMPUTABLE_COUNT);
        for (name, _) in UNAVAILABLE {
            assert!(
                library.get(name).is_none(),
                "{name} is annotated as unavailable but is in the library"
            );
        }
    }
}
