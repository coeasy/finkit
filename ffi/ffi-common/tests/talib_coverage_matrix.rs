//! Verify the FFI TA-Lib catalog and dispatcher against the shared coverage matrix.

use finkit::operation::builtin_operation_registry;
use finkit_ffi_common::{
    is_profile_catalog_name, talib_profile_supported, TALIB_PROFILE_CATALOG_NAMES,
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn coverage_matrix() -> Value {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = workspace_root.join("tests/contracts/talib_coverage_matrix_v1.json");
    let source = fs::read_to_string(path).expect("read TA-Lib coverage matrix");
    serde_json::from_str(&source).expect("parse TA-Lib coverage matrix")
}

fn dispatcher_names() -> BTreeSet<String> {
    let mut names = TALIB_PROFILE_CATALOG_NAMES
        .iter()
        .map(|name| (*name).to_string())
        .collect::<BTreeSet<_>>();
    for spec in builtin_operation_registry().iter() {
        if talib_profile_supported(&spec.name) {
            names.insert(spec.name.clone());
        }
    }
    names
}

#[test]
fn ffi_catalog_and_dispatcher_match_the_coverage_matrix() {
    let matrix = coverage_matrix();
    let catalog = &matrix["surfaces"]["profile_catalog"];
    let dispatcher = &matrix["surfaces"]["dispatcher_smoke"];
    let numeric = matrix["surfaces"]["numeric_reference"]["indicators"]
        .as_array()
        .expect("numeric reference names");

    assert_eq!(catalog["status"], "registered");
    assert_eq!(
        catalog["expected_count"].as_u64(),
        Some(TALIB_PROFILE_CATALOG_NAMES.len() as u64)
    );
    assert_eq!(dispatcher["status"], "executable_smoke");

    let profile_names = TALIB_PROFILE_CATALOG_NAMES
        .iter()
        .copied()
        .collect::<Vec<_>>();
    let unique_profile_names = profile_names.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(
        unique_profile_names.len(),
        profile_names.len(),
        "TA-Lib catalog contains duplicate names"
    );
    assert!(
        profile_names.windows(2).all(|pair| pair[0] < pair[1]),
        "TA-Lib catalog must remain sorted"
    );

    let executable = dispatcher_names();
    assert_eq!(
        dispatcher["expected_count"].as_u64(),
        Some(executable.len() as u64)
    );
    for name in TALIB_PROFILE_CATALOG_NAMES {
        assert!(is_profile_catalog_name(name));
        assert!(
            talib_profile_supported(name),
            "catalog entry {name} must be covered by dispatcher smoke"
        );
    }
    for name in numeric {
        let name = name.as_str().expect("numeric reference name");
        assert!(
            executable.contains(name),
            "numeric reference {name} must be executable by the TA-Lib dispatcher"
        );
    }
}
