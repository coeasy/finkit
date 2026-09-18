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
CPP_OUTPUT_PATH = ROOT / "ffi/c-binding/tests/talib_numeric_contract_generated.hpp"
C_OUTPUT_PATH = ROOT / "ffi/c-binding/tests/talib_numeric_contract_generated.h"
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


def cpp_raw(value: str) -> str:
    """Render a JSON fragment as a collision-safe C++ raw string literal."""

    for delimiter in ("finkit", "finkit_json", "finkit_contract", "finkit_numeric"):
        if f"){delimiter}\"" not in value:
            return f'R"{delimiter}({value}){delimiter}"'
    raise ValueError("could not find a safe C++ raw-string delimiter")


def c_string(value: str) -> str:
    """Render an ASCII/UTF-8 JSON fragment as a portable C string literal."""

    encoded = json.dumps(value, ensure_ascii=True)
    return encoded


def render_cpp_contract(payload: dict[str, Any]) -> str:
    """Project the JSON contract into a dependency-free C++17 test fixture."""

    inputs = json.dumps(payload["inputs"], separators=(",", ":"), allow_nan=False)
    lines = [
        "// @generated by scripts/gen_talib_numeric_contract.py; DO NOT EDIT.",
        "#pragma once",
        "",
        "#include <cstddef>",
        "",
        "namespace finkit_test_contract {",
        "",
        "struct NumericContractVector {",
        "    const char* operation;",
        "    const char* input_order_json;",
        "    const char* params_json;",
        "    const char* expected_json;",
        "    double atol;",
        "    double rtol;",
        "};",
        "",
        f"inline constexpr const char* kSemanticProfile = {cpp_raw(payload['semantic_profile'])};",
        f"inline constexpr const char* kInputsJson = {cpp_raw(inputs)};",
        "",
        "inline constexpr NumericContractVector kVectors[] = {",
    ]
    for vector in payload["vectors"]:
        input_order = json.dumps(vector["input_order"], separators=(",", ":"), allow_nan=False)
        params = json.dumps(vector["params"], separators=(",", ":"), allow_nan=False)
        expected = json.dumps(vector["expected"], separators=(",", ":"), allow_nan=False)
        lines.append(
            "    {"
            f"{cpp_raw(vector['operation'])}, "
            f"{cpp_raw(input_order)}, "
            f"{cpp_raw(params)}, "
            f"{cpp_raw(expected)}, "
            f"{vector['tolerance']['atol']:.17g}, "
            f"{vector['tolerance']['rtol']:.17g}"
            "},"
        )
    lines.extend(
        [
            "};",
            "",
            "inline constexpr std::size_t kVectorCount = sizeof(kVectors) / sizeof(kVectors[0]);",
            "",
            "}  // namespace finkit_test_contract",
            "",
        ]
    )
    return "\n".join(lines)


def render_c_contract(payload: dict[str, Any]) -> str:
    """Project the same contract into a dependency-free C99 test fixture."""

    inputs = json.dumps(payload["inputs"], separators=(",", ":"), allow_nan=False)
    lines = [
        "/* @generated by scripts/gen_talib_numeric_contract.py; DO NOT EDIT. */",
        "#ifndef FINKIT_TEST_TALIB_NUMERIC_CONTRACT_GENERATED_H",
        "#define FINKIT_TEST_TALIB_NUMERIC_CONTRACT_GENERATED_H",
        "",
        "#include <stddef.h>",
        "",
        "typedef struct FinkitNumericContractVector {",
        "    const char* operation;",
        "    const char* input_order_json;",
        "    const char* params_json;",
        "    const char* expected_json;",
        "    double atol;",
        "    double rtol;",
        "} FinkitNumericContractVector;",
        "",
        f"static const char finkit_test_contract_semantic_profile[] = {c_string(payload['semantic_profile'])};",
        f"static const char finkit_test_contract_inputs_json[] = {c_string(inputs)};",
        "",
        "static const FinkitNumericContractVector finkit_test_contract_vectors[] = {",
    ]
    for vector in payload["vectors"]:
        input_order = json.dumps(vector["input_order"], separators=(",", ":"), allow_nan=False)
        params = json.dumps(vector["params"], separators=(",", ":"), allow_nan=False)
        expected = json.dumps(vector["expected"], separators=(",", ":"), allow_nan=False)
        lines.append(
            "    {"
            f"{c_string(vector['operation'])}, "
            f"{c_string(input_order)}, "
            f"{c_string(params)}, "
            f"{c_string(expected)}, "
            f"{vector['tolerance']['atol']:.17g}, "
            f"{vector['tolerance']['rtol']:.17g}"
            "},"
        )
    lines.extend(
        [
            "};",
            "",
            "static const size_t finkit_test_contract_vector_count =",
            "    sizeof(finkit_test_contract_vectors) / sizeof(finkit_test_contract_vectors[0]);",
            "",
            "#endif  /* FINKIT_TEST_TALIB_NUMERIC_CONTRACT_GENERATED_H */",
            "",
        ]
    )
    return "\n".join(lines)


def normalized_text(path: Path) -> str:
    """Read generated text independent of checkout newline policy."""

    return path.read_text(encoding="utf-8").replace("\r\n", "\n")


def assert_generated(path: Path, expected: str) -> None:
    """Fail with an actionable CI annotation when a generated file drifts."""

    try:
        actual = normalized_text(path)
    except OSError as error:
        print(f"::error file={path}::cannot read generated contract: {error}")
        raise SystemExit(f"missing generated file: {path}") from error
    if actual == expected:
        return
    limit = min(len(actual), len(expected))
    first_difference = next(
        (index for index in range(limit) if actual[index] != expected[index]), limit
    )
    detail = (
        f"out of date (checked_length={len(actual)}, generated_length={len(expected)}, "
        f"first_difference={first_difference})"
    )
    print(f"::error file={path}::{detail}")
    raise SystemExit(f"out of date: {path}; {detail}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify the checked-in contract is generated from current goldens",
    )
    args = parser.parse_args()
    payload = build_payload()
    generated = json.dumps(payload, indent=2) + "\n"
    generated_cpp = render_cpp_contract(payload)
    generated_c = render_c_contract(payload)
    if args.check:
        assert_generated(OUTPUT_PATH, generated)
        assert_generated(CPP_OUTPUT_PATH, generated_cpp)
        assert_generated(C_OUTPUT_PATH, generated_c)
        print(f"checked {OUTPUT_PATH}")
        print(f"checked {CPP_OUTPUT_PATH}")
        print(f"checked {C_OUTPUT_PATH}")
    else:
        OUTPUT_PATH.write_text(generated, encoding="utf-8")
        CPP_OUTPUT_PATH.write_text(generated_cpp, encoding="utf-8")
        C_OUTPUT_PATH.write_text(generated_c, encoding="utf-8")
        print(f"wrote {OUTPUT_PATH}")
        print(f"wrote {CPP_OUTPUT_PATH}")
        print(f"wrote {C_OUTPUT_PATH}")


if __name__ == "__main__":
    main()
