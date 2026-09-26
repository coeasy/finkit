#!/usr/bin/env python3
"""Require a reason on every `dead_code` suppression.

Why this exists
---------------
`#[allow(dead_code)]` turns off the one compiler check that finds orphan logic.
An item behind it is invisible to `cargo check`, to `cargo build --release`, and
to every test -- so the only signal left is the reader's ability to tell a
deliberate suppression from a forgotten one.

Most of the suppressions in this tree are legitimate and structurally
necessary:

* an alternative implementation compiled only under a `#[cfg]` that the
  current build does not select (`prefix_sum_fallback` exists for the
  non-AVX2 x86_64 build, so a build *with* AVX2 sees it as unused);
* a legacy spelling kept for one release of backward compatibility;
* a helper used only by a benchmark or an integration test target.

All of those look identical to "this function was never wired up" unless
someone wrote down why. So: every suppression carries a reason, on the line
itself or in the attribute block directly above it.

This is the same shape as the other allowlists in this repository
(`MANUAL_TOOLS` in `check_orphan_scripts.py`, `RECORDED_MISSING` in
`check_script_references.py`): the exception is permitted, but it has to be a
deliberate, reviewable edit.

How the reason is located
-------------------------
The window is the suppression line itself, then walking **upward through the
contiguous run of attribute lines**, then the one line above that block. The
walk matters: a comment is usually written above the `#[cfg]` that explains the
suppression, so

    // Scalar fallback for prefix_sum (used when AVX2 is not available)
    #[cfg(all(feature = "std", target_arch = "x86_64"))]
    #[allow(dead_code)]

is a documented suppression, while a bare

    #[allow(dead_code)]

two lines below a closing brace is not.

Exit codes
----------
0  every suppression has a reason next to it
1  at least one suppression does not
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

SCAN_ROOTS = ("core", "factor-analysis", "visualization", "cli", "ffi", "wasm")

# `#[allow(dead_code)]`, `#![allow(dead_code)]`, and the same with `expect`,
# including the multi-lint form `#[allow(non_snake_case, dead_code)]`.
SUPPRESS = re.compile(
    r"^(?P<indent>\s*)#!?\[(?:allow|expect)\((?P<lints>[^)]*)\)\]\s*(?P<rest>.*)$"
)
ATTR_LINE = re.compile(r"^\s*#!?\[")
COMMENT = re.compile(r"//\s*(?P<text>\S.*)$")


def mentions_dead_code(lints: str) -> bool:
    return any(part.strip() == "dead_code" for part in lints.split(","))


def tracked_files() -> list[str]:
    # `-z` is required: plain `git ls-files` octal-escapes non-ASCII paths, and
    # `Path.is_file()` then fails, silently dropping them from the scan.
    out = subprocess.run(
        ["git", "ls-files", "-z"], capture_output=True, text=True, check=True, cwd=ROOT
    ).stdout
    return [
        line
        for line in out.split("\0")
        if line and (ROOT / line).is_file() and line.startswith(SCAN_ROOTS) and line.endswith(".rs")
    ]


def has_reason(lines: list[str], index: int) -> bool:
    """True if a comment sits on `lines[index]` or in the attribute block above."""
    own = COMMENT.search(lines[index])
    if own and len(own.group("text").strip()) >= 3:
        return True

    # Walk up over the contiguous run of attributes above this line.
    i = index - 1
    while i >= 0 and ATTR_LINE.match(lines[i]):
        m = COMMENT.search(lines[i])
        if m and len(m.group("text").strip()) >= 3:
            return True
        i -= 1

    # The line just above the block is where the explanation normally lives.
    if i >= 0:
        m = COMMENT.search(lines[i])
        if m and len(m.group("text").strip()) >= 3:
            return True
    return False


def item_name(lines: list[str], index: int) -> str:
    """The name of the item the suppression is attached to, for the report."""
    defn = re.compile(
        r"^\s*(?:(?:pub(?:\([^)]*\))?|const|unsafe|async|extern(?:\s+\"[^\"]*\")?)\s+)*"
        r"(?:fn|struct|enum|trait|type|mod|const|static)\s+(?P<name>[A-Za-z0-9_]+)"
    )
    for j in range(index + 1, min(index + 6, len(lines))):
        m = defn.match(lines[j])
        if m:
            return m.group("name")
    return ""


def main() -> int:
    files = tracked_files()
    total = 0
    offenders: list[tuple[str, int, str]] = []

    for rel in files:
        lines = (ROOT / rel).read_text(encoding="utf-8", errors="replace").splitlines()
        for i, line in enumerate(lines):
            m = SUPPRESS.match(line)
            if not m or not mentions_dead_code(m.group("lints")):
                continue
            total += 1
            if not has_reason(lines, i):
                offenders.append((rel, i + 1, item_name(lines, i)))

    print(
        f"scanned {len(files)} Rust files; {total} `dead_code` suppression(s), "
        f"{len(offenders)} without a reason"
    )

    if not offenders:
        print("OK: every `dead_code` suppression states why the item is allowed to be unused")
        return 0

    print("\nEach of these silences the only compiler check that can see orphan")
    print("logic. Add a `// why` comment on the line, inside the attribute block")
    print("above it, or on the line directly above that block:\n")
    for rel, line_no, name in offenders:
        suffix = f"  ({name})" if name else ""
        print(f"  {rel}:{line_no}{suffix}")
    return 1


if __name__ == "__main__":
    sys.exit(main())
