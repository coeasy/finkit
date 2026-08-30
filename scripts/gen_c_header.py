#!/usr/bin/env python3
"""Registry-driven C header generator for AlphaTA's C FFI.

Single source of truth: docs/indicator_registry.json (each indicator that is
exposed over C carries an `ffi` block produced by scripts/enrich_registry_ffi.py).
This script emits ffi/c-binding/include/alpha_ta.h:

  - the stable ABI boilerplate (include guard, TA_API macros, FfiStatus enum,
    version/error reporting functions) — fixed template;
  - one `TA_API ta_result_t <c_name>(...)` declaration per registry indicator
    that has an `ffi` block, grouped by `ffi.doc_group`;
  - the K-line visualization API — fixed template (not an indicator).

Usage:
    python scripts/gen_c_header.py --generate [PATH]   # write header (default: alpha_ta.h)
    python scripts/gen_c_header.py --check    [PATH]   # fail if header != generated

`--check` is wired into CI so the committed header can never silently drift
from the registry.
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REGISTRY = ROOT / "docs/indicator_registry.json"
DEFAULT_HEADER = ROOT / "ffi/c-binding/include/alpha_ta.h"

# Canonical section order (mirrors alpha_ta.h).
GROUPS = [
    "Moving averages & overlays",
    "Momentum & oscillators",
    "Volatility & volume",
    "Hilbert transform",
    "Statistics & price transforms",
    "Candlestick patterns",
    "Chart patterns (FTA-native)",
]

HEAD = """#ifndef ALPHA_TA_H
#define ALPHA_TA_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>

#ifdef _WIN32
  #ifdef ALPHA_TA_EXPORTS
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

/* ── Version & error reporting ─────────────────────────────────────────── */

TA_API char *ta_version(void);
TA_API char *ta_last_error(void);
TA_API int32_t ta_last_error_code(void);
TA_API void alpha_ta_free_string(char *s);

"""

KLINE = """/* ── K-line visualization ────────────────────────────────────────────────── */

typedef int64_t alpha_ta_kline_data_t;
typedef int64_t alpha_ta_kline_chart_t;

TA_API alpha_ta_kline_data_t alpha_ta_kline_data_new(
    const char * const *dates,
    const double *opens,
    const double *highs,
    const double *lows,
    const double *closes,
    const double *volumes,
    int32_t len);
TA_API void alpha_ta_kline_data_free(alpha_ta_kline_data_t handle);
TA_API int32_t alpha_ta_kline_data_validate(alpha_ta_kline_data_t handle);

TA_API alpha_ta_kline_chart_t alpha_ta_kline_chart_new(
    alpha_ta_kline_data_t data_handle,
    const char *language,
    const char *title,
    uint32_t width,
    uint32_t height);
TA_API void alpha_ta_kline_chart_free(alpha_ta_kline_chart_t handle);

TA_API int32_t alpha_ta_kline_chart_add_ma(
    alpha_ta_kline_chart_t handle,
    const int32_t *periods,
    int32_t periods_len);
TA_API int32_t alpha_ta_kline_chart_add_macd(
    alpha_ta_kline_chart_t handle,
    int32_t fast,
    int32_t slow,
    int32_t signal);
TA_API int32_t alpha_ta_kline_chart_add_rsi(
    alpha_ta_kline_chart_t handle,
    int32_t period);
TA_API int32_t alpha_ta_kline_chart_add_boll(
    alpha_ta_kline_chart_t handle,
    int32_t period,
    double nb_dev);

TA_API int32_t alpha_ta_kline_chart_save_as_svg(
    alpha_ta_kline_chart_t handle,
    const char *path);
TA_API char *alpha_ta_kline_chart_to_svg(alpha_ta_kline_chart_t handle);

#ifdef __cplusplus
}
#endif

#endif /* ALPHA_TA_H */
"""


def load_registry() -> dict:
    return json.loads(REGISTRY.read_text(encoding="utf-8"))


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
    lines.append("")  # blank separator before K-line section
    lines.extend(KLINE.splitlines())
    return "\n".join(lines) + "\n"


FN_RE = re.compile(r"TA_API\s+ta_result_t\s+(ta_\w+)\s*\((.*?)\)\s*;", re.DOTALL)


def signatures_of(text: str) -> dict[str, str]:
    sigs = {}
    for m in FN_RE.finditer(text):
        cname = m.group(1)
        norm = re.sub(r"\s+", "", m.group(2))
        sigs[cname] = norm
    return sigs


def check(header_path: Path) -> bool:
    generated = generate()
    current = header_path.read_text(encoding="utf-8")
    a = signatures_of(generated)
    b = signatures_of(current)
    if a == b:
        print(f"[check] OK: {len(a)} indicator signatures match ✅")
        return True
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


def main() -> None:
    ap = argparse.ArgumentParser(description="Generate AlphaTA C FFI header from registry")
    ap.add_argument("--generate", nargs="?", const=str(DEFAULT_HEADER), default=None,
                    metavar="PATH", help="write the generated header")
    ap.add_argument("--check", nargs="?", const=str(DEFAULT_HEADER), default=None,
                    metavar="PATH", help="verify header matches generation")
    args = ap.parse_args()

    if args.check is not None:
        ok = check(Path(args.check))
        sys.exit(0 if ok else 1)
    if args.generate is not None:
        Path(args.generate).write_text(generate(), encoding="utf-8")
        print(f"[generate] wrote {args.generate}")
        return
    # default: print to stdout
    print(generate())


if __name__ == "__main__":
    main()
