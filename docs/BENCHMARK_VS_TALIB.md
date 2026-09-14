# Finkit vs TA-Lib — Benchmark & Precision Report

> Companion to the one-click build setup (see the repo root `README.md`
> for the bootstrap command). This document explains how to read the
> outputs of `scripts/bench-vs-talib.sh` and what to do when a regression
> appears.

## Where the data comes from

`scripts/bench-vs-talib.sh` is the single command that runs the entire
head-to-head:

1. **`python scripts/benchmark_talib_all_current_gate.py`** — runs the
   release-wheel gate across the complete TA-Lib-compatible matrix at 100K and
   1M values. The current v0.1.5 snapshot covers 155 cases and 310 observations.

2. The JSON output records per-function timings, geometric mean, parity masks,
   and errors. `docs/BENCHMARK_REPORT.md` is the human-readable release
   snapshot; it is not a claim about every CPU or every individual function.

3. **`scripts/bench_vs_talib_precision.py`** *(optional)* —
   for each indicator, generates 100 000 random OHLCV samples, calls
   Finkit and TA-Lib on the same input, then writes
   `dist/bench/precision.{md,json}` and merges `delta_pp` back into
   `dist/bench/results.json`.

4. **Summary renderer** — `dist/bench/summary.md` is a compact table
   that joins speedup with `delta_pp`. The single file to read first.

## Reading the summary

```
| Indicator | Category | Finkit (us) | TA-Lib C (us) | Speedup | Δ (pp)  | Status |
| SMA_20    | Overlap  | 0.83        | 1.12          | 1.35x   | 2.0e-12 | ✅     |
| ATR_14    | Volat.   | 1.10        | 0.78          | 0.71x   | 4.1e-13 | ❌     |
```

| Column      | What to look for                                              |
| ----------- | ------------------------------------------------------------- |
| `Finkit (us)` / `TA-Lib C (us)` | Wall-clock per call (microseconds, smaller = better) |
| `Speedup`   | `TA-Lib C / Finkit` (higher = Finkit is faster). `>1.0x` is good |
| `Δ (pp)`    | Max relative diff vs TA-Lib output (precision SLA: < 1e-12)   |
| `Status`    | `✅` within gate, `⚠️` borderline/noisy, `❌` needs optimization |

Bottom of the file summarizes:

The current release-wheel snapshot is **307 / 310 faster**, with geometric
mean **2.03x**. The three slower observations are documented in
[BENCHMARK_REPORT.md](BENCHMARK_REPORT.md).

## SLA gates

| Gate                | Threshold              | What it means                                |
| ------------------- | ---------------------- | -------------------------------------------- |
| Speed gate          | Finkit ≤ 1.25 × TA-Lib | Listed in the `Watch List` if exceeded       |
| Speed gate (hard)   | Finkit ≤ 1.0 × TA-Lib  | Listed under `Needs Optimization`            |
| Regression gate     | Finkit ≤ 1.05 × baseline (`docs/benchmark-baseline.json`) | Catches local regressions vs committed numbers |
| 1M ns/bar SLA       | Per-indicator ceiling in `ONE_M_NS_BAR_SLA` (in `bench_report.py`) | Catches O(n²) algorithms scaling poorly at 1M bars |
| Precision SLA       | `max_abs < 1e-9` and `max_rel < 1e-12` (default) | Catches algorithm divergence in the parity check |

## When a regression appears

1. **Speed regression** (`Speedup < 1.0x` and `Status == ❌`):
   * Re-run with `--bench-filter <indicator>` to isolate the noise.
   * Compare to `docs/benchmark-baseline.json` to confirm it's not noise.
   * If reproducible, profile with `cargo flamegraph --bench talib_c_comparison`.

2. **Precision regression** (`Δ (pp) > 1e-10`):
   * Run `python scripts/bench_vs_talib_precision.py --exit-on-fail`.
   * Check the per-array breakdown in `dist/bench/precision.md`.
   * The aggregate picks the **worst** of the component arrays; if a
     single sub-output (e.g. `MACD.hist`) regresses, the whole row flags.

3. **Both at once** — the most common cause is a forgotten `py.allow_threads`
   wrapper in the Python binding or a SIMD lane misalignment. Re-run
   `cargo test -p finkit` and inspect the failing parity tests.

## Reproducing the numbers

```bash
# Same hardware, same commit and wheel
python scripts/benchmark_talib_all_current_gate.py \
  --sizes 100000 1000000 \
  --output dist/bench/talib-all-current-gate.json

# Cross-machine comparison
./scripts/bench-vs-talib.sh --precision 2>&1 | tee /tmp/$(hostname).log
```

For machine-to-machine comparisons, use the JSON:

```bash
jq '.benchmarks | to_entries
   | map({k:.key, v:.value.speedup}) | sort_by(-.v)' \
   dist/bench/results.json
```

## Known precision caveats

| Indicator group        | Expected `max_rel` | Why                                                                  |
| ---------------------- | ------------------ | -------------------------------------------------------------------- |
| SMA / EMA / WMA        | 0                  | O(1) update identical to TA-Lib's O(1) update                        |
| RSI                    | 0                  | Wilder smoothing is identical                                        |
| MACD (line / signal)   | ~1e-15             | EMA of EMA introduces one extra rounding step                        |
| MACD (hist)            | ~1e-13             | hist = line - signal amplifies the EMA error                         |
| BBANDS                 | ~1e-13             | Popvar uses two-pass; Welford uses one-pass; ~1 ULP drift            |
| ATR                    | ~1e-13             | Wilder smoothing on TR                                               |
| ADX                    | ~1e-10             | DM smoothing uses RMA inside Wilder; sign of `+DM` vs `-DM`          |
| STOCH (slowk)          | ~1e-13             | SMA of raw %K                                                        |
| STOCH (slowd)          | ~1e-12             | SMA of slowK                                                         |
| OBV                    | 0                  | Pure cumulative sum                                                  |
| Hilbert Transform      | ~1e-10             | Internal accumulator precision; within tolerance                      |

If a row drifts outside its expected range, file an issue with the
`dist/bench/precision.json` and the failing commit.

## CI integration

There is no dedicated weekly CI workflow for the precision comparison.
Run the weekly precision check locally with:

```bash
./scripts/bench-vs-talib.sh --precision
```

A `❌` in `summary.md` (or a precision row > 1e-9) indicates a regression.

## See also

* [BENCHMARK_REPORT.md](BENCHMARK_REPORT.md) — the long-form report
  (auto-generated, refreshed on every bench run).
* [Benchmark results](benchmark-results.md) — concise performance summary.
* `EFFICIENCY_COMPARISON.md` — broader ecosystem comparison (planned;
  not yet published).
