//! Golden parity for the TA-Lib 0.7/0.8 formula surface.
//!
//! The kernels behind these 31 names already had golden vectors in
//! `tests/golden/talib`, but they were unreachable from the formula DSL: a user
//! writing `ZLEMA(CLOSE, 20)` got "unknown function". This test pins the thing
//! the numeric contract could not see -- that the *formula entry point*
//! produces the same numbers as the library entry point.
//!
//! Every value here is compared against the checked-in TA-Lib 0.8.0 reference
//! on the shared `ashare` fixture, so a regression in either the wrapper or the
//! kernel fails this test.

use ndarray::Array1;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use finkit::formula::functions::get_builtin_functions;
use finkit::formula::types::FormulaContext;

const FIXTURE: &str = "tests/fixtures/ashare_sh_index_250d.csv";
const DATASET: &str = "ashare";

#[derive(Debug, Deserialize)]
struct GoldenFile {
    metadata: GoldenMetadata,
    results: HashMap<String, DatasetResult>,
}

#[derive(Debug, Deserialize)]
struct GoldenMetadata {
    indicator: String,
    talib_version: String,
    /// Parsed so the golden file's parameter block is part of the deserialized
    /// contract; the values themselves are already pinned by `CASES`.
    #[allow(dead_code)]
    parameters: HashMap<String, serde_json::Value>,
    outputs: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DatasetResult {
    #[allow(dead_code)]
    fixture_path: String,
    outputs: HashMap<String, Vec<Option<f64>>>,
}

struct Case {
    name: &'static str,
    inputs: &'static [&'static str],
    params: &'static [f64],
    output: &'static str,
    /// Absolute tolerance. Most of these match to 1e-10.
    tol: f64,
    /// Relative tolerance, applied as `tol + rel * |expected|` -- the same
    /// shape `tests/contracts/talib_numeric_contract_v1.json` uses.
    ///
    /// Non-zero only for the volume-scaled cumulative families, whose values
    /// reach 1e8 on the shared fixture. There the disagreement is float
    /// representation (~6e-16 relative), not an algorithmic difference, and the
    /// numeric contract already gates those with `rtol = 1e-8`.
    rel: f64,
}

/// Per-family tolerance policy, mirroring `core/tests/golden_talib_tests.rs`.
///
/// The default is 1e-10 (the plan's acceptance bar). A looser value is only
/// used where the *same* kernel already needed it in the library-level golden
/// test, so the formula surface is never held to a different standard than the
/// library surface.
const CASES: &[Case] = &[
    Case {
        name: "AC",
        inputs: &["high", "low"],
        params: &[5.0, 34.0, 5.0],
        output: "real",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "ACCBANDS",
        inputs: &["high", "low", "close"],
        params: &[20.0],
        output: "upperband",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "ADR",
        inputs: &["high", "low"],
        params: &[14.0],
        output: "real",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "AO",
        inputs: &["high", "low"],
        params: &[5.0, 34.0],
        output: "ao",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "AROON",
        inputs: &["high", "low"],
        params: &[14.0],
        output: "aroonup",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "CMOU",
        inputs: &["close"],
        params: &[14.0],
        output: "real",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "COPPOCK",
        inputs: &["close"],
        params: &[10.0, 14.0, 11.0],
        output: "coppock",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "CVI",
        inputs: &["high", "low"],
        params: &[10.0, 10.0],
        output: "real",
        tol: 1e-10,
        rel: 0.0,
    },
    // EFI/PVT are volume-scaled cumulative sums: values reach ~1e8 on the
    // shared fixture, so a purely absolute bar compares float representation
    // noise. The numeric contract gates these with rtol = 1e-8; mirror it.
    Case {
        name: "EFI",
        inputs: &["close", "volume"],
        params: &[13.0],
        output: "real",
        tol: 1e-10,
        rel: 1e-8,
    },
    Case {
        name: "ER",
        inputs: &["close"],
        params: &[10.0],
        output: "er",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "ERI",
        inputs: &["high", "low", "close"],
        params: &[13.0],
        output: "bullpower",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "FOSC",
        inputs: &["close"],
        params: &[5.0],
        output: "real",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "FRACTAL",
        inputs: &["high", "low"],
        params: &[2.0, 2.0],
        output: "swinghigh",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "HA",
        inputs: &["open", "high", "low", "close"],
        params: &[],
        output: "haclose",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "KC",
        inputs: &["high", "low", "close"],
        params: &[20.0, 10.0, 2.0],
        output: "upperband",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "MAMA",
        inputs: &["close"],
        params: &[0.5, 0.05],
        output: "mama",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "MARKETFI",
        inputs: &["high", "low", "volume"],
        params: &[],
        output: "real",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "MASSI",
        inputs: &["high", "low"],
        params: &[9.0, 25.0],
        output: "real",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "NVI",
        inputs: &["close", "volume"],
        params: &[],
        output: "nvi",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "PERCENTRANK",
        inputs: &["close"],
        params: &[100.0],
        output: "percentrank",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "PVI",
        inputs: &["close", "volume"],
        params: &[],
        output: "pvi",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "PVO",
        inputs: &["volume"],
        params: &[12.0, 26.0, 1.0],
        output: "real",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "PVT",
        inputs: &["close", "volume"],
        params: &[],
        output: "pvt",
        tol: 1e-10,
        rel: 1e-8,
    },
    Case {
        name: "QSTICK",
        inputs: &["open", "close"],
        params: &[10.0],
        output: "real",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "RVI",
        inputs: &["close"],
        params: &[14.0, 10.0],
        output: "real",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "RVOL",
        inputs: &["volume"],
        params: &[20.0],
        output: "real",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "SMI",
        inputs: &["high", "low", "close"],
        params: &[13.0, 2.0, 25.0, 9.0],
        output: "smi",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "VHF",
        inputs: &["close"],
        params: &[28.0],
        output: "real",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "VORTEX",
        inputs: &["high", "low", "close"],
        params: &[14.0],
        output: "plusvi",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "WAD",
        inputs: &["high", "low", "close"],
        params: &[],
        output: "real",
        tol: 1e-10,
        rel: 0.0,
    },
    Case {
        name: "ZLEMA",
        inputs: &["close"],
        params: &[30.0],
        output: "zlema",
        tol: 1e-10,
        rel: 0.0,
    },
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

struct Frame {
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
}

fn read_fixture() -> Frame {
    let content = fs::read_to_string(workspace_root().join(FIXTURE)).expect("read fixture");
    let mut lines = content
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty());
    let header = lines.next().expect("fixture header");
    let columns: HashMap<String, usize> = header
        .split(',')
        .enumerate()
        .map(|(index, name)| (name.trim().to_lowercase(), index))
        .collect();
    let mut frame = Frame {
        open: Vec::new(),
        high: Vec::new(),
        low: Vec::new(),
        close: Vec::new(),
        volume: Vec::new(),
    };
    for line in lines {
        let parts: Vec<&str> = line.split(',').collect();
        let cell =
            |name: &str| -> f64 { parts[columns[name]].trim().parse().expect("numeric cell") };
        frame.open.push(cell("open"));
        frame.high.push(cell("high"));
        frame.low.push(cell("low"));
        frame.close.push(cell("close"));
        frame.volume.push(cell("volume"));
    }
    frame
}

fn column<'a>(frame: &'a Frame, name: &str) -> &'a [f64] {
    match name {
        "open" => &frame.open,
        "high" => &frame.high,
        "low" => &frame.low,
        "close" => &frame.close,
        "volume" => &frame.volume,
        other => panic!("unknown fixture column {other}"),
    }
}

fn load_golden(indicator: &str, output: &str) -> Vec<Option<f64>> {
    let path = workspace_root()
        .join("tests/golden/talib")
        .join(format!("{}.json", indicator.to_ascii_lowercase()));
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing golden file {}", path.display()));
    let golden: GoldenFile = serde_json::from_str(&raw).expect("parse golden");
    // A golden vector that does not identify itself is not evidence. These
    // assertions are what make the file self-describing: without them, a
    // renamed or regenerated file would still be compared against, silently
    // pinning this surface to whatever numbers happened to be in it.
    assert_eq!(
        golden.metadata.indicator,
        indicator.to_ascii_uppercase(),
        "golden {} does not identify itself as {indicator}",
        path.display()
    );
    assert_eq!(
        golden.metadata.talib_version,
        "0.8.0",
        "golden {} reference version drifted",
        path.display()
    );
    assert!(
        golden
            .metadata
            .outputs
            .iter()
            .any(|name| name.as_str() == output),
        "golden {} does not declare output `{output}` (has {:?})",
        path.display(),
        golden.metadata.outputs
    );
    let dataset = golden
        .results
        .get(DATASET)
        .unwrap_or_else(|| panic!("golden {indicator} has no `{DATASET}` dataset"));
    dataset
        .outputs
        .get(output)
        .unwrap_or_else(|| panic!("golden {indicator} has no output `{output}`"))
        .clone()
}

#[test]
fn talib_081_formula_surface_matches_golden() {
    let frame = read_fixture();
    let len = frame.close.len();
    let array = |name: &str| Array1::from_vec(column(&frame, name).to_vec());
    let ctx = FormulaContext::new(
        array("open"),
        array("high"),
        array("low"),
        array("close"),
        array("volume"),
        None,
    );
    let functions = get_builtin_functions();

    let mut failures = Vec::new();
    for case in CASES {
        let function = functions
            .get(case.name)
            .unwrap_or_else(|| panic!("{} is not registered in the formula table", case.name));
        let mut args: Vec<Array1<f64>> = case
            .inputs
            .iter()
            .map(|name| Array1::from_vec(column(&frame, name).to_vec()))
            .collect();
        for value in case.params {
            args.push(Array1::from_elem(1, *value));
        }

        let expected = load_golden(case.name, case.output);
        let actual = match function(&ctx, &args) {
            Ok(values) => values,
            Err(error) => {
                failures.push(format!("{}: evaluation failed: {error}", case.name));
                continue;
            }
        };
        assert_eq!(actual.len(), len, "{}: length mismatch", case.name);

        let mut max_abs = 0.0_f64;
        let mut compared = 0usize;
        let mut first_bad: Option<(usize, f64, f64, f64)> = None;
        for index in 0..len.min(expected.len()) {
            let expected_value = match expected[index] {
                Some(value) => value,
                None => continue,
            };
            let actual_value = actual[index];
            if !actual_value.is_finite() {
                if first_bad.is_none() {
                    first_bad = Some((index, expected_value, actual_value, f64::INFINITY));
                }
                compared += 1;
                continue;
            }
            let diff = (expected_value - actual_value).abs();
            let limit = case.tol + case.rel * expected_value.abs();
            max_abs = max_abs.max(diff);
            compared += 1;
            if diff > limit && first_bad.is_none() {
                first_bad = Some((index, expected_value, actual_value, diff));
            }
        }
        if let Some((index, expected_value, actual_value, diff)) = first_bad {
            failures.push(format!(
                "{}: first mismatch at {index}: expected {expected_value}, got {actual_value}, diff {diff} (tol {}, max_abs {max_abs}, compared {compared})",
                case.name, case.tol
            ));
        } else {
            println!(
                "{}: ok (compared {compared}, max_abs {max_abs:e})",
                case.name
            );
        }
    }

    assert!(
        failures.is_empty(),
        "formula surface parity failures:\n{}",
        failures.join("\n")
    );
}
