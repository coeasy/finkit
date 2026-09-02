//! Stable function metadata and introspection registry.
//!
//! The registry does not replace the existing indicator or formula executors.
//! It gives bindings, CLI tools, documentation generators, and compatibility
//! layers one canonical description of public functions.

use std::collections::BTreeMap;

/// High-level category used for discovery and documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FunctionCategory {
    /// Moving averages and trend overlays.
    Overlap,
    /// Momentum and oscillator indicators.
    Momentum,
    /// Volatility indicators.
    Volatility,
    /// Volume indicators.
    Volume,
    /// Statistical functions.
    Statistics,
    /// Formula time-series primitive.
    Formula,
    /// Factor transform or factor helper.
    Factor,
}

/// Required input shape for a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    /// A single numeric series.
    Series,
    /// High, low, close series.
    Hlc,
    /// High, low, close, volume series.
    Hlcv,
    /// Open, high, low, close, volume series.
    Ohlcv,
    /// Formula expression arguments determine the exact inputs.
    Dynamic,
}

/// Parameter metadata used by bindings and help output.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamSpec {
    /// Stable parameter name.
    pub name: &'static str,
    /// Human-readable type name.
    pub value_type: &'static str,
    /// Optional default value rendered as text.
    pub default: Option<&'static str>,
    /// Human-readable constraint.
    pub constraint: Option<&'static str>,
}

impl ParamSpec {
    /// Create a parameter description.
    pub const fn new(
        name: &'static str,
        value_type: &'static str,
        default: Option<&'static str>,
        constraint: Option<&'static str>,
    ) -> Self {
        Self {
            name,
            value_type,
            default,
            constraint,
        }
    }
}

/// Lookback contract for warm-up behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookbackSpec {
    /// No warm-up rows are required.
    None,
    /// Warm-up equals `period - 1` for the primary period argument.
    PeriodMinusOne,
    /// Warm-up equals the primary period argument.
    Period,
    /// Function-specific lookback; consult the implementation.
    Dynamic,
}

/// Canonical metadata for a public indicator/formula function.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSpec {
    /// Canonical uppercase name.
    pub name: &'static str,
    /// Accepted compatibility aliases.
    pub aliases: &'static [&'static str],
    /// Discovery category.
    pub category: FunctionCategory,
    /// Input shape.
    pub input: InputKind,
    /// Parameter declarations.
    pub params: &'static [ParamSpec],
    /// Number of output series.
    pub outputs: usize,
    /// Warm-up/lookback behavior.
    pub lookback: LookbackSpec,
    /// Whether the function can be evaluated incrementally.
    pub streaming: bool,
    /// Whether repeated execution with identical input is deterministic.
    pub deterministic: bool,
}

/// Deterministic registry for function metadata.
#[derive(Debug, Clone, Default)]
pub struct FunctionRegistry {
    specs: BTreeMap<String, FunctionSpec>,
    aliases: BTreeMap<String, String>,
}

impl FunctionRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a function and all of its aliases.
    pub fn register(&mut self, spec: FunctionSpec) -> Result<(), String> {
        let canonical = normalize_name(spec.name);
        if self.specs.contains_key(&canonical) {
            return Err(format!("function already registered: {}", spec.name));
        }
        for alias in spec.aliases {
            let normalized = normalize_name(alias);
            if self.aliases.contains_key(&normalized) || self.specs.contains_key(&normalized) {
                return Err(format!("function alias already registered: {alias}"));
            }
        }
        for alias in spec.aliases {
            self.aliases
                .insert(normalize_name(alias), canonical.clone());
        }
        self.specs.insert(canonical, spec);
        Ok(())
    }

    /// Resolve a canonical name or alias case-insensitively.
    pub fn get(&self, name: &str) -> Option<&FunctionSpec> {
        let normalized = normalize_name(name);
        if let Some(spec) = self.specs.get(&normalized) {
            return Some(spec);
        }
        self.aliases
            .get(&normalized)
            .and_then(|canonical| self.specs.get(canonical))
    }

    /// Iterate over canonical specs in stable name order.
    pub fn iter(&self) -> impl Iterator<Item = &FunctionSpec> {
        self.specs.values()
    }

    /// Return all functions in a category.
    pub fn by_category(&self, category: FunctionCategory) -> Vec<&FunctionSpec> {
        self.specs
            .values()
            .filter(|spec| spec.category == category)
            .collect()
    }

    /// Number of registered canonical functions.
    pub fn len(&self) -> usize {
        self.specs.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }
}

const PERIOD_14: &[ParamSpec] = &[ParamSpec::new(
    "period",
    "usize",
    Some("14"),
    Some("> 0"),
)];
const PERIOD_20: &[ParamSpec] = &[ParamSpec::new(
    "period",
    "usize",
    Some("20"),
    Some("> 0"),
)];
const PERIOD_REQUIRED: &[ParamSpec] = &[ParamSpec::new(
    "period",
    "usize",
    None,
    Some("> 0"),
)];
const MACD_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("fast_period", "usize", Some("12"), Some("> 0")),
    ParamSpec::new("slow_period", "usize", Some("26"), Some("> fast_period")),
    ParamSpec::new("signal_period", "usize", Some("9"), Some("> 0")),
];
const BBANDS_PARAMS: &[ParamSpec] = &[
    ParamSpec::new("period", "usize", Some("20"), Some("> 1")),
    ParamSpec::new("stddev", "f64", Some("2.0"), Some(">= 0")),
];
const REF_PARAMS: &[ParamSpec] = &[ParamSpec::new(
    "bars",
    "usize",
    None,
    Some(">= 0"),
)];
const TWO_SERIES: &[ParamSpec] = &[];

/// Build the stable v0.1.0 public function registry.
pub fn builtin_function_registry() -> FunctionRegistry {
    let mut registry = FunctionRegistry::new();
    let specs = [
        FunctionSpec {
            name: "SMA",
            aliases: &["MA"],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "EMA",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "WMA",
            aliases: &[],
            category: FunctionCategory::Overlap,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MACD",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: MACD_PARAMS,
            outputs: 3,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "RSI",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: PERIOD_14,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "ROC",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "MOM",
            aliases: &["MOMENTUM"],
            category: FunctionCategory::Momentum,
            input: InputKind::Series,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "CCI",
            aliases: &[],
            category: FunctionCategory::Momentum,
            input: InputKind::Hlc,
            params: PERIOD_14,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "ATR",
            aliases: &[],
            category: FunctionCategory::Volatility,
            input: InputKind::Hlc,
            params: PERIOD_14,
            outputs: 1,
            lookback: LookbackSpec::Period,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "BBANDS",
            aliases: &["BOLL", "BOLLINGER"],
            category: FunctionCategory::Volatility,
            input: InputKind::Series,
            params: BBANDS_PARAMS,
            outputs: 3,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "OBV",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Hlcv,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::None,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "VWAP",
            aliases: &[],
            category: FunctionCategory::Volume,
            input: InputKind::Hlcv,
            params: PERIOD_20,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "REF",
            aliases: &["SHIFT"],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: REF_PARAMS,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "HHV",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "LLV",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::PeriodMinusOne,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "COUNT",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: PERIOD_REQUIRED,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "BARSLAST",
            aliases: &[],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "CROSS",
            aliases: &["CROSSOVER"],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: TWO_SERIES,
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
        FunctionSpec {
            name: "IF",
            aliases: &["IFF"],
            category: FunctionCategory::Formula,
            input: InputKind::Dynamic,
            params: &[],
            outputs: 1,
            lookback: LookbackSpec::Dynamic,
            streaming: true,
            deterministic: true,
        },
    ];

    for spec in specs {
        registry
            .register(spec)
            .expect("built-in function names and aliases are unique");
    }
    registry
}

fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_resolve_case_insensitively() {
        let registry = builtin_function_registry();
        assert_eq!(registry.get("boll").unwrap().name, "BBANDS");
        assert_eq!(registry.get("shift").unwrap().name, "REF");
    }

    #[test]
    fn duplicate_aliases_are_rejected() {
        let mut registry = FunctionRegistry::new();
        registry
            .register(FunctionSpec {
                name: "ONE",
                aliases: &["ALIAS"],
                category: FunctionCategory::Formula,
                input: InputKind::Dynamic,
                params: &[],
                outputs: 1,
                lookback: LookbackSpec::None,
                streaming: true,
                deterministic: true,
            })
            .unwrap();
        let error = registry
            .register(FunctionSpec {
                name: "TWO",
                aliases: &["alias"],
                category: FunctionCategory::Formula,
                input: InputKind::Dynamic,
                params: &[],
                outputs: 1,
                lookback: LookbackSpec::None,
                streaming: true,
                deterministic: true,
            })
            .unwrap_err();
        assert!(error.contains("alias"));
    }

    #[test]
    fn core_formula_primitives_are_discoverable() {
        let registry = builtin_function_registry();
        for name in ["REF", "HHV", "LLV", "COUNT", "BARSLAST", "CROSS", "IF"] {
            assert!(registry.get(name).is_some(), "missing metadata for {name}");
        }
    }
}