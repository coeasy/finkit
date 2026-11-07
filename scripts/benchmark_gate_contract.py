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


def evaluate_aggregate_performance_rows(
    rows: Iterable[Mapping[str, Any]],
    expected_names: Iterable[str],
    *,
    minimum_geomean: float = 1.0,
    expected_sizes: Iterable[int] = (),
) -> dict[str, Any]:
    """Validate complete benchmark coverage and an aggregate geometric mean.

    Individual observations below 1x remain visible in ``below_speed_floor``;
    the broad all-current gate passes when the complete, valid set has a
    geometric mean above the configured floor.
    """

    materialized = list(rows)
    result = evaluate_performance_rows(
        materialized,
        expected_names,
        minimum_speedup=1.0,
        expected_sizes=expected_sizes,
    )
    valid_speedups = []
    for row in materialized:
        try:
            speedup = float(row["finkit_speedup_x"])
        except (KeyError, TypeError, ValueError):
            continue
        if math.isfinite(speedup) and speedup > 0:
            valid_speedups.append(speedup)
    geomean = (
        math.exp(math.fsum(math.log(value) for value in valid_speedups) / len(valid_speedups))
        if valid_speedups
        else 0.0
    )
    result["aggregate_geomean_finkit_speedup"] = geomean
    result["minimum_geomean_speedup_x"] = minimum_geomean
    result["aggregate_speed_pass"] = (
        not result["invalid_speed_rows"]
        and len(valid_speedups) == len(materialized)
        and geomean > minimum_geomean
    )
    return result
