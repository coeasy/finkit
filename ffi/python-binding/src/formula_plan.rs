//! Persistent compiled formula plan exposed to Python.
//!
//! The plan owns the parsed/optimized formula and a FormulaEngine.  The engine
//! keeps its bytecode/JIT caches and pooled scratch buffers alive across calls.
//! The NumPy zero-copy entry point requires contiguous float64 arrays and keeps
//! the evaluation under the GIL while borrowing their memory.

use ::finkit::formula::{CompiledFormula, FormulaContext, FormulaEngine};
use ::finkit::math::rolling_stats;
use ndarray::Array1;
use numpy::{PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
enum CanonicalFormula {
    Sma { period: usize },
    Ema { period: usize },
    Wma { period: usize },
    Kama { period: usize },
    Rsi { period: usize },
    Mom { period: usize },
    Roc { period: usize },
    Rocp { period: usize },
    Rocr { period: usize },
    Rocr100 { period: usize },
    Max { period: usize },
    Min { period: usize },
    Sum { period: usize },
    Atr { period: usize },
    Natr { period: usize },
    Cci { period: usize },
    Mfi { period: usize },
    Obv,
    Ad,
    Std { period: usize },
    Boll { period: usize, nbdev: f64 },
}

fn normalize_formula(source: &str) -> String {
    source
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .flat_map(char::to_uppercase)
        .collect()
}

fn is_high(name: &str) -> bool {
    matches!(name, "H" | "HIGH")
}

fn is_low(name: &str) -> bool {
    matches!(name, "L" | "LOW")
}

fn is_close(name: &str) -> bool {
    matches!(name, "C" | "CLOSE")
}

fn is_volume(name: &str) -> bool {
    matches!(name, "V" | "VOL" | "VOLUME")
}

fn canonical_formula(source: &str) -> Option<CanonicalFormula> {
    let normalized = normalize_formula(source);
    let open = normalized.find('(')?;
    let name = &normalized[..open];
    let body = normalized.get(open + 1..normalized.len().checked_sub(1)?)?;
    if !normalized.ends_with(')') {
        return None;
    }
    let args: Vec<&str> = body.split(',').collect();

    match name {
        "MA" | "SMA" | "BOLLMID" if args.len() == 2 && is_close(args[0]) => {
            let period = args[1].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Sma { period })
        }
        "EMA" if args.len() == 2 && is_close(args[0]) => {
            let period = args[1].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Ema { period })
        }
        "WMA" if args.len() == 2 && is_close(args[0]) => {
            let period = args[1].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Wma { period })
        }
        "KAMA" if args.len() == 2 && is_close(args[0]) => {
            let period = args[1].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Kama { period })
        }
        "RSI" if args.len() == 2 && is_close(args[0]) => {
            let period = args[1].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Rsi { period })
        }
        "MOM" if args.len() == 2 && is_close(args[0]) => {
            let period = args[1].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Mom { period })
        }
        "ROC" if args.len() == 2 && is_close(args[0]) => {
            let period = args[1].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Roc { period })
        }
        "ROCP" if args.len() == 2 && is_close(args[0]) => {
            let period = args[1].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Rocp { period })
        }
        "ROCR" if args.len() == 2 && is_close(args[0]) => {
            let period = args[1].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Rocr { period })
        }
        "ROCR100" if args.len() == 2 && is_close(args[0]) => {
            let period = args[1].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Rocr100 { period })
        }
        "MAX" if args.len() == 2 && is_close(args[0]) => {
            let period = args[1].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Max { period })
        }
        "MIN" if args.len() == 2 && is_close(args[0]) => {
            let period = args[1].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Min { period })
        }
        "SUM" if args.len() == 2 && is_close(args[0]) => {
            let period = args[1].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Sum { period })
        }
        "ATR" if args.len() == 4 && is_high(args[0]) && is_low(args[1]) && is_close(args[2]) => {
            let period = args[3].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Atr { period })
        }
        "NATR" if args.len() == 4 && is_high(args[0]) && is_low(args[1]) && is_close(args[2]) => {
            let period = args[3].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Natr { period })
        }
        "CCI" if args.len() == 4 && is_high(args[0]) && is_low(args[1]) && is_close(args[2]) => {
            let period = args[3].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Cci { period })
        }
        "MFI"
            if args.len() == 5
                && is_high(args[0])
                && is_low(args[1])
                && is_close(args[2])
                && is_volume(args[3]) =>
        {
            let period = args[4].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Mfi { period })
        }
        "OBV" if args.len() == 2 && is_close(args[0]) && is_volume(args[1]) => {
            Some(CanonicalFormula::Obv)
        }
        "AD" if args.len() == 4
            && is_high(args[0])
            && is_low(args[1])
            && is_close(args[2])
            && is_volume(args[3]) =>
        {
            Some(CanonicalFormula::Ad)
        }
        "STD" if args.len() == 2 && is_close(args[0]) => {
            let period = args[1].parse::<usize>().ok()?;
            (period > 0).then_some(CanonicalFormula::Std { period })
        }
        "BOLL" if args.len() == 3 && is_close(args[0]) => {
            let period = args[1].parse::<usize>().ok()?;
            let nbdev = args[2].parse::<f64>().ok()?;
            (period > 0).then_some(CanonicalFormula::Boll { period, nbdev })
        }
        _ => None,
    }
}

fn eval_canonical_formula(
    formula: CanonicalFormula,
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
) -> PyResult<Array1<f64>> {
    match formula {
        CanonicalFormula::Sma { period } => {
            let mut output = vec![0.0; close.len()];
            ::finkit::math::simd_kernels::sma_simd_into(close, period, &mut output);
            Ok(Array1::from_vec(output))
        }
        CanonicalFormula::Ema { period } => {
            let mut output = vec![0.0; close.len()];
            ::finkit::math::moving_avg::ema_fast_into(close, period, &mut output).map_err(
                |error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string()),
            )?;
            Ok(Array1::from_vec(output))
        }
        CanonicalFormula::Wma { period } => {
            let mut output = vec![0.0; close.len()];
            ::finkit::math::moving_avg::wma_into(close, period, &mut output).map_err(|error| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())
            })?;
            Ok(Array1::from_vec(output))
        }
        CanonicalFormula::Kama { period } => ::finkit::math::moving_avg::kama(close, period, 2, 30)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),
        CanonicalFormula::Rsi { period } => ::finkit::indicators::rsi(close, period)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),
        CanonicalFormula::Mom { period } => ::finkit::indicators::mom(close, period)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),
        CanonicalFormula::Roc { period } => ::finkit::indicators::roc(close, period)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),
        CanonicalFormula::Rocp { period } => ::finkit::indicators::rocp(close, period)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),
        CanonicalFormula::Rocr { period } => ::finkit::indicators::rocr(close, period)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),
        CanonicalFormula::Rocr100 { period } => ::finkit::indicators::rocr100(close, period)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),
        CanonicalFormula::Max { period } => ::finkit::indicators::max(close, period)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),
        CanonicalFormula::Min { period } => ::finkit::indicators::min(close, period)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),
        CanonicalFormula::Sum { period } => ::finkit::indicators::sum(close, period)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),
        CanonicalFormula::Atr { period } => ::finkit::indicators::atr(high, low, close, period)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),
        CanonicalFormula::Natr { period } => ::finkit::indicators::natr(high, low, close, period)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),
        CanonicalFormula::Cci { period } => ::finkit::indicators::cci(high, low, close, period)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),
        CanonicalFormula::Mfi { period } => {
            ::finkit::indicators::mfi(high, low, close, volume, period).map_err(|error| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())
            })
        }
        CanonicalFormula::Obv => ::finkit::indicators::obv(close, volume)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),
        CanonicalFormula::Ad => ::finkit::indicators::ad(high, low, close, volume)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),
        CanonicalFormula::Std { period } => rolling_stats::stddev(close, period, 1.0)
            .map(Array1::from_vec)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),
        CanonicalFormula::Boll { period, nbdev } => {
            rolling_stats::bbands_sma(close, period, nbdev, nbdev)
                .map(|(upper, _, _)| Array1::from_vec(upper))
                .map_err(|error| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())
                })
        }
    }
}

fn read_array<'py>(name: &str, array: PyReadonlyArray1<'py, f64>) -> PyResult<Vec<f64>> {
    array
        .as_slice()
        .map(|slice| slice.to_vec())
        .map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "{name} must be a contiguous one-dimensional float64 NumPy array: {error}"
            ))
        })
}

fn validate_lengths(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    amount: Option<&[f64]>,
) -> PyResult<usize> {
    let expected = close.len();
    if expected == 0 {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "formula inputs must not be empty",
        ));
    }

    let inputs = [
        ("open", open.len()),
        ("high", high.len()),
        ("low", low.len()),
        ("close", close.len()),
        ("volume", volume.len()),
    ];
    for (name, length) in inputs {
        if length != expected {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "formula input {name} has length {length}, expected {expected}"
            )));
        }
    }
    if let Some(length) = amount.map(<[f64]>::len) {
        if length != expected {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "formula input amount has length {length}, expected {expected}"
            )));
        }
    }
    Ok(expected)
}

fn formula_runtime_error(error: ::finkit::formula::FormulaError) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())
}

fn make_context(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    amount: Option<Vec<f64>>,
) -> FormulaContext {
    FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
        amount.map(Array1::from_vec),
    )
}

fn result_dict<'py>(
    py: Python<'py>,
    context: &FormulaContext,
    result: Array1<f64>,
) -> PyResult<Bound<'py, PyDict>> {
    let output = PyDict::new(py);
    for (name, value) in &context.variables {
        if name.as_ref().starts_with("_CSE") {
            continue;
        }
        output.set_item(
            name.as_ref(),
            PyArray1::from_vec(py, value.clone().into_raw_vec()),
        )?;
    }
    output.set_item("__result__", PyArray1::from_vec(py, result.into_raw_vec()))?;
    Ok(output)
}

/// A reusable formula compilation plan.
///
/// Construct this once and call eval repeatedly with different NumPy arrays.
/// The stream context retained after eval is also the backing store for
/// append_bar/eval_last, so repeated streaming updates do not concatenate the
/// complete history.
#[pyclass(name = "CompiledFormula", unsendable)]
pub struct PyCompiledFormula {
    source: String,
    engine: Option<FormulaEngine>,
    compiled: Arc<CompiledFormula>,
    stream_context: Option<FormulaContext>,
    canonical: Option<CanonicalFormula>,
}

#[pymethods]
impl PyCompiledFormula {
    #[new]
    fn new(source: String) -> PyResult<Self> {
        let mut engine = FormulaEngine::new();
        let compiled = engine
            .compile(&source)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PySyntaxError, _>(error.to_string()))?;
        let canonical = canonical_formula(&source);
        Ok(Self {
            source,
            engine: Some(engine),
            compiled: Arc::new(compiled),
            stream_context: None,
            canonical,
        })
    }

    #[getter]
    fn source(&self) -> &str {
        &self.source
    }

    /// Evaluate using the pooled engine. Inputs are copied into the owned
    /// stream context so the context can safely be reused by append_bar.
    #[pyo3(signature = (open, high, low, close, volume, amount=None))]
    #[allow(clippy::too_many_arguments)]
    fn eval<'py>(
        &mut self,
        py: Python<'py>,
        open: PyReadonlyArray1<'py, f64>,
        high: PyReadonlyArray1<'py, f64>,
        low: PyReadonlyArray1<'py, f64>,
        close: PyReadonlyArray1<'py, f64>,
        volume: PyReadonlyArray1<'py, f64>,
        amount: Option<PyReadonlyArray1<'py, f64>>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let open = read_array("open", open)?;
        let high = read_array("high", high)?;
        let low = read_array("low", low)?;
        let close = read_array("close", close)?;
        let volume = read_array("volume", volume)?;
        let amount = amount
            .map(|array| read_array("amount", array))
            .transpose()?;
        validate_lengths(&open, &high, &low, &close, &volume, amount.as_deref())?;

        if let Some(formula) = self.canonical {
            let result = eval_canonical_formula(formula, &high, &low, &close, &volume)?;
            let context = make_context(open, high, low, close, volume, amount);
            self.stream_context = Some(context);
            return result_dict(py, self.stream_context.as_ref().unwrap(), result);
        }

        let engine = self.engine.take().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "compiled formula is already being evaluated",
            )
        })?;
        let compiled = Arc::clone(&self.compiled);
        let (execution, context, engine) = py.detach(move || {
            let mut context = make_context(open, high, low, close, volume, amount);
            let execution = engine
                .execute_zero_copy_cached(&compiled, &mut context)
                .map_err(formula_runtime_error);
            (execution, context, engine)
        });
        self.engine = Some(engine);
        let result = execution?;
        self.stream_context = Some(context);
        result_dict(py, self.stream_context.as_ref().unwrap(), result)
    }

    /// Evaluate without copying the contiguous NumPy OHLCV inputs.
    ///
    /// Common pure indicator formulas reuse the exact same canonical kernels
    /// as the public indicator API. Other formulas continue through the
    /// compiled zero-copy engine.
    #[pyo3(signature = (open, high, low, close, volume, amount=None))]
    #[allow(clippy::too_many_arguments)]
    fn eval_zero_copy<'py>(
        &mut self,
        py: Python<'py>,
        open: PyReadonlyArray1<'py, f64>,
        high: PyReadonlyArray1<'py, f64>,
        low: PyReadonlyArray1<'py, f64>,
        close: PyReadonlyArray1<'py, f64>,
        volume: PyReadonlyArray1<'py, f64>,
        amount: Option<PyReadonlyArray1<'py, f64>>,
    ) -> PyResult<Bound<'py, PyDict>> {
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
        let amount = amount
            .as_ref()
            .map(|array| {
                array.as_slice().map_err(|error| {
                    PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                        "amount must be a contiguous float64 NumPy array: {error}"
                    ))
                })
            })
            .transpose()?;
        validate_lengths(open, high, low, close, volume, amount)?;

        if let Some(formula) = self.canonical {
            let result = py.detach(|| eval_canonical_formula(formula, high, low, close, volume))?;
            let output = PyDict::new(py);
            output.set_item("__result__", PyArray1::from_vec(py, result.into_raw_vec()))?;
            return Ok(output);
        }

        let engine = self.engine.take().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "compiled formula is already being evaluated",
            )
        })?;
        let execution =
            engine.eval_zero_copy_inputs(&self.compiled, open, high, low, close, volume, amount);
        self.engine = Some(engine);
        let result = execution.map_err(formula_runtime_error)?;
        let output = PyDict::new(py);
        output.set_item("__result__", PyArray1::from_vec(py, result.into_raw_vec()))?;
        Ok(output)
    }

    /// Evaluate a half-open range [start, end).  The compiled plan only
    /// materialises the dependency window required for this range.
    #[pyo3(signature = (open, high, low, close, volume, start, end, amount=None))]
    #[allow(clippy::too_many_arguments)]
    fn eval_range<'py>(
        &mut self,
        py: Python<'py>,
        open: PyReadonlyArray1<'py, f64>,
        high: PyReadonlyArray1<'py, f64>,
        low: PyReadonlyArray1<'py, f64>,
        close: PyReadonlyArray1<'py, f64>,
        volume: PyReadonlyArray1<'py, f64>,
        start: usize,
        end: usize,
        amount: Option<PyReadonlyArray1<'py, f64>>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let open = read_array("open", open)?;
        let high = read_array("high", high)?;
        let low = read_array("low", low)?;
        let close = read_array("close", close)?;
        let volume = read_array("volume", volume)?;
        let amount = amount
            .map(|array| read_array("amount", array))
            .transpose()?;
        let data_len = validate_lengths(&open, &high, &low, &close, &volume, amount.as_deref())?;
        if end > data_len || start > end {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "eval_range expects 0 <= start <= end <= input length",
            ));
        }

        let engine = self.engine.take().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "compiled formula is already being evaluated",
            )
        })?;
        let compiled = Arc::clone(&self.compiled);
        let (execution, context, engine) = py.detach(move || {
            let context = make_context(open, high, low, close, volume, amount);
            let execution = engine
                .eval_range(&compiled, &context, start, end)
                .map_err(formula_runtime_error);
            (execution, context, engine)
        });
        self.engine = Some(engine);
        let result = execution?;
        let output = PyDict::new(py);
        output.set_item("__result__", PyArray1::from_vec(py, result.into_raw_vec()))?;
        self.stream_context = Some(context);
        Ok(output)
    }

    /// Evaluate the last bar. With no arrays this reuses the context retained
    /// by the previous eval/eval_range and is the preferred streaming form.
    #[pyo3(signature = (open=None, high=None, low=None, close=None, volume=None, amount=None))]
    #[allow(clippy::too_many_arguments)]
    fn eval_last<'py>(
        &mut self,
        py: Python<'py>,
        open: Option<PyReadonlyArray1<'py, f64>>,
        high: Option<PyReadonlyArray1<'py, f64>>,
        low: Option<PyReadonlyArray1<'py, f64>>,
        close: Option<PyReadonlyArray1<'py, f64>>,
        volume: Option<PyReadonlyArray1<'py, f64>>,
        amount: Option<PyReadonlyArray1<'py, f64>>,
    ) -> PyResult<f64> {
        let context = match (open, high, low, close, volume, amount) {
            (None, None, None, None, None, None) => {
                self.stream_context.take().ok_or_else(|| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        "eval_last() without arrays requires a previous eval() or eval_range()",
                    )
                })?
            }
            (Some(open), Some(high), Some(low), Some(close), Some(volume), amount) => {
                let open = read_array("open", open)?;
                let high = read_array("high", high)?;
                let low = read_array("low", low)?;
                let close = read_array("close", close)?;
                let volume = read_array("volume", volume)?;
                let amount = amount
                    .map(|array| read_array("amount", array))
                    .transpose()?;
                validate_lengths(&open, &high, &low, &close, &volume, amount.as_deref())?;
                make_context(open, high, low, close, volume, amount)
            }
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "eval_last() requires all OHLCV arrays or no arrays",
                ))
            }
        };

        let engine = self.engine.take().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "compiled formula is already being evaluated",
            )
        })?;
        let compiled = Arc::clone(&self.compiled);
        let (execution, context, engine) = py.detach(move || {
            let execution = engine
                .eval_last(&compiled, &context)
                .map_err(formula_runtime_error);
            (execution, context, engine)
        });
        self.engine = Some(engine);
        self.stream_context = Some(context);
        execution
    }

    /// Append one bar to the retained streaming context in amortized O(1).
    fn append_bar(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> PyResult<()> {
        self.stream_context
            .as_mut()
            .ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "append_bar() requires a previous eval() or eval_range()",
                )
            })?
            .append_bar(open, high, low, close, volume);
        Ok(())
    }

    /// Reserve capacity for future streaming bars.
    fn reserve_bars(&mut self, additional: usize) -> PyResult<()> {
        self.stream_context
            .as_mut()
            .ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "reserve_bars() requires a previous eval() or eval_range()",
                )
            })?
            .reserve_bars(additional);
        Ok(())
    }

    /// Discard the retained stream context while keeping the compiled plan and caches.
    ///
    /// The next eval(), eval_range(), or eval_last() call with arrays starts a
    /// fresh context. This does not invalidate the compiled formula.
    fn reset(&mut self) {
        self.stream_context = None;
    }

    fn __repr__(&self) -> String {
        format!("CompiledFormula({:?})", self.source)
    }
}
