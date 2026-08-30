#!/usr/bin/env python3
"""Add py.allow_threads() to all #[pyfunction] entries in lib.rs."""

from __future__ import annotations

import re
from pathlib import Path

LIB_RS = Path(__file__).resolve().parent.parent / "src" / "lib.rs"

FORMULA_EVAL_FUNCS = {
    "formula_eval",
    "formula_eval_bytecode",
    "formula_eval_optimized",
    "formula_eval_jit",
    "formula_eval_simd",
    "formula_eval_zero_copy",
    "formula_eval_debug",
}

FORMULA_TEMPLATE_FUNCS = {
    "formula_get_template",
    "formula_search_templates",
    "formula_list_categories",
}


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
    """Return list of (start, end, func_name) for each #[pyfunction] body."""
    blocks = []
    pattern = re.compile(r"#\[pyfunction\]", re.MULTILINE)
    for m in pattern.finditer(content):
        start = m.start()
        # Skip pymodule (shouldn't have #[pyfunction] anyway)
        fn_match = re.search(
            r"(?:pub\s+)?fn\s+(\w+)\s*\(",
            content[m.end() : m.end() + 2000],
        )
        if not fn_match:
            continue
        func_name = fn_match.group(1)
        if func_name == "finkit":
            continue
        paren_start = m.end() + fn_match.end() - 1
        # Find closing paren of signature (handle nested generics)
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


def has_py_param(signature: str) -> bool:
    return bool(re.search(r"\bpy\s*:\s*(?:pyo3::)?Python<'_>", signature))


def add_py_param(signature: str) -> str:
    return re.sub(
        r"((?:pub\s+)?fn\s+\w+\s*\()",
        r"\1py: Python<'_>, ",
        signature,
        count=1,
    )


def indent_body(body: str, extra: int = 4) -> str:
    lines = body.split("\n")
    result = []
    for line in lines:
        if line.strip():
            result.append(" " * extra + line)
        else:
            result.append("")
    return "\n".join(result)


def wrap_body(body: str) -> str:
    stripped = body.strip("\n")
    if "allow_threads" in stripped:
        return body
    inner = indent_body(stripped, 4)
    return f"\n    py.allow_threads(|| {{\n{inner}\n    }})\n"


def transform_formula_eval(prefix: str, signature: str, body: str, func_name: str) -> str:
    """Wrap engine evaluation in allow_threads; keep extract and dict building on GIL."""
    if "allow_threads" in body:
        return prefix + signature + " {" + body + "}"

    # Find `let result = engine.` or `(final_result, debugger) = engine.`
    eval_patterns = [
        r"(\n    let result = engine\.[\s\S]*?\?\;)\n\n    let dict",
        r"(\n    let \(final_result, debugger\) =[\s\S]*?\?\;)\n\n    let result_dict",
    ]
    for pat in eval_patterns:
        m = re.search(pat, body)
        if m:
            eval_block = m.group(1)
            wrapped = (
                "\n    py.allow_threads(|| {"
                + indent_body(eval_block.strip("\n"), 4)
                + "\n    })?;\n\n"
            )
            new_body = body[: m.start(1)] + wrapped + body[m.end(1) :]
            return prefix + signature + " {" + new_body + "}"

    # Fallback: wrap entire body
    return prefix + signature + " {" + wrap_body(body) + "}"


def transform_fibonacci(prefix: str, signature: str, body: str) -> str:
    if "allow_threads" in body:
        return prefix + signature + " {" + body + "}"

    m = re.search(
        r"(\n    let result = indicators::fibonacci_retracement\([\s\S]*?\?\;)\n\n    let dict",
        body,
    )
    if not m:
        return prefix + signature + " {" + wrap_body(body) + "}"

    compute = m.group(1).strip()
    wrapped = f"\n    let result = py.allow_threads(|| {{\n        {compute.lstrip()}\n    }})?;\n\n"
    new_body = body[: m.start(1)] + wrapped + body[m.end(1) :]
    return prefix + signature + " {" + new_body + "}"


def transform_formula_validate(prefix: str, signature: str, body: str) -> str:
    sig = add_py_param(signature) if not has_py_param(signature) else signature
    return prefix + sig + " {" + wrap_body(body) + "}"


def transform_formula_template(prefix: str, signature: str, body: str, func_name: str) -> str:
    """Wrap FormulaEngine work in allow_threads where possible."""
    if "allow_threads" in body:
        return prefix + signature + " {" + body + "}"

    if func_name == "formula_get_template":
        m = re.search(
            r"(\n    let dict = pyo3::types::PyDict::new\(py\);\n    let engine = FormulaEngine::new\(\);\n\n    match engine\.get_template\(name\) \{[\s\S]*?\n    \}\n\n    Ok\(dict\.into\(\)\))",
            body,
        )
        if m:
            lookup = m.group(1)
            wrapped = (
                "\n    let template_info = py.allow_threads(|| {\n"
                "        let engine = FormulaEngine::new();\n"
                "        engine.get_template(name).map(|t| {\n"
                "            (t.name.clone(), t.category, t.description.clone(), t.source.clone(), t.parameters.clone())\n"
                "        })\n"
                "    });\n\n"
                "    let dict = pyo3::types::PyDict::new(py);\n"
                "    match template_info {\n"
                "        Ok(template) => {\n"
                "            dict.set_item(\"name\", template.0.as_str())?;\n"
                "            dict.set_item(\"category\", format!(\"{:?}\", template.1))?;\n"
                "            dict.set_item(\"description\", template.2.as_str())?;\n"
                "            dict.set_item(\"formula\", template.3.as_str())?;\n"
                "            let params_dict = pyo3::types::PyDict::new(py);\n"
                "            for (param_name, default, min, max) in &template.4 {\n"
                "                let param_info = pyo3::types::PyDict::new(py);\n"
                "                param_info.set_item(\"default\", default)?;\n"
                "                param_info.set_item(\"min\", min)?;\n"
                "                param_info.set_item(\"max\", max)?;\n"
                "                params_dict.set_item(param_name.as_str(), param_info)?;\n"
                "            }\n"
                "            dict.set_item(\"parameters\", params_dict)?;\n"
                "        }\n"
                "        Err(()) => {\n"
                "            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(\n"
                "                \"Template '{}' not found\",\n"
                "                name\n"
                "            )));\n"
                "        }\n"
                "    }\n\n"
                "    Ok(dict.into())"
            )
            return prefix + signature + " {" + wrapped + "\n}"

    # For search/list - wrap engine creation + query
    return prefix + signature + " {" + wrap_body(body) + "}"


def transform_block(block: str, func_name: str) -> str:
    prefix, signature, body = get_signature_and_body(block, func_name)

    if func_name in FORMULA_EVAL_FUNCS:
        return transform_formula_eval(prefix, signature, body, func_name)
    if func_name == "fibonacci_retracement":
        return transform_fibonacci(prefix, signature, body)
    if func_name == "formula_validate":
        return transform_formula_validate(prefix, signature, body)
    if func_name in FORMULA_TEMPLATE_FUNCS:
        # Template lookup is lightweight; engine work stays on GIL for PyDict building.
        return block

    if has_py_param(signature):
        # Already has py — only specialized transforms above apply.
        return block

    new_sig = add_py_param(signature)
    new_body = wrap_body(body)
    return prefix + new_sig + " {" + new_body + "}"


def main() -> None:
    content = LIB_RS.read_text(encoding="utf-8")
    blocks = find_pyfunction_blocks(content)
    print(f"Found {len(blocks)} #[pyfunction] blocks")

    # Process from end to start to preserve offsets
    for start, end, func_name in reversed(blocks):
        block = content[start:end]
        new_block = transform_block(block, func_name)
        if new_block != block:
            content = content[:start] + new_block + content[end:]
            print(f"  transformed: {func_name}")

    LIB_RS.write_text(content, encoding="utf-8")
    print("Done.")


if __name__ == "__main__":
    main()
