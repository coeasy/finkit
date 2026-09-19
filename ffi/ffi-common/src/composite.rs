//! Shared JSON contract for dependency-aware Composite execution.

use crate::shared_runtime::with_unified_engine;
use crate::stream_contract::require_scope_and_revision;
use finkit::composite::{CompositeDefinition, CompositeExpr, CompositeOp};
use finkit::factors::FactorContext;
use finkit::operation::OperationRequest;
use finkit::unified_runtime::{DirtyRange, RuntimeExecutionMode};
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
    /// Explicit cache/provenance namespace, usually `SYMBOL@TIMEFRAME`.
    scope: Option<String>,
    /// Caller-owned monotonic revision. A changed input must advance it.
    data_revision: Option<u64>,
    #[serde(default)]
    previous: Option<std::collections::BTreeMap<String, Vec<f64>>>,
    #[serde(default)]
    dirty_range: Option<[usize; 2]>,
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
    let (scope, data_revision) = require_scope_and_revision(
        request.scope.as_deref(),
        request.data_revision,
        "composite contract",
    )?;
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
    let definition_count = request.definitions.len();
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
    let range_lookback = with_unified_engine(|unified| {
        unified
            .composite_engine_mut()
            .compile_cached(&definitions, &output_refs)
            .map(|plan| plan.range_lookback())
            .map_err(|error| error.to_string())
    })?;
    if request.previous.is_some() != request.dirty_range.is_some() {
        return Err("composite range execution requires both previous and dirty_range".to_string());
    }
    let borrowed = context.as_borrowed();
    let runtime = match (request.previous.as_ref(), request.dirty_range) {
        (Some(previous), Some([start, end])) => {
            let (result, trace) = with_unified_engine(|unified| {
                unified
                    .execute_composite_range(
                        &definitions,
                        &output_refs,
                        &borrowed,
                        previous,
                        DirtyRange::new(start, end),
                    )
                    .map_err(|error| error.to_string())
            })?;
            finkit::unified_runtime::RuntimeExecution {
                output: result.values,
                trace,
            }
        }
        (None, None) => {
            // Route complete graph execution through the canonical operation
            // façade so the public Composite contract shares the same
            // revision-scoped cache and dispatcher as direct Runtime users.
            let result = with_unified_engine(|unified| {
                unified
                    .execute(OperationRequest::Composite {
                        definitions: &definitions,
                        outputs: &output_refs,
                        context: &borrowed,
                        data_revision: Some(data_revision),
                        cache_scope: Some(scope),
                    })
                    .map_err(|error| error.to_string())
            })?;
            finkit::unified_runtime::RuntimeExecution {
                output: result.values,
                trace: finkit::unified_runtime::RuntimeExecutionTrace {
                    mode: RuntimeExecutionMode::Full,
                    rows: context.len(),
                    executed_nodes: definition_count,
                    recomputed_rows: context.len(),
                },
            }
        }
        _ => unreachable!("range pair was validated"),
    };
    let trace = runtime.trace;
    let values = runtime.output;
    let serialized = values
        .into_iter()
        .map(|(name, series)| (name, nullable_series(&series)))
        .collect::<serde_json::Map<_, _>>();
    serde_json::to_string(&json!({
        "schema_version": COMPOSITE_CONTRACT_SCHEMA_VERSION,
        "shape": if output_names.len() > 1 { "multi_series" } else { "series" },
        "primary": (output_names.len() == 1).then(|| output_names[0].clone()),
        "range_lookback": range_lookback,
        "scope": scope,
        "data_revision": data_revision,
        "execution": execution_envelope(trace.mode),
        "values": Value::Object(serialized),
    }))
    .map_err(|error| error.to_string())
}

fn execution_envelope(mode: RuntimeExecutionMode) -> Value {
    match mode {
        RuntimeExecutionMode::Full => json!({"mode": "full"}),
        RuntimeExecutionMode::Range {
            input_dirty,
            affected,
            recompute,
        } => json!({
            "mode": "range",
            "input_dirty": [input_dirty.start, input_dirty.end],
            "affected": [affected.start, affected.end],
            "recompute": [recompute.start, recompute.end],
        }),
    }
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
            "scope":"TEST@1d",
            "data_revision":0,
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
        assert_eq!(payload["execution"]["mode"], "full");
        assert_eq!(payload["scope"], "TEST@1d");
        assert_eq!(payload["data_revision"], 0);
        assert_eq!(payload["range_lookback"], 2);
    }

    #[test]
    fn batch_contract_preserves_explicit_cache_scope_and_revision() {
        let request = r#"{
            "schema_version":1,
            "scope":"BBB@5m",
            "data_revision":9,
            "inputs":{"close":[1.0,2.0,3.0,4.0]},
            "definitions":[{"name":"sma3","function":"sma","inputs":["close"],"params":[3]}],
            "outputs":["sma3"]
        }"#;
        let payload: Value =
            serde_json::from_str(&evaluate_composite_json(request).unwrap()).unwrap();
        assert_eq!(payload["scope"], "BBB@5m");
        assert_eq!(payload["data_revision"], 9);
        assert_eq!(payload["execution"]["mode"], "full");
    }

    #[test]
    fn contract_executes_finite_composite_dirty_range() {
        let request = r#"{
            "schema_version":1,
            "scope":"TEST@1d",
            "data_revision":0,
            "inputs":{"close":[1.0,2.0,3.0,4.0,10.0,6.0]},
            "definitions":[{"name":"sma3","function":"sma","inputs":["close"],"params":[3]}],
            "outputs":["sma3"],
            "previous":{"sma3":[0.0,0.0,2.0,3.0,5.666666666666667,6.666666666666667]},
            "dirty_range":[4,5]
        }"#;
        let payload: Value =
            serde_json::from_str(&evaluate_composite_json(request).unwrap()).unwrap();
        assert_eq!(payload["execution"]["mode"], "range");
        assert_eq!(payload["execution"]["affected"], json!([4, 6]));
        assert_eq!(payload["values"]["sma3"][4], 17.0 / 3.0);
        assert_eq!(payload["values"]["sma3"][5], 20.0 / 3.0);
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
