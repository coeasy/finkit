#!/usr/bin/env python3
"""Fail when a caller names a `scripts/` file that is not in the tree.

`check_orphan_scripts.py` catches one direction of the same break: a script
with no consumer. This catches the other. A workflow step, a Makefile recipe or
a document that says `python scripts/foo.py` while `scripts/foo.py` does not
exist reads exactly like an automated gate -- it has a name, a place in the
release checklist, and often a paragraph explaining what it enforces -- but it
is a link to nothing. Nothing compiles it, nothing tests it, and the only way
to notice is to try to run it.

Two real cases motivated this check:

* `.github/workflows/apply-architecture-v3-round2.yml` wrote a script to a
  hidden path, ran it, then deleted it, so the workflow's own command line
  pointed at a file that never existed in the tree. That pattern is now
  rejected outright: a runtime-generated helper has to be recorded in
  `RECORDED_MISSING` with a reason, so the exception is a deliberate,
  reviewable edit rather than an invisible one.
* `docs/competitive-analysis/...-2026-09-23.md` lists
  `scripts/emit_speedup_matrix.py` as an item to add. It has not been written
  yet, which is legitimate for a plan, but the reference must be *recorded* so
  the gap is visible rather than indistinguishable from a typo.

How a reference is found
------------------------
Every tracked text file is scanned for `scripts/<path>` tokens with a
script-like extension. Brace forms are expanded first, so the Makefile's
`scripts/build-usage.{sh,ps1}` counts as two references, not one. Tokens that
still contain a wildcard or a shell variable are skipped: they cannot be
resolved statically, and a glob that matches nothing is a different problem
from a literal path that does not exist.

Recorded exceptions
-------------------
A reference may legitimately point at a path that is not in the tree. Those
are listed in `RECORDED_MISSING` with a reason, which makes the exception a
deliberate, reviewable edit instead of an invisible one. The list is checked in
both directions, so an entry that stops being needed fails and the list can
only shrink by an intentional change.

Exit codes
----------
0  every literal `scripts/` reference resolves to a file, or is recorded
1  a reference does not resolve and is not recorded, or a recorded entry is stale
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# Extensions that identify a reference as pointing at an executable artefact
# rather than, say, a prose mention of a directory.
SCRIPT_EXTENSIONS = (
    ".py",
    ".sh",
    ".ps1",
    ".cmd",
    ".mjs",
    ".js",
    ".rs",
    ".dockerfile",
)

# Binary or machine-generated files whose `scripts/...` strings are not calls.
SKIP_EXTENSIONS = {
    ".whl",
    ".zip",
    ".gz",
    ".tar",
    ".png",
    ".jpg",
    ".jpeg",
    ".ico",
    ".svg",
    ".so",
    ".dll",
    ".lib",
    ".rlib",
    ".pdb",
}
SKIP_NAMES = {"package-lock.json", "Cargo.lock"}

# Paths a caller legitimately names even though they are not in the tree. Two
# reasons are acceptable: the caller writes the file itself before running it
# (a run-time helper), or a plan document lists it as work still to do. The
# reason is required so the exception stays reviewable. Checked in both
# directions, like `MANUAL_TOOLS` in `check_orphan_scripts.py`.
RECORDED_MISSING: dict[str, str] = {
    "scripts/emit_speedup_matrix.py": (
        "planned, not yet written: docs/competitive-analysis/"
        "finkit-落地开发计划-2026-09-23.md M1-2 lists it as a new file to add "
        "for the 201-function speed-up matrix"
    ),
}

SELF_DESCRIBING = {"scripts/check_script_references.py"}

BRACE = re.compile(r"([\w./-]*)\{([^{}]*)\}")
TOKEN = re.compile(r"scripts/[A-Za-z0-9_./{},\-]+")
UNRESOLVABLE = set("*?<>$")


def tracked_files() -> list[str]:
    """Every tracked path that still exists in the working tree.

    `-z` is load-bearing, not cosmetic: without it git octal-escapes any path
    containing a non-ASCII byte, so `docs/competitive-analysis/finkit-<cjk>.md`
    arrives as `"docs/...finkit-\\350\\220\\275..."` and `Path.is_file()` then
    fails. Nine tracked files were invisible that way, including the plan
    document whose `scripts/emit_speedup_matrix.py` reference this check exists
    to report.

    A path whose working-tree file was deleted but whose removal is not staged
    is also still listed; it is gone as far as this check is concerned.
    """
    out = subprocess.run(
        ["git", "ls-files", "-z"], capture_output=True, text=True, check=True, cwd=ROOT
    ).stdout
    return [line for line in out.split("\0") if line and (ROOT / line).is_file()]


def expand_braces(text: str) -> str:
    """Rewrite `scripts/foo.{sh,ps1}` as `scripts/foo.sh scripts/foo.ps1`.

    The prefix must be repeated per alternative, otherwise only the first file
    is seen as referenced and the rest look missing.
    """
    return BRACE.sub(
        lambda m: " ".join(m.group(1) + part.strip() for part in m.group(2).split(",")),
        text,
    )


def resolve(token: str) -> bool:
    """True if the token names a file in the tree.

    Trailing sentence punctuation is stripped, because prose writes
    "`scripts/foo.py`." and the full stop is not part of the path.
    """
    for candidate in (token, token.rstrip(".,;:")):
        if (ROOT / candidate).is_file():
            return True
    return False


def references(files: list[str]) -> dict[str, set[str]]:
    missing: dict[str, set[str]] = {}
    for rel in files:
        if rel in SELF_DESCRIBING:
            continue
        path = Path(rel)
        if path.suffix in SKIP_EXTENSIONS or path.name in SKIP_NAMES:
            continue
        try:
            if (ROOT / rel).stat().st_size > 800_000:
                continue
            text = expand_braces((ROOT / rel).read_text(encoding="utf-8", errors="replace"))
        except (OSError, UnicodeDecodeError):
            continue
        for match in TOKEN.finditer(text):
            token = match.group(0)
            if not token.endswith(SCRIPT_EXTENSIONS):
                continue
            if any(ch in token for ch in UNRESOLVABLE):
                continue
            token = token.rstrip(".,;:")
            if not resolve(token):
                missing.setdefault(token, set()).add(rel)
    return missing


def main() -> int:
    files = tracked_files()
    absent = references(files)
    # A recorded path is still counted as absent, otherwise the staleness test
    # below would always see it as unused and demand its removal.
    missing = {k: v for k, v in absent.items() if k not in RECORDED_MISSING}
    stale_allowlist = [p for p in RECORDED_MISSING if p not in absent]

    print(
        f"scanned {len(files)} tracked files for `scripts/` references; "
        f"{len(RECORDED_MISSING)} recorded as legitimately absent"
    )

    if not missing and not stale_allowlist:
        print("OK: every `scripts/` reference resolves to a file, or is recorded")
        return 0

    if missing:
        print("\nFAIL: references to scripts that do not exist:", file=sys.stderr)
        for token in sorted(missing):
            print(f"  - {token}", file=sys.stderr)
            for src in sorted(missing[token])[:5]:
                print(f"      referenced by {src}", file=sys.stderr)
        print(
            "  Fix the caller, add the missing script, or record it in "
            "RECORDED_MISSING with a reason.",
            file=sys.stderr,
        )
    if stale_allowlist:
        print(
            "\nFAIL: RECORDED_MISSING entries that are no longer needed:",
            file=sys.stderr,
        )
        for token in stale_allowlist:
            print(f"  - {token}", file=sys.stderr)
        print("  Remove them so the list can only shrink.", file=sys.stderr)

    print(f"\nFAIL: {len(missing) + len(stale_allowlist)} problem(s)", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
