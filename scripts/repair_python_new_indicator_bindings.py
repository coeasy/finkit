#!/usr/bin/env python3
"""Repair two registered Python APIs that are not part of the C-ABI registry.

CFO and Twiggs Money Flow have Rust core implementations and Python module
registrations, but they are not members of the 78-entry C ABI SSOT consumed by
sync_bindings.py. Therefore generated.rs is the wrong ownership boundary: every
SSOT regeneration correctly replaces that file and drops these extra wrappers.
Keep these two ndarray-direct wrappers in lib.rs, while generated.rs remains
owned exclusively by the registry generator.
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
    have_cfo = "fn chande_forecast_oscillator(" in text
    have_tmf = "fn twiggs_money_flow(" in text
    if have_cfo and have_tmf:
        print("Python CFO/TMF lib.rs wrappers already present")
        return 0
    if have_cfo != have_tmf:
        raise RuntimeError("partial CFO/TMF lib.rs binding state requires review")
    if MARKER not in text:
        raise RuntimeError("Formula System marker not found")
    LIB.write_text(text.replace(MARKER, WRAPPERS + MARKER, 1), encoding="utf-8")
    print("restored ndarray-direct CFO/TMF wrappers outside generated.rs")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
