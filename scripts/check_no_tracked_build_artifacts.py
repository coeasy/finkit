#!/usr/bin/env python3
"""No-tracked-build-artifacts gate (V4 plan §3.3 / P1-03).

`core/target/criterion/` accumulated 273 tracked files (benchmark JSON, SVG
reports, HTML) that polluted the source tree, the release archive and every
`git diff`. `.gitignore` ignored the path, but files that were tracked before
the ignore rule stay tracked forever until they are explicitly removed.

This gate fails when Git tracks anything that looks like build output:

    **/target/**  **/dist/**  **/node_modules/**  **/obj/**
    **/bin/** (except Rust's conventional src/bin/ sources)
    *.so  *.dll  *.dylib  *.node  *.exe  *.pdb  *.lib  *.a
    renderable output sitting in the repository root (*.html, *.svg, *.json)

unless the path is recorded in `scripts/tracked_build_artifacts_allowlist.json`
with a reason. The allowlist is non-rotting: an entry whose path is no longer
tracked is itself a failure.

Why the root-directory rule exists
----------------------------------
The path rules above miss the most expensive offender this repository had.
`cargo run -p finkit-visualization --example gpu_large_chart` writes
`gpu_large_chart.html` into the repository *root*, and the same CI job then runs
`scripts/test_webgl_runtime.mjs gpu_large_chart.html` against it. Five sibling
files come from `--example improved_chart`. All six were tracked: 7.3 MB of
output that CI regenerates on every run, so the tracked copies were pure bloat
and left `git status` dirty after every local run. None of them matched a
`target/`/`dist/` path or a binary extension, so the previous rule set reported
"none look like build output" while shipping 7 MB of it.

The rule is deliberately scoped to the repository **root** and to renderable
formats. Source trees legitimately track JSON (schemas, fixtures),
HTML (docs, reports) and SVG (diagrams) — but not at the top level, and not as
the output of an example binary. Root-level configuration (`Cargo.lock`,
`deny.toml`, `rust-toolchain.toml`, `docker-compose.yml`, `*.yml`, `Makefile`)
and documentation (`*.md`, `LICENSE*`, `Dockerfile`) are unaffected.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ALLOWLIST_PATH = Path(__file__).resolve().parent / "tracked_build_artifacts_allowlist.json"

BUILD_DIR = re.compile(r"(^|/)(target|dist|node_modules|obj)(/|$)")
BIN_DIR = re.compile(r"(^|/)bin(/|$)")
BINARY_EXT = re.compile(r"\.(so|dll|dylib|node|exe|pdb|lib|a)$", re.IGNORECASE)
# Renderable output in the repository root: the shape the CI example jobs write.
ROOT_RENDERABLE = re.compile(r"^[^/]+\.(html|htm|svg|json|csv)$", re.IGNORECASE)


def tracked_files() -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", "-z"], cwd=ROOT, check=True, capture_output=True
    ).stdout
    return [p.decode("utf-8") for p in out.split(b"\0") if p]


def is_build_artifact(path: str) -> bool:
    if BUILD_DIR.search(path):
        return True
    # Rust's conventional binary-source directory contains real sources.
    if BIN_DIR.search(path) and not path.endswith(".rs"):
        return True
    if BINARY_EXT.search(path):
        return True
    if ROOT_RENDERABLE.search(path):
        return True
    return False


def main() -> int:
    files = tracked_files()
    offenders = sorted(f for f in files if is_build_artifact(f))

    allowlist: list[dict[str, str]] = []
    if ALLOWLIST_PATH.exists():
        allowlist = json.loads(ALLOWLIST_PATH.read_text(encoding="utf-8"))
    allowlisted = {entry["path"]: entry for entry in allowlist}

    failures = []
    for path in offenders:
        if path in allowlisted:
            continue
        failures.append(f"tracked build artifact: {path}")

    stale = []
    tracked = set(files)
    for path in allowlisted:
        if path not in tracked:
            stale.append(
                f"stale allowlist entry: {path} is no longer tracked; remove the entry"
            )

    for problem in failures + stale:
        print(f"::error {problem}")
    if failures:
        print(
            "hint: `git rm -r --cached <path>` untracks the artifact; the "
            ".gitignore rule keeps it out afterwards"
        )
    if failures or stale:
        print(
            f"build-artifact gate: {len(failures)} tracked artifact(s), "
            f"{len(stale)} stale allowlist entr(ies)"
        )
        return 1
    print(f"build-artifact gate: {len(files)} tracked files, none look like build output")
    return 0


if __name__ == "__main__":
    sys.exit(main())
