//! Shared utilities for Finkit language bindings.
//!
//! This crate is the single home for cross-language FFI concerns so that the
//! bindings no longer re-implement validation, errors, research contracts or
//! quantitative evaluation formulas.
//!
//! * [`error`] — common FFI error semantics.
//! * [`registry`] — canonical indicator registry view.
//! * [`types`] — numeric / array conversion helpers.
//! * [`golden`] — cross-language golden reference vectors.
//! * [`research`] — versioned factor-research and generic quantitative
//!   evaluation JSON contracts. Statistical logic remains in Rust.

pub mod error;
pub mod execute;
pub mod formula;
pub mod golden;
pub mod leak;
pub mod operation;
pub mod panic;
pub mod registry;
pub mod research;
pub mod types;

pub use execute::{
    execute_operation_json, talib_profile_supported, OPERATION_RESULT_SCHEMA_VERSION,
};
pub use formula::{evaluate_formula_json, FORMULA_CONTRACT_SCHEMA_VERSION};
pub use operation::{operation_catalog, operation_catalog_json, OperationCatalogEnvelope};
pub use research::{
    factor_study_error_json, factor_study_json, quant_evaluation_error_json, quant_evaluation_json,
    FactorStudyRequest, FactorStudyResponse, QuantEvaluationApiError, QuantEvaluationApiReport,
    QuantEvaluationRequest, QuantEvaluationResponse, ResearchApiError,
    FACTOR_STUDY_MIN_SCHEMA_VERSION, FACTOR_STUDY_SCHEMA_VERSION, QUANT_EVALUATION_SCHEMA_VERSION,
};
