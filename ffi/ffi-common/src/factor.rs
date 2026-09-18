//! Shared JSON contract for compiled built-in Factor execution.

use finkit::factor_system::FactorCatalog;
use finkit::factors::{builtin_factor_registry, FactorContext, FactorEngine};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Version of the cross-language Factor result envelope.
pub const FACTOR_CONTRACT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Deserialize)]
struct FactorRequest {
    schema_version: Option<u16>,
    targets: Vec<String>,
    inputs: BTreeMap<String, Vec<f64>>,
}

/// Execute built-in factors through the compiled Factor plan contract.
/// Requests must include `schema_version: 1`; omitted versions are rejected
/// so language bindings cannot silently fall back to an older request shape.
///
/// The initial cross-language catalog intentionally exposes only the stable
/// built-in factors. User-defined Rust closures remain available through the
/// typed Rust API and must not be silently serialized as if they were portable.
pub fn evaluate_factor_json(request: &str) -> Result<String, String> {
    let request: FactorRequest =
        serde_json::from_str(request).map_err(|error| error.to_string())?;
    let version = request
        .schema_version
        .ok_or_else(|| "factor contract schema_version is required".to_string())?;
    if version != FACTOR_CONTRACT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported factor contract schema_version: {version}"
        ));
    }
    if request.targets.is_empty() {
        return Err("factor targets must not be empty".to_string());
    }
    let unique_targets = request
        .targets
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    if unique_targets.len() != request.targets.len() {
        return Err("factor targets must be unique".to_string());
    }
    if request.inputs.is_empty() {
        return Err("factor inputs must not be empty".to_string());
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
    let mut context = FactorContext::new();
    for (name, values) in request.inputs {
        context
            .insert(name, values)
            .map_err(|error| error.to_string())?;
    }
    let engine = FactorEngine::new(registry);
    let values = plan
        .execute_borrowed(&engine, &context.as_borrowed())
        .map_err(|error| error.to_string())?;
    let mut serialized = serde_json::Map::new();
    for target in &request.targets {
        let values = values
            .get(target)
            .ok_or_else(|| format!("compiled factor plan did not produce {target}"))?;
        serialized.insert(target.clone(), nullable_series(values));
    }
    serde_json::to_string(&json!({
        "schema_version": FACTOR_CONTRACT_SCHEMA_VERSION,
        "shape": if request.targets.len() > 1 { "multi_series" } else { "series" },
        "primary": (request.targets.len() == 1).then(|| request.targets[0].clone()),
        "targets": request.targets,
        "semantic_identity": plan.semantic_identity(),
        "range_lookback": plan.range_lookback(),
        "values": Value::Object(serialized),
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
    fn contract_executes_compiled_builtin_factor() {
        let request = r#"{
            "schema_version":1,
            "targets":["momentum_5"],
            "inputs":{"close":[1.0,2.0,3.0,4.0,5.0,6.0]}
        }"#;
        let payload: Value = serde_json::from_str(&evaluate_factor_json(request).unwrap()).unwrap();
        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["shape"], "series");
        assert_eq!(payload["primary"], "momentum_5");
        assert_eq!(payload["values"]["momentum_5"][0], Value::Null);
        assert_eq!(payload["values"]["momentum_5"][5], 5.0);
        assert!(payload["semantic_identity"].as_array().is_some());
    }

    #[test]
    fn contract_requires_version_and_rejects_duplicate_targets() {
        let missing_version = r#"{
            "targets":["momentum_5"],
            "inputs":{"close":[1.0,2.0,3.0,4.0,5.0,6.0]}
        }"#;
        assert_eq!(
            evaluate_factor_json(missing_version).unwrap_err(),
            "factor contract schema_version is required"
        );

        let duplicate_targets = r#"{
            "schema_version":1,
            "targets":["momentum_5","momentum_5"],
            "inputs":{"close":[1.0,2.0,3.0,4.0,5.0,6.0]}
        }"#;
        assert_eq!(
            evaluate_factor_json(duplicate_targets).unwrap_err(),
            "factor targets must be unique"
        );
    }
}
