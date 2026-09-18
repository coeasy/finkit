//! Versioned formula execution contract shared by language bindings.
//!
//! Bindings should use this low-frequency control-plane envelope when they
//! need dialect selection and named outputs. Numeric hot paths can continue
//! to use typed indicator functions or zero-copy formula APIs.

use finkit::formula::{parse_formula_with_dialect, FormulaContext, FormulaDialect, FormulaEngine};
use ndarray::Array1;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

/// Version of the cross-language formula result envelope.
pub const FORMULA_CONTRACT_SCHEMA_VERSION: u16 = 1;

/// Evaluate one formula and serialize the language-neutral result envelope.
pub fn evaluate_formula_json(
    source: &str,
    dialect: &str,
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
) -> Result<String, String> {
    let expected = open.len();
    if high.len() != expected
        || low.len() != expected
        || close.len() != expected
        || volume.len() != expected
    {
        return Err("all OHLCV series must have the same length".to_string());
    }
    let dialect = FormulaDialect::from_str(dialect)
        .ok_or_else(|| format!("unsupported formula dialect: {dialect}"))?;
    let mut context = FormulaContext::new(
        Array1::from_vec(open.to_vec()),
        Array1::from_vec(high.to_vec()),
        Array1::from_vec(low.to_vec()),
        Array1::from_vec(close.to_vec()),
        Array1::from_vec(volume.to_vec()),
        None,
    );
    let mut engine = FormulaEngine::new();
    let mut values = BTreeMap::new();
    let final_value = match dialect {
        FormulaDialect::AlphaTA
        | FormulaDialect::TongDaXin
        | FormulaDialect::TongHuaShun
        | FormulaDialect::EastMoney => {
            let result = engine
                .eval_multi(source, &mut context)
                .map_err(|error| error.to_string())?;
            values.extend(
                result
                    .outputs
                    .into_iter()
                    .map(|(name, value)| (name, value.to_vec())),
            );
            result.final_value.to_vec()
        }
        FormulaDialect::Pine => {
            let ast = parse_formula_with_dialect(source, FormulaDialect::Pine)
                .map_err(|error| error.to_string())?;
            let variables_before: HashSet<String> =
                context.variables.keys().map(ToString::to_string).collect();
            let result = engine
                .eval_ast(&ast, &mut context)
                .map_err(|error| error.to_string())?;
            for (name, value) in &context.variables {
                let name = name.to_string();
                if !variables_before.contains(&name) {
                    values.insert(name, value.to_vec());
                }
            }
            result.to_vec()
        }
    };
    values.insert("__PRIMARY__".to_string(), final_value);

    let serialized_values = values
        .into_iter()
        .map(|(name, series)| (name, nullable_series(&series)))
        .collect::<BTreeMap<_, _>>();
    serde_json::to_string(&json!({
        "schema_version": FORMULA_CONTRACT_SCHEMA_VERSION,
        "dialect": dialect.as_str(),
        "primary": "__PRIMARY__",
        "values": serialized_values,
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
    fn contract_contains_named_values_and_null_warmup() {
        let values = [1.0, f64::NAN, 3.0];
        let payload = evaluate_formula_json(
            "CLOSE", "alpha_ta", &values, &values, &values, &values, &values,
        )
        .unwrap();
        let json: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["primary"], "__PRIMARY__");
        assert_eq!(json["values"]["__PRIMARY__"][1], Value::Null);
    }

    #[test]
    fn contract_preserves_pine_dialect_identity() {
        let values = [1.0, 2.0, 3.0];
        let payload = evaluate_formula_json(
            "//@version=5\nindicator(\"x\")\nr = ta.sma(close, 2)\nplot(r)",
            "pine",
            &values,
            &values,
            &values,
            &values,
            &values,
        )
        .unwrap();
        let json: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(json["dialect"], "pine");
    }
}
