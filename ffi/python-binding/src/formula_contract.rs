//! JSON contract endpoints for the formula subsystem.

use super::*;

#[pyfunction]
#[pyo3(signature = (source, dialect, open, high, low, close, volume))]
#[cfg(feature = "formula")]
#[allow(clippy::too_many_arguments)]
pub fn formula_eval_contract_json(
    source: &str,
    dialect: &str,
    open: &Bound<'_, PyAny>,
    high: &Bound<'_, PyAny>,
    low: &Bound<'_, PyAny>,
    close: &Bound<'_, PyAny>,
    volume: &Bound<'_, PyAny>,
) -> PyResult<String> {
    let open = extract_array_bound(open)?;
    let high = extract_array_bound(high)?;
    let low = extract_array_bound(low)?;
    let close = extract_array_bound(close)?;
    let volume = extract_array_bound(volume)?;
    finkit_ffi_common::evaluate_formula_json(source, dialect, &open, &high, &low, &close, &volume)
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_eval_temporal_contract_json(request_json: &str) -> PyResult<String> {
    finkit_ffi_common::evaluate_formula_temporal_json(request_json)
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_eval_panel_contract_json(request_json: &str) -> PyResult<String> {
    finkit_ffi_common::evaluate_formula_panel_json(request_json)
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_eval_cross_sectional_contract_json(request_json: &str) -> PyResult<String> {
    finkit_ffi_common::evaluate_formula_cross_sectional_json(request_json)
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

#[pyfunction]
#[pyo3(signature = (source, terminal = "finkit"))]
#[cfg(feature = "formula")]
pub fn formula_compatibility_report_json(source: &str, terminal: &str) -> PyResult<String> {
    finkit_ffi_common::formula_compatibility_report_json(source, terminal)
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_stream_execute_json(request_json: &str) -> PyResult<String> {
    finkit_ffi_common::evaluate_formula_stream_json(request_json)
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_validate(py: Python<'_>, source: &str) -> PyResult<bool> {
    py.detach(|| match parse_formula(source) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    })
}
