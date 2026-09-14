#[path = "native_fast_path.rs"]
mod native_fast_path;

use finkit::transforms::{
    Diff, DiffN, LogReturn, MinMaxScaler, PctChange, PercentileRank, Pipeline, Rank, RollingMean,
    RollingStd, RollingSum, StandardScaler, Transform, ZScore,
};
use numpy::PyReadonlyArray1;
use pyo3::prelude::*;

#[pyclass(name = "Pipeline")]
pub struct PyPipeline {
    inner: Pipeline,
}

#[pymethods]
impl PyPipeline {
    #[new]
    fn new() -> Self {
        Self {
            inner: Pipeline::new(),
        }
    }

    fn add_log_return(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        let pipeline = std::mem::take(&mut slf.inner);
        slf.inner = pipeline.add(LogReturn);
        slf
    }

    fn add_pct_change(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        let pipeline = std::mem::take(&mut slf.inner);
        slf.inner = pipeline.add(PctChange);
        slf
    }

    fn add_zscore(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        let pipeline = std::mem::take(&mut slf.inner);
        slf.inner = pipeline.add(ZScore);
        slf
    }

    fn add_standard_scaler(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        let pipeline = std::mem::take(&mut slf.inner);
        slf.inner = pipeline.add(StandardScaler);
        slf
    }

    fn add_min_max_scaler(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        let pipeline = std::mem::take(&mut slf.inner);
        slf.inner = pipeline.add(MinMaxScaler);
        slf
    }

    fn add_rank(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        let pipeline = std::mem::take(&mut slf.inner);
        slf.inner = pipeline.add(Rank);
        slf
    }

    fn add_percentile_rank(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        let pipeline = std::mem::take(&mut slf.inner);
        slf.inner = pipeline.add(PercentileRank);
        slf
    }

    fn add_diff(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        let pipeline = std::mem::take(&mut slf.inner);
        slf.inner = pipeline.add(Diff);
        slf
    }

    fn add_diff_n(mut slf: PyRefMut<'_, Self>, order: usize) -> PyRefMut<'_, Self> {
        let pipeline = std::mem::take(&mut slf.inner);
        slf.inner = pipeline.add(DiffN { order });
        slf
    }

    fn add_rolling_mean(mut slf: PyRefMut<'_, Self>, window: usize) -> PyRefMut<'_, Self> {
        let pipeline = std::mem::take(&mut slf.inner);
        slf.inner = pipeline.add(RollingMean { window });
        slf
    }

    fn add_rolling_std(mut slf: PyRefMut<'_, Self>, window: usize) -> PyRefMut<'_, Self> {
        let pipeline = std::mem::take(&mut slf.inner);
        slf.inner = pipeline.add(RollingStd { window });
        slf
    }

    fn add_rolling_sum(mut slf: PyRefMut<'_, Self>, window: usize) -> PyRefMut<'_, Self> {
        let pipeline = std::mem::take(&mut slf.inner);
        slf.inner = pipeline.add(RollingSum { window });
        slf
    }

    fn transform(&self, py: Python<'_>, data: PyReadonlyArray1<'_, f64>) -> PyResult<Vec<f64>> {
        let slice = data
            .as_slice()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
        Ok(py.detach(|| self.inner.transform(slice)))
    }
}

#[pyfunction]
pub fn transform_log_return(py: Python<'_>, data: PyReadonlyArray1<'_, f64>) -> PyResult<Vec<f64>> {
    let slice = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| LogReturn.transform(slice)))
}

#[pyfunction]
pub fn transform_zscore(py: Python<'_>, data: PyReadonlyArray1<'_, f64>) -> PyResult<Vec<f64>> {
    let slice = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| ZScore.transform(slice)))
}

#[pyfunction]
pub fn transform_rank(py: Python<'_>, data: PyReadonlyArray1<'_, f64>) -> PyResult<Vec<f64>> {
    let slice = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| Rank.transform(slice)))
}

#[pyfunction]
pub fn transform_diff(py: Python<'_>, data: PyReadonlyArray1<'_, f64>) -> PyResult<Vec<f64>> {
    let slice = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| Diff.transform(slice)))
}

#[pyfunction]
#[pyo3(signature = (data, window=5))]
pub fn transform_rolling_mean(
    py: Python<'_>,
    data: PyReadonlyArray1<'_, f64>,
    window: usize,
) -> PyResult<Vec<f64>> {
    let slice = data
        .as_slice()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    Ok(py.detach(|| RollingMean { window }.transform(slice)))
}

pub fn register_transform_classes(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPipeline>()?;
    m.add_function(pyo3::wrap_pyfunction!(transform_log_return, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(transform_zscore, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(transform_rank, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(transform_diff, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(transform_rolling_mean, m)?)?;
    native_fast_path::register(m)?;
    Ok(())
}
