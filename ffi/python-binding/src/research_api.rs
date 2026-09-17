use pyo3::prelude::*;

/// Run the canonical panel-aware factor research API.
#[pyfunction]
fn factor_study_json(request_json: &str) -> String {
    finkit_factor_analysis::run_factor_study_json(request_json)
}

/// Evaluate arbitrary strategy/benchmark/trade/portfolio returns using the
/// same canonical metrics as Rust and all other language bindings.
#[pyfunction]
fn quant_evaluation_json(request_json: &str) -> String {
    finkit_factor_analysis::run_quant_evaluation_json(request_json)
}

pub fn register_research_api(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(factor_study_json, module)?)?;
    module.add_function(wrap_pyfunction!(quant_evaluation_json, module)?)?;
    Ok(())
}
