#!/usr/bin/env python3
"""Fail when canonical quantitative helpers are reimplemented outside their owner.

The check intentionally targets algorithm function definitions, not compatibility
facades. New aliases may delegate to the owner, but must not introduce another
implementation with one of these canonical helper names.
"""

from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]

# Function name -> paths allowed to define that algorithm name.
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

SKIP_PARTS = {"target", ".git"}
violations: list[str] = []

for path in ROOT.rglob("*.rs"):
    if any(part in SKIP_PARTS for part in path.parts):
        continue
    relative = path.relative_to(ROOT).as_posix()
    text = path.read_text(encoding="utf-8")
    for name, owners in OWNERS.items():
        pattern = re.compile(rf"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+{re.escape(name)}\s*\(")
        if pattern.search(text) and relative not in owners:
            violations.append(
                f"{relative}: defines {name} outside canonical owner(s) {sorted(owners)}"
            )

if violations:
    print("Research SSOT check failed:")
    for violation in violations:
        print(f"  - {violation}")
    print("Delegate to the canonical kernel or explicitly update the owner map after review.")
    sys.exit(1)

print(f"Research SSOT check passed for {len(OWNERS)} canonical algorithm families.")
