//! Python bindings for parameter sweep API.

use pyo3::prelude::*;
use alpha_ta_core::indicators::sweep::{ema_sweep, rsi_sweep, sma_sweep};
use alpha_ta_core::indicators::sweep_engine::{ParamRange, SweepEngine};
use alpha_ta_core::indicators::sweepable::{EmaSweepable, RsiSweepable, SmaSweepable};

/// Sweep SMA across multiple periods.
///
/// Returns a list of lists, one per period.
#[pyfunction]
#[pyo3(signature = (data, periods))]
pub fn sweep_sma(py: Python<'_>, data: Vec<f64>, periods: Vec<usize>) -> PyResult<Vec<Vec<f64>>> {
    py.allow_threads(|| {
        sma_sweep(&data, &periods)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    })
}

/// Sweep EMA across multiple periods.
#[pyfunction]
#[pyo3(signature = (data, periods))]
pub fn sweep_ema(py: Python<'_>, data: Vec<f64>, periods: Vec<usize>) -> PyResult<Vec<Vec<f64>>> {
    py.allow_threads(|| {
        ema_sweep(&data, &periods)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    })
}

/// Sweep RSI across multiple periods.
#[pyfunction]
#[pyo3(signature = (data, periods))]
pub fn sweep_rsi(py: Python<'_>, data: Vec<f64>, periods: Vec<usize>) -> PyResult<Vec<Vec<f64>>> {
    py.allow_threads(|| {
        rsi_sweep(&data, &periods)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    })
}

/// Generic SweepEngine: Cartesian-product parameter scan.
///
/// Args:
///     indicator: One of "sma", "ema", "rsi"
///     data: Price series
///     ranges: List of (start, end, step) tuples defining parameter ranges
///
/// Returns dict with "indicator", "param_count", "results" (list of dicts with "params" and "values")
#[pyfunction]
#[pyo3(signature = (indicator, data, ranges))]
pub fn sweep_engine(
    py: Python<'_>,
    indicator: &str,
    data: Vec<f64>,
    ranges: Vec<(usize, usize, usize)>,
) -> PyResult<PyObject> {
    let param_ranges: Vec<ParamRange> = ranges
        .iter()
        .map(|(s, e, step)| ParamRange::new(*s, *e, *step))
        .collect();

    let engine = SweepEngine::new();
    let result = py.allow_threads(|| {
        match indicator.to_lowercase().as_str() {
            "sma" => engine.run(&SmaSweepable, &data, &param_ranges),
            "ema" => engine.run(&EmaSweepable, &data, &param_ranges),
            "rsi" => engine.run(&RsiSweepable, &data, &param_ranges),
            other => Err(alpha_ta_core::error::TaError::InvalidParameter {
                name: "indicator".to_string(),
                constraint: format!("one of 'sma','ema','rsi', got '{other}'"),
            }),
        }
    })
    .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let dict = pyo3::types::PyDict::new(py);
    dict.set_item("indicator", result.indicator_name)?;
    dict.set_item("param_count", result.param_count)?;

    let results_list: Vec<PyObject> = result
        .results
        .iter()
        .map(|r| {
            let d = pyo3::types::PyDict::new(py);
            let params_repr = format!("{:?}", r.params);
            d.set_item("params", params_repr).unwrap();
            d.set_item("values", r.values.clone()).unwrap();
            d.into_any().unbind()
        })
        .collect();
    dict.set_item("results", results_list)?;

    Ok(dict.into_any().unbind())
}

pub fn register_sweep_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(sweep_sma, m)?)?;
    m.add_function(wrap_pyfunction!(sweep_ema, m)?)?;
    m.add_function(wrap_pyfunction!(sweep_rsi, m)?)?;
    m.add_function(wrap_pyfunction!(sweep_engine, m)?)?;
    Ok(())
}
