//! Language-neutral execution contract for direct registered operations.
//!
//! This is the control-plane entry point shared by the official bindings. It
//! deliberately accepts JSON so every language can submit the same operation
//! name, ordered inputs, and numeric parameters while high-throughput callers
//! keep using typed indicator APIs.

use crate::shared_runtime::with_unified_engine;
use crate::talib_catalog::{is_profile_catalog_name, TALIB_SEMANTIC_PROFILE};
use finkit::formula::FormulaContext;
use finkit::operation::{OperationRequest, PRIMARY_OUTPUT_NAME};
use ndarray::Array1;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

/// Version of the direct operation result envelope.
pub const OPERATION_RESULT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Deserialize)]
struct OperationRequestJson {
    operation: String,
    #[serde(default = "default_semantic_profile")]
    semantic_profile: String,
    #[serde(default)]
    inputs: BTreeMap<String, Vec<f64>>,
    #[serde(default)]
    input_order: Vec<String>,
    #[serde(default)]
    params: Vec<f64>,
}

fn default_semantic_profile() -> String {
    "core_registry".to_string()
}

/// Execute one registered built-in operation and return its named result JSON.
///
/// This endpoint uses the canonical Core registry semantics (`semantic_profile`
/// is `core_registry`). TA-Lib-specific seed and warm-up variants remain
/// explicit typed operations until the variant registry is wired into this
/// control-plane request; same-spelled formula and TA-Lib functions must not be
/// silently conflated.
///
/// Request shape:
///
/// ```json
/// {
///   "operation": "SMA",
///   "input_order": ["CLOSE"],
///   "inputs": {"CLOSE": [1, 2, 3]},
///   "params": [2]
/// }
/// ```
///
/// The five canonical OHLCV fields are inferred from the supplied inputs only
/// to construct the formula context. The operation's declared input list is
/// still validated by the Core dispatcher, so missing required inputs fail
/// instead of silently producing a different calculation.
pub fn execute_operation_json(request: &str) -> String {
    match execute_operation(request) {
        Ok(value) => value.to_string(),
        Err((code, message)) => json!({
            "schema_version": OPERATION_RESULT_SCHEMA_VERSION,
            "error": {
                "code": code,
                "message": message,
            }
        })
        .to_string(),
    }
}

fn execute_operation(request: &str) -> Result<Value, (&'static str, String)> {
    let request: OperationRequestJson =
        serde_json::from_str(request).map_err(|error| ("invalid_json", error.to_string()))?;
    if request.operation.trim().is_empty() {
        return Err(("invalid_request", "operation must not be empty".to_string()));
    }
    if request.inputs.is_empty() {
        return Err((
            "invalid_request",
            "inputs must contain at least one series".to_string(),
        ));
    }

    let mut inputs = BTreeMap::new();
    let mut length = None;
    for (name, values) in request.inputs {
        let name = normalize_name(&name);
        if name.is_empty() {
            return Err((
                "invalid_request",
                "input names must not be empty".to_string(),
            ));
        }
        if let Some(expected) = length {
            if values.len() != expected {
                return Err((
                    "invalid_request",
                    "all input series must have the same length".to_string(),
                ));
            }
        } else {
            length = Some(values.len());
        }
        if inputs.insert(name.clone(), values).is_some() {
            return Err((
                "invalid_request",
                format!("duplicate input after normalization: {name}"),
            ));
        }
    }
    let length = length.unwrap_or(0);
    if length == 0 {
        return Err((
            "invalid_request",
            "input series must not be empty".to_string(),
        ));
    }

    let input_order = if request.input_order.is_empty() {
        inputs.keys().cloned().collect::<Vec<_>>()
    } else {
        request
            .input_order
            .into_iter()
            .map(|name| normalize_name(&name))
            .collect::<Vec<_>>()
    };
    if input_order.iter().any(String::is_empty) {
        return Err((
            "invalid_request",
            "input_order contains an empty name".to_string(),
        ));
    }
    if input_order.iter().any(|name| !inputs.contains_key(name)) {
        let missing = input_order
            .iter()
            .find(|name| !inputs.contains_key(*name))
            .expect("missing input exists");
        return Err((
            "invalid_request",
            format!("input_order references missing series: {missing}"),
        ));
    }

    let semantic_profile = normalize_profile(&request.semantic_profile);
    if semantic_profile == TALIB_SEMANTIC_PROFILE {
        return execute_talib_profile(&request.operation, &input_order, &inputs, &request.params);
    }
    if semantic_profile != "core_registry" {
        return Err((
            "unsupported_profile",
            format!("unsupported semantic_profile: {}", request.semantic_profile),
        ));
    }

    let fallback = inputs
        .values()
        .next()
        .cloned()
        .ok_or(("invalid_request", "inputs must not be empty".to_string()))?;
    let close = inputs
        .get("CLOSE")
        .cloned()
        .unwrap_or_else(|| fallback.clone());
    let open = inputs.get("OPEN").cloned().unwrap_or_else(|| close.clone());
    let high = inputs.get("HIGH").cloned().unwrap_or_else(|| close.clone());
    let low = inputs.get("LOW").cloned().unwrap_or_else(|| close.clone());
    let volume = inputs
        .get("VOLUME")
        .cloned()
        .unwrap_or_else(|| vec![0.0; length]);
    let mut context = FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
        None,
    );
    for (name, values) in &inputs {
        context.set_variable(name.clone(), Array1::from_vec(values.clone()));
    }

    let input_refs = input_order.iter().map(String::as_str).collect::<Vec<_>>();
    let (canonical_name, operation_id, output_names, result) = with_unified_engine(|engine| {
        let (canonical_name, operation_id, output_names) = {
            let spec = engine
                .catalog()
                .get(&request.operation)
                .ok_or_else(|| ("unknown_operation", request.operation.clone()))?;
            (spec.name.clone(), spec.id().0, spec.output_names.clone())
        };
        let result = engine
            .execute(OperationRequest::Indicator {
                name: &request.operation,
                inputs: &input_refs,
                params: &request.params,
                context: &mut context,
            })
            .map_err(|error| ("execution_error", error.to_string()))?;
        Ok::<_, (&'static str, String)>((canonical_name, operation_id, output_names, result))
    })?;

    let single_output_name = output_names
        .first()
        .cloned()
        .unwrap_or_else(|| canonical_name.clone());
    let mut values = BTreeMap::new();
    for (name, series) in result.values {
        let name = if name == PRIMARY_OUTPUT_NAME {
            single_output_name.clone()
        } else {
            name
        };
        values.insert(name, nullable_series(&series));
    }
    let primary = result.primary.map(|name| {
        if name == PRIMARY_OUTPUT_NAME {
            single_output_name
        } else {
            name
        }
    });
    Ok(json!({
        "schema_version": OPERATION_RESULT_SCHEMA_VERSION,
        "semantic_profile": "core_registry",
        "operation": canonical_name,
        "operation_id": operation_id,
        "shape": value_shape_name(result.shape),
        "primary": primary,
        "values": values,
    }))
}

fn normalize_name(value: &str) -> String {
    value.trim().to_ascii_uppercase()
}

/// Whether the explicit TA-Lib execution profile has a typed dispatcher for
/// this operation. This is also used when projecting binding metadata.
pub fn talib_profile_supported(operation: &str) -> bool {
    let name = normalize_name(operation);
    if name.starts_with("CDL") {
        return candlestick_detector(&name).is_some();
    }
    is_profile_catalog_name(&name) || finkit::formula::ta_lib_function_contract(&name).is_some()
}

fn candlestick_detector(
    name: &str,
) -> Option<
    fn(
        &[f64],
        &[f64],
        &[f64],
        &[f64],
    ) -> std::result::Result<ndarray::Array1<i32>, finkit::error::TaError>,
> {
    use finkit::patterns::candlestick as c;

    match name {
        "CDL2CROWS" => Some(c::cdl_2crows),
        "CDL3BLACKCROWS" => Some(c::cdl_3black_crows),
        "CDL3INSIDE" => Some(c::cdl_3inside),
        "CDL3LINESTRIKE" => Some(c::cdl_3linestrike),
        "CDL3OUTSIDE" => Some(c::cdl_3outside),
        "CDL3STARSINSOUTH" => Some(c::cdl_3starsinsouth),
        "CDL3WHITESOLDIERS" => Some(c::cdl_3white_soldiers),
        "CDLABANDONEDBABY" => Some(c::cdl_abandoned_baby),
        "CDLADVANCEBLOCK" => Some(c::cdl_advanceblock),
        "CDLBELTHOLD" => Some(c::cdl_belthold),
        "CDLBREAKAWAY" => Some(c::cdl_breakaway),
        "CDLCLOSINGMARUBOZU" => Some(c::cdl_closingmarubozu),
        "CDLCONCEALBABYSWALL" => Some(c::cdl_concealbabyswall),
        "CDLCOUNTERATTACK" => Some(c::cdl_counterattack),
        "CDLDARKCLOUDCOVER" => Some(c::cdl_darkcloudcover),
        "CDLDOJI" => Some(c::cdl_doji),
        "CDLDOJISTAR" => Some(c::cdl_doji_star),
        "CDLDRAGONFLYDOJI" => Some(c::cdl_dragonflydoji),
        "CDLENGULFING" => Some(c::cdl_engulfing),
        "CDLEVENINGDOJISTAR" => Some(c::cdl_eveningdojistar),
        "CDLEVENINGSTAR" => Some(c::cdl_eveningstar),
        "CDLGAPSIDESIDEWHITE" => Some(c::cdl_gap_side_white),
        "CDLGRAVESTONEDOJI" => Some(c::cdl_gravestonedoji),
        "CDLHAMMER" => Some(c::cdl_hammer),
        "CDLHANGINGMAN" => Some(c::cdl_hangingman),
        "CDLHARAMI" => Some(c::cdl_harami),
        "CDLHARAMICROSS" => Some(c::cdl_haramicross),
        "CDLHIGHWAVE" => Some(c::cdl_highwave),
        "CDLHIKKAKE" => Some(c::cdl_hikkake),
        "CDLHIKKAKEMOD" => Some(c::cdl_hikkake_mod),
        "CDLHOMINGPIGEON" => Some(c::cdl_homing_pigeon),
        "CDLIDENTICAL3CROWS" => Some(c::cdl_identical3crows),
        "CDLINNECK" => Some(c::cdl_inneck),
        "CDLINVERTEDHAMMER" => Some(c::cdl_invertedhammer),
        "CDLKICKING" => Some(c::cdl_kicking),
        "CDLKICKINGBYLENGTH" => Some(c::cdl_kickingbylength),
        "CDLLADDERBOTTOM" => Some(c::cdl_ladder_bottom),
        "CDLLONGLEGGEDDOJI" => Some(c::cdl_longleggeddoji),
        "CDLLONGLINE" => Some(c::cdl_longline),
        "CDLMARUBOZU" => Some(c::cdl_marubozu),
        "CDLMATCHINGLOW" => Some(c::cdl_matchinglow),
        "CDLMATHOLD" => Some(c::cdl_mathold),
        "CDLMORNINGDOJISTAR" => Some(c::cdl_morningdojistar),
        "CDLMORNINGSTAR" => Some(c::cdl_morningstar),
        "CDLONNECK" => Some(c::cdl_onneck),
        "CDLPIERCING" => Some(c::cdl_piercing),
        "CDLRICKSHAWMAN" => Some(c::cdl_rickshawman),
        "CDLRISEFALL3METHODS" => Some(c::cdl_rise_fall_3methods),
        "CDLSEPARATINGLINES" => Some(c::cdl_separatinglines),
        "CDLSHOOTINGSTAR" => Some(c::cdl_shootingstar),
        "CDLSHORTLINE" => Some(c::cdl_shortline),
        "CDLSPINNINGTOP" => Some(c::cdl_spinningtop),
        "CDLSTALLEDPATTERN" => Some(c::cdl_stalledpattern),
        "CDLSTICKSANDWICH" => Some(c::cdl_sticksandwich),
        "CDLTAKURI" => Some(c::cdl_takuri),
        "CDLTASUKIGAP" => Some(c::cdl_tasukigap),
        "CDLTHRUSTING" => Some(c::cdl_thrusting),
        "CDLTRISTAR" => Some(c::cdl_tristar),
        "CDLUNIQUE3RIVER" => Some(c::cdl_unique3river),
        "CDLUPSIDEGAP2CROWS" => Some(c::cdl_upsidegap2crows),
        "CDLXSIDEGAP3METHODS" => Some(c::cdl_xsidegap3methods),
        _ => None,
    }
}

fn normalize_profile(value: &str) -> String {
    let profile = value.trim().to_ascii_lowercase();
    // Existing in-tree unit fixtures are intentionally kept as historical
    // regression inputs, but the compatibility alias is compiled only for
    // tests. Production bindings expose and accept the latest profile only.
    #[cfg(test)]
    if profile == "talib_0_7_1" {
        return TALIB_SEMANTIC_PROFILE.to_string();
    }
    profile
}

fn execute_talib_profile(
    operation: &str,
    input_order: &[String],
    inputs: &BTreeMap<String, Vec<f64>>,
    params: &[f64],
) -> Result<Value, (&'static str, String)> {
    let name = normalize_name(operation);
    if !talib_profile_supported(&name) {
        return Err((
            "unsupported_operation",
            format!("TA-Lib profile does not yet dispatch {name}"),
        ));
    }
    let contract = crate::operation::talib_profile_contract(&name).ok_or((
        "internal_contract_error",
        format!("missing TA-Lib profile contract for {name}"),
    ))?;
    if params.len() > contract.params.len() {
        return Err((
            "invalid_request",
            format!(
                "{name} accepts at most {} parameters, received {}",
                contract.params.len(),
                params.len()
            ),
        ));
    }
    let mut values = BTreeMap::new();
    let primary = match name.as_str() {
        "AC" => {
            let h = named_series(inputs, "HIGH", &name)?;
            let l = named_series(inputs, "LOW", &name)?;
            let r = finkit::indicators::talib_ext::ac(
                h,
                l,
                parameter_usize(params, 0, 5, &name)?,
                parameter_usize(params, 1, 34, &name)?,
                parameter_usize(params, 2, 5, &name)?,
            )
            .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("REAL".to_string(), r.to_vec());
            "REAL"
        }
        "ADR" => {
            let h = named_series(inputs, "HIGH", &name)?;
            let l = named_series(inputs, "LOW", &name)?;
            let r =
                finkit::indicators::talib_ext::adr(h, l, parameter_usize(params, 0, 14, &name)?)
                    .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("REAL".to_string(), r.to_vec());
            "REAL"
        }
        "CMOU" => {
            let x = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let r = finkit::indicators::talib_ext::cmou(x, parameter_usize(params, 0, 14, &name)?)
                .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("REAL".to_string(), r.to_vec());
            "REAL"
        }
        "CVI" => {
            let h = named_series(inputs, "HIGH", &name)?;
            let l = named_series(inputs, "LOW", &name)?;
            let r = finkit::indicators::talib_ext::cvi(
                h,
                l,
                parameter_usize(params, 0, 10, &name)?,
                parameter_usize(params, 1, 10, &name)?,
            )
            .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("REAL".to_string(), r.to_vec());
            "REAL"
        }
        "EFI" => {
            let c = named_series(inputs, "CLOSE", &name)?;
            let v = named_series(inputs, "VOLUME", &name)?;
            let r =
                finkit::indicators::talib_ext::efi(c, v, parameter_usize(params, 0, 13, &name)?)
                    .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("REAL".to_string(), r.to_vec());
            "REAL"
        }
        "ERI" => {
            let h = named_series(inputs, "HIGH", &name)?;
            let l = named_series(inputs, "LOW", &name)?;
            let c = named_series(inputs, "CLOSE", &name)?;
            let r =
                finkit::indicators::talib_ext::eri(h, l, c, parameter_usize(params, 0, 13, &name)?)
                    .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("BULLPOWER".to_string(), r.bullpower.to_vec());
            values.insert("BEARPOWER".to_string(), r.bearpower.to_vec());
            "BULLPOWER"
        }
        "FOSC" => {
            let x = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let r = finkit::indicators::talib_ext::fosc(x, parameter_usize(params, 0, 5, &name)?)
                .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("REAL".to_string(), r.to_vec());
            "REAL"
        }
        "FRACTAL" => {
            let h = named_series(inputs, "HIGH", &name)?;
            let l = named_series(inputs, "LOW", &name)?;
            let r = finkit::indicators::talib_ext::fractal(
                h,
                l,
                parameter_usize(params, 0, 2, &name)?,
                parameter_usize(params, 1, 2, &name)?,
            )
            .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            // TA-Lib pattern-style outputs use zero for bars before a
            // confirmed pivot. The extension kernel uses NaN for those
            // undefined rows, so normalize only at this compatibility
            // boundary rather than changing the Core API.
            values.insert(
                "SWINGHIGH".to_string(),
                r.swinghigh
                    .iter()
                    .map(|value| if value.is_finite() { *value } else { 0.0 })
                    .collect(),
            );
            values.insert(
                "SWINGLOW".to_string(),
                r.swinglow
                    .iter()
                    .map(|value| if value.is_finite() { *value } else { 0.0 })
                    .collect(),
            );
            "SWINGHIGH"
        }
        "KC" => {
            let h = named_series(inputs, "HIGH", &name)?;
            let l = named_series(inputs, "LOW", &name)?;
            let c = named_series(inputs, "CLOSE", &name)?;
            let r = finkit::indicators::talib_ext::kc(
                h,
                l,
                c,
                parameter_usize(params, 0, 20, &name)?,
                parameter_usize(params, 1, 10, &name)?,
                parameter_f64(params, 2, 2.0, &name)?,
            )
            .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("UPPERBAND".to_string(), r.upperband.to_vec());
            values.insert("MIDDLEBAND".to_string(), r.middleband.to_vec());
            values.insert("LOWERBAND".to_string(), r.lowerband.to_vec());
            "MIDDLEBAND"
        }
        "KDJ" => {
            let h = named_series(inputs, "HIGH", &name)?;
            let l = named_series(inputs, "LOW", &name)?;
            let c = named_series(inputs, "CLOSE", &name)?;
            let r = finkit::indicators::talib_ext::kdj(
                h,
                l,
                c,
                parameter_usize(params, 0, 9, &name)?,
                parameter_usize(params, 1, 3, &name)?,
                parameter_usize(params, 2, 13, &name)?,
                parameter_usize(params, 3, 3, &name)?,
                parameter_usize(params, 4, 13, &name)?,
            )
            .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("K".to_string(), r.k.to_vec());
            values.insert("D".to_string(), r.d.to_vec());
            values.insert("J".to_string(), r.j.to_vec());
            "K"
        }
        "MARKETFI" => {
            let h = named_series(inputs, "HIGH", &name)?;
            let l = named_series(inputs, "LOW", &name)?;
            let v = named_series(inputs, "VOLUME", &name)?;
            let r = finkit::indicators::talib_ext::marketfi(h, l, v)
                .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("REAL".to_string(), r.to_vec());
            "REAL"
        }
        "MASSI" => {
            let h = named_series(inputs, "HIGH", &name)?;
            let l = named_series(inputs, "LOW", &name)?;
            let r = finkit::indicators::talib_ext::massi(
                h,
                l,
                parameter_usize(params, 0, 9, &name)?,
                parameter_usize(params, 1, 25, &name)?,
            )
            .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("REAL".to_string(), r.to_vec());
            "REAL"
        }
        "PERCENTILE" => {
            let x = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let r = finkit::indicators::talib_ext::percentile(
                x,
                parameter_usize(params, 0, 30, &name)?,
                parameter_f64(params, 1, 50.0, &name)?,
            )
            .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("REAL".to_string(), r.to_vec());
            "REAL"
        }
        "PVO" => {
            let x = named_series(inputs, "VOLUME", &name)?;
            let r = finkit::indicators::talib_ext::pvo(
                x,
                parameter_usize(params, 0, 12, &name)?,
                parameter_usize(params, 1, 26, &name)?,
                parameter_usize(params, 2, 1, &name)?,
            )
            .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("REAL".to_string(), r.to_vec());
            "REAL"
        }
        "QSTICK" => {
            let o = named_series(inputs, "OPEN", &name)?;
            let c = named_series(inputs, "CLOSE", &name)?;
            let r =
                finkit::indicators::talib_ext::qstick(o, c, parameter_usize(params, 0, 10, &name)?)
                    .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("REAL".to_string(), r.to_vec());
            "REAL"
        }
        "RMA" => {
            let x = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let r = finkit::indicators::talib_ext::rma_profile(
                x,
                parameter_usize(params, 0, 30, &name)?,
            )
            .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("REAL".to_string(), r.to_vec());
            "REAL"
        }
        "RVI" => {
            let x = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let r = finkit::indicators::talib_ext::rvi_profile(
                x,
                parameter_usize(params, 0, 14, &name)?,
                parameter_usize(params, 1, 10, &name)?,
            )
            .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("REAL".to_string(), r.to_vec());
            "REAL"
        }
        "RVOL" => {
            let x = named_series(inputs, "VOLUME", &name)?;
            let r = finkit::indicators::talib_ext::rvol(x, parameter_usize(params, 0, 20, &name)?)
                .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("REAL".to_string(), r.to_vec());
            "REAL"
        }
        "SMI" => {
            let h = named_series(inputs, "HIGH", &name)?;
            let l = named_series(inputs, "LOW", &name)?;
            let c = named_series(inputs, "CLOSE", &name)?;
            let r = finkit::indicators::talib_ext::smi(
                h,
                l,
                c,
                parameter_usize(params, 0, 13, &name)?,
                parameter_usize(params, 1, 2, &name)?,
                parameter_usize(params, 2, 25, &name)?,
                parameter_usize(params, 3, 9, &name)?,
            )
            .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("SMI".to_string(), r.smi.to_vec());
            values.insert("SMISIGNAL".to_string(), r.smisignal.to_vec());
            "SMI"
        }
        "VHF" => {
            let x = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let r = finkit::indicators::talib_ext::vhf(x, parameter_usize(params, 0, 28, &name)?)
                .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert("REAL".to_string(), r.to_vec());
            "REAL"
        }
        "WAD" => {
            let h = named_series(inputs, "HIGH", &name)?;
            let l = named_series(inputs, "LOW", &name)?;
            let c = named_series(inputs, "CLOSE", &name)?;
            let r = finkit::indicators::talib_ext::wad(h, l, c)
                .map_err(|e| ("execution_error", format!("{name}: {e}")))?;
            values.insert(
                "REAL".to_string(),
                r.iter()
                    .map(|value| if value.is_finite() { *value } else { 0.0 })
                    .collect(),
            );
            "REAL"
        }
        "MA" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 30, &name)?;
            let ma_type = talib_ma_type(params, 1, 0, &name)?;
            values.insert(
                "MA".to_string(),
                indicator_values(
                    finkit::indicators::overlap::ma(input, period, ma_type),
                    &name,
                )?,
            );
            "MA"
        }
        "SMA" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 30, &name)?;
            values.insert(
                "SMA".to_string(),
                indicator_values(finkit::math::moving_avg::sma(input, period), &name)?,
            );
            "SMA"
        }
        "EMA" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 30, &name)?;
            values.insert(
                "EMA".to_string(),
                indicator_values(finkit::math::moving_avg::ema(input, period), &name)?,
            );
            "EMA"
        }
        "WMA" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 30, &name)?;
            values.insert(
                "WMA".to_string(),
                indicator_values(finkit::math::moving_avg::wma(input, period), &name)?,
            );
            "WMA"
        }
        "DEMA" | "TEMA" | "TRIMA" | "KAMA" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 30, &name)?;
            let series = match name.as_str() {
                "DEMA" => finkit::math::moving_avg::dema(input, period),
                "TEMA" => finkit::math::moving_avg::tema(input, period),
                "TRIMA" => finkit::math::moving_avg::trima(input, period),
                "KAMA" => finkit::math::moving_avg::kama(input, period, 2, 30),
                _ => unreachable!(),
            };
            values.insert(name.clone(), indicator_values(series, &name)?);
            name.as_str()
        }
        "T3" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 5, &name)?;
            let vfactor = parameter_f64(params, 1, 0.7, &name)?;
            values.insert(
                "T3".to_string(),
                indicator_values(
                    finkit::indicators::overlap::t3(input, period, vfactor),
                    &name,
                )?,
            );
            "T3"
        }
        "MAMA" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let fast_limit = parameter_f64(params, 0, 0.5, &name)?;
            let slow_limit = parameter_f64(params, 1, 0.05, &name)?;
            let output = finkit::indicators::overlap::mama(input, fast_limit, slow_limit)
                .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("MAMA".to_string(), output.mama.to_vec());
            values.insert("FAMA".to_string(), output.fama.to_vec());
            "MAMA"
        }
        "MAVP" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let periods = ordered_series(inputs, input_order, 1, "PERIODS", &name)?;
            let min_period = parameter_usize(params, 0, 2, &name)?;
            let max_period = parameter_usize(params, 1, 30, &name)?;
            let ma_type = talib_ma_type(params, 2, 0, &name)?;
            values.insert(
                "MAVP".to_string(),
                indicator_values(
                    finkit::indicators::overlap::mavp_with_ma_type(
                        input, periods, min_period, max_period, ma_type,
                    ),
                    &name,
                )?,
            );
            "MAVP"
        }
        "ACCBANDS" => {
            let high = named_series(inputs, "HIGH", &name)?;
            let low = named_series(inputs, "LOW", &name)?;
            let close = named_series(inputs, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 20, &name)?;
            let output = finkit::indicators::overlap::accbands(high, low, close, period)
                .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("UPPERBAND".to_string(), output.upper.to_vec());
            values.insert("MIDDLEBAND".to_string(), output.middle.to_vec());
            values.insert("LOWERBAND".to_string(), output.lower.to_vec());
            "MIDDLEBAND"
        }
        "AVGDEV" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "AVGDEV".to_string(),
                indicator_values(finkit::indicators::statistics::avgdev(input, period), &name)?,
            );
            "AVGDEV"
        }
        "IMI" => {
            let open = named_series(inputs, "OPEN", &name)?;
            let close = named_series(inputs, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "IMI".to_string(),
                indicator_values(
                    finkit::indicators::momentum_ext::imi(open, close, period),
                    &name,
                )?,
            );
            "IMI"
        }
        "AO" => {
            let high = named_series(inputs, "HIGH", &name)?;
            let low = named_series(inputs, "LOW", &name)?;
            let fast = parameter_usize(params, 0, 5, &name)?;
            let slow = parameter_usize(params, 1, 34, &name)?;
            values.insert(
                "AO".to_string(),
                indicator_values(
                    finkit::indicators::momentum_ext::ao(high, low, fast, slow),
                    &name,
                )?,
            );
            "AO"
        }
        "CMF" => {
            let high = named_series(inputs, "HIGH", &name)?;
            let low = named_series(inputs, "LOW", &name)?;
            let close = named_series(inputs, "CLOSE", &name)?;
            let volume = named_series(inputs, "VOLUME", &name)?;
            let period = parameter_usize(params, 0, 20, &name)?;
            values.insert(
                "CMF".to_string(),
                indicator_values(
                    finkit::indicators::volume_ext::cmf(high, low, close, volume, period),
                    &name,
                )?,
            );
            "CMF"
        }
        "COPPOCK" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let wma_period = parameter_usize(params, 0, 10, &name)?;
            let roc1_period = parameter_usize(params, 1, 11, &name)?;
            let roc2_period = parameter_usize(params, 2, 14, &name)?;
            values.insert(
                "COPPOCK".to_string(),
                indicator_values(
                    finkit::indicators::momentum_ext::coppock(
                        input,
                        wma_period,
                        roc1_period,
                        roc2_period,
                    ),
                    &name,
                )?,
            );
            "COPPOCK"
        }
        "CUMSUM" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let mut output = Vec::with_capacity(input.len());
            let mut sum = 0.0;
            for value in input {
                sum += *value;
                output.push(sum);
            }
            values.insert("CUMSUM".to_string(), output);
            "CUMSUM"
        }
        "DONCHIAN" => {
            let high = named_series(inputs, "HIGH", &name)?;
            let low = named_series(inputs, "LOW", &name)?;
            let period = parameter_usize(params, 0, 20, &name)?;
            let output = finkit::indicators::donchian::donchian(high, low, period)
                .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("UPPERBAND".to_string(), output.upper.to_vec());
            values.insert("MIDDLEBAND".to_string(), output.middle.to_vec());
            values.insert("LOWERBAND".to_string(), output.lower.to_vec());
            "MIDDLEBAND"
        }
        "DPO" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 20, &name)?;
            values.insert(
                "DPO".to_string(),
                indicator_values(finkit::indicators::china::dpo(input, period), &name)?,
            );
            "DPO"
        }
        "ER" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 10, &name)?;
            values.insert(
                "ER".to_string(),
                indicator_values(
                    finkit::indicators::overlap::efficiency_ratio(input, period),
                    &name,
                )?,
            );
            "ER"
        }
        "HA" => {
            let open = named_series(inputs, "OPEN", &name)?;
            let high = named_series(inputs, "HIGH", &name)?;
            let low = named_series(inputs, "LOW", &name)?;
            let close = named_series(inputs, "CLOSE", &name)?;
            let output = finkit::indicators::chart::heikin_ashi(open, high, low, close)
                .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("HAOPEN".to_string(), output.ha_open.to_vec());
            values.insert("HAHIGH".to_string(), output.ha_high.to_vec());
            values.insert("HALOW".to_string(), output.ha_low.to_vec());
            values.insert("HACLOSE".to_string(), output.ha_close.to_vec());
            "HACLOSE"
        }
        "HMA" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 20, &name)?;
            values.insert(
                "HMA".to_string(),
                indicator_values(finkit::indicators::overlap::hma(input, period), &name)?,
            );
            "HMA"
        }
        "NVI" | "PVI" => {
            let close = named_series(inputs, "CLOSE", &name)?;
            let volume = named_series(inputs, "VOLUME", &name)?;
            let result = if name == "NVI" {
                finkit::indicators::volume_ext::nvi(close, volume)
            } else {
                finkit::indicators::volume_ext::pvi(close, volume)
            };
            values.insert(name.clone(), indicator_values(result, &name)?);
            name.as_str()
        }
        "PERCENTRANK" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 100, &name)?;
            values.insert(
                "PERCENTRANK".to_string(),
                indicator_values(
                    finkit::indicators::statistics::percent_rank(input, period),
                    &name,
                )?,
            );
            "PERCENTRANK"
        }
        "PVT" => {
            let close = named_series(inputs, "CLOSE", &name)?;
            let volume = named_series(inputs, "VOLUME", &name)?;
            values.insert(
                "PVT".to_string(),
                indicator_values(finkit::indicators::volume_ext::pvt(close, volume), &name)?,
            );
            "PVT"
        }
        "SUPERTREND" => {
            let high = named_series(inputs, "HIGH", &name)?;
            let low = named_series(inputs, "LOW", &name)?;
            let close = named_series(inputs, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 10, &name)?;
            let multiplier = parameter_f64(params, 1, 3.0, &name)?;
            let output =
                finkit::indicators::supertrend::supertrend(high, low, close, period, multiplier)
                    .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("SUPERTREND".to_string(), output.trend_line.to_vec());
            values.insert(
                "TREND".to_string(),
                output.direction.iter().map(|value| *value as f64).collect(),
            );
            "SUPERTREND"
        }
        "TSI" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let first = parameter_usize(params, 0, 25, &name)?;
            let second = parameter_usize(params, 1, 13, &name)?;
            values.insert(
                "TSI".to_string(),
                indicator_values(
                    finkit::indicators::momentum_ext::tsi(input, first, second),
                    &name,
                )?,
            );
            "TSI"
        }
        "VORTEX" => {
            let high = named_series(inputs, "HIGH", &name)?;
            let low = named_series(inputs, "LOW", &name)?;
            let close = named_series(inputs, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            let output = finkit::indicators::momentum_ext::vortex(high, low, close, period)
                .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("PLUSVI".to_string(), output.vi_plus.to_vec());
            values.insert("MINUSVI".to_string(), output.vi_minus.to_vec());
            "PLUSVI"
        }
        "VWAP" => {
            let high = named_series(inputs, "HIGH", &name)?;
            let low = named_series(inputs, "LOW", &name)?;
            let close = named_series(inputs, "CLOSE", &name)?;
            let volume = named_series(inputs, "VOLUME", &name)?;
            values.insert(
                "VWAP".to_string(),
                indicator_values(
                    finkit::indicators::volume::vwap(high, low, close, volume),
                    &name,
                )?,
            );
            "VWAP"
        }
        "VWMA" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let volume = named_series(inputs, "VOLUME", &name)?;
            let period = parameter_usize(params, 0, 30, &name)?;
            values.insert(
                "VWMA".to_string(),
                indicator_values(finkit::math::moving_avg::vwma(input, volume, period), &name)?,
            );
            "VWMA"
        }
        "ZLEMA" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 30, &name)?;
            values.insert(
                "ZLEMA".to_string(),
                indicator_values(finkit::math::moving_avg::zlema(input, period), &name)?,
            );
            "ZLEMA"
        }
        "SAREXT" => {
            let high = named_series(inputs, "HIGH", &name)?;
            let low = named_series(inputs, "LOW", &name)?;
            let start_value = parameter_f64(params, 0, 0.0, &name)?;
            let offset_on_reverse = parameter_f64(params, 1, 0.0, &name)?;
            let af_init_long = parameter_f64(params, 2, 0.02, &name)?;
            let af_long = parameter_f64(params, 3, 0.02, &name)?;
            let af_max_long = parameter_f64(params, 4, 0.2, &name)?;
            let af_init_short = parameter_f64(params, 5, 0.02, &name)?;
            let af_short = parameter_f64(params, 6, 0.02, &name)?;
            let af_max_short = parameter_f64(params, 7, 0.2, &name)?;
            let output = finkit::indicators::overlap::sarext(
                high,
                low,
                start_value,
                offset_on_reverse,
                af_init_long,
                af_long,
                af_max_long,
                af_init_short,
                af_short,
                af_max_short,
            )
            .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert(
                "SAREXT".to_string(),
                output
                    .sar
                    .iter()
                    .enumerate()
                    .map(|(index, value)| if index == 0 { f64::NAN } else { *value })
                    .collect(),
            );
            "SAREXT"
        }
        "HT_DCPERIOD" | "HT_DCPHASE" | "HT_TRENDMODE" | "HT_TRENDLINE" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let series = match name.as_str() {
                "HT_DCPERIOD" => finkit::indicators::cycle::ht_dcperiod(input),
                "HT_DCPHASE" => finkit::indicators::cycle::ht_dcphase(input),
                "HT_TRENDMODE" => finkit::indicators::cycle::ht_trendmode(input),
                "HT_TRENDLINE" => finkit::indicators::cycle::ht_trendline(input),
                _ => unreachable!(),
            };
            let output = indicator_values(series, &name)?;
            let output = if name == "HT_TRENDMODE" {
                output
                    .into_iter()
                    .map(|value| if value.is_finite() { value } else { 0.0 })
                    .collect()
            } else {
                output
            };
            values.insert(name.clone(), output);
            name.as_str()
        }
        "HT_PHASOR" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let output = finkit::indicators::cycle::ht_phasor(input)
                .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("INPHASE".to_string(), output.0.to_vec());
            values.insert("QUADRATURE".to_string(), output.1.to_vec());
            "INPHASE"
        }
        "HT_SINE" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let output = finkit::indicators::cycle::ht_sine(input)
                .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("SINE".to_string(), output.0.to_vec());
            values.insert("LEADSINE".to_string(), output.1.to_vec());
            "SINE"
        }
        "CDLDOJI" | "CDLDRAGONFLYDOJI" | "CDLGRAVESTONEDOJI" | "CDLENGULFING" | "CDLHAMMER"
        | "CDLHANGINGMAN" | "CDLHARAMI" | "CDLMARUBOZU" | "CDLPIERCING" | "CDLSHOOTINGSTAR"
        | "CDLSPINNINGTOP" => {
            let (open, high, low, close) = ohlc(inputs, &name)?;
            let result = match name.as_str() {
                "CDLDOJI" => finkit::patterns::candlestick::cdl_doji(open, high, low, close),
                "CDLDRAGONFLYDOJI" => {
                    finkit::patterns::candlestick::cdl_dragonflydoji(open, high, low, close)
                }
                "CDLGRAVESTONEDOJI" => {
                    finkit::patterns::candlestick::cdl_gravestonedoji(open, high, low, close)
                }
                "CDLENGULFING" => {
                    finkit::patterns::candlestick::cdl_engulfing(open, high, low, close)
                }
                "CDLHAMMER" => finkit::patterns::candlestick::cdl_hammer(open, high, low, close),
                "CDLHANGINGMAN" => {
                    finkit::patterns::candlestick::cdl_hangingman(open, high, low, close)
                }
                "CDLHARAMI" => finkit::patterns::candlestick::cdl_harami(open, high, low, close),
                "CDLMARUBOZU" => {
                    finkit::patterns::candlestick::cdl_marubozu(open, high, low, close)
                }
                "CDLPIERCING" => {
                    finkit::patterns::candlestick::cdl_piercing(open, high, low, close)
                }
                "CDLSHOOTINGSTAR" => {
                    finkit::patterns::candlestick::cdl_shootingstar(open, high, low, close)
                }
                "CDLSPINNINGTOP" => {
                    finkit::patterns::candlestick::cdl_spinningtop(open, high, low, close)
                }
                _ => unreachable!(),
            };
            values.insert(name.clone(), pattern_values(result, &name)?);
            name.as_str()
        }
        pattern if pattern.starts_with("CDL") => {
            let detector = candlestick_detector(pattern).ok_or((
                "unsupported_operation",
                format!("TA-Lib profile does not yet dispatch {pattern}"),
            ))?;
            let (open, high, low, close) = ohlc(inputs, &name)?;
            values.insert(
                name.clone(),
                pattern_values(detector(open, high, low, close), &name)?,
            );
            name.as_str()
        }
        "RSI" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "RSI".to_string(),
                indicator_values(finkit::indicators::momentum::rsi(input, period), &name)?,
            );
            "RSI"
        }
        "MACDEXT" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let fast = parameter_usize(params, 0, 12, &name)?;
            let fast_type = talib_ma_type(params, 1, 0, &name)?;
            let slow = parameter_usize(params, 2, 26, &name)?;
            let slow_type = talib_ma_type(params, 3, 0, &name)?;
            let signal = parameter_usize(params, 4, 9, &name)?;
            let signal_type = talib_ma_type(params, 5, 0, &name)?;
            let output = finkit::indicators::momentum::macdext(
                input,
                fast,
                fast_type,
                slow,
                slow_type,
                signal,
                signal_type,
            )
            .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("MACD".to_string(), output.macd.to_vec());
            values.insert("MACD_SIGNAL".to_string(), output.signal.to_vec());
            values.insert("MACD_HIST".to_string(), output.hist.to_vec());
            "MACD"
        }
        "MACDFIX" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let signal = parameter_usize(params, 0, 9, &name)?;
            let output = finkit::indicators::momentum::macdfix_with_signal(input, signal)
                .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("MACD".to_string(), output.macd.to_vec());
            values.insert("MACD_SIGNAL".to_string(), output.signal.to_vec());
            values.insert("MACD_HIST".to_string(), output.hist.to_vec());
            "MACD"
        }
        "MACD" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let fast = parameter_usize(params, 0, 12, &name)?;
            let slow = parameter_usize(params, 1, 26, &name)?;
            let signal = parameter_usize(params, 2, 9, &name)?;
            let output = finkit::indicators::momentum::macd(input, fast, slow, signal)
                .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("MACD".to_string(), output.macd.to_vec());
            values.insert("MACD_SIGNAL".to_string(), output.signal.to_vec());
            values.insert("MACD_HIST".to_string(), output.hist.to_vec());
            "MACD"
        }
        "BBANDS" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 20, &name)?;
            let deviation_up = parameter_f64(params, 1, 2.0, &name)?;
            let deviation_dn = parameter_f64(params, 2, 2.0, &name)?;
            let ma_type = talib_ma_type(params, 3, 0, &name)?;
            let output = finkit::indicators::overlap::bbands_with_ma_type(
                input,
                period,
                deviation_up,
                deviation_dn,
                ma_type,
            )
            .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("UPPERBAND".to_string(), output.upper.to_vec());
            values.insert("MIDDLEBAND".to_string(), output.middle.to_vec());
            values.insert("LOWERBAND".to_string(), output.lower.to_vec());
            "MIDDLEBAND"
        }
        "ATR" => {
            let (high, low, close) = hlc(inputs, &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "ATR".to_string(),
                indicator_values(
                    finkit::indicators::volatility::atr(high, low, close, period),
                    &name,
                )?,
            );
            "ATR"
        }
        "NATR" => {
            let (high, low, close) = hlc(inputs, &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "NATR".to_string(),
                indicator_values(
                    finkit::indicators::volatility::natr(high, low, close, period),
                    &name,
                )?,
            );
            "NATR"
        }
        "TRANGE" => {
            let (high, low, close) = hlc(inputs, &name)?;
            values.insert(
                "TRANGE".to_string(),
                indicator_values(
                    finkit::indicators::volatility::trange(high, low, close),
                    &name,
                )?,
            );
            "TRANGE"
        }
        "ADX" => {
            let (high, low, close) = hlc(inputs, &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "ADX".to_string(),
                indicator_values(
                    finkit::indicators::momentum::adx(high, low, close, period),
                    &name,
                )?,
            );
            "ADX"
        }
        "ADXR" => {
            let (high, low, close) = hlc(inputs, &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "ADXR".to_string(),
                indicator_values(
                    finkit::indicators::momentum::adxr(high, low, close, period),
                    &name,
                )?,
            );
            "ADXR"
        }
        "DX" => {
            let (high, low, close) = hlc(inputs, &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "DX".to_string(),
                indicator_values(
                    finkit::indicators::momentum::dx(high, low, close, period),
                    &name,
                )?,
            );
            "DX"
        }
        "PLUS_DI" => {
            let (high, low, close) = hlc(inputs, &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "PLUS_DI".to_string(),
                indicator_values(
                    finkit::indicators::momentum::plus_di(high, low, close, period),
                    &name,
                )?,
            );
            "PLUS_DI"
        }
        "MINUS_DI" => {
            let (high, low, close) = hlc(inputs, &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "MINUS_DI".to_string(),
                indicator_values(
                    finkit::indicators::momentum::minus_di(high, low, close, period),
                    &name,
                )?,
            );
            "MINUS_DI"
        }
        "CCI" => {
            let (high, low, close) = hlc(inputs, &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "CCI".to_string(),
                indicator_values(
                    finkit::indicators::momentum::cci(high, low, close, period),
                    &name,
                )?,
            );
            "CCI"
        }
        "AROON" => {
            let (high, low, _) = hlc(inputs, &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            let output = finkit::indicators::momentum::aroon(high, low, period)
                .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("AROON_UP".to_string(), output.aroon_up.to_vec());
            values.insert("AROON_DOWN".to_string(), output.aroon_down.to_vec());
            "AROON_UP"
        }
        "AROONOSC" => {
            let (high, low, _) = hlc(inputs, &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "AROONOSC".to_string(),
                indicator_values(
                    finkit::indicators::momentum::aroonosc(high, low, period),
                    &name,
                )?,
            );
            "AROONOSC"
        }
        "APO" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let fast = parameter_usize(params, 0, 12, &name)?;
            let slow = parameter_usize(params, 1, 26, &name)?;
            let ma_type = talib_ma_type(params, 2, 0, &name)?;
            values.insert(
                "APO".to_string(),
                indicator_values(
                    finkit::indicators::momentum::apo_with_ma_type(input, fast, slow, ma_type),
                    &name,
                )?,
            );
            "APO"
        }
        "BOP" => {
            let (open, high, low, close) = ohlc(inputs, &name)?;
            values.insert(
                "BOP".to_string(),
                indicator_values(
                    finkit::indicators::momentum::bop(open, high, low, close),
                    &name,
                )?,
            );
            "BOP"
        }
        "TRIX" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 30, &name)?;
            values.insert(
                "TRIX".to_string(),
                indicator_values(finkit::indicators::momentum::trix(input, period), &name)?,
            );
            "TRIX"
        }
        "CMO" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "CMO".to_string(),
                indicator_values(finkit::indicators::momentum::cmo(input, period), &name)?,
            );
            "CMO"
        }
        "STOCH" => {
            let (high, low, close) = hlc(inputs, &name)?;
            let fastk_period = parameter_usize(params, 0, 5, &name)?;
            let slowk_period = parameter_usize(params, 1, 3, &name)?;
            let slowk_ma_type = talib_ma_type(params, 2, 0, &name)?;
            let slowd_period = parameter_usize(params, 3, 3, &name)?;
            let slowd_ma_type = talib_ma_type(params, 4, 0, &name)?;
            let output = finkit::indicators::momentum::stoch_with_ma_types(
                high,
                low,
                close,
                fastk_period,
                slowk_period,
                slowk_ma_type,
                slowd_period,
                slowd_ma_type,
            )
            .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("SLOWK".to_string(), output.k.to_vec());
            values.insert("SLOWD".to_string(), output.d.to_vec());
            "SLOWK"
        }
        "STOCHF" => {
            let (high, low, close) = hlc(inputs, &name)?;
            let fast_k = parameter_usize(params, 0, 5, &name)?;
            let fast_d = parameter_usize(params, 1, 3, &name)?;
            let fast_d_ma_type = talib_ma_type(params, 2, 0, &name)?;
            let output = finkit::indicators::momentum::stochf_with_ma_type(
                high,
                low,
                close,
                fast_k,
                fast_d,
                fast_d_ma_type,
            )
            .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("FASTK".to_string(), output.k.to_vec());
            values.insert("FASTD".to_string(), output.d.to_vec());
            "FASTK"
        }
        "STOCHRSI" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let timeperiod = parameter_usize(params, 0, 14, &name)?;
            let fast_k = parameter_usize(params, 1, 5, &name)?;
            let fast_d = parameter_usize(params, 2, 3, &name)?;
            let fast_d_ma_type = talib_ma_type(params, 3, 0, &name)?;
            let output = finkit::indicators::momentum::stochrsi_with_ma_type(
                input,
                timeperiod,
                fast_k,
                fast_d,
                fast_d_ma_type,
            )
            .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("FASTK".to_string(), output.k.to_vec());
            values.insert("FASTD".to_string(), output.d.to_vec());
            "FASTK"
        }
        "WILLR" => {
            let (high, low, close) = hlc(inputs, &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "WILLR".to_string(),
                indicator_values(
                    finkit::indicators::momentum::willr(high, low, close, period),
                    &name,
                )?,
            );
            "WILLR"
        }
        "MOM" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 10, &name)?;
            values.insert(
                "MOM".to_string(),
                indicator_values(finkit::indicators::momentum::mom(input, period), &name)?,
            );
            "MOM"
        }
        "ROC" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 10, &name)?;
            values.insert(
                "ROC".to_string(),
                indicator_values(finkit::indicators::momentum::roc(input, period), &name)?,
            );
            "ROC"
        }
        "ROCP" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 10, &name)?;
            values.insert(
                "ROCP".to_string(),
                indicator_values(finkit::indicators::momentum::rocp(input, period), &name)?,
            );
            "ROCP"
        }
        "ROCR" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 10, &name)?;
            values.insert(
                "ROCR".to_string(),
                indicator_values(finkit::indicators::momentum::rocr(input, period), &name)?,
            );
            "ROCR"
        }
        "ROCR100" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 10, &name)?;
            values.insert(
                "ROCR100".to_string(),
                indicator_values(finkit::indicators::momentum::rocr100(input, period), &name)?,
            );
            "ROCR100"
        }
        "OBV" => {
            let close = named_series(inputs, "CLOSE", &name)?;
            let volume = named_series(inputs, "VOLUME", &name)?;
            values.insert(
                "OBV".to_string(),
                indicator_values(finkit::indicators::volume::obv(close, volume), &name)?,
            );
            "OBV"
        }
        "MFI" => {
            let (high, low, close) = hlc(inputs, &name)?;
            let volume = named_series(inputs, "VOLUME", &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "MFI".to_string(),
                indicator_values(
                    finkit::indicators::momentum::mfi(high, low, close, volume, period),
                    &name,
                )?,
            );
            "MFI"
        }
        "AVGPRICE" => {
            let (open, high, low, close) = ohlc(inputs, &name)?;
            values.insert(
                "AVGPRICE".to_string(),
                indicator_values(
                    finkit::indicators::price_transform::avgprice(open, high, low, close),
                    &name,
                )?,
            );
            "AVGPRICE"
        }
        "MEDPRICE" => {
            let high = named_series(inputs, "HIGH", &name)?;
            let low = named_series(inputs, "LOW", &name)?;
            values.insert(
                "MEDPRICE".to_string(),
                indicator_values(
                    finkit::indicators::price_transform::medprice(high, low),
                    &name,
                )?,
            );
            "MEDPRICE"
        }
        "TYPPRICE" => {
            let (high, low, close) = hlc(inputs, &name)?;
            values.insert(
                "TYPPRICE".to_string(),
                indicator_values(
                    finkit::indicators::price_transform::typprice(high, low, close),
                    &name,
                )?,
            );
            "TYPPRICE"
        }
        "WCLPRICE" => {
            let (high, low, close) = hlc(inputs, &name)?;
            values.insert(
                "WCLPRICE".to_string(),
                indicator_values(
                    finkit::indicators::price_transform::wclprice(high, low, close),
                    &name,
                )?,
            );
            "WCLPRICE"
        }
        "MIDPOINT" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "MIDPOINT".to_string(),
                indicator_values(finkit::indicators::overlap::midpoint(input, period), &name)?,
            );
            "MIDPOINT"
        }
        "MIDPRICE" => {
            let high = named_series(inputs, "HIGH", &name)?;
            let low = named_series(inputs, "LOW", &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "MIDPRICE".to_string(),
                indicator_values(
                    finkit::indicators::overlap::midprice(high, low, period),
                    &name,
                )?,
            );
            "MIDPRICE"
        }
        "SAR" => {
            let high = named_series(inputs, "HIGH", &name)?;
            let low = named_series(inputs, "LOW", &name)?;
            let acceleration = parameter_f64(params, 0, 0.02, &name)?;
            let maximum = parameter_f64(params, 1, 0.2, &name)?;
            let output = finkit::indicators::overlap::sar(high, low, acceleration, maximum)
                .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("SAR".to_string(), output.sar.to_vec());
            "SAR"
        }
        "AD" => {
            let (high, low, close, volume) = ohlcv(inputs, &name)?;
            values.insert(
                "AD".to_string(),
                indicator_values(
                    finkit::indicators::volume::ad(high, low, close, volume),
                    &name,
                )?,
            );
            "AD"
        }
        "ADOSC" => {
            let (high, low, close, volume) = ohlcv(inputs, &name)?;
            let fast = parameter_usize(params, 0, 3, &name)?;
            let slow = parameter_usize(params, 1, 10, &name)?;
            let mut output = indicator_values(
                finkit::indicators::volume::adosc(high, low, close, volume, fast, slow),
                &name,
            )?;
            // TA-Lib ADOSC has no value until the slow EMA window closes;
            // the Core helper intentionally returns a neutral zero during
            // that phase for general-purpose composition.
            for value in output.iter_mut().take(slow.saturating_sub(1)) {
                *value = f64::NAN;
            }
            values.insert("ADOSC".to_string(), output);
            "ADOSC"
        }
        "PLUS_DM" => {
            let high = named_series(inputs, "HIGH", &name)?;
            let low = named_series(inputs, "LOW", &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "PLUS_DM".to_string(),
                indicator_values(
                    finkit::indicators::momentum::plus_dm_with_period(high, low, period),
                    &name,
                )?,
            );
            "PLUS_DM"
        }
        "MINUS_DM" => {
            let high = named_series(inputs, "HIGH", &name)?;
            let low = named_series(inputs, "LOW", &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            values.insert(
                "MINUS_DM".to_string(),
                indicator_values(
                    finkit::indicators::momentum::minus_dm_with_period(high, low, period),
                    &name,
                )?,
            );
            "MINUS_DM"
        }
        "PPO" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let fast = parameter_usize(params, 0, 12, &name)?;
            let slow = parameter_usize(params, 1, 26, &name)?;
            let ma_type = talib_ma_type(params, 2, 0, &name)?;
            values.insert(
                "PPO".to_string(),
                indicator_values(
                    finkit::indicators::momentum::ppo_with_ma_type(input, fast, slow, ma_type),
                    &name,
                )?,
            );
            "PPO"
        }
        "ULTOSC" => {
            let (high, low, close) = hlc(inputs, &name)?;
            let first = parameter_usize(params, 0, 7, &name)?;
            let second = parameter_usize(params, 1, 14, &name)?;
            let third = parameter_usize(params, 2, 28, &name)?;
            values.insert(
                "ULTOSC".to_string(),
                indicator_values(
                    finkit::indicators::momentum::ultosc(high, low, close, first, second, third),
                    &name,
                )?,
            );
            "ULTOSC"
        }
        "BETA" => {
            let asset = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let benchmark = ordered_series(inputs, input_order, 1, "BENCHMARK", &name)?;
            let period = parameter_usize(params, 0, 5, &name)?;
            values.insert(
                "BETA".to_string(),
                indicator_values(
                    finkit::indicators::statistics::beta(asset, benchmark, period),
                    &name,
                )?,
            );
            "BETA"
        }
        "CORREL" => {
            let first = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let second = ordered_series(inputs, input_order, 1, "BENCHMARK", &name)?;
            let period = parameter_usize(params, 0, 30, &name)?;
            values.insert(
                "CORREL".to_string(),
                indicator_values(
                    finkit::indicators::statistics::correlation(first, second, period),
                    &name,
                )?,
            );
            "CORREL"
        }
        "LINEARREG"
        | "LINEARREG_ANGLE"
        | "LINEARREG_INTERCEPT"
        | "LINEARREG_SLOPE"
        | "TSF"
        | "STDDEV"
        | "VAR" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 14, &name)?;
            let series = match name.as_str() {
                "LINEARREG" => finkit::indicators::statistics::linearreg(input, period),
                "LINEARREG_ANGLE" => finkit::indicators::statistics::linearreg_angle(input, period),
                "LINEARREG_INTERCEPT" => {
                    finkit::indicators::statistics::linearreg_intercept(input, period)
                }
                "LINEARREG_SLOPE" => finkit::indicators::statistics::linearreg_slope(input, period),
                "TSF" => finkit::indicators::statistics::tsf(input, period),
                "STDDEV" => {
                    let nb_dev = parameter_f64(params, 1, 1.0, &name)?;
                    finkit::indicators::statistics::std_dev(input, period, nb_dev)
                }
                "VAR" => {
                    let nb_dev = parameter_f64(params, 1, 1.0, &name)?;
                    finkit::indicators::statistics::var(input, period, nb_dev)
                }
                _ => unreachable!(),
            };
            values.insert(name.clone(), indicator_values(series, &name)?);
            name.as_str()
        }
        "ADD" | "SUB" | "MULT" | "DIV" => {
            let first = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let second = ordered_series(inputs, input_order, 1, "OPEN", &name)?;
            let series = match name.as_str() {
                "ADD" => finkit::indicators::math_operators::add(first, second),
                "SUB" => finkit::indicators::math_operators::sub(first, second),
                "MULT" => finkit::indicators::math_operators::mult(first, second),
                "DIV" => finkit::indicators::math_operators::div(first, second),
                _ => unreachable!(),
            };
            values.insert(name.clone(), indicator_values(series, &name)?);
            name.as_str()
        }
        "MAX" | "MIN" | "SUM" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 30, &name)?;
            let series = match name.as_str() {
                "MAX" => finkit::indicators::math_operators::max(input, period),
                "MIN" => finkit::indicators::math_operators::min(input, period),
                "SUM" => finkit::indicators::math_operators::sum(input, period),
                _ => unreachable!(),
            };
            values.insert(name.clone(), indicator_values(series, &name)?);
            name.as_str()
        }
        "MAXINDEX" | "MININDEX" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 30, &name)?;
            let result = if name == "MAXINDEX" {
                finkit::indicators::math_operators::maxindex(input, period)
            } else {
                finkit::indicators::math_operators::minindex(input, period)
            }
            .map(|series| talib_absolute_index_values(&series, period))
            .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert(name.clone(), result);
            name.as_str()
        }
        "MINMAX" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 30, &name)?;
            let (minimum, maximum) = finkit::indicators::math_operators::minmax(input, period)
                .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("MIN".to_string(), minimum.to_vec());
            values.insert("MAX".to_string(), maximum.to_vec());
            "MIN"
        }
        "MINMAXINDEX" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let period = parameter_usize(params, 0, 30, &name)?;
            let (minimum, maximum) = finkit::indicators::math_operators::minmaxindex(input, period)
                .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert(
                "MININDEX".to_string(),
                talib_absolute_index_values(&minimum, period),
            );
            values.insert(
                "MAXINDEX".to_string(),
                talib_absolute_index_values(&maximum, period),
            );
            "MININDEX"
        }
        "ACOS" | "ASIN" | "ATAN" | "CEIL" | "COS" | "COSH" | "EXP" | "FLOOR" | "LN" | "LOG10"
        | "SIN" | "SINH" | "SQRT" | "TAN" | "TANH" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let series = match name.as_str() {
                "ACOS" => finkit::indicators::math_transform::acos(input),
                "ASIN" => finkit::indicators::math_transform::asin(input),
                "ATAN" => finkit::indicators::math_transform::atan(input),
                "CEIL" => finkit::indicators::math_transform::ceil(input),
                "COS" => finkit::indicators::math_transform::cos(input),
                "COSH" => finkit::indicators::math_transform::cosh(input),
                "EXP" => finkit::indicators::math_transform::exp(input),
                "FLOOR" => finkit::indicators::math_transform::floor(input),
                "LN" => finkit::indicators::math_transform::ln(input),
                "LOG10" => finkit::indicators::math_transform::log10(input),
                "SIN" => finkit::indicators::math_transform::sin(input),
                "SINH" => finkit::indicators::math_transform::sinh(input),
                "SQRT" => finkit::indicators::math_transform::sqrt(input),
                "TAN" => finkit::indicators::math_transform::tan(input),
                "TANH" => finkit::indicators::math_transform::tanh(input),
                _ => unreachable!(),
            };
            values.insert(name.clone(), indicator_values(series, &name)?);
            name.as_str()
        }
        _ => {
            return Err((
                "unsupported_operation",
                format!("TA-Lib profile does not yet dispatch {name}"),
            ))
        }
    };

    let expected_names = contract
        .output_names
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let actual_names = values.keys().cloned().collect::<BTreeSet<_>>();
    if actual_names != expected_names || values.len() != contract.outputs {
        return Err((
            "internal_contract_error",
            format!(
                "{name} returned output fields {:?}, expected {:?}",
                actual_names, expected_names
            ),
        ));
    }
    let expected_length = inputs.values().next().map(Vec::len).unwrap_or(0);
    if values
        .values()
        .any(|series| series.len() != expected_length)
    {
        return Err((
            "internal_contract_error",
            format!("{name} returned an unaligned output series"),
        ));
    }
    let value_shape = if values.len() > 1 {
        "multi_series"
    } else {
        "series"
    };
    if value_shape != contract.value_shape {
        return Err((
            "internal_contract_error",
            format!(
                "{name} returned shape {value_shape}, expected {}",
                contract.value_shape
            ),
        ));
    }
    let serialized_values = values
        .iter()
        .map(|(name, series)| (name.clone(), nullable_series(series)))
        .collect::<BTreeMap<_, _>>();
    Ok(json!({
        "schema_version": OPERATION_RESULT_SCHEMA_VERSION,
        "semantic_profile": TALIB_SEMANTIC_PROFILE,
        "operation": name.clone(),
        "operation_id": finkit::operation::OperationId::from_name(&name).0,
        "primary": primary,
        "shape": value_shape,
        "values": serialized_values,
    }))
}

fn ordered_series<'a>(
    inputs: &'a BTreeMap<String, Vec<f64>>,
    order: &[String],
    index: usize,
    fallback: &str,
    operation: &str,
) -> Result<&'a [f64], (&'static str, String)> {
    let name = order.get(index).map(String::as_str).unwrap_or(fallback);
    named_series(inputs, name, operation)
}

fn named_series<'a>(
    inputs: &'a BTreeMap<String, Vec<f64>>,
    name: &str,
    operation: &str,
) -> Result<&'a [f64], (&'static str, String)> {
    inputs.get(name).map(Vec::as_slice).ok_or((
        "invalid_request",
        format!("{operation} requires input series {name}"),
    ))
}

fn hlc<'a>(
    inputs: &'a BTreeMap<String, Vec<f64>>,
    operation: &str,
) -> Result<(&'a [f64], &'a [f64], &'a [f64]), (&'static str, String)> {
    Ok((
        named_series(inputs, "HIGH", operation)?,
        named_series(inputs, "LOW", operation)?,
        named_series(inputs, "CLOSE", operation)?,
    ))
}

fn ohlcv<'a>(
    inputs: &'a BTreeMap<String, Vec<f64>>,
    operation: &str,
) -> Result<(&'a [f64], &'a [f64], &'a [f64], &'a [f64]), (&'static str, String)> {
    Ok((
        named_series(inputs, "HIGH", operation)?,
        named_series(inputs, "LOW", operation)?,
        named_series(inputs, "CLOSE", operation)?,
        named_series(inputs, "VOLUME", operation)?,
    ))
}

fn ohlc<'a>(
    inputs: &'a BTreeMap<String, Vec<f64>>,
    operation: &str,
) -> Result<(&'a [f64], &'a [f64], &'a [f64], &'a [f64]), (&'static str, String)> {
    Ok((
        named_series(inputs, "OPEN", operation)?,
        named_series(inputs, "HIGH", operation)?,
        named_series(inputs, "LOW", operation)?,
        named_series(inputs, "CLOSE", operation)?,
    ))
}

fn parameter_usize(
    params: &[f64],
    index: usize,
    default: usize,
    operation: &str,
) -> Result<usize, (&'static str, String)> {
    let value = params.get(index).copied().unwrap_or(default as f64);
    if !value.is_finite() || value < 1.0 || value.fract() != 0.0 {
        return Err((
            "invalid_request",
            format!("{operation} parameter {index} must be a positive integer"),
        ));
    }
    Ok(value as usize)
}

fn parameter_f64(
    params: &[f64],
    index: usize,
    default: f64,
    operation: &str,
) -> Result<f64, (&'static str, String)> {
    let value = params.get(index).copied().unwrap_or(default);
    if !value.is_finite() {
        return Err((
            "invalid_request",
            format!("{operation} parameter {index} must be finite"),
        ));
    }
    Ok(value)
}

fn talib_ma_type(
    params: &[f64],
    index: usize,
    default: usize,
    operation: &str,
) -> Result<finkit::indicators::overlap::MaType, (&'static str, String)> {
    let value = params.get(index).copied().unwrap_or(default as f64);
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 {
        return Err((
            "invalid_request",
            format!("{operation} MA type parameter {index} must be a non-negative integer"),
        ));
    }
    match value as usize {
        0 => Ok(finkit::indicators::overlap::MaType::Sma),
        1 => Ok(finkit::indicators::overlap::MaType::Ema),
        2 => Ok(finkit::indicators::overlap::MaType::Wma),
        3 => Ok(finkit::indicators::overlap::MaType::Dema),
        4 => Ok(finkit::indicators::overlap::MaType::Tema),
        5 => Ok(finkit::indicators::overlap::MaType::Trima),
        6 => Ok(finkit::indicators::overlap::MaType::Kama),
        7 => Ok(finkit::indicators::overlap::MaType::Mama),
        8 => Ok(finkit::indicators::overlap::MaType::T3),
        _ => Err((
            "invalid_request",
            format!("{operation} MA type parameter {index} is outside TA-Lib range 0..8"),
        )),
    }
}

fn indicator_values<T: std::fmt::Display>(
    result: Result<Array1<f64>, T>,
    operation: &str,
) -> Result<Vec<f64>, (&'static str, String)> {
    result
        .map(|values| values.to_vec())
        .map_err(|error| ("execution_error", format!("{operation}: {error}")))
}

/// TA-Lib's MAXINDEX/MININDEX family returns absolute input indices and uses
/// zero during the initial lookback. Core math helpers intentionally expose
/// relative window offsets with `-1` warm-up markers, so the profile boundary
/// converts the native representation here without changing the core API.
fn talib_absolute_index_values(values: &Array1<i64>, period: usize) -> Vec<f64> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            if *value < 0 {
                0.0
            } else {
                let window_start = index.saturating_add(1).saturating_sub(period);
                (*value + window_start as i64) as f64
            }
        })
        .collect()
}

fn pattern_values<T: std::fmt::Display>(
    result: std::result::Result<Array1<i32>, T>,
    operation: &str,
) -> Result<Vec<f64>, (&'static str, String)> {
    result
        .map(|values| values.iter().map(|value| *value as f64).collect())
        .map_err(|error| ("execution_error", format!("{operation}: {error}")))
}

fn value_shape_name(shape: finkit::operation::ValueShape) -> &'static str {
    match shape {
        finkit::operation::ValueShape::Series => "series",
        finkit::operation::ValueShape::MultiSeries => "multi_series",
        finkit::operation::ValueShape::CrossSection => "cross_section",
        finkit::operation::ValueShape::Event => "event",
        finkit::operation::ValueShape::Report => "report",
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
    use crate::talib_catalog::TALIB_PROFILE_CATALOG_NAMES;

    #[test]
    fn talib_support_is_derived_from_canonical_catalogs() {
        for contract in finkit::formula::ta_lib_function_contracts() {
            assert!(
                talib_profile_supported(&contract.name),
                "TA-Lib contract is not executable: {}",
                contract.name
            );
        }
        for name in TALIB_PROFILE_CATALOG_NAMES {
            assert!(
                talib_profile_supported(name),
                "profile catalog name is not executable: {name}"
            );
        }
    }

    #[test]
    fn executes_single_output_with_canonical_name_and_core_warmup() {
        let request = r#"{
            "operation":"SMA",
            "input_order":["CLOSE"],
            "inputs":{"CLOSE":[1.0,2.0,3.0]},
            "params":[2]
        }"#;
        let payload: Value = serde_json::from_str(&execute_operation_json(request)).unwrap();
        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["semantic_profile"], "core_registry");
        assert_eq!(payload["operation"], "SMA");
        assert_eq!(payload["primary"], "SMA");
        assert_eq!(payload["values"]["SMA"][0], 1.0);
        assert_eq!(payload["values"]["SMA"][2], 2.25);
    }

    #[test]
    fn preserves_named_multi_output_contract() {
        let request = r#"{
            "operation":"MACD",
            "input_order":["CLOSE"],
            "inputs":{"CLOSE":[1.0,2.0,3.0,4.0,5.0]},
            "params":[2,3,2]
        }"#;
        let payload: Value = serde_json::from_str(&execute_operation_json(request)).unwrap();
        assert_eq!(payload["shape"], "multi_series");
        assert!(payload["values"].get("MACD").is_some());
        assert!(payload["values"].get("MACD_SIGNAL").is_some());
        assert!(payload["values"].get("MACD_HIST").is_some());
    }

    #[test]
    fn talib_profile_is_explicit_and_uses_talib_sma_warmup() {
        let request = r#"{
            "operation":"SMA",
            "semantic_profile":"talib_0_8_0",
            "input_order":["CLOSE"],
            "inputs":{"CLOSE":[1.0,2.0,3.0]},
            "params":[2]
        }"#;
        let payload: Value = serde_json::from_str(&execute_operation_json(request)).unwrap();
        assert_eq!(payload["semantic_profile"], "talib_0_8_0");
        assert_eq!(payload["values"]["SMA"][0], Value::Null);
        assert_eq!(payload["values"]["SMA"][2], 2.5);
    }

    #[test]
    fn talib_profile_rejects_parameters_outside_catalog_contract() {
        let request = r#"{
            "operation":"SMA",
            "semantic_profile":"talib_0_8_0",
            "input_order":["CLOSE"],
            "inputs":{"CLOSE":[1.0,2.0,3.0]},
            "params":[2,99]
        }"#;
        let payload: Value = serde_json::from_str(&execute_operation_json(request)).unwrap();
        assert_eq!(payload["error"]["code"], "invalid_request");
        assert!(payload["error"]["message"]
            .as_str()
            .unwrap()
            .contains("at most 1 parameters"));
    }

    #[test]
    fn unversioned_talib_profile_is_rejected() {
        let request = r#"{
            "operation":"SMA",
            "semantic_profile":"talib",
            "input_order":["CLOSE"],
            "inputs":{"CLOSE":[1.0,2.0,3.0]},
            "params":[2]
        }"#;
        let payload: Value = serde_json::from_str(&execute_operation_json(request)).unwrap();
        assert_eq!(payload["error"]["code"], "unsupported_profile");
    }

    #[test]
    fn talib_profile_dispatches_extended_overlap_group() {
        let request = r#"{
            "operation":"T3",
            "semantic_profile":"talib_0_8_0",
            "input_order":["CLOSE"],
            "inputs":{"CLOSE":[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0,10.0,11.0,12.0,13.0,14.0]},
            "params":[2,0.7]
        }"#;
        let payload: Value = serde_json::from_str(&execute_operation_json(request)).unwrap();
        assert_eq!(payload["semantic_profile"], "talib_0_8_0");
        assert_eq!(payload["operation"], "T3");
        assert_eq!(payload["values"]["T3"].as_array().unwrap().len(), 14);
        assert!(payload["values"]["T3"][13].as_f64().unwrap().is_finite());
    }

    #[test]
    fn talib_profile_dispatches_mama_with_named_outputs() {
        let close = (0..40)
            .map(|index| 44.0 + (index as f64 * 0.2).sin())
            .collect::<Vec<_>>();
        let request = serde_json::json!({
            "operation": "MAMA",
            "semantic_profile": "talib_0_7_1",
            "input_order": ["CLOSE"],
            "inputs": {"CLOSE": close.clone()},
            "params": [0.5, 0.05]
        })
        .to_string();
        let payload: Value = serde_json::from_str(&execute_operation_json(&request)).unwrap();
        assert_eq!(payload["shape"], "multi_series");
        assert_eq!(payload["primary"], "MAMA");
        assert!(payload["values"].get("MAMA").is_some());
        assert!(payload["values"].get("FAMA").is_some());

        let core_request = serde_json::json!({
            "operation": "MAMA",
            "input_order": ["CLOSE"],
            "inputs": {"CLOSE": close},
            "params": [0.5, 0.05]
        })
        .to_string();
        let core_payload: Value =
            serde_json::from_str(&execute_operation_json(&core_request)).unwrap();
        assert_eq!(core_payload["semantic_profile"], "core_registry");
        assert_eq!(core_payload["primary"], "MAMA");
        assert_eq!(core_payload["values"].get("FAMA").is_some(), true);
    }

    #[test]
    fn talib_profile_dispatches_cycle_multi_output_contract() {
        let close = (0..64)
            .map(|index| 100.0 + (index as f64 / 5.0).sin())
            .collect::<Vec<_>>();
        let request = serde_json::json!({
            "operation": "HT_SINE",
            "semantic_profile": "talib_0_7_1",
            "input_order": ["CLOSE"],
            "inputs": {"CLOSE": close},
        })
        .to_string();
        let payload: Value = serde_json::from_str(&execute_operation_json(&request)).unwrap();
        assert_eq!(payload["shape"], "multi_series");
        assert_eq!(payload["primary"], "SINE");
        assert!(payload["values"].get("SINE").is_some());
        assert!(payload["values"].get("LEADSINE").is_some());
    }

    #[test]
    fn talib_profile_dispatches_candlestick_signal_series() {
        let request = serde_json::json!({
            "operation": "CDLDOJI",
            "semantic_profile": "talib_0_7_1",
            "input_order": ["OPEN", "HIGH", "LOW", "CLOSE"],
            "inputs": {
                "OPEN": [10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0],
                "HIGH": [10.5, 10.4, 10.6, 10.5, 10.4, 10.6, 10.5, 10.4, 10.6, 10.5],
                "LOW": [9.5, 9.6, 9.4, 9.5, 9.6, 9.4, 9.5, 9.6, 9.4, 9.5],
                "CLOSE": [10.0, 10.01, 9.99, 10.0, 10.01, 9.99, 10.0, 10.01, 9.99, 10.0]
            }
        })
        .to_string();
        let payload: Value = serde_json::from_str(&execute_operation_json(&request)).unwrap();
        assert_eq!(payload["shape"], "series", "payload: {payload}");
        assert_eq!(payload["primary"], "CDLDOJI");
        assert_eq!(payload["values"]["CDLDOJI"].as_array().unwrap().len(), 10);
    }

    #[test]
    fn talib_profile_dispatches_the_complete_candlestick_catalog() {
        let names = [
            "CDL2CROWS",
            "CDL3BLACKCROWS",
            "CDL3INSIDE",
            "CDL3LINESTRIKE",
            "CDL3OUTSIDE",
            "CDL3STARSINSOUTH",
            "CDL3WHITESOLDIERS",
            "CDLABANDONEDBABY",
            "CDLADVANCEBLOCK",
            "CDLBELTHOLD",
            "CDLBREAKAWAY",
            "CDLCLOSINGMARUBOZU",
            "CDLCONCEALBABYSWALL",
            "CDLCOUNTERATTACK",
            "CDLDARKCLOUDCOVER",
            "CDLDOJI",
            "CDLDOJISTAR",
            "CDLDRAGONFLYDOJI",
            "CDLENGULFING",
            "CDLEVENINGDOJISTAR",
            "CDLEVENINGSTAR",
            "CDLGAPSIDESIDEWHITE",
            "CDLGRAVESTONEDOJI",
            "CDLHAMMER",
            "CDLHANGINGMAN",
            "CDLHARAMI",
            "CDLHARAMICROSS",
            "CDLHIGHWAVE",
            "CDLHIKKAKE",
            "CDLHIKKAKEMOD",
            "CDLHOMINGPIGEON",
            "CDLIDENTICAL3CROWS",
            "CDLINNECK",
            "CDLINVERTEDHAMMER",
            "CDLKICKING",
            "CDLKICKINGBYLENGTH",
            "CDLLADDERBOTTOM",
            "CDLLONGLEGGEDDOJI",
            "CDLLONGLINE",
            "CDLMARUBOZU",
            "CDLMATCHINGLOW",
            "CDLMATHOLD",
            "CDLMORNINGDOJISTAR",
            "CDLMORNINGSTAR",
            "CDLONNECK",
            "CDLPIERCING",
            "CDLRICKSHAWMAN",
            "CDLRISEFALL3METHODS",
            "CDLSEPARATINGLINES",
            "CDLSHOOTINGSTAR",
            "CDLSHORTLINE",
            "CDLSPINNINGTOP",
            "CDLSTALLEDPATTERN",
            "CDLSTICKSANDWICH",
            "CDLTAKURI",
            "CDLTASUKIGAP",
            "CDLTHRUSTING",
            "CDLTRISTAR",
            "CDLUNIQUE3RIVER",
            "CDLUPSIDEGAP2CROWS",
            "CDLXSIDEGAP3METHODS",
        ];
        let open = (0..80)
            .map(|index| 100.0 + index as f64 * 0.1)
            .collect::<Vec<_>>();
        let high = open.iter().map(|value| value + 1.0).collect::<Vec<_>>();
        let low = open.iter().map(|value| value - 1.0).collect::<Vec<_>>();
        let close = open
            .iter()
            .enumerate()
            .map(|(index, value)| value + if index % 2 == 0 { 0.4 } else { -0.4 })
            .collect::<Vec<_>>();

        for name in names {
            assert!(
                talib_profile_supported(name),
                "unsupported catalog name {name}"
            );
            let request = serde_json::json!({
                "operation": name,
                "semantic_profile": "talib_0_7_1",
                "input_order": ["OPEN", "HIGH", "LOW", "CLOSE"],
                "inputs": {
                    "OPEN": open,
                    "HIGH": high,
                    "LOW": low,
                    "CLOSE": close
                }
            })
            .to_string();
            let payload: Value = serde_json::from_str(&execute_operation_json(&request)).unwrap();
            assert!(payload.get("error").is_none(), "{name}: {payload}");
            assert_eq!(payload["primary"], name);
            assert_eq!(payload["values"][name].as_array().unwrap().len(), 80);
        }
    }

    #[test]
    fn talib_profile_executes_every_catalog_name_through_json_contract() {
        use std::collections::BTreeSet;

        let mut names = BTreeSet::new();
        names.extend(
            crate::talib_catalog::TALIB_PROFILE_CATALOG_NAMES
                .iter()
                .copied(),
        );
        let registry = finkit::operation::builtin_operation_registry();
        for spec in registry.iter() {
            if talib_profile_supported(&spec.name) {
                names.insert(spec.name.as_str());
            }
        }
        assert_eq!(names.len(), 201, "TA-Lib catalog must contain 201 names");
        let catalog = crate::operation::operation_catalog(&registry);

        let length = 256;
        let close = (0..length)
            .map(|index| 100.0 + (index as f64 * 0.17).sin() * 3.0 + index as f64 * 0.01)
            .collect::<Vec<_>>();
        let open = close
            .iter()
            .enumerate()
            .map(|(index, value)| value + if index % 2 == 0 { 0.2 } else { -0.15 })
            .collect::<Vec<_>>();
        let high = open
            .iter()
            .zip(close.iter())
            .map(|(open, close)| open.max(*close) + 0.8)
            .collect::<Vec<_>>();
        let low = open
            .iter()
            .zip(close.iter())
            .map(|(open, close)| open.min(*close) - 0.8)
            .collect::<Vec<_>>();
        let volume = (0..length)
            .map(|index| 1_000.0 + (index % 17) as f64 * 13.0)
            .collect::<Vec<_>>();
        let benchmark = close
            .iter()
            .enumerate()
            .map(|(index, value)| value * 0.97 + index as f64 * 0.02)
            .collect::<Vec<_>>();
        let math = (0..length)
            .map(|index| 0.25 + (index as f64 * 0.11).sin() * 0.2)
            .collect::<Vec<_>>();
        let periods = (0..length)
            .map(|index| 5.0 + (index % 10) as f64)
            .collect::<Vec<_>>();

        for name in names {
            let (input_order, params): (Vec<&str>, Vec<f64>) =
                if name.starts_with("CDL") || matches!(name, "AVGPRICE" | "BOP") {
                    (vec!["OPEN", "HIGH", "LOW", "CLOSE"], vec![])
                } else if matches!(name, "MEDPRICE" | "MIDPRICE" | "SAR" | "SAREXT" | "AROON") {
                    let params = match name {
                        "MEDPRICE" => vec![],
                        "MIDPRICE" => vec![14.0],
                        "SAREXT" => vec![0.0, 0.0, 0.02, 0.02, 0.2, 0.02, 0.02, 0.2],
                        "SAR" => vec![0.02, 0.2],
                        "AROON" => vec![14.0],
                        _ => unreachable!(),
                    };
                    (vec!["HIGH", "LOW"], params)
                } else if name == "MAVP" {
                    (vec!["CLOSE", "PERIODS"], vec![2.0, 30.0, 0.0])
                } else if matches!(name, "BETA" | "CORREL") {
                    (vec!["CLOSE", "BENCHMARK"], vec![30.0])
                } else if matches!(name, "ADD" | "DIV" | "MULT" | "SUB") {
                    (vec!["CLOSE", "OPEN"], vec![])
                } else if matches!(
                    name,
                    "AD" | "ADOSC"
                        | "ATR"
                        | "ADX"
                        | "ADXR"
                        | "CCI"
                        | "DX"
                        | "MFI"
                        | "MINUS_DI"
                        | "MINUS_DM"
                        | "NATR"
                        | "PLUS_DI"
                        | "PLUS_DM"
                        | "STOCH"
                        | "STOCHF"
                        | "STOCHRSI"
                        | "TRANGE"
                        | "ULTOSC"
                        | "WILLR"
                ) {
                    let params = match name {
                        "ADOSC" => vec![3.0, 10.0],
                        "STOCH" => vec![14.0, 3.0, 0.0, 3.0, 0.0],
                        "STOCHF" => vec![14.0, 3.0, 0.0],
                        "STOCHRSI" => vec![14.0, 5.0, 3.0, 0.0],
                        "ULTOSC" => vec![7.0, 14.0, 28.0],
                        "TRANGE" | "AD" | "MFI" => vec![],
                        _ => vec![14.0],
                    };
                    let order = if matches!(name, "AD" | "ADOSC" | "MFI") {
                        vec!["HIGH", "LOW", "CLOSE", "VOLUME"]
                    } else {
                        vec!["HIGH", "LOW", "CLOSE"]
                    };
                    (order, params)
                } else if matches!(name, "MACD" | "MACDEXT" | "MACDFIX") {
                    let params = match name {
                        "MACD" => vec![12.0, 26.0, 9.0],
                        "MACDEXT" => vec![12.0, 0.0, 26.0, 0.0, 9.0, 0.0],
                        _ => vec![9.0],
                    };
                    (vec!["CLOSE"], params)
                } else if name == "BBANDS" {
                    (vec!["CLOSE"], vec![20.0, 2.0, 2.0, 0.0])
                } else if name == "T3" {
                    (vec!["CLOSE"], vec![5.0, 0.7])
                } else if name == "MAMA" {
                    (vec!["CLOSE"], vec![0.5, 0.05])
                } else if name == "PPO" {
                    (vec!["CLOSE"], vec![12.0, 26.0])
                } else if matches!(
                    name,
                    "MAX" | "MIN" | "MAXINDEX" | "MININDEX" | "MINMAX" | "MINMAXINDEX" | "SUM"
                ) {
                    (vec!["CLOSE"], vec![30.0])
                } else if matches!(
                    name,
                    "LINEARREG"
                        | "LINEARREG_ANGLE"
                        | "LINEARREG_INTERCEPT"
                        | "LINEARREG_SLOPE"
                        | "STDDEV"
                        | "TSF"
                        | "VAR"
                ) {
                    (vec!["CLOSE"], vec![30.0])
                } else if name == "APO" {
                    (vec!["CLOSE"], vec![12.0, 26.0])
                } else if matches!(name, "ROCP" | "ROCR" | "ROCR100") {
                    (vec!["CLOSE"], vec![14.0])
                } else if matches!(name, "MOM" | "ROC" | "RSI" | "CMO" | "TRIX") {
                    (vec!["CLOSE"], vec![14.0])
                } else if matches!(
                    name,
                    "ACOS"
                        | "ASIN"
                        | "ATAN"
                        | "CEIL"
                        | "COS"
                        | "COSH"
                        | "EXP"
                        | "FLOOR"
                        | "LN"
                        | "LOG10"
                        | "SIN"
                        | "SINH"
                        | "SQRT"
                        | "TAN"
                        | "TANH"
                ) {
                    (vec!["MATH"], vec![])
                } else {
                    (vec!["CLOSE"], vec![])
                };

            let request = serde_json::json!({
                "operation": name,
                "semantic_profile": "talib_0_7_1",
                "input_order": input_order,
                "inputs": {
                    "OPEN": open,
                    "HIGH": high,
                    "LOW": low,
                    "CLOSE": close,
                    "VOLUME": volume,
                    "BENCHMARK": benchmark,
                    "PERIODS": periods,
                    "MATH": math
                },
                "params": params
            })
            .to_string();
            let payload: Value = serde_json::from_str(&execute_operation_json(&request)).unwrap();
            assert!(payload.get("error").is_none(), "{name}: {payload}");
            assert!(payload["values"].as_object().is_some(), "{name}: {payload}");
            let expected_outputs = catalog
                .operations
                .iter()
                .find(|operation| operation.name == name)
                .and_then(|operation| {
                    operation
                        .profile_output_contracts
                        .get(crate::talib_catalog::TALIB_SEMANTIC_PROFILE)
                })
                .unwrap_or_else(|| panic!("missing profile output contract for {name}"));
            let actual_names = payload["values"]
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
            let expected_names = expected_outputs
                .output_names
                .iter()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(
                actual_names, expected_names,
                "{name} output names must match the catalog profile contract"
            );
            for (output_name, values) in payload["values"].as_object().unwrap() {
                assert_eq!(
                    values.as_array().map(Vec::len),
                    Some(length),
                    "{name}/{output_name}: {payload}"
                );
            }
        }
    }

    #[test]
    fn talib_profile_dispatches_generic_ma_macd_variants_and_cmo() {
        let request = serde_json::json!({
            "operation": "MA",
            "semantic_profile": "talib_0_7_1",
            "input_order": ["CLOSE"],
            "inputs": {"CLOSE": [1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0,10.0]},
            "params": [3, 1]
        })
        .to_string();
        let payload: Value = serde_json::from_str(&execute_operation_json(&request)).unwrap();
        assert_eq!(payload["values"]["MA"][9], 9.0);

        let macdext = serde_json::json!({
            "operation": "MACDEXT",
            "semantic_profile": "talib_0_7_1",
            "input_order": ["CLOSE"],
            "inputs": {"CLOSE": [1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0,10.0]},
            "params": [2, 0, 3, 0, 2, 0]
        })
        .to_string();
        let macd_payload: Value = serde_json::from_str(&execute_operation_json(&macdext)).unwrap();
        assert_eq!(macd_payload["shape"], "multi_series");
        assert!(macd_payload["values"].get("MACD_HIST").is_some());

        let cmo = serde_json::json!({
            "operation": "CMO",
            "semantic_profile": "talib_0_7_1",
            "input_order": ["CLOSE"],
            "inputs": {"CLOSE": [1.0,2.0,3.0,4.0,5.0]},
            "params": [3]
        })
        .to_string();
        let cmo_payload: Value = serde_json::from_str(&execute_operation_json(&cmo)).unwrap();
        assert_eq!(cmo_payload["values"]["CMO"][4], 100.0);
    }

    #[test]
    fn talib_profile_honors_stochastic_and_apo_ma_types() {
        let length = 256usize;
        let open: Vec<f64> = (0..length)
            .map(|i| 100.0 + 0.17 * i as f64 + (i as f64 * 0.17).sin())
            .collect();
        let high: Vec<f64> = open
            .iter()
            .enumerate()
            .map(|(i, value)| value + 1.0 + (i as f64 * 0.13).cos() * 0.1)
            .collect();
        let low: Vec<f64> = open
            .iter()
            .enumerate()
            .map(|(i, value)| value - 1.0 - (i as f64 * 0.13).cos() * 0.1)
            .collect();
        let close: Vec<f64> = open
            .iter()
            .enumerate()
            .map(|(i, value)| value + (i as f64 * 0.07).sin() * 0.25)
            .collect();
        let periods: Vec<f64> = (0..length).map(|i| 2.0 + (i % 29) as f64).collect();

        let execute = |operation: &str, params: &[f64]| -> Value {
            let input_order = if matches!(operation, "APO" | "BBANDS") {
                vec!["CLOSE"]
            } else if operation == "MAVP" {
                vec!["CLOSE", "PERIODS"]
            } else {
                vec!["HIGH", "LOW", "CLOSE"]
            };
            let request = serde_json::json!({
                "operation": operation,
                "semantic_profile": "talib_0_7_1",
                "input_order": input_order,
                "inputs": {
                    "HIGH": high,
                    "LOW": low,
                    "CLOSE": close,
                    "PERIODS": periods
                },
                "params": params,
            })
            .to_string();
            serde_json::from_str(&execute_operation_json(&request)).unwrap()
        };
        let last = |payload: &Value, output: &str| {
            payload["values"][output]
                .as_array()
                .unwrap()
                .last()
                .and_then(Value::as_f64)
                .unwrap()
        };
        let first_finite = |payload: &Value, output: &str| {
            payload["values"][output]
                .as_array()
                .unwrap()
                .iter()
                .position(Value::is_number)
                .unwrap()
        };
        let fixture: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tests/golden/talib/profile_matype_variants.json"
        )))
        .unwrap();
        let expected = |operation: &str, output: &str, field: &str| -> f64 {
            fixture["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|case| case["operation"] == operation)
                .unwrap()["expected"][output][field]
                .as_f64()
                .unwrap()
        };
        let expected_first = |operation: &str, output: &str| -> usize {
            expected(operation, output, "first_finite") as usize
        };

        let apo = execute("APO", &[12.0, 26.0, 1.0]);
        assert_eq!(first_finite(&apo, "APO"), expected_first("APO", "APO"));
        assert!(
            (last(&apo, "APO") - expected("APO", "APO", "last")).abs() < 1e-10,
            "APO actual {}",
            last(&apo, "APO")
        );

        let bbands = execute("BBANDS", &[20.0, 2.0, 2.0, 1.0]);
        for output in ["UPPERBAND", "MIDDLEBAND", "LOWERBAND"] {
            assert_eq!(
                first_finite(&bbands, output),
                expected_first("BBANDS", output)
            );
            assert!(
                (last(&bbands, output) - expected("BBANDS", output, "last")).abs() < 1e-10,
                "BBANDS/{output} actual {}",
                last(&bbands, output)
            );
        }

        let mavp = execute("MAVP", &[2.0, 30.0, 1.0]);
        assert_eq!(first_finite(&mavp, "MAVP"), expected_first("MAVP", "MAVP"));
        assert!(
            (last(&mavp, "MAVP") - expected("MAVP", "MAVP", "last")).abs() < 1e-8,
            "MAVP actual {}",
            last(&mavp, "MAVP")
        );

        let stoch_sma = execute("STOCH", &[14.0, 3.0, 0.0, 3.0, 0.0]);
        assert!((last(&stoch_sma, "SLOWK") - 67.19470511372855).abs() < 1e-10);
        assert!((last(&stoch_sma, "SLOWD") - 64.049258295922).abs() < 1e-10);

        let stochf_sma = execute("STOCHF", &[14.0, 3.0, 0.0]);
        assert!((last(&stochf_sma, "FASTK") - 70.12998468945).abs() < 1e-10);
        assert!((last(&stochf_sma, "FASTD") - 67.19470511372855).abs() < 1e-10);

        let stoch = execute("STOCH", &[14.0, 3.0, 1.0, 3.0, 1.0]);
        for output in ["SLOWK", "SLOWD"] {
            assert_eq!(
                first_finite(&stoch, output),
                expected_first("STOCH", output)
            );
            assert!((last(&stoch, output) - expected("STOCH", output, "last")).abs() < 1e-10);
        }

        let stochf = execute("STOCHF", &[14.0, 3.0, 1.0]);
        for output in ["FASTK", "FASTD"] {
            assert_eq!(
                first_finite(&stochf, output),
                expected_first("STOCHF", output)
            );
            assert!((last(&stochf, output) - expected("STOCHF", output, "last")).abs() < 1e-10);
        }

        let stochrsi = {
            let request = serde_json::json!({
                "operation": "STOCHRSI",
                "semantic_profile": "talib_0_7_1",
                "input_order": ["CLOSE"],
                "inputs": {"CLOSE": close},
                "params": [14.0, 5.0, 3.0, 1.0],
            })
            .to_string();
            serde_json::from_str(&execute_operation_json(&request)).unwrap()
        };
        for output in ["FASTK", "FASTD"] {
            assert_eq!(
                first_finite(&stochrsi, output),
                expected_first("STOCHRSI", output)
            );
            assert!((last(&stochrsi, output) - expected("STOCHRSI", output, "last")).abs() < 1e-10);
        }

        let stochrsi_sma = {
            let request = serde_json::json!({
                "operation": "STOCHRSI",
                "semantic_profile": "talib_0_7_1",
                "input_order": ["CLOSE"],
                "inputs": {"CLOSE": close},
                "params": [14.0, 5.0, 3.0, 0.0],
            })
            .to_string();
            serde_json::from_str(&execute_operation_json(&request)).unwrap()
        };
        assert!((last(&stochrsi_sma, "FASTK") - 100.0).abs() < 1e-10);
        assert!((last(&stochrsi_sma, "FASTD") - 100.0).abs() < 1e-10);
    }

    #[test]
    fn talib_profile_dispatches_accbands_avgdev_and_imi() {
        let high = vec![10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let low = vec![8.0, 9.0, 10.0, 11.0, 12.0, 13.0];
        let close = vec![9.0, 10.0, 11.0, 12.0, 13.0, 14.0];
        let accbands = serde_json::json!({
            "operation": "ACCBANDS",
            "semantic_profile": "talib_0_7_1",
            "input_order": ["HIGH", "LOW", "CLOSE"],
            "inputs": {"HIGH": high, "LOW": low, "CLOSE": close},
            "params": [3]
        })
        .to_string();
        let acc_payload: Value = serde_json::from_str(&execute_operation_json(&accbands)).unwrap();
        assert_eq!(acc_payload["shape"], "multi_series");
        assert!(acc_payload["values"].get("UPPERBAND").is_some());

        let avgdev = serde_json::json!({
            "operation": "AVGDEV",
            "semantic_profile": "talib_0_7_1",
            "input_order": ["CLOSE"],
            "inputs": {"CLOSE": [1.0,2.0,3.0,4.0,5.0]},
            "params": [3]
        })
        .to_string();
        let avg_payload: Value = serde_json::from_str(&execute_operation_json(&avgdev)).unwrap();
        assert!(avg_payload["values"]["AVGDEV"][4].as_f64().unwrap() > 0.0);

        let imi = serde_json::json!({
            "operation": "IMI",
            "semantic_profile": "talib_0_7_1",
            "input_order": ["OPEN", "CLOSE"],
            "inputs": {"OPEN": [1.0,2.0,3.0,4.0], "CLOSE": [2.0,3.0,4.0,5.0]},
            "params": [2]
        })
        .to_string();
        let imi_payload: Value = serde_json::from_str(&execute_operation_json(&imi)).unwrap();
        assert_eq!(imi_payload["values"]["IMI"][3], 100.0);
    }

    #[test]
    fn talib_profile_dispatches_price_transform_with_named_output() {
        let request = r#"{
            "operation":"AVGPRICE",
            "semantic_profile":"talib_0_7_1",
            "input_order":["OPEN","HIGH","LOW","CLOSE"],
            "inputs":{
                "OPEN":[1.0,2.0],"HIGH":[3.0,4.0],
                "LOW":[0.0,1.0],"CLOSE":[2.0,3.0]
            }
        }"#;
        let payload: Value = serde_json::from_str(&execute_operation_json(request)).unwrap();
        assert_eq!(payload["values"]["AVGPRICE"][0], 1.5);
        assert_eq!(payload["values"]["AVGPRICE"][1], 2.5);
    }

    #[test]
    fn talib_profile_dispatches_math_operator_and_normalizes_nonfinite_values() {
        let request = r#"{
            "operation":"DIV",
            "semantic_profile":"talib_0_7_1",
            "input_order":["A","B"],
            "inputs":{"A":[2.0,4.0],"B":[1.0,0.0]}
        }"#;
        let payload: Value = serde_json::from_str(&execute_operation_json(request)).unwrap();
        assert_eq!(payload["values"]["DIV"][0], 2.0);
        assert_eq!(payload["values"]["DIV"][1], Value::Null);
    }

    #[test]
    fn talib_profile_dispatches_minmax_and_minmaxindex_multi_output() {
        let request = serde_json::json!({
            "operation": "MINMAX",
            "semantic_profile": "talib_0_7_1",
            "input_order": ["CLOSE"],
            "inputs": {"CLOSE": [3.0, 1.0, 4.0, 1.0, 5.0]},
            "params": [3]
        })
        .to_string();
        let payload: Value = serde_json::from_str(&execute_operation_json(&request)).unwrap();
        assert_eq!(payload["shape"], "multi_series");
        assert_eq!(payload["primary"], "MIN");
        assert_eq!(payload["values"]["MIN"][0], Value::Null);
        assert_eq!(payload["values"]["MIN"][2], 1.0);
        assert_eq!(payload["values"]["MAX"][2], 4.0);
        assert_eq!(payload["values"]["MAX"][4], 5.0);

        let request = serde_json::json!({
            "operation": "MINMAXINDEX",
            "semantic_profile": "talib_0_7_1",
            "input_order": ["CLOSE"],
            "inputs": {"CLOSE": [3.0, 1.0, 4.0, 1.0, 5.0]},
            "params": [3]
        })
        .to_string();
        let payload: Value = serde_json::from_str(&execute_operation_json(&request)).unwrap();
        assert_eq!(payload["shape"], "multi_series");
        assert_eq!(payload["primary"], "MININDEX");
        assert_eq!(payload["values"]["MININDEX"][2], 1.0);
        assert_eq!(payload["values"]["MAXINDEX"][2], 2.0);
        assert_eq!(payload["values"]["MININDEX"][4], 3.0);
        assert_eq!(payload["values"]["MAXINDEX"][4], 4.0);
    }

    #[test]
    fn talib_profile_normalizes_absolute_index_and_trendmode_warmup() {
        let close = (0..40).map(|index| index as f64).collect::<Vec<_>>();
        let request = serde_json::json!({
            "operation": "MAXINDEX",
            "semantic_profile": "talib_0_7_1",
            "input_order": ["CLOSE"],
            "inputs": {"CLOSE": close},
            "params": [5]
        })
        .to_string();
        let payload: Value = serde_json::from_str(&execute_operation_json(&request)).unwrap();
        assert_eq!(payload["values"]["MAXINDEX"][0], 0.0);
        assert_eq!(payload["values"]["MAXINDEX"][4], 4.0);
        assert_eq!(payload["values"]["MAXINDEX"][10], 10.0);

        let request = serde_json::json!({
            "operation": "HT_TRENDMODE",
            "semantic_profile": "talib_0_7_1",
            "input_order": ["CLOSE"],
            "inputs": {"CLOSE": (0..80).map(|index| index as f64).collect::<Vec<_>>()}
        })
        .to_string();
        let payload: Value = serde_json::from_str(&execute_operation_json(&request)).unwrap();
        let values = payload["values"]["HT_TRENDMODE"].as_array().unwrap();
        assert_eq!(values.len(), 80);
        assert!(values.iter().take(32).all(|value| value == &json!(0.0)));
    }

    #[test]
    fn core_registry_dispatches_newly_registered_formula_operation() {
        let request = r#"{
            "operation":"ADD",
            "input_order":["LEFT","RIGHT"],
            "inputs":{"LEFT":[1.0,2.0],"RIGHT":[10.0,20.0]}
        }"#;
        let payload: Value = serde_json::from_str(&execute_operation_json(request)).unwrap();
        assert_eq!(payload["semantic_profile"], "core_registry");
        assert_eq!(payload["values"]["ADD"][0], 11.0);
        assert_eq!(payload["values"]["ADD"][1], 22.0);
    }

    #[test]
    fn talib_profile_dispatches_statistics_and_multi_output_math() {
        let stats = r#"{
            "operation":"STDDEV",
            "semantic_profile":"talib_0_7_1",
            "input_order":["CLOSE"],
            "inputs":{"CLOSE":[1.0,2.0,3.0,4.0]},
            "params":[2]
        }"#;
        let stats_payload: Value = serde_json::from_str(&execute_operation_json(stats)).unwrap();
        assert_eq!(stats_payload["values"]["STDDEV"][0], Value::Null);
        assert!(stats_payload["values"]["STDDEV"][3].as_f64().unwrap() > 0.0);

        let macd = r#"{
            "operation":"MACD",
            "semantic_profile":"talib_0_7_1",
            "input_order":["CLOSE"],
            "inputs":{"CLOSE":[1.0,2.0,3.0,4.0,5.0]},
            "params":[2,3,2]
        }"#;
        let macd_payload: Value = serde_json::from_str(&execute_operation_json(macd)).unwrap();
        assert_eq!(macd_payload["shape"], "multi_series");
        assert!(macd_payload["values"].get("MACD").is_some());
        assert!(macd_payload["values"].get("MACD_SIGNAL").is_some());
    }

    #[test]
    fn rejects_mismatched_input_lengths_with_structured_error() {
        let request = r#"{
            "operation":"SMA",
            "inputs":{"CLOSE":[1.0,2.0],"OPEN":[1.0]},
            "params":[2]
        }"#;
        let payload: Value = serde_json::from_str(&execute_operation_json(request)).unwrap();
        assert_eq!(payload["error"]["code"], "invalid_request");
    }
}
