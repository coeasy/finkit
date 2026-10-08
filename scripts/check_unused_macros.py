#!/usr/bin/env python3
"""Unused `#[macro_export]` macro gate (V5 Batch 3).

`macro_rules!` definitions are invisible to `check_dead_code_allows.py`:
a macro generates code only where it is *invoked*, and rustc never flags an
exported macro nobody calls. That is exactly how `streaming/macros.rs` grew
an `impl_repaint!` and `streaming/repaint.rs` an entire module of
`impl_compute_bar!` / `impl_next_with_time!` — three repaint code-generation
macros with **zero** call sites while ten streaming indicators hand-wrote
the same logic (V5 finding R2-1). Those are gone now; this gate keeps them
from coming back.

Rule: every `#[macro_export] macro_rules! NAME` in `core/src` must be
invoked at least once somewhere in the workspace (core, the other crates,
bindings, wasm, cli, examples, tests) *outside its own definition lines*.
Doc-comment examples do not count as uses; `macro_rules!` definitions and
their doc lines are excluded.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SEARCH_ROOTS = [
    "core",
    "factor-analysis",
    "visualization",
    "cli",
    "wasm",
    "ffi",
    "examples",
    "fuzz",
]

DEFINITION_RE = re.compile(r"^#\[macro_export\]\s*$", re.MULTILINE)
MACRO_NAME_RE = re.compile(r"macro_rules!\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*\{")


def collect_sources() -> list[Path]:
    roots = [REPO / root for root in SEARCH_ROOTS if (REPO / root).exists()]
    files: list[Path] = []
    for root in roots:
        files.extend(root.rglob("*.rs"))
    return [f for f in files if "target" not in f.parts]


def main() -> int:
    sources = collect_sources()
    # First pass: find every exported macro and its definition file/line.
    exported: list[tuple[str, Path, int]] = []
    for path in sources:
        text = path.read_text(encoding="utf-8", errors="replace")
        for match in re.finditer(
            r"#\[macro_export\]\s*\nmacro_rules!\s*([a-zA-Z_][a-zA-Z0-9_]*)", text
        ):
            line = text[: match.start(1)].count("\n") + 1
            exported.append((match.group(1), path, line))

    if not exported:
        print("No exported macros found — nothing to check.")
        return 0

    # Second pass: count invocations outside the definition's own lines.
    # A doc-comment (`///`, `//!`, `//`) mention is documentation, not a use.
    failures: list[str] = []
    for name, def_path, def_line in exported:
        invocations = 0
        for path in sources:
            text = path.read_text(encoding="utf-8", errors="replace")
            for line_no, line in enumerate(text.splitlines(), start=1):
                if path == def_path and abs(line_no - def_line) <= 1:
                    continue  # the `macro_rules!` definition line itself
                if line.lstrip().startswith("//"):
                    continue  # doc comments / commented-out examples
                if re.search(rf"\b{name}\s*!", line):
                    invocations += 1
        if invocations == 0:
            failures.append(
                f"{def_path.relative_to(REPO)}:{def_line}: `#{name}!` is "
                f"exported but never invoked anywhere in the workspace"
            )
        else:
            print(f"[OK]   {name}! — {invocations} invocation(s)")

    if failures:
        print("\nUnused exported macros found:")
        for failure in failures:
            print(f"  - {failure}")
        print(
            "\nDead exported macros are invisible to the dead-code gate: "
            "either delete the macro or migrate its intended callers."
        )
        return 1
    print("\nEvery exported macro has at least one invocation.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
