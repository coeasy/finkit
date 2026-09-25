#!/usr/bin/env python3
"""Fail when a script or generated artefact under `scripts/` has no consumer.

Motivation: `scripts/` had accumulated nine files that nothing referenced any
more -- and they were not merely unused, they were actively misleading:

* `check-versions.ps1` was a second, *weaker* implementation of the version
  gate (`check_versions.py`). It checked `Cargo.toml`/`pyproject.toml`/
  `package.json` but not `Cargo.lock`, `package-lock.json`, `pom.xml`,
  `Finkit.csproj` or any of the dozen gated documents -- and it shipped a `-Fix`
  switch that rewrites version strings. Anyone reaching for it would have
  "verified" a subset and believed the whole gate had passed.
* `check-versions.sh` described itself as a wrapper "for existing local/CI
  callers". There were none.
* `audit_talib_numeric_contract.mjs` re-implemented
  `ffi/node-binding/test/talib_numeric.mjs` (which `npm test` runs in CI) from
  the same fixture -- with weaker assertions.
* `verify_python_binding.py` hard-coded `sys.path.insert(0, r'P:\\llm_code\\...')`,
  a path from a different machine, so it would silently fall back to whatever
  `finkit` happened to be importable -- the exact failure mode the real binding
  tests were written to avoid.
* `with-path.sh` hard-coded `/p/python/miniforge3`, which does not exist on this
  machine.
* `benchmark_full_coverage.py` / `full_accuracy_diagnose.py` hand-listed ~158
  TA-Lib calls; the live 201-vector contract supersedes them.
* `generated_samples/*.rs` claimed "Source of truth: docs/indicator_registry.json
  (ffi block). Regenerate with scripts/gen_binding.py". That file has **zero**
  indicators carrying an `ffi` block (the metadata moved to
  `docs/ffi_registry.json`) and `gen_binding.py` refuses to run, so the stated
  provenance was provably false.

Dead scripts are not inert: they read as documentation, they get copied, and a
second implementation of a gate quietly lowers the bar. This check makes "a
script nobody calls" a build-time failure instead of a code-review discovery.

How a script counts as referenced
---------------------------------
A file is considered consumed if its basename **or** its stem appears in any
other tracked file, or if a wildcard pattern in the corpus matches it (the
Makefile discovers `build-usage-*.sh` with `$(wildcard ...)`, so a literal
mention is not the only legitimate form of reference). Mentions inside the file
itself do not count.

Manual tools
------------
Some files are meant to be invoked by a human rather than by CI -- a Dockerfile
you pass to `docker build -f`, for instance. Those are recorded in
`MANUAL_TOOLS` with a reason. The list is checked in both directions: an entry
that stops being needed fails the check, so it can only shrink by a deliberate
edit.

Exit codes
----------
0  every checkable file is referenced, or every unreferenced one is recorded
1  an unreferenced file is not recorded, or a recorded file is no longer
   unreferenced
"""

from __future__ import annotations

import fnmatch
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# Only these are "scripts or generated artefacts" for the purpose of this check.
CHECK_EXTENSIONS = {".py", ".sh", ".ps1", ".cmd", ".mjs", ".js", ".dockerfile", ".rs"}

# Helper modules loaded by import rather than invoked; they have no callers to
# look for, so they are out of scope.
EXEMPT_STEMS = {"__init__"}

# Intentionally invoked by a human, not by CI or another script.
MANUAL_TOOLS = {
    "scripts/bench-vs-talib.dockerfile": (
        "manual tool: documented for `docker build -f scripts/bench-vs-talib.dockerfile`; "
        "docker-compose.yml and the Makefile use the root Dockerfile instead"
    ),
}

# Files that are themselves part of the reference machinery: a mention here is
# not evidence that a script is used.
SELF_DESCRIBING = {
    "scripts/check_orphan_scripts.py",
}

WILDCARD = re.compile(r"[\w./-]*\*[\w./*-]*")
BRACE = re.compile(r"([\w./-]*)\{([^{}]*)\}")


def tracked_files() -> list[str]:
    out = subprocess.run(
        ["git", "ls-files"], capture_output=True, text=True, check=True, cwd=ROOT
    ).stdout
    # `git ls-files` still lists a path whose working-tree file has been deleted
    # but whose removal is not staged yet; those are gone as far as this check is
    # concerned, and leaving them in would make the gate red for no reason.
    return [line for line in out.splitlines() if line and (ROOT / line).is_file()]


def checkable_scripts(files: list[str]) -> list[str]:
    out = []
    for rel in files:
        if not rel.startswith("scripts/"):
            continue
        path = Path(rel)
        if path.suffix not in CHECK_EXTENSIONS:
            continue
        if path.stem in EXEMPT_STEMS:
            continue
        out.append(rel)
    return sorted(out)


def expand_braces(text: str) -> str:
    """Rewrite `scripts/foo.{sh,ps1}` as `scripts/foo.sh scripts/foo.ps1`.

    The Makefile and its help text use the brace form. The prefix has to be
    repeated for each alternative: expanding to `scripts/foo.sh ps1` would leave
    the second file looking unreferenced, which is a false positive.
    """
    return BRACE.sub(
        lambda m: " ".join(m.group(1) + part.strip() for part in m.group(2).split(",")),
        text,
    )


def read_corpus(files: list[str]) -> dict[str, str]:
    corpus = {}
    for rel in files:
        if rel in SELF_DESCRIBING:
            continue
        try:
            corpus[rel] = expand_braces(
                (ROOT / rel).read_text(encoding="utf-8", errors="replace")
            )
        except (OSError, UnicodeDecodeError):
            continue
    return corpus


def wildcard_tokens(text: str) -> list[str]:
    """Path-shaped glob patterns, e.g. `/scripts/build-usage-*.sh`.

    Deliberately strict, because a loose rule makes the whole check vacuous: a
    bare `*` (`.gitattributes` has `* text=auto`) or `_*` (found inside a
    minified vendor bundle) matches every filename. A token only counts when it
    is a path, its filename part has a literal prefix of at least three
    characters, and that prefix contains an alphanumeric.
    """
    out = []
    for token in WILDCARD.findall(text):
        if "/" not in token:
            continue
        base = Path(token).name
        prefix = base.split("*", 1)[0]
        if len(prefix) < 3 or not any(ch.isalnum() for ch in prefix):
            continue
        out.append(token)
    return out


def is_referenced(rel: str, corpus: dict[str, str], unique_stems: set[str]) -> bool:
    """True if any tracked file other than `rel` refers to it.

    The full basename always counts. The bare stem only counts when no other
    checkable file shares it: `bench-vs-talib` is the stem of a shell script, a
    PowerShell script *and* a Dockerfile, so a stem hit would make the orphaned
    Dockerfile look referenced by its live siblings.
    """
    name = Path(rel).name
    stem = Path(rel).stem
    stem_counts = stem in unique_stems
    for other, text in corpus.items():
        if other == rel:
            continue
        if name in text:
            return True
        if stem_counts and stem in text:
            return True
        # A wildcard pattern may be the only reference (e.g. `build-usage-*.sh`).
        for token in wildcard_tokens(text):
            if fnmatch.fnmatch(name, Path(token).name) or fnmatch.fnmatch(rel, token.lstrip("/")):
                return True
    return False


def main() -> int:
    files = tracked_files()
    scripts = checkable_scripts(files)
    corpus = read_corpus(files)

    stem_tally = Counter(Path(s).stem for s in scripts)
    unique_stems = {stem for stem, count in stem_tally.items() if count == 1}

    unreferenced = [s for s in scripts if not is_referenced(s, corpus, unique_stems)]

    orphans = [s for s in unreferenced if s not in MANUAL_TOOLS]
    stale_allowlist = [s for s in MANUAL_TOOLS if s not in unreferenced]

    print(
        f"scanned {len(scripts)} scripts/artefacts under scripts/ "
        f"against {len(corpus)} tracked files"
    )
    print(f"unreferenced: {len(unreferenced)} ({len(MANUAL_TOOLS)} recorded as manual)")

    if not orphans and not stale_allowlist:
        print("OK: every script has a consumer")
        return 0

    if orphans:
        print("\nFAIL: unreferenced and not recorded:", file=sys.stderr)
        for rel in orphans:
            print(f"  - {rel}", file=sys.stderr)
        print(
            "  Wire it into a workflow/Makefile/doc, delete it, or add it to "
            "MANUAL_TOOLS with a reason.",
            file=sys.stderr,
        )
    if stale_allowlist:
        print("\nFAIL: MANUAL_TOOLS entries that are no longer unreferenced:", file=sys.stderr)
        for rel in stale_allowlist:
            print(f"  - {rel}", file=sys.stderr)
        print("  Remove them so the list can only shrink.", file=sys.stderr)

    print(f"\nFAIL: {len(orphans) + len(stale_allowlist)} problem(s)", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
