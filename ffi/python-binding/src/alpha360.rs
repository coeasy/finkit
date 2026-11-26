//! Python access to the built-in Qlib Alpha158 and Alpha360 feature sets.

use ::finkit::features::{alpha158_features, alpha360, alpha360_names, FeatureMatrix};
use numpy::{PyArray2, PyArrayMethods, PyReadonlyArray1};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// One built-in feature set, exposed as `finkit.feature_set(name)`.
#[pyclass(name = "FeatureSet", unsendable)]
pub struct PyFeatureSet {
    name: String,
}

#[pymethods]
impl PyFeatureSet {
    #[new]
    fn new(name: &str) -> PyResult<Self> {
        if name.eq_ignore_ascii_case("alpha158") || name.eq_ignore_ascii_case("alpha360") {
            Ok(Self {
                name: name.to_ascii_lowercase(),
            })
        } else {
            Err(PyValueError::new_err(format!(
                "unknown feature set `{name}`; available: {:?}",
                ["alpha158", "alpha360"]
            )))
        }
    }

    /// Feature set name.
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    /// Number of feature columns.
    fn __len__(&self) -> usize {
        if self.name == "alpha158" {
            158
        } else {
            360
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "<finkit.FeatureSet {:?} with {} features>",
            self.name,
            self.__len__()
        )
    }

    /// Stable output column names in Qlib order.
    fn names(&self) -> Vec<String> {
        if self.name == "alpha158" {
            ::finkit::factors::builtin::alpha158::EXPRESSIONS
                .iter()
                .map(|(name, _)| (*name).to_string())
                .collect()
        } else {
            alpha360_names()
        }
    }

    /// Evaluate features and return a NumPy matrix with shape `(time, features)`.
    ///
    /// Every input must have the same length. Columns follow the selected
    /// Qlib feature order; warm-up rows use `NaN` where the reference does.
    #[pyo3(signature = (open, high, low, close, vwap, volume))]
    fn evaluate<'py>(
        &self,
        py: Python<'py>,
        open: PyReadonlyArray1<'py, f64>,
        high: PyReadonlyArray1<'py, f64>,
        low: PyReadonlyArray1<'py, f64>,
        close: PyReadonlyArray1<'py, f64>,
        vwap: PyReadonlyArray1<'py, f64>,
        volume: PyReadonlyArray1<'py, f64>,
    ) -> PyResult<Bound<'py, PyArray2<f64>>> {
        let open = open.as_slice()?.to_vec();
        let high = high.as_slice()?.to_vec();
        let low = low.as_slice()?.to_vec();
        let close = close.as_slice()?.to_vec();
        let vwap = vwap.as_slice()?.to_vec();
        let volume = volume.as_slice()?.to_vec();

        let library_name = self.name.clone();
        let matrix = py
            .detach(|| -> Result<FeatureMatrix, String> {
                match library_name.as_str() {
                    "alpha360" => alpha360(&open, &high, &low, &close, &vwap, &volume)
                        .map_err(|error| error.to_string()),
                    _ => alpha158_features(&open, &high, &low, &close, &vwap, &volume)
                        .map_err(|error| error.to_string()),
                }
            })
            .map_err(PyValueError::new_err)?;
        let output = PyArray2::zeros(py, [matrix.rows(), matrix.cols()], false);
        {
            let mut writable = output.readwrite();
            let values = writable.as_slice_mut().map_err(|error| {
                PyValueError::new_err(format!("could not access feature output array: {error}"))
            })?;
            for row in 0..matrix.rows() {
                for col in 0..matrix.cols() {
                    values[row * matrix.cols() + col] = matrix.get(row, col);
                }
            }
        }
        Ok(output)
    }
}

/// Load a built-in feature set by name.
#[pyfunction(name = "feature_set")]
pub fn feature_set_py(name: &str) -> PyResult<PyFeatureSet> {
    PyFeatureSet::new(name)
}

/// Names accepted by `finkit.feature_set`.
#[pyfunction]
pub fn available_feature_sets() -> Vec<&'static str> {
    vec!["alpha158", "alpha360"]
}

/// Register built-in feature set access on the root `finkit` module.
pub fn register_alpha360(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyFeatureSet>()?;
    m.add_function(wrap_pyfunction!(feature_set_py, m)?)?;
    m.add_function(wrap_pyfunction!(available_feature_sets, m)?)?;
    Ok(())
}
