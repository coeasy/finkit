#!/usr/bin/env python3
"""Convert Vec<f64> indicator inputs to PyReadonlyArray1 zero-copy in lib.rs."""

from __future__ import annotations

import re
from pathlib import Path

LIB_RS = Path(__file__).resolve().parent.parent / "src" / "lib.rs"

SKIP_FUNCS = {
    "formula_eval",
    "formula_eval_bytecode",
    "formula_eval_optimized",
    "formula_eval_jit",
    "formula_eval_simd",
    "formula_eval_zero_copy",
    "formula_eval_debug",
    "formula_validate",
    "formula_get_template",
    "formula_search_templates",
    "formula_list_categories",
    "fibonacci_retracement",
    "alpha_ta",
}

SLICE_ERR = (
    'PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e))'
)
PARAM_VEC_RE = re.compile(r"(\w+):\s*Vec<f64>")


def find_matching_brace(text: str, open_pos: int) -> int:
    depth = 0
    i = open_pos
    while i < len(text):
        c = text[i]
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    raise ValueError(f"Unmatched brace at {open_pos}")


def find_pyfunction_blocks(content: str) -> list[tuple[int, int, str]]:
    blocks = []
    pattern = re.compile(r"#\[pyfunction\]", re.MULTILINE)
    for m in pattern.finditer(content):
        start = m.start()
        fn_match = re.search(
            r"(?:pub\s+)?fn\s+(\w+)\s*\(",
            content[m.end() : m.end() + 2000],
        )
        if not fn_match:
            continue
        func_name = fn_match.group(1)
        paren_start = m.end() + fn_match.end() - 1
        depth = 0
        i = paren_start
        while i < len(content):
            c = content[i]
            if c == "(":
                depth += 1
            elif c == ")":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        body_brace = content.find("{", i)
        if body_brace == -1:
            continue
        body_end = find_matching_brace(content, body_brace)
        blocks.append((start, body_end + 1, func_name))
    return blocks


def get_signature_and_body(block: str, func_name: str) -> tuple[str, str, str]:
    fn_match = re.search(rf"((?:pub\s+)?fn\s+{re.escape(func_name)}\s*\()", block)
    if not fn_match:
        raise ValueError(f"Cannot find fn {func_name}")
    sig_start = fn_match.start(1)
    paren_start = fn_match.end(1) - 1
    depth = 0
    i = paren_start
    while i < len(block):
        c = block[i]
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                break
        i += 1
    ret_match = re.match(r"\s*->\s*[^{]+", block[i + 1 :])
    if not ret_match:
        raise ValueError(f"Cannot find return type for {func_name}")
    sig_end = i + 1 + ret_match.end()
    signature = block[sig_start:sig_end].strip()
    body_brace = block.find("{", sig_end)
    body = block[body_brace + 1 : block.rfind("}")]
    prefix = block[:sig_start]
    return prefix, signature, body


def extract_array_params(signature: str) -> list[str]:
    paren_start = signature.index("(")
    depth = 0
    i = paren_start
    while i < len(signature):
        c = signature[i]
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                break
        i += 1
    params_section = signature[paren_start + 1 : i]
    return [m.group(1) for m in PARAM_VEC_RE.finditer(params_section)]


def transform_signature(signature: str) -> str:
    paren_start = signature.index("(")
    depth = 0
    i = paren_start
    while i < len(signature):
        c = signature[i]
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                break
        i += 1
    params = signature[paren_start + 1 : i]
    ret = signature[i:]
    new_params = PARAM_VEC_RE.sub(r"\1: PyReadonlyArray1<'_, f64>", params)
    return signature[: paren_start + 1] + new_params + ret


def slice_lines(params: list[str]) -> str:
    lines = []
    for name in params:
        lines.append(
            f"    let {name} = {name}.as_slice().map_err(|e| {SLICE_ERR})?;"
        )
    return "\n".join(lines) + "\n"


def replace_param_refs(body: str, params: list[str]) -> str:
    for name in params:
        body = re.sub(rf"&{re.escape(name)}\b", name, body)
    return body


def transform_body(body: str, params: list[str]) -> str:
    if not params:
        return body
    if "PyReadonlyArray1" in body and ".as_slice()" in body:
        return body

    body = replace_param_refs(body, params)
    extraction = slice_lines(params)

    allow_idx = body.find("py.allow_threads(|| {")
    if allow_idx == -1:
        return extraction + body

    before = body[:allow_idx]
    after = body[allow_idx:]
    if ".as_slice()" in before:
        return body
    return before + extraction + after


def transform_block(block: str, func_name: str) -> str:
    if func_name in SKIP_FUNCS:
        return block

    prefix, signature, body = get_signature_and_body(block, func_name)
    params = extract_array_params(signature)
    if not params:
        return block

    new_sig = transform_signature(signature)
    new_body = transform_body(body, params)
    return prefix + new_sig + " {" + new_body + "}"


def add_numpy_import(content: str) -> str:
    needle = "use pyo3::prelude::*;\n"
    import_line = "use numpy::PyReadonlyArray1;\n"
    if import_line in content:
        return content
    if needle not in content:
        raise ValueError("Could not find pyo3 prelude import")
    return content.replace(needle, needle + import_line, 1)


def main() -> None:
    content = LIB_RS.read_text(encoding="utf-8")
    content = add_numpy_import(content)
    blocks = find_pyfunction_blocks(content)
    print(f"Found {len(blocks)} #[pyfunction] blocks")

    changed = 0
    for start, end, func_name in reversed(blocks):
        block = content[start:end]
        new_block = transform_block(block, func_name)
        if new_block != block:
            content = content[:start] + new_block + content[end:]
            changed += 1
            print(f"  transformed: {func_name}")

    LIB_RS.write_text(content, encoding="utf-8")
    print(f"Done. Transformed {changed} functions.")


if __name__ == "__main__":
    main()
