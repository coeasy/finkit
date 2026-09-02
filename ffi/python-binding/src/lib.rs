#![allow(missing_docs)]
#![allow(missing_debug_implementations)]
// `into_raw_vec` is deprecated in ndarray 0.16 in favour of
// `into_raw_vec_and_offset`, but the offset is always 0 for 1D arrays and
// rewriting the 100+ call sites in the FFI surface is noisy. Suppress here
// so the deprecation churn stays contained to the `core` crate.
#![allow(deprecated)]

use ::finkit::indicators;
use ::finkit::indicators::PivotMethod;
use ::finkit::math::moving_avg;
use ::finkit::patterns::{candlestick, chart};
use finkit_visualization::config::{
    ChartConfig, ChartConfigBuilder, IndicatorConfig, IndicatorType,
};
use finkit_visualization::data::KlineData;
use finkit_visualization::error::VisualizationError;
use finkit_visualization::language::Language;
#[cfg(feature = "formula")]
use formula_plan::PyCompiledFormula;
use numpy::{PyArray1, PyReadonlyArray1};
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
    let direct_result: PyResult<Option<Array1<f64>>> = Python::attach(|py| {
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

#[pyclass]
#[derive(Clone)]
struct PyKlineData {
    inner: KlineData,
}

#[pymethods]
impl PyKlineData {
    #[new]
    fn new(
        dates: Vec<String>,
        opens: Vec<f64>,
        highs: Vec<f64>,
        lows: Vec<f64>,
        closes: Vec<f64>,
        volumes: Vec<f64>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: KlineData::new(dates, opens, highs, lows, closes, volumes),
        })
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn validate(&self) -> bool {
        self.inner.validate()
    }

    fn push(&mut self, date: String, open: f64, high: f64, low: f64, close: f64, volume: f64) {
        self.inner.push(date, open, high, low, close, volume);
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
}

#[pyclass]
struct PyKlineChart {
    data: PyKlineData,
    indicators: Vec<IndicatorConfig>,
    config: ChartConfig,
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
        Ok(Self {
            data,
            indicators: Vec::new(),
            config,
        })
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

    fn save_as_svg(&self, py: Python<'_>, path: &str) -> PyResult<()> {
        let data = self.data.inner.clone();
        let config = self.config.clone();
        let indicators = self.indicators.clone();
        let svg = py.detach(|| -> PyResult<String> {
            let mut chart = finkit_visualization::chart::KlineChart::new(config);
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
            chart
                .build_draw_list(&data, &indicators)
                .map_err(convert_vis_error)?;
            chart.to_html_string().map_err(convert_vis_error)
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
            chart
                .build_draw_list(&data, &indicators)
                .map_err(convert_vis_error)?;
            chart.to_svg_string().map_err(convert_vis_error)
        })
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
#[pyo3(signature = (close, requests, open=None, high=None, low=None, volume=None, secondary=None))]
fn compute_indicators<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'_, f64>,
    requests: Vec<(String, Vec<f64>)>,
    open: Option<PyReadonlyArray1<'_, f64>>,
    high: Option<PyReadonlyArray1<'_, f64>>,
    low: Option<PyReadonlyArray1<'_, f64>>,
    volume: Option<PyReadonlyArray1<'_, f64>>,
    secondary: Option<PyReadonlyArray1<'_, f64>>,
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
        let result = compute_single_indicator(open, high, low, close, volume, secondary, req);
        results.push((key, result));
    }

    results
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
        "beta" => IndicatorResult::Error(
            "Beta requires benchmark data (use individual function)".to_string(),
        ),
        "correl" | "correlation" => IndicatorResult::Error(
            "Correlation requires second series data (use individual function)".to_string(),
        ),
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

    Ok(())
}
