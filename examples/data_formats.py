#!/usr/bin/env python3
"""
OHLCV data format interoperability examples for Finkit.

Demonstrates loading and converting between:
  - CSV (stdlib)
  - Parquet / Arrow (optional pyarrow)
  - pandas DataFrame (optional pandas)
  - polars DataFrame (optional polars)

Run:
    python examples/data_formats.py
    python examples/data_formats.py --check   # self-test all available backends
"""

from __future__ import annotations

import argparse
import csv
import io
import os
import sys
import tempfile
from typing import Any, Callable, Dict, List, Optional

# ---------------------------------------------------------------------------
# Sample OHLCV payload (stdlib)
# ---------------------------------------------------------------------------

SAMPLE_ROWS: List[Dict[str, Any]] = [
    {"date": "2024-01-01", "open": 100.0, "high": 105.0, "low": 98.0, "close": 103.0, "volume": 1000.0},
    {"date": "2024-01-02", "open": 103.0, "high": 108.0, "low": 101.0, "close": 107.0, "volume": 1200.0},
    {"date": "2024-01-03", "open": 107.0, "high": 110.0, "low": 104.0, "close": 106.0, "volume": 900.0},
    {"date": "2024-01-04", "open": 106.0, "high": 109.0, "low": 102.0, "close": 108.0, "volume": 1100.0},
    {"date": "2024-01-05", "open": 108.0, "high": 112.0, "low": 106.0, "close": 111.0, "volume": 1300.0},
    {"date": "2024-01-06", "open": 111.0, "high": 114.0, "low": 109.0, "close": 113.0, "volume": 1400.0},
    {"date": "2024-01-07", "open": 113.0, "high": 116.0, "low": 110.0, "close": 115.0, "volume": 1500.0},
    {"date": "2024-01-08", "open": 115.0, "high": 118.0, "low": 112.0, "close": 117.0, "volume": 1600.0},
    {"date": "2024-01-09", "open": 117.0, "high": 120.0, "low": 114.0, "close": 119.0, "volume": 1700.0},
    {"date": "2024-01-10", "open": 119.0, "high": 122.0, "low": 116.0, "close": 121.0, "volume": 1800.0},
]

OHLCV_COLUMNS = ("open", "high", "low", "close", "volume")


def _optional_import(module: str) -> Optional[Any]:
    try:
        return __import__(module)
    except ImportError:
        return None


# ---------------------------------------------------------------------------
# Stdlib CSV
# ---------------------------------------------------------------------------

def rows_to_csv_text(rows: List[Dict[str, Any]]) -> str:
    buf = io.StringIO()
    writer = csv.DictWriter(buf, fieldnames=list(rows[0].keys()))
    writer.writeheader()
    writer.writerows(rows)
    return buf.getvalue()


def load_ohlcv_from_csv(path: str) -> List[Dict[str, Any]]:
    with open(path, newline="", encoding="utf-8") as f:
        return list(csv.DictReader(f))


def example_csv() -> List[Dict[str, Any]]:
    print("=== CSV (stdlib) ===")
    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".csv", delete=False, encoding="utf-8", newline=""
    ) as tmp:
        tmp.write(rows_to_csv_text(SAMPLE_ROWS))
        csv_path = tmp.name

    rows = load_ohlcv_from_csv(csv_path)
    print(f"  Loaded {len(rows)} rows from {csv_path}")
    os.unlink(csv_path)
    return rows


# ---------------------------------------------------------------------------
# Optional: pyarrow Parquet / Arrow
# ---------------------------------------------------------------------------

def example_parquet_arrow(rows: List[Dict[str, Any]]) -> Optional[Any]:
    pa = _optional_import("pyarrow")
    if pa is None:
        print("=== Parquet/Arrow (pyarrow) - SKIPPED (not installed) ===")
        return None

    print("=== Parquet / Arrow (pyarrow) ===")
    import pyarrow as pa
    import pyarrow.parquet as pq

    table = pa.Table.from_pylist(rows)
    with tempfile.NamedTemporaryFile(suffix=".parquet", delete=False) as tmp:
        parquet_path = tmp.name

    pq.write_table(table, parquet_path)
    loaded_table = pq.read_table(parquet_path)
    print(f"  Parquet round-trip: {loaded_table.num_rows} rows")

    arrow_bytes = loaded_table.serialize().to_pybytes()
    from_arrow = pa.ipc.open_stream(arrow_bytes).read_all()
    print(f"  Arrow IPC stream: {from_arrow.num_rows} rows")

    os.unlink(parquet_path)
    return loaded_table


# ---------------------------------------------------------------------------
# Optional: pandas
# ---------------------------------------------------------------------------

def example_pandas(rows: List[Dict[str, Any]]) -> Optional[Any]:
    pd = _optional_import("pandas")
    if pd is None:
        print("=== pandas - SKIPPED (not installed) ===")
        return None

    print("=== pandas ===")
    import pandas as pd

    df = pd.DataFrame(rows)
    for col in OHLCV_COLUMNS:
        df[col] = df[col].astype(float)

    print(f"  DataFrame shape: {df.shape}")
    print(f"  Columns: {list(df.columns)}")

    # CSV round-trip via pandas
    csv_buf = io.StringIO()
    df.to_csv(csv_buf, index=False)
    df_from_csv = pd.read_csv(io.StringIO(csv_buf.getvalue()))
    print(f"  pandas CSV round-trip: {len(df_from_csv)} rows")

    # Parquet round-trip when pyarrow is available
    if _optional_import("pyarrow") is not None:
        with tempfile.NamedTemporaryFile(suffix=".parquet", delete=False) as tmp:
            parquet_path = tmp.name
        df.to_parquet(parquet_path, index=False)
        df_parquet = pd.read_parquet(parquet_path)
        print(f"  pandas Parquet round-trip: {len(df_parquet)} rows")
        os.unlink(parquet_path)

    return df


# ---------------------------------------------------------------------------
# Optional: polars
# ---------------------------------------------------------------------------

def example_polars(rows: List[Dict[str, Any]]) -> Optional[Any]:
    pl = _optional_import("polars")
    if pl is None:
        print("=== polars - SKIPPED (not installed) ===")
        return None

    print("=== polars ===")
    import polars as pl

    df = pl.DataFrame(rows)
    for col in OHLCV_COLUMNS:
        df = df.with_columns(pl.col(col).cast(pl.Float64))

    print(f"  DataFrame shape: {df.shape}")
    print(f"  Columns: {df.columns}")

    # CSV round-trip
    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".csv", delete=False, encoding="utf-8", newline=""
    ) as tmp:
        csv_path = tmp.name
    df.write_csv(csv_path)
    df_csv = pl.read_csv(csv_path)
    print(f"  polars CSV round-trip: {df_csv.height} rows")
    os.unlink(csv_path)

    # Parquet round-trip when pyarrow is available
    if _optional_import("pyarrow") is not None:
        with tempfile.NamedTemporaryFile(suffix=".parquet", delete=False) as tmp:
            parquet_path = tmp.name
        df.write_parquet(parquet_path)
        df_parquet = pl.read_parquet(parquet_path)
        print(f"  polars Parquet round-trip: {df_parquet.height} rows")
        os.unlink(parquet_path)

    return df


# ---------------------------------------------------------------------------
# Cross-format conversion
# ---------------------------------------------------------------------------

def cross_convert(rows: List[Dict[str, Any]]) -> None:
    print("=== Cross-format conversion ===")
    pd = _optional_import("pandas")
    pl = _optional_import("polars")

    if pd is None or pl is None:
        print("  pandas <-> polars conversion - SKIPPED (need both installed)")
        return

    import pandas as pd
    import polars as pl

    pdf = pd.DataFrame(rows)
    plf = pl.from_pandas(pdf)
    back_to_pandas = plf.to_pandas()
    print(f"  pandas -> polars -> pandas: {len(back_to_pandas)} rows")

    pl_direct = pl.DataFrame(rows)
    pd_from_pl = pl_direct.to_pandas()
    print(f"  polars -> pandas: {len(pd_from_pl)} rows")


# ---------------------------------------------------------------------------
# Finkit integration (optional)
# ---------------------------------------------------------------------------

def example_finkit_compute(rows: List[Dict[str, Any]]) -> None:
    ta = _optional_import("finkit")
    if ta is None:
        print("=== finkit compute - SKIPPED (finkit not installed) ===")
        return

    print("=== finkit indicator on loaded OHLCV ===")
    import finkit as ta

    closes = [float(r["close"]) for r in rows]
    rsi = ta.rsi(closes, timeperiod=5)
    sma = ta.sma(closes, timeperiod=5)
    print(f"  RSI(5) last value: {rsi[-1]:.4f}")
    print(f"  SMA(5) last value: {sma[-1]:.4f}")


# ---------------------------------------------------------------------------
# Self-check (--check)
# ---------------------------------------------------------------------------

def run_check() -> bool:
    """Run all available examples and verify row counts."""
    print("Running self-check...\n")
    errors: List[str] = []

    rows = example_csv()
    if len(rows) != len(SAMPLE_ROWS):
        errors.append(f"CSV: expected {len(SAMPLE_ROWS)} rows, got {len(rows)}")

    arrow_table = example_parquet_arrow(rows)
    if arrow_table is not None and arrow_table.num_rows != len(SAMPLE_ROWS):
        errors.append("Parquet/Arrow row count mismatch")

    pdf = example_pandas(rows)
    if pdf is not None and len(pdf) != len(SAMPLE_ROWS):
        errors.append("pandas row count mismatch")

    plf = example_polars(rows)
    if plf is not None and plf.height != len(SAMPLE_ROWS):
        errors.append("polars row count mismatch")

    cross_convert(rows)
    example_finkit_compute(rows)

    print()
    if errors:
        for err in errors:
            print(f"CHECK FAILED: {err}")
        return False

    print("CHECK PASSED: all available backends OK")
    return True


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> int:
    parser = argparse.ArgumentParser(description="OHLCV data format examples for Finkit")
    parser.add_argument(
        "--check",
        action="store_true",
        help="Run self-check on all available format backends",
    )
    args = parser.parse_args()

    if args.check:
        return 0 if run_check() else 1

    rows = example_csv()
    example_parquet_arrow(rows)
    example_pandas(rows)
    example_polars(rows)
    cross_convert(rows)
    example_finkit_compute(rows)
    print("\nDone.")
    return 0


if __name__ == "__main__":
    sys.exit(main())