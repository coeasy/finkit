//! Language-neutral factor research and quantitative evaluation bridge.
//!
//! Bindings should delegate to this module instead of reconstructing research
//! validation, performance calculations, weighting, report assembly, or error
//! envelopes in each language.

pub use finkit_factor_analysis::{
    run_factor_study, run_factor_study_json, run_factor_study_response, run_quant_evaluation,
    run_quant_evaluation_json, run_quant_evaluation_response, FactorStudyRequest,
    FactorStudyResponse, QuantEvaluationApiError, QuantEvaluationApiReport, QuantEvaluationRequest,
    QuantEvaluationResponse, ResearchApiError, FACTOR_STUDY_MIN_SCHEMA_VERSION,
    FACTOR_STUDY_SCHEMA_VERSION, QUANT_EVALUATION_SCHEMA_VERSION,
};

/// Run a versioned JSON request through the canonical Rust factor research engine.
#[inline]
pub fn factor_study_json(request_json: &str) -> String {
    run_factor_study_json(request_json)
}

/// Run a versioned generic strategy/portfolio evaluation request.
#[inline]
pub fn quant_evaluation_json(request_json: &str) -> String {
    run_quant_evaluation_json(request_json)
}

/// Build the stable factor-study error envelope used by native language boundaries.
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

/// Build the stable generic evaluation error envelope used by native bindings.
pub fn quant_evaluation_error_json(code: &str, message: &str) -> String {
    serde_json::json!({
        "schema_version": QUANT_EVALUATION_SCHEMA_VERSION,
        "library_version": env!("CARGO_PKG_VERSION"),
        "ok": false,
        "error": {
            "code": code,
            "message": message,
        }
    })
    .to_string()
}
