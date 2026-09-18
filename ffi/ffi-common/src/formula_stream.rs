//! Shared JSON contract for stateful Formula streaming.

use finkit::formula::{FormulaDialect, FormulaStatefulCheckpoint, FormulaStatefulStream};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Version of the language-neutral stateful Formula stream contract.
pub const FORMULA_STREAM_CONTRACT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Deserialize)]
struct FormulaStreamRequest {
    schema_version: Option<u16>,
    source: String,
    dialect: String,
    inputs: BTreeMap<String, Vec<f64>>,
    mode: Option<String>,
    checkpoint: Option<Value>,
}

/// Execute a verified stateful Formula subset and return the next checkpoint.
pub fn evaluate_formula_stream_json(request: &str) -> Result<String, String> {
    let request: FormulaStreamRequest = serde_json::from_str(request)
        .map_err(|error| format!("invalid formula stream request: {error}"))?;
    let version = request
        .schema_version
        .ok_or_else(|| "formula stream schema_version is required".to_string())?;
    if version != FORMULA_STREAM_CONTRACT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported formula stream schema_version: {version}"
        ));
    }
    let mode = request.mode.as_deref().unwrap_or("stateful");
    if mode != "stateful" {
        return Err(format!("unsupported formula stream mode: {mode}"));
    }
    if request.source.trim().is_empty() {
        return Err("formula stream source must not be empty".to_string());
    }
    if request.inputs.is_empty() {
        return Err("formula stream inputs must not be empty".to_string());
    }
    let dialect = FormulaDialect::from_str(&request.dialect)
        .ok_or_else(|| format!("unsupported formula dialect: {}", request.dialect))?;
    let mut stream = FormulaStatefulStream::from_source(&request.source, dialect)
        .map_err(|error| error.to_string())?;
    if let Some(checkpoint) = request.checkpoint {
        let checkpoint = match checkpoint {
            Value::String(checkpoint) => checkpoint,
            checkpoint => checkpoint.to_string(),
        };
        let checkpoint = FormulaStatefulCheckpoint::from_json(&checkpoint)?;
        stream
            .restore(&checkpoint)
            .map_err(|error| error.to_string())?;
    }
    let mut values = Vec::new();
    stream
        .push_batch_into(&request.inputs, &mut values)
        .map_err(|error| error.to_string())?;
    let checkpoint_json = stream
        .checkpoint()
        .to_json()
        .map_err(|error| error.to_string())?;
    let checkpoint: Value =
        serde_json::from_str(&checkpoint_json).map_err(|error| error.to_string())?;
    let nullable = values
        .into_iter()
        .map(|value| {
            if value.is_finite() {
                json!(value)
            } else {
                Value::Null
            }
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&json!({
        "schema_version": FORMULA_STREAM_CONTRACT_SCHEMA_VERSION,
        "dialect": dialect.as_str(),
        "primary": "__PRIMARY__",
        "values": {"__PRIMARY__": nullable},
        "checkpoint": checkpoint,
        "execution": {
            "mode": "stateful_streaming",
            "rows": stream.rows(),
            "total_rows": stream.rows(),
            "signature": stream.signature(),
            "required_inputs": stream.required_inputs().collect::<Vec<_>>(),
        }
    }))
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stateful_formula_stream_executes_and_resumes() {
        let first = serde_json::json!({
            "schema_version": 1,
            "source": "EMA(CLOSE, 3)",
            "dialect": "tdx",
            "inputs": {"close": [10.0, 11.0, 12.0, 15.0]}
        });
        let first: Value =
            serde_json::from_str(&evaluate_formula_stream_json(&first.to_string()).unwrap())
                .unwrap();
        assert_eq!(
            first["values"]["__PRIMARY__"],
            serde_json::json!([null, null, 11.0, 13.0])
        );
        assert_eq!(first["execution"]["mode"], "stateful_streaming");

        let second = serde_json::json!({
            "schema_version": 1,
            "source": "EMA(CLOSE, 3)",
            "dialect": "tdx",
            "inputs": {"close": [14.0, 16.0]},
            "checkpoint": first["checkpoint"].clone()
        });
        let second: Value =
            serde_json::from_str(&evaluate_formula_stream_json(&second.to_string()).unwrap())
                .unwrap();
        assert_eq!(
            second["values"]["__PRIMARY__"],
            serde_json::json!([13.5, 14.75])
        );
    }

    #[test]
    fn stateful_formula_stream_rejects_unsupported_mode() {
        let request = serde_json::json!({
            "schema_version": 1,
            "source": "CLOSE",
            "dialect": "tdx",
            "mode": "bounded",
            "inputs": {"close": [1.0]}
        });
        assert!(evaluate_formula_stream_json(&request.to_string())
            .unwrap_err()
            .contains("unsupported formula stream mode"));
    }
}
