//! Shared TA-Lib coverage matrix loader.
//!
//! The matrix intentionally separates registration, executable smoke coverage,
//! and fixed numeric references.  A catalog entry is not silently treated as
//! numeric parity until it has a checked-in golden reference.

#![allow(dead_code)]

use finkit_ffi_common::{TALIB_PROFILE_CATALOG_NAMES, TALIB_SEMANTIC_PROFILE};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct TalibCoverageMatrix {
    pub schema_version: u32,
    pub semantic_profile: String,
    pub talib_core_version: String,
    pub python_reference_version: String,
    pub surfaces: TalibCoverageSurfaces,
}

#[derive(Debug, Deserialize)]
pub struct TalibCoverageSurfaces {
    pub profile_catalog: TalibCoverageSurface,
    pub dispatcher_smoke: TalibCoverageSurface,
    pub numeric_reference: TalibNumericReference,
}

#[derive(Debug, Deserialize)]
pub struct TalibCoverageSurface {
    pub status: String,
    pub expected_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct TalibNumericReference {
    pub status: String,
    pub expected_count: usize,
    pub indicators: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GoldenMetadata {
    indicator: String,
    talib_version: String,
}

#[derive(Debug, Deserialize)]
struct GoldenFile {
    metadata: GoldenMetadata,
}

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

pub fn matrix_path() -> PathBuf {
    workspace_root().join("tests/contracts/talib_coverage_matrix_v1.json")
}

pub fn golden_dir() -> PathBuf {
    workspace_root().join("tests/golden/talib")
}

pub fn load_matrix() -> TalibCoverageMatrix {
    let raw = fs::read_to_string(matrix_path()).expect("read TA-Lib coverage matrix");
    serde_json::from_str(&raw).expect("parse TA-Lib coverage matrix")
}

pub fn golden_indicator_names() -> BTreeSet<String> {
    fs::read_dir(golden_dir())
        .expect("read TA-Lib golden directory")
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json")
                || path.file_name().and_then(|value| value.to_str())
                    == Some("profile_matype_variants.json")
            {
                return None;
            }
            let raw = fs::read_to_string(&path).expect("read TA-Lib golden file");
            let golden: GoldenFile = serde_json::from_str(&raw).expect("parse TA-Lib golden file");
            assert_eq!(
                golden.metadata.indicator,
                path.file_stem()
                    .and_then(|value| value.to_str())
                    .expect("golden file stem")
                    .to_ascii_uppercase(),
                "golden metadata indicator must match its filename"
            );
            assert_eq!(
                golden.metadata.talib_version, "0.8.0",
                "TA-Lib golden reference version drifted"
            );
            Some(golden.metadata.indicator)
        })
        .collect()
}

pub fn assert_catalog_matches_matrix(matrix: &TalibCoverageMatrix) {
    assert_eq!(matrix.schema_version, 1);
    assert_eq!(matrix.semantic_profile, TALIB_SEMANTIC_PROFILE);
    assert_eq!(matrix.talib_core_version, "0.7.1");
    assert_eq!(matrix.python_reference_version, "0.8.0");
    assert_eq!(
        matrix.surfaces.profile_catalog.status, "registered",
        "catalog status must describe registration only"
    );
    assert_eq!(
        matrix.surfaces.dispatcher_smoke.status, "executable_smoke",
        "dispatcher status must describe smoke execution only"
    );
    assert_eq!(
        matrix.surfaces.numeric_reference.status, "fixed_golden",
        "numeric status must describe fixed references only"
    );
    assert_eq!(
        TALIB_PROFILE_CATALOG_NAMES.len(),
        matrix.surfaces.profile_catalog.expected_count
    );
    assert_eq!(
        dispatcher_names().len(),
        matrix.surfaces.dispatcher_smoke.expected_count
    );
    assert_eq!(
        matrix.surfaces.numeric_reference.indicators.len(),
        matrix.surfaces.numeric_reference.expected_count
    );
    let catalog = TALIB_PROFILE_CATALOG_NAMES
        .iter()
        .copied()
        .collect::<Vec<_>>();
    let unique = catalog.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(
        unique.len(),
        catalog.len(),
        "TA-Lib catalog contains duplicates"
    );
    assert!(
        catalog.windows(2).all(|pair| pair[0] < pair[1]),
        "TA-Lib catalog must remain sorted"
    );
}

pub fn dispatcher_names() -> BTreeSet<String> {
    let mut names = TALIB_PROFILE_CATALOG_NAMES
        .iter()
        .map(|name| (*name).to_string())
        .collect::<BTreeSet<_>>();
    for spec in finkit::operation::builtin_operation_registry().iter() {
        if finkit_ffi_common::talib_profile_supported(&spec.name) {
            names.insert(spec.name.clone());
        }
    }
    names
}
