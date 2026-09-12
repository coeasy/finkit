//! Execution scheduling helpers for full and DirtyRange evaluation.

use super::execution_plan::{ExecutionPlan, NodeId};
use crate::unified_runtime::DirtyRange;
use std::fmt;

/// Runtime schedule validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    /// Dirty range exceeds the authoritative row count.
    DirtyRangeOutOfBounds { dirty: DirtyRange, rows: usize },
    /// The plan contains recursive state and no replay checkpoint was supplied.
    RecursiveDependencyRequiresCheckpoint,
    /// One or more finite horizons depend on runtime parameters that have not
    /// yet been resolved into fixed lookbacks.
    DynamicHorizonRequiresResolution,
    /// A supplied checkpoint is newer than the first changed row and therefore
    /// already contains state contaminated by the historical edit.
    CheckpointAfterDirty {
        checkpoint_row: usize,
        dirty_start: usize,
    },
    /// Checkpoint lies outside the authoritative row range.
    CheckpointOutOfBounds { checkpoint_row: usize, rows: usize },
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DirtyRangeOutOfBounds { dirty, rows } => write!(
                f,
                "dirty range {}..{} exceeds runtime rows {rows}",
                dirty.start, dirty.end
            ),
            Self::RecursiveDependencyRequiresCheckpoint => write!(
                f,
                "dirty-range execution contains recursive state and requires checkpoint replay"
            ),
            Self::DynamicHorizonRequiresResolution => write!(
                f,
                "dirty-range execution contains unresolved runtime-dependent lookbacks"
            ),
            Self::CheckpointAfterDirty {
                checkpoint_row,
                dirty_start,
            } => write!(
                f,
                "checkpoint row {checkpoint_row} is after dirty start {dirty_start}"
            ),
            Self::CheckpointOutOfBounds {
                checkpoint_row,
                rows,
            } => write!(
                f,
                "checkpoint row {checkpoint_row} exceeds runtime rows {rows}"
            ),
        }
    }
}

impl std::error::Error for ScheduleError {}

/// Immutable row/node schedule emitted for one runtime execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledExecution {
    /// Topological nodes to execute.
    pub nodes: Vec<NodeId>,
    /// Raw input rows changed by the caller.
    pub input_dirty: DirtyRange,
    /// Output rows potentially affected after propagation.
    pub affected: DirtyRange,
    /// Input rows required to reconstruct state and affected outputs.
    pub recompute: DirtyRange,
}

/// Scheduler shared by batch and dirty-range execution.
#[derive(Debug, Clone, Copy, Default)]
pub struct ExecutionScheduler;

impl ExecutionScheduler {
    /// Schedule every row and every node.
    #[must_use]
    pub fn full(plan: &ExecutionPlan, rows: usize) -> ScheduledExecution {
        let full = DirtyRange::full(rows);
        ScheduledExecution {
            nodes: plan.execution_order().to_vec(),
            input_dirty: full,
            affected: full,
            recompute: full,
        }
    }

    /// Schedule the minimal row interval implied by finite cumulative
    /// lookbacks. Recursive state and unresolved runtime lookbacks are rejected
    /// rather than guessed.
    pub fn dirty(
        plan: &ExecutionPlan,
        dirty: DirtyRange,
        rows: usize,
    ) -> Result<ScheduledExecution, ScheduleError> {
        Self::validate_dirty(dirty, rows)?;
        if dirty.is_empty() {
            return Ok(Self::empty(dirty));
        }
        if plan.has_dynamic_horizon() {
            return Err(ScheduleError::DynamicHorizonRequiresResolution);
        }
        if plan.has_recursive_horizon() {
            return Err(ScheduleError::RecursiveDependencyRequiresCheckpoint);
        }

        let lookback = plan
            .max_lookback()
            .expect("fixed-only plan has a finite cumulative lookback");
        Ok(Self::fixed_window_schedule(plan, dirty, rows, lookback))
    }

    /// Schedule a historical edit with a checkpoint whose state represents all
    /// rows strictly before `checkpoint_row`.
    ///
    /// Dynamic horizons must still be resolved first. Fixed-window plans use
    /// their smaller mathematically proven range. Recursive plans replay from
    /// the checkpoint through the end because a changed historical value can
    /// affect every subsequent recursive state.
    pub fn dirty_from_checkpoint(
        plan: &ExecutionPlan,
        dirty: DirtyRange,
        rows: usize,
        checkpoint_row: usize,
    ) -> Result<ScheduledExecution, ScheduleError> {
        Self::validate_dirty(dirty, rows)?;
        if checkpoint_row > rows {
            return Err(ScheduleError::CheckpointOutOfBounds {
                checkpoint_row,
                rows,
            });
        }
        if dirty.is_empty() {
            return Ok(Self::empty(dirty));
        }
        if plan.has_dynamic_horizon() {
            return Err(ScheduleError::DynamicHorizonRequiresResolution);
        }
        if checkpoint_row > dirty.start {
            return Err(ScheduleError::CheckpointAfterDirty {
                checkpoint_row,
                dirty_start: dirty.start,
            });
        }

        if !plan.has_recursive_horizon() {
            let lookback = plan
                .max_lookback()
                .expect("fixed-only plan has a finite cumulative lookback");
            return Ok(Self::fixed_window_schedule(plan, dirty, rows, lookback));
        }

        Ok(ScheduledExecution {
            nodes: plan.execution_order().to_vec(),
            input_dirty: dirty,
            affected: DirtyRange::new(dirty.start, rows),
            recompute: DirtyRange::new(checkpoint_row, rows),
        })
    }

    fn fixed_window_schedule(
        plan: &ExecutionPlan,
        dirty: DirtyRange,
        rows: usize,
        lookback: usize,
    ) -> ScheduledExecution {
        let affected = dirty.propagate_forward(lookback, rows);
        let recompute = affected.with_lookback(lookback);
        ScheduledExecution {
            nodes: plan.execution_order().to_vec(),
            input_dirty: dirty,
            affected,
            recompute,
        }
    }

    fn validate_dirty(dirty: DirtyRange, rows: usize) -> Result<(), ScheduleError> {
        if !dirty.is_within(rows) {
            Err(ScheduleError::DirtyRangeOutOfBounds { dirty, rows })
        } else {
            Ok(())
        }
    }

    fn empty(dirty: DirtyRange) -> ScheduledExecution {
        ScheduledExecution {
            nodes: Vec::new(),
            input_dirty: dirty,
            affected: dirty,
            recompute: dirty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::kernels::KernelFamily;
    use crate::runtime_engine::ExecutionPlanBuilder;

    #[test]
    fn dirty_schedule_uses_cumulative_plan_lookback() {
        let mut builder = ExecutionPlanBuilder::new();
        let input = builder
            .intern_node(KernelFamily::Scalar, "close", vec![], 0)
            .unwrap();
        let sma = builder
            .intern_node(KernelFamily::MovingAverage, "sma:20", vec![input], 19)
            .unwrap();
        builder
            .intern_node(KernelFamily::Statistics, "stddev:10", vec![sma], 9)
            .unwrap();
        let plan = builder.build().unwrap();

        let scheduled = ExecutionScheduler::dirty(&plan, DirtyRange::new(50, 51), 100).unwrap();
        assert_eq!(plan.max_lookback(), Some(28));
        assert_eq!(scheduled.affected, DirtyRange::new(50, 79));
        assert_eq!(scheduled.recompute, DirtyRange::new(22, 79));
    }

    #[test]
    fn recursive_plan_requires_checkpoint_for_dirty_replay() {
        let mut builder = ExecutionPlanBuilder::new();
        let input = builder
            .intern_node(KernelFamily::Scalar, "close", vec![], 0)
            .unwrap();
        builder
            .intern_recursive_node(KernelFamily::MovingAverage, "ema:20", vec![input])
            .unwrap();
        let plan = builder.build().unwrap();
        assert_eq!(
            ExecutionScheduler::dirty(&plan, DirtyRange::new(50, 51), 100).unwrap_err(),
            ScheduleError::RecursiveDependencyRequiresCheckpoint
        );
    }

    #[test]
    fn dynamic_plan_requires_runtime_resolution() {
        let mut builder = ExecutionPlanBuilder::new();
        builder
            .intern_dynamic_node(KernelFamily::MovingAverage, "sma:runtime", vec![])
            .unwrap();
        let plan = builder.build().unwrap();
        assert_eq!(
            ExecutionScheduler::dirty(&plan, DirtyRange::new(10, 11), 100).unwrap_err(),
            ScheduleError::DynamicHorizonRequiresResolution
        );
        assert_eq!(
            ExecutionScheduler::dirty_from_checkpoint(
                &plan,
                DirtyRange::new(10, 11),
                100,
                0,
            )
            .unwrap_err(),
            ScheduleError::DynamicHorizonRequiresResolution
        );
    }

    #[test]
    fn recursive_plan_replays_from_safe_checkpoint_to_end() {
        let mut builder = ExecutionPlanBuilder::new();
        let input = builder
            .intern_node(KernelFamily::Scalar, "close", vec![], 0)
            .unwrap();
        builder
            .intern_recursive_node(KernelFamily::MovingAverage, "ema:20", vec![input])
            .unwrap();
        let plan = builder.build().unwrap();
        let scheduled =
            ExecutionScheduler::dirty_from_checkpoint(&plan, DirtyRange::new(50, 51), 100, 40)
                .unwrap();
        assert_eq!(scheduled.affected, DirtyRange::new(50, 100));
        assert_eq!(scheduled.recompute, DirtyRange::new(40, 100));
    }

    #[test]
    fn contaminated_checkpoint_is_rejected() {
        let mut builder = ExecutionPlanBuilder::new();
        builder
            .intern_recursive_node(KernelFamily::Volatility, "atr:14", vec![])
            .unwrap();
        let plan = builder.build().unwrap();
        assert_eq!(
            ExecutionScheduler::dirty_from_checkpoint(&plan, DirtyRange::new(50, 51), 100, 60)
                .unwrap_err(),
            ScheduleError::CheckpointAfterDirty {
                checkpoint_row: 60,
                dirty_start: 50,
            }
        );
    }

    #[test]
    fn empty_dirty_range_schedules_no_nodes() {
        let plan = ExecutionPlanBuilder::new().build().unwrap();
        let scheduled = ExecutionScheduler::dirty(&plan, DirtyRange::new(5, 5), 10).unwrap();
        assert!(scheduled.nodes.is_empty());
        assert_eq!(scheduled.recompute, DirtyRange::new(5, 5));
    }
}
