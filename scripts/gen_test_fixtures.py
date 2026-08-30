#!/usr/bin/env python3
"""Generate shared test fixture datasets for the FTA workspace.

Produces four reproducible CSV datasets under tests/fixtures/ (by default):
  - A-share daily OHLCV (250 trading days, virtual Shanghai index)
  - Crypto minute OHLCV (BTC/USDT simulation, 1000 bars)
  - Synthetic OHLCV (sine / triangle / step waves + noise)
  - Random OHLCV (fixed-seed normal distribution for property tests)

Uses only the Python standard library. Fixed seed (42) ensures reproducibility.

Usage:
    python scripts/gen_test_fixtures.py
    python scripts/gen_test_fixtures.py --dry-run
    python scripts/gen_test_fixtures.py --output-dir tests/fixtures/
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import random
from datetime import date, datetime, timedelta
from pathlib import Path
from typing import Any, Callable, Sequence

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT_DIR = ROOT / "tests" / "fixtures"

FIXTURE_VERSION = "1.0"
GENERATOR_VERSION = "1.0.0"
SEED = 42

DATASETS: list[dict[str, Any]] = [
    {
        "id": "ashare",
        "filename": "ashare_sh_index_250d.csv",
        "type": "ashare",
        "description": "Virtual Shanghai Composite daily OHLCV, 250 trading days",
        "rows": 250,
        "columns": ["date", "open", "high", "low", "close", "volume"],
    },
    {
        "id": "crypto",
        "filename": "crypto_btc_usdt_1m_1000.csv",
        "type": "crypto",
        "description": "Simulated BTC/USDT 1-minute OHLCV, 1000 bars",
        "rows": 1000,
        "columns": ["timestamp", "open", "high", "low", "close", "volume"],
    },
    {
        "id": "synthetic",
        "filename": "synthetic_waves_500.csv",
        "type": "synthetic",
        "description": "Sine / triangle / step waves with noise for boundary testing",
        "rows": 500,
        "columns": ["index", "open", "high", "low", "close", "volume"],
    },
    {
        "id": "random",
        "filename": "random_normal_1000.csv",
        "type": "random",
        "description": "Fixed-seed normal-distribution OHLCV for property-based tests",
        "rows": 1000,
        "columns": ["index", "open", "high", "low", "close", "volume"],
    },
]


def triangle_wave(i: int, period: int, amplitude: float) -> float:
    """Triangle wave centered at zero with given period and amplitude."""
    if period <= 0:
        return 0.0
    phase = (i % period) / period
    return amplitude * (4.0 * abs(phase - 0.5) - 1.0)


def step_value(i: int, period: int, levels: Sequence[float]) -> float:
    """Piecewise-constant step function over repeating blocks."""
    if not levels:
        return 0.0
    block = (i // max(period, 1)) % len(levels)
    return levels[block]


def trading_days(count: int, start: date) -> list[date]:
    """Generate `count` weekday dates starting from `start`."""
    days: list[date] = []
    current = start
    while len(days) < count:
        if current.weekday() < 5:
            days.append(current)
        current += timedelta(days=1)
    return days


def generate_ashare(rng: random.Random) -> list[list[str]]:
    """Virtual Shanghai Composite-style daily OHLCV."""
    days = trading_days(250, date(2023, 1, 3))
    base_price = 3089.26
    rows: list[list[str]] = []
    close = base_price

    for i, d in enumerate(days):
        drift = 0.15 * math.sin(i * 0.08) + 0.05 * rng.gauss(0, 1)
        close = max(100.0, close + drift)
        spread = 8.0 + 4.0 * abs(math.sin(i * 0.11))
        open_p = close + rng.gauss(0, spread * 0.3)
        high_p = max(open_p, close) + spread * (0.3 + rng.random() * 0.7)
        low_p = min(open_p, close) - spread * (0.3 + rng.random() * 0.7)
        volume = int(1_500_000_000 + 500_000_000 * abs(math.sin(i * 0.05)) + rng.gauss(0, 80_000_000))
        volume = max(100_000_000, volume)
        rows.append(
            [
                d.isoformat(),
                f"{open_p:.4f}",
                f"{high_p:.4f}",
                f"{low_p:.4f}",
                f"{close:.4f}",
                str(volume),
            ]
        )
    return rows


def generate_crypto(rng: random.Random) -> list[list[str]]:
    """Simulated BTC/USDT 1-minute OHLCV."""
    start = datetime(2024, 6, 1, 0, 0, 0)
    price = 67500.0
    rows: list[list[str]] = []

    for i in range(1000):
        ts = start + timedelta(minutes=i)
        micro_move = rng.gauss(0, 12.5) + 8.0 * math.sin(i * 0.07)
        price = max(1000.0, price + micro_move)
        wick = 15.0 + 25.0 * rng.random()
        open_p = price + rng.gauss(0, 5.0)
        close_p = price + rng.gauss(0, 5.0)
        high_p = max(open_p, close_p) + wick * rng.random()
        low_p = min(open_p, close_p) - wick * rng.random()
        volume = 0.5 + abs(rng.gauss(2.0, 1.5)) + 0.3 * abs(math.sin(i * 0.03))
        rows.append(
            [
                ts.strftime("%Y-%m-%d %H:%M:%S"),
                f"{open_p:.2f}",
                f"{high_p:.2f}",
                f"{low_p:.2f}",
                f"{close_p:.2f}",
                f"{volume:.6f}",
            ]
        )
        price = close_p
    return rows


def generate_synthetic(rng: random.Random) -> list[list[str]]:
    """Sine, triangle, and step waves combined with noise."""
    rows: list[list[str]] = []
    sine_period = 50
    tri_period = 37
    step_period = 25
    step_levels = [100.0, 130.0, 115.0, 145.0, 90.0]

    for i in range(500):
        sine = 20.0 * math.sin(2.0 * math.pi * i / sine_period)
        tri = triangle_wave(i, tri_period, 15.0)
        step = step_value(i, step_period, step_levels)
        noise = rng.gauss(0, 2.5)
        base = step + sine + tri + noise

        spread = 3.0 + 2.0 * abs(math.sin(i * 0.2))
        open_p = base + rng.gauss(0, spread * 0.2)
        close_p = base + rng.gauss(0, spread * 0.2)
        high_p = max(open_p, close_p) + spread * (0.2 + 0.6 * rng.random())
        low_p = min(open_p, close_p) - spread * (0.2 + 0.6 * rng.random())
        volume = 1000.0 + 500.0 * abs(math.sin(i * 0.1)) + abs(rng.gauss(0, 50.0))

        rows.append(
            [
                str(i),
                f"{open_p:.6f}",
                f"{high_p:.6f}",
                f"{low_p:.6f}",
                f"{close_p:.6f}",
                f"{volume:.2f}",
            ]
        )
    return rows


def generate_random(rng: random.Random) -> list[list[str]]:
    """Normal-distribution OHLCV for property-based testing."""
    rows: list[list[str]] = []
    mean = 100.0
    sigma = 5.0

    for i in range(1000):
        open_p = rng.gauss(mean, sigma)
        close_p = rng.gauss(mean, sigma)
        high_p = max(open_p, close_p) + abs(rng.gauss(0, sigma * 0.3))
        low_p = min(open_p, close_p) - abs(rng.gauss(0, sigma * 0.3))
        volume = max(0.0, rng.gauss(10_000.0, 2_500.0))
        rows.append(
            [
                str(i),
                f"{open_p:.8f}",
                f"{high_p:.8f}",
                f"{low_p:.8f}",
                f"{close_p:.8f}",
                f"{volume:.4f}",
            ]
        )
    return rows


GENERATORS: dict[str, Callable[[random.Random], list[list[str]]]] = {
    "ashare": generate_ashare,
    "crypto": generate_crypto,
    "synthetic": generate_synthetic,
    "random": generate_random,
}


def metadata_header(generation_date: str) -> list[str]:
    return [
        f"# version={FIXTURE_VERSION}",
        f"# generation_date={generation_date}",
        f"# seed={SEED}",
        f"# generator_version={GENERATOR_VERSION}",
    ]


def write_csv(
    path: Path,
    columns: Sequence[str],
    rows: Sequence[Sequence[str]],
    generation_date: str,
) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as fh:
        for line in metadata_header(generation_date):
            fh.write(line + "\n")
        writer = csv.writer(fh)
        writer.writerow(columns)
        writer.writerows(rows)


def build_metadata_json(
    output_dir: Path,
    generation_date: str,
    datasets: list[dict[str, Any]],
) -> dict[str, Any]:
    return {
        "version": FIXTURE_VERSION,
        "generation_date": generation_date,
        "seed": SEED,
        "generator_version": GENERATOR_VERSION,
        "output_dir": str(output_dir.relative_to(ROOT)).replace("\\", "/"),
        "datasets": [
            {
                **meta,
                "path": str(
                    (output_dir / meta["filename"]).relative_to(ROOT)
                ).replace("\\", "/"),
            }
            for meta in datasets
        ],
    }


def write_metadata(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def print_plan(output_dir: Path, generation_date: str) -> None:
    print(f"Fixture generator v{GENERATOR_VERSION} (dry-run)")
    print(f"  seed: {SEED}")
    print(f"  generation_date: {generation_date}")
    print(f"  output_dir: {output_dir}")
    print()
    for meta in DATASETS:
        path = output_dir / meta["filename"]
        print(f"  [{meta['id']}] {path}")
        print(f"    type: {meta['type']}")
        print(f"    rows: {meta['rows']}")
        print(f"    columns: {', '.join(meta['columns'])}")
        print(f"    description: {meta['description']}")
    metadata_path = output_dir / "metadata.json"
    print()
    print(f"  [metadata] {metadata_path}")
    print("No files written (dry-run).")


def generate_all(output_dir: Path, generation_date: str) -> dict[str, Any]:
    rng = random.Random(SEED)
    written: list[dict[str, Any]] = []

    for meta in DATASETS:
        gen = GENERATORS[meta["id"]]
        rows = gen(rng)
        path = output_dir / meta["filename"]
        write_csv(path, meta["columns"], rows, generation_date)
        written.append({**meta, "actual_rows": len(rows)})
        print(f"Wrote {path} ({len(rows)} rows)")

    payload = build_metadata_json(output_dir, generation_date, DATASETS)
    metadata_path = output_dir / "metadata.json"
    write_metadata(metadata_path, payload)
    print(f"Wrote {metadata_path}")
    return payload


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate shared test fixture datasets for FTA."
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print generation plan without writing files.",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=DEFAULT_OUTPUT_DIR,
        help="Output directory (default: tests/fixtures/)",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    output_dir = args.output_dir.resolve()
    generation_date = date.today().isoformat()

    if args.dry_run:
        print_plan(output_dir, generation_date)
        return 0

    generate_all(output_dir, generation_date)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
