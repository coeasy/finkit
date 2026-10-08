#!/usr/bin/env python3
"""Source-file size budget: a ratchet on how big a file is allowed to get.

V5 Batch 2 asks for the three largest core files to be split by category.
That is a real refactor and it should happen; this gate is what keeps it from
getting *worse* in the meantime, and it makes "split `functions_legacy.rs`" a
number that can be tracked rather than a note in a plan.

The budget is a per-file line cap, ratcheted: the current offenders are
recorded in `scripts/file_size_budget.json` at their present size, and the gate
fails if any file **grows** past its recorded size. Shrinking a file lowers its
entry in the same commit. A file that is not in the baseline may not exceed
`DEFAULT_CAP`.

Why a cap and not a ban: an awkward but honest 3000-line file that works beats
a prematurely fragmented tree where every indicator's registration lives in a
different module and the "where is this implemented?" question gets harder than
"which line of which file?". The budget's job is to stop the trend, not to
force a split nobody has scoped yet.

Usage:
    python scripts/check_file_size_budget.py            # check (gate)
    python scripts/check_file_size_budget.py --write    # re-baseline
    python scripts/check_file_size_budget.py --top 15   # show the biggest files

Exit codes: 0 = within budget, 1 = a file outgrew its budget.
"""
from __future__ import annotations

import json
import pathlib
import sys

BASELINE = pathlib.Path("scripts/file_size_budget.json")
ROOTS = ("core/src", "ffi", "wasm", "factor-analysis", "visualization", "cli")
# A file absent from the baseline must stay under this.
DEFAULT_CAP = 1500
# Files that legitimately carry generated or table-like bulk.
EXEMPT_SUFFIXES = (".generated.rs",)


def line_counts() -> dict[str, int]:
    counts: dict[str, int] = {}
    for root in ROOTS:
        base = pathlib.Path(root)
        if not base.exists():
            continue
        for path in base.rglob("*.rs"):
            if path.name.endswith(EXEMPT_SUFFIXES):
                continue
            try:
                counts[path.as_posix()] = len(
                    path.read_text(encoding="utf-8", errors="replace").splitlines()
                )
            except OSError:
                continue
    return counts


def main() -> int:
    args = set(sys.argv[1:])
    counts = line_counts()

    if "--top" in args:
        n = 15
        for i, a in enumerate(sys.argv):
            if a == "--top" and i + 1 < len(sys.argv):
                n = int(sys.argv[i + 1])
        for path, lines in sorted(counts.items(), key=lambda kv: -kv[1])[:n]:
            print(f"{lines:6d}  {path}")
        return 0

    if "--write" in args:
        offenders = {
            p: n for p, n in counts.items() if n > DEFAULT_CAP
        }
        BASELINE.write_text(
            json.dumps({"default_cap": DEFAULT_CAP, "files": offenders},
                       indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        print(f"[check_file_size_budget] baseline written: "
              f"{len(offenders)} file(s) above {DEFAULT_CAP} lines")
        for p, n in sorted(offenders.items(), key=lambda kv: -kv[1]):
            print(f"  {n:6d}  {p}")
        return 0

    if not BASELINE.exists():
        print("[check_file_size_budget] FAIL: no baseline. Run with --write once.")
        return 1

    base = json.loads(BASELINE.read_text(encoding="utf-8"))
    cap = base.get("default_cap", DEFAULT_CAP)
    recorded: dict[str, int] = base.get("files", {})

    violations = []
    for path, lines in counts.items():
        allowed = recorded.get(path, cap)
        if lines > allowed:
            violations.append((path, allowed, lines))

    if violations:
        print(
            f"[check_file_size_budget] FAIL: {len(violations)} file(s) outgrew "
            f"their budget (default cap {cap} lines)."
        )
        for path, allowed, lines in sorted(violations, key=lambda v: -(v[2] - v[1])):
            print(f"  {path}: {lines} lines (budget {allowed}, +{lines - allowed})")
        print(
            "  Split the file, or --write the baseline if the growth is "
            "deliberate and reviewed."
        )
        return 1

    tracked = sum(1 for p in counts if p in recorded)
    print(
        f"[check_file_size_budget] OK: {len(counts)} source files within budget "
        f"({tracked} tracked above the {cap}-line default cap)."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
