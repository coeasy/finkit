//! Language-neutral canonical operation catalog.
//!
//! The catalog is generated from the Rust core registry at runtime and
//! serialized into a small stable JSON envelope. All bindings can expose this
//! exact metadata without maintaining language-specific copies of parameter,
//! shape, or capability declarations.

use crate::execute::talib_profile_supported;
use crate::talib_catalog::{TALIB_PROFILE_CATALOG_NAMES, TALIB_SEMANTIC_PROFILE};
use finkit::operation::{
    builtin_operation_registry, OperationCapabilities, OperationKind, OperationRegistry,
    OperationSpec,
};
use finkit::registry::{FunctionCategory, InputKind, LookbackSpec};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

static BUILTIN_OPERATION_CATALOG: OnceLock<OperationCatalogEnvelope> = OnceLock::new();

/// Versioned operation catalog envelope shared by all bindings.
#[derive(Debug, Clone, Serialize)]
pub struct OperationCatalogEnvelope {
    /// Catalog schema version.
    pub schema_version: u16,
    /// Core package version that produced this snapshot.
    pub engine_version: &'static str,
    /// Canonical operation entries in deterministic order.
    pub operations: Vec<OperationCatalogEntry>,
}

/// Serializable representation of one canonical operation.
#[derive(Debug, Clone, Serialize)]
pub struct OperationCatalogEntry {
    /// Stable numeric operation identity.
    pub operation_id: u32,
    /// Canonical uppercase operation name.
    pub name: String,
    /// Normalized aliases.
    pub aliases: Vec<String>,
    /// Top-level operation kind.
    pub kind: &'static str,
    /// Result shape.
    pub value_shape: &'static str,
    /// Legacy category when available.
    pub category: Option<&'static str>,
    /// Input kind when available.
    pub input: Option<&'static str>,
    /// Parameter declarations.
    pub params: Vec<OperationParameter>,
    /// Number of output series.
    pub outputs: usize,
    /// Stable names for output series in result order.
    pub output_names: Vec<String>,
    /// Profile-specific output schemas for operations whose compatibility
    /// profile intentionally has a different result shape than Core.
    pub profile_output_contracts: BTreeMap<String, OperationProfileOutputContract>,
    /// Lookback behavior.
    pub lookback: &'static str,
    /// Explicit execution and semantic capabilities.
    pub capabilities: OperationCapabilitiesJson,
    /// Operation metadata schema version.
    pub schema_version: u16,
    /// Explicit semantic profiles available to the dispatcher.
    pub semantic_profiles: Vec<String>,
}

/// Serializable parameter declaration.
#[derive(Debug, Clone, Serialize)]
pub struct OperationParameter {
    /// Stable parameter name.
    pub name: String,
    /// Human-readable parameter type.
    pub value_type: String,
    /// Optional textual default.
    pub default: Option<String>,
    /// Human-readable constraint.
    pub constraint: Option<String>,
}

/// Output schema selected by a semantic profile.
#[derive(Debug, Clone, Serialize)]
pub struct OperationProfileOutputContract {
    /// Result shape under the selected profile.
    pub value_shape: &'static str,
    /// Parameter declarations selected by the profile.
    pub params: Vec<OperationParameter>,
    /// Number of aligned output series.
    pub outputs: usize,
    /// Stable output names in result order.
    pub output_names: Vec<String>,
}

/// Serializable capability set.
#[derive(Debug, Clone, Serialize)]
pub struct OperationCapabilitiesJson {
    pub batch: bool,
    pub streaming: bool,
    pub cross_sectional: bool,
    pub multi_symbol: bool,
    pub multi_timeframe: bool,
    pub parallel: bool,
    pub deterministic: bool,
    pub causal: bool,
    pub lookahead: bool,
    pub repaint: bool,
    pub stateful: bool,
    pub drawable: bool,
}

impl From<OperationCapabilities> for OperationCapabilitiesJson {
    fn from(value: OperationCapabilities) -> Self {
        Self {
            batch: value.batch,
            streaming: value.streaming,
            cross_sectional: value.cross_sectional,
            multi_symbol: value.multi_symbol,
            multi_timeframe: value.multi_timeframe,
            parallel: value.parallel,
            deterministic: value.deterministic,
            causal: value.causal,
            lookahead: value.lookahead,
            repaint: value.repaint,
            stateful: value.stateful,
            drawable: value.drawable,
        }
    }
}

impl OperationCatalogEntry {
    fn from_spec(spec: &OperationSpec) -> Self {
        let profile_output_contracts = if talib_profile_supported(&spec.name) {
            let (value_shape, output_names) =
                talib_profile_output_shape(&spec.name, &spec.output_names);
            let outputs = output_names.len();
            BTreeMap::from([(
                TALIB_SEMANTIC_PROFILE.to_string(),
                OperationProfileOutputContract {
                    value_shape,
                    params: talib_profile_params_for(&spec.name, &spec.params),
                    outputs,
                    output_names,
                },
            )])
        } else {
            BTreeMap::new()
        };
        Self {
            operation_id: spec.id().0,
            name: spec.name.clone(),
            aliases: spec.aliases.clone(),
            kind: operation_kind_name(spec.kind),
            value_shape: value_shape_name(spec.value_shape),
            category: spec.category.map(function_category_name),
            input: spec.input.map(input_kind_name),
            params: spec
                .params
                .iter()
                .map(|param| OperationParameter {
                    name: param.name.to_string(),
                    value_type: param.value_type.to_string(),
                    default: param.default.map(str::to_string),
                    constraint: param.constraint.map(str::to_string),
                })
                .collect(),
            outputs: spec.outputs,
            output_names: spec.output_names.clone(),
            profile_output_contracts,
            lookback: lookback_name(spec.lookback),
            capabilities: spec.capabilities.into(),
            schema_version: spec.schema_version,
            semantic_profiles: {
                let mut profiles = vec!["core_registry".to_string()];
                if talib_profile_supported(&spec.name) {
                    profiles.push(TALIB_SEMANTIC_PROFILE.to_string());
                }
                profiles
            },
        }
    }
}

/// Build the shared operation catalog from a supplied registry.
pub fn operation_catalog(registry: &OperationRegistry) -> OperationCatalogEnvelope {
    let mut operations: Vec<OperationCatalogEntry> = registry
        .iter()
        .map(OperationCatalogEntry::from_spec)
        .collect();

    // The native Core registry intentionally contains only the canonical Core
    // surface.  TA-Lib has additional profile-only names (notably its full
    // candlestick directory and math transforms), but they are still public
    // executable operations through the versioned dispatcher.  Keep them in
    // the same catalog so language bindings do not have to maintain a second
    // hand-written TA-Lib name list.
    for name in TALIB_PROFILE_CATALOG_NAMES {
        if registry.get(name).is_none() {
            operations.push(profile_only_talib_entry(name));
        }
    }
    operations.sort_by(|left, right| left.name.cmp(&right.name));

    OperationCatalogEnvelope {
        schema_version: 1,
        engine_version: env!("CARGO_PKG_VERSION"),
        operations,
    }
}

/// Return the process-stable catalog for the built-in operation registry.
///
/// The catalog is both discovery metadata and the runtime schema used by the
/// compatibility dispatcher. Building it once avoids recreating the complete
/// profile projection for every JSON request while keeping all bindings on the
/// same immutable snapshot.
pub fn builtin_operation_catalog() -> &'static OperationCatalogEnvelope {
    BUILTIN_OPERATION_CATALOG.get_or_init(|| operation_catalog(&builtin_operation_registry()))
}

pub(crate) fn talib_profile_contract(
    name: &str,
) -> Option<&'static OperationProfileOutputContract> {
    builtin_operation_catalog()
        .operations
        .iter()
        .find(|operation| operation.name == name)
        .and_then(|operation| {
            operation
                .profile_output_contracts
                .get(TALIB_SEMANTIC_PROFILE)
        })
}

fn profile_only_talib_entry(name: &str) -> OperationCatalogEntry {
    let is_pattern = name.starts_with("CDL");
    let (value_shape, output_names) = match name {
        "HA" => (
            "multi_series",
            vec![
                "HAOPEN".to_string(),
                "HAHIGH".to_string(),
                "HALOW".to_string(),
                "HACLOSE".to_string(),
            ],
        ),
        "VORTEX" => (
            "multi_series",
            vec!["PLUSVI".to_string(), "MINUSVI".to_string()],
        ),
        "AROON" => (
            "multi_series",
            vec!["AROON_UP".to_string(), "AROON_DOWN".to_string()],
        ),
        "STOCH" => (
            "multi_series",
            vec!["SLOWK".to_string(), "SLOWD".to_string()],
        ),
        "STOCHF" => (
            "multi_series",
            vec!["FASTK".to_string(), "FASTD".to_string()],
        ),
        "STOCHRSI" => (
            "multi_series",
            vec!["FASTK".to_string(), "FASTD".to_string()],
        ),
        "ERI" => (
            "multi_series",
            vec!["BULLPOWER".to_string(), "BEARPOWER".to_string()],
        ),
        "FRACTAL" => (
            "multi_series",
            vec!["SWINGHIGH".to_string(), "SWINGLOW".to_string()],
        ),
        "KC" => (
            "multi_series",
            vec![
                "UPPERBAND".to_string(),
                "MIDDLEBAND".to_string(),
                "LOWERBAND".to_string(),
            ],
        ),
        "SMI" => (
            "multi_series",
            vec!["SMI".to_string(), "SMISIGNAL".to_string()],
        ),
        "AC" | "ADR" | "CMOU" | "CVI" | "EFI" | "FOSC" | "MARKETFI" | "MASSI" | "PERCENTILE"
        | "PVO" | "QSTICK" | "RMA" | "RVI" | "RVOL" | "VHF" | "WAD" => {
            ("series", vec!["REAL".to_string()])
        }
        _ => ("series", vec![name.to_string()]),
    };
    let outputs = output_names.len();
    let input = if is_pattern || matches!(name, "AVGPRICE" | "BOP" | "HA") {
        Some("ohlcv")
    } else if matches!(
        name,
        "AO" | "DONCHIAN" | "VORTEX" | "SUPERTREND" | "MEDPRICE" | "MIDPRICE" | "SAR"
    ) {
        Some("hlc")
    } else if matches!(name, "CMF") {
        Some("hlcv")
    } else {
        Some("series")
    };
    OperationCatalogEntry {
        operation_id: finkit::operation::OperationId::from_name(name).0,
        name: name.to_string(),
        aliases: Vec::new(),
        kind: "indicator",
        value_shape,
        category: Some("talib"),
        input,
        params: talib_profile_params(name),
        outputs,
        output_names: output_names.clone(),
        lookback: "dynamic",
        capabilities: OperationCapabilities::indicator(false, true).into(),
        schema_version: 1,
        semantic_profiles: vec![TALIB_SEMANTIC_PROFILE.to_string()],
        profile_output_contracts: BTreeMap::from([(
            TALIB_SEMANTIC_PROFILE.to_string(),
            OperationProfileOutputContract {
                value_shape,
                params: talib_profile_params(name),
                outputs,
                output_names: output_names.clone(),
            },
        )]),
    }
}

fn talib_profile_output_shape(name: &str, fallback: &[String]) -> (&'static str, Vec<String>) {
    match name {
        "HA" => (
            "multi_series",
            vec![
                "HAOPEN".to_string(),
                "HAHIGH".to_string(),
                "HALOW".to_string(),
                "HACLOSE".to_string(),
            ],
        ),
        "VORTEX" => (
            "multi_series",
            vec!["PLUSVI".to_string(), "MINUSVI".to_string()],
        ),
        "AROON" => (
            "multi_series",
            vec!["AROON_UP".to_string(), "AROON_DOWN".to_string()],
        ),
        "STOCH" => (
            "multi_series",
            vec!["SLOWK".to_string(), "SLOWD".to_string()],
        ),
        "STOCHF" | "STOCHRSI" => (
            "multi_series",
            vec!["FASTK".to_string(), "FASTD".to_string()],
        ),
        "ERI" => (
            "multi_series",
            vec!["BULLPOWER".to_string(), "BEARPOWER".to_string()],
        ),
        "FRACTAL" => (
            "multi_series",
            vec!["SWINGHIGH".to_string(), "SWINGLOW".to_string()],
        ),
        "KC" => (
            "multi_series",
            vec![
                "UPPERBAND".to_string(),
                "MIDDLEBAND".to_string(),
                "LOWERBAND".to_string(),
            ],
        ),
        "DONCHIAN" => (
            "multi_series",
            vec![
                "UPPERBAND".to_string(),
                "MIDDLEBAND".to_string(),
                "LOWERBAND".to_string(),
            ],
        ),
        "KDJ" => (
            "multi_series",
            vec!["K".to_string(), "D".to_string(), "J".to_string()],
        ),
        "SMI" => (
            "multi_series",
            vec!["SMI".to_string(), "SMISIGNAL".to_string()],
        ),
        "SUPERTREND" => (
            "multi_series",
            vec!["SUPERTREND".to_string(), "TREND".to_string()],
        ),
        "AC" | "ADR" | "CMOU" | "CVI" | "EFI" | "FOSC" | "MARKETFI" | "MASSI" | "PERCENTILE"
        | "PVO" | "QSTICK" | "RMA" | "RVI" | "RVOL" | "VHF" | "WAD" => {
            ("series", vec!["REAL".to_string()])
        }
        _ if fallback.len() > 1 => ("multi_series", fallback.to_vec()),
        _ if !fallback.is_empty() => ("series", fallback.to_vec()),
        _ => ("series", vec![name.to_string()]),
    }
}

fn talib_profile_params_for(
    name: &str,
    fallback: &[finkit::registry::ParamSpec],
) -> Vec<OperationParameter> {
    let explicit = talib_profile_params(name);
    if !explicit.is_empty() {
        return explicit;
    }
    fallback
        .iter()
        .map(|param| OperationParameter {
            name: param.name.to_string(),
            value_type: param.value_type.to_string(),
            default: param.default.map(str::to_string),
            constraint: param.constraint.map(str::to_string),
        })
        .collect()
}

fn talib_parameter(
    name: &str,
    value_type: &str,
    default: Option<&str>,
    constraint: Option<&str>,
) -> OperationParameter {
    OperationParameter {
        name: name.to_string(),
        value_type: value_type.to_string(),
        default: default.map(str::to_string),
        constraint: constraint.map(str::to_string),
    }
}

fn period_parameter(default: &'static str) -> OperationParameter {
    talib_parameter("timeperiod", "integer", Some(default), Some("integer >= 1"))
}

/// Parameters for names that are not projected from the Core registry.
///
/// These describe the currently executable shared-dispatcher contract. The
/// TA-Lib MA type range is intentionally exposed only where the underlying
/// Core kernel consumes it; a catalog entry must never advertise a parameter
/// that the executor silently ignores.
fn talib_profile_params(name: &str) -> Vec<OperationParameter> {
    let period = || period_parameter("14");
    match name {
        "MA" => vec![
            period_parameter("30"),
            talib_parameter("matype", "integer", Some("0"), Some("integer in 0..8")),
        ],
        "SMA" => vec![period_parameter("30")],
        "SAR" => vec![
            talib_parameter("acceleration", "number", Some("0.02"), Some("finite")),
            talib_parameter("maximum", "number", Some("0.2"), Some("finite")),
        ],
        "SAREXT" => vec![
            talib_parameter("startvalue", "number", Some("0.0"), Some("finite")),
            talib_parameter("offsetonreverse", "number", Some("0.0"), Some("finite")),
            talib_parameter("afinitlong", "number", Some("0.02"), Some("finite")),
            talib_parameter("aflong", "number", Some("0.02"), Some("finite")),
            talib_parameter("afmaxlong", "number", Some("0.2"), Some("finite")),
            talib_parameter("afinitshort", "number", Some("0.02"), Some("finite")),
            talib_parameter("afshort", "number", Some("0.02"), Some("finite")),
            talib_parameter("afmaxshort", "number", Some("0.2"), Some("finite")),
        ],
        "MIDPRICE" => vec![period_parameter("14")],
        "STDDEV" | "VAR" => vec![
            period_parameter("30"),
            talib_parameter("nbdev", "number", Some("1.0"), Some("finite")),
        ],
        "AC" => vec![
            talib_parameter("fastperiod", "integer", Some("5"), Some("integer >= 2")),
            talib_parameter("slowperiod", "integer", Some("34"), Some("integer >= 2")),
            talib_parameter("signalperiod", "integer", Some("5"), Some("integer >= 2")),
        ],
        "ADR" => vec![period_parameter("14")],
        "CMOU" => vec![period_parameter("14")],
        "CVI" => vec![
            period_parameter("10"),
            talib_parameter("rocperiod", "integer", Some("10"), Some("integer >= 1")),
        ],
        "EFI" => vec![period_parameter("13")],
        "ERI" => vec![period_parameter("13")],
        "FOSC" => vec![period_parameter("5")],
        "FRACTAL" => vec![
            talib_parameter("leftbars", "integer", Some("2"), Some("integer >= 1")),
            talib_parameter("rightbars", "integer", Some("2"), Some("integer >= 1")),
        ],
        "KC" => vec![
            talib_parameter("timeperiod", "integer", Some("20"), Some("integer >= 2")),
            talib_parameter("atrperiod", "integer", Some("10"), Some("integer >= 1")),
            talib_parameter("nbdev", "number", Some("2.0"), Some("finite number")),
        ],
        "KDJ" => vec![
            talib_parameter("fastk_period", "integer", Some("9"), Some("integer >= 1")),
            talib_parameter("slowk_period", "integer", Some("3"), Some("integer >= 1")),
            talib_parameter(
                "slowk_matype",
                "integer",
                Some("13"),
                Some("TA-Lib MA type"),
            ),
            talib_parameter("slowd_period", "integer", Some("3"), Some("integer >= 1")),
            talib_parameter(
                "slowd_matype",
                "integer",
                Some("13"),
                Some("TA-Lib MA type"),
            ),
        ],
        "MARKETFI" => vec![],
        "MASSI" => vec![
            talib_parameter("fastperiod", "integer", Some("9"), Some("integer >= 1")),
            talib_parameter("slowperiod", "integer", Some("25"), Some("integer >= 1")),
        ],
        "PERCENTILE" => vec![
            talib_parameter("timeperiod", "integer", Some("30"), Some("integer >= 1")),
            talib_parameter(
                "percentile",
                "number",
                Some("50.0"),
                Some("0 <= value <= 100"),
            ),
        ],
        "PVO" => vec![
            talib_parameter("fastperiod", "integer", Some("12"), Some("integer >= 1")),
            talib_parameter("slowperiod", "integer", Some("26"), Some("integer >= 1")),
            talib_parameter("matype", "integer", Some("1"), Some("TA-Lib MA type")),
        ],
        "QSTICK" => vec![period_parameter("10")],
        "RMA" => vec![period_parameter("30")],
        "RVI" => vec![
            period_parameter("14"),
            talib_parameter("stddevperiod", "integer", Some("10"), Some("integer >= 2")),
        ],
        "RVOL" => vec![period_parameter("20")],
        "SMI" => vec![
            talib_parameter("timeperiod", "integer", Some("13"), Some("integer >= 2")),
            talib_parameter("fastperiod", "integer", Some("2"), Some("integer >= 2")),
            talib_parameter("slowperiod", "integer", Some("25"), Some("integer >= 2")),
            talib_parameter("signalperiod", "integer", Some("9"), Some("integer >= 2")),
        ],
        "VHF" => vec![period_parameter("28")],
        "WAD" => vec![],
        "AO" => vec![
            talib_parameter("fastperiod", "integer", Some("5"), Some("integer >= 1")),
            talib_parameter("slowperiod", "integer", Some("34"), Some("integer >= 1")),
        ],
        "CMF" => vec![period_parameter("20")],
        "COPPOCK" => vec![
            talib_parameter("wmaperiod", "integer", Some("10"), Some("integer >= 1")),
            talib_parameter("roc1period", "integer", Some("11"), Some("integer >= 1")),
            talib_parameter("roc2period", "integer", Some("14"), Some("integer >= 1")),
        ],
        "DPO" => vec![period_parameter("20")],
        "ER" => vec![period_parameter("10")],
        "HMA" => vec![period_parameter("20")],
        "PERCENTRANK" => vec![period_parameter("100")],
        "TSI" => vec![
            talib_parameter("firstperiod", "integer", Some("25"), Some("integer >= 1")),
            talib_parameter("secondperiod", "integer", Some("13"), Some("integer >= 1")),
        ],
        "VORTEX" => vec![period_parameter("14")],
        "VWMA" | "ZLEMA" => vec![period_parameter("30")],
        "SUPERTREND" => vec![
            period_parameter("10"),
            talib_parameter("multiplier", "number", Some("3.0"), Some("value > 0")),
        ],
        "ADX" | "ADXR" | "AROON" | "AROONOSC" | "CCI" | "CMO" | "DX" | "MFI" | "MINUS_DI"
        | "MINUS_DM" | "PLUS_DI" | "PLUS_DM" | "RSI" | "WILLR" | "ROCP" | "ROCR" | "ROCR100" => {
            vec![period()]
        }
        "APO" | "PPO" => vec![
            talib_parameter("fastperiod", "integer", Some("12"), Some("integer >= 1")),
            talib_parameter("slowperiod", "integer", Some("26"), Some("integer >= 1")),
            talib_parameter("matype", "integer", Some("0"), Some("integer in 0..8")),
        ],
        "STOCH" => vec![
            talib_parameter("fastk_period", "integer", Some("5"), Some("integer >= 1")),
            talib_parameter("slowk_period", "integer", Some("3"), Some("integer >= 1")),
            talib_parameter(
                "slowk_matype",
                "integer",
                Some("0"),
                Some("integer in 0..8"),
            ),
            talib_parameter("slowd_period", "integer", Some("3"), Some("integer >= 1")),
            talib_parameter(
                "slowd_matype",
                "integer",
                Some("0"),
                Some("integer in 0..8"),
            ),
        ],
        "STOCHF" => vec![
            talib_parameter("fastk_period", "integer", Some("5"), Some("integer >= 1")),
            talib_parameter("fastd_period", "integer", Some("3"), Some("integer >= 1")),
            talib_parameter(
                "fastd_matype",
                "integer",
                Some("0"),
                Some("integer in 0..8"),
            ),
        ],
        "STOCHRSI" => vec![
            talib_parameter("timeperiod", "integer", Some("14"), Some("integer >= 1")),
            talib_parameter("fastk_period", "integer", Some("5"), Some("integer >= 1")),
            talib_parameter("fastd_period", "integer", Some("3"), Some("integer >= 1")),
            talib_parameter(
                "fastd_matype",
                "integer",
                Some("0"),
                Some("integer in 0..8"),
            ),
        ],
        "MAVP" => vec![
            talib_parameter("minperiod", "integer", Some("2"), Some("integer >= 2")),
            talib_parameter(
                "maxperiod",
                "integer",
                Some("30"),
                Some("integer >= minperiod"),
            ),
            talib_parameter("matype", "integer", Some("0"), Some("integer in 0..8")),
        ],
        "BETA" | "CORREL" => vec![period()],
        "MAX"
        | "MIN"
        | "MAXINDEX"
        | "MININDEX"
        | "MINMAX"
        | "MINMAXINDEX"
        | "SUM"
        | "LINEARREG"
        | "LINEARREG_ANGLE"
        | "LINEARREG_INTERCEPT"
        | "LINEARREG_SLOPE"
        | "TSF" => vec![period_parameter("30")],
        "T3" => vec![
            talib_parameter("timeperiod", "integer", Some("5"), Some("integer >= 1")),
            talib_parameter(
                "vfactor",
                "number",
                Some("0.7"),
                Some("0.0 <= value <= 1.0"),
            ),
        ],
        "TRIX" => vec![period_parameter("30")],
        _ => Vec::new(),
    }
}

/// Build the built-in operation catalog as a JSON string.
pub fn operation_catalog_json() -> Result<String, serde_json::Error> {
    serde_json::to_string(builtin_operation_catalog())
}

fn operation_kind_name(kind: OperationKind) -> &'static str {
    match kind {
        OperationKind::Indicator => "indicator",
        OperationKind::FormulaFunction => "formula_function",
        OperationKind::Factor => "factor",
        OperationKind::Composite => "composite",
        OperationKind::Draw => "draw",
    }
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

fn function_category_name(category: FunctionCategory) -> &'static str {
    match category {
        FunctionCategory::Overlap => "overlap",
        FunctionCategory::Momentum => "momentum",
        FunctionCategory::Volatility => "volatility",
        FunctionCategory::Volume => "volume",
        FunctionCategory::Statistics => "statistics",
        FunctionCategory::Formula => "formula",
        FunctionCategory::Factor => "factor",
    }
}

fn input_kind_name(input: InputKind) -> &'static str {
    match input {
        InputKind::Series => "series",
        InputKind::Hlc => "hlc",
        InputKind::Hlcv => "hlcv",
        InputKind::Ohlcv => "ohlcv",
        InputKind::Dynamic => "dynamic",
    }
}

fn lookback_name(lookback: LookbackSpec) -> &'static str {
    match lookback {
        LookbackSpec::None => "none",
        LookbackSpec::PeriodMinusOne => "period_minus_one",
        LookbackSpec::Period => "period",
        LookbackSpec::Dynamic => "dynamic",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_json_is_stable_and_contains_capabilities() {
        let json = operation_catalog_json().unwrap();
        assert!(json.contains("\"schema_version\":1"));
        assert!(json.contains("\"name\":\"EMA\""));
        assert!(json.contains("\"operation_id\":"));
        assert!(json.contains("\"multi_symbol\":false"));
        assert!(json.contains("\"value_shape\":\"series\""));
        assert!(json.contains("\"semantic_profiles\":[\"core_registry\",\"talib_0_8_0\"]"));
        assert!(json.contains("\"profile_output_contracts\""));
    }

    #[test]
    fn catalog_is_deterministic_and_includes_registry_entries() {
        let registry = builtin_operation_registry();
        let catalog = operation_catalog(&registry);
        assert_eq!(
            catalog.operations.len(),
            registry.len()
                + TALIB_PROFILE_CATALOG_NAMES
                    .iter()
                    .filter(|name| registry.get(name).is_none())
                    .count()
        );
        let names: Vec<_> = catalog
            .operations
            .iter()
            .map(|operation| operation.name.as_str())
            .collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
    }

    #[test]
    fn catalog_exposes_named_multi_output_contracts() {
        let catalog = operation_catalog(&builtin_operation_registry());
        let macd = catalog
            .operations
            .iter()
            .find(|operation| operation.name == "MACD")
            .unwrap();
        assert_eq!(macd.output_names, vec!["MACD", "MACD_SIGNAL", "MACD_HIST"]);
        let bbands = catalog
            .operations
            .iter()
            .find(|operation| operation.name == "BBANDS")
            .unwrap();
        assert_eq!(
            bbands.output_names,
            vec!["UPPERBAND", "MIDDLEBAND", "LOWERBAND"]
        );
        let mama = catalog
            .operations
            .iter()
            .find(|operation| operation.name == "MAMA")
            .unwrap();
        assert_eq!(mama.output_names, vec!["MAMA", "FAMA"]);
        let accbands = catalog
            .operations
            .iter()
            .find(|operation| operation.name == "ACCBANDS")
            .unwrap();
        assert_eq!(
            accbands.output_names,
            vec!["UPPERBAND", "MIDDLEBAND", "LOWERBAND"]
        );
        let minmax = catalog
            .operations
            .iter()
            .find(|operation| operation.name == "MINMAX")
            .unwrap();
        assert_eq!(minmax.output_names, vec!["MIN", "MAX"]);
        let minmaxindex = catalog
            .operations
            .iter()
            .find(|operation| operation.name == "MINMAXINDEX")
            .unwrap();
        assert_eq!(minmaxindex.output_names, vec!["MININDEX", "MAXINDEX"]);
    }

    #[test]
    fn catalog_marks_new_talib_dispatch_groups() {
        let catalog = operation_catalog(&builtin_operation_registry());
        for name in [
            "ADD",
            "STDDEV",
            "LINEARREG",
            "LINEARREG_ANGLE",
            "LINEARREG_INTERCEPT",
            "LINEARREG_SLOPE",
            "AD",
            "ADOSC",
            "SQRT",
            "DEMA",
            "TEMA",
            "T3",
            "MAMA",
            "HT_DCPERIOD",
            "HT_DCPHASE",
            "HT_PHASOR",
            "HT_SINE",
            "HT_TRENDMODE",
            "HT_TRENDLINE",
            "CDLDOJI",
            "CDL2CROWS",
            "CDLABANDONEDBABY",
            "CDLDOJISTAR",
            "CDLENGULFING",
            "CDLHAMMER",
            "CDLHARAMI",
            "CDLMARUBOZU",
            "CDLPIERCING",
            "CDLSHOOTINGSTAR",
            "CDLSPINNINGTOP",
            "CDLXSIDEGAP3METHODS",
            "MAVP",
            "SAREXT",
            "CMO",
            "MACDEXT",
            "MACDFIX",
            "ACCBANDS",
            "AVGDEV",
            "IMI",
            "MINMAX",
            "MINMAXINDEX",
        ] {
            let operation = catalog
                .operations
                .iter()
                .find(|operation| operation.name == name)
                .unwrap_or_else(|| panic!("missing operation {name}"));
            assert!(operation
                .semantic_profiles
                .iter()
                .any(|profile| profile == "talib_0_8_0"));
        }
    }

    #[test]
    fn catalog_exposes_every_profile_only_talib_name() {
        let registry = builtin_operation_registry();
        let catalog = operation_catalog(&registry);
        for name in TALIB_PROFILE_CATALOG_NAMES {
            let operation = catalog
                .operations
                .iter()
                .find(|operation| operation.name == *name)
                .unwrap_or_else(|| panic!("missing TA-Lib profile operation {name}"));
            if registry.get(name).is_some() {
                assert_eq!(
                    operation.semantic_profiles,
                    vec!["core_registry", "talib_0_8_0"]
                );
            } else {
                assert_eq!(operation.semantic_profiles, vec!["talib_0_8_0"]);
            }
            assert!(talib_profile_supported(name));
        }
    }

    #[test]
    fn profile_only_entries_publish_executable_parameter_contracts() {
        let catalog = operation_catalog(&builtin_operation_registry());
        let find = |name: &str| {
            catalog
                .operations
                .iter()
                .find(|operation| operation.name == name)
                .unwrap_or_else(|| panic!("missing operation {name}"))
        };

        let stoch = find("STOCH");
        assert_eq!(
            stoch
                .params
                .iter()
                .map(|param| param.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "fastk_period",
                "slowk_period",
                "slowk_matype",
                "slowd_period",
                "slowd_matype",
            ]
        );
        assert_eq!(stoch.params[0].default.as_deref(), Some("5"));

        let apo = find("APO");
        assert_eq!(apo.params.len(), 3);
        assert_eq!(apo.params[2].name, "matype");
        assert_eq!(apo.params[2].constraint.as_deref(), Some("integer in 0..8"));

        let candle = find("CDLDOJI");
        assert!(candle.params.is_empty());
    }

    #[test]
    fn profile_only_output_names_match_dispatcher_contract() {
        let catalog = operation_catalog(&builtin_operation_registry());
        let expected = [
            ("AC", vec!["REAL"]),
            ("ADR", vec!["REAL"]),
            ("AROON", vec!["AROON_UP", "AROON_DOWN"]),
            ("ERI", vec!["BULLPOWER", "BEARPOWER"]),
            ("FRACTAL", vec!["SWINGHIGH", "SWINGLOW"]),
            ("KC", vec!["UPPERBAND", "MIDDLEBAND", "LOWERBAND"]),
            ("SMI", vec!["SMI", "SMISIGNAL"]),
            ("VORTEX", vec!["PLUSVI", "MINUSVI"]),
        ];

        for (name, output_names) in expected {
            let entry = catalog
                .operations
                .iter()
                .find(|operation| operation.name == name)
                .unwrap_or_else(|| panic!("missing operation {name}"));
            assert_eq!(
                entry.output_names, output_names,
                "catalog output names for {name}"
            );
            assert_eq!(
                entry.outputs,
                output_names.len(),
                "catalog output count for {name}"
            );
        }

        let kdj = catalog
            .operations
            .iter()
            .find(|operation| operation.name == "KDJ")
            .expect("missing core KDJ operation");
        let kdj_profile = kdj
            .profile_output_contracts
            .get(TALIB_SEMANTIC_PROFILE)
            .expect("KDJ must expose its TA-Lib profile output schema");
        assert_eq!(kdj_profile.value_shape, "multi_series");
        assert_eq!(kdj_profile.output_names, vec!["K", "D", "J"]);
        assert_eq!(
            kdj_profile
                .params
                .iter()
                .map(|param| param.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "fastk_period",
                "slowk_period",
                "slowk_matype",
                "slowd_period",
                "slowd_matype",
            ]
        );
    }
}
