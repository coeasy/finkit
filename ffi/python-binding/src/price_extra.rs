//! Ichimoku / VWAP / Donchian / pivot wrappers exposed to Python.

use super::*;

#[pyfunction]
#[pyo3(signature = (high, low, close, tenkan_period=9, kijun_period=26, senkou_b_period=52))]
#[allow(clippy::type_complexity)]
pub fn vec_ichimoku_impl(
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

#[pyfunction]
#[pyo3(signature = (high, low, close, tenkan_period=9, kijun_period=26, senkou_b_period=52))]
#[allow(clippy::type_complexity)]
pub fn ichimoku(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    tenkan_period: usize,
    kijun_period: usize,
    senkou_b_period: usize,
) -> PyResult<(
    Py<PyArray1<f64>>,
    Py<PyArray1<f64>>,
    Py<PyArray1<f64>>,
    Py<PyArray1<f64>>,
    Py<PyArray1<f64>>,
)> {
    let (out_0, out_1, out_2, out_3, out_4) = vec_ichimoku_impl(
        py,
        high,
        low,
        close,
        tenkan_period,
        kijun_period,
        senkou_b_period,
    )?;
    Ok((
        PyArray1::from_vec(py, out_0).unbind(),
        PyArray1::from_vec(py, out_1).unbind(),
        PyArray1::from_vec(py, out_2).unbind(),
        PyArray1::from_vec(py, out_3).unbind(),
        PyArray1::from_vec(py, out_4).unbind(),
    ))
}

#[pyfunction]
#[pyo3(signature = (high, low, close, period=10, multiplier=3.0))]
#[allow(clippy::type_complexity)]
pub fn supertrend(
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

#[pyfunction]
#[pyo3(signature = (high, low, close, volume))]
pub fn vec_vwap_impl(
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

#[pyfunction]
#[pyo3(signature = (high, low, close, volume))]
pub fn vwap(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_vwap_impl(py, high, low, close, volume)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

#[pyfunction]
#[pyo3(signature = (high, low, close, volume, start_index))]
pub fn vec_anchored_vwap_impl(
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

#[pyfunction]
#[pyo3(signature = (high, low, close, volume, start_index))]
pub fn anchored_vwap(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    start_index: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_anchored_vwap_impl(py, high, low, close, volume, start_index)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

#[pyfunction]
#[pyo3(signature = (high, low, close, volume, timeperiod=20, nb_dev=2.0))]
pub fn vec_vwap_bands_impl(
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

#[pyfunction]
#[pyo3(signature = (high, low, close, volume, timeperiod=20, nb_dev=2.0))]
pub fn vwap_bands(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    timeperiod: usize,
    nb_dev: f64,
) -> PyResult<(Py<PyArray1<f64>>, Py<PyArray1<f64>>, Py<PyArray1<f64>>)> {
    let (out_0, out_1, out_2) =
        vec_vwap_bands_impl(py, high, low, close, volume, timeperiod, nb_dev)?;
    Ok((
        PyArray1::from_vec(py, out_0).unbind(),
        PyArray1::from_vec(py, out_1).unbind(),
        PyArray1::from_vec(py, out_2).unbind(),
    ))
}

#[pyfunction]
#[pyo3(signature = (high, low, close, volume, period=13))]
pub fn vec_elder_ray_impl(
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

#[pyfunction]
#[pyo3(signature = (high, low, close, volume, period=13))]
pub fn elder_ray(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<(Py<PyArray1<f64>>, Py<PyArray1<f64>>, Py<PyArray1<f64>>)> {
    let (out_0, out_1, out_2) = vec_elder_ray_impl(py, high, low, close, volume, period)?;
    Ok((
        PyArray1::from_vec(py, out_0).unbind(),
        PyArray1::from_vec(py, out_1).unbind(),
        PyArray1::from_vec(py, out_2).unbind(),
    ))
}

#[pyfunction]
#[pyo3(signature = (high, low, period=20))]
#[allow(clippy::type_complexity)]
pub fn vec_donchian_impl(
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

#[pyfunction]
#[pyo3(signature = (high, low, period=20))]
#[allow(clippy::type_complexity)]
pub fn donchian(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<(
    Py<PyArray1<f64>>,
    Py<PyArray1<f64>>,
    Py<PyArray1<f64>>,
    Py<PyArray1<f64>>,
)> {
    let (out_0, out_1, out_2, out_3) = vec_donchian_impl(py, high, low, period)?;
    Ok((
        PyArray1::from_vec(py, out_0).unbind(),
        PyArray1::from_vec(py, out_1).unbind(),
        PyArray1::from_vec(py, out_2).unbind(),
        PyArray1::from_vec(py, out_3).unbind(),
    ))
}

#[pyfunction]
#[pyo3(signature = (high, low, close, method="standard"))]
#[allow(clippy::type_complexity)]
pub fn vec_pivot_points_impl(
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

#[pyfunction]
#[pyo3(signature = (high, low, close, method="standard"))]
#[allow(clippy::type_complexity)]
pub fn pivot_points(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    method: &str,
) -> PyResult<(
    Py<PyArray1<f64>>,
    Py<PyArray1<f64>>,
    Py<PyArray1<f64>>,
    Py<PyArray1<f64>>,
    Py<PyArray1<f64>>,
    Py<PyArray1<f64>>,
    Py<PyArray1<f64>>,
)> {
    let (out_0, out_1, out_2, out_3, out_4, out_5, out_6) =
        vec_pivot_points_impl(py, high, low, close, method)?;
    Ok((
        PyArray1::from_vec(py, out_0).unbind(),
        PyArray1::from_vec(py, out_1).unbind(),
        PyArray1::from_vec(py, out_2).unbind(),
        PyArray1::from_vec(py, out_3).unbind(),
        PyArray1::from_vec(py, out_4).unbind(),
        PyArray1::from_vec(py, out_5).unbind(),
        PyArray1::from_vec(py, out_6).unbind(),
    ))
}

#[pyfunction]
#[pyo3(signature = (high, low, close, volume, num_bins=24))]
pub fn volume_profile(
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

#[pyfunction]
#[pyo3(signature = (high, low, start_index, end_index))]
pub fn fibonacci_retracement(
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
