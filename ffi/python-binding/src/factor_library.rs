//! The shipped factor libraries, exposed to Python as one call.
//!
//! The M0-3 criterion is `finkit.factor_library("alpha158")` as a **one-line**
//! entry point, so this module is deliberately thin: it looks a library up,
//! hands out its metadata, and evaluates factors against `NumPy` arrays. Nothing
//! here re-implements factor arithmetic — every value comes from the same
//! compiled [`FactorGraphPlan`] the Rust side runs, which is what keeps the two
//! surfaces from drifting.
//!
//! ```python
//! import finkit, numpy as np
//!
//! lib = finkit.factor_library("alpha158")
//! len(lib)                      # 158
//! lib.expression("MA20")        # 'MA(CLOSE, 20)/CLOSE'
//! lib.dependencies("MA20")      # ['close']
//! values = lib.evaluate("MA20", close=close, open=open, high=high,
//!                       low=low, volume=volume, vwap=vwap)
//! ```
//!
//! # Why series are passed by keyword and validated
//!
//! A factor declares its dependencies, so the binding can check the caller's
//! series against them *before* running the plan. Without that check a missing
//! series becomes an all-`NaN` column, which is indistinguishable from a factor
//! that legitimately has no signal — the caller sees a plausible result and no
//! error. The check turns that into a named exception.

use ::finkit::factors::builtin::{factor_library, FactorLibrary, LIBRARY_NAMES};
use ::finkit::factors::FactorDirection;
use ::finkit::formula::FormulaContext;
use ndarray::Array1;
use numpy::{
    PyArray1, PyArray2, PyArrayMethods, PyReadonlyArray1, PyReadonlyArray2, PyUntypedArrayMethods,
};
use pyo3::exceptions::{PyKeyError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::BTreeMap;

/// The five OHLCV slots a `FormulaContext` owns, plus the extra series
/// Alpha158's price block reads.
///
/// Kept in one list so the keyword arguments, the validation and the context
/// construction cannot disagree about which names exist.
const SERIES_NAMES: &[&str] = &["open", "high", "low", "close", "volume", "vwap"];

/// A shipped factor library.
#[pyclass(name = "FactorLibrary", unsendable)]
pub struct PyFactorLibrary {
    library: FactorLibrary,
}

/// Convert a Rust-side factor error into the Python exception a caller expects.
fn to_py_error(error: ::finkit::factors::FactorError) -> PyErr {
    match error {
        ::finkit::factors::FactorError::UnknownLibrary(name) => PyValueError::new_err(format!(
            "unknown factor library `{name}`; available: {LIBRARY_NAMES:?}"
        )),
        ::finkit::factors::FactorError::UnknownFactor(name) => PyKeyError::new_err(name),
        other => PyValueError::new_err(other.to_string()),
    }
}

impl PyFactorLibrary {
    fn from_library(library: FactorLibrary) -> Self {
        Self { library }
    }

    /// Look up a factor, mapping a miss to `KeyError`.
    fn factor(
        &self,
        name: &str,
    ) -> PyResult<&std::sync::Arc<::finkit::factors::builtin::CompiledFactor>> {
        self.library
            .get(name)
            .ok_or_else(|| PyKeyError::new_err(name.to_string()))
    }

    /// Collect the supplied series, validate them, and build a context.
    ///
    /// Returns the context plus the row count, so `evaluate` can check the plan
    /// actually produced one value per bar.
    fn context(
        &self,
        factor_name: &str,
        series: &BTreeMap<&'static str, Vec<f64>>,
    ) -> PyResult<FormulaContext> {
        let factor = self.factor(factor_name)?;

        let missing: Vec<String> = factor
            .dependencies()
            .iter()
            .map(|dependency| dependency.to_ascii_lowercase())
            .filter(|dependency| !series.contains_key(dependency.as_str()))
            .collect();
        if !missing.is_empty() {
            // Reported in the caller's own spelling, not the graph's canonical
            // one: a message naming `CLOSE` when the keyword is `close` sends the
            // reader looking for an argument they cannot pass.
            return Err(PyValueError::new_err(format!(
                "factor `{factor_name}` needs series {missing:?}; \
                 this call supplied {:?}",
                series.keys().collect::<Vec<_>>()
            )));
        }

        let rows = series.values().map(Vec::len).max().unwrap_or(0);
        for (name, values) in series {
            if values.len() != rows {
                return Err(PyValueError::new_err(format!(
                    "series `{name}` has {} rows but the longest supplied series has {rows}; \
                     every series must be the same length",
                    values.len()
                )));
            }
        }
        if rows == 0 {
            return Err(PyValueError::new_err(
                "no series supplied; pass at least one of `close`, `open`, `high`, `low`, \
                 `volume`, `vwap`",
            ));
        }

        // `FormulaContext::new` takes its `data_len` from `open`, and the OHLCV
        // slots are owned rather than optional, so an omitted series has to be
        // present as a full-length column. It is filled with `NaN` rather than
        // zero: zero is a valid price and would silently produce a number, while
        // `NaN` propagates. The dependency check above already guarantees no
        // *needed* series is missing, so this only covers unused slots.
        let slot = |name: &str| -> Array1<f64> {
            match series.get(name) {
                Some(values) => Array1::from_vec(values.clone()),
                None => Array1::from_elem(rows, f64::NAN),
            }
        };
        let mut context = FormulaContext::new(
            slot("open"),
            slot("high"),
            slot("low"),
            slot("close"),
            slot("volume"),
            None,
        );
        // `vwap` is not one of the five owned slots, so it travels as a formula
        // variable under the canonical spelling the compiled plan uses.
        if let Some(vwap) = series.get("vwap") {
            context.set_variable("VWAP".to_string(), Array1::from_vec(vwap.clone()));
        }
        Ok(context)
    }
}

fn with_panel<T>(
    timestamps: &[i64],
    symbols: &[String],
    fields: &Bound<'_, PyDict>,
    evaluate: impl FnOnce(&::finkit::factors::panel::FactorPanel<'_>) -> PyResult<T>,
) -> PyResult<T> {
    let symbol_refs: Vec<&str> = symbols.iter().map(String::as_str).collect();
    let mut field_names = Vec::with_capacity(fields.len());
    let mut field_values = Vec::with_capacity(fields.len());
    for (key, value) in fields.iter() {
        let field_name = key
            .extract::<String>()
            .map_err(|_| PyValueError::new_err("panel field names must be strings"))?;
        let array = value.extract::<PyReadonlyArray2<'_, f64>>().map_err(|_| {
            PyValueError::new_err(format!(
                "panel field `{field_name}` must be a two-dimensional float NumPy array"
            ))
        })?;
        if array.shape() != [timestamps.len(), symbols.len()] {
            return Err(PyValueError::new_err(format!(
                "panel field `{field_name}` has shape {:?}; expected ({}, {})",
                array.shape(),
                timestamps.len(),
                symbols.len()
            )));
        }
        field_names.push(field_name);
        field_values.push(array.as_slice()?.to_vec());
    }
    let field_refs: Vec<(&str, &[f64])> = field_names
        .iter()
        .zip(&field_values)
        .map(|(name, values)| (name.as_str(), values.as_slice()))
        .collect();
    let panel = ::finkit::factors::panel::FactorPanel::new(timestamps, &symbol_refs, &field_refs)
        .map_err(to_py_error)?;
    evaluate(&panel)
}

/// Evaluate an arbitrary formula over a `time × symbols` panel.
#[pyfunction(name = "evaluate_panel_formula")]
#[pyo3(signature = (expression, timestamps, symbols, fields))]
fn evaluate_panel_formula_py<'py>(
    py: Python<'py>,
    expression: &str,
    timestamps: Vec<i64>,
    symbols: Vec<String>,
    fields: &Bound<'py, PyDict>,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    with_panel(&timestamps, &symbols, fields, |panel| {
        let result = panel.evaluate(expression).map_err(to_py_error)?;
        let output = PyArray2::zeros(py, [result.rows(), result.cols()], false);
        let mut writable = output.readwrite();
        let values = writable.as_slice_mut().map_err(|error| {
            PyValueError::new_err(format!("could not access panel output array: {error}"))
        })?;
        values.copy_from_slice(&result.values);
        Ok(output)
    })
}

#[pymethods]
impl PyFactorLibrary {
    #[new]
    fn new(name: &str) -> PyResult<Self> {
        let library = factor_library(name).map_err(to_py_error)?;
        Ok(Self { library })
    }

    /// Library name, as passed to `finkit.factor_library`.
    #[getter]
    fn name(&self) -> String {
        self.library.name().to_string()
    }

    /// Number of factors.
    fn __len__(&self) -> usize {
        self.library.len()
    }

    fn __contains__(&self, name: &str) -> bool {
        self.library.get(name).is_some()
    }

    fn __repr__(&self) -> String {
        format!(
            "<finkit.FactorLibrary {:?} with {} factors>",
            self.library.name(),
            self.library.len()
        )
    }

    /// Factor names, ascending.
    fn names(&self) -> Vec<String> {
        self.library.names().map(str::to_string).collect()
    }

    /// The expression a factor was compiled from, verbatim.
    ///
    /// Exposed because the expression is the specification: it is what a reader
    /// checks against Qlib's published definition, and it is the only thing that
    /// makes "this library implements Alpha158" auditable rather than asserted.
    fn expression(&self, name: &str) -> PyResult<String> {
        Ok(self.factor(name)?.expression().to_string())
    }

    /// External series a factor reads, in the spelling `evaluate` accepts.
    ///
    /// Lower-case deliberately. The compiled graph canonicalises to `CLOSE`, but
    /// the keyword arguments of `evaluate` are lower-case and so are the names
    /// `FactorDefinition` declares, so returning the canonical spelling here
    /// would hand the caller a name they cannot pass back.
    fn dependencies(&self, name: &str) -> PyResult<Vec<String>> {
        Ok(self
            .factor(name)?
            .dependencies()
            .iter()
            .map(|dependency| dependency.to_ascii_lowercase())
            .collect())
    }

    /// Preferred ranking direction as a string.
    fn direction(&self, name: &str) -> PyResult<String> {
        Ok(format!("{:?}", self.factor(name)?.direction()))
    }

    /// Metadata for one factor, without evaluating it.
    fn describe<'py>(&self, py: Python<'py>, name: &str) -> PyResult<Bound<'py, PyDict>> {
        let factor = self.factor(name)?;
        let output = PyDict::new(py);
        output.set_item("name", factor.name())?;
        output.set_item("expression", factor.expression())?;
        // Lower-case, matching `dependencies` and the `evaluate` keywords.
        output.set_item(
            "dependencies",
            factor
                .dependencies()
                .iter()
                .map(|dependency| dependency.to_ascii_lowercase())
                .collect::<Vec<_>>(),
        )?;
        output.set_item("direction", format!("{:?}", factor.direction()))?;
        output.set_item("library", self.library.name())?;
        Ok(output)
    }

    /// Evaluate one factor over the supplied series.
    ///
    /// Every series must be the same length; omitted series that the factor does
    /// not read are ignored, and omitted series that it *does* read raise
    /// `ValueError` naming them.
    #[pyo3(signature = (name, open=None, high=None, low=None, close=None, volume=None, vwap=None))]
    #[allow(clippy::too_many_arguments)]
    fn evaluate<'py>(
        &self,
        py: Python<'py>,
        name: &str,
        open: Option<PyReadonlyArray1<'py, f64>>,
        high: Option<PyReadonlyArray1<'py, f64>>,
        low: Option<PyReadonlyArray1<'py, f64>>,
        close: Option<PyReadonlyArray1<'py, f64>>,
        volume: Option<PyReadonlyArray1<'py, f64>>,
        vwap: Option<PyReadonlyArray1<'py, f64>>,
    ) -> PyResult<Bound<'py, PyArray1<f64>>> {
        let supplied = [
            ("open", open),
            ("high", high),
            ("low", low),
            ("close", close),
            ("volume", volume),
            ("vwap", vwap),
        ];
        let mut series: BTreeMap<&'static str, Vec<f64>> = BTreeMap::new();
        for (label, array) in supplied {
            if let Some(array) = array {
                series.insert(label, array.as_slice()?.to_vec());
            }
        }

        let context = self.context(name, &series)?;
        let values = self.library.evaluate(name, &context).map_err(to_py_error)?;
        Ok(PyArray1::from_vec(py, values))
    }

    /// Evaluate every factor in the library, returning `{name: ndarray}`.
    ///
    /// The plans are already compiled and shared, so a sweep costs one pass per
    /// factor and no parsing — the property `benches/factor_library_bench.rs`
    /// measures on the Rust side.
    #[pyo3(signature = (open=None, high=None, low=None, close=None, volume=None, vwap=None))]
    #[allow(clippy::too_many_arguments)]
    fn evaluate_all<'py>(
        &self,
        py: Python<'py>,
        open: Option<PyReadonlyArray1<'py, f64>>,
        high: Option<PyReadonlyArray1<'py, f64>>,
        low: Option<PyReadonlyArray1<'py, f64>>,
        close: Option<PyReadonlyArray1<'py, f64>>,
        volume: Option<PyReadonlyArray1<'py, f64>>,
        vwap: Option<PyReadonlyArray1<'py, f64>>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let supplied = [
            ("open", open),
            ("high", high),
            ("low", low),
            ("close", close),
            ("volume", volume),
            ("vwap", vwap),
        ];
        let mut series: BTreeMap<&'static str, Vec<f64>> = BTreeMap::new();
        for (label, array) in supplied {
            if let Some(array) = array {
                series.insert(label, array.as_slice()?.to_vec());
            }
        }

        let names: Vec<String> = self.library.names().map(str::to_string).collect();
        let output = PyDict::new(py);
        for factor_name in names {
            // A factor whose dependency is absent from *this* call is skipped
            // rather than fatal: a sweep over a caller who supplied only `close`
            // should return the factors it can compute. `evaluate` is the strict
            // entry point for a single factor.
            let Ok(context) = self.context(&factor_name, &series) else {
                continue;
            };
            let values = self
                .library
                .evaluate(&factor_name, &context)
                .map_err(to_py_error)?;
            output.set_item(factor_name, PyArray1::from_vec(py, values))?;
        }
        Ok(output)
    }

    /// Evaluate one factor across an aligned `(time, symbols)` input panel.
    ///
    /// `fields` maps each input name to a C-contiguous NumPy matrix. The result
    /// keeps the same axis order. Use explicit `CS_RANK`/`CS_SCALE` operations
    /// for cross-sectional transforms in custom formulas; rolling functions
    /// continue to run down each symbol column.
    #[pyo3(signature = (name, timestamps, symbols, fields))]
    fn evaluate_panel<'py>(
        &self,
        py: Python<'py>,
        name: &str,
        timestamps: Vec<i64>,
        symbols: Vec<String>,
        fields: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyArray2<f64>>> {
        self.factor(name)?;
        with_panel(&timestamps, &symbols, fields, |panel| {
            let result = self
                .library
                .evaluate_panel(name, panel)
                .map_err(to_py_error)?;
            let output = PyArray2::zeros(py, [result.rows(), result.cols()], false);
            let mut writable = output.readwrite();
            let values = writable.as_slice_mut().map_err(|error| {
                PyValueError::new_err(format!("could not access panel output array: {error}"))
            })?;
            values.copy_from_slice(&result.values);
            Ok(output)
        })
    }

    /// Evaluate all factors over an aligned panel, returning `{name: ndarray}`.
    #[pyo3(signature = (timestamps, symbols, fields))]
    fn evaluate_panel_all<'py>(
        &self,
        py: Python<'py>,
        timestamps: Vec<i64>,
        symbols: Vec<String>,
        fields: &Bound<'py, PyDict>,
    ) -> PyResult<Bound<'py, PyDict>> {
        with_panel(&timestamps, &symbols, fields, |panel| {
            let results = self
                .library
                .evaluate_all_panel(panel)
                .map_err(to_py_error)?;
            let output = PyDict::new(py);
            for (name, result) in results {
                let values = PyArray2::zeros(py, [result.rows(), result.cols()], false);
                let mut writable = values.readwrite();
                let destination = writable.as_slice_mut().map_err(|error| {
                    PyValueError::new_err(format!(
                        "could not access panel output array for `{name}`: {error}"
                    ))
                })?;
                destination.copy_from_slice(&result.values);
                output.set_item(name, values)?;
            }
            Ok(output)
        })
    }
}

/// The factor-library entry point: `finkit.factor_library("alpha158")`.
///
/// The exported name is spelled explicitly: `PyO3` defaults to the Rust function
/// name, which would have published this as `factor_library_py`.
#[pyfunction(name = "factor_library")]
pub fn factor_library_py(name: &str) -> PyResult<PyFactorLibrary> {
    PyFactorLibrary::new(name)
}

/// Build a caller-owned factor library from a name-to-expression mapping.
///
/// The expressions are compiled in memory and are not added to Finkit's built-in
/// catalog or package. This lets callers load their own privately licensed packs.
#[pyfunction(name = "factor_library_from_expressions")]
fn factor_library_from_expressions_py(
    name: &str,
    expressions: &Bound<'_, PyDict>,
) -> PyResult<PyFactorLibrary> {
    let mut owned = Vec::with_capacity(expressions.len());
    for (key, value) in expressions.iter() {
        let factor_name = key
            .extract::<String>()
            .map_err(|_| PyValueError::new_err("factor names must be strings"))?;
        let expression = value.extract::<String>().map_err(|_| {
            PyValueError::new_err(format!("expression for `{factor_name}` must be a string"))
        })?;
        owned.push((factor_name, expression));
    }
    let entries: Vec<(&str, &str, FactorDirection)> = owned
        .iter()
        .map(|(factor_name, expression)| {
            (
                factor_name.as_str(),
                expression.as_str(),
                FactorDirection::Neutral,
            )
        })
        .collect();
    let library = FactorLibrary::from_named_expressions(name, &entries).map_err(to_py_error)?;
    Ok(PyFactorLibrary::from_library(library))
}

/// Names accepted by `finkit.factor_library`.
#[pyfunction]
pub fn available_factor_libraries() -> Vec<&'static str> {
    LIBRARY_NAMES.to_vec()
}

/// Every factor name in the shipped registry, ascending.
///
/// The registry is the union of the libraries and the demo factors; it is built
/// once inside the crate and shared, so this call does not recompile anything.
#[pyfunction]
pub fn factor_registry_names() -> Vec<String> {
    ::finkit::factors::builtin_factor_registry()
        .names()
        .map(str::to_string)
        .collect()
}

/// The series keywords `evaluate` / `evaluate_all` accept.
#[pyfunction]
pub fn factor_library_series_names() -> Vec<&'static str> {
    SERIES_NAMES.to_vec()
}

/// Register the factor-library surface on the `finkit` module.
pub fn register_factor_library(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyFactorLibrary>()?;
    m.add_function(wrap_pyfunction!(factor_library_py, m)?)?;
    m.add_function(wrap_pyfunction!(factor_library_from_expressions_py, m)?)?;
    m.add_function(wrap_pyfunction!(available_factor_libraries, m)?)?;
    m.add_function(wrap_pyfunction!(factor_registry_names, m)?)?;
    m.add_function(wrap_pyfunction!(factor_library_series_names, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_panel_formula_py, m)?)?;
    Ok(())
}
