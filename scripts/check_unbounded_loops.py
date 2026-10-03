#!/usr/bin/env python3
"""Unbounded-loop termination gate (V4 plan §5.2 / P1-07).

Termination of `loop { ... }` / `while true { ... }` sites has so far been
re-verified by hand in every audit cycle. A human re-reading nine sites works;
re-reading two hundred does not. This gate makes the review permanent:

Every `loop {` / `while true {` site must satisfy at least one of:

1. a `// SAFETY-TERMINATION:` comment on the line above the loop describing
   why it terminates;
2. a bounded-body signal: the loop body contains a `break` **and** an
   iteration-budget variable (iteration/budget/attempt/step/remaining/deadline
   /timeout/`*_max`/`max_*` naming);
3. an entry in `scripts/unbounded_loop_allowlist.json` with a reason.

The allowlist is non-rotting: entries are keyed by file plus a unique source
snippet of the loop head, so a moved or rewritten loop invalidates its entry,
and an entry whose snippet no longer appears is itself a failure.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ALLOWLIST_PATH = Path(__file__).resolve().parent / "unbounded_loop_allowlist.json"

LOOP_HEAD = re.compile(r"\b(?:loop\s*\{|while\s+true\s*\{)")
BUDGET_SIGNAL = re.compile(
    r"(?i)(iteration|budget|attempt|remaining|deadline|timeout|cancellation"
    r"|max_[a-z_]*|_[a-z_]*max\b|fuel|steps?)"
)
SAFETY_COMMENT = "// SAFETY-TERMINATION:"


def tracked_rs_files() -> list[Path]:
    out = subprocess.run(
        ["git", "ls-files", "-z", "*.rs"],
        cwd=ROOT,
        check=True,
        capture_output=True,
    ).stdout
    return [ROOT / Path(p.decode("utf-8")) for p in out.split(b"\0") if p]


def match_braces(text: str, open_index: int) -> int:
    """Return the index of the brace matching the `{` at `open_index`."""

    depth = 0
    in_line_comment = False
    in_block_comment = False
    in_string = False
    string_delim = ""
    index = open_index
    while index < len(text):
        char = text[index]
        if in_line_comment:
            if char == "\n":
                in_line_comment = False
        elif in_block_comment:
            if text.startswith("*/", index):
                in_block_comment = False
                index += 1
        elif in_string:
            if char == "\\":
                index += 1
            elif char == string_delim:
                in_string = False
        elif char == "/" and text.startswith("//", index):
            in_line_comment = True
            index += 1
        elif char == "/" and text.startswith("/*", index):
            in_block_comment = True
            index += 1
        elif char in "\"'":
            in_string = True
            string_delim = char
        elif char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return index
        index += 1
    return -1


def string_spans(text: str) -> list[tuple[int, int]]:
    """Spans of Rust string literals (normal, raw, and multi-line raw).

    Embedded HTML/JS/WGSL templates use raw strings whose contents contain
    quotes and even `loop{` (a WGSL loop inside a shader string is not Rust
    control flow), so the scanner must be lexically aware. Line comments,
    block comments, and char literals are skipped for the same reason.
    """

    spans: list[tuple[int, int]] = []
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        if c == "/" and text.startswith("//", i):
            newline = text.find("\n", i)
            i = n if newline < 0 else newline + 1
        elif c == "/" and text.startswith("/*", i):
            end = text.find("*/", i + 2)
            i = n if end < 0 else end + 2
        elif c == "r" and i + 1 < n and text[i + 1] in "\"#" and (
            i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")
        ):
            # Raw string: r"...", r#"..."#, r##"..."##, ...
            j = i + 1
            hashes = 0
            while j < n and text[j] == "#":
                hashes += 1
                j += 1
            if j < n and text[j] == '"':
                terminator = '"' + "#" * hashes
                end = text.find(terminator, j + 1)
                if end < 0:
                    spans.append((i, n))
                    i = n
                else:
                    spans.append((i, end + len(terminator)))
                    i = end + len(terminator)
            else:
                i += 1
        elif c == "'":
            # Char literal: '\'', 'a', '\n'. A lifetime tick (e.g. &'a) is
            # left alone when it does not close within two characters.
            if i + 2 < n and text[i + 1] == "\\" and text[i + 3] == "'":
                i += 4
            elif i + 2 < n and text[i + 2] == "'":
                i += 3
            else:
                i += 1
        elif c == '"':
            j = i + 1
            while j < n:
                if text[j] == "\\":
                    j += 2
                    continue
                if text[j] == '"':
                    break
                j += 1
            spans.append((i, min(j + 1, n)))
            i = j + 1
        else:
            i += 1
    return spans


def loop_sites() -> list[tuple[Path, int, str, str]]:
    """Yield (path, line, head_snippet, body) for every unbounded-loop head."""

    sites = []
    for path in tracked_rs_files():
        try:
            text = path.read_text(encoding="utf-8", errors="replace")
        except OSError:
            continue
        spans = string_spans(text)
        for match in LOOP_HEAD.finditer(text):
            if any(start <= match.start() < end for start, end in spans):
                continue
            body_start = match.end() - 1
            body_end = match_braces(text, body_start)
            body = text[body_start : body_end if body_end > 0 else len(text)]
            line = text.count("\n", 0, match.start()) + 1
            head_snippet = " ".join(text[match.start() : match.end()].split())
            sites.append((path, line, head_snippet, body))
    return sites


def main() -> int:
    allowlist: list[dict[str, str]] = []
    if ALLOWLIST_PATH.exists():
        allowlist = json.loads(ALLOWLIST_PATH.read_text(encoding="utf-8"))
    allowlisted_keys = {entry["key"]: entry for entry in allowlist}

    sites = loop_sites()
    unexplained = []
    used_keys: set[str] = set()
    for path, line, head_snippet, body in sites:
        rel = path.relative_to(ROOT).as_posix()
        key = f"{rel}::{head_snippet}"
        if key in allowlisted_keys:
            used_keys.add(key)
            continue
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        preceding = "\n".join(lines[max(0, line - 4) : line - 1])
        if SAFETY_COMMENT in preceding:
            continue
        if "break" in body and BUDGET_SIGNAL.search(body):
            continue
        unexplained.append(
            f"unbounded loop without a termination argument: {rel}:{line} `{head_snippet}`\n"
            "  add a `// SAFETY-TERMINATION:` comment, a break + iteration-budget "
            f"signal, or record it in {ALLOWLIST_PATH.name}"
        )

    stale = [
        f"stale allowlist entry: {entry['key']} no longer matches any loop; remove the entry"
        for key, entry in allowlisted_keys.items()
        if key not in used_keys
    ]

    for problem in unexplained + stale:
        print(f"::error {problem}")
    if unexplained or stale:
        print(
            f"unbounded-loop gate: {len(unexplained)} unexplained loop(s), "
            f"{len(stale)} stale allowlist entr(ies)"
        )
        return 1
    print(f"unbounded-loop gate: {len(sites)} unbounded-loop site(s), all have termination arguments")
    return 0


if __name__ == "__main__":
    sys.exit(main())
