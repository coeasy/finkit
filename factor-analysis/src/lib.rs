//! Finkit factor research layer.
//!
//! Composes the existing Finkit compute, feature, calendar, risk, and numerical
//! kernels into panel-aware factor research workflows without duplicating core algorithms.

pub mod analysis;
pub mod api;
pub mod artifacts;
#[deprecated(
    note = "use artifacts::ResearchArtifactStore and MaterializationKey; cache is a compatibility-only revision cache"
)]
pub mod cache;
pub mod compat;
pub mod context;
pub mod data;
pub mod error;
pub mod evaluation_api;
pub mod event;
pub mod execution_context;
pub mod executor;
pub mod factor_metrics;
pub mod incremental;
pub mod mining;
pub mod multifactor;
pub mod orchestration;
pub mod performance;
pub mod policy;
pub mod portfolio;
pub mod portfolio_performance;
pub mod prepare;
pub mod report;
pub mod risk_model;
pub mod scenario;
pub mod session;
pub mod stability;
pub mod validation;

pub use analysis::*;
pub use api::*;
pub use artifacts::*;
pub use context::*;
pub use data::*;
pub use error::*;
pub use evaluation_api::*;
pub use executor::*;
pub use factor_metrics::*;
pub use performance::*;
pub use policy::*;
pub use portfolio_performance::*;
pub use prepare::*;
pub use report::*;
pub use session::*;
