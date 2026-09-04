#!/usr/bin/env python3
"""Rewrite numeric PyO3 pyfunctions to return NumPy arrays directly.

The Python binding historically exposed ``Vec<T>`` from ``#[pyfunction]``.
PyO3 materializes those vectors as Python lists, after which the package layer
converted the lists to NumPy arrays.  For large series that list materialization
can dominate the actual indicator calculation.

This module performs a source-to-source rewrite that preserves the existing
Rust implementation as ``vec_<name>_impl`` and inserts a thin public wrapper
with the original Python/Rust function name.  The wrapper converts the computed
Vec(s) directly into ``numpy::PyArray1`` without creating Python float objects.

Supported return shapes:

* ``PyResult<Vec<f64>>`` / ``PyResult<Vec<i32>>`` / ``PyResult<Vec<i64>>``
* tuples containing 2-6 numeric Vec outputs, including mixed i32/f64 tuples

The transformation is idempotent and is intentionally limited to top-level
``#[pyfunction]`` items.  ``#[pymethods]`` getters and non-numeric container
APIs are left unchanged.
"""

from __future__ import annotations

import argparse
import re
from pathlib import Path

_NUMERIC = {"f64", "f32", "i64", "i32", "u64", "u32", "i16", "u16", "i8", "u8"}


def _match_pair(text: str, start: int, opening: str, closing: str) -> int:
    """Find a balanced Rust delimiter pair without mistaking lifetimes for strings."""
    depth = 0
    in_string = False
    escaped = False
    for i in range(start, len(text)):
        ch = text[i]
        if in_string:
            if escaped:
                escaped = False
            elif ch == "\\":
                escaped = True
            elif ch == '"':
                in_string = False
            continue
        # Rust lifetimes such as Python<'_> use apostrophes.  Only a double
        # quote starts the string literals that matter while scanning function
        # signatures/bodies here; treating `'` as a quote makes the scanner
        # skip closing parentheses after a lifetime.
        if ch == '"':
            in_string = True
            continue
        if ch == opening:
            depth += 1
        elif ch == closing:
            depth -= 1
            if depth == 0:
                return i
    raise ValueError(f"unbalanced {opening}{closing} at offset {start}")


def _split_top_level(text: str) -> list[str]:
    parts: list[str] = []
    start = 0
    angle = paren = bracket = 0
    for i, ch in enumerate(text):
        if ch == "<":
            angle += 1
        elif ch == ">" and angle:
            angle -= 1
        elif ch == "(":
            paren += 1
        elif ch == ")" and paren:
            paren -= 1
        elif ch == "[":
            bracket += 1
        elif ch == "]" and bracket:
            bracket -= 1
        elif ch == "," and not angle and not paren and not bracket:
            parts.append(text[start:i].strip())
            start = i + 1
    tail = text[start:].strip()
    if tail:
        parts.append(tail)
    return parts


def _numeric_vec_components(return_type: str) -> list[str] | None:
    compact = re.sub(r"\s+", "", return_type)
    if not (compact.startswith("PyResult<") and compact.endswith(">")):
        return None
    inner = compact[len("PyResult<") : -1]
    if inner.startswith("Vec<") and inner.endswith(">"):
        ty = inner[4:-1]
        return [ty] if ty in _NUMERIC else None
    if inner.startswith("(") and inner.endswith(")"):
        fields = _split_top_level(inner[1:-1])
        out: list[str] = []
        for field in fields:
            m = re.fullmatch(r"Vec<([A-Za-z0-9_:]+)>", field)
            if not m:
                return None
            ty = m.group(1).split("::")[-1]
            if ty not in _NUMERIC:
                return None
            out.append(ty)
        return out if out else None
    return None


def _strip_rust_doc_prose(params: str) -> str:
    """Remove doc/comment fragments that cannot be Rust parameters."""
    cleaned: list[str] = []
    in_block_comment = False
    for raw_line in params.splitlines():
        line = raw_line.strip()
        if in_block_comment:
            if "*/" in line:
                in_block_comment = False
            continue
        if line.startswith("/*"):
            if "*/" not in line:
                in_block_comment = True
            continue
        if line.startswith("//") or line.startswith("///") or line.startswith("#"):
            continue
        cleaned.append(raw_line)
    return "\n".join(cleaned)


def _call_arg_names(params: str) -> list[str]:
    names: list[str] = []
    params = _strip_rust_doc_prose(params)
    for part in _split_top_level(params):
        if not part:
            continue
        m = re.search(r"(?:^|\s)(?:mut\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:", part)
        if not m:
            if ":" not in part:
                continue
            raise ValueError(f"unsupported pyfunction parameter pattern: {part!r}")
        names.append(m.group(1))
    return names


def _numpy_return_type(components: list[str]) -> str:
    converted = [f"Py<PyArray1<{ty}>>" for ty in components]
    if len(converted) == 1:
        return f"PyResult<{converted[0]}>"
    return "PyResult<(" + ", ".join(converted) + ")>"


def _wrapper_body(impl_name: str, args: list[str], components: list[str]) -> str:
    call = f"{impl_name}({', '.join(args)})?"
    if len(components) == 1:
        return (
            "{\n"
            f"    let result = {call};\n"
            "    Ok(PyArray1::from_vec(py, result).unbind())\n"
            "}"
        )
    vars_ = [f"out_{i}" for i in range(len(components))]
    values = ",\n        ".join(
        f"PyArray1::from_vec(py, {name}).unbind()" for name in vars_
    )
    return (
        "{\n"
        f"    let ({', '.join(vars_)}) = {call};\n"
        "    Ok((\n"
        f"        {values},\n"
        "    ))\n"
        "}"
    )


def optimize_source(source: str) -> tuple[str, int]:
    """Return transformed source and number of optimized pyfunctions."""

    marker = "#[pyfunction]"
    cursor = 0
    out: list[str] = []
    changed = 0

    while True:
        attr_start = source.find(marker, cursor)
        if attr_start < 0:
            out.append(source[cursor:])
            break

        out.append(source[cursor:attr_start])
        fn_match = re.search(r"\bfn\s+([a-z][a-z0-9_]*)\s*", source[attr_start:])
        if not fn_match:
            out.append(source[attr_start:])
            break
        fn_name = fn_match.group(1)
        fn_pos = attr_start + fn_match.start()
        fn_name_pos = attr_start + fn_match.start(1)

        if fn_name.startswith("vec_") and fn_name.endswith("_impl"):
            brace = source.find("{", fn_pos)
            if brace < 0:
                out.append(source[attr_start:])
                break
            end = _match_pair(source, brace, "{", "}") + 1
            out.append(source[attr_start:end])
            cursor = end
            continue

        paren = source.find("(", fn_name_pos + len(fn_name))
        if paren < 0:
            out.append(source[attr_start:])
            break
        paren_end = _match_pair(source, paren, "(", ")")
        brace = source.find("{", paren_end)
        if brace < 0:
            out.append(source[attr_start:])
            break
        fn_end = _match_pair(source, brace, "{", "}") + 1

        signature_tail = source[paren_end + 1 : brace]
        arrow = signature_tail.find("->")
        if arrow < 0:
            out.append(source[attr_start:fn_end])
            cursor = fn_end
            continue
        return_type = signature_tail[arrow + 2 :].strip()
        components = _numeric_vec_components(return_type)
        if not components:
            out.append(source[attr_start:fn_end])
            cursor = fn_end
            continue

        params = source[paren + 1 : paren_end]
        args = _call_arg_names(params)
        if "py" not in args:
            out.append(source[attr_start:fn_end])
            cursor = fn_end
            continue

        impl_name = f"vec_{fn_name}_impl"
        original_item = source[attr_start:fn_end]
        local_name_offset = fn_name_pos - attr_start
        renamed = (
            original_item[:local_name_offset]
            + impl_name
            + original_item[local_name_offset + len(fn_name) :]
        )

        attrs = source[attr_start:fn_pos]
        pyo3_attrs = "\n".join(
            line.strip()
            for line in attrs.splitlines()
            if line.strip().startswith("#[pyo3(") or line.strip().startswith("#[allow(")
        )
        wrapper_attrs = "#[pyfunction]"
        if pyo3_attrs:
            wrapper_attrs += "\n" + pyo3_attrs

        original_signature = source[fn_pos:brace]
        new_return = _numpy_return_type(components)
        wrapper_signature = re.sub(
            r"->\s*PyResult<.*>\s*$",
            f"-> {new_return} ",
            original_signature,
            flags=re.S,
        )
        wrapper = (
            "\n\n"
            + wrapper_attrs
            + "\n"
            + wrapper_signature
            + _wrapper_body(impl_name, args, components)
        )

        out.append(renamed)
        out.append(wrapper)
        cursor = fn_end
        changed += 1

    return "".join(out), changed


def optimize_file(path: Path, *, check: bool = False) -> int:
    original = path.read_text(encoding="utf-8")
    optimized, count = optimize_source(original)
    if check:
        if optimized != original:
            raise SystemExit(f"{path}: {count} numeric pyfunctions still need NumPy-direct wrappers")
        print(f"{path}: NumPy-direct binding check OK")
        return 0
    if optimized != original:
        path.write_text(optimized, encoding="utf-8")
    print(f"{path}: optimized {count} numeric pyfunctions")
    return count


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("paths", nargs="+", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    total = 0
    for path in args.paths:
        total += optimize_file(path, check=args.check)
    if not args.check:
        print(f"optimized numeric pyfunctions: {total}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
