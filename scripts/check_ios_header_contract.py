#!/usr/bin/env python3
"""Keep the iOS C header honest about what the iOS static library exports.

Why this exists
---------------
`ffi/c-binding/include/*.h` is checked against the Rust C-binding by
`gen_c_header.py --check`. The iOS binding has its own header
(`ffi/ios-binding/include/finkit.h`) and no equivalent gate, so it silently
drifted: `finkit_ios_factor_study_json`, `finkit_ios_factor_study_free_string`
and `finkit_ios_quant_evaluation_json` shipped for months without being declared.

An undeclared export is not a crash — the Swift wrappers reach it through
`@_silgen_name`, which bypasses the header. It is still a defect: a plain C
consumer of the iOS static library, reading the header, cannot see that the
research and quant-evaluation entry points exist at all. The header is the
documented contract, so it must list every shipped symbol.

What counts as a shipped symbol
-------------------------------
`#[no_mangle] pub (unsafe) extern "C" fn`, excluding anything carrying
`#[cfg(test)]` — test-only exports are deliberately absent from a release
build and must not appear in the header.

Usage
-----
    python scripts/check_ios_header_contract.py
    python scripts/check_ios_header_contract.py --verbose
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
IOS_SRC = ROOT / "ffi" / "ios-binding" / "src"
IOS_HEADER = ROOT / "ffi" / "ios-binding" / "include" / "finkit.h"

# Attributes immediately preceding `#[no_mangle]`, so `#[cfg(test)]` can be
# spotted without parsing the whole file.
NO_MANGLE_RE = re.compile(
    r"(?P<attrs>(?:#\[[^\]]*\]\s*)*)"
    r'#\[no_mangle\]\s*pub\s+(?:unsafe\s+)?extern\s+"C"\s+fn\s+(?P<name>\w+)'
)
CFG_TEST_RE = re.compile(r"#\[cfg\(\s*test\s*\)\]")

# A C declaration in the iOS header: `<return type> <name>(`. The iOS binding
# uses two prefixes: the legacy `alpha_ta_*` set and the newer `finkit_ios_*`
# set.
DECL_RE = re.compile(
    r"^\s*[A-Za-z_][\w\s\*]*?\b(?P<name>(?:alpha_ta|finkit_ios)_\w+)\s*\(",
    re.MULTILINE,
)


def rust_exports() -> dict[str, str]:
    """Every `#[no_mangle] extern "C"` symbol the iOS binding ships."""
    out: dict[str, str] = {}
    if not IOS_SRC.is_dir():
        return out
    for f in sorted(IOS_SRC.glob("*.rs")):
        text = f.read_text(encoding="utf-8", errors="replace")
        for m in NO_MANGLE_RE.finditer(text):
            if CFG_TEST_RE.search(m.group("attrs") or ""):
                continue
            out[m.group("name")] = f.name
    return out


def header_decls() -> dict[str, str]:
    """Every iOS C function declared in the committed header."""
    if not IOS_HEADER.is_file():
        return {}
    text = IOS_HEADER.read_text(encoding="utf-8", errors="replace")
    # Strip comments so a name mentioned only in prose is not counted as a
    # declaration. The header documents the free function in a comment block.
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    text = re.sub(r"//[^\n]*", "", text)
    return {m.group("name"): IOS_HEADER.name for m in DECL_RE.finditer(text)}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--verbose", action="store_true", help="list every symbol")
    args = ap.parse_args()

    if not IOS_SRC.is_dir() or not IOS_HEADER.is_file():
        print(f"FAILED: missing {IOS_SRC} or {IOS_HEADER}")
        return 1

    exported = rust_exports()
    declared = header_decls()
    if not exported or not declared:
        print(f"FAILED: empty symbol set (rust={len(exported)}, header={len(declared)})")
        return 1

    undeclared = sorted(set(exported) - set(declared))
    phantom = sorted(set(declared) - set(exported))

    if args.verbose:
        for name in sorted(exported):
            mark = "declared" if name in declared else "UNDECLARED"
            print(f"  {name:<44} {mark}")

    if not undeclared and not phantom:
        print(f"OK: {len(exported)} iOS exports match the header declarations")
        return 0

    if undeclared:
        print("iOS exports with no header declaration (add them to the header):")
        for name in undeclared:
            print(f"  - {name}  ({exported[name]})")
    if phantom:
        print("Header declarations with no iOS export (remove them, or fix the export):")
        for name in phantom:
            print(f"  - {name}")
    print(f"FAILED: header({len(declared)}) vs ios-binding({len(exported)})")
    return 1


if __name__ == "__main__":
    sys.exit(main())
