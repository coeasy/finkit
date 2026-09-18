//! Shared JSON contract for compiled built-in Factor execution.

use crate::shared_runtime::with_unified_engine;
use finkit::data_contract::CrossSectionView;
use finkit::factor_system::FactorCatalog;
use finkit::factors::{builtin_factor_registry, FactorContext, FactorEngine, FactorKind};
use finkit::operation::OperationRequest;
use finkit::unified_runtime::{DirtyRange, RuntimeExecutionMode};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Version of the cross-language Factor result envelope.
pub const FACTOR_CONTRACT_SCHEMA_VERSION: u16 = 1;

/// Version of the row-major cross-sectional Factor result envelope.
pub const FACTOR_CROSS_SECTIONAL_CONTRACT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Deserialize)]
struct FactorRequest {
    schema_version: Option<u16>,
    targets: Vec<String>,
    inputs: BTreeMap<String, Vec<f64>>,
    /// Explicit cache/provenance namespace, usually `SYMBOL@TIMEFRAME`.
    #[serde(default)]
    scope: String,
    /// Caller-owned monotonic revision. A changed input must advance it.
    #[serde(default)]
    data_revision: u64,
    #[serde(default)]
    previous: Option<BTreeMap<String, Vec<f64>>>,
    #[serde(default)]
    dirty_range: Option<[usize; 2]>,
}

#[derive(Debug, Deserialize)]
struct CrossSectionalFactorRequest {
    schema_version: Option<u16>,
    target: String,
    timestamps: Vec<i64>,
    symbols: Vec<String>,
    inputs: BTreeMap<String, Vec<Option<f64>>>,
    /// Explicit cache/provenance namespace for the panel.
    #[serde(default)]
    scope: String,
    /// Caller-owned revision for the panel snapshot.
    #[serde(default)]
    data_revision: u64,
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
    let engine = FactorEngine::new(registry.clone());
    if request.previous.is_some() != request.dirty_range.is_some() {
        return Err("factor range execution requires both previous and dirty_range".to_string());
    }
    let borrowed = context.as_borrowed();
    let runtime = match (request.previous.as_ref(), request.dirty_range) {
        (Some(previous), Some([start, end])) => plan
            .execute_range_borrowed(&engine, &borrowed, previous, DirtyRange::new(start, end))
            .map_err(|error| error.to_string())?,
        (None, None) => {
            // Complete batch execution goes through the same operation
            // façade used by the direct operation API. This keeps catalog
            // resolution, compiled-plan caching and result caching in one
            // Runtime path for every language binding.
            let mut output = BTreeMap::new();
            for target in &request.targets {
                let result = with_unified_engine(|unified| {
                    unified
                        .execute(OperationRequest::Factor {
                            name: target,
                            context: &borrowed,
                            data_revision: (!request.scope.is_empty())
                                .then_some(request.data_revision),
                            cache_scope: (!request.scope.is_empty())
                                .then_some(request.scope.as_str()),
                        })
                        .map_err(|error| error.to_string())
                })?;
                output.extend(result.values);
            }
            finkit::unified_runtime::RuntimeExecution {
                output,
                trace: finkit::unified_runtime::RuntimeExecutionTrace {
                    mode: RuntimeExecutionMode::Full,
                    rows: context.len(),
                    executed_nodes: plan.execution_order().len(),
                    recomputed_rows: context.len(),
                },
            }
        }
        _ => unreachable!("range pair was validated"),
    };
    let trace = runtime.trace;
    let values = runtime.output;
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
        "scope": request.scope,
        "data_revision": request.data_revision,
        "execution": execution_envelope(trace.mode),
        "values": Value::Object(serialized),
    }))
    .map_err(|error| error.to_string())
}

/// Execute one portable cross-sectional Factor over row-major symbol panels.
///
/// `inputs[name]` is laid out as `timestamp * symbols + symbol`. Every input
/// must use the same timestamp and symbol axes. The evaluator applies the
/// Factor independently to each timestamp row and preserves nulls for
/// non-finite numeric results in the JSON response.
pub fn evaluate_factor_cross_sectional_json(request: &str) -> Result<String, String> {
    let request: CrossSectionalFactorRequest =
        serde_json::from_str(request).map_err(|error| error.to_string())?;
    let version = request
        .schema_version
        .ok_or_else(|| "cross-sectional factor schema_version is required".to_string())?;
    if version != FACTOR_CROSS_SECTIONAL_CONTRACT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported cross-sectional factor schema_version: {version}"
        ));
    }
    if request.target.trim().is_empty() {
        return Err("cross-sectional factor target must not be empty".to_string());
    }
    if request.timestamps.is_empty() {
        return Err("cross-sectional factor timestamps must not be empty".to_string());
    }
    if request.symbols.is_empty() {
        return Err("cross-sectional factor symbols must not be empty".to_string());
    }
    if request.inputs.is_empty() {
        return Err("cross-sectional factor inputs must not be empty".to_string());
    }

    let registry = builtin_factor_registry();
    let catalog = FactorCatalog::from_registry(registry.clone());
    let target = catalog
        .resolve_name(&request.target)
        .ok_or_else(|| format!("unknown cross-sectional factor: {}", request.target))?;
    let descriptor = catalog
        .descriptor(target)
        .ok_or_else(|| format!("unknown cross-sectional factor: {target}"))?;
    if descriptor.kind != FactorKind::CrossSectional {
        return Err(format!("factor {target} is not cross-sectional"));
    }

    let symbol_refs: Vec<&str> = request.symbols.iter().map(String::as_str).collect();
    let numeric_inputs: BTreeMap<String, Vec<f64>> = request
        .inputs
        .iter()
        .map(|(name, values)| {
            (
                name.clone(),
                values
                    .iter()
                    .map(|value| value.unwrap_or(f64::NAN))
                    .collect(),
            )
        })
        .collect();
    let mut views = Vec::with_capacity(numeric_inputs.len());
    for (name, values) in &numeric_inputs {
        let view = CrossSectionView::new(&request.timestamps, &symbol_refs, values)
            .map_err(|error| format!("invalid cross-sectional input {name}: {error}"))?;
        views.push((name.as_str(), view));
    }
    let input_views: Vec<(&str, &CrossSectionView<'_>)> =
        views.iter().map(|(name, view)| (*name, view)).collect();
    let result = with_unified_engine(|unified| {
        unified
            .execute(OperationRequest::CrossSectionalFactor {
                name: target,
                inputs: &input_views,
            })
            .map_err(|error| error.to_string())
    })?;
    let values = result
        .values
        .get(target)
        .cloned()
        .ok_or_else(|| format!("cross-sectional factor did not produce {target}"))?;
    let mut serialized = serde_json::Map::new();
    serialized.insert(target.to_string(), nullable_series(&values));

    serde_json::to_string(&json!({
        "schema_version": FACTOR_CROSS_SECTIONAL_CONTRACT_SCHEMA_VERSION,
        "contract": "factor.cross_sectional.v1",
        "shape": "cross_section",
        "primary": target,
        "target": target,
        "scope": request.scope,
        "data_revision": request.data_revision,
        "semantic_identity": [format!("{}@{}", target, descriptor.metadata.version)],
        "timestamps": request.timestamps,
        "symbols": request.symbols,
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
        assert_eq!(payload["execution"]["mode"], "full");
        assert_eq!(payload["scope"], "");
        assert_eq!(payload["data_revision"], 0);
        assert_eq!(payload["range_lookback"], 5);
    }

    #[test]
    fn batch_contract_preserves_explicit_cache_scope_and_revision() {
        let request = r#"{
            "schema_version":1,
            "targets":["momentum_5"],
            "scope":"AAA@1d",
            "data_revision":42,
            "inputs":{"close":[1.0,2.0,3.0,4.0,5.0,6.0]}
        }"#;
        with_unified_engine(|engine| engine.clear_cache());
        let payload: Value = serde_json::from_str(&evaluate_factor_json(request).unwrap()).unwrap();
        assert_eq!(payload["scope"], "AAA@1d");
        assert_eq!(payload["data_revision"], 42);
        assert_eq!(payload["execution"]["mode"], "full");
        let after_first = with_unified_engine(|engine| engine.cache_stats());
        assert_eq!(after_first.misses, 1);
        assert_eq!(after_first.hits, 0);

        let second: Value = serde_json::from_str(&evaluate_factor_json(request).unwrap()).unwrap();
        assert_eq!(second["values"], payload["values"]);
        let after_second = with_unified_engine(|engine| engine.cache_stats());
        assert_eq!(after_second.misses, 1);
        assert_eq!(after_second.hits, 1);
    }

    #[test]
    fn contract_executes_factor_dirty_range_with_shared_envelope() {
        let request = r#"{
            "schema_version":1,
            "targets":["momentum_5"],
            "inputs":{"close":[10.0,11.0,12.0,13.0,14.0,20.0,16.0]},
            "previous":{"momentum_5":[0.0,0.0,0.0,0.0,0.0,0.5,0.5]},
            "dirty_range":[5,6]
        }"#;
        let payload: Value = serde_json::from_str(&evaluate_factor_json(request).unwrap()).unwrap();
        assert_eq!(payload["execution"]["mode"], "range");
        assert_eq!(payload["execution"]["input_dirty"], json!([5, 6]));
        assert_eq!(payload["execution"]["affected"], json!([5, 7]));
        assert_eq!(payload["values"]["momentum_5"][5], 1.0);
        assert!((payload["values"]["momentum_5"][6].as_f64().unwrap() - 5.0 / 11.0).abs() < 1e-12);
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

    #[test]
    fn cross_sectional_contract_preserves_axes_and_row_major_nulls() {
        let request = r#"{
            "schema_version":1,
            "target":"cross_rank",
            "timestamps":[10,20],
            "symbols":["AAA","BBB","CCC"],
            "inputs":{"score":[1.0,3.0,2.0,5.0,null,9.0]}
        }"#;
        let payload: Value =
            serde_json::from_str(&evaluate_factor_cross_sectional_json(request).unwrap()).unwrap();
        assert_eq!(payload["contract"], "factor.cross_sectional.v1");
        assert_eq!(payload["shape"], "cross_section");
        assert_eq!(payload["timestamps"], json!([10, 20]));
        assert_eq!(payload["symbols"], json!(["AAA", "BBB", "CCC"]));
        assert_eq!(payload["scope"], "");
        assert_eq!(payload["data_revision"], 0);
        assert_eq!(
            payload["values"]["cross_rank"],
            json!([0.0, 1.0, 0.5, 0.0, null, 1.0])
        );
    }

    #[test]
    fn cross_sectional_contract_rejects_mismatched_input_shape() {
        let request = r#"{
            "schema_version":1,
            "target":"cross_zscore",
            "timestamps":[10,20],
            "symbols":["AAA","BBB"],
            "inputs":{"score":[1.0,2.0,3.0]}
        }"#;
        assert!(evaluate_factor_cross_sectional_json(request)
            .unwrap_err()
            .contains("invalid cross-sectional input score"));
    }

    #[test]
    fn cross_sectional_contract_preserves_scope_and_revision() {
        let request = r#"{
            "schema_version":1,
            "target":"cross_rank",
            "scope":"panel@1d",
            "data_revision":11,
            "timestamps":[10],
            "symbols":["AAA","BBB"],
            "inputs":{"score":[1.0,2.0]}
        }"#;
        let payload: Value =
            serde_json::from_str(&evaluate_factor_cross_sectional_json(request).unwrap()).unwrap();
        assert_eq!(payload["scope"], "panel@1d");
        assert_eq!(payload["data_revision"], 11);
    }
}
