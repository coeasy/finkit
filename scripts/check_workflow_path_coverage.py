#!/usr/bin/env python3
"""Verify every `paths`-filtered workflow also lists the local path
dependencies (transitively) of every crate it builds.

A workflow that runs ``cargo build -p finkit-wasm`` while its
``pull_request.paths`` filter omits ``visualization/**`` -- a crate
``finkit-wasm`` depends on -- will silently skip when *only* the visualization
crate changes. A breaking change to that dependency then passes CI on the PR
and only fails at release time, when ``multilang-release.yml`` finally builds
the wasm32 target. That is a dormant breakage: green on the change, red on the
ship. The same shape affects any language binding that depends on
``visualization`` or ``factor-analysis``.

The check is deliberately narrow so it cannot rot into a false-positive chore:

* Workflows without a ``pull_request.paths`` block are skipped entirely -- they
  run on every PR, so path coverage is moot (``ci.yml`` is in this class and is
  the host-target backstop).
* Only crates the workflow *actually builds or tests* (``-p <crate>``) seed the
  analysis. A workflow that merely references a crate in a comment does not pull
  it in.
* The dependency graph is the *local* ``path = "..."`` edges only; registry
  dependencies are out of scope.

Verified by injection during development: removing ``visualization/**`` from a
workflow that builds ``finkit-wasm`` flips this gate to failure.
"""

from __future__ import annotations

import os
import re
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
WORKFLOW_DIR = os.path.join(REPO, ".github", "workflows")


def workspace_members() -> list[tuple[str, str]]:
    """[(package_name, member_dir), ...] from the root Cargo.toml members."""
    text = open(os.path.join(REPO, "Cargo.toml"), encoding="utf-8").read()
    block = re.search(r"members\s*=\s*\[(.*?)\]", text, re.S)
    if not block:
        return []
    members = re.findall(r'"([^"]+)"', block.group(1))
    result: list[tuple[str, str]] = []
    for d in members:
        ct = os.path.join(REPO, d, "Cargo.toml")
        if not os.path.exists(ct):
            continue
        ctext = open(ct, encoding="utf-8").read()
        nm = re.search(r'^name\s*=\s*"([^"]+)"', ctext, re.M)
        if nm:
            result.append((nm.group(1), d))
    return result


def local_path_deps(member_dir: str) -> set[str]:
    """Member dirs that *member_dir* depends on via a local ``path = "..."``
    inside a dependency table (``foo = { path = "../bar" }``). Bare ``path``
    fields such as ``[lib] path = "src/lib.rs"`` are not dependencies and are
    excluded by requiring the match to sit inside an unterminated ``{`` block."""
    ct = os.path.join(REPO, member_dir, "Cargo.toml")
    text = open(ct, encoding="utf-8").read()
    deps: set[str] = set()
    for pm in re.finditer(r'\bpath\s*=\s*"([^"]+)"', text):
        before = text[: pm.start()]
        if before.count("{") - before.count("}") <= 0:
            continue  # outside any dependency table
        full = os.path.normpath(os.path.join(member_dir, pm.group(1))).replace("\\", "/")
        deps.add(full)
    return deps


def closure(start_dirs: list[str]) -> set[str]:
    """Transitive closure over local path dependencies."""
    seen: set[str] = set()
    stack = list(start_dirs)
    while stack:
        d = stack.pop()
        if d in seen:
            continue
        seen.add(d)
        for dep in local_path_deps(d):
            if dep not in seen:
                stack.append(dep)
    return seen


def built_crates(workflow_text: str) -> set[str]:
    """Crate names referenced by ``cargo build|test|check|clippy -p <crate>``."""
    crates: set[str] = set()
    for m in re.finditer(
        r"cargo\s+(?:build|test|check|clippy)\b[^\n]*?-p\s+([A-Za-z0-9_-]+)",
        workflow_text,
    ):
        crates.add(m.group(1))
    return crates


def pr_paths(workflow_text: str) -> list[str] | None:
    """The ``pull_request.paths`` list, or ``None`` when the workflow has no
    pull_request paths filter (covers plain ``pull_request:`` with only
    branches, and workflows that run unconditionally on PRs)."""
    lines = workflow_text.splitlines()
    pr_start: int | None = None
    for i, ln in enumerate(lines):
        if re.match(r"^\s*pull_request\s*:", ln):
            pr_start = i
            break
    if pr_start is None:
        return None
    block: list[str] = []
    for ln in lines[pr_start + 1 :]:
        if re.match(r"^[A-Za-z]", ln):  # next top-level key ends the block
            break
        block.append(ln)
    block_text = "\n".join(block)
    mp = re.search(r"paths\s*:", block_text)
    if not mp:
        return None
    after = block_text[mp.end() :]
    paths: list[str] = []
    ended = False
    for ln in after.splitlines():
        m = re.match(r"\s+-\s+\"([^\"]+)\"", ln)
        if m:
            paths.append(m.group(1))
        elif re.match(r"\S", ln) and not ln.strip().startswith("-"):
            # a sibling key (e.g. `branches:`) at the same indent ends the list
            ended = True
            break
    if ended and not paths:
        return None
    return paths if paths else None


def main() -> int:
    members = workspace_members()
    name_to_dir = {n: d for n, d in members}
    problems: list[tuple[str, set[str], list[str]]] = []

    for wf in sorted(os.listdir(WORKFLOW_DIR)):
        if not wf.endswith(".yml"):
            continue
        path = os.path.join(WORKFLOW_DIR, wf)
        text = open(path, encoding="utf-8").read()
        paths = pr_paths(text)
        if paths is None:
            continue
        crates = built_crates(text)
        if not crates:
            continue
        start_dirs = [name_to_dir[c] for c in crates if c in name_to_dir]
        if not start_dirs:
            continue
        closure_dirs = closure(start_dirs)
        path_dirs = {p.rstrip("/**") for p in paths}
        # A member directory is covered if any path entry is the directory itself
        # or lives beneath it (so `core/src/formula/simd.rs` counts as covering
        # `core`, matching a workflow that filters to a specific sub-tree).
        def covered(d: str) -> bool:
            return any(d == p or p.startswith(d + "/") for p in path_dirs)

        missing = sorted(d for d in closure_dirs if not covered(d))
        if missing:
            problems.append((wf, crates, missing))

    if problems:
        print("WORKFLOW PATH COVERAGE FAILED")
        for wf, crates, missing in problems:
            print(
                f"  {wf}: builds {', '.join(sorted(crates))} but pull_request.paths "
                f"omits {'/**, '.join(missing)}/**"
            )
        return 1
    print("OK: every paths-filtered workflow that builds local crates also "
          "lists the transitive local dependencies of those crates")
    return 0


if __name__ == "__main__":
    sys.exit(main())
