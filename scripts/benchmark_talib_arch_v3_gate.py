#!/usr/bin/env python3
"""Architecture v3 installed-wheel performance gate.

The legacy benchmark remains the data collector so parity cases stay in one
place. This gate converts its TA-Lib/Finkit latency ratios into the positive
Finkit speedup semantics specified by docs/finkit-outperform-talib-architecture-v3.md.
"""

from __future__ import annotations

import argparse
import json
import math
import statistics
from pathlib import Path
from types import SimpleNamespace

import benchmark_talib_release_gate as benchmark


def geomean(values: list[float]) -> float:
    if not values:
        return 0.0
    return math.exp(statistics.mean(math.log(value) for value in values))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sizes", type=int, nargs="+", default=[10_000, 100_000, 1_000_000])
    parser.add_argument("--min-geomean-speedup", type=float, default=1.15)
    parser.add_argument("--min-100k-speedup", type=float, default=1.15)
    parser.add_argument("--min-1m-speedup", type=float, default=1.20)
    parser.add_argument("--min-top20-speedup", type=float, default=1.05)
    parser.add_argument("--min-consistent-floor", type=float, default=0.95)
    args = parser.parse_args()

    summary = benchmark.run(
        SimpleNamespace(
            sizes=args.sizes,
            gate=False,
            max_indicator_geomean=math.inf,
            max_formula_geomean=math.inf,
        )
    )

    indicator_rows = [row for row in summary["rows"] if row["kind"] == "indicator"]
    for row in indicator_rows:
        row["finkit_speedup_x"] = row["talib_us"] / row["finkit_us"]

    all_speedups = [row["finkit_speedup_x"] for row in indicator_rows]
    speedup_100k = [
        row["finkit_speedup_x"] for row in indicator_rows if row["n"] == 100_000
    ]
    speedup_1m = [
        row["finkit_speedup_x"] for row in indicator_rows if row["n"] == 1_000_000
    ]

    names: list[str] = []
    for row in indicator_rows:
        if row["name"] not in names:
            names.append(row["name"])
    top20 = names[:20]
    top20_rows = [row for row in indicator_rows if row["name"] in top20]

    by_name: dict[str, list[float]] = {}
    for row in indicator_rows:
        by_name.setdefault(row["name"], []).append(row["finkit_speedup_x"])

    metrics = {
        "indicator_geomean_finkit_speedup_x": geomean(all_speedups),
        "indicator_100k_geomean_finkit_speedup_x": geomean(speedup_100k),
        "indicator_1m_geomean_finkit_speedup_x": geomean(speedup_1m),
        "top20_min_finkit_speedup_x": min(
            (row["finkit_speedup_x"] for row in top20_rows), default=0.0
        ),
        "consistently_below_floor": sorted(
            name
            for name, values in by_name.items()
            if values and all(value < args.min_consistent_floor for value in values)
        ),
        "thresholds": {
            "min_geomean_speedup": args.min_geomean_speedup,
            "min_100k_speedup": args.min_100k_speedup,
            "min_1m_speedup": args.min_1m_speedup,
            "min_top20_speedup": args.min_top20_speedup,
            "min_consistent_floor": args.min_consistent_floor,
        },
        "top20": top20,
    }

    out = Path("dist/bench")
    out.mkdir(parents=True, exist_ok=True)
    (out / "talib-architecture-v3-gate.json").write_text(
        json.dumps({"metrics": metrics, "summary": summary}, indent=2),
        encoding="utf-8",
    )

    failures: list[str] = []
    if summary["errors"]:
        failures.append(f"{len(summary['errors'])} API errors")
    if summary["parity_failures"]:
        failures.append(f"{len(summary['parity_failures'])} parity failures")
    if metrics["indicator_geomean_finkit_speedup_x"] < args.min_geomean_speedup:
        failures.append(
            "indicator geomean "
            f"{metrics['indicator_geomean_finkit_speedup_x']:.3f}x < "
            f"{args.min_geomean_speedup:.3f}x"
        )
    if speedup_100k and metrics["indicator_100k_geomean_finkit_speedup_x"] < args.min_100k_speedup:
        failures.append(
            "100K geomean "
            f"{metrics['indicator_100k_geomean_finkit_speedup_x']:.3f}x < "
            f"{args.min_100k_speedup:.3f}x"
        )
    if speedup_1m and metrics["indicator_1m_geomean_finkit_speedup_x"] < args.min_1m_speedup:
        failures.append(
            "1M geomean "
            f"{metrics['indicator_1m_geomean_finkit_speedup_x']:.3f}x < "
            f"{args.min_1m_speedup:.3f}x"
        )
    if top20_rows and metrics["top20_min_finkit_speedup_x"] < args.min_top20_speedup:
        failures.append(
            "top20 minimum "
            f"{metrics['top20_min_finkit_speedup_x']:.3f}x < "
            f"{args.min_top20_speedup:.3f}x"
        )
    if metrics["consistently_below_floor"]:
        failures.append(
            "consistently below floor: " + ", ".join(metrics["consistently_below_floor"])
        )

    print("ARCH_V3", json.dumps(metrics, ensure_ascii=False))
    if failures:
        raise SystemExit("architecture v3 release gate failed: " + "; ".join(failures))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
