//! Shared JSON contract for bounded Factor streaming and checkpoints.

use finkit::factor_system::{FactorCatalog, FactorStreamCheckpoint};
use finkit::factors::{builtin_factor_registry, FactorEngine};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Version of the language-neutral bounded Factor streaming contract.
pub const FACTOR_STREAM_CONTRACT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Deserialize)]
struct FactorStreamRequest {
    schema_version: Option<u16>,
    targets: Vec<String>,
    /// New rows to append. Every series must have the same length.
    inputs: BTreeMap<String, Vec<f64>>,
    #[serde(default)]
    checkpoint: Option<FactorStreamCheckpointRequest>,
}

#[derive(Debug, Deserialize)]
struct FactorStreamCheckpointRequest {
    semantic_identity: Vec<String>,
    row_count: usize,
    inputs: BTreeMap<String, Vec<f64>>,
    outputs: BTreeMap<String, Vec<Option<f64>>>,
}

/// Execute finite-lookback Factor rows and return the next portable checkpoint.
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

    let registry = builtin_factor_registry();
    let catalog = FactorCatalog::from_registry(registry.clone());
    let target_refs = request
        .targets
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let plan = catalog
        .compile(&target_refs)
        .map_err(|error| error.to_string())?;
    let engine = FactorEngine::new(registry);
    let mut stream = plan.stream(engine).map_err(|error| error.to_string())?;

    if let Some(checkpoint) = request.checkpoint {
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

    let mut emitted = request
        .targets
        .iter()
        .map(|target| (target.clone(), Vec::with_capacity(input_rows)))
        .collect::<BTreeMap<_, _>>();
    for row_index in 0..input_rows {
        let row = plan
            .required_raw_inputs()
            .iter()
            .map(|name| {
                request
                    .inputs
                    .get(name)
                    .map(|values| values[row_index])
                    .ok_or_else(|| format!("factor stream missing input {name}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let values = stream
            .push_values(&row)
            .map_err(|error| error.to_string())?;
        for target in &request.targets {
            let value = values
                .get(target)
                .ok_or_else(|| format!("factor stream did not produce target {target}"))?;
            emitted
                .get_mut(target)
                .expect("emitted target was initialized")
                .push(*value);
        }
    }

    let checkpoint = stream.checkpoint();
    let checkpoint_json = json!({
        "semantic_identity": checkpoint.semantic_identity(),
        "row_count": checkpoint.rows(),
        "inputs": checkpoint.inputs(),
        "outputs": nullable_map(checkpoint.outputs()),
    });
    serde_json::to_string(&json!({
        "schema_version": FACTOR_STREAM_CONTRACT_SCHEMA_VERSION,
        "shape": if request.targets.len() > 1 { "multi_series" } else { "series" },
        "primary": (request.targets.len() == 1).then(|| request.targets[0].clone()),
        "targets": request.targets,
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
            "targets":["momentum_5"],
            "inputs":{"close":[10.0,11.0,12.0,15.0,14.0,16.0]}
        }"#;
        let first_payload: Value =
            serde_json::from_str(&evaluate_factor_stream_json(first).unwrap()).unwrap();
        assert_eq!(first_payload["schema_version"], 1);
        assert_eq!(first_payload["execution"]["mode"], "streaming");
        assert_eq!(first_payload["execution"]["total_rows"], 6);
        assert_eq!(
            first_payload["values"]["momentum_5"]
                .as_array()
                .unwrap()
                .len(),
            6
        );

        let second = json!({
            "schema_version": 1,
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
    fn stream_contract_rejects_unaligned_rows() {
        let request = r#"{
            "schema_version":1,
            "targets":["momentum_5"],
            "inputs":{"close":[1.0,2.0],"other":[1.0]}
        }"#;
        assert!(evaluate_factor_stream_json(request)
            .unwrap_err()
            .contains("equal lengths"));
    }
}
