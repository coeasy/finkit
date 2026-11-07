"""Tests for the TA-Lib benchmark acceptance gates."""

from __future__ import annotations

import math
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from bench_report import gate_vs_geomean, geometric_mean_speedup


def rows(*speedups: float) -> dict[str, dict[str, float]]:
    return {f"row_{index}": {"speedup": value} for index, value in enumerate(speedups)}


class GeometricMeanGateTests(unittest.TestCase):
    def test_accepts_slow_rows_when_full_set_geomean_exceeds_one(self) -> None:
        benchmarks = rows(0.5, 2.5, 2.0)
        failures, geomean = gate_vs_geomean(benchmarks, 1.0)
        self.assertFalse(failures)
        self.assertAlmostEqual(geomean or 0, (0.5 * 2.5 * 2.0) ** (1 / 3))

    def test_rejects_geomean_equal_to_or_below_minimum(self) -> None:
        for benchmarks in (rows(0.5, 2.0), rows(0.5, 1.0)):
            with self.subTest(benchmarks=benchmarks):
                failures, _ = gate_vs_geomean(benchmarks, 1.0)
                self.assertTrue(failures)

    def test_rejects_empty_or_invalid_speedups(self) -> None:
        invalid_cases = (
            {},
            rows(0),
            rows(-1),
            rows(math.inf),
            rows(math.nan),
            {"missing": {}},
        )
        for benchmarks in invalid_cases:
            with self.subTest(benchmarks=benchmarks):
                failures, geomean = gate_vs_geomean(benchmarks, 1.0)
                self.assertTrue(failures)
                self.assertIsNone(geomean)

    def test_geometric_mean_helper_uses_all_rows(self) -> None:
        self.assertAlmostEqual(geometric_mean_speedup(rows(0.25, 4.0)) or 0, 1.0)


if __name__ == "__main__":
    unittest.main()
