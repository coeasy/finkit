//! Classical chart-pattern detectors exposed to Python.

use super::*;

#[pyfunction]
#[pyo3(signature = (high, min_bars=5, head_ratio=1.1))]
pub fn detect_head_shoulders(
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

#[pyfunction]
#[pyo3(signature = (low, min_bars=5, head_ratio=0.9))]
pub fn detect_head_shoulders_bottom(
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

#[pyfunction]
#[pyo3(signature = (high, lookback=20, tolerance=0.03))]
pub fn detect_double_top(
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

#[pyfunction]
#[pyo3(signature = (low, lookback=20, tolerance=0.03))]
pub fn detect_double_bottom(
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

#[pyfunction]
#[pyo3(signature = (high, lookback=30, tolerance=0.03))]
pub fn detect_triple_top(
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

#[pyfunction]
#[pyo3(signature = (low, lookback=30, tolerance=0.03))]
pub fn detect_triple_bottom(
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

#[pyfunction]
#[pyo3(signature = (high, low, lookback=20, tolerance=0.05))]
pub fn detect_ascending_triangle(
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

#[pyfunction]
#[pyo3(signature = (high, low, lookback=20, tolerance=0.05))]
pub fn detect_descending_triangle(
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

#[pyfunction]
#[pyo3(signature = (high, low, lookback=20))]
pub fn detect_symmetrical_triangle(
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

#[pyfunction]
#[pyo3(signature = (high, low, lookback=20))]
pub fn detect_rising_wedge(
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

#[pyfunction]
#[pyo3(signature = (high, low, lookback=20))]
pub fn detect_falling_wedge(
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

#[pyfunction]
#[pyo3(signature = (high, low, close, flagpole_period=10, flag_period=5))]
pub fn detect_flag(
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

#[pyfunction]
#[pyo3(signature = (high, low, close, flagpole_period=10, pennant_period=5))]
pub fn detect_pennant(
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

#[pyfunction]
#[pyo3(signature = (high, low, lookback=20, tolerance=0.05))]
pub fn detect_rectangle(
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
