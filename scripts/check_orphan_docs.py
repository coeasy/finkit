#!/usr/bin/env python3
"""Fail when a tracked Markdown document is unreachable from any other document.

Why this exists
---------------
`check_orphan_scripts.py` catches a script no one calls. Documents have the same
failure mode and nothing was watching it: `docs/formula-talib-contract.md` and
`docs/quant-evaluation.md` were both current, substantive documents with **zero**
inbound references anywhere in the repository. A reader following the
documentation index could not reach them, and a document nothing links to is
indistinguishable from one that was deleted.

Reachability
------------
A document counts as reachable when another tracked Markdown file either

- links to it: `[text](path/to/doc.md)`, or
- names it as a backticked repository-relative path: `` `docs/foo.md` ``.

The second form is deliberate. Index documents such as `docs/README.md` list
generated artifacts as backticked bullets rather than links, and `docs/archive/`
records its contents the same way. Treating those as unreachable would produce
false positives that train people to ignore the gate.

Every document is expected to be reachable, including archived ones — the
archive has its own index (`docs/archive/README.md`) precisely so it stays
navigable.

Usage
-----
    python scripts/check_orphan_docs.py
    python scripts/check_orphan_docs.py --verbose
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

LINK_RE = re.compile(r"\]\(([^)\s]+\.md)(?:#[^)\s]*)?\)")
TICK_RE = re.compile(r"`([^`\n]+\.md)`")


def tracked_files() -> list[str]:
    """Repository-relative POSIX paths of every tracked file.

    `-z` is mandatory: the newline-separated form octal-escapes non-ASCII paths,
    which would silently drop every Chinese-named document from the scan.
    """
    result = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if result.returncode != 0:
        print("FAILED: `git ls-files` failed; run this inside the repository")
        raise SystemExit(2)
    return [p for p in result.stdout.split("\0") if p]


def resolve(candidate: Path) -> str | None:
    try:
        return candidate.resolve().relative_to(ROOT).as_posix()
    except (OSError, ValueError):
        return None


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--verbose", action="store_true", help="list reachability per document")
    args = ap.parse_args()

    files = tracked_files()
    docs = [f for f in files if f.startswith("docs/") and f.endswith(".md")]
    markdown = [f for f in files if f.endswith(".md")]
    if not docs:
        print("FAILED: no tracked documents under docs/")
        return 1

    known = set(docs)
    reachable: set[str] = set()
    for name in markdown:
        text = (ROOT / name).read_text(encoding="utf-8", errors="replace")
        base = (ROOT / name).parent
        for match in LINK_RE.finditer(text):
            resolved = resolve(base / match.group(1))
            if resolved in known:
                reachable.add(resolved)
        for match in TICK_RE.finditer(text):
            raw = match.group(1).strip()
            for candidate in (base / raw, ROOT / raw):
                resolved = resolve(candidate)
                if resolved in known:
                    reachable.add(resolved)

    orphans = sorted(d for d in docs if d not in reachable)

    if args.verbose:
        for name in docs:
            mark = "reachable" if name in reachable else "ORPHAN"
            print(f"  {name:<72} {mark}")

    if orphans:
        print("Tracked documents that nothing links to:")
        for name in orphans:
            print(f"  - {name}")
        print(
            "Add each to an index (for example docs/README.md or "
            "docs/archive/README.md), or delete it."
        )
        print(f"FAILED: {len(orphans)} of {len(docs)} documents are unreachable")
        return 1

    print(f"OK: all {len(docs)} tracked documents are reachable from another document")
    return 0


if __name__ == "__main__":
    sys.exit(main())
