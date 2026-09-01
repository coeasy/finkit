#!/usr/bin/env python3
"""Render and gate Finkit vs TA-Lib Criterion benchmark results."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

DEFAULT_CRITERION_DIR = Path("target/criterion")
DEFAULT_OUTPUT = Path("docs/BENCHMARK_REPORT.md")
DEFAULT_BASELINE = Path("docs/benchmark-baseline.json")


def load_point(path: Path) -> float | None:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
        return float(payload["mean"]["point_estimate"])
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError):
        return None


def collect_pairs(criterion_dir: Path) -> dict[str, dict[str, Any]]:
    pairs: dict[tuple[str, str, str], dict[str, Any]] = {}
    if not criterion_dir.is_dir():
        return {}

    for estimate in criterion_dir.glob("**/new/estimates.json"):
        try:
            parts = estimate.relative_to(criterion_dir).parts
        except ValueError:
            continue
        if len(parts) < 4 or parts[-2] != "new":
            continue
        group, bench_name = parts[0], parts[-3]
        scale = parts[-4] if len(parts) >= 5 else ""
        point_ns = load_point(estimate)
        if point_ns is None:
            continue
        if bench_name.startswith("FTA_"):
            role, key = "fta_us", bench_name[4:]
        elif bench_name.startswith("TALib_"):
            role, key = "talib_us", bench_name[6:]
        else:
            continue
        entry = pairs.setdefault((group, scale, key), {"category": group})
        entry[role] = point_ns / 1000.0

    result: dict[str, dict[str, Any]] = {}
    for (group, scale, key), entry in sorted(pairs.items()):
        if "fta_us" not in entry or "talib_us" not in entry:
            continue
        result_key = key if not group.startswith("scaled_") else f"{key}@{scale or group}"
        fta_us = float(entry["fta_us"])
        talib_us = float(entry["talib_us"])
        ratio = talib_us / fta_us if fta_us > 0 else 0.0
        status = "✅" if fta_us <= talib_us else ("⚠️" if fta_us <= talib_us * 1.25 else "❌")
        result[result_key] = {
            "category": group.removesuffix("_vs_talib"),
            "fta_us": fta_us,
            "talib_us": talib_us,
            "speedup": ratio,
            "status": status,
        }
    return result


def render_markdown(benchmarks: dict[str, dict[str, Any]]) -> str:
    lines = [
        "# Finkit vs TA-Lib C Benchmark Report",
        "",
        "> Auto-generated from Criterion JSON by scripts/bench_report.py.",
        "",
        "| Indicator | Category | Finkit (µs) | TA-Lib C (µs) | Speedup | Status |",
        "|---|---|---:|---:|---:|:---:|",
    ]
    for key, row in sorted(benchmarks.items()):
        lines.append(
            f"| {key} | {row['category']} | {row['fta_us']:.2f} | "
            f"{row['talib_us']:.2f} | {row['speedup']:.2f}x | {row['status']} |"
        )
    lines.extend(["", f"- **Total paired benchmarks**: {len(benchmarks)}", ""])
    return "\n".join(lines)


def load_baseline(path: Path) -> dict[str, dict[str, float]]:
    try:
        return json.loads(path.read_text(encoding="utf-8")).get("indicators", {})
    except (OSError, TypeError, json.JSONDecodeError):
        return {}


def gate_vs_talib(benchmarks: dict[str, dict[str, Any]], threshold: float) -> list[str]:
    return [
        f"{key}: {row['fta_us']:.2f}µs vs {row['talib_us']:.2f}µs (>{threshold:.1f}% slower)"
        for key, row in benchmarks.items()
        if row["fta_us"] > row["talib_us"] * (1.0 + threshold / 100.0)
    ]


def gate_vs_baseline(criterion_dir: Path, baseline_path: Path, scale: str, threshold: float) -> list[str]:
    current = collect_pairs(criterion_dir)
    baseline = load_baseline(baseline_path)
    failures = []
    expected = ("SMA_20", "EMA_12", "RSI_14", "MACD", "BBANDS_20", "ATR_14")
    for key in expected:
        row = current.get(f"{key}@{scale}")
        base = baseline.get(key, {}).get(scale)
        if row is None:
            failures.append(f"{key}@{scale}: no current Criterion data")
        elif base is None:
            failures.append(f"{key}@{scale}: no baseline value")
        elif row["fta_us"] > float(base) * (1.0 + threshold / 100.0):
            failures.append(f"{key}@{scale}: {row['fta_us']:.2f}µs vs baseline {float(base):.2f}µs")
    return failures


def run_gate(name: str, failures: list[str]) -> int:
    if failures:
        print(f"{name} FAILED:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1
    print(f"{name} PASSED")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--criterion-dir", default=str(DEFAULT_CRITERION_DIR))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    parser.add_argument("--json-out", default=None)
    parser.add_argument("--threshold", type=float, default=20.0)
    parser.add_argument("--baseline", default=str(DEFAULT_BASELINE))
    parser.add_argument("--gate", action="store_true")
    parser.add_argument("--regression-gate", action="store_true")
    parser.add_argument("--sla-1m", action="store_true")
    args = parser.parse_args()

    criterion_dir = Path(args.criterion_dir)
    benchmarks = collect_pairs(criterion_dir)
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(render_markdown(benchmarks), encoding="utf-8")
    if args.json_out:
        json_path = Path(args.json_out)
        json_path.parent.mkdir(parents=True, exist_ok=True)
        json_path.write_text(json.dumps({"name": "finkit-vs-talib", "benchmarks": benchmarks}, indent=2) + "\n", encoding="utf-8")
    print(f"Report written to {output}")
    if args.gate:
        return run_gate(f"TA-Lib comparison gate ({args.threshold:.1f}%)", gate_vs_talib(benchmarks, args.threshold))
    if args.regression_gate:
        return run_gate("Benchmark regression gate (5.0%)", gate_vs_baseline(criterion_dir, Path(args.baseline), "10K", 5.0))
    if args.sla_1m:
        return run_gate("1M SLA gate (10.0%)", gate_vs_baseline(criterion_dir, Path(args.baseline), "1M", 10.0))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
