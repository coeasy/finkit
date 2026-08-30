#!/usr/bin/env python3
"""Verify workspace version consistency across Rust, Python, and Node bindings.

Reads the canonical version from the root Cargo.toml (`workspace.package.version`)
and compares it against:
  - ffi/python-binding/pyproject.toml  (`project.version`)
  - ffi/node-binding/package.json      (`version`)
  - ffi/node-binding/package.json      (`optionalDependencies` platform packages)

Exit 0 when all versions match; exit 1 on mismatch.

Usage:
    python scripts/check_versions.py
    python scripts/check_versions.py --fix   # rewrite mismatched files to canonical version
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CARGO_TOML = ROOT / "Cargo.toml"
PYPROJECT = ROOT / "ffi" / "python-binding" / "pyproject.toml"
NODE_PACKAGE = ROOT / "ffi" / "node-binding" / "package.json"

WORKSPACE_VERSION_RE = re.compile(
    r"^\[workspace\.package\]\s*$.*?^version\s*=\s*\"([^\"]+)\"",
    re.MULTILINE | re.DOTALL,
)
PYPROJECT_VERSION_RE = re.compile(r"^version\s*=\s*\"([^\"]+)\"", re.MULTILINE)


def read_workspace_version() -> str:
    text = CARGO_TOML.read_text(encoding="utf-8")
    # Prefer parsing only [workspace.package] block for clarity.
    in_block = False
    for line in text.splitlines():
        if line.strip() == "[workspace.package]":
            in_block = True
            continue
        if in_block and line.startswith("[") and not line.startswith("[workspace"):
            break
        if in_block:
            m = re.match(r"^version\s*=\s*\"([^\"]+)\"", line)
            if m:
                return m.group(1)
    m = WORKSPACE_VERSION_RE.search(text)
    if not m:
        raise ValueError(f"workspace.package.version not found in {CARGO_TOML}")
    return m.group(1)


def read_pyproject_version() -> str:
    text = PYPROJECT.read_text(encoding="utf-8")
    in_project = False
    for line in text.splitlines():
        if line.strip() == "[project]":
            in_project = True
            continue
        if in_project and line.startswith("[") and line.strip() != "[project]":
            break
        if in_project:
            m = re.match(r"^version\s*=\s*\"([^\"]+)\"", line)
            if m:
                return m.group(1)
    m = PYPROJECT_VERSION_RE.search(text)
    if not m:
        raise ValueError(f"project.version not found in {PYPROJECT}")
    return m.group(1)


def read_node_version() -> tuple[str, dict[str, str]]:
    data = json.loads(NODE_PACKAGE.read_text(encoding="utf-8"))
    version = data.get("version", "")
    optional = data.get("optionalDependencies") or {}
    return version, {k: str(v) for k, v in optional.items()}


def fix_pyproject_version(canonical: str) -> None:
    text = PYPROJECT.read_text(encoding="utf-8")
    new_text, count = re.subn(
        r"(?m)^version\s*=\s*\"[^\"]+\"",
        f'version = "{canonical}"',
        text,
        count=1,
    )
    if count != 1:
        raise ValueError(f"failed to update version in {PYPROJECT}")
    PYPROJECT.write_text(new_text, encoding="utf-8")


def fix_node_version(canonical: str) -> None:
    data = json.loads(NODE_PACKAGE.read_text(encoding="utf-8"))
    data["version"] = canonical
    optional = data.get("optionalDependencies") or {}
    for key in optional:
        optional[key] = canonical
    data["optionalDependencies"] = optional
    NODE_PACKAGE.write_text(
        json.dumps(data, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="Check cross-package version consistency.")
    parser.add_argument(
        "--fix",
        action="store_true",
        help="Rewrite mismatched Python/Node versions to match workspace Cargo.toml.",
    )
    args = parser.parse_args()

    try:
        canonical = read_workspace_version()
    except ValueError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2

    errors: list[str] = []

    py_ver = read_pyproject_version()
    if py_ver != canonical:
        errors.append(f"pyproject.toml: {py_ver} != {canonical}")
        if args.fix:
            fix_pyproject_version(canonical)
            print(f"fixed pyproject.toml -> {canonical}")

    node_ver, node_optional = read_node_version()
    if node_ver != canonical:
        errors.append(f"package.json version: {node_ver} != {canonical}")
    for pkg, ver in sorted(node_optional.items()):
        if ver != canonical:
            errors.append(f"package.json optionalDependencies[{pkg}]: {ver} != {canonical}")

    if errors and args.fix:
        fix_node_version(canonical)
        print(f"fixed package.json -> {canonical}")
        errors = []

    if errors:
        print("Version mismatch (canonical = workspace Cargo.toml):", file=sys.stderr)
        print(f"  canonical: {canonical}", file=sys.stderr)
        for err in errors:
            print(f"  - {err}", file=sys.stderr)
        return 1

    print(f"OK: all checked versions match {canonical}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
