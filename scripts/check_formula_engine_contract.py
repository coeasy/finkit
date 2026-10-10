#!/usr/bin/env python3
"""Keep the formula engine's public contract and its docs in lock-step.

Why this exists
---------------
`docs/architecture/formula-engine.md` once described a **different engine** from
the one in the tree: it said "production execution goes through the compiled-plan
path" while `FormulaExecutionMode::default()` was — and is — `Tree`, and it
described the cache as one LRU while the engine actually holds several bounded
caches (one LRU AST cache plus FIFO plan/semantic/bytecode caches). Nothing
objected, because prose has no compiler.

The same drift already happened once to the *entry-point matrix*: the
`FormulaEngine` doc comment, the runtime contract and the test that is supposed
to check the split disagreed on how many tree-only entries exist and which ones
they are.

This gate is the compiler for those claims. It reads one source of truth —
`require_tree_backend("NAME")` call sites in `core/src/formula/engine.rs` — and
asserts three separate documents agree with it:

  1. the table in `docs/architecture/formula-engine.md`;
  2. the `assert_tree_only!("NAME", ..)` roster in
     `core/tests/formula_execution_mode.rs`;
  3. the enum default (`#[default]` must sit on `Tree`).

It also asserts the capacities written in the doc still match the constants the
code defines, so a cache-capacity change cannot land with stale prose.

Usage
-----
    python scripts/check_formula_engine_contract.py

Exit codes: 0 = consistent, 1 = drift, 2 = a source file could not be read.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ENGINE = ROOT / "core" / "src" / "formula" / "engine.rs"
TEST = ROOT / "core" / "tests" / "formula_execution_mode.rs"
DOC = ROOT / "docs" / "architecture" / "formula-engine.md"

# Phrases that described the pre-2026-09 engine and must not come back.
STALE_PHRASES = (
    "Production execution goes through the compiled-plan path",
    "LRU eviction bounds the working set",
)

REQUIRE_TREE = re.compile(r'require_tree_backend\("([^"]+)"\)')
ASSERT_TREE = re.compile(r'assert_tree_only!\(\s*"([^"]+)"')
ENUM_DEFAULT_TREE = re.compile(r"#\[default\]\s*\n\s*Tree,")
CACHE_CAPACITY = re.compile(r"const ENGINE_CACHE_CAPACITY: usize = (\d+);")
AST_CACHE_CAPACITY = re.compile(r"FormulaCache::new\((\d+)\)")


def read(path: Path) -> str:
    if not path.is_file():
        print(f"FAILED: missing {path.relative_to(ROOT)}", file=sys.stderr)
        raise SystemExit(2)
    return path.read_text(encoding="utf-8", errors="replace")


def doc_tree_only_entries(text: str) -> set[str]:
    """Backticked `eval*` names from the doc's 'Governed, tree-only' row."""
    for line in text.splitlines():
        if "Governed, tree-only" in line:
            return {
                token
                for token in re.findall(r"`([^`]+)`", line)
                if token.startswith("eval")
            }
    return set()


def report(label: str, code: set[str], other: set[str]) -> list[str]:
    problems = []
    if code - other:
        problems.append(f"{label} is missing: {sorted(code - other)}")
    if other - code:
        problems.append(f"{label} lists entries the code does not guard: {sorted(other - code)}")
    return problems


def main() -> int:
    engine = read(ENGINE)
    test = read(TEST)
    doc = read(DOC)

    code_entries = set(REQUIRE_TREE.findall(engine))
    if not code_entries:
        print("FAILED: no require_tree_backend(\"..\") sites found — did the guard move?")
        return 2

    failures: list[str] = []

    # 1. The enum default must still be Tree.
    if not ENUM_DEFAULT_TREE.search(engine):
        failures.append(
            "FormulaExecutionMode's #[default] is no longer on `Tree`; "
            "flipping the default is a release decision that must update this gate "
            "and docs/architecture/formula-engine.md together"
        )

    # 2. Doc + test must agree with the guarded entry set.
    failures += report(
        "docs/architecture/formula-engine.md",
        code_entries,
        doc_tree_only_entries(doc),
    )
    failures += report(
        "core/tests/formula_execution_mode.rs",
        code_entries,
        set(ASSERT_TREE.findall(test)),
    )

    # 3. No stale prose.
    for phrase in STALE_PHRASES:
        if phrase in doc:
            failures.append(f"docs/architecture/formula-engine.md still contains stale text: {phrase!r}")

    # 4. Cache capacities in the doc track the constants.
    for match in CACHE_CAPACITY.findall(engine):
        if f"= {match}" not in doc and match not in doc:
            failures.append(f"doc does not mention ENGINE_CACHE_CAPACITY value {match}")
    for match in AST_CACHE_CAPACITY.findall(engine):
        if match not in doc:
            failures.append(f"doc does not mention the AST cache capacity {match}")

    if failures:
        print(f"Formula-engine contract drift ({len(failures)}):")
        for line in failures:
            print(f"  - {line}")
        return 1

    print(
        f"OK: {len(code_entries)} tree-only entries agree across engine guard, "
        f"docs/architecture/formula-engine.md and formula_execution_mode.rs; "
        f"default is Tree; cache capacities match the constants"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
