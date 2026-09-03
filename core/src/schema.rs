//! Owned canonical API schema derived from the function registry.
//!
//! Bindings, CLI tooling and documentation generators should consume this
//! schema rather than duplicate parameter defaults, aliases, lookback or
//! execution capability metadata in each language package.

use crate::compute::{ComputeCapabilities, ComputeEffect, LookbackRequirement};
use crate::registry::{
    builtin_function_registry, FunctionCategory, FunctionRegistry, FunctionSpec, InputKind,
};

/// Stable schema identifier for the v1 function metadata contract.
pub const FUNCTION_SCHEMA_VERSION: &str = "finkit.function.v1";

/// Owned machine-readable snapshot of the canonical function registry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FunctionApiSchema {
    /// Schema contract identifier, independent from the Finkit package version.
    pub schema_version: String,
    /// Canonical functions in deterministic name order.
    pub functions: Vec<FunctionApiSpec>,
}

impl FunctionApiSchema {
    /// Build a schema snapshot from an explicit function registry.
    pub fn from_registry(registry: &FunctionRegistry) -> Self {
        Self {
            schema_version: FUNCTION_SCHEMA_VERSION.to_string(),
            functions: registry.iter().map(FunctionApiSpec::from_spec).collect(),
        }
    }

    /// Build the schema for Finkit's built-in canonical function registry.
    pub fn builtin() -> Self {
        Self::from_registry(&builtin_function_registry())
    }

    /// Resolve a canonical function name or compatibility alias.
    pub fn get(&self, name: &str) -> Option<&FunctionApiSpec> {
        let requested = name.trim();
        self.functions.iter().find(|spec| {
            spec.name.eq_ignore_ascii_case(requested)
                || spec
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(requested))
        })
    }
}

/// Owned public function metadata suitable for FFI/binding code generation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FunctionApiSpec {
    /// Canonical uppercase function name.
    pub name: String,
    /// Compatibility aliases.
    pub aliases: Vec<String>,
    /// Stable category identifier.
    pub category: String,
    /// Stable input-shape identifier.
    pub input: String,
    /// Parameter declarations.
    pub params: Vec<ParamApiSpec>,
    /// Number of output series.
    pub outputs: usize,
    /// Stable lookback identifier.
    pub lookback: String,
    /// Whether incremental/streaming evaluation is available.
    pub streaming: bool,
    /// Whether identical inputs deterministically produce identical outputs.
    pub deterministic: bool,
    /// Whether the canonical value operation itself is stateful.
    pub stateful: bool,
    /// Planner-visible effect class.
    pub effect: String,
}

impl FunctionApiSpec {
    fn from_spec(spec: &FunctionSpec) -> Self {
        let capabilities = ComputeCapabilities::from_function_spec(spec);
        Self {
            name: spec.name.to_string(),
            aliases: spec
                .aliases
                .iter()
                .map(|alias| (*alias).to_string())
                .collect(),
            category: category_name(spec.category).to_string(),
            input: input_name(spec.input).to_string(),
            params: spec.params.iter().map(ParamApiSpec::from_spec).collect(),
            outputs: spec.outputs,
            lookback: lookback_name(capabilities.lookback).to_string(),
            streaming: capabilities.streaming,
            deterministic: capabilities.deterministic,
            stateful: capabilities.stateful,
            effect: effect_name(&capabilities.effect).to_string(),
        }
    }
}

/// Owned parameter metadata used by generated SDK surfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ParamApiSpec {
    /// Stable parameter name.
    pub name: String,
    /// Language-neutral textual value type.
    pub value_type: String,
    /// Optional default rendered as canonical text.
    pub default: Option<String>,
    /// Optional human-readable constraint.
    pub constraint: Option<String>,
}

impl ParamApiSpec {
    fn from_spec(spec: &crate::registry::ParamSpec) -> Self {
        Self {
            name: spec.name.to_string(),
            value_type: spec.value_type.to_string(),
            default: spec.default.map(str::to_string),
            constraint: spec.constraint.map(str::to_string),
        }
    }
}

const fn category_name(category: FunctionCategory) -> &'static str {
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

const fn input_name(input: InputKind) -> &'static str {
    match input {
        InputKind::Series => "series",
        InputKind::Hlc => "hlc",
        InputKind::Hlcv => "hlcv",
        InputKind::Ohlcv => "ohlcv",
        InputKind::Dynamic => "dynamic",
    }
}

const fn lookback_name(lookback: LookbackRequirement) -> &'static str {
    match lookback {
        LookbackRequirement::None => "none",
        LookbackRequirement::PeriodMinusOne => "period_minus_one",
        LookbackRequirement::Period => "period",
        LookbackRequirement::Fixed(_) => "fixed",
        LookbackRequirement::Dynamic => "dynamic",
    }
}

const fn effect_name(effect: &ComputeEffect) -> &'static str {
    match effect {
        ComputeEffect::Pure => "pure",
        ComputeEffect::WriteVariable(_) => "write_variable",
        ComputeEffect::EmitOutput(_) => "emit_output",
        ComputeEffect::Draw => "draw",
        ComputeEffect::Stateful => "stateful",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_schema_is_deterministically_sorted() {
        let schema = FunctionApiSchema::builtin();
        let names: Vec<&str> = schema
            .functions
            .iter()
            .map(|spec| spec.name.as_str())
            .collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
        assert!(!schema.functions.is_empty());
        assert_eq!(schema.schema_version, FUNCTION_SCHEMA_VERSION);
    }

    #[test]
    fn schema_preserves_parameters_and_compute_capabilities() {
        let schema = FunctionApiSchema::builtin();
        let ma = schema.get("MA").unwrap();
        let sma = schema.get("SMA").unwrap();

        assert!(ma.aliases.is_empty());
        assert_eq!(ma.category, "overlap");
        assert_eq!(ma.input, "series");
        assert_eq!(ma.outputs, 1);
        assert_eq!(ma.lookback, "period_minus_one");
        assert!(ma.streaming);
        assert!(ma.deterministic);
        assert!(!ma.stateful);
        assert_eq!(ma.effect, "pure");
        assert_eq!(ma.params.len(), 1);
        assert_eq!(ma.params[0].name, "period");

        assert!(sma.aliases.is_empty());
        assert_eq!(sma.lookback, "dynamic");
        assert_eq!(sma.params.len(), 2);
        assert_eq!(sma.params[0].name, "period");
        assert_eq!(sma.params[1].name, "m");
        assert_eq!(sma.params[1].default.as_deref(), Some("1"));
    }

    #[test]
    fn schema_lookup_resolves_names_and_aliases_case_insensitively() {
        let schema = FunctionApiSchema::builtin();
        assert_eq!(schema.get("ma").unwrap().name, "MA");
        assert_eq!(schema.get("sma").unwrap().name, "SMA");
        assert_eq!(schema.get("boll").unwrap().name, "BBANDS");
        assert!(schema.get("not-a-function").is_none());
    }
}
