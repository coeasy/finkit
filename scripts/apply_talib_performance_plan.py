#!/usr/bin/env python3
"""Apply the Finkit-vs-TA-Lib performance plan source migrations.

This script is deliberately idempotent.  It updates the Python binding SSOT,
regenerates the Python indicator surface, optimizes every numeric PyO3 function
to return NumPy arrays directly, removes the package-level list->ndarray hot
path, and makes batch computation borrow NumPy inputs / return NumPy outputs.

It is kept in-tree so future registry regenerations cannot silently reintroduce
the Python-list materialization regression.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

from optimize_python_bindings import optimize_file

ROOT = Path(__file__).resolve().parents[1]


def _write(path: Path, text: str) -> None:
    path.write_text(text, encoding="utf-8")


def _replace_once(text: str, old: str, new: str, *, label: str) -> str:
    count = text.count(old)
    if count == 0:
        if new in text:
            return text
        raise RuntimeError(f"{label}: expected source fragment not found")
    if count != 1:
        raise RuntimeError(f"{label}: expected one source fragment, found {count}")
    return text.replace(old, new, 1)


def patch_sync_bindings() -> None:
    path = ROOT / "scripts" / "sync_bindings.py"
    text = path.read_text(encoding="utf-8")

    import_line = "from optimize_python_bindings import optimize_source as optimize_python_source\n"
    if import_line not in text:
        text = _replace_once(
            text,
            "from pathlib import Path\n",
            "from pathlib import Path\n\n" + import_line,
            label="sync_bindings import",
        )

    marker = "        text = emit_generated(lang, inds)\n"
    optimized = (
        "        text = emit_generated(lang, inds)\n"
        "        if lang == \"python\":\n"
        "            text, optimized_count = optimize_python_source(text)\n"
        "            print(f\"[gen/python] NumPy-direct wrappers: {optimized_count}\")\n"
    )
    if optimized not in text:
        text = _replace_once(text, marker, optimized, label="sync_bindings generate hook")

    check_marker = "            body_now = extracted[nm][\"body\"]\n"
    check_replacement = (
        "            body_now = extracted[nm][\"body\"]\n"
        "            if lang == \"python\":\n"
        "                impl_name = f\"vec_{nm}_impl\"\n"
        "                if impl_name in extracted:\n"
        "                    body_now = extracted[impl_name][\"body\"].replace(\n"
        "                        f\"fn {impl_name}\", f\"fn {nm}\", 1\n"
        "                    )\n"
    )
    if check_replacement not in text:
        text = _replace_once(text, check_marker, check_replacement, label="sync_bindings check hook")

    _write(path, text)


def _patch_bbands(body: str) -> str:
    body = body.replace(
        "#[pyo3(signature = (close, timeperiod=5, nbdevup=2.0, nbdevdn=2.0))]",
        "#[pyo3(signature = (close, timeperiod=5, nbdevup=2.0, nbdevdn=2.0, matype=0))]",
    )
    if "matype: i32" not in body:
        body = body.replace("    nbdevdn: f64,\n) ->", "    nbdevdn: f64,\n    matype: i32,\n) ->")
    guard = (
        "    if matype != 0 {\n"
        "        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(\n"
        "            \"bollinger_bands currently supports TA-Lib matype=0 (SMA) only\",\n"
        "        ));\n"
        "    }\n"
    )
    if guard not in body:
        body = body.replace("{\n    let close = close", "{\n" + guard + "    let close = close", 1)
    return body


def _patch_stoch(body: str) -> str:
    body = body.replace(
        "#[pyo3(signature = (high, low, close, fastk_period=5, slowk_period=3, slowd_period=3))]",
        "#[pyo3(signature = (high, low, close, fastk_period=5, slowk_period=3, slowk_matype=0, slowd_period=3, slowd_matype=0))]",
    )
    old_params = (
        "    fastk_period: usize,\n"
        "    slowk_period: usize,\n"
        "    slowd_period: usize,\n"
    )
    new_params = (
        "    fastk_period: usize,\n"
        "    slowk_period: usize,\n"
        "    slowk_matype: i32,\n"
        "    slowd_period: usize,\n"
        "    slowd_matype: i32,\n"
    )
    if "slowk_matype: i32" not in body:
        body = body.replace(old_params, new_params, 1)
    guard = (
        "    if slowk_matype != 0 || slowd_matype != 0 {\n"
        "        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(\n"
        "            \"stoch currently supports TA-Lib slowk_matype=0 and slowd_matype=0 only\",\n"
        "        ));\n"
        "    }\n"
    )
    if guard not in body:
        body = body.replace("{\n    let high = high", "{\n" + guard + "    let high = high", 1)
    return body


def _patch_sar(body: str) -> str:
    body = body.replace(
        ") -> PyResult<(Vec<f64>, Vec<f64>)> {",
        ") -> PyResult<Vec<f64>> {",
        1,
    )
    body = body.replace(
        ".map(|res| (res.sar.into_raw_vec(), res.af.into_raw_vec()))",
        ".map(|res| res.sar.into_raw_vec())",
        1,
    )
    return body


def patch_python_registry_contract() -> None:
    path = ROOT / "docs" / "indicator_registry.json"
    reg = json.loads(path.read_text(encoding="utf-8"))
    changed = 0
    handlers = {
        "ta_bbands": _patch_bbands,
        "ta_stoch": _patch_stoch,
        "ta_sar": _patch_sar,
    }
    found: set[str] = set()
    for indicator in reg.get("indicators", []):
        ffi = indicator.get("ffi", {})
        c_name = ffi.get("c_name")
        handler = handlers.get(c_name)
        if handler is None:
            continue
        body = ffi.get("bodies", {}).get("python")
        if not body:
            raise RuntimeError(f"registry entry {c_name} has no Python body")
        new_body = handler(body)
        found.add(c_name)
        if new_body != body:
            ffi["bodies"]["python"] = new_body
            changed += 1
    missing = set(handlers) - found
    if missing:
        raise RuntimeError(f"registry entries missing: {sorted(missing)}")
    path.write_text(json.dumps(reg, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"registry Python API contract entries updated: {changed}")


def regenerate_python_binding() -> None:
    subprocess.run(
        [sys.executable, str(ROOT / "scripts" / "sync_bindings.py"), "--lang", "python", "--generate"],
        cwd=ROOT,
        check=True,
    )


def patch_package_init() -> None:
    path = ROOT / "ffi" / "python-binding" / "finkit" / "__init__.py"
    text = path.read_text(encoding="utf-8")
    text = text.replace("\nimport numpy as np\n", "\n")

    # The native extension now returns ndarrays directly. Remove the recursive
    # list conversion wrapper from every hot-path API call.
    start = text.find("\ndef _as_numpy_result(")
    end = text.find("\n\nfor _name in _native_all:", start)
    if start >= 0 and end >= 0:
        text = text[:start] + text[end:]
    text = text.replace(
        "        globals()[_name] = _as_numpy_result(_name, wrapped)",
        "        globals()[_name] = wrapped",
    )

    alias_block = (
        "\n# Stable TA-Lib-compatible public aliases.  The native Rust functions use\n"
        "# descriptive internal names; keep both spellings public.\n"
        "_PUBLIC_ALIASES = {\"stddev\": \"std_dev\", \"correl\": \"correlation\"}\n"
        "for _alias, _target in _PUBLIC_ALIASES.items():\n"
        "    if _target in globals():\n"
        "        globals()[_alias] = globals()[_target]\n"
    )
    insertion = "\n\ndef register_accessor():"
    if alias_block not in text:
        if insertion not in text:
            raise RuntimeError("package __init__: register_accessor marker missing")
        text = text.replace(insertion, alias_block + insertion, 1)

    old_all = "__all__ = list(_native_all) + ["
    new_all = "__all__ = list(dict.fromkeys(list(_native_all) + list(_PUBLIC_ALIASES))) + ["
    if old_all in text:
        text = text.replace(old_all, new_all, 1)

    _write(path, text)


def patch_batch_compute() -> None:
    path = ROOT / "ffi" / "python-binding" / "src" / "lib.rs"
    text = path.read_text(encoding="utf-8")

    old = (
        "    let open_vec: Option<Vec<f64>> = open.as_ref().map(|arr| arr.as_array().to_vec());\n"
        "    let high_vec: Option<Vec<f64>> = high.as_ref().map(|arr| arr.as_array().to_vec());\n"
        "    let low_vec: Option<Vec<f64>> = low.as_ref().map(|arr| arr.as_array().to_vec());\n"
        "    let volume_vec: Option<Vec<f64>> = volume.as_ref().map(|arr| arr.as_array().to_vec());\n"
        "    let secondary_vec: Option<Vec<f64>> = secondary.as_ref().map(|arr| arr.as_array().to_vec());\n"
    )
    new = (
        "    let open_slice = open.as_ref().map(|arr| arr.as_slice().map_err(|e| {\n"
        "        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(\"open must be contiguous float64: {e}\"))\n"
        "    })).transpose()?;\n"
        "    let high_slice = high.as_ref().map(|arr| arr.as_slice().map_err(|e| {\n"
        "        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(\"high must be contiguous float64: {e}\"))\n"
        "    })).transpose()?;\n"
        "    let low_slice = low.as_ref().map(|arr| arr.as_slice().map_err(|e| {\n"
        "        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(\"low must be contiguous float64: {e}\"))\n"
        "    })).transpose()?;\n"
        "    let volume_slice = volume.as_ref().map(|arr| arr.as_slice().map_err(|e| {\n"
        "        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(\"volume must be contiguous float64: {e}\"))\n"
        "    })).transpose()?;\n"
        "    let secondary_slice = secondary.as_ref().map(|arr| arr.as_slice().map_err(|e| {\n"
        "        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(\"secondary must be contiguous float64: {e}\"))\n"
        "    })).transpose()?;\n"
    )
    if old in text:
        text = text.replace(old, new, 1)

    text = text.replace(
        "            open_vec.as_deref(),\n            high_vec.as_deref(),\n            low_vec.as_deref(),\n            close_slice,\n            volume_vec.as_deref(),\n            secondary_vec.as_deref(),",
        "            open_slice,\n            high_slice,\n            low_slice,\n            close_slice,\n            volume_slice,\n            secondary_slice,",
        1,
    )

    start = text.find("fn compute_indicators<'py>(")
    end = text.find("\n/// Result type for indicator computation.", start)
    if start < 0 or end < 0:
        raise RuntimeError("compute_indicators function not found")
    segment = text[start:end]
    segment = segment.replace("dict.set_item(key, arr)?;", "dict.set_item(key, PyArray1::from_vec(py, arr))?;")
    for suffix, value in (("_0", "a"), ("_1", "b"), ("_2", "c"), ("_3", "d")):
        segment = segment.replace(
            f'dict.set_item(format!("{{}}{suffix}", key), {value})?;',
            f'dict.set_item(format!("{{}}{suffix}", key), PyArray1::from_vec(py, {value}))?;',
        )
    text = text[:start] + segment + text[end:]
    _write(path, text)


def optimize_all_numeric_pyfunctions() -> None:
    optimize_file(ROOT / "ffi" / "python-binding" / "src" / "generated.rs")
    optimize_file(ROOT / "ffi" / "python-binding" / "src" / "lib.rs")


def main() -> int:
    patch_sync_bindings()
    patch_python_registry_contract()
    regenerate_python_binding()
    patch_package_init()
    patch_batch_compute()
    optimize_all_numeric_pyfunctions()

    # Verify generator/check stability after optimization.  This catches a
    # future regression where a registry regeneration would restore Vec-returning
    # public functions.
    subprocess.run(
        [sys.executable, str(ROOT / "scripts" / "sync_bindings.py"), "--lang", "python", "--check"],
        cwd=ROOT,
        check=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
