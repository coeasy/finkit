# Benchmark results

The v0.1.5 release-wheel gate compared Finkit with TA-Lib Python `0.6.8` on
155 compatible functions at 100K and 1M values (310 observations). Finkit was
faster on 307 observations with a geometric-mean speedup of **2.03x**.

There were no runtime errors. The two existing `HT_TRENDMODE` mask flags are
documented parity exceptions; their numerical differences were zero. Three
individual observations were slightly below `1.0x`: `TRANGE`/1M,
`CDLLONGLINE`/1M, and `CDLSHORTLINE`/1M. See the [full report](BENCHMARK_REPORT.md)
for the methodology and reproducible command.

The cross-market screening formulas are a Finkit-native capability rather than
TA-Lib-compatible functions. Their API and selection recipes are documented in
[screening-formulas.md](screening-formulas.md), and their correctness is
covered by focused Rust and formula-router tests.

## Other performance layers

- Streaming indicators use incremental state and are intended for O(1) work per
  bar after initialization.
- Formula execution has a compiled-plan/cache path; benchmark formula runtime
  and native calls separately.
- SIMD and zero-copy claims must be read from the benchmark produced on the
  target CPU; they are not interchangeable with the Python public-wheel gate.

## Reproduce

```bash
python scripts/benchmark_talib_all_current_gate.py \
  --sizes 100000 1000000 \
  --output dist/bench/talib-all-current-gate.json
```
