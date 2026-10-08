#!/usr/bin/env python3
"""Guard the forming-bar rollback discipline (V5 R2-1 follow-up).

Ten streaming indicators used to each hand-write the same two fields — an
`Option<SnapshotState>` plus a `last_open_time: i64` — and each hand-write the
`t != 0 && t == self.last_open_time` test that decides whether the fold has to
be undone. The discipline now lives in `streaming::forming_bar::FormingBar`.

A hand-written copy's failure mode is not a compile error, it is restoring the
indicator's own fields while forgetting `last_open_time`: the next re-delivery
then no longer matches and the bar is folded in twice, silently. So this gate
watches for the shape rather than the behaviour — cheaper and stricter than a
numeric test would be.

Exit codes: 0 = clean, 1 = a hand-rolled pair has reappeared.
"""
from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path("core/src/streaming")
LEGACY_FIELDS = re.compile(
    r"^\s*(?:pub(?:\(crate\))?\s+)?"
    r"(?:snapshot:\s*Option<SnapshotState>|last_open_time:\s*i64)\s*,?\s*$"
)
# `last_open_time` legitimately survives in the public `SmaSnapshot` type, whose
# field is `pub(crate)` and is a wire-format concern, not internal bookkeeping.
ALLOWED = {ROOT / "overlap" / "sma.rs"}
ALLOWED_LINES = {ROOT / "overlap" / "sma.rs": "pub(crate) last_open_time: i64,"}


def main() -> int:
    violations: list[tuple[str, int, str]] = []
    for path in sorted(ROOT.rglob("*.rs")):
        if path.name == "forming_bar.rs":
            continue
        for idx, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            stripped = line.strip()
            if stripped.startswith("//") or stripped.startswith("///"):
                continue
            if not LEGACY_FIELDS.match(line):
                continue
            if path in ALLOWED and stripped == ALLOWED_LINES.get(path):
                continue
            violations.append((str(path), idx, stripped))

    if violations:
        print(
            f"[check_forming_bar_discipline] FAIL: {len(violations)} hand-rolled "
            f"forming-bar field(s).\n"
            f"  Use `bar: FormingBar<SnapshotState>` with "
            f"`bar.take_rollback(t)` / `bar.begin(t, snap)`:\n"
            f"  see core/src/streaming/forming_bar.rs"
        )
        for path, line_no, text in violations[:30]:
            print(f"  {path}:{line_no}: {text}")
        return 1

    print(
        "[check_forming_bar_discipline] OK: every streaming indicator routes "
        "its repaint rollback through FormingBar"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
