"""Pure helpers for strict per-observation performance release gates."""

from __future__ import annotations

import math
from collections.abc import Iterable, Mapping
from typing import Any


def evaluate_performance_rows(
    rows: Iterable[Mapping[str, Any]],
    expected_names: Iterable[str],
    *,
    minimum_speedup: float = 1.0,
    expected_sizes: Iterable[int] = (),
) -> dict[str, Any]:
    """Validate row coverage and the TA-Lib/Finkit speed floor.

    Each observation is a distinct metric (normally function × input size).
    Aggregates are deliberately not accepted here, so a fast function cannot
    hide a slow one.
    """

    materialized = list(rows)
    expected = {name.upper() for name in expected_names}
    observed = {str(row.get("name", "")).upper() for row in materialized}
    missing = sorted(expected - observed)
    unexpected = sorted(observed - expected)
    sizes = {int(size) for size in expected_sizes}
    expected_points = {(name, size) for name in expected for size in sizes}
    observed_points = {
        (str(row.get("name", "")).upper(), int(row["n"]))
        for row in materialized
        if row.get("n") is not None
    }
    missing_points = sorted(expected_points - observed_points)
    invalid_rows: list[dict[str, Any]] = []
    slow_rows: list[dict[str, Any]] = []

    for row in materialized:
        name = str(row.get("name", ""))
        size = row.get("n")
        try:
            speedup = float(row["finkit_speedup_x"])
        except (KeyError, TypeError, ValueError):
            invalid_rows.append({"name": name, "n": size, "reason": "missing_or_non_numeric_speedup"})
            continue
        if not math.isfinite(speedup) or speedup <= 0:
            invalid_rows.append({"name": name, "n": size, "reason": "non_finite_or_non_positive_speedup"})
        elif speedup < minimum_speedup:
            slow_rows.append({"name": name, "n": size, "finkit_speedup_x": speedup})

    return {
        "expected_function_count": len(expected),
        "observed_function_count": len(observed),
        "missing_functions": missing,
        "missing_observations": [
            {"name": name, "n": size} for name, size in missing_points
        ],
        "unexpected_functions": unexpected,
        "invalid_speed_rows": invalid_rows,
        "below_speed_floor": slow_rows,
        "minimum_speedup_x": minimum_speedup,
        "coverage_pass": not missing and not unexpected and not missing_points,
        "speed_pass": not invalid_rows and not slow_rows,
    }
