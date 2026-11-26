//! Versioned JSON contract for mixed time-series and cross-sectional factors.

use finkit::factors::builtin::factor_library;
use finkit::factors::panel::FactorPanel;
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;

/// Version of the time-by-symbol factor panel contract.
pub const FACTOR_PANEL_CONTRACT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Deserialize)]
struct FactorPanelRequest {
    schema_version: u16,
    timestamps: Vec<i64>,
    symbols: Vec<String>,
    fields: BTreeMap<String, Vec<Option<f64>>>,
    #[serde(default)]
    library: Option<String>,
    #[serde(default)]
    factor: Option<String>,
    #[serde(default)]
    expression: Option<String>,
}

/// Evaluate one built-in factor or an explicit expression over an aligned
/// row-major panel. JSON null inputs become NaN; non-finite outputs become null.
pub fn evaluate_factor_panel_json(request: &str) -> Result<String, String> {
    let request: FactorPanelRequest =
        serde_json::from_str(request).map_err(|error| error.to_string())?;
    if request.schema_version != FACTOR_PANEL_CONTRACT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported factor panel schema_version: {}",
            request.schema_version
        ));
    }
    if request.factor.is_some() == request.expression.is_some() {
        return Err("provide exactly one of factor or expression".to_string());
    }
    if request.factor.is_some() && request.library.is_none() {
        return Err("library is required when factor is provided".to_string());
    }
    if request.expression.is_some() && request.library.is_some() {
        return Err("library must be omitted when expression is provided".to_string());
    }
    if request.timestamps.is_empty() || request.symbols.is_empty() {
        return Err("factor panel axes must not be empty".to_string());
    }
    let cell_count = request
        .timestamps
        .len()
        .checked_mul(request.symbols.len())
        .ok_or_else(|| "factor panel dimensions overflow address space".to_string())?;
    let mut owned_fields = BTreeMap::new();
    for (name, values) in request.fields {
        if values.len() != cell_count {
            return Err(format!(
                "field {name} has {} cells; expected {cell_count}",
                values.len()
            ));
        }
        owned_fields.insert(
            name,
            values
                .into_iter()
                .map(|value| value.unwrap_or(f64::NAN))
                .collect::<Vec<_>>(),
        );
    }
    let symbol_refs: Vec<&str> = request.symbols.iter().map(String::as_str).collect();
    let field_refs: Vec<(&str, &[f64])> = owned_fields
        .iter()
        .map(|(name, values)| (name.as_str(), values.as_slice()))
        .collect();
    let panel = FactorPanel::new(&request.timestamps, &symbol_refs, &field_refs)
        .map_err(|error| error.to_string())?;

    let (primary, result) = if let Some(expression) = request.expression {
        (
            "__PRIMARY__".to_string(),
            panel
                .evaluate(&expression)
                .map_err(|error| error.to_string())?,
        )
    } else {
        let factor = request.factor.expect("exactly one target was validated");
        let library_name = request.library.expect("library was validated");
        let library = factor_library(&library_name).map_err(|error| error.to_string())?;
        let values = library
            .evaluate_panel(&factor, &panel)
            .map_err(|error| error.to_string())?;
        (factor, values)
    };
    let nullable_values: Vec<Option<f64>> = result
        .values
        .iter()
        .map(|value| value.is_finite().then_some(*value))
        .collect();
    serde_json::to_string(&json!({
        "schema_version": FACTOR_PANEL_CONTRACT_SCHEMA_VERSION,
        "contract": "factor.panel.v1",
        "primary": primary,
        "layout": "row_major",
        "shape": [result.rows(), result.cols()],
        "timestamps": result.timestamps,
        "symbols": result.symbols,
        "values": nullable_values,
    }))
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn panel_contract_preserves_axes_shape_and_null_warmup() {
        let request = json!({
            "schema_version": 1,
            "timestamps": [1, 2, 3],
            "symbols": ["AAA", "BBB"],
            "fields": { "close": [1.0, 10.0, 2.0, 20.0, 3.0, 30.0] },
            "expression": "REF(CLOSE, 1)"
        });
        let response: Value =
            serde_json::from_str(&evaluate_factor_panel_json(&request.to_string()).unwrap())
                .unwrap();
        assert_eq!(response["contract"], "factor.panel.v1");
        assert_eq!(response["layout"], "row_major");
        assert_eq!(response["shape"], json!([3, 2]));
        assert_eq!(response["timestamps"], json!([1, 2, 3]));
        assert_eq!(response["symbols"], json!(["AAA", "BBB"]));
        assert_eq!(response["values"][0], Value::Null);
        assert_eq!(response["values"][1], Value::Null);
        assert_eq!(response["values"][2], json!(1.0));
        assert_eq!(response["values"][3], json!(10.0));
    }

    #[test]
    fn panel_contract_rejects_bad_dimensions_and_target_ambiguity() {
        let wrong_shape = json!({
            "schema_version": 1,
            "timestamps": [1],
            "symbols": ["AAA", "BBB"],
            "fields": { "close": [1.0] },
            "expression": "CLOSE"
        });
        assert!(evaluate_factor_panel_json(&wrong_shape.to_string())
            .unwrap_err()
            .contains("expected 2"));

        let ambiguous = json!({
            "schema_version": 1,
            "timestamps": [1],
            "symbols": ["AAA"],
            "fields": { "close": [1.0] },
            "expression": "CLOSE",
            "factor": "Alpha6",
            "library": "worldquant101"
        });
        assert!(evaluate_factor_panel_json(&ambiguous.to_string())
            .unwrap_err()
            .contains("exactly one"));
    }
}
