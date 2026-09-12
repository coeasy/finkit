//! Stateful runtime session binding plan nodes to typed kernel state.

use super::execution_plan::{ExecutionPlan, NodeId};
use super::scheduler::{ExecutionScheduler, ScheduleError, ScheduledExecution};
use super::state_arena::{StateArena, StateArenaCheckpoint, StateArenaError, StateHandle};
use crate::unified_runtime::DirtyRange;
use std::any::Any;
use std::fmt;

/// Runtime-session failures.
#[derive(Debug)]
pub enum RuntimeSessionError {
    /// Requested node is not present in the compiled plan.
    UnknownNode(NodeId),
    /// A state is already bound to the node.
    StateAlreadyBound(NodeId),
    /// No state has been bound to the node.
    StateNotBound(NodeId),
    /// Typed state-arena access failed.
    State(StateArenaError),
    /// Row scheduling failed.
    Schedule(ScheduleError),
    /// A checkpoint without a row boundary cannot prove safe recursive replay.
    CheckpointHasNoRowBoundary,
}

impl fmt::Display for RuntimeSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(node) => write!(f, "unknown runtime plan node {}", node.0),
            Self::StateAlreadyBound(node) => {
                write!(f, "runtime state already bound to node {}", node.0)
            }
            Self::StateNotBound(node) => write!(f, "runtime state not bound to node {}", node.0),
            Self::State(error) => error.fmt(f),
            Self::Schedule(error) => error.fmt(f),
            Self::CheckpointHasNoRowBoundary => write!(
                f,
                "runtime checkpoint has no row boundary for recursive dirty replay"
            ),
        }
    }
}

impl std::error::Error for RuntimeSessionError {}

impl From<StateArenaError> for RuntimeSessionError {
    fn from(value: StateArenaError) -> Self {
        Self::State(value)
    }
}

impl From<ScheduleError> for RuntimeSessionError {
    fn from(value: ScheduleError) -> Self {
        Self::Schedule(value)
    }
}

/// Checkpoint of all bound kernel states for a compiled plan.
#[derive(Debug, Clone)]
pub struct RuntimeSessionCheckpoint {
    arena: StateArenaCheckpoint,
    next_row: Option<usize>,
}

impl RuntimeSessionCheckpoint {
    /// First row not represented in the saved kernel states. `None` means the
    /// checkpoint is state-only and cannot prove a historical replay boundary.
    #[must_use]
    pub const fn next_row(&self) -> Option<usize> {
        self.next_row
    }
}

/// Long-lived execution session that reuses a plan and all kernel state.
#[derive(Debug)]
pub struct RuntimeSession {
    plan: ExecutionPlan,
    arena: StateArena,
    state_handles: Vec<Option<StateHandle>>,
}

impl RuntimeSession {
    /// Create a session with no node states allocated yet.
    #[must_use]
    pub fn new(plan: ExecutionPlan) -> Self {
        let state_handles = vec![None; plan.len()];
        Self {
            plan,
            arena: StateArena::new(),
            state_handles,
        }
    }

    /// Reusable compiled plan.
    #[must_use]
    pub const fn plan(&self) -> &ExecutionPlan {
        &self.plan
    }

    /// Number of bound node states.
    #[must_use]
    pub fn bound_state_count(&self) -> usize {
        self.state_handles
            .iter()
            .filter(|handle| handle.is_some())
            .count()
    }

    /// Bind one typed state to a plan node.
    pub fn bind_state<T>(
        &mut self,
        node: NodeId,
        state: T,
    ) -> Result<StateHandle, RuntimeSessionError>
    where
        T: Any + Clone + Send + Sync,
    {
        self.validate_node(node)?;
        if self.state_handles[node.0].is_some() {
            return Err(RuntimeSessionError::StateAlreadyBound(node));
        }
        let handle = self.arena.insert(state);
        self.state_handles[node.0] = Some(handle);
        Ok(handle)
    }

    /// Borrow one node's typed state.
    pub fn state<T>(&self, node: NodeId) -> Result<&T, RuntimeSessionError>
    where
        T: Any + Clone + Send + Sync,
    {
        let handle = self.state_handle(node)?;
        Ok(self.arena.get::<T>(handle)?)
    }

    /// Mutably borrow one node's typed state.
    pub fn state_mut<T>(&mut self, node: NodeId) -> Result<&mut T, RuntimeSessionError>
    where
        T: Any + Clone + Send + Sync,
    {
        let handle = self.state_handle(node)?;
        Ok(self.arena.get_mut::<T>(handle)?)
    }

    /// Capture all node states without attaching a row boundary.
    ///
    /// This remains useful for branch/rollback workflows, but recursive
    /// historical replay must use [`Self::checkpoint_at`].
    #[must_use]
    pub fn checkpoint(&self) -> RuntimeSessionCheckpoint {
        RuntimeSessionCheckpoint {
            arena: self.arena.checkpoint(),
            next_row: None,
        }
    }

    /// Capture node states after all rows strictly before `next_row` have been
    /// processed. This row boundary can later prove safe recursive replay.
    #[must_use]
    pub fn checkpoint_at(&self, next_row: usize) -> RuntimeSessionCheckpoint {
        RuntimeSessionCheckpoint {
            arena: self.arena.checkpoint(),
            next_row: Some(next_row),
        }
    }

    /// Restore every bound node state to an earlier checkpoint.
    pub fn restore(&mut self, checkpoint: &RuntimeSessionCheckpoint) {
        self.arena.restore(&checkpoint.arena);
    }

    /// Build a full-row execution schedule.
    #[must_use]
    pub fn schedule_full(&self, rows: usize) -> ScheduledExecution {
        ExecutionScheduler::full(&self.plan, rows)
    }

    /// Build a proven minimal DirtyRange schedule for a finite-window plan.
    pub fn schedule_dirty(
        &self,
        dirty: DirtyRange,
        rows: usize,
    ) -> Result<ScheduledExecution, RuntimeSessionError> {
        Ok(ExecutionScheduler::dirty(&self.plan, dirty, rows)?)
    }

    /// Build a correct historical-edit schedule using a row-addressable
    /// checkpoint. Recursive plans replay from that checkpoint to the end.
    pub fn schedule_dirty_from_checkpoint(
        &self,
        checkpoint: &RuntimeSessionCheckpoint,
        dirty: DirtyRange,
        rows: usize,
    ) -> Result<ScheduledExecution, RuntimeSessionError> {
        let next_row = checkpoint
            .next_row
            .ok_or(RuntimeSessionError::CheckpointHasNoRowBoundary)?;
        Ok(ExecutionScheduler::dirty_from_checkpoint(
            &self.plan, &dirty, rows, next_row,
        )?)
    }

    fn state_handle(&self, node: NodeId) -> Result<StateHandle, RuntimeSessionError> {
        self.validate_node(node)?;
        self.state_handles[node.0].ok_or(RuntimeSessionError::StateNotBound(node))
    }

    fn validate_node(&self, node: NodeId) -> Result<(), RuntimeSessionError> {
        if self.plan.node(node).is_some() {
            Ok(())
        } else {
            Err(RuntimeSessionError::UnknownNode(node))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::kernels::{KernelFamily, MovingAverageKind, MovingAverageState};
    use crate::runtime_engine::ExecutionPlanBuilder;

    #[test]
    fn node_state_survives_checkpoint_restore() {
        let mut builder = ExecutionPlanBuilder::new();
        let node = builder
            .intern_node(KernelFamily::MovingAverage, "sma:3", vec![], 2)
            .unwrap();
        let plan = builder.build().unwrap();
        let mut session = RuntimeSession::new(plan);
        session
            .bind_state(node, MovingAverageState::new(MovingAverageKind::Sma, 3))
            .unwrap();

        assert_eq!(
            session
                .state_mut::<MovingAverageState>(node)
                .unwrap()
                .update(3.0),
            3.0
        );
        let checkpoint = session.checkpoint();
        assert_eq!(
            session
                .state_mut::<MovingAverageState>(node)
                .unwrap()
                .update(6.0),
            4.5
        );
        session.restore(&checkpoint);
        assert_eq!(
            session
                .state_mut::<MovingAverageState>(node)
                .unwrap()
                .update(9.0),
            6.0
        );
    }

    #[test]
    fn dirty_schedule_is_available_from_session() {
        let mut builder = ExecutionPlanBuilder::new();
        builder
            .intern_node(KernelFamily::MovingAverage, "sma:5", vec![], 4)
            .unwrap();
        let session = RuntimeSession::new(builder.build().unwrap());
        let scheduled = session.schedule_dirty(DirtyRange::new(10, 11), 20).unwrap();
        assert_eq!(scheduled.affected, DirtyRange::new(10, 15));
        assert_eq!(scheduled.recompute, DirtyRange::new(6, 15));
    }

    #[test]
    fn recursive_dirty_schedule_uses_checkpoint_boundary() {
        let mut builder = ExecutionPlanBuilder::new();
        builder
            .intern_recursive_node(KernelFamily::Volatility, "atr:14", vec![])
            .unwrap();
        let session = RuntimeSession::new(builder.build().unwrap());
        let checkpoint = session.checkpoint_at(40);
        let scheduled = session
            .schedule_dirty_from_checkpoint(&checkpoint, DirtyRange::new(50, 51), 100)
            .unwrap();
        assert_eq!(scheduled.affected, DirtyRange::new(50, 100));
        assert_eq!(scheduled.recompute, DirtyRange::new(40, 100));
    }

    #[test]
    fn state_only_checkpoint_cannot_drive_recursive_replay() {
        let mut builder = ExecutionPlanBuilder::new();
        builder
            .intern_recursive_node(KernelFamily::MovingAverage, "ema:10", vec![])
            .unwrap();
        let session = RuntimeSession::new(builder.build().unwrap());
        let checkpoint = session.checkpoint();
        assert!(matches!(
            session.schedule_dirty_from_checkpoint(&checkpoint, DirtyRange::new(5, 6), 10),
            Err(RuntimeSessionError::CheckpointHasNoRowBoundary)
        ));
    }
}
