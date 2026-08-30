#!/usr/bin/env python3
"""Generate TA-Lib compatibility matrix from golden test results.

Reads `target/talib_compat_report.json` produced by `core/tests/golden_talib_tests.rs`
(or scans golden JSON presence) and writes `docs/COMPAT_MATRIX.md`.

Usage:
    python scripts/gen_compat_matrix.py
    python scripts/gen_compat_matrix.py --dry-run
    python scripts/gen_compat_matrix.py --report target/talib_compat_report.json
"""

from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = ROOT / "docs" / "COMPAT_MATRIX.md"
DEFAULT_REPORT = ROOT / "target" / "talib_compat_report.json"
GOLDEN_DIR = ROOT / "tests" / "golden" / "talib"

# Synced with scripts/gen_talib_golden.py INDICATORS keys.
INDICATORS: dict[str, dict[str, Any]] = {
    "SMA": {"family": "overlap", "tolerance": 1e-10},
    "EMA": {"family": "overlap", "tolerance": 1e-8},
    "WMA": {"family": "overlap", "tolerance": 1e-10},
    "DEMA": {"family": "overlap", "tolerance": 1e-8},
    "TEMA": {"family": "overlap", "tolerance": 1e-8},
    "RSI": {"family": "momentum", "tolerance": 1e-8},
    "MACD": {"family": "momentum", "tolerance": 1e-8},
    "BBANDS": {"family": "overlap", "tolerance": 1e-8},
    "ATR": {"family": "volatility", "tolerance": 1e-8},
    "NATR": {"family": "volatility", "tolerance": 1e-8},
    "ADX": {"family": "momentum", "tolerance": 1e-8},
    "STOCH": {"family": "momentum", "tolerance": 1e-8},
    "CCI": {"family": "momentum", "tolerance": 1e-8},
    "WILLR": {"family": "momentum", "tolerance": 1e-8},
    "MOM": {"family": "momentum", "tolerance": 1e-8},
    "ROC": {"family": "momentum", "tolerance": 1e-8},
    "TRIX": {"family": "momentum", "tolerance": 1e-8},
    "OBV": {"family": "volume", "tolerance": 1e-8},
    "AD": {"family": "volume", "tolerance": 1e-8},
    "APO": {"family": "momentum", "tolerance": 1e-8},
    "CMO": {"family": "momentum", "tolerance": 1e-8},
    "AROON": {"family": "momentum", "tolerance": 1e-8},
}

HT_TOLERANCE = 1e-5
PATTERN_TOLERANCE = 0.0  # exact equality


def tolerance_for(name: str) -> float:
    if name.startswith("HT_"):
        return HT_TOLERANCE
    if name.startswith("CDL"):
        return PATTERN_TOLERANCE
    return float(INDICATORS.get(name, {}).get("tolerance", 1e-8))


def reproduce_cmd(name: str) -> str:
    return (
        f"cargo test -p alpha-ta-core --test golden_talib_tests "
        f"golden_talib_{name.lower()} -- --nocapture"
    )


def status_emoji(status: str) -> str:
    mapping = {
        "pass": "✅",
        "warn": "🟡",
        "fail": "🔴",
        "skip": "⏭",
        "pending": "⏳",
    }
    return mapping.get(status, "⏳")


def load_report(path: Path) -> dict[str, Any] | None:
    if not path.is_file():
        return None
    with path.open(encoding="utf-8") as fh:
        return json.load(fh)


def golden_exists(name: str) -> bool:
    return (GOLDEN_DIR / f"{name.lower()}.json").is_file()


def build_rows_from_report(report: dict[str, Any]) -> list[dict[str, Any]]:
    by_name: dict[str, dict[str, Any]] = {}
    for entry in report.get("indicators", []):
        by_name[entry.get("name", "")] = entry

    rows: list[dict[str, Any]] = []
    for name in INDICATORS:
        entry = by_name.get(name)
        if entry is None:
            status = "skip" if not golden_exists(name) else "pending"
            rows.append(
                {
                    "name": name,
                    "status": status,
                    "tolerance": tolerance_for(name),
                    "notes": "no report entry — run golden_talib_tests",
                    "reproduce_cmd": reproduce_cmd(name),
                    "pass_rate_pct": 0.0,
                }
            )
            continue

        notes = entry.get("notes", "")
        status = entry.get("status", "pending")
        if status == "warn":
            notes = f"{notes}; reproduce: {entry.get('reproduce_cmd', reproduce_cmd(name))}"

        rows.append(
            {
                "name": name,
                "status": status,
                "tolerance": entry.get("tolerance", tolerance_for(name)),
                "notes": notes,
                "reproduce_cmd": entry.get("reproduce_cmd", reproduce_cmd(name)),
                "pass_rate_pct": entry.get("pass_rate_pct", 0.0),
            }
        )
    return rows


def build_template_rows() -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for name, meta in INDICATORS.items():
        has_golden = golden_exists(name)
        status = "pending" if has_golden else "skip"
        notes = (
            "golden JSON present — run tests then regenerate"
            if has_golden
            else f"golden missing: tests/golden/talib/{name.lower()}.json"
        )
        rows.append(
            {
                "name": name,
                "status": status,
                "tolerance": tolerance_for(name),
                "notes": notes,
                "reproduce_cmd": reproduce_cmd(name),
                "pass_rate_pct": 0.0,
                "family": meta.get("family", ""),
            }
        )
    return rows


def format_tolerance(tol: float) -> str:
    if tol == 0.0:
        return "exact"
    if tol >= 1e-3:
        return f"{tol:g}"
    return f"{tol:.0e}"


def format_matrix_md(rows: list[dict[str, Any]], dry_run: bool, report_path: Path) -> str:
    generated = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    mode = "template (dry-run)" if dry_run else "from test report"

    lines = [
        "# TA-Lib Compatibility Matrix",
        "",
        f"> Auto-generated by `scripts/gen_compat_matrix.py` on {generated}.",
        f"> Mode: {mode}. Source report: `{report_path.relative_to(ROOT)}`.",
        "",
        "AlphaTA batch indicators vs TA-Lib C reference (`tests/golden/talib/*.json`).",
        "",
        "| 指标 | 状态 | 容差 | 备注 |",
        "|------|------|------|------|",
    ]

    for row in rows:
        name = row["name"]
        emoji = status_emoji(row["status"])
        tol = format_tolerance(float(row["tolerance"]))
        notes = row.get("notes", "")
        if row["status"] == "warn" and "reproduce:" not in notes:
            notes = f"{notes}; 复现: `{row.get('reproduce_cmd', '')}`"
        if row["status"] == "pending":
            notes = f"{notes}; 复现: `{row.get('reproduce_cmd', '')}`"
        lines.append(f"| {name} | {emoji} | {tol} | {notes} |")

    pass_n = sum(1 for r in rows if r["status"] == "pass")
    warn_n = sum(1 for r in rows if r["status"] == "warn")
    fail_n = sum(1 for r in rows if r["status"] == "fail")
    skip_n = sum(1 for r in rows if r["status"] == "skip")
    pending_n = sum(1 for r in rows if r["status"] == "pending")
    total = len(rows)
    concluded = pass_n + warn_n + fail_n
    pct = (concluded / total * 100.0) if total else 0.0

    lines.extend(
        [
            "",
            "## Summary",
            "",
            f"- Total indicators: {total}",
            f"- ✅ pass: {pass_n} | 🟡 warn: {warn_n} | 🔴 fail: {fail_n} | "
            f"⏭ skip: {skip_n} | ⏳ pending: {pending_n}",
            f"- Concluded (✅/🟡/🔴): {concluded}/{total} ({pct:.1f}%)",
            "",
            "## Tolerance policy",
            "",
            "| Family | Tolerance |",
            "|--------|-----------|",
            "| SMA / WMA | 1e-10 |",
            "| EMA / DEMA / TEMA | 1e-8 |",
            "| HT_* (Hilbert) | 1e-5 |",
            "| Pattern (CDL*, ±100/0) | exact |",
            "",
            "## Regenerate",
            "",
            "```bash",
            "python scripts/gen_talib_golden.py      # TA-Lib reference JSON",
            "cargo test -p alpha-ta-core --test golden_talib_tests",
            "python scripts/gen_compat_matrix.py",
            "```",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate docs/COMPAT_MATRIX.md")
    parser.add_argument(
        "--output",
        default=str(DEFAULT_OUTPUT),
        help="Output markdown path (default: docs/COMPAT_MATRIX.md)",
    )
    parser.add_argument(
        "--report",
        default=str(DEFAULT_REPORT),
        help="JSON report from golden_talib_tests (default: target/talib_compat_report.json)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Generate template matrix without requiring test report",
    )
    args = parser.parse_args()

    output_path = Path(args.output)
    report_path = Path(args.report)

    if args.dry_run:
        rows = build_template_rows()
    else:
        report = load_report(report_path)
        if report is None:
            print(
                f"[compat-matrix] report not found: {report_path}; "
                "falling back to template rows",
                file=sys.stderr,
            )
            rows = build_template_rows()
        else:
            rows = build_rows_from_report(report)

    md = format_matrix_md(rows, dry_run=args.dry_run, report_path=report_path)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(md, encoding="utf-8")
    print(f"[compat-matrix] wrote {output_path} ({len(rows)} indicators)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
