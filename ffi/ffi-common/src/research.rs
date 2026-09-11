//! Language-neutral factor research bridge.
//!
//! Bindings should delegate to this module instead of reconstructing research
//! validation, quantization, weighting, report assembly, or error envelopes in
//! each language.

pub use finkit_factor_analysis::{
    run_factor_study, run_factor_study_json, run_factor_study_response, FactorStudyRequest,
    FactorStudyResponse, ResearchApiError, FACTOR_STUDY_MIN_SCHEMA_VERSION,
    FACTOR_STUDY_SCHEMA_VERSION,
};

/// Run a versioned JSON request through the canonical Rust research engine.
#[inline]
pub fn factor_study_json(request_json: &str) -> String {
    run_factor_study_json(request_json)
}

/// Build the stable error envelope used by native language boundaries.
pub fn factor_study_error_json(code: &str, message: &str) -> String {
    serde_json::json!({
        "schema_version": FACTOR_STUDY_SCHEMA_VERSION,
        "library_version": env!("CARGO_PKG_VERSION"),
        "ok": false,
        "error": {
            "code": code,
            "message": message,
        }
    })
    .to_string()
}