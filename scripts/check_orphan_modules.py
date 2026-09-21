#!/usr/bin/env python3
"""Fail when a public module in the core crate has no callers.

Motivation (see docs/refactor-plan-2026-09-21.md §3-B): the core crate had
accumulated roughly 3300 lines of public modules that were declared in
``core/src/lib.rs`` but referenced by nothing -- not even by tests:

* ``runtime_engine`` (+ the ``runtime/`` directory it pulls in via ``#[path]``)
* ``factor_graph``
* ``factor_provider``

They compiled, so CI stayed green, and they rotted silently. This script turns
"a public module nobody calls" from a code-review discovery into a build-time
failure.

Exit codes
----------
0  every public module has at least one production reference, or every orphan
   is recorded in KNOWN_ORPHANS below.
1  a module is orphaned (or test-only) without being recorded, **or** a
   recorded module is no longer orphaned.

The second half is what stops the allowlist from rotting: an entry that stops
being needed fails the check, so the list can only shrink by a deliberate edit.
That is the same contract used by the differential allowlist in
``core/tests/formula_plan_differential.rs``.
"""

from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
CORE_LIB = ROOT / "core" / "src" / "lib.rs"

# Directories scanned for *production* references (anything that ships).
PROD_DIRS = [
    "core/src",
    "cli/src",
    "wasm/src",
    "factor-analysis/src",
    "visualization/src",
]
# `ffi/**/src` is expanded at runtime because the sub-crates are numerous.
PROD_DIRS += [str(p) for p in sorted((ROOT / "ffi").glob("*/src"))]

# Directories scanned only to distinguish "unused" from "used by tests only".
TEST_DIRS = ["core/tests", "cli/tests", "factor-analysis/tests"]

# Modules a reference may legitimately start from.
REF_PREFIX = r"(?:crate|finkit|super|self)"

# Modules declared in lib.rs that are re-export shims / feature-gated plumbing
# rather than units of functionality. They are referenced by name in lib.rs
# itself, so they would otherwise pass vacuously; listing them keeps the report
# honest about what is actually being checked.
EXEMPT = set()

# Features that ship enabled. A module gated behind anything *else* is
# opt-in for downstream consumers, so having no in-repo caller is expected and
# must not be reported as an orphan. Keep in sync with `[features] default` in
# core/Cargo.toml.
DEFAULT_FEATURES = {
    "std",
    "formula",
    "serde",
    "tracing",
    "metrics",
    "formula-jit",
    "formula-simd",
    "indicators-all",
}

# --------------------------------------------------------------------------
# The allowlist. Non-rotting by construction: an entry that is no longer
# orphaned fails the check (see `main`).
#
# Each entry is (module, expected_state) where expected_state is "orphan"
# (no production and no test reference) or "test-only" (referenced by tests
# only). Delete an entry when the module gains real callers.
# --------------------------------------------------------------------------
KNOWN = {
    # -- Internal plumbing with no product narrative: delete or wire it -------
    # `factor_graph` and `factor_provider` used to be listed here: both were
    # built but had no caller. They are now wired into `operation.rs` (the
    # production seam) and are ordinary "ok" modules, so the entries are gone.
    # Kept as a comment because the failure mode they guard against is easy to
    # reintroduce: build a module, declare it in lib.rs, never call it.
    # -- Domain modules outside the "compute runtime" product boundary --------
    # `backtest`, `backtest_evaluation`, `selectors` (stock selection) and
    # `sector` (sector rotation) were removed outright: Finkit is a compute
    # engine and does not do backtesting or stock selection.
    #
    # `multi_period_resonance` is *reachable public API*, not dead internals:
    # a downstream user can call it today despite there being no in-repo
    # caller. It stays listed until the product boundary decision covers it.
    "multi_period_resonance": "orphan",
}


def rust_files(directories: list[str]) -> list[pathlib.Path]:
    files: list[pathlib.Path] = []
    for d in directories:
        base = ROOT / d
        if not base.is_dir():
            continue
        files.extend(sorted(base.rglob("*.rs")))
    return files


def parse_declared_modules(lib_text: str) -> list[tuple[str, str | None, bool]]:
    """Return (module_name, path_attr_or_None, optional_feature) for lib.rs.

    A preceding `#[path = "..."]` attribute is captured so a module that pulls
    its body from elsewhere owns those files and does not count them as
    external callers.

    ``optional_feature`` is True when the declaration sits behind a
    ``#[cfg(feature = "...")]`` for a feature that is *not* on by default --
    such a module is opt-in for downstream consumers, so having no in-repo
    caller is expected.
    """
    out: list[tuple[str, str | None, bool]] = []
    lines = lib_text.splitlines()
    pending_path: str | None = None
    pending_optional = False
    for line in lines:
        stripped = line.strip()
        m_path = re.match(r'#\[path\s*=\s*"([^"]+)"\s*\]', stripped)
        if m_path:
            pending_path = m_path.group(1)
            continue
        m_cfg = re.search(r'feature\s*=\s*"([^"]+)"', stripped)
        if stripped.startswith("#[cfg(") and m_cfg:
            pending_optional = m_cfg.group(1) not in DEFAULT_FEATURES
            continue
        m_mod = re.match(r"pub mod (\w+);", stripped)
        if m_mod:
            out.append((m_mod.group(1), pending_path, pending_optional))
            pending_path = None
            pending_optional = False
            continue
        # A doc comment or any other statement resets pending attributes.
        if stripped and not stripped.startswith("///") and not stripped.startswith("//"):
            pending_path = None
            pending_optional = False
    return out


def own_files(name: str, path_attr: str | None) -> set[pathlib.Path]:
    """Files that belong to the module itself and must not count as callers."""
    core_src = ROOT / "core" / "src"
    owned: set[pathlib.Path] = set()
    if path_attr:
        target = core_src / path_attr
        if target.is_dir():
            owned.update(target.rglob("*.rs"))
        elif target.is_file():
            owned.add(target)
    plain = core_src / f"{name}.rs"
    if plain.is_file():
        owned.add(plain)
        # A module may pull its body from elsewhere *from inside its own file*
        # (`runtime_engine.rs` -> `runtime/*.rs`). Those files belong to this
        # module, not to a hypothetical sibling, so claim them too. Otherwise
        # the module's own self-tests look like external callers -- which is
        # exactly how runtime_engine hid behind a green CI for months.
        try:
            body = plain.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            body = ""
        for rel in re.findall(r'#\[path\s*=\s*"([^"]+)"\s*\]', body):
            target = core_src / rel
            if target.is_dir():
                owned.update(target.rglob("*.rs"))
            elif target.is_file():
                owned.add(target)
    mod_dir = core_src / name
    if mod_dir.is_dir():
        owned.update(mod_dir.rglob("*.rs"))
    return {p.resolve() for p in owned if p.exists()}


def count_references(module: str, files: list[pathlib.Path],
                     exclude: set[pathlib.Path]) -> int:
    """Count path references to `module` outside `exclude`.

    Requires a further path segment (`crate::foo::Bar`) or a `use ...::foo;`
    form so that a bare doc link such as `[`crate::factor_graph`]` does not
    count as a real caller.
    """
    pattern = re.compile(
        r"\b" + REF_PREFIX + r"::" + re.escape(module) + r"\s*::"
        r"|"
        r"\buse\s+" + REF_PREFIX + r"::" + re.escape(module) + r"\s*;"
    )
    total = 0
    for f in files:
        resolved = f.resolve()
        if resolved in exclude:
            continue
        try:
            text = f.read_text(encoding="utf-8", errors="ignore")
        except OSError:
            continue
        total += len(pattern.findall(text))
    return total


def main() -> int:
    if not CORE_LIB.is_file():
        print(f"[orphan-modules] cannot find {CORE_LIB}", file=sys.stderr)
        return 1

    declared = parse_declared_modules(CORE_LIB.read_text(encoding="utf-8"))
    prod_files = rust_files(PROD_DIRS)
    test_files = rust_files(TEST_DIRS)

    problems: list[str] = []
    stale: list[str] = []
    report: list[str] = []

    seen: set[str] = set()
    for name, path_attr, optional in declared:
        if name in EXEMPT or name in seen:
            continue
        seen.add(name)
        exclude = own_files(name, path_attr)
        prod = count_references(name, prod_files, exclude)
        test = count_references(name, test_files, exclude)

        if prod > 0:
            state = "ok"
        elif optional:
            # Opt-in feature with no in-repo caller: expected for a library.
            state = "optional"
        elif test > 0:
            state = "test-only"
        else:
            state = "orphan"

        report.append(f"  {name:<24} prod={prod:<5} test={test:<5} {state}")

        # An opt-in feature module with no in-repo caller is the normal shape
        # for a library crate, so it is informational only.
        if state == "optional":
            continue

        expected = KNOWN.get(name)
        if expected is None:
            if state != "ok":
                problems.append(
                    f"{name}: {state} but not recorded in KNOWN -- wire it to a "
                    f"caller or delete it, or record it deliberately"
                )
        elif expected != state:
            stale.append(
                f"{name}: recorded as '{expected}' but is now '{state}' -- "
                f"the allowlist entry is stale, delete it"
            )

    print("[orphan-modules] public module reference census")
    for line in report:
        print(line)

    if stale:
        print("\n[orphan-modules] STALE allowlist entries (must be removed):")
        for s in stale:
            print(f"  - {s}")
    if problems:
        print("\n[orphan-modules] UNRECORDED unused public modules:")
        for p in problems:
            print(f"  - {p}")

    if problems or stale:
        print("\n[orphan-modules] FAIL")
        return 1

    print(f"\n[orphan-modules] OK ({len(seen)} public modules checked)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
