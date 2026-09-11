#!/usr/bin/env python3
"""Enforce one canonical implementation per quantitative semantic family.

The gate tracks semantic families instead of assuming every compatibility
facade must have the same function name as its canonical implementation.
Compatibility/binding facades are accepted only when explicitly registered and
verified to delegate to the reviewed canonical target without owning loops.
Same-named functions with genuinely different return shape/semantics must be
registered as distinct semantic owners and expose an unambiguous public alias.
"""

from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]

# Semantic family -> (canonical path, canonical function name).
CANONICAL = {
    "sharpe_ratio": ("core/src/risk.rs", "sharpe_ratio"),
    "sortino_ratio": ("core/src/risk.rs", "sortino_ratio"),
    "max_drawdown": ("core/src/risk.rs", "max_drawdown"),
    "mutual_information_continuous": (
        "core/src/math/information.rs",
        "mutual_information_continuous",
    ),
    "discretize": ("core/src/math/quantile.rs", "discretize"),
    "fractional_ranks": ("core/src/math/rank.rs", "fractional_ranks"),
    "pairwise_pearson": ("core/src/math/information.rs", "pairwise_pearson"),
    "simple_slope": ("core/src/math/regression.rs", "simple_slope"),
    "forward_return": ("core/src/returns.rs", "forward_return"),
}

# (path, same-named function) -> unambiguous semantic alias exported from
# indicators/mod.rs. These are real implementations with semantics distinct
# from the aggregate risk functions above.
DISTINCT_SEMANTIC_OWNERS = {
    ("core/src/indicators/volatility_ext.rs", "sortino_ratio"): "rolling_sortino_ratio",
    ("core/src/indicators/volatility_ext.rs", "max_drawdown"): "rolling_max_drawdown",
}

# (path, facade function) -> token proving the reviewed delegate target.
# Facades may adapt output/error/container types, but may not implement their
# own algorithmic traversal.
DELEGATING_FACADES = {
    ("core/src/features/labels.rs", "forward_return"): "core_forward_return",
    ("core/src/features/labels.rs", "forward_return_arithmetic"): "core_forward_return",
    ("core/src/features/rolling_stats.rs", "linear_regression_slope"): "simple_slope",
    ("core/src/features/importance.rs", "mutual_info_discrete"): "mutual_information_discrete",
    ("core/src/features/importance.rs", "mutual_info_continuous"): "mutual_information_continuous",
    ("wasm/src/lib.rs", "sortino_ratio"): "indicators::sortino_ratio",
    ("wasm/src/lib.rs", "max_drawdown"): "indicators::max_drawdown",
}

SKIP_PARTS = {"target", ".git"}
FUNCTION_PATTERN = (
    r"(?m)^\s*(?:#\[[^\n]+\]\s*)*"
    r"(?:pub(?:\([^)]*\))?\s+)?fn\s+{name}\s*\("
)
violations: list[str] = []


def function_body(text: str, name: str) -> str | None:
    pattern = re.compile(FUNCTION_PATTERN.format(name=re.escape(name)))
    match = pattern.search(text)
    if not match:
        return None
    start = text.find("{", match.end())
    if start < 0:
        return None
    depth = 0
    for index in range(start, len(text)):
        char = text[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return text[start + 1 : index]
    return None


def has_function(text: str, name: str) -> bool:
    return re.search(FUNCTION_PATTERN.format(name=re.escape(name)), text) is not None


def is_verified_delegate(relative: str, name: str, text: str) -> bool:
    required = DELEGATING_FACADES.get((relative, name))
    if required is None:
        return False
    body = function_body(text, name)
    if body is None or required not in body:
        return False
    # Container conversion and error adaptation are permitted. Owning an
    # explicit traversal is not: that would create a second algorithm body.
    forbidden = ("for ", "while ", "loop {")
    return not any(token in body for token in forbidden)


rust_sources: dict[str, str] = {}
for path in ROOT.rglob("*.rs"):
    if any(part in SKIP_PARTS for part in path.parts):
        continue
    relative = path.relative_to(ROOT).as_posix()
    rust_sources[relative] = path.read_text(encoding="utf-8")

# Verify each canonical symbol exists exactly where the semantic family says it
# belongs, then reject unregistered same-named owners elsewhere.
for family, (owner_path, owner_name) in CANONICAL.items():
    owner_text = rust_sources.get(owner_path)
    if owner_text is None or not has_function(owner_text, owner_name):
        violations.append(
            f"missing canonical owner for {family}: expected {owner_path}::{owner_name}"
        )
        continue

    for relative, text in rust_sources.items():
        if relative == owner_path or not has_function(text, owner_name):
            continue
        distinct = (relative, owner_name)
        if distinct in DISTINCT_SEMANTIC_OWNERS:
            continue
        if is_verified_delegate(relative, owner_name, text):
            continue
        violations.append(
            f"{relative}: defines {owner_name} outside canonical semantic family {family} "
            f"owned by {owner_path}::{owner_name}"
        )

# Verify all differently named compatibility facades independently, so renaming
# a facade cannot silently bypass the ownership gate.
for (relative, name), target in DELEGATING_FACADES.items():
    text = rust_sources.get(relative)
    if text is None:
        violations.append(f"missing registered facade file {relative} for {name}")
        continue
    body = function_body(text, name)
    if body is None:
        violations.append(f"missing registered facade {relative}::{name}")
        continue
    if not is_verified_delegate(relative, name, text):
        violations.append(
            f"{relative}::{name} must remain a loop-free delegate containing target token {target}"
        )

indicator_exports = (ROOT / "core/src/indicators/mod.rs").read_text(encoding="utf-8")
for owner, alias in DISTINCT_SEMANTIC_OWNERS.items():
    relative, name = owner
    text = rust_sources.get(relative)
    if text is None or not has_function(text, name):
        violations.append(f"missing distinct semantic owner {relative}::{name}")
        continue
    expected = f"pub use volatility_ext::{name} as {alias};"
    if expected not in indicator_exports:
        violations.append(
            f"{relative}::{name} must expose unambiguous public alias {alias} in indicators/mod.rs"
        )

if violations:
    print("Research SSOT check failed:")
    for violation in violations:
        print(f"  - {violation}")
    print(
        "Move the algorithm to its canonical owner, register a genuinely distinct semantic owner, "
        "or register only a small verified delegating facade."
    )
    sys.exit(1)

print(
    f"Research SSOT check passed for {len(CANONICAL)} semantic families, "
    f"{len(DISTINCT_SEMANTIC_OWNERS)} distinct semantic owners, and "
    f"{len(DELEGATING_FACADES)} reviewed facades."
)
