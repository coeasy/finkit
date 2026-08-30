#!/usr/bin/env python3
"""D-6: Verify CHANGELOG.md follows Keep a Changelog 1.1.

Exit 0 if the most recent version block has at least one of:
  - ### Added
  - ### Changed
  - ### Fixed
  - ### Removed
  - ### Deprecated
  - ### Security

Exit 1 if the changelog is malformed or the latest version is empty.

Usage:
    python scripts/check_changelog.py docs/CHANGELOG.md
"""

import re
import sys
from pathlib import Path

# Per Keep a Changelog 1.1: https://keepachangelog.com/en/1.1.0/
REQUIRED_SECTIONS = (
    "### Added",
    "### Changed",
    "### Fixed",
    "### Removed",
    "### Deprecated",
    "### Security",
)

VERSION_RE = re.compile(r"^## \[?(\d+\.\d+(?:\.\d+)?)\]?")


def main() -> int:
    if len(sys.argv) != 2:
        print("Usage: check_changelog.py <path-to-CHANGELOG.md>", file=sys.stderr)
        return 2

    path = Path(sys.argv[1])
    if not path.is_file():
        print(f"CHANGELOG not found: {path}", file=sys.stderr)
        return 2

    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()

    # Find the latest version block
    start = None
    for i, line in enumerate(lines):
        if VERSION_RE.match(line):
            start = i
            break

    if start is None:
        print("ERROR: no version heading found (expected `## [X.Y.Z] - YYYY-MM-DD`)", file=sys.stderr)
        return 1

    # Find the next version heading (or EOF)
    end = len(lines)
    for i in range(start + 1, len(lines)):
        if VERSION_RE.match(lines[i]):
            end = i
            break

    block = "\n".join(lines[start:end])
    version = VERSION_RE.match(lines[start]).group(1)

    found = [s for s in REQUIRED_SECTIONS if s in block]
    if not found:
        print(
            f"ERROR: latest version {version} has no Keep-a-Changelog sections",
            file=sys.stderr,
        )
        print("  expected at least one of:", file=sys.stderr)
        for s in REQUIRED_SECTIONS:
            print(f"    {s}", file=sys.stderr)
        print("  block was:", file=sys.stderr)
        for line in lines[start:end]:
            print(f"    {line}", file=sys.stderr)
        return 1

    print(f"OK: CHANGELOG version {version} has sections: {', '.join(found)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
