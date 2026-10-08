//! Directional-movement and variance wrappers exposed to Python.

use super::*;

#[pyfunction]
#[pyo3(signature = (high, low, close, timeperiod=14))]
pub fn vec_dx_impl(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::dx(high, low, close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (high, low, close, timeperiod=14))]
pub fn dx(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_dx_impl(py, high, low, close, timeperiod)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

#[pyfunction]
#[pyo3(signature = (high, low, close, timeperiod=14))]
pub fn vec_minus_di_impl(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::minus_di(high, low, close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (high, low, close, timeperiod=14))]
pub fn minus_di(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_minus_di_impl(py, high, low, close, timeperiod)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

#[pyfunction]
#[pyo3(signature = (high, low))]
pub fn vec_minus_dm_impl(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::minus_dm(high, low)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (high, low))]
pub fn minus_dm(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_minus_dm_impl(py, high, low)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

#[pyfunction]
#[pyo3(signature = (high, low, close, timeperiod=14))]
pub fn vec_plus_di_impl(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::plus_di(high, low, close, timeperiod)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (high, low, close, timeperiod=14))]
pub fn plus_di(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_plus_di_impl(py, high, low, close, timeperiod)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

#[pyfunction]
#[pyo3(signature = (high, low))]
pub fn vec_plus_dm_impl(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::plus_dm(high, low)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (high, low))]
pub fn plus_dm(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_plus_dm_impl(py, high, low)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

#[pyfunction]
#[pyo3(signature = (close, timeperiod=5, nbdev=1.0))]
pub fn vec_var_impl(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
    nbdev: f64,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::var(close, timeperiod, nbdev)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (close, timeperiod=5, nbdev=1.0))]
pub fn var(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
    nbdev: f64,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_var_impl(py, close, timeperiod, nbdev)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}
