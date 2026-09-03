#!/usr/bin/env python3
"""Continuation for the one-shot warning cleanup after ambiguous-name guard.

Temporary branch helper; removed before PR creation.
"""
from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(path: str, old: str, new: str, label: str) -> None:
    file = ROOT / path
    text = file.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match in {path}, found {count}")
    file.write_text(text.replace(old, new, 1), encoding="utf-8")
    print(f"fixed {label}")


def main() -> int:
    replace_once(
        "core/tests/formula_cache_tests.rs",
        '''    fn test_cache_miss_new_formula() {\n        let mut engine = FormulaEngine::new();\n\n        assert!(!engine.cache_hit("CLOSE + OPEN"));\n''',
        '''    fn test_cache_miss_new_formula() {\n        let engine = FormulaEngine::new();\n\n        assert!(!engine.cache_hit("CLOSE + OPEN"));\n''',
        "unnecessary mutable cache-miss engine",
    )

    replace_once(
        "core/tests/edge_case_invalid_input.rs",
        '''//! `Err(TaError::InvalidParameter { .. })` (and never panic) when the\n''',
        '''//! `Err(TaError::Indicator(IndicatorError::InvalidParameter { .. }))` (and never panic) when the\n''',
        "invalid-input error model documentation",
    )
    replace_once(
        "core/tests/edge_case_invalid_input.rs",
        "use finkit::error::TaError;\n",
        "use finkit::error::{IndicatorError, TaError};\n",
        "canonical invalid-input error import",
    )
    replace_once(
        "core/tests/edge_case_invalid_input.rs",
        '''        Err(TaError::InvalidParameter { name, constraint }) => {\n            assert!(\n                name == "input" || name == "period" || name == "output" || name == "close",\n                "unexpected param name: {name}"\n            );\n            assert!(\n                constraint.contains(needle),\n                "constraint {constraint:?} should contain {needle:?}"\n            );\n        }\n''',
        '''        Err(TaError::Indicator(IndicatorError::InvalidParameter { param, reason })) => {\n            assert!(\n                param == "input" || param == "period" || param == "output" || param == "close",\n                "unexpected param name: {param}"\n            );\n            assert!(\n                reason.contains(needle),\n                "reason {reason:?} should contain {needle:?}"\n            );\n        }\n''',
        "canonical invalid-input helper match",
    )
    replace_once(
        "core/tests/edge_case_invalid_input.rs",
        '''        Err(TaError::InvalidParameter { name, constraint }) => {\n            assert_eq!(name, "period");\n            assert!(constraint.contains("greater than 0"));\n        }\n''',
        '''        Err(TaError::Indicator(IndicatorError::InvalidParameter { param, reason })) => {\n            assert_eq!(param, "period");\n            assert!(reason.contains("greater than 0"));\n        }\n''',
        "canonical zero-period error match",
    )

    path = ROOT / "core/benches/simd_statistics_bench.rs"
    text = path.read_text(encoding="utf-8")
    start = text.index("fn bench_linear_reg(c: &mut Criterion)")
    end = text.index("\nfn bench_linear_reg_angle(c: &mut Criterion)", start)
    block = text[start:end]
    line = "        let mut result_scalar = vec![0.0; DATA_LEN];\n"
    if block.count(line) != 1:
        raise SystemExit("bench_linear_reg: expected exactly one unused result_scalar buffer")
    block = block.replace(line, "", 1)
    path.write_text(text[:start] + block + text[end:], encoding="utf-8")
    print("fixed unused linear-reg scalar result buffer")

    replace_once(
        "core/benches/accuracy_test.rs",
        "fn test_indicator_accuracy(c: &mut Criterion) {\n    let (open, high, low, close, volume) = generate_test_data(1000);",
        "fn test_indicator_accuracy(_c: &mut Criterion) {\n    let (_open, high, low, close, volume) = generate_test_data(1000);",
        "unused accuracy benchmark context/open",
    )
    replace_once(
        "core/benches/accuracy_test.rs",
        "if let Ok((inphase, quadrature)) = indicators::ht_phasor(&close)",
        "if let Ok((inphase, _quadrature)) = indicators::ht_phasor(&close)",
        "unused HT phasor quadrature",
    )
    replace_once(
        "core/benches/accuracy_test.rs",
        "if let Ok((sine, lead_sine)) = indicators::ht_sine(&close)",
        "if let Ok((sine, _lead_sine)) = indicators::ht_sine(&close)",
        "unused HT sine lead output",
    )

    replace_once(
        "core/benches/formula_performance_bench.rs",
        '''                |mut ctx| {\n                    let _ = black_box(engine.execute_bytecode(&bytecode, &ctx).unwrap());\n''',
        '''                |ctx| {\n                    let _ = black_box(engine.execute_bytecode(&bytecode, &ctx).unwrap());\n''',
        "unnecessary mutable bytecode context",
    )
    replace_once(
        "core/benches/simple_perf.rs",
        "fn benchmark_indicator<F>(name: &str, iterations: usize, f: F) -> f64",
        "fn benchmark_indicator<F>(_name: &str, iterations: usize, f: F) -> f64",
        "unused benchmark name parameter",
    )
    replace_once(
        "core/benches/performance_benchmark.rs",
        "use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};",
        "use criterion::{black_box, criterion_group, criterion_main, Criterion};",
        "unused BenchmarkId import",
    )
    replace_once(
        "core/benches/formula_bench.rs",
        "let arr_pow_b: Vec<f64> = (0..data_len).map(|i| ((i % 5 + 1) as f64 * 0.5)).collect();",
        "let arr_pow_b: Vec<f64> = (0..data_len).map(|i| (i % 5 + 1) as f64 * 0.5).collect();",
        "unnecessary formula benchmark parentheses",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
