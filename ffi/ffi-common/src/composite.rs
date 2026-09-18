//! Shared JSON contract for dependency-aware Composite execution.

use finkit::composite::{CompositeDefinition, CompositeEngine, CompositeExpr, CompositeOp};
use finkit::factors::FactorContext;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;

/// Version of the cross-language Composite result envelope.
pub const COMPOSITE_CONTRACT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Deserialize)]
struct CompositeRequest {
    schema_version: Option<u16>,
    inputs: std::collections::BTreeMap<String, Vec<f64>>,
    definitions: Vec<CompositeDefinitionRequest>,
    outputs: Option<Vec<String>>,
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

/// Evaluate a Composite graph through the language-neutral JSON contract.
///
/// Request shape:
///
/// ```json
/// {
///   "schema_version": 1,
///   "inputs": {"close": [1, 2, 3]},
///   "definitions": [{"name":"sma3","function":"sma","inputs":["close"],"params":[3]}],
///   "outputs": ["sma3"]
/// }
/// ```
pub fn evaluate_composite_json(request: &str) -> Result<String, String> {
    let request: CompositeRequest =
        serde_json::from_str(request).map_err(|error| error.to_string())?;
    let version = request
        .schema_version
        .ok_or_else(|| "composite contract schema_version is required".to_string())?;
    if version != COMPOSITE_CONTRACT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported composite contract schema_version: {version}"
        ));
    }
    if request.inputs.is_empty() {
        return Err("composite inputs must not be empty".to_string());
    }

    let names = request
        .definitions
        .iter()
        .map(|definition| definition.name.clone())
        .collect::<BTreeSet<_>>();
    if names.len() != request.definitions.len() {
        return Err("composite definition names must be unique".to_string());
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
        return Err("composite outputs must not be empty".to_string());
    }
    let output_refs = output_names.iter().map(String::as_str).collect::<Vec<_>>();

    let mut context = FactorContext::new();
    for (name, values) in request.inputs {
        context
            .insert(name, values)
            .map_err(|error| error.to_string())?;
    }
    let engine = CompositeEngine::new();
    let plan = engine
        .compile(&definitions, &output_refs)
        .map_err(|error| error.to_string())?;
    let values = engine
        .evaluate_compiled(&plan, &context.as_borrowed())
        .map_err(|error| error.to_string())?;
    let serialized = values
        .into_iter()
        .map(|(name, series)| (name, nullable_series(&series)))
        .collect::<serde_json::Map<_, _>>();
    serde_json::to_string(&json!({
        "schema_version": COMPOSITE_CONTRACT_SCHEMA_VERSION,
        "shape": if output_names.len() > 1 { "multi_series" } else { "series" },
        "primary": (output_names.len() == 1).then(|| output_names[0].clone()),
        "values": Value::Object(serialized),
    }))
    .map_err(|error| error.to_string())
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
    fn contract_executes_composite_and_serializes_warmup_as_null() {
        let request = r#"{
            "schema_version":1,
            "inputs":{"close":[1.0,2.0,3.0,4.0]},
            "definitions":[{"name":"sma3","function":"sma","inputs":["close"],"params":[3]}],
            "outputs":["sma3"]
        }"#;
        let payload: Value =
            serde_json::from_str(&evaluate_composite_json(request).unwrap()).unwrap();
        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["shape"], "series");
        assert_eq!(payload["primary"], "sma3");
        assert_eq!(payload["values"]["sma3"][0], Value::Null);
        assert_eq!(payload["values"]["sma3"][3], 3.0);
    }

    #[test]
    fn contract_requires_version() {
        let request = r#"{
            "inputs":{"close":[1.0,2.0,3.0]},
            "definitions":[{"name":"sma3","function":"sma","inputs":["close"],"params":[3]}],
            "outputs":["sma3"]
        }"#;
        assert_eq!(
            evaluate_composite_json(request).unwrap_err(),
            "composite contract schema_version is required"
        );
    }
}
