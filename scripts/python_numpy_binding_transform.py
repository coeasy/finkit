#!/usr/bin/env python3
"""Transform PyO3 numeric Vec returns into NumPy array returns.

This module is intentionally dependency-free so both the binding generator and
one-shot migration scripts can import it.  It rewrites #[pyfunction] functions
whose return type is a Vec<f64>/Vec<i32> (or tuples of those) into a private
raw function plus a public wrapper that materialises numpy::PyArray1 directly.
The Rust calculation remains outside the GIL exactly as before; only the Python
object materialisation changes.
"""
from __future__ import annotations

import re

_SUPPORTED = {
    "PyResult<Vec<f64>>": ("PyResult<Py<PyArray1<f64>>>", 1, "f64"),
    "PyResult<Vec<i32>>": ("PyResult<Py<PyArray1<i32>>>", 1, "i32"),
    "PyResult<(Vec<f64>,Vec<f64>)>": (
        "PyResult<(Py<PyArray1<f64>>, Py<PyArray1<f64>>)>", 2, "f64"
    ),
    "PyResult<(Vec<f64>,Vec<f64>,Vec<f64>)>": (
        "PyResult<(Py<PyArray1<f64>>, Py<PyArray1<f64>>, Py<PyArray1<f64>>)>", 3, "f64"
    ),
    "PyResult<(Vec<f64>,Vec<f64>,Vec<f64>,Vec<f64>)>": (
        "PyResult<(Py<PyArray1<f64>>, Py<PyArray1<f64>>, Py<PyArray1<f64>>, Py<PyArray1<f64>>)>", 4, "f64"
    ),
}


def _compact(value: str) -> str:
    return re.sub(r"\s+", "", value)


def _matching(src: str, start: int, opening: str, closing: str) -> int:
    depth = 0
    i = start
    in_string = False
    escaped = False
    while i < len(src):
        c = src[i]
        if in_string:
            if escaped:
                escaped = False
            elif c == "\\":
                escaped = True
            elif c == '"':
                in_string = False
        else:
            if c == '"':
                in_string = True
            elif c == opening:
                depth += 1
            elif c == closing:
                depth -= 1
                if depth == 0:
                    return i
        i += 1
    raise ValueError(f"unbalanced {opening}{closing}")


def _split_args(value: str) -> list[str]:
    out: list[str] = []
    start = 0
    paren = angle = bracket = 0
    for i, c in enumerate(value):
        if c == "(": paren += 1
        elif c == ")": paren -= 1
        elif c == "<": angle += 1
        elif c == ">": angle = max(0, angle - 1)
        elif c == "[": bracket += 1
        elif c == "]": bracket -= 1
        elif c == "," and paren == 0 and angle == 0 and bracket == 0:
            part = value[start:i].strip()
            if part:
                out.append(part)
            start = i + 1
    tail = value[start:].strip()
    if tail:
        out.append(tail)
    return out


def _arg_names(args: str) -> list[str]:
    names: list[str] = []
    for arg in _split_args(args):
        if ":" not in arg:
            continue
        left = arg.split(":", 1)[0].strip()
        left = re.sub(r"\bmut\b", "", left).strip()
        if left.startswith("_") and left != "_":
            names.append(left)
        elif re.fullmatch(r"[A-Za-z][A-Za-z0-9_]*", left):
            names.append(left)
    return names


def _strip_py_attrs(prefix: str) -> str:
    lines = []
    for line in prefix.splitlines(keepends=True):
        stripped = line.strip()
        if stripped == "#[pyfunction]" or stripped.startswith("#[pyo3("):
            continue
        lines.append(line)
    return "".join(lines)


def _py_attrs(prefix: str) -> str:
    lines = []
    for line in prefix.splitlines(keepends=True):
        stripped = line.strip()
        if stripped == "#[pyfunction]" or stripped.startswith("#[pyo3("):
            lines.append(line)
    if not any(line.strip() == "#[pyfunction]" for line in lines):
        lines.insert(0, "#[pyfunction]\n")
    return "".join(lines)


def _convert_expr(count: int) -> str:
    if count == 1:
        return "    Ok(PyArray1::from_vec(py, result).unbind())\n"
    names = [chr(ord("a") + i) for i in range(count)]
    unpack = ", ".join(names)
    converted = ",\n        ".join(
        f"PyArray1::from_vec(py, {name}).unbind()" for name in names
    )
    return f"    let ({unpack}) = result;\n    Ok((\n        {converted},\n    ))\n"


def transform_python_pyfunctions(src: str) -> str:
    """Return source with numeric #[pyfunction] Vec returns converted to ndarray."""
    marker = "// PY_NUMPY_DIRECT_RETURN_V1"
    if marker in src:
        return src

    cursor = 0
    pieces: list[str] = []
    changed = 0
    while True:
        attr = src.find("#[pyfunction]", cursor)
        if attr < 0:
            pieces.append(src[cursor:])
            break

        # Include immediately preceding doc comments/attributes in the function block.
        block_start = attr
        probe = attr
        while probe > cursor:
            line_start = src.rfind("\n", cursor, probe - 1) + 1
            line = src[line_start:probe].strip()
            if line.startswith("///") or line.startswith("#[") or line == "":
                block_start = line_start
                probe = line_start
                if line == "":
                    break
            else:
                break

        fn_match = re.search(r"\b(?:pub\s+)?fn\s+([a-zA-Z_][a-zA-Z0-9_]*)", src[attr:])
        if not fn_match:
            pieces.append(src[cursor:attr + 13])
            cursor = attr + 13
            continue
        fn_pos = attr + fn_match.start()
        name = fn_match.group(1)
        name_pos = attr + fn_match.start(1)
        args_open = src.find("(", name_pos + len(name))
        if args_open < 0:
            pieces.append(src[cursor:attr + 13]); cursor = attr + 13; continue
        try:
            args_close = _matching(src, args_open, "(", ")")
        except ValueError:
            pieces.append(src[cursor:attr + 13]); cursor = attr + 13; continue
        body_open = src.find("{", args_close)
        if body_open < 0:
            pieces.append(src[cursor:attr + 13]); cursor = attr + 13; continue
        return_text = src[args_close + 1:body_open].strip()
        if not return_text.startswith("->"):
            pieces.append(src[cursor:attr + 13]); cursor = attr + 13; continue
        original_ret = return_text[2:].strip()
        spec = _SUPPORTED.get(_compact(original_ret))
        if spec is None:
            pieces.append(src[cursor:attr + 13]); cursor = attr + 13; continue
        try:
            body_close = _matching(src, body_open, "{", "}")
        except ValueError:
            pieces.append(src[cursor:attr + 13]); cursor = attr + 13; continue

        prefix = src[block_start:fn_pos]
        args_text = src[args_open + 1:args_close]
        names = _arg_names(args_text)
        if not names or names[0] != "py":
            pieces.append(src[cursor:attr + 13]); cursor = attr + 13; continue

        public_ret, count, _dtype = spec
        signature_prefix = src[fn_pos:name_pos]
        raw_signature = (
            signature_prefix + name + "_vec" + src[name_pos + len(name):args_close + 1]
            + " -> " + original_ret + " "
        )
        raw_body = src[body_open:body_close + 1]
        raw_prefix = _strip_py_attrs(prefix)
        attrs = _py_attrs(prefix)
        call = ", ".join(names)
        public_signature = (
            signature_prefix + name + src[name_pos + len(name):args_close + 1]
            + " -> " + public_ret + " {\n"
        )
        wrapper = (
            attrs
            + public_signature
            + f"    let result = {name}_vec({call})?;\n"
            + _convert_expr(count)
            + "}\n"
        )

        pieces.append(src[cursor:block_start])
        pieces.append(raw_prefix + raw_signature + raw_body + "\n\n" + wrapper)
        cursor = body_close + 1
        changed += 1

    out = "".join(pieces)
    if changed:
        out = marker + f" // transformed={changed}\n" + out
    return out


if __name__ == "__main__":
    import argparse
    from pathlib import Path

    parser = argparse.ArgumentParser()
    parser.add_argument("path")
    args = parser.parse_args()
    path = Path(args.path)
    path.write_text(transform_python_pyfunctions(path.read_text(encoding="utf-8")), encoding="utf-8")
