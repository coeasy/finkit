//! Python-facing fast paths for Architecture 3.0.
//!
//! Inputs are borrowed NumPy buffers. Vector outputs are built from Rust-owned
//! vectors with `PyArray1::from_vec`, transferring the allocation to NumPy
//! instead of materialising a Python list and copying it again in `np.asarray`.
//! Caller-owned `out=` paths borrow the destination ndarray and execute the same
//! canonical `*_into` kernels without allocating an intermediate output.
//!
//! The generic dispatch entry points below intentionally live outside the
//! registry-generated binding. The registry remains the public API SSOT while
//! the package facade can route performance-critical calls through this direct
//! ndarray transport until the generator itself emits native ndarray results.

use ::finkit::indicators;
use ::finkit::math::{moving_avg, reduction, typed_moving_avg, volume_kernels};
use numpy::{PyArray1, PyReadonlyArray1, PyReadwriteArray1};
use pyo3::prelude::*;

#[inline]
fn value_error(error: impl std::fmt::Display) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(error.to_string())
}

#[inline]
fn unsupported(operation: &str) -> PyErr {
    value_error(format!("unsupported Architecture v3 fast operation: {operation}"))
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

#[pyfunction(name = "_fast_sma_into")]
#[pyo3(signature = (close, output, timeperiod=14))]
fn fast_sma_into(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    mut output: PyReadwriteArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<()> {
    let close = close.as_slice().map_err(value_error)?;
    let output = output.as_slice_mut().map_err(value_error)?;
    py.detach(|| moving_avg::sma_into(close, timeperiod, output))
        .map_err(value_error)
}

#[pyfunction(name = "_fast_sma_f32")]
#[pyo3(signature = (close, timeperiod=14))]
fn fast_sma_f32<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f32>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f32>>> {
    let close = close.as_slice().map_err(value_error)?;
    let mut output = vec![0.0f32; close.len()];
    py.detach(|| typed_moving_avg::sma_f32_into(close, timeperiod, &mut output))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output))
}

#[pyfunction(name = "_fast_sma_f32_into")]
#[pyo3(signature = (close, output, timeperiod=14))]
fn fast_sma_f32_into(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f32>,
    mut output: PyReadwriteArray1<'_, f32>,
    timeperiod: usize,
) -> PyResult<()> {
    let close = close.as_slice().map_err(value_error)?;
    let output = output.as_slice_mut().map_err(value_error)?;
    py.detach(|| typed_moving_avg::sma_f32_into(close, timeperiod, output))
        .map_err(value_error)
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

#[pyfunction(name = "_fast_ema_into")]
#[pyo3(signature = (close, output, timeperiod=14))]
fn fast_ema_into(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    mut output: PyReadwriteArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<()> {
    let close = close.as_slice().map_err(value_error)?;
    let output = output.as_slice_mut().map_err(value_error)?;
    py.detach(|| moving_avg::ema_into(close, timeperiod, output))
        .map_err(value_error)
}

#[pyfunction(name = "_fast_ema_f32")]
#[pyo3(signature = (close, timeperiod=14))]
fn fast_ema_f32<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f32>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f32>>> {
    let close = close.as_slice().map_err(value_error)?;
    let mut output = vec![0.0f32; close.len()];
    py.detach(|| typed_moving_avg::ema_f32_into(close, timeperiod, &mut output))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output))
}

#[pyfunction(name = "_fast_ema_f32_into")]
#[pyo3(signature = (close, output, timeperiod=14))]
fn fast_ema_f32_into(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f32>,
    mut output: PyReadwriteArray1<'_, f32>,
    timeperiod: usize,
) -> PyResult<()> {
    let close = close.as_slice().map_err(value_error)?;
    let output = output.as_slice_mut().map_err(value_error)?;
    py.detach(|| typed_moving_avg::ema_f32_into(close, timeperiod, output))
        .map_err(value_error)
}

#[pyfunction(name = "_fast_wma")]
#[pyo3(signature = (close, timeperiod=14))]
fn fast_wma<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let close = close.as_slice().map_err(value_error)?;
    let mut output = vec![0.0; close.len()];
    py.detach(|| moving_avg::wma_into(close, timeperiod, &mut output))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output))
}

#[pyfunction(name = "_fast_wma_into")]
#[pyo3(signature = (close, output, timeperiod=14))]
fn fast_wma_into(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    mut output: PyReadwriteArray1<'_, f64>,
    timeperiod: usize,
) -> PyResult<()> {
    let close = close.as_slice().map_err(value_error)?;
    let output = output.as_slice_mut().map_err(value_error)?;
    py.detach(|| moving_avg::wma_into(close, timeperiod, output))
        .map_err(value_error)
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

#[pyfunction(name = "_fast_obv_into")]
fn fast_obv_into(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    mut output: PyReadwriteArray1<'_, f64>,
) -> PyResult<()> {
    let close = close.as_slice().map_err(value_error)?;
    let volume = volume.as_slice().map_err(value_error)?;
    let output = output.as_slice_mut().map_err(value_error)?;
    py.detach(|| volume_kernels::obv_into(close, volume, output))
        .map_err(value_error)
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

#[pyfunction(name = "_fast_vwap_into")]
fn fast_vwap_into(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    mut output: PyReadwriteArray1<'_, f64>,
) -> PyResult<()> {
    let high = high.as_slice().map_err(value_error)?;
    let low = low.as_slice().map_err(value_error)?;
    let close = close.as_slice().map_err(value_error)?;
    let volume = volume.as_slice().map_err(value_error)?;
    let output = output.as_slice_mut().map_err(value_error)?;
    py.detach(|| volume_kernels::vwap_into(high, low, close, volume, output))
        .map_err(value_error)
}

/// Direct-ndarray transport for one-input period indicators.
#[pyfunction(name = "_fast_unary_period")]
fn fast_unary_period<'py>(
    py: Python<'py>,
    operation: &str,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let close = close.as_slice().map_err(value_error)?;
    let output = py
        .detach(|| match operation {
            "dema" => moving_avg::dema(close, timeperiod),
            "tema" => moving_avg::tema(close, timeperiod),
            "midpoint" => indicators::midpoint(close, timeperiod),
            "rsi" => indicators::rsi(close, timeperiod),
            "mom" => indicators::mom(close, timeperiod),
            "roc" => indicators::roc(close, timeperiod),
            "cmo" => indicators::cmo(close, timeperiod),
            _ => return Err(::finkit::error::TaError::InvalidParameter {
                name: "operation".to_string(),
                constraint: format!("unsupported fast operation: {operation}"),
            }),
        })
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output.into_raw_vec()))
}

/// Direct-ndarray transport for one-input period indicators with a scale parameter.
#[pyfunction(name = "_fast_unary_period_scale")]
fn fast_unary_period_scale<'py>(
    py: Python<'py>,
    operation: &str,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
    scale: f64,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let close = close.as_slice().map_err(value_error)?;
    let output = py
        .detach(|| match operation {
            "stddev" => indicators::std_dev(close, timeperiod, scale),
            "var" => indicators::var(close, timeperiod, scale),
            _ => return Err(::finkit::error::TaError::InvalidParameter {
                name: "operation".to_string(),
                constraint: format!("unsupported fast operation: {operation}"),
            }),
        })
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output.into_raw_vec()))
}

#[pyfunction(name = "_fast_kama")]
#[pyo3(signature = (close, timeperiod=10, fastperiod=2, slowperiod=30))]
fn fast_kama<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
    fastperiod: usize,
    slowperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let close = close.as_slice().map_err(value_error)?;
    let output = py
        .detach(|| moving_avg::kama(close, timeperiod, fastperiod, slowperiod))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output.into_raw_vec()))
}

/// Direct-ndarray transport for two-input rolling indicators.
#[pyfunction(name = "_fast_binary_period")]
fn fast_binary_period<'py>(
    py: Python<'py>,
    operation: &str,
    input_a: PyReadonlyArray1<'py, f64>,
    input_b: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let input_a = input_a.as_slice().map_err(value_error)?;
    let input_b = input_b.as_slice().map_err(value_error)?;
    let output = py
        .detach(|| match operation {
            "midprice" => indicators::midprice(input_a, input_b, timeperiod),
            "correl" => indicators::correlation(input_a, input_b, timeperiod),
            _ => return Err(::finkit::error::TaError::InvalidParameter {
                name: "operation".to_string(),
                constraint: format!("unsupported fast operation: {operation}"),
            }),
        })
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output.into_raw_vec()))
}

/// Direct-ndarray transport for HLC period indicators.
#[pyfunction(name = "_fast_hlc_period")]
fn fast_hlc_period<'py>(
    py: Python<'py>,
    operation: &str,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let high = high.as_slice().map_err(value_error)?;
    let low = low.as_slice().map_err(value_error)?;
    let close = close.as_slice().map_err(value_error)?;
    let output = py
        .detach(|| match operation {
            "adx" => indicators::adx(high, low, close, timeperiod),
            "cci" => indicators::cci(high, low, close, timeperiod),
            "willr" => indicators::willr(high, low, close, timeperiod),
            "plus_di" => indicators::plus_di(high, low, close, timeperiod),
            "minus_di" => indicators::minus_di(high, low, close, timeperiod),
            "atr" => indicators::atr(high, low, close, timeperiod),
            "natr" => indicators::natr(high, low, close, timeperiod),
            _ => return Err(::finkit::error::TaError::InvalidParameter {
                name: "operation".to_string(),
                constraint: format!("unsupported fast operation: {operation}"),
            }),
        })
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output.into_raw_vec()))
}

#[pyfunction(name = "_fast_trange")]
fn fast_trange<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let high = high.as_slice().map_err(value_error)?;
    let low = low.as_slice().map_err(value_error)?;
    let close = close.as_slice().map_err(value_error)?;
    let output = py
        .detach(|| indicators::trange(high, low, close))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output.into_raw_vec()))
}

#[pyfunction(name = "_fast_mfi")]
#[pyo3(signature = (high, low, close, volume, timeperiod=14))]
fn fast_mfi<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    volume: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let high = high.as_slice().map_err(value_error)?;
    let low = low.as_slice().map_err(value_error)?;
    let close = close.as_slice().map_err(value_error)?;
    let volume = volume.as_slice().map_err(value_error)?;
    let output = py
        .detach(|| indicators::mfi(high, low, close, volume, timeperiod))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output.into_raw_vec()))
}

#[pyfunction(name = "_fast_ad")]
fn fast_ad<'py>(
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
    let output = py
        .detach(|| indicators::ad(high, low, close, volume))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output.into_raw_vec()))
}

#[pyfunction(name = "_fast_adosc")]
#[pyo3(signature = (high, low, close, volume, fastperiod=3, slowperiod=10))]
fn fast_adosc<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    volume: PyReadonlyArray1<'py, f64>,
    fastperiod: usize,
    slowperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let high = high.as_slice().map_err(value_error)?;
    let low = low.as_slice().map_err(value_error)?;
    let close = close.as_slice().map_err(value_error)?;
    let volume = volume.as_slice().map_err(value_error)?;
    let output = py
        .detach(|| indicators::adosc(high, low, close, volume, fastperiod, slowperiod))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output.into_raw_vec()))
}

#[pyfunction(name = "_fast_bop")]
fn fast_bop<'py>(
    py: Python<'py>,
    open: PyReadonlyArray1<'py, f64>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let open = open.as_slice().map_err(value_error)?;
    let high = high.as_slice().map_err(value_error)?;
    let low = low.as_slice().map_err(value_error)?;
    let close = close.as_slice().map_err(value_error)?;
    let output = py
        .detach(|| indicators::bop(open, high, low, close))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output.into_raw_vec()))
}

#[pyfunction(name = "_fast_bbands")]
#[pyo3(signature = (close, timeperiod=20, nbdevup=2.0, nbdevdn=2.0))]
fn fast_bbands<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
    nbdevup: f64,
    nbdevdn: f64,
) -> PyResult<(
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
)> {
    let close = close.as_slice().map_err(value_error)?;
    let result = py
        .detach(|| indicators::bbands(close, timeperiod, nbdevup, nbdevdn))
        .map_err(value_error)?;
    Ok((
        PyArray1::from_vec(py, result.upper.into_raw_vec()),
        PyArray1::from_vec(py, result.middle.into_raw_vec()),
        PyArray1::from_vec(py, result.lower.into_raw_vec()),
    ))
}

#[pyfunction(name = "_fast_sar")]
#[pyo3(signature = (high, low, acceleration=0.02, maximum=0.2))]
fn fast_sar<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    acceleration: f64,
    maximum: f64,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let high = high.as_slice().map_err(value_error)?;
    let low = low.as_slice().map_err(value_error)?;
    let result = py
        .detach(|| indicators::sar(high, low, acceleration, maximum))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, result.sar.into_raw_vec()))
}

#[pyfunction(name = "_fast_macd")]
#[pyo3(signature = (close, fastperiod=12, slowperiod=26, signalperiod=9))]
fn fast_macd<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    fastperiod: usize,
    slowperiod: usize,
    signalperiod: usize,
) -> PyResult<(
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
)> {
    let close = close.as_slice().map_err(value_error)?;
    let result = py
        .detach(|| indicators::macd(close, fastperiod, slowperiod, signalperiod))
        .map_err(value_error)?;
    Ok((
        PyArray1::from_vec(py, result.macd.into_raw_vec()),
        PyArray1::from_vec(py, result.signal.into_raw_vec()),
        PyArray1::from_vec(py, result.hist.into_raw_vec()),
    ))
}

#[pyfunction(name = "_fast_stoch")]
#[pyo3(signature = (high, low, close, fastk_period=5, slowk_period=3, slowd_period=3))]
fn fast_stoch<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    fastk_period: usize,
    slowk_period: usize,
    slowd_period: usize,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let high = high.as_slice().map_err(value_error)?;
    let low = low.as_slice().map_err(value_error)?;
    let close = close.as_slice().map_err(value_error)?;
    let result = py
        .detach(|| indicators::stoch(high, low, close, fastk_period, slowk_period, slowd_period))
        .map_err(value_error)?;
    Ok((
        PyArray1::from_vec(py, result.k.into_raw_vec()),
        PyArray1::from_vec(py, result.d.into_raw_vec()),
    ))
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
    m.add_function(wrap_pyfunction!(fast_sma_into, m)?)?;
    m.add_function(wrap_pyfunction!(fast_sma_f32, m)?)?;
    m.add_function(wrap_pyfunction!(fast_sma_f32_into, m)?)?;
    m.add_function(wrap_pyfunction!(fast_ema, m)?)?;
    m.add_function(wrap_pyfunction!(fast_ema_into, m)?)?;
    m.add_function(wrap_pyfunction!(fast_ema_f32, m)?)?;
    m.add_function(wrap_pyfunction!(fast_ema_f32_into, m)?)?;
    m.add_function(wrap_pyfunction!(fast_wma, m)?)?;
    m.add_function(wrap_pyfunction!(fast_wma_into, m)?)?;
    m.add_function(wrap_pyfunction!(fast_obv, m)?)?;
    m.add_function(wrap_pyfunction!(fast_obv_into, m)?)?;
    m.add_function(wrap_pyfunction!(fast_vwap, m)?)?;
    m.add_function(wrap_pyfunction!(fast_vwap_into, m)?)?;
    m.add_function(wrap_pyfunction!(fast_unary_period, m)?)?;
    m.add_function(wrap_pyfunction!(fast_unary_period_scale, m)?)?;
    m.add_function(wrap_pyfunction!(fast_kama, m)?)?;
    m.add_function(wrap_pyfunction!(fast_binary_period, m)?)?;
    m.add_function(wrap_pyfunction!(fast_hlc_period, m)?)?;
    m.add_function(wrap_pyfunction!(fast_trange, m)?)?;
    m.add_function(wrap_pyfunction!(fast_mfi, m)?)?;
    m.add_function(wrap_pyfunction!(fast_ad, m)?)?;
    m.add_function(wrap_pyfunction!(fast_adosc, m)?)?;
    m.add_function(wrap_pyfunction!(fast_bop, m)?)?;
    m.add_function(wrap_pyfunction!(fast_bbands, m)?)?;
    m.add_function(wrap_pyfunction!(fast_sar, m)?)?;
    m.add_function(wrap_pyfunction!(fast_macd, m)?)?;
    m.add_function(wrap_pyfunction!(fast_stoch, m)?)?;
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
