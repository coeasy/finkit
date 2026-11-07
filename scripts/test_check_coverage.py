#!/usr/bin/env python3
"""Regression tests for the indicator coverage report's matching contract."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

import check_coverage


class CoverageMatchingTests(unittest.TestCase):
    def test_normalized_exact_match(self) -> None:
        result = check_coverage.match_name(
            "Bollinger-Bands",
            {"bollingerbands": [{"name": "BollingerBands", "source": "x.rs"}]},
            {},
        )
        self.assertEqual(result["status"], "exact")

    def test_prefix_and_substring_are_not_matches(self) -> None:
        symbols = {"deathvalley": [{"name": "death_valley", "source": "x.rs"}]}
        self.assertEqual(check_coverage.match_name("HV", symbols, {})["status"], "unmatched")
        self.assertEqual(check_coverage.match_name("CORREL", symbols, {})["status"], "unmatched")

    def test_reviewed_alias_must_resolve_to_a_real_symbol(self) -> None:
        symbols = {"correlation": [{"name": "correlation", "source": "x.rs"}]}
        result = check_coverage.match_name("CORREL", symbols, {"CORREL": "correlation"})
        self.assertEqual(result["status"], "alias")
        self.assertEqual(result["name"], "correlation")
        self.assertEqual(
            check_coverage.validate_aliases({"CORREL": "missing"}, symbols, "batch"),
            ["batch alias 'CORREL' -> 'missing' is missing"],
        )

    def test_current_registry_aliases_resolve_and_formula_extractor_is_live(self) -> None:
        report = check_coverage.build_report()
        self.assertEqual(report["alias_errors"], [])
        self.assertGreater(report["formula"]["registered_function_count"], 0)
        by_name = {row["name"]: row for row in report["rows"]}
        self.assertIn(by_name["Williams %R"]["streaming"]["status"], {"exact", "alias"})
        self.assertIn(by_name["LINEAR_REG"]["streaming"]["status"], {"exact", "alias"})


if __name__ == "__main__":
    unittest.main()
