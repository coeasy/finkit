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
//! | `cache`, `cache_key` (shape only) | **adopted (R4)** | re-base onto `operation::OperationCacheKey` |
//! | `executor` | superseded | `unified_executor::UnifiedExecutor` + `KernelDispatcher` + `BufferArena` |
//! | `scheduler` | superseded | `compute::ComputePlanError::DependencyCycle` (reports the cycle path) |
//! | `registry` | superseded | `factors::FactorRegistry` |
//! | `factories` | superseded | `core`'s indicator set |
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
