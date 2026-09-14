#!/usr/bin/env python3
"""Remove default formula variable cloning from the Python hot path.

`CompiledFormula.eval()` retains an owned FormulaContext for streaming use.  It
used to additionally clone every named variable into a new NumPy array on every
call.  Production batch callers usually only consume ``__result__``; named
variables are now opt-in through ``return_variables=True``.
"""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old in text:
        return text.replace(old, new, 1)
    if new in text:
        return text
    raise RuntimeError(f"{label}: source fragment not found")


def patch_formula_plan() -> None:
    path = ROOT / "ffi/python-binding/src/formula_plan.rs"
    text = path.read_text(encoding="utf-8")

    old = '''fn result_dict<'py>(
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
}'''
    new = '''fn result_dict<'py>(
    py: Python<'py>,
    context: &FormulaContext,
    result: Array1<f64>,
    return_variables: bool,
) -> PyResult<Bound<'py, PyDict>> {
    let output = PyDict::new(py);
    if return_variables {
        for (name, value) in &context.variables {
            if name.as_ref().starts_with("_CSE") {
                continue;
            }
            // Variables remain owned by the retained streaming context, so an
            // explicit variable request necessarily materializes a copy.  The
            // default result-only path avoids this O(vars * bars) cost.
            output.set_item(
                name.as_ref(),
                PyArray1::from_vec(py, value.to_vec()),
            )?;
        }
    }
    output.set_item("__result__", PyArray1::from_vec(py, result.into_raw_vec()))?;
    Ok(output)
}'''
    text = replace_once(text, old, new, "result_dict")

    text = replace_once(
        text,
        '#[pyo3(signature = (open, high, low, close, volume, amount=None))]\n    #[allow(clippy::too_many_arguments)]\n    fn eval<\'py>(',
        '#[pyo3(signature = (open, high, low, close, volume, amount=None, return_variables=false))]\n    #[allow(clippy::too_many_arguments)]\n    fn eval<\'py>(',
        "eval signature",
    )
    old_param = '''        volume: PyReadonlyArray1<'py, f64>,
        amount: Option<PyReadonlyArray1<'py, f64>>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let open = read_array("open", open)?;'''
    new_param = '''        volume: PyReadonlyArray1<'py, f64>,
        amount: Option<PyReadonlyArray1<'py, f64>>,
        return_variables: bool,
    ) -> PyResult<Bound<'py, PyDict>> {
        let open = read_array("open", open)?;'''
    text = replace_once(text, old_param, new_param, "eval return_variables parameter")
    text = replace_once(
        text,
        '        result_dict(py, self.stream_context.as_ref().unwrap(), result)',
        '        result_dict(\n            py,\n            self.stream_context.as_ref().unwrap(),\n            result,\n            return_variables,\n        )',
        "eval result_dict call",
    )
    path.write_text(text, encoding="utf-8")


def patch_stub() -> None:
    path = ROOT / "ffi/python-binding/finkit/__init__.pyi"
    text = path.read_text(encoding="utf-8")
    old = '''        volume: ArrayLike,
        amount: Optional[ArrayLike] = ...,
    ) -> Dict[str, Array1D]:
        """Evaluate the compiled formula and return NumPy arrays."""
        ...'''
    new = '''        volume: ArrayLike,
        amount: Optional[ArrayLike] = ...,
        return_variables: bool = ...,
    ) -> Dict[str, Array1D]:
        """Evaluate and retain an owned stream context; named variables are opt-in."""
        ...'''
    if old in text:
        text = text.replace(old, new, 1)
    elif "return_variables: bool" not in text:
        # The runtime-performance migration may not have expanded the stub yet;
        # it runs before this script in CI, so fail loudly if its shape changes.
        raise RuntimeError("CompiledFormula.eval stub not found")
    path.write_text(text, encoding="utf-8")


def patch_tests() -> None:
    path = ROOT / "ffi/python-binding/tests/test_talib_public_contract.py"
    text = path.read_text(encoding="utf-8")
    test = '''

def test_compiled_formula_named_outputs_are_opt_in():
    open_, high, low, close, volume = sample()
    plan = ta.CompiledFormula("MA5 := MA(CLOSE,5); MA5")

    fast = plan.eval(open_, high, low, close, volume)
    assert set(fast) == {"__result__"}
    assert type(fast["__result__"]) is np.ndarray

    expanded = plan.eval(
        open_, high, low, close, volume, return_variables=True
    )
    assert "MA5" in expanded
    assert "__result__" in expanded
    assert type(expanded["MA5"]) is np.ndarray
'''
    if "test_compiled_formula_named_outputs_are_opt_in" not in text:
        text += test
    path.write_text(text, encoding="utf-8")


def main() -> int:
    patch_formula_plan()
    patch_stub()
    patch_tests()
    print("formula result-only fast path applied")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
