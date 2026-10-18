#!/usr/bin/env python3
"""Installed-wheel parity and stable performance-regression gate.

The public API benchmark is noisy across hosted runners.  This gate runs three
independent rounds, compares their median against a checked-in baseline built
from three independent GitHub Actions observations, and allows a 10% regression
margin.  The historical absolute targets remain performance goals, not a
single-run release veto.
"""

from __future__ import annotations

import argparse
import json
import math
import statistics
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import benchmark_talib_release_gate as benchmark


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_BASELINE = ROOT / "docs/benchmark-baselines/architecture-v3-installed-wheel.json"
DEFAULT_REPETITIONS = 3


def geomean(values: list[float]) -> float:
    if not values or any(value <= 0 or not math.isfinite(value) for value in values):
        return 0.0
    return math.exp(statistics.mean(math.log(value) for value in values))


def summarize(summary: dict[str, Any]) -> dict[str, Any]:
    """Compute positive Finkit speedup metrics for one benchmark round."""

    indicator_rows = [row for row in summary["rows"] if row["kind"] == "indicator"]
    for row in indicator_rows:
        row["finkit_speedup_x"] = row["talib_us"] / row["finkit_us"]

    names: list[str] = []
    for row in indicator_rows:
        if row["name"] not in names:
            names.append(row["name"])
    top20 = names[:20]
    top20_rows = [row for row in indicator_rows if row["name"] in top20]
    by_name: dict[str, list[float]] = {}
    for row in indicator_rows:
        by_name.setdefault(row["name"], []).append(row["finkit_speedup_x"])

    return {
        "indicator_geomean_finkit_speedup_x": geomean(
            [row["finkit_speedup_x"] for row in indicator_rows]
        ),
        "indicator_100k_geomean_finkit_speedup_x": geomean(
            [row["finkit_speedup_x"] for row in indicator_rows if row["n"] == 100_000]
        ),
        "indicator_1m_geomean_finkit_speedup_x": geomean(
            [row["finkit_speedup_x"] for row in indicator_rows if row["n"] == 1_000_000]
        ),
        "top20_geomean_finkit_speedup_x": geomean(
            [row["finkit_speedup_x"] for row in top20_rows]
        ),
        "top20_min_finkit_speedup_x": min(
            (row["finkit_speedup_x"] for row in top20_rows), default=0.0
        ),
        "consistently_below_parity": sorted(
            name
            for name, values in by_name.items()
            if values and all(value < 1.0 for value in values)
        ),
        "top20": top20,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sizes", type=int, nargs="+", default=[10_000, 100_000, 1_000_000])
    parser.add_argument("--repetitions", type=int, default=DEFAULT_REPETITIONS)
    parser.add_argument("--baseline", type=Path, default=DEFAULT_BASELINE)
    args = parser.parse_args()
    if args.repetitions < 1:
        parser.error("--repetitions must be at least 1")

    baseline = json.loads(args.baseline.read_text(encoding="utf-8"))
    baseline_metrics = baseline["median_metrics"]
    tolerance = baseline["max_regression_fraction"]
    if not 0.0 <= tolerance < 1.0:
        raise SystemExit("baseline max_regression_fraction must be between 0 and 1")

    rounds: list[dict[str, Any]] = []
    round_metrics: list[dict[str, Any]] = []
    for index in range(args.repetitions):
        summary = benchmark.run(
            SimpleNamespace(
                sizes=args.sizes,
                gate=False,
                max_indicator_geomean=math.inf,
                max_formula_geomean=math.inf,
            )
        )
        metrics = summarize(summary)
        rounds.append({"summary": summary, "metrics": metrics})
        round_metrics.append(metrics)

    metric_names = (
        "indicator_geomean_finkit_speedup_x",
        "indicator_100k_geomean_finkit_speedup_x",
        "indicator_1m_geomean_finkit_speedup_x",
        "top20_geomean_finkit_speedup_x",
        "top20_min_finkit_speedup_x",
    )
    metrics = {
        name: statistics.median(round_[name] for round_ in round_metrics)
        for name in metric_names
    }
    metrics["consistently_below_parity"] = sorted(
        set.intersection(
            *(set(round_["consistently_below_parity"]) for round_ in round_metrics)
        )
    )
    metrics["top20"] = round_metrics[-1]["top20"]
    metrics["rounds"] = round_metrics
    metrics["baseline"] = {
        "path": args.baseline.as_posix(),
        "observations": baseline["observations"],
        "median_metrics": baseline_metrics,
        "max_regression_fraction": tolerance,
        "release_floors": {
            name: baseline_metrics[name] * (1.0 - tolerance)
            for name in (
                "indicator_geomean_finkit_speedup_x",
                "indicator_100k_geomean_finkit_speedup_x",
                "indicator_1m_geomean_finkit_speedup_x",
                "top20_geomean_finkit_speedup_x",
            )
        },
    }

    out = Path("dist/bench")
    out.mkdir(parents=True, exist_ok=True)
    (out / "talib-architecture-v3-gate.json").write_text(
        json.dumps({"metrics": metrics, "rounds": rounds}, indent=2),
        encoding="utf-8",
    )

    failures: list[str] = []
    for round_ in rounds:
        summary = round_["summary"]
        if summary["errors"]:
            failures.append(f"{len(summary['errors'])} API errors")
        if summary["parity_failures"]:
            failures.append(f"{len(summary['parity_failures'])} parity failures")

    for name, floor in metrics["baseline"]["release_floors"].items():
        measured = metrics[name]
        if measured < floor:
            failures.append(f"{name} {measured:.3f}x < calibrated floor {floor:.3f}x")

    print("ARCH_V3", json.dumps(metrics, ensure_ascii=False))
    if failures:
        message = "architecture v3 release gate failed: " + "; ".join(failures)
        print(f"::error title=TA-Lib architecture v3 gate::{message}")
        raise SystemExit(message)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
