//! Language-neutral factor research bridge.
//!
//! Bindings should delegate to this module instead of reconstructing research
//! validation, quantization, weighting, or report assembly in each language.

pub use finkit_factor_analysis::{
    run_factor_study, run_factor_study_json, run_factor_study_response, FactorStudyRequest,
    FactorStudyResponse, ResearchApiError, FACTOR_STUDY_SCHEMA_VERSION,
};

/// Run a versioned JSON request through the canonical Rust research engine.
#[inline]
pub fn factor_study_json(request_json: &str) -> String {
    run_factor_study_json(request_json)
}
