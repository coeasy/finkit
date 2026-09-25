//! Python bindings for the feature engineering module.

use finkit::features;
use finkit::indicators;
use finkit_factor_analysis::{
    AssetId, FactorStudy, GroupId, PanelIndex, QuantizeConfig, ResearchFrame,
};
use numpy::{PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;
use pyo3::types::PyDict;

type PyDictResult<'py> = PyResult<Bound<'py, PyDict>>;

// ============================================================================
// Multi-period Features
// ============================================================================

/// Generate a FeatureSet with multiple indicators and periods.
///
/// Returns a dict mapping feature names to numpy arrays.
///
/// # Arguments
/// * `close` - Close price data
/// * `indicators` - List of (indicator_name, [periods]) tuples
#[pyfunction]
fn feature_set(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    indicators: Vec<(String, Vec<usize>)>,
) -> PyResult<Vec<(String, Vec<f64>)>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        let mut engine = features::FeatureSet::new();
        for (name, periods) in &indicators {
            engine.add_indicator(name, periods);
        }
        let matrix = engine.generate(close);
        let result: Vec<(String, Vec<f64>)> = matrix
            .column_names()
            .iter()
            .enumerate()
            .map(|(i, name)| (name.to_string(), matrix.column(i).to_vec()))
            .collect();
        Ok(result)
    })
}

// ============================================================================
// Rolling Statistics
// ============================================================================

/// Rolling skewness over a window.
#[pyfunction]
fn vec_rolling_skewness_impl(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::rolling_skewness(data, window).to_vec()))
}

#[pyfunction]
fn rolling_skewness(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_rolling_skewness_impl(py, data, window)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Rolling kurtosis over a window.
#[pyfunction]
fn vec_rolling_kurtosis_impl(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::rolling_kurtosis(data, window).to_vec()))
}

#[pyfunction]
fn rolling_kurtosis(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_rolling_kurtosis_impl(py, data, window)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Rolling entropy (information entropy using histogram binning).
#[pyfunction]
fn vec_rolling_entropy_impl(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
    bins: usize,
) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::rolling_entropy(data, window, bins).to_vec()))
}

#[pyfunction]
fn rolling_entropy(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
    bins: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_rolling_entropy_impl(py, data, window, bins)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Rolling z-score.
#[pyfunction]
fn vec_rolling_zscore_impl(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::rolling_zscore(data, window).to_vec()))
}

#[pyfunction]
fn rolling_zscore(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_rolling_zscore_impl(py, data, window)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Rolling percentile rank within window.
#[pyfunction]
fn vec_rolling_percentile_impl(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::rolling_percentile(data, window).to_vec()))
}

#[pyfunction]
fn rolling_percentile(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_rolling_percentile_impl(py, data, window)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

// ============================================================================
// Normalization
// ============================================================================

/// Rolling z-score normalization.
#[pyfunction]
fn vec_rolling_zscore_normalize_impl(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::rolling_zscore_normalize(data, window).to_vec()))
}

#[pyfunction]
fn rolling_zscore_normalize(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_rolling_zscore_normalize_impl(py, data, window)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Rolling min-max normalization to [0, 1].
#[pyfunction]
fn vec_rolling_minmax_impl(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::rolling_minmax(data, window).to_vec()))
}

#[pyfunction]
fn rolling_minmax(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_rolling_minmax_impl(py, data, window)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Robust scaler using median and IQR.
#[pyfunction]
fn vec_robust_scaler_impl(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::robust_scaler(data, window).to_vec()))
}

#[pyfunction]
fn robust_scaler(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_robust_scaler_impl(py, data, window)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Rank normalization within a rolling window.
#[pyfunction]
fn vec_rank_normalize_impl(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::rank_normalize(data, window).to_vec()))
}

#[pyfunction]
fn rank_normalize(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_rank_normalize_impl(py, data, window)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

// ============================================================================
// Time Series
// ============================================================================

/// Lag (shift forward) by n positions.
#[pyfunction]
fn vec_lag_impl(py: Python<'_>, data: PyReadonlyArray1<'_, f64>, n: usize) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::lag(data, n).to_vec()))
}

#[pyfunction]
fn lag(py: Python<'_>, data: PyReadonlyArray1<'_, f64>, n: usize) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_lag_impl(py, data, n)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Lead (shift backward) by n positions.
#[pyfunction]
fn vec_lead_impl(py: Python<'_>, data: PyReadonlyArray1<'_, f64>, n: usize) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::lead(data, n).to_vec()))
}

#[pyfunction]
fn lead(py: Python<'_>, data: PyReadonlyArray1<'_, f64>, n: usize) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_lead_impl(py, data, n)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// N-th order difference.
#[pyfunction]
fn vec_diff_impl(py: Python<'_>, data: PyReadonlyArray1<'_, f64>, n: usize) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::diff(data, n).to_vec()))
}

#[pyfunction]
fn diff(py: Python<'_>, data: PyReadonlyArray1<'_, f64>, n: usize) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_diff_impl(py, data, n)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Percentage change over n periods.
#[pyfunction]
fn vec_pct_change_impl(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    n: usize,
) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::pct_change(data, n).to_vec()))
}

#[pyfunction]
fn pct_change(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    n: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_pct_change_impl(py, data, n)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

// ============================================================================
// Labels
// ============================================================================

/// Forward log return over n periods.
#[pyfunction]
fn vec_forward_return_impl(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    n: usize,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::forward_return(close, n).to_vec()))
}

#[pyfunction]
fn forward_return(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    n: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_forward_return_impl(py, close, n)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Binary label: 1 if forward return > threshold, else 0.
#[pyfunction]
fn vec_binary_label_impl(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    n: usize,
    threshold: f64,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::binary_label(close, n, threshold).to_vec()))
}

#[pyfunction]
fn binary_label(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    n: usize,
    threshold: f64,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_binary_label_impl(py, close, n, threshold)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Fixed horizon label: +1, 0, -1 based on return vs threshold.
#[pyfunction]
fn vec_fixed_horizon_label_impl(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    horizon: usize,
    threshold: f64,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::fixed_horizon_label(close, horizon, threshold).to_vec()))
}

#[pyfunction]
fn fixed_horizon_label(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    horizon: usize,
    threshold: f64,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_fixed_horizon_label_impl(py, close, horizon, threshold)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Triple barrier label (López de Prado method).
///
/// Returns list of (label, duration, return) tuples.
#[pyfunction]
fn triple_barrier(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    pt_factor: f64,
    sl_factor: f64,
    max_hold: usize,
) -> PyResult<Vec<(i8, usize, f64)>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| {
        features::triple_barrier(close, high, low, pt_factor, sl_factor, max_hold)
            .into_iter()
            .map(|b| (b.label, b.duration, b.ret))
            .collect()
    }))
}

// ============================================================================
// Combinations
// ============================================================================

/// Element-wise ratio: a[i] / b[i].
#[pyfunction]
fn vec_feature_ratio_impl(
    py: Python<'_>,
    a: PyReadonlyArray1<'_, f64>,
    b: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<f64>> {
    let a = a
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let b = b
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::feature_ratio(a, b).to_vec()))
}

#[pyfunction]
fn feature_ratio(
    py: Python<'_>,
    a: PyReadonlyArray1<'_, f64>,
    b: PyReadonlyArray1<'_, f64>,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_feature_ratio_impl(py, a, b)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Element-wise spread: a[i] - b[i].
#[pyfunction]
fn vec_feature_spread_impl(
    py: Python<'_>,
    a: PyReadonlyArray1<'_, f64>,
    b: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<f64>> {
    let a = a
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let b = b
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::feature_spread(a, b).to_vec()))
}

#[pyfunction]
fn feature_spread(
    py: Python<'_>,
    a: PyReadonlyArray1<'_, f64>,
    b: PyReadonlyArray1<'_, f64>,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_feature_spread_impl(py, a, b)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Rolling Pearson correlation between two series.
#[pyfunction]
fn vec_rolling_correlation_impl(
    py: Python<'_>,
    a: PyReadonlyArray1<'_, f64>,
    b: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Vec<f64>> {
    let a = a
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let b = b
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::rolling_correlation(a, b, window).to_vec()))
}

#[pyfunction]
fn rolling_correlation(
    py: Python<'_>,
    a: PyReadonlyArray1<'_, f64>,
    b: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_rolling_correlation_impl(py, a, b, window)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

// ============================================================================
// SIMD Batch Operations
// ============================================================================

/// Batch z-score normalization (SIMD-optimized).
#[pyfunction]
fn vec_batch_zscore_impl(py: Python<'_>, data: PyReadonlyArray1<'_, f64>) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::batch_zscore_simd(data).to_vec()))
}

#[pyfunction]
fn batch_zscore(py: Python<'_>, data: PyReadonlyArray1<'_, f64>) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_batch_zscore_impl(py, data)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Batch min-max normalization (SIMD-optimized).
#[pyfunction]
fn vec_batch_minmax_impl(py: Python<'_>, data: PyReadonlyArray1<'_, f64>) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::batch_minmax_simd(data).to_vec()))
}

#[pyfunction]
fn batch_minmax(py: Python<'_>, data: PyReadonlyArray1<'_, f64>) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_batch_minmax_impl(py, data)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Pearson correlation between two arrays (SIMD-optimized).
#[pyfunction]
fn correlation(
    py: Python<'_>,
    a: PyReadonlyArray1<'_, f64>,
    b: PyReadonlyArray1<'_, f64>,
) -> PyResult<f64> {
    let a = a
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let b = b
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::correlation_simd(a, b)))
}

// ============================================================================
// Overlap Indicators
// ============================================================================

/// Hull Moving Average (HMA).
#[pyfunction]
#[pyo3(signature = (close, period=14))]
fn vec_hma_impl(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::hma(close, period)
            .map(|arr| arr.into_raw_vec_and_offset().0)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (close, period=14))]
fn hma(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_hma_impl(py, close, period)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Arnaud Legoux Moving Average (ALMA).
#[pyfunction]
#[pyo3(signature = (close, period=9, offset_factor=0.85, sigma=6.0))]
fn vec_alma_impl(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
    offset_factor: f64,
    sigma: f64,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::alma(close, period, offset_factor, sigma)
            .map(|arr| arr.into_raw_vec_and_offset().0)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (close, period=9, offset_factor=0.85, sigma=6.0))]
fn alma(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
    offset_factor: f64,
    sigma: f64,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_alma_impl(py, close, period, offset_factor, sigma)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Variable Index Dynamic Average (VIDYA).
#[pyfunction]
#[pyo3(signature = (close, short_period=9, long_period=14))]
fn vec_vidya_impl(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    short_period: usize,
    long_period: usize,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::vidya(close, short_period, long_period)
            .map(|arr| arr.into_raw_vec_and_offset().0)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (close, short_period=9, long_period=14))]
fn vidya(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    short_period: usize,
    long_period: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_vidya_impl(py, close, short_period, long_period)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// MESA Adaptive Moving Average (MAMA).
#[pyfunction]
#[pyo3(signature = (close, fastlimit=0.5, slowlimit=0.05))]
fn mama<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'_, f64>,
    fastlimit: f64,
    slowlimit: f64,
) -> PyDictResult<'py> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let result = py.detach(|| {
        indicators::mama(close, fastlimit, slowlimit)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })?;
    let dict = PyDict::new(py);
    dict.set_item("mama", result.mama.into_raw_vec_and_offset().0)?;
    dict.set_item("fama", result.fama.into_raw_vec_and_offset().0)?;
    Ok(dict)
}

/// Fractal Adaptive Moving Average (FRAMA).
#[pyfunction]
#[pyo3(signature = (close, period=16))]
fn vec_frama_impl(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::frama(close, period)
            .map(|arr| arr.into_raw_vec_and_offset().0)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (close, period=16))]
fn frama(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_frama_impl(py, close, period)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

// ============================================================================
// Extended Momentum Indicators
// ============================================================================

/// Connors RSI composite oscillator.
#[pyfunction]
#[pyo3(signature = (close, rsi_period=3, streak_period=2, rank_period=100))]
fn vec_connors_rsi_impl(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    rsi_period: usize,
    streak_period: usize,
    rank_period: usize,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::connors_rsi(close, rsi_period, streak_period, rank_period)
            .map(|arr| arr.into_raw_vec_and_offset().0)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (close, rsi_period=3, streak_period=2, rank_period=100))]
fn connors_rsi(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    rsi_period: usize,
    streak_period: usize,
    rank_period: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_connors_rsi_impl(py, close, rsi_period, streak_period, rank_period)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Stochastic RSI.
#[pyfunction]
#[pyo3(signature = (close, rsi_period=14, stoch_period=14, k_period=3, d_period=3))]
fn stoch_rsi<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'_, f64>,
    rsi_period: usize,
    stoch_period: usize,
    k_period: usize,
    d_period: usize,
) -> PyDictResult<'py> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let result = py.detach(|| {
        indicators::stoch_rsi(close, rsi_period, stoch_period, k_period, d_period)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })?;
    let dict = PyDict::new(py);
    dict.set_item("k", result.k.into_raw_vec_and_offset().0)?;
    dict.set_item("d", result.d.into_raw_vec_and_offset().0)?;
    Ok(dict)
}

/// Relative Vigor Index (RVI).
#[pyfunction]
#[pyo3(signature = (open, high, low, close, period=10))]
fn rvi<'py>(
    py: Python<'py>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyDictResult<'py> {
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
    let result = py.detach(|| {
        indicators::rvi(open, high, low, close, period)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })?;
    let dict = PyDict::new(py);
    dict.set_item("rvi", result.rvi.into_raw_vec_and_offset().0)?;
    dict.set_item("signal", result.signal.into_raw_vec_and_offset().0)?;
    Ok(dict)
}

// ============================================================================
// Extended Volatility Indicators
// ============================================================================

/// Garman-Klass volatility estimator.
#[pyfunction]
#[pyo3(signature = (open, high, low, close, period=14))]
fn vec_garman_klass_volatility_impl(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Vec<f64>> {
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
        indicators::garman_klass_volatility(open, high, low, close, period)
            .map(|arr| arr.into_raw_vec_and_offset().0)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (open, high, low, close, period=14))]
fn garman_klass_volatility(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_garman_klass_volatility_impl(py, open, high, low, close, period)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Parkinson volatility estimator.
#[pyfunction]
#[pyo3(signature = (high, low, period=14))]
fn vec_parkinson_volatility_impl(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Vec<f64>> {
    let high = high
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low = low
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::parkinson_volatility(high, low, period)
            .map(|arr| arr.into_raw_vec_and_offset().0)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (high, low, period=14))]
fn parkinson_volatility(
    py: Python<'_>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_parkinson_volatility_impl(py, high, low, period)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Rogers-Satchell volatility estimator.
#[pyfunction]
#[pyo3(signature = (open, high, low, close, period=14))]
fn vec_rogers_satchell_volatility_impl(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Vec<f64>> {
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
        indicators::rogers_satchell_volatility(open, high, low, close, period)
            .map(|arr| arr.into_raw_vec_and_offset().0)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (open, high, low, close, period=14))]
fn rogers_satchell_volatility(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_rogers_satchell_volatility_impl(py, open, high, low, close, period)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Yang-Zhang volatility estimator.
#[pyfunction]
#[pyo3(signature = (open, high, low, close, period=14))]
fn vec_yang_zhang_volatility_impl(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Vec<f64>> {
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
        indicators::yang_zhang_volatility(open, high, low, close, period)
            .map(|arr| arr.into_raw_vec_and_offset().0)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (open, high, low, close, period=14))]
fn yang_zhang_volatility(
    py: Python<'_>,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_yang_zhang_volatility_impl(py, open, high, low, close, period)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Realized volatility from log returns.
#[pyfunction]
#[pyo3(signature = (close, period=14))]
fn vec_realized_volatility_impl(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::realized_volatility(close, period)
            .map(|arr| arr.into_raw_vec_and_offset().0)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (close, period=14))]
fn realized_volatility(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_realized_volatility_impl(py, close, period)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Semivariance (downside volatility).
#[pyfunction]
#[pyo3(signature = (close, period=14))]
fn vec_semivariance_impl(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    py.detach(|| {
        indicators::semivariance(close, period)
            .map(|arr| arr.into_raw_vec_and_offset().0)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })
}

#[pyfunction]
#[pyo3(signature = (close, period=14))]
fn semivariance(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    period: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_semivariance_impl(py, close, period)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

// ============================================================================
// Extended Volume Indicators
// ============================================================================

/// Volume Weighted MACD (VWMACD).
#[pyfunction]
#[pyo3(signature = (close, volume, fastperiod=12, slowperiod=26, signalperiod=9))]
fn vwmacd<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    fastperiod: usize,
    slowperiod: usize,
    signalperiod: usize,
) -> PyDictResult<'py> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let volume = volume
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let result = py.detach(|| {
        indicators::vwmacd(close, volume, fastperiod, slowperiod, signalperiod)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })?;
    let dict = PyDict::new(py);
    dict.set_item("macd", result.macd.into_raw_vec_and_offset().0)?;
    dict.set_item("signal", result.signal.into_raw_vec_and_offset().0)?;
    dict.set_item("hist", result.hist.into_raw_vec_and_offset().0)?;
    Ok(dict)
}

// ============================================================================
// Extended Rolling Statistics
// ============================================================================

/// Hurst exponent via R/S analysis.
#[pyfunction]
#[pyo3(signature = (data, min_window=20))]
fn hurst_exponent(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    min_window: usize,
) -> PyResult<f64> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::hurst_exponent(data, min_window)))
}

/// Autocorrelation function (ACF).
#[pyfunction]
#[pyo3(signature = (data, max_lag=10))]
fn vec_acf_impl(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    max_lag: usize,
) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::acf(data, max_lag)))
}

#[pyfunction]
#[pyo3(signature = (data, max_lag=10))]
fn acf(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    max_lag: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_acf_impl(py, data, max_lag)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Partial autocorrelation function (PACF).
#[pyfunction]
#[pyo3(signature = (data, max_lag=10))]
fn vec_pacf_impl(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    max_lag: usize,
) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::pacf(data, max_lag)))
}

#[pyfunction]
#[pyo3(signature = (data, max_lag=10))]
fn pacf(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    max_lag: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_pacf_impl(py, data, max_lag)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Augmented Dickey-Fuller unit-root test.
#[pyfunction]
#[pyo3(signature = (data, max_lag=1))]
fn adf_test<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'_, f64>,
    max_lag: usize,
) -> PyDictResult<'py> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let result = py.detach(|| {
        features::adf_test(data, max_lag)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })?;
    let dict = PyDict::new(py);
    dict.set_item("test_statistic", result.test_statistic)?;
    dict.set_item("p_value", result.p_value)?;
    dict.set_item("lags_used", result.lags_used)?;
    dict.set_item("is_stationary", result.is_stationary)?;
    Ok(dict)
}

/// Engle-Granger cointegration test.
#[pyfunction]
#[pyo3(signature = (series_x, series_y, max_lag=1))]
fn cointegration_test<'py>(
    py: Python<'py>,
    series_x: PyReadonlyArray1<'_, f64>,
    series_y: PyReadonlyArray1<'_, f64>,
    max_lag: usize,
) -> PyDictResult<'py> {
    let series_x = series_x
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let series_y = series_y
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let result = py.detach(|| {
        features::cointegration_test(series_x, series_y, max_lag)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))
    })?;
    let dict = PyDict::new(py);
    dict.set_item("test_statistic", result.test_statistic)?;
    dict.set_item("p_value", result.p_value)?;
    dict.set_item(
        "cointegration_coefficient",
        result.cointegration_coefficient,
    )?;
    dict.set_item("is_cointegrated", result.is_cointegrated)?;
    Ok(dict)
}

// ============================================================================
// Cross Features
// ============================================================================

/// Element-wise cross product: a[i] * b[i].
#[pyfunction]
fn vec_feature_cross_impl(
    py: Python<'_>,
    a: PyReadonlyArray1<'_, f64>,
    b: PyReadonlyArray1<'_, f64>,
) -> PyResult<Vec<f64>> {
    let a = a
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let b = b
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::feature_cross(a, b).to_vec()))
}

#[pyfunction]
fn feature_cross(
    py: Python<'_>,
    a: PyReadonlyArray1<'_, f64>,
    b: PyReadonlyArray1<'_, f64>,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_feature_cross_impl(py, a, b)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Generate all pairwise cross-product features.
///
/// Returns a list of (column_name, values) tuples.
#[pyfunction]
fn auto_cross(
    py: Python<'_>,
    columns: Vec<(String, PyReadonlyArray1<'_, f64>)>,
) -> PyResult<Vec<(String, Vec<f64>)>> {
    let columns_owned: Vec<(String, Vec<f64>)> = columns
        .into_iter()
        .map(|(name, arr)| {
            let slice = arr
                .as_slice()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
            Ok((name, slice.to_vec()))
        })
        .collect::<PyResult<_>>()?;
    Ok(py.detach(move || {
        let refs: Vec<(&str, &[f64])> = columns_owned
            .iter()
            .map(|(name, data)| (name.as_str(), data.as_slice()))
            .collect();
        let matrix = features::auto_cross(&refs);
        matrix
            .column_names()
            .iter()
            .enumerate()
            .map(|(i, name)| (name.to_string(), matrix.column(i).to_vec()))
            .collect()
    }))
}

// ============================================================================
// Microstructure Features
// ============================================================================

/// Tick imbalance: rolling mean of price direction signs.
#[pyfunction]
#[pyo3(signature = (close, window=20))]
fn vec_tick_imbalance_impl(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::tick_imbalance(close, window).to_vec()))
}

#[pyfunction]
#[pyo3(signature = (close, window=20))]
fn tick_imbalance(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_tick_imbalance_impl(py, close, window)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Volume imbalance: rolling ratio of signed volume to total volume.
#[pyfunction]
#[pyo3(signature = (close, volume, window=20))]
fn vec_volume_imbalance_impl(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let volume = volume
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::volume_imbalance(close, volume, window).to_vec()))
}

#[pyfunction]
#[pyo3(signature = (close, volume, window=20))]
fn volume_imbalance(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_volume_imbalance_impl(py, close, volume, window)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Kyle's lambda: rolling price impact coefficient.
#[pyfunction]
#[pyo3(signature = (close, volume, window=20))]
fn vec_kyle_lambda_impl(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let volume = volume
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::kyle_lambda(close, volume, window).to_vec()))
}

#[pyfunction]
#[pyo3(signature = (close, volume, window=20))]
fn kyle_lambda(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_kyle_lambda_impl(py, close, volume, window)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Roll (1984) implied spread estimator.
#[pyfunction]
#[pyo3(signature = (close, window=20))]
fn vec_roll_spread_impl(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Vec<f64>> {
    let close = close
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::roll_spread(close, window).to_vec()))
}

#[pyfunction]
#[pyo3(signature = (close, window=20))]
fn roll_spread(
    py: Python<'_>,
    close: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_roll_spread_impl(py, close, window)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

// ============================================================================
// Regime Detection
// ============================================================================

/// Threshold-based volatility regime classifier.
#[pyfunction]
#[pyo3(signature = (data, window=20, low_pct=25.0, high_pct=75.0))]
fn vec_threshold_regime_impl(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
    low_pct: f64,
    high_pct: f64,
) -> PyResult<Vec<f64>> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| features::threshold_regime(data, window, low_pct, high_pct).to_vec()))
}

#[pyfunction]
#[pyo3(signature = (data, window=20, low_pct=25.0, high_pct=75.0))]
fn threshold_regime(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
    low_pct: f64,
    high_pct: f64,
) -> PyResult<Py<PyArray1<f64>>> {
    let result = vec_threshold_regime_impl(py, data, window, low_pct, high_pct)?;
    Ok(PyArray1::from_vec(py, result).unbind())
}

/// Gaussian HMM regime detection.
#[pyfunction]
#[pyo3(signature = (data, n_states=2, max_iter=50))]
fn hmm_regime<'py>(
    py: Python<'py>,
    data: PyReadonlyArray1<'_, f64>,
    n_states: usize,
    max_iter: usize,
) -> PyDictResult<'py> {
    let data = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let result = py.detach(|| features::hmm_regime(data, n_states, max_iter));
    let dict = PyDict::new(py);
    dict.set_item("states", result.states.to_vec())?;
    dict.set_item("state_probs", result.state_probs)?;
    dict.set_item("means", result.means)?;
    dict.set_item("stds", result.stds)?;
    Ok(dict)
}

/// Run a complete panel-aware factor study and return its serializable JSON report.
///
/// Rows must be sorted by `(timestamp, asset)` or at least grouped by ascending timestamp.
/// `groups` is optional and enables date-local group quantization.
#[pyfunction]
#[pyo3(signature = (timestamps, assets, factor, prices, periods, quantiles=5, groups=None))]
fn factor_study_json(
    py: Python<'_>,
    timestamps: PyReadonlyArray1<'_, i64>,
    assets: PyReadonlyArray1<'_, u32>,
    factor: PyReadonlyArray1<'_, f64>,
    prices: PyReadonlyArray1<'_, f64>,
    periods: Vec<usize>,
    quantiles: u16,
    groups: Option<PyReadonlyArray1<'_, u32>>,
) -> PyResult<String> {
    let timestamps = timestamps
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))?
        .to_vec();
    let assets: Vec<AssetId> = assets
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))?
        .iter()
        .copied()
        .map(AssetId)
        .collect();
    let factor = factor
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))?
        .to_vec();
    let prices = prices
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))?
        .to_vec();
    let groups: Option<Vec<GroupId>> = groups
        .map(|values| {
            values
                .as_slice()
                .map(|slice| slice.iter().copied().map(GroupId).collect())
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{e}")))
        })
        .transpose()?;

    py.detach(|| -> Result<String, String> {
        let index = PanelIndex::new(timestamps, assets).map_err(|e| e.to_string())?;
        let mut frame = ResearchFrame::new(index);
        frame
            .add_numeric("factor", "factor", factor)
            .map_err(|e| e.to_string())?;
        frame
            .add_numeric("price", "market", prices)
            .map_err(|e| e.to_string())?;
        if let Some(groups) = groups {
            frame
                .add_group("group", groups)
                .map_err(|e| e.to_string())?;
        }
        let by_group = frame.has_group("group").then(|| "group".to_string());
        let report = FactorStudy::new(&frame, "factor", "price", periods)
            .quantize_config(QuantizeConfig {
                quantiles,
                by_group,
                zero_aware: false,
            })
            .full_report()
            .map_err(|e| e.to_string())?;
        report.to_json_pretty().map_err(|e| e.to_string())
    })
    .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))
}

/// Register the features submodule.
pub fn register_features_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "features")?;
    m.add_function(wrap_pyfunction!(feature_set, &m)?)?;
    m.add_function(wrap_pyfunction!(rolling_skewness, &m)?)?;
    m.add_function(wrap_pyfunction!(rolling_kurtosis, &m)?)?;
    m.add_function(wrap_pyfunction!(rolling_entropy, &m)?)?;
    m.add_function(wrap_pyfunction!(rolling_zscore, &m)?)?;
    m.add_function(wrap_pyfunction!(rolling_percentile, &m)?)?;
    m.add_function(wrap_pyfunction!(rolling_zscore_normalize, &m)?)?;
    m.add_function(wrap_pyfunction!(rolling_minmax, &m)?)?;
    m.add_function(wrap_pyfunction!(robust_scaler, &m)?)?;
    m.add_function(wrap_pyfunction!(rank_normalize, &m)?)?;
    m.add_function(wrap_pyfunction!(lag, &m)?)?;
    m.add_function(wrap_pyfunction!(lead, &m)?)?;
    m.add_function(wrap_pyfunction!(diff, &m)?)?;
    m.add_function(wrap_pyfunction!(pct_change, &m)?)?;
    m.add_function(wrap_pyfunction!(forward_return, &m)?)?;
    m.add_function(wrap_pyfunction!(binary_label, &m)?)?;
    m.add_function(wrap_pyfunction!(fixed_horizon_label, &m)?)?;
    m.add_function(wrap_pyfunction!(triple_barrier, &m)?)?;
    m.add_function(wrap_pyfunction!(feature_ratio, &m)?)?;
    m.add_function(wrap_pyfunction!(feature_spread, &m)?)?;
    m.add_function(wrap_pyfunction!(rolling_correlation, &m)?)?;
    m.add_function(wrap_pyfunction!(batch_zscore, &m)?)?;
    m.add_function(wrap_pyfunction!(batch_minmax, &m)?)?;
    m.add_function(wrap_pyfunction!(correlation, &m)?)?;
    m.add_function(wrap_pyfunction!(hma, &m)?)?;
    m.add_function(wrap_pyfunction!(alma, &m)?)?;
    m.add_function(wrap_pyfunction!(vidya, &m)?)?;
    m.add_function(wrap_pyfunction!(mama, &m)?)?;
    m.add_function(wrap_pyfunction!(frama, &m)?)?;
    m.add_function(wrap_pyfunction!(connors_rsi, &m)?)?;
    m.add_function(wrap_pyfunction!(stoch_rsi, &m)?)?;
    m.add_function(wrap_pyfunction!(rvi, &m)?)?;
    m.add_function(wrap_pyfunction!(garman_klass_volatility, &m)?)?;
    m.add_function(wrap_pyfunction!(parkinson_volatility, &m)?)?;
    m.add_function(wrap_pyfunction!(rogers_satchell_volatility, &m)?)?;
    m.add_function(wrap_pyfunction!(yang_zhang_volatility, &m)?)?;
    m.add_function(wrap_pyfunction!(realized_volatility, &m)?)?;
    m.add_function(wrap_pyfunction!(semivariance, &m)?)?;
    m.add_function(wrap_pyfunction!(vwmacd, &m)?)?;
    m.add_function(wrap_pyfunction!(hurst_exponent, &m)?)?;
    m.add_function(wrap_pyfunction!(acf, &m)?)?;
    m.add_function(wrap_pyfunction!(pacf, &m)?)?;
    m.add_function(wrap_pyfunction!(adf_test, &m)?)?;
    m.add_function(wrap_pyfunction!(cointegration_test, &m)?)?;
    m.add_function(wrap_pyfunction!(feature_cross, &m)?)?;
    m.add_function(wrap_pyfunction!(auto_cross, &m)?)?;
    m.add_function(wrap_pyfunction!(tick_imbalance, &m)?)?;
    m.add_function(wrap_pyfunction!(volume_imbalance, &m)?)?;
    m.add_function(wrap_pyfunction!(kyle_lambda, &m)?)?;
    m.add_function(wrap_pyfunction!(roll_spread, &m)?)?;
    m.add_function(wrap_pyfunction!(threshold_regime, &m)?)?;
    m.add_function(wrap_pyfunction!(hmm_regime, &m)?)?;
    m.add_function(wrap_pyfunction!(factor_study_json, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
