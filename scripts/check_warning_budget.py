#!/usr/bin/env python3
"""Warning budget: the ratchet that keeps clippy output from growing.

V4 recorded ~5.5k clippy warnings on `finkit` core. Silently paying that down
over dozens of commits is how a budget turns into a backlog nobody believes;
what actually works is a ratchet — a committed count per lint, and a gate that
**fails when the number goes up**. Fixing a lint lowers the count and lowers the
baseline in the same commit; nothing is allowed to regress.

Scope is `cargo clippy -p finkit --all-targets`: the core crate, where the
warnings are. The FFI/wasm crates are covered by their own builds, and pulling
them in here would make the gate hostage to toolchains this machine does not
have (the python binding's pyo3 build step, for one).

Usage:
    python scripts/check_warning_budget.py            # check (gate)
    python scripts/check_warning_budget.py --write    # re-baseline after fixes
    python scripts/check_warning_budget.py --json     # machine-readable summary

Exit codes: 0 = within budget, 1 = regression, 2 = clippy could not run.
"""
from __future__ import annotations

import json
import os
import pathlib

import subprocess
import sys

BASELINE = pathlib.Path("scripts/warning_budget_baseline.json")
# `warning: unused variable `x`--> src/a.rs:1:2`  /  `error[E0308]: ...`


def run_clippy() -> tuple[dict[str, int], int, str]:
    """Return (per-lint warning counts, total, raw output tail)."""
    # JSON is the only message format that carries the lint code, and a
    # single-number baseline is not a ratchet: "1200 fewer warnings" and
    # "the same total with 300 new `needless_range_loop`s" look identical.
    cmd = [
        "cargo", "clippy", "-p", "finkit", "--all-targets",
        "--message-format=json", "--offline",
    ]
    # Incremental compilation replays diagnostics cached from an earlier
    # revision, so a build that was already clean can still report a warning
    # that no longer exists in the source (and vice versa). The budget is a
    # source-level ratchet, so pin the input: no incremental, ever.
    env = {**os.environ, "CARGO_INCREMENTAL": "0"}
    try:
        proc = subprocess.run(
            cmd, capture_output=True, text=True, errors="replace", timeout=3600,
            env=env,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return {}, 0, f"clippy could not run: {exc}"
    out = proc.stdout + proc.stderr

    counts: dict[str, int] = {}
    for line in out.splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            continue
        if msg.get("reason") != "compiler-message":
            continue
        detail = msg.get("message") or {}
        if detail.get("level") != "warning":
            continue
        # Only diagnostics that point at a source file in this repo. Without
        # this the count is dominated by toolchain noise — on a sandboxed
        # Windows box, `error deleting lock file for incremental compilation
        # session directory` alone contributed 110 "warnings" that had nothing
        # to do with the code, and the budget's verdict moved with the
        # filesystem's mood.
        if not _points_at_source(detail):
            continue
        code = (detail.get("code") or {}).get("code") or "rustc"
        counts[code] = counts.get(code, 0) + 1
    return counts, sum(counts.values()), out[-4000:]


def _points_at_source(detail: dict) -> bool:
    """Whether any span of the diagnostic names a readable file in this repo.

    Deliberately "any span" rather than "the primary span": lints raised inside
    a macro expansion often carry no `is_primary` span of their own, and
    dropping those would hide ~400 real `clippy::float_cmp` hits. The
    toolchain noise being filtered out (`error deleting lock file for
    incremental compilation session directory`) carries no spans at all.
    """
    for span in detail.get("spans") or []:
        name = span.get("file_name") or ""
        if not name.endswith(".rs"):
            continue
        try:
            if pathlib.Path(name).is_file():
                return True
        except OSError:
            continue
    return False


def main() -> int:
    args = set(sys.argv[1:])
    counts, total, tail = run_clippy()
    if total == 0 and "could not run" in tail:
        print("[check_warning_budget] SKIP:", tail.strip().splitlines()[-1])
        return 2

    if "--json" in args:
        print(json.dumps({"total": total, "by_lint": counts}, indent=2, sort_keys=True))
        return 0

    if "--write" in args:
        BASELINE.write_text(
            json.dumps({"total": total, "by_lint": counts}, indent=2, sort_keys=True)
            + "\n",
            encoding="utf-8",
        )
        print(f"[check_warning_budget] baseline written: {total} warnings")
        for code, n in sorted(counts.items(), key=lambda kv: -kv[1]):
            print(f"  {n:6d}  {code}")
        return 0

    if not BASELINE.exists():
        print(
            "[check_warning_budget] FAIL: no baseline. "
            "Run `python scripts/check_warning_budget.py --write` once and commit it."
        )
        return 1

    base = json.loads(BASELINE.read_text(encoding="utf-8"))
    base_by_lint: dict[str, int] = base.get("by_lint", {})
    base_total: int = base.get("total", 0)

    regressions = []
    for code, n in sorted(counts.items()):
        was = base_by_lint.get(code, 0)
        if n > was:
            regressions.append((code, was, n))
    for code, was in sorted(base_by_lint.items()):
        if code not in counts:
            regressions.append((code, was, 0))

    if total > base_total or regressions:
        print(
            f"[check_warning_budget] FAIL: {total} warnings vs budget {base_total} "
            f"(+{total - base_total}).\n  Regressions (lint, was, now):"
        )
        for code, was, now in regressions:
            print(f"    {code}: {was} -> {now}")
        print("  Fix the lint, or --write the baseline if the change is deliberate.")
        return 1

    print(
        f"[check_warning_budget] OK: {total} warnings, at or under the "
        f"{base_total} budget ({len(counts)} lints)."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
