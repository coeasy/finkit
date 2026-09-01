//! Persistent compiled formula plan exposed to Python.
//!
//! This is the preferred high-throughput formula API.  The formula is parsed
//! and compiled once, while each evaluation only prepares the input context
//! and executes the cached plan.  NumPy inputs are read as contiguous views;
//! the current FormulaContext still owns its execution arrays, so this API
//! removes Python object iteration and returns NumPy arrays without Python
//! scalar boxing.

use ::finkit::formula::{CompiledFormula, FormulaContext, FormulaEngine};
use ndarray::Array1;
use numpy::{PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;

fn read_array<'py>(name: &str, array: PyReadonlyArray1<'py, f64>) -> PyResult<Vec<f64>> {
    array
        .as_slice()
        .map(|slice| slice.to_vec())
        .map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "{name} must be a contiguous one-dimensional float64 NumPy array: {error}"
            ))
        })
}

fn validate_lengths(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    amount: Option<&[f64]>,
) -> PyResult<usize> {
    let expected = close.len();
    if expected == 0 {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "formula inputs must not be empty",
        ));
    }

    let inputs = [
        ("open", open.len()),
        ("high", high.len()),
        ("low", low.len()),
        ("close", close.len()),
        ("volume", volume.len()),
    ];
    for (name, length) in inputs {
        if length != expected {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "formula input {name} has length {length}, expected {expected}"
            )));
        }
    }
    if let Some(length) = amount.map(<[f64]>::len) {
        if length != expected {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "formula input amount has length {length}, expected {expected}"
            )));
        }
    }
    Ok(expected)
}

/// A reusable formula compilation plan.
///
/// Construct this once and call `eval` repeatedly with different NumPy
/// arrays.  The object is intentionally marked unsendable because the current
/// Rust formula executor contains interior mutable per-engine caches.
#[pyclass(name = "CompiledFormula", unsendable)]
pub struct PyCompiledFormula {
    source: String,
    engine: Option<FormulaEngine>,
    compiled: Arc<CompiledFormula>,
}

#[pymethods]
impl PyCompiledFormula {
    #[new]
    fn new(source: String) -> PyResult<Self> {
        let mut engine = FormulaEngine::new();
        let compiled = engine
            .compile(&source)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PySyntaxError, _>(error.to_string()))?;
        Ok(Self {
            source,
            engine: Some(engine),
            compiled: Arc::new(compiled),
        })
    }

    #[getter]
    fn source(&self) -> &str {
        &self.source
    }

    /// Evaluate the cached formula using contiguous float64 NumPy arrays.
    #[pyo3(signature = (open, high, low, close, volume, amount=None))]
    #[allow(clippy::too_many_arguments)]
    fn eval<'py>(
        &mut self,
        py: Python<'py>,
        open: PyReadonlyArray1<'py, f64>,
        high: PyReadonlyArray1<'py, f64>,
        low: PyReadonlyArray1<'py, f64>,
        close: PyReadonlyArray1<'py, f64>,
        volume: PyReadonlyArray1<'py, f64>,
        amount: Option<PyReadonlyArray1<'py, f64>>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let open = read_array("open", open)?;
        let high = read_array("high", high)?;
        let low = read_array("low", low)?;
        let close = read_array("close", close)?;
        let volume = read_array("volume", volume)?;
        let amount = amount
            .map(|array| read_array("amount", array))
            .transpose()?;

        validate_lengths(&open, &high, &low, &close, &volume, amount.as_deref())?;

        let mut engine = self.engine.take().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "compiled formula is already being evaluated",
            )
        })?;
        let compiled = Arc::clone(&self.compiled);
        let (execution, variables, engine) = py.detach(move || {
            let mut context = FormulaContext::new(
                Array1::from_vec(open),
                Array1::from_vec(high),
                Array1::from_vec(low),
                Array1::from_vec(close),
                Array1::from_vec(volume),
                amount.map(Array1::from_vec),
            );
            let execution = engine.execute(&compiled, &mut context).map_err(|error| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())
            });
            (execution, context.variables, engine)
        });
        self.engine = Some(engine);
        let result = execution?;

        let output = PyDict::new(py);
        for (name, value) in variables {
            output.set_item(name.as_ref(), PyArray1::from_vec(py, value.into_raw_vec()))?;
        }
        output.set_item("__result__", PyArray1::from_vec(py, result.into_raw_vec()))?;
        Ok(output)
    }

    fn __repr__(&self) -> String {
        format!("CompiledFormula({:?})", self.source)
    }
}
