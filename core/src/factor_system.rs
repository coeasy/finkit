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
use std::collections::{BTreeMap, BTreeSet, VecDeque};

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
            .map(|name| {
                (
                    name.to_string(),
                    builtin_factor_metadata(name).unwrap_or_default(),
                )
            })
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

/// Metadata for the portable built-in factors. The computation closures live
/// in `factors`; this table is the single planning-side declaration of their
/// finite lookback contracts.
fn builtin_factor_metadata(name: &str) -> Option<FactorMetadata> {
    if let Some(period) = name
        .strip_prefix("momentum_")
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|period| *period > 0)
    {
        return Some(FactorMetadata {
            version: "1".to_string(),
            description: format!("Arithmetic return over {period} bars"),
            streaming: true,
            incremental: true,
            fixed_lookback: Some(period),
            ..FactorMetadata::default()
        });
    }
    match name {
        "volatility_20" => Some(FactorMetadata {
            version: "1".to_string(),
            description: "Rolling population volatility of one-bar returns".to_string(),
            streaming: true,
            incremental: true,
            fixed_lookback: Some(20),
            ..FactorMetadata::default()
        }),
        "reversal_5" => Some(FactorMetadata {
            version: "1".to_string(),
            description: "Sign-inverted five-bar arithmetic return".to_string(),
            streaming: true,
            incremental: true,
            fixed_lookback: Some(0),
            ..FactorMetadata::default()
        }),
        _ => None,
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

    /// Create a bounded-window append-only stream for this plan.
    ///
    /// The stream retains only the proven lookback window of raw inputs and
    /// materialized outputs, so each append can reuse the unified range
    /// executor without replaying the full history. This is a production-safe
    /// streaming path for finite-lookback plans; recursive or whole-series
    /// plans are rejected and must use a stateful kernel implementation or
    /// full execution instead.
    pub fn stream(&self, engine: FactorEngine) -> FactorResult<FactorStream> {
        let _ = self.require_range_lookback()?;
        Ok(FactorStream {
            plan: self.clone(),
            engine,
            row_count: 0,
            inputs: BTreeMap::new(),
            output: BTreeMap::new(),
        })
    }

    /// Create an O(1)-per-row stateful stream for supported built-in factors.
    ///
    /// Custom closure factors are rejected because their state cannot be
    /// inferred or serialized safely from the legacy batch callback. They must
    /// register an explicit state kernel before using this path.
    pub fn stateful_stream(&self) -> FactorResult<StatefulFactorStream> {
        StatefulFactorStream::from_plan(self)
    }
}

/// Persistable checkpoint for [`FactorStream`].
#[derive(Debug, Clone, PartialEq)]
pub struct FactorStreamCheckpoint {
    semantic_identity: Vec<String>,
    row_count: usize,
    inputs: BTreeMap<String, Vec<f64>>,
    output: BTreeMap<String, Vec<f64>>,
}

impl FactorStreamCheckpoint {
    /// Semantic identities of the factor nodes in this checkpoint.
    #[must_use]
    pub fn semantic_identity(&self) -> &[String] {
        &self.semantic_identity
    }

    /// Number of rows represented by this checkpoint.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.row_count
    }

    /// Retained raw input window.
    #[must_use]
    pub fn inputs(&self) -> &BTreeMap<String, Vec<f64>> {
        &self.inputs
    }

    /// Retained materialized output window.
    #[must_use]
    pub fn outputs(&self) -> &BTreeMap<String, Vec<f64>> {
        &self.output
    }

    /// Construct a checkpoint payload for a compiled plan.
    ///
    /// The stream validates the semantic identity and buffer shape when the
    /// checkpoint is restored. This constructor is intended for language
    /// bindings that carry the checkpoint over a wire format.
    #[must_use]
    pub fn from_parts(
        semantic_identity: Vec<String>,
        row_count: usize,
        inputs: BTreeMap<String, Vec<f64>>,
        output: BTreeMap<String, Vec<f64>>,
    ) -> Self {
        Self {
            semantic_identity,
            row_count,
            inputs,
            output,
        }
    }
}

/// Append-only bounded-window factor executor with checkpoint/restore.
///
/// This type is deliberately backed by the same compiled plan and range
/// executor as batch and historical-edit execution. It therefore shares the
/// exact numeric semantics while retaining only `lookback + 1` rows. The
/// bounded buffer is also the portable checkpoint format; callers that need
/// O(1) state for recursive indicators must register a dedicated stateful
/// kernel.
#[derive(Clone)]
pub struct FactorStream {
    plan: CompiledFactorPlan,
    engine: FactorEngine,
    row_count: usize,
    inputs: BTreeMap<String, Vec<f64>>,
    output: BTreeMap<String, Vec<f64>>,
}

impl FactorStream {
    /// Compiled plan used by this stream.
    #[must_use]
    pub fn plan(&self) -> &CompiledFactorPlan {
        &self.plan
    }

    /// Number of rows accepted by the stream.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.row_count
    }

    /// Return the retained output series for all planned factor nodes.
    #[must_use]
    pub fn outputs(&self) -> &BTreeMap<String, Vec<f64>> {
        &self.output
    }

    /// Append one aligned row and return the newly computed target values.
    pub fn push_row(
        &mut self,
        values: &BTreeMap<String, f64>,
    ) -> FactorResult<BTreeMap<String, f64>> {
        let ordered = self
            .plan
            .required_raw_inputs()
            .iter()
            .map(|input| {
                values
                    .get(input)
                    .copied()
                    .ok_or_else(|| FactorError::MissingInput(input.clone()))
            })
            .collect::<FactorResult<Vec<_>>>()?;
        self.push_values(&ordered)
    }

    /// Append one row in the plan's sorted raw-input order.
    ///
    /// This avoids constructing a string-keyed map in hot loops. The order is
    /// exactly [`CompiledFactorPlan::required_raw_inputs`].
    pub fn push_values(&mut self, values: &[f64]) -> FactorResult<BTreeMap<String, f64>> {
        let required = self.plan.required_raw_inputs();
        if values.len() != required.len() {
            return Err(FactorError::LengthMismatch {
                name: "factor_stream_row".to_string(),
                expected: required.len(),
                actual: values.len(),
            });
        }

        let row = self.inputs.values().next().map_or(0, Vec::len);
        for (input, value) in required.iter().zip(values.iter().copied()) {
            self.inputs.entry(input.clone()).or_default().push(value);
        }
        for name in self.plan.execution_order() {
            self.output.entry(name.clone()).or_default().push(f64::NAN);
        }

        let result = (|| {
            let context = borrowed_context(&self.inputs)?;
            self.plan.execute_range_into_borrowed(
                &self.engine,
                &context,
                &mut self.output,
                DirtyRange::new(row, row + 1),
            )?;
            Ok(self
                .plan
                .targets()
                .iter()
                .map(|name| {
                    (
                        name.clone(),
                        self.output
                            .get(name)
                            .and_then(|series| series.last().copied())
                            .unwrap_or(f64::NAN),
                    )
                })
                .collect())
        })();

        if result.is_ok() {
            self.row_count = self.row_count.saturating_add(1);
            let capacity = self.plan.range_lookback().unwrap_or(0).saturating_add(1);
            if self
                .inputs
                .values()
                .next()
                .is_some_and(|series| series.len() > capacity)
            {
                for input in self.plan.required_raw_inputs() {
                    if let Some(series) = self.inputs.get_mut(input) {
                        series.remove(0);
                    }
                }
                for name in self.plan.execution_order() {
                    if let Some(series) = self.output.get_mut(name) {
                        series.remove(0);
                    }
                }
            }
        } else {
            for input in self.plan.required_raw_inputs() {
                if let Some(series) = self.inputs.get_mut(input) {
                    series.pop();
                }
            }
            for name in self.plan.execution_order() {
                if let Some(series) = self.output.get_mut(name) {
                    series.pop();
                }
            }
        }
        result
    }

    /// Append an aligned batch and execute one range evaluation for the batch.
    ///
    /// This is the preferred ingestion path for adapters and data feeds. The
    /// input map may contain extra columns, but every raw input required by the
    /// plan must be present and all required columns must have equal lengths.
    pub fn push_batch(
        &mut self,
        values: &BTreeMap<String, Vec<f64>>,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        let mut emitted = BTreeMap::new();
        self.push_batch_into(values, &mut emitted)?;
        Ok(emitted)
    }

    /// Append an aligned batch and write target values into reusable buffers.
    ///
    /// The caller owns `emitted` and may keep it across calls. Existing vector
    /// capacity is reused, so steady-state ingestion does not allocate a new
    /// result vector for every batch. On validation or execution failure the
    /// output buffers are left unchanged.
    pub fn push_batch_into(
        &mut self,
        values: &BTreeMap<String, Vec<f64>>,
        emitted: &mut BTreeMap<String, Vec<f64>>,
    ) -> FactorResult<()> {
        let required = self.plan.required_raw_inputs();
        let rows = required
            .first()
            .and_then(|name| values.get(name))
            .ok_or_else(|| {
                FactorError::MissingInput(
                    required
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "factor_stream_input".to_string()),
                )
            })?
            .len();
        for input in required {
            let series = values
                .get(input)
                .ok_or_else(|| FactorError::MissingInput(input.clone()))?;
            if series.len() != rows {
                return Err(FactorError::LengthMismatch {
                    name: input.clone(),
                    expected: rows,
                    actual: series.len(),
                });
            }
        }
        if rows == 0 {
            for name in self.plan.targets() {
                emitted.entry(name.clone()).or_default().clear();
            }
            return Ok(());
        }

        let previous_rows = self.inputs.values().next().map_or(0, Vec::len);
        for input in required {
            self.inputs
                .entry(input.clone())
                .or_default()
                .extend_from_slice(&values[input]);
        }
        let total_rows = previous_rows + rows;
        for name in self.plan.execution_order() {
            self.output
                .entry(name.clone())
                .or_default()
                .resize(total_rows, f64::NAN);
        }

        let result = (|| {
            let context = borrowed_context(&self.inputs)?;
            self.plan.execute_range_into_borrowed(
                &self.engine,
                &context,
                &mut self.output,
                DirtyRange::new(previous_rows, total_rows),
            )?;
            Ok(())
        })();

        if result.is_ok() {
            for name in self.plan.targets() {
                let target = emitted.entry(name.clone()).or_default();
                target.clear();
                if let Some(series) = self.output.get(name) {
                    target.extend_from_slice(&series[previous_rows..total_rows]);
                }
            }
            self.row_count = self.row_count.saturating_add(rows);
            let capacity = self.plan.range_lookback().unwrap_or(0).saturating_add(1);
            let retained = self.inputs.values().next().map_or(0, Vec::len);
            let excess = retained.saturating_sub(capacity);
            if excess > 0 {
                for input in required {
                    if let Some(series) = self.inputs.get_mut(input) {
                        series.drain(..excess);
                    }
                }
                for name in self.plan.execution_order() {
                    if let Some(series) = self.output.get_mut(name) {
                        series.drain(..excess);
                    }
                }
            }
        } else {
            for input in required {
                if let Some(series) = self.inputs.get_mut(input) {
                    series.truncate(previous_rows);
                }
            }
            for name in self.plan.execution_order() {
                if let Some(series) = self.output.get_mut(name) {
                    series.truncate(previous_rows);
                }
            }
        }
        result
    }

    /// Capture a portable checkpoint at the current append boundary.
    #[must_use]
    pub fn checkpoint(&self) -> FactorStreamCheckpoint {
        FactorStreamCheckpoint {
            semantic_identity: self.plan.semantic_identity.clone(),
            row_count: self.row_count,
            inputs: self.inputs.clone(),
            output: self.output.clone(),
        }
    }

    /// Restore a checkpoint produced by the same semantic plan.
    pub fn restore(&mut self, checkpoint: &FactorStreamCheckpoint) -> FactorResult<()> {
        if checkpoint.semantic_identity != self.plan.semantic_identity {
            return Err(FactorError::InvalidParameter(
                "factor stream checkpoint belongs to a different semantic plan".to_string(),
            ));
        }
        validate_stream_checkpoint(
            &self.plan,
            checkpoint.row_count,
            &checkpoint.inputs,
            &checkpoint.output,
        )?;
        self.row_count = checkpoint.row_count;
        self.inputs = checkpoint.inputs.clone();
        self.output = checkpoint.output.clone();
        Ok(())
    }
}

/// O(1)-per-row stateful executor for the portable built-in Factor DAG.
#[derive(Clone)]
pub struct StatefulFactorStream {
    semantic_identity: Vec<String>,
    targets: Vec<(String, usize)>,
    nodes: Vec<StatefulFactorNode>,
    values: Vec<f64>,
    output_scratch: Vec<f64>,
    row_count: usize,
}

/// Persistable state image for [`StatefulFactorStream`].
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StatefulFactorCheckpoint {
    semantic_identity: Vec<String>,
    row_count: usize,
    nodes: Vec<StatefulFactorNode>,
}

impl StatefulFactorCheckpoint {
    /// Stable factor identities associated with this state image.
    #[must_use]
    pub fn semantic_identity(&self) -> &[String] {
        &self.semantic_identity
    }

    /// Number of rows already consumed by the state image.
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.row_count
    }

    /// Serialize this state image for a language adapter when serde support is
    /// enabled.
    #[cfg(feature = "serde")]
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|error| error.to_string())
    }

    /// Restore a state image received from a language adapter.
    #[cfg(feature = "serde")]
    pub fn from_json(value: &str) -> Result<Self, String> {
        serde_json::from_str(value).map_err(|error| error.to_string())
    }
}

impl StatefulFactorStream {
    fn from_plan(plan: &CompiledFactorPlan) -> FactorResult<Self> {
        if plan.required_raw_inputs() != ["close".to_string()] {
            return Err(FactorError::InvalidParameter(
                "stateful built-in Factor streaming currently requires the close input only"
                    .to_string(),
            ));
        }
        let mut nodes = Vec::with_capacity(plan.execution_order().len());
        let mut indexes = BTreeMap::new();
        for name in plan.execution_order() {
            let node = if let Some(period) = name
                .strip_prefix("momentum_")
                .and_then(|value| value.parse::<usize>().ok())
                .filter(|period| *period > 0)
            {
                StatefulFactorNode::Momentum(LaggedReturnState::new(period))
            } else if name == "volatility_20" {
                StatefulFactorNode::Volatility(VolatilityState::new(20))
            } else if name == "reversal_5" {
                let dependency = indexes.get("momentum_5").copied().ok_or_else(|| {
                    FactorError::InvalidParameter(
                        "stateful reversal_5 requires momentum_5 in the compiled plan".to_string(),
                    )
                })?;
                StatefulFactorNode::Negate { dependency }
            } else {
                return Err(FactorError::InvalidParameter(format!(
                    "stateful Factor function is unsupported: {name}"
                )));
            };
            indexes.insert(name.clone(), nodes.len());
            nodes.push(node);
        }
        let targets = plan
            .targets()
            .iter()
            .map(|name| {
                indexes
                    .get(name)
                    .copied()
                    .map(|index| (name.clone(), index))
                    .ok_or_else(|| FactorError::UnknownFactor(name.clone()))
            })
            .collect::<FactorResult<Vec<_>>>()?;
        Ok(Self {
            semantic_identity: plan.semantic_identity.clone(),
            targets,
            values: vec![f64::NAN; nodes.len()],
            output_scratch: vec![f64::NAN; plan.targets().len()],
            nodes,
            row_count: 0,
        })
    }

    /// Stable identities of all stateful factor nodes.
    #[must_use]
    pub fn semantic_identity(&self) -> &[String] {
        &self.semantic_identity
    }

    /// Requested output names in deterministic order.
    #[must_use]
    pub fn targets(&self) -> impl Iterator<Item = &str> {
        self.targets.iter().map(|(name, _)| name.as_str())
    }

    /// Number of rows consumed by this executor.
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.row_count
    }

    /// Advance one row in the plan's raw-input order and write target values.
    pub fn push_values_into(&mut self, values: &[f64], outputs: &mut [f64]) -> FactorResult<()> {
        if values.len() != 1 {
            return Err(FactorError::LengthMismatch {
                name: "stateful_factor_row".to_string(),
                expected: 1,
                actual: values.len(),
            });
        }
        if outputs.len() != self.output_scratch.len() {
            return Err(FactorError::LengthMismatch {
                name: "stateful_factor_output".to_string(),
                expected: self.output_scratch.len(),
                actual: outputs.len(),
            });
        }
        self.advance(values[0]);
        outputs.copy_from_slice(&self.output_scratch);
        Ok(())
    }

    /// Append a close batch into reusable output vectors.
    pub fn push_batch_into(
        &mut self,
        values: &BTreeMap<String, Vec<f64>>,
        emitted: &mut BTreeMap<String, Vec<f64>>,
    ) -> FactorResult<()> {
        let close = values
            .get("close")
            .ok_or_else(|| FactorError::MissingInput("close".to_string()))?;
        for name in self.targets() {
            let output = emitted.entry(name.to_string()).or_default();
            output.clear();
            output.reserve(close.len());
        }
        for value in close {
            self.advance(*value);
            for (index, (name, _)) in self.targets.iter().enumerate() {
                emitted
                    .get_mut(name)
                    .expect("stateful factor output initialized")
                    .push(self.output_scratch[index]);
            }
        }
        Ok(())
    }

    /// Capture a state-only checkpoint without retaining input history.
    #[must_use]
    pub fn checkpoint(&self) -> StatefulFactorCheckpoint {
        StatefulFactorCheckpoint {
            semantic_identity: self.semantic_identity.clone(),
            row_count: self.row_count,
            nodes: self.nodes.clone(),
        }
    }

    /// Restore a checkpoint created by the same compiled Factor plan.
    pub fn restore(&mut self, checkpoint: &StatefulFactorCheckpoint) -> FactorResult<()> {
        if checkpoint.semantic_identity != self.semantic_identity {
            return Err(FactorError::InvalidParameter(
                "stateful factor checkpoint belongs to a different semantic plan".to_string(),
            ));
        }
        if checkpoint.nodes.len() != self.nodes.len() {
            return Err(FactorError::InvalidParameter(
                "stateful factor checkpoint node layout does not match plan".to_string(),
            ));
        }
        self.nodes.clone_from(&checkpoint.nodes);
        self.values.fill(f64::NAN);
        self.output_scratch.fill(f64::NAN);
        self.row_count = checkpoint.row_count;
        Ok(())
    }

    fn advance(&mut self, close: f64) {
        for (index, node) in self.nodes.iter_mut().enumerate() {
            self.values[index] = node.next(close, &self.values);
        }
        for (index, (_, node)) in self.targets.iter().enumerate() {
            self.output_scratch[index] = self.values[*node];
        }
        self.row_count = self.row_count.saturating_add(1);
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum StatefulFactorNode {
    Momentum(LaggedReturnState),
    Volatility(VolatilityState),
    Negate { dependency: usize },
}

impl StatefulFactorNode {
    fn next(&mut self, close: f64, values: &[f64]) -> f64 {
        match self {
            Self::Momentum(state) => state.next(close),
            Self::Volatility(state) => state.next(close),
            Self::Negate { dependency } => {
                let value = values[*dependency];
                if value.is_finite() {
                    -value
                } else {
                    f64::NAN
                }
            }
        }
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct LaggedReturnState {
    period: usize,
    values: VecDeque<f64>,
}

impl LaggedReturnState {
    fn new(period: usize) -> Self {
        Self {
            period,
            values: VecDeque::with_capacity(period),
        }
    }

    fn next(&mut self, value: f64) -> f64 {
        if !value.is_finite() {
            self.values.clear();
            return f64::NAN;
        }
        let result = self.values.front().map_or(f64::NAN, |start| {
            if self.values.len() == self.period {
                crate::returns::return_between(
                    *start,
                    value,
                    crate::returns::ReturnKind::Arithmetic,
                )
            } else {
                f64::NAN
            }
        });
        self.values.push_back(value);
        if self.values.len() > self.period {
            self.values.pop_front();
        }
        result
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct VolatilityState {
    period: usize,
    previous: Option<f64>,
    returns: VecDeque<f64>,
    sum: f64,
    sum_sq: f64,
}

impl VolatilityState {
    fn new(period: usize) -> Self {
        Self {
            period,
            previous: None,
            returns: VecDeque::with_capacity(period),
            sum: 0.0,
            sum_sq: 0.0,
        }
    }

    fn next(&mut self, value: f64) -> f64 {
        if !value.is_finite() {
            self.previous = None;
            self.returns.clear();
            self.sum = 0.0;
            self.sum_sq = 0.0;
            return f64::NAN;
        }
        let Some(previous) = self.previous.replace(value) else {
            return f64::NAN;
        };
        let current =
            crate::returns::return_between(previous, value, crate::returns::ReturnKind::Arithmetic);
        self.returns.push_back(current);
        self.sum += current;
        self.sum_sq += current * current;
        if self.returns.len() > self.period {
            let old = self.returns.pop_front().expect("factor volatility state");
            self.sum -= old;
            self.sum_sq -= old * old;
        }
        if self.returns.len() == self.period {
            let mean = self.sum / self.period as f64;
            ((self.sum_sq - self.sum * mean) / self.period as f64)
                .max(0.0)
                .sqrt()
        } else {
            f64::NAN
        }
    }
}

fn borrowed_context(
    inputs: &BTreeMap<String, Vec<f64>>,
) -> FactorResult<BorrowedFactorContext<'_>> {
    let mut context = BorrowedFactorContext::new();
    for (name, values) in inputs {
        context.insert(name.clone(), values.as_slice())?;
    }
    Ok(context)
}

fn validate_stream_checkpoint(
    plan: &CompiledFactorPlan,
    row_count: usize,
    inputs: &BTreeMap<String, Vec<f64>>,
    output: &BTreeMap<String, Vec<f64>>,
) -> FactorResult<()> {
    let rows = inputs.values().next().map_or(0, Vec::len);
    if rows > row_count {
        return Err(FactorError::InvalidParameter(
            "factor stream checkpoint buffer exceeds its logical row count".to_string(),
        ));
    }
    for input in plan.required_raw_inputs() {
        let values = inputs
            .get(input)
            .ok_or_else(|| FactorError::MissingInput(input.clone()))?;
        if values.len() != rows {
            return Err(FactorError::LengthMismatch {
                name: input.clone(),
                expected: rows,
                actual: values.len(),
            });
        }
    }
    for name in plan.execution_order() {
        let values = output.get(name).ok_or_else(|| {
            FactorError::InvalidParameter(format!(
                "factor stream checkpoint is missing output {name}"
            ))
        })?;
        if values.len() != rows {
            return Err(FactorError::LengthMismatch {
                name: name.clone(),
                expected: rows,
                actual: values.len(),
            });
        }
    }
    Ok(())
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
    fn built_in_factor_catalog_publishes_incremental_lookbacks() {
        let catalog = FactorCatalog::from_registry(crate::factors::builtin_factor_registry());
        let momentum = catalog.compile(&["momentum_5"]).unwrap();
        assert!(momentum.supports_range_incremental());
        assert_eq!(momentum.range_lookback(), Some(5));

        let reversal = catalog.compile(&["reversal_5"]).unwrap();
        assert!(reversal.supports_range_incremental());
        assert_eq!(reversal.range_lookback(), Some(5));

        let descriptor = catalog.descriptor("volatility_20").unwrap();
        assert!(descriptor.metadata.incremental);
        assert_eq!(descriptor.metadata.fixed_lookback, Some(20));
        assert!(descriptor.metadata.streaming);
    }

    #[test]
    fn finite_factor_stream_matches_batch_and_restores_checkpoint() {
        let catalog = FactorCatalog::from_registry(crate::factors::builtin_factor_registry());
        let plan = catalog.compile(&["momentum_5"]).unwrap();
        let engine = FactorEngine::new(catalog.into_registry());
        let mut stream = plan.stream(engine.clone()).unwrap();
        let close = [10.0, 11.0, 12.0, 15.0, 14.0, 16.0, 17.0, 18.0];
        let mut rows = Vec::new();
        for value in close {
            let mut row = BTreeMap::new();
            row.insert("close".to_string(), value);
            rows.push(stream.push_row(&row).unwrap()["momentum_5"]);
        }

        let context = BorrowedFactorContext::new()
            .with_series("close", &close)
            .unwrap();
        let batch = plan.execute_borrowed(&engine, &context).unwrap();
        assert!(rows.iter().zip(&batch["momentum_5"]).all(|(left, right)| {
            (left.is_nan() && right.is_nan()) || (left - right).abs() < 1e-12
        }));

        let mut batch_stream = plan.stream(engine.clone()).unwrap();
        let batch_inputs = BTreeMap::from([(String::from("close"), close.to_vec())]);
        let batch_values = batch_stream.push_batch(&batch_inputs).unwrap();
        assert!(batch_values["momentum_5"]
            .iter()
            .zip(&batch["momentum_5"])
            .all(|(left, right)| {
                (left.is_nan() && right.is_nan()) || (left - right).abs() < 1e-12
            }));

        let mut reusable_stream = plan.stream(engine.clone()).unwrap();
        let mut emitted = BTreeMap::new();
        reusable_stream
            .push_batch_into(&batch_inputs, &mut emitted)
            .unwrap();
        let first_capacity = emitted["momentum_5"].capacity();
        let first_pointer = emitted["momentum_5"].as_ptr();
        let next_inputs = BTreeMap::from([(String::from("close"), vec![19.0, 20.0])]);
        reusable_stream
            .push_batch_into(&next_inputs, &mut emitted)
            .unwrap();
        assert_eq!(emitted["momentum_5"].len(), 2);
        assert_eq!(emitted["momentum_5"].capacity(), first_capacity);
        assert_eq!(emitted["momentum_5"].as_ptr(), first_pointer);

        let checkpoint = stream.checkpoint();
        let mut next = BTreeMap::new();
        next.insert("close".to_string(), 16.0);
        let expected = stream.push_row(&next).unwrap();
        stream.restore(&checkpoint).unwrap();
        let restored = stream.push_row(&next).unwrap();
        assert!(restored.iter().all(|(name, value)| {
            let expected_value = expected[name];
            (value.is_nan() && expected_value.is_nan()) || (value - expected_value).abs() < 1e-12
        }));
        assert_eq!(stream.rows(), 9);
    }

    #[test]
    fn stateful_builtin_factor_stream_matches_batch_and_restores_checkpoint() {
        let catalog = FactorCatalog::from_registry(crate::factors::builtin_factor_registry());
        let close: Vec<f64> = (0..120)
            .map(|index| 100.0 + index as f64 * 0.2 + (index as f64 * 0.09).sin())
            .collect();
        for target in ["momentum_5", "volatility_20", "reversal_5"] {
            let plan = catalog.compile(&[target]).unwrap();
            let engine = FactorEngine::new(catalog.registry().clone());
            let context = BorrowedFactorContext::new()
                .with_series("close", &close)
                .unwrap();
            let batch = plan.execute_borrowed(&engine, &context).unwrap();
            let mut stream = plan.stateful_stream().unwrap();
            let mut actual = Vec::with_capacity(close.len());
            let mut output = [f64::NAN];
            for value in &close {
                stream.push_values_into(&[*value], &mut output).unwrap();
                actual.push(output[0]);
            }
            assert!(actual.iter().zip(&batch[target]).all(|(left, right)| {
                (left.is_nan() && right.is_nan()) || (left - right).abs() < 1e-10
            }));

            let checkpoint = stream.checkpoint();
            stream.push_values_into(&[close[0]], &mut output).unwrap();
            stream.restore(&checkpoint).unwrap();
            let mut restored = [f64::NAN];
            stream.push_values_into(&[close[0]], &mut restored).unwrap();
            assert_eq!(output[0], restored[0]);
        }
    }

    #[test]
    fn recursive_factor_plan_rejects_bounded_streaming() {
        let mut catalog = FactorCatalog::new();
        catalog
            .register(identity("recursive", "close"), FactorMetadata::default())
            .unwrap();
        let plan = catalog.compile(&["recursive"]).unwrap();
        let engine = FactorEngine::new(catalog.into_registry());
        assert!(matches!(
            plan.stream(engine),
            Err(FactorError::InvalidParameter(_))
        ));
    }

    #[test]
    fn range_plan_accumulates_dependency_lookback_and_propagates_dirty_rows() {
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
                input_dirty: DirtyRange::new(5, 6),
                affected: DirtyRange::new(5, 8),
                recompute: DirtyRange::new(0, 8),
            }
        );
        assert_eq!(ranged.trace.recomputed_rows, 8);
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
