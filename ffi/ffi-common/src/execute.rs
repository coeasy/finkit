//! Language-neutral execution contract for direct registered operations.
//!
//! This is the control-plane entry point shared by the official bindings. It
//! deliberately accepts JSON so every language can submit the same operation
//! name, ordered inputs, and numeric parameters while high-throughput callers
//! keep using typed indicator APIs.

use finkit::factors::FactorRegistry;
use finkit::formula::FormulaContext;
use finkit::operation::{OperationRequest, UnifiedOperationEngine, PRIMARY_OUTPUT_NAME};
use ndarray::Array1;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Version of the direct operation result envelope.
pub const OPERATION_RESULT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Deserialize)]
struct OperationRequestJson {
    operation: String,
    #[serde(default)]
    inputs: BTreeMap<String, Vec<f64>>,
    #[serde(default)]
    input_order: Vec<String>,
    #[serde(default)]
    params: Vec<f64>,
}

/// Execute one registered built-in operation and return its named result JSON.
///
/// This endpoint uses the canonical Core registry semantics (`semantic_profile`
/// is `core_registry`). TA-Lib-specific seed and warm-up variants remain
/// explicit typed operations until the variant registry is wired into this
/// control-plane request; same-spelled formula and TA-Lib functions must not be
/// silently conflated.
///
/// Request shape:
///
/// ```json
/// {
///   "operation": "SMA",
///   "input_order": ["CLOSE"],
///   "inputs": {"CLOSE": [1, 2, 3]},
///   "params": [2]
/// }
/// ```
///
/// The five canonical OHLCV fields are inferred from the supplied inputs only
/// to construct the formula context. The operation's declared input list is
/// still validated by the Core dispatcher, so missing required inputs fail
/// instead of silently producing a different calculation.
pub fn execute_operation_json(request: &str) -> String {
    match execute_operation(request) {
        Ok(value) => value.to_string(),
        Err((code, message)) => json!({
            "schema_version": OPERATION_RESULT_SCHEMA_VERSION,
            "error": {
                "code": code,
                "message": message,
            }
        })
        .to_string(),
    }
}

fn execute_operation(request: &str) -> Result<Value, (&'static str, String)> {
    let request: OperationRequestJson =
        serde_json::from_str(request).map_err(|error| ("invalid_json", error.to_string()))?;
    if request.operation.trim().is_empty() {
        return Err(("invalid_request", "operation must not be empty".to_string()));
    }
    if request.inputs.is_empty() {
        return Err((
            "invalid_request",
            "inputs must contain at least one series".to_string(),
        ));
    }

    let mut inputs = BTreeMap::new();
    let mut length = None;
    for (name, values) in request.inputs {
        let name = normalize_name(&name);
        if name.is_empty() {
            return Err((
                "invalid_request",
                "input names must not be empty".to_string(),
            ));
        }
        if let Some(expected) = length {
            if values.len() != expected {
                return Err((
                    "invalid_request",
                    "all input series must have the same length".to_string(),
                ));
            }
        } else {
            length = Some(values.len());
        }
        if inputs.insert(name.clone(), values).is_some() {
            return Err((
                "invalid_request",
                format!("duplicate input after normalization: {name}"),
            ));
        }
    }
    let length = length.unwrap_or(0);
    if length == 0 {
        return Err((
            "invalid_request",
            "input series must not be empty".to_string(),
        ));
    }

    let input_order = if request.input_order.is_empty() {
        inputs.keys().cloned().collect::<Vec<_>>()
    } else {
        request
            .input_order
            .into_iter()
            .map(|name| normalize_name(&name))
            .collect::<Vec<_>>()
    };
    if input_order.iter().any(String::is_empty) {
        return Err((
            "invalid_request",
            "input_order contains an empty name".to_string(),
        ));
    }
    if input_order.iter().any(|name| !inputs.contains_key(name)) {
        let missing = input_order
            .iter()
            .find(|name| !inputs.contains_key(*name))
            .expect("missing input exists");
        return Err((
            "invalid_request",
            format!("input_order references missing series: {missing}"),
        ));
    }

    let fallback = inputs
        .values()
        .next()
        .cloned()
        .ok_or(("invalid_request", "inputs must not be empty".to_string()))?;
    let close = inputs
        .get("CLOSE")
        .cloned()
        .unwrap_or_else(|| fallback.clone());
    let open = inputs.get("OPEN").cloned().unwrap_or_else(|| close.clone());
    let high = inputs.get("HIGH").cloned().unwrap_or_else(|| close.clone());
    let low = inputs.get("LOW").cloned().unwrap_or_else(|| close.clone());
    let volume = inputs
        .get("VOLUME")
        .cloned()
        .unwrap_or_else(|| vec![0.0; length]);
    let mut context = FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
        None,
    );
    for (name, values) in &inputs {
        context.set_variable(name.clone(), Array1::from_vec(values.clone()));
    }

    let input_refs = input_order.iter().map(String::as_str).collect::<Vec<_>>();
    let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
    let (canonical_name, operation_id, output_names) = {
        let spec = engine
            .catalog()
            .get(&request.operation)
            .ok_or_else(|| ("unknown_operation", request.operation.clone()))?;
        (spec.name.clone(), spec.id().0, spec.output_names.clone())
    };
    let result = engine
        .execute(OperationRequest::Indicator {
            name: &request.operation,
            inputs: &input_refs,
            params: &request.params,
            context: &mut context,
        })
        .map_err(|error| ("execution_error", error.to_string()))?;

    let single_output_name = output_names
        .first()
        .cloned()
        .unwrap_or_else(|| canonical_name.clone());
    let mut values = BTreeMap::new();
    for (name, series) in result.values {
        let name = if name == PRIMARY_OUTPUT_NAME {
            single_output_name.clone()
        } else {
            name
        };
        values.insert(name, nullable_series(&series));
    }
    let primary = result.primary.map(|name| {
        if name == PRIMARY_OUTPUT_NAME {
            single_output_name
        } else {
            name
        }
    });
    Ok(json!({
        "schema_version": OPERATION_RESULT_SCHEMA_VERSION,
        "semantic_profile": "core_registry",
        "operation": canonical_name,
        "operation_id": operation_id,
        "shape": value_shape_name(result.shape),
        "primary": primary,
        "values": values,
    }))
}

fn normalize_name(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

fn value_shape_name(shape: finkit::operation::ValueShape) -> &'static str {
    match shape {
        finkit::operation::ValueShape::Series => "series",
        finkit::operation::ValueShape::MultiSeries => "multi_series",
        finkit::operation::ValueShape::CrossSection => "cross_section",
        finkit::operation::ValueShape::Event => "event",
        finkit::operation::ValueShape::Report => "report",
    }
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
    fn executes_single_output_with_canonical_name_and_core_warmup() {
        let request = r#"{
            "operation":"SMA",
            "input_order":["CLOSE"],
            "inputs":{"CLOSE":[1.0,2.0,3.0]},
            "params":[2]
        }"#;
        let payload: Value = serde_json::from_str(&execute_operation_json(request)).unwrap();
        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["semantic_profile"], "core_registry");
        assert_eq!(payload["operation"], "SMA");
        assert_eq!(payload["primary"], "SMA");
        assert_eq!(payload["values"]["SMA"][0], 1.0);
        assert_eq!(payload["values"]["SMA"][2], 2.25);
    }

    #[test]
    fn preserves_named_multi_output_contract() {
        let request = r#"{
            "operation":"MACD",
            "input_order":["CLOSE"],
            "inputs":{"CLOSE":[1.0,2.0,3.0,4.0,5.0]},
            "params":[2,3,2]
        }"#;
        let payload: Value = serde_json::from_str(&execute_operation_json(request)).unwrap();
        assert_eq!(payload["shape"], "multi_series");
        assert!(payload["values"].get("MACD").is_some());
        assert!(payload["values"].get("MACD_SIGNAL").is_some());
        assert!(payload["values"].get("MACD_HIST").is_some());
    }

    #[test]
    fn rejects_mismatched_input_lengths_with_structured_error() {
        let request = r#"{
            "operation":"SMA",
            "inputs":{"CLOSE":[1.0,2.0],"OPEN":[1.0]},
            "params":[2]
        }"#;
        let payload: Value = serde_json::from_str(&execute_operation_json(request)).unwrap();
        assert_eq!(payload["error"]["code"], "invalid_request");
    }
}
