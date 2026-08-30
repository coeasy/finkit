#!/usr/bin/env python3
"""Registry-driven binding code generator for AlphaTA.

Consumes the single-source-of-truth ``docs/indicator_registry.json`` (each
indicator carries an ``ffi`` block enriched by ``extract_corecalls.py``) and
emits FFI wrappers for several languages:

    --lang c       C ABI wrappers (``#[no_mangle] extern "C"``) — full,
                   regenerates ffi/c-binding/src/lib.rs indicator section.
    --lang python  pyo3 ``#[pymethods]`` indicator wrappers.
    --lang node    napi ``#[napi]`` indicator wrappers.

Modes:
    --generate PATH     write the generated source to PATH
    --rewrite-cbinding  (c only) write generated.rs AND rewrite
                        ffi/c-binding/src/lib.rs to include it (drops the
                        hand-written indicator functions, keeps helpers/infra)
    --check             verify the generated function set/params match the
                        hand-written binding (no write)

The C path is the reference implementation: the header (gen_c_header.py) and
the Rust wrappers are both regenerated from the registry, so adding an
indicator becomes a single registry edit + two generators.

Usage examples:
    python3 scripts/gen_binding.py --lang c --rewrite-cbinding
    python3 scripts/gen_binding.py --lang c --check
    python3 scripts/gen_binding.py --lang python --generate /tmp/py_gen.rs --check
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REG = ROOT / "docs" / "indicator_registry.json"
C_LIB = ROOT / "ffi" / "c-binding" / "src" / "lib.rs"


def load_registry() -> dict:
    return json.loads(REG.read_text(encoding="utf-8"))


def indicators_with_ffi(reg: dict) -> list[dict]:
    out = []
    for ind in reg.get("indicators", []):
        ff = ind.get("ffi")
        if ff and ff.get("c_name"):
            out.append(ind)
    out.sort(key=lambda i: i["ffi"].get("order", 0))
    return out


# ──────────────────────────────────────────────────────────────────────────
# C emitter
# ──────────────────────────────────────────────────────────────────────────
def emit_c_indicator(ffi: dict) -> str:
    name = ffi["c_name"]
    params = ffi["c_params"]
    sig = ",\n    ".join(f"{p['name']}: {p['ty']}" for p in params)
    # The function body is stored verbatim from the original lib.rs, so the
    # regenerated wrapper is byte-faithful. It already contains the
    # ffi_catch_i32(|| unsafe { ... }) wrapper, null/data/param checks, slice
    # declarations and the core match + copies.
    body = "\n".join(ffi["c_body"].splitlines())
    return f'''#[no_mangle]
pub unsafe extern "C" fn {name}(
    {sig},
) -> i32 {{
{body}
}}
'''


def emit_c_file(inds: list[dict]) -> str:
    header = (
        "// ─────────────────────────────────────────────────────────────────────\n"
        "// GENERATED FILE — do not edit by hand.\n"
        "// Source of truth: docs/indicator_registry.json (ffi block).\n"
        "// Regenerate with: python3 scripts/gen_binding.py --lang c --rewrite-cbinding\n"
        "// ─────────────────────────────────────────────────────────────────────\n"
    )
    body = "\n\n".join(emit_c_indicator(i["ffi"]) for i in inds)
    return header + "\n" + body + "\n"


# ──────────────────────────────────────────────────────────────────────────
# Python (pyo3) + Node (napi) emitters — templated from language-agnostic
# `core_arg_kinds` + `copies` so the SAME registry drives every binding.
# ──────────────────────────────────────────────────────────────────────────
def resolve_input_kind(ffi: dict, input_ptr: str) -> str:
    for inp in ffi.get("inputs", []):
        if inp.get("name") == input_ptr:
            return inp.get("kind", input_ptr)
    return input_ptr  # OHLC ptr names already equal their kind


def lang_call_args(ffi: dict, lang: str) -> list[str]:
    """Build the core call argument list for python/node (Vec/&Vec inputs)."""
    args = []
    for k in ffi.get("core_arg_kinds", []):
        if "input_ptr" in k:
            kind = resolve_input_kind(ffi, k["input_ptr"])
            args.append(f"&{kind}" if lang == "python" else f"&{kind}")
        else:
            nm = k["param"]
            args.append(f"{nm} as usize" if k.get("usize") else nm)
    return args


def emit_py_node_indicator(ffi: dict, lang: str) -> str:
    name = ffi["c_name"][len("ta_"):]  # sma, rsi, macd, ...
    # signature
    sig_params = []
    for inp in ffi.get("inputs", []):
        kind = inp.get("kind", inp.get("name"))
        sig_params.append(f"{kind}: Vec<f64>")
    for p in ffi.get("params", []):
        sig_params.append(f"{p['name']}: i32")
    sig = ", ".join(sig_params)

    core_call = ffi["core_call"]
    args = ", ".join(lang_call_args(ffi, lang))
    if lang == "python":
        ret = "PyResult<Vec<f64>>" if ffi.get("out_kind") != "int" else "PyResult<Vec<i32>>"
        head = f'    #[pyo3(text_signature = "({sig})")]\n    fn {name}(&self, {sig}) -> {ret} {{'
        err = 'PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e))'
        body_core = f"{core_call}({args})"
        if ffi.get("out_kind") == "int":
            ret_expr = "Ok(result.into_raw_vec())"
        elif ffi.get("out_kind") == "struct":
            fields = [c.split("&result.")[1].split(",")[0].strip()
                      for c in ffi.get("copies", [])]
            tup = ", ".join(f"result.{f}.into_raw_vec()" for f in fields)
            ret_expr = f"Ok(({tup}))"
        else:
            ret_expr = "Ok(result.into_raw_vec())"
        return (
            f"{head}\n"
            f"        let result = {body_core}.map_err(|e| {err})?;\n"
            f"        {ret_expr}\n"
            f"    }}\n"
        )
    else:  # node
        ret = "Result<Vec<f64>>" if ffi.get("out_kind") != "int" else "Result<Vec<i32>>"
        head = f'    #[napi]\n    pub fn {name}({sig}) -> {ret} {{'
        err = 'Error::new(Status::InvalidArg, format!("{}", e))'
        body_core = f"{core_call}({args})"
        if ffi.get("out_kind") == "int":
            ret_expr = "Ok(arr.into_raw_vec_and_offset().0)"
        elif ffi.get("out_kind") == "struct":
            fields = [c.split("&result.")[1].split(",")[0].strip()
                      for c in ffi.get("copies", [])]
            tup = ", ".join(f"arr.{f}.into_raw_vec_and_offset().0" for f in fields)
            ret_expr = f"Ok(({tup}))"
        else:
            ret_expr = "Ok(arr.into_raw_vec_and_offset().0)"
        return (
            f"{head}\n"
            f"        {body_core}\n"
            f"            .map(|arr| {ret_expr})\n"
            f"            .map_err(|e| {err})\n"
            f"    }}\n"
        )


def emit_py_node_file(inds: list[dict], lang: str) -> str:
    header = (
        f"// GENERATED FILE — do not edit by hand.\n"
        f"// Source of truth: docs/indicator_registry.json (ffi block).\n"
        f"// Regenerate with: python3 scripts/gen_binding.py --lang {lang} --generate <path>\n"
    )
    if lang == "python":
        # pyo3 `#[pymethods]` require being inside an `impl` block on a struct.
        # For a clean swap, paste this module's `impl AlphaTaGenerated` block
        # into the binding (or rename to the existing struct) and drop the
        # hand-written indicator methods.
        prelude = (
            "use pyo3::prelude::*;\n"
            "use alpha_ta_core::indicators;\n"
            "use alpha_ta_core::math::moving_avg;\n"
            "use alpha_ta_core::patterns::candlestick;\n\n"
            "struct AlphaTaGenerated;\n\n"
            "#[pymethods]\n"
            "impl AlphaTaGenerated {\n"
        )
        fns = "\n".join(emit_py_node_indicator(i["ffi"], lang) for i in inds)
        return header + "\n" + prelude + fns + "}\n"
    else:
        # napi uses free `#[napi] fn` items directly.
        body = "\n".join(emit_py_node_indicator(i["ffi"], lang) for i in inds)
        return header + "\n\n" + body + "\n"


# ──────────────────────────────────────────────────────────────────────────
# C binding lib.rs rewrite
# ──────────────────────────────────────────────────────────────────────────
def find_ta_functions(src: str) -> list[tuple[int, int, str]]:
    """Return [(start, end_exclusive, name), ...] for each `pub unsafe extern
    \"C\" fn ta_*` function, with brace-matched span."""
    out = []
    header_re = re.compile(r"pub\s+unsafe\s+extern\s+\"C\"\s+fn\s+(ta_[A-Za-z0-9_]+)\s*\(")
    for m in header_re.finditer(src):
        name = m.group(1)
        # find signature close
        depth = 0
        i = m.end() - 1
        while i < len(src):
            if src[i] == "(":
                depth += 1
            elif src[i] == ")":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        j = src.find("{", i)
        depth = 0
        k = j
        while k < len(src):
            if src[k] == "{":
                depth += 1
            elif src[k] == "}":
                depth -= 1
                if depth == 0:
                    break
            k += 1
        out.append((m.start(), k + 1, name))
    return out


def rewrite_cbinding(inds: list[dict]) -> tuple[str, str]:
    reg_cnames = {i["ffi"]["c_name"] for i in inds}
    src = C_LIB.read_text(encoding="utf-8")
    fns = find_ta_functions(src)
    spans = [(s, e, n) for (s, e, n) in fns if n in reg_cnames]
    keep_spans = [(s, e, n) for (s, e, n) in fns if n not in reg_cnames]

    if not spans:
        raise SystemExit("No registry-matched indicator functions found in lib.rs")

    # Build new lib.rs: copy everything, drop indicator spans, insert include
    # at the position of the first dropped span.
    first_start = min(s for s, _, _ in spans)
    insert = '\ninclude!("generated.rs");\n'
    result = []
    cursor = 0
    inserted = False
    drop_set = {(s, e) for s, e, _ in spans}
    for s, e, n in fns:
        if (s, e) in drop_set:
            if not inserted and s >= first_start:
                result.append(src[cursor:s])
                result.append(insert)
                inserted = True
            cursor = e
        # keep_spans are copied naturally (we only skip drop spans)
    result.append(src[cursor:])
    if not inserted:
        # all drops were before first_start (shouldn't happen) — append include
        result.append(insert)
    new_lib = "".join(result)
    return emit_c_file(inds), new_lib


# ──────────────────────────────────────────────────────────────────────────
# --check : compare generated function set/params vs hand-written binding
# ──────────────────────────────────────────────────────────────────────────
def check_against_handwritten(lang: str, inds: list[dict]) -> int:
    # Infrastructure functions intentionally NOT in the indicator registry
    # (kept hand-written); never treat them as drift.
    KNOWN_INFRA = {
        "ta_version", "ta_last_error", "ta_last_error_code", "ta_ffi_panic_test",
    }
    if lang == "c":
        src = C_LIB.read_text(encoding="utf-8")
        present = {n for _, _, n in find_ta_functions(src)}
        reg = {i["ffi"]["c_name"] for i in inds}
        missing = reg - present
        extra = present - reg - KNOWN_INFRA
        print(f"[check/c] hand-written ta_* count : {len(present)}")
        print(f"[check/c] registry indicator count : {len(reg)}")
        print(f"[check/c] missing in lib.rs         : {sorted(missing) if missing else 'none'}")
        print(f"[check/c] unexpected in lib.rs      : {sorted(extra) if extra else 'none'}")
        return 0 if not missing and not extra else 1
    else:
        # python/node: parse hand-written `#[pymethods]/#[napi] fn <name>(`
        binding = ROOT / f"ffi/{lang}-binding" / "src" / "lib.rs"
        src = binding.read_text(encoding="utf-8")
        present = set(re.findall(r"fn\s+([a-z][a-z0-9_]+)\s*\(", src))
        reg = {i["ffi"]["c_name"][len("ta_"):] for i in inds}
        missing = reg - present
        extra = present - reg
        print(f"[check/{lang}] hand-written fn count : {len(present)}")
        print(f"[check/{lang}] registry indicator count : {len(reg)}")
        print(f"[check/{lang}] not generated (missing): {sorted(missing)[:20] if missing else 'none'}")
        print(f"[check/{lang}] not in registry (extra) : {sorted(extra)[:20] if extra else 'none'}")
        return 0 if not missing else 1


def main() -> int:
    args = sys.argv[1:]
    lang = None
    mode_generate = None
    do_rewrite = False
    do_check = False
    for a in args:
        if a == "--lang":
            pass
        elif a.startswith("--lang="):
            lang = a.split("=", 1)[1]
        elif a == "--generate":
            pass
        elif a.startswith("--generate="):
            mode_generate = a.split("=", 1)[1]
        elif a == "--rewrite-cbinding":
            do_rewrite = True
        elif a == "--check":
            do_check = True
    # simple positional-ish parsing for --lang X --generate Y
    if lang is None and "--lang" in args:
        lang = args[args.index("--lang") + 1]
    if mode_generate is None and "--generate" in args:
        mode_generate = args[args.index("--generate") + 1]

    if lang not in ("c", "python", "node"):
        print("usage: gen_binding.py --lang c|python|node [--generate PATH|--rewrite-cbinding|--check]")
        return 2

    reg = load_registry()
    inds = indicators_with_ffi(reg)

    if do_check:
        return check_against_handwritten(lang, inds)

    if lang == "c":
        if do_rewrite:
            gen, new_lib = rewrite_cbinding(inds)
            (ROOT / "ffi" / "c-binding" / "src" / "generated.rs").write_text(
                gen, encoding="utf-8"
            )
            C_LIB.write_text(new_lib, encoding="utf-8")
            print(f"[gen/c] wrote ffi/c-binding/src/generated.rs ({len(inds)} indicators)")
            print(f"[gen/c] rewrote ffi/c-binding/src/lib.rs (indicator fns now include!-d)")
            return 0
        out = emit_c_file(inds)
    elif lang == "python":
        out = emit_py_node_file(inds, "python")
    else:
        out = emit_py_node_file(inds, "node")

    if mode_generate:
        Path(mode_generate).write_text(out, encoding="utf-8")
        print(f"[gen/{lang}] wrote {mode_generate} ({len(inds)} indicators)")
    else:
        print(out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
