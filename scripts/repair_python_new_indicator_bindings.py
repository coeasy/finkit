#!/usr/bin/env python3
"""Repair the two registered TASK-166~180 Python indicators missing wrappers.

The Rust core implementations and module registrations already exist, but the
Python wrappers were never added. Architecture v3 SSOT regeneration correctly
exposes the latent compile error. Keep the repair ndarray-direct so these APIs
never introduce the historical Vec -> Python list -> ndarray path.
"""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LIB = ROOT / "ffi" / "python-binding" / "src" / "lib.rs"
MARKER = "// ============================================================================\n// Formula System\n// ============================================================================"

WRAPPERS = r'''
/// Chande Forecast Oscillator (CFO).
#[pyfunction]
#[pyo3(signature = (close, period=14))]
fn chande_forecast_oscillator(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let result = py.detach(|| {
        indicators::chande_forecast_oscillator(close, period)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Twiggs Money Flow (TMF).
#[pyfunction]
#[pyo3(signature = (high, low, close, volume, period=14))]
fn twiggs_money_flow(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Py<PyArray1<f64>>> {
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
    let result = py.detach(|| {
        indicators::twiggs_money_flow(high, low, close, volume, period)
            .map(|arr| arr.into_raw_vec())
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

'''


def main() -> int:
    text = LIB.read_text(encoding="utf-8")
    missing = []
    if "fn chande_forecast_oscillator(" not in text:
        missing.append("chande_forecast_oscillator")
    if "fn twiggs_money_flow(" not in text:
        missing.append("twiggs_money_flow")
    if not missing:
        print("Python CFO/TMF wrappers already present")
        return 0
    if set(missing) != {"chande_forecast_oscillator", "twiggs_money_flow"}:
        raise RuntimeError(f"partial CFO/TMF binding state requires review: {missing}")
    if MARKER not in text:
        raise RuntimeError("Formula System marker not found")
    text = text.replace(MARKER, WRAPPERS + MARKER, 1)
    LIB.write_text(text, encoding="utf-8")
    print("restored ndarray-direct CFO/TMF Python wrappers")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
