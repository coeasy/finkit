//! `IF` truthiness must not depend on how long the series is.
//!
//! This is a regression guard for a bug that was invisible to every
//! differential gate: the hand-written conditional sites tested `> 0.0` while
//! `SimdOps::select` tested `!= 0.0`, and the SIMD path only engages from 16
//! elements up. So `IF(-1, 10, 20)` returned 20 for a 15-bar series and 10 for
//! a 16-bar one. Nothing compared the same formula at two lengths, so nothing
//! noticed.
//!
//! The rule now lives in `formula::truth`; these tests pin the *property*
//! (length independence and the documented rule), not one path's behaviour.

use finkit::formula::truth::is_true;
use finkit::formula::{FormulaContext, FormulaEngine};
use ndarray::Array1;

fn ctx(n: usize) -> FormulaContext {
    FormulaContext::new(
        Array1::from_vec(vec![1.0; n]),
        Array1::from_vec(vec![2.0; n]),
        Array1::from_vec(vec![0.5; n]),
        Array1::from_vec(vec![1.5; n]),
        Array1::from_vec(vec![10.0; n]),
        None,
    )
}

/// Lengths straddling the 16-element threshold where the old split happened.
const LENGTHS: [usize; 6] = [1, 4, 15, 16, 17, 64];

#[test]
fn the_rule_is_nonzero_not_positive() {
    assert!(is_true(1.0));
    assert!(
        is_true(-1.0),
        "a negative condition is non-zero, so it is true"
    );
    assert!(is_true(f64::NAN), "`NaN != 0.0` is true");
    assert!(!is_true(0.0));
    assert!(!is_true(-0.0));
}

/// The bug: the same formula, the same data shape, two different answers.
#[test]
fn a_negative_condition_selects_the_then_branch_at_every_length() {
    for n in LENGTHS {
        let value = FormulaEngine::new()
            .eval("IF(-1, 10, 20)", &mut ctx(n))
            .unwrap_or_else(|e| panic!("len {n}: {e}"))[0];
        assert_eq!(
            value, 10.0,
            "len {n}: IF(-1, 10, 20) must select the then branch at every length"
        );
    }
}

/// The case that made this reach real formulas: a down bar makes `CLOSE - OPEN`
/// negative, so `IF(CLOSE - OPEN, ...)` used to flip on series length.
#[test]
fn a_negative_expression_condition_is_also_length_independent() {
    for n in LENGTHS {
        let mut context = ctx(n);
        // CLOSE below OPEN: the difference is negative.
        let value = FormulaEngine::new()
            .eval("IF(CLOSE - 10, 1, 0)", &mut context)
            .unwrap_or_else(|e| panic!("len {n}: {e}"))[0];
        assert_eq!(
            value, 1.0,
            "len {n}: a negative but non-zero condition must be true"
        );
    }
}

/// Zero must stay false, and must not have been caught up in the change.
#[test]
fn zero_still_selects_the_else_branch_at_every_length() {
    for n in LENGTHS {
        let value = FormulaEngine::new()
            .eval("IF(0, 10, 20)", &mut ctx(n))
            .unwrap_or_else(|e| panic!("len {n}: {e}"))[0];
        assert_eq!(value, 20.0, "len {n}: IF(0, 10, 20) must select else");
    }
}

/// The ordinary case, where both rules agree, must not have moved.
#[test]
fn a_comparison_condition_is_unaffected() {
    for n in LENGTHS {
        let then_value = FormulaEngine::new()
            .eval("IF(CLOSE > 1, 10, 20)", &mut ctx(n))
            .unwrap_or_else(|e| panic!("len {n}: {e}"))[0];
        let else_value = FormulaEngine::new()
            .eval("IF(CLOSE > 100, 10, 20)", &mut ctx(n))
            .unwrap_or_else(|e| panic!("len {n}: {e}"))[0];
        assert_eq!(then_value, 10.0, "len {n}: true comparison");
        assert_eq!(else_value, 20.0, "len {n}: false comparison");
    }
}
