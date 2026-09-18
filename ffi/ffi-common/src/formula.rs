//! Versioned formula execution contract shared by language bindings.
//!
//! Bindings should use this low-frequency control-plane envelope when they
//! need dialect selection and named outputs. Numeric hot paths can continue
//! to use typed indicator functions or zero-copy formula APIs.

use finkit::formula::{
    inspect_formula_compatibility, FormulaContext, FormulaDialect, FormulaEngine, FormulaTerminal,
};
use ndarray::Array1;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Version of the cross-language formula result envelope.
pub const FORMULA_CONTRACT_SCHEMA_VERSION: u16 = 1;

/// Version of the language-neutral formula compatibility report envelope.
pub const FORMULA_COMPATIBILITY_SCHEMA_VERSION: u16 = 1;

/// Inspect a formula through the shared cross-language compatibility contract.
///
/// The report is intentionally emitted as one JSON shape for Rust, Python,
/// Go, Java, .NET, C, C++ and Node.  Binding-specific APIs should only decode
/// or forward this payload; they must not recreate terminal semantics.
pub fn formula_compatibility_report_json(source: &str, terminal: &str) -> Result<String, String> {
    let terminal = FormulaTerminal::from_str(terminal)
        .ok_or_else(|| format!("unknown formula terminal: {terminal}"))?;
    let report = inspect_formula_compatibility(source, terminal)?;
    let mut payload = serde_json::to_value(report).map_err(|error| error.to_string())?;
    let object = payload
        .as_object_mut()
        .ok_or_else(|| "formula compatibility report is not a JSON object".to_string())?;
    object.insert(
        "schema_version".to_string(),
        serde_json::json!(FORMULA_COMPATIBILITY_SCHEMA_VERSION),
    );
    serde_json::to_string(&payload).map_err(|error| error.to_string())
}

/// Evaluate one formula and serialize the language-neutral result envelope.
pub fn evaluate_formula_json(
    source: &str,
    dialect: &str,
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
) -> Result<String, String> {
    let expected = open.len();
    if high.len() != expected
        || low.len() != expected
        || close.len() != expected
        || volume.len() != expected
    {
        return Err("all OHLCV series must have the same length".to_string());
    }
    let dialect = FormulaDialect::from_str(dialect)
        .ok_or_else(|| format!("unsupported formula dialect: {dialect}"))?;
    let mut context = FormulaContext::new(
        Array1::from_vec(open.to_vec()),
        Array1::from_vec(high.to_vec()),
        Array1::from_vec(low.to_vec()),
        Array1::from_vec(close.to_vec()),
        Array1::from_vec(volume.to_vec()),
        None,
    );
    let mut engine = FormulaEngine::new();
    let result = engine
        .eval_multi_with_dialect(source, dialect, &mut context)
        .map_err(|error| error.to_string())?;
    let mut values = result
        .outputs
        .into_iter()
        .map(|(name, value)| (name, value.to_vec()))
        .collect::<BTreeMap<_, _>>();
    let final_value = result.final_value.to_vec();
    values.insert("__PRIMARY__".to_string(), final_value);

    let serialized_values = values
        .into_iter()
        .map(|(name, series)| (name, nullable_series(&series)))
        .collect::<BTreeMap<_, _>>();
    serde_json::to_string(&json!({
        "schema_version": FORMULA_CONTRACT_SCHEMA_VERSION,
        "dialect": dialect.as_str(),
        "primary": "__PRIMARY__",
        "values": serialized_values,
    }))
    .map_err(|error| error.to_string())
}

fn nullable_series(values: &[f64]) -> Value {
    Value::Array(
        values
            .iter()
            .map(|value| {
                if value.is_finite() {
                    json!(value)
                } else {
                    Value::Null
                }
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_contains_named_values_and_null_warmup() {
        let values = [1.0, f64::NAN, 3.0];
        let payload = evaluate_formula_json(
            "CLOSE", "alpha_ta", &values, &values, &values, &values, &values,
        )
        .unwrap();
        let json: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["primary"], "__PRIMARY__");
        assert_eq!(json["values"]["__PRIMARY__"][1], Value::Null);
    }

    #[test]
    fn contract_preserves_pine_dialect_identity() {
        let values = [1.0, 2.0, 3.0];
        let payload = evaluate_formula_json(
            "//@version=5\nindicator(\"x\")\nr = ta.sma(close, 2)\nplot(r)",
            "pine",
            &values,
            &values,
            &values,
            &values,
            &values,
        )
        .unwrap();
        let json: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(json["dialect"], "pine");
    }

    #[test]
    fn contract_normalizes_domestic_source_before_execution() {
        let values = [1.0, 2.0, 3.0];
        let payload = evaluate_formula_json(
            "\u{feff}X:=CLOSE;\r\nX",
            "tdx",
            &values,
            &values,
            &values,
            &values,
            &values,
        )
        .unwrap();
        let json: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(json["dialect"], "tdx");
        assert_eq!(
            json["values"]["__PRIMARY__"],
            serde_json::json!([1.0, 2.0, 3.0])
        );
    }

    #[test]
    fn compatibility_contract_contains_capability_matrix_and_schema() {
        let payload =
            formula_compatibility_report_json("X:=MA(CLOSE,5); X + SECURITY(CLOSE, 'WEEK')", "tdx")
                .unwrap();
        let json: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(json["schema_version"], FORMULA_COMPATIBILITY_SCHEMA_VERSION);
        assert_eq!(json["terminal"], "tdx");
        assert!(json["capabilities"].as_array().unwrap().iter().any(|item| {
            item["name"] == "cross_timeframe" && item["status"] == "host_required"
        }));
    }
}
