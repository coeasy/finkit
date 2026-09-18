#!/usr/bin/env python3
"""Audit the checked-in TA-Lib surface against an installed Python wrapper.

This is an audit gate, not a golden-data generator.  It intentionally compares
the upstream public function directory with the repository's fixed numeric
reference set so that a version bump cannot be mistaken for full coverage.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MATRIX = ROOT / "tests" / "contracts" / "talib_coverage_matrix_v1.json"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--expected-version",
        default="0.8.0",
        help="Required TA-Lib Python wrapper version (default: 0.8.0)",
    )
    parser.add_argument(
        "--require-complete",
        action="store_true",
        help="Return failure when the checked-in numeric reference set is incomplete",
    )
    args = parser.parse_args()

    try:
        import talib
    except ImportError as exc:
        print(f"TA-Lib Python wrapper is unavailable: {exc}", file=sys.stderr)
        return 2

    installed_version = str(getattr(talib, "__version__", "unknown"))
    upstream = {name.upper() for name in talib.get_functions()}
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    checked_in = {
        name.upper()
        for name in matrix["surfaces"]["numeric_reference"]["indicators"]
    }
    missing = sorted(upstream - checked_in)
    extra = sorted(checked_in - upstream)

    print(f"TA-Lib Python: {installed_version}")
    print(f"TA-Lib core: {getattr(talib, '__ta_version__', 'unknown')}")
    print(f"upstream functions: {len(upstream)}")
    print(f"checked-in numeric references: {len(checked_in)}")
    print(f"missing numeric references: {len(missing)}")
    if missing:
        print("missing: " + " ".join(missing))
    if extra:
        print("extra: " + " ".join(extra))

    if installed_version != args.expected_version:
        print(
            f"version mismatch: expected {args.expected_version}, "
            f"got {installed_version}",
            file=sys.stderr,
        )
        return 2
    if args.require_complete and missing:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
