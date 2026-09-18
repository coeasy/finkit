//! Language-neutral canonical operation catalog.
//!
//! The catalog is generated from the Rust core registry at runtime and
//! serialized into a small stable JSON envelope. All bindings can expose this
//! exact metadata without maintaining language-specific copies of parameter,
//! shape, or capability declarations.

use crate::execute::talib_profile_supported;
use finkit::operation::{
    builtin_operation_registry, OperationCapabilities, OperationKind, OperationRegistry,
    OperationSpec,
};
use finkit::registry::{FunctionCategory, InputKind, LookbackSpec};
use serde::Serialize;

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
            lookback: lookback_name(spec.lookback),
            capabilities: spec.capabilities.into(),
            schema_version: spec.schema_version,
            semantic_profiles: {
                let mut profiles = vec!["core_registry".to_string()];
                if talib_profile_supported(&spec.name) {
                    profiles.push("talib_0_7_1".to_string());
                }
                profiles
            },
        }
    }
}

/// Build the shared operation catalog from a supplied registry.
pub fn operation_catalog(registry: &OperationRegistry) -> OperationCatalogEnvelope {
    OperationCatalogEnvelope {
        schema_version: 1,
        engine_version: env!("CARGO_PKG_VERSION"),
        operations: registry
            .iter()
            .map(OperationCatalogEntry::from_spec)
            .collect(),
    }
}

/// Build the built-in operation catalog as a JSON string.
pub fn operation_catalog_json() -> Result<String, serde_json::Error> {
    serde_json::to_string(&operation_catalog(&builtin_operation_registry()))
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
        assert!(json.contains("\"semantic_profiles\":[\"core_registry\",\"talib_0_7_1\"]"));
    }

    #[test]
    fn catalog_projection_preserves_registry_order_and_count() {
        let registry = builtin_operation_registry();
        let catalog = operation_catalog(&registry);
        assert_eq!(catalog.operations.len(), registry.len());
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
        ] {
            let operation = catalog
                .operations
                .iter()
                .find(|operation| operation.name == name)
                .unwrap_or_else(|| panic!("missing operation {name}"));
            assert!(operation
                .semantic_profiles
                .iter()
                .any(|profile| profile == "talib_0_7_1"));
        }
    }
}
