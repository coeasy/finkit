//! Persistent compiled formula plan exposed to Python.
//!
//! The plan owns the parsed/optimized formula and a FormulaEngine.  The engine
//! keeps its bytecode/JIT caches and pooled scratch buffers alive across calls.
//! The NumPy zero-copy entry point requires contiguous float64 arrays and keeps
//! the evaluation under the GIL while borrowing their memory.

use ::finkit::formula::{
    inspect_formula_compatibility, CompiledFormula, FormulaContext, FormulaEngine,
};
use ndarray::Array1;
use numpy::{PyArray1, PyReadonlyArray1};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::Arc;

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
}

/// Reusable registry for parameterized expression components.
///
/// Components are expanded and validated by the Rust engine before a plan is
/// returned, so Python callers can build the same nested custom indicators as
/// native Rust callers without textual source rewriting.
#[pyclass(name = "FormulaRegistry", unsendable)]
pub struct PyFormulaRegistry {
    engine: Option<FormulaEngine>,
}

#[pymethods]
impl PyFormulaRegistry {
    #[new]
    fn new() -> Self {
        Self {
            engine: Some(FormulaEngine::new()),
        }
    }

    /// Register an expression-only component, e.g. `MA(X, N) + EMA(X, N)`.
    fn register(&mut self, name: String, parameters: Vec<String>, source: String) -> PyResult<()> {
        let refs: Vec<&str> = parameters.iter().map(String::as_str).collect();
        self.engine
            .as_mut()
            .expect("formula registry engine is available")
            .register_custom_formula(&name, &refs, &source)
            .map_err(formula_runtime_error)
    }

    /// Remove one component and return whether it existed.
    fn unregister(&mut self, name: &str) -> PyResult<bool> {
        self.engine
            .as_mut()
            .expect("formula registry engine is available")
            .unregister_custom_formula(name)
            .map_err(formula_runtime_error)
    }

    /// Return registered component names in deterministic order.
    fn names(&self) -> Vec<String> {
        self.engine
            .as_ref()
            .expect("formula registry engine is available")
            .custom_formula_names()
    }

    /// Compile a source formula using this registry.
    fn compile(&mut self, source: String) -> PyResult<PyCompiledFormula> {
        let mut engine = self
            .engine
            .take()
            .expect("formula registry engine is available");
        let compiled = match engine.compile(&source) {
            Ok(compiled) => compiled,
            Err(error) => {
                self.engine = Some(engine);
                return Err(PyErr::new::<pyo3::exceptions::PySyntaxError, _>(
                    error.to_string(),
                ));
            }
        };
        Ok(PyCompiledFormula {
            source,
            engine: Some(engine),
            compiled: Arc::new(compiled),
            stream_context: None,
        })
    }
}

#[pymethods]
impl PyCompiledFormula {
    #[new]
    fn new(source: String) -> PyResult<Self> {
        let mut engine = FormulaEngine::new();
        let compiled = engine
            .compile(&source)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PySyntaxError, _>(error.to_string()))?;
        Ok(Self {
            source,
            engine: Some(engine),
            compiled: Arc::new(compiled),
            stream_context: None,
        })
    }

    #[getter]
    fn source(&self) -> &str {
        &self.source
    }

    /// Return dependency, lookback, future-data and streaming diagnostics.
    fn analyze<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let analysis = self
            .engine
            .as_ref()
            .expect("compiled formula engine is available")
            .analyze_ast(&self.compiled.ast);
        let output = PyDict::new(py);
        output.set_item("input_variables", analysis.input_variables)?;
        output.set_item("assigned_variables", analysis.assigned_variables)?;
        output.set_item("called_functions", analysis.called_functions)?;
        output.set_item("unknown_functions", analysis.unknown_functions)?;
        output.set_item("required_lookback", analysis.required_lookback)?;
        output.set_item("estimated_nodes", analysis.estimated_nodes)?;
        output.set_item("estimated_cost", analysis.estimated_cost)?;
        output.set_item("has_future_data", analysis.has_future_data)?;
        output.set_item("has_stateful_functions", analysis.has_stateful_functions)?;
        output.set_item("has_observable_effects", analysis.has_observable_effects)?;
        output.set_item("has_control_flow", analysis.has_control_flow)?;
        output.set_item("supports_streaming", analysis.supports_streaming)?;
        let diagnostics: Vec<_> = analysis
            .diagnostics
            .iter()
            .map(|item| {
                let dict = PyDict::new(py);
                dict.set_item("code", &item.code)?;
                dict.set_item("level", format!("{:?}", item.level))?;
                dict.set_item("message", &item.message)?;
                Ok::<_, PyErr>(dict.into_any())
            })
            .collect::<PyResult<_>>()?;
        output.set_item("diagnostics", diagnostics)?;
        Ok(output)
    }

    /// Return the stable result shape, warm-up and null-value contract.
    #[pyo3(signature = (data_len = 0))]
    fn metadata<'py>(&self, py: Python<'py>, data_len: usize) -> PyResult<Bound<'py, PyDict>> {
        let metadata = self
            .engine
            .as_ref()
            .expect("compiled formula engine is available")
            .metadata_for_formula(&self.compiled, data_len);
        let output = PyDict::new(py);
        output.set_item("schema_version", metadata.schema_version)?;
        output.set_item("length", metadata.length)?;
        output.set_item("dtype", metadata.dtype)?;
        output.set_item("output_names", metadata.output_names)?;
        output.set_item("null_policy", metadata.null_policy)?;
        output.set_item("required_lookback", metadata.required_lookback)?;
        output.set_item("warmup", metadata.warmup)?;
        output.set_item("valid_start", metadata.valid_start)?;
        output.set_item("has_future_data", metadata.has_future_data)?;
        output.set_item("supports_streaming", metadata.supports_streaming)?;
        output.set_item("has_observable_effects", metadata.has_observable_effects)?;
        Ok(output)
    }

    /// Return the complete TA-Lib public function catalog used by compatibility reports.
    fn talib_catalog<'py>(&self, py: Python<'py>) -> PyResult<Vec<Bound<'py, PyDict>>> {
        ::finkit::formula::ta_lib_function_contracts()
            .into_iter()
            .map(|item| {
                let dict = PyDict::new(py);
                dict.set_item("name", item.name)?;
                dict.set_item("category", item.category)?;
                dict.set_item("input_shape", item.input_shape)?;
                dict.set_item("outputs", item.outputs)?;
                dict.set_item("lookback", item.lookback)?;
                dict.set_item("warmup_policy", item.warmup_policy)?;
                dict.set_item("nan_policy", item.nan_policy)?;
                dict.set_item("runtime_registered", item.runtime_registered)?;
                Ok(dict)
            })
            .collect()
    }

    /// Inspect terminal-specific semantic compatibility without evaluation.
    #[pyo3(signature = (terminal = "finkit"))]
    fn compatibility_report<'py>(
        &self,
        py: Python<'py>,
        terminal: &str,
    ) -> PyResult<Bound<'py, PyDict>> {
        let terminal = ::finkit::formula::FormulaTerminal::from_str(terminal).ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "unknown formula terminal: {terminal}"
            ))
        })?;
        let report = inspect_formula_compatibility(&self.source, terminal)
            .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(error))?;
        let output = PyDict::new(py);
        output.set_item("terminal", terminal.as_str())?;
        output.set_item("normalized_source", report.normalized_source)?;
        output.set_item("profile_id", report.profile.id)?;
        output.set_item("null_policy", report.profile.null_policy)?;
        output.set_item("sma_policy", report.profile.sma_policy)?;
        output.set_item("lookahead_policy", report.profile.lookahead_policy)?;
        output.set_item(
            "requires_session_metadata",
            report.profile.requires_session_metadata,
        )?;
        let functions: Vec<_> = report
            .functions
            .iter()
            .map(|item| {
                let dict = PyDict::new(py);
                dict.set_item("name", &item.name)?;
                dict.set_item("status", item.status.as_str())?;
                dict.set_item("message", &item.message)?;
                dict.set_item("cataloged", item.cataloged)?;
                dict.set_item("runtime_registered", item.runtime_registered)?;
                dict.set_item("category", &item.category)?;
                dict.set_item("outputs", item.outputs)?;
                Ok::<_, PyErr>(dict.into_any())
            })
            .collect::<PyResult<_>>()?;
        output.set_item("functions", functions)?;
        output.set_item("ta_lib_catalog_version", report.ta_lib_catalog_version)?;
        output.set_item("ta_lib_function_count", report.ta_lib_function_count)?;
        output.set_item(
            "ta_lib_runtime_registered_count",
            report.ta_lib_runtime_registered_count,
        )?;
        Ok(output)
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
    /// The complete synchronous evaluation borrows the contiguous NumPy OHLCV
    /// buffers. Direct MA/EMA/RSI/BOLLMID formulas additionally avoid input
    /// argument materialisation; complex formulas may allocate intermediates
    /// required by the existing array-based built-in ABI.
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

    /// Evaluate a half-open range while borrowing contiguous NumPy inputs.
    /// Unlike `eval_range`, this method does not establish an owned retained
    /// stream context and is intended for repeated chart-window refreshes.
    #[pyo3(signature = (open, high, low, close, volume, start, end, amount=None))]
    #[allow(clippy::too_many_arguments)]
    fn eval_range_zero_copy<'py>(
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
        let open = open.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!("open: {error}"))
        })?;
        let high = high.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!("high: {error}"))
        })?;
        let low = low.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!("low: {error}"))
        })?;
        let close = close.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!("close: {error}"))
        })?;
        let volume = volume.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!("volume: {error}"))
        })?;
        let amount = amount
            .as_ref()
            .map(|array| {
                array.as_slice().map_err(|error| {
                    PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!("amount: {error}"))
                })
            })
            .transpose()?;
        let data_len = validate_lengths(open, high, low, close, volume, amount)?;
        if start > end || end > data_len {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "eval_range_zero_copy expects 0 <= start <= end <= input length",
            ));
        }
        let engine = self.engine.take().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "compiled formula is already being evaluated",
            )
        })?;
        let result = engine
            .eval_range_zero_copy_inputs(
                &self.compiled,
                open,
                high,
                low,
                close,
                volume,
                amount,
                start,
                end,
            )
            .map_err(formula_runtime_error);
        self.engine = Some(engine);
        let result = result?;
        let output = PyDict::new(py);
        output.set_item("__result__", PyArray1::from_vec(py, result.into_raw_vec()))?;
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
