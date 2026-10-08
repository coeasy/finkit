//! Composite indicator evaluation and visualisation glue.

use super::*;

pub fn convert_vis_error(e: VisualizationError) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e))
}

#[pyfunction]
#[pyo3(signature = (close, definitions, outputs=None, open=None, high=None, low=None, volume=None))]
pub fn compute_composite<'py>(
    py: Python<'py>,
    close: PyReadonlyArray1<'_, f64>,
    definitions: Vec<(String, String, Vec<String>, Vec<f64>)>,
    outputs: Option<Vec<String>>,
    open: Option<PyReadonlyArray1<'_, f64>>,
    high: Option<PyReadonlyArray1<'_, f64>>,
    low: Option<PyReadonlyArray1<'_, f64>>,
    volume: Option<PyReadonlyArray1<'_, f64>>,
) -> PyResult<Bound<'py, pyo3::types::PyDict>> {
    let close = close
        .as_slice()
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}")))?
        .to_vec();
    let open = optional_array_to_vec(open)?;
    let high = optional_array_to_vec(high)?;
    let low = optional_array_to_vec(low)?;
    let volume = optional_array_to_vec(volume)?;
    let names: std::collections::BTreeSet<String> =
        definitions.iter().map(|item| item.0.clone()).collect();
    if names.len() != definitions.len() {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "composite definition names must be unique",
        ));
    }
    let definitions = definitions
        .into_iter()
        .map(|(name, function, inputs, params)| {
            let expression_inputs = inputs
                .into_iter()
                .map(|input| composite_input_expression(&input, &names))
                .collect::<PyResult<Vec<_>>>()?;
            let expression = match function.to_ascii_lowercase().as_str() {
                "add" => CompositeExpr::Op {
                    op: CompositeOp::Add,
                    inputs: expression_inputs,
                },
                "sub" => CompositeExpr::Op {
                    op: CompositeOp::Sub,
                    inputs: expression_inputs,
                },
                "mul" => CompositeExpr::Op {
                    op: CompositeOp::Mul,
                    inputs: expression_inputs,
                },
                "div" => CompositeExpr::Op {
                    op: CompositeOp::Div,
                    inputs: expression_inputs,
                },
                "min" => CompositeExpr::Op {
                    op: CompositeOp::Min,
                    inputs: expression_inputs,
                },
                "max" => CompositeExpr::Op {
                    op: CompositeOp::Max,
                    inputs: expression_inputs,
                },
                "weighted_average" | "weightedaverage" => {
                    CompositeExpr::call("weighted_average", expression_inputs, params)
                }
                _ => CompositeExpr::call(function, expression_inputs, params),
            };
            Ok(CompositeDefinition::new(name, expression))
        })
        .collect::<PyResult<Vec<_>>>()?;
    let output_names = outputs.unwrap_or_else(|| {
        definitions
            .iter()
            .map(|definition| definition.name.clone())
            .collect()
    });
    let output_refs: Vec<&str> = output_names.iter().map(String::as_str).collect();

    let result = py
        .detach(|| -> Result<_, String> {
            let mut context = FactorContext::new();
            context
                .insert("close", close)
                .map_err(|error| error.to_string())?;
            if let Some(values) = open {
                context
                    .insert("open", values)
                    .map_err(|error| error.to_string())?;
            }
            if let Some(values) = high {
                context
                    .insert("high", values)
                    .map_err(|error| error.to_string())?;
            }
            if let Some(values) = low {
                context
                    .insert("low", values)
                    .map_err(|error| error.to_string())?;
            }
            if let Some(values) = volume {
                context
                    .insert("volume", values)
                    .map_err(|error| error.to_string())?;
            }
            CompositeEngine::new()
                .evaluate(&definitions, &output_refs, &context)
                .map_err(|error| error.to_string())
        })
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyValueError, _>(error))?;

    let dict = pyo3::types::PyDict::new(py);
    for (name, values) in result {
        dict.set_item(name, values)?;
    }
    Ok(dict)
}

pub fn optional_array_to_vec(
    array: Option<PyReadonlyArray1<'_, f64>>,
) -> PyResult<Option<Vec<f64>>> {
    array
        .map(|array| {
            array
                .as_slice()
                .map(|values| values.to_vec())
                .map_err(|error| {
                    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{error}"))
                })
        })
        .transpose()
}

pub fn composite_input_expression(
    input: &str,
    definition_names: &std::collections::BTreeSet<String>,
) -> PyResult<CompositeExpr> {
    if let Some(value) = input.strip_prefix("const:") {
        let value = value.parse::<f64>().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "invalid composite constant: {input}"
            ))
        })?;
        return Ok(CompositeExpr::Constant(value));
    }
    if definition_names.contains(input) {
        Ok(CompositeExpr::reference(input))
    } else {
        Ok(CompositeExpr::series(input))
    }
}
