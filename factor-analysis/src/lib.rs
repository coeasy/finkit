//! Finkit factor research layer.
//!
//! Composes the existing Finkit compute, feature, calendar, risk, and numerical
//! kernels into panel-aware factor research workflows without duplicating core algorithms.

pub mod analysis;
pub mod api;
pub mod cache;
pub mod compat;
pub mod data;
pub mod error;
pub mod event;
pub mod incremental;
pub mod mining;
pub mod multifactor;
pub mod orchestration;
pub mod portfolio;
pub mod prepare;
pub mod report;
pub mod risk_model;
pub mod scenario;
pub mod stability;
pub mod validation;

pub use analysis::*;
pub use api::*;
pub use data::*;
pub use error::*;
pub use prepare::*;
pub use report::*;
