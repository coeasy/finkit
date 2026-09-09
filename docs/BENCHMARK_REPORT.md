# Finkit v0.1.5 vs TA-Lib benchmark report

> This report records the release-wheel validation run on 2026-09-09. The
> result is a local machine snapshot, not a universal latency guarantee.

## Executive summary

The current Python ABI3 wheel was compared with TA-Lib Python `0.6.8` across
155 TA-Lib-compatible functions, two input sizes, and 310 observations:

| Measure | Result |
| --- | ---: |
| Input sizes | 100,000 and 1,000,000 values |
| Compatible function cases | 155 |
| Total observations | 310 |
| Finkit faster | 307 / 310 |
| Geometric-mean speedup | **2.03x** |
| Runtime errors | 0 |
| Known parity exceptions | 2 (`HT_TRENDMODE`, existing) |

The current run supports the release goal that most compatible functions
outperform TA-Lib on this host. It does not support claiming that every
individual observation is faster. The three slower observations were `TRANGE`
at 1M bars (`0.985x`), `CDLLONGLINE` at 1M (`0.954x`), and `CDLSHORTLINE`
at 1M (`0.857x`).

## Parity status

All functions completed without runtime errors. The only parity flags were the
two existing `HT_TRENDMODE` observations at 100K and 1M bars. The reported
absolute and relative differences were both zero; the flag is caused by the
boolean/valid-mask contract rather than a numerical value difference. This is
the same known issue tracked by the existing Hilbert tests and is independent
of the v0.1.5 documentation, packaging, and screening changes.

## Representative speedups

The following values are from the same 100K/1M run and show the intended
performance profile:

| Function family | Representative result |
| --- | ---: |
| `EMA` | about 2.29x at 100K; 1.52x at 1M |
| `ATR` | about 2.30x at 100K; 2.45x at 1M |
| `MACD` | about 1.63x at 100K; 1.97x at 1M |
| `HT_SINE` | about 3.77x at 100K; 3.78x at 1M |
| `LINEARREG` | about 3.47x at 100K; 2.85x at 1M |
| `COSH` / `EXP` | about 4.34x / 2.56x at 100K; 9.93x / 9.95x at 1M |

## Scope of the new screening layer

`GOLDEN_CROSS`, `DEAD_CROSS`, `BREAKOUT`, `BREAKDOWN`, `VOLUME_SURGE`,
`MA_ALIGN`, `RELATIVE_STRENGTH`, `GAP_SIGNAL`, and `TREND_BREAKOUT` are
Finkit-native selection primitives. TA-Lib has no equivalent unified
cross-market screening API, so they are not included in the 155-function parity
denominator. Their validation contract covers warm-up, NaN, equal-length,
look-ahead exclusion, and parameter checks; see
[screening-formulas.md](screening-formulas.md).

## Reproduce

Build the wheel and run the complete current gate:

```bash
maturin build --release --features abi3 --strip --out dist/python/current
python -m pip install --force-reinstall --no-deps dist/python/current/finkit-*.whl
python scripts/benchmark_talib_all_current_gate.py \
  --sizes 100000 1000000 \
  --output dist/bench/talib-all-current-gate.json
```

The JSON output contains every function, size, timing, parity mask, and error.
Use the geometric mean and the per-observation rows together; a single noisy
short function should not be presented as the overall result.

## Interpretation rules

1. Compare Finkit and TA-Lib in the same process, Python runtime, input data,
   and warm-up policy.
2. Report parity separately from speed. A faster result with different
   warm-up semantics is not an equivalent implementation.
3. Repeat borderline ratios near `1.0x` before treating them as a regression.
4. Keep public-wheel results separate from Rust in-process microbenchmarks.
