# Finkit vs TA-Lib C — Benchmark & Precision Contract

> This document defines how Finkit compares against TA-Lib C without turning one benchmark machine into a universal marketing claim. For the broader ecosystem strategy, see [competitive-positioning-zh.md](competitive-positioning-zh.md).

## Why TA-Lib remains the primary numerical baseline

TA-Lib is a long-lived production technical-analysis library with a C/C++ core, broad language support, a native Rust implementation path, and an expanding streaming API. Finkit therefore treats TA-Lib as both:

- a **numerical compatibility reference** for overlapping indicators; and
- a **direct native-performance competitor** for equivalent batch workloads.

The comparison target used by the scheduled workflow is **TA-Lib 0.7.1**. When another version is used locally, that version must appear in `dist/bench/environment.json`.

Official reference: <https://ta-lib.org/>.

## One-command local run

```bash
./scripts/bench-vs-talib.sh --precision
```

The script performs these stages:

1. detect or install TA-Lib C;
2. record commit/platform/compiler/TA-Lib metadata;
3. run `cargo bench -p finkit --bench talib_c_comparison --features talib-c --locked`;
4. parse Criterion results with `scripts/bench_report.py`;
5. render a compact paired summary;
6. optionally run the Python precision comparison.

Outputs under `dist/bench/`:

- `environment.json` — machine and toolchain evidence;
- `results.json` — versioned machine-readable benchmark schema;
- `summary.md` — compact paired table;
- `finkit-vs-talib.md` — long-form Criterion report;
- `precision.json` / `precision.md` when `--precision` is enabled.

## Evidence contract

A performance statement is valid only when all of the following are known:

- exact Finkit commit SHA;
- clean/dirty working-tree state;
- CPU architecture / machine class;
- Rust compiler and Cargo version;
- build profile and feature flags;
- TA-Lib version;
- dataset size and parameters;
- benchmark harness version;
- paired Finkit and TA-Lib rows from the same run.

If paired rows are missing, the run is invalid. `bench_report.py --require-pairs N` exists specifically to prevent empty-report success.

## Reading the summary

```text
| Indicator | Category | Finkit (us) | TA-Lib C (us) | Speedup | Delta | Status |
| SMA_20    | Overlap  | 0.83        | 1.12          | 1.35x   | ...   | OK     |
```

`Speedup = TA-Lib time / Finkit time`:

- `> 1.0x`: Finkit is faster on that measured workload;
- `= 1.0x`: effectively tied at the point estimate;
- `< 1.0x`: Finkit is slower and belongs on the optimization watchlist.

The reporter's display status currently uses:

- `✅`: Finkit <= TA-Lib;
- `⚠️`: Finkit is slower but within 25%;
- `❌`: Finkit is more than 25% slower.

The 25% boundary is a **severe-regression guardrail**, not the end goal. For the canonical hot-path watchlist, the optimization target is `speedup >= 1.0x`.

## Precision before speed

Performance is only comparable after semantics are aligned. The precision stage uses identical OHLCV inputs and verifies overlapping outputs.

Default parity expectations are governed by the test/precision harness, including:

- warm-up position and output alignment;
- NaN handling;
- multi-output arrays independently;
- absolute and relative numerical tolerances;
- no silent row filtering.

A faster implementation with incompatible output semantics is not counted as a competitive win.

## CI and scheduled competitor evidence

Normal PR CI protects correctness and implementation regressions:

```bash
cargo test -p finkit --locked
cargo test -p finkit --test memory_regression --release --locked -- --test-threads=1
cargo test -p finkit --test performance_regression --release --locked -- --test-threads=1
cargo bench -p finkit --no-run --locked
```

The dedicated `.github/workflows/competitive-benchmark.yml` runs weekly and manually:

1. installs TA-Lib 0.7.1;
2. runs the paired Criterion suite;
3. requires a non-empty benchmark set;
4. blocks >25% competitor regressions;
5. uploads report + JSON + environment evidence.

This separation is intentional: PR CI must stay reliable and fast enough for development, while direct native competitor measurements run in a reproducible dedicated workflow.

## Performance gates

| Gate | Contract | Purpose |
| --- | --- | --- |
| Numerical parity | per-indicator tolerance | reject semantic drift |
| `_into` allocation | 0 hot-path heap allocations where contracted | reject hidden allocation regressions |
| Algorithmic regression | optimized path retains relative advantage | catch O(n × period) fallback |
| HT_SINE throughput | `< 1000 ns/bar`, release + one test thread | preserve existing whole-function budget without debug-runner jitter |
| TA-Lib severe guardrail | Finkit <= 1.25 × TA-Lib | weekly competitor regression protection |
| TA-Lib superiority target | Finkit <= TA-Lib | ongoing canonical hot-path goal |
| Historical baseline | script-specific committed baseline | detect same-project regressions |

## Historical checked-in snapshot

`BENCHMARK_REPORT.md` currently contains a **2026-06-24 Windows x86_64 AVX2** snapshot. It is useful as historical evidence, not as a promise about every current CPU or commit.

Any new product claim should prefer a freshly generated artifact from the current head.

## Diagnosing a slow row

When `speedup < 1.0x`:

1. rerun only that indicator with the same input size;
2. confirm parity first;
3. inspect allocation count and caller-owned `_into` availability;
4. profile kernel vs wrapper/FFI time separately;
5. look for repeated parsing, dependency discovery, temporary buffers, and duplicated intermediate series;
6. compare small, medium, and 1M-row scaling before changing the algorithm;
7. update the watchlist only after the same implementation passes correctness tests.

Typical optimization classes:

- O(1) rolling state instead of window re-scan;
- shared TR / DM / EMA / rolling-stat intermediates;
- caller-owned output buffers;
- borrowed input instead of copies;
- SIMD only when its setup cost wins at the tested scale;
- persistent plan / scratch reuse for repeated workloads.

## Beyond single indicators

TA-Lib head-to-head covers only one part of Finkit's product value. The competitive suite must also grow around:

- Formula parse+execute vs compile-once+execute-many;
- Factor discovery vs precompiled `FactorPlan`;
- full vs DirtyRange range/range-into recomputation;
- compute-many shared intermediates;
- Python / Node / C ABI end-to-end overhead;
- research-pipeline workloads.

These workloads are where Finkit's Unified Runtime can create a structural advantage that a single isolated indicator benchmark cannot demonstrate.

## Claim rules

Allowed:

> On benchmark artifact X, Finkit SMA/RSI/etc. was Yx faster than TA-Lib 0.7.1 on the recorded runner.

Allowed:

> Finkit has a weekly TA-Lib paired benchmark guardrail and a stricter long-term target of matching or beating TA-Lib on canonical hot paths.

Not allowed without broader evidence:

> Finkit is universally faster than TA-Lib.

Not allowed:

> Finkit is the fastest quantitative library.

Performance superiority must remain a reproducible result, not a permanent adjective.
