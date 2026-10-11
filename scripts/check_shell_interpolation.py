#!/usr/bin/env python3
"""Reject shell variables interpolated into an *unquoted* interpreter heredoc.

The bug this exists for
-----------------------
`scripts/build-usage-packages.sh` wrote the release manifest from an inline
Python program whose source interpolated `${DIST}`. Under Git Bash on Windows
`$ROOT` is `/p/repo` -- a POSIX path with no drive letter -- so the Python that
then ran saw `pathlib.Path("/p/repo/dist")`, which Windows resolves to
`\\p\\repo\\dist` relative to the current drive, and the step died with:

    FileNotFoundError: [Errno 2] No such file or directory:
      '\\p\\...\\dist\\manifest.json'

Nothing was wrong with the build. The C and Go packages had already been
produced; they simply were not recorded, which is why
`refresh_release_manifests.py --check` reported thirty-"artifact missing" lines
for artifacts that were sitting on disk, and why the manifest on this host still
described `AlphaTA 1.0.0` from a previous product identity.

The failure is silent in the way that costs the most: it only appears on one
platform, at the end of a long build, and it looks like a missing dependency
rather than a quoting mistake.

The rule
--------
An unquoted heredoc lets the shell rewrite the program's body, so any variable
that reaches it is text, not a value -- and the shell has no idea the text is a
path. A quoted heredoc (`<<'PY'`) disables that rewriting entirely and forces the
data across the boundary as an environment variable, where the reader decides
how to interpret it. So: an interpreter heredoc that interpolates a shell
variable must be quoted.

Interpolating nothing is fine, quoting is fine, and non-interpreter heredocs
(plain text, SQL, shell) are out of scope -- only the shell-to-Python boundary
carries this particular hazard.

Only *paths* are checked, not every interpolated value. A version string or a
comma-separated language list crosses the same boundary harmlessly; flagging
those would make the gate noisy enough that someone would switch it off, which
is worse than not having it. A line is in scope when the value reaches something
that opens or locates a file (`pathlib.Path`, `open`, `read_text`, …), or when
the variable is named like a path (`*_DIR`, `DIST*`, `OUT*`, …).

Declared exceptions
-------------------
A variable is allowed through an unquoted heredoc when the file says so *and
says why*, on a `# shell-interp-ok: <VAR> -- <reason>` comment:

    # shell-interp-ok: DIST_WIN -- converted with `cygpath -w` on line 42
    ...
    root = pathlib.Path(r"${DIST_WIN}")

`build-quick.sh` is the case this exists for: it interpolates `DIST_WIN`, but
`DIST_WIN` was already produced by `cygpath -w`, so the value *is* a native path
and quoting is not required for correctness. Without a way to record that, the
choice is a gate that fails a correct script or a gate that is switched off.

The reason is mandatory and the declarations are checked in both directions: a
declaration whose variable no longer appears anywhere in the file fails the
gate, so the list can only shrink. Same shape as `MANUAL_TOOLS` in
`check_orphan_scripts.py` and the reason requirement in
`check_dead_code_allows.py`.

Exit codes
----------
0  every unquoted interpreter heredoc interpolation is quoted or declared
1  at least one is neither, or a declaration is stale
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# Shell surfaces that can hold a heredoc. `Makefile` recipes are shell too, and
# are the most likely place for this to reappear.
SUFFIXES = (".sh", ".bash", ".zsh", ".cmd")
EXTRA_FILES = ("Makefile",)

INTERPRETERS = ("python", "python3", "perl", "ruby", "node")
MAX_HEREDOC_LINES = 200

# `<<PY`, `<<-PY`, `<<'PY'`, `<<"PY"` -- group 2 is the quote char, if any.
HEREDOC_RE = re.compile(
    r"<<(?P<dash>-?)\s*(?P<quote>['\"]?)(?P<tag>[A-Za-z_][A-Za-z0-9_]*)(?P=quote)"
)
SHELL_VAR_RE = re.compile(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}|\$([A-Za-z_][A-Za-z0-9_]*)")

# Only *path* interpolation is a hazard, so the gate has to tell a path from a
# version string. Two signals, either is enough:
#   * the line hands the value to something that opens or locates a file;
#   * the variable is named like a path by convention.
# A blob of text can carry the wrong separator on Windows without breaking, and
# flagging it would make the gate noisy enough to be switched off. These are the
# calls the two real bugs in this repository went through.
PATH_CONTEXT_RE = re.compile(
    r"pathlib\.Path|(?<![\w.])Path\(|(?<![\w.])open\(|read_text|write_text"
    r"|rglob|(?<![\w.])glob\(|shutil\.|os\.path|(?<![\w.])mkdir\(|\.resolve\("
)
PATH_NAME_RE = re.compile(
    r"^(?:[A-Za-z0-9]*_)?(?:ROOT|DIR|DIST|PATH|PREFIX|HOME|TARGET|VENV|BUILD|OUT|OUTPUT|SOURCE|SRC|DEST|BASE|TMP|TEMP)"
    r"(?:_[A-Za-z0-9]*)?$",
    re.IGNORECASE,
)


def is_path_suspect(line: str, var: str) -> bool:
    return bool(PATH_CONTEXT_RE.search(line)) or bool(PATH_NAME_RE.match(var))

# `# shell-interp-ok: VAR -- why`. The separator is required: a declaration
# without a reason is the thing this file exists to prevent.
DECLARATION_RE = re.compile(
    r"#\s*shell-interp-ok:\s*(?P<var>[A-Za-z_][A-Za-z0-9_]*)\s*(?:--|—|–)\s*(?P<reason>\S.*)$"
)
MIN_REASON_CHARS = 15


def tracked_shell_files() -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=ROOT,
        capture_output=True,
        encoding="utf-8",
        errors="replace",
        check=True,
    ).stdout
    files = [line for line in out.split("\0") if line]
    return sorted(
        f for f in files
        if (f.endswith(SUFFIXES) or Path(f).name in EXTRA_FILES)
        and (ROOT / f).is_file()
    )


def shell_vars(text: str) -> list[str]:
    return [a or b for a, b in SHELL_VAR_RE.findall(text)]


def declared_vars(lines: list[str]) -> tuple[dict[str, str], list[str]]:
    """Return ({var: reason}, malformed_declaration_lines)."""
    declared: dict[str, str] = {}
    malformed: list[str] = []
    for number, line in enumerate(lines, start=1):
        if "shell-interp-ok" not in line:
            continue
        match = DECLARATION_RE.search(line)
        if not match or len(match.group("reason")) < MIN_REASON_CHARS:
            malformed.append(f"line {number}: needs `# shell-interp-ok: VAR -- <reason>`")
            continue
        declared[match.group("var")] = match.group("reason")
    return declared, malformed


def scan(path: str) -> list[tuple[int, str, list[tuple[int, str]]]]:
    """Return (opener_line, tag, [(body_line, variable)]) per offender."""
    lines = (ROOT / path).read_text(encoding="utf-8", errors="replace").splitlines()
    offenders: list[tuple[int, str, list[tuple[int, str]]]] = []

    for index, line in enumerate(lines):
        match = HEREDOC_RE.search(line)
        if not match or match.group("quote"):
            continue  # quoted (or no heredoc): the shell rewrites nothing
        tag = match.group("tag")
        if not any(word in line for word in INTERPRETERS):
            continue

        body: list[tuple[int, str]] = []
        terminated = False
        for offset in range(index + 1, min(index + 1 + MAX_HEREDOC_LINES, len(lines))):
            candidate = lines[offset]
            if candidate.strip() == tag:
                terminated = True
                break
            for name in shell_vars(candidate):
                if is_path_suspect(candidate, name):
                    body.append((offset + 1, name))  # 1-based for reporting
        if not terminated:
            # An unterminated heredoc is a different bug; do not claim it here.
            continue
        if body:
            offenders.append((index + 1, tag, body))

    return offenders


def main() -> int:
    files = tracked_shell_files()
    findings: list[str] = []
    stale: list[str] = []

    for path in files:
        lines = (ROOT / path).read_text(encoding="utf-8", errors="replace").splitlines()
        declared, malformed = declared_vars(lines)
        for bad in malformed:
            findings.append(f"{path}: {bad}")

        used_declared: set[str] = set()
        for opener, tag, body in scan(path):
            escaped = [name for _, name in body if name in declared]
            used_declared.update(escaped)
            remaining = [(line, name) for line, name in body if name not in declared]
            if not remaining:
                continue
            shown = ", ".join(f"{name} (line {line})" for line, name in remaining[:4])
            if len(remaining) > 4:
                shown += f", +{len(remaining) - 4} more"
            findings.append(
                f"{path}:{opener}: `<<{tag}` feeds an interpreter and interpolates "
                f"{shown}"
            )

        for name in declared:
            if name not in used_declared:
                stale.append(f"{path}: declaration for {name!r} is no longer used")

    print(f"scanned {len(files)} shell file(s) for unquoted interpreter heredocs")

    if not findings and not stale:
        print("OK: every interpreter heredoc is quoted, or its variables are declared")
        return 0

    if findings:
        print(f"FAIL: {len(findings)} unquoted interpolation(s):")
        for finding in findings:
            print(f"  - {finding}")
    for line in stale:
        print(f"FAIL: {line}")
    print(
        "\nQuote the heredoc (`<<'PY'`) and pass the values as environment\n"
        "variables instead. A POSIX path handed to a native tool is not a path:\n"
        "Windows Python turns `/p/repo/dist` into `\\p\\repo\\dist`, and Windows Go\n"
        "rejects it in a `replace` directive. When a native path really is needed,\n"
        "convert it first (`cygpath -w`, see `native_path` in\n"
        "scripts/build-usage-packages.sh)."
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
