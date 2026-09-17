//! Canonical operation contracts shared by indicators, formulas, factors,
//! composites, and drawing adapters.
//!
//! This module is deliberately metadata-only. It is the stable discovery and
//! planning layer; execution remains in the existing indicator/formula/factor
//! engines until each operation has a verified dispatcher and golden vectors.

use crate::registry::{
    builtin_function_registry, FunctionCategory, FunctionSpec, InputKind, LookbackSpec, ParamSpec,
};
use std::collections::BTreeMap;
use std::fmt;

/// Top-level kind of a public operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationKind {
    /// TA-Lib-style or native technical indicator.
    Indicator,
    /// A callable formula-language primitive.
    FormulaFunction,
    /// A reusable one-output or multi-output factor transform.
    Factor,
    /// A composition of operations and/or factors.
    Composite,
    /// A rendering instruction or derived chart series.
    Draw,
}

/// Shape of an operation's value result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValueShape {
    /// One value per row.
    Series,
    /// Multiple aligned series per row, such as MACD or Bollinger Bands.
    MultiSeries,
    /// One value per symbol and timestamp.
    CrossSection,
    /// Discrete events aligned to rows.
    Event,
    /// A non-row result, such as a report or summary.
    Report,
}

/// Explicit execution and semantic capabilities.
///
/// A capability is only set when the underlying implementation has a public
/// contract for it. In particular, `parallel`, `multi_symbol`, and
/// `multi_timeframe` are not inferred from a batch function accepting slices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationCapabilities {
    /// Supports finite batch evaluation.
    pub batch: bool,
    /// Supports incremental/stateful evaluation.
    pub streaming: bool,
    /// Supports a cross-sectional execution context.
    pub cross_sectional: bool,
    /// Accepts multiple symbols as one logical request.
    pub multi_symbol: bool,
    /// Accepts multiple explicit timeframes without implicit resampling.
    pub multi_timeframe: bool,
    /// Can be partitioned by the planner without changing results.
    pub parallel: bool,
    /// Repeated evaluation is deterministic for identical inputs/configuration.
    pub deterministic: bool,
    /// Does not read future rows.
    pub causal: bool,
    /// May intentionally read future rows.
    pub lookahead: bool,
    /// May revise prior values after new input arrives.
    pub repaint: bool,
    /// Requires mutable state between evaluations.
    pub stateful: bool,
    /// Produces or mutates a drawing scene.
    pub drawable: bool,
}

impl OperationCapabilities {
    /// Conservative capabilities for a regular causal indicator.
    pub const fn indicator(streaming: bool, deterministic: bool) -> Self {
        Self {
            batch: true,
            streaming,
            cross_sectional: false,
            multi_symbol: false,
            multi_timeframe: false,
            parallel: false,
            deterministic,
            causal: true,
            lookahead: false,
            repaint: false,
            stateful: streaming,
            drawable: false,
        }
    }
}

/// Stable identifier derived from the canonical normalized operation name.
///
/// The identifier is not a cryptographic hash. The registry checks collisions
/// at registration time, and the canonical name remains the human-readable
/// source of truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(pub u32);

impl OperationId {
    /// Build the stable id for a canonical operation name.
    pub fn from_name(name: &str) -> Self {
        let normalized = normalize_name(name);
        let mut hash = 0x811c_9dc5_u32;
        for byte in normalized.bytes() {
            hash ^= u32::from(byte);
            hash = hash.wrapping_mul(0x0100_0193);
        }
        Self(hash)
    }
}

/// Canonical metadata consumed by planners, bindings, docs, and adapters.
#[derive(Debug, Clone, PartialEq)]
pub struct OperationSpec {
    /// Canonical uppercase name.
    pub name: String,
    /// Accepted aliases, normalized at registration time.
    pub aliases: Vec<String>,
    /// Stable operation kind.
    pub kind: OperationKind,
    /// Result shape.
    pub value_shape: ValueShape,
    /// Legacy discovery category where one exists.
    pub category: Option<FunctionCategory>,
    /// Input contract inherited from the function registry when available.
    pub input: Option<InputKind>,
    /// Parameter declarations.
    pub params: Vec<ParamSpec>,
    /// Number of aligned output series.
    pub outputs: usize,
    /// Warm-up/lookback behavior.
    pub lookback: LookbackSpec,
    /// Execution and semantic capabilities.
    pub capabilities: OperationCapabilities,
    /// Version of this metadata contract.
    pub schema_version: u16,
}

impl OperationSpec {
    /// Project an existing public function description into the canonical
    /// operation contract without claiming capabilities it did not declare.
    pub fn from_function(spec: &FunctionSpec) -> Self {
        let kind = match spec.category {
            FunctionCategory::Formula => OperationKind::FormulaFunction,
            FunctionCategory::Factor => OperationKind::Factor,
            _ => OperationKind::Indicator,
        };
        let value_shape = if spec.outputs > 1 {
            ValueShape::MultiSeries
        } else {
            ValueShape::Series
        };
        Self {
            name: normalize_name(spec.name),
            aliases: spec
                .aliases
                .iter()
                .map(|alias| normalize_name(alias))
                .collect(),
            kind,
            value_shape,
            category: Some(spec.category),
            input: Some(spec.input),
            params: spec.params.to_vec(),
            outputs: spec.outputs,
            lookback: spec.lookback,
            capabilities: OperationCapabilities::indicator(spec.streaming, spec.deterministic),
            schema_version: 1,
        }
    }
}

/// Errors raised while registering a canonical operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationRegistryError {
    /// The canonical name or an alias is empty.
    EmptyName,
    /// The operation name is already registered.
    DuplicateName(String),
    /// An alias conflicts with an operation name or another alias.
    DuplicateAlias(String),
    /// Two canonical names produced the same stable id.
    IdCollision {
        id: OperationId,
        existing: String,
        incoming: String,
    },
}

impl fmt::Display for OperationRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("operation name must not be empty"),
            Self::DuplicateName(name) => write!(formatter, "operation already registered: {name}"),
            Self::DuplicateAlias(alias) => {
                write!(formatter, "operation alias already registered: {alias}")
            }
            Self::IdCollision {
                id,
                existing,
                incoming,
            } => write!(
                formatter,
                "operation id collision {}: {existing} vs {incoming}",
                id.0
            ),
        }
    }
}

impl std::error::Error for OperationRegistryError {}

/// Deterministic registry for the unified public operation catalog.
#[derive(Debug, Clone, Default)]
pub struct OperationRegistry {
    specs: BTreeMap<String, OperationSpec>,
    aliases: BTreeMap<String, String>,
    ids: BTreeMap<OperationId, String>,
}

impl OperationRegistry {
    /// Create an empty operation registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an operation atomically.
    pub fn register(&mut self, mut spec: OperationSpec) -> Result<(), OperationRegistryError> {
        let canonical = normalize_name(&spec.name);
        if canonical.is_empty() {
            return Err(OperationRegistryError::EmptyName);
        }
        if self.specs.contains_key(&canonical) || self.aliases.contains_key(&canonical) {
            return Err(OperationRegistryError::DuplicateName(canonical));
        }

        let id = OperationId::from_name(&canonical);
        if let Some(existing) = self.ids.get(&id) {
            return Err(OperationRegistryError::IdCollision {
                id,
                existing: existing.clone(),
                incoming: canonical,
            });
        }

        let mut normalized_aliases = BTreeMap::new();
        for alias in &spec.aliases {
            let normalized = normalize_name(alias);
            if normalized.is_empty()
                || normalized == canonical
                || self.specs.contains_key(&normalized)
                || self.aliases.contains_key(&normalized)
                || normalized_aliases.insert(normalized.clone(), ()).is_some()
            {
                return Err(OperationRegistryError::DuplicateAlias(normalized));
            }
        }

        spec.name = canonical.clone();
        spec.aliases = normalized_aliases.keys().cloned().collect();
        self.ids.insert(id, canonical.clone());
        for alias in spec.aliases.iter() {
            self.aliases.insert(alias.clone(), canonical.clone());
        }
        self.specs.insert(canonical, spec);
        Ok(())
    }

    /// Register all entries from the existing function metadata registry.
    pub fn from_function_registry(
        functions: &crate::registry::FunctionRegistry,
    ) -> Result<Self, OperationRegistryError> {
        let mut registry = Self::new();
        for function in functions.iter() {
            registry.register(OperationSpec::from_function(function))?;
        }
        Ok(registry)
    }

    /// Resolve a canonical name or alias case-insensitively.
    pub fn get(&self, name: &str) -> Option<&OperationSpec> {
        let normalized = normalize_name(name);
        self.specs.get(&normalized).or_else(|| {
            self.aliases
                .get(&normalized)
                .and_then(|canonical| self.specs.get(canonical))
        })
    }

    /// Resolve a stable id.
    pub fn get_by_id(&self, id: OperationId) -> Option<&OperationSpec> {
        self.ids.get(&id).and_then(|name| self.specs.get(name))
    }

    /// Iterate in stable canonical-name order.
    pub fn iter(&self) -> impl Iterator<Item = &OperationSpec> {
        self.specs.values()
    }

    /// Number of canonical operations.
    pub fn len(&self) -> usize {
        self.specs.len()
    }

    /// Whether no operations are registered.
    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }
}

/// Build the canonical catalog from all currently registered public
/// functions. Formula-language, factor, composite, and drawing entries will
/// be added through the same registry as their verified dispatchers land.
pub fn builtin_operation_registry() -> OperationRegistry {
    OperationRegistry::from_function_registry(&builtin_function_registry())
        .expect("built-in operation names and aliases are unique")
}

fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_project_without_losing_function_contracts() {
        let functions = builtin_function_registry();
        let operations = OperationRegistry::from_function_registry(&functions).unwrap();

        assert_eq!(operations.len(), functions.len());
        let ema = operations.get(" ema ").unwrap();
        assert_eq!(ema.kind, OperationKind::Indicator);
        assert_eq!(ema.value_shape, ValueShape::Series);
        assert!(ema.capabilities.batch);
        assert!(ema.capabilities.streaming);
        assert!(ema.capabilities.causal);
        assert!(!ema.capabilities.lookahead);
    }

    #[test]
    fn ids_are_case_insensitive_and_resolvable() {
        let operations = builtin_operation_registry();
        let ema = operations.get("EMA").unwrap();
        let id = OperationId::from_name("ema");

        assert_eq!(operations.get_by_id(id), Some(ema));
        assert_eq!(id, OperationId::from_name(" EMA "));
    }

    #[test]
    fn registration_rejects_alias_conflicts_atomically() {
        let mut operations = OperationRegistry::new();
        let first = OperationSpec {
            name: "FIRST".to_string(),
            aliases: vec!["one".to_string()],
            kind: OperationKind::Indicator,
            value_shape: ValueShape::Series,
            category: None,
            input: None,
            params: Vec::new(),
            outputs: 1,
            lookback: LookbackSpec::None,
            capabilities: OperationCapabilities::indicator(false, true),
            schema_version: 1,
        };
        operations.register(first).unwrap();

        let conflicting = OperationSpec {
            name: "SECOND".to_string(),
            aliases: vec!["ONE".to_string(), "two".to_string()],
            kind: OperationKind::Composite,
            value_shape: ValueShape::MultiSeries,
            category: None,
            input: None,
            params: Vec::new(),
            outputs: 2,
            lookback: LookbackSpec::Dynamic,
            capabilities: OperationCapabilities::indicator(false, true),
            schema_version: 1,
        };

        assert!(matches!(
            operations.register(conflicting),
            Err(OperationRegistryError::DuplicateAlias(_))
        ));
        assert_eq!(operations.len(), 1);
        assert!(operations.get("SECOND").is_none());
        assert!(operations.get("TWO").is_none());
    }
}
