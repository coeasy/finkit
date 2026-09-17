//! Unified runtime contracts shared by formula, factor, and research layers.
//!
//! This module owns cross-domain execution identity and dirty-range semantics.
//! Domain planners remain responsible for proving that a node is range-safe;
//! once that proof exists, [`UnifiedRuntime`] executes only the required input
//! slice and splices the affected rows into the retained materialization.

use crate::compute::FactorPlan;
use crate::factors::{
    BorrowedFactorContext, FactorContext, FactorEngine, FactorError, FactorResult,
};
use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;

/// Stable, typed content identity for a materialized artifact.
///
/// The hash deliberately uses a fixed FNV-1a 64-bit algorithm rather than
/// `DefaultHasher`, whose algorithm is not a persistence contract. This makes
/// persisted artifact references deterministic across processes and platforms.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ArtifactHash(u64);

impl ArtifactHash {
    const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

    /// Build a typed hash from an already validated raw value.
    #[must_use]
    pub const fn from_u64(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw numeric representation for ABI/legacy adapters.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Compute a deterministic content hash from canonical bytes.
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut state = Self::FNV_OFFSET_BASIS;
        for byte in bytes {
            state ^= u64::from(*byte);
            state = state.wrapping_mul(Self::FNV_PRIME);
        }
        Self(state)
    }
}

impl fmt::Display for ArtifactHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl fmt::LowerHex for ArtifactHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

/// Half-open row range invalidated by a data change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DirtyRange {
    /// First dirty row, inclusive.
    pub start: usize,
    /// First clean row after the dirty region, exclusive.
    pub end: usize,
}

impl DirtyRange {
    /// Build a normalized half-open dirty range.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }

    /// A dirty range covering every row.
    #[must_use]
    pub const fn full(rows: usize) -> Self {
        Self {
            start: 0,
            end: rows,
        }
    }

    /// Whether no rows are dirty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Number of dirty rows.
    #[must_use]
    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Whether the range is valid for an input with `rows` rows.
    #[must_use]
    pub const fn is_within(self, rows: usize) -> bool {
        self.start <= self.end && self.end <= rows
    }

    /// Convert to a standard Rust range.
    #[must_use]
    pub const fn as_range(self) -> Range<usize> {
        self.start..self.end
    }

    /// Expand backwards to provide historical rows required by an output
    /// interval. This does not change the output interval itself.
    #[must_use]
    pub const fn with_lookback(self, lookback: usize) -> Self {
        Self {
            start: self.start.saturating_sub(lookback),
            end: self.end,
        }
    }

    /// Propagate an input mutation forward through a trailing-window dependency.
    ///
    /// If a node needs `lookback` previous rows, mutating input row `i` can
    /// affect outputs from `i` through `i + lookback`. Dependency-chain
    /// lookbacks therefore expand the dirty range's end before execution.
    #[must_use]
    pub const fn propagate_forward(self, lookback: usize, rows: usize) -> Self {
        if self.is_empty() {
            return self;
        }
        let expanded_end = self.end.saturating_add(lookback);
        Self {
            start: self.start,
            end: if expanded_end < rows {
                expanded_end
            } else {
                rows
            },
        }
    }

    /// Merge two invalidation ranges conservatively.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        Self {
            start: if self.start < other.start {
                self.start
            } else {
                other.start
            },
            end: if self.end > other.end {
                self.end
            } else {
                other.end
            },
        }
    }
}

/// Runtime path used for one plan execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeExecutionMode {
    /// Every row was recomputed.
    Full,
    /// Only rows affected by one input dirty range were evaluated.
    Range {
        /// Rows changed in the authoritative raw input.
        input_dirty: DirtyRange,
        /// Output interval affected after dependency propagation.
        affected: DirtyRange,
        /// Input slice actually evaluated after historical lookback expansion.
        recompute: DirtyRange,
    },
}

/// Evidence emitted by the unified runtime for correctness/performance gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeExecutionTrace {
    /// Selected execution path.
    pub mode: RuntimeExecutionMode,
    /// Total rows in the authoritative input.
    pub rows: usize,
    /// Number of factor nodes executed.
    pub executed_nodes: usize,
    /// Number of rows evaluated by each executed node.
    pub recomputed_rows: usize,
}

/// Typed runtime result carrying both values and execution evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeExecution<T> {
    /// Materialized plan outputs.
    pub output: T,
    /// Runtime execution evidence.
    pub trace: RuntimeExecutionTrace,
}

/// Canonical plan execution boundary.
///
/// The factor implementation is the first domain wired through this boundary.
/// Formula and research planners can use the same contracts without creating a
/// second DirtyRange or artifact identity model.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnifiedRuntime;

impl UnifiedRuntime {
    /// Execute a factor plan over owned aligned input.
    pub fn execute_factor_plan(
        plan: &FactorPlan,
        engine: &FactorEngine,
        context: &FactorContext,
    ) -> FactorResult<RuntimeExecution<BTreeMap<String, Vec<f64>>>> {
        let output = plan.execute(engine, context)?;
        Ok(RuntimeExecution {
            output,
            trace: RuntimeExecutionTrace {
                mode: RuntimeExecutionMode::Full,
                rows: context.len(),
                executed_nodes: plan.execution_order().len(),
                recomputed_rows: context.len(),
            },
        })
    }

    /// Execute a factor plan over zero-copy borrowed aligned input.
    pub fn execute_factor_plan_borrowed(
        plan: &FactorPlan,
        engine: &FactorEngine,
        context: &BorrowedFactorContext<'_>,
    ) -> FactorResult<RuntimeExecution<BTreeMap<String, Vec<f64>>>> {
        let output = plan.execute_borrowed(engine, context)?;
        Ok(RuntimeExecution {
            output,
            trace: RuntimeExecutionTrace {
                mode: RuntimeExecutionMode::Full,
                rows: context.len(),
                executed_nodes: plan.execution_order().len(),
                recomputed_rows: context.len(),
            },
        })
    }

    /// Recompute only rows affected by a proven-safe raw-input dirty interval.
    ///
    /// `lookback` is supplied by the semantic planner after accumulating the
    /// historical requirements of the complete dependency chain. The runtime
    /// first propagates the raw change forward through that dependency chain,
    /// then expands backwards only as far as needed to evaluate those outputs.
    pub fn execute_factor_plan_range_borrowed(
        plan: &FactorPlan,
        engine: &FactorEngine,
        context: &BorrowedFactorContext<'_>,
        previous: &BTreeMap<String, Vec<f64>>,
        dirty: DirtyRange,
        lookback: usize,
    ) -> FactorResult<RuntimeExecution<BTreeMap<String, Vec<f64>>>> {
        let mut output = previous.clone();
        let trace = Self::execute_factor_plan_range_into_borrowed(
            plan,
            engine,
            context,
            &mut output,
            dirty,
            lookback,
        )?;
        Ok(RuntimeExecution { output, trace })
    }

    /// In-place dirty-range execution that preserves all unaffected rows and
    /// their allocations.
    pub fn execute_factor_plan_range_into_borrowed(
        plan: &FactorPlan,
        engine: &FactorEngine,
        context: &BorrowedFactorContext<'_>,
        output: &mut BTreeMap<String, Vec<f64>>,
        dirty: DirtyRange,
        lookback: usize,
    ) -> FactorResult<RuntimeExecutionTrace> {
        plan.validate_borrowed_context(context)?;
        let rows = context.len();
        if !dirty.is_within(rows) {
            return Err(FactorError::InvalidParameter(format!(
                "dirty range {}..{} exceeds factor input rows {rows}",
                dirty.start, dirty.end
            )));
        }
        Self::validate_retained_outputs(plan, output, rows)?;

        if dirty.is_empty() {
            return Ok(RuntimeExecutionTrace {
                mode: RuntimeExecutionMode::Range {
                    input_dirty: dirty,
                    affected: dirty,
                    recompute: dirty,
                },
                rows,
                executed_nodes: 0,
                recomputed_rows: 0,
            });
        }

        let affected = dirty.propagate_forward(lookback, rows);
        let recompute = affected.with_lookback(lookback);
        let mut sliced = BorrowedFactorContext::new();
        for input in plan.required_raw_inputs() {
            let values = context
                .get(input)
                .ok_or_else(|| FactorError::MissingInput(input.clone()))?;
            sliced.insert(input.clone(), &values[recompute.as_range()])?;
        }

        let partial = plan.execute_borrowed(engine, &sliced)?;
        let offset = affected.start - recompute.start;
        let source_end = offset + affected.len();
        for name in plan.execution_order() {
            let source = partial.get(name).ok_or_else(|| {
                FactorError::InvalidParameter(format!(
                    "factor runtime did not materialize planned node {name}"
                ))
            })?;
            if source_end > source.len() {
                return Err(FactorError::LengthMismatch {
                    name: name.clone(),
                    expected: source_end,
                    actual: source.len(),
                });
            }
            let target = output.get_mut(name).ok_or_else(|| {
                FactorError::InvalidParameter(format!(
                    "dirty-range execution requires retained output for {name}"
                ))
            })?;
            target[affected.as_range()].copy_from_slice(&source[offset..source_end]);
        }

        Ok(RuntimeExecutionTrace {
            mode: RuntimeExecutionMode::Range {
                input_dirty: dirty,
                affected,
                recompute,
            },
            rows,
            executed_nodes: plan.execution_order().len(),
            recomputed_rows: recompute.len(),
        })
    }

    fn validate_retained_outputs(
        plan: &FactorPlan,
        output: &BTreeMap<String, Vec<f64>>,
        rows: usize,
    ) -> FactorResult<()> {
        for name in plan.execution_order() {
            let values = output.get(name).ok_or_else(|| {
                FactorError::InvalidParameter(format!(
                    "dirty-range execution requires retained output for {name}"
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factors::{FactorDefinition, FactorDirection, FactorKind, FactorRegistry};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn typed_artifact_hash_is_stable_and_distinct() {
        let first = ArtifactHash::from_bytes(b"factor-output");
        assert_eq!(first, ArtifactHash::from_bytes(b"factor-output"));
        assert_ne!(first, ArtifactHash::from_bytes(b"factor-output-2"));
        assert_eq!(first.to_string().len(), 16);
        assert_eq!(ArtifactHash::from_u64(first.get()), first);
    }

    #[test]
    fn dirty_range_propagates_forward_then_expands_for_history() {
        let dirty = DirtyRange::new(5, 6);
        let affected = dirty.propagate_forward(3, 12);
        assert_eq!(affected, DirtyRange::new(5, 9));
        assert_eq!(affected.with_lookback(3), DirtyRange::new(2, 9));
        assert_eq!(dirty.len(), 1);
        assert!(dirty.is_within(12));
    }

    #[test]
    fn range_runtime_executes_only_affected_window_plus_history() {
        let visited_rows = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&visited_rows);
        let mut registry = FactorRegistry::new();
        registry
            .register(FactorDefinition::new(
                "score",
                ["close"],
                FactorKind::TimeSeries,
                FactorDirection::HigherBetter,
                Arc::new(move |inputs| {
                    let close = inputs.get("close")?;
                    counter.fetch_add(close.len(), Ordering::SeqCst);
                    Ok(close.iter().map(|value| value * 2.0).collect())
                }),
            ))
            .unwrap();
        let plan = FactorPlan::compile(&registry, &["score"]).unwrap();
        let engine = FactorEngine::new(registry);

        let original = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let original_context = BorrowedFactorContext::new()
            .with_series("close", &original)
            .unwrap();
        let full = UnifiedRuntime::execute_factor_plan_borrowed(&plan, &engine, &original_context)
            .unwrap();
        assert_eq!(visited_rows.load(Ordering::SeqCst), original.len());

        let changed = [1.0, 2.0, 3.0, 40.0, 5.0, 6.0];
        let changed_context = BorrowedFactorContext::new()
            .with_series("close", &changed)
            .unwrap();
        let ranged = UnifiedRuntime::execute_factor_plan_range_borrowed(
            &plan,
            &engine,
            &changed_context,
            &full.output,
            DirtyRange::new(3, 4),
            1,
        )
        .unwrap();

        assert_eq!(visited_rows.load(Ordering::SeqCst), original.len() + 3);
        assert_eq!(
            ranged.output["score"],
            vec![2.0, 4.0, 6.0, 80.0, 10.0, 12.0]
        );
        assert_eq!(
            ranged.trace.mode,
            RuntimeExecutionMode::Range {
                input_dirty: DirtyRange::new(3, 4),
                affected: DirtyRange::new(3, 5),
                recompute: DirtyRange::new(2, 5),
            }
        );
        assert_eq!(ranged.trace.recomputed_rows, 3);
    }
}
