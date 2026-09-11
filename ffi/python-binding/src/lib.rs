#![allow(missing_docs)]
#![allow(missing_debug_implementations)]
// `into_raw_vec` is deprecated in ndarray 0.16 in favour of
// `into_raw_vec_and_offset`, but the offset is always 0 for 1D arrays and
// rewriting the 100+ call sites in the FFI surface is noisy. Suppress here
// so the deprecation churn stays contained to the `core` crate.
#![allow(deprecated)]

use ::finkit::calendar::{MarketCalendarPreset, SessionWindow, TimeZoneSpec, TradingCalendar};
use ::finkit::chan::{
    analyze as analyze_chan, CenterPolicy, ChanConfig, ChanVariant, FractalKind, FractalPolicy,
    StrokePolicy,
};
use ::finkit::chan_mtf::{
    analyze_multi as analyze_chan_multi,
    analyze_multi_timestamps_with_calendar as analyze_chan_multi_timestamps_calendar,
    analyze_multi_timestamps_with_origin as analyze_chan_multi_timestamps, ChanMultiConfig,
};
use ::finkit::composite::{CompositeDefinition, CompositeEngine, CompositeExpr, CompositeOp};
use ::finkit::factors::FactorContext;
use ::finkit::indicators;
use ::finkit::indicators::PivotMethod;
use ::finkit::math::moving_avg;
use ::finkit::patterns::{candlestick, chart};
use finkit_visualization::chart::EventMarker;
use finkit_visualization::config::{
    ChartConfig, ChartConfigBuilder, IndicatorConfig, IndicatorType,
};
use finkit_visualization::data::KlineData;
use finkit_visualization::error::VisualizationError;
use finkit_visualization::interaction::ReplayState;
use finkit_visualization::language::Language;
use finkit_visualization::primitive::Color;
use finkit_visualization::viewport::{LodLevel, LodPolicy, Viewport};
#[cfg(feature = "formula")]
use formula_plan::PyCompiledFormula;
use numpy::{PyArray1, PyReadonlyArray1};
use pyo3::marker::Ungil;
use pyo3::prelude::*;

mod features;
#[cfg(feature = "formula")]
mod formula_plan;
mod streaming;
mod sweep;
mod transforms;

#[cfg(feature = "formula")]
use ::finkit::formula::{
    parse_formula, parse_formula_with_dialect, FormulaContext, FormulaDialect, FormulaEngine,
    FormulaError,
};
#[cfg(feature = "formula")]
use ndarray::Array1;

#[cfg(feature = "formula")]
fn extract_array_bound(obj: &Bound<'_, PyAny>) -> PyResult<Vec<f64>> {
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
fn extract_array_pyobject(obj: Py<PyAny>) -> PyResult<Vec<f64>> {
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
fn formula_error_to_pyerr(e: FormulaError) -> PyErr {
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

// ============================================================================
// Overlap Studies
// ============================================================================

/// Convert an owned Rust result into a NumPy array after the expensive core
/// calculation has released the GIL.  Keeping this boundary in one helper
/// prevents the generated indicator bindings from materializing Python lists.
fn py_array_f64<'py, F>(py: Python<'py>, calculate: F) -> PyResult<Py<PyArray1<f64>>>
where
    F: Ungil + FnOnce() -> PyResult<Vec<f64>>,
{
    let values = py.detach(calculate)?;
    Ok(PyArray1::from_vec(py, values).unbind())
}

fn py_arrays2_f64<'py, F>(
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

fn py_arrays3_f64<'py, F>(
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

include!("generated.rs");

// ============================================================================
// Momentum Indicators
// ============================================================================

/// Directional Movement Index (DX)
///
/// Measures trend direction and strength.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (high, low, close, timeperiod=14))]
fn dx(
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

/// Minus Directional Indicator (MINUS_DI)
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (high, low, close, timeperiod=14))]
fn minus_di(
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

/// Minus Directional Movement (MINUS_DM)
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
#[pyfunction]
#[pyo3(signature = (high, low))]
fn minus_dm(
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

/// Plus Directional Indicator (PLUS_DI)
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `timeperiod` - Lookback period (default: 14)
#[pyfunction]
#[pyo3(signature = (high, low, close, timeperiod=14))]
fn plus_di(
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

/// Plus Directional Movement (PLUS_DM)
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
#[pyfunction]
#[pyo3(signature = (high, low))]
fn plus_dm(
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

// ============================================================================
// Cycle Indicators (Hilbert Transform)
// ============================================================================

// ============================================================================
// Volume Indicators
// ============================================================================

// ============================================================================
// Volatility Indicators
// ============================================================================

// ============================================================================
// Price Transform Functions
// ============================================================================

// ============================================================================
// Statistics Functions
// ============================================================================

/// Variance (VAR)
///
/// Calculates the rolling variance scaled by nb_dev.
///
/// # Arguments
/// * `close` - Input data series
/// * `timeperiod` - Rolling window size (default: 5)
/// * `nbdev` - Variance multiplier (default: 1.0)
#[pyfunction]
#[pyo3(signature = (close, timeperiod=5, nbdev=1.0))]
fn var(
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

// ============================================================================
// Candlestick Pattern Recognition
// ============================================================================

/// 4 Price Doji (四价十字)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_doji_4prices(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
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
        candlestick::doji_4prices(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Harami Cross (十字孕线)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_harami_cross(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
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
        candlestick::harami_cross(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Morning Doji Star (十字晨星)
#[pyfunction]
#[pyo3(signature = (open, high, low, close, doji_pct=0.1))]
fn cdl_morning_doji_star(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    doji_pct: f64,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
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
        candlestick::morning_doji_star(open, high, low, close, doji_pct)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Evening Doji Star (十字暮星)
#[pyfunction]
#[pyo3(signature = (open, high, low, close, doji_pct=0.1))]
fn cdl_evening_doji_star(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    doji_pct: f64,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
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
        candlestick::evening_doji_star(open, high, low, close, doji_pct)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Three Inside Up (内包向上)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_three_inside_up(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
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
        candlestick::three_inside_up(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Three Outside Up (外包向上)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_three_outside_up(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
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
        candlestick::three_outside_up(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Three Inside Down (内包向下)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_three_inside_down(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
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
        candlestick::three_inside_down(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Three Outside Down (外包向下)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_three_outside_down(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
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
        candlestick::three_outside_down(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Piercing Pattern (刺透形态)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_piercing(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
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
        candlestick::piercing(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Dark Cloud Cover (乌云盖顶)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_dark_cloud_cover(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
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
        candlestick::dark_cloud_cover(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Belt Hold (捉腰带线)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_belt_hold(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
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
        candlestick::belt_hold(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Spinning Top (纺锤线)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_spinning_top(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
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
        candlestick::spinning_top(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// High Wave (高浪线)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_high_wave(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
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
        candlestick::high_wave(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Rickshaw Man (黄包车夫)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_rickshaw_man(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
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
        candlestick::rickshaw_man(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Short Line Candle (短蜡烛)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_short_line(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
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
        candlestick::short_line(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Long Line Candle (长蜡烛)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_long_line(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
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
        candlestick::long_line(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Kicking (反冲形态)
#[pyfunction]
#[pyo3(signature = (open, high, low, close))]
fn cdl_kicking(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<i32>> {
    let open = open
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
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
        candlestick::kicking(open, high, low, close)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

// ============================================================================
// Chart Pattern Recognition
// ============================================================================

/// Head and Shoulders Top Pattern (头肩顶)
///
/// # Arguments
/// * `high` - High prices
/// * `min_bars` - Minimum bars between peaks (default: 5)
/// * `head_ratio` - Head height ratio vs shoulders (default: 1.1)
///
/// # Returns
/// Array with 1 where pattern is detected
#[pyfunction]
#[pyo3(signature = (high, min_bars=5, head_ratio=1.1))]
fn detect_head_shoulders(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    min_bars: usize,
    head_ratio: f64,
) -> PyResult<Vec<usize>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        let signals = chart::head_and_shoulders_top(high, min_bars, head_ratio)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;

        let indices: Vec<usize> = signals
            .iter()
            .enumerate()
            .filter(|(_, &v)| v == 1)
            .map(|(i, _)| i)
            .collect();
        Ok(indices)
    })
}

/// Head and Shoulders Bottom Pattern (头肩底)
#[pyfunction]
#[pyo3(signature = (low, min_bars=5, head_ratio=0.9))]
fn detect_head_shoulders_bottom(
    py: Python<'_>,
    low: PyReadonlyArray1<'_, f64>,
    min_bars: usize,
    head_ratio: f64,
) -> PyResult<Vec<usize>> {
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        let signals = chart::head_and_shoulders_bottom(low, min_bars, head_ratio)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;

        let indices: Vec<usize> = signals
            .iter()
            .enumerate()
            .filter(|(_, &v)| v == 1)
            .map(|(i, _)| i)
            .collect();
        Ok(indices)
    })
}

/// Double Top Pattern (双顶)
#[pyfunction]
#[pyo3(signature = (high, lookback=20, tolerance=0.03))]
fn detect_double_top(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    lookback: usize,
    tolerance: f64,
) -> PyResult<Vec<i32>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        chart::double_top(high, lookback, tolerance)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Double Bottom Pattern (双底)
#[pyfunction]
#[pyo3(signature = (low, lookback=20, tolerance=0.03))]
fn detect_double_bottom(
    py: Python<'_>,
    low: PyReadonlyArray1<'_, f64>,
    lookback: usize,
    tolerance: f64,
) -> PyResult<Vec<i32>> {
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        chart::double_bottom(low, lookback, tolerance)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Triple Top Pattern (三顶)
#[pyfunction]
#[pyo3(signature = (high, lookback=30, tolerance=0.03))]
fn detect_triple_top(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    lookback: usize,
    tolerance: f64,
) -> PyResult<Vec<i32>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        chart::triple_top(high, lookback, tolerance)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Triple Bottom Pattern (三底)
#[pyfunction]
#[pyo3(signature = (low, lookback=30, tolerance=0.03))]
fn detect_triple_bottom(
    py: Python<'_>,
    low: PyReadonlyArray1<'_, f64>,
    lookback: usize,
    tolerance: f64,
) -> PyResult<Vec<i32>> {
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        chart::triple_bottom(low, lookback, tolerance)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Ascending Triangle Pattern (上升三角形)
#[pyfunction]
#[pyo3(signature = (high, low, lookback=20, tolerance=0.05))]
fn detect_ascending_triangle(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    lookback: usize,
    tolerance: f64,
) -> PyResult<Vec<i32>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        chart::ascending_triangle(high, low, lookback, tolerance)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Descending Triangle Pattern (下降三角形)
#[pyfunction]
#[pyo3(signature = (high, low, lookback=20, tolerance=0.05))]
fn detect_descending_triangle(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    lookback: usize,
    tolerance: f64,
) -> PyResult<Vec<i32>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        chart::descending_triangle(high, low, lookback, tolerance)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Symmetrical Triangle Pattern (对称三角形)
#[pyfunction]
#[pyo3(signature = (high, low, lookback=20))]
fn detect_symmetrical_triangle(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    lookback: usize,
) -> PyResult<Vec<i32>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        chart::symmetrical_triangle(high, low, lookback)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Rising Wedge Pattern (上升楔形)
#[pyfunction]
#[pyo3(signature = (high, low, lookback=20))]
fn detect_rising_wedge(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    lookback: usize,
) -> PyResult<Vec<i32>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        chart::rising_wedge(high, low, lookback)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Falling Wedge Pattern (下降楔形)
#[pyfunction]
#[pyo3(signature = (high, low, lookback=20))]
fn detect_falling_wedge(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    lookback: usize,
) -> PyResult<Vec<i32>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        chart::falling_wedge(high, low, lookback)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Flag Pattern (旗形)
#[pyfunction]
#[pyo3(signature = (high, low, close, flagpole_period=10, flag_period=5))]
fn detect_flag(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    flagpole_period: usize,
    flag_period: usize,
) -> PyResult<Vec<i32>> {
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
        chart::flag(high, low, close, flagpole_period, flag_period)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Pennant Pattern (三角旗形)
#[pyfunction]
#[pyo3(signature = (high, low, close, flagpole_period=10, pennant_period=5))]
fn detect_pennant(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    flagpole_period: usize,
    pennant_period: usize,
) -> PyResult<Vec<i32>> {
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
        chart::pennant(high, low, close, flagpole_period, pennant_period)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Rectangle Pattern (矩形)
#[pyfunction]
#[pyo3(signature = (high, low, lookback=20, tolerance=0.05))]
fn detect_rectangle(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    lookback: usize,
    tolerance: f64,
) -> PyResult<Vec<i32>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        chart::rectangle(high, low, lookback, tolerance)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

// ============================================================================
// New Indicators (TASK-166~180)
// ============================================================================

// ============================================================================
// Advanced Indicators
// ============================================================================

/// Ichimoku Cloud (Ichimoku Kinko Hyo)
///
/// A comprehensive indicator that shows support and resistance, identifies trend direction,
/// gauges momentum, and provides trading signals.
///
/// # Components
/// - Tenkan-sen (Conversion Line): (tenkan_period high + tenkan_period low) / 2
/// - Kijun-sen (Base Line): (kijun_period high + kijun_period low) / 2
/// - Senkou Span A (Leading Span A): (Tenkan-sen + Kijun-sen) / 2, displaced forward
/// - Senkou Span B (Leading Span B): (senkou_b_period high + senkou_b_period low) / 2, displaced forward
/// - Chikou Span (Lagging Span): Close price, displaced backward
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `tenkan_period` - Tenkan-sen period (default: 9)
/// * `kijun_period` - Kijun-sen period (default: 26)
/// * `senkou_b_period` - Senkou Span B period (default: 52)
///
/// # Returns
/// Tuple of (tenkan_sen, kijun_sen, senkou_span_a, senkou_span_b, chikou_span) arrays
#[pyfunction]
#[pyo3(signature = (high, low, close, tenkan_period=9, kijun_period=26, senkou_b_period=52))]
#[allow(clippy::type_complexity)]
fn ichimoku(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    tenkan_period: usize,
    kijun_period: usize,
    senkou_b_period: usize,
) -> PyResult<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> {
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
        let displacement = kijun_period;
        indicators::ichimoku(
            high,
            low,
            close,
            tenkan_period,
            kijun_period,
            senkou_b_period,
            displacement,
        )
        .map(|res| {
            (
                res.tenkan_sen.into_raw_vec(),
                res.kijun_sen.into_raw_vec(),
                res.senkou_span_a.into_raw_vec(),
                res.senkou_span_b.into_raw_vec(),
                res.chikou_span.into_raw_vec(),
            )
        })
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// SuperTrend Indicator
///
/// A trend-following indicator that uses ATR to calculate upper and lower bands,
/// then determines trend direction based on price relationship to the bands.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `period` - ATR calculation period (default: 10)
/// * `multiplier` - ATR multiplier for band width (default: 3.0)
///
/// # Returns
/// Tuple of (direction, trend_line, upper_band, lower_band) arrays
/// - direction: 1 for uptrend, -1 for downtrend
#[pyfunction]
#[pyo3(signature = (high, low, close, period=10, multiplier=3.0))]
#[allow(clippy::type_complexity)]
fn supertrend(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
    multiplier: f64,
) -> PyResult<(Vec<i32>, Vec<f64>, Vec<f64>, Vec<f64>)> {
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
        indicators::supertrend(high, low, close, period, multiplier)
            .map(|res| {
                (
                    res.direction.into_raw_vec(),
                    res.trend_line.into_raw_vec(),
                    res.upper_band.into_raw_vec(),
                    res.lower_band.into_raw_vec(),
                )
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Volume Weighted Average Price (VWAP)
///
/// A trading benchmark that represents the average price a security has traded at,
/// based on both volume and price.
///
/// # Formula
/// VWAP = Σ(Typical Price × Volume) / Σ(Volume)
/// where Typical Price = (High + Low + Close) / 3
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
///
/// # Returns
/// Array of VWAP values
#[pyfunction]
#[pyo3(signature = (high, low, close, volume))]
fn vwap(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
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
    let volume = volume
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::vwap(high, low, close, volume)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Anchored Volume Weighted Average Price (Anchored VWAP)
///
/// Similar to VWAP, but allows traders to specify a starting point (anchor) from which
/// the calculation begins. Useful for measuring average price from significant events.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `start_index` - The index from which to start calculating VWAP
///
/// # Returns
/// Array of Anchored VWAP values (NaN for indices before start_index)
#[pyfunction]
#[pyo3(signature = (high, low, close, volume, start_index))]
fn anchored_vwap(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    start_index: usize,
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
    let volume = volume
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::anchored_vwap(high, low, close, volume, start_index)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// VWAP Bands
///
/// VWAP with upper and lower bands based on rolling standard deviation.
/// These bands help identify overbought and oversold levels relative to VWAP.
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `timeperiod` - Lookback period for standard deviation calculation (default: 20)
/// * `nb_dev` - Number of standard deviations for the bands (default: 2.0)
///
/// # Returns
/// Tuple of (vwap, upper_band, lower_band) arrays
#[pyfunction]
#[pyo3(signature = (high, low, close, volume, timeperiod=20, nb_dev=2.0))]
fn vwap_bands(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
    nb_dev: f64,
) -> PyResult<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let volume = volume
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::vwap_bands(high, low, close, volume, timeperiod, nb_dev)
            .map(|res| {
                (
                    res.vwap.into_raw_vec(),
                    res.upper.into_raw_vec(),
                    res.lower.into_raw_vec(),
                )
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Elder-Ray Indicator
///
/// Developed by Alexander Elder, this indicator evaluates the balance of power
/// between bulls and bears in the market using three components.
///
/// # Components
/// - Force Index: (Close[i] - Close[i-1]) × Volume[i]
/// - Bull Power: High[i] - EMA(Close, period)[i]
/// - Bear Power: Low[i] - EMA(Close, period)[i]
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `period` - EMA lookback period (default: 13)
///
/// # Returns
/// Tuple of (force_index, bull_power, bear_power) arrays
#[pyfunction]
#[pyo3(signature = (high, low, close, volume, period=13))]
fn elder_ray(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let volume = volume
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::elder_ray(high, low, close, volume, period)
            .map(|res| {
                (
                    res.force_index.into_raw_vec(),
                    res.bull_power.into_raw_vec(),
                    res.bear_power.into_raw_vec(),
                )
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Donchian Channel
///
/// A trend-following indicator that displays the highest and lowest prices
/// over a given period. The width shows the volatility range.
///
/// # Formula
/// - Upper Band = Highest High over N periods
/// - Lower Band = Lowest Low over N periods
/// - Middle Band = (Upper Band + Lower Band) / 2
/// - Width = Upper Band - Lower Band
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `period` - Lookback period (default: 20)
///
/// # Returns
/// Tuple of (upper, middle, lower, width) arrays
#[pyfunction]
#[pyo3(signature = (high, low, period=20))]
#[allow(clippy::type_complexity)]
fn donchian(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>)> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::donchian(high, low, period)
            .map(|res| {
                (
                    res.upper.into_raw_vec(),
                    res.middle.into_raw_vec(),
                    res.lower.into_raw_vec(),
                    res.width.into_raw_vec(),
                )
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Pivot Points
///
/// A technical analysis indicator used to determine the overall trend direction
/// and potential support/resistance levels.
///
/// # Supported Methods
/// - **standard**: Classic pivot points using (H + L + C) / 3
/// - **fibonacci**: Uses Fibonacci ratios (0.382, 0.618) for support/resistance
/// - **woodies**: Places more emphasis on the close price
/// - **camarilla**: Provides levels for day trading
/// - **demark**: Uses a conditional formula based on open/close relationship
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `method` - Calculation method (default: "standard")
///
/// # Returns
/// Tuple of (pivot, r1, r2, r3, s1, s2, s3) arrays
#[pyfunction]
#[pyo3(signature = (high, low, close, method="standard"))]
#[allow(clippy::type_complexity)]
fn pivot_points(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    method: &str,
) -> PyResult<(
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
    Vec<f64>,
)> {
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
        let pivot_method = match method {
            "standard" => PivotMethod::Standard,
            "fibonacci" => PivotMethod::Fibonacci,
            "woodie" | "woodies" => PivotMethod::Woodie,
            "camarilla" => PivotMethod::Camarilla,
            "demark" => PivotMethod::DeMark,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Unknown pivot method: {}. Use: standard, fibonacci, woodie, camarilla, demark",
                    method
                )))
            }
        };
        indicators::pivot_points(high, low, close, pivot_method)
            .map(|res| {
                (
                    res.pivot.into_raw_vec(),
                    res.r1.into_raw_vec(),
                    res.r2.into_raw_vec(),
                    res.r3.into_raw_vec(),
                    res.s1.into_raw_vec(),
                    res.s2.into_raw_vec(),
                    res.s3.into_raw_vec(),
                )
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Volume Profile
///
/// Analyzes volume distribution across price levels to identify key support/resistance zones.
///
/// # Key Levels
/// - POC (Point of Control): Price level with the highest traded volume
/// - VAH (Value Area High): Upper boundary of the 70% value area
/// - VAL (Value Area Low): Lower boundary of the 70% value area
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Close prices
/// * `volume` - Volume data
/// * `num_bins` - Number of price bins for volume distribution (default: 24)
///
/// # Returns
/// Tuple of (poc, vah, val) scalar values
#[pyfunction]
#[pyo3(signature = (high, low, close, volume, num_bins=24))]
fn volume_profile(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    num_bins: usize,
) -> PyResult<(f64, f64, f64)> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let volume = volume
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::volume_profile(high, low, close, volume, num_bins)
            .map(|res| (res.poc, res.vah, res.val))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

/// Fibonacci Retracement
///
/// Calculates Fibonacci retracement and extension levels based on
/// the highest and lowest prices in a specified range.
///
/// # Levels Returned
/// - Retracement: 0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0
/// - Extension: 1.272, 1.618
///
/// # Arguments
/// * `high` - High prices
/// * `low` - Low prices
/// * `start_index` - Start index of the range (inclusive)
/// * `end_index` - End index of the range (inclusive)
///
/// # Returns
/// Dictionary mapping Fibonacci ratio to price level
#[pyfunction]
#[pyo3(signature = (high, low, start_index, end_index))]
fn fibonacci_retracement(
    py: pyo3::Python<'_>,
    high: Vec<f64>,
    low: Vec<f64>,
    start_index: usize,
    end_index: usize,
) -> PyResult<pyo3::Bound<'_, pyo3::types::PyDict>> {
    let result = py.detach(|| {
        indicators::fibonacci_retracement(&high, &low, start_index, end_index)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })?;
    let dict = pyo3::types::PyDict::new(py);
    for level in &result.levels {
        dict.set_item(level.ratio, level.price)?;
    }
    Ok(dict)
}

// ============================================================================
// Formula System
// ============================================================================

/// Execute a trading formula
///
/// This function compiles and executes a trading formula string similar to
/// TongDaXin (通达信) formula language.
///
/// # Arguments
///
/// * `source` - Formula source code
/// * `open` - Opening prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Closing prices
/// * `volume` - Trading volume
/// * `amount` - Trading amount (optional)
///
/// # Returns
///
/// Dictionary with output variable names as keys and arrays as values.
///
/// # Example
///
/// ```python
/// import finkit
/// result = finkit.formula_eval(
///     "MA5:=MA(C,5); MA10:=MA(C,10); CROSS(MA5, MA10)",
///     open, high, low, close, volume
/// )
/// ```
#[pyfunction]
#[pyo3(signature = (source, open, high, low, close, volume, amount=None))]
#[cfg(feature = "formula")]
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn formula_eval(
    py: pyo3::Python<'_>,
    source: &str,
    open: &Bound<'_, PyAny>,
    high: &Bound<'_, PyAny>,
    low: &Bound<'_, PyAny>,
    close: &Bound<'_, PyAny>,
    volume: &Bound<'_, PyAny>,
    amount: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let open_vec = extract_array_bound(open)?;
    let high_vec = extract_array_bound(high)?;
    let low_vec = extract_array_bound(low)?;
    let close_vec = extract_array_bound(close)?;
    let volume_vec = extract_array_bound(volume)?;
    let amount_vec = amount.map(extract_array_bound).transpose()?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);
    let amount_array = amount_vec.map(Array1::from_vec);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        amount_array,
    );
    let mut engine = FormulaEngine::new();

    let result = py.detach(|| {
        engine
            .eval(source, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let dict = pyo3::types::PyDict::new(py);

    for (name, value) in &ctx.variables {
        let vec_value = value.to_vec();
        dict.set_item(name.to_string(), vec_value)?;
    }

    dict.set_item("__result__", result.to_vec())?;

    Ok(dict.into())
}

/// Execute a formula using a specific dialect (alpha_ta or pine).
///
/// Same as [`formula_eval`] but parses the source with the requested dialect.
/// Pine Script v5 scripts (`//@version=5 ...`) are mapped to the AlphaTA AST
/// and evaluated through the same execution pipeline.
///
/// # Arguments
///
/// * `source` - Formula / Pine Script source code
/// * `dialect` - `"alpha_ta"` (default TongDaXin) or `"pine"` (Pine Script v5)
/// * `open`, `high`, `low`, `close`, `volume` - OHLCV series
/// * `amount` - optional trading amount series
#[pyfunction]
#[pyo3(signature = (source, open, high, low, close, volume, dialect = "alpha_ta", amount=None))]
#[cfg(feature = "formula")]
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn formula_eval_dialect(
    py: pyo3::Python<'_>,
    source: &str,
    open: &Bound<'_, PyAny>,
    high: &Bound<'_, PyAny>,
    low: &Bound<'_, PyAny>,
    close: &Bound<'_, PyAny>,
    volume: &Bound<'_, PyAny>,
    dialect: &str,
    amount: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let open_vec = extract_array_bound(open)?;
    let high_vec = extract_array_bound(high)?;
    let low_vec = extract_array_bound(low)?;
    let close_vec = extract_array_bound(close)?;
    let volume_vec = extract_array_bound(volume)?;
    let amount_vec = amount.map(extract_array_bound).transpose()?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);
    let amount_array = amount_vec.map(Array1::from_vec);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        amount_array,
    );
    let mut engine = FormulaEngine::new();

    let dialect = FormulaDialect::from_str(dialect).unwrap_or(FormulaDialect::AlphaTA);
    let result = py.detach(|| -> PyResult<Array1<f64>> {
        let ast = match dialect {
            FormulaDialect::AlphaTA => {
                return engine
                    .eval(source, &mut ctx)
                    .map_err(formula_error_to_pyerr);
            }
            FormulaDialect::Pine => parse_formula_with_dialect(source, FormulaDialect::Pine)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))?,
        };
        engine
            .eval_ast(&ast, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let dict = pyo3::types::PyDict::new(py);

    for (name, value) in &ctx.variables {
        let vec_value = value.to_vec();
        dict.set_item(name.to_string(), vec_value)?;
    }

    dict.set_item("__result__", result.to_vec())?;

    Ok(dict.into())
}

/// Validate a formula without executing
///
/// Checks if the formula syntax is valid without actually running it.
///
/// # Arguments
///
/// * `source` - Formula source code to validate
///
/// # Returns
///
/// `True` if the formula is syntactically valid, `False` otherwise.
#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_validate(py: Python<'_>, source: &str) -> PyResult<bool> {
    py.detach(|| match parse_formula(source) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    })
}

/// Execute formula with bytecode compilation (faster)
///
/// Compiles the formula to bytecode before execution for improved performance.
/// Suitable for repeated execution of the same formula.
///
/// # Arguments
/// * `source` - Formula source code
/// * `open` - Opening prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Closing prices
/// * `volume` - Trading volume
///
/// # Returns
/// Dictionary with output variable names as keys and arrays as values.
/// The special key "__result__" contains the final expression result.
#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_eval_bytecode(
    py: Python<'_>,
    source: &str,
    open: Py<PyAny>,
    high: Py<PyAny>,
    low: Py<PyAny>,
    close: Py<PyAny>,
    volume: Py<PyAny>,
) -> PyResult<Py<PyAny>> {
    let open_vec = extract_array_pyobject(open)?;
    let high_vec = extract_array_pyobject(high)?;
    let low_vec = extract_array_pyobject(low)?;
    let close_vec = extract_array_pyobject(close)?;
    let volume_vec = extract_array_pyobject(volume)?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);

    let (result, variables): (
        Array1<f64>,
        std::collections::HashMap<std::sync::Arc<str>, Array1<f64>>,
    ) = py.detach(|| {
        let ctx = FormulaContext::new(
            open_array,
            high_array,
            low_array,
            close_array,
            volume_array,
            None,
        );
        let mut engine = FormulaEngine::new();
        let result = engine
            .compile_bytecode(source)
            .and_then(|bc| engine.execute_bytecode(&bc, &ctx))
            .map_err(formula_error_to_pyerr)?;
        Result::<_, PyErr>::Ok((result, ctx.variables))
    })?;

    let dict = pyo3::types::PyDict::new(py);

    for (name, value) in &variables {
        let vec_value = value.to_vec();
        dict.set_item(name.to_string(), vec_value)?;
    }

    dict.set_item("__result__", result.to_vec())?;

    Ok(dict.into())
}

/// Execute formula with optimization
///
/// Applies optimization passes (constant folding, dead code elimination)
/// before execution for maximum performance.
///
/// # Arguments
/// * `source` - Formula source code
/// * `open` - Opening prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Closing prices
/// * `volume` - Trading volume
///
/// # Returns
/// Dictionary with output variable names as keys and arrays as values.
/// The special key "__result__" contains the final expression result.
#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_eval_optimized(
    py: Python<'_>,
    source: &str,
    open: Py<PyAny>,
    high: Py<PyAny>,
    low: Py<PyAny>,
    close: Py<PyAny>,
    volume: Py<PyAny>,
) -> PyResult<Py<PyAny>> {
    let open_vec = extract_array_pyobject(open)?;
    let high_vec = extract_array_pyobject(high)?;
    let low_vec = extract_array_pyobject(low)?;
    let close_vec = extract_array_pyobject(close)?;
    let volume_vec = extract_array_pyobject(volume)?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        None,
    );
    let mut engine = FormulaEngine::new();

    let result = py.detach(|| {
        engine
            .eval_optimized(source, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let dict = pyo3::types::PyDict::new(py);

    for (name, value) in &ctx.variables {
        let vec_value = value.to_vec();
        dict.set_item(name.to_string(), vec_value)?;
    }

    dict.set_item("__result__", result.to_vec())?;

    Ok(dict.into())
}

/// Execute formula with JIT compilation
///
/// Compiles the formula using Just-In-Time compilation for maximum execution speed.
/// This is ideal for formulas that need to be executed repeatedly with different data.
///
/// # Arguments
/// * `source` - Formula source code
/// * `open` - Opening prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Closing prices
/// * `volume` - Trading volume
///
/// # Returns
/// Dictionary with output variable names as keys and arrays as values.
/// The special key "__result__" contains the final expression result.
#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_eval_jit(
    py: Python<'_>,
    source: &str,
    open: Py<PyAny>,
    high: Py<PyAny>,
    low: Py<PyAny>,
    close: Py<PyAny>,
    volume: Py<PyAny>,
) -> PyResult<Py<PyAny>> {
    let open_vec = extract_array_pyobject(open)?;
    let high_vec = extract_array_pyobject(high)?;
    let low_vec = extract_array_pyobject(low)?;
    let close_vec = extract_array_pyobject(close)?;
    let volume_vec = extract_array_pyobject(volume)?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        None,
    );
    let mut engine = FormulaEngine::new();

    let result = py.detach(|| {
        engine
            .eval_jit(source, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let dict = pyo3::types::PyDict::new(py);

    for (name, value) in &ctx.variables {
        let vec_value = value.to_vec();
        dict.set_item(name.to_string(), vec_value)?;
    }

    dict.set_item("__result__", result.to_vec())?;

    Ok(dict.into())
}

/// Execute formula with SIMD optimization
///
/// Uses SIMD (Single Instruction Multiple Data) vectorization to accelerate
/// formula execution on supported hardware. Best suited for data-parallel
/// operations on large datasets.
///
/// # Arguments
/// * `source` - Formula source code
/// * `open` - Opening prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Closing prices
/// * `volume` - Trading volume
///
/// # Returns
/// Dictionary with output variable names as keys and arrays as values.
/// The special key "__result__" contains the final expression result.
#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_eval_simd(
    py: Python<'_>,
    source: &str,
    open: Py<PyAny>,
    high: Py<PyAny>,
    low: Py<PyAny>,
    close: Py<PyAny>,
    volume: Py<PyAny>,
) -> PyResult<Py<PyAny>> {
    let open_vec = extract_array_pyobject(open)?;
    let high_vec = extract_array_pyobject(high)?;
    let low_vec = extract_array_pyobject(low)?;
    let close_vec = extract_array_pyobject(close)?;
    let volume_vec = extract_array_pyobject(volume)?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        None,
    );
    let mut engine = FormulaEngine::new();

    let result = py.detach(|| {
        engine
            .eval_simd(source, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let dict = pyo3::types::PyDict::new(py);

    for (name, value) in &ctx.variables {
        let vec_value = value.to_vec();
        dict.set_item(name.to_string(), vec_value)?;
    }

    dict.set_item("__result__", result.to_vec())?;

    Ok(dict.into())
}

/// Execute formula with zero-copy optimization
///
/// Minimizes memory allocations by operating directly on input buffers
/// without copying data. This provides the lowest latency execution path
/// for latency-sensitive applications.
///
/// # Arguments
/// * `source` - Formula source code
/// * `open` - Opening prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Closing prices
/// * `volume` - Trading volume
///
/// # Returns
/// Dictionary with output variable names as keys and arrays as values.
/// The special key "__result__" contains the final expression result.
/// Evaluate a contiguous float64 NumPy input without copying the OHLCV
/// buffers. Direct MA/EMA/RSI/BOLLMID formulas use borrowed slices; complex
/// formulas fall back to the regular formula ABI for intermediate arrays.
#[pyfunction]
#[pyo3(signature = (source, open, high, low, close, volume))]
#[cfg(feature = "formula")]
pub fn formula_eval_numpy_zero_copy(
    py: Python<'_>,
    source: &str,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
) -> PyResult<Py<PyAny>> {
    let open = open.as_slice().map_err(|error| {
        PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
            "open must be a contiguous float64 NumPy array: {error}"
        ))
    })?;
    let high = high.as_slice().map_err(|error| {
        PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
            "high must be a contiguous float64 NumPy array: {error}"
        ))
    })?;
    let low = low.as_slice().map_err(|error| {
        PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
            "low must be a contiguous float64 NumPy array: {error}"
        ))
    })?;
    let close = close.as_slice().map_err(|error| {
        PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
            "close must be a contiguous float64 NumPy array: {error}"
        ))
    })?;
    let volume = volume.as_slice().map_err(|error| {
        PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
            "volume must be a contiguous float64 NumPy array: {error}"
        ))
    })?;
    if close.is_empty()
        || [open, high, low, volume]
            .iter()
            .any(|values| values.len() != close.len())
    {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "all OHLCV arrays must be non-empty and have equal lengths",
        ));
    }

    let mut engine = FormulaEngine::new();
    let formula = engine.compile(source).map_err(formula_error_to_pyerr)?;
    let result = engine
        .eval_zero_copy_inputs(&formula, open, high, low, close, volume, None)
        .map_err(formula_error_to_pyerr)?;
    let dict = pyo3::types::PyDict::new(py);
    dict.set_item("__result__", PyArray1::from_vec(py, result.into_raw_vec()))?;
    Ok(dict.into())
}

#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_eval_zero_copy(
    py: Python<'_>,
    source: &str,
    open: Py<PyAny>,
    high: Py<PyAny>,
    low: Py<PyAny>,
    close: Py<PyAny>,
    volume: Py<PyAny>,
) -> PyResult<Py<PyAny>> {
    // Preserve the legacy list/tuple API, but use the borrowed NumPy path
    // whenever all five inputs are contiguous float64 arrays.
    let direct_result: Option<Array1<f64>> = Python::attach(|py| {
        let open_array = match open.bind(py).extract::<PyReadonlyArray1<'_, f64>>() {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };
        let high_array = match high.bind(py).extract::<PyReadonlyArray1<'_, f64>>() {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };
        let low_array = match low.bind(py).extract::<PyReadonlyArray1<'_, f64>>() {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };
        let close_array = match close.bind(py).extract::<PyReadonlyArray1<'_, f64>>() {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };
        let volume_array = match volume.bind(py).extract::<PyReadonlyArray1<'_, f64>>() {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };

        let open = open_array.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "open must be a contiguous float64 NumPy array: {error}"
            ))
        })?;
        let high = high_array.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "high must be a contiguous float64 NumPy array: {error}"
            ))
        })?;
        let low = low_array.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "low must be a contiguous float64 NumPy array: {error}"
            ))
        })?;
        let close = close_array.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "close must be a contiguous float64 NumPy array: {error}"
            ))
        })?;
        let volume = volume_array.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "volume must be a contiguous float64 NumPy array: {error}"
            ))
        })?;

        if close.is_empty()
            || [open, high, low, volume]
                .iter()
                .any(|values| values.len() != close.len())
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "all OHLCV arrays must be non-empty and have equal lengths",
            ));
        }

        let mut engine = FormulaEngine::new();
        let formula = engine.compile(source).map_err(formula_error_to_pyerr)?;
        engine
            .eval_zero_copy_inputs(&formula, open, high, low, close, volume, None)
            .map(Some)
            .map_err(formula_error_to_pyerr)
    })?;

    if let Some(result) = direct_result {
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("__result__", PyArray1::from_vec(py, result.into_raw_vec()))?;
        return Ok(dict.into());
    }

    let open_vec = extract_array_pyobject(open)?;
    let high_vec = extract_array_pyobject(high)?;
    let low_vec = extract_array_pyobject(low)?;
    let close_vec = extract_array_pyobject(close)?;
    let volume_vec = extract_array_pyobject(volume)?;

    let mut ctx = FormulaContext::new(
        Array1::from_vec(open_vec),
        Array1::from_vec(high_vec),
        Array1::from_vec(low_vec),
        Array1::from_vec(close_vec),
        Array1::from_vec(volume_vec),
        None,
    );
    let mut engine = FormulaEngine::new();

    let result = py.detach(|| {
        engine
            .eval_zero_copy(source, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let dict = pyo3::types::PyDict::new(py);
    for (name, value) in &ctx.variables {
        dict.set_item(name.to_string(), value.to_vec())?;
    }
    dict.set_item("__result__", result.to_vec())?;
    Ok(dict.into())
}

#[pyfunction]
#[pyo3(signature = (source, open, high, low, close, volume, amount=None))]
#[cfg(feature = "formula")]
pub fn formula_eval_multi(
    py: Python<'_>,
    source: &str,
    open: &Bound<'_, PyAny>,
    high: &Bound<'_, PyAny>,
    low: &Bound<'_, PyAny>,
    close: &Bound<'_, PyAny>,
    volume: &Bound<'_, PyAny>,
    amount: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let open_vec = extract_array_bound(open)?;
    let high_vec = extract_array_bound(high)?;
    let low_vec = extract_array_bound(low)?;
    let close_vec = extract_array_bound(close)?;
    let volume_vec = extract_array_bound(volume)?;
    let amount_vec = amount.map(extract_array_bound).transpose()?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);
    let amount_array = amount_vec.map(Array1::from_vec);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        amount_array,
    );
    let mut engine = FormulaEngine::new();

    let multi_output = py.detach(|| {
        engine
            .eval_multi(source, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let names_list = pyo3::types::PyList::empty(py);
    let values_list = pyo3::types::PyList::empty(py);

    for name in multi_output.names() {
        names_list.append(name.as_str())?;
        if let Some(arr) = multi_output.get(name) {
            values_list.append(arr.to_vec())?;
        }
    }

    let result_dict = pyo3::types::PyDict::new(py);
    result_dict.set_item("names", names_list)?;
    result_dict.set_item("values", values_list)?;
    result_dict.set_item("__result__", multi_output.final_value.to_vec())?;

    Ok(result_dict.into())
}

#[pyfunction]
#[pyo3(signature = (source, open, high, low, close, volume, amount=None))]
#[cfg(feature = "formula")]
pub fn formula_eval_draw(
    py: Python<'_>,
    source: &str,
    open: &Bound<'_, PyAny>,
    high: &Bound<'_, PyAny>,
    low: &Bound<'_, PyAny>,
    close: &Bound<'_, PyAny>,
    volume: &Bound<'_, PyAny>,
    amount: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    use ::finkit::formula::DrawCommand;

    let open_vec = extract_array_bound(open)?;
    let high_vec = extract_array_bound(high)?;
    let low_vec = extract_array_bound(low)?;
    let close_vec = extract_array_bound(close)?;
    let volume_vec = extract_array_bound(volume)?;
    let amount_vec = amount.map(extract_array_bound).transpose()?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);
    let amount_array = amount_vec.map(Array1::from_vec);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        amount_array,
    );
    let mut engine = FormulaEngine::new();

    let _result = py.detach(|| {
        engine
            .eval(source, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let draw_commands = ctx.draw_commands.borrow();
    let draw_list = pyo3::types::PyList::empty(py);
    for cmd in &draw_commands.commands {
        let cmd_dict = pyo3::types::PyDict::new(py);
        match cmd {
            DrawCommand::Text {
                condition: _,
                price: _,
                text,
                color,
            } => {
                cmd_dict.set_item("type", "Text")?;
                cmd_dict.set_item("text", text.as_str())?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::Icon {
                condition: _,
                price: _,
                icon_type,
                color,
            } => {
                cmd_dict.set_item("type", "Icon")?;
                cmd_dict.set_item("iconType", *icon_type)?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::StickLine {
                condition: _,
                price1: _,
                price2: _,
                width,
                empty,
                color,
            } => {
                cmd_dict.set_item("type", "StickLine")?;
                cmd_dict.set_item("width", *width)?;
                cmd_dict.set_item("empty", *empty)?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::Line {
                cond1: _,
                price1: _,
                cond2: _,
                price2: _,
                expand,
                color,
            } => {
                cmd_dict.set_item("type", "Line")?;
                cmd_dict.set_item("expand", *expand)?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::Band {
                val1: _,
                color1,
                val2: _,
                color2,
            } => {
                cmd_dict.set_item("type", "Band")?;
                cmd_dict.set_item("color1", color1.as_str())?;
                cmd_dict.set_item("color2", color2.as_str())?;
            }
            DrawCommand::KLine { .. } => {
                cmd_dict.set_item("type", "KLine")?;
            }
            DrawCommand::Rect {
                x1: _,
                y1: _,
                x2: _,
                y2: _,
                color,
            } => {
                cmd_dict.set_item("type", "Rect")?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::FillRgn {
                cond: _,
                price1: _,
                price2: _,
                color,
            } => {
                cmd_dict.set_item("type", "FillRgn")?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::PartLine {
                cond: _,
                price: _,
                color,
            } => {
                cmd_dict.set_item("type", "PartLine")?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::PolyLine {
                cond: _,
                price: _,
                color,
            } => {
                cmd_dict.set_item("type", "PolyLine")?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::Background { cond: _, color } => {
                cmd_dict.set_item("type", "Background")?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::SlopeLine {
                cond1: _,
                price1: _,
                cond2: _,
                price2: _,
                color,
            } => {
                cmd_dict.set_item("type", "SlopeLine")?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::TextFix { x, y, text, color } => {
                cmd_dict.set_item("type", "TextFix")?;
                cmd_dict.set_item("x", *x)?;
                cmd_dict.set_item("y", *y)?;
                cmd_dict.set_item("text", text.as_str())?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::Number {
                condition: _,
                price: _,
                number: _,
                precision,
                color,
            } => {
                cmd_dict.set_item("type", "Number")?;
                cmd_dict.set_item("precision", *precision)?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::VertLine {
                condition: _,
                color,
            } => {
                cmd_dict.set_item("type", "VertLine")?;
                cmd_dict.set_item("color", color.as_str())?;
            }
        }
        draw_list.append(cmd_dict)?;
    }

    let result_dict = pyo3::types::PyDict::new(py);
    for (name, value) in &ctx.variables {
        let vec_value = value.to_vec();
        result_dict.set_item(name.to_string(), vec_value)?;
    }
    result_dict.set_item("drawCommands", draw_list)?;

    Ok(result_dict.into())
}

/// Execute formula with debug info
///
/// Returns execution results along with debug information including
/// variable values at each step, execution trace, and timing.
///
/// # Arguments
/// * `source` - Formula source code
/// * `open` - Opening prices
/// * `high` - High prices
/// * `low` - Low prices
/// * `close` - Closing prices
/// * `volume` - Trading volume
///
/// # Returns
/// Dictionary with keys:
/// - "result": execution result dictionary
/// - "debug": debug information dictionary with step-by-step trace
#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_eval_debug(
    py: Python<'_>,
    source: &str,
    open: Py<PyAny>,
    high: Py<PyAny>,
    low: Py<PyAny>,
    close: Py<PyAny>,
    volume: Py<PyAny>,
) -> PyResult<Py<PyAny>> {
    let open_vec = extract_array_pyobject(open)?;
    let high_vec = extract_array_pyobject(high)?;
    let low_vec = extract_array_pyobject(low)?;
    let close_vec = extract_array_pyobject(close)?;
    let volume_vec = extract_array_pyobject(volume)?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        None,
    );
    let mut engine = FormulaEngine::new();

    let (final_result, debugger) = py.detach(|| {
        engine
            .eval_with_debug(source, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let result_dict = pyo3::types::PyDict::new(py);

    for (name, value) in &ctx.variables {
        let vec_value = value.to_vec();
        result_dict.set_item(name.to_string(), vec_value)?;
    }

    result_dict.set_item("__result__", final_result.to_vec())?;

    let debug_dict = pyo3::types::PyDict::new(py);
    let event_list = pyo3::types::PyList::empty(py);
    for event in debugger.get_events() {
        event_list.append(format!("{event:?}"))?;
    }
    debug_dict.set_item("events", event_list)?;

    let output_dict = pyo3::types::PyDict::new(py);
    output_dict.set_item("result", result_dict)?;
    output_dict.set_item("debug", debug_dict)?;

    Ok(output_dict.into())
}

/// Get formula template by name
///
/// Returns a specific formula template from the built-in template library.
///
/// # Arguments
/// * `name` - Template name (e.g., "MACD", "KDJ", "BOLL")
///
/// # Returns
/// Dictionary with template information:
/// - "name": template name
/// - "category": template category
/// - "description": template description
/// - "formula": formula source code
/// - "parameters": parameter descriptions
#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_get_template(py: Python<'_>, name: &str) -> PyResult<Py<PyAny>> {
    use ::finkit::formula::FormulaEngine;

    let dict = pyo3::types::PyDict::new(py);
    let engine = FormulaEngine::new();

    match engine.get_template(name) {
        Some(template) => {
            dict.set_item("name", template.name.as_str())?;
            dict.set_item("category", format!("{:?}", template.category))?;
            dict.set_item("description", template.description.as_str())?;
            dict.set_item("formula", template.source.as_str())?;

            let params_dict = pyo3::types::PyDict::new(py);
            for (param_name, default, min, max) in &template.parameters {
                let param_info = pyo3::types::PyDict::new(py);
                param_info.set_item("default", default)?;
                param_info.set_item("min", min)?;
                param_info.set_item("max", max)?;
                params_dict.set_item(param_name.as_str(), param_info)?;
            }
            dict.set_item("parameters", params_dict)?;
        }
        None => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Template '{}' not found",
                name
            )));
        }
    }

    Ok(dict.into())
}

/// Search formula templates by keyword
///
/// Searches the built-in template library for templates matching the given keyword.
///
/// # Arguments
/// * `keyword` - Search keyword (searches name, description, and category)
///
/// # Returns
/// List of matching template dictionaries.
#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_search_templates(py: Python<'_>, keyword: &str) -> PyResult<Py<PyAny>> {
    use ::finkit::formula::FormulaEngine;

    let engine = FormulaEngine::new();
    let templates = engine.search_templates(keyword);
    let list = pyo3::types::PyList::empty(py);

    for template in templates {
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("name", template.name.as_str())?;
        dict.set_item("category", format!("{:?}", template.category))?;
        dict.set_item("description", template.description.as_str())?;
        dict.set_item("formula", template.source.as_str())?;

        let params_dict = pyo3::types::PyDict::new(py);
        for (param_name, default, min, max) in &template.parameters {
            let param_info = pyo3::types::PyDict::new(py);
            param_info.set_item("default", default)?;
            param_info.set_item("min", min)?;
            param_info.set_item("max", max)?;
            params_dict.set_item(param_name.as_str(), param_info)?;
        }
        dict.set_item("parameters", params_dict)?;

        list.append(dict)?;
    }

    Ok(list.into())
}

/// List all template categories
///
/// Returns all available formula template categories.
///
/// # Returns
/// List of category names with their template counts.
#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_list_categories(py: Python<'_>) -> PyResult<Py<PyAny>> {
    use ::finkit::formula::templates::FormulaTemplates;

    let templates = FormulaTemplates::new();
    let list = pyo3::types::PyList::empty(py);

    for category in FormulaTemplates::categories() {
        let count = templates.get_by_category(&category).len();
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("category", format!("{category:?}"))?;
        dict.set_item("count", count)?;
        list.append(dict)?;
    }

    Ok(list.into())
}

fn convert_vis_error(e: VisualizationError) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e))
}

/// Computes Chanlun structures and returns plain Python dictionaries.
#[pyfunction]
#[pyo3(signature = (open, high, low, close, volume, min_stroke_bars=6, variant="standard", fractal_policy="strict", stroke_policy="configurable", center_policy="dynamic", min_stroke_change_ratio=0.0, min_fractal_range_ratio=0.0, signal_min_strength=0.0, center_break_ratio=0.0))]
fn chan_analyze(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    min_stroke_bars: usize,
    variant: &str,
    fractal_policy: &str,
    stroke_policy: &str,
    center_policy: &str,
    min_stroke_change_ratio: f64,
    min_fractal_range_ratio: f64,
    signal_min_strength: f64,
    center_break_ratio: f64,
) -> PyResult<Py<PyAny>> {
    let open = open
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let high = high
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let low = low
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let close = close
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let volume = volume
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let mut chan_config = parse_chan_config(
        min_stroke_bars,
        variant,
        fractal_policy,
        stroke_policy,
        center_policy,
    )?;
    chan_config.thresholds.min_stroke_change_ratio = min_stroke_change_ratio;
    chan_config.thresholds.min_fractal_range_ratio = min_fractal_range_ratio;
    chan_config.thresholds.signal_min_strength = signal_min_strength;
    chan_config.thresholds.center_break_ratio = center_break_ratio;
    let analysis = py
        .detach(|| analyze_chan(open, high, low, close, volume, chan_config))
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;

    let result = pyo3::types::PyDict::new(py);
    result.set_item("bar_count", analysis.bars.len())?;
    result.set_item("fractal_count", analysis.fractals.len())?;
    result.set_item("stroke_count", analysis.strokes.len())?;
    result.set_item("segment_count", analysis.segments.len())?;
    result.set_item("center_count", analysis.centers.len())?;
    result.set_item(
        "trend",
        match analysis.trend {
            ::finkit::chan::ChanTrend::Unknown => "unknown",
            ::finkit::chan::ChanTrend::Bullish => "bullish",
            ::finkit::chan::ChanTrend::Bearish => "bearish",
            ::finkit::chan::ChanTrend::Range => "range",
        },
    )?;
    if let Some(fractal) = analysis.developing_fractal {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("index", fractal.index)?;
        item.set_item(
            "kind",
            match fractal.kind {
                FractalKind::Top => "top",
                FractalKind::Bottom => "bottom",
            },
        )?;
        item.set_item("high", fractal.high)?;
        item.set_item("low", fractal.low)?;
        item.set_item("value", fractal.value)?;
        result.set_item("developing_fractal", item)?;
    } else {
        result.set_item("developing_fractal", py.None())?;
    }

    let fractals = pyo3::types::PyList::empty(py);
    for fractal in analysis.fractals {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("index", fractal.index)?;
        item.set_item(
            "kind",
            match fractal.kind {
                FractalKind::Top => "top",
                FractalKind::Bottom => "bottom",
            },
        )?;
        item.set_item("high", fractal.high)?;
        item.set_item("low", fractal.low)?;
        item.set_item("value", fractal.value)?;
        fractals.append(item)?;
    }
    result.set_item("fractals", fractals)?;

    let strokes = pyo3::types::PyList::empty(py);
    for stroke in analysis.strokes {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("start_index", stroke.start.index)?;
        item.set_item("end_index", stroke.end.index)?;
        item.set_item(
            "direction",
            match stroke.direction {
                ::finkit::chan::ChanDirection::Up => "up",
                ::finkit::chan::ChanDirection::Down => "down",
            },
        )?;
        item.set_item("high", stroke.high)?;
        item.set_item("low", stroke.low)?;
        item.set_item("bars", stroke.bars)?;
        item.set_item("change", stroke.change)?;
        item.set_item("slope", stroke.slope)?;
        item.set_item("strength", stroke.strength)?;
        strokes.append(item)?;
    }
    result.set_item("strokes", strokes)?;

    let segments = pyo3::types::PyList::empty(py);
    for segment in analysis.segments {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("start_index", segment.start_index)?;
        item.set_item("end_index", segment.end_index)?;
        item.set_item("start_stroke", segment.start_stroke)?;
        item.set_item("end_stroke", segment.end_stroke)?;
        item.set_item("change", segment.change)?;
        segments.append(item)?;
    }
    result.set_item("segments", segments)?;

    let centers = pyo3::types::PyList::empty(py);
    for center in analysis.centers {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("start_index", center.start_index)?;
        item.set_item("end_index", center.end_index)?;
        item.set_item("upper", center.upper)?;
        item.set_item("lower", center.lower)?;
        item.set_item("middle", center.middle)?;
        item.set_item("level", center.level)?;
        item.set_item("start_stroke", center.start_stroke)?;
        item.set_item("end_stroke", center.end_stroke)?;
        centers.append(item)?;
    }
    result.set_item("centers", centers)?;

    let signals = pyo3::types::PyList::empty(py);
    for signal in analysis.signals {
        let item = pyo3::types::PyDict::new(py);
        item.set_item(
            "kind",
            match signal.kind {
                ::finkit::chan::ChanSignalKind::Buy1 => "buy1",
                ::finkit::chan::ChanSignalKind::Buy2 => "buy2",
                ::finkit::chan::ChanSignalKind::Buy3 => "buy3",
                ::finkit::chan::ChanSignalKind::Sell1 => "sell1",
                ::finkit::chan::ChanSignalKind::Sell2 => "sell2",
                ::finkit::chan::ChanSignalKind::Sell3 => "sell3",
            },
        )?;
        item.set_item("index", signal.index)?;
        item.set_item("price", signal.price)?;
        item.set_item("confirmed", signal.confirmed)?;
        item.set_item("strength", signal.strength)?;
        item.set_item("reason", signal.reason)?;
        item.set_item("evidence", signal.evidence.clone())?;
        signals.append(item)?;
    }
    result.set_item("signals", signals)?;

    let divergences = pyo3::types::PyList::empty(py);
    for divergence in analysis.divergences {
        let item = pyo3::types::PyDict::new(py);
        item.set_item(
            "kind",
            match divergence.kind {
                ::finkit::chan::ChanDivergenceKind::Bullish => "bullish",
                ::finkit::chan::ChanDivergenceKind::Bearish => "bearish",
            },
        )?;
        item.set_item("start_stroke", divergence.start_stroke)?;
        item.set_item("end_stroke", divergence.end_stroke)?;
        item.set_item("index", divergence.index)?;
        item.set_item("price", divergence.price)?;
        item.set_item("confirmed", divergence.confirmed)?;
        item.set_item("strength", divergence.strength)?;
        item.set_item("reason", divergence.reason)?;
        divergences.append(item)?;
    }
    result.set_item("divergences", divergences)?;

    Ok(result.into())
}

fn parse_chan_config(
    min_stroke_bars: usize,
    variant: &str,
    fractal_policy: &str,
    stroke_policy: &str,
    center_policy: &str,
) -> PyResult<ChanConfig> {
    let variant = match variant.to_ascii_lowercase().as_str() {
        "conservative" | "strict" => ChanVariant::Conservative,
        "standard" | "default" => ChanVariant::Standard,
        "aggressive" | "loose" => ChanVariant::Aggressive,
        value => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "unknown Chan variant: {value}"
            )))
        }
    };
    let fractal_policy = match fractal_policy.to_ascii_lowercase().as_str() {
        "strict" => FractalPolicy::Strict,
        "loose" => FractalPolicy::Loose,
        "right_confirmed" | "right-confirmed" => FractalPolicy::RightConfirmed,
        value => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "unknown fractal policy: {value}"
            )))
        }
    };
    let stroke_policy = match stroke_policy.to_ascii_lowercase().as_str() {
        "configurable" | "min_bars" => StrokePolicy::Configurable,
        "fixed5" | "5" => StrokePolicy::Fixed5,
        "fixed6" | "6" => StrokePolicy::Fixed6,
        "fixed7" | "7" => StrokePolicy::Fixed7,
        "threshold" => StrokePolicy::Threshold,
        value => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "unknown stroke policy: {value}"
            )))
        }
    };
    let center_policy = match center_policy.to_ascii_lowercase().as_str() {
        "three_stroke" | "three-stroke" => CenterPolicy::ThreeStroke,
        "dynamic" => CenterPolicy::Dynamic,
        "hierarchical" | "multi_level" | "multi-level" => CenterPolicy::Hierarchical,
        value => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "unknown center policy: {value}"
            )))
        }
    };
    let mut config = ChanConfig::default().with_variant(variant);
    config.min_stroke_bars = min_stroke_bars;
    config.fractal_policy = fractal_policy;
    config.stroke_policy = stroke_policy;
    config.center_policy = center_policy;
    Ok(config)
}

/// Analyze automatically selected or explicitly supplied Chanlun timeframes.
#[pyfunction]
#[pyo3(signature = (open, high, low, close, volume, factors=None, auto_levels=3, min_frame_bars=20, min_stroke_bars=6, variant="standard"))]
fn chan_analyze_multi(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    factors: Option<Vec<usize>>,
    auto_levels: usize,
    min_frame_bars: usize,
    min_stroke_bars: usize,
    variant: &str,
) -> PyResult<Py<PyAny>> {
    let open = open
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?
        .to_vec();
    let high = high
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?
        .to_vec();
    let low = low
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?
        .to_vec();
    let close = close
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?
        .to_vec();
    let volume = volume
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?
        .to_vec();
    let config = ChanMultiConfig {
        factors: factors.unwrap_or_default(),
        auto_levels,
        min_frame_bars,
        chan: parse_chan_config(
            min_stroke_bars,
            variant,
            "strict",
            "configurable",
            "dynamic",
        )?,
    };
    let result = py
        .detach(|| analyze_chan_multi(&open, &high, &low, &close, &volume, &config))
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let resonance = result.resonance();
    let frames = pyo3::types::PyList::empty(py);
    for frame in result.frames {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("factor", frame.timeframe.factor)?;
        item.set_item("label", frame.timeframe.label)?;
        item.set_item("seconds", frame.timeframe.seconds)?;
        item.set_item("source_ranges", frame.source_ranges.clone())?;
        item.set_item("analysis", chan_analysis_summary(py, &frame.analysis)?)?;
        frames.append(item)?;
    }
    let output = pyo3::types::PyDict::new(py);
    output.set_item("frames", frames)?;
    output.set_item(
        "resonance_direction",
        match resonance.direction {
            ::finkit::chan_mtf::ChanResonanceDirection::Bullish => "bullish",
            ::finkit::chan_mtf::ChanResonanceDirection::Bearish => "bearish",
            ::finkit::chan_mtf::ChanResonanceDirection::Mixed => "mixed",
            ::finkit::chan_mtf::ChanResonanceDirection::Unknown => "unknown",
        },
    )?;
    output.set_item("resonance_score", resonance.score)?;
    output.set_item("aligned_frames", resonance.aligned_frames)?;
    output.set_item("conflicting_frames", resonance.conflicting_frames)?;
    Ok(output.into())
}

/// Analyze multi-timeframe Chanlun structures from Unix-second timestamps.
#[pyfunction(name = "chan_analyze_multi_timestamps")]
#[pyo3(signature = (timestamps, open, high, low, close, volume, durations_seconds=None, min_stroke_bars=6, variant="standard", origin_seconds=0))]
fn chan_analyze_multi_timestamps_py(
    py: Python<'_>,
    timestamps: PyReadonlyArray1<'_, i64>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    durations_seconds: Option<Vec<i64>>,
    min_stroke_bars: usize,
    variant: &str,
    origin_seconds: i64,
) -> PyResult<Py<PyAny>> {
    let to_vec = |array: &PyReadonlyArray1<'_, f64>| {
        array
            .as_slice()
            .map(|values| values.to_vec())
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))
    };
    let timestamps = timestamps
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?
        .to_vec();
    let open = to_vec(&open)?;
    let high = to_vec(&high)?;
    let low = to_vec(&low)?;
    let close = to_vec(&close)?;
    let volume = to_vec(&volume)?;
    let config = ChanMultiConfig {
        chan: parse_chan_config(
            min_stroke_bars,
            variant,
            "strict",
            "configurable",
            "dynamic",
        )?,
        ..ChanMultiConfig::default()
    };
    let result = py
        .detach(|| {
            analyze_chan_multi_timestamps(
                &timestamps,
                &open,
                &high,
                &low,
                &close,
                &volume,
                durations_seconds.as_deref().unwrap_or_default(),
                &config,
                origin_seconds,
            )
        })
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let resonance = result.resonance();
    let frames = pyo3::types::PyList::empty(py);
    for frame in result.frames {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("factor", frame.timeframe.factor)?;
        item.set_item("label", frame.timeframe.label)?;
        item.set_item("seconds", frame.timeframe.seconds)?;
        item.set_item("source_ranges", frame.source_ranges)?;
        item.set_item("analysis", chan_analysis_summary(py, &frame.analysis)?)?;
        frames.append(item)?;
    }
    let output = pyo3::types::PyDict::new(py);
    output.set_item("frames", frames)?;
    output.set_item(
        "resonance_direction",
        match resonance.direction {
            ::finkit::chan_mtf::ChanResonanceDirection::Bullish => "bullish",
            ::finkit::chan_mtf::ChanResonanceDirection::Bearish => "bearish",
            ::finkit::chan_mtf::ChanResonanceDirection::Mixed => "mixed",
            ::finkit::chan_mtf::ChanResonanceDirection::Unknown => "unknown",
        },
    )?;
    output.set_item("resonance_score", resonance.score)?;
    output.set_item("aligned_frames", resonance.aligned_frames)?;
    output.set_item("conflicting_frames", resonance.conflicting_frames)?;
    Ok(output.into())
}

/// Analyze timestamped Chanlun structures with a configurable exchange
/// calendar and timezone adapter.
#[pyfunction(name = "chan_analyze_multi_timestamps_calendar")]
#[pyo3(signature = (timestamps, open, high, low, close, volume, market="a_share", durations_seconds=None, timezone=None, holidays=None, sessions=None, special_sessions=None, min_stroke_bars=6, variant="standard"))]
fn chan_analyze_multi_timestamps_calendar_py(
    py: Python<'_>,
    timestamps: PyReadonlyArray1<'_, i64>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    market: &str,
    durations_seconds: Option<Vec<i64>>,
    timezone: Option<&str>,
    holidays: Option<Vec<String>>,
    sessions: Option<Vec<(u32, u32)>>,
    special_sessions: Option<Vec<(String, Vec<(u32, u32)>)>>,
    min_stroke_bars: usize,
    variant: &str,
) -> PyResult<Py<PyAny>> {
    let to_vec = |array: &PyReadonlyArray1<'_, f64>| {
        array
            .as_slice()
            .map(|values| values.to_vec())
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))
    };
    let timestamps = timestamps
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?
        .to_vec();
    let open = to_vec(&open)?;
    let high = to_vec(&high)?;
    let low = to_vec(&low)?;
    let close = to_vec(&close)?;
    let volume = to_vec(&volume)?;
    let preset = MarketCalendarPreset::parse(market)
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let mut calendar = TradingCalendar::for_market(preset);
    if let Some(timezone) = timezone {
        calendar = calendar.with_timezone(TimeZoneSpec::parse(timezone).map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
        })?);
    }
    if let Some(holidays) = holidays {
        for holiday in holidays {
            calendar.add_holiday(&holiday).map_err(|error| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
            })?;
        }
    }
    if let Some(sessions) = sessions {
        let sessions: Vec<SessionWindow> = sessions
            .into_iter()
            .map(|(open, close)| SessionWindow::new(open, close))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
        calendar = calendar.with_sessions(&sessions);
    }
    if let Some(special_sessions) = special_sessions {
        for (date, sessions) in special_sessions {
            let sessions: Vec<SessionWindow> = sessions
                .into_iter()
                .map(|(open, close)| SessionWindow::new(open, close))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
                })?;
            calendar
                .set_special_sessions(&date, &sessions)
                .map_err(|error| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
                })?;
        }
    }
    let config = ChanMultiConfig {
        chan: parse_chan_config(
            min_stroke_bars,
            variant,
            "strict",
            "configurable",
            "dynamic",
        )?,
        ..ChanMultiConfig::default()
    };
    let result = py
        .detach(|| {
            analyze_chan_multi_timestamps_calendar(
                &timestamps,
                &open,
                &high,
                &low,
                &close,
                &volume,
                durations_seconds.as_deref().unwrap_or_default(),
                &config,
                &calendar,
                0,
            )
        })
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let resonance = result.resonance();
    let frames = pyo3::types::PyList::empty(py);
    for frame in result.frames {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("factor", frame.timeframe.factor)?;
        item.set_item("label", frame.timeframe.label)?;
        item.set_item("seconds", frame.timeframe.seconds)?;
        item.set_item("source_ranges", frame.source_ranges)?;
        item.set_item("analysis", chan_analysis_summary(py, &frame.analysis)?)?;
        frames.append(item)?;
    }
    let output = pyo3::types::PyDict::new(py);
    output.set_item("frames", frames)?;
    output.set_item(
        "resonance_direction",
        match resonance.direction {
            ::finkit::chan_mtf::ChanResonanceDirection::Bullish => "bullish",
            ::finkit::chan_mtf::ChanResonanceDirection::Bearish => "bearish",
            ::finkit::chan_mtf::ChanResonanceDirection::Mixed => "mixed",
            ::finkit::chan_mtf::ChanResonanceDirection::Unknown => "unknown",
        },
    )?;
    output.set_item("resonance_score", resonance.score)?;
    output.set_item("aligned_frames", resonance.aligned_frames)?;
    output.set_item("conflicting_frames", resonance.conflicting_frames)?;
    Ok(output.into())
}

/// Resolve a timestamp with a built-in market preset and optional calendar
/// overrides. The JSON/CSV resolvers remain available for exchange-published
/// definitions with source and revision metadata.
#[pyfunction(name = "resolve_market_session")]
#[pyo3(signature = (market, timestamp, timezone=None, holidays=None, sessions=None, special_sessions=None))]
fn resolve_market_session_py(
    py: Python<'_>,
    market: &str,
    timestamp: i64,
    timezone: Option<&str>,
    holidays: Option<Vec<String>>,
    sessions: Option<Vec<(u32, u32)>>,
    special_sessions: Option<Vec<(String, Vec<(u32, u32)>)>>,
) -> PyResult<Option<Py<PyAny>>> {
    let preset = MarketCalendarPreset::parse(market)
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let mut calendar = TradingCalendar::for_market(preset);
    if let Some(timezone) = timezone {
        calendar = calendar.with_timezone(TimeZoneSpec::parse(timezone).map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
        })?);
    }
    if let Some(holidays) = holidays {
        for holiday in holidays {
            calendar.add_holiday(&holiday).map_err(|error| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
            })?;
        }
    }
    if let Some(sessions) = sessions {
        let sessions: Vec<SessionWindow> = sessions
            .into_iter()
            .map(|(open, close)| SessionWindow::new(open, close))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
        calendar = calendar.with_sessions(&sessions);
    }
    if let Some(special_sessions) = special_sessions {
        for (date, sessions) in special_sessions {
            let sessions: Vec<SessionWindow> = sessions
                .into_iter()
                .map(|(open, close)| SessionWindow::new(open, close))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
                })?;
            calendar
                .set_special_sessions(&date, &sessions)
                .map_err(|error| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
                })?;
        }
    }
    let session = calendar
        .session_for_timestamp_local(timestamp)
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let Some(session) = session else {
        return Ok(None);
    };
    let output = pyo3::types::PyDict::new(py);
    output.set_item("session_day", session.session_day)?;
    output.set_item("session_index", session.session_index)?;
    output.set_item("open_timestamp", session.open_timestamp)?;
    output.set_item("close_timestamp", session.close_timestamp)?;
    output.set_item("market", preset.as_str())?;
    let timezone_name = match calendar.timezone() {
        TimeZoneSpec::Utc => "UTC".to_string(),
        TimeZoneSpec::AsiaShanghai => "Asia/Shanghai".to_string(),
        TimeZoneSpec::AsiaHongKong => "Asia/Hong_Kong".to_string(),
        TimeZoneSpec::AmericaNewYork => "America/New_York".to_string(),
        TimeZoneSpec::Fixed(offset_seconds) => format!("UTC{offset_seconds:+}"),
    };
    output.set_item("timezone", timezone_name)?;
    output.set_item("source", calendar.source())?;
    output.set_item("revision", calendar.revision())?;
    Ok(Some(output.into()))
}

/// Resolve a timestamp from a versioned JSON exchange-calendar definition.
#[pyfunction(name = "resolve_market_session_config")]
#[pyo3(signature = (config_json, timestamp))]
fn resolve_market_session_config_py(
    py: Python<'_>,
    config_json: &str,
    timestamp: i64,
) -> PyResult<Option<Py<PyAny>>> {
    let calendar = TradingCalendar::from_json(config_json)
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let session = calendar
        .session_for_timestamp_local(timestamp)
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let Some(session) = session else {
        return Ok(None);
    };
    let output = pyo3::types::PyDict::new(py);
    output.set_item("session_day", session.session_day)?;
    output.set_item("session_index", session.session_index)?;
    output.set_item("open_timestamp", session.open_timestamp)?;
    output.set_item("close_timestamp", session.close_timestamp)?;
    output.set_item("source", calendar.source())?;
    output.set_item("revision", calendar.revision())?;
    Ok(Some(output.into()))
}

/// Resolve a timestamp from an exchange-published annual CSV calendar.
#[pyfunction(name = "resolve_market_session_csv")]
#[pyo3(signature = (csv, market, timestamp, timezone=None))]
fn resolve_market_session_csv_py(
    py: Python<'_>,
    csv: &str,
    market: &str,
    timestamp: i64,
    timezone: Option<&str>,
) -> PyResult<Option<Py<PyAny>>> {
    let calendar = TradingCalendar::from_csv(csv, Some(market), timezone)
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let session = calendar
        .session_for_timestamp_local(timestamp)
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?;
    let Some(session) = session else {
        return Ok(None);
    };
    let output = pyo3::types::PyDict::new(py);
    output.set_item("session_day", session.session_day)?;
    output.set_item("session_index", session.session_index)?;
    output.set_item("open_timestamp", session.open_timestamp)?;
    output.set_item("close_timestamp", session.close_timestamp)?;
    output.set_item("source", calendar.source())?;
    output.set_item("revision", calendar.revision())?;
    Ok(Some(output.into()))
}

fn chan_analysis_summary<'py>(
    py: Python<'py>,
    analysis: &::finkit::chan::ChanAnalysis,
) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
    let result = pyo3::types::PyDict::new(py);
    result.set_item("bar_count", analysis.bars.len())?;
    result.set_item("fractal_count", analysis.fractals.len())?;
    result.set_item("stroke_count", analysis.strokes.len())?;
    result.set_item("segment_count", analysis.segments.len())?;
    result.set_item("center_count", analysis.centers.len())?;
    result.set_item(
        "trend",
        match analysis.trend {
            ::finkit::chan::ChanTrend::Unknown => "unknown",
            ::finkit::chan::ChanTrend::Bullish => "bullish",
            ::finkit::chan::ChanTrend::Bearish => "bearish",
            ::finkit::chan::ChanTrend::Range => "range",
        },
    )?;
    let signals = pyo3::types::PyList::empty(py);
    for signal in &analysis.signals {
        let item = pyo3::types::PyDict::new(py);
        item.set_item("index", signal.index)?;
        item.set_item("price", signal.price)?;
        item.set_item("strength", signal.strength)?;
        item.set_item("confirmed", signal.confirmed)?;
        item.set_item("reason", signal.reason.as_str())?;
        item.set_item("evidence", signal.evidence.clone())?;
        signals.append(item)?;
    }
    result.set_item("signals", signals)?;
    let divergences = pyo3::types::PyList::empty(py);
    for divergence in &analysis.divergences {
        let item = pyo3::types::PyDict::new(py);
        item.set_item(
            "kind",
            match divergence.kind {
                ::finkit::chan::ChanDivergenceKind::Bullish => "bullish",
                ::finkit::chan::ChanDivergenceKind::Bearish => "bearish",
            },
        )?;
        item.set_item("start_stroke", divergence.start_stroke)?;
        item.set_item("end_stroke", divergence.end_stroke)?;
        item.set_item("index", divergence.index)?;
        item.set_item("price", divergence.price)?;
        item.set_item("confirmed", divergence.confirmed)?;
        item.set_item("strength", divergence.strength)?;
        item.set_item("reason", divergence.reason.as_str())?;
        divergences.append(item)?;
    }
    result.set_item("divergences", divergences)?;
    Ok(result)
}

/// Evaluate a dependency-aware graph of custom composite indicators.
///
/// `definitions` uses `(name, function, inputs, params)` tuples. Inputs refer
/// to OHLCV/raw series names or definition names; `const:<n>` creates a
/// broadcast scalar. Built-ins include `sma`, `ema`, `wma`, `rsi`, `atr`,
/// `macd`, `boll_mid`, `boll_upper`, `boll_lower`, `vwma`, `return`,
/// `zscore`, `cross_up`, and `cross_down`. Element-wise functions `add`,
/// `sub`, `mul`, `div`, `min`, `max`, and `weighted_average` compose outputs;
/// weighted-average parameters are supplied in the same order as its inputs.
#[pyfunction]
#[pyo3(signature = (close, definitions, outputs=None, open=None, high=None, low=None, volume=None))]
fn compute_composite<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'_, f64>,
    definitions: Vec<(String, String, Vec<String>, Vec<f64>)>,
    outputs: Option<Vec<String>>,
    open: Option<PyReadonlyArray1<'_, f64>>,
    high: Option<PyReadonlyArray1<'_, f64>>,
    low: Option<PyReadonlyArray1<'_, f64>>,
    volume: Option<PyReadonlyArray1<'_, f64>>,
) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
    let close = close
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?
        .to_vec();
    let open = optional_array_to_vec(open)?;
    let high = optional_array_to_vec(high)?;
    let low = optional_array_to_vec(low)?;
    let volume = optional_array_to_vec(volume)?;
    let names: std::collections::BTreeSet<String> =
        definitions.iter().map(|item| item.0.clone()).collect();
    if names.len() != definitions.len() {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "composite definition names must be unique",
        ));
    }
    let definitions = definitions
        .into_iter()
        .map(|(name, function, inputs, params)| {
            let expression_inputs = inputs
                .into_iter()
                .map(|input| composite_input_expression(&input, &names))
                .collect::<PyResult<Vec<_>>>()?;
            let expression = match function.to_ascii_lowercase().as_str() {
                "add" => CompositeExpr::Op {
                    op: CompositeOp::Add,
                    inputs: expression_inputs,
                },
                "sub" => CompositeExpr::Op {
                    op: CompositeOp::Sub,
                    inputs: expression_inputs,
                },
                "mul" => CompositeExpr::Op {
                    op: CompositeOp::Mul,
                    inputs: expression_inputs,
                },
                "div" => CompositeExpr::Op {
                    op: CompositeOp::Div,
                    inputs: expression_inputs,
                },
                "min" => CompositeExpr::Op {
                    op: CompositeOp::Min,
                    inputs: expression_inputs,
                },
                "max" => CompositeExpr::Op {
                    op: CompositeOp::Max,
                    inputs: expression_inputs,
                },
                "weighted_average" | "weightedaverage" => {
                    CompositeExpr::call("weighted_average", expression_inputs, params)
                }
                _ => CompositeExpr::call(function, expression_inputs, params),
            };
            Ok(CompositeDefinition::new(name, expression))
        })
        .collect::<PyResult<Vec<_>>>()?;
    let output_names = outputs.unwrap_or_else(|| {
        definitions
            .iter()
            .map(|definition| definition.name.clone())
            .collect()
    });
    let output_refs: Vec<&str> = output_names.iter().map(String::as_str).collect();

    let result = py
        .detach(|| -> Result<_, String> {
            let mut context = FactorContext::new();
            context
                .insert("close", close)
                .map_err(|error| error.to_string())?;
            if let Some(values) = open {
                context
                    .insert("open", values)
                    .map_err(|error| error.to_string())?;
            }
            if let Some(values) = high {
                context
                    .insert("high", values)
                    .map_err(|error| error.to_string())?;
            }
            if let Some(values) = low {
                context
                    .insert("low", values)
                    .map_err(|error| error.to_string())?;
            }
            if let Some(values) = volume {
                context
                    .insert("volume", values)
                    .map_err(|error| error.to_string())?;
            }
            CompositeEngine::new()
                .evaluate(&definitions, &output_refs, &context)
                .map_err(|error| error.to_string())
        })
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(error))?;

    let dict = pyo3::types::PyDict::new(py);
    for (name, values) in result {
        dict.set_item(name, values)?;
    }
    Ok(dict)
}

fn optional_array_to_vec(array: Option<PyReadonlyArray1<'_, f64>>) -> PyResult<Option<Vec<f64>>> {
    array
        .map(|array| {
            array
                .as_slice()
                .map(|values| values.to_vec())
                .map_err(|error| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
                })
        })
        .transpose()
}

fn composite_input_expression(
    input: &str,
    definition_names: &std::collections::BTreeSet<String>,
) -> PyResult<CompositeExpr> {
    if let Some(value) = input.strip_prefix("const:") {
        let value = value.parse::<f64>().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "invalid composite constant: {input}"
            ))
        })?;
        return Ok(CompositeExpr::Constant(value));
    }
    if definition_names.contains(input) {
        Ok(CompositeExpr::reference(input))
    } else {
        Ok(CompositeExpr::series(input))
    }
}

#[pyclass]
#[derive(Clone)]
struct PyKlineData {
    inner: KlineData,
}

#[pymethods]
impl PyKlineData {
    #[new]
    #[pyo3(signature = (dates, opens, highs, lows, closes, volumes, timestamps=None))]
    fn new(
        dates: Vec<String>,
        opens: Vec<f64>,
        highs: Vec<f64>,
        lows: Vec<f64>,
        closes: Vec<f64>,
        volumes: Vec<f64>,
        timestamps: Option<Vec<i64>>,
    ) -> PyResult<Self> {
        let mut inner = KlineData::new(dates, opens, highs, lows, closes, volumes);
        if let Some(timestamps) = timestamps {
            if !inner.set_timestamps(timestamps) {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "timestamps must be strictly increasing and match data length",
                ));
            }
        }
        Ok(Self { inner })
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn validate(&self) -> bool {
        self.inner.validate()
    }

    fn validate_ohlcv(&self) -> bool {
        self.inner.validate_ohlcv()
    }

    fn validation_errors(&self) -> Vec<String> {
        self.inner.validation_errors()
    }

    fn set_timestamps(&mut self, timestamps: Vec<i64>) -> PyResult<()> {
        if self.inner.set_timestamps(timestamps) {
            Ok(())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "timestamps must be strictly increasing and match data length",
            ))
        }
    }

    fn push(&mut self, date: String, open: f64, high: f64, low: f64, close: f64, volume: f64) {
        self.inner.push(date, open, high, low, close, volume);
    }

    fn push_timestamped(
        &mut self,
        timestamp: i64,
        date: String,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> PyResult<()> {
        if self
            .inner
            .push_timestamped(timestamp, date, open, high, low, close, volume)
        {
            Ok(())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "timestamped push requires a complete, strictly increasing timestamp index",
            ))
        }
    }

    #[staticmethod]
    fn from_json(json_str: &str) -> PyResult<Self> {
        KlineData::from_json(json_str)
            .map(|d| Self { inner: d })
            .map_err(convert_vis_error)
    }

    #[staticmethod]
    fn from_csv(csv_str: &str) -> PyResult<Self> {
        KlineData::from_csv(csv_str)
            .map(|d| Self { inner: d })
            .map_err(convert_vis_error)
    }

    #[getter]
    fn dates(&self) -> Vec<String> {
        self.inner.dates().to_vec()
    }

    #[getter]
    fn opens(&self) -> Vec<f64> {
        self.inner.opens().to_vec()
    }

    #[getter]
    fn highs(&self) -> Vec<f64> {
        self.inner.highs().to_vec()
    }

    #[getter]
    fn lows(&self) -> Vec<f64> {
        self.inner.lows().to_vec()
    }

    #[getter]
    fn closes(&self) -> Vec<f64> {
        self.inner.closes().to_vec()
    }

    #[getter]
    fn volumes(&self) -> Vec<f64> {
        self.inner.volumes().to_vec()
    }

    #[getter]
    fn timestamps(&self) -> Vec<i64> {
        self.inner.timestamps.clone()
    }
}

fn validate_stream_bar(open: f64, high: f64, low: f64, close: f64, volume: f64) -> PyResult<()> {
    let candidate = KlineData::new(
        vec!["live".to_string()],
        vec![open],
        vec![high],
        vec![low],
        vec![close],
        vec![volume],
    );
    let errors = candidate.validation_errors();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            errors.join("; "),
        ))
    }
}

#[pyclass]
struct PyKlineChart {
    data: PyKlineData,
    indicators: Vec<IndicatorConfig>,
    custom_indicator_series: Vec<(String, Vec<f64>)>,
    event_markers: Vec<EventMarker>,
    config: ChartConfig,
    viewport: Viewport,
    replay: ReplayState,
    lod_policy: LodPolicy,
    hidden_layers: Vec<String>,
    chan_multi: Option<ChanMultiConfig>,
}

#[pymethods]
impl PyKlineChart {
    #[new]
    #[pyo3(signature = (data, language="zh", title="", width=1200, height=600))]
    fn new(
        data: PyKlineData,
        language: &str,
        title: &str,
        width: u32,
        height: u32,
    ) -> PyResult<Self> {
        let lang = match language {
            "en" => Language::EnUs,
            _ => Language::ZhCn,
        };
        let config = ChartConfigBuilder::new()
            .with_title(title)
            .with_language(lang)
            .with_dimensions(width, height)
            .build();
        let data_len = data.inner.len();
        Ok(Self {
            data,
            indicators: Vec::new(),
            custom_indicator_series: Vec::new(),
            event_markers: Vec::new(),
            config,
            viewport: Viewport::full(),
            replay: ReplayState::new(data_len, 200),
            lod_policy: LodPolicy::Auto,
            hidden_layers: Vec::new(),
            chan_multi: None,
        })
    }

    /// Restrict rendering to a source-index window. `end=0` restores full data.
    #[pyo3(signature = (start=0, end=0, pixel_width=1200, pixel_height=600, overscan_bars=0, follow_latest=false))]
    fn set_viewport(
        &mut self,
        start: usize,
        end: usize,
        pixel_width: u32,
        pixel_height: u32,
        overscan_bars: usize,
        follow_latest: bool,
    ) {
        self.viewport = if end == 0 {
            Viewport::full()
        } else {
            Viewport::new(start, end)
                .with_pixels(pixel_width, pixel_height)
                .with_overscan(overscan_bars)
                .with_follow_latest(follow_latest)
        };
    }

    /// Append a validated bar to the live chart source.
    fn append_kline(
        &mut self,
        date: &str,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> PyResult<()> {
        validate_stream_bar(open, high, low, close, volume)?;
        self.data
            .inner
            .push(date.to_string(), open, high, low, close, volume);
        for (_, values) in &mut self.custom_indicator_series {
            values.push(f64::NAN);
        }
        Ok(())
    }

    /// Replace the current live bar, expanding high/low when omitted.
    #[pyo3(signature = (close, high=None, low=None, volume=None))]
    fn update_last_kline(
        &mut self,
        close: f64,
        high: Option<f64>,
        low: Option<f64>,
        volume: Option<f64>,
    ) -> PyResult<()> {
        if self.data.inner.is_empty() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "cannot update an empty chart",
            ));
        }
        let index = self.data.inner.len() - 1;
        let next_high = high.unwrap_or(self.data.inner.highs[index].max(close));
        let next_low = low.unwrap_or(self.data.inner.lows[index].min(close));
        let next_volume = volume.unwrap_or(self.data.inner.volumes[index]);
        validate_stream_bar(
            self.data.inner.opens[index],
            next_high,
            next_low,
            close,
            next_volume,
        )?;
        self.data.inner.highs[index] = next_high;
        self.data.inner.lows[index] = next_low;
        self.data.inner.closes[index] = close;
        self.data.inner.volumes[index] = next_volume;
        self.data.inner.bump_revision();
        for (_, values) in &mut self.custom_indicator_series {
            if values.len() > index {
                values[index] = f64::NAN;
            }
        }
        Ok(())
    }

    /// Append a new bar or revise the current bar when the date is repeated.
    fn upsert_kline(
        &mut self,
        date: &str,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> PyResult<String> {
        let same_date = self
            .data
            .inner
            .dates
            .last()
            .is_some_and(|last| last == date);
        if same_date {
            validate_stream_bar(open, high, low, close, volume)?;
            let index = self.data.inner.len() - 1;
            self.data.inner.opens[index] = open;
            self.data.inner.highs[index] = high;
            self.data.inner.lows[index] = low;
            self.data.inner.closes[index] = close;
            self.data.inner.volumes[index] = volume;
            self.data.inner.bump_revision();
            for (_, values) in &mut self.custom_indicator_series {
                if values.len() > index {
                    values[index] = f64::NAN;
                }
            }
            Ok("updated".to_string())
        } else {
            self.append_kline(date, open, high, low, close, volume)?;
            Ok("appended".to_string())
        }
    }

    /// Timestamp-preserving variant for exchange-calendar-aware live feeds.
    fn upsert_kline_timestamped(
        &mut self,
        timestamp: i64,
        date: &str,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> PyResult<String> {
        validate_stream_bar(open, high, low, close, volume)?;
        if self.data.inner.timestamps.len() != self.data.inner.len()
            || self
                .data
                .inner
                .timestamps
                .last()
                .is_some_and(|last| timestamp < *last)
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "timestamp must extend the existing timestamp index",
            ));
        }
        let same_date = self
            .data
            .inner
            .dates
            .last()
            .is_some_and(|last| last == date);
        if same_date {
            let index = self.data.inner.len() - 1;
            self.data.inner.opens[index] = open;
            self.data.inner.highs[index] = high;
            self.data.inner.lows[index] = low;
            self.data.inner.closes[index] = close;
            self.data.inner.volumes[index] = volume;
            self.data.inner.timestamps[index] = timestamp;
            self.data.inner.bump_revision();
            for (_, values) in &mut self.custom_indicator_series {
                if values.len() > index {
                    values[index] = f64::NAN;
                }
            }
            Ok("updated".to_string())
        } else if self.data.inner.push_timestamped(
            timestamp,
            date.to_string(),
            open,
            high,
            low,
            close,
            volume,
        ) {
            for (_, values) in &mut self.custom_indicator_series {
                values.push(f64::NAN);
            }
            Ok("appended".to_string())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "timestamped update must extend the existing timestamp index",
            ))
        }
    }

    /// Apply live quotes in one call and defer chart rendering until export.
    fn upsert_klines(
        &mut self,
        updates: Vec<(String, f64, f64, f64, f64, f64)>,
    ) -> PyResult<Vec<String>> {
        updates
            .into_iter()
            .map(|(date, open, high, low, close, volume)| {
                self.upsert_kline(&date, open, high, low, close, volume)
            })
            .collect()
    }

    /// Selects `auto`, `raw`, `balanced` or `overview` level of detail.
    #[pyo3(signature = (level="auto"))]
    fn set_lod_policy(&mut self, level: &str) -> PyResult<()> {
        self.lod_policy = match level.to_ascii_lowercase().as_str() {
            "auto" => LodPolicy::Auto,
            "raw" => LodPolicy::Fixed(LodLevel::Raw),
            "balanced" => LodPolicy::Fixed(LodLevel::Balanced),
            "overview" => LodPolicy::Fixed(LodLevel::Overview),
            value => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "unknown LOD policy: {value}"
                )))
            }
        };
        Ok(())
    }

    /// Set a deterministic replay window and return its resolved source range.
    #[pyo3(signature = (window=200, cursor=None, pixel_width=1200, pixel_height=600, overscan_bars=0))]
    fn set_replay_window(
        &mut self,
        window: usize,
        cursor: Option<usize>,
        pixel_width: u32,
        pixel_height: u32,
        overscan_bars: usize,
    ) -> (usize, usize) {
        self.replay.window = window.max(1);
        if let Some(cursor) = cursor {
            self.replay.seek(cursor, self.data.inner.len());
        } else {
            self.replay.reset(self.data.inner.len());
        }
        let (start, end) = self.replay.visible_range(self.data.inner.len());
        self.viewport = Viewport::new(start, end)
            .with_pixels(pixel_width, pixel_height)
            .with_overscan(overscan_bars);
        (start, end)
    }

    /// Advance the replay by one configured step and return its source range.
    fn replay_next(&mut self) -> Option<(usize, usize)> {
        self.replay.advance(self.data.inner.len())?;
        let range = self.replay.visible_range(self.data.inner.len());
        self.viewport = Viewport::new(range.0, range.1);
        Some(range)
    }

    /// Enables or disables a semantic chart layer.
    fn set_layer_visible(&mut self, layer: &str, visible: bool) {
        if visible {
            self.hidden_layers.retain(|value| value != layer);
        } else if !self.hidden_layers.iter().any(|value| value == layer) {
            self.hidden_layers.push(layer.to_string());
        }
    }

    fn add_ma(&mut self, periods: Vec<usize>) {
        self.indicators.push(IndicatorConfig::new(
            IndicatorType::MA,
            periods.iter().map(|&p| p as f64).collect(),
        ));
    }

    fn add_ema(&mut self, periods: Vec<usize>) {
        self.indicators.push(IndicatorConfig::new(
            IndicatorType::EMA,
            periods.iter().map(|&p| p as f64).collect(),
        ));
    }

    #[pyo3(signature = (period=20, nb_dev=2.0))]
    fn add_boll(&mut self, period: usize, nb_dev: f64) {
        self.indicators.push(IndicatorConfig::new(
            IndicatorType::BOLL,
            vec![period as f64, nb_dev],
        ));
    }

    #[pyo3(signature = (fast=12, slow=26, signal=9))]
    fn add_macd(&mut self, fast: usize, slow: usize, signal: usize) {
        self.indicators.push(IndicatorConfig::new(
            IndicatorType::MACD,
            vec![fast as f64, slow as f64, signal as f64],
        ));
    }

    #[pyo3(signature = (period=14))]
    fn add_rsi(&mut self, period: usize) {
        self.indicators.push(IndicatorConfig::new(
            IndicatorType::RSI,
            vec![period as f64],
        ));
    }

    #[pyo3(signature = (fast_k=9, slow_k=3, slow_d=3))]
    fn add_kdj(&mut self, fast_k: usize, slow_k: usize, slow_d: usize) {
        self.indicators.push(IndicatorConfig::new(
            IndicatorType::KDJ,
            vec![fast_k as f64, slow_k as f64, slow_d as f64],
        ));
    }

    #[pyo3(signature = (acceleration=0.02, maximum=0.2))]
    fn add_sar(&mut self, acceleration: f64, maximum: f64) {
        self.indicators.push(IndicatorConfig::new(
            IndicatorType::Custom("SAR".to_string()),
            vec![acceleration, maximum],
        ));
    }

    /// Adds a user-supplied indicator line to the chart and the HTML data window.
    /// Values must align one-to-one with the source K-line rows.
    fn add_custom_indicator(&mut self, name: &str, values: Vec<f64>) -> PyResult<()> {
        self.set_custom_indicator_series(name, values)
    }

    /// Replaces or registers a user-supplied indicator line.
    ///
    /// This is the real-time update path: after an append/upsert, callers can
    /// submit the recalculated aligned series without creating duplicate
    /// indicator definitions.
    fn set_custom_indicator_series(&mut self, name: &str, values: Vec<f64>) -> PyResult<()> {
        if name.trim().is_empty() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "custom indicator name must not be empty",
            ));
        }
        if values.len() != self.data.inner.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "custom indicator '{}' has {} values, expected {}",
                name,
                values.len(),
                self.data.inner.len()
            )));
        }
        if !self.indicators.iter().any(|indicator| {
            matches!(&indicator.indicator_type, IndicatorType::Custom(existing) if existing == name)
        }) {
            self.indicators.push(IndicatorConfig::new(
                IndicatorType::Custom(name.to_string()),
                Vec::new(),
            ));
        }
        self.custom_indicator_series
            .retain(|(existing, _)| existing != name);
        self.custom_indicator_series
            .push((name.to_string(), values));
        Ok(())
    }

    /// Adds a semantic event marker to the main chart panel.
    #[pyo3(signature = (index, label, value=None, color="#f59e0b", priority=30))]
    fn add_event_marker(
        &mut self,
        index: usize,
        label: &str,
        value: Option<f64>,
        color: &str,
        priority: i32,
    ) {
        let mut marker = EventMarker::new(index, label)
            .with_color(Color::from_hex(color))
            .with_priority(priority);
        if let Some(value) = value {
            marker = marker.with_value(value);
        }
        self.event_markers.push(marker);
    }

    /// Configures TDX-style crosshair, data-window, pan/zoom and keyboard input.
    #[pyo3(signature = (enabled=true, show_crosshair=true, show_data_window=true, enable_pan_zoom=true, enable_keyboard=true))]
    fn set_interaction(
        &mut self,
        enabled: bool,
        show_crosshair: bool,
        show_data_window: bool,
        enable_pan_zoom: bool,
        enable_keyboard: bool,
    ) {
        self.config.interaction.enabled = enabled;
        self.config.interaction.show_crosshair = show_crosshair;
        self.config.interaction.show_data_window = show_data_window;
        self.config.interaction.enable_pan_zoom = enable_pan_zoom;
        self.config.interaction.enable_keyboard = enable_keyboard;
    }

    /// Enables the Chanlun overlay on the main price panel.
    #[pyo3(signature = (min_stroke_bars=6, show_labels=false, variant="standard", stroke_policy="configurable", center_policy="dynamic", signal_min_strength=0.0, show_multi_timeframe_annotations=false))]
    fn add_chan(
        &mut self,
        min_stroke_bars: usize,
        show_labels: bool,
        variant: &str,
        stroke_policy: &str,
        center_policy: &str,
        signal_min_strength: f64,
        show_multi_timeframe_annotations: bool,
    ) {
        self.config.chan.enabled = true;
        self.config.chan.min_stroke_bars = min_stroke_bars;
        self.config.chan.show_labels = show_labels;
        self.config.chan.variant = variant.to_string();
        self.config.chan.stroke_policy = stroke_policy.to_string();
        self.config.chan.center_policy = center_policy.to_string();
        self.config.chan.signal_min_strength = signal_min_strength;
        self.config.chan.show_multi_timeframe_annotations = show_multi_timeframe_annotations;
        self.chan_multi = None;
    }

    /// Enable automatic or explicit multi-timeframe Chanlun overlays.
    #[pyo3(signature = (factors=None, variant="standard"))]
    fn add_chan_multi(&mut self, factors: Option<Vec<usize>>, variant: &str) -> PyResult<()> {
        self.config.chan.enabled = true;
        self.config.chan.variant = variant.to_string();
        self.config.chan.show_multi_timeframe_annotations = true;
        let mut chan = parse_chan_config(
            self.config.chan.min_stroke_bars,
            variant,
            &self.config.chan.fractal_policy,
            &self.config.chan.stroke_policy,
            &self.config.chan.center_policy,
        )?;
        chan.thresholds.min_stroke_change_ratio = self.config.chan.min_stroke_change_ratio;
        chan.thresholds.min_fractal_range_ratio = self.config.chan.min_fractal_range_ratio;
        chan.thresholds.signal_min_strength = self.config.chan.signal_min_strength;
        chan.thresholds.center_break_ratio = self.config.chan.center_break_ratio;
        self.chan_multi = Some(ChanMultiConfig {
            factors: factors.unwrap_or_default(),
            chan,
            ..ChanMultiConfig::default()
        });
        Ok(())
    }

    /// Configure numeric Chan thresholds without changing the selected variant.
    fn set_chan_thresholds(
        &mut self,
        min_stroke_change_ratio: f64,
        min_fractal_range_ratio: f64,
        signal_min_strength: f64,
        center_break_ratio: f64,
    ) {
        self.config.chan.min_stroke_change_ratio = min_stroke_change_ratio;
        self.config.chan.min_fractal_range_ratio = min_fractal_range_ratio;
        self.config.chan.signal_min_strength = signal_min_strength;
        self.config.chan.center_break_ratio = center_break_ratio;
        if let Some(config) = &mut self.chan_multi {
            config.chan.thresholds.min_stroke_change_ratio = min_stroke_change_ratio;
            config.chan.thresholds.min_fractal_range_ratio = min_fractal_range_ratio;
            config.chan.thresholds.signal_min_strength = signal_min_strength;
            config.chan.thresholds.center_break_ratio = center_break_ratio;
        }
    }

    fn save_as_svg(&self, py: Python<'_>, path: &str) -> PyResult<()> {
        let data = self.data.inner.clone();
        let config = self.config.clone();
        let indicators = self.indicators.clone();
        let svg = py.detach(|| -> PyResult<String> {
            let mut chart = finkit_visualization::chart::KlineChart::new(config);
            self.configure_chart(&mut chart);
            chart
                .build_draw_list(&data, &indicators)
                .map_err(convert_vis_error)?;
            chart.to_svg_string().map_err(convert_vis_error)
        })?;
        std::fs::write(path, svg)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("{}", e)))
    }

    fn save_as_html(&self, py: Python<'_>, path: &str) -> PyResult<()> {
        let data = self.data.inner.clone();
        let config = self.config.clone();
        let indicators = self.indicators.clone();
        let html = py.detach(|| -> PyResult<String> {
            let mut chart = finkit_visualization::chart::KlineChart::new(config);
            self.configure_chart(&mut chart);
            chart
                .build_draw_list(&data, &indicators)
                .map_err(convert_vis_error)?;
            chart.to_html_string().map_err(convert_vis_error)
        })?;
        std::fs::write(path, html)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("{}", e)))
    }

    /// Save a high-volume Canvas 2D command-stream HTML document.
    fn save_as_canvas_html(&self, py: Python<'_>, path: &str) -> PyResult<()> {
        let data = self.data.inner.clone();
        let config = self.config.clone();
        let indicators = self.indicators.clone();
        let html = py.detach(|| -> PyResult<String> {
            let mut chart = finkit_visualization::chart::KlineChart::new(config);
            self.configure_chart(&mut chart);
            chart
                .build_draw_list(&data, &indicators)
                .map_err(convert_vis_error)?;
            chart.to_canvas_html_string().map_err(convert_vis_error)
        })?;
        std::fs::write(path, html)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("{}", e)))
    }

    fn to_svg_string(&self, py: Python<'_>) -> PyResult<String> {
        let data = self.data.inner.clone();
        let config = self.config.clone();
        let indicators = self.indicators.clone();
        py.detach(|| {
            let mut chart = finkit_visualization::chart::KlineChart::new(config);
            self.configure_chart(&mut chart);
            chart
                .build_draw_list(&data, &indicators)
                .map_err(convert_vis_error)?;
            chart.to_svg_string().map_err(convert_vis_error)
        })
    }

    /// Return a high-volume Canvas 2D command-stream HTML document.
    fn to_canvas_html(&self, py: Python<'_>) -> PyResult<String> {
        let data = self.data.inner.clone();
        let config = self.config.clone();
        let indicators = self.indicators.clone();
        py.detach(|| {
            let mut chart = finkit_visualization::chart::KlineChart::new(config);
            self.configure_chart(&mut chart);
            chart
                .build_draw_list(&data, &indicators)
                .map_err(convert_vis_error)?;
            chart.to_canvas_html_string().map_err(convert_vis_error)
        })
    }

    /// Return an instanced WebGL2 HTML document with a Canvas overlay.
    fn to_webgl_html(&self, py: Python<'_>) -> PyResult<String> {
        let data = self.data.inner.clone();
        let config = self.config.clone();
        let indicators = self.indicators.clone();
        py.detach(|| {
            let mut chart = finkit_visualization::chart::KlineChart::new(config);
            self.configure_chart(&mut chart);
            chart
                .build_draw_list(&data, &indicators)
                .map_err(convert_vis_error)?;
            chart.to_webgl_html_string().map_err(convert_vis_error)
        })
    }

    /// Save an instanced WebGL2 HTML document.
    fn save_as_webgl_html(&self, py: Python<'_>, path: &str) -> PyResult<()> {
        let html = self.to_webgl_html(py)?;
        std::fs::write(path, html)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("{e}")))
    }

    /// Return an explicitly WebGPU-preferred HTML document.
    fn to_webgpu_html(&self, py: Python<'_>) -> PyResult<String> {
        let data = self.data.inner.clone();
        let config = self.config.clone();
        let indicators = self.indicators.clone();
        py.detach(|| {
            let mut chart = finkit_visualization::chart::KlineChart::new(config);
            self.configure_chart(&mut chart);
            chart
                .build_draw_list(&data, &indicators)
                .map_err(convert_vis_error)?;
            chart.to_webgpu_html_string().map_err(convert_vis_error)
        })
    }

    /// Save an explicitly WebGPU-preferred HTML document.
    fn save_as_webgpu_html(&self, py: Python<'_>, path: &str) -> PyResult<()> {
        let html = self.to_webgpu_html(py)?;
        std::fs::write(path, html)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("{e}")))
    }
}

impl PyKlineChart {
    fn configure_chart(&self, chart: &mut finkit_visualization::chart::KlineChart) {
        chart.set_data(self.data.inner.clone());
        chart.set_viewport(self.viewport);
        chart.set_lod_policy(self.lod_policy);
        for layer in &self.hidden_layers {
            chart.set_layer_visible(layer, false);
        }
        for (name, values) in &self.custom_indicator_series {
            let _ = chart.set_custom_indicator_series(name.clone(), values.clone());
        }
        for marker in &self.event_markers {
            chart.add_event_marker(marker.clone());
        }
        if let Some(config) = &self.chan_multi {
            let _ = chart.analyze_and_set_chan_multi(config.clone());
        }
    }
}

// ============================================================================
// Batch Computation (Single GIL Release)
// ============================================================================

/// Indicator request for batch computation.
///
/// Each request specifies an indicator name and its parameters.
#[derive(Debug, Clone)]
struct IndicatorRequest {
    name: String,
    params: Vec<f64>,
}

/// Parse indicator requests from Python input.
///
/// Input format: list of dicts with "name" and "params" keys.
/// Example: [{"name": "sma", "params": [14]}, {"name": "ema", "params": [20]}]
fn parse_indicator_requests(requests: Vec<(String, Vec<f64>)>) -> Vec<IndicatorRequest> {
    requests
        .into_iter()
        .map(|(name, params)| IndicatorRequest { name, params })
        .collect()
}

/// Compute multiple indicators in a single GIL release.
///
/// This function accepts OHLCV data and a list of indicator requests,
/// computes all indicators in one batch without repeated GIL acquisition,
/// and returns results as a dictionary.
///
/// # Arguments
/// * `open` - Open prices (optional, required for some indicators like BOP)
/// * `high` - High prices (optional, required for indicators like ADX, Stoch)
/// * `low` - Low prices (optional, required for indicators like ADX, Stoch)
/// * `close` - Close prices (required for most indicators)
/// * `volume` - Volume data (optional, required for indicators like OBV, MFI)
/// * `requests` - List of (indicator_name, params) tuples
/// * `talib_compat` - Apply TA-Lib Python lookback, NaN and index conventions
///   without changing the default native finkit semantics.
///
/// # Returns
/// Dictionary mapping indicator names (with params suffix) to computed values.
///
/// # Example
/// ```python
/// import numpy as np
/// import finkit_python as ta
///
/// close = np.array([1.0, 2.0, 3.0, ...], dtype=np.float64)
/// requests = [("sma", [14]), ("ema", [20]), ("rsi", [14])]
/// results = ta.compute_indicators(close=close, requests=requests)
/// print(results["sma_14"])
/// ```
#[pyfunction]
#[pyo3(signature = (close, requests, open=None, high=None, low=None, volume=None, secondary=None, talib_compat=false))]
fn compute_indicators<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'_, f64>,
    requests: Vec<(String, Vec<f64>)>,
    open: Option<PyReadonlyArray1<'_, f64>>,
    high: Option<PyReadonlyArray1<'_, f64>>,
    low: Option<PyReadonlyArray1<'_, f64>>,
    volume: Option<PyReadonlyArray1<'_, f64>>,
    secondary: Option<PyReadonlyArray1<'_, f64>>,
    talib_compat: bool,
) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
    let close_slice = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;

    let open_vec: Option<Vec<f64>> = open.as_ref().map(|arr| arr.as_array().to_vec());
    let high_vec: Option<Vec<f64>> = high.as_ref().map(|arr| arr.as_array().to_vec());
    let low_vec: Option<Vec<f64>> = low.as_ref().map(|arr| arr.as_array().to_vec());
    let volume_vec: Option<Vec<f64>> = volume.as_ref().map(|arr| arr.as_array().to_vec());
    let secondary_vec: Option<Vec<f64>> = secondary.as_ref().map(|arr| arr.as_array().to_vec());

    let indicator_requests = parse_indicator_requests(requests);

    let results: Vec<(String, IndicatorResult)> = py.detach(|| {
        compute_all_indicators(
            open_vec.as_deref(),
            high_vec.as_deref(),
            low_vec.as_deref(),
            close_slice,
            volume_vec.as_deref(),
            secondary_vec.as_deref(),
            &indicator_requests,
            talib_compat,
        )
    });

    let dict = pyo3::types::PyDict::new(py);
    for (key, value) in results {
        match value {
            IndicatorResult::Single(arr) => {
                dict.set_item(key, arr)?;
            }
            IndicatorResult::Double(a, b) => {
                dict.set_item(format!("{}_0", key), a)?;
                dict.set_item(format!("{}_1", key), b)?;
            }
            IndicatorResult::Triple(a, b, c) => {
                dict.set_item(format!("{}_0", key), a)?;
                dict.set_item(format!("{}_1", key), b)?;
                dict.set_item(format!("{}_2", key), c)?;
            }
            IndicatorResult::Quad(a, b, c, d) => {
                dict.set_item(format!("{}_0", key), a)?;
                dict.set_item(format!("{}_1", key), b)?;
                dict.set_item(format!("{}_2", key), c)?;
                dict.set_item(format!("{}_3", key), d)?;
            }
            IndicatorResult::Error(msg) => {
                dict.set_item(format!("{}_error", key), msg)?;
            }
        }
    }
    Ok(dict)
}

/// Result type for indicator computation.
enum IndicatorResult {
    Single(Vec<f64>),
    Double(Vec<f64>, Vec<f64>),
    Triple(Vec<f64>, Vec<f64>, Vec<f64>),
    Quad(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>),
    Error(String),
}

/// Compute all indicators in batch (called inside detach).
fn compute_all_indicators(
    open: Option<&[f64]>,
    high: Option<&[f64]>,
    low: Option<&[f64]>,
    close: &[f64],
    volume: Option<&[f64]>,
    secondary: Option<&[f64]>,
    requests: &[IndicatorRequest],
    talib_compat: bool,
) -> Vec<(String, IndicatorResult)> {
    let mut results = Vec::with_capacity(requests.len());

    for req in requests {
        let key = format!(
            "{}_{}",
            req.name,
            req.params
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join("_")
        );
        let result = if talib_compat && req.name.eq_ignore_ascii_case("ppo") {
            talib_percentage_oscillator(close, &req.params)
        } else if talib_compat
            && matches!(
                req.name.to_ascii_lowercase().as_str(),
                "plus_dm" | "minus_dm"
            )
        {
            match (high, low) {
                (Some(high), Some(low)) => talib_directional_movement(
                    high,
                    low,
                    params_first_or(req, 14),
                    req.name.eq_ignore_ascii_case("plus_dm"),
                ),
                _ => IndicatorResult::Error(format!(
                    "{} requires high and low data",
                    req.name.to_ascii_uppercase()
                )),
            }
        } else {
            compute_single_indicator(open, high, low, close, volume, secondary, req)
        };
        let result = if talib_compat {
            apply_talib_compatibility(&req.name, &req.params, result)
        } else {
            result
        };
        results.push((key, result));
    }

    results
}

fn talib_percentage_oscillator(input: &[f64], params: &[f64]) -> IndicatorResult {
    let fast = params.first().copied().unwrap_or(12.0).max(1.0) as usize;
    let slow = params.get(1).copied().unwrap_or(26.0).max(1.0) as usize;
    let ma_type = params.get(2).copied().unwrap_or(0.0) as usize;
    let kind = match ma_type {
        0 => indicators::MaType::Sma,
        1 => indicators::MaType::Ema,
        2 => indicators::MaType::Wma,
        3 => indicators::MaType::Dema,
        4 => indicators::MaType::Tema,
        5 => indicators::MaType::Trima,
        6 => indicators::MaType::Kama,
        8 => indicators::MaType::T3,
        _ => indicators::MaType::Ema,
    };
    match (
        indicators::ma(input, fast, kind),
        indicators::ma(input, slow, kind),
    ) {
        (Ok(fast_ma), Ok(slow_ma)) => {
            let mut output = vec![f64::NAN; input.len()];
            for i in 0..input.len() {
                if fast_ma[i].is_finite() && slow_ma[i].is_finite() && slow_ma[i].abs() > 1e-15 {
                    output[i] = (fast_ma[i] - slow_ma[i]) / slow_ma[i] * 100.0;
                }
            }
            IndicatorResult::Single(output)
        }
        (Err(error), _) | (_, Err(error)) => IndicatorResult::Error(error.to_string()),
    }
}

fn params_first_or(req: &IndicatorRequest, default: usize) -> usize {
    req.params
        .first()
        .copied()
        .unwrap_or(default as f64)
        .max(1.0) as usize
}

fn talib_directional_movement(
    high: &[f64],
    low: &[f64],
    period: usize,
    plus: bool,
) -> IndicatorResult {
    if high.len() != low.len() || high.len() < period {
        return IndicatorResult::Error(
            "high and low must have equal lengths and contain at least one period".to_string(),
        );
    }
    let mut raw = vec![0.0; high.len()];
    for i in 1..high.len() {
        let up_move = high[i] - high[i - 1];
        let down_move = low[i - 1] - low[i];
        raw[i] = if plus {
            if up_move > 0.0 && up_move > down_move {
                up_move
            } else {
                0.0
            }
        } else if down_move > 0.0 && down_move > up_move {
            down_move
        } else {
            0.0
        };
    }

    let mut output = vec![f64::NAN; high.len()];
    let first = period - 1;
    let mut smooth: f64 = raw[1..=first].iter().sum();
    output[first] = smooth;
    let inv_period = 1.0 / period as f64;
    for i in (first + 1)..high.len() {
        smooth = smooth - smooth * inv_period + raw[i];
        output[i] = smooth;
    }
    IndicatorResult::Single(output)
}

/// Normalize the intentionally permissive internal indicator contract to the
/// TA-Lib Python contract. The default batch API keeps finkit's native
/// semantics; callers opt into this adapter explicitly so existing formulas
/// and charting code do not change underneath them.
fn apply_talib_compatibility(
    name: &str,
    params: &[f64],
    result: IndicatorResult,
) -> IndicatorResult {
    let name = name.to_ascii_lowercase();
    if matches!(result, IndicatorResult::Error(_)) {
        return result;
    }

    let period = |index: usize, default: usize| {
        params
            .get(index)
            .copied()
            .unwrap_or(default as f64)
            .max(1.0) as usize
    };
    let lookback = match name.as_str() {
        "kama" => period(0, 10),
        "mama" => 32,
        "mavp" => period(1, 30).saturating_sub(1),
        "sar" | "sarext" => 1,
        "t3" => 6 * period(0, 5).saturating_sub(1),
        "dema" => 2 * period(0, 30).saturating_sub(1),
        "tema" => 3 * period(0, 30).saturating_sub(1),
        "ht_trendline" => 63,
        "adx" => 2 * period(0, 14).saturating_sub(1),
        "adxr" => 3 * period(0, 14).saturating_sub(1),
        "apo" | "ppo" => period(1, 26).saturating_sub(1),
        "aroon" | "aroonosc" => period(0, 14),
        "cmo" | "rsi" => period(0, 14),
        "macd" | "macdext" | "macdfix" => {
            let slow = if name == "macdfix" {
                26
            } else if name == "macdext" {
                period(2, 26)
            } else {
                period(1, 26)
            };
            let signal = if name == "macdfix" {
                9
            } else if name == "macdext" {
                period(4, 9)
            } else {
                period(2, 9)
            };
            slow + signal - 2
        }
        "stoch" => period(0, 5) + period(1, 3) + period(3, 3) - 3,
        "stochf" => period(0, 5) + period(1, 3) - 2,
        "stochrsi" => period(0, 14) + period(1, 5) + period(3, 3) - 2,
        "trix" => 3 * period(0, 30) - 2,
        "ultosc" => period(2, 28),
        "atr" | "natr" => period(0, 14),
        "trange" => 1,
        "adosc" => period(1, 10).saturating_sub(1),
        "beta" => period(0, 5),
        "correl"
        | "correlation"
        | "linearreg"
        | "linear_reg"
        | "linearreg_angle"
        | "linearreg_intercept"
        | "linearreg_slope"
        | "stddev"
        | "std_dev"
        | "tsf"
        | "var"
        | "max"
        | "min"
        | "minmax"
        | "sum"
        | "accbands"
        | "avgdev"
        | "imi" => period(0, 30).saturating_sub(1),
        "maxindex" | "minindex" | "minmaxindex" => 0,
        _ => 0,
    };

    // The native SAR result also carries its acceleration-factor trace. The
    // TA-Lib public function exposes only the SAR series.
    let result = if name == "sar" {
        match result {
            IndicatorResult::Double(sar, _) => IndicatorResult::Single(sar),
            other => other,
        }
    } else {
        result
    };

    match name.as_str() {
        "maxindex" | "minindex" | "minmaxindex" => {
            let p = period(0, 30);
            fn absolute_index(values: &mut [f64], period: usize) {
                for (i, value) in values.iter_mut().enumerate() {
                    if i < period.saturating_sub(1) || *value < 0.0 {
                        *value = 0.0;
                    } else {
                        *value += (i + 1 - period) as f64;
                    }
                }
            }
            match result {
                IndicatorResult::Single(mut values) => {
                    absolute_index(&mut values, p);
                    IndicatorResult::Single(values)
                }
                IndicatorResult::Double(mut first, mut second) => {
                    absolute_index(&mut first, p);
                    absolute_index(&mut second, p);
                    IndicatorResult::Double(first, second)
                }
                other => other,
            }
        }
        "aroon" => match result {
            IndicatorResult::Double(mut up, mut down) => {
                let end = lookback.min(up.len()).min(down.len());
                up[..end].fill(f64::NAN);
                down[..end].fill(f64::NAN);
                // TA-Lib returns (aroondown, aroonup), while the native
                // finkit result is (aroonup, aroondown).
                IndicatorResult::Double(down, up)
            }
            other => other,
        },
        _ => {
            fn mask(values: &mut [f64], lookback: usize) {
                let end = lookback.min(values.len());
                values[..end].fill(f64::NAN);
            }
            match result {
                IndicatorResult::Single(mut values) => {
                    mask(&mut values, lookback);
                    IndicatorResult::Single(values)
                }
                IndicatorResult::Double(mut first, mut second) => {
                    mask(&mut first, lookback);
                    mask(&mut second, lookback);
                    IndicatorResult::Double(first, second)
                }
                IndicatorResult::Triple(mut first, mut second, mut third) => {
                    mask(&mut first, lookback);
                    mask(&mut second, lookback);
                    mask(&mut third, lookback);
                    IndicatorResult::Triple(first, second, third)
                }
                IndicatorResult::Quad(mut first, mut second, mut third, mut fourth) => {
                    mask(&mut first, lookback);
                    mask(&mut second, lookback);
                    mask(&mut third, lookback);
                    mask(&mut fourth, lookback);
                    IndicatorResult::Quad(first, second, third, fourth)
                }
                other => other,
            }
        }
    }
}

fn pattern_result(result: ::finkit::Result<candlestick::PatternResult>) -> IndicatorResult {
    result
        .map(|arr| {
            IndicatorResult::Single(
                arr.into_raw_vec()
                    .into_iter()
                    .map(|value| value as f64)
                    .collect(),
            )
        })
        .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
}

/// Compute a single indicator based on request.
fn compute_single_indicator(
    open: Option<&[f64]>,
    high: Option<&[f64]>,
    low: Option<&[f64]>,
    close: &[f64],
    volume: Option<&[f64]>,
    secondary: Option<&[f64]>,
    req: &IndicatorRequest,
) -> IndicatorResult {
    let name = req.name.to_lowercase();
    let params = &req.params;

    match name.as_str() {
        "sma" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            moving_avg::sma(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "ema" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            moving_avg::ema(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "wma" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            moving_avg::wma(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "dema" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            moving_avg::dema(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "tema" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            moving_avg::tema(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "kama" => {
            let period = params.first().copied().unwrap_or(10.0) as usize;
            let fast = params.get(1).copied().unwrap_or(2.0) as usize;
            let slow = params.get(2).copied().unwrap_or(30.0) as usize;
            moving_avg::kama(close, period, fast, slow)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "t3" => {
            let period = params.first().copied().unwrap_or(5.0) as usize;
            let vfactor = params.get(1).copied().unwrap_or(0.7);
            indicators::t3(close, period, vfactor)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "sarext" => match (high, low) {
            (Some(h), Some(l)) => {
                let start_value = params.first().copied().unwrap_or(0.0);
                let offset_on_reverse = params.get(1).copied().unwrap_or(0.0);
                let af_init_long = params.get(2).copied().unwrap_or(0.02);
                let af_long = params.get(3).copied().unwrap_or(0.02);
                let af_max_long = params.get(4).copied().unwrap_or(0.2);
                let af_init_short = params.get(5).copied().unwrap_or(0.02);
                let af_short = params.get(6).copied().unwrap_or(0.02);
                let af_max_short = params.get(7).copied().unwrap_or(0.2);
                indicators::sarext(
                    h,
                    l,
                    start_value,
                    offset_on_reverse,
                    af_init_long,
                    af_long,
                    af_max_long,
                    af_init_short,
                    af_short,
                    af_max_short,
                )
                .map(|res| IndicatorResult::Single(res.sar.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("SAREXT requires high and low data".to_string()),
        },
        "accbands" => {
            let period = params.first().copied().unwrap_or(20.0) as usize;
            match (high, low) {
                (Some(h), Some(l)) => indicators::accbands(h, l, close, period)
                    .map(|res| {
                        IndicatorResult::Triple(
                            res.upper.into_raw_vec(),
                            res.middle.into_raw_vec(),
                            res.lower.into_raw_vec(),
                        )
                    })
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
                _ => IndicatorResult::Error("ACCBANDS requires high and low data".to_string()),
            }
        }
        "imi" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            match open {
                Some(o) => indicators::imi(o, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
                None => IndicatorResult::Error("IMI requires open data".to_string()),
            }
        }
        "nvi" => match volume {
            Some(v) => indicators::nvi(close, v)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            None => IndicatorResult::Error("NVI requires volume data".to_string()),
        },
        "pvi" => match volume {
            Some(v) => indicators::pvi(close, v)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            None => IndicatorResult::Error("PVI requires volume data".to_string()),
        },
        "rsi" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::rsi(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "mom" => {
            let period = params.first().copied().unwrap_or(10.0) as usize;
            indicators::mom(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "roc" => {
            let period = params.first().copied().unwrap_or(10.0) as usize;
            indicators::roc(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "cmo" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::cmo(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "trix" => {
            let period = params.first().copied().unwrap_or(30.0) as usize;
            indicators::trix(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "apo" => {
            let fast = params.first().copied().unwrap_or(12.0) as usize;
            let slow = params.get(1).copied().unwrap_or(26.0) as usize;
            indicators::apo(close, fast, slow)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "macd" => {
            let fast = params.first().copied().unwrap_or(12.0) as usize;
            let slow = params.get(1).copied().unwrap_or(26.0) as usize;
            let signal = params.get(2).copied().unwrap_or(9.0) as usize;
            indicators::macd(close, fast, slow, signal)
                .map(|res| {
                    IndicatorResult::Triple(
                        res.macd.into_raw_vec(),
                        res.signal.into_raw_vec(),
                        res.hist.into_raw_vec(),
                    )
                })
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "bollinger_bands" | "bbands" => {
            let period = params.first().copied().unwrap_or(5.0) as usize;
            let nbdevup = params.get(1).copied().unwrap_or(2.0);
            let nbdevdn = params.get(2).copied().unwrap_or(2.0);
            indicators::bbands(close, period, nbdevup, nbdevdn)
                .map(|res| {
                    IndicatorResult::Triple(
                        res.upper.into_raw_vec(),
                        res.middle.into_raw_vec(),
                        res.lower.into_raw_vec(),
                    )
                })
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "midpoint" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::midpoint(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "ht_dcperiod" => indicators::ht_dcperiod(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "ht_dcphase" => indicators::ht_dcphase(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "ht_phasor" => indicators::ht_phasor(close)
            .map(|res| IndicatorResult::Double(res.0.into_raw_vec(), res.1.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "ht_sine" => indicators::ht_sine(close)
            .map(|res| IndicatorResult::Double(res.0.into_raw_vec(), res.1.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "ht_trendmode" => indicators::ht_trendmode(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "ht_trendline" => indicators::ht_trendline(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),

        "ma" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::ma(close, period, indicators::MaType::Sma)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "trima" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            moving_avg::trima(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "mavp" => match secondary {
            Some(periods) => {
                let min_period = params.first().copied().unwrap_or(2.0) as usize;
                let max_period = params.get(1).copied().unwrap_or(30.0) as usize;
                moving_avg::mavp(close, periods, min_period, max_period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            None => IndicatorResult::Error(
                "MAVP requires a periods array as secondary data".to_string(),
            ),
        },
        "macdext" => {
            let fast = params.first().copied().unwrap_or(12.0) as usize;
            let slow = params.get(2).copied().unwrap_or(26.0) as usize;
            let signal = params.get(4).copied().unwrap_or(9.0) as usize;
            indicators::macdext(
                close,
                fast,
                indicators::MaType::Sma,
                slow,
                indicators::MaType::Sma,
                signal,
                indicators::MaType::Sma,
            )
            .map(|res| {
                IndicatorResult::Triple(
                    res.macd.into_raw_vec(),
                    res.signal.into_raw_vec(),
                    res.hist.into_raw_vec(),
                )
            })
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "macdfix" => indicators::macdfix(close)
            .map(|res| {
                IndicatorResult::Triple(
                    res.macd.into_raw_vec(),
                    res.signal.into_raw_vec(),
                    res.hist.into_raw_vec(),
                )
            })
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "adxr" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::adxr(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("ADXR requires high and low data".to_string()),
        },
        "aroonosc" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::aroonosc(h, l, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("AroonOsc requires high and low data".to_string()),
        },
        "ppo" => {
            let fast = params.first().copied().unwrap_or(12.0) as usize;
            let slow = params.get(1).copied().unwrap_or(26.0) as usize;
            indicators::ppo(close, fast, slow)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "rocp" => {
            let period = params.first().copied().unwrap_or(10.0) as usize;
            indicators::rocp(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "rocr" => {
            let period = params.first().copied().unwrap_or(10.0) as usize;
            indicators::rocr(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "rocr100" => {
            let period = params.first().copied().unwrap_or(10.0) as usize;
            indicators::rocr100(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "stochf" => match (high, low) {
            (Some(h), Some(l)) => {
                let fastk = params.first().copied().unwrap_or(5.0) as usize;
                let fastd = params.get(1).copied().unwrap_or(3.0) as usize;
                indicators::stochf(h, l, close, fastk, fastd)
                    .map(|res| IndicatorResult::Double(res.k.into_raw_vec(), res.d.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("STOCHF requires high and low data".to_string()),
        },
        "stochrsi" => {
            let rsi_period = params.first().copied().unwrap_or(14.0) as usize;
            let stoch_period = params.get(1).copied().unwrap_or(5.0) as usize;
            let fastk = params.get(2).copied().unwrap_or(3.0) as usize;
            let fastd = params.get(3).copied().unwrap_or(0.0) as usize;
            indicators::stochrsi(close, rsi_period, stoch_period, fastk, fastd)
                .map(|res| IndicatorResult::Double(res.k.into_raw_vec(), res.d.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "ultosc" => match (high, low) {
            (Some(h), Some(l)) => {
                let p1 = params.first().copied().unwrap_or(7.0) as usize;
                let p2 = params.get(1).copied().unwrap_or(14.0) as usize;
                let p3 = params.get(2).copied().unwrap_or(28.0) as usize;
                indicators::ultosc(h, l, close, p1, p2, p3)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("ULTOSC requires high and low data".to_string()),
        },
        "avgdev" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::avgdev(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "linearreg_angle" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::linearreg_angle(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "linearreg_intercept" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::linearreg_intercept(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "linearreg_slope" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::linearreg_slope(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "var" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::var(close, period, 1.0)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "beta" => match secondary {
            Some(other) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::beta(close, other, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            None => IndicatorResult::Error("BETA requires secondary data".to_string()),
        },
        "correl" | "correlation" => match secondary {
            Some(other) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::correlation(close, other, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            None => IndicatorResult::Error("CORREL requires secondary data".to_string()),
        },
        "acos" => indicators::acos(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "asin" => indicators::asin(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "atan" => indicators::atan(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "ceil" => indicators::ceil(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "cos" => indicators::cos(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "cosh" => indicators::cosh(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "exp" => indicators::exp(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "floor" => indicators::floor(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "ln" => indicators::ln(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "log10" => indicators::log10(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "sin" => indicators::sin(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "sinh" => indicators::sinh(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "sqrt" => indicators::sqrt(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "tan" => indicators::tan(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "tanh" => indicators::tanh(close)
            .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "add" => match secondary {
            Some(other) => indicators::add(close, other)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            None => IndicatorResult::Error("ADD requires secondary data".to_string()),
        },
        "div" => match secondary {
            Some(other) => indicators::div(close, other)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            None => IndicatorResult::Error("DIV requires secondary data".to_string()),
        },
        "mult" => match secondary {
            Some(other) => indicators::mult(close, other)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            None => IndicatorResult::Error("MULT requires secondary data".to_string()),
        },
        "sub" => match secondary {
            Some(other) => indicators::sub(close, other)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            None => IndicatorResult::Error("SUB requires secondary data".to_string()),
        },
        "max" => {
            let period = params.first().copied().unwrap_or(30.0) as usize;
            indicators::max(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "min" => {
            let period = params.first().copied().unwrap_or(30.0) as usize;
            indicators::min(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "sum" => {
            let period = params.first().copied().unwrap_or(30.0) as usize;
            indicators::sum(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "maxindex" => {
            let period = params.first().copied().unwrap_or(30.0) as usize;
            indicators::maxindex(close, period)
                .map(|arr| {
                    IndicatorResult::Single(
                        arr.into_raw_vec()
                            .into_iter()
                            .map(|value| value as f64)
                            .collect(),
                    )
                })
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "minindex" => {
            let period = params.first().copied().unwrap_or(30.0) as usize;
            indicators::minindex(close, period)
                .map(|arr| {
                    IndicatorResult::Single(
                        arr.into_raw_vec()
                            .into_iter()
                            .map(|value| value as f64)
                            .collect(),
                    )
                })
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "minmax" => {
            let period = params.first().copied().unwrap_or(30.0) as usize;
            indicators::minmax(close, period)
                .map(|(min_values, max_values)| {
                    IndicatorResult::Double(min_values.into_raw_vec(), max_values.into_raw_vec())
                })
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "minmaxindex" => {
            let period = params.first().copied().unwrap_or(30.0) as usize;
            indicators::minmaxindex(close, period)
                .map(|(min_values, max_values)| {
                    IndicatorResult::Double(
                        min_values
                            .into_raw_vec()
                            .into_iter()
                            .map(|value| value as f64)
                            .collect(),
                        max_values
                            .into_raw_vec()
                            .into_iter()
                            .map(|value| value as f64)
                            .collect(),
                    )
                })
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "zscore" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::zscore(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "linear_reg" | "linreg" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::linear_reg(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "tsf" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::tsf(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "std_dev" => {
            let period = params.first().copied().unwrap_or(5.0) as usize;
            let nb_dev = params.get(1).copied().unwrap_or(1.0);
            indicators::std_dev(close, period, nb_dev)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "percent_rank" => {
            let period = params.first().copied().unwrap_or(10.0) as usize;
            indicators::percent_rank(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "adx" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::adx(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("ADX requires high and low data".to_string()),
        },
        "aroon" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::aroon(h, l, period)
                    .map(|res| {
                        IndicatorResult::Double(
                            res.aroon_up.into_raw_vec(),
                            res.aroon_down.into_raw_vec(),
                        )
                    })
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("Aroon requires high and low data".to_string()),
        },
        "cci" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::cci(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("CCI requires high and low data".to_string()),
        },
        "willr" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::willr(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("WillR requires high and low data".to_string()),
        },
        "dx" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::dx(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("DX requires high and low data".to_string()),
        },
        "minus_di" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::minus_di(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("MinusDI requires high and low data".to_string()),
        },
        "plus_di" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::plus_di(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("PlusDI requires high and low data".to_string()),
        },
        "minus_dm" => match (high, low) {
            (Some(h), Some(l)) => indicators::minus_dm(h, l)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("MinusDM requires high and low data".to_string()),
        },
        "plus_dm" => match (high, low) {
            (Some(h), Some(l)) => indicators::plus_dm(h, l)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("PlusDM requires high and low data".to_string()),
        },
        "stoch" => match (high, low) {
            (Some(h), Some(l)) => {
                let fastk = params.first().copied().unwrap_or(5.0) as usize;
                let slowk = params.get(1).copied().unwrap_or(3.0) as usize;
                let slowd = params.get(2).copied().unwrap_or(3.0) as usize;
                indicators::stoch(h, l, close, fastk, slowk, slowd)
                    .map(|res| IndicatorResult::Double(res.k.into_raw_vec(), res.d.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("Stoch requires high and low data".to_string()),
        },
        "atr" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::atr(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("ATR requires high and low data".to_string()),
        },
        "natr" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::natr(h, l, close, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("NATR requires high and low data".to_string()),
        },
        "trange" => match (high, low) {
            (Some(h), Some(l)) => indicators::trange(h, l, close)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("TRange requires high and low data".to_string()),
        },
        "mfi" => match (high, low, volume) {
            (Some(h), Some(l), Some(v)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::mfi(h, l, close, v, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("MFI requires high, low and volume data".to_string()),
        },
        "obv" => match volume {
            Some(v) => indicators::obv(close, v)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("OBV requires volume data".to_string()),
        },
        "ad" => match (high, low, volume) {
            (Some(h), Some(l), Some(v)) => indicators::ad(h, l, close, v)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("AD requires high, low and volume data".to_string()),
        },
        "adosc" => match (high, low, volume) {
            (Some(h), Some(l), Some(v)) => {
                let fast = params.first().copied().unwrap_or(3.0) as usize;
                let slow = params.get(1).copied().unwrap_or(10.0) as usize;
                indicators::adosc(h, l, close, v, fast, slow)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("ADOSC requires high, low and volume data".to_string()),
        },
        "bop" => match (open, high, low) {
            (Some(o), Some(h), Some(l)) => indicators::bop(o, h, l, close)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("BOP requires open, high and low data".to_string()),
        },
        "avgprice" => match (open, high, low) {
            (Some(o), Some(h), Some(l)) => indicators::avgprice(o, h, l, close)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("AvgPrice requires open, high and low data".to_string()),
        },
        "medprice" => match (high, low) {
            (Some(h), Some(l)) => indicators::medprice(h, l)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("MedPrice requires high and low data".to_string()),
        },
        "typprice" => match (high, low) {
            (Some(h), Some(l)) => indicators::typprice(h, l, close)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("TypPrice requires high and low data".to_string()),
        },
        "wclprice" => match (high, low) {
            (Some(h), Some(l)) => indicators::wclprice(h, l, close)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
            _ => IndicatorResult::Error("WclPrice requires high and low data".to_string()),
        },
        "mama" => {
            let fastlimit = params.first().copied().unwrap_or(0.5);
            let slowlimit = params.get(1).copied().unwrap_or(0.05);
            indicators::mama(close, fastlimit, slowlimit)
                .map(|res| {
                    IndicatorResult::Double(res.mama.into_raw_vec(), res.fama.into_raw_vec())
                })
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "sar" => match (high, low) {
            (Some(h), Some(l)) => {
                let acceleration = params.first().copied().unwrap_or(0.02);
                let maximum = params.get(1).copied().unwrap_or(0.2);
                indicators::sar(h, l, acceleration, maximum)
                    .map(|res| {
                        IndicatorResult::Double(res.sar.into_raw_vec(), res.af.into_raw_vec())
                    })
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("SAR requires high and low data".to_string()),
        },
        "midprice" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::midprice(h, l, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("MidPrice requires high and low data".to_string()),
        },
        "vortex" => match (high, low) {
            (Some(h), Some(l)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::vortex(h, l, close, period)
                    .map(|res| {
                        IndicatorResult::Double(
                            res.vi_plus.into_raw_vec(),
                            res.vi_minus.into_raw_vec(),
                        )
                    })
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("Vortex requires high and low data".to_string()),
        },
        "vzo" => match volume {
            Some(v) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::vzo(close, v, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("VZO requires volume data".to_string()),
        },
        "volume_momentum" => match volume {
            Some(v) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::volume_momentum(v, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("VolumeMomentum requires volume data".to_string()),
        },
        "volume_roc" => match volume {
            Some(v) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::volume_roc(v, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("VolumeROC requires volume data".to_string()),
        },
        "chande_forecast_oscillator" | "cfo" => {
            let period = params.first().copied().unwrap_or(14.0) as usize;
            indicators::chande_forecast_oscillator(close, period)
                .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "twiggs_money_flow" => match (high, low, volume) {
            (Some(h), Some(l), Some(v)) => {
                let period = params.first().copied().unwrap_or(14.0) as usize;
                indicators::twiggs_money_flow(h, l, close, v, period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error(
                "TwiggsMoneyFlow requires high, low and volume data".to_string(),
            ),
        },
        "inertia" => match (open, high, low) {
            (Some(o), Some(h), Some(l)) => {
                let rvi_period = params.first().copied().unwrap_or(10.0) as usize;
                let linreg_period = params.get(1).copied().unwrap_or(14.0) as usize;
                indicators::inertia(o, h, l, close, rvi_period, linreg_period)
                    .map(|arr| IndicatorResult::Single(arr.into_raw_vec()))
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("Inertia requires open, high and low data".to_string()),
        },
        "darvas_box" => match (high, low) {
            (Some(h), Some(l)) => {
                let lookback = params.first().copied().unwrap_or(5.0) as usize;
                let confirmation = params.get(1).copied().unwrap_or(3.0) as usize;
                indicators::darvas_box(h, l, close, lookback, confirmation)
                    .map(|r| {
                        IndicatorResult::Triple(
                            r.box_top.into_raw_vec(),
                            r.box_bottom.into_raw_vec(),
                            r.signal.into_iter().map(|v| v as f64).collect(),
                        )
                    })
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("DarvasBox requires high and low data".to_string()),
        },
        "renko" => match (high, low) {
            (Some(h), Some(l)) => {
                let box_size = params.first().copied().unwrap_or(1.0);
                indicators::renko(h, l, box_size)
                    .map(|r| {
                        IndicatorResult::Double(
                            r.bricks.into_raw_vec(),
                            r.direction.into_iter().map(|v| v as f64).collect(),
                        )
                    })
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("Renko requires high and low data".to_string()),
        },
        "kagi" => {
            let reversal = params.first().copied().unwrap_or(1.0);
            indicators::kagi(close, reversal)
                .map(|r| {
                    IndicatorResult::Double(
                        r.kagi.into_raw_vec(),
                        r.direction.into_iter().map(|v| v as f64).collect(),
                    )
                })
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "point_and_figure" | "pnf" => match (high, low) {
            (Some(h), Some(l)) => {
                let box_size = params.first().copied().unwrap_or(1.0);
                let reversal = params.get(1).copied().unwrap_or(3.0) as usize;
                indicators::point_and_figure(h, l, box_size, reversal)
                    .map(|r| {
                        IndicatorResult::Triple(
                            r.pnf.into_raw_vec(),
                            r.column_type.into_iter().map(|v| v as f64).collect(),
                            r.new_column.into_iter().map(|v| v as f64).collect(),
                        )
                    })
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
            }
            _ => IndicatorResult::Error("PointAndFigure requires high and low data".to_string()),
        },
        "three_line_break" | "tlb" => {
            let lines = params.first().copied().unwrap_or(3.0) as usize;
            indicators::three_line_break(close, lines)
                .map(|r| {
                    IndicatorResult::Double(
                        r.line.into_raw_vec(),
                        r.direction.into_iter().map(|v| v as f64).collect(),
                    )
                })
                .unwrap_or_else(|e| IndicatorResult::Error(e.to_string()))
        }
        "williams_alligator" | "alligator" => indicators::williams_alligator(close)
            .map(|r| {
                IndicatorResult::Triple(
                    r.jaw.into_raw_vec(),
                    r.teeth.into_raw_vec(),
                    r.lips.into_raw_vec(),
                )
            })
            .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
        "heikin_ashi" | "ha" => match open {
            Some(o) => match (high, low) {
                (Some(h), Some(l)) => indicators::heikin_ashi(o, h, l, close)
                    .map(|r| {
                        IndicatorResult::Quad(
                            r.ha_open.into_raw_vec(),
                            r.ha_high.into_raw_vec(),
                            r.ha_low.into_raw_vec(),
                            r.ha_close.into_raw_vec(),
                        )
                    })
                    .unwrap_or_else(|e| IndicatorResult::Error(e.to_string())),
                _ => IndicatorResult::Error("HeikinAshi requires open, high and low".to_string()),
            },
            _ => IndicatorResult::Error("HeikinAshi requires open data".to_string()),
        },
        name if name.starts_with("cdl") => match (open, high, low) {
            (Some(o), Some(h), Some(l)) => match name {
                "cdl2crows" => pattern_result(candlestick::cdl_2crows(o, h, l, close)),
                "cdl3blackcrows" => pattern_result(candlestick::cdl_3black_crows(o, h, l, close)),
                "cdl3inside" => pattern_result(candlestick::cdl_3inside(o, h, l, close)),
                "cdl3linestrike" => pattern_result(candlestick::cdl_3linestrike(o, h, l, close)),
                "cdl3outside" => pattern_result(candlestick::cdl_3outside(o, h, l, close)),
                "cdl3starsinsouth" => {
                    pattern_result(candlestick::cdl_3starsinsouth(o, h, l, close))
                }
                "cdl3whitesoldiers" => {
                    pattern_result(candlestick::cdl_3white_soldiers(o, h, l, close))
                }
                "cdlabandonedbaby" => {
                    pattern_result(candlestick::cdl_abandoned_baby(o, h, l, close))
                }
                "cdladvanceblock" => pattern_result(candlestick::cdl_advanceblock(o, h, l, close)),
                "cdlbelthold" => pattern_result(candlestick::cdl_belthold(o, h, l, close)),
                "cdlbreakaway" => pattern_result(candlestick::cdl_breakaway(o, h, l, close)),
                "cdlclosingmarubozu" => {
                    pattern_result(candlestick::cdl_closingmarubozu(o, h, l, close))
                }
                "cdlconcealbabyswall" => {
                    pattern_result(candlestick::cdl_concealbabyswall(o, h, l, close))
                }
                "cdlcounterattack" => {
                    pattern_result(candlestick::cdl_counterattack(o, h, l, close))
                }
                "cdldarkcloudcover" => {
                    pattern_result(candlestick::cdl_darkcloudcover(o, h, l, close))
                }
                "cdldoji" => pattern_result(candlestick::cdl_doji(o, h, l, close)),
                "cdldojistar" => pattern_result(candlestick::cdl_doji_star(o, h, l, close)),
                "cdldragonflydoji" => {
                    pattern_result(candlestick::cdl_dragonflydoji(o, h, l, close))
                }
                "cdlengulfing" => pattern_result(candlestick::cdl_engulfing(o, h, l, close)),
                "cdleveningdojistar" => {
                    pattern_result(candlestick::cdl_eveningdojistar(o, h, l, close))
                }
                "cdleveningstar" => pattern_result(candlestick::cdl_eveningstar(o, h, l, close)),
                "cdlgapsidesidewhite" => {
                    pattern_result(candlestick::cdl_gap_side_white(o, h, l, close))
                }
                "cdlgravestonedoji" => {
                    pattern_result(candlestick::cdl_gravestonedoji(o, h, l, close))
                }
                "cdlhammer" => pattern_result(candlestick::cdl_hammer(o, h, l, close)),
                "cdlhangingman" => pattern_result(candlestick::cdl_hangingman(o, h, l, close)),
                "cdlharami" => pattern_result(candlestick::cdl_harami(o, h, l, close)),
                "cdlharamicross" => pattern_result(candlestick::cdl_haramicross(o, h, l, close)),
                "cdlhighwave" => pattern_result(candlestick::cdl_highwave(o, h, l, close)),
                "cdlhikkake" => pattern_result(candlestick::cdl_hikkake(o, h, l, close)),
                "cdlhikkakemod" => pattern_result(candlestick::cdl_hikkake_mod(o, h, l, close)),
                "cdlhomingpigeon" => pattern_result(candlestick::cdl_homing_pigeon(o, h, l, close)),
                "cdlidentical3crows" => {
                    pattern_result(candlestick::cdl_identical3crows(o, h, l, close))
                }
                "cdlinneck" => pattern_result(candlestick::cdl_inneck(o, h, l, close)),
                "cdlinvertedhammer" => {
                    pattern_result(candlestick::cdl_invertedhammer(o, h, l, close))
                }
                "cdlkicking" => pattern_result(candlestick::cdl_kicking(o, h, l, close)),
                "cdlkickingbylength" => {
                    pattern_result(candlestick::cdl_kickingbylength(o, h, l, close))
                }
                "cdlladderbottom" => pattern_result(candlestick::cdl_ladder_bottom(o, h, l, close)),
                "cdllongleggeddoji" => {
                    pattern_result(candlestick::cdl_longleggeddoji(o, h, l, close))
                }
                "cdllongline" => pattern_result(candlestick::cdl_longline(o, h, l, close)),
                "cdlmarubozu" => pattern_result(candlestick::cdl_marubozu(o, h, l, close)),
                "cdlmatchinglow" => pattern_result(candlestick::cdl_matchinglow(o, h, l, close)),
                "cdlmathold" => pattern_result(candlestick::cdl_mathold(o, h, l, close)),
                "cdlmorningdojistar" => {
                    pattern_result(candlestick::cdl_morningdojistar(o, h, l, close))
                }
                "cdlmorningstar" => pattern_result(candlestick::cdl_morningstar(o, h, l, close)),
                "cdlonneck" => pattern_result(candlestick::cdl_onneck(o, h, l, close)),
                "cdlpiercing" => pattern_result(candlestick::cdl_piercing(o, h, l, close)),
                "cdlrickshawman" => pattern_result(candlestick::cdl_rickshawman(o, h, l, close)),
                "cdlrisefall3methods" => {
                    pattern_result(candlestick::cdl_rise_fall_3methods(o, h, l, close))
                }
                "cdlseparatinglines" => {
                    pattern_result(candlestick::cdl_separatinglines(o, h, l, close))
                }
                "cdlshootingstar" => pattern_result(candlestick::cdl_shootingstar(o, h, l, close)),
                "cdlshortline" => pattern_result(candlestick::cdl_shortline(o, h, l, close)),
                "cdlspinningtop" => pattern_result(candlestick::cdl_spinningtop(o, h, l, close)),
                "cdlstalledpattern" => {
                    pattern_result(candlestick::cdl_stalledpattern(o, h, l, close))
                }
                "cdlsticksandwich" => {
                    pattern_result(candlestick::cdl_sticksandwich(o, h, l, close))
                }
                "cdltakuri" => pattern_result(candlestick::cdl_takuri(o, h, l, close)),
                "cdltasukigap" => pattern_result(candlestick::cdl_tasukigap(o, h, l, close)),
                "cdlthrusting" => pattern_result(candlestick::cdl_thrusting(o, h, l, close)),
                "cdltristar" => pattern_result(candlestick::cdl_tristar(o, h, l, close)),
                "cdlunique3river" => pattern_result(candlestick::cdl_unique3river(o, h, l, close)),
                "cdlupsidegap2crows" => {
                    pattern_result(candlestick::cdl_upsidegap2crows(o, h, l, close))
                }
                "cdlxsidegap3methods" => {
                    pattern_result(candlestick::cdl_xsidegap3methods(o, h, l, close))
                }
                _ => IndicatorResult::Error(format!("Unsupported candlestick function: {}", name)),
            },
            _ => IndicatorResult::Error(
                "Candlestick functions require open, high and low data".to_string(),
            ),
        },
        _ => IndicatorResult::Error(format!("Unknown indicator: {}", name)),
    }
}

// ============================================================================
// Python Module Registration
// ============================================================================

/// finkit: High-performance technical analysis library for Python
///
/// This module provides over 100 technical indicators powered by Rust,
/// offering 10-100x speedup compared to pure Python implementations.
///
/// Categories:
/// - Overlap Studies (Moving Averages, BBANDS, SAR, etc.)
/// - Momentum Indicators (RSI, MACD, STOCH, ADX, etc.)
/// - Cycle Indicators (Hilbert Transform family)
/// - Volume Indicators (OBV, AD, ADOSC, VWAP, Volume Profile)
/// - Volatility Indicators (ATR, NATR, TRANGE, SuperTrend)
/// - Price Transforms (AVGPRICE, MEDPRICE, TYPPRICE, WCLPRICE)
/// - Statistics (Z-Score, Beta, Correlation, StdDev, TSF)
/// - Candlestick Patterns (60+ patterns)
/// - Chart Patterns (Head & Shoulders, Double Top/Bottom, etc.)
/// - Advanced Indicators (Ichimoku, Donchian, Elder-Ray, Pivot Points, Fibonacci)
#[pymodule]
fn finkit(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyKlineData>()?;
    m.add_class::<PyKlineChart>()?;
    m.add_function(wrap_pyfunction!(chan_analyze, m)?)?;
    m.add_function(wrap_pyfunction!(chan_analyze_multi, m)?)?;
    m.add_function(wrap_pyfunction!(chan_analyze_multi_timestamps_py, m)?)?;
    m.add_function(wrap_pyfunction!(
        chan_analyze_multi_timestamps_calendar_py,
        m
    )?)?;
    m.add_function(wrap_pyfunction!(resolve_market_session_py, m)?)?;
    m.add_function(wrap_pyfunction!(resolve_market_session_config_py, m)?)?;
    m.add_function(wrap_pyfunction!(resolve_market_session_csv_py, m)?)?;

    // Overlap Studies
    m.add_function(wrap_pyfunction!(sma, m)?)?;
    m.add_function(wrap_pyfunction!(ema, m)?)?;
    m.add_function(wrap_pyfunction!(wma, m)?)?;
    m.add_function(wrap_pyfunction!(dema, m)?)?;
    m.add_function(wrap_pyfunction!(tema, m)?)?;
    m.add_function(wrap_pyfunction!(kama, m)?)?;
    m.add_function(wrap_pyfunction!(mama, m)?)?;
    m.add_function(wrap_pyfunction!(t3, m)?)?;
    m.add_function(wrap_pyfunction!(bollinger_bands, m)?)?;
    m.add_function(wrap_pyfunction!(sar, m)?)?;
    m.add_function(wrap_pyfunction!(midpoint, m)?)?;
    m.add_function(wrap_pyfunction!(midprice, m)?)?;

    // Momentum Indicators
    m.add_function(wrap_pyfunction!(rsi, m)?)?;
    m.add_function(wrap_pyfunction!(macd, m)?)?;
    m.add_function(wrap_pyfunction!(stoch, m)?)?;
    m.add_function(wrap_pyfunction!(adx, m)?)?;
    m.add_function(wrap_pyfunction!(aroon, m)?)?;
    m.add_function(wrap_pyfunction!(cci, m)?)?;
    m.add_function(wrap_pyfunction!(mom, m)?)?;
    m.add_function(wrap_pyfunction!(roc, m)?)?;
    m.add_function(wrap_pyfunction!(willr, m)?)?;
    m.add_function(wrap_pyfunction!(apo, m)?)?;
    m.add_function(wrap_pyfunction!(bop, m)?)?;
    m.add_function(wrap_pyfunction!(cmo, m)?)?;
    m.add_function(wrap_pyfunction!(dx, m)?)?;
    m.add_function(wrap_pyfunction!(mfi, m)?)?;
    m.add_function(wrap_pyfunction!(minus_di, m)?)?;
    m.add_function(wrap_pyfunction!(minus_dm, m)?)?;
    m.add_function(wrap_pyfunction!(plus_di, m)?)?;
    m.add_function(wrap_pyfunction!(plus_dm, m)?)?;
    m.add_function(wrap_pyfunction!(trix, m)?)?;

    // Cycle Indicators (Hilbert Transform)
    m.add_function(wrap_pyfunction!(ht_dcperiod, m)?)?;
    m.add_function(wrap_pyfunction!(ht_dcphase, m)?)?;
    m.add_function(wrap_pyfunction!(ht_phasor, m)?)?;
    m.add_function(wrap_pyfunction!(ht_sine, m)?)?;
    m.add_function(wrap_pyfunction!(ht_trendmode, m)?)?;
    m.add_function(wrap_pyfunction!(ht_trendline, m)?)?;

    // Volume Indicators
    m.add_function(wrap_pyfunction!(obv, m)?)?;
    m.add_function(wrap_pyfunction!(ad, m)?)?;
    m.add_function(wrap_pyfunction!(adosc, m)?)?;

    // Volatility Indicators
    m.add_function(wrap_pyfunction!(atr, m)?)?;
    m.add_function(wrap_pyfunction!(natr, m)?)?;
    m.add_function(wrap_pyfunction!(trange, m)?)?;

    // Price Transforms
    m.add_function(wrap_pyfunction!(avgprice, m)?)?;
    m.add_function(wrap_pyfunction!(medprice, m)?)?;
    m.add_function(wrap_pyfunction!(typprice, m)?)?;
    m.add_function(wrap_pyfunction!(wclprice, m)?)?;

    // Statistics Functions
    m.add_function(wrap_pyfunction!(zscore, m)?)?;
    m.add_function(wrap_pyfunction!(percent_rank, m)?)?;
    m.add_function(wrap_pyfunction!(beta, m)?)?;
    m.add_function(wrap_pyfunction!(correlation, m)?)?;
    m.add_function(wrap_pyfunction!(std_dev, m)?)?;
    m.add_function(wrap_pyfunction!(var, m)?)?;
    m.add_function(wrap_pyfunction!(linear_reg, m)?)?;
    m.add_function(wrap_pyfunction!(tsf, m)?)?;

    // Candlestick Patterns
    m.add_function(wrap_pyfunction!(cdl_doji, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_dragonfly_doji, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_gravestone_doji, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_long_legged_doji, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_doji_4prices, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_hammer, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_inverted_hammer, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_hanging_man, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_shooting_star, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_engulfing, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_harami, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_harami_cross, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_morning_star, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_evening_star, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_morning_doji_star, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_evening_doji_star, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_marubozu, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_three_white_soldiers, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_three_black_crows, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_three_inside_up, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_three_outside_up, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_three_inside_down, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_three_outside_down, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_piercing, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_dark_cloud_cover, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_belt_hold, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_spinning_top, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_high_wave, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_rickshaw_man, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_short_line, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_long_line, m)?)?;
    m.add_function(wrap_pyfunction!(cdl_kicking, m)?)?;

    // Chart Patterns
    m.add_function(wrap_pyfunction!(detect_head_shoulders, m)?)?;
    m.add_function(wrap_pyfunction!(detect_head_shoulders_bottom, m)?)?;
    m.add_function(wrap_pyfunction!(detect_double_top, m)?)?;
    m.add_function(wrap_pyfunction!(detect_double_bottom, m)?)?;
    m.add_function(wrap_pyfunction!(detect_triple_top, m)?)?;
    m.add_function(wrap_pyfunction!(detect_triple_bottom, m)?)?;
    m.add_function(wrap_pyfunction!(detect_ascending_triangle, m)?)?;
    m.add_function(wrap_pyfunction!(detect_descending_triangle, m)?)?;
    m.add_function(wrap_pyfunction!(detect_symmetrical_triangle, m)?)?;
    m.add_function(wrap_pyfunction!(detect_rising_wedge, m)?)?;
    m.add_function(wrap_pyfunction!(detect_falling_wedge, m)?)?;
    m.add_function(wrap_pyfunction!(detect_flag, m)?)?;
    m.add_function(wrap_pyfunction!(detect_pennant, m)?)?;
    m.add_function(wrap_pyfunction!(detect_rectangle, m)?)?;

    // Advanced Indicators
    m.add_function(wrap_pyfunction!(ichimoku, m)?)?;
    m.add_function(wrap_pyfunction!(supertrend, m)?)?;
    m.add_function(wrap_pyfunction!(vwap, m)?)?;
    m.add_function(wrap_pyfunction!(anchored_vwap, m)?)?;
    m.add_function(wrap_pyfunction!(vwap_bands, m)?)?;
    m.add_function(wrap_pyfunction!(elder_ray, m)?)?;
    m.add_function(wrap_pyfunction!(donchian, m)?)?;
    m.add_function(wrap_pyfunction!(pivot_points, m)?)?;
    m.add_function(wrap_pyfunction!(volume_profile, m)?)?;
    m.add_function(wrap_pyfunction!(fibonacci_retracement, m)?)?;

    // New Indicators (TASK-166~180)
    m.add_function(wrap_pyfunction!(vortex, m)?)?;
    m.add_function(wrap_pyfunction!(inertia, m)?)?;
    m.add_function(wrap_pyfunction!(vzo, m)?)?;
    m.add_function(wrap_pyfunction!(volume_momentum, m)?)?;
    m.add_function(wrap_pyfunction!(volume_roc, m)?)?;
    m.add_function(wrap_pyfunction!(chande_forecast_oscillator, m)?)?;
    m.add_function(wrap_pyfunction!(twiggs_money_flow, m)?)?;

    // Formula System
    #[cfg(feature = "formula")]
    {
        m.add_class::<PyCompiledFormula>()?;
        m.add_function(wrap_pyfunction!(formula_eval, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_dialect, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_bytecode, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_optimized, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_jit, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_simd, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_zero_copy, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_numpy_zero_copy, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_multi, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_draw, m)?)?;
        m.add_function(wrap_pyfunction!(formula_eval_debug, m)?)?;
        m.add_function(wrap_pyfunction!(formula_validate, m)?)?;
        m.add_function(wrap_pyfunction!(formula_get_template, m)?)?;
        m.add_function(wrap_pyfunction!(formula_search_templates, m)?)?;
        m.add_function(wrap_pyfunction!(formula_list_categories, m)?)?;
    }

    // Streaming Indicators
    streaming::register_streaming_classes(m)?;

    // Sweep API
    sweep::register_sweep_functions(m)?;

    // Transform Pipeline
    transforms::register_transform_classes(m)?;

    // Feature Engineering
    features::register_features_module(m)?;

    // Batch Computation (Single GIL Release)
    m.add_function(wrap_pyfunction!(compute_indicators, m)?)?;
    m.add_function(wrap_pyfunction!(compute_composite, m)?)?;

    Ok(())
}
