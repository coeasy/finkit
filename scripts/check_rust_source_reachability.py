#!/usr/bin/env python3
"""Rust source reachability gate (V4 plan §3.2 / P1-02).

`check_orphan_modules.py` can only see modules that are *declared* in a
crate's `lib.rs`; a tracked `.rs` file that no module declaration references
at all is invisible to it. That is exactly how `core/src/circuit_breaker.rs`
rotted: a complete, documented, tested implementation that was never part of
the module graph and therefore never compiled into any product.

This gate walks the module graph of every workspace crate starting from its
root targets (`lib.rs`, `main.rs`, `src/bin/*.rs`) and requires every tracked
`.rs` file under `src/` (excluding the `tests/`, `benches/` and `examples/`
target subtrees) to be reachable, or to be explicitly recorded in
`scripts/rust_source_reachability_allowlist.json` with a reason.

The allowlist is non-rotting: an entry whose file has become reachable (or
that no longer exists) is itself a failure, so entries can only be removed by
deliberate edit.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ALLOWLIST_PATH = Path(__file__).resolve().parent / "rust_source_reachability_allowlist.json"

# Module declarations: `mod x;`, `pub mod x;`, `pub(crate) mod x;`, and the
# `#[path = "..."]` attribute form. `mod x {}` inline blocks introduce no file
# unless they carry a `#[path]` (handled by MOD_WITH_PATH). The combined
# pattern first captures every `#[path]`-attributed declaration (so the
# attribute can be separated from `mod` only by other attributes), then plain
# declarations not already consumed by an attributed match.
MOD_WITH_PATH = re.compile(
    r'#\s*\[\s*path\s*=\s*"([^"]+)"\s*\]\s*(?:#\[[^\]]*\]\s*)*'
    r"(?:pub(?:\([^)]*\))?\s+|unsafe\s+)*mod\s+(\w+)\s*[;{]"
)
MOD_DECL = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+|unsafe\s+)*mod\s+(\w+)\s*[;{]", re.MULTILINE
)
EXCLUDED_SUBTREES = ("tests", "benches", "examples")


def tracked_rs_files() -> list[Path]:
    out = subprocess.run(
        ["git", "ls-files", "-z", "*.rs"],
        cwd=ROOT,
        check=True,
        capture_output=True,
    ).stdout
    return [ROOT / Path(p.decode("utf-8")) for p in out.split(b"\0") if p]


INCLUDE_FILE = re.compile(r'(?m)^\s*include!\(\s*"([^"]+)"\s*\)\s*;')


def crate_roots(crate_dir: Path) -> list[Path]:
    """Root targets of a crate: lib.rs / main.rs / src/bin/*.rs, plus a
    custom `[lib] path = "..."` override from Cargo.toml."""

    cargo_toml = crate_dir / "Cargo.toml"
    src = crate_dir / "src"
    roots = []
    if cargo_toml.exists():
        text = cargo_toml.read_text(encoding="utf-8", errors="replace")
        # Stop at the next `[section]` header at line start; array values such
        # as `crate-type = ["cdylib"]` contain brackets mid-line.
        lib_section = re.search(r"^\[lib\]((?:(?!^\[).)*)", text, re.DOTALL | re.MULTILINE)
        if lib_section:
            path_override = re.search(r'path\s*=\s*"([^"]+)"', lib_section.group(1))
            if path_override:
                roots.append(crate_dir / path_override.group(1))
    for name in ("lib.rs", "main.rs"):
        candidate = src / name
        if candidate.exists():
            roots.append(candidate)
    bin_dir = src / "bin"
    if bin_dir.is_dir():
        roots.extend(sorted(bin_dir.glob("*.rs")))
    return roots


def child_module_candidates(declaring_file: Path, module_name: str) -> list[Path]:
    """Resolve `mod <name>;` declared in `declaring_file` to candidate files.

    Child modules of `lib.rs`, `main.rs` and `mod.rs` live beside the
    declaring file; child modules of a named module file (`foo.rs`) live in
    the sibling `foo/` directory (2018 edition rules).
    """

    parent = declaring_file.parent
    bases = [parent]
    if declaring_file.name not in ("lib.rs", "main.rs", "mod.rs"):
        bases.append(parent / declaring_file.stem)
    candidates = []
    for base in bases:
        candidates.append(base / f"{module_name}.rs")
        candidates.append(base / module_name / "mod.rs")
    return candidates


def reachable_files(roots: list[Path]) -> set[Path]:
    seen: set[Path] = set()
    queue = [root.resolve() for root in roots if root.exists()]
    while queue:
        current = queue.pop()
        if current in seen or not current.exists():
            continue
        seen.add(current)
        text = current.read_text(encoding="utf-8", errors="replace")

        # 1. `#[path = "..."] mod x;` — resolve relative to the declaring
        #    module's directory (mod.rs / lib.rs / main.rs) or its stem dir.
        for match in MOD_WITH_PATH.finditer(text):
            attr_value, _name = match.group(1), match.group(2)
            base = current.parent
            if current.name not in ("lib.rs", "main.rs", "mod.rs"):
                base = base / current.stem
            queue.append((base / attr_value).resolve())

        # 2. Plain `mod x;` declarations. A `#[path]`-attributed declaration
        #    also matches here; the conventional candidates it produces do not
        #    exist in that case and are skipped, while the `#[path]` target
        #    was already queued above — so enqueuing both is always safe.
        for match in MOD_DECL.finditer(text):
            queue.extend(
                p.resolve() for p in child_module_candidates(current, match.group(1))
            )

        # 3. `include!("file.rs");` — textual inclusion is part of the module
        #    graph (the FFI generated surfaces are wired in this way).
        for match in INCLUDE_FILE.finditer(text):
            queue.append((current.parent / match.group(1)).resolve())
    return seen


def in_excluded_subtree(path: Path, crate_src: Path) -> bool:
    try:
        rel = path.relative_to(crate_src)
    except ValueError:
        return False
    return any(part in EXCLUDED_SUBTREES for part in rel.parts[:-1])


def discover_crate_srcs() -> list[Path]:
    srcs = []
    for cargo_toml in tracked_rs_files() and ROOT.rglob("Cargo.toml"):
        rel = cargo_toml.relative_to(ROOT)
        if any(part in ("target", "node_modules") for part in rel.parts):
            continue
        src = cargo_toml.parent / "src"
        if src.is_dir():
            srcs.append(src)
    return srcs


def main() -> int:
    srcs = discover_crate_srcs()
    roots: list[Path] = []
    for src in srcs:
        roots.extend(crate_roots(src.parent))
    if not roots:
        print("no crate roots found; nothing to check")
        return 0

    reachable = reachable_files(roots)
    checked = {
        path.resolve()
        for path in tracked_rs_files()
        if path.suffix == ".rs"
        and any(path.resolve().is_relative_to(src) for src in srcs)
        and not any(in_excluded_subtree(path.resolve(), src) for src in srcs)
    }
    orphans = sorted(checked - reachable)

    allowlist: list[dict[str, str]] = []
    if ALLOWLIST_PATH.exists():
        allowlist = json.loads(ALLOWLIST_PATH.read_text(encoding="utf-8"))
    allowlisted = {entry["path"]: entry for entry in allowlist}

    failures = []
    for orphan in orphans:
        rel = orphan.relative_to(ROOT).as_posix()
        if rel in allowlisted:
            continue
        failures.append(
            f"orphan source (never enters the module graph): {rel}\n"
            "  declare it, delete it, or add it to "
            f"{ALLOWLIST_PATH.name} with a reason"
        )

    stale = []
    for rel, entry in allowlisted.items():
        resolved = (ROOT / rel).resolve()
        if not resolved.exists() or resolved in reachable:
            stale.append(
                f"stale allowlist entry: {rel} "
                f"({'file is gone' if not resolved.exists() else 'file is now module-reachable'}); "
                "remove the entry"
            )

    for problem in failures + stale:
        print(f"::error {problem}")
    if failures or stale:
        print(f"reachability gate: {len(failures)} orphan(s), {len(stale)} stale allowlist entr(ies)")
        return 1
    print(f"reachability gate: {len(checked)} tracked source files, all module-reachable")
    return 0


if __name__ == "__main__":
    sys.exit(main())
