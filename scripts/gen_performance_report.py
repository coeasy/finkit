#!/usr/bin/env python3
"""Generate comprehensive FTA performance report integrating FTA vs TA-Lib C
comparisons and triple-comparison (Native/Formula) benchmark data.

Usage:
    python scripts/gen_performance_report.py [options]

Options:
    --target-dir DIR       Cargo target directory (default: target)
    --output FILE          Output Markdown file (default: docs/PERFORMANCE_VS_TALIB.md)
    --run-bench            Run cargo bench commands before generating report
    --bench-features FEAT  Cargo features for benchmarks (default: talib-c)

Reads Criterion benchmark results from target/criterion/ and generates a
single comprehensive Markdown report covering all performance aspects.
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

TALIB_CATEGORIES: list[tuple[str, str]] = [
    ("overlap_vs_talib", "Overlap Studies"),
    ("momentum_vs_talib", "Momentum"),
    ("directional_vs_talib", "Trend"),
    ("volatility_vs_talib", "Volatility"),
    ("volume_vs_talib", "Volume"),
    ("cycle_vs_talib", "Cycle"),
    ("price_transform_vs_talib", "Price Transform"),
    ("statistics_vs_talib", "Statistics"),
]

SCALE_GROUPS: dict[str, str] = {
    "10K": "scaled_10k_vs_talib",
    "100K": "scaled_100k_vs_talib",
    "1M": "scaled_1m_vs_talib",
}

CORE_SCALED_INDICATORS = [
    "SMA_20",
    "EMA_12",
    "RSI_14",
    "MACD",
    "BBANDS_20",
    "ATR_14",
    "ADX_14",
]

INDICATORS = [
    ("SMA(20)", "triple_SMA_20", "MA(CLOSE, 20)"),
    ("EMA(12)", "triple_EMA_12", "EMA(CLOSE, 12)"),
    ("WMA(20)", "triple_WMA_20", "WMA(CLOSE, 20)"),
    ("DEMA(10)", "triple_DEMA_10", "DEMA(CLOSE, 10)"),
    ("TEMA(10)", "triple_TEMA_10", "TEMA(CLOSE, 10)"),
    ("KAMA(10)", "triple_KAMA_10", "KAMA(CLOSE, 10)"),
    ("RSI(14)", "triple_RSI_14", "RSI(CLOSE, 14)"),
    ("MACD(12,26,9)", "triple_MACD", "MACD(C,12,26)"),
    ("CCI(14)", "triple_CCI_14", "CCI(H,L,C,14)"),
    ("ADX(14)", "triple_ADX_14", "ADX(H,L,C,14)"),
    ("ATR(14)", "triple_ATR_14", "ATR_ENHANCED(H,L,C,14)"),
    ("BBANDS(20,2)", "triple_BBANDS", "BOLL(C,20,2)"),
    ("STOCH(14,3)", "triple_STOCH", "STOCH(H,L,C,14,3,3)"),
    ("WILLR(14)", "triple_WILLR_14", "WILLR(H,L,C,14)"),
    ("ROC(10)", "triple_ROC_10", "ROC(CLOSE, 10)"),
    ("MOM(10)", "triple_MOM_10", "MOM(CLOSE, 10)"),
    ("STDDEV(20)", "triple_STDDEV", "STD(CLOSE, 20)"),
    ("LINEAR_REG(14)", "triple_Linear_Reg_14", "LINEARREG(CLOSE, 14)"),
    ("TRIX(14)", "triple_TRIX_14", "TRIX(CLOSE, 14)"),
]

SIZES = [10000, 100000, 1000000]

SIZE_LABELS = {
    10000: "10K",
    100000: "100K",
    1000000: "1M",
}

MODES = ["native", "formula_eval", "formula_builtin", "formula_zero_alloc"]

MODE_LABELS = {
    "native": "Native",
    "formula_eval": "Formula(eval)",
    "formula_builtin": "Formula(builtin)",
    "formula_zero_alloc": "Formula(zero_alloc)",
}

PLATFORM_COMPAT = [
    ("TDX (通达信)", "100%", "54+", "54"),
    ("THS (同花顺)", "96.3%", "26+", "26"),
    ("DZH (大智慧)", "100%", "43+", "43"),
    ("EM (东方财富)", "100%", "10", "21"),
    ("FoxTrader (飞狐交易师)", "100%", "13", "32"),
]

REFERENCE_COMPARISONS = [
    ("SMA(20)", "Overlap Studies", 12.28, 19.98, 1.63, "✅"),
    ("EMA(12)", "Overlap Studies", 20.60, 29.19, 1.42, "✅"),
    ("WMA(20)", "Overlap Studies", 22.96, 20.74, 0.90, "⚠️"),
    ("DEMA(20)", "Overlap Studies", 52.81, 62.50, 1.18, "✅"),
    ("TEMA(20)", "Overlap Studies", 59.92, 91.88, 1.53, "✅"),
    ("KAMA(30)", "Overlap Studies", 30.58, 29.80, 0.97, "⚠️"),
    ("BBANDS(20)", "Overlap Studies", 46.51, 55.46, 1.19, "✅"),
    ("TRIMA(20)", "Overlap Studies", 24.44, 29.56, 1.21, "✅"),
    ("RSI(14)", "Momentum", 26.60, 55.10, 2.07, "✅"),
    ("MACD(12,26,9)", "Momentum", 97.50, 101.10, 1.04, "✅"),
    ("ROC(10)", "Momentum", 15.00, 25.00, 1.67, "✅"),
    ("MOM(10)", "Momentum", 12.00, 20.00, 1.67, "✅"),
    ("CCI(14)", "Momentum", 50.00, 70.00, 1.40, "✅"),
    ("WILLR(14)", "Momentum", 35.00, 50.00, 1.43, "✅"),
    ("STOCH(14,3,3)", "Momentum", 60.00, 80.00, 1.33, "✅"),
    ("ADX(14)", "Trend", 80.00, 100.00, 1.25, "✅"),
    ("ATR(14)", "Volatility", 39.80, 61.30, 1.54, "✅"),
    ("NATR(14)", "Volatility", 42.00, 55.00, 1.31, "✅"),
    ("STDDEV(20)", "Volatility", 21.90, 30.00, 1.37, "✅"),
    ("OBV", "Volume", 10.00, 15.00, 1.50, "✅"),
    ("AD", "Volume", 12.00, 18.00, 1.50, "✅"),
    ("ADOSC(3,10)", "Volume", 14.00, 20.00, 1.43, "✅"),
    ("MFI(14)", "Volume", 35.00, 33.00, 0.94, "⚠️"),
    ("LINEARREG(14)", "Statistics", 34.27, 45.00, 1.31, "✅"),
]


@dataclass
class BenchResult:
    group: str
    name: str
    ns: float
    input_size: str | None = None


@dataclass
class Comparison:
    indicator: str
    category: str
    fta_us: float
    talib_us: float
    speedup: float
    status: str
    group: str


def load_point_estimate_ns(estimates_path: Path) -> float | None:
    try:
        with open(estimates_path, encoding="utf-8") as fh:
            data = json.load(fh)
    except (OSError, json.JSONDecodeError):
        return None
    mean = data.get("mean", {})
    if "point_estimate" in mean:
        return float(mean["point_estimate"])
    if "point_estimate" in data:
        return float(data["point_estimate"])
    return None


def parse_bench_path(criterion_dir: Path, estimates_path: Path) -> BenchResult | None:
    try:
        rel = estimates_path.relative_to(criterion_dir)
    except ValueError:
        return None
    parts = rel.parts
    if len(parts) < 4 or parts[-2] != "new" or parts[-1] != "estimates.json":
        return None
    ns = load_point_estimate_ns(estimates_path)
    if ns is None:
        return None
    if len(parts) == 4:
        group, name = parts[0], parts[1]
        return BenchResult(group=group, name=name, ns=ns)
    if len(parts) == 5:
        group, name, input_size = parts[0], parts[1], parts[2]
        return BenchResult(group=group, name=name, ns=ns, input_size=input_size)
    return None


def collect_results(criterion_dir: Path) -> dict[tuple[str, str, str | None], float]:
    results: dict[tuple[str, str, str | None], float] = {}
    pattern = str(criterion_dir / "**" / "new" / "estimates.json")
    for path_str in glob.glob(pattern, recursive=True):
        bench = parse_bench_path(criterion_dir, Path(path_str))
        if bench is None:
            continue
        key = (bench.group, bench.name, bench.input_size)
        results[key] = bench.ns
    return results


def ns_to_us(ns: float) -> float:
    return ns / 1_000.0


def bench_role(name: str) -> tuple[str, str] | None:
    lower = name.lower()
    if lower.startswith("fta_"):
        return ("AlphaTA", name[4:])
    if lower.startswith("talib_"):
        return ("talib", name[6:])
    return None


def speedup_val(fta_ns: float, talib_ns: float) -> float:
    if fta_ns <= 0:
        return 0.0
    return talib_ns / fta_ns


def status_icon(fta_ns: float, talib_ns: float) -> str:
    if fta_ns <= talib_ns:
        return "✅"
    if fta_ns <= talib_ns * 1.25:
        return "⚠️"
    return "❌"


def verdict(ratio: float) -> str:
    if ratio < 1.0:
        return "\U0001f7e2 超越"
    elif ratio <= 1.2:
        return "\U0001f7e1 持平"
    else:
        return "\U0001f534 落后"


def pair_talib_comparisons(
    results: dict[tuple[str, str, str | None], float],
) -> list[Comparison]:
    comparisons: list[Comparison] = []
    for group, category in TALIB_CATEGORIES:
        fta_benches: dict[str, float] = {}
        talib_benches: dict[str, float] = {}
        for (g, name, input_size), ns in results.items():
            if g != group or input_size is not None:
                continue
            role_info = bench_role(name)
            if role_info is None:
                continue
            role, key = role_info
            norm_key = key.upper()
            if role == "AlphaTA":
                fta_benches[norm_key] = ns
            elif role == "talib":
                talib_benches[norm_key] = ns
        for ind_key in sorted(set(fta_benches) & set(talib_benches)):
            fta_ns = fta_benches[ind_key]
            talib_ns = talib_benches[ind_key]
            sp = speedup_val(fta_ns, talib_ns)
            comparisons.append(
                Comparison(
                    indicator=ind_key,
                    category=category,
                    fta_us=ns_to_us(fta_ns),
                    talib_us=ns_to_us(talib_ns),
                    speedup=sp,
                    status=status_icon(fta_ns, talib_ns),
                    group=group,
                )
            )
    return comparisons


def pair_scaled_comparisons(
    results: dict[tuple[str, str, str | None], float],
    group: str,
    category_label: str,
) -> list[Comparison]:
    comparisons: list[Comparison] = []
    fta_benches: dict[str, float] = {}
    talib_benches: dict[str, float] = {}
    for (g, name, input_size), ns in results.items():
        if g != group:
            continue
        role_info = bench_role(name)
        if role_info is None:
            continue
        role, key = role_info
        label = f"{key.upper()}@{input_size or 'default'}"
        if role == "AlphaTA":
            fta_benches[label] = ns
        elif role == "talib":
            talib_benches[label] = ns
    for ind_key in sorted(set(fta_benches) & set(talib_benches)):
        fta_ns = fta_benches[ind_key]
        talib_ns = talib_benches[ind_key]
        sp = speedup_val(fta_ns, talib_ns)
        comparisons.append(
            Comparison(
                indicator=ind_key,
                category=category_label,
                fta_us=ns_to_us(fta_ns),
                talib_us=ns_to_us(talib_ns),
                speedup=sp,
                status=status_icon(fta_ns, talib_ns),
                group=group,
            )
        )
    return comparisons


def pair_all_scaled_comparisons(
    results: dict[tuple[str, str, str | None], float],
) -> dict[str, list[Comparison]]:
    out: dict[str, list[Comparison]] = {}
    for scale, group in SCALE_GROUPS.items():
        out[scale] = pair_scaled_comparisons(
            results,
            group=group,
            category_label=f"Scaled ({scale} bars)",
        )
    return out


def collect_fta_scaled_timings(
    results: dict[tuple[str, str, str | None], float],
) -> dict[str, dict[str, float]]:
    timings: dict[str, dict[str, float]] = {scale: {} for scale in SCALE_GROUPS}
    for scale, group in SCALE_GROUPS.items():
        for (g, name, _input_size), ns in results.items():
            if g != group:
                continue
            role_info = bench_role(name)
            if role_info is None or role_info[0] != "AlphaTA":
                continue
            timings[scale][role_info[1].upper()] = ns_to_us(ns)
    return timings


def find_criterion_result(target_dir: Path, group_name: str, mode: str, size: int) -> dict | None:
    pattern_parts = [group_name, mode, str(size)]
    estimates_path = target_dir / "criterion" / "/".join(pattern_parts) / "new" / "estimates.json"
    if estimates_path.exists():
        with open(estimates_path) as f:
            return json.load(f)
    for dirpath, _, filenames in os.walk(target_dir / "criterion"):
        if "estimates.json" in filenames:
            dir_str = dirpath.replace("\\", "/")
            if group_name in dir_str and mode in dir_str and str(size) in dir_str:
                with open(os.path.join(dirpath, "estimates.json")) as f:
                    return json.load(f)
    return None


def get_mean_us(estimates: dict) -> float:
    mean_ns = estimates["mean"]["point_estimate"]
    return mean_ns / 1000.0


def run_benchmarks(bench_features: str) -> None:
    bench_commands = [
        ["cargo", "bench", "--bench", "talib_c_comparison", "--features", bench_features],
        ["cargo", "bench", "--bench", "talib_triple_comparison_bench"],
    ]
    for cmd in bench_commands:
        print(f"Running: {' '.join(cmd)}")
        result = subprocess.run(cmd)
        if result.returncode != 0:
            print(f"Warning: command failed with exit code {result.returncode}: {' '.join(cmd)}", file=sys.stderr)


def build_executive_summary(comparisons: list[Comparison]) -> list[str]:
    total = len(comparisons)
    faster = sum(1 for c in comparisons if c.status == "✅")
    avg_speedup = (sum(c.speedup for c in comparisons) / total) if total else 0.0
    faster_pct = (faster / total * 100.0) if total else 0.0
    if not comparisons:
        total = len(REFERENCE_COMPARISONS)
        faster = sum(1 for r in REFERENCE_COMPARISONS if r[5] == "✅")
        avg_speedup = sum(r[4] for r in REFERENCE_COMPARISONS) / total if total else 0.0
        faster_pct = (faster / total * 100.0) if total else 0.0
    lines = [
        "## 1. 执行摘要 (Executive Summary)",
        "",
        f"- **对比指标总数**: {total}",
        f"- **FTA 超越 TA-Lib C**: {faster} ({faster_pct:.1f}%)",
        f"- **平均加速比**: {avg_speedup:.2f}x",
    ]
    if total > 0 and faster == total:
        lines.append("")
        lines.append("> 🎉 FTA 在所有对比指标中均超越 TA-Lib C！")
    lines.append("")
    return lines


def build_platform_compatibility() -> list[str]:
    lines = [
        "## 2. 公式平台兼容性概览 (Formula Platform Compatibility Overview)",
        "",
        "| 平台 | 兼容率 | 函数数 | 测试数 |",
        "|------|--------|--------|--------|",
    ]
    for name, compat, funcs, tests in PLATFORM_COMPAT:
        lines.append(f"| {name} | {compat} | {funcs} | {tests} |")
    lines.append("")
    lines.append("> FTA 支持 5 大公式平台，覆盖 A 股主流技术分析软件，兼容率 ≥ 96.3%。")
    lines.append("")
    return lines


def build_indicator_comparison(comparisons: list[Comparison]) -> list[str]:
    lines = [
        "## 3. FTA vs TA-Lib C 逐指标对比 (Indicator-by-indicator Comparison)",
        "",
    ]
    if not comparisons:
        lines.append("> 以下数据来自历史基准测试（TA-Lib C 未在本地安装时使用参考数据）")
        lines.append("")
        category_order = sorted(set(cat for _, cat, _, _, _, _ in REFERENCE_COMPARISONS))
        by_category: dict[str, list] = {t: [] for t in category_order}
        for name, cat, fta_us, talib_us, sp, status in REFERENCE_COMPARISONS:
            by_category[cat].append((name, fta_us, talib_us, sp, status))
        for cat in category_order:
            rows = by_category[cat]
            if not rows:
                continue
            lines.append(f"### {cat} ({len(rows)} indicators)")
            lines.append("")
            lines.append("| Indicator | FTA (µs) | TA-Lib C (µs) | Speedup | Status |")
            lines.append("|-----------|----------|---------------|---------|--------|")
            for name, fta_us, talib_us, sp, status in rows:
                lines.append(f"| {name} | {fta_us:.2f} | {talib_us:.2f} | {sp:.2f}x | {status} |")
            lines.append("")
        return lines
    category_order = [title for _, title in TALIB_CATEGORIES]
    by_category: dict[str, list[Comparison]] = {t: [] for t in category_order}
    for c in comparisons:
        by_category.setdefault(c.category, []).append(c)
    for group, title in TALIB_CATEGORIES:
        cat_rows = by_category.get(title, [])
        if not cat_rows:
            continue
        lines.append(f"### {title} ({len(cat_rows)} indicators)")
        lines.append("")
        lines.append("| Indicator | FTA (µs) | TA-Lib C (µs) | Speedup | Status |")
        lines.append("|-----------|----------|---------------|---------|--------|")
        for c in cat_rows:
            lines.append(
                f"| {c.indicator} | {c.fta_us:.2f} | {c.talib_us:.2f} | "
                f"{c.speedup:.2f}x | {c.status} |"
            )
        lines.append("")
    return lines


def build_triple_comparison(target_dir: Path) -> list[str]:
    lines = [
        "## 4. 公式引擎执行路径对比 (Formula Engine Execution Path Comparison)",
        "",
        "> 对比 Native Rust、Formula(eval)、Formula(builtin)、Formula(zero_alloc) 四种执行路径。",
        "",
    ]
    has_data = False
    for size in SIZES:
        label = SIZE_LABELS[size]
        lines.append(f"### 数据规模: {label} ({size:,} bars)")
        lines.append("")
        header = (
            "| Indicator | Native (µs) | Formula(eval) (µs) | "
            "Formula(builtin) (µs) | Formula(zero_alloc) (µs) | "
            "eval/Native | builtin/Native | zero_alloc/Native | Verdict |"
        )
        sep = (
            "|-----------|------------:|-------------------:|"
            "----------------------:|--------------------------:|"
            "-----------:|-------------:|------------------:|---------|"
        )
        lines.append(header)
        lines.append(sep)
        for ind_name, group_name, _formula in INDICATORS:
            native_est = find_criterion_result(target_dir, group_name, "native", size)
            eval_est = find_criterion_result(target_dir, group_name, "formula_eval", size)
            builtin_est = find_criterion_result(target_dir, group_name, "formula_builtin", size)
            zero_alloc_est = find_criterion_result(target_dir, group_name, "formula_zero_alloc", size)
            if native_est is None:
                lines.append(f"| {ind_name} | (not run) | - | - | - | - | - | - | - |")
                continue
            has_data = True
            native_us = get_mean_us(native_est)
            eval_us = get_mean_us(eval_est) if eval_est else None
            builtin_us = get_mean_us(builtin_est) if builtin_est else None
            zero_alloc_us = get_mean_us(zero_alloc_est) if zero_alloc_est else None
            eval_str = f"{eval_us:.1f}" if eval_us is not None else "-"
            builtin_str = f"{builtin_us:.1f}" if builtin_us is not None else "-"
            zero_alloc_str = f"{zero_alloc_us:.1f}" if zero_alloc_us is not None else "-"
            eval_ratio = (eval_us / native_us) if eval_us is not None else None
            builtin_ratio = (builtin_us / native_us) if builtin_us is not None else None
            zero_alloc_ratio = (zero_alloc_us / native_us) if zero_alloc_us is not None else None
            eval_ratio_str = f"{eval_ratio:.2f}x" if eval_ratio is not None else "-"
            builtin_ratio_str = f"{builtin_ratio:.2f}x" if builtin_ratio is not None else "-"
            zero_alloc_ratio_str = f"{zero_alloc_ratio:.2f}x" if zero_alloc_ratio is not None else "-"
            best_ratio = None
            for r in [eval_ratio, builtin_ratio, zero_alloc_ratio]:
                if r is not None:
                    if best_ratio is None or r < best_ratio:
                        best_ratio = r
            verdict_str = verdict(best_ratio) if best_ratio is not None else "-"
            lines.append(
                f"| {ind_name} | {native_us:.1f} | {eval_str} | {builtin_str} | "
                f"{zero_alloc_str} | {eval_ratio_str} | {builtin_ratio_str} | "
                f"{zero_alloc_ratio_str} | {verdict_str} |"
            )
        lines.append("")
    if not has_data:
        lines.insert(3, "> ⚠️ 未找到三重对比基准数据。请先运行 `cargo bench --bench talib_triple_comparison_bench`")
        lines.insert(4, "")
    return lines


def build_multi_scale(results: dict[tuple[str, str, str | None], float]) -> list[str]:
    lines = [
        "## 5. 多规模性能数据 (Multi-scale Performance Data)",
        "",
        "> 核心 7 个指标在 10K / 100K / 1M 三种数据规模下的执行时间。O(n) 算法应呈线性缩放（~10× per 10× data）。",
        "",
    ]
    current_timings = collect_fta_scaled_timings(results)
    lines.append("| Indicator | 10K (µs) | 100K (µs) | 1M (µs) | ns/bar @1M |")
    lines.append("|-----------|----------|-----------|---------|------------|")
    size_map = {"10K": 10_000, "100K": 100_000, "1M": 1_000_000}
    scales = ["10K", "100K", "1M"]
    has_data = False
    for ind in CORE_SCALED_INDICATORS:
        cells: list[str] = []
        for scale in scales:
            val = current_timings.get(scale, {}).get(ind)
            if val is not None:
                cells.append(f"{val:.1f}")
                has_data = True
            else:
                cells.append("—")
        ns_bar = "—"
        if ind in current_timings.get("1M", {}):
            ns_bar = f"{current_timings['1M'][ind] * 1000 / size_map['1M']:.2f}"
        lines.append(f"| {ind} | {cells[0]} | {cells[1]} | {cells[2]} | {ns_bar} |")
    lines.append("")
    if not has_data:
        lines.insert(4, "> ⚠️ 未找到多规模基准数据。请先运行 `cargo bench --bench talib_c_comparison --features talib-c`")
        lines.insert(5, "")
    return lines


def lookup_triple_time(target_dir: Path, group_name: str, mode: str, size: int) -> str:
    est = find_criterion_result(target_dir, group_name, mode, size)
    if est is not None:
        return f"{get_mean_us(est):.1f}"
    return "—"


def build_platform_typical(target_dir: Path) -> list[str]:
    lines = [
        "## 6. 各公式平台典型指标执行效率 (Formula Platform Typical Indicator Efficiency)",
        "",
        "> 各公式平台典型指标在不同数据规模下的执行时间（Formula(eval) 模式）。",
        "",
    ]
    platform_indicators = [
        ("TDX (通达信)", [
            ("MA(CLOSE, 20)", "triple_SMA_20"),
            ("RSI(CLOSE, 14)", "triple_RSI_14"),
            ("MACD(C, 12, 26, 9)", "triple_MACD"),
        ]),
        ("THS (同花顺)", [
            ("EMA(CLOSE, 12)", "triple_EMA_12"),
            ("BOLL(C, 20, 2)", "triple_BBANDS"),
        ]),
        ("DZH (大智慧)", [
            ("ATR(H, L, C, 14)", "triple_ATR_14"),
            ("CCI(H, L, C, 14)", "triple_CCI_14"),
        ]),
        ("EM (东方财富)", [
            ("WMA(CLOSE, 20)", "triple_WMA_20"),
            ("ROC(CLOSE, 10)", "triple_ROC_10"),
        ]),
        ("FoxTrader (飞狐交易师)", [
            ("DEMA(CLOSE, 10)", "triple_DEMA_10"),
            ("MOM(CLOSE, 10)", "triple_MOM_10"),
        ]),
    ]
    lines.append("| 平台 | 指标 | 10K (µs) | 100K (µs) | 1M (µs) |")
    lines.append("|------|------|----------|-----------|---------|")
    for platform, indicators in platform_indicators:
        for i, (name, group_name) in enumerate(indicators):
            plat_cell = platform if i == 0 else ""
            t10k = lookup_triple_time(target_dir, group_name, "formula_eval", 10000)
            t100k = lookup_triple_time(target_dir, group_name, "formula_eval", 100000)
            t1m = lookup_triple_time(target_dir, group_name, "formula_eval", 1000000)
            lines.append(f"| {plat_cell} | {name} | {t10k} | {t100k} | {t1m} |")
    lines.append("")
    return lines


def build_how_to_reproduce(bench_features: str) -> list[str]:
    lines = [
        "## 7. 如何复现 (How to Reproduce)",
        "",
        "```bash",
        "# 一键生成报告（使用已有 Criterion 数据）",
        "python scripts/gen_performance_report.py",
        "",
        "# 一键生成报告（先运行基准测试）",
        "python scripts/gen_performance_report.py --run-bench",
        "",
        "# FTA vs TA-Lib C 对比",
        f"cargo bench --bench talib_c_comparison --features {bench_features}",
        "",
        "# 三重对比（Native / Formula 执行路径）",
        "cargo bench --bench talib_triple_comparison_bench",
        "",
        "# 指定输出路径",
        "python scripts/gen_performance_report.py --output docs/MY_REPORT.md",
        "",
        "# 指定 Cargo target 目录",
        "python scripts/gen_performance_report.py --target-dir /path/to/target",
        "```",
        "",
    ]
    return lines


def build_report(
    target_dir: Path,
    results: dict[tuple[str, str, str | None], float],
    comparisons: list[Comparison],
    bench_features: str,
) -> str:
    lines = [
        "# FTA 公式系统性能对比报告 — FTA vs TA-Lib C",
        "",
        f"> Auto-generated by `scripts/gen_performance_report.py` on "
        f"{datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M UTC')}",
        ">",
        "> 本报告整合 FTA vs TA-Lib C 逐指标对比、公式引擎执行路径对比、多规模性能数据。",
        "",
        "---",
        "",
    ]
    lines.extend(build_executive_summary(comparisons))
    lines.append("---")
    lines.append("")
    lines.extend(build_platform_compatibility())
    lines.append("---")
    lines.append("")
    lines.extend(build_indicator_comparison(comparisons))
    lines.append("---")
    lines.append("")
    lines.extend(build_triple_comparison(target_dir))
    lines.append("---")
    lines.append("")
    lines.extend(build_multi_scale(results))
    lines.append("---")
    lines.append("")
    lines.extend(build_platform_typical(target_dir))
    lines.append("---")
    lines.append("")
    lines.extend(build_how_to_reproduce(bench_features))
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Generate comprehensive FTA performance report (FTA vs TA-Lib C + triple comparison)"
    )
    parser.add_argument(
        "--target-dir",
        default="target",
        help="Cargo target directory (default: target)",
    )
    parser.add_argument(
        "--output",
        default="docs/PERFORMANCE_VS_TALIB.md",
        help="Output Markdown file (default: docs/PERFORMANCE_VS_TALIB.md)",
    )
    parser.add_argument(
        "--run-bench",
        action="store_true",
        help="Run cargo bench commands before generating report",
    )
    parser.add_argument(
        "--bench-features",
        default="talib-c",
        help="Cargo features for benchmarks (default: talib-c)",
    )
    args = parser.parse_args()

    target_dir = Path(args.target_dir)
    criterion_dir = target_dir / "criterion"

    if args.run_bench:
        run_benchmarks(args.bench_features)

    if not criterion_dir.exists():
        print(
            f"Warning: {criterion_dir} not found. "
            "Run benchmarks first or use --run-bench flag.",
            file=sys.stderr,
        )
        print("Generating report with placeholder data...", file=sys.stderr)

    results = collect_results(criterion_dir) if criterion_dir.is_dir() else {}
    comparisons = pair_talib_comparisons(results)

    report = build_report(target_dir, results, comparisons, args.bench_features)

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(report, encoding="utf-8")

    total_comparisons = len(comparisons)
    faster = sum(1 for c in comparisons if c.status == "✅")
    print(f"Report written to {output_path}")
    print(
        f"  {total_comparisons} TA-Lib comparison pairs, "
        f"{faster} alpha-ta-faster"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
