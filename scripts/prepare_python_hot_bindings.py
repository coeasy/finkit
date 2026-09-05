#!/usr/bin/env python3
"""Prepare the canonical NumPy-direct Python binding surface for wheel builds.

This is a permanent build step, not a migration helper. It keeps the canonical
indicator registry unchanged, builds the transient Python SSOT overlay, regenerates
the generated PyO3 surface, and rewrites numeric PyO3 functions to return NumPy
arrays directly instead of materializing Python lists first.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from optimize_python_bindings import optimize_file

ROOT = Path(__file__).resolve().parents[1]
GENERATED = ROOT / "ffi" / "python-binding" / "src" / "generated.rs"
LIB = ROOT / "ffi" / "python-binding" / "src" / "lib.rs"


def run(*args: str) -> None:
    env = os.environ.copy()
    # Windows hosted runners default redirected stdout to a legacy code page.
    # Force one UTF-8 process contract so SSOT/generator diagnostics are
    # identical on Linux, macOS, and Windows and cannot abort on Unicode text.
    env["PYTHONIOENCODING"] = "utf-8"
    env["PYTHONUTF8"] = "1"
    subprocess.run([sys.executable, *args], cwd=ROOT, env=env, check=True)


def replace_once_or_verify(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count == 1:
        return text.replace(old, new, 1)
    if count == 0 and new in text:
        return text
    raise RuntimeError(f"{label}: expected exactly one source anchor, found {count}")


def patch_batch_numpy_contract() -> None:
    """Keep the batch API borrowed on input and NumPy-native on output."""

    text = LIB.read_text(encoding="utf-8")

    old_inputs = '''    let open_vec: Option<Vec<f64>> = open.as_ref().map(|arr| arr.as_array().to_vec());
    let high_vec: Option<Vec<f64>> = high.as_ref().map(|arr| arr.as_array().to_vec());
    let low_vec: Option<Vec<f64>> = low.as_ref().map(|arr| arr.as_array().to_vec());
    let volume_vec: Option<Vec<f64>> = volume.as_ref().map(|arr| arr.as_array().to_vec());
    let secondary_vec: Option<Vec<f64>> = secondary.as_ref().map(|arr| arr.as_array().to_vec());
'''
    new_inputs = '''    let open_slice = open
        .as_ref()
        .map(|arr| arr.as_slice())
        .transpose()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let high_slice = high
        .as_ref()
        .map(|arr| arr.as_slice())
        .transpose()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let low_slice = low
        .as_ref()
        .map(|arr| arr.as_slice())
        .transpose()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let volume_slice = volume
        .as_ref()
        .map(|arr| arr.as_slice())
        .transpose()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
    let secondary_slice = secondary
        .as_ref()
        .map(|arr| arr.as_slice())
        .transpose()
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e)))?;
'''
    text = replace_once_or_verify(text, old_inputs, new_inputs, "batch borrowed inputs")

    old_call = '''            open_vec.as_deref(),
            high_vec.as_deref(),
            low_vec.as_deref(),
            close_slice,
            volume_vec.as_deref(),
            secondary_vec.as_deref(),
'''
    new_call = '''            open_slice,
            high_slice,
            low_slice,
            close_slice,
            volume_slice,
            secondary_slice,
'''
    text = replace_once_or_verify(text, old_call, new_call, "batch slice dispatch")

    old_outputs = '''        match value {
            IndicatorResult::Single(arr) => {
                dict.set_item(key, arr)?;
            }
            IndicatorResult::Double(a, b) => {
                dict.set_item(format!("{}_0", key), a)?;
                dict.set_item(format!("{}_1", key), b)?;
            }
            IndicatorResult::Triple(a, b, c) => {
                dict.set_item(format!("{}_0", key), a)?;
                dict.set_item(format!("{}_1", key), b)?;
                dict.set_item(format!("{}_2", key), c)?;
            }
            IndicatorResult::Quad(a, b, c, d) => {
                dict.set_item(format!("{}_0", key), a)?;
                dict.set_item(format!("{}_1", key), b)?;
                dict.set_item(format!("{}_2", key), c)?;
                dict.set_item(format!("{}_3", key), d)?;
            }
'''
    new_outputs = '''        match value {
            IndicatorResult::Single(arr) => {
                dict.set_item(key, PyArray1::from_vec(py, arr))?;
            }
            IndicatorResult::Double(a, b) => {
                dict.set_item(format!("{}_0", key), PyArray1::from_vec(py, a))?;
                dict.set_item(format!("{}_1", key), PyArray1::from_vec(py, b))?;
            }
            IndicatorResult::Triple(a, b, c) => {
                dict.set_item(format!("{}_0", key), PyArray1::from_vec(py, a))?;
                dict.set_item(format!("{}_1", key), PyArray1::from_vec(py, b))?;
                dict.set_item(format!("{}_2", key), PyArray1::from_vec(py, c))?;
            }
            IndicatorResult::Quad(a, b, c, d) => {
                dict.set_item(format!("{}_0", key), PyArray1::from_vec(py, a))?;
                dict.set_item(format!("{}_1", key), PyArray1::from_vec(py, b))?;
                dict.set_item(format!("{}_2", key), PyArray1::from_vec(py, c))?;
                dict.set_item(format!("{}_3", key), PyArray1::from_vec(py, d))?;
            }
'''
    text = replace_once_or_verify(text, old_outputs, new_outputs, "batch ndarray outputs")

    if ".as_array().to_vec()" in text[text.index("fn compute_indicators"):text.index("/// Result type for indicator computation.")]:
        raise RuntimeError("batch contract still copies a NumPy input column")
    if "dict.set_item(key, PyArray1::from_vec(py, arr))?;" not in text:
        raise RuntimeError("batch contract is missing ndarray-direct outputs")

    LIB.write_text(text, encoding="utf-8")


def main() -> int:
    # CFO and Twiggs Money Flow are public Python APIs backed by core kernels,
    # but they are intentionally outside the 78-entry C-ABI registry. Keep
    # their wrappers in lib.rs before regenerating the registry-owned file.
    run(str(ROOT / "scripts" / "repair_python_new_indicator_bindings.py"))

    # Build an ephemeral Python-only registry overlay. The helper restores the
    # canonical docs registry before it exits and teaches sync_bindings to read
    # the overlay for this build workspace.
    run(str(ROOT / "scripts" / "prepare_python_registry_ssot.py"))

    # Regenerate from SSOT first; optimization is deliberately post-generation
    # so generated bindings can never silently fall back to Vec -> Python list.
    run(
        str(ROOT / "scripts" / "sync_bindings.py"),
        "--lang",
        "python",
        "--generate",
    )

    # Formula ROC/MOM are classified once when the compiled formula is created
    # and then execute the same canonical indicator kernels as the public API.
    # This removes the generic Formula executor from these common hot paths.
    run(str(ROOT / "scripts" / "apply_formula_v3_fast_path.py"))

    # Batch input arrays remain borrowed while Rust owns the GIL-free compute
    # interval; results cross back into Python as ndarrays, never Python lists.
    patch_batch_numpy_contract()

    generated_count = optimize_file(GENERATED)
    lib_count = optimize_file(LIB)

    # Idempotence is part of the build contract: a second optimization pass must
    # find nothing left to rewrite.
    optimize_file(GENERATED, check=True)
    optimize_file(LIB, check=True)

    print(
        "[prepare/python-hot] NumPy-direct binding surface ready: "
        f"generated={generated_count}, lib={lib_count}, batch=zero-copy, formula=canonical"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
