#!/usr/bin/env python3
"""Verify release version consistency across the workspace and published bindings.

The canonical version is [workspace.package].version in Cargo.toml. This check
covers Rust package manifests, Cargo.lock workspace packages, Python metadata,
Node's root and platform packages, .NET/Java binding metadata,
release-facing documentation.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CARGO_TOML = ROOT / "Cargo.toml"
CARGO_LOCK = ROOT / "Cargo.lock"
PYPROJECT = ROOT / "ffi" / "python-binding" / "pyproject.toml"
NODE_PACKAGE = ROOT / "ffi" / "node-binding" / "package.json"
NODE_LOCK = ROOT / "ffi" / "node-binding" / "package-lock.json"
NODE_PLATFORM_DIR = ROOT / "ffi" / "node-binding" / "npm"
DOTNET_PROJECT = ROOT / "ffi" / "dotnet-binding" / "src" / "Finkit" / "Finkit.csproj"
JAVA_POM = ROOT / "ffi" / "java-binding" / "pom.xml"
CMAKE_PROJECT = ROOT / "ffi" / "c-binding" / "CMakeLists.txt"
XML_PROJECT_VERSIONS = ((DOTNET_PROJECT, "Version"), (JAVA_POM, "version"))
DOC_VERSION_FILES = (
    ROOT / "README.md",
    ROOT / "docs" / "api-reference.md",
    ROOT / "docs" / "generated" / "version-matrix.md",
    ROOT / "docs" / "installation.md",
    ROOT / "docs" / "python.md",
    ROOT / "ffi" / "python-binding" / "README.md",
    ROOT / "examples" / "README.md",
    ROOT / "docs" / "indicator_registry.json",
)


def _section(text: str, heading: str) -> str:
    start = text.find(heading)
    if start < 0:
        return ""
    tail = text[start + len(heading) :]
    next_heading = re.search(r"(?m)^\[", tail)
    return tail[: next_heading.start()] if next_heading else tail


def read_workspace_version() -> str:
    text = CARGO_TOML.read_text(encoding="utf-8")
    block = _section(text, "[workspace.package]")
    match = re.search(r'(?m)^version\s*=\s*"([^"]+)"', block)
    if not match:
        raise ValueError(f"workspace.package.version not found in {CARGO_TOML}")
    return match.group(1)


def read_pyproject_version() -> str:
    text = PYPROJECT.read_text(encoding="utf-8")
    block = _section(text, "[project]")
    match = re.search(r'(?m)^version\s*=\s*"([^"]+)"', block)
    if not match:
        raise ValueError(f"project.version not found in {PYPROJECT}")
    return match.group(1)


def read_json_version(path: Path) -> str:
    data = json.loads(path.read_text(encoding="utf-8"))
    version = data.get("version")
    if not isinstance(version, str) or not version:
        raise ValueError(f"version not found in {path}")
    return version


def read_node_versions() -> dict[str, str]:
    versions = {str(NODE_PACKAGE.relative_to(ROOT)): read_json_version(NODE_PACKAGE)}
    for path in sorted(NODE_PLATFORM_DIR.glob("*/package.json")):
        versions[str(path.relative_to(ROOT))] = read_json_version(path)
    return versions


def read_node_lock_versions() -> dict[str, str]:
    data = json.loads(NODE_LOCK.read_text(encoding="utf-8"))
    root = data.get("packages", {}).get("")
    if not isinstance(root, dict):
        raise ValueError(f"root package entry not found in {NODE_LOCK}")
    versions: dict[str, str] = {}
    relative = str(NODE_LOCK.relative_to(ROOT))
    versions[relative] = data.get("version", "")
    versions[f"{relative} packages[\"\"]"] = root.get("version", "")
    for name, version in sorted((root.get("optionalDependencies") or {}).items()):
        versions[f"{relative} optionalDependencies.{name}"] = version
    return versions


def read_xml_project_version(path: Path, tag: str) -> str:
    """Read a required release version tag from an XML binding manifest."""
    text = path.read_text(encoding="utf-8")
    pattern = rf"(?m)^[ \t]*<{re.escape(tag)}>([^<]+)</{re.escape(tag)}>[ \t]*$"
    match = re.search(pattern, text)
    if not match:
        raise ValueError(f"{tag} version not found in {path}")
    return match.group(1).strip()

def read_cmake_project_version() -> str:
    text = CMAKE_PROJECT.read_text(encoding="utf-8")
    match = re.search(
        r"project\(finkit\s+VERSION\s+([0-9]+\.[0-9]+\.[0-9]+)",
        text,
        re.MULTILINE,
    )
    if not match:
        raise ValueError(f"CMake project version not found in {CMAKE_PROJECT}")
    return match.group(1)


def read_cargo_package_versions() -> dict[str, str]:
    versions: dict[str, str] = {}
    for path in sorted(ROOT.rglob("Cargo.toml")):
        if any(part in {".git", "target"} for part in path.parts):
            continue
        text = path.read_text(encoding="utf-8")
        block_match = re.search(r"(?ms)^\[package\]\s*(.*?)(?=^\[|\Z)", text)
        if not block_match:
            continue
        block = block_match.group(1)
        if re.search(r"(?m)^version\.workspace\s*=\s*true", block):
            version = "workspace"
        else:
            match = re.search(r'(?m)^version\s*=\s*"([^"]+)"', block)
            if not match:
                continue
            version = match.group(1)
        versions[str(path.relative_to(ROOT))] = version
    return versions


def read_lock_workspace_versions() -> dict[str, str]:
    text = CARGO_LOCK.read_text(encoding="utf-8")
    pattern = re.compile(
        r'\[\[package\]\]\s*name = "(finkit(?:-[^"]+)?)"\s*'
        r'version = "([^"]+)"',
        re.MULTILINE,
    )
    return {match.group(1): match.group(2) for match in pattern.finditer(text)}


def collect_errors(canonical: str) -> list[str]:
    errors: list[str] = []

    for path, version in sorted(read_cargo_package_versions().items()):
        if version not in {"workspace", canonical}:
            errors.append(f"{path}: {version} != {canonical}")

    for name, version in sorted(read_lock_workspace_versions().items()):
        if version != canonical:
            errors.append(f"Cargo.lock {name}: {version} != {canonical}")

    py_version = read_pyproject_version()
    if py_version != canonical:
        errors.append(f"ffi/python-binding/pyproject.toml: {py_version} != {canonical}")

    for path, version in sorted(read_node_versions().items()):
        if version != canonical:
            errors.append(f"{path}: {version} != {canonical}")

    for path, version in sorted(read_node_lock_versions().items()):
        if version != canonical:
            errors.append(f"{path}: {version} != {canonical}")

    for path, tag in XML_PROJECT_VERSIONS:
        version = read_xml_project_version(path, tag)
        if version != canonical:
            errors.append(f"{path.relative_to(ROOT)}: {version} != {canonical}")

    cmake_version = read_cmake_project_version()
    if cmake_version != canonical:
        errors.append(f"{CMAKE_PROJECT.relative_to(ROOT)}: {cmake_version} != {canonical}")
    for path in DOC_VERSION_FILES:
        if not path.exists():
            errors.append(f"missing release-facing document: {path.relative_to(ROOT)}")
            continue
        text = path.read_text(encoding="utf-8")
        if path.name == "indicator_registry.json":
            version = json.loads(text).get("version")
            if version != canonical:
                errors.append(f"{path.relative_to(ROOT)}: {version} != {canonical}")
        else:
            release_versions = set(re.findall(r"(?<!\d)0\.1\.\d+(?!\d)", text))
            stale_versions = sorted(version for version in release_versions if version != canonical)
            if stale_versions:
                errors.append(
                    f"{path.relative_to(ROOT)}: contains stale release version(s) "
                    f"{', '.join(stale_versions)}; expected {canonical}"
                )

    return errors


def replace_first_version(path: Path, canonical: str) -> None:
    text = path.read_text(encoding="utf-8")
    updated, count = re.subn(
        r'(?m)^version\s*=\s*"[^"]+"',
        f'version = "{canonical}"',
        text,
        count=1,
    )
    if count != 1:
        raise ValueError(f"failed to update version in {path}")
    path.write_text(updated, encoding="utf-8")

def replace_xml_project_version(path: Path, tag: str, canonical: str) -> None:
    text = path.read_text(encoding="utf-8")
    pattern = rf"(?m)^([ \t]*<{re.escape(tag)}>)[^<]+(</{re.escape(tag)}>[ \t]*)$"
    replacement = rf"\g<1>{canonical}\g<2>"
    updated, count = re.subn(pattern, replacement, text, count=1)
    if count != 1:
        raise ValueError(f"failed to update {tag} version in {path}")
    path.write_text(updated, encoding="utf-8")


def fix_versions(canonical: str) -> None:
    for path, tag in XML_PROJECT_VERSIONS:
        replace_xml_project_version(path, tag, canonical)

    cmake_text = CMAKE_PROJECT.read_text(encoding="utf-8")
    cmake_text, count = re.subn(
        r"(project\(finkit\s+VERSION\s+)[0-9]+\.[0-9]+\.[0-9]+",
        rf"\g<1>{canonical}",
        cmake_text,
        count=1,
        flags=re.MULTILINE,
    )
    if count != 1:
        raise ValueError(f"failed to update CMake version in {CMAKE_PROJECT}")
    CMAKE_PROJECT.write_text(cmake_text, encoding="utf-8")

    replace_first_version(PYPROJECT, canonical)

    node_paths = [NODE_PACKAGE, *sorted(NODE_PLATFORM_DIR.glob("*/package.json"))]
    for path in node_paths:
        data = json.loads(path.read_text(encoding="utf-8"))
        data["version"] = canonical
        if path == NODE_PACKAGE:
            optional = data.get("optionalDependencies") or {}
            data["optionalDependencies"] = {
                name: canonical for name in optional
            }
        path.write_text(
            json.dumps(data, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )

    lock_data = json.loads(NODE_LOCK.read_text(encoding="utf-8"))
    lock_data["version"] = canonical
    lock_root = lock_data.setdefault("packages", {}).setdefault("", {})
    lock_root["version"] = canonical
    lock_root["optionalDependencies"] = {
        name: canonical for name in (lock_root.get("optionalDependencies") or {})
    }
    NODE_LOCK.write_text(
        json.dumps(lock_data, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )

    for path, version in read_cargo_package_versions().items():
        if version == "workspace":
            continue
        manifest = ROOT / path
        text = manifest.read_text(encoding="utf-8")
        block_match = re.search(r"(?ms)^\[package\]\s*(.*?)(?=^\[|\Z)", text)
        if not block_match:
            continue
        block = block_match.group(1)
        updated_block, count = re.subn(
            r'(?m)^version\s*=\s*"[^"]+"',
            f'version = "{canonical}"',
            block,
            count=1,
        )
        if count == 1:
            start = block_match.start(1)
            manifest.write_text(
                text[:start] + updated_block + text[block_match.end(1) :],
                encoding="utf-8",
            )

    lock = CARGO_LOCK.read_text(encoding="utf-8")
    lock = re.sub(
        r'(\[\[package\]\]\s*name = "finkit(?:-[^"]+)?"\s*'
        r'version = ")[^"]+(")',
        rf"\g<1>{canonical}\g<2>",
        lock,
        flags=re.MULTILINE,
    )
    CARGO_LOCK.write_text(lock, encoding="utf-8")

    for path in DOC_VERSION_FILES:
        if not path.exists() or path.name == "indicator_registry.json":
            continue
        text = path.read_text(encoding="utf-8")
        text = re.sub(r"(?<!\d)0\.1\.\d+(?!\d)", canonical, text)
        path.write_text(text, encoding="utf-8")

    registry = json.loads(
        (ROOT / "docs" / "indicator_registry.json").read_text(encoding="utf-8")
    )
    registry["version"] = canonical
    (ROOT / "docs" / "indicator_registry.json").write_text(
        json.dumps(registry, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="Check release version consistency.")
    parser.add_argument(
        "--fix",
        action="store_true",
        help="Rewrite checked metadata and release documents to the canonical version.",
    )
    args = parser.parse_args()

    try:
        canonical = read_workspace_version()
        if args.fix:
            fix_versions(canonical)
        errors = collect_errors(canonical)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2

    if errors:
        print(f"Version mismatch (canonical = {canonical}):", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print(f"OK: all checked versions match {canonical}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
