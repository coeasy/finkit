//! Versioned formula execution contract shared by language bindings.
//!
//! Bindings should use this low-frequency control-plane envelope when they
//! need dialect selection and named outputs. Numeric hot paths can continue
//! to use typed indicator functions or zero-copy formula APIs.

use finkit::data_contract::{FundamentalSeries, TemporalAlignment, TemporalSeries};
use finkit::formula::{
    inspect_formula_compatibility, DrawCommand, DrawResult, FormulaContext, FormulaDialect,
    FormulaEngine, FormulaTerminal,
};
use ndarray::Array1;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

/// Version of the cross-language formula result envelope.
pub const FORMULA_CONTRACT_SCHEMA_VERSION: u16 = 1;

/// Version of the normalized drawing payload embedded in the Formula result.
pub const FORMULA_DRAW_CONTRACT_SCHEMA_VERSION: u16 = 1;

/// Version of the explicit multi-timeframe / point-in-time Formula contract.
pub const FORMULA_TEMPORAL_CONTRACT_SCHEMA_VERSION: u16 = 1;

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

#[derive(Debug, Deserialize)]
struct TemporalFormulaRequest {
    schema_version: u16,
    source: String,
    dialect: String,
    frame: TemporalFrameRequest,
    #[serde(default)]
    inputs: Vec<TemporalInputRequest>,
    #[serde(default)]
    fundamentals: Vec<TemporalInputRequest>,
}

#[derive(Debug, Deserialize)]
struct TemporalFrameRequest {
    symbol: String,
    timeframe: String,
    timestamps: Vec<i64>,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    #[serde(default)]
    amount: Option<Vec<f64>>,
}

#[derive(Debug, Deserialize)]
struct TemporalInputRequest {
    name: String,
    timestamps: Vec<i64>,
    values: Vec<f64>,
    alignment: String,
}

/// Execute a Formula against an explicitly timestamped frame and external
/// time series. `Exact` and `AsOfClosed` are the only accepted alignment
/// policies; no resampling or implicit future-value propagation is performed.
///
/// The same request/response JSON is intended for Rust, Python, Go, Java,
/// .NET, C, C++ and Node bindings. Fundamental inputs use publication-time
/// as-of semantics and are kept separate from ordinary aligned inputs so a
/// caller cannot accidentally weaken point-in-time guarantees.
pub fn evaluate_formula_temporal_json(request: &str) -> Result<String, String> {
    let request: TemporalFormulaRequest =
        serde_json::from_str(request).map_err(|error| error.to_string())?;
    if request.schema_version != FORMULA_TEMPORAL_CONTRACT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported formula temporal contract schema: {}",
            request.schema_version
        ));
    }
    let dialect = FormulaDialect::from_str(&request.dialect)
        .ok_or_else(|| format!("unsupported formula dialect: {}", request.dialect))?;
    let frame = &request.frame;
    if frame.symbol.trim().is_empty() || frame.timeframe.trim().is_empty() {
        return Err("frame symbol and timeframe must not be empty".to_string());
    }
    let expected = frame.close.len();
    for (name, length) in [
        ("timestamps", frame.timestamps.len()),
        ("open", frame.open.len()),
        ("high", frame.high.len()),
        ("low", frame.low.len()),
        ("volume", frame.volume.len()),
    ] {
        if length != expected {
            return Err(format!(
                "frame {name} length mismatch: expected {expected}, got {length}"
            ));
        }
    }
    if let Some(amount) = &frame.amount {
        if amount.len() != expected {
            return Err(format!(
                "frame amount length mismatch: expected {expected}, got {}",
                amount.len()
            ));
        }
    }
    TemporalSeries::new("__FRAME__", &frame.timestamps, &frame.close)
        .map_err(|error| error.to_string())?;

    let mut names = HashSet::new();
    let mut context = FormulaContext::new(
        Array1::from_vec(frame.open.clone()),
        Array1::from_vec(frame.high.clone()),
        Array1::from_vec(frame.low.clone()),
        Array1::from_vec(frame.close.clone()),
        Array1::from_vec(frame.volume.clone()),
        frame.amount.clone().map(Array1::from_vec),
    );
    context.datetime = Some(Array1::from_vec(frame.timestamps.clone()));
    let mut input_metadata = Vec::with_capacity(request.inputs.len());
    let mut fundamental_metadata = Vec::with_capacity(request.fundamentals.len());

    for input in &request.inputs {
        let name = external_formula_name(&input.name)?;
        if !names.insert(name.clone()) {
            return Err(format!("duplicate temporal input: {name}"));
        }
        let alignment = parse_temporal_alignment(&input.alignment)?;
        let series = TemporalSeries::new(&name, &input.timestamps, &input.values)
            .map_err(|error| error.to_string())?;
        let values = series
            .align_to(&frame.timestamps, alignment)
            .map_err(|error| error.to_string())?;
        context
            .variables
            .insert(std::sync::Arc::from(name), Array1::from_vec(values));
        input_metadata.push(json!({
            "name": input.name,
            "alignment": input.alignment.trim().to_ascii_lowercase(),
        }));
    }

    for fundamental in &request.fundamentals {
        let name = external_formula_name(&fundamental.name)?;
        if !names.insert(name.clone()) {
            return Err(format!("duplicate fundamental input: {name}"));
        }
        if parse_temporal_alignment(&fundamental.alignment)? != TemporalAlignment::AsOfClosed {
            return Err(format!(
                "fundamental input `{name}` must use `as_of_closed` alignment"
            ));
        }
        let series = FundamentalSeries::new(&name, &fundamental.timestamps, &fundamental.values)
            .map_err(|error| error.to_string())?;
        let values = frame
            .timestamps
            .iter()
            .map(|&timestamp| series.as_of(timestamp).unwrap_or(f64::NAN))
            .collect::<Vec<_>>();
        context
            .variables
            .insert(std::sync::Arc::from(name), Array1::from_vec(values));
        fundamental_metadata.push(json!({
            "name": fundamental.name,
            "alignment": "as_of_closed",
        }));
    }

    let mut engine = FormulaEngine::new();
    let result = engine
        .eval_multi_with_dialect(&request.source, dialect, &mut context)
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
    values.insert("__PRIMARY__".to_string(), result.final_value.to_vec());
    let serialized_values = values
        .into_iter()
        .map(|(name, series)| (name, nullable_series(&series)))
        .collect::<BTreeMap<_, _>>();
    serde_json::to_string(&json!({
        "schema_version": FORMULA_TEMPORAL_CONTRACT_SCHEMA_VERSION,
        "contract": "formula.temporal.v1",
        "dialect": dialect.as_str(),
        "primary": "__PRIMARY__",
        "frame": {
            "symbol": frame.symbol,
            "timeframe": frame.timeframe,
            "timestamps": frame.timestamps,
        },
        "inputs": input_metadata,
        "fundamentals": fundamental_metadata,
        "values": serialized_values,
        "draw": draw,
    }))
    .map_err(|error| error.to_string())
}

fn external_formula_name(name: &str) -> Result<String, String> {
    let name = name.trim().to_uppercase();
    if name.is_empty() {
        return Err("temporal input name must not be empty".to_string());
    }
    if matches!(
        name.as_str(),
        "OPEN" | "HIGH" | "LOW" | "CLOSE" | "VOLUME" | "AMOUNT"
    ) {
        return Err(format!(
            "temporal input cannot shadow built-in field: {name}"
        ));
    }
    Ok(name)
}

fn parse_temporal_alignment(value: &str) -> Result<TemporalAlignment, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "exact" => Ok(TemporalAlignment::Exact),
        "as_of_closed" => Ok(TemporalAlignment::AsOfClosed),
        _ => Err(format!(
            "unsupported temporal alignment `{value}`; expected `exact` or `as_of_closed`"
        )),
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
    fn temporal_contract_aligns_external_series_and_fundamentals_without_lookahead() {
        let request = serde_json::json!({
            "schema_version": FORMULA_TEMPORAL_CONTRACT_SCHEMA_VERSION,
            "source": "HIGHER + EARNINGS + CLOSE",
            "dialect": "tdx",
            "frame": {
                "symbol": "AAA",
                "timeframe": "1m",
                "timestamps": [10, 20, 30, 40],
                "open": [1.0, 2.0, 3.0, 4.0],
                "high": [1.0, 2.0, 3.0, 4.0],
                "low": [1.0, 2.0, 3.0, 4.0],
                "close": [1.0, 2.0, 3.0, 4.0],
                "volume": [10.0, 20.0, 30.0, 40.0]
            },
            "inputs": [{
                "name": "higher",
                "timestamps": [10, 30],
                "values": [100.0, 300.0],
                "alignment": "as_of_closed"
            }],
            "fundamentals": [{
                "name": "earnings",
                "timestamps": [15, 35],
                "values": [1.0, 2.0],
                "alignment": "as_of_closed"
            }]
        });
        let payload = evaluate_formula_temporal_json(&request.to_string()).unwrap();
        let json: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(json["contract"], "formula.temporal.v1");
        assert_eq!(json["frame"]["symbol"], "AAA");
        assert_eq!(
            json["values"]["__PRIMARY__"],
            serde_json::json!([null, 103.0, 304.0, 306.0])
        );
    }

    #[test]
    fn temporal_contract_rejects_unsafe_or_ambiguous_requests() {
        let mut request = serde_json::json!({
            "schema_version": FORMULA_TEMPORAL_CONTRACT_SCHEMA_VERSION,
            "source": "HIGHER",
            "dialect": "tdx",
            "frame": {
                "symbol": "AAA",
                "timeframe": "1m",
                "timestamps": [10, 20],
                "open": [1.0, 2.0],
                "high": [1.0, 2.0],
                "low": [1.0, 2.0],
                "close": [1.0, 2.0],
                "volume": [10.0, 20.0]
            },
            "inputs": [{
                "name": "higher",
                "timestamps": [20, 10],
                "values": [2.0, 1.0],
                "alignment": "as_of_closed"
            }]
        });
        let error = evaluate_formula_temporal_json(&request.to_string()).unwrap_err();
        assert!(error.contains("monotonic"));

        request["inputs"][0]["alignment"] = serde_json::json!("forward_fill");
        request["inputs"][0]["timestamps"] = serde_json::json!([10, 20]);
        let error = evaluate_formula_temporal_json(&request.to_string()).unwrap_err();
        assert!(error.contains("unsupported temporal alignment"));

        request["inputs"] = serde_json::json!([]);
        request["source"] = serde_json::json!("EARNINGS");
        request["fundamentals"] = serde_json::json!([{
            "name": "earnings",
            "timestamps": [10],
            "values": [1.0],
            "alignment": "exact"
        }]);
        let error = evaluate_formula_temporal_json(&request.to_string()).unwrap_err();
        assert!(error.contains("must use `as_of_closed`"));
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
