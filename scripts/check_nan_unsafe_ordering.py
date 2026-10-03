#!/usr/bin/env python3
"""Forbid `partial_cmp(...).unwrap()` on floats -- it panics on NaN.

Why this exists
---------------
`f64::partial_cmp` returns `Option<Ordering>`: it is `None` whenever either
operand is `NaN`. Calling `.unwrap()` on that `None` **panics**, which turns a
missing/gap value (a NaN in the input series -- the normal way real market data
arrives) into a process abort instead of a NaN in the output.

That is the wrong failure mode for this codebase. Everywhere else NaN is a
value that *propagates* (`null_policy: "nan"`, `is_nan()` guards, and the
explicit `test_detect_peaks_nan_skipped` test in `patterns/common.rs`), so a
`partial_cmp(...).unwrap()` that aborts the whole evaluation is both a crash and
an inconsistency.

The idiomatic replacement is `f64::total_cmp`, a total order that never returns
`None` (NaN sorts to a defined end). It is already the house style -- see
`factors.rs`, `performance.rs`, `math/quantile.rs`, `math/rank.rs`,
`math/regression.rs`, `features/selection.rs`, `features/importance.rs`,
`formula/ops/cross_sectional.rs`. This gate keeps the outliers from coming back.

What is *not* flagged
---------------------
`.partial_cmp(...).unwrap_or(Ordering::Equal)` is NaN-safe (it yields a value,
not a panic) and is left alone; only the bare `.unwrap()` is a defect.

Deliberate exceptions
---------------------
A line may opt out with a trailing `// nan-safe-ok` comment when the operands
are provably finite and the panic is genuinely intended. Like the other
allowlists in this repository (`MANUAL_TOOLS` in `check_orphan_scripts.py`,
`RECORDED_MISSING` in `check_script_references.py`), the exception is permitted
but has to be a deliberate, reviewable edit.

Exit codes
----------
0  no `partial_cmp(...).unwrap()` in the scanned Rust sources
1  at least one
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# Same roots the sibling hygiene gates scan.
SCAN_ROOTS = ("core", "factor-analysis", "visualization", "cli", "ffi", "wasm")

# `x.partial_cmp(y).unwrap()` and `x.partial_cmp(&y).unwrap()`, on one line.
# `[^)]*` matches the argument; `\.unwrap\(\)` (empty parens) deliberately does
# NOT match `unwrap_or(..)`, which is NaN-safe.
OFFENDER = re.compile(r"partial_cmp\([^)]*\)\s*\.unwrap\(\)")

# A deliberate, reviewable opt-out.
ALLOW_MARKER = "nan-safe-ok"


def tracked_files() -> list[str]:
    # `-z` is required: plain `git ls-files` octal-escapes non-ASCII paths and
    # `Path.is_file()` then drops them silently. `encoding` is mandatory on
    # Windows or non-ASCII paths crash the reader thread (stdout becomes None).
    out = subprocess.run(
        ["git", "ls-files", "-z"],
        capture_output=True,
        encoding="utf-8",
        errors="replace",
        check=True,
        cwd=ROOT,
    ).stdout
    return [
        line
        for line in out.split("\0")
        if line and (ROOT / line).is_file() and line.startswith(SCAN_ROOTS) and line.endswith(".rs")
    ]


def main() -> int:
    files = tracked_files()
    offenders: list[tuple[str, int, str]] = []

    for rel in files:
        lines = (ROOT / rel).read_text(encoding="utf-8", errors="replace").splitlines()
        for i, line in enumerate(lines):
            # Match only the code portion: a comment that *describes* the
            # anti-pattern (e.g. a regression test's `// ... partial_cmp .. unwrap`)
            # must not be reported. The opt-out marker is a comment, so it is
            # checked against the whole line.
            code = line.split("//", 1)[0]
            if not OFFENDER.search(code):
                continue
            if ALLOW_MARKER in line:
                continue
            offenders.append((rel, i + 1, line.strip()))

    print(f"scanned {len(files)} Rust files; {len(offenders)} `partial_cmp(..).unwrap()` site(s)")

    if not offenders:
        print("OK: float ordering uses a NaN-safe comparator (total_cmp / unwrap_or)")
        return 0

    print("\n`partial_cmp(..)` is `None` when an operand is NaN, so `.unwrap()`")
    print("panics on a gap value instead of propagating NaN. Use `total_cmp`")
    print("(never None) or `.unwrap_or(Ordering::Equal)`. Add a trailing")
    print(f"`// {ALLOW_MARKER}` comment only if the operands are provably finite:\n")
    for rel, line_no, text in offenders:
        print(f"  {rel}:{line_no}: {text}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
