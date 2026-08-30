#!/usr/bin/env python3
"""Fail when public Finkit package versions drift from the release version."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
EXPECTED = "0.1.0"
errors: list[str] = []


def fail(label: str, actual: str | None) -> None:
    errors.append(f"{label}: expected {EXPECTED}, found {actual!r}")


def regex_version(path: Path, pattern: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    match = re.search(pattern, text, re.MULTILINE)
    actual = match.group(1) if match else None
    if actual != EXPECTED:
        fail(label, actual)


# Rust workspace is the source of truth for every workspace crate.
regex_version(
    ROOT / "Cargo.toml",
    r"(?ms)^\[workspace\.package\].*?^version\s*=\s*\"([^\"]+)\"",
    "Cargo workspace",
)

# Python distribution metadata and runtime version must agree.
regex_version(
    ROOT / "ffi/python-binding/pyproject.toml",
    r"(?ms)^\[project\].*?^version\s*=\s*\"([^\"]+)\"",
    "Python distribution",
)
regex_version(
    ROOT / "ffi/python-binding/finkit/__init__.py",
    r"^__version__\s*=\s*\"([^\"]+)\"",
    "Python runtime",
)

# Node main package and every native optional package must be on one version.
node_root = ROOT / "ffi/node-binding"
main_node = json.loads((node_root / "package.json").read_text(encoding="utf-8"))
if main_node.get("version") != EXPECTED:
    fail("Node main package", main_node.get("version"))

for package_file in sorted((node_root / "npm").glob("*/package.json")):
    package = json.loads(package_file.read_text(encoding="utf-8"))
    if package.get("version") != EXPECTED:
        fail(f"Node native package {package_file.parent.name}", package.get("version"))

for dependency, version in sorted(main_node.get("optionalDependencies", {}).items()):
    if version != EXPECTED:
        fail(f"Node optional dependency {dependency}", version)

# .NET package metadata.
dotnet = ET.parse(ROOT / "ffi/dotnet-binding/src/AlphaTA/AlphaTA.csproj").getroot()
dotnet_version = dotnet.findtext(".//Version")
if dotnet_version != EXPECTED:
    fail(".NET package", dotnet_version)

# Java Maven metadata. Maven's XML namespace must be included in queries.
java_root = ET.parse(ROOT / "ffi/java-binding/pom.xml").getroot()
ns = {"m": "http://maven.apache.org/POM/4.0.0"}
java_version = java_root.findtext("m:version", namespaces=ns)
if java_version != EXPECTED:
    fail("Java package", java_version)

# Cargo.lock must describe every local AlphaTA compatibility crate at the same
# release version. This catches a common failure mode where Cargo.toml is bumped
# but a release is built from a stale lock file.
lock_text = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
for name, version in re.findall(
    r'(?ms)^\[\[package\]\]\s*\nname\s*=\s*"(alpha-ta-[^"]+)"\s*\nversion\s*=\s*"([^"]+)"',
    lock_text,
):
    if version != EXPECTED:
        fail(f"Cargo.lock package {name}", version)

if errors:
    print("Finkit version alignment check failed:", file=sys.stderr)
    for error in errors:
        print(f"  - {error}", file=sys.stderr)
    raise SystemExit(1)

print(f"Finkit version alignment OK: {EXPECTED}")
