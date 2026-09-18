#!/usr/bin/env python3
"""Generate the cross-language TA-Lib numeric contract from checked-in goldens.

The golden JSON files remain the only reference source.  This generator merely
projects one deterministic slice of the synthetic fixture into the shared JSON
operation contract so every binding can execute the same requests and compare
the same TA-Lib 0.8.0 values.
"""

from __future__ import annotations

import csv
import argparse
import json
import math
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
MATRIX_PATH = ROOT / "tests/contracts/talib_coverage_matrix_v1.json"
GOLDEN_DIR = ROOT / "tests/golden/talib"
FIXTURE_PATH = ROOT / "tests/fixtures/synthetic_waves_500.csv"
OUTPUT_PATH = ROOT / "tests/contracts/talib_numeric_contract_v1.json"
ROWS = 160

# Profile-only operations whose public TA-Lib output is named REAL rather than
# after the operation.  Multi-output names are handled by the aliases below.
REAL_OUTPUTS = {
    "AC",
    "ADR",
    "CMOU",
    "CVI",
    "EFI",
    "FOSC",
    "MARKETFI",
    "MASSI",
    "PERCENTILE",
    "PVO",
    "QSTICK",
    "RMA",
    "RVI",
    "RVOL",
    "VHF",
    "WAD",
}

OUTPUT_ALIASES = {
    "MACDSIGNAL": "MACD_SIGNAL",
    "MACDHIST": "MACD_HIST",
    "AROONDOWN": "AROON_DOWN",
    "AROONUP": "AROON_UP",
    "DCPERIOD": "HT_DCPERIOD",
    "DCPHASE": "HT_DCPHASE",
    "TRENDLINE": "HT_TRENDLINE",
    "TRENDMODE": "HT_TRENDMODE",
}


def read_fixture() -> dict[str, list[float]]:
    series = {name: [] for name in ("open", "high", "low", "close", "volume")}
    with FIXTURE_PATH.open(encoding="utf-8", newline="") as handle:
        rows = (line for line in handle if not line.startswith("#"))
        reader = csv.DictReader(rows)
        for row in reader:
            for name in series:
                series[name].append(float(row[name]))

    close = series["close"]
    series["math"] = [0.25 + math.sin(index * 0.11) * 0.2 for index in range(len(close))]
    series["benchmark"] = [value * 0.97 + index * 0.02 for index, value in enumerate(close)]
    series["periods"] = [float(2 + index % 29) for index in range(len(close))]
    return {name: values[:ROWS] for name, values in series.items()}


def tolerance(indicator: str) -> dict[str, float]:
    if indicator.startswith("CDL"):
        return {"atol": 0.0, "rtol": 0.0}
    if indicator.startswith("HT_"):
        return {"atol": 1e-5, "rtol": 1e-8}
    if indicator in {"SMA", "WMA", "BBANDS", "AD"}:
        return {"atol": 1e-6, "rtol": 1e-8}
    if indicator in {"ADOSC", "CORREL", "STDDEV", "VAR"}:
        return {"atol": 2e-7, "rtol": 2e-7}
    if indicator in {"EMA", "DEMA", "TEMA"}:
        return {"atol": 1e-8, "rtol": 1e-8}
    return {"atol": 1e-8, "rtol": 1e-8}


def actual_output_names(indicator: str, golden_names: list[str]) -> list[str]:
    if len(golden_names) == 1 and indicator in REAL_OUTPUTS:
        return ["REAL"]
    names = []
    for name in golden_names:
        canonical = name.upper()
        canonical = OUTPUT_ALIASES.get(canonical, canonical)
        names.append(canonical)
    return names


def golden_outputs(indicator: str, metadata: dict[str, Any], raw: dict[str, Any]) -> dict[str, list[Any]]:
    golden_names = list(metadata["outputs"])
    actual_names = actual_output_names(indicator, golden_names)
    if len(golden_names) != len(set(actual_names)):
        raise ValueError(f"{indicator}: duplicate projected output names: {actual_names}")
    result: dict[str, list[Any]] = {}
    for golden_name, actual_name in zip(golden_names, actual_names, strict=True):
        values = raw["results"]["synthetic"]["outputs"][golden_name][:ROWS]
        if len(values) != ROWS:
            raise ValueError(f"{indicator}/{golden_name}: expected {ROWS} values")
        result[actual_name] = values
    return result


def build_payload() -> dict[str, Any]:
    matrix = json.loads(MATRIX_PATH.read_text(encoding="utf-8"))
    names = matrix["surfaces"]["numeric_reference"]["indicators"]
    inputs = read_fixture()
    vectors = []
    golden_names = set()

    for indicator in names:
        path = GOLDEN_DIR / f"{indicator.lower()}.json"
        raw = json.loads(path.read_text(encoding="utf-8"))
        metadata = raw["metadata"]
        if metadata["indicator"] != indicator or metadata["talib_version"] != "0.8.0":
            raise ValueError(f"{indicator}: golden metadata is not pinned to TA-Lib 0.8.0")
        golden_names.add(indicator)
        input_order = [name.upper() for name in metadata["inputs"]]
        missing = [name for name in metadata["inputs"] if name not in inputs]
        if missing:
            raise ValueError(f"{indicator}: missing generated inputs {missing}")
        vectors.append(
            {
                "operation": indicator,
                "input_order": input_order,
                "params": list(metadata["parameters"].values()),
                "expected": golden_outputs(indicator, metadata, raw),
                "tolerance": tolerance(indicator),
            }
        )

    if set(names) != golden_names or len(vectors) != 201:
        raise ValueError("coverage matrix and golden files must contain exactly 201 indicators")

    return {
        "schema_version": 1,
        "semantic_profile": matrix["semantic_profile"],
        "reference": {
            "talib_python_version": matrix["python_reference_version"],
            "dataset_id": "synthetic",
            "rows": ROWS,
            "generator": "scripts/gen_talib_numeric_contract.py",
            "generator_version": "1.0.0",
        },
        "inputs": {name.upper(): values for name, values in inputs.items()},
        "vectors": vectors,
    }
def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify the checked-in contract is generated from current goldens",
    )
    args = parser.parse_args()
    generated = json.dumps(build_payload(), indent=2) + "\n"
    if args.check:
        checked_in = OUTPUT_PATH.read_text(encoding="utf-8")
        if checked_in != generated:
            raise SystemExit(f"out of date: {OUTPUT_PATH}")
        print(f"checked {OUTPUT_PATH}")
    else:
        OUTPUT_PATH.write_text(generated, encoding="utf-8")
        print(f"wrote {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
