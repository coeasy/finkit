//! First-class factor catalog and compiled factor runtime.
//!
//! This module adds stable factor metadata, aliases, version identity and a
//! precompiled execution facade without changing the legacy `FactorDefinition`
//! shape. Numerical work remains owned by `factors`, dependency planning by
//! `compute::FactorPlan`, and execution by the canonical unified runtime.

use crate::compute::FactorPlan;
use crate::factors::{
    BorrowedFactorContext, FactorContext, FactorDefinition, FactorEngine, FactorError, FactorKind,
    FactorRegistry, FactorResult,
};
use crate::unified_runtime::{DirtyRange, RuntimeExecution, RuntimeExecutionTrace, UnifiedRuntime};
use std::collections::{BTreeMap, BTreeSet};

/// Stable metadata attached to a registered factor independently of its
/// computation closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactorMetadata {
    /// Stable semantic version for cache/provenance identity.
    pub version: String,
    /// Human-readable description.
    pub description: String,
    /// Alternate public names resolved by the catalog.
    pub aliases: Vec<String>,
    /// Whether repeated execution over equal inputs is deterministic.
    pub deterministic: bool,
    /// Whether a stateful streaming implementation is available.
    pub streaming: bool,
    /// Whether this factor explicitly supports incremental/range execution.
    ///
    /// Range execution is enabled only when this is true, the factor is a
    /// time-series factor, and `fixed_lookback` is known for every planned
    /// factor node.
    pub incremental: bool,
    /// Fixed number of historical rows required by this factor node.
    /// `None` means dynamic/unknown and therefore prevents safe range execution.
    pub fixed_lookback: Option<usize>,
}

impl Default for FactorMetadata {
    fn default() -> Self {
        Self {
            version: "1".to_string(),
            description: String::new(),
            aliases: Vec::new(),
            deterministic: true,
            streaming: false,
            incremental: false,
            fixed_lookback: None,
        }
    }
}

/// Metadata view returned by the factor catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactorDescriptor {
    /// Canonical factor name.
    pub name: String,
    /// Time-series or cross-sectional execution intent.
    pub kind: FactorKind,
    /// Named canonical dependencies.
    pub dependencies: Vec<String>,
    /// Sidecar metadata used by planning, documentation and provenance.
    pub metadata: FactorMetadata,
}

/// Canonical discovery point for factor definitions and their metadata.
///
/// The catalog intentionally stores metadata beside the existing
/// `FactorRegistry` rather than adding fields to `FactorDefinition`, preserving
/// source compatibility for callers that construct legacy definitions.
#[derive(Clone, Default)]
pub struct FactorCatalog {
    registry: FactorRegistry,
    metadata: BTreeMap<String, FactorMetadata>,
    aliases: BTreeMap<String, String>,
}

impl FactorCatalog {
    /// Create an empty catalog.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a catalog around an existing registry using conservative metadata.
    ///
    /// Legacy definitions remain full-recompute by default until callers add
    /// explicit incremental capability metadata.
    #[must_use]
    pub fn from_registry(registry: FactorRegistry) -> Self {
        let metadata = registry
            .names()
            .map(|name| (name.to_string(), FactorMetadata::default()))
            .collect();
        Self {
            registry,
            metadata,
            aliases: BTreeMap::new(),
        }
    }

    /// Register a canonical factor and its stable metadata.
    pub fn register(
        &mut self,
        definition: FactorDefinition,
        metadata: FactorMetadata,
    ) -> FactorResult<()> {
        let name = definition.name.clone();
        if metadata.version.trim().is_empty() {
            return Err(FactorError::InvalidParameter(format!(
                "factor version must not be empty: {name}"
            )));
        }
        let mut seen = BTreeSet::new();
        for alias in &metadata.aliases {
            let alias = alias.trim();
            if alias.is_empty() {
                return Err(FactorError::InvalidParameter(format!(
                    "factor alias must not be empty: {name}"
                )));
            }
            if alias == name || !seen.insert(alias.to_string()) {
                return Err(FactorError::InvalidParameter(format!(
                    "factor alias must be unique and differ from its canonical name: {alias}"
                )));
            }
            if self.registry.get(alias).is_some() || self.aliases.contains_key(alias) {
                return Err(FactorError::DuplicateFactor(alias.to_string()));
            }
        }
        if self.aliases.contains_key(&name) {
            return Err(FactorError::DuplicateFactor(name));
        }

        self.registry.register(definition)?;
        for alias in &metadata.aliases {
            self.aliases.insert(alias.clone(), name.clone());
        }
        self.metadata.insert(name, metadata);
        Ok(())
    }

    /// Access the canonical computation registry.
    #[must_use]
    pub fn registry(&self) -> &FactorRegistry {
        &self.registry
    }

    /// Consume the catalog and return its registry.
    #[must_use]
    pub fn into_registry(self) -> FactorRegistry {
        self.registry
    }

    /// Resolve a canonical name or alias to the canonical factor name.
    #[must_use]
    pub fn resolve_name<'a>(&'a self, name: &'a str) -> Option<&'a str> {
        if self.registry.get(name).is_some() {
            Some(name)
        } else {
            self.aliases.get(name).map(String::as_str)
        }
    }

    /// Return a complete descriptor for a canonical name or alias.
    #[must_use]
    pub fn descriptor(&self, name: &str) -> Option<FactorDescriptor> {
        let canonical = self.resolve_name(name)?;
        let definition = self.registry.get(canonical)?;
        Some(FactorDescriptor {
            name: canonical.to_string(),
            kind: definition.kind,
            dependencies: definition.dependencies.clone(),
            metadata: self.metadata.get(canonical).cloned().unwrap_or_default(),
        })
    }

    /// Compile one or more canonical/alias targets into a reusable execution
    /// plan. Shared dependencies occur only once in the underlying `FactorPlan`.
    pub fn compile(&self, targets: &[&str]) -> FactorResult<CompiledFactorPlan> {
        let mut canonical = Vec::with_capacity(targets.len());
        let mut seen = BTreeSet::new();
        for target in targets {
            let resolved = self
                .resolve_name(target)
                .ok_or_else(|| FactorError::UnknownFactor((*target).to_string()))?;
            if seen.insert(resolved.to_string()) {
                canonical.push(resolved.to_string());
            }
        }
        let refs: Vec<&str> = canonical.iter().map(String::as_str).collect();
        let plan = FactorPlan::compile(&self.registry, &refs)?;
        let identity = plan
            .execution_order()
            .iter()
            .map(|name| {
                let version = self
                    .metadata
                    .get(name)
                    .map_or("1", |metadata| metadata.version.as_str());
                format!("{name}@{version}")
            })
            .collect();

        let range_lookback = self.range_lookback_for_plan(&plan);
        Ok(CompiledFactorPlan {
            plan,
            targets: canonical,
            semantic_identity: identity,
            range_lookback,
        })
    }

    /// Prove range safety and calculate the full dependency-chain lookback.
    ///
    /// Each node's own lookback is added to the maximum cumulative lookback of
    /// computed factor dependencies. Raw inputs contribute zero. Returning
    /// `None` is a mandatory full-recompute fallback, never a guessed window.
    fn range_lookback_for_plan(&self, plan: &FactorPlan) -> Option<usize> {
        let mut cumulative = BTreeMap::<String, usize>::new();
        let mut plan_max = 0usize;

        for name in plan.execution_order() {
            let definition = self.registry.get(name)?;
            let metadata = self.metadata.get(name).cloned().unwrap_or_default();
            if !metadata.incremental || definition.kind != FactorKind::TimeSeries {
                return None;
            }
            let own_lookback = metadata.fixed_lookback?;
            let upstream = definition
                .dependencies
                .iter()
                .filter_map(|dependency| cumulative.get(dependency).copied())
                .max()
                .unwrap_or(0);
            let total = upstream.saturating_add(own_lookback);
            cumulative.insert(name.clone(), total);
            plan_max = plan_max.max(total);
        }
        Some(plan_max)
    }
}

/// Reusable compiled factor execution plan with stable semantic identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledFactorPlan {
    plan: FactorPlan,
    targets: Vec<String>,
    semantic_identity: Vec<String>,
    /// `Some` is a proof that every factor node can safely execute over a
    /// bounded time-series slice. The value is the dependency-chain lookback.
    range_lookback: Option<usize>,
}

impl CompiledFactorPlan {
    /// Canonical requested output names.
    #[must_use]
    pub fn targets(&self) -> &[String] {
        &self.targets
    }

    /// Dependency-first canonical execution order.
    #[must_use]
    pub fn execution_order(&self) -> &[String] {
        self.plan.execution_order()
    }

    /// Raw input names required by the plan.
    #[must_use]
    pub fn required_raw_inputs(&self) -> &[String] {
        self.plan.required_raw_inputs()
    }

    /// Versioned identities of all executed factor nodes. This is suitable as
    /// one component of a cache/provenance key.
    #[must_use]
    pub fn semantic_identity(&self) -> &[String] {
        &self.semantic_identity
    }

    /// Whether the complete factor DAG has an explicit safe range contract.
    #[must_use]
    pub const fn supports_range_incremental(&self) -> bool {
        self.range_lookback.is_some()
    }

    /// Historical rows required before the dirty interval for safe execution.
    #[must_use]
    pub const fn range_lookback(&self) -> Option<usize> {
        self.range_lookback
    }

    /// Execute over owned aligned inputs through the unified runtime.
    pub fn execute(
        &self,
        engine: &FactorEngine,
        context: &FactorContext,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        Ok(self.execute_runtime(engine, context)?.output)
    }

    /// Execute over borrowed aligned inputs without copying raw input arrays.
    pub fn execute_borrowed(
        &self,
        engine: &FactorEngine,
        context: &BorrowedFactorContext<'_>,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        Ok(self.execute_runtime_borrowed(engine, context)?.output)
    }

    /// Execute through the unified runtime and retain runtime evidence.
    pub fn execute_runtime(
        &self,
        engine: &FactorEngine,
        context: &FactorContext,
    ) -> FactorResult<RuntimeExecution<BTreeMap<String, Vec<f64>>>> {
        UnifiedRuntime::execute_factor_plan(&self.plan, engine, context)
    }

    /// Zero-copy borrowed execution through the unified runtime.
    pub fn execute_runtime_borrowed(
        &self,
        engine: &FactorEngine,
        context: &BorrowedFactorContext<'_>,
    ) -> FactorResult<RuntimeExecution<BTreeMap<String, Vec<f64>>>> {
        UnifiedRuntime::execute_factor_plan_borrowed(&self.plan, engine, context)
    }

    /// Recompute only the dirty region when every node has proved range safety.
    pub fn execute_range_borrowed(
        &self,
        engine: &FactorEngine,
        context: &BorrowedFactorContext<'_>,
        previous: &BTreeMap<String, Vec<f64>>,
        dirty: DirtyRange,
    ) -> FactorResult<RuntimeExecution<BTreeMap<String, Vec<f64>>>> {
        let lookback = self.require_range_lookback()?;
        UnifiedRuntime::execute_factor_plan_range_borrowed(
            &self.plan, engine, context, previous, dirty, lookback,
        )
    }

    /// In-place form of dirty-range execution. Clean rows and their allocations
    /// are retained unchanged.
    pub fn execute_range_into_borrowed(
        &self,
        engine: &FactorEngine,
        context: &BorrowedFactorContext<'_>,
        output: &mut BTreeMap<String, Vec<f64>>,
        dirty: DirtyRange,
    ) -> FactorResult<RuntimeExecutionTrace> {
        let lookback = self.require_range_lookback()?;
        UnifiedRuntime::execute_factor_plan_range_into_borrowed(
            &self.plan, engine, context, output, dirty, lookback,
        )
    }

    fn require_range_lookback(&self) -> FactorResult<usize> {
        self.range_lookback.ok_or_else(|| {
            FactorError::InvalidParameter(
                "factor plan is not range-safe: every node must be incremental, time-series, and have a fixed lookback"
                    .to_string(),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factors::{FactorDirection, FactorInputs};
    use crate::unified_runtime::RuntimeExecutionMode;
    use std::sync::Arc;

    fn identity(name: &str, dependency: &str) -> FactorDefinition {
        let dependency_owned = dependency.to_string();
        FactorDefinition::new(
            name,
            [dependency],
            FactorKind::TimeSeries,
            FactorDirection::HigherBetter,
            Arc::new(move |inputs: &FactorInputs<'_>| Ok(inputs.get(&dependency_owned)?.to_vec())),
        )
    }

    fn range_metadata(version: &str, lookback: usize) -> FactorMetadata {
        FactorMetadata {
            version: version.to_string(),
            incremental: true,
            fixed_lookback: Some(lookback),
            ..FactorMetadata::default()
        }
    }

    #[test]
    fn catalog_resolves_aliases_and_preserves_versions() {
        let mut catalog = FactorCatalog::new();
        catalog
            .register(
                identity("momentum", "close"),
                FactorMetadata {
                    version: "2".to_string(),
                    aliases: vec!["mom".to_string()],
                    incremental: true,
                    fixed_lookback: Some(0),
                    ..FactorMetadata::default()
                },
            )
            .unwrap();

        assert_eq!(catalog.resolve_name("mom"), Some("momentum"));
        assert_eq!(catalog.descriptor("mom").unwrap().metadata.version, "2");
        let plan = catalog.compile(&["mom", "momentum"]).unwrap();
        assert_eq!(plan.targets(), &["momentum".to_string()]);
        assert_eq!(plan.semantic_identity(), &["momentum@2".to_string()]);
        assert!(plan.supports_range_incremental());
    }

    #[test]
    fn compiled_plan_reuses_dependency_order_and_borrowed_inputs() {
        let mut catalog = FactorCatalog::new();
        catalog
            .register(identity("base", "close"), FactorMetadata::default())
            .unwrap();
        catalog
            .register(identity("score", "base"), FactorMetadata::default())
            .unwrap();
        let plan = catalog.compile(&["score"]).unwrap();
        assert_eq!(plan.execution_order(), &["base", "score"]);
        assert!(!plan.supports_range_incremental());

        let close = [1.0, 2.0, 3.0];
        let context = BorrowedFactorContext::new()
            .with_series("close", &close)
            .unwrap();
        let engine = FactorEngine::new(catalog.into_registry());
        let result = plan.execute_borrowed(&engine, &context).unwrap();
        assert_eq!(result["score"], close);
    }

    #[test]
    fn range_plan_accumulates_dependency_lookback_and_splices_dirty_rows() {
        let mut catalog = FactorCatalog::new();
        catalog
            .register(identity("base", "close"), range_metadata("1", 2))
            .unwrap();
        catalog
            .register(identity("score", "base"), range_metadata("1", 3))
            .unwrap();
        let plan = catalog.compile(&["score"]).unwrap();
        assert_eq!(plan.range_lookback(), Some(5));

        let original = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let initial = BorrowedFactorContext::new()
            .with_series("close", &original)
            .unwrap();
        let engine = FactorEngine::new(catalog.clone().into_registry());
        let full = plan.execute_runtime_borrowed(&engine, &initial).unwrap();

        let changed = [1.0, 2.0, 3.0, 4.0, 5.0, 60.0, 7.0, 8.0];
        let revised = BorrowedFactorContext::new()
            .with_series("close", &changed)
            .unwrap();
        let ranged = plan
            .execute_range_borrowed(&engine, &revised, &full.output, DirtyRange::new(5, 6))
            .unwrap();

        assert_eq!(ranged.output["score"], changed);
        assert_eq!(
            ranged.trace.mode,
            RuntimeExecutionMode::Range {
                dirty: DirtyRange::new(5, 6),
                recompute: DirtyRange::new(0, 6),
            }
        );
        assert_eq!(ranged.trace.recomputed_rows, 6);
    }

    #[test]
    fn cross_sectional_or_unbounded_nodes_force_full_fallback() {
        let mut catalog = FactorCatalog::new();
        let cross = FactorDefinition::new(
            "cross",
            ["close"],
            FactorKind::CrossSectional,
            FactorDirection::HigherBetter,
            Arc::new(|inputs| Ok(inputs.get("close")?.to_vec())),
        );
        catalog.register(cross, range_metadata("1", 0)).unwrap();
        let plan = catalog.compile(&["cross"]).unwrap();
        assert!(!plan.supports_range_incremental());

        let close = [1.0, 2.0];
        let context = BorrowedFactorContext::new()
            .with_series("close", &close)
            .unwrap();
        let engine = FactorEngine::new(catalog.into_registry());
        let full = plan.execute_borrowed(&engine, &context).unwrap();
        let error = plan
            .execute_range_borrowed(&engine, &context, &full, DirtyRange::new(1, 2))
            .unwrap_err();
        assert!(matches!(error, FactorError::InvalidParameter(_)));
    }
}
