//! Shared JSON contract for bounded Composite streaming and checkpoints.

use finkit::composite::{
    CompositeDefinition, CompositeEngine, CompositeExpr, CompositeOp, CompositeStreamCheckpoint,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

/// Version of the language-neutral bounded Composite streaming contract.
pub const COMPOSITE_STREAM_CONTRACT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Deserialize)]
struct CompositeStreamRequest {
    schema_version: Option<u16>,
    inputs: BTreeMap<String, Vec<f64>>,
    definitions: Vec<CompositeDefinitionRequest>,
    outputs: Option<Vec<String>>,
    #[serde(default)]
    checkpoint: Option<CompositeStreamCheckpointRequest>,
}

#[derive(Debug, Deserialize)]
struct CompositeDefinitionRequest {
    name: String,
    function: String,
    #[serde(default)]
    inputs: Vec<String>,
    #[serde(default)]
    params: Vec<f64>,
}

#[derive(Debug, Deserialize)]
struct CompositeStreamCheckpointRequest {
    signature: u64,
    row_count: usize,
    inputs: BTreeMap<String, Vec<f64>>,
    outputs: BTreeMap<String, Vec<Option<f64>>>,
}

/// Execute finite-lookback Composite rows and return the next portable checkpoint.
pub fn evaluate_composite_stream_json(request: &str) -> Result<String, String> {
    let request: CompositeStreamRequest =
        serde_json::from_str(request).map_err(|error| error.to_string())?;
    let version = request
        .schema_version
        .ok_or_else(|| "composite stream schema_version is required".to_string())?;
    if version != COMPOSITE_STREAM_CONTRACT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported composite stream schema_version: {version}"
        ));
    }
    if request.inputs.is_empty() {
        return Err("composite stream inputs must not be empty".to_string());
    }
    let input_rows = aligned_input_rows(&request.inputs)?;
    if input_rows == 0 {
        return Err("composite stream inputs must contain at least one row".to_string());
    }
    let names = request
        .definitions
        .iter()
        .map(|definition| definition.name.clone())
        .collect::<BTreeSet<_>>();
    if names.len() != request.definitions.len() {
        return Err("composite stream definition names must be unique".to_string());
    }
    let definitions = request
        .definitions
        .into_iter()
        .map(|definition| {
            let inputs = definition
                .inputs
                .iter()
                .map(|input| input_expression(input, &names))
                .collect::<Result<Vec<_>, _>>()?;
            let function = definition.function.to_ascii_lowercase();
            let expression = match function.as_str() {
                "add" => CompositeExpr::Op {
                    op: CompositeOp::Add,
                    inputs,
                },
                "sub" => CompositeExpr::Op {
                    op: CompositeOp::Sub,
                    inputs,
                },
                "mul" => CompositeExpr::Op {
                    op: CompositeOp::Mul,
                    inputs,
                },
                "div" => CompositeExpr::Op {
                    op: CompositeOp::Div,
                    inputs,
                },
                "min" => CompositeExpr::Op {
                    op: CompositeOp::Min,
                    inputs,
                },
                "max" => CompositeExpr::Op {
                    op: CompositeOp::Max,
                    inputs,
                },
                "weighted_average" | "weightedaverage" => {
                    CompositeExpr::call("weighted_average", inputs, definition.params)
                }
                _ => CompositeExpr::call(definition.function, inputs, definition.params),
            };
            Ok(CompositeDefinition::new(definition.name, expression))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let output_names = request.outputs.unwrap_or_else(|| {
        definitions
            .iter()
            .map(|definition| definition.name.clone())
            .collect()
    });
    if output_names.is_empty() {
        return Err("composite stream outputs must not be empty".to_string());
    }
    let output_refs = output_names.iter().map(String::as_str).collect::<Vec<_>>();
    let engine = CompositeEngine::new();
    let plan = engine
        .compile(&definitions, &output_refs)
        .map_err(|error| error.to_string())?;
    let mut stream = plan.stream(engine).map_err(|error| error.to_string())?;

    if let Some(checkpoint) = request.checkpoint {
        let outputs = checkpoint
            .outputs
            .into_iter()
            .map(|(name, values)| {
                decode_nullable_series(&name, &values).map(|values| (name, values))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let checkpoint = CompositeStreamCheckpoint::from_parts(
            checkpoint.signature,
            checkpoint.row_count,
            checkpoint.inputs,
            outputs,
        );
        stream
            .restore(&checkpoint)
            .map_err(|error| error.to_string())?;
    }

    let mut emitted = output_names
        .iter()
        .map(|output| (output.clone(), Vec::with_capacity(input_rows)))
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
                    .ok_or_else(|| format!("composite stream missing input {name}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let values = stream
            .push_values(&row)
            .map_err(|error| error.to_string())?;
        for output in &output_names {
            let value = values
                .get(output)
                .ok_or_else(|| format!("composite stream did not produce output {output}"))?;
            emitted
                .get_mut(output)
                .expect("emitted output was initialized")
                .push(*value);
        }
    }

    let checkpoint = stream.checkpoint();
    let checkpoint_json = json!({
        "signature": checkpoint.signature(),
        "row_count": checkpoint.rows(),
        "inputs": checkpoint.inputs(),
        "outputs": nullable_map(checkpoint.outputs()),
    });
    serde_json::to_string(&json!({
        "schema_version": COMPOSITE_STREAM_CONTRACT_SCHEMA_VERSION,
        "shape": if output_names.len() > 1 { "multi_series" } else { "series" },
        "primary": (output_names.len() == 1).then(|| output_names[0].clone()),
        "outputs": output_names,
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
        return Err("composite stream input series must have equal lengths".to_string());
    }
    Ok(rows)
}

fn input_expression(
    input: &str,
    definition_names: &BTreeSet<String>,
) -> Result<CompositeExpr, String> {
    if let Some(value) = input.strip_prefix("const:") {
        let value = value
            .parse::<f64>()
            .map_err(|_| format!("invalid composite constant: {input}"))?;
        return Ok(CompositeExpr::Constant(value));
    }
    if definition_names.contains(input) {
        Ok(CompositeExpr::reference(input))
    } else {
        Ok(CompositeExpr::series(input))
    }
}

fn decode_nullable_series(name: &str, values: &[Option<f64>]) -> Result<Vec<f64>, String> {
    values
        .iter()
        .map(|value| match value {
            Some(value) if value.is_finite() => Ok(*value),
            Some(_) => Err(format!(
                "composite stream checkpoint output is not finite: {name}"
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
            "inputs":{"close":[10.0,11.0,12.0,15.0]},
            "definitions":[{"name":"sma3","function":"sma","inputs":["close"],"params":[3]}],
            "outputs":["sma3"]
        }"#;
        let first_payload: Value =
            serde_json::from_str(&evaluate_composite_stream_json(first).unwrap()).unwrap();
        assert_eq!(first_payload["schema_version"], 1);
        assert_eq!(first_payload["execution"]["mode"], "streaming");
        assert_eq!(first_payload["execution"]["total_rows"], 4);

        let second = json!({
            "schema_version": 1,
            "inputs": {"close": [14.0, 16.0]},
            "definitions":[{"name":"sma3","function":"sma","inputs":["close"],"params":[3]}],
            "outputs":["sma3"],
            "checkpoint": first_payload["checkpoint"].clone(),
        });
        let second_payload: Value =
            serde_json::from_str(&evaluate_composite_stream_json(&second.to_string()).unwrap())
                .unwrap();
        assert_eq!(second_payload["execution"]["total_rows"], 6);
        assert_eq!(second_payload["values"]["sma3"][1], 15.0);
        assert_eq!(second_payload["checkpoint"]["row_count"], 6);
    }

    #[test]
    fn stream_contract_rejects_recursive_graphs() {
        let request = r#"{
            "schema_version":1,
            "inputs":{"close":[1.0,2.0,3.0]},
            "definitions":[{"name":"ema","function":"ema","inputs":["close"],"params":[3]}],
            "outputs":["ema"]
        }"#;
        assert!(evaluate_composite_stream_json(request)
            .unwrap_err()
            .contains("not range-safe"));
    }
}
