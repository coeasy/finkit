//! Execution scheduling helpers for full and DirtyRange evaluation.

use super::execution_plan::{ExecutionPlan, NodeId};
use crate::unified_runtime::DirtyRange;
use std::fmt;

/// Runtime schedule validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    /// Dirty range exceeds the authoritative row count.
    DirtyRangeOutOfBounds { dirty: DirtyRange, rows: usize },
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DirtyRangeOutOfBounds { dirty, rows } => write!(
                f,
                "dirty range {}..{} exceeds runtime rows {rows}",
                dirty.start, dirty.end
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
    /// Output rows potentially affected after forward propagation.
    pub affected: DirtyRange,
    /// Input rows required after backward lookback expansion.
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

    /// Schedule the minimal row interval implied by the plan's cumulative
    /// lookback contract.
    pub fn dirty(
        plan: &ExecutionPlan,
        dirty: DirtyRange,
        rows: usize,
    ) -> Result<ScheduledExecution, ScheduleError> {
        if !dirty.is_within(rows) {
            return Err(ScheduleError::DirtyRangeOutOfBounds { dirty, rows });
        }
        if dirty.is_empty() {
            return Ok(ScheduledExecution {
                nodes: Vec::new(),
                input_dirty: dirty,
                affected: dirty,
                recompute: dirty,
            });
        }

        let lookback = plan.max_lookback();
        let affected = dirty.propagate_forward(lookback, rows);
        let recompute = affected.with_lookback(lookback);
        Ok(ScheduledExecution {
            nodes: plan.execution_order().to_vec(),
            input_dirty: dirty,
            affected,
            recompute,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::kernels::KernelFamily;
    use crate::runtime::execution_plan::ExecutionPlanBuilder;

    #[test]
    fn dirty_schedule_uses_cumulative_plan_lookback() {
        let mut builder = ExecutionPlanBuilder::new();
        let input = builder
            .intern_node(KernelFamily::MovingAverage, "close", vec![], 0)
            .unwrap();
        let sma = builder
            .intern_node(KernelFamily::MovingAverage, "sma:20", vec![input], 19)
            .unwrap();
        builder
            .intern_node(KernelFamily::Statistics, "stddev:10", vec![sma], 9)
            .unwrap();
        let plan = builder.build().unwrap();

        let scheduled = ExecutionScheduler::dirty(&plan, DirtyRange::new(50, 51), 100).unwrap();
        assert_eq!(plan.max_lookback(), 28);
        assert_eq!(scheduled.affected, DirtyRange::new(50, 79));
        assert_eq!(scheduled.recompute, DirtyRange::new(22, 79));
    }

    #[test]
    fn empty_dirty_range_schedules_no_nodes() {
        let plan = ExecutionPlanBuilder::new().build().unwrap();
        let scheduled = ExecutionScheduler::dirty(&plan, DirtyRange::new(5, 5), 10).unwrap();
        assert!(scheduled.nodes.is_empty());
        assert_eq!(scheduled.recompute, DirtyRange::new(5, 5));
    }
}
