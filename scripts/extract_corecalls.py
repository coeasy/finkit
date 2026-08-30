#!/usr/bin/env python3
"""Learn per-indicator C-binding metadata from ffi/c-binding/src/lib.rs.

For every `ta_*` indicator wrapper we capture the *structured* pieces needed to
regenerate it from the indicator registry (single source of truth):

    ffi.core_call   : "module::fn"              (e.g. "moving_avg::sma")
    ffi.core_args   : [arg expr, ...]           (e.g. ["data", "period as usize"])
    ffi.c_params    : [{name, ty}, ...]         (verbatim signature params)
    ffi.c_slices    : [{var, ptr}, ...]         (slice::from_raw_parts mapping)
    ffi.c_checks    : [verbatim `if` lines]      (data/param guards, excl. null-check)
    ffi.c_copies    : [verbatim copy lines]      (copy_result / copy_int_result)
    ffi.c_out_kind  : "single" | "struct" | "int"

The null-check is regenerated uniformly by the generator, so it is not stored.
Infrastructure functions (ta_version, kline chart, error getters, ...) that are
not present in the registry's `ffi.c_name` are left untouched (hand-written).

Usage:
    python3 scripts/extract_corecalls.py            # merge into registry, print report
    python3 scripts/extract_corecalls.py --dry-run  # only print report, no write
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LIB = ROOT / "ffi" / "c-binding" / "src" / "lib.rs"
REG = ROOT / "docs" / "indicator_registry.json"


def parse_functions(src: str) -> dict[str, dict]:
    """Return {name: {params, body, ...}} for every `pub unsafe extern "C" fn ta_*`."""
    funcs: dict[str, dict] = {}
    # Match the function header; names may contain underscores and digits.
    header_re = re.compile(
        r"pub\s+unsafe\s+extern\s+\"C\"\s+fn\s+(ta_[A-Za-z0-9_]+)\s*\("
    )
    for m in header_re.finditer(src):
        name = m.group(1)
        # signature spans until the first unescaped ')'
        depth = 0
        i = m.end() - 1  # at '('
        while i < len(src):
            c = src[i]
            if c == "(":
                depth += 1
            elif c == ")":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        sig = src[m.end(): i]  # between '(' and ')'
        # body starts at the first '{' after the closing ')'
        j = src.find("{", i)
        # match braces for the body
        depth = 0
        k = j
        while k < len(src):
            c = src[k]
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
                if depth == 0:
                    break
            k += 1
        body = src[j + 1: k]
        funcs[name] = {"params": parse_params(sig), "body": body}
    return funcs


def parse_params(sig: str) -> list[dict]:
    params = []
    # strip newlines, then split top-level commas
    sig = " ".join(sig.split())
    depth = 0
    cur = ""
    for ch in sig:
        if ch == "<":
            depth += 1
        elif ch == ">":
            depth -= 1
        if ch == "," and depth == 0:
            cur and params.append(cur.strip())
            cur = ""
        else:
            cur += ch
    if cur.strip():
        params.append(cur.strip())
    out = []
    for p in params:
        if ":" not in p:
            continue
        nm, ty = p.split(":", 1)
        out.append({"name": nm.strip(), "ty": ty.strip()})
    return out


def analyze_body(body: str) -> dict:
    res: dict = {
        "slices": [],
        "checks": [],
        "core_call": None,
        "core_args": [],
        "copies": [],
        "out_kind": "single",
    }
    # slices
    for sm in re.finditer(
        r"let\s+(\w+)\s*=\s*slice::from_raw_parts\(\s*(\w+)\s*,\s*len\s+as\s+usize\s*\)\s*;",
        body,
    ):
        res["slices"].append({"var": sm.group(1), "ptr": sm.group(2)})
    # checks: every `if ... { return invalid_input(); }` line, verbatim and in
    # order (null-check, data-size check, scalar param check, ...). The
    # generator re-emits them as-is so behaviour is byte-faithful.
    for cm in re.finditer(r"if\s+(.*?)\s*\{\s*return\s+invalid_input\(\);\s*\}", body, re.S):
        cond = " ".join(cm.group(1).split())
        res["checks"].append(f"if {cond} {{ return invalid_input(); }}")
    # core call
    cm = re.search(r"match\s+([\w:]+)\s*\((.*)\)\s*\{", body, re.S)
    if cm:
        res["core_call"] = cm.group(1)
        args = cm.group(2)
        res["core_args"] = split_args(args)
        # language-agnostic arg kinds: input (ptr) or param (name) + usize flag
        slice_by_var = {s["var"]: s["ptr"] for s in res["slices"]}
        kinds = []
        for a in res["core_args"]:
            a = a.strip()
            usize = " as usize" in a
            base = a.replace(" as usize", "").strip()
            if base in slice_by_var:
                kinds.append({"input_ptr": slice_by_var[base], "usize": usize})
            else:
                kinds.append({"param": base, "usize": usize})
        res["core_arg_kinds"] = kinds
    # copies
    for cp in re.finditer(r"(copy_result|copy_int_result)\([^;]*?\)\s*;", body, re.S):
        line = " ".join(cp.group(0).split())
        res["copies"].append(line)
        if cp.group(1) == "copy_int_result":
            res["out_kind"] = "int"
        elif re.search(r"&result\.\w+", line):
            res["out_kind"] = "struct"
        else:
            res["out_kind"] = "single"
    return res


def split_args(s: str) -> list[str]:
    s = s.strip()
    if not s:
        return []
    depth = 0
    cur = ""
    out = []
    for ch in s:
        if ch in "<(":
            depth += 1
        elif ch in ">)":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur.strip())
    return out


def main() -> int:
    src = LIB.read_text(encoding="utf-8")
    funcs = parse_functions(src)
    reg = json.loads(REG.read_text(encoding="utf-8"))

    by_cname: dict[str, dict] = {}
    for ind in reg.get("indicators", []):
        ff = ind.get("ffi")
        if ff and ff.get("c_name"):
            by_cname.setdefault(ff["c_name"], []).append(ind)

    matched = 0
    skipped_no_func = 0
    updated = 0
    for cname, inds in by_cname.items():
        fn = funcs.get(cname)
        if fn is None:
            skipped_no_func += 1
            continue
        matched += 1
        meta = analyze_body(fn["body"])
        meta["c_params"] = fn["params"]
        # Verbatim function body — the C generator re-emits it as-is, so the
        # regenerated wrapper is guaranteed byte-faithful even for indicators
        # with multi-line / conditional copy logic (e.g. heikin_ashi,
        # three_line_break). The body is self-contained (slice decls + match
        # + ffi_catch_i32 wrapper live inside it).
        meta["c_body"] = fn["body"]
        for ind in inds:
            ind["ffi"].update(meta)
            updated += 1

    print(f"[extract_corecalls] parsed {len(funcs)} ta_* functions from lib.rs")
    print(f"[extract_corecalls] registry c_name entries matched : {matched}")
    print(f"[extract_corecalls] registry entries updated        : {updated}")
    print(f"[extract_corecalls] c_name with no lib.rs function  : {skipped_no_func}")
    # report any parsed fn not present in registry (infra functions, expected)
    registry_cnames = set(by_cname.keys())
    infra = [n for n in funcs if n not in registry_cnames]
    print(f"[extract_corecalls] lib.rs fns NOT in registry (infra, kept hand-written): {len(infra)}")
    if infra:
        print("    " + ", ".join(sorted(infra)[:20]) + (" ..." if len(infra) > 20 else ""))

    if "--dry-run" not in sys.argv:
        REG.write_text(json.dumps(reg, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"[extract_corecalls] wrote {REG}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
