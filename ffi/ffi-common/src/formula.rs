//! Versioned formula execution contract shared by language bindings.
//!
//! Bindings should use this low-frequency control-plane envelope when they
//! need dialect selection and named outputs. Numeric hot paths can continue
//! to use typed indicator functions or zero-copy formula APIs.

use finkit::formula::{
    inspect_formula_compatibility, DrawCommand, DrawResult, FormulaContext, FormulaDialect,
    FormulaEngine, FormulaTerminal,
};
use ndarray::Array1;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Version of the cross-language formula result envelope.
pub const FORMULA_CONTRACT_SCHEMA_VERSION: u16 = 1;

/// Version of the normalized drawing payload embedded in the Formula result.
pub const FORMULA_DRAW_CONTRACT_SCHEMA_VERSION: u16 = 1;

/// Version of the language-neutral formula compatibility report envelope.
pub const FORMULA_COMPATIBILITY_SCHEMA_VERSION: u16 = 1;

/// Inspect a formula through the shared cross-language compatibility contract.
///
/// The report is intentionally emitted as one JSON shape for Rust, Python,
/// Go, Java, .NET, C, C++ and Node.  Binding-specific APIs should only decode
/// or forward this payload; they must not recreate terminal semantics.
pub fn formula_compatibility_report_json(source: &str, terminal: &str) -> Result<String, String> {
    let terminal = FormulaTerminal::from_str(terminal)
        .ok_or_else(|| format!("unknown formula terminal: {terminal}"))?;
    let report = inspect_formula_compatibility(source, terminal)?;
    let mut payload = serde_json::to_value(report).map_err(|error| error.to_string())?;
    let object = payload
        .as_object_mut()
        .ok_or_else(|| "formula compatibility report is not a JSON object".to_string())?;
    object.insert(
        "schema_version".to_string(),
        serde_json::json!(FORMULA_COMPATIBILITY_SCHEMA_VERSION),
    );
    serde_json::to_string(&payload).map_err(|error| error.to_string())
}

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
    let result = engine
        .eval_multi_with_dialect(source, dialect, &mut context)
        .map_err(|error| error.to_string())?;
    let draw = {
        let draw = context.draw_commands.borrow();
        json!({
            "schema_version": FORMULA_DRAW_CONTRACT_SCHEMA_VERSION,
            "commands": draw_commands_json(&draw),
        })
    };
    let mut values = result
        .outputs
        .into_iter()
        .map(|(name, value)| (name, value.to_vec()))
        .collect::<BTreeMap<_, _>>();
    let final_value = result.final_value.to_vec();
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
        "draw": draw,
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

fn nullable_array(values: &ndarray::Array1<f64>) -> Value {
    nullable_series(
        values
            .as_slice()
            .expect("formula drawing arrays must be contiguous"),
    )
}

fn draw_commands_json(draw: &DrawResult) -> Vec<Value> {
    draw.commands
        .iter()
        .map(|command| match command {
            DrawCommand::Text {
                condition,
                price,
                text,
                color,
            } => json!({
                "type": "Text",
                "condition": nullable_array(condition),
                "price": nullable_array(price),
                "text": text,
                "color": color,
            }),
            DrawCommand::Icon {
                condition,
                price,
                icon_type,
                color,
            } => json!({
                "type": "Icon",
                "condition": nullable_array(condition),
                "price": nullable_array(price),
                "iconType": icon_type,
                "color": color,
            }),
            DrawCommand::StickLine {
                condition,
                price1,
                price2,
                width,
                empty,
                color,
            } => json!({
                "type": "StickLine",
                "condition": nullable_array(condition),
                "price1": nullable_array(price1),
                "price2": nullable_array(price2),
                "width": width,
                "empty": empty,
                "color": color,
            }),
            DrawCommand::Line {
                cond1,
                price1,
                cond2,
                price2,
                expand,
                color,
            } => json!({
                "type": "Line",
                "cond1": nullable_array(cond1),
                "price1": nullable_array(price1),
                "cond2": nullable_array(cond2),
                "price2": nullable_array(price2),
                "expand": expand,
                "color": color,
            }),
            DrawCommand::Band {
                val1,
                color1,
                val2,
                color2,
            } => json!({
                "type": "Band",
                "val1": nullable_array(val1),
                "color1": color1,
                "val2": nullable_array(val2),
                "color2": color2,
            }),
            DrawCommand::KLine {
                open,
                high,
                low,
                close,
            } => json!({
                "type": "KLine",
                "open": nullable_array(open),
                "high": nullable_array(high),
                "low": nullable_array(low),
                "close": nullable_array(close),
            }),
            DrawCommand::Rect {
                x1,
                y1,
                x2,
                y2,
                color,
            } => json!({
                "type": "Rect",
                "x1": nullable_array(x1),
                "y1": nullable_array(y1),
                "x2": nullable_array(x2),
                "y2": nullable_array(y2),
                "color": color,
            }),
            DrawCommand::FillRgn {
                cond,
                price1,
                price2,
                color,
            } => json!({
                "type": "FillRgn",
                "cond": nullable_array(cond),
                "price1": nullable_array(price1),
                "price2": nullable_array(price2),
                "color": color,
            }),
            DrawCommand::PartLine { cond, price, color } => json!({
                "type": "PartLine",
                "cond": nullable_array(cond),
                "price": nullable_array(price),
                "color": color,
            }),
            DrawCommand::PolyLine { cond, price, color } => json!({
                "type": "PolyLine",
                "cond": nullable_array(cond),
                "price": nullable_array(price),
                "color": color,
            }),
            DrawCommand::Background { cond, color } => json!({
                "type": "Background",
                "cond": nullable_array(cond),
                "color": color,
            }),
            DrawCommand::SlopeLine {
                cond1,
                price1,
                cond2,
                price2,
                color,
            } => json!({
                "type": "SlopeLine",
                "cond1": nullable_array(cond1),
                "price1": nullable_array(price1),
                "cond2": nullable_array(cond2),
                "price2": nullable_array(price2),
                "color": color,
            }),
            DrawCommand::TextFix { x, y, text, color } => json!({
                "type": "TextFix",
                "x": x,
                "y": y,
                "text": text,
                "color": color,
            }),
            DrawCommand::Number {
                condition,
                price,
                number,
                precision,
                color,
            } => json!({
                "type": "Number",
                "condition": nullable_array(condition),
                "price": nullable_array(price),
                "number": nullable_array(number),
                "precision": precision,
                "color": color,
            }),
            DrawCommand::VertLine { condition, color } => json!({
                "type": "VertLine",
                "condition": nullable_array(condition),
                "color": color,
            }),
        })
        .collect()
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
        assert_eq!(json["draw"]["schema_version"], 1);
        assert_eq!(json["draw"]["commands"], serde_json::json!([]));
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

    #[test]
    fn contract_normalizes_domestic_source_before_execution() {
        let values = [1.0, 2.0, 3.0];
        let payload = evaluate_formula_json(
            "\u{feff}X:=CLOSE;\r\nX",
            "tdx",
            &values,
            &values,
            &values,
            &values,
            &values,
        )
        .unwrap();
        let json: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(json["dialect"], "tdx");
        assert_eq!(
            json["values"]["__PRIMARY__"],
            serde_json::json!([1.0, 2.0, 3.0])
        );
    }

    #[test]
    fn contract_serializes_numeric_drawing_payload_for_chart_adapters() {
        let payload = evaluate_formula_json(
            "DRAWICON(CLOSE > OPEN, CLOSE, 1)",
            "tdx",
            &[1.0, 1.0, 1.0],
            &[2.0, 2.0, 2.0],
            &[0.0, 0.0, 0.0],
            &[1.0, 2.0, 1.0],
            &[10.0, 20.0, 30.0],
        )
        .unwrap();
        let json: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(json["draw"]["schema_version"], 1);
        assert_eq!(json["draw"]["commands"].as_array().unwrap().len(), 1);
        assert_eq!(
            json["draw"]["commands"][0]["condition"],
            serde_json::json!([0.0, 1.0, 0.0])
        );
        assert_eq!(
            json["draw"]["commands"][0]["price"],
            serde_json::json!([1.0, 2.0, 1.0])
        );
        assert_eq!(json["draw"]["commands"][0]["iconType"], 1);
    }

    #[test]
    fn compatibility_contract_contains_capability_matrix_and_schema() {
        let payload =
            formula_compatibility_report_json("X:=MA(CLOSE,5); X + SECURITY(CLOSE, 'WEEK')", "tdx")
                .unwrap();
        let json: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(json["schema_version"], FORMULA_COMPATIBILITY_SCHEMA_VERSION);
        assert_eq!(json["terminal"], "tdx");
        assert!(json["capabilities"].as_array().unwrap().iter().any(|item| {
            item["name"] == "cross_timeframe" && item["status"] == "host_required"
        }));
    }
}
