#!/usr/bin/env python3
"""Enforce one canonical implementation per quantitative semantic family.

Compatibility and binding facades are allowed only when explicitly registered
below and their body is a small delegate to the reviewed canonical target.
Same-named functions with genuinely different return shape/semantics must be
registered as distinct semantic owners and expose an unambiguous public alias.
"""

from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]

# Function name -> paths allowed to own that algorithm implementation.
OWNERS = {
    "sharpe_ratio": {"core/src/risk.rs"},
    "sortino_ratio": {"core/src/risk.rs"},
    "max_drawdown": {"core/src/risk.rs"},
    "compute_mi": {"core/src/math/information.rs"},
    "discretize": {"core/src/math/quantile.rs"},
    "fractional_ranks": {"core/src/math/rank.rs"},
    "pearson_correlation": {"core/src/math/statistics.rs"},
    "linear_regression_slope": {"core/src/math/regression.rs"},
    "forward_return_arithmetic": {"core/src/returns.rs"},
}

# (path, same-named function) -> unambiguous semantic alias that must be
# exported from indicators/mod.rs. These are real implementations, but of a
# different semantic family from the aggregate risk functions above.
DISTINCT_SEMANTIC_OWNERS = {
    ("core/src/indicators/volatility_ext.rs", "sortino_ratio"): "rolling_sortino_ratio",
    ("core/src/indicators/volatility_ext.rs", "max_drawdown"): "rolling_max_drawdown",
}

# (path, public/helper name) -> token that must occur in the function body.
# These are reviewed compatibility/FFI adapters, never implementation owners.
DELEGATING_FACADES = {
    ("core/src/features/labels.rs", "forward_return_arithmetic"): "core_forward_return",
    ("core/src/features/rolling_stats.rs", "linear_regression_slope"): "simple_slope",
    ("wasm/src/lib.rs", "sortino_ratio"): "indicators::sortino_ratio",
    ("wasm/src/lib.rs", "max_drawdown"): "indicators::max_drawdown",
}

SKIP_PARTS = {"target", ".git"}
violations: list[str] = []


def function_body(text: str, name: str) -> str | None:
    match = re.search(
        rf"(?m)^\s*(?:#\[[^\n]+\]\s*)*(?:pub(?:\([^)]*\))?\s+)?fn\s+{re.escape(name)}\s*\(",
        text,
    )
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


def is_verified_delegate(relative: str, name: str, text: str) -> bool:
    required = DELEGATING_FACADES.get((relative, name))
    if required is None:
        return False
    body = function_body(text, name)
    if body is None or required not in body:
        return False
    # A facade may adapt result/error types, but it must not contain an
    # algorithmic loop or allocate algorithm scratch state.
    forbidden = ("for ", "while ", "loop {", "Vec::with_capacity", "vec![")
    return not any(token in body for token in forbidden)


seen_owner: dict[str, bool] = {name: False for name in OWNERS}
seen_distinct: set[tuple[str, str]] = set()
for path in ROOT.rglob("*.rs"):
    if any(part in SKIP_PARTS for part in path.parts):
        continue
    relative = path.relative_to(ROOT).as_posix()
    text = path.read_text(encoding="utf-8")
    for name, owners in OWNERS.items():
        pattern = re.compile(
            rf"(?m)^\s*(?:#\[[^\n]+\]\s*)*(?:pub(?:\([^)]*\))?\s+)?fn\s+{re.escape(name)}\s*\("
        )
        if not pattern.search(text):
            continue
        if relative in owners:
            seen_owner[name] = True
            continue
        if (relative, name) in DISTINCT_SEMANTIC_OWNERS:
            seen_distinct.add((relative, name))
            continue
        if is_verified_delegate(relative, name, text):
            continue
        violations.append(
            f"{relative}: defines {name} outside canonical owner(s) {sorted(owners)}"
        )

for name, owners in OWNERS.items():
    if not seen_owner[name]:
        violations.append(
            f"missing canonical owner definition for {name}: expected one of {sorted(owners)}"
        )

indicator_exports = (ROOT / "core/src/indicators/mod.rs").read_text(encoding="utf-8")
for owner, alias in DISTINCT_SEMANTIC_OWNERS.items():
    relative, name = owner
    if owner not in seen_distinct:
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
    f"Research SSOT check passed for {len(OWNERS)} canonical algorithm families, "
    f"{len(DISTINCT_SEMANTIC_OWNERS)} distinct semantic owners, and "
    f"{len(DELEGATING_FACADES)} reviewed facades."
)
