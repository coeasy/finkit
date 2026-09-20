//! Execution runtime foundation for factor graphs.
//!
//! # Mixed: partially adopted, mostly superseded
//!
//! Part of the unconverged `crates/finkit-*` migration track -- see
//! `docs/runtime-carrier-adoption-plan-2026-09-20.md`.
//!
//! **This crate does not become the Runtime carrier.** Its [`Executor`] is
//! weaker than `core`'s on every axis: no buffer arena, no kernel dispatch, no
//! range/incremental execution, no NaN or warm-up policy, one dependency per
//! node, and a `Vec<f64>` copy per node -- against an already differential-tested
//! path. Adopting it as the carrier would regress `core`.
//!
//! | Module | Verdict | `core` counterpart |
//! |---|---|---|
//! | `factory` (`FactorProvider`, `FactorFactoryRequest`, typed param errors) | **adopted (R2)** | none -- genuine gap |
//! | `graph` (`FactorGraph`/`FactorNode`) | **adopted (R3)** | none -- lowers onto `ComputeNodeId` |
//! | `cache`, `cache_key` (shape only) | **adopted (R4)** | key re-based onto `operation::OperationCacheKey`; `get_or_compute` + counters wired onto the existing `OperationResultCache` |
//! | `executor` | superseded | `unified_executor::UnifiedExecutor` + `KernelDispatcher` + `BufferArena` |
//! | `scheduler` | superseded | `compute::ComputePlanError::DependencyCycle` (reports the cycle path) |
//! | `registry` | superseded | `factors::FactorRegistry` |
//! | `factories` | superseded | `core`'s indicator set |
//!
//! Two R4 decisions are worth recording here, because the retired code did the
//! opposite:
//!
//! * `FactorCacheKey::time_range: Option<String>` is **not** adopted. A
//!   caller-owned monotonic `data_revision` replaces it, so the key stays O(1) to
//!   compare no matter how a caller spells a range.
//! * **No fourth cache was added.** `get_or_compute` and the hit/miss counters
//!   were wired onto the existing operation cache, whose key already had the
//!   required `dialect` + frame + revision shape. A second result cache would
//!   have re-created the exact double-track this migration is meant to remove.
//!
//! Do not add public API to the superseded modules.

mod cache;
mod cache_key;
mod config;
mod executor;
mod factories;
mod factory;
mod graph;
mod output;
mod registry;
mod scheduler;

pub use cache::FactorCache;
pub use cache_key::FactorCacheKey;
pub use config::FactorConfig;
pub use executor::{Executor, RuntimeError};
pub use factories::{EmaFactory, MacdFactory, RsiFactory, SmaFactory};
pub use factory::{
    DynFactor, FactorFactory, FactorFactoryError, FactorFactoryRequest, FactorProvider,
    SimpleFactorFactory,
};
pub use graph::{FactorGraph, FactorNode};
pub use output::FactorOutput;
pub use registry::{FactorRegistry, FactorRegistryError};
pub use scheduler::{Scheduler, SchedulerError};

#[derive(Clone, Debug)]
pub struct ExecutionPlan {
    pub name: String,
    pub nodes: Vec<FactorNode>,
}

impl ExecutionPlan {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
        }
    }

    pub fn from_graph(name: impl Into<String>, graph: &FactorGraph) -> Self {
        Self {
            name: name.into(),
            nodes: graph.nodes.clone(),
        }
    }
}
