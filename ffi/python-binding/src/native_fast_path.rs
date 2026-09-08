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
fn rate_change_vec(input: &[f64], period: usize, mode: u8) -> PyResult<Vec<f64>> {
    validate_period(input.len(), period + 1)?;
    let len = input.len();
    let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe { raw_output.set_len(len) };
    let output = unsafe { std::slice::from_raw_parts_mut(raw_output.as_mut_ptr().cast(), len) };
    output[..period].fill(f64::NAN);
    #[cfg(all(target_arch = "x86_64"))]
    if is_x86_feature_detected!("avx2") {
        unsafe { rate_change_avx2(input, period, mode, output) };
    } else {
        rate_change_scalar(input, period, mode, output);
    }
    #[cfg(not(all(target_arch = "x86_64")))]
    rate_change_scalar(input, period, mode, output);
    let ptr = raw_output.as_mut_ptr().cast::<f64>();
    let capacity = raw_output.capacity();
    std::mem::forget(raw_output);
    Ok(unsafe { Vec::from_raw_parts(ptr, len, capacity) })
}

#[inline(always)]
fn rate_change_scalar(input: &[f64], period: usize, mode: u8, output: &mut [f64]) {
    match mode {
        0 => {
            for i in period..input.len() {
                let previous = input[i - period];
                output[i] = if previous.abs() > 1e-15 {
                    (input[i] - previous) / previous
                } else {
                    f64::NAN
                };
            }
        }
        1 => {
            for i in period..input.len() {
                let previous = input[i - period];
                output[i] = if previous.abs() > 1e-15 {
                    input[i] / previous
                } else {
                    f64::NAN
                };
            }
        }
        _ => {
            for i in period..input.len() {
                let previous = input[i - period];
                output[i] = if previous.abs() > 1e-15 {
                    input[i] / previous * 100.0
                } else {
                    f64::NAN
                };
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn rate_change_avx2(input: &[f64], period: usize, mode: u8, output: &mut [f64]) {
    use std::arch::x86_64::*;

    let len = input.len();
    let body = len - period;
    let chunks = body / 4;
    let eps = _mm256_set1_pd(1e-15);
    let nan = _mm256_set1_pd(f64::NAN);
    let scale = _mm256_set1_pd(if mode == 2 { 100.0 } else { 1.0 });
    let input_ptr = input.as_ptr();
    let output_ptr = output.as_mut_ptr();

    for chunk in 0..chunks {
        let offset = period + chunk * 4;
        let current = unsafe { _mm256_loadu_pd(input_ptr.add(offset)) };
        let previous = unsafe { _mm256_loadu_pd(input_ptr.add(offset - period)) };
        let abs_previous = _mm256_andnot_pd(_mm256_set1_pd(-0.0), previous);
        let valid = _mm256_cmp_pd(abs_previous, eps, _CMP_GT_OS);
        let ratio = if mode == 0 {
            _mm256_div_pd(_mm256_sub_pd(current, previous), previous)
        } else {
            _mm256_div_pd(current, previous)
        };
        let scaled = _mm256_mul_pd(ratio, scale);
        unsafe {
            _mm256_storeu_pd(output_ptr.add(offset), _mm256_blendv_pd(nan, scaled, valid));
        }
    }

    let start = period + chunks * 4;
    for i in start..len {
        let previous = input[i - period];
        output[i] = if previous.abs() > 1e-15 {
            if mode == 0 {
                (input[i] - previous) / previous
            } else if mode == 1 {
                input[i] / previous
            } else {
                input[i] / previous * 100.0
            }
        } else {
            f64::NAN
        };
    }
}

#[pyfunction(name = "_fast_rocp")]
#[pyo3(signature = (close, timeperiod=10))]
fn fast_rocp<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let close = close.as_slice().map_err(value_error)?;
    let values = py.detach(|| rate_change_vec(close, timeperiod, 0))?;
    Ok(PyArray1::from_vec(py, values))
}

#[pyfunction(name = "_fast_rocr")]
#[pyo3(signature = (close, timeperiod=10))]
fn fast_rocr<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let close = close.as_slice().map_err(value_error)?;
    let values = py.detach(|| rate_change_vec(close, timeperiod, 1))?;
    Ok(PyArray1::from_vec(py, values))
}

#[pyfunction(name = "_fast_rocr100")]
#[pyo3(signature = (close, timeperiod=10))]
fn fast_rocr100<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let close = close.as_slice().map_err(value_error)?;
    let values = py.detach(|| rate_change_vec(close, timeperiod, 2))?;
    Ok(PyArray1::from_vec(py, values))
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

#[inline]
fn mom_vec(input: &[f64], period: usize) -> Vec<f64> {
    // Use the canonical SIMD kernel while avoiding a redundant zero-fill:
    // the kernel overwrites both the warm-up prefix and every valid sample.
    let len = input.len();
    let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe { raw_output.set_len(len) };
    let output = unsafe { std::slice::from_raw_parts_mut(raw_output.as_mut_ptr().cast(), len) };
    ::finkit::math::simd_ops::simd_mom(input, period, output);
    let ptr = raw_output.as_mut_ptr().cast::<f64>();
    let capacity = raw_output.capacity();
    std::mem::forget(raw_output);
    unsafe { Vec::from_raw_parts(ptr, len, capacity) }
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
    py.detach(|| moving_avg::ema_fast_into(close, timeperiod, &mut output))
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
    py.detach(|| moving_avg::ema_fast_into(close, timeperiod, output))
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
    let output = py
        .detach(|| volume_kernels::obv_vec(close, volume))
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
    Ok(PyArray1::from_vec(
        py,
        py.detach(|| mom_vec(close, timeperiod)),
    ))
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
            let mut output = vec![0.0; close.len()];
            py.detach(|| indicators::midpoint_into(close, timeperiod, &mut output))
                .map_err(value_error)?;
            output
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
        "cmo" => {
            let mut output = vec![0.0; close.len()];
            py.detach(|| indicators::cmo_fast_into(close, timeperiod, &mut output))
                .map_err(value_error)?;
            output
        }
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
            let mut output = vec![0.0; input_a.len()];
            py.detach(|| indicators::midprice_into(input_a, input_b, timeperiod, &mut output))
                .map_err(value_error)?;
            output
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
            let mut output = vec![0.0; high.len()];
            py.detach(|| indicators::willr_into(high, low, close, timeperiod, &mut output))
                .map_err(value_error)?;
            output
        }
        "adx" => py
            .detach(|| {
                let len = high.len();
                let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(len);
                unsafe { raw_output.set_len(len) };
                let output = unsafe {
                    std::slice::from_raw_parts_mut(raw_output.as_mut_ptr().cast::<f64>(), len)
                };
                indicators::adx_into(high, low, close, timeperiod, output).map_err(value_error)?;
                let ptr = raw_output.as_mut_ptr().cast::<f64>();
                let capacity = raw_output.capacity();
                std::mem::forget(raw_output);
                Ok::<Vec<f64>, crate::PyErr>(unsafe { Vec::from_raw_parts(ptr, len, capacity) })
            })
            .map_err(value_error)?,
        "cci" => {
            let len = high.len();
            let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(len);
            unsafe { raw_output.set_len(len) };
            let output = unsafe {
                std::slice::from_raw_parts_mut(raw_output.as_mut_ptr().cast::<f64>(), len)
            };
            py.detach(|| indicators::cci_into(high, low, close, timeperiod, output))
                .map_err(value_error)?;
            let ptr = raw_output.as_mut_ptr().cast::<f64>();
            let capacity = raw_output.capacity();
            std::mem::forget(raw_output);
            unsafe { Vec::from_raw_parts(ptr, len, capacity) }
        }
        "plus_di" => py
            .detach(|| {
                let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(high.len());
                unsafe { raw_output.set_len(high.len()) };
                let output = unsafe {
                    std::slice::from_raw_parts_mut(
                        raw_output.as_mut_ptr().cast::<f64>(),
                        high.len(),
                    )
                };
                indicators::plus_di_fast_into(high, low, close, timeperiod, output)
                    .map_err(value_error)?;
                let ptr = raw_output.as_mut_ptr().cast::<f64>();
                let capacity = raw_output.capacity();
                std::mem::forget(raw_output);
                Ok::<Vec<f64>, crate::PyErr>(unsafe {
                    Vec::from_raw_parts(ptr, high.len(), capacity)
                })
            })
            .map_err(value_error)?,
        "minus_di" => py
            .detach(|| {
                let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(high.len());
                unsafe { raw_output.set_len(high.len()) };
                let output = unsafe {
                    std::slice::from_raw_parts_mut(
                        raw_output.as_mut_ptr().cast::<f64>(),
                        high.len(),
                    )
                };
                indicators::minus_di_fast_into(high, low, close, timeperiod, output)
                    .map_err(value_error)?;
                let ptr = raw_output.as_mut_ptr().cast::<f64>();
                let capacity = raw_output.capacity();
                std::mem::forget(raw_output);
                Ok::<Vec<f64>, crate::PyErr>(unsafe {
                    Vec::from_raw_parts(ptr, high.len(), capacity)
                })
            })
            .map_err(value_error)?,
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
    let len = high.len();
    let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe { raw_output.set_len(len) };
    let output =
        unsafe { std::slice::from_raw_parts_mut(raw_output.as_mut_ptr().cast::<f64>(), len) };
    py.detach(|| indicators::trange_into(high, low, close, output))
        .map_err(value_error)?;
    let ptr = raw_output.as_mut_ptr().cast::<f64>();
    let capacity = raw_output.capacity();
    std::mem::forget(raw_output);
    Ok(PyArray1::from_vec(py, unsafe {
        Vec::from_raw_parts(ptr, len, capacity)
    }))
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
    let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(close.len());
    unsafe { raw_output.set_len(close.len()) };
    let output = unsafe {
        std::slice::from_raw_parts_mut(raw_output.as_mut_ptr().cast::<f64>(), close.len())
    };
    py.detach(|| indicators::mfi_into(high, low, close, volume, timeperiod, output))
        .map_err(value_error)?;
    let ptr = raw_output.as_mut_ptr().cast::<f64>();
    let len = raw_output.len();
    let capacity = raw_output.capacity();
    forget(raw_output);
    let output = unsafe { Vec::from_raw_parts(ptr, len, capacity) };
    Ok(PyArray1::from_vec(py, output))
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
    let len = high.len();
    let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe { raw_output.set_len(len) };
    let output =
        unsafe { std::slice::from_raw_parts_mut(raw_output.as_mut_ptr().cast::<f64>(), len) };
    py.detach(|| volume_kernels::ad_into(high, low, close, volume, output))
        .map_err(value_error)?;
    let ptr = raw_output.as_mut_ptr().cast::<f64>();
    let capacity = raw_output.capacity();
    std::mem::forget(raw_output);
    Ok(PyArray1::from_vec(py, unsafe {
        Vec::from_raw_parts(ptr, len, capacity)
    }))
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
    let len = high.len();
    let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe { raw_output.set_len(len) };
    let output =
        unsafe { std::slice::from_raw_parts_mut(raw_output.as_mut_ptr().cast::<f64>(), len) };
    py.detach(|| {
        volume_kernels::adosc_into(high, low, close, volume, fastperiod, slowperiod, output)
    })
    .map_err(value_error)?;
    let ptr = raw_output.as_mut_ptr().cast::<f64>();
    let capacity = raw_output.capacity();
    std::mem::forget(raw_output);
    Ok(PyArray1::from_vec(py, unsafe {
        Vec::from_raw_parts(ptr, len, capacity)
    }))
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

#[pyfunction(name = "_fast_price_transform")]
fn fast_price_transform<'py>(
    py: Python<'py>,
    operation: &str,
    open: Option<PyReadonlyArray1<'py, f64>>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: Option<PyReadonlyArray1<'py, f64>>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let high = high.as_slice().map_err(value_error)?;
    let low = low.as_slice().map_err(value_error)?;
    validate_same_len(high.len(), low.len())?;
    let open = open
        .as_ref()
        .map(|values| values.as_slice().map_err(value_error))
        .transpose()?;
    let close = close
        .as_ref()
        .map(|values| values.as_slice().map_err(value_error))
        .transpose()?;
    if let Some(values) = open {
        validate_same_len(high.len(), values.len())?;
    }
    if let Some(values) = close {
        validate_same_len(high.len(), values.len())?;
    }

    let len = high.len();
    let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe { raw_output.set_len(len) };
    let output =
        unsafe { std::slice::from_raw_parts_mut(raw_output.as_mut_ptr().cast::<f64>(), len) };
    let result = match operation {
        "avgprice" => {
            let open = open.ok_or_else(|| value_error("AVGPRICE requires open data"))?;
            let close = close.ok_or_else(|| value_error("AVGPRICE requires close data"))?;
            py.detach(|| indicators::avgprice_into(open, high, low, close, output))
        }
        "medprice" => py.detach(|| indicators::medprice_into(high, low, output)),
        "typprice" => {
            let close = close.ok_or_else(|| value_error("TYPPRICE requires close data"))?;
            py.detach(|| indicators::typprice_into(high, low, close, output))
        }
        "wclprice" => {
            let close = close.ok_or_else(|| value_error("WCLPRICE requires close data"))?;
            py.detach(|| indicators::wclprice_into(high, low, close, output))
        }
        _ => {
            return Err(value_error(format!(
                "invalid parameter: unsupported price transform {operation}"
            )))
        }
    };
    result.map_err(value_error)?;
    let ptr = raw_output.as_mut_ptr().cast::<f64>();
    let capacity = raw_output.capacity();
    std::mem::forget(raw_output);
    Ok(PyArray1::from_vec(py, unsafe {
        Vec::from_raw_parts(ptr, len, capacity)
    }))
}

#[pyfunction(name = "_fast_apo")]
#[pyo3(signature = (close, fastperiod=12, slowperiod=26))]
fn fast_apo<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    fastperiod: usize,
    slowperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let close = close.as_slice().map_err(value_error)?;
    let output = py
        .detach(|| indicators::apo(close, fastperiod, slowperiod))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output.into_raw_vec()))
}

#[pyfunction(name = "_fast_dx")]
#[pyo3(signature = (high, low, close, timeperiod=14))]
fn fast_dx<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let high = high.as_slice().map_err(value_error)?;
    let low = low.as_slice().map_err(value_error)?;
    let close = close.as_slice().map_err(value_error)?;
    let mut output = vec![0.0; close.len()];
    py.detach(|| indicators::dx_into(high, low, close, timeperiod, &mut output))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output))
}

#[pyfunction(name = "_fast_aroon")]
#[pyo3(signature = (high, low, timeperiod=14))]
fn fast_aroon<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let high = high.as_slice().map_err(value_error)?;
    let low = low.as_slice().map_err(value_error)?;
    let mut aroon_up = vec![0.0; high.len()];
    let mut aroon_down = vec![0.0; high.len()];
    py.detach(|| indicators::aroon_into(high, low, timeperiod, &mut aroon_up, &mut aroon_down))
        .map_err(value_error)?;
    Ok((
        PyArray1::from_vec(py, aroon_up),
        PyArray1::from_vec(py, aroon_down),
    ))
}

#[pyfunction(name = "_fast_trix")]
#[pyo3(signature = (close, timeperiod=14))]
fn fast_trix<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let close = close.as_slice().map_err(value_error)?;
    let output = py
        .detach(|| indicators::trix(close, timeperiod))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output.into_raw_vec()))
}

#[pyfunction(name = "_fast_t3")]
#[pyo3(signature = (close, timeperiod=5, vfactor=0.7))]
fn fast_t3<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
    vfactor: f64,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let close = close.as_slice().map_err(value_error)?;
    let len = close.len();
    let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe { raw_output.set_len(len) };
    let output =
        unsafe { std::slice::from_raw_parts_mut(raw_output.as_mut_ptr().cast::<f64>(), len) };
    py.detach(|| indicators::t3_into(close, timeperiod, vfactor, output))
        .map_err(value_error)?;
    let ptr = raw_output.as_mut_ptr().cast::<f64>();
    let capacity = raw_output.capacity();
    forget(raw_output);
    Ok(PyArray1::from_vec(py, unsafe {
        Vec::from_raw_parts(ptr, len, capacity)
    }))
}

#[pyfunction(name = "_fast_tsf")]
#[pyo3(signature = (close, timeperiod=14))]
fn fast_tsf<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let close = close.as_slice().map_err(value_error)?;
    let output = py
        .detach(|| indicators::tsf(close, timeperiod))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output.into_raw_vec()))
}

#[pyfunction(name = "_fast_beta")]
#[pyo3(signature = (asset, benchmark, timeperiod=5))]
fn fast_beta<'py>(
    py: Python<'py>,
    asset: PyReadonlyArray1<'py, f64>,
    benchmark: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let asset = asset.as_slice().map_err(value_error)?;
    let benchmark = benchmark.as_slice().map_err(value_error)?;
    validate_same_len(asset.len(), benchmark.len())?;
    let output = py
        .detach(|| indicators::beta(asset, benchmark, timeperiod))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output.into_raw_vec()))
}

#[pyfunction(name = "_fast_mama")]
#[pyo3(signature = (close, fastlimit=0.5, slowlimit=0.05))]
fn fast_mama<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
    fastlimit: f64,
    slowlimit: f64,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let close = close.as_slice().map_err(value_error)?;
    // `mama_into` initializes every output slot, including the warm-up NaNs.
    // Leave storage uninitialized so the binding does not clear both buffers
    // before the core kernel fills them.
    let len = close.len();
    let mut mama_raw = Vec::<MaybeUninit<f64>>::with_capacity(len);
    let mut fama_raw = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe {
        mama_raw.set_len(len);
        fama_raw.set_len(len);
    }
    let mama = unsafe { std::slice::from_raw_parts_mut(mama_raw.as_mut_ptr().cast::<f64>(), len) };
    let fama = unsafe { std::slice::from_raw_parts_mut(fama_raw.as_mut_ptr().cast::<f64>(), len) };
    py.detach(|| indicators::mama_into(close, fastlimit, slowlimit, mama, fama))
        .map_err(value_error)?;
    let mama_ptr = mama_raw.as_mut_ptr().cast::<f64>();
    let fama_ptr = fama_raw.as_mut_ptr().cast::<f64>();
    let mama_capacity = mama_raw.capacity();
    let fama_capacity = fama_raw.capacity();
    forget(mama_raw);
    forget(fama_raw);
    Ok((
        PyArray1::from_vec(py, unsafe {
            Vec::from_raw_parts(mama_ptr, len, mama_capacity)
        }),
        PyArray1::from_vec(py, unsafe {
            Vec::from_raw_parts(fama_ptr, len, fama_capacity)
        }),
    ))
}

macro_rules! fast_ht_unary {
    ($name:ident, $py_name:literal, $indicator:ident) => {
        #[pyfunction(name = $py_name)]
        fn $name<'py>(
            py: Python<'py>,
            close: PyReadonlyArray1<'py, f64>,
        ) -> PyResult<Bound<'py, PyArray1<f64>>> {
            let close = close.as_slice().map_err(value_error)?;
            let len = close.len();
            let mut raw_output = Vec::<MaybeUninit<f64>>::with_capacity(len);
            unsafe { raw_output.set_len(len) };
            let output = unsafe {
                std::slice::from_raw_parts_mut(raw_output.as_mut_ptr().cast::<f64>(), len)
            };
            py.detach(|| indicators::$indicator(close, output))
                .map_err(value_error)?;
            let ptr = raw_output.as_mut_ptr().cast::<f64>();
            let capacity = raw_output.capacity();
            forget(raw_output);
            Ok(PyArray1::from_vec(py, unsafe {
                Vec::from_raw_parts(ptr, len, capacity)
            }))
        }
    };
}

fast_ht_unary!(fast_ht_dcperiod, "_fast_ht_dcperiod", ht_dcperiod_into);
fast_ht_unary!(fast_ht_dcphase, "_fast_ht_dcphase", ht_dcphase_into);
fast_ht_unary!(fast_ht_trendline, "_fast_ht_trendline", ht_trendline_into);
fast_ht_unary!(fast_ht_trendmode, "_fast_ht_trendmode", ht_trendmode_into);

#[pyfunction(name = "_fast_ht_phasor")]
fn fast_ht_phasor<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let close = close.as_slice().map_err(value_error)?;
    let len = close.len();
    let mut in_phase_raw = Vec::<MaybeUninit<f64>>::with_capacity(len);
    let mut quadrature_raw = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe {
        in_phase_raw.set_len(len);
        quadrature_raw.set_len(len);
    }
    let in_phase =
        unsafe { std::slice::from_raw_parts_mut(in_phase_raw.as_mut_ptr().cast::<f64>(), len) };
    let quadrature =
        unsafe { std::slice::from_raw_parts_mut(quadrature_raw.as_mut_ptr().cast::<f64>(), len) };
    py.detach(|| indicators::ht_phasor_into(close, in_phase, quadrature))
        .map_err(value_error)?;
    let in_phase_ptr = in_phase_raw.as_mut_ptr().cast::<f64>();
    let quadrature_ptr = quadrature_raw.as_mut_ptr().cast::<f64>();
    let in_phase_capacity = in_phase_raw.capacity();
    let quadrature_capacity = quadrature_raw.capacity();
    forget(in_phase_raw);
    forget(quadrature_raw);
    Ok((
        PyArray1::from_vec(py, unsafe {
            Vec::from_raw_parts(in_phase_ptr, len, in_phase_capacity)
        }),
        PyArray1::from_vec(py, unsafe {
            Vec::from_raw_parts(quadrature_ptr, len, quadrature_capacity)
        }),
    ))
}

#[pyfunction(name = "_fast_ht_sine")]
fn fast_ht_sine<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'py, f64>,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let close = close.as_slice().map_err(value_error)?;
    let len = close.len();
    let mut sine_raw = Vec::<MaybeUninit<f64>>::with_capacity(len);
    let mut lead_sine_raw = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe {
        sine_raw.set_len(len);
        lead_sine_raw.set_len(len);
    }
    let sine = unsafe { std::slice::from_raw_parts_mut(sine_raw.as_mut_ptr().cast::<f64>(), len) };
    let lead_sine =
        unsafe { std::slice::from_raw_parts_mut(lead_sine_raw.as_mut_ptr().cast::<f64>(), len) };
    py.detach(|| indicators::ht_sine_into(close, sine, lead_sine))
        .map_err(value_error)?;
    let sine_ptr = sine_raw.as_mut_ptr().cast::<f64>();
    let lead_sine_ptr = lead_sine_raw.as_mut_ptr().cast::<f64>();
    let sine_capacity = sine_raw.capacity();
    let lead_sine_capacity = lead_sine_raw.capacity();
    forget(sine_raw);
    forget(lead_sine_raw);
    Ok((
        PyArray1::from_vec(py, unsafe {
            Vec::from_raw_parts(sine_ptr, len, sine_capacity)
        }),
        PyArray1::from_vec(py, unsafe {
            Vec::from_raw_parts(lead_sine_ptr, len, lead_sine_capacity)
        }),
    ))
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
    let mut macd = vec![0.0; close.len()];
    let mut signal = vec![0.0; close.len()];
    let mut hist = vec![0.0; close.len()];
    py.detach(|| {
        indicators::macd_fast_into(
            close,
            fastperiod,
            slowperiod,
            signalperiod,
            &mut macd,
            &mut signal,
            &mut hist,
        )
    })
    .map_err(value_error)?;
    Ok((
        PyArray1::from_vec(py, macd),
        PyArray1::from_vec(py, signal),
        PyArray1::from_vec(py, hist),
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
    let len = close.len();
    // The STOCH kernels write both output slices completely (including the
    // warm-up prefix), so avoid clearing two full buffers before dispatch.
    let mut k_raw = Vec::<MaybeUninit<f64>>::with_capacity(len);
    let mut d_raw = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe {
        k_raw.set_len(len);
        d_raw.set_len(len);
    }
    let k_out = unsafe { std::slice::from_raw_parts_mut(k_raw.as_mut_ptr().cast::<f64>(), len) };
    let d_out = unsafe { std::slice::from_raw_parts_mut(d_raw.as_mut_ptr().cast::<f64>(), len) };
    py.detach(|| {
        indicators::stoch_into(
            high,
            low,
            close,
            fastk_period,
            slowk_period,
            slowd_period,
            k_out,
            d_out,
        )
    })
    .map_err(value_error)?;
    let k_ptr = k_raw.as_mut_ptr().cast::<f64>();
    let d_ptr = d_raw.as_mut_ptr().cast::<f64>();
    let k_capacity = k_raw.capacity();
    let d_capacity = d_raw.capacity();
    std::mem::forget(k_raw);
    std::mem::forget(d_raw);
    let k_vec = unsafe { Vec::from_raw_parts(k_ptr, len, k_capacity) };
    let d_vec = unsafe { Vec::from_raw_parts(d_ptr, len, d_capacity) };
    Ok((PyArray1::from_vec(py, k_vec), PyArray1::from_vec(py, d_vec)))
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
    m.add_function(wrap_pyfunction!(fast_rocp, m)?)?;
    m.add_function(wrap_pyfunction!(fast_rocr, m)?)?;
    m.add_function(wrap_pyfunction!(fast_rocr100, m)?)?;
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
    m.add_function(wrap_pyfunction!(fast_price_transform, m)?)?;
    m.add_function(wrap_pyfunction!(fast_apo, m)?)?;
    m.add_function(wrap_pyfunction!(fast_dx, m)?)?;
    m.add_function(wrap_pyfunction!(fast_aroon, m)?)?;
    m.add_function(wrap_pyfunction!(fast_trix, m)?)?;
    m.add_function(wrap_pyfunction!(fast_t3, m)?)?;
    m.add_function(wrap_pyfunction!(fast_tsf, m)?)?;
    m.add_function(wrap_pyfunction!(fast_beta, m)?)?;
    m.add_function(wrap_pyfunction!(fast_mama, m)?)?;
    m.add_function(wrap_pyfunction!(fast_ht_dcperiod, m)?)?;
    m.add_function(wrap_pyfunction!(fast_ht_dcphase, m)?)?;
    m.add_function(wrap_pyfunction!(fast_ht_phasor, m)?)?;
    m.add_function(wrap_pyfunction!(fast_ht_sine, m)?)?;
    m.add_function(wrap_pyfunction!(fast_ht_trendline, m)?)?;
    m.add_function(wrap_pyfunction!(fast_ht_trendmode, m)?)?;
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
