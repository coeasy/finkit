//! Versioned formula execution contract shared by language bindings.
//!
//! Bindings should use this low-frequency control-plane envelope when they
//! need dialect selection and named outputs. Numeric hot paths can continue
//! to use typed indicator functions or zero-copy formula APIs.

use finkit::data_contract::{FrameKey, FundamentalSeries, TemporalAlignment, TemporalSeries};
use finkit::factors::FactorRegistry;
use finkit::formula::{
    inspect_formula_compatibility, AstNode, DrawCommand, DrawResult, FormulaContext,
    FormulaDialect, FormulaEngine, FormulaTerminal, PineAstNode, PineMapperError,
    PineSecurityResolver,
};
use finkit::operation::{OperationRequest, UnifiedOperationEngine};
use ndarray::Array1;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

/// Version of the cross-language formula result envelope.
pub const FORMULA_CONTRACT_SCHEMA_VERSION: u16 = 1;

/// Version of the normalized drawing payload embedded in the Formula result.
pub const FORMULA_DRAW_CONTRACT_SCHEMA_VERSION: u16 = 1;

/// Version of the explicit multi-timeframe / point-in-time Formula contract.
pub const FORMULA_TEMPORAL_CONTRACT_SCHEMA_VERSION: u16 = 1;

/// Version of the explicit multi-symbol / multi-timeframe Formula contract.
pub const FORMULA_PANEL_CONTRACT_SCHEMA_VERSION: u16 = 1;

/// Version of the row-major cross-sectional Formula contract.
pub const FORMULA_CROSS_SECTIONAL_CONTRACT_SCHEMA_VERSION: u16 = 1;

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
    let mut unified = UnifiedOperationEngine::new(FactorRegistry::new());
    let result = unified
        .execute(OperationRequest::Formula {
            source,
            dialect,
            context: &mut context,
        })
        .map_err(|error| error.to_string())?;
    let draw = {
        let draw = result.draw.unwrap_or_default();
        json!({
            "schema_version": FORMULA_DRAW_CONTRACT_SCHEMA_VERSION,
            "commands": draw_commands_json(&draw),
        })
    };
    let values = result.values;

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
    /// Explicitly aligned data providers for Pine `request.security` calls.
    #[serde(default, alias = "security_inputs")]
    security: Vec<TemporalSecurityInputRequest>,
}

#[derive(Debug, Deserialize, Serialize)]
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
struct FormulaPanelRequest {
    schema_version: u16,
    source: String,
    dialect: String,
    frames: Vec<TemporalFrameRequest>,
}

#[derive(Debug, Deserialize)]
struct FormulaCrossSectionalRequest {
    schema_version: u16,
    source: String,
    dialect: String,
    timestamps: Vec<i64>,
    symbols: Vec<String>,
    inputs: BTreeMap<String, Vec<Option<f64>>>,
}

#[derive(Debug, Deserialize)]
struct TemporalInputRequest {
    name: String,
    timestamps: Vec<i64>,
    values: Vec<f64>,
    alignment: String,
}

#[derive(Debug, Deserialize)]
struct TemporalSecurityInputRequest {
    symbol: String,
    timeframe: String,
    expression: String,
    timestamps: Vec<i64>,
    values: Vec<f64>,
    alignment: String,
}

struct TemporalSecurityResolver<'a> {
    frame_symbol: &'a str,
    providers: &'a BTreeMap<String, String>,
}

impl PineSecurityResolver for TemporalSecurityResolver<'_> {
    fn resolve_security(
        &self,
        args: &[(Option<String>, PineAstNode)],
    ) -> Result<AstNode, PineMapperError> {
        if args.len() != 3 {
            return Err(PineMapperError {
                message: format!(
                    "request.security requires exactly 3 arguments, got {}",
                    args.len()
                ),
            });
        }
        let symbol = pine_security_text(&args[0].1).ok_or_else(|| PineMapperError {
            message: "request.security symbol must be a literal or syminfo.tickerid".to_string(),
        })?;
        let symbol = if symbol.eq_ignore_ascii_case("syminfo.tickerid") {
            self.frame_symbol.to_string()
        } else {
            symbol
        };
        let timeframe = pine_security_text(&args[1].1).ok_or_else(|| PineMapperError {
            message: "request.security timeframe must be a literal string".to_string(),
        })?;
        let expression = pine_security_expression(&args[2].1).ok_or_else(|| PineMapperError {
            message: "request.security provider currently requires a named OHLCV expression"
                .to_string(),
        })?;
        let key = temporal_security_key(&symbol, &timeframe, &expression);
        let alias = self.providers.get(&key).ok_or_else(|| PineMapperError {
            message: format!(
                "request.security provider data is missing for {symbol}@{timeframe}:{expression}"
            ),
        })?;
        Ok(AstNode::Variable(alias.clone()))
    }
}

fn pine_security_text(node: &PineAstNode) -> Option<String> {
    match node {
        PineAstNode::Identifier(value) | PineAstNode::StringLit(value) => Some(value.clone()),
        _ => None,
    }
}

fn pine_security_expression(node: &PineAstNode) -> Option<String> {
    let value = match node {
        PineAstNode::Identifier(value) => value,
        _ => return None,
    };
    let value = value.trim().to_ascii_lowercase();
    matches!(
        value.as_str(),
        "open" | "high" | "low" | "close" | "volume" | "hl2" | "hlc3" | "ohlc4"
    )
    .then_some(value)
}

fn temporal_security_key(symbol: &str, timeframe: &str, expression: &str) -> String {
    format!(
        "{}|{}|{}",
        symbol.trim().to_ascii_uppercase(),
        timeframe.trim().to_ascii_uppercase(),
        expression.trim().to_ascii_lowercase()
    )
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
    if !request.security.is_empty() && dialect != FormulaDialect::Pine {
        return Err("temporal security providers currently require the pine dialect".to_string());
    }

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
    let mut security_providers = BTreeMap::new();
    let mut security_metadata = Vec::with_capacity(request.security.len());
    for (index, provider) in request.security.iter().enumerate() {
        if provider.symbol.trim().is_empty()
            || provider.timeframe.trim().is_empty()
            || provider.expression.trim().is_empty()
        {
            return Err(
                "security provider symbol, timeframe and expression must not be empty".to_string(),
            );
        }
        if provider.timestamps.len() != provider.values.len() {
            return Err(format!(
                "security provider {} length mismatch: timestamps={}, values={}",
                provider.expression,
                provider.timestamps.len(),
                provider.values.len()
            ));
        }
        let expression = provider.expression.trim().to_ascii_lowercase();
        if !matches!(
            expression.as_str(),
            "open" | "high" | "low" | "close" | "volume" | "hl2" | "hlc3" | "ohlc4"
        ) {
            return Err(format!(
                "unsupported security provider expression `{}`",
                provider.expression
            ));
        }
        let alignment = parse_temporal_alignment(&provider.alignment)?;
        let source_name = format!("__SECURITY_SOURCE_{index}");
        let series = TemporalSeries::new(&source_name, &provider.timestamps, &provider.values)
            .map_err(|error| error.to_string())?;
        let values = series
            .align_to(&frame.timestamps, alignment)
            .map_err(|error| error.to_string())?;
        let alias = format!("__FINKIT_SECURITY_{index}");
        if !names.insert(alias.clone()) {
            return Err(format!("duplicate security provider alias: {alias}"));
        }
        context.variables.insert(
            std::sync::Arc::from(alias.clone()),
            Array1::from_vec(values),
        );
        let key = temporal_security_key(&provider.symbol, &provider.timeframe, &expression);
        if security_providers.insert(key, alias).is_some() {
            return Err(format!(
                "duplicate security provider: {}@{}:{}",
                provider.symbol, provider.timeframe, provider.expression
            ));
        }
        security_metadata.push(json!({
            "symbol": provider.symbol,
            "timeframe": provider.timeframe,
            "expression": expression,
            "alignment": provider.alignment.trim().to_ascii_lowercase(),
        }));
    }
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
    let result = if dialect == FormulaDialect::Pine {
        let resolver = TemporalSecurityResolver {
            frame_symbol: &frame.symbol,
            providers: &security_providers,
        };
        engine.eval_multi_with_pine_security(&request.source, &mut context, &resolver)
    } else {
        engine.eval_multi_with_dialect(&request.source, dialect, &mut context)
    }
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
        "security": security_metadata,
        "values": serialized_values,
        "draw": draw,
    }))
    .map_err(|error| error.to_string())
}

/// Execute one Formula independently for every explicit symbol/timeframe
/// frame. Frames are never concatenated and no state or drawing command is
/// shared between them. Each frame reuses the `formula.temporal.v1` execution
/// and result semantics, keeping null handling and dialect behavior identical
/// across single-frame and panel requests.
pub fn evaluate_formula_panel_json(request: &str) -> Result<String, String> {
    let request: FormulaPanelRequest =
        serde_json::from_str(request).map_err(|error| error.to_string())?;
    if request.schema_version != FORMULA_PANEL_CONTRACT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported formula panel contract schema: {}",
            request.schema_version
        ));
    }
    if request.frames.is_empty() {
        return Err("formula panel must contain at least one frame".to_string());
    }
    let mut seen = HashSet::new();
    let mut frames = BTreeMap::new();
    for frame in request.frames {
        let key =
            FrameKey::new(&frame.symbol, &frame.timeframe).map_err(|error| error.to_string())?;
        let key_name = format!("{}@{}", key.symbol, key.timeframe);
        if !seen.insert(key_name.clone()) {
            return Err(format!("duplicate formula panel frame: {key_name}"));
        }
        let frame_value = serde_json::to_value(&frame).map_err(|error| error.to_string())?;
        let child_request = json!({
            "schema_version": FORMULA_TEMPORAL_CONTRACT_SCHEMA_VERSION,
            "source": &request.source,
            "dialect": &request.dialect,
            "frame": frame_value,
        });
        let child: Value =
            serde_json::from_str(&evaluate_formula_temporal_json(&child_request.to_string())?)
                .map_err(|error| error.to_string())?;
        frames.insert(
            key_name,
            json!({
                "symbol": key.symbol,
                "timeframe": key.timeframe,
                "timestamps": child["frame"]["timestamps"],
                "primary": child["primary"],
                "values": child["values"],
                "draw": child["draw"],
            }),
        );
    }
    serde_json::to_string(&json!({
        "schema_version": FORMULA_PANEL_CONTRACT_SCHEMA_VERSION,
        "contract": "formula.panel.v1",
        "dialect": request.dialect,
        "frames": frames.into_values().collect::<Vec<_>>(),
    }))
    .map_err(|error| error.to_string())
}

/// Execute a Formula independently across every timestamp row of a symbol
/// panel. The explicit `CS_*` functions operate across the symbol columns;
/// legacy time-series functions retain their ordinary series semantics and
/// therefore are not silently reinterpreted as cross-sectional operations.
pub fn evaluate_formula_cross_sectional_json(request: &str) -> Result<String, String> {
    let request: FormulaCrossSectionalRequest =
        serde_json::from_str(request).map_err(|error| error.to_string())?;
    if request.schema_version != FORMULA_CROSS_SECTIONAL_CONTRACT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported formula cross-sectional contract schema: {}",
            request.schema_version
        ));
    }
    let dialect = FormulaDialect::from_str(&request.dialect)
        .ok_or_else(|| format!("unsupported formula dialect: {}", request.dialect))?;
    if request.source.trim().is_empty() {
        return Err("formula cross-sectional source must not be empty".to_string());
    }
    if request.timestamps.is_empty() {
        return Err("formula cross-sectional timestamps must not be empty".to_string());
    }
    if request.symbols.is_empty() {
        return Err("formula cross-sectional symbols must not be empty".to_string());
    }
    if request.inputs.is_empty() {
        return Err("formula cross-sectional inputs must not be empty".to_string());
    }

    let symbol_refs: Vec<&str> = request.symbols.iter().map(String::as_str).collect();
    let mut names = HashSet::new();
    let mut inputs = BTreeMap::new();
    for (raw_name, values) in &request.inputs {
        let name = normalize_cross_sectional_input_name(raw_name)?;
        if !names.insert(name.clone()) {
            return Err(format!("duplicate formula cross-sectional input: {name}"));
        }
        let values = values
            .iter()
            .map(|value| value.unwrap_or(f64::NAN))
            .collect::<Vec<_>>();
        finkit::data_contract::CrossSectionView::new(&request.timestamps, &symbol_refs, &values)
            .map_err(|error| format!("invalid formula cross-sectional input {name}: {error}"))?;
        inputs.insert(name, values);
    }

    let symbols_per_row = request.symbols.len();
    let mut engine = FormulaEngine::new();
    let mut output_values: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut draw_rows = Vec::with_capacity(request.timestamps.len());
    for (row, timestamp) in request.timestamps.iter().copied().enumerate() {
        let start = row * symbols_per_row;
        let end = start + symbols_per_row;
        let row_values = |name: &str| {
            inputs
                .get(name)
                .map(|values| values[start..end].to_vec())
                .unwrap_or_else(|| vec![f64::NAN; symbols_per_row])
        };
        let open = row_values("OPEN");
        let high = row_values("HIGH");
        let low = row_values("LOW");
        let close = row_values("CLOSE");
        let volume = row_values("VOLUME");
        let amount = inputs
            .contains_key("AMOUNT")
            .then(|| Array1::from_vec(row_values("AMOUNT")));
        let mut context = FormulaContext::new(
            Array1::from_vec(open),
            Array1::from_vec(high),
            Array1::from_vec(low),
            Array1::from_vec(close),
            Array1::from_vec(volume),
            amount,
        );
        context.datetime = Some(Array1::from_elem(symbols_per_row, timestamp));
        for (name, values) in &inputs {
            if matches!(
                name.as_str(),
                "OPEN" | "HIGH" | "LOW" | "CLOSE" | "VOLUME" | "AMOUNT"
            ) {
                continue;
            }
            context.variables.insert(
                std::sync::Arc::from(name.as_str()),
                Array1::from_vec(values[start..end].to_vec()),
            );
        }

        let result = engine
            .eval_multi_with_dialect(&request.source, dialect, &mut context)
            .map_err(|error| format!("formula cross-sectional row {timestamp}: {error}"))?;
        for (name, value) in result.outputs {
            if value.len() != symbols_per_row {
                return Err(format!(
                    "formula output {name} length mismatch at timestamp {timestamp}"
                ));
            }
            output_values
                .entry(name)
                .or_default()
                .extend(value.iter().copied());
        }
        if result.final_value.len() != symbols_per_row {
            return Err(format!(
                "formula primary output length mismatch at timestamp {timestamp}"
            ));
        }
        output_values
            .entry("__PRIMARY__".to_string())
            .or_default()
            .extend(result.final_value.iter().copied());
        let draw = context.draw_commands.borrow();
        draw_rows.push(json!({
            "timestamp": timestamp,
            "commands": draw_commands_json(&draw),
        }));
    }

    let serialized_values = output_values
        .into_iter()
        .map(|(name, values)| (name, nullable_series(&values)))
        .collect::<BTreeMap<_, _>>();
    serde_json::to_string(&json!({
        "schema_version": FORMULA_CROSS_SECTIONAL_CONTRACT_SCHEMA_VERSION,
        "contract": "formula.cross_sectional.v1",
        "dialect": dialect.as_str(),
        "primary": "__PRIMARY__",
        "timestamps": request.timestamps,
        "symbols": request.symbols,
        "values": serialized_values,
        "draw": {
            "schema_version": FORMULA_DRAW_CONTRACT_SCHEMA_VERSION,
            "mode": "per_row",
            "rows": draw_rows,
        },
    }))
    .map_err(|error| error.to_string())
}

fn normalize_cross_sectional_input_name(name: &str) -> Result<String, String> {
    let name = name.trim().to_uppercase();
    if name.is_empty() {
        return Err("formula cross-sectional input name must not be empty".to_string());
    }
    Ok(name)
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
    fn temporal_contract_executes_pine_security_from_explicit_provider_data() {
        let request = serde_json::json!({
            "schema_version": FORMULA_TEMPORAL_CONTRACT_SCHEMA_VERSION,
            "source": "//@version=5\nindicator(\"HTF\")\nhtf = request.security(syminfo.tickerid, \"D\", close)\nhtf",
            "dialect": "pine",
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
            "security": [{
                "symbol": "AAA",
                "timeframe": "D",
                "expression": "close",
                "timestamps": [10, 30],
                "values": [100.0, 300.0],
                "alignment": "as_of_closed"
            }]
        });
        let payload = evaluate_formula_temporal_json(&request.to_string()).unwrap();
        let json: Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(json["dialect"], "pine");
        assert_eq!(json["security"][0]["timeframe"], "D");
        assert_eq!(
            json["values"]["__PRIMARY__"],
            serde_json::json!([100.0, 100.0, 300.0, 300.0])
        );
    }

    #[test]
    fn temporal_contract_rejects_pine_security_without_provider_data() {
        let request = serde_json::json!({
            "schema_version": FORMULA_TEMPORAL_CONTRACT_SCHEMA_VERSION,
            "source": "//@version=5\nindicator(\"HTF\")\nrequest.security(syminfo.tickerid, \"D\", close)",
            "dialect": "pine",
            "frame": {
                "symbol": "AAA",
                "timeframe": "1m",
                "timestamps": [10, 20],
                "open": [1.0, 2.0],
                "high": [1.0, 2.0],
                "low": [1.0, 2.0],
                "close": [1.0, 2.0],
                "volume": [10.0, 20.0]
            }
        });
        let error = evaluate_formula_temporal_json(&request.to_string()).unwrap_err();
        assert!(error.contains("provider data is missing"));
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
    fn panel_contract_keeps_symbol_and_timeframe_results_isolated() {
        let frame = |symbol: &str, timeframe: &str, close: &[f64]| {
            serde_json::json!({
                "symbol": symbol,
                "timeframe": timeframe,
                "timestamps": [10, 20],
                "open": close,
                "high": close,
                "low": close,
                "close": close,
                "volume": [10.0, 20.0]
            })
        };
        let request = serde_json::json!({
            "schema_version": FORMULA_PANEL_CONTRACT_SCHEMA_VERSION,
            "source": "CLOSE + 1",
            "dialect": "tdx",
            "frames": [
                frame("BBB", "1d", &[10.0, 20.0]),
                frame("AAA", "1m", &[1.0, 2.0])
            ]
        });
        let payload: Value =
            serde_json::from_str(&evaluate_formula_panel_json(&request.to_string()).unwrap())
                .unwrap();
        assert_eq!(payload["contract"], "formula.panel.v1");
        assert_eq!(payload["frames"].as_array().unwrap().len(), 2);
        assert_eq!(payload["frames"][0]["symbol"], "AAA");
        assert_eq!(
            payload["frames"][0]["values"]["__PRIMARY__"],
            serde_json::json!([2.0, 3.0])
        );
        assert_eq!(payload["frames"][1]["symbol"], "BBB");
        assert_eq!(
            payload["frames"][1]["values"]["__PRIMARY__"],
            serde_json::json!([11.0, 21.0])
        );

        let duplicate = serde_json::json!({
            "schema_version": FORMULA_PANEL_CONTRACT_SCHEMA_VERSION,
            "source": "CLOSE",
            "dialect": "tdx",
            "frames": [
                frame("AAA", "1m", &[1.0, 1.0]),
                frame("AAA", "1m", &[2.0, 2.0])
            ]
        });
        let error = evaluate_formula_panel_json(&duplicate.to_string()).unwrap_err();
        assert!(error.contains("duplicate formula panel frame"));
    }

    #[test]
    fn cross_sectional_contract_runs_explicit_cs_functions_per_row() {
        let fixture: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/contracts/formula_cross_sectional_contract_v1.json"
        )))
        .unwrap();
        let payload: Value = serde_json::from_str(
            &evaluate_formula_cross_sectional_json(&fixture["request"].to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(payload["contract"], "formula.cross_sectional.v1");
        assert_eq!(
            payload["values"]["__PRIMARY__"],
            fixture["expected"]["primary"]
        );
        assert_eq!(payload["draw"]["mode"], "per_row");
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
