//! Contract test for the versioned TA-Lib coverage state matrix.

mod common;

use common::talib_coverage::{
    assert_catalog_matches_matrix, assert_formula_surface_is_callable, dispatcher_names,
    golden_indicator_names, load_matrix,
};
use finkit_ffi_common::{
    is_profile_catalog_name, talib_profile_supported, TALIB_PROFILE_CATALOG_NAMES,
};
use std::collections::BTreeSet;

#[test]
fn talib_coverage_surfaces_are_explicit_and_consistent() {
    let matrix = load_matrix();
    assert_catalog_matches_matrix(&matrix);

    // A golden vector alone is not enough: the same name must be callable from
    // the formula DSL. See `assert_formula_surface_is_callable`. Checked before
    // `numeric_reference.indicators` is consumed below.
    assert_formula_surface_is_callable(&matrix);

    let numeric = matrix
        .surfaces
        .numeric_reference
        .indicators
        .into_iter()
        .collect::<BTreeSet<_>>();
    let golden = golden_indicator_names();
    assert_eq!(numeric, golden);
    let dispatchable = dispatcher_names();

    for name in TALIB_PROFILE_CATALOG_NAMES {
        assert!(is_profile_catalog_name(name));
        assert!(
            talib_profile_supported(name),
            "catalog entry {name} must be covered by dispatcher smoke"
        );
    }
    for name in numeric {
        assert!(
            dispatchable.contains(&name),
            "numeric reference {name} must be executable by the TA-Lib dispatcher"
        );
    }
}
