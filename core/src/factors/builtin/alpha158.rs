//! Qlib's **Alpha158** factor set, expressed in finkit's formula language.
//!
//! # Provenance
//!
//! The 158 factors, their names and their order are transcribed from
//! `qlib/contrib/data/loader.py::Alpha158DL.get_feature_config`. The structure
//! is 9 kbar shape factors + 4 price factors + 29 rolling operators across the
//! windows `[5, 10, 20, 30, 60]`, i.e. `9 + 4 + 29 * 5 = 158`.
//!
//! # The renaming, stated once
//!
//! Qlib's expression language is not finkit's, so each Qlib operator is mapped
//! onto the finkit function that computes the same quantity. Nothing else is
//! changed: the `+1e-12` guards, the `Ref($close, 1)` shift, the window list and
//! the operator order are Qlib's.
//!
//! | Qlib                | finkit              | note                                        |
//! | ------------------- | ------------------- | ------------------------------------------- |
//! | `$open` … `$vwap`   | `OPEN` … `VWAP`     | data fields                                 |
//! | `Mean`              | `MA`                | rolling arithmetic mean                     |
//! | `Std`               | `STDDEV_SAMPLE`     | **not** `STD`: see below                    |
//! | `Sum`               | `SUM`               |                                             |
//! | `Max` / `Min`       | `HHV` / `LLV`       | *rolling* extremum                          |
//! | `Ref`               | `REF`               | `shift(N)`; both are NaN for the first `N`  |
//! | `Abs` / `Log`       | `ABS` / `LN`        | `Log` is the natural log                    |
//! | `Greater` / `Less`  | `MAX` / `MIN`       | element-wise, **not** boolean               |
//! | `Slope`             | `LINEARREG_SLOPE`   | OLS slope against the bar index             |
//! | `Rsquare`           | `RSQUARE`           | OLS `R²`                                    |
//! | `Resi`              | `RESI`              | residual of the newest bar                  |
//! | `Quantile`          | `QUANTILE`          | linear interpolation                        |
//! | `Rank`              | `RANK_PCT`          | pandas `rolling.rank(pct=True)`, scale `(0, 1]` |
//! | `Corr`              | `CORREL`            | rolling Pearson correlation                 |
//! | `IdxMax` / `IdxMin` | `MAXINDEX` / `MININDEX` **+ 1** | Qlib counts from 1, finkit from 0 |
//!
//! The `+ 1` is the only *structural* rewrite rather than a renaming, and it is
//! visible in the table below as an explicit `+1`. In `IMXD{d}` the two `+1`s
//! cancel, but they are written out rather than cancelled so that each
//! expression stays a literal translation of Qlib's.
//!
//! # Why `Std` is not `STD`
//!
//! finkit's `STD` and `STDDEV` are TA-Lib's, which divide the second moment by
//! `n`; pandas' `rolling(N).std()` — and therefore Qlib's `Std`, which is
//! literally `series.rolling(N, min_periods=1).std()` — divides by `n - 1`. The
//! two differ by the exact factor `sqrt((n - 1) / n)`, so they are not the same
//! statistic under different names. Measured on this library's own reference
//! market, the ratio is `0.894427191` for `STD5`, `0.974679434` for `STD20` and
//! `0.991631652` for `STD60` — matching `sqrt((n - 1) / n)` to nine places, and
//! invariant across every post-warm-up bar.
//!
//! A ~10% scale error at `n = 5` is not something a tolerance can absorb, so the
//! sample convention got its own name (`STDDEV_SAMPLE`, delegating to
//! [`crate::math::rolling_stats::stddev_sample_into`]) rather than `STD` being
//! changed. Changing `STD` would have broken TA-Lib parity for every existing
//! caller in order to serve one factor library. Fifteen factors are affected:
//! `STD{d}`, `VSTD{d}` and `WVMA{d}` for each of the five windows.
//!
//! This was found by measurement, not by reading: an earlier draft of this
//! comment asserted that `Std` and `STD` "both use the sample convention", which
//! the parity diagnostic falsified.
//!
//! # What is verified
//!
//! The numbers are not taken on trust. `tests/alpha158_parity.rs` evaluates every
//! factor on a deterministic market and compares it against
//! `tests/golden/alpha158/reference_v1.json`, which `scripts/gen_alpha158_reference.py`
//! produces by running Qlib's own expressions on pandas — the substrate
//! `qlib/data/ops.py` itself delegates to. Agreement is asserted to `1e-8` on
//! the common support, and the warm-up divergence is asserted separately to be
//! exactly the `min_periods` convention difference, so a real error during
//! warm-up still fails.
//!
//! # Direction
//!
//! Every factor is [`FactorDirection::Neutral`]. Alpha158 is a feature set, not
//! a signal set: Qlib assigns no ranking direction to any of its 158 factors,
//! and inventing one here would be a claim the reference does not make.

use crate::factors::builtin::FactorLibrary;
use crate::factors::{FactorDirection, FactorError};

/// Number of factors, pinned so a dropped entry is a test failure rather than a
/// smaller library.
pub const FACTOR_COUNT: usize = 158;

/// Rolling windows, from `Alpha158DL.get_feature_config`'s default `rolling` block.
pub const WINDOWS: [usize; 5] = [5, 10, 20, 30, 60];

/// The 158 `(name, finkit expression)` pairs, in Qlib's order.
pub const EXPRESSIONS: &[(&str, &str)] = &[
    ("KMID", "(CLOSE-OPEN)/OPEN"),
    ("KLEN", "(HIGH-LOW)/OPEN"),
    ("KMID2", "(CLOSE-OPEN)/(HIGH-LOW+1e-12)"),
    ("KUP", "(HIGH-MAX(OPEN, CLOSE))/OPEN"),
    ("KUP2", "(HIGH-MAX(OPEN, CLOSE))/(HIGH-LOW+1e-12)"),
    ("KLOW", "(MIN(OPEN, CLOSE)-LOW)/OPEN"),
    ("KLOW2", "(MIN(OPEN, CLOSE)-LOW)/(HIGH-LOW+1e-12)"),
    ("KSFT", "(2*CLOSE-HIGH-LOW)/OPEN"),
    ("KSFT2", "(2*CLOSE-HIGH-LOW)/(HIGH-LOW+1e-12)"),
    ("OPEN0", "OPEN/CLOSE"),
    ("HIGH0", "HIGH/CLOSE"),
    ("LOW0", "LOW/CLOSE"),
    ("VWAP0", "VWAP/CLOSE"),
    ("ROC5", "REF(CLOSE, 5)/CLOSE"),
    ("MA5", "MA(CLOSE, 5)/CLOSE"),
    ("STD5", "STDDEV_SAMPLE(CLOSE, 5)/CLOSE"),
    ("BETA5", "LINEARREG_SLOPE(CLOSE, 5)/CLOSE"),
    ("RSQR5", "RSQUARE(CLOSE, 5)"),
    ("RESI5", "RESI(CLOSE, 5)/CLOSE"),
    ("MAX5", "HHV(HIGH, 5)/CLOSE"),
    ("MIN5", "LLV(LOW, 5)/CLOSE"),
    ("QTLU5", "QUANTILE(CLOSE, 5, 0.8)/CLOSE"),
    ("QTLD5", "QUANTILE(CLOSE, 5, 0.2)/CLOSE"),
    ("RANK5", "RANK_PCT(CLOSE, 5)"),
    ("RSV5", "(CLOSE-LLV(LOW, 5))/(HHV(HIGH, 5)-LLV(LOW, 5)+1e-12)"),
    ("IMAX5", "(MAXINDEX(HIGH, 5)+1)/5"),
    ("IMIN5", "(MININDEX(LOW, 5)+1)/5"),
    ("IMXD5", "((MAXINDEX(HIGH, 5)+1)-(MININDEX(LOW, 5)+1))/5"),
    ("CORR5", "CORREL(CLOSE, LN(VOLUME+1), 5)"),
    ("CORD5", "CORREL(CLOSE/REF(CLOSE,1), LN(VOLUME/REF(VOLUME, 1)+1), 5)"),
    ("CNTP5", "MA(CLOSE>REF(CLOSE, 1), 5)"),
    ("CNTN5", "MA(CLOSE<REF(CLOSE, 1), 5)"),
    ("CNTD5", "MA(CLOSE>REF(CLOSE, 1), 5)-MA(CLOSE<REF(CLOSE, 1), 5)"),
    ("SUMP5", "SUM(MAX(CLOSE-REF(CLOSE, 1), 0), 5)/(SUM(ABS(CLOSE-REF(CLOSE, 1)), 5)+1e-12)"),
    ("SUMN5", "SUM(MAX(REF(CLOSE, 1)-CLOSE, 0), 5)/(SUM(ABS(CLOSE-REF(CLOSE, 1)), 5)+1e-12)"),
    ("SUMD5", "(SUM(MAX(CLOSE-REF(CLOSE, 1), 0), 5)-SUM(MAX(REF(CLOSE, 1)-CLOSE, 0), 5))/(SUM(ABS(CLOSE-REF(CLOSE, 1)), 5)+1e-12)"),
    ("VMA5", "MA(VOLUME, 5)/(VOLUME+1e-12)"),
    ("VSTD5", "STDDEV_SAMPLE(VOLUME, 5)/(VOLUME+1e-12)"),
    ("WVMA5", "STDDEV_SAMPLE(ABS(CLOSE/REF(CLOSE,1)-1)*VOLUME, 5)/(MA(ABS(CLOSE/REF(CLOSE,1)-1)*VOLUME, 5)+1e-12)"),
    ("VSUMP5", "SUM(MAX(VOLUME-REF(VOLUME, 1), 0), 5)/(SUM(ABS(VOLUME-REF(VOLUME, 1)), 5)+1e-12)"),
    ("VSUMN5", "SUM(MAX(REF(VOLUME, 1)-VOLUME, 0), 5)/(SUM(ABS(VOLUME-REF(VOLUME, 1)), 5)+1e-12)"),
    ("VSUMD5", "(SUM(MAX(VOLUME-REF(VOLUME, 1), 0), 5)-SUM(MAX(REF(VOLUME, 1)-VOLUME, 0), 5))/(SUM(ABS(VOLUME-REF(VOLUME, 1)), 5)+1e-12)"),
    ("ROC10", "REF(CLOSE, 10)/CLOSE"),
    ("MA10", "MA(CLOSE, 10)/CLOSE"),
    ("STD10", "STDDEV_SAMPLE(CLOSE, 10)/CLOSE"),
    ("BETA10", "LINEARREG_SLOPE(CLOSE, 10)/CLOSE"),
    ("RSQR10", "RSQUARE(CLOSE, 10)"),
    ("RESI10", "RESI(CLOSE, 10)/CLOSE"),
    ("MAX10", "HHV(HIGH, 10)/CLOSE"),
    ("MIN10", "LLV(LOW, 10)/CLOSE"),
    ("QTLU10", "QUANTILE(CLOSE, 10, 0.8)/CLOSE"),
    ("QTLD10", "QUANTILE(CLOSE, 10, 0.2)/CLOSE"),
    ("RANK10", "RANK_PCT(CLOSE, 10)"),
    ("RSV10", "(CLOSE-LLV(LOW, 10))/(HHV(HIGH, 10)-LLV(LOW, 10)+1e-12)"),
    ("IMAX10", "(MAXINDEX(HIGH, 10)+1)/10"),
    ("IMIN10", "(MININDEX(LOW, 10)+1)/10"),
    ("IMXD10", "((MAXINDEX(HIGH, 10)+1)-(MININDEX(LOW, 10)+1))/10"),
    ("CORR10", "CORREL(CLOSE, LN(VOLUME+1), 10)"),
    ("CORD10", "CORREL(CLOSE/REF(CLOSE,1), LN(VOLUME/REF(VOLUME, 1)+1), 10)"),
    ("CNTP10", "MA(CLOSE>REF(CLOSE, 1), 10)"),
    ("CNTN10", "MA(CLOSE<REF(CLOSE, 1), 10)"),
    ("CNTD10", "MA(CLOSE>REF(CLOSE, 1), 10)-MA(CLOSE<REF(CLOSE, 1), 10)"),
    ("SUMP10", "SUM(MAX(CLOSE-REF(CLOSE, 1), 0), 10)/(SUM(ABS(CLOSE-REF(CLOSE, 1)), 10)+1e-12)"),
    ("SUMN10", "SUM(MAX(REF(CLOSE, 1)-CLOSE, 0), 10)/(SUM(ABS(CLOSE-REF(CLOSE, 1)), 10)+1e-12)"),
    ("SUMD10", "(SUM(MAX(CLOSE-REF(CLOSE, 1), 0), 10)-SUM(MAX(REF(CLOSE, 1)-CLOSE, 0), 10))/(SUM(ABS(CLOSE-REF(CLOSE, 1)), 10)+1e-12)"),
    ("VMA10", "MA(VOLUME, 10)/(VOLUME+1e-12)"),
    ("VSTD10", "STDDEV_SAMPLE(VOLUME, 10)/(VOLUME+1e-12)"),
    ("WVMA10", "STDDEV_SAMPLE(ABS(CLOSE/REF(CLOSE,1)-1)*VOLUME, 10)/(MA(ABS(CLOSE/REF(CLOSE,1)-1)*VOLUME, 10)+1e-12)"),
    ("VSUMP10", "SUM(MAX(VOLUME-REF(VOLUME, 1), 0), 10)/(SUM(ABS(VOLUME-REF(VOLUME, 1)), 10)+1e-12)"),
    ("VSUMN10", "SUM(MAX(REF(VOLUME, 1)-VOLUME, 0), 10)/(SUM(ABS(VOLUME-REF(VOLUME, 1)), 10)+1e-12)"),
    ("VSUMD10", "(SUM(MAX(VOLUME-REF(VOLUME, 1), 0), 10)-SUM(MAX(REF(VOLUME, 1)-VOLUME, 0), 10))/(SUM(ABS(VOLUME-REF(VOLUME, 1)), 10)+1e-12)"),
    ("ROC20", "REF(CLOSE, 20)/CLOSE"),
    ("MA20", "MA(CLOSE, 20)/CLOSE"),
    ("STD20", "STDDEV_SAMPLE(CLOSE, 20)/CLOSE"),
    ("BETA20", "LINEARREG_SLOPE(CLOSE, 20)/CLOSE"),
    ("RSQR20", "RSQUARE(CLOSE, 20)"),
    ("RESI20", "RESI(CLOSE, 20)/CLOSE"),
    ("MAX20", "HHV(HIGH, 20)/CLOSE"),
    ("MIN20", "LLV(LOW, 20)/CLOSE"),
    ("QTLU20", "QUANTILE(CLOSE, 20, 0.8)/CLOSE"),
    ("QTLD20", "QUANTILE(CLOSE, 20, 0.2)/CLOSE"),
    ("RANK20", "RANK_PCT(CLOSE, 20)"),
    ("RSV20", "(CLOSE-LLV(LOW, 20))/(HHV(HIGH, 20)-LLV(LOW, 20)+1e-12)"),
    ("IMAX20", "(MAXINDEX(HIGH, 20)+1)/20"),
    ("IMIN20", "(MININDEX(LOW, 20)+1)/20"),
    ("IMXD20", "((MAXINDEX(HIGH, 20)+1)-(MININDEX(LOW, 20)+1))/20"),
    ("CORR20", "CORREL(CLOSE, LN(VOLUME+1), 20)"),
    ("CORD20", "CORREL(CLOSE/REF(CLOSE,1), LN(VOLUME/REF(VOLUME, 1)+1), 20)"),
    ("CNTP20", "MA(CLOSE>REF(CLOSE, 1), 20)"),
    ("CNTN20", "MA(CLOSE<REF(CLOSE, 1), 20)"),
    ("CNTD20", "MA(CLOSE>REF(CLOSE, 1), 20)-MA(CLOSE<REF(CLOSE, 1), 20)"),
    ("SUMP20", "SUM(MAX(CLOSE-REF(CLOSE, 1), 0), 20)/(SUM(ABS(CLOSE-REF(CLOSE, 1)), 20)+1e-12)"),
    ("SUMN20", "SUM(MAX(REF(CLOSE, 1)-CLOSE, 0), 20)/(SUM(ABS(CLOSE-REF(CLOSE, 1)), 20)+1e-12)"),
    ("SUMD20", "(SUM(MAX(CLOSE-REF(CLOSE, 1), 0), 20)-SUM(MAX(REF(CLOSE, 1)-CLOSE, 0), 20))/(SUM(ABS(CLOSE-REF(CLOSE, 1)), 20)+1e-12)"),
    ("VMA20", "MA(VOLUME, 20)/(VOLUME+1e-12)"),
    ("VSTD20", "STDDEV_SAMPLE(VOLUME, 20)/(VOLUME+1e-12)"),
    ("WVMA20", "STDDEV_SAMPLE(ABS(CLOSE/REF(CLOSE,1)-1)*VOLUME, 20)/(MA(ABS(CLOSE/REF(CLOSE,1)-1)*VOLUME, 20)+1e-12)"),
    ("VSUMP20", "SUM(MAX(VOLUME-REF(VOLUME, 1), 0), 20)/(SUM(ABS(VOLUME-REF(VOLUME, 1)), 20)+1e-12)"),
    ("VSUMN20", "SUM(MAX(REF(VOLUME, 1)-VOLUME, 0), 20)/(SUM(ABS(VOLUME-REF(VOLUME, 1)), 20)+1e-12)"),
    ("VSUMD20", "(SUM(MAX(VOLUME-REF(VOLUME, 1), 0), 20)-SUM(MAX(REF(VOLUME, 1)-VOLUME, 0), 20))/(SUM(ABS(VOLUME-REF(VOLUME, 1)), 20)+1e-12)"),
    ("ROC30", "REF(CLOSE, 30)/CLOSE"),
    ("MA30", "MA(CLOSE, 30)/CLOSE"),
    ("STD30", "STDDEV_SAMPLE(CLOSE, 30)/CLOSE"),
    ("BETA30", "LINEARREG_SLOPE(CLOSE, 30)/CLOSE"),
    ("RSQR30", "RSQUARE(CLOSE, 30)"),
    ("RESI30", "RESI(CLOSE, 30)/CLOSE"),
    ("MAX30", "HHV(HIGH, 30)/CLOSE"),
    ("MIN30", "LLV(LOW, 30)/CLOSE"),
    ("QTLU30", "QUANTILE(CLOSE, 30, 0.8)/CLOSE"),
    ("QTLD30", "QUANTILE(CLOSE, 30, 0.2)/CLOSE"),
    ("RANK30", "RANK_PCT(CLOSE, 30)"),
    ("RSV30", "(CLOSE-LLV(LOW, 30))/(HHV(HIGH, 30)-LLV(LOW, 30)+1e-12)"),
    ("IMAX30", "(MAXINDEX(HIGH, 30)+1)/30"),
    ("IMIN30", "(MININDEX(LOW, 30)+1)/30"),
    ("IMXD30", "((MAXINDEX(HIGH, 30)+1)-(MININDEX(LOW, 30)+1))/30"),
    ("CORR30", "CORREL(CLOSE, LN(VOLUME+1), 30)"),
    ("CORD30", "CORREL(CLOSE/REF(CLOSE,1), LN(VOLUME/REF(VOLUME, 1)+1), 30)"),
    ("CNTP30", "MA(CLOSE>REF(CLOSE, 1), 30)"),
    ("CNTN30", "MA(CLOSE<REF(CLOSE, 1), 30)"),
    ("CNTD30", "MA(CLOSE>REF(CLOSE, 1), 30)-MA(CLOSE<REF(CLOSE, 1), 30)"),
    ("SUMP30", "SUM(MAX(CLOSE-REF(CLOSE, 1), 0), 30)/(SUM(ABS(CLOSE-REF(CLOSE, 1)), 30)+1e-12)"),
    ("SUMN30", "SUM(MAX(REF(CLOSE, 1)-CLOSE, 0), 30)/(SUM(ABS(CLOSE-REF(CLOSE, 1)), 30)+1e-12)"),
    ("SUMD30", "(SUM(MAX(CLOSE-REF(CLOSE, 1), 0), 30)-SUM(MAX(REF(CLOSE, 1)-CLOSE, 0), 30))/(SUM(ABS(CLOSE-REF(CLOSE, 1)), 30)+1e-12)"),
    ("VMA30", "MA(VOLUME, 30)/(VOLUME+1e-12)"),
    ("VSTD30", "STDDEV_SAMPLE(VOLUME, 30)/(VOLUME+1e-12)"),
    ("WVMA30", "STDDEV_SAMPLE(ABS(CLOSE/REF(CLOSE,1)-1)*VOLUME, 30)/(MA(ABS(CLOSE/REF(CLOSE,1)-1)*VOLUME, 30)+1e-12)"),
    ("VSUMP30", "SUM(MAX(VOLUME-REF(VOLUME, 1), 0), 30)/(SUM(ABS(VOLUME-REF(VOLUME, 1)), 30)+1e-12)"),
    ("VSUMN30", "SUM(MAX(REF(VOLUME, 1)-VOLUME, 0), 30)/(SUM(ABS(VOLUME-REF(VOLUME, 1)), 30)+1e-12)"),
    ("VSUMD30", "(SUM(MAX(VOLUME-REF(VOLUME, 1), 0), 30)-SUM(MAX(REF(VOLUME, 1)-VOLUME, 0), 30))/(SUM(ABS(VOLUME-REF(VOLUME, 1)), 30)+1e-12)"),
    ("ROC60", "REF(CLOSE, 60)/CLOSE"),
    ("MA60", "MA(CLOSE, 60)/CLOSE"),
    ("STD60", "STDDEV_SAMPLE(CLOSE, 60)/CLOSE"),
    ("BETA60", "LINEARREG_SLOPE(CLOSE, 60)/CLOSE"),
    ("RSQR60", "RSQUARE(CLOSE, 60)"),
    ("RESI60", "RESI(CLOSE, 60)/CLOSE"),
    ("MAX60", "HHV(HIGH, 60)/CLOSE"),
    ("MIN60", "LLV(LOW, 60)/CLOSE"),
    ("QTLU60", "QUANTILE(CLOSE, 60, 0.8)/CLOSE"),
    ("QTLD60", "QUANTILE(CLOSE, 60, 0.2)/CLOSE"),
    ("RANK60", "RANK_PCT(CLOSE, 60)"),
    ("RSV60", "(CLOSE-LLV(LOW, 60))/(HHV(HIGH, 60)-LLV(LOW, 60)+1e-12)"),
    ("IMAX60", "(MAXINDEX(HIGH, 60)+1)/60"),
    ("IMIN60", "(MININDEX(LOW, 60)+1)/60"),
    ("IMXD60", "((MAXINDEX(HIGH, 60)+1)-(MININDEX(LOW, 60)+1))/60"),
    ("CORR60", "CORREL(CLOSE, LN(VOLUME+1), 60)"),
    ("CORD60", "CORREL(CLOSE/REF(CLOSE,1), LN(VOLUME/REF(VOLUME, 1)+1), 60)"),
    ("CNTP60", "MA(CLOSE>REF(CLOSE, 1), 60)"),
    ("CNTN60", "MA(CLOSE<REF(CLOSE, 1), 60)"),
    ("CNTD60", "MA(CLOSE>REF(CLOSE, 1), 60)-MA(CLOSE<REF(CLOSE, 1), 60)"),
    ("SUMP60", "SUM(MAX(CLOSE-REF(CLOSE, 1), 0), 60)/(SUM(ABS(CLOSE-REF(CLOSE, 1)), 60)+1e-12)"),
    ("SUMN60", "SUM(MAX(REF(CLOSE, 1)-CLOSE, 0), 60)/(SUM(ABS(CLOSE-REF(CLOSE, 1)), 60)+1e-12)"),
    ("SUMD60", "(SUM(MAX(CLOSE-REF(CLOSE, 1), 0), 60)-SUM(MAX(REF(CLOSE, 1)-CLOSE, 0), 60))/(SUM(ABS(CLOSE-REF(CLOSE, 1)), 60)+1e-12)"),
    ("VMA60", "MA(VOLUME, 60)/(VOLUME+1e-12)"),
    ("VSTD60", "STDDEV_SAMPLE(VOLUME, 60)/(VOLUME+1e-12)"),
    ("WVMA60", "STDDEV_SAMPLE(ABS(CLOSE/REF(CLOSE,1)-1)*VOLUME, 60)/(MA(ABS(CLOSE/REF(CLOSE,1)-1)*VOLUME, 60)+1e-12)"),
    ("VSUMP60", "SUM(MAX(VOLUME-REF(VOLUME, 1), 0), 60)/(SUM(ABS(VOLUME-REF(VOLUME, 1)), 60)+1e-12)"),
    ("VSUMN60", "SUM(MAX(REF(VOLUME, 1)-VOLUME, 0), 60)/(SUM(ABS(VOLUME-REF(VOLUME, 1)), 60)+1e-12)"),
    ("VSUMD60", "(SUM(MAX(VOLUME-REF(VOLUME, 1), 0), 60)-SUM(MAX(REF(VOLUME, 1)-VOLUME, 0), 60))/(SUM(ABS(VOLUME-REF(VOLUME, 1)), 60)+1e-12)"),
];

/// Build the Alpha158 library.
///
/// All 158 graphs are parsed and compiled here, once. Everything the library
/// hands out afterwards — [`FactorLibrary::evaluate`], the registry returned by
/// [`FactorLibrary::registry`] — reuses those compiled plans.
///
/// # Errors
///
/// [`FactorError::Compute`] naming the factor if any expression fails to compile.
/// That would be a defect in this module rather than in the caller, which is why
/// it is reported with the factor name instead of being skipped.
pub fn library() -> Result<FactorLibrary, FactorError> {
    let entries: Vec<(&str, &str, FactorDirection)> = EXPRESSIONS
        .iter()
        .map(|(name, expression)| (*name, *expression, FactorDirection::Neutral))
        .collect();
    FactorLibrary::from_expressions("alpha158", &entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn the_table_has_the_documented_shape() {
        assert_eq!(EXPRESSIONS.len(), FACTOR_COUNT);
        let names: BTreeSet<&str> = EXPRESSIONS.iter().map(|(name, _)| *name).collect();
        assert_eq!(names.len(), FACTOR_COUNT, "factor names must be unique");

        // 9 kbar + 4 price + 29 operators x 5 windows.
        let rolling = EXPRESSIONS
            .iter()
            .filter(|(name, _)| {
                WINDOWS
                    .iter()
                    .any(|window| name.ends_with(&window.to_string()))
            })
            .count();
        assert_eq!(rolling, 29 * WINDOWS.len());
        assert_eq!(FACTOR_COUNT - rolling, 13);
    }

    #[test]
    fn every_window_has_the_same_operator_set() {
        // The per-window operator list is generated from one template in Qlib, so
        // a window whose operator set differs means the table was hand-edited.
        let operator_of = |name: &str| {
            WINDOWS
                .iter()
                .find_map(|window| name.strip_suffix(&window.to_string()).map(str::to_string))
        };
        let mut baseline: Option<BTreeSet<String>> = None;
        for window in WINDOWS {
            let operators: BTreeSet<String> = EXPRESSIONS
                .iter()
                .filter(|(name, _)| name.ends_with(&window.to_string()))
                .map(|(name, _)| operator_of(name).expect("suffix was just matched"))
                .collect();
            match &baseline {
                None => baseline = Some(operators),
                Some(expected) => assert_eq!(&operators, expected, "window {window} differs"),
            }
        }
    }

    #[test]
    fn every_expression_compiles() {
        let library = library().expect("alpha158 compiles");
        assert_eq!(library.len(), FACTOR_COUNT);
    }

    #[test]
    fn dependencies_are_the_ohlcv_fields_plus_vwap() {
        let library = library().expect("alpha158 compiles");
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for (_, factor) in library.iter() {
            for dependency in factor.dependencies() {
                seen.insert(dependency.as_str());
            }
        }
        let expected: BTreeSet<&str> = ["OPEN", "HIGH", "LOW", "CLOSE", "VOLUME", "VWAP"]
            .into_iter()
            .collect();
        assert_eq!(seen, expected);
    }

    #[test]
    fn only_the_kbar_block_and_open0_read_open() {
        // Eight of the nine kbar factors read `OPEN`; `KSFT2` is the exception,
        // because it divides by `HIGH-LOW+1e-12` rather than by `OPEN`. `OPEN0`
        // is `open/close` by construction. Nothing else touches `OPEN`, so a
        // factor that suddenly depends on it would mean a mistranslated template.
        let library = library().expect("alpha158 compiles");
        let with_open: BTreeSet<&str> = library
            .iter()
            .filter(|(_, factor)| factor.dependencies().iter().any(|name| name == "OPEN"))
            .map(|(name, _)| name)
            .collect();
        assert_eq!(
            with_open,
            ["KMID", "KLEN", "KMID2", "KUP", "KUP2", "KLOW", "KLOW2", "KSFT", "OPEN0",]
                .into_iter()
                .collect()
        );
    }
}
