#!/usr/bin/env python3
"""Fail when a checked Markdown document links to a missing local path.

This intentionally validates repository-local paths only. External URLs and anchors are
left to external link checkers because CI should not depend on network availability.

It also fails when a link target exists in the working tree but is **not tracked
by git**. CI checks out the committed tree, so an ignored or untracked file
satisfies this check locally while a fresh clone gets a dangling link — which is
exactly how `docs/archive/README.md` stayed invisible: `.gitignore` listed
`docs/archive/`, so the archive index was present on the author's disk, absent
from every clone, and this gate reported success anyway.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]
LINK_RE = re.compile(r"(?<!!)\[[^\]]*\]\(([^)]+)\)")
EXTERNAL_SCHEMES = (
    "http://",
    "https://",
    "mailto:",
    "tel:",
    "data:",
    "javascript:",
)


def tracked_files() -> set[str] | None:
    """Repository-relative POSIX paths of every git-tracked file.

    `None` when git is unavailable or this is not a work tree, in which case the
    tracked-target check is skipped rather than reported as a failure.

    `git ls-files -z` is required: the newline-separated form octal-escapes
    non-ASCII paths, which would drop every Chinese-named document from the set.
    """
    try:
        result = subprocess.run(
            ["git", "ls-files", "-z"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
        )
    except OSError:
        return None
    if result.returncode != 0:
        return None
    return {p for p in result.stdout.split("\0") if p}


def markdown_files() -> list[Path]:
    files: set[Path] = set()
    for relative in ("README.md", "CONTRIBUTING.md", "core/README.md"):
        path = ROOT / relative
        if path.exists():
            files.add(path)

    files.update((ROOT / "docs").rglob("*.md"))
    files.update((ROOT / "ffi").glob("*/README.md"))
    return sorted(files)


def strip_fenced_code(text: str) -> list[tuple[int, str]]:
    visible: list[tuple[int, str]] = []
    in_fence = False
    fence = ""

    for lineno, line in enumerate(text.splitlines(), 1):
        stripped = line.lstrip()
        if stripped.startswith("```") or stripped.startswith("~~~"):
            marker = stripped[:3]
            if not in_fence:
                in_fence = True
                fence = marker
            elif marker == fence:
                in_fence = False
                fence = ""
            continue
        if not in_fence:
            visible.append((lineno, line))
    return visible


def normalize_target(raw: str) -> str:
    target = raw.strip()
    if target.startswith("<") and ">" in target:
        target = target[1 : target.index(">")]
    elif " " in target:
        # Markdown permits an optional quoted title after a destination.
        target = target.split(None, 1)[0]
    return unquote(target)


def resolve_local(source: Path, target: str) -> Path | None:
    lower = target.lower()
    if not target or target.startswith("#") or lower.startswith(EXTERNAL_SCHEMES):
        return None

    path_part = target.split("#", 1)[0].split("?", 1)[0]
    if not path_part:
        return None

    if path_part.startswith("/"):
        candidate = ROOT / path_part.lstrip("/")
    else:
        candidate = source.parent / path_part

    try:
        return candidate.resolve(strict=False)
    except OSError:
        return candidate


def main() -> int:
    errors: list[str] = []
    tracked = tracked_files()

    for source in markdown_files():
        text = source.read_text(encoding="utf-8")
        for lineno, line in strip_fenced_code(text):
            for match in LINK_RE.finditer(line):
                target = normalize_target(match.group(1))
                candidate = resolve_local(source, target)
                if candidate is None:
                    continue

                try:
                    relative = candidate.relative_to(ROOT).as_posix()
                except ValueError:
                    errors.append(
                        f"{source.relative_to(ROOT)}:{lineno}: local link escapes repository: {target}"
                    )
                    continue

                if not candidate.exists():
                    errors.append(
                        f"{source.relative_to(ROOT)}:{lineno}: missing local link target: {target}"
                    )
                elif tracked is not None and relative not in tracked:
                    errors.append(
                        f"{source.relative_to(ROOT)}:{lineno}: link target exists on disk but is "
                        f"not tracked by git, so a fresh clone will not have it: {target} "
                        f"(add it with `git add -f`, or stop ignoring it)"
                    )

    if errors:
        print("Documentation link check failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(f"Documentation link check passed ({len(markdown_files())} Markdown files).")
    if tracked is None:
        print("Note: git is unavailable, so link targets were not checked for tracked status.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
