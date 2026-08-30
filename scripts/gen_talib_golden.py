#!/usr/bin/env python3
"""Generate TA-Lib C reference outputs for FTA golden tests.

Calls the TA-Lib Python package (C binding) on shared fixture datasets under
``tests/fixtures/`` and writes one JSON file per indicator to
``tests/golden/talib/``.

Usage:
    python scripts/gen_talib_golden.py
    python scripts/gen_talib_golden.py --dry-run
    python scripts/gen_talib_golden.py --fixtures-dir tests/fixtures
    python scripts/gen_talib_golden.py --output-dir tests/golden/talib

If fixtures are missing, run first:
    python scripts/gen_test_fixtures.py

Requires (non dry-run): numpy, TA-Lib Python package + TA-Lib C library.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import subprocess
import sys
from datetime import date
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FIXTURES_DIR = ROOT / "tests" / "fixtures"
DEFAULT_OUTPUT_DIR = ROOT / "tests" / "golden" / "talib"
FIXTURES_SCRIPT = ROOT / "scripts" / "gen_test_fixtures.py"

GENERATOR_VERSION = "1.0.0"

# Three fixture datasets: A-share daily, crypto minute, synthetic waves.
DATASET_IDS = ("ashare", "crypto", "synthetic")

# Indicator definitions — parameters aligned with tests/golden/*.csv naming.
INDICATORS: dict[str, dict[str, Any]] = {
    "SMA": {
        "params": {"timeperiod": 10},
        "fn_name": "SMA",
        "inputs": ("close",),
        "outputs": ("sma",),
    },
    "EMA": {
        "params": {"timeperiod": 10},
        "fn_name": "EMA",
        "inputs": ("close",),
        "outputs": ("ema",),
    },
    "RSI": {
        "params": {"timeperiod": 14},
        "fn_name": "RSI",
        "inputs": ("close",),
        "outputs": ("rsi",),
    },
    "MACD": {
        "params": {
            "fastperiod": 12,
            "slowperiod": 26,
            "signalperiod": 9,
        },
        "fn_name": "MACD",
        "inputs": ("close",),
        "outputs": ("macd", "macdsignal", "macdhist"),
    },
    "BBANDS": {
        "params": {
            "timeperiod": 20,
            "nbdevup": 2.0,
            "nbdevdn": 2.0,
            "matype": 0,
        },
        "fn_name": "BBANDS",
        "inputs": ("close",),
        "outputs": ("upperband", "middleband", "lowerband"),
    },
    "ATR": {
        "params": {"timeperiod": 14},
        "fn_name": "ATR",
        "inputs": ("high", "low", "close"),
        "outputs": ("atr",),
    },
    "ADX": {
        "params": {"timeperiod": 14},
        "fn_name": "ADX",
        "inputs": ("high", "low", "close"),
        "outputs": ("adx",),
    },
    "STOCH": {
        "params": {
            "fastk_period": 14,
            "slowk_period": 3,
            "slowk_matype": 0,
            "slowd_period": 3,
            "slowd_matype": 0,
        },
        "fn_name": "STOCH",
        "inputs": ("high", "low", "close"),
        "outputs": ("slowk", "slowd"),
    },
    "CCI": {
        "params": {"timeperiod": 14},
        "fn_name": "CCI",
        "inputs": ("high", "low", "close"),
        "outputs": ("cci",),
    },
    "WILLR": {
        "params": {"timeperiod": 14},
        "fn_name": "WILLR",
        "inputs": ("high", "low", "close"),
        "outputs": ("willr",),
    },
    "MOM": {
        "params": {"timeperiod": 10},
        "fn_name": "MOM",
        "inputs": ("close",),
        "outputs": ("mom",),
    },
    "ROC": {
        "params": {"timeperiod": 10},
        "fn_name": "ROC",
        "inputs": ("close",),
        "outputs": ("roc",),
    },
    "TRIX": {
        "params": {"timeperiod": 14},
        "fn_name": "TRIX",
        "inputs": ("close",),
        "outputs": ("trix",),
    },
    "OBV": {
        "params": {},
        "fn_name": "OBV",
        "inputs": ("close", "volume"),
        "outputs": ("obv",),
    },
    "AD": {
        "params": {},
        "fn_name": "AD",
        "inputs": ("high", "low", "close", "volume"),
        "outputs": ("ad",),
    },
    "DEMA": {
        "params": {"timeperiod": 10},
        "fn_name": "DEMA",
        "inputs": ("close",),
        "outputs": ("dema",),
    },
    "TEMA": {
        "params": {"timeperiod": 10},
        "fn_name": "TEMA",
        "inputs": ("close",),
        "outputs": ("tema",),
    },
    "WMA": {
        "params": {"timeperiod": 10},
        "fn_name": "WMA",
        "inputs": ("close",),
        "outputs": ("wma",),
    },
    "NATR": {
        "params": {"timeperiod": 14},
        "fn_name": "NATR",
        "inputs": ("high", "low", "close"),
        "outputs": ("natr",),
    },
    "APO": {
        "params": {"fastperiod": 12, "slowperiod": 26, "matype": 0},
        "fn_name": "APO",
        "inputs": ("close",),
        "outputs": ("apo",),
    },
    "CMO": {
        "params": {"timeperiod": 14},
        "fn_name": "CMO",
        "inputs": ("close",),
        "outputs": ("cmo",),
    },
    "AROON": {
        "params": {"timeperiod": 14},
        "fn_name": "AROON",
        "inputs": ("high", "low"),
        "outputs": ("aroondown", "aroonup"),
    },
}

INSTALL_HINT = """
TA-Lib is not installed or could not be imported.

Install the C library first, then the Python bindings:

  macOS:
    brew install ta-lib
    pip install numpy TA-Lib

  Linux (Debian/Ubuntu):
    sudo apt install libta-lib0-dev
    pip install numpy TA-Lib

  Windows:
    Download TA-Lib C library from https://github.com/TA-Lib/ta-lib/releases
    Extract to C:\\ta-lib (or set TA_LIBRARY_PATH / TA_INCLUDE_PATH)
    pip install numpy TA-Lib

See also: packaging/usage/all/bench-vs-talib.md
""".strip()


def load_fixture_metadata(fixtures_dir: Path) -> dict[str, Any]:
    meta_path = fixtures_dir / "metadata.json"
    if not meta_path.is_file():
        return {}
    return json.loads(meta_path.read_text(encoding="utf-8"))


def dataset_files(fixtures_dir: Path, metadata: dict[str, Any]) -> dict[str, Path]:
    """Map dataset id -> CSV path for ashare, crypto, synthetic."""
    by_id: dict[str, Path] = {}
    for entry in metadata.get("datasets", []):
        if entry.get("id") in DATASET_IDS:
            by_id[entry["id"]] = fixtures_dir / entry["filename"]

    # Fallback filenames when metadata.json is absent.
    fallbacks = {
        "ashare": "ashare_sh_index_250d.csv",
        "crypto": "crypto_btc_usdt_1m_1000.csv",
        "synthetic": "synthetic_waves_500.csv",
    }
    for ds_id, filename in fallbacks.items():
        if ds_id not in by_id:
            by_id[ds_id] = fixtures_dir / filename
    return by_id


def missing_datasets(paths: dict[str, Path]) -> list[str]:
    return [ds_id for ds_id, path in paths.items() if not path.is_file()]


def read_ohlcv_csv(path: Path) -> dict[str, list[float]]:
    """Load OHLCV columns from a fixture CSV (skips # comment headers)."""
    rows_open: list[float] = []
    rows_high: list[float] = []
    rows_low: list[float] = []
    rows_close: list[float] = []
    rows_volume: list[float] = []

    with path.open(encoding="utf-8", newline="") as fh:
        reader = csv.reader(
            (line for line in fh if not line.startswith("#")),
        )
        header = next(reader, None)
        if header is None:
            raise ValueError(f"empty fixture: {path}")

        col_index = {name.strip().lower(): i for i, name in enumerate(header)}
        required = ("open", "high", "low", "close", "volume")
        for name in required:
            if name not in col_index:
                raise ValueError(f"{path}: missing column '{name}' in {header}")

        for row in reader:
            if not row or all(not cell.strip() for cell in row):
                continue
            rows_open.append(float(row[col_index["open"]]))
            rows_high.append(float(row[col_index["high"]]))
            rows_low.append(float(row[col_index["low"]]))
            rows_close.append(float(row[col_index["close"]]))
            rows_volume.append(float(row[col_index["volume"]]))

    return {
        "open": rows_open,
        "high": rows_high,
        "low": rows_low,
        "close": rows_close,
        "volume": rows_volume,
    }


def serialize_values(values: Any) -> list[Any]:
    """Convert numpy array / sequence to JSON-safe list (NaN -> null)."""
    out: list[Any] = []
    for v in values:
        fv = float(v)
        out.append(None if math.isnan(fv) or math.isinf(fv) else fv)
    return out


def talib_version_str(talib_module: Any) -> str:
    for attr in ("__version__", "VERSION", "version"):
        ver = getattr(talib_module, attr, None)
        if ver is not None:
            return str(ver)
    try:
        return str(talib_module.get_functions()[0])  # pragma: no cover
    except Exception:
        return "unknown"


def compute_indicator(
    talib_module: Any,
    np_module: Any,
    spec: dict[str, Any],
    ohlcv: dict[str, list[float]],
) -> dict[str, list[Any]]:
    fn = getattr(talib_module, spec["fn_name"])
    args = [np_module.asarray(ohlcv[name], dtype=np_module.float64) for name in spec["inputs"]]
    result = fn(*args, **spec["params"])

    if isinstance(result, tuple):
        series = result
    else:
        series = (result,)

    output_names = spec["outputs"]
    if len(series) != len(output_names):
        raise RuntimeError(
            f"{spec['fn_name']}: expected {len(output_names)} outputs, got {len(series)}"
        )

    return {
        name: serialize_values(arr)
        for name, arr in zip(output_names, series, strict=True)
    }


def build_indicator_payload(
    indicator_name: str,
    spec: dict[str, Any],
    talib_version: str,
    generation_date: str,
    dataset_results: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    return {
        "metadata": {
            "generator": "gen_talib_golden.py",
            "generator_version": GENERATOR_VERSION,
            "generation_date": generation_date,
            "talib_version": talib_version,
            "indicator": indicator_name,
            "talib_function": spec["fn_name"],
            "parameters": spec["params"],
            "inputs": list(spec["inputs"]),
            "outputs": list(spec["outputs"]),
            "datasets": list(DATASET_IDS),
        },
        "results": dataset_results,
    }


def print_plan(
    fixtures_dir: Path,
    output_dir: Path,
    dataset_paths: dict[str, Path],
    generation_date: str,
    talib_available: bool,
    talib_version: str | None,
) -> None:
    print(f"TA-Lib golden generator v{GENERATOR_VERSION} (dry-run)")
    print(f"  generation_date: {generation_date}")
    print(f"  fixtures_dir: {fixtures_dir}")
    print(f"  output_dir: {output_dir}")
    if talib_available and talib_version:
        print(f"  talib_version: {talib_version}")
    else:
        print("  talib_version: (not checked in dry-run)")
    print()
    print("Datasets:")
    for ds_id in DATASET_IDS:
        path = dataset_paths[ds_id]
        exists = path.is_file()
        status = "OK" if exists else "MISSING"
        print(f"  [{status}] {ds_id}: {path}")
    print()
    print(f"Indicators ({len(INDICATORS)}) → one JSON file each:")
    for name, spec in INDICATORS.items():
        out_path = output_dir / f"{name.lower()}.json"
        params = ", ".join(f"{k}={v}" for k, v in spec["params"].items()) or "default"
        outputs = ", ".join(spec["outputs"])
        print(f"  {out_path.name}: {spec['fn_name']}({params}) → {outputs}")
    readme = output_dir / "README.md"
    print()
    print(f"  [readme] {readme}")
    print("No files written (dry-run).")


def ensure_fixtures(fixtures_dir: Path, auto_generate: bool) -> dict[str, Path]:
    metadata = load_fixture_metadata(fixtures_dir)
    paths = dataset_files(fixtures_dir, metadata)
    missing = missing_datasets(paths)
    if not missing:
        return paths

    if auto_generate and FIXTURES_SCRIPT.is_file():
        print(f"Missing fixtures: {', '.join(missing)}")
        print(f"Running {FIXTURES_SCRIPT} ...")
        subprocess.run(
            [sys.executable, str(FIXTURES_SCRIPT)],
            check=True,
            cwd=str(ROOT),
        )
        metadata = load_fixture_metadata(fixtures_dir)
        paths = dataset_files(fixtures_dir, metadata)
        missing = missing_datasets(paths)

    if missing:
        print("Required fixture files are missing:", file=sys.stderr)
        for ds_id in missing:
            print(f"  {ds_id}: {paths[ds_id]}", file=sys.stderr)
        print(
            f"\nGenerate them with:\n  python {FIXTURES_SCRIPT.relative_to(ROOT)}",
            file=sys.stderr,
        )
        sys.exit(1)

    return paths


def import_talib() -> tuple[Any, Any]:
    try:
        import numpy as np
        import talib
    except ImportError as exc:
        print(INSTALL_HINT, file=sys.stderr)
        print(f"\nImport error: {exc}", file=sys.stderr)
        sys.exit(2)
    return talib, np


def generate_all(
    fixtures_dir: Path,
    output_dir: Path,
    dataset_paths: dict[str, Path],
    generation_date: str,
    talib_module: Any,
    np_module: Any,
) -> None:
    output_dir.mkdir(parents=True, exist_ok=True)
    talib_version = talib_version_str(talib_module)
    metadata = load_fixture_metadata(fixtures_dir)

    # Load all datasets once.
    loaded: dict[str, dict[str, Any]] = {}
    for ds_id in DATASET_IDS:
        path = dataset_paths[ds_id]
        ohlcv = read_ohlcv_csv(path)
        rel_path = str(path.relative_to(ROOT)).replace("\\", "/")
        ds_meta = next(
            (d for d in metadata.get("datasets", []) if d.get("id") == ds_id),
            {},
        )
        loaded[ds_id] = {
            "dataset_id": ds_id,
            "fixture_path": rel_path,
            "fixture_type": ds_meta.get("type", ds_id),
            "rows": len(ohlcv["close"]),
            "ohlcv": ohlcv,
        }

    for indicator_name, spec in INDICATORS.items():
        dataset_results: dict[str, Any] = {}
        for ds_id in DATASET_IDS:
            entry = loaded[ds_id]
            outputs = compute_indicator(
                talib_module,
                np_module,
                spec,
                entry["ohlcv"],
            )
            dataset_results[ds_id] = {
                "dataset_id": ds_id,
                "fixture_path": entry["fixture_path"],
                "fixture_type": entry["fixture_type"],
                "rows": entry["rows"],
                "outputs": outputs,
            }

        payload = build_indicator_payload(
            indicator_name,
            spec,
            talib_version,
            generation_date,
            dataset_results,
        )
        out_path = output_dir / f"{indicator_name.lower()}.json"
        out_path.write_text(
            json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )
        print(f"Wrote {out_path}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate TA-Lib C reference JSON golden files for FTA.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print generation plan without importing TA-Lib or writing files.",
    )
    parser.add_argument(
        "--fixtures-dir",
        type=Path,
        default=DEFAULT_FIXTURES_DIR,
        help="Directory containing test fixture CSVs (default: tests/fixtures/)",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=DEFAULT_OUTPUT_DIR,
        help="Output directory for golden JSON files (default: tests/golden/talib/)",
    )
    parser.add_argument(
        "--no-auto-fixtures",
        action="store_true",
        help="Do not auto-run gen_test_fixtures.py when fixtures are missing.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    fixtures_dir = args.fixtures_dir.resolve()
    output_dir = args.output_dir.resolve()
    generation_date = date.today().isoformat()

    if args.dry_run:
        metadata = load_fixture_metadata(fixtures_dir)
        dataset_paths = dataset_files(fixtures_dir, metadata)
        print_plan(
            fixtures_dir,
            output_dir,
            dataset_paths,
            generation_date,
            talib_available=False,
            talib_version=None,
        )
        missing = missing_datasets(dataset_paths)
        if missing:
            print()
            print(
                f"Warning: {len(missing)} fixture file(s) missing "
                f"(dry-run continues). Run: python scripts/gen_test_fixtures.py"
            )
        return 0

    dataset_paths = ensure_fixtures(
        fixtures_dir,
        auto_generate=not args.no_auto_fixtures,
    )

    talib_module, np_module = import_talib()
    generate_all(
        fixtures_dir,
        output_dir,
        dataset_paths,
        generation_date,
        talib_module,
        np_module,
    )
    print(f"Done. TA-Lib version: {talib_version_str(talib_module)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
