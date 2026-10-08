//! Thin JSON endpoints for the operation / factor catalogues.

use super::*;

#[pyfunction]
pub fn operation_catalog_json() -> PyResult<String> {
    finkit_ffi_common::operation::operation_catalog_json()
        .map_err(|error| pyo3::exceptions::PyRuntimeError::new_err(error.to_string()))
}

#[pyfunction]
pub fn factor_catalog_json() -> PyResult<String> {
    finkit_ffi_common::factor_catalog::factor_catalog_json()
        .map_err(|error| pyo3::exceptions::PyRuntimeError::new_err(error.to_string()))
}

#[pyfunction]
pub fn operation_execute_json(request_json: &str) -> String {
    finkit_ffi_common::execute_operation_json(request_json)
}

#[pyfunction]
pub fn composite_execute_json(request_json: &str) -> PyResult<String> {
    finkit_ffi_common::evaluate_composite_json(request_json)
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

#[pyfunction]
pub fn composite_stream_execute_json(request_json: &str) -> PyResult<String> {
    finkit_ffi_common::evaluate_composite_stream_json(request_json)
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

#[pyfunction]
pub fn factor_execute_json(request_json: &str) -> PyResult<String> {
    finkit_ffi_common::evaluate_factor_json(request_json)
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

#[pyfunction]
pub fn factor_cross_sectional_execute_json(request_json: &str) -> PyResult<String> {
    finkit_ffi_common::evaluate_factor_cross_sectional_json(request_json)
        .map_err(pyo3::exceptions::PyValueError::new_err)
}

#[pyfunction]
pub fn factor_stream_execute_json(request_json: &str) -> PyResult<String> {
    finkit_ffi_common::evaluate_factor_stream_json(request_json)
        .map_err(pyo3::exceptions::PyValueError::new_err)
}
