#!/usr/bin/env python3
"""Generate the authoritative FTA benchmark report from Criterion JSON output.

Usage:
    python scripts/gen_benchmark_report.py
    python scripts/gen_benchmark_report.py --dry-run
    python scripts/gen_benchmark_report.py --perf-gate --threshold 10

Reads Criterion results from target/criterion/ and writes docs/BENCHMARK_REPORT.md
with environment fingerprint, per-benchmark median/mean/stddev tables, competitive
comparison placeholders, and measurement methodology notes.
"""

from __future__ import annotations

import argparse
import glob
import json
import math
import platform
import subprocess
import sys
from dataclasses import dataclass
from datetime import date, datetime, timezone
from pathlib import Path

DEFAULT_CRITERION_DIR = Path("target/criterion")
DEFAULT_OUTPUT = Path("docs/BENCHMARK_REPORT.md")
DEFAULT_BASELINE = Path("docs/benchmark-baseline.json")

# Core indicators tracked in perf-gate (10K bars, FTA batch API via scaled group).
PERF_GATE_INDICATORS: list[tuple[str, str, str]] = [
    ("SMA(20)", "FTA_SMA_20", "SMA_20"),
    ("EMA(12)", "FTA_EMA_12", "EMA_12"),
    ("RSI(14)", "FTA_RSI_14", "RSI_14"),
    ("MACD(12,26,9)", "FTA_MACD", "MACD"),
    ("ATR(14)", "FTA_ATR_14", "ATR_14"),
]

EXTENDED_INDICATORS: list[tuple[str, str, str]] = [
    ("BBANDS(20,2)", "FTA_BBANDS_20", "BBANDS_20"),
    ("ADX(14)", "FTA_ADX_14", "ADX_14"),
    ("WMA(20)", "FTA_WMA_20", "WMA_20"),
    ("ROC(10)", "FTA_ROC_10", "ROC_10"),
    ("MOM(10)", "FTA_MOM_10", "MOM_10"),
    ("CCI(14)", "FTA_CCI_14", "CCI_14"),
    ("STOCH(14,3,3)", "FTA_STOCH_14_3_3", "STOCH_14_3_3"),
    ("OBV", "FTA_OBV", "OBV"),
]

COMPETITOR_SOURCES = [
    ("TA-Lib C", "https://github.com/ta-lib/ta-lib"),
    ("Kand", "https://github.com/rust-ta/kand"),
    ("quantedge-ta", "https://github.com/dluksza/quantedge-ta"),
    ("ta-rs", "https://github.com/greyblake/ta-rs"),
]


@dataclass
class BenchStats:
    group: str
    name: str
    input_size: str | None
    mean_ns: float | None
    median_ns: float | None
    stddev_ns: float | None


@dataclass
class EnvFingerprint:
    cpu: str
    isa: str
    rust_version: str
    os_info: str
    report_date: str
    commit_hash: str
    simd_detected: str  # runtime SIMD capability summary


def run_cmd(cmd: list[str], default: str = "unknown") -> str:
    try:
        out = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            check=False,
            timeout=15,
        )
        if out.returncode == 0 and out.stdout.strip():
            return out.stdout.strip()
    except (OSError, subprocess.SubprocessError):
        pass
    return default


def git_commit_hash() -> str:
    return run_cmd(["git", "rev-parse", "--short", "HEAD"], "unknown")


def rust_version() -> str:
    return run_cmd(["rustc", "--version"], "rustc unknown")


def detect_cpu_model() -> str:
    system = platform.system()
    if system == "Linux":
        try:
            with open("/proc/cpuinfo", encoding="utf-8", errors="replace") as fh:
                for line in fh:
                    if line.lower().startswith("model name"):
                        return line.split(":", 1)[1].strip()
        except OSError:
            pass
        return run_cmd(["lscpu"], platform.processor() or "unknown").split("\n")[0]
    if system == "Windows":
        raw = run_cmd(
            ["wmic", "cpu", "get", "Name", "/value"],
            platform.processor() or "unknown",
        )
        for line in raw.splitlines():
            line = line.strip()
            if line.lower().startswith("name="):
                return line.split("=", 1)[1].strip()
        return raw.replace("Name=", "").strip() or platform.processor() or "unknown"
    if system == "Darwin":
        return run_cmd(["sysctl", "-n", "machdep.cpu.brand_string"], platform.processor() or "unknown")
    return platform.processor() or platform.machine() or "unknown"


def detect_instruction_sets() -> str:
    flags: set[str] = set()
    system = platform.system()

    if system == "Linux":
        try:
            with open("/proc/cpuinfo", encoding="utf-8", errors="replace") as fh:
                for line in fh:
                    if line.lower().startswith("flags"):
                        parts = line.split(":", 1)[1].strip().split()
                        flags.update(parts)
                        break
        except OSError:
            pass
    elif system == "Windows":
        raw = run_cmd(["wmic", "cpu", "get", "Caption", "/value"], "")
        for token in raw.replace("Caption=", "").split():
            flags.add(token.lower())
        # Also check via environment variable set by some CI
        env_flags = run_cmd(
            ["powershell", "-Command",
             "Get-CimInstance Win32_Processor | Select-Object -ExpandProperty Caption"],
            "",
        )
        for token in env_flags.lower().split():
            flags.add(token)
    elif system == "Darwin":
        raw = run_cmd(["sysctl", "-n", "machdep.cpu.features"], "")
        for token in raw.lower().split():
            flags.add(token)
        raw2 = run_cmd(["sysctl", "-n", "machdep.cpu.leaf7_features"], "")
        for token in raw2.lower().split():
            flags.add(token)

    ordered = [
        "avx512f",
        "avx2",
        "avx",
        "fma",
        "bmi2",
        "sse4_2",
        "sse4_1",
        "sse2",
    ]
    present = [name.upper().replace("_", "") for name in ordered if name in flags]
    if present:
        return ", ".join(present)
    return "SSE2 (default x86-64) or host SIMD not detected"


def detect_rustc_simd_support() -> str:
    """Detect SIMD target features Rust would use via `rustc --print cfg`."""
    raw = run_cmd(["rustc", "--print", "cfg"], "")
    features: list[str] = []
    for line in raw.splitlines():
        line = line.strip()
        if line.startswith('target_feature="'):
            feat = line.split('"')[1]
            if feat in ("avx2", "avx", "sse4.1", "sse4.2", "fma", "avx512f"):
                features.append(feat.upper().replace(".", ""))
    if features:
        return ", ".join(sorted(set(features)))
    return "default (no explicit target features)"


def collect_env_fingerprint() -> EnvFingerprint:
    return EnvFingerprint(
        cpu=detect_cpu_model(),
        isa=detect_instruction_sets(),
        rust_version=rust_version(),
        os_info=f"{platform.system()} {platform.release()} ({platform.machine()})",
        report_date=date.today().isoformat(),
        commit_hash=git_commit_hash(),
        simd_detected=detect_rustc_simd_support(),
    )


def load_estimate_point(estimates: dict, key: str) -> float | None:
    section = estimates.get(key, {})
    if isinstance(section, dict):
        if "point_estimate" in section:
            return float(section["point_estimate"])
        if "estimate" in section:
            return float(section["estimate"])
    return None


def stddev_from_sample(sample_path: Path) -> float | None:
    try:
        with open(sample_path, encoding="utf-8") as fh:
            data = json.load(fh)
    except (OSError, json.JSONDecodeError):
        return None

    times = data.get("times", [])
    flat: list[float] = []
    for entry in times:
        if isinstance(entry, list):
            flat.extend(float(x) for x in entry)
        else:
            flat.append(float(entry))
    if len(flat) < 2:
        return None
    mean = sum(flat) / len(flat)
    variance = sum((x - mean) ** 2 for x in flat) / (len(flat) - 1)
    return math.sqrt(variance)


def parse_bench_path(criterion_dir: Path, estimates_path: Path) -> BenchStats | None:
    try:
        rel = estimates_path.relative_to(criterion_dir)
    except ValueError:
        return None

    parts = rel.parts
    if len(parts) < 4 or parts[-2] != "new" or parts[-1] != "estimates.json":
        return None

    try:
        with open(estimates_path, encoding="utf-8") as fh:
            estimates = json.load(fh)
    except (OSError, json.JSONDecodeError):
        return None

    sample_path = estimates_path.parent / "sample.json"
    stddev = stddev_from_sample(sample_path)

    if len(parts) == 4:
        group, name = parts[0], parts[1]
        input_size = None
    elif len(parts) == 5:
        group, name, input_size = parts[0], parts[1], parts[2]
    else:
        return None

    return BenchStats(
        group=group,
        name=name,
        input_size=input_size,
        mean_ns=load_estimate_point(estimates, "mean"),
        median_ns=load_estimate_point(estimates, "median"),
        stddev_ns=stddev,
    )


def collect_all_stats(criterion_dir: Path) -> dict[tuple[str, str, str | None], BenchStats]:
    results: dict[tuple[str, str, str | None], BenchStats] = {}
    if not criterion_dir.is_dir():
        return results

    pattern = str(criterion_dir / "**" / "new" / "estimates.json")
    for path_str in glob.glob(pattern, recursive=True):
        stats = parse_bench_path(criterion_dir, Path(path_str))
        if stats is None:
            continue
        key = (stats.group, stats.name, stats.input_size)
        results[key] = stats
    return results


def find_stats(
    results: dict[tuple[str, str, str | None], BenchStats],
    group: str,
    bench_name: str,
    input_size: str | None = "10000",
) -> BenchStats | None:
    key = (group, bench_name, input_size)
    if key in results:
        return results[key]

    for (g, name, size), stats in results.items():
        if g == group and name == bench_name and (input_size is None or size == input_size):
            return stats
    return None


def ns_to_us(ns: float) -> float:
    return ns / 1_000.0


def format_ns(value: float | None, unit: str = "µs") -> str:
    if value is None:
        return "—"
    if unit == "µs":
        return f"{ns_to_us(value):.2f}"
    return f"{value:.2f}"


def format_indicator_table(
    indicators: list[tuple[str, str, str]],
    results: dict[tuple[str, str, str | None], BenchStats],
    group: str = "scaled_10k_vs_talib",
    input_size: str = "10000",
    dry_run: bool = False,
) -> str:
    lines = [
        "| Indicator | Median (µs) | Mean (µs) | Stddev (µs) | ns/bar |",
        "|-----------|-------------|-----------|-------------|--------|",
    ]
    size_n = int(input_size)

    for display, bench_name, _baseline_key in indicators:
        if dry_run:
            lines.append(
                f"| {display} | — | — | — | — |"
            )
            continue

        stats = find_stats(results, group, bench_name, input_size)
        if stats is None:
            # Fallback: overlap_vs_talib group (no input size subdir).
            stats = find_stats(results, "overlap_vs_talib", bench_name, None)
            if stats is None and bench_name.startswith("FTA_"):
                alt = bench_name.replace("FTA_", "FTA_", 1)
                stats = find_stats(results, "overlap_vs_talib", alt, None)

        if stats is None:
            lines.append(f"| {display} | (not run) | — | — | — |")
            continue

        ns_bar = "—"
        if stats.mean_ns is not None and size_n > 0:
            ns_bar = f"{stats.mean_ns / size_n:.2f}"

        lines.append(
            f"| {display} | {format_ns(stats.median_ns)} | "
            f"{format_ns(stats.mean_ns)} | {format_ns(stats.stddev_ns)} | {ns_bar} |"
        )

    return "\n".join(lines)


def format_env_section(env: EnvFingerprint) -> str:
    return "\n".join(
        [
            "## Environment Fingerprint",
            "",
            "| Field | Value |",
            "|-------|-------|",
            f"| **CPU** | {env.cpu} |",
            f"| **Instruction sets** | {env.isa} |",
            f"| **SIMD (rustc)** | {env.simd_detected} |",
            f"| **Rust** | {env.rust_version} |",
            f"| **OS** | {env.os_info} |",
            f"| **Date** | {env.report_date} |",
            f"| **Commit** | `{env.commit_hash}` |",
            "",
        ]
    )


def format_competitor_section() -> str:
    lines = [
        "## Competitive Comparison",
        "",
        "> Placeholder — populate after running `cargo bench --bench competitive_bench` "
        "and `cargo bench --bench talib_c_comparison --features talib-c`.",
        "",
        "| Library | Source | Status |",
        "|---------|--------|--------|",
    ]
    for name, url in COMPETITOR_SOURCES:
        lines.append(f"| {name} | [{url}]({url}) | _pending_ |")
    lines.append("")
    return "\n".join(lines)


def format_methodology_section() -> str:
    return "\n".join(
        [
            "## Measurement Methodology",
            "",
            "1. **Harness**: [Criterion](https://github.com/bheisler/criterion.rs) "
            "with the workspace `bench` profile (`lto = fat`, `opt-level = 3`).",
            "2. **Data**: Synthetic OHLCV series (10,000 bars by default); scaled groups "
            "use 10K / 100K / 1M bar inputs.",
            "3. **Metrics**: Median and mean wall-clock time per iteration (µs); stddev "
            "computed from Criterion `sample.json` when present.",
            "4. **ns/bar**: Mean time divided by input length — linear-scaling sanity check.",
            "5. **CI gate**: `perf-gate.yml` runs five core indicators at 10K; fails when "
            "runtime exceeds baseline by more than the configured **threshold** (default 10%).",
            "6. **Reproduce locally**:",
            "",
            "   ```bash",
            "   cargo bench -p finkit --bench talib_c_comparison --features talib-c",
            "   python scripts/gen_benchmark_report.py",
            "   ```",
            "",
            "7. **Template without data**:",
            "",
            "   ```bash",
            "   python scripts/gen_benchmark_report.py --dry-run",
            "   ```",
            "",
        ]
    )


def build_report(
    env: EnvFingerprint,
    results: dict[tuple[str, str, str | None], BenchStats],
    criterion_dir: Path,
    dry_run: bool,
) -> str:
    generated_at = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    total_benches = len(results)

    lines = [
        "# FTA Benchmark Report",
        "",
        f"> **Authoritative report** — auto-generated by `scripts/gen_benchmark_report.py` "
        f"on {generated_at}.",
        f"> Source: `{criterion_dir}` | Benchmarks collected: {total_benches if not dry_run else 0}",
        "",
        "---",
        "",
        format_env_section(env),
        "## Core Indicator Performance (10K bars)",
        "",
        "> SMA, EMA, RSI, MACD, ATR — primary perf-gate indicators.",
        "",
        format_indicator_table(PERF_GATE_INDICATORS, results, dry_run=dry_run),
        "",
        "## Extended Indicators (10K bars)",
        "",
        format_indicator_table(EXTENDED_INDICATORS, results, dry_run=dry_run),
        "",
        format_competitor_section(),
        format_methodology_section(),
    ]
    return "\n".join(lines)


def load_baseline(path: Path) -> dict[str, dict[str, float]]:
    if not path.is_file():
        return {}
    with open(path, encoding="utf-8") as fh:
        data = json.load(fh)
    return data.get("indicators", {})


def check_perf_gate(
    results: dict[tuple[str, str, str | None], BenchStats],
    baseline_path: Path,
    threshold_pct: float,
    group: str = "scaled_10k_vs_talib",
    input_size: str = "10000",
) -> list[str]:
    baseline = load_baseline(baseline_path)
    failures: list[str] = []
    ratio_limit = 1.0 + threshold_pct / 100.0

    for display, bench_name, baseline_key in PERF_GATE_INDICATORS:
        stats = find_stats(results, group, bench_name, input_size)
        if stats is None or stats.mean_ns is None:
            failures.append(f"{display}: no Criterion data for {group}/{bench_name}")
            continue

        base_us = baseline.get(baseline_key, {}).get("10K")
        if base_us is None:
            failures.append(f"{display}: no 10K baseline for {baseline_key}")
            continue

        current_us = ns_to_us(stats.mean_ns)
        ratio = current_us / base_us
        if ratio > ratio_limit:
            pct = (ratio - 1.0) * 100.0
            failures.append(
                f"{display}: {current_us:.2f}µs vs baseline {base_us:.2f}µs "
                f"({pct:.1f}% regression, threshold {threshold_pct}%)"
            )

    return failures


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Generate authoritative FTA benchmark report from Criterion JSON"
    )
    parser.add_argument(
        "--criterion-dir",
        default=str(DEFAULT_CRITERION_DIR),
        help="Criterion output directory (default: target/criterion)",
    )
    parser.add_argument(
        "--output",
        default=str(DEFAULT_OUTPUT),
        help="Output Markdown path (default: docs/BENCHMARK_REPORT.md)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Generate template report without requiring benchmark data",
    )
    parser.add_argument(
        "--perf-gate",
        action="store_true",
        help="Exit 1 if core indicators regress beyond --threshold vs baseline",
    )
    parser.add_argument(
        "--threshold",
        type=float,
        default=10.0,
        help="Regression threshold in percent for --perf-gate (default: 10)",
    )
    parser.add_argument(
        "--baseline",
        default=str(DEFAULT_BASELINE),
        help="Baseline JSON for perf gate (default: docs/benchmark-baseline.json)",
    )
    args = parser.parse_args()

    criterion_dir = Path(args.criterion_dir)
    env = collect_env_fingerprint()

    if args.dry_run:
        results: dict[tuple[str, str, str | None], BenchStats] = {}
    else:
        if not criterion_dir.is_dir():
            print(
                f"Warning: {criterion_dir} not found. "
                "Run benchmarks first or use --dry-run.",
                file=sys.stderr,
            )
        results = collect_all_stats(criterion_dir)

    report = build_report(env, results, criterion_dir, dry_run=args.dry_run)
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(report, encoding="utf-8")
    print(f"Report written to {output_path}")

    if args.perf_gate:
        failures = check_perf_gate(
            results,
            Path(args.baseline),
            args.threshold,
        )
        if failures:
            print(
                f"\nPerformance gate FAILED (threshold {args.threshold}%):",
                file=sys.stderr,
            )
            for msg in failures:
                print(f"  ❌ {msg}", file=sys.stderr)
            return 1
        if not results:
            print(
                "Performance gate SKIPPED (no Criterion data)",
                file=sys.stderr,
            )
            return 0
        print(f"Performance gate PASSED (threshold {args.threshold}%)")
        return 0

    return 0


if __name__ == "__main__":
    sys.exit(main())
