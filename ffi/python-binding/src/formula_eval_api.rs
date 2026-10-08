//! The `formula_eval*` family: parse, compile and run formulas.

use super::*;

#[pyfunction]
#[pyo3(signature = (source, open, high, low, close, volume, amount=None))]
#[cfg(feature = "formula")]
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn formula_eval(
    py: pyo3::Python<'_>,
    source: &str,
    open: &Bound<'_, PyAny>,
    high: &Bound<'_, PyAny>,
    low: &Bound<'_, PyAny>,
    close: &Bound<'_, PyAny>,
    volume: &Bound<'_, PyAny>,
    amount: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let open_vec = extract_array_bound(open)?;
    let high_vec = extract_array_bound(high)?;
    let low_vec = extract_array_bound(low)?;
    let close_vec = extract_array_bound(close)?;
    let volume_vec = extract_array_bound(volume)?;
    let amount_vec = amount.map(extract_array_bound).transpose()?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);
    let amount_array = amount_vec.map(Array1::from_vec);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        amount_array,
    );
    let mut engine = FormulaEngine::new();

    let result = py.detach(|| {
        engine
            .eval(source, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let dict = pyo3::types::PyDict::new(py);

    for (name, value) in &ctx.variables {
        let vec_value = value.to_vec();
        dict.set_item(name.to_string(), vec_value)?;
    }

    dict.set_item("__result__", result.to_vec())?;

    Ok(dict.into())
}

#[pyfunction]
#[pyo3(signature = (source, open, high, low, close, volume, dialect = "alpha_ta", amount=None))]
#[cfg(feature = "formula")]
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn formula_eval_dialect(
    py: pyo3::Python<'_>,
    source: &str,
    open: &Bound<'_, PyAny>,
    high: &Bound<'_, PyAny>,
    low: &Bound<'_, PyAny>,
    close: &Bound<'_, PyAny>,
    volume: &Bound<'_, PyAny>,
    dialect: &str,
    amount: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let open_vec = extract_array_bound(open)?;
    let high_vec = extract_array_bound(high)?;
    let low_vec = extract_array_bound(low)?;
    let close_vec = extract_array_bound(close)?;
    let volume_vec = extract_array_bound(volume)?;
    let amount_vec = amount.map(extract_array_bound).transpose()?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);
    let amount_array = amount_vec.map(Array1::from_vec);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        amount_array,
    );
    let mut engine = FormulaEngine::new();

    let dialect = FormulaDialect::from_str(dialect).unwrap_or(FormulaDialect::AlphaTA);
    let result = py.detach(|| -> PyResult<Array1<f64>> {
        engine
            .eval_with_dialect(source, dialect, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let dict = pyo3::types::PyDict::new(py);

    for (name, value) in &ctx.variables {
        let vec_value = value.to_vec();
        dict.set_item(name.to_string(), vec_value)?;
    }

    dict.set_item("__result__", result.to_vec())?;

    Ok(dict.into())
}

#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_eval_bytecode(
    py: Python<'_>,
    source: &str,
    open: Py<PyAny>,
    high: Py<PyAny>,
    low: Py<PyAny>,
    close: Py<PyAny>,
    volume: Py<PyAny>,
) -> PyResult<Py<PyAny>> {
    let open_vec = extract_array_pyobject(open)?;
    let high_vec = extract_array_pyobject(high)?;
    let low_vec = extract_array_pyobject(low)?;
    let close_vec = extract_array_pyobject(close)?;
    let volume_vec = extract_array_pyobject(volume)?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);

    let (result, variables): (
        Array1<f64>,
        std::collections::HashMap<std::sync::Arc<str>, Array1<f64>>,
    ) = py.detach(|| {
        let ctx = FormulaContext::new(
            open_array,
            high_array,
            low_array,
            close_array,
            volume_array,
            None,
        );
        let mut engine = FormulaEngine::new();
        let result = engine
            .compile_bytecode(source)
            .and_then(|bc| engine.execute_bytecode(&bc, &ctx))
            .map_err(formula_error_to_pyerr)?;
        Result::<_, PyErr>::Ok((result, ctx.variables))
    })?;

    let dict = pyo3::types::PyDict::new(py);

    for (name, value) in &variables {
        let vec_value = value.to_vec();
        dict.set_item(name.to_string(), vec_value)?;
    }

    dict.set_item("__result__", result.to_vec())?;

    Ok(dict.into())
}

#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_eval_optimized(
    py: Python<'_>,
    source: &str,
    open: Py<PyAny>,
    high: Py<PyAny>,
    low: Py<PyAny>,
    close: Py<PyAny>,
    volume: Py<PyAny>,
) -> PyResult<Py<PyAny>> {
    let open_vec = extract_array_pyobject(open)?;
    let high_vec = extract_array_pyobject(high)?;
    let low_vec = extract_array_pyobject(low)?;
    let close_vec = extract_array_pyobject(close)?;
    let volume_vec = extract_array_pyobject(volume)?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        None,
    );
    let mut engine = FormulaEngine::new();

    let result = py.detach(|| {
        engine
            .eval_optimized(source, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let dict = pyo3::types::PyDict::new(py);

    for (name, value) in &ctx.variables {
        let vec_value = value.to_vec();
        dict.set_item(name.to_string(), vec_value)?;
    }

    dict.set_item("__result__", result.to_vec())?;

    Ok(dict.into())
}

#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_eval_jit(
    py: Python<'_>,
    source: &str,
    open: Py<PyAny>,
    high: Py<PyAny>,
    low: Py<PyAny>,
    close: Py<PyAny>,
    volume: Py<PyAny>,
) -> PyResult<Py<PyAny>> {
    let open_vec = extract_array_pyobject(open)?;
    let high_vec = extract_array_pyobject(high)?;
    let low_vec = extract_array_pyobject(low)?;
    let close_vec = extract_array_pyobject(close)?;
    let volume_vec = extract_array_pyobject(volume)?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        None,
    );
    let mut engine = FormulaEngine::new();

    let result = py.detach(|| {
        engine
            .eval_jit(source, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let dict = pyo3::types::PyDict::new(py);

    for (name, value) in &ctx.variables {
        let vec_value = value.to_vec();
        dict.set_item(name.to_string(), vec_value)?;
    }

    dict.set_item("__result__", result.to_vec())?;

    Ok(dict.into())
}

#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_eval_simd(
    py: Python<'_>,
    source: &str,
    open: Py<PyAny>,
    high: Py<PyAny>,
    low: Py<PyAny>,
    close: Py<PyAny>,
    volume: Py<PyAny>,
) -> PyResult<Py<PyAny>> {
    let open_vec = extract_array_pyobject(open)?;
    let high_vec = extract_array_pyobject(high)?;
    let low_vec = extract_array_pyobject(low)?;
    let close_vec = extract_array_pyobject(close)?;
    let volume_vec = extract_array_pyobject(volume)?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        None,
    );
    let mut engine = FormulaEngine::new();

    let result = py.detach(|| {
        engine
            .eval_simd(source, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let dict = pyo3::types::PyDict::new(py);

    for (name, value) in &ctx.variables {
        let vec_value = value.to_vec();
        dict.set_item(name.to_string(), vec_value)?;
    }

    dict.set_item("__result__", result.to_vec())?;

    Ok(dict.into())
}

#[pyfunction]
#[pyo3(signature = (source, open, high, low, close, volume))]
#[cfg(feature = "formula")]
pub fn formula_eval_numpy_zero_copy(
    py: Python<'_>,
    source: &str,
    open: PyReadonlyArray1<'_, f64>,
    high: PyReadonlyArray1<'_, f64>,
    low: PyReadonlyArray1<'_, f64>,
    close: PyReadonlyArray1<'_, f64>,
    volume: PyReadonlyArray1<'_, f64>,
) -> PyResult<Py<PyAny>> {
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
    if close.is_empty()
        || [open, high, low, volume]
            .iter()
            .any(|values| values.len() != close.len())
    {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "all OHLCV arrays must be non-empty and have equal lengths",
        ));
    }

    let mut engine = FormulaEngine::new();
    let formula = engine.compile(source).map_err(formula_error_to_pyerr)?;
    let result = engine
        .eval_zero_copy_inputs(&formula, open, high, low, close, volume, None)
        .map_err(formula_error_to_pyerr)?;
    let dict = pyo3::types::PyDict::new(py);
    dict.set_item("__result__", PyArray1::from_vec(py, result.into_raw_vec()))?;
    Ok(dict.into())
}

#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_eval_zero_copy(
    py: Python<'_>,
    source: &str,
    open: Py<PyAny>,
    high: Py<PyAny>,
    low: Py<PyAny>,
    close: Py<PyAny>,
    volume: Py<PyAny>,
) -> PyResult<Py<PyAny>> {
    // Preserve the legacy list/tuple API, but use the borrowed NumPy path
    // whenever all five inputs are contiguous float64 arrays.
    let direct_result: Option<Array1<f64>> = Python::attach(|py| {
        let open_array = match open.bind(py).extract::<PyReadonlyArray1<'_, f64>>() {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };
        let high_array = match high.bind(py).extract::<PyReadonlyArray1<'_, f64>>() {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };
        let low_array = match low.bind(py).extract::<PyReadonlyArray1<'_, f64>>() {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };
        let close_array = match close.bind(py).extract::<PyReadonlyArray1<'_, f64>>() {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };
        let volume_array = match volume.bind(py).extract::<PyReadonlyArray1<'_, f64>>() {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };

        let open = open_array.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "open must be a contiguous float64 NumPy array: {error}"
            ))
        })?;
        let high = high_array.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "high must be a contiguous float64 NumPy array: {error}"
            ))
        })?;
        let low = low_array.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "low must be a contiguous float64 NumPy array: {error}"
            ))
        })?;
        let close = close_array.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "close must be a contiguous float64 NumPy array: {error}"
            ))
        })?;
        let volume = volume_array.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
                "volume must be a contiguous float64 NumPy array: {error}"
            ))
        })?;

        if close.is_empty()
            || [open, high, low, volume]
                .iter()
                .any(|values| values.len() != close.len())
        {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "all OHLCV arrays must be non-empty and have equal lengths",
            ));
        }

        let mut engine = FormulaEngine::new();
        let formula = engine.compile(source).map_err(formula_error_to_pyerr)?;
        engine
            .eval_zero_copy_inputs(&formula, open, high, low, close, volume, None)
            .map(Some)
            .map_err(formula_error_to_pyerr)
    })?;

    if let Some(result) = direct_result {
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("__result__", PyArray1::from_vec(py, result.into_raw_vec()))?;
        return Ok(dict.into());
    }

    let open_vec = extract_array_pyobject(open)?;
    let high_vec = extract_array_pyobject(high)?;
    let low_vec = extract_array_pyobject(low)?;
    let close_vec = extract_array_pyobject(close)?;
    let volume_vec = extract_array_pyobject(volume)?;

    let mut ctx = FormulaContext::new(
        Array1::from_vec(open_vec),
        Array1::from_vec(high_vec),
        Array1::from_vec(low_vec),
        Array1::from_vec(close_vec),
        Array1::from_vec(volume_vec),
        None,
    );
    let mut engine = FormulaEngine::new();

    let result = py.detach(|| {
        engine
            .eval_zero_copy(source, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let dict = pyo3::types::PyDict::new(py);
    for (name, value) in &ctx.variables {
        dict.set_item(name.to_string(), value.to_vec())?;
    }
    dict.set_item("__result__", result.to_vec())?;
    Ok(dict.into())
}

#[pyfunction]
#[pyo3(signature = (source, open, high, low, close, volume, amount=None))]
#[cfg(feature = "formula")]
pub fn formula_eval_multi(
    py: Python<'_>,
    source: &str,
    open: &Bound<'_, PyAny>,
    high: &Bound<'_, PyAny>,
    low: &Bound<'_, PyAny>,
    close: &Bound<'_, PyAny>,
    volume: &Bound<'_, PyAny>,
    amount: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let open_vec = extract_array_bound(open)?;
    let high_vec = extract_array_bound(high)?;
    let low_vec = extract_array_bound(low)?;
    let close_vec = extract_array_bound(close)?;
    let volume_vec = extract_array_bound(volume)?;
    let amount_vec = amount.map(extract_array_bound).transpose()?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);
    let amount_array = amount_vec.map(Array1::from_vec);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        amount_array,
    );
    let mut engine = FormulaEngine::new();

    let multi_output = py.detach(|| {
        engine
            .eval_multi(source, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let names_list = pyo3::types::PyList::empty(py);
    let values_list = pyo3::types::PyList::empty(py);

    for name in multi_output.names() {
        names_list.append(name.as_str())?;
        if let Some(arr) = multi_output.get(name) {
            values_list.append(arr.to_vec())?;
        }
    }

    let result_dict = pyo3::types::PyDict::new(py);
    result_dict.set_item("names", names_list)?;
    result_dict.set_item("values", values_list)?;
    result_dict.set_item("__result__", multi_output.final_value.to_vec())?;

    Ok(result_dict.into())
}

#[pyfunction]
#[pyo3(signature = (source, open, high, low, close, volume, amount=None))]
#[cfg(feature = "formula")]
pub fn formula_eval_draw(
    py: Python<'_>,
    source: &str,
    open: &Bound<'_, PyAny>,
    high: &Bound<'_, PyAny>,
    low: &Bound<'_, PyAny>,
    close: &Bound<'_, PyAny>,
    volume: &Bound<'_, PyAny>,
    amount: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    use ::finkit::formula::DrawCommand;

    let open_vec = extract_array_bound(open)?;
    let high_vec = extract_array_bound(high)?;
    let low_vec = extract_array_bound(low)?;
    let close_vec = extract_array_bound(close)?;
    let volume_vec = extract_array_bound(volume)?;
    let amount_vec = amount.map(extract_array_bound).transpose()?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);
    let amount_array = amount_vec.map(Array1::from_vec);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        amount_array,
    );
    let mut engine = FormulaEngine::new();

    let _result = py.detach(|| {
        engine
            .eval(source, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let draw_commands = ctx.draw_commands.borrow();
    let draw_list = pyo3::types::PyList::empty(py);
    for cmd in &draw_commands.commands {
        let cmd_dict = pyo3::types::PyDict::new(py);
        match cmd {
            DrawCommand::Text {
                condition: _,
                price: _,
                text,
                color,
            } => {
                cmd_dict.set_item("type", "Text")?;
                cmd_dict.set_item("text", text.as_str())?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::Icon {
                condition: _,
                price: _,
                icon_type,
                color,
            } => {
                cmd_dict.set_item("type", "Icon")?;
                cmd_dict.set_item("iconType", *icon_type)?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::StickLine {
                condition: _,
                price1: _,
                price2: _,
                width,
                empty,
                color,
            } => {
                cmd_dict.set_item("type", "StickLine")?;
                cmd_dict.set_item("width", *width)?;
                cmd_dict.set_item("empty", *empty)?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::Line {
                cond1: _,
                price1: _,
                cond2: _,
                price2: _,
                expand,
                color,
            } => {
                cmd_dict.set_item("type", "Line")?;
                cmd_dict.set_item("expand", *expand)?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::Band {
                val1: _,
                color1,
                val2: _,
                color2,
            } => {
                cmd_dict.set_item("type", "Band")?;
                cmd_dict.set_item("color1", color1.as_str())?;
                cmd_dict.set_item("color2", color2.as_str())?;
            }
            DrawCommand::KLine { .. } => {
                cmd_dict.set_item("type", "KLine")?;
            }
            DrawCommand::Rect {
                x1: _,
                y1: _,
                x2: _,
                y2: _,
                color,
            } => {
                cmd_dict.set_item("type", "Rect")?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::FillRgn {
                cond: _,
                price1: _,
                price2: _,
                color,
            } => {
                cmd_dict.set_item("type", "FillRgn")?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::PartLine {
                cond: _,
                price: _,
                color,
            } => {
                cmd_dict.set_item("type", "PartLine")?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::PolyLine {
                cond: _,
                price: _,
                color,
            } => {
                cmd_dict.set_item("type", "PolyLine")?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::Background { cond: _, color } => {
                cmd_dict.set_item("type", "Background")?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::SlopeLine {
                cond1: _,
                price1: _,
                cond2: _,
                price2: _,
                color,
            } => {
                cmd_dict.set_item("type", "SlopeLine")?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::TextFix { x, y, text, color } => {
                cmd_dict.set_item("type", "TextFix")?;
                cmd_dict.set_item("x", *x)?;
                cmd_dict.set_item("y", *y)?;
                cmd_dict.set_item("text", text.as_str())?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::Number {
                condition: _,
                price: _,
                number: _,
                precision,
                color,
            } => {
                cmd_dict.set_item("type", "Number")?;
                cmd_dict.set_item("precision", *precision)?;
                cmd_dict.set_item("color", color.as_str())?;
            }
            DrawCommand::VertLine {
                condition: _,
                color,
            } => {
                cmd_dict.set_item("type", "VertLine")?;
                cmd_dict.set_item("color", color.as_str())?;
            }
        }
        draw_list.append(cmd_dict)?;
    }

    let result_dict = pyo3::types::PyDict::new(py);
    for (name, value) in &ctx.variables {
        let vec_value = value.to_vec();
        result_dict.set_item(name.to_string(), vec_value)?;
    }
    result_dict.set_item("drawCommands", draw_list)?;

    Ok(result_dict.into())
}

#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_eval_debug(
    py: Python<'_>,
    source: &str,
    open: Py<PyAny>,
    high: Py<PyAny>,
    low: Py<PyAny>,
    close: Py<PyAny>,
    volume: Py<PyAny>,
) -> PyResult<Py<PyAny>> {
    let open_vec = extract_array_pyobject(open)?;
    let high_vec = extract_array_pyobject(high)?;
    let low_vec = extract_array_pyobject(low)?;
    let close_vec = extract_array_pyobject(close)?;
    let volume_vec = extract_array_pyobject(volume)?;

    let open_array = Array1::from_vec(open_vec);
    let high_array = Array1::from_vec(high_vec);
    let low_array = Array1::from_vec(low_vec);
    let close_array = Array1::from_vec(close_vec);
    let volume_array = Array1::from_vec(volume_vec);

    let mut ctx = FormulaContext::new(
        open_array,
        high_array,
        low_array,
        close_array,
        volume_array,
        None,
    );
    let mut engine = FormulaEngine::new();

    let (final_result, debugger) = py.detach(|| {
        engine
            .eval_with_debug(source, &mut ctx)
            .map_err(formula_error_to_pyerr)
    })?;

    let result_dict = pyo3::types::PyDict::new(py);

    for (name, value) in &ctx.variables {
        let vec_value = value.to_vec();
        result_dict.set_item(name.to_string(), vec_value)?;
    }

    result_dict.set_item("__result__", final_result.to_vec())?;

    let debug_dict = pyo3::types::PyDict::new(py);
    let event_list = pyo3::types::PyList::empty(py);
    for event in debugger.get_events() {
        event_list.append(format!("{event:?}"))?;
    }
    debug_dict.set_item("events", event_list)?;

    let output_dict = pyo3::types::PyDict::new(py);
    output_dict.set_item("result", result_dict)?;
    output_dict.set_item("debug", debug_dict)?;

    Ok(output_dict.into())
}

#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_get_template(py: Python<'_>, name: &str) -> PyResult<Py<PyAny>> {
    use ::finkit::formula::FormulaEngine;

    let dict = pyo3::types::PyDict::new(py);
    let engine = FormulaEngine::new();

    match engine.get_template(name) {
        Some(template) => {
            dict.set_item("name", template.name.as_str())?;
            dict.set_item("category", format!("{:?}", template.category))?;
            dict.set_item("description", template.description.as_str())?;
            dict.set_item("formula", template.source.as_str())?;

            let params_dict = pyo3::types::PyDict::new(py);
            for (param_name, default, min, max) in &template.parameters {
                let param_info = pyo3::types::PyDict::new(py);
                param_info.set_item("default", default)?;
                param_info.set_item("min", min)?;
                param_info.set_item("max", max)?;
                params_dict.set_item(param_name.as_str(), param_info)?;
            }
            dict.set_item("parameters", params_dict)?;
        }
        None => {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Template '{}' not found",
                name
            )));
        }
    }

    Ok(dict.into())
}

#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_search_templates(py: Python<'_>, keyword: &str) -> PyResult<Py<PyAny>> {
    use ::finkit::formula::FormulaEngine;

    let engine = FormulaEngine::new();
    let templates = engine.search_templates(keyword);
    let list = pyo3::types::PyList::empty(py);

    for template in templates {
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("name", template.name.as_str())?;
        dict.set_item("category", format!("{:?}", template.category))?;
        dict.set_item("description", template.description.as_str())?;
        dict.set_item("formula", template.source.as_str())?;

        let params_dict = pyo3::types::PyDict::new(py);
        for (param_name, default, min, max) in &template.parameters {
            let param_info = pyo3::types::PyDict::new(py);
            param_info.set_item("default", default)?;
            param_info.set_item("min", min)?;
            param_info.set_item("max", max)?;
            params_dict.set_item(param_name.as_str(), param_info)?;
        }
        dict.set_item("parameters", params_dict)?;

        list.append(dict)?;
    }

    Ok(list.into())
}

#[pyfunction]
#[cfg(feature = "formula")]
pub fn formula_list_categories(py: Python<'_>) -> PyResult<Py<PyAny>> {
    use ::finkit::formula::templates::FormulaTemplates;

    let templates = FormulaTemplates::new();
    let list = pyo3::types::PyList::empty(py);

    for category in FormulaTemplates::categories() {
        let count = templates.get_by_category(&category).len();
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("category", format!("{category:?}"))?;
        dict.set_item("count", count)?;
        list.append(dict)?;
    }

    Ok(list.into())
}
