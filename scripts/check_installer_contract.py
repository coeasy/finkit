#!/usr/bin/env python3
"""Keep the Windows MSI payload and the cross-platform native archive identical.

Two builders describe the same thing -- what a consumer of the native SDK gets:

  * ``packaging/wix/Product.wxs`` lists the files installed to Program Files.
  * ``scripts/build_native_archive.py`` lists the files packed into the
    portable archive.

Nothing links the two at build time, so they each used to be edited
independently and could disagree: a header added to the archive would be
missing from the MSI, and Windows users would get a different SDK from everyone
else with nothing failing to point it out. Since both ultimately describe one
payload, this gate treats them as one declaration and fails when they drift.

It is deliberately a static comparison. Both files are read and parsed rather
than imported/executed, so the gate still works in a checkout where the release
build has not run -- the point is to catch drift in CI *before* anything is
built.
"""

from __future__ import annotations

import ast
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WIX_PRODUCT = ROOT / "packaging" / "wix" / "Product.wxs"
NATIVE_ARCHIVE = ROOT / "scripts" / "build_native_archive.py"

WIX_NS = {"wix": "http://schemas.microsoft.com/wix/2006/wi"}
PAYLOAD_PREFIX = "$(var.PayloadDir)"


def wxs_members() -> list[str]:
    """Every payload member the MSI installs, as archive-relative paths."""
    if not WIX_PRODUCT.is_file():
        raise SystemExit(f"missing WiX product definition: {WIX_PRODUCT}")

    tree = ET.parse(WIX_PRODUCT)
    members: list[str] = []
    for element in tree.iter():
        if not element.tag.endswith("File"):
            continue
        source = element.get("Source") or ""
        if not source.startswith(PAYLOAD_PREFIX):
            continue
        relative = source[len(PAYLOAD_PREFIX) :].lstrip("\\/")
        members.append(relative.replace("\\", "/"))
    return sorted(members)


def archive_members(platform_slug: str = "windows-x64") -> list[str]:
    """Members of one platform profile in :data:`PLATFORM_PROFILES`.

    ``PLATFORM_PROFILES`` is read out of the module's AST instead of imported so
    that this gate runs without executing rocket-free but import-side-effectful
    module level code, and so it works even if a future dependency is missing.
    """
    if not NATIVE_ARCHIVE.is_file():
        raise SystemExit(f"missing native archive builder: {NATIVE_ARCHIVE}")

    tree = ast.parse(NATIVE_ARCHIVE.read_text(encoding="utf-8"))
    profiles: dict[str, list[str]] = {}
    headers: list[str] = []

    for targets, value in (_binding(n) for n in tree.body):
        if value is None:
            continue
        if "PLATFORM_PROFILES" in targets and isinstance(value, ast.Dict):
            for key_node, value_node in zip(value.keys, value.values):
                if not isinstance(key_node, ast.Constant):
                    continue
                slug = str(key_node.value)
                members: list[str] = []
                for entry in _dict_get(value_node, "members"):
                    first = entry.elts[0] if isinstance(entry, ast.Tuple) and entry.elts else None
                    if isinstance(first, ast.Constant):
                        members.append(str(first.value))
                profiles[slug] = members
        if "HEADER_MEMBERS" in targets and isinstance(value, ast.List):
            for entry in value.elts:
                first = entry.elts[0] if isinstance(entry, ast.Tuple) and entry.elts else None
                if isinstance(first, ast.Constant):
                    headers.append(str(first.value))

    if platform_slug not in profiles:
        raise SystemExit(
            f"{NATIVE_ARCHIVE.name}: no platform profile {platform_slug!r} "
            f"(found {sorted(profiles)})"
        )
    return sorted(profiles[platform_slug] + headers)


def _binding(node: ast.AST) -> tuple[list[str], ast.AST | None]:
    """Extract ``(names, value)`` from a module-level assignment.

    Handles both ``NAME = {...}`` and annotated ``NAME: T = {...}``. The latter
    is what the builders actually use, and matching only plain ``Assign`` nodes
    silently yields an empty table instead of an error -- which is exactly the
    kind of gate that reports success while checking nothing.
    """
    if isinstance(node, ast.Assign):
        targets = [t.id for t in node.targets if isinstance(t, ast.Name)]
        return targets, node.value
    if isinstance(node, ast.AnnAssign):
        targets = [node.target.id] if isinstance(node.target, ast.Name) else []
        return targets, node.value
    return [], None


def _dict_get(node: ast.AST, key: str) -> list[ast.AST]:
    if not isinstance(node, ast.Dict):
        return []
    for key_node, value_node in zip(node.keys, node.values):
        if isinstance(key_node, ast.Constant) and key_node.value == key:
            if isinstance(value_node, ast.List):
                return value_node.elts
            return []
    return []


def main() -> int:
    try:
        installed = wxs_members()
        packed = archive_members()
    except SystemExit as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1

    if not installed:
        print(
            f"FAIL: {WIX_PRODUCT} declares no $(var.PayloadDir) files -- nothing "
            "would be installed",
            file=sys.stderr,
        )
        return 1

    only_msi = sorted(set(installed) - set(packed))
    only_archive = sorted(set(packed) - set(installed))

    if only_msi or only_archive:
        print("FAIL: the MSI payload and the native archive disagree", file=sys.stderr)
        if only_msi:
            print(f"  installed by MSI but not packed: {only_msi}", file=sys.stderr)
        if only_archive:
            print(f"  packed but not installed by MSI: {only_archive}", file=sys.stderr)
        print(
            "\nEdit packaging/wix/Product.wxs and "
            "scripts/build_native_archive.py together: they declare one payload.",
            file=sys.stderr,
        )
        return 1

    print(
        f"OK: MSI payload matches the native archive "
        f"({len(installed)} members) - {WIX_PRODUCT.name}"
    )
    for member in installed:
        print(f"  {member}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
