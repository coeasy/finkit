#!/usr/bin/env python3
"""Assemble the native C/C++ SDK archive from a release build.

The archive is the C/C++ half of the release: a consumer links against
``lib/libfinkit_ffi.a`` (or Windows' ``finkit_ffi.lib``) using the headers
shipped beside it, and loads the shared library at run time. It has never had
an in-tree generator, so it used to be hand-zipped, which made the member list
and the timestamps drift from release to release.

Members per platform (every profile also ships the four public headers and the
license, so a consumer sees the same layout everywhere)::

    windows-x64   bin/finkit_ffi.dll  lib/finkit_ffi.dll.lib  lib/finkit_ffi.lib
    linux-x64     lib/libfinkit_ffi.so                        lib/libfinkit_ffi.a
    macos-*       lib/libfinkit_ffi.dylib                     lib/libfinkit_ffi.a

Every member is stored with the same fixed 1980-01-01 timestamp, so two runs
over identical inputs produce a byte-identical archive. Note that the *linker*
output is not reproducible, so a rebuilt archive legitimately has a different
digest even when no source changed -- refresh the ``size_bytes``/``sha256``
records in ``dist/manifest.json`` when you repack.

The release directory is resolved the way cargo resolves it: ``--target-dir`` if
given, otherwise ``$CARGO_TARGET_DIR/release``, otherwise ``target/release``,
otherwise ``.cargo-target/release``. Honouring ``CARGO_TARGET_DIR`` matters in
checkouts that redirect cargo: without it, packing and ``--verify`` would read
different trees, and since linker output differs between them the comparison
could never succeed.

This script is the single definition of the native payload.
``packaging/wix/Product.wxs`` lists the same files for the Windows MSI and
``scripts/check_installer_contract.py`` fails when the two disagree, so there is
no second copy of this list to drift.

Usage:
    python scripts/build_native_archive.py                 # build + report digests
    python scripts/build_native_archive.py --verify        # fail if a rebuild would differ
    python scripts/build_native_archive.py --print-members # list what would be packed
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import os
import platform as platform_mod
import re
import sys
import tarfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKSPACE_CARGO = ROOT / "Cargo.toml"
C_BINDING_INCLUDE = ROOT / "ffi" / "c-binding" / "include"

# The archive epoch floor. Fixing this is what makes the output reproducible.
FIXED_DATE = (1980, 1, 1, 0, 0, 0)
FIXED_EPOCH = 315532800  # 1980-01-01T00:00:00Z, POSIX timestamp version of the above

HEADER_MEMBERS = [
    ("include/finkit.h", "ffi/c-binding/include/finkit.h"),
    ("include/finkit.hpp", "ffi/c-binding/include/finkit.hpp"),
    ("include/finkit_research.h", "ffi/c-binding/include/finkit_research.h"),
    ("include/finkit_research.hpp", "ffi/c-binding/include/finkit_research.hpp"),
    ("share/finkit/LICENSE", "LICENSE"),
]

# key -> (member path within the archive, file name inside the target dir)
#
# ``runtime`` is the member that identifies the build tree: it is the one a
# ``--verify`` mismatch is reported against, and the one whose presence decides
# whether a candidate directory holds a usable release build.
PLATFORM_PROFILES: dict[str, dict[str, object]] = {
    "windows-x64": {
        "extension": "zip",
        "runtime": "finkit_ffi.dll",
        "members": [
            ("bin/finkit_ffi.dll", "finkit_ffi.dll"),
            ("lib/finkit_ffi.dll.lib", "finkit_ffi.dll.lib"),
            ("lib/finkit_ffi.lib", "finkit_ffi.lib"),
        ],
    },
    "linux-x64": {
        "extension": "tar.gz",
        "runtime": "libfinkit_ffi.so",
        "members": [
            ("lib/libfinkit_ffi.so", "libfinkit_ffi.so"),
            ("lib/libfinkit_ffi.a", "libfinkit_ffi.a"),
        ],
    },
    "linux-aarch64": {
        "extension": "tar.gz",
        "runtime": "libfinkit_ffi.so",
        "members": [
            ("lib/libfinkit_ffi.so", "libfinkit_ffi.so"),
            ("lib/libfinkit_ffi.a", "libfinkit_ffi.a"),
        ],
    },
    "macos-x64": {
        "extension": "tar.gz",
        "runtime": "libfinkit_ffi.dylib",
        "members": [
            ("lib/libfinkit_ffi.dylib", "libfinkit_ffi.dylib"),
            ("lib/libfinkit_ffi.a", "libfinkit_ffi.a"),
        ],
    },
    "macos-arm64": {
        "extension": "tar.gz",
        "runtime": "libfinkit_ffi.dylib",
        "members": [
            ("lib/libfinkit_ffi.dylib", "libfinkit_ffi.dylib"),
            ("lib/libfinkit_ffi.a", "libfinkit_ffi.a"),
        ],
    },
}

DEFAULT_PLATFORM = "windows-x64"


def detect_platform() -> str:
    """Map the running interpreter onto one of :data:`PLATFORM_PROFILES`.

    Detection is explicit rather than a bare ``sys.platform`` format string so
    that an unsupported combination fails here with the list of what *is*
    supported, instead of later with a confusing "no such profile" KeyError.
    """
    system = platform_mod.system()
    machine = platform_mod.machine().lower()
    if system == "Windows":
        return "windows-x64"
    if system == "Linux":
        return "linux-aarch64" if machine in {"aarch64", "arm64"} else "linux-x64"
    if system == "Darwin":
        return "macos-arm64" if machine in {"arm64", "aarch64"} else "macos-x64"
    raise SystemExit(
        f"cannot detect a native platform profile for {system}/{machine}; "
        f"pass --platform explicitly (one of: {', '.join(sorted(PLATFORM_PROFILES))})"
    )


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


def resolve_target_dir(
    explicit: Path | None, runtime_name: str
) -> tuple[Path, str | None]:
    """Return ``(release_dir, warning)``.

    The choice is deterministic: an explicit ``--target-dir`` wins, otherwise
    the first candidate from :func:`candidate_dirs` that holds a built runtime
    library. Two trees can coexist and their runtime library will differ,
    because linker output is not reproducible -- so when the one *not* chosen
    also holds a differing library, say so rather than letting the caller assume
    they got the build they meant. This is a warning, not a failure: refusing
    outright would leave ``make verify-native-archive`` permanently red in any
    two-tree checkout.
    """
    if explicit is not None:
        if not explicit.is_dir():
            raise SystemExit(f"--target-dir does not exist: {explicit}")
        if not (explicit / runtime_name).exists():
            raise SystemExit(f"--target-dir has no {runtime_name}: {explicit}")
        return explicit, None

    candidates = candidate_dirs()
    chosen: Path | None = None
    for path, _origin in candidates:
        if (path / runtime_name).exists():
            chosen = path
            break
    if chosen is None:
        raise SystemExit(
            f"no release build found; expected {runtime_name} in "
            + " or ".join(str(path) for path, _ in candidates)
            + "\nBuild one first: cargo build -p finkit-ffi --release --locked"
        )

    warning = None
    mine = hashlib.sha256((chosen / runtime_name).read_bytes()).hexdigest()
    for other, _origin in candidates:
        if other == chosen or not (other / runtime_name).exists():
            continue
        theirs = hashlib.sha256((other / runtime_name).read_bytes()).hexdigest()
        if mine != theirs:
            warning = (
                f"note: {other} also holds a *different* {runtime_name} "
                f"(sha256:{theirs[:16]}... vs sha256:{mine[:16]}...); using "
                f"{chosen}. Pass --target-dir to pack the other one."
            )
    return chosen, warning


def member_sources(platform_slug: str, target_dir: Path) -> list[tuple[str, Path]]:
    profile = PLATFORM_PROFILES[platform_slug]
    resolved: list[tuple[str, Path]] = []
    for member, built_name in profile["members"]:  # type: ignore[union-attr]
        resolved.append((member, target_dir / built_name))
    for member, repo_relative in HEADER_MEMBERS:
        resolved.append((member, ROOT / repo_relative))
    return resolved


def build_zip(archive: Path, sources: list[tuple[str, Path]]) -> None:
    archive.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for member, src in sources:
            info = zipfile.ZipInfo(member, date_time=FIXED_DATE)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            zf.writestr(info, src.read_bytes())


def build_tar_gz(archive: Path, sources: list[tuple[str, Path]]) -> None:
    """Write a reproducible ``tar.gz``.

    Reproducibility needs three normalisations that ``tarfile`` does not apply
    on its own: member order (otherwise it follows whatever order the filesystem
    handed back), every mtime/uid/gid (otherwise every member carries the build
    time), *and* the gzip header's own mtime field, which gzip sets to the wall
    clock even when the tar members are all epoch-zeroed. Missing the last one
    makes two otherwise-identical archives differ in their first bytes.
    """
    archive.parent.mkdir(parents=True, exist_ok=True)
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w", format=tarfile.GNU_FORMAT) as tf:
        for member, src in sorted(sources, key=lambda item: item[0]):
            info = tf.gettarinfo(str(src), arcname=member)
            info.mtime = FIXED_EPOCH
            info.uid = 0
            info.gid = 0
            info.uname = "root"
            info.gname = "root"
            info.mode = 0o644
            with src.open("rb") as handle:
                tf.addfile(info, handle)
    raw = buffer.getvalue()
    with gzip.GzipFile(filename="", mode="wb", fileobj=open(archive, "wb"), mtime=0) as gz:
        gz.write(raw)


def write_archive(archive: Path, sources: list[tuple[str, Path]]) -> tuple[int, str]:
    if archive.name.endswith(".zip"):
        build_zip(archive, sources)
    else:
        build_tar_gz(archive, sources)
    raw = archive.read_bytes()
    return len(raw), hashlib.sha256(raw).hexdigest()


def read_members(archive: Path) -> dict[str, bytes]:
    if archive.name.endswith(".zip"):
        with zipfile.ZipFile(archive) as zf:
            return {name: zf.read(name) for name in zf.namelist()}
    with tarfile.open(archive, mode="r:gz") as tf:
        payloads: dict[str, bytes] = {}
        for info in tf.getmembers():
            if not info.isfile():
                continue
            handle = tf.extractfile(info)
            if handle is None:
                continue
            payloads[info.name] = handle.read()
        return payloads


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
        default=None,
        help=(
            "platform slug used in the archive path "
            f"(default: autodetect; one of {', '.join(sorted(PLATFORM_PROFILES))})"
        ),
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=None,
        help=(
            "output archive (default: "
            "dist/native/<platform>/finkit-<version>-native-<platform>.<ext>)"
        ),
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

    platform_slug = args.platform or detect_platform()
    if platform_slug not in PLATFORM_PROFILES:
        print(
            f"FAIL: unknown platform {platform_slug!r}; "
            f"expected one of {sorted(PLATFORM_PROFILES)}",
            file=sys.stderr,
        )
        return 1

    profile = PLATFORM_PROFILES[platform_slug]
    extension = str(profile["extension"])
    runtime_name = str(profile["runtime"])

    archive = args.out or (
        ROOT
        / "dist"
        / "native"
        / platform_slug
        / f"finkit-{version}-native-{platform_slug}.{extension}"
    )

    try:
        target_dir, warning = resolve_target_dir(args.target_dir, runtime_name)
    except SystemExit as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1
    if warning:
        print(warning, file=sys.stderr)

    sources = member_sources(platform_slug, target_dir)
    runtime_sha = hashlib.sha256((target_dir / runtime_name).read_bytes()).hexdigest()

    if args.print_members:
        print(f"target dir: {target_dir}")
        print(f"{runtime_name} sha256: {runtime_sha}")
        for member, src in sources:
            print(f"{member:<30} <- {src}")
        return 0

    missing = [str(src) for _, src in sources if not src.exists()]
    if missing:
        print("FAIL: missing source file(s):", file=sys.stderr)
        for path_str in missing:
            print(f"  - {path_str}", file=sys.stderr)
        print(
            "\nHint: run 'cargo build -p finkit-ffi --release --locked' first.",
            file=sys.stderr,
        )
        return 1

    if args.verify:
        if not archive.exists():
            print(f"FAIL: {archive} does not exist", file=sys.stderr)
            return 1
        payloads = read_members(archive)
        expected_names = [member for member, _ in sources]
        if sorted(payloads) != sorted(expected_names):
            print(
                f"FAIL: member list differs\n  archive: {sorted(payloads)}\n"
                f"  expected: {sorted(expected_names)}",
                file=sys.stderr,
            )
            return 1
        mismatched = [
            (member, src) for member, src in sources if payloads[member] != src.read_bytes()
        ]
        if mismatched:
            for member, src in mismatched:
                print(
                    f"FAIL: {member} in the archive differs from {src}", file=sys.stderr
                )
            # "differs" on its own is not actionable when two release trees
            # coexist: the caller cannot tell whether the archive is stale or
            # merely packed from the other tree. Name the tree it matches.
            packed_from = payloads.get(
                next(iter(profile["members"]))[0]  # type: ignore[index]
            )
            for other, origin in candidate_dirs():
                if other == target_dir:
                    continue
                candidate = other / runtime_name
                if candidate.exists() and packed_from == candidate.read_bytes():
                    print(
                        f"  the archive was packed from {other} ({origin}); "
                        f"re-run with --target-dir {other}",
                        file=sys.stderr,
                    )
                    break
            return 1
        print(f"OK: {archive} matches the current build ({len(expected_names)} members)")
        return 0

    size, digest = write_archive(archive, sources)
    print(f"target dir: {target_dir}")
    print(f"{runtime_name} sha256: {runtime_sha}")
    for member, src in sources:
        print(f"  {member:<30} {src.stat().st_size:>10}  <- {src}")
    print()
    print(f"wrote {archive}")
    print(f"size_bytes: {size}")
    print(f"sha256: {digest}")
    print()
    print(
        "note: the linker output is not reproducible, so this digest changes on "
        "every rebuild. Refresh dist/manifest.json when you repack."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
