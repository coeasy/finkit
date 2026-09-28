#!/usr/bin/env python3
"""refresh_release_manifests.py — recompute the release manifests' digests.

`dist/manifest.json` and `dist/python/windows-x64/manifest.json` record a
`size_bytes`/`sha256` pair per shipped artifact. Those numbers were refreshed by
hand, which is the same failure mode the native archive already had: the linker
output is not reproducible, so *every* rebuild changes the digests, and a stale
record is indistinguishable from a correct one by inspection.

This script recomputes both fields from the artifacts on disk and then re-reads
the manifests it just wrote to confirm they match. Run it after any rebuild that
replaces a shipped binary (see docs/release-checklist.md).

Usage:
    python scripts/refresh_release_manifests.py            # update in place
    python scripts/refresh_release_manifests.py --check    # fail if stale

Exit codes: 0 in sync (or updated), 1 an artifact is missing or stale under
`--check`.
"""
from __future__ import annotations

import argparse
import hashlib
import json
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
    updates = 0
    now = datetime.now(timezone.utc).replace(microsecond=0).isoformat()

    for manifest in MANIFESTS:
        if not manifest.is_file():
            problems.append(f'{manifest.relative_to(ROOT)}: missing')
            continue
        doc = json.loads(manifest.read_text(encoding='utf-8'))
        changed = False

        entries = next(
            (doc[key] for key in ENTRY_LIST_KEYS if key in doc), None
        )
        if entries is None:
            # An unknown schema must not be mistaken for an empty manifest:
            # both would yield "OK", but only one is actually verified.
            problems.append(
                f'{manifest.relative_to(ROOT)}: no recognised entry list '
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

    if args.check:
        if problems:
            print('FAIL: release manifests are stale:')
            for p in problems:
                print(f'  - {p}')
            print('FAIL: 1 problem(s)')
            return 1
        print('OK: release manifests match the artifacts on disk')
        return 0

    if problems:
        print('FAIL:')
        for p in problems:
            print(f'  - {p}')
        return 1

    if updates:
        print(f'OK: refreshed {updates} digest record(s) in {len(MANIFESTS)} manifest(s)')
    else:
        print('OK: release manifests already match the artifacts on disk')
    return 0


if __name__ == '__main__':
    sys.exit(main())
