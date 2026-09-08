//! Direct TA-Lib-compatible bindings for indicators that were previously only
//! reachable through the batch dispatcher.  Keeping these calls direct avoids
//! dictionary construction and lets the Rust kernels remain the single source
//! of numerical semantics for both the batch and Python APIs.

use ::finkit::indicators;
use ::finkit::math::moving_avg;
use ::finkit::patterns::candlestick;
use numpy::{PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;
use std::mem::MaybeUninit;

#[inline]
fn value_error(error: impl std::fmt::Display) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(error.to_string())
}

macro_rules! unary_period {
    ($rust_name:ident, $py_name:literal, $kernel:path, $default:literal) => {
        #[pyfunction(name = $py_name)]
        #[pyo3(signature = (real, timeperiod=$default))]
        fn $rust_name<'py>(
            py: Python<'py>,
            real: PyReadonlyArray1<'py, f64>,
            timeperiod: usize,
        ) -> PyResult<Bound<'py, PyArray1<f64>>> {
            let real = real.as_slice().map_err(value_error)?;
            let values = py
                .detach(|| $kernel(real, timeperiod))
                .map_err(value_error)?
                .into_raw_vec();
            Ok(PyArray1::from_vec(py, values))
        }
    };
}

macro_rules! unary_transform {
    ($rust_name:ident, $py_name:literal, $kernel:path) => {
        #[pyfunction(name = $py_name)]
        fn $rust_name<'py>(
            py: Python<'py>,
            real: PyReadonlyArray1<'py, f64>,
        ) -> PyResult<Bound<'py, PyArray1<f64>>> {
            let real = real.as_slice().map_err(value_error)?;
            let values = py
                .detach(|| $kernel(real))
                .map_err(value_error)?
                .into_raw_vec();
            Ok(PyArray1::from_vec(py, values))
        }
    };
}

macro_rules! binary_transform {
    ($rust_name:ident, $py_name:literal, $kernel:path) => {
        #[pyfunction(name = $py_name)]
        fn $rust_name<'py>(
            py: Python<'py>,
            real0: PyReadonlyArray1<'py, f64>,
            real1: PyReadonlyArray1<'py, f64>,
        ) -> PyResult<Bound<'py, PyArray1<f64>>> {
            let real0 = real0.as_slice().map_err(value_error)?;
            let real1 = real1.as_slice().map_err(value_error)?;
            let values = py
                .detach(|| $kernel(real0, real1))
                .map_err(value_error)?
                .into_raw_vec();
            Ok(PyArray1::from_vec(py, values))
        }
    };
}

macro_rules! candle {
    ($rust_name:ident, $py_name:literal, $kernel:ident) => {
        #[pyfunction(name = $py_name)]
        fn $rust_name<'py>(
            py: Python<'py>,
            open: PyReadonlyArray1<'py, f64>,
            high: PyReadonlyArray1<'py, f64>,
            low: PyReadonlyArray1<'py, f64>,
            close: PyReadonlyArray1<'py, f64>,
        ) -> PyResult<Bound<'py, PyArray1<i32>>> {
            let open = open.as_slice().map_err(value_error)?;
            let high = high.as_slice().map_err(value_error)?;
            let low = low.as_slice().map_err(value_error)?;
            let close = close.as_slice().map_err(value_error)?;
            let values = py
                .detach(|| candlestick::$kernel(open, high, low, close))
                .map_err(value_error)?
                .into_raw_vec();
            Ok(PyArray1::from_vec(py, values))
        }
    };
}

macro_rules! index_period {
    ($rust_name:ident, $py_name:literal, $kernel:path) => {
        #[pyfunction(name = $py_name)]
        #[pyo3(signature = (real, timeperiod=30))]
        fn $rust_name<'py>(
            py: Python<'py>,
            real: PyReadonlyArray1<'py, f64>,
            timeperiod: usize,
        ) -> PyResult<Bound<'py, PyArray1<i64>>> {
            let real = real.as_slice().map_err(value_error)?;
            let values = py
                .detach(|| $kernel(real, timeperiod))
                .map_err(value_error)?
                .into_raw_vec();
            // TA-Lib returns the absolute input index, while the core Rust
            // math API intentionally exposes the cheaper window-relative
            // offset. Keep both contracts explicit at the FFI boundary.
            let mut values = values;
            let first = timeperiod.saturating_sub(1);
            for value in values.iter_mut().take(first) {
                *value = 0;
            }
            for (index, value) in values.iter_mut().enumerate().skip(first) {
                if *value >= 0 {
                    *value += (index + 1 - timeperiod) as i64;
                }
            }
            Ok(PyArray1::from_vec(py, values))
        }
    };
}

macro_rules! hlc_period {
    ($rust_name:ident, $py_name:literal, $kernel:path, $default:literal) => {
        #[pyfunction(name = $py_name)]
        #[pyo3(signature = (high, low, close, timeperiod=$default))]
        fn $rust_name<'py>(
            py: Python<'py>,
            high: PyReadonlyArray1<'py, f64>,
            low: PyReadonlyArray1<'py, f64>,
            close: PyReadonlyArray1<'py, f64>,
            timeperiod: usize,
        ) -> PyResult<Bound<'py, PyArray1<f64>>> {
            let high = high.as_slice().map_err(value_error)?;
            let low = low.as_slice().map_err(value_error)?;
            let close = close.as_slice().map_err(value_error)?;
            let values = py
                .detach(|| $kernel(high, low, close, timeperiod))
                .map_err(value_error)?
                .into_raw_vec();
            Ok(PyArray1::from_vec(py, values))
        }
    };
}

macro_rules! hl_period {
    ($rust_name:ident, $py_name:literal, $kernel:path, $default:literal) => {
        #[pyfunction(name = $py_name)]
        #[pyo3(signature = (high, low, timeperiod=$default))]
        fn $rust_name<'py>(
            py: Python<'py>,
            high: PyReadonlyArray1<'py, f64>,
            low: PyReadonlyArray1<'py, f64>,
            timeperiod: usize,
        ) -> PyResult<Bound<'py, PyArray1<f64>>> {
            let high = high.as_slice().map_err(value_error)?;
            let low = low.as_slice().map_err(value_error)?;
            let values = py
                .detach(|| $kernel(high, low, timeperiod))
                .map_err(value_error)?
                .into_raw_vec();
            Ok(PyArray1::from_vec(py, values))
        }
    };
}

unary_period!(trima, "trima", moving_avg::trima, 30);
unary_period!(rocp, "rocp", indicators::rocp, 10);
unary_period!(rocr, "rocr", indicators::rocr, 10);
unary_period!(rocr100, "rocr100", indicators::rocr100, 10);
unary_period!(linearreg, "linearreg", indicators::linearreg, 14);
unary_period!(
    linearreg_angle,
    "linearreg_angle",
    indicators::linearreg_angle,
    14
);
unary_period!(
    linearreg_intercept,
    "linearreg_intercept",
    indicators::linearreg_intercept,
    14
);
unary_period!(
    linearreg_slope,
    "linearreg_slope",
    indicators::linearreg_slope,
    14
);
unary_period!(max_value, "max", indicators::max, 30);
unary_period!(min_value, "min", indicators::min, 30);
unary_period!(sum_value, "sum", indicators::sum, 30);

unary_transform!(acos, "acos", indicators::acos);
unary_transform!(asin, "asin", indicators::asin);
unary_transform!(atan, "atan", indicators::atan);
unary_transform!(ceil, "ceil", indicators::ceil);
unary_transform!(cos, "cos", indicators::cos);
unary_transform!(cosh, "cosh", indicators::cosh);
unary_transform!(exp, "exp", indicators::exp);
unary_transform!(floor, "floor", indicators::floor);
unary_transform!(ln, "ln", indicators::ln);
unary_transform!(log10, "log10", indicators::log10);
unary_transform!(sin, "sin", indicators::sin);
unary_transform!(sinh, "sinh", indicators::sinh);
unary_transform!(sqrt, "sqrt", indicators::sqrt);
unary_transform!(tan, "tan", indicators::tan);
unary_transform!(tanh, "tanh", indicators::tanh);

binary_transform!(add, "add", indicators::add);
binary_transform!(div, "div", indicators::div);
binary_transform!(mult, "mult", indicators::mult);
binary_transform!(sub, "sub", indicators::sub);

hlc_period!(adxr, "adxr", indicators::adxr, 14);
hl_period!(aroonosc, "aroonosc", indicators::aroonosc, 14);

#[pyfunction(name = "bbands")]
#[pyo3(signature = (real, timeperiod=5, nbdevup=2.0, nbdevdn=2.0, matype=0))]
fn bbands<'py>(
    py: Python<'py>,
    real: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
    nbdevup: f64,
    nbdevdn: f64,
    matype: i32,
) -> PyResult<(
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
)> {
    let real = real.as_slice().map_err(value_error)?;
    let _ = matype;
    let len = real.len();
    let mut raw_upper = Vec::<MaybeUninit<f64>>::with_capacity(len);
    let mut raw_middle = Vec::<MaybeUninit<f64>>::with_capacity(len);
    let mut raw_lower = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe {
        raw_upper.set_len(len);
        raw_middle.set_len(len);
        raw_lower.set_len(len);
    }
    let upper =
        unsafe { std::slice::from_raw_parts_mut(raw_upper.as_mut_ptr().cast::<f64>(), len) };
    let middle =
        unsafe { std::slice::from_raw_parts_mut(raw_middle.as_mut_ptr().cast::<f64>(), len) };
    let lower =
        unsafe { std::slice::from_raw_parts_mut(raw_lower.as_mut_ptr().cast::<f64>(), len) };
    py.detach(|| indicators::bbands_into(real, timeperiod, nbdevup, nbdevdn, middle, upper, lower))
        .map_err(value_error)?;
    let upper_ptr = raw_upper.as_mut_ptr().cast::<f64>();
    let middle_ptr = raw_middle.as_mut_ptr().cast::<f64>();
    let lower_ptr = raw_lower.as_mut_ptr().cast::<f64>();
    let upper_cap = raw_upper.capacity();
    let middle_cap = raw_middle.capacity();
    let lower_cap = raw_lower.capacity();
    std::mem::forget(raw_upper);
    std::mem::forget(raw_middle);
    std::mem::forget(raw_lower);
    Ok((
        PyArray1::from_vec(py, unsafe {
            Vec::from_raw_parts(upper_ptr, len, upper_cap)
        }),
        PyArray1::from_vec(py, unsafe {
            Vec::from_raw_parts(middle_ptr, len, middle_cap)
        }),
        PyArray1::from_vec(py, unsafe {
            Vec::from_raw_parts(lower_ptr, len, lower_cap)
        }),
    ))
}

#[pyfunction(name = "ma")]
#[pyo3(signature = (real, timeperiod=30, matype=0))]
fn ma<'py>(
    py: Python<'py>,
    real: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
    matype: i32,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let real = real.as_slice().map_err(value_error)?;
    let ma_type = match matype {
        0 => indicators::MaType::Sma,
        1 => indicators::MaType::Ema,
        2 => indicators::MaType::Wma,
        3 => indicators::MaType::Dema,
        4 => indicators::MaType::Tema,
        5 => indicators::MaType::Trima,
        6 => indicators::MaType::Kama,
        8 => indicators::MaType::T3,
        _ => indicators::MaType::Sma,
    };
    let values = py
        .detach(|| indicators::ma(real, timeperiod, ma_type))
        .map_err(value_error)?
        .into_raw_vec();
    Ok(PyArray1::from_vec(py, values))
}

#[pyfunction(name = "ppo")]
#[pyo3(signature = (real, fastperiod=12, slowperiod=26, matype=0))]
fn ppo_compat<'py>(
    py: Python<'py>,
    real: PyReadonlyArray1<'py, f64>,
    fastperiod: usize,
    slowperiod: usize,
    matype: i32,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let real = real.as_slice().map_err(value_error)?;
    let values = py
        .detach(|| {
            if matype == 0 {
                if fastperiod == 0
                    || slowperiod == 0
                    || real.len() < fastperiod
                    || real.len() < slowperiod
                {
                    return indicators::ppo(real, fastperiod, slowperiod);
                }
                let mut output = vec![f64::NAN; real.len()];
                let mut fast_sum = real[..fastperiod].iter().sum::<f64>();
                let mut slow_sum = real[..slowperiod].iter().sum::<f64>();
                for i in 0..real.len() {
                    if i >= fastperiod {
                        fast_sum += real[i] - real[i - fastperiod];
                    }
                    if i >= slowperiod {
                        slow_sum += real[i] - real[i - slowperiod];
                    }
                    if i + 1 >= slowperiod {
                        let slow = slow_sum / slowperiod as f64;
                        output[i] = if slow.abs() > 1e-15 {
                            (fast_sum / fastperiod as f64 - slow) / slow * 100.0
                        } else {
                            0.0
                        };
                    }
                }
                return Ok::<_, ::finkit::error::TaError>(ndarray::Array1::from_vec(output));
            }
            let ma_type = match matype {
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
            let fast = indicators::ma(real, fastperiod, ma_type)?;
            let slow = indicators::ma(real, slowperiod, ma_type)?;
            let mut output = fast.clone();
            for i in 0..real.len() {
                if !fast[i].is_nan() && !slow[i].is_nan() {
                    output[i] = if slow[i].abs() > 1e-15 {
                        (fast[i] - slow[i]) / slow[i] * 100.0
                    } else {
                        0.0
                    };
                } else {
                    output[i] = f64::NAN;
                }
            }
            Ok::<_, ::finkit::error::TaError>(output)
        })
        .map_err(value_error)?
        .into_raw_vec();
    Ok(PyArray1::from_vec(py, values))
}

#[pyfunction(name = "macdfix")]
#[pyo3(signature = (real, signalperiod=9))]
fn macdfix<'py>(
    py: Python<'py>,
    real: PyReadonlyArray1<'py, f64>,
    signalperiod: usize,
) -> PyResult<(
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
)> {
    let real = real.as_slice().map_err(value_error)?;
    let len = real.len();
    let mut raw_macd = Vec::<MaybeUninit<f64>>::with_capacity(len);
    let mut raw_signal = Vec::<MaybeUninit<f64>>::with_capacity(len);
    let mut raw_hist = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe {
        raw_macd.set_len(len);
        raw_signal.set_len(len);
        raw_hist.set_len(len);
    }
    let macd = unsafe { std::slice::from_raw_parts_mut(raw_macd.as_mut_ptr().cast::<f64>(), len) };
    let signal =
        unsafe { std::slice::from_raw_parts_mut(raw_signal.as_mut_ptr().cast::<f64>(), len) };
    let hist = unsafe { std::slice::from_raw_parts_mut(raw_hist.as_mut_ptr().cast::<f64>(), len) };
    py.detach(|| indicators::macdfix_into(real, signalperiod, macd, signal, hist))
        .map_err(value_error)?;
    let macd_ptr = raw_macd.as_mut_ptr().cast::<f64>();
    let signal_ptr = raw_signal.as_mut_ptr().cast::<f64>();
    let hist_ptr = raw_hist.as_mut_ptr().cast::<f64>();
    let macd_cap = raw_macd.capacity();
    let signal_cap = raw_signal.capacity();
    let hist_cap = raw_hist.capacity();
    std::mem::forget(raw_macd);
    std::mem::forget(raw_signal);
    std::mem::forget(raw_hist);
    Ok((
        PyArray1::from_vec(py, unsafe { Vec::from_raw_parts(macd_ptr, len, macd_cap) }),
        PyArray1::from_vec(py, unsafe {
            Vec::from_raw_parts(signal_ptr, len, signal_cap)
        }),
        PyArray1::from_vec(py, unsafe { Vec::from_raw_parts(hist_ptr, len, hist_cap) }),
    ))
}

#[pyfunction(name = "macdext")]
#[pyo3(signature = (real, fastperiod=12, fastmatype=0, slowperiod=26, slowmatype=0, signalperiod=9, signalmatype=0))]
fn macdext<'py>(
    py: Python<'py>,
    real: PyReadonlyArray1<'py, f64>,
    fastperiod: usize,
    fastmatype: i32,
    slowperiod: usize,
    slowmatype: i32,
    signalperiod: usize,
    signalmatype: i32,
) -> PyResult<(
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
    Bound<'py, PyArray1<f64>>,
)> {
    let real = real.as_slice().map_err(value_error)?;
    let map_type = |value: i32| match value {
        0 => indicators::MaType::Sma,
        1 => indicators::MaType::Ema,
        2 => indicators::MaType::Wma,
        3 => indicators::MaType::Dema,
        4 => indicators::MaType::Tema,
        5 => indicators::MaType::Trima,
        6 => indicators::MaType::Kama,
        8 => indicators::MaType::T3,
        _ => indicators::MaType::Sma,
    };
    let result = py
        .detach(|| {
            indicators::macdext(
                real,
                fastperiod,
                map_type(fastmatype),
                slowperiod,
                map_type(slowmatype),
                signalperiod,
                map_type(signalmatype),
            )
        })
        .map_err(value_error)?;
    Ok((
        PyArray1::from_vec(py, result.macd.into_raw_vec()),
        PyArray1::from_vec(py, result.signal.into_raw_vec()),
        PyArray1::from_vec(py, result.hist.into_raw_vec()),
    ))
}

#[pyfunction(name = "mavp")]
#[pyo3(signature = (real, periods, minperiod=2, maxperiod=30, matype=0))]
fn mavp<'py>(
    py: Python<'py>,
    real: PyReadonlyArray1<'py, f64>,
    periods: PyReadonlyArray1<'py, f64>,
    minperiod: usize,
    maxperiod: usize,
    matype: i32,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let real = real.as_slice().map_err(value_error)?;
    let periods = periods.as_slice().map_err(value_error)?;
    let uniform_period = matype == 0 && periods.windows(2).all(|window| window[0] == window[1]);
    let values = py
        .detach(|| {
            if uniform_period {
                let period = (periods.first().copied().unwrap_or(minperiod as f64).round()
                    as usize)
                    .clamp(minperiod, maxperiod);
                moving_avg::sma(real, period)
            } else {
                moving_avg::mavp(real, periods, minperiod, maxperiod)
            }
        })
        .map_err(value_error)?
        .into_raw_vec();
    let mut values = values;
    for value in values.iter_mut().take(maxperiod.saturating_sub(1)) {
        *value = f64::NAN;
    }
    Ok(PyArray1::from_vec(py, values))
}

#[pyfunction(name = "stochf")]
#[pyo3(signature = (high, low, close, fastk_period=5, fastd_period=3, fastd_matype=0))]
fn stochf<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    fastk_period: usize,
    fastd_period: usize,
    fastd_matype: i32,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let high = high.as_slice().map_err(value_error)?;
    let _ = fastd_matype;
    let low = low.as_slice().map_err(value_error)?;
    let close = close.as_slice().map_err(value_error)?;
    let result = py
        .detach(|| indicators::stochf(high, low, close, fastk_period, fastd_period))
        .map_err(value_error)?;
    Ok((
        PyArray1::from_vec(py, result.k.into_raw_vec()),
        PyArray1::from_vec(py, result.d.into_raw_vec()),
    ))
}

#[pyfunction(name = "stochrsi")]
#[pyo3(signature = (real, timeperiod=14, fastk_period=5, fastd_period=3, fastd_matype=0))]
fn stochrsi<'py>(
    py: Python<'py>,
    real: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
    fastk_period: usize,
    fastd_period: usize,
    fastd_matype: i32,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let real = real.as_slice().map_err(value_error)?;
    let _ = fastd_matype;
    let len = real.len();
    let mut raw_k = Vec::<MaybeUninit<f64>>::with_capacity(len);
    let mut raw_d = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe {
        raw_k.set_len(len);
        raw_d.set_len(len);
    }
    let mut out_k =
        unsafe { std::slice::from_raw_parts_mut(raw_k.as_mut_ptr().cast::<f64>(), len) };
    let mut out_d =
        unsafe { std::slice::from_raw_parts_mut(raw_d.as_mut_ptr().cast::<f64>(), len) };
    py.detach(|| {
        indicators::stochrsi_into(
            real,
            timeperiod,
            fastk_period,
            fastk_period,
            fastd_period,
            &mut out_k,
            &mut out_d,
        )
    })
    .map_err(value_error)?;
    let k_ptr = raw_k.as_mut_ptr().cast::<f64>();
    let d_ptr = raw_d.as_mut_ptr().cast::<f64>();
    let k_cap = raw_k.capacity();
    let d_cap = raw_d.capacity();
    std::mem::forget(raw_k);
    std::mem::forget(raw_d);
    Ok((
        PyArray1::from_vec(py, unsafe { Vec::from_raw_parts(k_ptr, len, k_cap) }),
        PyArray1::from_vec(py, unsafe { Vec::from_raw_parts(d_ptr, len, d_cap) }),
    ))
}

#[pyfunction(name = "ultosc")]
#[pyo3(signature = (high, low, close, timeperiod1=7, timeperiod2=14, timeperiod3=28))]
fn ultosc<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
    timeperiod1: usize,
    timeperiod2: usize,
    timeperiod3: usize,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let high = high.as_slice().map_err(value_error)?;
    let low = low.as_slice().map_err(value_error)?;
    let close = close.as_slice().map_err(value_error)?;
    let values = py
        .detach(|| indicators::ultosc(high, low, close, timeperiod1, timeperiod2, timeperiod3))
        .map_err(value_error)?
        .into_raw_vec();
    Ok(PyArray1::from_vec(py, values))
}

#[pyfunction(name = "sarext")]
#[pyo3(signature = (high, low, startvalue=0.0, offsetonreverse=0.0, afinitlong=0.02, aflong=0.02, afmaxlong=0.2, afinitshort=0.02, afshort=0.02, afmaxshort=0.2))]
#[allow(clippy::too_many_arguments)]
fn sarext<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    startvalue: f64,
    offsetonreverse: f64,
    afinitlong: f64,
    aflong: f64,
    afmaxlong: f64,
    afinitshort: f64,
    afshort: f64,
    afmaxshort: f64,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let high = high.as_slice().map_err(value_error)?;
    let low = low.as_slice().map_err(value_error)?;
    let len = high.len();
    let mut raw_values = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe { raw_values.set_len(len) };
    let mut values =
        unsafe { std::slice::from_raw_parts_mut(raw_values.as_mut_ptr().cast::<f64>(), len) };
    py.detach(|| {
        indicators::sarext_sar_into(
            high,
            low,
            startvalue,
            offsetonreverse,
            afinitlong,
            aflong,
            afmaxlong,
            afinitshort,
            afshort,
            afmaxshort,
            &mut values,
        )
    })
    .map_err(value_error)?;
    let ptr = raw_values.as_mut_ptr().cast::<f64>();
    let capacity = raw_values.capacity();
    std::mem::forget(raw_values);
    Ok(PyArray1::from_vec(py, unsafe {
        Vec::from_raw_parts(ptr, len, capacity)
    }))
}

#[pyfunction(name = "minmax")]
#[pyo3(signature = (real, timeperiod=30))]
fn minmax<'py>(
    py: Python<'py>,
    real: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<(Bound<'py, PyArray1<f64>>, Bound<'py, PyArray1<f64>>)> {
    let real = real.as_slice().map_err(value_error)?;
    let (min_values, max_values) = py
        .detach(|| indicators::minmax(real, timeperiod))
        .map_err(value_error)?;
    Ok((
        PyArray1::from_vec(py, min_values.into_raw_vec()),
        PyArray1::from_vec(py, max_values.into_raw_vec()),
    ))
}

index_period!(maxindex, "maxindex", indicators::maxindex);
index_period!(minindex, "minindex", indicators::minindex);

#[pyfunction(name = "minmaxindex")]
#[pyo3(signature = (real, timeperiod=30))]
fn minmaxindex<'py>(
    py: Python<'py>,
    real: PyReadonlyArray1<'py, f64>,
    timeperiod: usize,
) -> PyResult<(Bound<'py, PyArray1<i64>>, Bound<'py, PyArray1<i64>>)> {
    let real = real.as_slice().map_err(value_error)?;
    let (min_values, max_values) = py
        .detach(|| indicators::minmaxindex(real, timeperiod))
        .map_err(value_error)?;
    let mut min_values = min_values.into_raw_vec();
    let mut max_values = max_values.into_raw_vec();
    let first = timeperiod.saturating_sub(1);
    for value in min_values.iter_mut().take(first) {
        *value = 0;
    }
    for value in max_values.iter_mut().take(first) {
        *value = 0;
    }
    for (index, value) in min_values.iter_mut().enumerate().skip(first) {
        if *value >= 0 {
            *value += (index + 1 - timeperiod) as i64;
        }
    }
    for (index, value) in max_values.iter_mut().enumerate().skip(first) {
        if *value >= 0 {
            *value += (index + 1 - timeperiod) as i64;
        }
    }
    Ok((
        PyArray1::from_vec(py, min_values),
        PyArray1::from_vec(py, max_values),
    ))
}

candle!(cdl2crows, "cdl2crows", cdl_2crows);
candle!(cdl3blackcrows, "cdl3blackcrows", cdl_3black_crows);
candle!(cdl3inside, "cdl3inside", cdl_3inside);
candle!(cdl3linestrike, "cdl3linestrike", cdl_3linestrike);
candle!(cdl3outside, "cdl3outside", cdl_3outside);
candle!(cdl3starsinsouth, "cdl3starsinsouth", cdl_3starsinsouth);
candle!(cdl3whitesoldiers, "cdl3whitesoldiers", cdl_3white_soldiers);
candle!(cdlabandonedbaby, "cdlabandonedbaby", cdl_abandoned_baby);
candle!(cdladvanceblock, "cdladvanceblock", cdl_advanceblock);
candle!(cdlbelthold, "cdlbelthold", cdl_belthold);
candle!(cdlbreakaway, "cdlbreakaway", cdl_breakaway);
candle!(
    cdlclosingmarubozu,
    "cdlclosingmarubozu",
    cdl_closingmarubozu
);
candle!(
    cdlconcealbabyswall,
    "cdlconcealbabyswall",
    cdl_concealbabyswall
);
candle!(cdlcounterattack, "cdlcounterattack", cdl_counterattack);
candle!(cdldarkcloudcover, "cdldarkcloudcover", cdl_darkcloudcover);
candle!(cdldoji, "cdldoji", cdl_doji);
candle!(cdldojistar, "cdldojistar", cdl_doji_star);
candle!(cdldragonflydoji, "cdldragonflydoji", cdl_dragonflydoji);
candle!(cdlengulfing, "cdlengulfing", cdl_engulfing);
candle!(
    cdleveningdojistar,
    "cdleveningdojistar",
    cdl_eveningdojistar
);
candle!(cdleveningstar, "cdleveningstar", cdl_eveningstar);
candle!(
    cdlgapsidesidewhite,
    "cdlgapsidesidewhite",
    cdl_gap_side_white
);
candle!(cdlgravestonedoji, "cdlgravestonedoji", cdl_gravestonedoji);
candle!(cdlhammer, "cdlhammer", cdl_hammer);
candle!(cdlhangingman, "cdlhangingman", cdl_hangingman);
candle!(cdlharami, "cdlharami", cdl_harami);
candle!(cdlharamicross, "cdlharamicross", cdl_haramicross);
candle!(cdlhighwave, "cdlhighwave", cdl_highwave);
candle!(cdlhikkake, "cdlhikkake", cdl_hikkake);
candle!(cdlhikkakemod, "cdlhikkakemod", cdl_hikkake_mod);
candle!(cdlhomingpigeon, "cdlhomingpigeon", cdl_homing_pigeon);
candle!(
    cdlidentical3crows,
    "cdlidentical3crows",
    cdl_identical3crows
);
candle!(cdlinneck, "cdlinneck", cdl_inneck);
candle!(cdlinvertedhammer, "cdlinvertedhammer", cdl_invertedhammer);
candle!(cdlkicking, "cdlkicking", cdl_kicking);
candle!(
    cdlkickingbylength,
    "cdlkickingbylength",
    cdl_kickingbylength
);
candle!(cdlladderbottom, "cdlladderbottom", cdl_ladder_bottom);
candle!(cdllongleggeddoji, "cdllongleggeddoji", cdl_longleggeddoji);
candle!(cdllongline, "cdllongline", cdl_longline);
candle!(cdlmarubozu, "cdlmarubozu", cdl_marubozu);
candle!(cdlmatchinglow, "cdlmatchinglow", cdl_matchinglow);
candle!(cdlmathold, "cdlmathold", cdl_mathold);
candle!(
    cdlmorningdojistar,
    "cdlmorningdojistar",
    cdl_morningdojistar
);
candle!(cdlmorningstar, "cdlmorningstar", cdl_morningstar);
candle!(cdlonneck, "cdlonneck", cdl_onneck);
candle!(cdlpiercing, "cdlpiercing", cdl_piercing);
candle!(cdlrickshawman, "cdlrickshawman", cdl_rickshawman);
candle!(
    cdlrisefall3methods,
    "cdlrisefall3methods",
    cdl_rise_fall_3methods
);
candle!(
    cdlseparatinglines,
    "cdlseparatinglines",
    cdl_separatinglines
);
candle!(cdlshootingstar, "cdlshootingstar", cdl_shootingstar);
candle!(cdlshortline, "cdlshortline", cdl_shortline);
candle!(cdlspinningtop, "cdlspinningtop", cdl_spinningtop);
candle!(cdlstalledpattern, "cdlstalledpattern", cdl_stalledpattern);
candle!(cdlsticksandwich, "cdlsticksandwich", cdl_sticksandwich);
candle!(cdltakuri, "cdltakuri", cdl_takuri);
candle!(cdltasukigap, "cdltasukigap", cdl_tasukigap);
candle!(cdlthrusting, "cdlthrusting", cdl_thrusting);
candle!(cdltristar, "cdltristar", cdl_tristar);
candle!(cdlunique3river, "cdlunique3river", cdl_unique3river);
candle!(
    cdlupsidegap2crows,
    "cdlupsidegap2crows",
    cdl_upsidegap2crows
);
candle!(
    cdlxsidegap3methods,
    "cdlxsidegap3methods",
    cdl_xsidegap3methods
);

pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    macro_rules! add {
        ($($name:ident),+ $(,)?) => { $(m.add_function(wrap_pyfunction!($name, m)?)?;)+ };
    }
    add!(
        trima,
        rocp,
        rocr,
        rocr100,
        linearreg,
        linearreg_angle,
        linearreg_intercept,
        linearreg_slope,
        max_value,
        min_value,
        sum_value,
        acos,
        asin,
        atan,
        ceil,
        cos,
        cosh,
        exp,
        floor,
        ln,
        log10,
        sin,
        sinh,
        sqrt,
        tan,
        tanh,
        add,
        div,
        mult,
        sub,
        adxr,
        aroonosc,
        bbands,
        ma,
        ppo_compat,
        macdfix,
        macdext,
        mavp,
        stochf,
        stochrsi,
        ultosc,
        sarext,
        minmax,
        maxindex,
        minindex,
        minmaxindex,
        cdl2crows,
        cdl3blackcrows,
        cdl3inside,
        cdl3linestrike,
        cdl3outside,
        cdl3starsinsouth,
        cdl3whitesoldiers,
        cdlabandonedbaby,
        cdladvanceblock,
        cdlbelthold,
        cdlbreakaway,
        cdlclosingmarubozu,
        cdlconcealbabyswall,
        cdlcounterattack,
        cdldarkcloudcover,
        cdldoji,
        cdldojistar,
        cdldragonflydoji,
        cdlengulfing,
        cdleveningdojistar,
        cdleveningstar,
        cdlgapsidesidewhite,
        cdlgravestonedoji,
        cdlhammer,
        cdlhangingman,
        cdlharami,
        cdlharamicross,
        cdlhighwave,
        cdlhikkake,
        cdlhikkakemod,
        cdlhomingpigeon,
        cdlidentical3crows,
        cdlinneck,
        cdlinvertedhammer,
        cdlkicking,
        cdlkickingbylength,
        cdlladderbottom,
        cdllongleggeddoji,
        cdllongline,
        cdlmarubozu,
        cdlmatchinglow,
        cdlmathold,
        cdlmorningdojistar,
        cdlmorningstar,
        cdlonneck,
        cdlpiercing,
        cdlrickshawman,
        cdlrisefall3methods,
        cdlseparatinglines,
        cdlshootingstar,
        cdlshortline,
        cdlspinningtop,
        cdlstalledpattern,
        cdlsticksandwich,
        cdltakuri,
        cdltasukigap,
        cdlthrusting,
        cdltristar,
        cdlunique3river,
        cdlupsidegap2crows,
        cdlxsidegap3methods,
    );
    Ok(())
}
