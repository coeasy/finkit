#!/usr/bin/env python3
"""One-shot maintenance: move repository workflows off deprecated Node 20 actions.

Temporary helper for this branch only. It upgrades official GitHub actions to
current major releases verified on 2026-09-03, excluding the temporary
maintenance workflow itself.
"""
from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = ROOT / ".github" / "workflows"

REPLACEMENTS = {
    "actions/checkout@v4": "actions/checkout@v7",
    "actions/setup-python@v5": "actions/setup-python@v7",
    "actions/setup-python@v6": "actions/setup-python@v7",
    "actions/setup-node@v4": "actions/setup-node@v7",
    "actions/setup-java@v4": "actions/setup-java@v6",
    "actions/setup-dotnet@v4": "actions/setup-dotnet@v6",
    "actions/cache@v4": "actions/cache@v6",
    "actions/upload-artifact@v4": "actions/upload-artifact@v7",
    "actions/download-artifact@v4": "actions/download-artifact@v8",
}


def main() -> int:
    changed_files: list[str] = []
    total = 0
    for path in sorted(WORKFLOWS.glob("*.yml")):
        if path.name == "_apply-warning-cleanup.yml":
            continue
        text = path.read_text(encoding="utf-8")
        updated = text
        local = 0
        for old, new in REPLACEMENTS.items():
            count = updated.count(old)
            if count:
                updated = updated.replace(old, new)
                local += count
        if updated != text:
            path.write_text(updated, encoding="utf-8")
            changed_files.append(path.name)
            total += local
    if not changed_files:
        raise SystemExit("no deprecated official Action references found")
    print(f"upgraded {total} Action reference(s): {', '.join(changed_files)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
