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
    let (value_shape, output_names) = talib_profile_output_shape(name, &[]);
    let outputs = output_names.len();
    let input = Some(talib_profile_input_kind(name));
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

const TALIB_PROFILE_INPUT_SPECS: &[(&str, &str)] = &[
    ("AVGPRICE", "ohlcv"),
    ("BOP", "ohlcv"),
    ("HA", "ohlcv"),
    ("AC", "hlc"),
    ("ADR", "hlc"),
    ("AO", "hlc"),
    ("DONCHIAN", "hlc"),
    ("ERI", "hlc"),
    ("FRACTAL", "hlc"),
    ("KC", "hlc"),
    ("MASSI", "hlc"),
    ("MEDPRICE", "hlc"),
    ("MIDPRICE", "hlc"),
    ("SAR", "hlc"),
    ("SUPERTREND", "hlc"),
    ("VORTEX", "hlc"),
    ("WAD", "hlc"),
    ("CMF", "hlcv"),
    ("MARKETFI", "dynamic"),
    ("EFI", "dynamic"),
    ("NVI", "dynamic"),
    ("PVI", "dynamic"),
    ("PVO", "dynamic"),
    ("PVT", "dynamic"),
    ("QSTICK", "dynamic"),
    ("RVOL", "dynamic"),
];

fn talib_profile_input_kind(name: &str) -> &'static str {
    if name.starts_with("CDL") {
        return "ohlcv";
    }
    TALIB_PROFILE_INPUT_SPECS
        .iter()
        .find(|(operation, _)| *operation == name)
        .map_or("series", |(_, input)| input)
}

struct TalibProfileOutputSpec {
    name: &'static str,
    value_shape: &'static str,
    output_names: &'static [&'static str],
}

const TALIB_PROFILE_OUTPUT_SPECS: &[TalibProfileOutputSpec] = &[
    TalibProfileOutputSpec {
        name: "HA",
        value_shape: "multi_series",
        output_names: &["HAOPEN", "HAHIGH", "HALOW", "HACLOSE"],
    },
    TalibProfileOutputSpec {
        name: "VORTEX",
        value_shape: "multi_series",
        output_names: &["PLUSVI", "MINUSVI"],
    },
    TalibProfileOutputSpec {
        name: "AROON",
        value_shape: "multi_series",
        output_names: &["AROON_UP", "AROON_DOWN"],
    },
    TalibProfileOutputSpec {
        name: "STOCH",
        value_shape: "multi_series",
        output_names: &["SLOWK", "SLOWD"],
    },
    TalibProfileOutputSpec {
        name: "STOCHF",
        value_shape: "multi_series",
        output_names: &["FASTK", "FASTD"],
    },
    TalibProfileOutputSpec {
        name: "STOCHRSI",
        value_shape: "multi_series",
        output_names: &["FASTK", "FASTD"],
    },
    TalibProfileOutputSpec {
        name: "ERI",
        value_shape: "multi_series",
        output_names: &["BULLPOWER", "BEARPOWER"],
    },
    TalibProfileOutputSpec {
        name: "FRACTAL",
        value_shape: "multi_series",
        output_names: &["SWINGHIGH", "SWINGLOW"],
    },
    TalibProfileOutputSpec {
        name: "KC",
        value_shape: "multi_series",
        output_names: &["UPPERBAND", "MIDDLEBAND", "LOWERBAND"],
    },
    TalibProfileOutputSpec {
        name: "DONCHIAN",
        value_shape: "multi_series",
        output_names: &["UPPERBAND", "MIDDLEBAND", "LOWERBAND"],
    },
    TalibProfileOutputSpec {
        name: "KDJ",
        value_shape: "multi_series",
        output_names: &["K", "D", "J"],
    },
    TalibProfileOutputSpec {
        name: "SMI",
        value_shape: "multi_series",
        output_names: &["SMI", "SMISIGNAL"],
    },
    TalibProfileOutputSpec {
        name: "SUPERTREND",
        value_shape: "multi_series",
        output_names: &["SUPERTREND", "TREND"],
    },
    TalibProfileOutputSpec {
        name: "AC",
        value_shape: "series",
        output_names: &["REAL"],
    },
    TalibProfileOutputSpec {
        name: "ADR",
        value_shape: "series",
        output_names: &["REAL"],
    },
    TalibProfileOutputSpec {
        name: "CMOU",
        value_shape: "series",
        output_names: &["REAL"],
    },
    TalibProfileOutputSpec {
        name: "CVI",
        value_shape: "series",
        output_names: &["REAL"],
    },
    TalibProfileOutputSpec {
        name: "EFI",
        value_shape: "series",
        output_names: &["REAL"],
    },
    TalibProfileOutputSpec {
        name: "FOSC",
        value_shape: "series",
        output_names: &["REAL"],
    },
    TalibProfileOutputSpec {
        name: "MARKETFI",
        value_shape: "series",
        output_names: &["REAL"],
    },
    TalibProfileOutputSpec {
        name: "MASSI",
        value_shape: "series",
        output_names: &["REAL"],
    },
    TalibProfileOutputSpec {
        name: "PERCENTILE",
        value_shape: "series",
        output_names: &["REAL"],
    },
    TalibProfileOutputSpec {
        name: "PVO",
        value_shape: "series",
        output_names: &["REAL"],
    },
    TalibProfileOutputSpec {
        name: "QSTICK",
        value_shape: "series",
        output_names: &["REAL"],
    },
    TalibProfileOutputSpec {
        name: "RMA",
        value_shape: "series",
        output_names: &["REAL"],
    },
    TalibProfileOutputSpec {
        name: "RVI",
        value_shape: "series",
        output_names: &["REAL"],
    },
    TalibProfileOutputSpec {
        name: "RVOL",
        value_shape: "series",
        output_names: &["REAL"],
    },
    TalibProfileOutputSpec {
        name: "VHF",
        value_shape: "series",
        output_names: &["REAL"],
    },
    TalibProfileOutputSpec {
        name: "WAD",
        value_shape: "series",
        output_names: &["REAL"],
    },
];

fn talib_profile_output_shape(name: &str, fallback: &[String]) -> (&'static str, Vec<String>) {
    if let Some(spec) = TALIB_PROFILE_OUTPUT_SPECS
        .iter()
        .find(|spec| spec.name == name)
    {
        return (
            spec.value_shape,
            spec.output_names
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
        );
    }
    if fallback.len() > 1 {
        ("multi_series", fallback.to_vec())
    } else if let Some(fallback_name) = fallback.first() {
        ("series", vec![fallback_name.clone()])
    } else {
        ("series", vec![name.to_string()])
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

#[derive(Debug, Clone, Copy)]
struct TalibParameterSpec {
    name: &'static str,
    value_type: &'static str,
    default: Option<&'static str>,
    constraint: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
struct TalibProfileParameterSpec {
    name: &'static str,
    params: &'static [TalibParameterSpec],
}

macro_rules! define_talib_params {
    ($name:ident: $(($param:literal, $value_type:literal, $default:expr, $constraint:expr)),* $(,)?) => {
        const $name: &[TalibParameterSpec] = &[
            $(TalibParameterSpec {
                name: $param,
                value_type: $value_type,
                default: $default,
                constraint: $constraint,
            }),*
        ];
    };
}

define_talib_params!(TALIB_PARAMS_PERIOD_5:
    ("timeperiod", "integer", Some("5"), Some("integer >= 1")),
);
define_talib_params!(TALIB_PARAMS_PERIOD_10:
    ("timeperiod", "integer", Some("10"), Some("integer >= 1")),
);
define_talib_params!(TALIB_PARAMS_PERIOD_13:
    ("timeperiod", "integer", Some("13"), Some("integer >= 1")),
);
define_talib_params!(TALIB_PARAMS_PERIOD_14:
    ("timeperiod", "integer", Some("14"), Some("integer >= 1")),
);
define_talib_params!(TALIB_PARAMS_PERIOD_20:
    ("timeperiod", "integer", Some("20"), Some("integer >= 1")),
);
define_talib_params!(TALIB_PARAMS_PERIOD_28:
    ("timeperiod", "integer", Some("28"), Some("integer >= 1")),
);
define_talib_params!(TALIB_PARAMS_PERIOD_30:
    ("timeperiod", "integer", Some("30"), Some("integer >= 1")),
);
define_talib_params!(TALIB_PARAMS_PERIOD_100:
    ("timeperiod", "integer", Some("100"), Some("integer >= 1")),
);
define_talib_params!(TALIB_PARAMS_MA:
    ("timeperiod", "integer", Some("30"), Some("integer >= 1")),
    ("matype", "integer", Some("0"), Some("integer in 0..8")),
);
define_talib_params!(TALIB_PARAMS_SAR:
    ("acceleration", "number", Some("0.02"), Some("finite")),
    ("maximum", "number", Some("0.2"), Some("finite")),
);
define_talib_params!(TALIB_PARAMS_SAREXT:
    ("startvalue", "number", Some("0.0"), Some("finite")),
    ("offsetonreverse", "number", Some("0.0"), Some("finite")),
    ("afinitlong", "number", Some("0.02"), Some("finite")),
    ("aflong", "number", Some("0.02"), Some("finite")),
    ("afmaxlong", "number", Some("0.2"), Some("finite")),
    ("afinitshort", "number", Some("0.02"), Some("finite")),
    ("afshort", "number", Some("0.02"), Some("finite")),
    ("afmaxshort", "number", Some("0.2"), Some("finite")),
);
define_talib_params!(TALIB_PARAMS_PERIOD_30_NBDEV:
    ("timeperiod", "integer", Some("30"), Some("integer >= 1")),
    ("nbdev", "number", Some("1.0"), Some("finite")),
);
define_talib_params!(TALIB_PARAMS_AC:
    ("fastperiod", "integer", Some("5"), Some("integer >= 2")),
    ("slowperiod", "integer", Some("34"), Some("integer >= 2")),
    ("signalperiod", "integer", Some("5"), Some("integer >= 2")),
);
define_talib_params!(TALIB_PARAMS_CVI:
    ("timeperiod", "integer", Some("10"), Some("integer >= 1")),
    ("rocperiod", "integer", Some("10"), Some("integer >= 1")),
);
define_talib_params!(TALIB_PARAMS_FRACTAL:
    ("leftbars", "integer", Some("2"), Some("integer >= 1")),
    ("rightbars", "integer", Some("2"), Some("integer >= 1")),
);
define_talib_params!(TALIB_PARAMS_KC:
    ("timeperiod", "integer", Some("20"), Some("integer >= 2")),
    ("atrperiod", "integer", Some("10"), Some("integer >= 1")),
    ("nbdev", "number", Some("2.0"), Some("finite number")),
);
define_talib_params!(TALIB_PARAMS_KDJ:
    ("fastk_period", "integer", Some("9"), Some("integer >= 1")),
    ("slowk_period", "integer", Some("3"), Some("integer >= 1")),
    ("slowk_matype", "integer", Some("13"), Some("TA-Lib MA type")),
    ("slowd_period", "integer", Some("3"), Some("integer >= 1")),
    ("slowd_matype", "integer", Some("13"), Some("TA-Lib MA type")),
);
define_talib_params!(TALIB_PARAMS_MASSI:
    ("fastperiod", "integer", Some("9"), Some("integer >= 1")),
    ("slowperiod", "integer", Some("25"), Some("integer >= 1")),
);
define_talib_params!(TALIB_PARAMS_PERCENTILE:
    ("timeperiod", "integer", Some("30"), Some("integer >= 1")),
    ("percentile", "number", Some("50.0"), Some("0 <= value <= 100")),
);
define_talib_params!(TALIB_PARAMS_PVO:
    ("fastperiod", "integer", Some("12"), Some("integer >= 1")),
    ("slowperiod", "integer", Some("26"), Some("integer >= 1")),
    ("matype", "integer", Some("1"), Some("TA-Lib MA type")),
);
define_talib_params!(TALIB_PARAMS_RVI:
    ("timeperiod", "integer", Some("14"), Some("integer >= 1")),
    ("stddevperiod", "integer", Some("10"), Some("integer >= 2")),
);
define_talib_params!(TALIB_PARAMS_SMI:
    ("timeperiod", "integer", Some("13"), Some("integer >= 2")),
    ("fastperiod", "integer", Some("2"), Some("integer >= 2")),
    ("slowperiod", "integer", Some("25"), Some("integer >= 2")),
    ("signalperiod", "integer", Some("9"), Some("integer >= 2")),
);
define_talib_params!(TALIB_PARAMS_AO:
    ("fastperiod", "integer", Some("5"), Some("integer >= 1")),
    ("slowperiod", "integer", Some("34"), Some("integer >= 1")),
);
define_talib_params!(TALIB_PARAMS_COPPOCK:
    ("wmaperiod", "integer", Some("10"), Some("integer >= 1")),
    ("roc1period", "integer", Some("11"), Some("integer >= 1")),
    ("roc2period", "integer", Some("14"), Some("integer >= 1")),
);
define_talib_params!(TALIB_PARAMS_TSI:
    ("firstperiod", "integer", Some("25"), Some("integer >= 1")),
    ("secondperiod", "integer", Some("13"), Some("integer >= 1")),
);
define_talib_params!(TALIB_PARAMS_SUPERTREND:
    ("timeperiod", "integer", Some("10"), Some("integer >= 1")),
    ("multiplier", "number", Some("3.0"), Some("value > 0")),
);
define_talib_params!(TALIB_PARAMS_APO:
    ("fastperiod", "integer", Some("12"), Some("integer >= 1")),
    ("slowperiod", "integer", Some("26"), Some("integer >= 1")),
    ("matype", "integer", Some("0"), Some("integer in 0..8")),
);
define_talib_params!(TALIB_PARAMS_STOCH:
    ("fastk_period", "integer", Some("5"), Some("integer >= 1")),
    ("slowk_period", "integer", Some("3"), Some("integer >= 1")),
    ("slowk_matype", "integer", Some("0"), Some("integer in 0..8")),
    ("slowd_period", "integer", Some("3"), Some("integer >= 1")),
    ("slowd_matype", "integer", Some("0"), Some("integer in 0..8")),
);
define_talib_params!(TALIB_PARAMS_STOCHF:
    ("fastk_period", "integer", Some("5"), Some("integer >= 1")),
    ("fastd_period", "integer", Some("3"), Some("integer >= 1")),
    ("fastd_matype", "integer", Some("0"), Some("integer in 0..8")),
);
define_talib_params!(TALIB_PARAMS_STOCHRSI:
    ("timeperiod", "integer", Some("14"), Some("integer >= 1")),
    ("fastk_period", "integer", Some("5"), Some("integer >= 1")),
    ("fastd_period", "integer", Some("3"), Some("integer >= 1")),
    ("fastd_matype", "integer", Some("0"), Some("integer in 0..8")),
);
define_talib_params!(TALIB_PARAMS_MAVP:
    ("minperiod", "integer", Some("2"), Some("integer >= 2")),
    ("maxperiod", "integer", Some("30"), Some("integer >= minperiod")),
    ("matype", "integer", Some("0"), Some("integer in 0..8")),
);
define_talib_params!(TALIB_PARAMS_STAT_PERIOD_14:
    ("timeperiod", "integer", Some("14"), Some("integer >= 1")),
);
define_talib_params!(TALIB_PARAMS_STAT_PERIOD_30:
    ("timeperiod", "integer", Some("30"), Some("integer >= 1")),
);
define_talib_params!(TALIB_PARAMS_T3:
    ("timeperiod", "integer", Some("5"), Some("integer >= 1")),
    ("vfactor", "number", Some("0.7"), Some("0.0 <= value <= 1.0")),
);

const TALIB_PROFILE_PARAMETER_SPECS: &[TalibProfileParameterSpec] = &[
    TalibProfileParameterSpec {
        name: "MA",
        params: TALIB_PARAMS_MA,
    },
    TalibProfileParameterSpec {
        name: "SMA",
        params: TALIB_PARAMS_PERIOD_30,
    },
    TalibProfileParameterSpec {
        name: "SAR",
        params: TALIB_PARAMS_SAR,
    },
    TalibProfileParameterSpec {
        name: "SAREXT",
        params: TALIB_PARAMS_SAREXT,
    },
    TalibProfileParameterSpec {
        name: "MIDPOINT",
        params: TALIB_PARAMS_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "MIDPRICE",
        params: TALIB_PARAMS_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "STDDEV",
        params: TALIB_PARAMS_PERIOD_30_NBDEV,
    },
    TalibProfileParameterSpec {
        name: "VAR",
        params: TALIB_PARAMS_PERIOD_30_NBDEV,
    },
    TalibProfileParameterSpec {
        name: "AC",
        params: TALIB_PARAMS_AC,
    },
    TalibProfileParameterSpec {
        name: "ADR",
        params: TALIB_PARAMS_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "CMOU",
        params: TALIB_PARAMS_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "CVI",
        params: TALIB_PARAMS_CVI,
    },
    TalibProfileParameterSpec {
        name: "EFI",
        params: TALIB_PARAMS_PERIOD_13,
    },
    TalibProfileParameterSpec {
        name: "ERI",
        params: TALIB_PARAMS_PERIOD_13,
    },
    TalibProfileParameterSpec {
        name: "FOSC",
        params: TALIB_PARAMS_PERIOD_5,
    },
    TalibProfileParameterSpec {
        name: "FRACTAL",
        params: TALIB_PARAMS_FRACTAL,
    },
    TalibProfileParameterSpec {
        name: "KC",
        params: TALIB_PARAMS_KC,
    },
    TalibProfileParameterSpec {
        name: "KDJ",
        params: TALIB_PARAMS_KDJ,
    },
    TalibProfileParameterSpec {
        name: "MASSI",
        params: TALIB_PARAMS_MASSI,
    },
    TalibProfileParameterSpec {
        name: "PERCENTILE",
        params: TALIB_PARAMS_PERCENTILE,
    },
    TalibProfileParameterSpec {
        name: "PVO",
        params: TALIB_PARAMS_PVO,
    },
    TalibProfileParameterSpec {
        name: "QSTICK",
        params: TALIB_PARAMS_PERIOD_10,
    },
    TalibProfileParameterSpec {
        name: "RMA",
        params: TALIB_PARAMS_PERIOD_30,
    },
    TalibProfileParameterSpec {
        name: "RVI",
        params: TALIB_PARAMS_RVI,
    },
    TalibProfileParameterSpec {
        name: "RVOL",
        params: TALIB_PARAMS_PERIOD_20,
    },
    TalibProfileParameterSpec {
        name: "SMI",
        params: TALIB_PARAMS_SMI,
    },
    TalibProfileParameterSpec {
        name: "VHF",
        params: TALIB_PARAMS_PERIOD_28,
    },
    TalibProfileParameterSpec {
        name: "AO",
        params: TALIB_PARAMS_AO,
    },
    TalibProfileParameterSpec {
        name: "CMF",
        params: TALIB_PARAMS_PERIOD_20,
    },
    TalibProfileParameterSpec {
        name: "COPPOCK",
        params: TALIB_PARAMS_COPPOCK,
    },
    TalibProfileParameterSpec {
        name: "DPO",
        params: TALIB_PARAMS_PERIOD_20,
    },
    TalibProfileParameterSpec {
        name: "ER",
        params: TALIB_PARAMS_PERIOD_10,
    },
    TalibProfileParameterSpec {
        name: "HMA",
        params: TALIB_PARAMS_PERIOD_20,
    },
    TalibProfileParameterSpec {
        name: "PERCENTRANK",
        params: TALIB_PARAMS_PERIOD_100,
    },
    TalibProfileParameterSpec {
        name: "TSI",
        params: TALIB_PARAMS_TSI,
    },
    TalibProfileParameterSpec {
        name: "VORTEX",
        params: TALIB_PARAMS_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "VWMA",
        params: TALIB_PARAMS_PERIOD_30,
    },
    TalibProfileParameterSpec {
        name: "ZLEMA",
        params: TALIB_PARAMS_PERIOD_30,
    },
    TalibProfileParameterSpec {
        name: "SUPERTREND",
        params: TALIB_PARAMS_SUPERTREND,
    },
    TalibProfileParameterSpec {
        name: "ADX",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "ADXR",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "AROON",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "AROONOSC",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "CCI",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "CMO",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "DX",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "MFI",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "MINUS_DI",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "MINUS_DM",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "PLUS_DI",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "PLUS_DM",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "RSI",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "WILLR",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "ROCP",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "ROCR",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "ROCR100",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "APO",
        params: TALIB_PARAMS_APO,
    },
    TalibProfileParameterSpec {
        name: "PPO",
        params: TALIB_PARAMS_APO,
    },
    TalibProfileParameterSpec {
        name: "STOCH",
        params: TALIB_PARAMS_STOCH,
    },
    TalibProfileParameterSpec {
        name: "STOCHF",
        params: TALIB_PARAMS_STOCHF,
    },
    TalibProfileParameterSpec {
        name: "STOCHRSI",
        params: TALIB_PARAMS_STOCHRSI,
    },
    TalibProfileParameterSpec {
        name: "MAVP",
        params: TALIB_PARAMS_MAVP,
    },
    TalibProfileParameterSpec {
        name: "BETA",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "CORREL",
        params: TALIB_PARAMS_STAT_PERIOD_14,
    },
    TalibProfileParameterSpec {
        name: "MAX",
        params: TALIB_PARAMS_STAT_PERIOD_30,
    },
    TalibProfileParameterSpec {
        name: "MIN",
        params: TALIB_PARAMS_STAT_PERIOD_30,
    },
    TalibProfileParameterSpec {
        name: "MAXINDEX",
        params: TALIB_PARAMS_STAT_PERIOD_30,
    },
    TalibProfileParameterSpec {
        name: "MININDEX",
        params: TALIB_PARAMS_STAT_PERIOD_30,
    },
    TalibProfileParameterSpec {
        name: "MINMAX",
        params: TALIB_PARAMS_STAT_PERIOD_30,
    },
    TalibProfileParameterSpec {
        name: "MINMAXINDEX",
        params: TALIB_PARAMS_STAT_PERIOD_30,
    },
    TalibProfileParameterSpec {
        name: "SUM",
        params: TALIB_PARAMS_STAT_PERIOD_30,
    },
    TalibProfileParameterSpec {
        name: "LINEARREG",
        params: TALIB_PARAMS_STAT_PERIOD_30,
    },
    TalibProfileParameterSpec {
        name: "LINEARREG_ANGLE",
        params: TALIB_PARAMS_STAT_PERIOD_30,
    },
    TalibProfileParameterSpec {
        name: "LINEARREG_INTERCEPT",
        params: TALIB_PARAMS_STAT_PERIOD_30,
    },
    TalibProfileParameterSpec {
        name: "LINEARREG_SLOPE",
        params: TALIB_PARAMS_STAT_PERIOD_30,
    },
    TalibProfileParameterSpec {
        name: "TSF",
        params: TALIB_PARAMS_STAT_PERIOD_30,
    },
    TalibProfileParameterSpec {
        name: "T3",
        params: TALIB_PARAMS_T3,
    },
    TalibProfileParameterSpec {
        name: "TRIX",
        params: TALIB_PARAMS_STAT_PERIOD_30,
    },
];

/// Parameters for names that are not projected from the Core registry.
///
/// These describe the currently executable shared-dispatcher contract. The
/// TA-Lib MA type range is intentionally exposed only where the underlying
/// Core kernel consumes it; a catalog entry must never advertise a parameter
/// that the executor silently ignores.
fn talib_profile_params(name: &str) -> Vec<OperationParameter> {
    let Some(spec) = TALIB_PROFILE_PARAMETER_SPECS
        .iter()
        .find(|spec| spec.name == name)
    else {
        return Vec::new();
    };
    spec.params
        .iter()
        .map(|param| OperationParameter {
            name: param.name.to_string(),
            value_type: param.value_type.to_string(),
            default: param.default.map(str::to_string),
            constraint: param.constraint.map(str::to_string),
        })
        .collect()
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
            ("DONCHIAN", vec!["UPPERBAND", "MIDDLEBAND", "LOWERBAND"]),
            ("ERI", vec!["BULLPOWER", "BEARPOWER"]),
            ("FRACTAL", vec!["SWINGHIGH", "SWINGLOW"]),
            ("KC", vec!["UPPERBAND", "MIDDLEBAND", "LOWERBAND"]),
            ("SMI", vec!["SMI", "SMISIGNAL"]),
            ("SUPERTREND", vec!["SUPERTREND", "TREND"]),
            ("VORTEX", vec!["PLUSVI", "MINUSVI"]),
        ];

        for (name, output_names) in expected {
            let entry = catalog
                .operations
                .iter()
                .find(|operation| operation.name == name)
                .unwrap_or_else(|| panic!("missing operation {name}"));
            let profile = entry
                .profile_output_contracts
                .get(TALIB_SEMANTIC_PROFILE)
                .unwrap_or_else(|| panic!("missing TA-Lib profile for {name}"));
            assert_eq!(
                profile.output_names, output_names,
                "TA-Lib profile output names for {name}"
            );
            assert_eq!(
                profile.outputs,
                output_names.len(),
                "TA-Lib profile output count for {name}"
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

    #[test]
    fn talib_profile_output_schema_table_is_unique_and_non_empty() {
        let mut names = std::collections::BTreeSet::new();
        for spec in TALIB_PROFILE_OUTPUT_SPECS {
            assert!(
                names.insert(spec.name),
                "duplicate TA-Lib output spec: {}",
                spec.name
            );
            assert!(
                !spec.output_names.is_empty(),
                "empty output spec: {}",
                spec.name
            );
            assert!(
                matches!(spec.value_shape, "series" | "multi_series"),
                "invalid output shape for {}: {}",
                spec.name,
                spec.value_shape
            );
        }
    }

    #[test]
    fn talib_profile_parameter_schema_table_is_unique_and_valid() {
        let mut operation_names = std::collections::BTreeSet::new();
        for spec in TALIB_PROFILE_PARAMETER_SPECS {
            assert!(
                operation_names.insert(spec.name),
                "duplicate TA-Lib parameter spec: {}",
                spec.name
            );
            assert!(
                talib_profile_supported(spec.name),
                "parameter spec is not executable: {}",
                spec.name
            );
            let mut parameter_names = std::collections::BTreeSet::new();
            for param in spec.params {
                assert!(
                    parameter_names.insert(param.name),
                    "duplicate parameter {} for {}",
                    param.name,
                    spec.name
                );
                assert!(!param.value_type.is_empty());
                assert!(param.default.is_some());
                assert!(param.constraint.is_some());
            }
        }
    }

    #[test]
    fn talib_profile_input_schema_table_is_unique_and_valid() {
        let mut operation_names = std::collections::BTreeSet::new();
        for (name, input) in TALIB_PROFILE_INPUT_SPECS {
            assert!(
                operation_names.insert(*name),
                "duplicate TA-Lib input spec: {name}"
            );
            assert!(
                talib_profile_supported(name),
                "input spec is not executable: {name}"
            );
            assert!(matches!(
                *input,
                "series" | "hlc" | "hlcv" | "ohlcv" | "dynamic"
            ));
            assert_eq!(talib_profile_input_kind(name), *input);
        }
        assert_eq!(talib_profile_input_kind("CDLDOJI"), "ohlcv");
        assert_eq!(talib_profile_input_kind("UNKNOWN"), "series");
    }
}
