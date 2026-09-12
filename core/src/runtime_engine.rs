//! Architecture V4 execution engine.
//!
//! This module is the stateful planning layer that sits above canonical math
//! kernels and below Formula/Factor/Streaming frontends. It is intentionally
//! separate from [`crate::runtime`], which remains the stable market-data and
//! warm-up contract during the migration.

#[path = "runtime/execution_plan.rs"]
pub mod execution_plan;
#[path = "runtime/kernel_adapter.rs"]
pub mod kernel_adapter;
#[path = "runtime/scheduler.rs"]
pub mod scheduler;
#[path = "runtime/state_arena.rs"]
pub mod state_arena;

pub use execution_plan::{
    ExecutionPlan, ExecutionPlanBuilder, ExecutionPlanError, NodeId, PlanNode,
};
pub use kernel_adapter::{
    ExtremaKernelAdapter, KernelAdapterError, KernelExecutor, OhlcInput, StatisticsOutput,
};
pub use scheduler::{ExecutionScheduler, ScheduleError, ScheduledExecution};
pub use state_arena::{StateArena, StateArenaCheckpoint, StateArenaError, StateHandle};
