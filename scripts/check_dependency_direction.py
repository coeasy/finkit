#!/usr/bin/env python3
"""Enforce the workspace layering contract (V5 plan §4 / this review's 阶段 4).

Why this exists
---------------
`core` carries the numeric kernel, the formula language, streaming state, the
semantic graph and the runtime — the whole computation layer. Everything else
either uses it (factor-analysis, visualization), wraps its C ABI
(ffi-common), or adapts it to a language/CLI (cli, wasm, ffi/*-binding).

That direction is the product contract: **a kernel may never depend on an
adapter**. A single `use finkit_cli` (or a reverse `path =` dependency) inside
`core` would compile fine and silently pull a presentation/FFI layer into the
hot path, invert the build graph, and make `cargo check -p finkit` require the
whole world. `cargo` will not object, so this gate does.

The layering
------------
    L0  finkit                                  (kernels, contracts, runtime)
    L1  finkit-factor-analysis                  (research workflows)
        finkit-visualization                    (rendering)
    L2  finkit-ffi-common                       (shared C ABI)
    L3  finkit-cli, finkit-wasm, finkit-ffi,    (adapters: CLI, language bindings)
        finkit-python, finkit-node, finkit-go,
        finkit-java, finkit-dotnet, finkit-ios,
        finkit-android

Rule: an intra-workspace dependency edge `u -> v` must satisfy
`layer(v) < layer(u)` (strictly lower). A same-layer or upward edge is a
violation, and so is an edge to a crate that has not been assigned a layer —
a new workspace member must be placed deliberately, not inherited silently.

Usage
-----
    python scripts/check_dependency_direction.py
    python scripts/check_dependency_direction.py --verbose

Exit codes: 0 = the graph respects the layering, 1 = a violation was found,
2 = the workspace manifests could not be read.
"""

from __future__ import annotations

import argparse
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# Crate name -> layer. Assigning a new workspace member is a deliberate edit.
LAYERS: dict[str, int] = {
    "finkit": 0,
    "finkit-factor-analysis": 1,
    "finkit-visualization": 1,
    "finkit-ffi-common": 2,
    "finkit-cli": 3,
    "finkit-wasm": 3,
    "finkit-ffi": 3,
    "finkit-python": 3,
    "finkit-node": 3,
    "finkit-go": 3,
    "finkit-java": 3,
    "finkit-dotnet": 3,
    "finkit-ios": 3,
    "finkit-android": 3,
}

DEP_TABLES = ("dependencies", "build-dependencies")
# Dev-dependencies do not ship in a crate's public build, so a test-only edge
# is reported but not gate-fatal: `core`'s tests deliberately pull
# `finkit-ffi-common` to share golden reference vectors with the bindings.
DEV_TABLES = ("dev-dependencies",)


def load_manifest(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def workspace_members() -> list[Path]:
    root = load_manifest(ROOT / "Cargo.toml")
    members = root.get("workspace", {}).get("members")
    if not members:
        print("FAILED: Cargo.toml has no [workspace].members", file=sys.stderr)
        raise SystemExit(2)
    return [ROOT / m for m in members]


def member_name(manifest_dir: Path) -> str:
    manifest = load_manifest(manifest_dir / "Cargo.toml")
    name = manifest.get("package", {}).get("name")
    if not name:
        print(f"FAILED: {manifest_dir}/Cargo.toml has no [package].name", file=sys.stderr)
        raise SystemExit(2)
    return name


def internal_edges(manifest_dir: Path, by_name: dict[str, Path], tables=DEP_TABLES) -> set[str]:
    """Intra-workspace dependencies of one member, by crate name."""
    manifest = load_manifest(manifest_dir / "Cargo.toml")
    edges: set[str] = set()
    for table in tables:
        for dep_name, spec in manifest.get(table, {}).items():
            if dep_name in by_name:
                edges.add(dep_name)
                continue
            # A renamed dependency still points at a workspace member by path.
            if isinstance(spec, dict) and "path" in spec:
                target = (manifest_dir / spec["path"]).resolve()
                for name, member_dir in by_name.items():
                    if member_dir.resolve() == target:
                        edges.add(name)
    return edges


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--verbose", action="store_true", help="print every internal edge")
    args = ap.parse_args()

    members = workspace_members()
    by_name: dict[str, Path] = {member_name(d): d for d in members}

    # A member with no assigned layer is itself a failure: the gate must not
    # silently pass a crate it cannot reason about.
    unassigned = sorted(set(by_name) - set(LAYERS))
    if unassigned:
        print("Workspace members with no assigned layer:")
        for name in unassigned:
            print(f"  - {name}  ({by_name[name].relative_to(ROOT).as_posix()})")
        print(
            "Assign a layer in scripts/check_dependency_direction.py before merging. "
            "A new crate's position in the dependency graph is a design decision."
        )
        return 1

    violations: list[str] = []
    edge_count = 0
    dev_edges: list[tuple[str, int, str, int]] = []
    for name in sorted(by_name, key=lambda n: (LAYERS[n], n)):
        for dep in sorted(internal_edges(by_name[name], by_name)):
            edge_count += 1
            up, dp = LAYERS[name], LAYERS[dep]
            ok = dp < up
            marker = "ok  " if ok else "BAD "
            if args.verbose:
                print(f"  {marker} L{up} {name} -> L{dp} {dep}")
            if not ok:
                kind = "same-layer" if dp == up else "upward"
                violations.append(
                    f"{name} (L{up}) depends on {dep} (L{dp}) — {kind} dependency; "
                    f"an adapter/kernel cannot depend on a higher layer"
                )
        # Test-only edges: reported, not fatal (they never reach a consumer).
        for dep in sorted(internal_edges(by_name[name], by_name, DEV_TABLES)):
            dev_edges.append((name, LAYERS[name], dep, LAYERS[dep]))

    if violations:
        print(f"Layering violations ({len(violations)}):")
        for line in violations:
            print(f"  - {line}")
        return 1

    if dev_edges:
        print("Test-only (dev-dependency) edges, not gate-fatal:")
        for name, up, dep, dp in dev_edges:
            print(f"  - L{up} {name} --dev--> L{dp} {dep}")

    print(
        f"OK: {edge_count} shipped intra-workspace dependency edges across "
        f"{len(by_name)} crates all point strictly downward "
        f"(L0 core -> L1 research/render -> L2 ffi-common -> L3 adapters)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
