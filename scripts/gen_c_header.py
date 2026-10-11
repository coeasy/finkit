#!/usr/bin/env python3
"""Registry-driven C header generator for Finkit's C FFI.

Single source of truth: docs/indicator_registry.json (each indicator that is
exposed over C carries an `ffi` block produced by scripts/enrich_registry_ffi.py).
This script emits ffi/c-binding/include/finkit.h:

  - the stable ABI boilerplate (include guard, TA_API macros, FfiStatus enum,
    the error-tier documentation, and the `char *` JSON entry points) — fixed
    template;
  - one `TA_API ta_result_t <c_name>(...)` declaration per registry indicator
    that has an `ffi` block, grouped by `ffi.doc_group`;
  - a footer that records why K-line visualization is **not** in the C ABI.

Usage:
    python scripts/gen_c_header.py --generate [PATH]   # write header (default: finkit.h)
    python scripts/gen_c_header.py --check    [PATH]   # fail if header != generated

`--check` is wired into CI so the committed header can never silently drift
from the registry. It runs two comparisons, because one was not enough: the
indicator-signature comparison cannot see any declaration that does not return
`ta_result_t`, and `--generate` was both dropping the `char *` JSON family and
injecting eleven `finkit_kline_*` names that exist nowhere in Rust. See
`check_generate_is_lossless`.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REGISTRY = ROOT / "docs/indicator_registry.json"
FFI_REGISTRY = ROOT / "docs/ffi_registry.json"
DEFAULT_HEADER = ROOT / "ffi/c-binding/include/finkit.h"

# Canonical section order (mirrors finkit.h).
GROUPS = [
    "Moving averages & overlays",
    "Momentum & oscillators",
    "Volatility & volume",
    "Hilbert transform",
    "Statistics & price transforms",
    "Candlestick patterns",
    "Chart patterns (FTA-native)",
]

HEAD = """#ifndef FINKIT_H
#define FINKIT_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>

#ifdef _WIN32
  #ifdef FINKIT_EXPORTS
    #define TA_API __declspec(dllexport)
  #else
    #define TA_API __declspec(dllimport)
  #endif
#else
  #define TA_API __attribute__((visibility("default")))
#endif

typedef int32_t ta_result_t;

/**
 * Stable ABI error classification (`#[repr(i32)]` in lib.rs).
 * Detailed codes are available via ta_last_error_code().
 */
typedef enum FfiStatus {
    FfiStatus_Ok = 0,
    FfiStatus_NullPointer = -1,
    FfiStatus_InvalidParameter = -2,
    FfiStatus_InsufficientData = -3,
    FfiStatus_InternalError = -4,
    FfiStatus_InvalidUtf8 = -5,
    FfiStatus_Unknown = -99
} FfiStatus;

/*
 * NOTE: the legacy indicator entry points (ta_sma, ta_rsi, ...) do NOT return
 * an FfiStatus. Their return value uses the older pair
 * TA_ERR_INVALID_INPUT (-1) / TA_ERR_CALCULATION (-2), which collides
 * numerically with FfiStatus_NullPointer (-1) / FfiStatus_InvalidParameter (-2)
 * while meaning something different -- -1 there covers any rejected argument
 * (null pointer, len == 0, period > len alike), not only a null pointer. Treat
 * the return value as a coarse "zero or not" test and read the envelope or
 * ta_last_error_code() when you need a classification.
 */

/*
 * Finer-grained tiers reported by ta_last_error_code(), which may carry a
 * positive code while the function return value carries a legacy negative one.
 *
 *   1 .. 2    FFI boundary (null pointer, buffer too small)
 *  10 .. 13   Indicator tier  (FFI_INDICATOR_BASE + offset)
 *  50 .. 61   Formula tier    (FFI_FORMULA_BASE + offset)
 *
 * The formula tier is 50 + formula_error_code(variant), and that mapping has 12
 * arms (offsets 0..11), so it ends at 61. Two entries matter to a C caller:
 *
 *   60  UnsupportedFunction  the formula names a function this build cannot run
 *   61  BackendUnsupported   the caller selected FormulaExecutionMode::Plan on
 *                            an entry point that only the tree backend serves
 *                            (for example eval_ast, eval_lazy, eval_template,
 *                            eval_with_params)
 *
 * 61 is a *programming* error, not a data error: the input was fine, the
 * requested backend simply cannot serve that entry point. Retry with the tree
 * backend, or call the plan-capable ta_formula_eval_contract_json. See
 * docs/ffi/error-codes.md and docs/formula-runtime-contract.md.
 */

/* ── Version & error reporting ─────────────────────────────────────────── */

TA_API char *ta_version(void);
TA_API char *ta_operation_catalog_json(void);
TA_API char *ta_factor_catalog_json(void);
TA_API char *ta_operation_execute_json(const char *request_json);
TA_API char *ta_factor_execute_json(const char *request_json);
TA_API char *ta_factor_cross_sectional_execute_json(const char *request_json);
TA_API char *ta_factor_stream_execute_json(const char *request_json);
TA_API char *ta_composite_execute_json(const char *request_json);
TA_API char *ta_composite_stream_execute_json(const char *request_json);
TA_API char *ta_formula_eval_contract_json(const char *source, const char *dialect,
    const double *open, const double *high, const double *low, const double *close,
    const double *volume, int32_t length);
TA_API char *ta_formula_eval_temporal_contract_json(const char *request_json);
TA_API char *ta_formula_eval_panel_contract_json(const char *request_json);
TA_API char *ta_formula_eval_cross_sectional_contract_json(const char *request_json);
TA_API char *ta_formula_stream_execute_json(const char *request_json);
TA_API char *ta_formula_compatibility_report_json(const char *source, const char *terminal);
TA_API char *ta_last_error(void);
TA_API int32_t ta_last_error_code(void);
TA_API void finkit_free_string(char *s);

"""

# K-line visualization is deliberately **not** part of the C ABI.
#
# This template used to emit eleven `finkit_kline_*` declarations
# (`finkit_kline_data_new`, `finkit_kline_chart_add_ma`, ...). None of them
# exists: `grep -rn finkit_kline_ --include=*.rs` matches nothing in the whole
# workspace. The K-line surface is implemented in the Python, Node and Java
# bindings only, which is exactly what the committed header says in the comment
# this footer writes. So `make gen-c-header` was not merely incomplete — it
# would inject eleven declarations for functions that cannot be linked, into a
# header that `gen_c_header.py --check` then certified as correct, because the
# signature comparison only looked at `TA_API ta_result_t ta_*(...)` and the
# phantom names start with `finkit_`.
FOOTER = """/* K-line visualization is exposed by the Python/Node bindings. */

#ifdef __cplusplus
}
#endif

#endif /* FINKIT_H */
"""


def load_registry() -> dict:
    reg = json.loads(REGISTRY.read_text(encoding="utf-8"))
    if not FFI_REGISTRY.exists():
        return reg
    ffi = json.loads(FFI_REGISTRY.read_text(encoding="utf-8"))
    for item in ffi.get("indicators", []):
        name = item.get("name")
        if not name:
            continue
        target = next((x for x in reg["indicators"] if x.get("name") == name), None)
        if target is None:
            target = {"name": name}
            reg.setdefault("indicators", []).append(target)
        target["ffi"] = item.get("ffi", {})
    return reg


def emit_decl(ffi: dict) -> str:
    c_name = ffi["c_name"]
    ins = ", ".join(f"const {i['c_type']} *{i['name']}" for i in ffi.get("inputs", []))
    outs = ", ".join(f"{o['c_type']} *{o['name']}" for o in ffi.get("outputs", []))
    ps = ", ".join(f"{p['c_type']} {p['name']}" for p in ffi.get("params", []))
    parts = []
    if ins:
        parts.append(ins)
    if outs:
        parts.append(outs)
    parts.append("int32_t len")
    if ps:
        parts.append(ps)
    return f"TA_API ta_result_t {c_name}({', '.join(parts)});"


def generate() -> str:
    reg = load_registry()
    by_group: dict[str, list[tuple[int, str]]] = {g: [] for g in GROUPS}
    for ind in reg.get("indicators", []):
        ffi = ind.get("ffi")
        if not ffi:
            continue
        by_group.setdefault(ffi.get("doc_group", ""), []).append(
            (ffi.get("order", 0), emit_decl(ffi))
        )

    lines = HEAD.splitlines()
    for group in GROUPS:
        items = by_group.get(group)
        if not items:
            continue
        items.sort(key=lambda t: t[0])
        dash = "─" * max(0, 78 - 6 - len(group) - 3)
        lines.append("")  # blank separator before each section
        lines.append(f"/* ── {group} {dash} */")
        for _, d in items:
            lines.append(d)
    lines.append("")  # blank separator before the footer
    lines.extend(FOOTER.splitlines())
    return "\n".join(lines) + "\n"


FN_RE = re.compile(r"TA_API\s+ta_result_t\s+(ta_\w+)\s*\((.*?)\)\s*;", re.DOTALL)


def signatures_of(text: str) -> dict[str, str]:
    sigs = {}
    for m in FN_RE.finditer(text):
        cname = m.group(1)
        norm = re.sub(r"\s+", "", m.group(2))
        sigs[cname] = norm
    return sigs


C_BINDING_SRC = ROOT / "ffi/c-binding/src"
C_BINDING_INCLUDE = ROOT / "ffi/c-binding/include"

# `#[no_mangle] pub [unsafe] extern "C" fn <name>`, together with any attribute
# run immediately above it (so a `#[cfg(test)]` guard can be detected).
NO_MANGLE_RE = re.compile(
    r"(?P<attrs>(?:#\[[^\]]*\]\s*)*)"
    r'#\[no_mangle\]\s*pub\s+(?:unsafe\s+)?extern\s+"C"\s+fn\s+(?P<name>\w+)'
)
CFG_TEST_RE = re.compile(r"#\[cfg\(\s*test\s*\)\]")

# A declaration in a C/C++ public header: `<MACRO> <return type> <name>(`.
# Covers the generated indicator set (`TA_API ta_result_t ta_*`), the fixed
# template API (`TA_API ta_version`, `TA_API void finkit_free_string`, ...) and
# the research surface (`FINKIT_RESEARCH_API char *finkit_factor_study_json`).
DECL_RE = re.compile(
    r"^\s*(?:TA_API|FINKIT_RESEARCH_API)\s+[A-Za-z_][\w\s\*]*?"
    r"\b(?P<name>(?:ta|finkit)_\w+)\s*\(",
    re.MULTILINE,
)


def rust_c_exports() -> dict[str, str]:
    """Every `#[no_mangle] extern "C"` symbol that ships in a release build.

    Test-only exports (`#[cfg(test)]`) are excluded: they are intentionally
    absent from the shipped DLL.
    """
    out: dict[str, str] = {}
    for f in sorted(C_BINDING_SRC.glob("*.rs")):
        text = f.read_text(encoding="utf-8", errors="replace")
        for m in NO_MANGLE_RE.finditer(text):
            if CFG_TEST_RE.search(m.group("attrs") or ""):
                continue
            out[m.group("name")] = f.name
    return out


def header_c_decls() -> dict[str, str]:
    """Every C FFI function declared across `ffi/c-binding/include/*.h`."""
    out: dict[str, str] = {}
    for f in sorted(C_BINDING_INCLUDE.glob("*.h")):
        text = f.read_text(encoding="utf-8", errors="replace")
        for m in DECL_RE.finditer(text):
            out[m.group("name")] = f.name
    return out


def check_c_binding() -> bool:
    """The Rust C-binding must export exactly the functions the headers declare.

    `ffi/c-binding/src/generated.rs` used to be emitted by `gen_binding.py`,
    which now refuses to run: its input was `docs/indicator_registry.json`'s
    `ffi` block, and that metadata moved to `docs/ffi_registry.json`. Nothing
    checked the artifact afterwards, so a function could disappear from the Rust
    side while the committed header kept promising it. Compare the two directly
    instead of trusting that the frozen file is still whole.

    The comparison spans *all* of `ffi/c-binding/src/*.rs`, not just
    `generated.rs`: the fixed-template entry points (`ta_version`,
    `ta_last_error`, `ta_factor_execute_json`, ...) live in `lib.rs` and the
    research surface in `research.rs`, and both are declared in the headers.
    """
    if not C_BINDING_SRC.is_dir():
        print(f"[check] FAILED: missing {C_BINDING_SRC}")
        return False

    exported = rust_c_exports()
    heads = header_c_decls()
    if not exported or not heads:
        print(f"[check] FAILED: empty export set (rust={len(exported)}, headers={len(heads)})")
        return False

    missing = sorted(set(heads) - set(exported))  # declared but not exported
    extra = sorted(set(exported) - set(heads))  # exported but undeclared
    if not missing and not extra:
        print(f"[check] OK: {len(exported)} C-binding exports match the headers")
        return True
    if missing:
        print(f"[check] C-binding MISSING exports (declared, not exported): {missing}")
    if extra:
        print(f"[check] C-binding EXTRA exports (exported, not declared): {extra}")
    print(f"[check] FAILED: headers({len(heads)}) vs c-binding({len(exported)})")
    return False


def check(header_path: Path) -> bool:
    generated = generate()
    current = header_path.read_text(encoding="utf-8")
    a = signatures_of(generated)
    b = signatures_of(current)
    if a == b:
        print(f"[check] OK: {len(a)} indicator signatures match (ok)")
        return check_c_binding()
    missing = set(a) - set(b)
    extra = set(b) - set(a)
    differing = {k for k in set(a) & set(b) if a[k] != b[k]}
    if missing:
        print(f"[check] MISSING in header: {sorted(missing)}")
    if extra:
        print(f"[check] EXTRA in header: {sorted(extra)}")
    if differing:
        for k in sorted(differing):
            print(f"[check] DIFFERS {k}:\n   gen: ({a[k]})\n   hdr: ({b[k]})")
    print(f"[check] FAILED: generated({len(a)}) vs header({len(b)})")
    return False


def check_generate_is_lossless(header_path: Path) -> bool:
    """`--generate` and the committed header must declare the same names.

    `signatures_of` only sees `TA_API ta_result_t ta_*(...)`, so every other
    family was invisible to `check()`. Two defects hid in that blind spot, in
    opposite directions:

    * **Dropped.** `generate()`'s template omitted the fourteen `char *` JSON
      entry points (`ta_operation_catalog_json`, `ta_formula_eval_contract_json`,
      ...), so `make gen-c-header` deleted them from a shipped header.
    * **Phantom.** The same template emitted eleven `finkit_kline_*`
      declarations for functions that exist nowhere in Rust, so regenerating
      injected declarations that cannot link.

    Both directions are therefore checked: a name on only one side is a defect,
    whichever side it is on.
    """
    generated = generate()
    current = header_path.read_text(encoding="utf-8")
    gen_names = {m.group("name") for m in DECL_RE.finditer(generated)}
    hdr_names = {m.group("name") for m in DECL_RE.finditer(current)}
    dropped = sorted(hdr_names - gen_names)
    phantom = sorted(gen_names - hdr_names)
    if not dropped and not phantom:
        print(f"[check] OK: --generate reproduces all {len(hdr_names)} header declarations")
        return True
    if dropped:
        print("[check] `--generate` would DROP these declarations (add them to the template):")
        for name in dropped:
            print(f"  - {name}")
    if phantom:
        print("[check] `--generate` would INJECT these undeclared names (fix the template):")
        for name in phantom:
            print(f"  - {name}")
    print(f"[check] FAILED: generate()={len(gen_names)} vs header={len(hdr_names)}")
    return False


def main() -> None:
    ap = argparse.ArgumentParser(description="Generate Finkit C FFI header from registry")
    ap.add_argument("--generate", nargs="?", const=str(DEFAULT_HEADER), default=None,
                    metavar="PATH", help="write the generated header")
    ap.add_argument("--check", nargs="?", const=str(DEFAULT_HEADER), default=None,
                    metavar="PATH", help="verify header matches generation")
    args = ap.parse_args()

    if args.check is not None:
        # Both directions matter and they fail for different reasons: `check`
        # compares indicator signatures, `check_generate_is_lossless` catches a
        # declaration the template cannot reproduce.
        ok = check(Path(args.check))
        ok = check_generate_is_lossless(Path(args.check)) and ok
        sys.exit(0 if ok else 1)
    if args.generate is not None:
        Path(args.generate).write_text(generate(), encoding="utf-8")
        print(f"[generate] wrote {args.generate}")
        return
    # default: print to stdout
    print(generate())


if __name__ == "__main__":
    main()
