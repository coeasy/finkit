//! Shared JSON contract for bounded Factor streaming and checkpoints.

use crate::shared_runtime::with_unified_engine;
use crate::stream_contract::{
    annotate_checkpoint, require_scope_and_revision, validate_checkpoint_metadata,
};
use finkit::factor_system::{FactorStreamCheckpoint, StatefulFactorCheckpoint};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Version of the language-neutral bounded Factor streaming contract.
pub const FACTOR_STREAM_CONTRACT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Deserialize)]
struct FactorStreamRequest {
    schema_version: Option<u16>,
    #[serde(default)]
    mode: Option<String>,
    targets: Vec<String>,
    /// New rows to append. Every series must have the same length.
    inputs: BTreeMap<String, Vec<f64>>,
    scope: Option<String>,
    data_revision: Option<u64>,
    #[serde(default)]
    checkpoint: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct FactorStreamCheckpointRequest {
    semantic_identity: Vec<String>,
    row_count: usize,
    inputs: BTreeMap<String, Vec<f64>>,
    outputs: BTreeMap<String, Vec<Option<f64>>>,
}

/// Execute bounded or stateful Factor rows and return the next portable
/// checkpoint. The default `mode` is `bounded`; `stateful` selects the O(1)
/// state DAG for supported built-in factors.
///
/// The request's `inputs` are appended to the optional checkpoint. The
/// checkpoint contains only the proven lookback window plus the semantic plan
/// identity, so another binding can resume without sharing Rust memory.
pub fn evaluate_factor_stream_json(request: &str) -> Result<String, String> {
    let request: FactorStreamRequest =
        serde_json::from_str(request).map_err(|error| error.to_string())?;
    let version = request
        .schema_version
        .ok_or_else(|| "factor stream schema_version is required".to_string())?;
    if version != FACTOR_STREAM_CONTRACT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported factor stream schema_version: {version}"
        ));
    }
    let (scope, data_revision) = require_scope_and_revision(
        request.scope.as_deref(),
        request.data_revision,
        "factor stream contract",
    )?;
    if request.targets.is_empty() {
        return Err("factor stream targets must not be empty".to_string());
    }
    if request.inputs.is_empty() {
        return Err("factor stream inputs must not be empty".to_string());
    }
    let input_rows = aligned_input_rows(&request.inputs)?;
    if input_rows == 0 {
        return Err("factor stream inputs must contain at least one row".to_string());
    }
    if request
        .targets
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != request.targets.len()
    {
        return Err("factor stream targets must be unique".to_string());
    }

    let target_refs = request
        .targets
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let (plan, stream_engine) = with_unified_engine(|unified| {
        unified
            .prepare_factor_stream(&target_refs)
            .map_err(|error| error.to_string())
    })?;

    let mode = request
        .mode
        .as_deref()
        .unwrap_or("bounded")
        .to_ascii_lowercase();
    if let Some(checkpoint) = request.checkpoint.as_ref() {
        validate_checkpoint_metadata(checkpoint, scope, data_revision)?;
    }
    if mode == "stateful" {
        let mut stream = plan.stateful_stream().map_err(|error| error.to_string())?;
        if let Some(checkpoint) = request.checkpoint {
            let checkpoint = StatefulFactorCheckpoint::from_json(
                &serde_json::to_string(&checkpoint).map_err(|error| error.to_string())?,
            )?;
            stream
                .restore(&checkpoint)
                .map_err(|error| error.to_string())?;
        }
        let mut emitted = BTreeMap::new();
        stream
            .push_batch_into(&request.inputs, &mut emitted)
            .map_err(|error| error.to_string())?;
        let checkpoint_json: Value = serde_json::from_str(
            &stream
                .checkpoint()
                .to_json()
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        let checkpoint_json = annotate_checkpoint(checkpoint_json, scope, data_revision)?;
        return serde_json::to_string(&json!({
            "schema_version": FACTOR_STREAM_CONTRACT_SCHEMA_VERSION,
            "shape": if request.targets.len() > 1 { "multi_series" } else { "series" },
            "primary": (request.targets.len() == 1).then(|| request.targets[0].clone()),
            "targets": request.targets,
            "scope": scope,
            "data_revision": data_revision,
            "semantic_identity": plan.semantic_identity(),
            "range_lookback": Value::Null,
            "execution": {
                "mode": "stateful_streaming",
                "input_rows": input_rows,
                "total_rows": stream.rows(),
            },
            "values": nullable_map(&emitted),
            "checkpoint": checkpoint_json,
        }))
        .map_err(|error| error.to_string());
    }
    if mode != "bounded" {
        return Err(format!("unsupported factor stream mode: {mode}"));
    }

    let mut stream = plan
        .stream(stream_engine)
        .map_err(|error| error.to_string())?;

    if let Some(checkpoint) = request.checkpoint {
        let checkpoint: FactorStreamCheckpointRequest =
            serde_json::from_value(checkpoint).map_err(|error| error.to_string())?;
        let outputs = checkpoint
            .outputs
            .into_iter()
            .map(|(name, values)| {
                decode_nullable_series(&name, &values).map(|values| (name, values))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let checkpoint = FactorStreamCheckpoint::from_parts(
            checkpoint.semantic_identity,
            checkpoint.row_count,
            checkpoint.inputs,
            outputs,
        );
        stream
            .restore(&checkpoint)
            .map_err(|error| error.to_string())?;
    }

    let emitted = stream
        .push_batch(&request.inputs)
        .map_err(|error| error.to_string())?;

    let checkpoint = stream.checkpoint();
    let checkpoint_json = annotate_checkpoint(
        json!({
            "semantic_identity": checkpoint.semantic_identity(),
            "row_count": checkpoint.rows(),
            "inputs": checkpoint.inputs(),
            "outputs": nullable_map(checkpoint.outputs()),
        }),
        scope,
        data_revision,
    )?;
    serde_json::to_string(&json!({
        "schema_version": FACTOR_STREAM_CONTRACT_SCHEMA_VERSION,
        "shape": if request.targets.len() > 1 { "multi_series" } else { "series" },
        "primary": (request.targets.len() == 1).then(|| request.targets[0].clone()),
        "targets": request.targets,
        "scope": scope,
        "data_revision": data_revision,
        "semantic_identity": plan.semantic_identity(),
        "range_lookback": plan.range_lookback(),
        "execution": {
            "mode": "streaming",
            "input_rows": input_rows,
            "total_rows": stream.rows(),
        },
        "values": nullable_map(&emitted),
        "checkpoint": checkpoint_json,
    }))
    .map_err(|error| error.to_string())
}

fn aligned_input_rows(inputs: &BTreeMap<String, Vec<f64>>) -> Result<usize, String> {
    let rows = inputs.values().next().map_or(0, Vec::len);
    if inputs.values().any(|values| values.len() != rows) {
        return Err("factor stream input series must have equal lengths".to_string());
    }
    Ok(rows)
}

fn decode_nullable_series(name: &str, values: &[Option<f64>]) -> Result<Vec<f64>, String> {
    values
        .iter()
        .map(|value| match value {
            Some(value) if value.is_finite() => Ok(*value),
            Some(_) => Err(format!(
                "factor stream checkpoint output is not finite: {name}"
            )),
            None => Ok(f64::NAN),
        })
        .collect()
}

fn nullable_map(values: &BTreeMap<String, Vec<f64>>) -> Value {
    Value::Object(
        values
            .iter()
            .map(|(name, values)| (name.clone(), nullable_series(values)))
            .collect(),
    )
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
    fn stream_contract_returns_checkpoint_and_resumes() {
        let first = r#"{
            "schema_version":1,
            "scope":"TEST@1d",
            "data_revision":0,
            "targets":["momentum_5"],
            "inputs":{"close":[10.0,11.0,12.0,15.0,14.0,16.0]}
        }"#;
        let first_payload: Value =
            serde_json::from_str(&evaluate_factor_stream_json(first).unwrap()).unwrap();
        assert_eq!(first_payload["schema_version"], 1);
        assert_eq!(first_payload["execution"]["mode"], "streaming");
        assert_eq!(first_payload["execution"]["total_rows"], 6);
        assert_eq!(first_payload["checkpoint"]["scope"], "TEST@1d");
        assert_eq!(first_payload["checkpoint"]["data_revision"], 0);
        assert_eq!(
            first_payload["values"]["momentum_5"]
                .as_array()
                .unwrap()
                .len(),
            6
        );

        let second = json!({
            "schema_version": 1,
            "scope":"TEST@1d",
            "data_revision":0,
            "targets": ["momentum_5"],
            "inputs": {"close": [17.0, 18.0]},
            "checkpoint": first_payload["checkpoint"].clone(),
        });
        let second_payload: Value =
            serde_json::from_str(&evaluate_factor_stream_json(&second.to_string()).unwrap())
                .unwrap();
        assert_eq!(second_payload["execution"]["total_rows"], 8);
        assert!(second_payload["values"]["momentum_5"][0].is_number());
        assert_eq!(second_payload["checkpoint"]["row_count"], 8);
    }

    #[test]
    fn stream_contract_rejects_checkpoint_scope_mismatch() {
        let first = serde_json::json!({
            "schema_version": 1,
            "targets": ["momentum_5"],
            "scope": "AAA@1d",
            "data_revision": 3,
            "inputs": {"close": [10.0,11.0,12.0,15.0,14.0,16.0]}
        });
        let first: Value =
            serde_json::from_str(&evaluate_factor_stream_json(&first.to_string()).unwrap())
                .unwrap();
        let second = serde_json::json!({
            "schema_version": 1,
            "targets": ["momentum_5"],
            "scope": "BBB@1d",
            "data_revision": 3,
            "inputs": {"close": [17.0]},
            "checkpoint": first["checkpoint"].clone()
        });
        assert!(evaluate_factor_stream_json(&second.to_string())
            .unwrap_err()
            .contains("scope mismatch"));

        let second = serde_json::json!({
            "schema_version": 1,
            "targets": ["momentum_5"],
            "scope": "AAA@1d",
            "data_revision": 4,
            "inputs": {"close": [17.0]},
            "checkpoint": first["checkpoint"].clone()
        });
        assert!(evaluate_factor_stream_json(&second.to_string())
            .unwrap_err()
            .contains("data_revision mismatch"));
    }

    #[test]
    fn stream_contract_requires_provenance_metadata() {
        let request = r#"{
            "schema_version":1,
            "targets":["momentum_5"],
            "inputs":{"close":[1.0,2.0,3.0,4.0,5.0,6.0]}
        }"#;
        assert_eq!(
            evaluate_factor_stream_json(request).unwrap_err(),
            "factor stream contract scope is required"
        );
    }

    #[test]
    fn stream_contract_rejects_unaligned_rows() {
        let request = r#"{
            "schema_version":1,
            "scope":"TEST@1d",
            "data_revision":0,
            "targets":["momentum_5"],
            "inputs":{"close":[1.0,2.0],"other":[1.0]}
        }"#;
        assert!(evaluate_factor_stream_json(request)
            .unwrap_err()
            .contains("equal lengths"));
    }

    #[test]
    fn stateful_stream_contract_executes_builtin_factor_and_resumes() {
        let first = r#"{
            "schema_version":1,
            "mode":"stateful",
            "scope":"TEST@1d",
            "data_revision":0,
            "targets":["momentum_5"],
            "inputs":{"close":[10.0,11.0,12.0,15.0,14.0,16.0]}
        }"#;
        let first_payload: Value =
            serde_json::from_str(&evaluate_factor_stream_json(first).unwrap()).unwrap();
        assert_eq!(first_payload["execution"]["mode"], "stateful_streaming");
        assert_eq!(first_payload["execution"]["total_rows"], 6);
        assert!(first_payload["checkpoint"]["nodes"].is_array());

        let second = json!({
            "schema_version": 1,
            "mode": "stateful",
            "scope":"TEST@1d",
            "data_revision":0,
            "targets": ["momentum_5"],
            "inputs": {"close": [17.0, 18.0]},
            "checkpoint": first_payload["checkpoint"].clone(),
        });
        let second_payload: Value =
            serde_json::from_str(&evaluate_factor_stream_json(&second.to_string()).unwrap())
                .unwrap();
        assert_eq!(second_payload["execution"]["total_rows"], 8);
        assert!(second_payload["values"]["momentum_5"][0].is_number());
    }

    #[test]
    fn stateful_stream_contract_rejects_unknown_mode() {
        let request = r#"{
            "schema_version":1,
            "mode":"full_recompute",
            "scope":"TEST@1d",
            "data_revision":0,
            "targets":["momentum_5"],
            "inputs":{"close":[1.0]}
        }"#;
        assert!(evaluate_factor_stream_json(request)
            .unwrap_err()
            .contains("unsupported factor stream mode"));
    }
}
