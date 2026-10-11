#!/usr/bin/env python3
"""refresh_release_manifests.py — recompute the release manifests' digests.

`dist/manifest.json` records a `size_bytes`/`sha256` pair per shipped artifact.
Those numbers were refreshed by hand, which is the same failure mode the native
archive already had: the linker output is not reproducible, so *every* rebuild
changes the digests, and a stale record is indistinguishable from a correct one
by inspection.

This script recomputes both fields from the artifacts on disk and then re-reads
the manifest it just wrote to confirm they match. Run it after any rebuild that
replaces a shipped binary (see docs/release-checklist.md).

Three scripts write this file -- `scripts/build-usage-packages.sh` (the full
multi-language build) and `build-quick.sh` / `build-quick.ps1` at the repository
root (the fast partial build). All three must agree on the schema, and that is a
thing to check rather than assume: they did not agree on `name`, and the
disagreement was invisible because nothing compared the field against the
repository. `workspace_name()` below now does.

Diagnosis order matters here, which is why the checks are not just "is the file
present" and "do the digests match". A manifest left over from a different
product identity -- the stale one on this host read `AlphaTA 1.0.0` while the
workspace was `finkit 0.2.0` -- makes *every* entry fail as "artifact missing",
drowning the one fact worth knowing. So the name/version are compared against
`Cargo.toml` first and reported as the headline, and only then are the digests
checked.

`dist/` is gitignored, so the manifest is build output rather than a repository
artifact: under `--check`, an absent manifest means "nothing was built here",
which is reported and skipped rather than failed. A manifest that exists and
disagrees with the tree is a real finding and still fails.

Usage:
    python scripts/refresh_release_manifests.py            # update in place
    python scripts/refresh_release_manifests.py --check    # fail if stale

Exit codes: 0 in sync (or updated, or nothing built to check), 1 the manifest
disagrees with the tree under `--check`.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# Only `dist/manifest.json` is written by anything in this repository
# (scripts/build-usage-packages.sh and build-quick.sh). The per-platform
# `dist/python/<platform>/manifest.json` used to be listed here too, but no
# script ever produced it -- which made `check-release-manifests` permanently
# red with "missing" for a file nobody could create. Listing only what exists is
# what lets the gate mean anything.
MANIFESTS = [
    ROOT / 'dist' / 'manifest.json',
]

# The manifests do not agree on one name for the list of shipped files, so both
# are accepted. Reading only `artifacts` (a key no writer ever emits) silently
# validated an empty list and let every refresh and check succeed while doing
# nothing -- the gate was green for exactly as long as nobody looked.
ENTRY_LIST_KEYS = ('components', 'artifacts')
ENTRY_PATH_KEYS = ('path', 'file')

# The manifest's `name` is the package identity, read from `core/Cargo.toml`
# rather than hardcoded, so the field is checked against the repository instead
# of against a constant that can drift from it. It had drifted: the three
# writers disagreed -- `scripts/build-usage-packages.sh` wrote "Finkit" while
# `build-quick.sh` and `build-quick.ps1` wrote "finkit" -- so the same field had
# two spellings depending on which script produced the file, and a checker with
# one expected value would have failed half the writers.
NAME_SOURCE = ROOT / 'core' / 'Cargo.toml'


def workspace_name() -> str | None:
    """The `finkit` package name, or None if it cannot be read."""
    try:
        text = NAME_SOURCE.read_text(encoding='utf-8')
    except OSError:
        return None
    section = re.search(r'^\[package\]\s*$(.*?)(?=^\[|\Z)', text, re.MULTILINE | re.DOTALL)
    if not section:
        return None
    name = re.search(r'^name\s*=\s*"([^"]+)"', section.group(1), re.MULTILINE)
    return name.group(1) if name else None


def workspace_version() -> str | None:
    """The `[workspace.package] version`, or None if it cannot be read."""
    try:
        text = (ROOT / 'Cargo.toml').read_text(encoding='utf-8')
    except OSError:
        return None
    section = re.search(r'^\[workspace\.package\]\s*$(.*?)(?=^\[|\Z)',
                        text, re.MULTILINE | re.DOTALL)
    if not section:
        return None
    version = re.search(r'^version\s*=\s*"([^"]+)"', section.group(1), re.MULTILINE)
    return version.group(1) if version else None


def sha256_of(path: Path) -> tuple[str, int]:
    digest = hashlib.sha256()
    with open(path, 'rb') as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b''):
            digest.update(chunk)
    return digest.hexdigest(), path.stat().st_size


def resolve(manifest: Path, entry: str) -> Path:
    """`dist/...` paths are repo-relative; bare names are manifest-relative."""
    if entry.startswith('dist/'):
        return ROOT / entry
    return manifest.parent / entry


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('--check', action='store_true',
                    help='report drift instead of rewriting the manifests')
    args = ap.parse_args()

    problems: list[str] = []
    skipped: list[str] = []
    updates = 0
    now = datetime.now(timezone.utc).replace(microsecond=0).isoformat()
    expected_version = workspace_version()
    expected_name = workspace_name()

    for manifest in MANIFESTS:
        rel = manifest.relative_to(ROOT)
        if not manifest.is_file():
            # `dist/` is gitignored, so this file is build output. Absent means
            # "not built here", not "drifted"; failing on it would make the
            # release gate red for every checkout that has not run a
            # twenty-minute multi-language build.
            skipped.append(str(rel))
            print(f'skip: {rel} is not present '
                  f'(gitignored build output; run scripts/build-usage-packages.sh to produce it)')
            continue
        doc = json.loads(manifest.read_text(encoding='utf-8'))
        changed = False

        # Identity first. A manifest from a previous product name makes every
        # single entry report "artifact missing", which buries the cause under
        # dozens of symptoms.
        drift = []
        if expected_name and doc.get('name') != expected_name:
            drift.append(f'name is {doc.get("name")!r}, {NAME_SOURCE.relative_to(ROOT)} '
                         f'says {expected_name!r}')
        if expected_version and doc.get('version') != expected_version:
            drift.append(f'version is {doc.get("version")!r}, Cargo.toml says {expected_version!r}')
        if drift:
            problems.append(
                f'{rel}: stale product identity -- ' + '; '.join(drift)
                + ' (regenerate: scripts/build-usage-packages.sh)'
            )
            continue

        languages = doc.get('languages')
        if isinstance(languages, list):
            print(f'  {rel}: covers {len(languages)} language(s): '
                  f'{", ".join(languages) or "(none)"}')

        entries = next(
            (doc[key] for key in ENTRY_LIST_KEYS if key in doc), None
        )
        if entries is None:
            # An unknown schema must not be mistaken for an empty manifest:
            # both would yield "OK", but only one is actually verified.
            problems.append(
                f'{rel}: no recognised entry list '
                f'(expected one of {", ".join(ENTRY_LIST_KEYS)})'
            )
            continue

        for entry in entries:
            relative = next(
                (entry[key] for key in ENTRY_PATH_KEYS if key in entry), None
            )
            if relative is None:
                problems.append(
                    f'{manifest.relative_to(ROOT)}: entry has none of '
                    f'{", ".join(ENTRY_PATH_KEYS)}'
                )
                continue
            target = resolve(manifest, relative)
            if not target.is_file():
                problems.append(f'{target.relative_to(ROOT)}: artifact missing')
                continue
            sha, size = sha256_of(target)
            if entry.get('sha256') != sha or entry.get('size_bytes') != size:
                if args.check:
                    problems.append(
                        f'{target.relative_to(ROOT)}: manifest says '
                        f'{entry.get("size_bytes")}/{str(entry.get("sha256"))[:16]}…, '
                        f'disk has {size}/{sha[:16]}…')
                else:
                    entry['sha256'], entry['size_bytes'] = sha, size
                    changed = True
                    updates += 1
            print(f'  {target.relative_to(ROOT)}  {size} bytes  {sha[:16]}…')

        if changed and not args.check:
            doc['generated_at'] = now
            manifest.write_text(json.dumps(doc, indent=2) + '\n', encoding='utf-8')

    tail = f' ({len(skipped)} not built here, skipped)' if skipped else ''

    if args.check:
        if problems:
            print('FAIL: release manifest does not match the tree:')
            for p in problems:
                print(f'  - {p}')
            print(f'FAIL: {len(problems)} problem(s)')
            return 1
        if skipped:
            # Nothing was verified, so the honest word is "skipped", not "OK
            # match" -- the difference matters to a release run.
            print('OK: nothing to verify' + tail)
            return 0
        print('OK: release manifest matches the artifacts on disk')
        return 0

    if problems:
        print('FAIL:')
        for p in problems:
            print(f'  - {p}')
        return 1

    processed = len(MANIFESTS) - len(skipped)
    if updates:
        print(f'OK: refreshed {updates} digest record(s) in {processed} manifest(s){tail}')
    elif processed == 0:
        print('OK: nothing to refresh' + tail)
    else:
        print('OK: release manifest already matches the artifacts on disk')
    return 0


if __name__ == '__main__':
    sys.exit(main())
