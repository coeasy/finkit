//! Executable regression runner for the domestic formula corpus.
//!
//! The corpus metadata is intentionally separate from the small numeric
//! contract in `tests/contracts`: this runner verifies that every checked-in
//! source formula can pass through the selected dialect, execute against the
//! shared OHLCV fixture, and produce each declared named output at the input
//! length. It is a structural/runtime gate, not a claim of terminal-wide
//! numeric equivalence.

use finkit::formula::{FormulaContext, FormulaDialect, FormulaEngine};
use ndarray::Array1;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

fn read_fixture(path: &Path) -> HashMap<String, Vec<f64>> {
    let text = fs::read_to_string(path).unwrap_or_else(|error| panic!("{path:?}: {error}"));
    let mut header = None;
    let mut columns = HashMap::<String, Vec<f64>>::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split(',').map(str::trim).collect::<Vec<_>>();
        if header.is_none() {
            for field in &fields {
                columns.insert((*field).to_string(), Vec::new());
            }
            header = Some(fields);
            continue;
        }
        let names = header.as_ref().unwrap();
        assert_eq!(
            fields.len(),
            names.len(),
            "CSV row width mismatch in {path:?}"
        );
        for (name, value) in names.iter().zip(fields.iter()) {
            if let Ok(value) = value.parse::<f64>() {
                columns.get_mut(*name).unwrap().push(value);
            }
        }
    }
    assert!(!columns.is_empty(), "CSV fixture has no columns: {path:?}");
    columns
}

/// Map a corpus `platform` tag onto a dialect the engine actually implements.
///
/// `dzh` (大智慧) is **not** an implemented dialect — there is no DaZhiHui
/// variant of [`FormulaDialect`]. The two corpus cases tagged `dzh` are
/// therefore evaluated under TongDaXin semantics, and passing here means
/// "correct for TongDaXin", not "correct for DaZhiHui". The mapping is written
/// out rather than folded into the `tdx` arm so that gap stays visible instead
/// of looking like deliberate support.
fn dialect_for(platform: &str) -> FormulaDialect {
    match platform.to_ascii_lowercase().as_str() {
        "tdx" | "cross" => FormulaDialect::TongDaXin,
        // No DaZhiHui dialect exists; run these under the closest implemented
        // profile. See the function documentation.
        "dzh" => FormulaDialect::TongDaXin,
        "ths" => FormulaDialect::TongHuaShun,
        "eastmoney" | "em" => FormulaDialect::EastMoney,
        "pine" | "tradingview" => FormulaDialect::Pine,
        other => panic!("unsupported formula corpus platform: {other}"),
    }
}

fn context(columns: &HashMap<String, Vec<f64>>) -> FormulaContext {
    let values = |name: &str| {
        Array1::from_vec(
            columns
                .get(name)
                .unwrap_or_else(|| panic!("required OHLCV column missing: {name}"))
                .clone(),
        )
    };
    FormulaContext::new(
        values("open"),
        values("high"),
        values("low"),
        values("close"),
        values("volume"),
        None,
    )
}

fn corpus_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/formula_corpus");
    let mut files = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("{root:?}: {error}"))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

#[test]
fn checked_in_formula_corpus_executes_through_declared_dialects() {
    let files = corpus_files();
    assert!(!files.is_empty(), "formula corpus is empty");
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut executed = 0usize;

    for path in files {
        let source = fs::read_to_string(&path).unwrap();
        let case: Value = serde_json::from_str(&source)
            .unwrap_or_else(|error| panic!("invalid corpus JSON {path:?}: {error}"));
        let input = &case["input"];
        let data_file = input["data_file"]
            .as_str()
            .unwrap_or_else(|| panic!("{} is missing input.data_file", path.display()));
        let columns = read_fixture(&repo_root.join(data_file));
        let mut context = context(&columns);
        let platform = case["platform"].as_str().unwrap();
        let dialect = dialect_for(platform);
        let mut engine = FormulaEngine::new();
        let result = engine
            .eval_multi_with_dialect(
                case["source_formula"].as_str().unwrap(),
                dialect,
                &mut context,
            )
            .unwrap_or_else(|error| panic!("{} ({platform}) failed: {error}", path.display()));
        let expected_columns = case["expected_output_columns"].as_array().unwrap();
        for expected in expected_columns {
            let name = expected.as_str().unwrap();
            let values = result
                .outputs
                .get(name)
                .or_else(|| context.variables.get(name))
                .unwrap_or_else(|| panic!("{} did not emit output {name}", path.display()));
            assert_eq!(
                values.len(),
                context.data_len,
                "{} output {name} length mismatch",
                path.display()
            );
        }
        executed += 1;
    }

    assert!(executed >= 10, "unexpectedly small corpus: {executed}");
}

/// The 31 TA-Lib 0.7/0.8 names M0-1 registered in the formula surface.
///
/// Kept here as a literal rather than imported from the crate: the point of the
/// gate below is to compare the *corpus* against a fixed historical list, so
/// reading the list from the same registry the corpus is validated against
/// would make the check circular.
const TALIB_081_NAMES: &[&str] = &[
    "AC",
    "ACCBANDS",
    "ADR",
    "AO",
    "AROON",
    "CMOU",
    "COPPOCK",
    "CVI",
    "EFI",
    "ER",
    "ERI",
    "FOSC",
    "FRACTAL",
    "HA",
    "KC",
    "MAMA",
    "MARKETFI",
    "MASSI",
    "NVI",
    "PERCENTRANK",
    "PVI",
    "PVO",
    "PVT",
    "QSTICK",
    "RVI",
    "RVOL",
    "SMI",
    "VHF",
    "VORTEX",
    "WAD",
    "ZLEMA",
];

/// Collect every identifier immediately followed by `(` in a formula source.
///
/// A plain `source.contains("AC(")` would also fire on `MACD(`-shaped names, so
/// the leading identifier characters are walked back explicitly.
fn call_targets(source: &str) -> BTreeSet<&str> {
    let bytes = source.as_bytes();
    let mut targets = BTreeSet::new();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'(' {
            continue;
        }
        let mut start = index;
        while start > 0 {
            let previous = bytes[start - 1];
            if previous.is_ascii_alphanumeric() || previous == b'_' {
                start -= 1;
            } else {
                break;
            }
        }
        if start < index {
            // `start`/`index` only ever move across ASCII bytes, so this is a
            // valid char boundary even if the formula contains CJK variable
            // names elsewhere.
            targets.insert(&source[start..index]);
        }
    }
    targets
}

/// Corpus entries that call at least one of [`TALIB_081_NAMES`].
fn talib_081_corpus_entries() -> BTreeSet<String> {
    corpus_files()
        .into_iter()
        .filter(|path| {
            let case: Value = serde_json::from_str(&fs::read_to_string(path).unwrap())
                .unwrap_or_else(|error| panic!("invalid corpus JSON {}: {error}", path.display()));
            let source = case["source_formula"].as_str().unwrap_or_default();
            call_targets(source)
                .iter()
                .any(|target| TALIB_081_NAMES.contains(target))
        })
        .map(|path| {
            path.file_name()
                .expect("corpus file name")
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

/// File names listed in the README's `### TA-Lib 0.7/0.8` table.
fn readme_talib_081_entries() -> BTreeSet<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/formula_corpus/README.md");
    let text =
        fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let mut entries = BTreeSet::new();
    let mut inside = false;
    for line in text.lines() {
        if line.starts_with("### TA-Lib") {
            inside = true;
            continue;
        }
        if inside && line.starts_with("## ") {
            break;
        }
        if !inside {
            continue;
        }
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("| `") else {
            continue;
        };
        let cell = rest.split('|').next().unwrap_or_default().trim();
        let name = cell.trim_end_matches('`');
        if Path::new(name)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        {
            entries.insert(name.to_string());
        }
    }
    assert!(
        !entries.is_empty(),
        "README's TA-Lib section has no table rows; the gate below would be vacuous"
    );
    entries
}

/// M0-1 criterion: the corpus must cover the new functions, and the README's
/// table must describe exactly the entries that do.
///
/// Both directions are checked, because each catches a different mistake:
/// a corpus entry added without a README row (undocumented coverage), and a
/// README row claiming coverage no entry provides (a promise with no gate).
/// The completeness half is what caught `AROON`: it was registered and
/// golden-green, but no corpus entry ever called it.
#[test]
fn talib_081_corpus_coverage_is_complete_and_documented() {
    let entries = talib_081_corpus_entries();
    let documented = readme_talib_081_entries();
    assert_eq!(
        entries,
        documented,
        "README's TA-Lib table and the corpus disagree \
         (undocumented: {:?}, documented-but-absent: {:?})",
        entries.difference(&documented).collect::<Vec<_>>(),
        documented.difference(&entries).collect::<Vec<_>>()
    );

    let mut covered: BTreeSet<&'static str> = BTreeSet::new();
    for path in corpus_files() {
        let case: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let source = case["source_formula"].as_str().unwrap_or_default();
        for target in call_targets(source) {
            // Re-key onto the `'static` literal so `covered` outlives `case`.
            if let Some(name) = TALIB_081_NAMES.iter().find(|name| **name == target) {
                covered.insert(name);
            }
        }
    }
    let missing = TALIB_081_NAMES
        .iter()
        .copied()
        .filter(|name| !covered.contains(name))
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "TA-Lib 0.7/0.8 functions with no corpus coverage: {missing:?}"
    );
    assert_eq!(
        covered.len(),
        TALIB_081_NAMES.len(),
        "every name in TALIB_081_NAMES must be covered"
    );
}
