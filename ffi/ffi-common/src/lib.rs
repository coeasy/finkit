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

pub mod composite;
pub mod composite_stream;
#[cfg(test)]
mod contract_conformance;
pub mod error;
pub mod execute;
pub mod factor;
pub mod factor_catalog;
pub mod factor_stream;
pub mod formula;
pub mod formula_stream;
pub mod golden;
pub mod leak;
pub mod operation;
pub mod panic;
pub mod registry;
pub mod research;
pub mod talib_catalog;
pub mod types;

pub use composite::{evaluate_composite_json, COMPOSITE_CONTRACT_SCHEMA_VERSION};
pub use composite_stream::{
    evaluate_composite_stream_json, COMPOSITE_STREAM_CONTRACT_SCHEMA_VERSION,
};
pub use execute::{
    execute_operation_json, talib_profile_supported, OPERATION_RESULT_SCHEMA_VERSION,
};
pub use factor::{
    evaluate_factor_cross_sectional_json, evaluate_factor_json, FACTOR_CONTRACT_SCHEMA_VERSION,
    FACTOR_CROSS_SECTIONAL_CONTRACT_SCHEMA_VERSION,
};
pub use factor_catalog::{factor_catalog, factor_catalog_json, FACTOR_CATALOG_SCHEMA_VERSION};
pub use factor_stream::{evaluate_factor_stream_json, FACTOR_STREAM_CONTRACT_SCHEMA_VERSION};
pub use formula::{
    evaluate_formula_json, evaluate_formula_panel_json, evaluate_formula_temporal_json,
    formula_compatibility_report_json, FORMULA_COMPATIBILITY_SCHEMA_VERSION,
    FORMULA_CONTRACT_SCHEMA_VERSION, FORMULA_DRAW_CONTRACT_SCHEMA_VERSION,
    FORMULA_PANEL_CONTRACT_SCHEMA_VERSION, FORMULA_TEMPORAL_CONTRACT_SCHEMA_VERSION,
};
pub use formula_stream::{evaluate_formula_stream_json, FORMULA_STREAM_CONTRACT_SCHEMA_VERSION};
pub use operation::{operation_catalog, operation_catalog_json, OperationCatalogEnvelope};
pub use research::{
    factor_study_error_json, factor_study_json, quant_evaluation_error_json, quant_evaluation_json,
    FactorStudyRequest, FactorStudyResponse, QuantEvaluationApiError, QuantEvaluationApiReport,
    QuantEvaluationRequest, QuantEvaluationResponse, ResearchApiError,
    FACTOR_STUDY_MIN_SCHEMA_VERSION, FACTOR_STUDY_SCHEMA_VERSION, QUANT_EVALUATION_SCHEMA_VERSION,
};
pub use talib_catalog::{
    is_profile_catalog_name, TALIB_CORE_VERSION, TALIB_PROFILE_CATALOG_NAMES,
    TALIB_SEMANTIC_PROFILE,
};
