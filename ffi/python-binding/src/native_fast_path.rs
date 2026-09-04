//! Python-facing fast paths for Architecture 3.0.
//!
//! Inputs are borrowed NumPy buffers.  Vector outputs are built from a Rust Vec with
//! `PyArray1::from_vec`, which transfers ownership of that allocation to NumPy instead
//! of materialising an intermediate Python list and then copying through `np.asarray`.

use ::finkit::math::{moving_avg, reduction, volume_kernels};
use numpy::{PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;

#[inline]
fn value_error(error: impl std::fmt::Display) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(error.to_string())
}

#[pyfunction(name = "_fast_sma")]
#[pyo3(signature = (close, timeperiod=14))]
fn fast_sma<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let close = close.as_slice().map_err(value_error)?;
    let mut output = vec![0.0; close.len()];
    py.detach(|| moving_avg::sma_into(close, timeperiod, &mut output))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output))
}

#[pyfunction(name = "_fast_ema")]
#[pyo3(signature = (close, timeperiod=14))]
fn fast_ema<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let close = close.as_slice().map_err(value_error)?;
    let mut output = vec![0.0; close.len()];
    py.detach(|| moving_avg::ema_into(close, timeperiod, &mut output))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output))
}

#[pyfunction(name = "_fast_obv")]
fn fast_obv<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    volume: PyReadonlyArray1<'py, f64>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let close = close.as_slice().map_err(value_error)?;
    let volume = volume.as_slice().map_err(value_error)?;
    let mut output = vec![0.0; close.len()];
    py.detach(|| volume_kernels::obv_into(close, volume, &mut output))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output))
}

#[pyfunction(name = "_fast_vwap")]
fn fast_vwap<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    volume: PyReadonlyArray1<'py, f64>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let high = high.as_slice().map_err(value_error)?;
    let low = low.as_slice().map_err(value_error)?;
    let close = close.as_slice().map_err(value_error)?;
    let volume = volume.as_slice().map_err(value_error)?;
    let mut output = vec![0.0; high.len()];
    py.detach(|| volume_kernels::vwap_into(high, low, close, volume, &mut output))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output))
}

#[pyfunction(name = "_reduce_sum_f64")]
fn reduce_sum_f64(data: PyReadonlyArray1<'_, f64>) -> PyResult<f64> {
    let data = data.as_slice().map_err(value_error)?;
    if data.is_empty() {
        return Err(value_error("input data is empty"));
    }
    Ok(reduction::sum_f64(data))
}

#[pyfunction(name = "_reduce_mean_f64")]
fn reduce_mean_f64(data: PyReadonlyArray1<'_, f64>) -> PyResult<f64> {
    let data = data.as_slice().map_err(value_error)?;
    if data.is_empty() {
        return Err(value_error("input data is empty"));
    }
    Ok(reduction::mean_f64(data))
}

#[pyfunction(name = "_reduce_min_f64")]
fn reduce_min_f64(data: PyReadonlyArray1<'_, f64>) -> PyResult<f64> {
    let data = data.as_slice().map_err(value_error)?;
    if data.is_empty() {
        return Err(value_error("input data is empty"));
    }
    Ok(reduction::min_f64(data))
}

#[pyfunction(name = "_reduce_max_f64")]
fn reduce_max_f64(data: PyReadonlyArray1<'_, f64>) -> PyResult<f64> {
    let data = data.as_slice().map_err(value_error)?;
    if data.is_empty() {
        return Err(value_error("input data is empty"));
    }
    Ok(reduction::max_f64(data))
}

#[pyfunction(name = "_reduce_stddev_f64")]
fn reduce_stddev_f64(data: PyReadonlyArray1<'_, f64>) -> PyResult<f64> {
    let data = data.as_slice().map_err(value_error)?;
    if data.is_empty() {
        return Err(value_error("input data is empty"));
    }
    Ok(reduction::stddev_f64(data))
}

#[pyfunction(name = "_reduce_sum_f32")]
fn reduce_sum_f32(data: PyReadonlyArray1<'_, f32>) -> PyResult<f32> {
    let data = data.as_slice().map_err(value_error)?;
    if data.is_empty() {
        return Err(value_error("input data is empty"));
    }
    Ok(reduction::sum_f32(data))
}

#[pyfunction(name = "_reduce_mean_f32")]
fn reduce_mean_f32(data: PyReadonlyArray1<'_, f32>) -> PyResult<f32> {
    let data = data.as_slice().map_err(value_error)?;
    if data.is_empty() {
        return Err(value_error("input data is empty"));
    }
    Ok(reduction::mean_f32(data))
}

#[pyfunction(name = "_reduce_min_f32")]
fn reduce_min_f32(data: PyReadonlyArray1<'_, f32>) -> PyResult<f32> {
    let data = data.as_slice().map_err(value_error)?;
    if data.is_empty() {
        return Err(value_error("input data is empty"));
    }
    Ok(reduction::min_f32(data))
}

#[pyfunction(name = "_reduce_max_f32")]
fn reduce_max_f32(data: PyReadonlyArray1<'_, f32>) -> PyResult<f32> {
    let data = data.as_slice().map_err(value_error)?;
    if data.is_empty() {
        return Err(value_error("input data is empty"));
    }
    Ok(reduction::max_f32(data))
}

#[pyfunction(name = "_reduce_stddev_f32")]
fn reduce_stddev_f32(data: PyReadonlyArray1<'_, f32>) -> PyResult<f32> {
    let data = data.as_slice().map_err(value_error)?;
    if data.is_empty() {
        return Err(value_error("input data is empty"));
    }
    Ok(reduction::stddev_f32(data))
}

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(fast_sma, m)?)?;
    m.add_function(wrap_pyfunction!(fast_ema, m)?)?;
    m.add_function(wrap_pyfunction!(fast_obv, m)?)?;
    m.add_function(wrap_pyfunction!(fast_vwap, m)?)?;
    m.add_function(wrap_pyfunction!(reduce_sum_f64, m)?)?;
    m.add_function(wrap_pyfunction!(reduce_mean_f64, m)?)?;
    m.add_function(wrap_pyfunction!(reduce_min_f64, m)?)?;
    m.add_function(wrap_pyfunction!(reduce_max_f64, m)?)?;
    m.add_function(wrap_pyfunction!(reduce_stddev_f64, m)?)?;
    m.add_function(wrap_pyfunction!(reduce_sum_f32, m)?)?;
    m.add_function(wrap_pyfunction!(reduce_mean_f32, m)?)?;
    m.add_function(wrap_pyfunction!(reduce_min_f32, m)?)?;
    m.add_function(wrap_pyfunction!(reduce_max_f32, m)?)?;
    m.add_function(wrap_pyfunction!(reduce_stddev_f32, m)?)?;
    Ok(())
}
