#!/usr/bin/env python3
"""Assemble the native C/C++ SDK archive from a release build.

The archive is the C/C++ half of the release: a consumer links
``bin/finkit_ffi.dll`` against the headers and libs shipped beside it. It has
never had an in-tree generator, so it used to be hand-zipped, which made the
member list and the timestamps drift from release to release.

Members are taken from the release build plus the committed public surface:

    bin/finkit_ffi.dll           <- <target>/release/finkit_ffi.dll
    lib/finkit_ffi.dll.lib       <- <target>/release/finkit_ffi.dll.lib
    lib/finkit_ffi.lib           <- <target>/release/finkit_ffi.lib
    include/finkit.h             <- ffi/c-binding/include/finkit.h
    include/finkit.hpp           <- ffi/c-binding/include/finkit.hpp
    include/finkit_research.h    <- ffi/c-binding/include/finkit_research.h
    include/finkit_research.hpp  <- ffi/c-binding/include/finkit_research.hpp
    share/finkit/LICENSE         <- LICENSE

Every member is stored with the same fixed 1980-01-01 timestamp, so two runs
over identical inputs produce a byte-identical archive. Note that the *linker*
output is not reproducible, so a rebuilt archive legitimately has a different
digest even when no source changed -- refresh the ``size_bytes``/``sha256``
records in ``dist/manifest.json`` and
``dist/python/windows-x64/manifest.json`` when you repack.

The release directory is located the way cargo locates it: ``--target-dir`` if
given, otherwise ``$CARGO_TARGET_DIR/release``, otherwise ``target/release``,
otherwise ``.cargo-target/release``. Honouring ``CARGO_TARGET_DIR`` matters in
checkouts that redirect cargo: without it, packing and ``--verify`` would read
different trees, and since linker output differs between them the comparison
could never succeed.

Usage:
    python scripts/build_native_archive.py                 # build + report digests
    python scripts/build_native_archive.py --verify        # fail if a rebuild would differ
    python scripts/build_native_archive.py --print-members # list what would be packed
"""

from __future__ import annotations

import argparse
import hashlib
import os
import re
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKSPACE_CARGO = ROOT / "Cargo.toml"
C_BINDING_INCLUDE = ROOT / "ffi" / "c-binding" / "include"

# The ZIP epoch floor. Fixing this is what makes the archive reproducible.
FIXED_DATE = (1980, 1, 1, 0, 0, 0)

# (member name in the archive, repo-relative source)
MEMBERS = [
    ("bin/finkit_ffi.dll", "ffi_ffi_dll"),
    ("lib/finkit_ffi.dll.lib", "ffi_ffi_dll_lib"),
    ("lib/finkit_ffi.lib", "ffi_ffi_lib"),
    ("include/finkit.h", "ffi/c-binding/include/finkit.h"),
    ("include/finkit.hpp", "ffi/c-binding/include/finkit.hpp"),
    ("include/finkit_research.h", "ffi/c-binding/include/finkit_research.h"),
    ("include/finkit_research.hpp", "ffi/c-binding/include/finkit_research.hpp"),
    ("share/finkit/LICENSE", "LICENSE"),
]

# Linker outputs, resolved against the chosen target directory.
BUILD_OUTPUTS = {
    "ffi_ffi_dll": "finkit_ffi.dll",
    "ffi_ffi_dll_lib": "finkit_ffi.dll.lib",
    "ffi_ffi_lib": "finkit_ffi.lib",
}


def workspace_version() -> str:
    text = WORKSPACE_CARGO.read_text(encoding="utf-8")
    section = text.split("[workspace.package]", 1)
    if len(section) != 2:
        raise SystemExit(f"{WORKSPACE_CARGO}: no [workspace.package] section")
    match = re.search(r'^version\s*=\s*"([^"]+)"', section[1], re.MULTILINE)
    if not match:
        raise SystemExit(f"{WORKSPACE_CARGO}: no version in [workspace.package]")
    return match.group(1)


def candidate_dirs() -> list[tuple[Path, str]]:
    """Release directories to look in, most authoritative first.

    ``CARGO_TARGET_DIR`` comes first because that is where cargo actually put
    the build. Honouring it is what keeps packing and ``--verify`` pointed at
    the same tree: in a checkout where the environment redirects cargo to
    ``.cargo-target``, preferring ``target/release`` unconditionally would pack
    one tree and verify against the other, and the comparison could never
    succeed -- linker output differs between the two, so
    ``make verify-native-archive`` would be permanently red.
    """
    out: list[tuple[Path, str]] = []
    seen: set[Path] = set()

    def add(path: Path, origin: str) -> None:
        resolved = path.resolve() if path.exists() else path
        if resolved in seen:
            return
        seen.add(resolved)
        out.append((path, origin))

    env = os.environ.get("CARGO_TARGET_DIR", "").strip()
    if env:
        base = Path(env)
        if not base.is_absolute():
            base = ROOT / base
        add(base / "release", f"CARGO_TARGET_DIR={env}")
    add(ROOT / "target" / "release", "cargo's default target/release")
    add(ROOT / ".cargo-target" / "release", "the repo's .cargo-target/release")
    return out


def resolve_target_dir(explicit: Path | None) -> tuple[Path, str | None]:
    """Return ``(release_dir, warning)``.

    The choice is deterministic: an explicit ``--target-dir`` wins, otherwise
    the first candidate from :func:`candidate_dirs` that holds a built
    ``finkit_ffi.dll``. Two trees can coexist and their ``finkit_ffi.dll`` will
    differ, because linker output is not reproducible -- so when the one *not*
    chosen also holds a differing library, say so rather than letting the
    caller assume they got the build they meant. This is a warning, not a
    failure: refusing outright would leave ``make verify-native-archive``
    permanently red in any two-tree checkout.
    """
    if explicit is not None:
        if not explicit.is_dir():
            raise SystemExit(f"--target-dir does not exist: {explicit}")
        if not (explicit / "finkit_ffi.dll").exists():
            raise SystemExit(f"--target-dir has no finkit_ffi.dll: {explicit}")
        return explicit, None

    candidates = candidate_dirs()
    chosen: Path | None = None
    for path, _origin in candidates:
        if (path / "finkit_ffi.dll").exists():
            chosen = path
            break
    if chosen is None:
        raise SystemExit(
            "no release build found; expected finkit_ffi.dll in "
            + " or ".join(str(path) for path, _ in candidates)
            + "\nBuild one first: cargo build -p finkit-ffi --release --locked"
        )

    warning = None
    mine = hashlib.sha256((chosen / "finkit_ffi.dll").read_bytes()).hexdigest()
    for other, _origin in candidates:
        if other == chosen or not (other / "finkit_ffi.dll").exists():
            continue
        theirs = hashlib.sha256((other / "finkit_ffi.dll").read_bytes()).hexdigest()
        if mine != theirs:
            warning = (
                f"note: {other} also holds a *different* finkit_ffi.dll "
                f"(sha256:{theirs[:16]}... vs sha256:{mine[:16]}...); using "
                f"{chosen}. Pass --target-dir to pack the other one."
            )
    return chosen, warning


def member_sources(target_dir: Path) -> list[tuple[str, Path]]:
    resolved: list[tuple[str, Path]] = []
    for member, key in MEMBERS:
        if key in BUILD_OUTPUTS:
            src = target_dir / BUILD_OUTPUTS[key]
        else:
            src = ROOT / key
        resolved.append((member, src))
    return resolved


def build(archive: Path, sources: list[tuple[str, Path]]) -> tuple[int, str]:
    archive.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for member, src in sources:
            info = zipfile.ZipInfo(member, date_time=FIXED_DATE)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            zf.writestr(info, src.read_bytes())
    raw = archive.read_bytes()
    return len(raw), hashlib.sha256(raw).hexdigest()


def main(argv: list[str] | None = None) -> int:
    version = workspace_version()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--target-dir",
        type=Path,
        default=None,
        help=(
            "release output directory (default: $CARGO_TARGET_DIR/release, then "
            "cargo's target/release, then .cargo-target/release)"
        ),
    )
    parser.add_argument(
        "--platform",
        default="windows-x64",
        help="platform slug used in the archive path (default: windows-x64)",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=None,
        help="output archive (default: dist/native/<platform>/finkit-<version>-native-<platform>.zip)",
    )
    parser.add_argument(
        "--verify",
        action="store_true",
        help="do not write; fail if a rebuild would not reproduce the archive on disk",
    )
    parser.add_argument(
        "--print-members",
        action="store_true",
        help="list the members that would be packed, then exit",
    )
    args = parser.parse_args(argv)

    archive = args.out or (
        ROOT
        / "dist"
        / "native"
        / args.platform
        / f"finkit-{version}-native-{args.platform}.zip"
    )

    try:
        target_dir, warning = resolve_target_dir(args.target_dir)
    except SystemExit as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1
    if warning:
        print(warning, file=sys.stderr)

    sources = member_sources(target_dir)
    dll_sha = hashlib.sha256((target_dir / "finkit_ffi.dll").read_bytes()).hexdigest()

    if args.print_members:
        print(f"target dir: {target_dir}")
        print(f"finkit_ffi.dll sha256: {dll_sha}")
        for member, src in sources:
            print(f"{member:<30} <- {src}")
        return 0

    missing = [str(src) for _, src in sources if not src.exists()]
    if missing:
        print("FAIL: missing source file(s):", file=sys.stderr)
        for path in missing:
            print(f"  - {path}", file=sys.stderr)
        return 1

    if args.verify:
        if not archive.exists():
            print(f"FAIL: {archive} does not exist", file=sys.stderr)
            return 1
        with zipfile.ZipFile(archive) as zf:
            current = {info.filename: info.date_time for info in zf.infolist()}
            payloads = {name: zf.read(name) for name in zf.namelist()}
        expected_names = [member for member, _ in sources]
        if sorted(current) != sorted(expected_names):
            print(
                f"FAIL: member list differs\n  archive: {sorted(current)}\n"
                f"  expected: {sorted(expected_names)}",
                file=sys.stderr,
            )
            return 1
        mismatched = [
            (member, src)
            for member, src in sources
            if payloads[member] != src.read_bytes()
        ]
        if mismatched:
            for member, src in mismatched:
                print(
                    f"FAIL: {member} in the archive differs from {src}", file=sys.stderr
                )
            # "differs" on its own is not actionable when two release trees
            # coexist: the caller cannot tell whether the archive is stale or
            # merely packed from the other tree. Name the tree it matches.
            packed_from = payloads.get("bin/finkit_ffi.dll")
            for other, origin in candidate_dirs():
                if other == target_dir:
                    continue
                dll = other / "finkit_ffi.dll"
                if dll.exists() and packed_from == dll.read_bytes():
                    print(
                        f"  the archive was packed from {other} ({origin}); "
                        f"re-run with --target-dir {other}",
                        file=sys.stderr,
                    )
                    break
            return 1
        stamps = set(current.values())
        if stamps != {FIXED_DATE}:
            print(f"FAIL: non-canonical timestamps in the archive: {sorted(stamps)}", file=sys.stderr)
            return 1
        print(f"OK: {archive} matches the current build ({len(expected_names)} members)")
        return 0

    size, digest = build(archive, sources)
    print(f"target dir: {target_dir}")
    print(f"finkit_ffi.dll sha256: {dll_sha}")
    for member, src in sources:
        print(f"  {member:<30} {src.stat().st_size:>10}  <- {src}")
    print()
    print(f"wrote {archive}")
    print(f"size_bytes: {size}")
    print(f"sha256: {digest}")
    print()
    print(
        "note: the linker output is not reproducible, so this digest changes on "
        "every rebuild. Refresh dist/manifest.json and "
        "dist/python/windows-x64/manifest.json."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
