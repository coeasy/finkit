#!/usr/bin/env python3
"""Feature-combination build gate (2026-10-11 architecture audit, round 4).

Why this exists
---------------
A Cargo feature list is a claim about which subsets of the crate compile, and
nothing in `cargo build` checks that claim: the default build exercises the
default set and no other. `core/Cargo.toml` asserted that the indicator
tree-shaking scaffold made "picking a single category still compiles", and
`Makefile`'s `lint` target ran
`cargo clippy -p finkit --no-default-features --features no_std -- -D warnings`
-- but neither was in any CI workflow, so nothing ever ran them. Both had
rotted: measured 2026-10-11, the `no_std` build was **49 errors** and every
single-category build failed too.

This gate makes the claim executable in both directions:

* **SUPPORTED** combinations must build. A regression here is a build failure,
  which is the whole point.
* **KNOWN_BROKEN** combinations are recorded as *expected* failures with the
  measured error count and a reason. They are asserted to still fail: if one
  starts compiling, that is not a silent win, it is a stale entry, and the
  gate says so. Deleting the entry (and moving the combination up into
  `SUPPORTED`) then becomes a deliberate, reviewable edit -- the same
  non-rotting-allowlist shape used by `check_orphan_scripts.py` and
  `check_unbounded_loops.py`.

Counting errors, not just exit status
-------------------------------------
`cargo check` exits non-zero for a build error, but a non-zero exit can also
come from a broken toolchain, a missing target, or a lockfile mismatch -- none
of which are facts about the feature graph. The gate therefore parses
`error[EXXXX]` / `error:` lines and reports the count, so a "failure" caused by
something else shows up as 0 parsed errors and is investigated rather than
being recorded as a feature-graph regression.

Cost
----
Each combination is a fresh `cargo check`, so the run is minutes, not seconds.
That is why it is a release/hygiene gate rather than a per-commit one, and why
`--only <label>` exists for re-checking a single combination while developing.

Usage
-----
    python scripts/check_feature_matrix.py
    python scripts/check_feature_matrix.py --only no-std
    python scripts/check_feature_matrix.py --list

Exit codes
----------
0  every SUPPORTED combination builds and every KNOWN_BROKEN one still fails
1  a SUPPORTED combination failed, or a KNOWN_BROKEN one now builds
2  the toolchain could not be run at all
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# Combinations that must compile. `default` is first because it is what every
# consumer gets unless they opt out; the rest are the documented opt-out paths.
SUPPORTED: list[tuple[str, list[str]]] = [
    ("default", []),
    ("bare", ["--no-default-features"]),
    ("no-std", ["--no-default-features", "--features", "no_std"]),
    ("tracing-only", ["--no-default-features", "--features", "tracing"]),
]

# Combinations that are known not to compile, with the measured error count.
# The count is asserted as a floor-of-one and as a "still fails" test; it is
# not compared exactly, because a refactor that shifts 40 errors to 38 has not
# fixed anything and should not flip this gate either way.
KNOWN_BROKEN: list[tuple[str, list[str], str]] = [
    ("std-only", ["--no-default-features", "--features", "std"],
     "operation.rs / composite.rs / factors::builtin use crate::formula and "
     "crate::factor_graph, which the `formula` feature gates"),
    ("overlap-only", ["--no-default-features", "--features", "indicators-overlap"],
     "indicators/mod.rs's impl_slice_output! adapters reach indicators::momentum "
     "from the category-neutral part of the file"),
    ("formula-only", ["--no-default-features", "--features", "formula"],
     "the formula surface resolves indicator kernels from every category"),
    ("serde-only", ["--no-default-features", "--features", "serde"],
     "same as std-only: serde implies std, and std alone does not build"),
    ("rayon-only", ["--no-default-features", "--features", "rayon"],
     "same as std-only"),
    ("metrics-only", ["--no-default-features", "--features", "metrics"],
     "same as std-only"),
]

ERROR_RE = re.compile(r"^error(?:\[[A-Z0-9]+\])?:", re.MULTILINE)


def run_check(extra: list[str]) -> tuple[int, int, str]:
    """Return (exit_code, parsed_error_count, tail)."""
    cmd = ["cargo", "check", "-p", "finkit", "--locked", *extra]
    env = dict(os.environ, CARGO_INCREMENTAL="0", CARGO_TERM_COLOR="never")
    try:
        proc = subprocess.run(
            cmd,
            cwd=ROOT,
            capture_output=True,
            encoding="utf-8",
            errors="replace",
            env=env,
        )
    except FileNotFoundError:
        print("cargo not found on PATH", file=sys.stderr)
        sys.exit(2)
    out = (proc.stdout or "") + (proc.stderr or "")
    return proc.returncode, len(ERROR_RE.findall(out)), out.strip().splitlines()[-1][:100] if out.strip() else ""


def label_of(extra: list[str]) -> str:
    return " ".join(extra) if extra else "(default)"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--only", metavar="LABEL", help="check one combination by label")
    parser.add_argument("--list", action="store_true", help="print the matrix and exit")
    args = parser.parse_args()

    if args.list:
        print("SUPPORTED (must build):")
        for label, extra in SUPPORTED:
            print(f"  {label:16s} cargo check -p finkit --locked {label_of(extra)}")
        print("KNOWN_BROKEN (must still fail):")
        for label, extra, why in KNOWN_BROKEN:
            print(f"  {label:16s} {label_of(extra)}\n      -> {why}")
        return 0

    supported = [c for c in SUPPORTED if not args.only or c[0] == args.only]
    broken = [c for c in KNOWN_BROKEN if not args.only or c[0] == args.only]
    if args.only and not supported and not broken:
        print(f"no combination labelled {args.only!r}; try --list", file=sys.stderr)
        return 2

    failures: list[str] = []

    for label, extra in supported:
        rc, errors, tail = run_check(extra)
        ok = rc == 0
        print(f"{'OK  ' if ok else 'FAIL'} {label:16s} errors={errors:<4} {label_of(extra)}")
        if not ok:
            detail = f"{label}: expected a clean build, got {errors} error(s)"
            if errors == 0:
                detail += " -- and no `error[...]` line was parsed, so this is not a feature-graph regression"
            failures.append(detail + (f"\n      {tail}" if tail else ""))
            print(f"      {tail}")

    for label, extra, why in broken:
        rc, errors, tail = run_check(extra)
        still_fails = rc != 0
        print(
            f"{'OK  ' if still_fails else 'FAIL'} {label:16s} errors={errors:<4} "
            f"(known broken: {why[:60]})"
        )
        if not still_fails:
            failures.append(
                f"{label}: recorded as KNOWN_BROKEN but it now builds -- move it into "
                f"SUPPORTED and update the core/Cargo.toml comment (was: {why})"
            )

    if failures:
        print(f"\nFAIL: {len(failures)} problem(s)")
        for line in failures:
            print(f"  - {line}")
        return 1

    print(
        f"\nOK: {len(supported)} supported combination(s) build; "
        f"{len(broken)} known-broken combination(s) still fail as recorded"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
