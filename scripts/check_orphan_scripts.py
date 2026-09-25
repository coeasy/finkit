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
A file is considered consumed if its basename appears in any other tracked
file, or if its bare stem appears there at an identifier boundary, or if a
wildcard pattern in the corpus matches it (the Makefile discovers
`build-usage-*.sh` with `$(wildcard ...)`, so a literal mention is not the only
legitimate form of reference). Mentions inside the file itself do not count.

That single-hop test is then peeled to a fixed point. A mention from another
script only counts while *that* script is itself reachable, because a cluster
of migration scripts that only mention each other satisfies the one-hop test
for every member while the group as a whole has no consumer. Six completed
TA-Lib migration codemods hid behind exactly that: four had no referencer at
all, and two were named only by a fifth script that nothing called.

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
    """Every tracked path that still exists in the working tree.

    `-z` is load-bearing, not cosmetic: without it git octal-escapes any path
    containing a non-ASCII byte, so `docs/competitive-analysis/finkit-<cjk>.md`
    arrives as `"docs/...finkit-\\350\\220\\275..."` and `Path.is_file()` then
    fails. Nine tracked files were invisible to this check that way, and every
    one of them was a document that mentions `scripts/` paths -- so the check
    could neither see a reference they made nor find them as consumers.

    A path whose working-tree file has been deleted but whose removal is not
    staged is also still listed; it is gone as far as this check is concerned,
    and leaving it in would make the gate red for no reason.
    """
    out = subprocess.run(
        ["git", "ls-files", "-z"], capture_output=True, text=True, check=True, cwd=ROOT
    ).stdout
    return [line for line in out.split("\0") if line and (ROOT / line).is_file()]


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
    """Rewrite `dir/foo.{sh,ps1}` as `dir/foo.sh dir/foo.ps1`.

    The Makefile and its help text use the brace form. The prefix has to be
    repeated for each alternative: expanding to `dir/foo.sh ps1` would leave
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


def stem_pattern(stem: str) -> re.Pattern[str]:
    """A bare stem only counts as a reference at an identifier boundary.

    Plain substring matching is too generous: a script named `_probe_a.py` was
    "referenced" by `fn the_probe_actually_exercises_the_degenerate_guard()`,
    because `e_probe_a` contains `_probe_a`. A false reference silently excuses
    a genuinely dead script, which is the failure this whole check exists to
    prevent, so the stem must not be flanked by identifier characters.
    """
    return re.compile(r"(?<![A-Za-z0-9_])" + re.escape(stem) + r"(?![A-Za-z0-9_])")


def referencers(
    rel: str,
    corpus: dict[str, str],
    wildcards: dict[str, list[str]],
    unique_stems: set[str],
) -> set[str]:
    """Every tracked file that refers to `rel`.

    The full basename always counts, because it carries its extension and so is
    distinctive on its own. The bare stem only counts when no other checkable
    file shares it: `bench-vs-talib` is the stem of a shell script, a PowerShell
    script *and* a Dockerfile, so a stem hit would make the orphaned Dockerfile
    look referenced by its live siblings.
    """
    name = Path(rel).name
    stem = Path(rel).stem
    pattern = stem_pattern(stem) if stem in unique_stems else None
    out: set[str] = set()
    for other, text in corpus.items():
        if other == rel:
            continue
        if name in text:
            out.add(other)
            continue
        if pattern is not None and pattern.search(text):
            out.add(other)
            continue
        # A wildcard pattern may be the only reference (e.g. `build-usage-*.sh`).
        for token in wildcards.get(other, ()):
            if fnmatch.fnmatch(name, Path(token).name) or fnmatch.fnmatch(
                rel, token.lstrip("/")
            ):
                out.add(other)
                break
    return out


def reachable(scripts: list[str], refs: dict[str, set[str]]) -> set[str]:
    """Scripts transitively reachable from a consumer that is not a script.

    Peeling to a fixed point is what makes the one-hop test meaningful for
    tooling: a group of scripts that only mention one another looks fully
    referenced to a single-hop check, yet nothing outside the group calls it.
    """
    script_set = set(scripts)
    alive = {s for s in scripts if any(r not in script_set for r in refs[s])}
    changed = True
    while changed:
        changed = False
        for s in scripts:
            if s in alive:
                continue
            if any(r in alive for r in refs[s]):
                alive.add(s)
                changed = True
    return alive


def main() -> int:
    files = tracked_files()
    scripts = checkable_scripts(files)
    corpus = read_corpus(files)
    wildcards = {rel: wildcard_tokens(text) for rel, text in corpus.items()}

    stem_tally = Counter(Path(s).stem for s in scripts)
    unique_stems = {stem for stem, count in stem_tally.items() if count == 1}

    refs = {s: referencers(s, corpus, wildcards, unique_stems) for s in scripts}
    alive = reachable(scripts, refs)
    unreferenced = [s for s in scripts if s not in alive]

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
