# Finkit v0.1.5 vs TA-Lib expanded benchmark results

> Run date: 2026-09-09
> Finkit: locally built `0.1.5` CPython ABI3 wheel
> TA-Lib Python: `0.6.8`
> Host: Windows x86_64, Python 3.13, NumPy 2.5.3

## Matrix summary

The current gate exercises 155 TA-Lib-compatible functions at 100K and 1M
values, for 310 timed observations. All calls completed without runtime
errors. Finkit was faster on 307 observations and the geometric-mean speedup
was **2.03x**.

Two existing `HT_TRENDMODE` observations were marked as parity exceptions due
to the valid-mask/boolean contract. Their absolute and relative numeric
differences were both zero. This is the same known Hilbert issue covered by the
existing tests and is not caused by the v0.1.5 screening or packaging work.

The three slower observations were:

| Function | Size | Finkit / TA-Lib |
| --- | ---: | ---: |
| `TRANGE` | 1M | 0.985x |
| `CDLLONGLINE` | 1M | 0.954x |
| `CDLSHORTLINE` | 1M | 0.857x |

Ratios near `1.0x` should be repeated on the target machine before being used
as an optimization decision.

## Coverage beyond TA-Lib

Finkit now also exposes cross-market screening primitives for A-shares, Hong
Kong stocks, US equities, ETFs, futures, and crypto bars:

- `GOLDEN_CROSS` / `DEAD_CROSS`;
- `BREAKOUT` / `BREAKDOWN`;
- `VOLUME_SURGE`;
- `MA_ALIGN`;
- `RELATIVE_STRENGTH`;
- `GAP_SIGNAL`;
- `TREND_BREAKOUT`.

These are not placed in the TA-Lib denominator because TA-Lib does not provide
an equivalent unified selection API. Their correctness and parameter contract
are documented in [screening-formulas.md](screening-formulas.md).

## Reproduction

```bash
maturin build --release --features abi3 --strip --out dist/python/current
python -m pip install --force-reinstall --no-deps dist/python/current/finkit-*.whl
python scripts/benchmark_talib_all_current_gate.py \
  --sizes 100000 1000000 \
  --output dist/bench/talib-all-current-gate.json
```

The full per-function JSON is intentionally kept as a local/CI artifact. The
committed report records the release conclusion and the exceptions so that the
numbers remain auditable without treating one host as a universal benchmark.
