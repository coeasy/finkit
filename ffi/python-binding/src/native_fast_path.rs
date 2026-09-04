//! Architecture v3 Python hot paths.
//!
//! Public functions remain registry-defined, while hot NumPy calls borrow input
//! slices and return NumPy-owned Rust vectors directly. Compatibility-sensitive
//! statistics and SAR use kernels that mirror TA-Lib core 0.7.1 semantics.

use ::finkit::indicators;
use ::finkit::math::{
    moving_avg, reduction, rolling_stats, sar as sar_kernel, typed_moving_avg, volume_kernels,
};
use numpy::{PyArray1, PyReadonlyArray1, PyReadwriteArray1};
use pyo3::prelude::*;
use std::mem::{forget, MaybeUninit};

#[inline]
fn value_error(error: impl std::fmt::Display) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(error.to_string())
}

#[inline]
fn validate_period(len: usize, period: usize) -> PyResult<()> {
    if period == 0 {
        return Err(value_error(
            "invalid parameter: period must be greater than 0",
        ));
    }
    if len < period {
        return Err(value_error(
            "input data length is less than required minimum",
        ));
    }
    Ok(())
}

#[inline]
fn validate_same_len(a: usize, b: usize) -> PyResult<()> {
    if a != b {
        return Err(value_error(
            "invalid parameter: input arrays must have the same length",
        ));
    }
    Ok(())
}

/// Sliding extrema with TA-Lib-style cached extreme indexes.
///
/// Typical technical-analysis windows are small (5-30). Keeping two full
/// monotonic queues costs O(n) memory and branch-heavy maintenance. Instead,
/// remember the current max/min indexes and only rescan the short window when
/// an extreme expires. Equal values choose the newest index (`>=` / `<=`),
/// matching the previous deque semantics.
fn rolling_extrema_map<F>(
    max_source: &[f64],
    min_source: &[f64],
    period: usize,
    mut map: F,
) -> Vec<f64>
where
    F: FnMut(usize, f64, f64) -> f64,
{
    let len = max_source.len();
    let mut output = vec![f64::NAN; len];
    if period == 0 || period > len {
        return output;
    }

    unsafe {
        let max_ptr = max_source.as_ptr();
        let min_ptr = min_source.as_ptr();
        let output_ptr = output.as_mut_ptr();

        let mut highest_idx = 0usize;
        let mut lowest_idx = 0usize;
        let mut highest = *max_ptr;
        let mut lowest = *min_ptr;
        for index in 1..period {
            let high = *max_ptr.add(index);
            let low = *min_ptr.add(index);
            if high >= highest {
                highest = high;
                highest_idx = index;
            }
            if low <= lowest {
                lowest = low;
                lowest_idx = index;
            }
        }
        *output_ptr.add(period - 1) = map(period - 1, highest, lowest);

        for index in period..len {
            let window_start = index + 1 - period;
            let new_high = *max_ptr.add(index);
            let new_low = *min_ptr.add(index);

            if highest_idx < window_start {
                highest = *max_ptr.add(window_start);
                highest_idx = window_start;
                for candidate in window_start + 1..=index {
                    let value = *max_ptr.add(candidate);
                    if value >= highest {
                        highest = value;
                        highest_idx = candidate;
                    }
                }
            } else if new_high >= highest {
                highest = new_high;
                highest_idx = index;
            }

            if lowest_idx < window_start {
                lowest = *min_ptr.add(window_start);
                lowest_idx = window_start;
                for candidate in window_start + 1..=index {
                    let value = *min_ptr.add(candidate);
                    if value <= lowest {
                        lowest = value;
                        lowest_idx = candidate;
                    }
                }
            } else if new_low <= lowest {
                lowest = new_low;
                lowest_idx = index;
            }

            *output_ptr.add(index) = map(index, highest, lowest);
        }
    }
    output
}

#[inline]
fn midpoint_vec(max_source: &[f64], min_source: &[f64], period: usize) -> Vec<f64> {
    rolling_extrema_map(max_source, min_source, period, |_, high, low| {
        (high + low) * 0.5
    })
}

#[inline]
fn willr_vec(high: &[f64], low: &[f64], close: &[f64], period: usize) -> Vec<f64> {
    rolling_extrema_map(high, low, period, |index, highest, lowest| {
        let range = highest - lowest;
        if range.abs() > 1e-15 {
            -100.0 * (highest - close[index]) / range
        } else {
            0.0
        }
    })
}

#[inline]
fn mom_vec(input: &[f64], period: usize) -> Vec<f64> {
    let len = input.len();
    let mut raw = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe {
        raw.set_len(len);
        let input_ptr = input.as_ptr();
        let output_ptr = raw.as_mut_ptr();
        for index in 0..period.min(len) {
            output_ptr.add(index).write(MaybeUninit::new(f64::NAN));
        }
        for index in period..len {
            output_ptr.add(index).write(MaybeUninit::new(
                *input_ptr.add(index) - *input_ptr.add(index - period),
            ));
        }
        let ptr = raw.as_mut_ptr().cast::<f64>();
        let capacity = raw.capacity();
        let length = raw.len();
        forget(raw);
        Vec::from_raw_parts(ptr, length, capacity)
    }
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

#[pyfunction(name = "_fast_mom")]
#[pyo3(signature = (close, timeperiod=10))]
fn fast_mom<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let close = close.as_slice().map_err(value_error)?;
    validate_period(close.len(), timeperiod)?;
    Ok(PyArray1::from_vec(py, py.detach(|| mom_vec(close, timeperiod))))
}

#[pyfunction(name = "_fast_unary_period")]
fn fast_unary_period<'py>(
    py: Python<'py>,
    operation: &str,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let close = close.as_slice().map_err(value_error)?;
    let output = match operation {
        "midpoint" => {
            validate_period(close.len(), timeperiod)?;
            py.detach(|| midpoint_vec(close, close, timeperiod))
        }
        "mom" => {
            validate_period(close.len(), timeperiod)?;
            py.detach(|| mom_vec(close, timeperiod))
        }
        "dema" => py
            .detach(|| moving_avg::dema(close, timeperiod))
            .map_err(value_error)?
            .into_raw_vec(),
        "tema" => py
            .detach(|| moving_avg::tema(close, timeperiod))
            .map_err(value_error)?
            .into_raw_vec(),
        "rsi" => py
            .detach(|| indicators::rsi(close, timeperiod))
            .map_err(value_error)?
            .into_raw_vec(),
        "roc" => py
            .detach(|| indicators::roc(close, timeperiod))
            .map_err(value_error)?
            .into_raw_vec(),
        "cmo" => py
            .detach(|| indicators::cmo(close, timeperiod))
            .map_err(value_error)?
            .into_raw_vec(),
        _ => {
            return Err(value_error(format!(
                "invalid parameter: unsupported fast operation {operation}"
            )))
        }
    };
    Ok(PyArray1::from_vec(py, output))
}

#[pyfunction(name = "_fast_unary_period_scale")]
fn fast_unary_period_scale<'py>(
    py: Python<'py>,
    operation: &str,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
    scale: f64,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let close = close.as_slice().map_err(value_error)?;
    let output = match operation {
        "stddev" => py
            .detach(|| rolling_stats::stddev(close, timeperiod, scale))
            .map_err(value_error)?,
        "var" => py
            .detach(|| rolling_stats::variance(close, timeperiod))
            .map_err(value_error)?,
        _ => {
            return Err(value_error(format!(
                "invalid parameter: unsupported fast operation {operation}"
            )))
        }
    };
    Ok(PyArray1::from_vec(py, output))
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
    validate_same_len(input_a.len(), input_b.len())?;
    let output = match operation {
        "midprice" => {
            validate_period(input_a.len(), timeperiod)?;
            py.detach(|| midpoint_vec(input_a, input_b, timeperiod))
        }
        "correl" => py
            .detach(|| rolling_stats::correlation(input_a, input_b, timeperiod))
            .map_err(value_error)?,
        _ => {
            return Err(value_error(format!(
                "invalid parameter: unsupported fast operation {operation}"
            )))
        }
    };
    Ok(PyArray1::from_vec(py, output))
}

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
    validate_same_len(high.len(), low.len())?;
    validate_same_len(high.len(), close.len())?;
    let output = match operation {
        "willr" => {
            validate_period(high.len(), timeperiod)?;
            py.detach(|| willr_vec(high, low, close, timeperiod))
        }
        "adx" => py
            .detach(|| indicators::adx(high, low, close, timeperiod))
            .map_err(value_error)?
            .into_raw_vec(),
        "cci" => py
            .detach(|| indicators::cci(high, low, close, timeperiod))
            .map_err(value_error)?
            .into_raw_vec(),
        "plus_di" => py
            .detach(|| indicators::plus_di(high, low, close, timeperiod))
            .map_err(value_error)?
            .into_raw_vec(),
        "minus_di" => py
            .detach(|| indicators::minus_di(high, low, close, timeperiod))
            .map_err(value_error)?
            .into_raw_vec(),
        "atr" => py
            .detach(|| indicators::atr(high, low, close, timeperiod))
            .map_err(value_error)?
            .into_raw_vec(),
        "natr" => py
            .detach(|| indicators::natr(high, low, close, timeperiod))
            .map_err(value_error)?
            .into_raw_vec(),
        _ => {
            return Err(value_error(format!(
                "invalid parameter: unsupported fast operation {operation}"
            )))
        }
    };
    Ok(PyArray1::from_vec(py, output))
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
    let (upper, middle, lower) = py
        .detach(|| rolling_stats::bbands_sma(close, timeperiod, nbdevup, nbdevdn))
        .map_err(value_error)?;
    Ok((
        PyArray1::from_vec(py, upper),
        PyArray1::from_vec(py, middle),
        PyArray1::from_vec(py, lower),
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
    let output = py
        .detach(|| sar_kernel::sar(high, low, acceleration, maximum))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output))
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

macro_rules! reduction_fn {
    ($name:ident, $py_name:literal, $ty:ty, $kernel:path) => {
        #[pyfunction(name = $py_name)]
        fn $name(data: PyReadonlyArray1<'_, $ty>) -> PyResult<$ty> {
            let data = data.as_slice().map_err(value_error)?;
            if data.is_empty() {
                return Err(value_error("input data is empty"));
            }
            Ok($kernel(data))
        }
    };
}

reduction_fn!(reduce_sum_f64, "_reduce_sum_f64", f64, reduction::sum_f64);
reduction_fn!(
    reduce_mean_f64,
    "_reduce_mean_f64",
    f64,
    reduction::mean_f64
);
reduction_fn!(reduce_min_f64, "_reduce_min_f64", f64, reduction::min_f64);
reduction_fn!(reduce_max_f64, "_reduce_max_f64", f64, reduction::max_f64);
reduction_fn!(
    reduce_stddev_f64,
    "_reduce_stddev_f64",
    f64,
    reduction::stddev_f64
);
reduction_fn!(reduce_sum_f32, "_reduce_sum_f32", f32, reduction::sum_f32);
reduction_fn!(
    reduce_mean_f32,
    "_reduce_mean_f32",
    f32,
    reduction::mean_f32
);
reduction_fn!(reduce_min_f32, "_reduce_min_f32", f32, reduction::min_f32);
reduction_fn!(reduce_max_f32, "_reduce_max_f32", f32, reduction::max_f32);
reduction_fn!(
    reduce_stddev_f32,
    "_reduce_stddev_f32",
    f32,
    reduction::stddev_f32
);

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
    m.add_function(wrap_pyfunction!(fast_mom, m)?)?;
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
