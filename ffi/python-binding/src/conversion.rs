//! Conversion helpers between numpy arrays and Rust vectors.

use super::*;

#[cfg(feature = "formula")]
pub fn extract_array_bound(obj: &Bound<'_, PyAny>) -> PyResult<Vec<f64>> {
    if let Ok(array) = obj.extract::<PyReadonlyArray1<'_, f64>>() {
        return array
            .as_slice()
            .map(|slice| slice.to_vec())
            .map_err(|error| {
                PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                    "Expected a contiguous one-dimensional float64 NumPy array: {error}"
                ))
            });
    }
    if let Ok(py_list) = obj.cast::<pyo3::types::PyList>() {
        let vec: Vec<f64> = py_list
            .iter()
            .map(|item| item.extract::<f64>())
            .collect::<PyResult<Vec<f64>>>()?;
        return Ok(vec);
    }
    if let Ok(py_tuple) = obj.cast::<pyo3::types::PyTuple>() {
        let vec: Vec<f64> = py_tuple
            .iter()
            .map(|item| item.extract::<f64>())
            .collect::<PyResult<Vec<f64>>>()?;
        return Ok(vec);
    }
    Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
        "Expected a one-dimensional float64 NumPy array, list, or tuple of floats",
    ))
}

#[cfg(feature = "formula")]
pub fn extract_array_pyobject(obj: Py<PyAny>) -> PyResult<Vec<f64>> {
    Python::attach(|py| {
        if let Ok(array) = obj.bind(py).extract::<PyReadonlyArray1<'_, f64>>() {
            return array
                .as_slice()
                .map(|slice| slice.to_vec())
                .map_err(|error| {
                    PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                        "Expected a contiguous one-dimensional float64 NumPy array: {error}"
                    ))
                });
        }
        if let Ok(py_list) = obj.cast_bound::<pyo3::types::PyList>(py) {
            let vec: Vec<f64> = py_list
                .iter()
                .map(|item| item.extract::<f64>())
                .collect::<PyResult<Vec<f64>>>()?;
            return Ok(vec);
        }
        if let Ok(py_tuple) = obj.cast_bound::<pyo3::types::PyTuple>(py) {
            let vec: Vec<f64> = py_tuple
                .iter()
                .map(|item| item.extract::<f64>())
                .collect::<PyResult<Vec<f64>>>()?;
            return Ok(vec);
        }
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Expected a one-dimensional float64 NumPy array, list, or tuple of floats",
        ))
    })
}

#[cfg(feature = "formula")]
pub fn formula_error_to_pyerr(e: FormulaError) -> PyErr {
    match e {
        FormulaError::ParseError(msg) => {
            PyErr::new::<pyo3::exceptions::PySyntaxError, _>(format!("Parse error: {}", msg))
        }
        FormulaError::Parse { line, col, message } => {
            PyErr::new::<pyo3::exceptions::PySyntaxError, _>(format!(
                "Parse error at line {}, col {}: {}",
                line, col, message
            ))
        }
        FormulaError::UndefinedFunction { name } => {
            PyErr::new::<pyo3::exceptions::PyNameError, _>(format!("Undefined function: {}", name))
        }
        FormulaError::TypeMismatch { expected, actual } => {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "Type mismatch: expected {}, got {}",
                expected, actual
            ))
        }
        FormulaError::RuntimeError(msg) => {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Runtime error: {}", msg))
        }
        FormulaError::InvalidParameter(msg) => {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid parameter: {}", msg))
        }
        FormulaError::InsufficientData(msg) => {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Insufficient data: {}", msg))
        }
        FormulaError::InvalidOperation(msg) => {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid operation: {}", msg))
        }
        FormulaError::UnsupportedFunction(msg) => {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Unsupported function: {}",
                msg
            ))
        }
        FormulaError::BackendUnsupported { backend, entry } => {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "`{}` is not available on the {} backend",
                entry, backend
            ))
        }
        FormulaError::Timeout { elapsed_ms } => PyErr::new::<pyo3::exceptions::PyTimeoutError, _>(
            format!("Execution timeout after {}ms", elapsed_ms),
        ),
        FormulaError::MemoryLimit { used, limit } => {
            PyErr::new::<pyo3::exceptions::PyMemoryError, _>(format!(
                "Memory limit exceeded: used {} bytes, limit is {} bytes",
                used, limit
            ))
        }
    }
}

pub fn py_array_f64<'py, F>(py: Python<'py>, calculate: F) -> PyResult<Py<PyArray1<f64>>>
where
    F: Ungil + FnOnce() -> PyResult<Vec<f64>>,
{
    let values = py.detach(calculate)?;
    Ok(PyArray1::from_vec(py, values).unbind())
}

pub fn py_arrays2_f64<'py, F>(
    py: Python<'py>,
    calculate: F,
) -> PyResult<(Py<PyArray1<f64>>, Py<PyArray1<f64>>)>
where
    F: Ungil + FnOnce() -> PyResult<(Vec<f64>, Vec<f64>)>,
{
    let (first, second) = py.detach(calculate)?;
    Ok((
        PyArray1::from_vec(py, first).unbind(),
        PyArray1::from_vec(py, second).unbind(),
    ))
}

pub fn py_arrays3_f64<'py, F>(
    py: Python<'py>,
    calculate: F,
) -> PyResult<(Py<PyArray1<f64>>, Py<PyArray1<f64>>, Py<PyArray1<f64>>)>
where
    F: Ungil + FnOnce() -> PyResult<(Vec<f64>, Vec<f64>, Vec<f64>)>,
{
    let (first, second, third) = py.detach(calculate)?;
    Ok((
        PyArray1::from_vec(py, first).unbind(),
        PyArray1::from_vec(py, second).unbind(),
        PyArray1::from_vec(py, third).unbind(),
    ))
}

pub fn py_arrays4_f64<'py, F>(
    py: Python<'py>,
    calculate: F,
) -> PyResult<(
    Py<PyArray1<f64>>,
    Py<PyArray1<f64>>,
    Py<PyArray1<f64>>,
    Py<PyArray1<f64>>,
)>
where
    F: Ungil + FnOnce() -> PyResult<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)>,
{
    let (first, second, third, fourth) = py.detach(calculate)?;
    Ok((
        PyArray1::from_vec(py, first).unbind(),
        PyArray1::from_vec(py, second).unbind(),
        PyArray1::from_vec(py, third).unbind(),
        PyArray1::from_vec(py, fourth).unbind(),
    ))
}

pub fn py_array_f64_i32<'py, F>(
    py: Python<'py>,
    calculate: F,
) -> PyResult<(Py<PyArray1<f64>>, Py<PyArray1<i32>>)>
where
    F: Ungil + FnOnce() -> PyResult<(Vec<f64>, Vec<i32>)>,
{
    let (values, states) = py.detach(calculate)?;
    Ok((
        PyArray1::from_vec(py, values).unbind(),
        PyArray1::from_vec(py, states).unbind(),
    ))
}

pub fn py_arrays2_f64_i32<'py, F>(
    py: Python<'py>,
    calculate: F,
) -> PyResult<(Py<PyArray1<f64>>, Py<PyArray1<f64>>, Py<PyArray1<i32>>)>
where
    F: Ungil + FnOnce() -> PyResult<(Vec<f64>, Vec<f64>, Vec<i32>)>,
{
    let (first, second, states) = py.detach(calculate)?;
    Ok((
        PyArray1::from_vec(py, first).unbind(),
        PyArray1::from_vec(py, second).unbind(),
        PyArray1::from_vec(py, states).unbind(),
    ))
}

pub fn py_array_f64_i32_i32<'py, F>(
    py: Python<'py>,
    calculate: F,
) -> PyResult<(Py<PyArray1<f64>>, Py<PyArray1<i32>>, Py<PyArray1<i32>>)>
where
    F: Ungil + FnOnce() -> PyResult<(Vec<f64>, Vec<i32>, Vec<i32>)>,
{
    let (values, first_state, second_state) = py.detach(calculate)?;
    Ok((
        PyArray1::from_vec(py, values).unbind(),
        PyArray1::from_vec(py, first_state).unbind(),
        PyArray1::from_vec(py, second_state).unbind(),
    ))
}
