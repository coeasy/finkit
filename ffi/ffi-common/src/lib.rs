//! Shared utilities for Finkit language bindings.
//!
//! This crate is the single home for cross-language FFI concerns so that the
//! eight bindings (C / Python / Node / Go / .NET / Java / iOS / Android) no
//! longer each re-implement the same boilerplate:
//!
//! * [`error`] — a common [`error::FfiError`] type and mapping helpers shared
//!   by every binding, so the *meaning* of a failure is consistent across
//!   Python / Node / Java / .NET / Go / C.
//! * [`registry`] — a typed view of `docs/indicator_registry.json`, the
//!   canonical single source of truth that also drives
//!   `scripts/gen_ssot_docs.py`. Embedding it at compile time lets bindings
//!   and the code generator read the exact same list the docs are built from,
//!   with no runtime file dependency.
//! * [`types`] — numeric / array conversion helpers to keep each binding's
//!   glue minimal.
//! * [`golden`] — cross-language golden reference vectors and a comparator,
//!   so every binding's test suite asserts against the same canonical values.
//! * [`research`] — the versioned JSON-in/JSON-out factor-research contract
//!   shared by every language binding. Statistical logic remains in Rust.

pub mod error;
pub mod golden;
pub mod leak;
pub mod panic;
pub mod registry;
pub mod research;
pub mod types;

pub use research::{
    factor_study_error_json, factor_study_json, FactorStudyRequest, FactorStudyResponse,
    ResearchApiError, FACTOR_STUDY_MIN_SCHEMA_VERSION, FACTOR_STUDY_SCHEMA_VERSION,
};