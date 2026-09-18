//! Language-neutral execution contract for direct registered operations.
//!
//! This is the control-plane entry point shared by the official bindings. It
//! deliberately accepts JSON so every language can submit the same operation
//! name, ordered inputs, and numeric parameters while high-throughput callers
//! keep using typed indicator APIs.

use finkit::factors::FactorRegistry;
use finkit::formula::FormulaContext;
use finkit::operation::{OperationRequest, UnifiedOperationEngine, PRIMARY_OUTPUT_NAME};
use ndarray::Array1;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

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
    if semantic_profile == "talib" || semantic_profile == "talib_0_7_1" {
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
    let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
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
    matches!(
        normalize_name(operation).as_str(),
        "MA" | "SMA"
            | "EMA"
            | "WMA"
            | "DEMA"
            | "TEMA"
            | "TRIMA"
            | "T3"
            | "KAMA"
            | "MAMA"
            | "MAVP"
            | "SAREXT"
            | "HT_DCPERIOD"
            | "HT_DCPHASE"
            | "HT_PHASOR"
            | "HT_SINE"
            | "HT_TRENDMODE"
            | "HT_TRENDLINE"
            | "CDLDOJI"
            | "CDLDRAGONFLYDOJI"
            | "CDLGRAVESTONEDOJI"
            | "CDLENGULFING"
            | "CDLHAMMER"
            | "CDLHANGINGMAN"
            | "CDLHARAMI"
            | "CDLMARUBOZU"
            | "CDLPIERCING"
            | "CDLSHOOTINGSTAR"
            | "CDLSPINNINGTOP"
            | "RSI"
            | "MACD"
            | "MACDEXT"
            | "MACDFIX"
            | "BBANDS"
            | "ATR"
            | "NATR"
            | "TRANGE"
            | "ADX"
            | "ADXR"
            | "DX"
            | "PLUS_DI"
            | "MINUS_DI"
            | "CCI"
            | "AROON"
            | "AROONOSC"
            | "APO"
            | "BOP"
            | "TRIX"
            | "STOCH"
            | "STOCHF"
            | "STOCHRSI"
            | "WILLR"
            | "MOM"
            | "ROC"
            | "ROCP"
            | "ROCR"
            | "ROCR100"
            | "OBV"
            | "MFI"
            | "AVGPRICE"
            | "MEDPRICE"
            | "TYPPRICE"
            | "WCLPRICE"
            | "MIDPOINT"
            | "MIDPRICE"
            | "SAR"
            | "AD"
            | "ADOSC"
            | "PLUS_DM"
            | "MINUS_DM"
            | "PPO"
            | "ULTOSC"
            | "BETA"
            | "CORREL"
            | "LINEARREG"
            | "LINEARREG_ANGLE"
            | "LINEARREG_INTERCEPT"
            | "LINEARREG_SLOPE"
            | "TSF"
            | "STDDEV"
            | "VAR"
            | "CMO"
            | "ADD"
            | "SUB"
            | "MULT"
            | "DIV"
            | "MAX"
            | "MIN"
            | "MAXINDEX"
            | "MININDEX"
            | "SUM"
            | "ACOS"
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
    )
}

fn normalize_profile(value: &str) -> String {
    value.trim().to_ascii_lowercase()
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
    let mut values = BTreeMap::new();
    let primary = match name.as_str() {
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
            values.insert(
                "MAVP".to_string(),
                indicator_values(
                    finkit::math::moving_avg::mavp(input, periods, min_period, max_period),
                    &name,
                )?,
            );
            "MAVP"
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
            values.insert("SAREXT".to_string(), output.sar.to_vec());
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
            values.insert(name.clone(), indicator_values(series, &name)?);
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
            let period = parameter_usize(params, 0, 5, &name)?;
            let deviation = parameter_f64(params, 1, 2.0, &name)?;
            let output = finkit::indicators::overlap::bbands(input, period, deviation, deviation)
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
            values.insert(
                "APO".to_string(),
                indicator_values(finkit::indicators::momentum::apo(input, fast, slow), &name)?,
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
            let k_period = parameter_usize(params, 0, 5, &name)?;
            let k_slow = parameter_usize(params, 1, 3, &name)?;
            let d_period = parameter_usize(params, 2, 3, &name)?;
            let output =
                finkit::indicators::momentum::stoch(high, low, close, k_period, k_slow, d_period)
                    .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("SLOWK".to_string(), output.k.to_vec());
            values.insert("SLOWD".to_string(), output.d.to_vec());
            "SLOWK"
        }
        "STOCHF" => {
            let (high, low, close) = hlc(inputs, &name)?;
            let fast_k = parameter_usize(params, 0, 5, &name)?;
            let fast_d = parameter_usize(params, 1, 3, &name)?;
            let output = finkit::indicators::momentum::stochf(high, low, close, fast_k, fast_d)
                .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert("FASTK".to_string(), output.k.to_vec());
            values.insert("FASTD".to_string(), output.d.to_vec());
            "FASTK"
        }
        "STOCHRSI" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let rsi_period = parameter_usize(params, 0, 14, &name)?;
            let stoch_period = parameter_usize(params, 1, 14, &name)?;
            let fast_k = parameter_usize(params, 2, 3, &name)?;
            let fast_d = parameter_usize(params, 3, 3, &name)?;
            let output = finkit::indicators::momentum::stochrsi(
                input,
                rsi_period,
                stoch_period,
                fast_k,
                fast_d,
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
            values.insert(
                "ADOSC".to_string(),
                indicator_values(
                    finkit::indicators::volume::adosc(high, low, close, volume, fast, slow),
                    &name,
                )?,
            );
            "ADOSC"
        }
        "PLUS_DM" => {
            let high = named_series(inputs, "HIGH", &name)?;
            let low = named_series(inputs, "LOW", &name)?;
            values.insert(
                "PLUS_DM".to_string(),
                indicator_values(finkit::indicators::momentum::plus_dm(high, low), &name)?,
            );
            "PLUS_DM"
        }
        "MINUS_DM" => {
            let high = named_series(inputs, "HIGH", &name)?;
            let low = named_series(inputs, "LOW", &name)?;
            values.insert(
                "MINUS_DM".to_string(),
                indicator_values(finkit::indicators::momentum::minus_dm(high, low), &name)?,
            );
            "MINUS_DM"
        }
        "PPO" => {
            let input = ordered_series(inputs, input_order, 0, "CLOSE", &name)?;
            let fast = parameter_usize(params, 0, 12, &name)?;
            let slow = parameter_usize(params, 1, 26, &name)?;
            values.insert(
                "PPO".to_string(),
                indicator_values(finkit::indicators::momentum::ppo(input, fast, slow), &name)?,
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
            .map(|series| series.iter().map(|value| *value as f64).collect::<Vec<_>>())
            .map_err(|error| ("execution_error", format!("{name}: {error}")))?;
            values.insert(name.clone(), result);
            name.as_str()
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

    let serialized_values = values
        .iter()
        .map(|(name, series)| (name.clone(), nullable_series(series)))
        .collect::<BTreeMap<_, _>>();
    Ok(json!({
        "schema_version": OPERATION_RESULT_SCHEMA_VERSION,
        "semantic_profile": "talib_0_7_1",
        "operation": name.clone(),
        "operation_id": finkit::operation::OperationId::from_name(&name).0,
        "primary": primary,
        "shape": if values.len() > 1 { "multi_series" } else { "series" },
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
            "semantic_profile":"talib_0_7_1",
            "input_order":["CLOSE"],
            "inputs":{"CLOSE":[1.0,2.0,3.0]},
            "params":[2]
        }"#;
        let payload: Value = serde_json::from_str(&execute_operation_json(request)).unwrap();
        assert_eq!(payload["semantic_profile"], "talib_0_7_1");
        assert_eq!(payload["values"]["SMA"][0], Value::Null);
        assert_eq!(payload["values"]["SMA"][2], 2.5);
    }

    #[test]
    fn talib_profile_dispatches_extended_overlap_group() {
        let request = r#"{
            "operation":"T3",
            "semantic_profile":"talib_0_7_1",
            "input_order":["CLOSE"],
            "inputs":{"CLOSE":[1.0,2.0,3.0,4.0,5.0,6.0,7.0,8.0,9.0,10.0,11.0,12.0,13.0,14.0]},
            "params":[2,0.7]
        }"#;
        let payload: Value = serde_json::from_str(&execute_operation_json(request)).unwrap();
        assert_eq!(payload["semantic_profile"], "talib_0_7_1");
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
