#!/usr/bin/env python3
"""One-shot maintenance for test/benchmark warning cleanup.

Temporary branch helper. Every replacement is asserted so the workflow fails
before writing ambiguous edits. The helper is removed before the PR is opened.
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
        "core/src/streaming/overlap/sma.rs",
        "use crate::{test_streaming_meta, test_streaming_reset, test_streaming_vs_batch};",
        "use crate::{test_streaming_meta, test_streaming_vs_batch};",
        "unused streaming reset macro import",
    )

    replace_once(
        "core/src/patterns/astock_kline.rs",
        '''    /// Build a synthetic series of `n` bars with the given opens/closes/...\n    fn synth_uptrend(n: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {\n        let mut opens = vec![10.0; n];\n        let mut highs = vec![10.0; n];\n        let mut lows = vec![10.0; n];\n        let mut closes = vec![10.0; n];\n        let mut volumes = vec![100.0; n];\n        for i in 0..n {\n            let p = 10.0 + (i as f64) * 0.1;\n            opens[i] = p - 0.2;\n            closes[i] = p + 0.2;\n            highs[i] = p + 0.5;\n            lows[i] = p - 0.5;\n            volumes[i] = 100.0 + (i as f64) * 5.0;\n        }\n        (opens, highs, lows, closes, volumes)\n    }\n\n''',
        "",
        "dead synth_uptrend test helper",
    )
    replace_once(
        "core/src/patterns/astock_kline.rs",
        '''        let mut h = vec![10.2; n];\n        let mut l = vec![9.5; n];\n        // Three bars with progressive gap-downs\n''',
        '''        let h = vec![10.2; n];\n        let mut l = vec![9.5; n];\n        // Three bars with progressive gap-downs\n''',
        "unnecessary mutable high series",
    )

    # Remove unused length declarations from tests that use literal vectors.
    for label, snippet in [
        ("tweezer_bottom unused n", "    fn test_tweezer_bottom() {\n        let n = 5;\n"),
        ("tower_bottom unused n", "    fn test_tower_bottom() {\n        let n = 10;\n"),
        ("tower_top unused n", "    fn test_tower_top() {\n        let n = 10;\n"),
        ("separation_lines unused n", "    fn test_separation_lines() {\n        let n = 5;\n"),
    ]:
        replace_once(
            "core/src/patterns/astock_kline2.rs",
            snippet,
            snippet.rsplit("        let n", 1)[0],
            label,
        )

    replace_once(
        "core/src/patterns/astock_kline2.rs",
        '''    fn test_kneading_line() {\n        let n = 5;\n        let o = vec![10.0, 9.0, 10.5];\n        let h = vec![10.2, 9.7, 11.0];\n        let l = vec![9.8, 8.7, 10.3];\n        let c = vec![9.5, 9.5, 10.7];\n        // bar 0: yin (close < open), body 0.5\n        // bar 1: yin, body 0.5, up 0.2, lo 0.3\n        // bar 2: yang, body 0.2 (smaller)\n        // Hmm bodies not equal\n        // Try:\n        let o = vec![10.0, 9.0, 9.5];\n        let h = vec![10.5, 9.7, 10.0];\n        let l = vec![8.5, 8.7, 8.5];\n        let c = vec![9.0, 9.5, 9.0];\n        // bar 0: yin body 1.0\n        // bar 1: yang body 0.5, up 0.2, lo 0.3 → up < 0.3*b=0.15? 0.2>0.15 ✓, lo < 0.15? 0.3>0.15 ✓\n        // bar 2: yin body 0.5\n        // check ratio: |1.0-0.5|/1.0=0.5 > 0.3\n        // Just check length\n        let _ = (o, h, l, c);\n        let (o, h, l, c, _) = flat_synth(5);\n''',
        '''    fn test_kneading_line() {\n        // Keep this as a smoke/shape contract; dedicated pattern fixtures live elsewhere.\n        let (o, h, l, c, _) = flat_synth(5);\n''',
        "shadowed kneading-line fixtures",
    )

    replace_once(
        "core/src/patterns/astock_kline2.rs",
        '''    fn test_rising_sun() {\n        let n = 5;\n        let o = vec![10.0, 11.0];\n        let h = vec![10.3, 11.5];\n        let l = vec![9.5, 9.5];\n        let c = vec![9.8, 11.4];\n        // bar 0: yin, body 0.2\n        // bar 1: yang, open 11.0, close 11.4, body 0.4\n        // prev close 9.8, cur open 11.0 → open > prev close → no\n        // Use:\n        let o = vec![10.0, 9.5];\n        let h = vec![10.3, 11.5];\n        let l = vec![9.5, 9.0];\n        let c = vec![9.8, 11.0];\n        // bar 0: yin, body 0.2 (open 10.0 close 9.8)\n        // bar 1: yang, open 9.5 < prev close 9.8 ✓\n        //        close 11.0 > prev open 10.0 ✓\n        // Need ATR warmup → use longer:\n''',
        '''    fn test_rising_sun() {\n        // Use a full warmup series so the fixture exercises the real ATR path.\n''',
        "shadowed rising-sun fixtures",
    )

    # The meeting-lines test only asserts the output shape, so discard the many
    # exploratory fixtures that are shadowed before the actual call.
    path = ROOT / "core/src/patterns/astock_kline2.rs"
    text = path.read_text(encoding="utf-8")
    start = text.index("    #[test]\n    fn test_meeting_lines() {")
    end = text.index("\n    #[test]\n    fn test_rising_three_methods()", start)
    replacement = '''    #[test]\n    fn test_meeting_lines() {\n        let (o, h, l, c, _) = flat_synth(5);\n        let r = meeting_lines(&o, &h, &l, &c).unwrap();\n        assert_eq!(r.len(), 5);\n    }\n'''
    path.write_text(text[:start] + replacement + text[end:], encoding="utf-8")
    print("fixed shadowed meeting-lines fixtures")

    replace_once(
        "core/src/patterns/astock_kline2.rs",
        "        let r = rising_three_methods(&o, &h, &l, &c).unwrap();\n",
        "",
        "unused first rising-three-methods result",
    )
    replace_once(
        "core/src/patterns/astock_kline2.rs",
        "        let v: Vec<f64> = vec![];\n",
        "",
        "unused validation vector",
    )

    replace_once(
        "core/src/math/simd_ops_wasm.rs",
        '''    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {\n        (a - b).abs() < tol\n    }\n\n''',
        "",
        "dead wasm approx_eq test helper",
    )

    replace_once(
        "core/benches/competitive_bench.rs",
        "indicators::stoch_rsi(&close, 14, 14, 3, 3)",
        "indicators::stochrsi(&close, 14, 14, 3, 3)",
        "deprecated stoch_rsi benchmark call",
    )

    replace_once(
        "core/benches/simd_statistics_bench.rs",
        '''fn generate_benchmark_data(len: usize) -> Vec<f64> {\n    (0..len)\n        .map(|i| {\n            let t = i as f64;\n            1000.0 + t * 0.05 + (t * 0.23).sin() * 50.0 + (t * 0.87).cos() * 30.0\n        })\n        .collect()\n}\n\n''',
        "",
        "dead benchmark data generator",
    )
    replace_once(
        "core/benches/simd_statistics_bench.rs",
        "        let mut result_scalar = vec![0.0; DATA_LEN];\n",
        "",
        "unused linear-reg scalar result buffer",
    )

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
