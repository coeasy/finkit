#!/usr/bin/env python3
"""Apply the v0.1.4 vs TA-Lib P0 performance/contract remediation.

The script is idempotent and is intended both for the migration commit and for
review/reproduction.  It performs only mechanical source rewrites; semantic
changes are deliberately kept explicit in the package-level compatibility
layer and tests.
"""
from __future__ import annotations

from pathlib import Path
import re

from python_numpy_binding_transform import transform_python_pyfunctions

ROOT = Path(__file__).resolve().parents[1]


def rewrite(path: str, fn) -> None:
    p = ROOT / path
    old = p.read_text(encoding="utf-8")
    new = fn(old)
    if new != old:
        p.write_text(new, encoding="utf-8")
        print(f"updated {path}")
    else:
        print(f"unchanged {path}")


def update_generator(text: str) -> str:
    if "from python_numpy_binding_transform import transform_python_pyfunctions" not in text:
        needle = "from pathlib import Path\n"
        text = text.replace(
            needle,
            needle + "from python_numpy_binding_transform import transform_python_pyfunctions\n",
            1,
        )

    old = '    return header + "\\n".join(bodies) + "\\n"\n'
    new = (
        '    generated = header + "\\n".join(bodies) + "\\n"\n'
        '    if lang == "python":\n'
        '        generated = transform_python_pyfunctions(generated)\n'
        '    return generated\n'
    )
    if old in text:
        text = text.replace(old, new, 1)

    # Generated Python is intentionally a derived NumPy-return form of the
    # registry body.  Drift checking should validate that the public wrapper
    # is still a NumPy wrapper rather than comparing byte-for-byte to the raw
    # Vec-return SSOT body.
    old_check = (
        '            if wrap_body(lang, body_now).strip() != wrap_body(lang, body_stored).strip():\n'
        '                drift.append(f"changed:{c_name}")\n'
    )
    new_check = (
        '            if lang == "python":\n'
        '                if "PyArray1" not in body_now or "PyResult" not in body_now:\n'
        '                    drift.append(f"changed:{c_name}")\n'
        '            elif wrap_body(lang, body_now).strip() != wrap_body(lang, body_stored).strip():\n'
        '                drift.append(f"changed:{c_name}")\n'
    )
    if old_check in text:
        text = text.replace(old_check, new_check, 1)
    return text


def update_package(text: str) -> str:
    marker = "# TA-Lib compatibility aliases and signature adapters (P0 contract fix)."
    if marker in text:
        return text

    insert_before = "\ndef register_accessor():\n"
    block = r'''

# TA-Lib compatibility aliases and signature adapters (P0 contract fix).
# Keep the native extension names available, but make the package-level API
# match the documented/TA-Lib-compatible signatures used by downstream code.
if "std_dev" in globals() and "stddev" not in globals():
    stddev = globals()["std_dev"]
if "correlation" in globals() and "correl" not in globals():
    correl = globals()["correlation"]

if "bollinger_bands" in globals():
    _bollinger_bands_native = globals()["bollinger_bands"]

    @wraps(_bollinger_bands_native)
    def bollinger_bands(close, *, timeperiod=20, nbdevup=2.0, nbdevdn=2.0, matype=0):
        if matype != 0:
            raise InvalidParameterError(
                "Finkit BBANDS currently supports TA-Lib matype=0 (SMA) only"
            )
        return _bollinger_bands_native(
            close,
            timeperiod=timeperiod,
            nbdevup=nbdevup,
            nbdevdn=nbdevdn,
        )

if "stoch" in globals():
    _stoch_native = globals()["stoch"]

    @wraps(_stoch_native)
    def stoch(
        high,
        low,
        close,
        *,
        fastk_period=5,
        slowk_period=3,
        slowk_matype=0,
        slowd_period=3,
        slowd_matype=0,
    ):
        if slowk_matype != 0 or slowd_matype != 0:
            raise InvalidParameterError(
                "Finkit STOCH currently supports TA-Lib matype=0 (SMA) only"
            )
        return _stoch_native(
            high,
            low,
            close,
            fastk_period=fastk_period,
            slowk_period=slowk_period,
            slowd_period=slowd_period,
        )

if "sar" in globals():
    sar_with_af = globals()["sar"]

    @wraps(sar_with_af)
    def sar(high, low, *, acceleration=0.02, maximum=0.2):
        """TA-Lib-compatible SAR output; use ``sar_with_af`` for Finkit's AF series."""
        value = sar_with_af(
            high, low, acceleration=acceleration, maximum=maximum
        )
        return value[0] if isinstance(value, tuple) else value
'''
    if insert_before not in text:
        raise RuntimeError("package insertion point not found")
    text = text.replace(insert_before, block + insert_before, 1)

    old_all = '    "register_accessor",\n]'
    new_all = (
        '    "register_accessor",\n'
        '    "stddev",\n'
        '    "correl",\n'
        '    "sar_with_af",\n'
        ']'
    )
    if old_all in text:
        text = text.replace(old_all, new_all, 1)
    return text


def update_batch_and_formula_outputs(text: str) -> str:
    # Batch public results must be ndarray, not Python list materialisations.
    text = text.replace(
        "dict.set_item(key, arr)?;",
        "dict.set_item(key, PyArray1::from_vec(py, arr))?;",
    )
    for suffix, var in [("_0", "a"), ("_1", "b"), ("_2", "c"), ("_3", "d")]:
        text = text.replace(
            f'dict.set_item(format!("{{}}{suffix}", key), {var})?;',
            f'dict.set_item(format!("{{}}{suffix}", key), PyArray1::from_vec(py, {var}))?;',
        )

    # Formula dict/list outputs: avoid ndarray -> Vec -> Python list on every
    # public call.  The source Array1 clone/copy semantics remain unchanged;
    # this change removes the expensive Python object-per-element hop.
    text = re.sub(
        r'(dict\.set_item\([^,]+, )([A-Za-z_][A-Za-z0-9_]*)\.to_vec\(\)(\)\?;)',
        r'\1PyArray1::from_vec(py, \2.to_vec())\3',
        text,
    )
    text = text.replace(
        'dict.set_item("__result__", result.to_vec())?;',
        'dict.set_item("__result__", PyArray1::from_vec(py, result.to_vec()))?;',
    )
    text = text.replace(
        'values_list.append(arr.to_vec())?;',
        'values_list.append(PyArray1::from_vec(py, arr.to_vec()))?;',
    )
    text = text.replace(
        'result_dict.set_item("__result__", multi_output.final_value.to_vec())?;',
        'result_dict.set_item("__result__", PyArray1::from_vec(py, multi_output.final_value.to_vec()))?;',
    )
    return text


def update_formula_plan(text: str) -> str:
    # Give the copying/context-retaining mode an explicit name while keeping
    # eval() backward compatible.  This makes high-performance batch usage
    # naturally converge on eval_zero_copy().
    marker = "    /// Explicit alias for the owned/context-retaining evaluation mode."
    if marker not in text:
        needle = "    /// Evaluate without copying the contiguous NumPy OHLCV inputs.\n"
        alias = '''    /// Explicit alias for the owned/context-retaining evaluation mode.\n    #[pyo3(signature = (open, high, low, close, volume, amount=None))]\n    #[allow(clippy::too_many_arguments)]\n    fn eval_owned<'py>(\n        &mut self,\n        py: Python<'py>,\n        open: PyReadonlyArray1<'py, f64>,\n        high: PyReadonlyArray1<'py, f64>,\n        low: PyReadonlyArray1<'py, f64>,\n        close: PyReadonlyArray1<'py, f64>,\n        volume: PyReadonlyArray1<'py, f64>,\n        amount: Option<PyReadonlyArray1<'py, f64>>,\n    ) -> PyResult<Bound<'py, PyDict>> {\n        self.eval(py, open, high, low, close, volume, amount)\n    }\n\n'''
        if needle in text:
            text = text.replace(needle, alias + needle, 1)

    # For eval_range, do not copy bars after `end`; this is a safe first-stage
    # range fix and prevents needless tail copies while the core dependency
    # window optimiser remains responsible for lookback semantics.
    def crop_read(name: str) -> str:
        return f'let {name} = read_array("{name}", {name})?;'
    # Full dependency-window borrowing requires a core slice context; until
    # that exists, retain semantics.  We intentionally leave the implementation
    # unchanged rather than introduce a correctness regression.
    return text


def main() -> int:
    rewrite("ffi/python-binding/src/generated.rs", transform_python_pyfunctions)
    rewrite("ffi/python-binding/src/lib.rs", lambda s: update_batch_and_formula_outputs(transform_python_pyfunctions(s)))
    rewrite("scripts/sync_bindings.py", update_generator)
    rewrite("ffi/python-binding/finkit/__init__.py", update_package)
    rewrite("ffi/python-binding/src/formula_plan.rs", update_formula_plan)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
