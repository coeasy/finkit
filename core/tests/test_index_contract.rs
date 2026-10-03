//! Contract gate for `core/tests/TEST_INDEX.md`.
//!
//! The index opens by stating its own contract:
//!
//! > **文件清单必须与 `ls core/tests/*.rs` 一致** —— 该目录只增不减地漂移过
//! > （曾列出 3 个不存在的文件、漏掉 23 个实际文件），所以每次增删 target 都要同步这里。
//!
//! That contract was aspirational. Nothing checked it, and the file drifted
//! again: by the time this gate was written it was missing eight test targets
//! (`alpha158_parity`, `formula_dialect_coverage`, `formula_registry_signature`,
//! `formula_talib_081_surface`, `multi_period_resonance_smoke`,
//! `rolling_volatility_consistency`, `runtime_convergence`,
//! `worldquant101_library`) — which is a documentation problem and a
//! discoverability problem at the same time, because the index is how a reader
//! finds out which target covers what.
//!
//! The gate is deliberately **bidirectional**, in the same spirit as the §18
//! allowlists:
//!
//! * every `core/tests/*.rs` target must be documented — so a new test cannot
//!   land silently undiscoverable;
//! * every documented `.rs` must exist — so deleting or renaming a target
//!   fails here instead of leaving a dead pointer in the index.
//!
//! Both directions are asserted in one test, so neither can be satisfied by
//! weakening the other.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
}

/// Top-level integration-test targets, exactly the set `ls core/tests/*.rs`
/// reports. Subdirectories hold helpers (`common/`) and data (`fixtures/`,
/// `golden/`), not targets.
fn actual_targets() -> BTreeSet<String> {
    let mut targets = BTreeSet::new();
    let entries = std::fs::read_dir(tests_dir()).expect("core/tests is readable");
    for entry in entries {
        let entry = entry.expect("directory entries are readable");
        if !entry.file_type().expect("entry type is readable").is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(stem) = name.strip_suffix(".rs") {
            targets.insert(stem.to_string());
        }
    }
    targets
}

/// Every `.rs` path the index mentions, as written in the markdown.
///
/// Entries look like:
///
/// ```text
/// - `name.rs` - description
/// - `dir/name.rs` - description
/// ```
///
/// The path is preserved so a `common/` helper is checked against its real
/// location rather than being compared to the top-level target set.
fn documented_paths(index: &str) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for (start, _) in index.match_indices('`') {
        let rest = &index[start + 1..];
        let Some(end) = rest.find('`') else {
            continue;
        };
        let candidate = &rest[..end];
        // Case-insensitive on purpose: a `Foo.RS` entry is a typo that this
        // gate should report as "not a real file", not one it silently skips.
        let is_rust_source = Path::new(candidate)
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"));
        if !is_rust_source {
            continue;
        }
        // A glob such as `core/tests/*.rs` is prose quoting the shell command
        // the contract is stated in, not an index entry. An entry is always a
        // path *relative to* `core/tests/`, so anything rooted at `core/` is a
        // cross-reference too.
        if candidate.contains('*') || candidate.starts_with("core/") {
            continue;
        }
        if candidate.contains(char::is_whitespace) {
            continue;
        }
        paths.insert(candidate.to_string());
    }
    paths
}

#[test]
fn test_index_documents_every_target_and_points_only_at_real_files() {
    let index_path = tests_dir().join("TEST_INDEX.md");
    let index = std::fs::read_to_string(&index_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", index_path.display()));
    assert!(
        index.len() > 1024,
        "TEST_INDEX.md is suspiciously small ({} bytes) — it is the only index of \
         core/tests, so an empty one is worse than a stale one",
        index.len()
    );

    let actual = actual_targets();
    let documented = documented_paths(&index);
    let documented_top_level: BTreeSet<String> = documented
        .iter()
        .filter(|path| !path.contains('/'))
        .map(|path| path.trim_end_matches(".rs").to_string())
        .collect();

    // Direction 1: every target is documented.
    let undocumented: Vec<&String> = actual.difference(&documented_top_level).collect();
    assert!(
        undocumented.is_empty(),
        "core/tests/TEST_INDEX.md does not list {} target(s): {:?}. \
         Every integration test must be discoverable from the index — add it \
         to the matching section.",
        undocumented.len(),
        undocumented
    );

    // Direction 2: every documented path exists where the index says it does.
    let mut missing: Vec<&String> = Vec::new();
    for path in &documented {
        if !tests_dir().join(path).is_file() {
            missing.push(path);
        }
    }
    assert!(
        missing.is_empty(),
        "core/tests/TEST_INDEX.md points at {} path(s) that do not exist: {:?}. \
         A dead pointer is invisible until someone follows it.",
        missing.len(),
        missing
    );

    // The gate must not be vacuous: if the parser silently stopped matching,
    // both sets would be empty and the two assertions above would pass.
    assert!(
        actual.len() > 50,
        "expected a populated test directory, saw {} target(s)",
        actual.len()
    );
    assert!(
        documented.len() > 50,
        "expected a populated index, saw {} documented path(s)",
        documented.len()
    );
}
