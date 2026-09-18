//! Numeric semantic contract for the supported terminal adapters.
//!
//! Parser/map/eval pass rates are useful diagnostics, but they are not enough
//! to prove that TDX, 同花顺, 东方财富 and Pine produce the same intended
//! values. This fixture keeps a small deterministic vector in the repository
//! and executes every case through the dialect-aware multi-output entry point.

use finkit::formula::{FormulaContext, FormulaDialect, FormulaEngine};
use ndarray::Array1;
use serde_json::Value;

fn fixture() -> Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/contracts/formula_terminal_contract_v1.json"
    )))
    .expect("formula terminal contract must be valid JSON")
}

fn array(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .expect("contract series must be arrays")
        .iter()
        .map(|item| item.as_f64().expect("contract input must be numeric"))
        .collect()
}

fn expected(value: &Value) -> Vec<Option<f64>> {
    value
        .as_array()
        .expect("expected output must be an array")
        .iter()
        .map(|item| item.as_f64())
        .collect()
}

fn context(input: &Value) -> FormulaContext {
    FormulaContext::new(
        Array1::from_vec(array(&input["open"])),
        Array1::from_vec(array(&input["high"])),
        Array1::from_vec(array(&input["low"])),
        Array1::from_vec(array(&input["close"])),
        Array1::from_vec(array(&input["volume"])),
        None,
    )
}

#[test]
fn terminal_numeric_contract_is_executed_for_every_public_formula_profile() {
    let fixture = fixture();
    assert_eq!(fixture["schema_version"], 1);
    let input = &fixture["input"];

    for case in fixture["cases"].as_array().unwrap() {
        let dialect = FormulaDialect::from_str(case["terminal"].as_str().unwrap())
            .expect("contract terminal must be supported");
        let mut engine = FormulaEngine::new();
        let mut context = context(input);
        let result = engine
            .eval_multi_with_dialect(case["source"].as_str().unwrap(), dialect, &mut context)
            .unwrap_or_else(|error| panic!("{} failed: {error}", case["id"]));
        let actual = result.final_value.to_vec();
        let expected = expected(&case["expected_primary"]);
        assert_eq!(actual.len(), expected.len(), "{} length", case["id"]);
        for (index, (actual, expected)) in actual.iter().zip(expected.iter()).enumerate() {
            match expected {
                Some(expected) => assert!(
                    (actual - expected).abs() <= 1e-12,
                    "{} row {index}: expected {expected}, got {actual}",
                    case["id"]
                ),
                None => assert!(
                    actual.is_nan(),
                    "{} row {index}: expected warm-up NaN, got {actual}",
                    case["id"]
                ),
            }
        }
    }
}
