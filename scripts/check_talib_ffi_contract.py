#!/usr/bin/env python3
"""Keep `core/src/talib_ffi.rs` honest about the TA-Lib C API it transcribes.

Why this exists
---------------
`core/src/talib_ffi.rs` is a hand-written transcription of TA-Lib's `ta_func.h`,
compiled only under the `talib-c` feature and used by the head-to-head benchmark
in `core/benches/talib_c_comparison.rs`. Nothing checked it against the library
it claims to mirror, so three separate defects accumulated in one file:

1. **Phantom declarations.** `TA_SKEWNESS` and `TA_KURTOSIS` were declared as if
   TA-Lib exported them. Neither name occurs in `ta_func.h` for v0.6.4 *or* for
   the pinned v0.8.1 — `grep -ci skewness|kurtosis` returns 0 against both. They
   went unnoticed because an unused `extern` declaration never reaches the
   linker: the benchmark built and passed while advertising a parity that has no
   upstream. Two doc comments in `core/src/indicators/statistics.rs` repeated the
   claim, telling a reader to cross-check against a function that does not exist.

2. **A cited release that cannot be the source.** The module header said "TA-Lib
   0.6.4", but the file binds `TA_PERCENTRANK`, which is absent from 0.6.4 and
   present in v0.8.1. The version named was provably not the one the declarations
   came from.

3. **Hand-written counts that disagree with each other and with the file.** The
   header claimed "all 158 functions" while its own category list summed to 161
   and the file declared 160; every section comment carried a count matching
   neither TA-Lib's grouping nor the declarations beneath it.

What is checked
---------------
* Every `fn TA_*` declaration must name a function in the pinned catalog
  (`tests/contracts/talib_coverage_matrix_v1.json` → `numeric_reference`, which
  is exactly TA-Lib v0.8.1's 201-function export set). This is what rejects a
  phantom extern, and it works offline because the catalog is committed.
* The non-indicator declarations must be exactly `TA_Initialize` and
  `TA_Shutdown`. A new unclassified extern then has to be acknowledged here
  rather than silently counted as an indicator.
* The header's stated indicator total must equal the number actually declared.
* Every `// [declared=N] <section>` comment must equal the number of
  declarations in its section.

Usage
-----
    python scripts/check_talib_ffi_contract.py
    python scripts/check_talib_ffi_contract.py --verbose
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "core" / "src" / "talib_ffi.rs"
MATRIX = ROOT / "tests" / "contracts" / "talib_coverage_matrix_v1.json"

# `fn TA_FOO(` — the transcription uses bare `pub fn` inside an `extern "C"`
# block, so there is no `extern` keyword to anchor on.
DECL_RE = re.compile(r"\bfn (TA_[A-Za-z0-9_]+)")

# Non-indicator entry points TA-Lib exports alongside the indicators.
LIFECYCLE = frozenset({"TA_Initialize", "TA_Shutdown"})

# `//! ... declares **158** indicator entry points ...`
HEADER_COUNT_RE = re.compile(
    r"declares\s+\*\*(?P<n>\d+)\*\*\s+indicator\s+entry\s+points"
)

# `// [declared=13] Overlap Studies — ...`
SECTION_RE = re.compile(r"//\s*\[declared=(?P<n>\d+)\]\s*(?P<label>.*)")


def pinned_catalog() -> set[str]:
    """The TA-Lib v0.8.1 export set, as committed in the coverage matrix."""
    data = json.loads(MATRIX.read_text(encoding="utf-8"))
    names = data["surfaces"]["numeric_reference"]["indicators"]
    return {n.upper() for n in names}


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--verbose", action="store_true", help="list every declaration")
    args = ap.parse_args()

    for path in (SOURCE, MATRIX):
        if not path.is_file():
            print(f"FAILED: missing {path.relative_to(ROOT)}")
            return 1

    text = SOURCE.read_text(encoding="utf-8")
    declared = sorted(set(DECL_RE.findall(text)))
    indicators = [n for n in declared if n not in LIFECYCLE]
    lifecycle = sorted(n for n in declared if n in LIFECYCLE)
    catalog = pinned_catalog()

    errors: list[str] = []

    if args.verbose:
        for name in declared:
            kind = "lifecycle" if name in LIFECYCLE else "indicator"
            mark = "ok" if name[3:].upper() in catalog else "NOT-IN-CATALOG"
            print(f"  {name:<28} {kind:<10} {mark}")

    # 1. No extern may name a function the pinned TA-Lib release does not export.
    phantom = [n for n in indicators if n[3:].upper() not in catalog]
    if phantom:
        errors.append(
            "declared but absent from the pinned TA-Lib catalog "
            f"({len(phantom)}): " + ", ".join(phantom)
        )

    # 2. The non-indicator set is closed: a new one must be classified on purpose.
    unexpected = sorted(set(n for n in declared if n in LIFECYCLE) ^ LIFECYCLE)
    missing_lifecycle = sorted(LIFECYCLE - set(declared))
    if missing_lifecycle:
        errors.append(
            "TA-Lib lifecycle entry points no longer declared: "
            + ", ".join(missing_lifecycle)
        )
    if unexpected:
        errors.append(
            "unclassified non-indicator declaration(s): " + ", ".join(unexpected)
        )

    # 3. The header's stated total must match the file.
    header = HEADER_COUNT_RE.search(text)
    if not header:
        errors.append(
            "module header no longer states a machine-checkable indicator total; "
            "expected a line matching "
            "'declares **N** indicator entry points'"
        )
    else:
        stated = int(header.group("n"))
        if stated != len(indicators):
            errors.append(
                f"module header claims {stated} indicator entry points but "
                f"{len(indicators)} are declared"
            )

    # 4. Each section comment must match its own section.
    marks = [
        (m.start(), int(m.group("n")), m.group("label").strip())
        for m in SECTION_RE.finditer(text)
    ]
    if not marks:
        errors.append("no `// [declared=N]` section comments found")
    total_from_sections = 0
    for idx, (pos, n, label) in enumerate(marks):
        end = marks[idx + 1][0] if idx + 1 < len(marks) else len(text)
        seg = text[pos:end]
        # The first declaration belongs to this section, so exclude nothing:
        # the comment itself sits above its own block.
        actual = len({m for m in DECL_RE.findall(seg)})
        total_from_sections += n
        if actual != n:
            errors.append(
                f"section '[declared={n}] {label[:48]}' covers {actual} declaration(s)"
            )
    if marks and total_from_sections != len(indicators):
        errors.append(
            f"section counts sum to {total_from_sections} but "
            f"{len(indicators)} indicators are declared"
        )

    if errors:
        print("TA-Lib FFI contract violations:", file=sys.stderr)
        for e in errors:
            print(f"  - {e}", file=sys.stderr)
        return 1

    print(
        f"OK: {len(indicators)} TA-Lib indicator declarations all present in the "
        f"pinned catalog ({len(catalog)} functions), plus {len(lifecycle)} "
        f"lifecycle calls; header total and {len(marks)} section counts agree"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
