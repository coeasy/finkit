//! Contract test for the versioned TA-Lib coverage state matrix.

mod common;

use common::talib_coverage::{
    assert_formula_surface_is_callable, assert_matrix_metadata, golden_indicator_names, load_matrix,
};
use std::collections::BTreeSet;

#[test]
fn talib_coverage_surfaces_are_explicit_and_consistent() {
    let matrix = load_matrix();
    assert_matrix_metadata(&matrix);

    // A golden vector alone is not enough: the same name must be callable from
    // the formula DSL. Check both core-owned surfaces in the core crate.
    assert_formula_surface_is_callable(&matrix);

    let numeric = matrix
        .surfaces
        .numeric_reference
        .indicators
        .into_iter()
        .collect::<BTreeSet<_>>();
    assert_eq!(numeric, golden_indicator_names());
}
