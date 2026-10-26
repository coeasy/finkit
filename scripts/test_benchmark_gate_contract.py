#!/usr/bin/env python3
"""Regression tests for strict per-indicator benchmark gates."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from benchmark_gate_contract import evaluate_performance_rows


def row(name: str, size: int, speedup: float) -> dict[str, object]:
    return {"name": name, "n": size, "finkit_speedup_x": speedup}


class BenchmarkGateContractTests(unittest.TestCase):
    def test_speedup_equal_to_floor_passes(self) -> None:
        result = evaluate_performance_rows(
            [row("SMA", 1_000, 1.0)], ["SMA"], minimum_speedup=1.0
        )
        self.assertTrue(result["coverage_pass"])
        self.assertTrue(result["speed_pass"])

    def test_a_single_slow_observation_fails_even_with_a_fast_observation(self) -> None:
        result = evaluate_performance_rows(
            [row("SMA", 1_000, 1.8), row("SMA", 100_000, 0.99)], ["SMA"]
        )
        self.assertFalse(result["speed_pass"])
        self.assertEqual(
            result["below_speed_floor"],
            [{"name": "SMA", "n": 100_000, "finkit_speedup_x": 0.99}],
        )

    def test_missing_or_unexpected_function_fails_coverage(self) -> None:
        result = evaluate_performance_rows(
            [row("SMA", 1_000, 1.1), row("FAKE", 1_000, 2.0)],
            ["SMA", "EMA"],
            expected_sizes=[1_000],
        )
        self.assertFalse(result["coverage_pass"])
        self.assertEqual(result["missing_functions"], ["EMA"])
        self.assertEqual(result["unexpected_functions"], ["FAKE"])
        self.assertEqual(result["missing_observations"], [{"name": "EMA", "n": 1_000}])

    def test_missing_one_function_at_one_size_fails_coverage(self) -> None:
        result = evaluate_performance_rows(
            [row("SMA", 1_000, 1.1), row("SMA", 100_000, 1.2), row("EMA", 1_000, 1.1)],
            ["SMA", "EMA"],
            expected_sizes=[1_000, 100_000],
        )
        self.assertFalse(result["coverage_pass"])
        self.assertEqual(result["missing_observations"], [{"name": "EMA", "n": 100_000}])

    def test_invalid_speed_measurement_fails_instead_of_being_ignored(self) -> None:
        result = evaluate_performance_rows(
            [row("SMA", 1_000, float("nan"))], ["SMA"]
        )
        self.assertFalse(result["speed_pass"])
        self.assertEqual(
            result["invalid_speed_rows"][0]["reason"],
            "non_finite_or_non_positive_speedup",
        )


if __name__ == "__main__":
    unittest.main()
