# Formula Engine Performance Benchmark

## Efficiency optimization plan (2026-11-05)

The latest complete 60-formula workload is recorded in
`dist/bench/formula-mode-workload-20261030-obv-ma-fastpath-repeat.txt`. It
shows that short-history Plan execution is slower than Tree in aggregate
(1.53x domestic and 1.65x Pine, where ratios are Tree/Plan), while 5K-row
execution is faster (0.93x domestic and 0.79x Pine). The largest short-history
gaps include domestic ZLEMA, COPPOCK, NVI/PVI, VORTEX and WAD, and Pine OBV,
VWAP, MOMENTUM, VWMA, TRIX, ROC, SMA overlay, RSI, WILLIAMS_R and ATR.

The work is ordered as follows:

1. Compare the slowest formula shapes with their existing direct kernels and
   executor dispatch. Keep parity-safe specialized execution ahead of general
   plan dispatch, and retain the current generic fallback for unsupported data
   or kernel errors.
2. Route single-output ATR and COPPOCK plans into their already compiled direct
   kernels. The primary-only dispatcher previously intercepted these shapes
   first, making the direct branches unreachable in ordinary no-details runs.
3. Reuse the original parameter range when scalar arithmetic is fused, rather
   than duplicating the same literal in the hot-plan parameter arena.
4. Recognize the checked-in Pine TRIX plot/zero-line shape and execute its
   direct TRIX kernel while retaining the same output and scalar variables.
5. Preserve output publication and sandbox accounting around the direct paths;
   avoid changing indicator arithmetic or NaN handling in this pass.
6. After all code changes are complete, run one unified formula workload and
   parity suite. Collect failures, repair them together, and rerun the complete
   suite before accepting the change. Compare warm native and 5K results with
   the recorded baseline, with special attention to the listed slow shapes.

This pass addresses dispatch overhead where an existing direct kernel was
being bypassed. The per-shape Pine and domestic rankings remain targets for
the unified measurements; they are not assumed to improve until that run
reports them.

### 2026-11-05 validation result

The final release workload remained non-degenerate and Plan/Tree consistent on
all **60/60** formulas at both 250 and 5K rows. At 250 rows, warm Plan/Tree
ratios were **1.520x domestic** and **1.548x Pine** (Plan is slower). At 5K,
they were **0.934x domestic** and **0.782x Pine** (Plan is faster). Warm
allocations averaged **15.1 vs 40.7** per domestic evaluation and **20.5 vs
101.3** per Pine evaluation (Plan vs Tree). The Pine TRIX case measured **13.4
µs Plan / 4.7 µs Tree** in the final run. Evidence:
`dist/bench/formula-mode-workload-20261105-final.txt`.

The data confirms a long-history advantage and lower allocation counts, but the
short-history aggregate target remains open. These results do not establish
that formula execution is faster than TA-Lib overall.

### 2026-11-05 follow-up dispatch audit

The latest 60-formula report still shows the largest 250-row Plan/Tree gaps in
Pine VWAP/TRIX/OBV/ATR/VWMA/RSI/Momentum and domestic ZLEMA/COPPOCK/OBV/WAD/
VORTEX/NVI-PVI. A root-MA dispatch experiment was reverted: the stage probe did
not establish an end-to-end benefit over the existing primary executor. The
retained Plan-versus-Tree regression coverage checks root MA output and Pine
TRIX output plus published context.

The final-code workload remained **60/60** consistent. At 250 rows, warm
Plan/Tree was **1.526x domestic / 1.609x Pine**; at 5K it was **0.934x / 0.784x**.
Pine TRIX measured **12.4 µs Plan / 4.5 µs Tree**. A primary-only TRIX direct
publication experiment also stayed **60/60** consistent, but TRIX measured
**13.1 µs Plan / 4.6 µs Tree**, so the experiment was removed. Timing variation
in the aggregate does not establish a short-history gain. The Python TA-Lib
wheel gate remains unavailable in the current Windows environment. Evidence:
`dist/bench/formula-mode-workload-20261105-final-audit.txt`,
`dist/bench/formula-plan-stage-20261105-final-audit.txt`, and
`dist/bench/formula-mode-workload-20261105-trix-direct-publish.txt`.

This document provides comprehensive performance benchmarks for the Formula Engine, comparing different execution modes and measuring performance across various formula types.

## Table of Contents

- [Overview](#overview)
- [Test Environment](#test-environment)
- [Formula vs Native Indicators](#formula-vs-native-indicators)
- [Bytecode vs AST Interpretation](#bytecode-vs-ast-interpretation)
- [SIMD Optimization Results](#simd-optimization-results)
- [Data Size Scaling](#data-size-scaling)
- [Individual Function Performance](#individual-function-performance)
- [Optimization Impact](#optimization-impact)
- [Memory Usage](#memory-usage)
- [Scaling Analysis](#scaling-analysis)
- [API Performance Comparison](#api-performance-comparison)
- [Recommendations](#recommendations)

## Overview

The Formula Engine supports three execution modes:

1. **AST Interpretation** - Default mode, traverses the AST tree directly
2. **Bytecode Compilation** - Compiles to bytecode VM instructions before execution
3. **Optimized Execution** - Applies constant folding, dead code elimination, and common subexpression elimination before bytecode compilation

### Performance Summary

| Execution Mode | Relative Speed | Best Use Case |
|---------------|----------------|---------------|
| AST Interpretation | 1x (baseline) | Quick one-off evaluations |
| Bytecode Compilation | 1.5-2x for complex formulas | Repeated formula execution |
| Optimized Execution | 1.1-1.5x faster than AST | Production systems |
| Builtin Function Mapping | Near-native (1.0-1.3x native) | Direct indicator calculation |

## Backend Choice: Tree vs Plan

The table above compares *interpretation strategies within the tree backend*
(AST / bytecode / optimized). It does not cover the orthogonal axis:
`FormulaExecutionMode::{Tree, Plan}`, which selects between the AST tree-walker
and the compiled-plan executor. That axis is measured separately, over the real
checked-in corpora, by:

```bash
cargo run -p finkit --release --example formula_mode_workload_probe -- --scaled
```

Measured 2026-10-13 over 60 corpus cases (35 domestic terminal formulas + 25
Pine scripts), warm (cached) execution, 250 and 5000 bars. Ratios are
`Plan / Tree`; below 1.0 means Plan is faster.

| Corpus | Bars | warm time | warm allocations |
|---|---:|---:|---:|
| domestic | 250 | 1.84x (Plan slower) | 1.39x |
| domestic | 5000 | 1.27x (Plan slower) | 1.39x |
| Pine | 250 | **0.25x (Plan 4x faster)** | **0.089x** |
| Pine | 5000 | **0.49x (Plan 2x faster)** | **0.089x** |

The domestic 5000-bar and allocation figures improved on the same day from
1.53x and 1.54x after fixing a hotspot in the `CALL:` compatibility bridge:
each bridged node built a full-length `FormulaContext` — five `len`-sized
zero-filled arrays — although all 57 functions behind the bridge read only
`data_len`. At 5000 bars that was ~200 KB of pointless memory traffic per call,
which dominated call-heavy formulas (`smi_cross` measured 4.36x before, 2.99x
after). The 250-bar tier is unchanged, as expected: the removed cost is per
*call*, so it only matters once `len` is large.

Two things drive the split, and neither is obvious from the numbers alone:

- **Pine previously favoured Plan heavily** because `eval_with_dialect`'s tree
  branch re-parsed and re-mapped Pine on every call. Tree mode now caches parsed
  Pine ASTs by normalized source, so the warm comparison measures execution
  rather than repeated parsing.
- **Domestic favours Tree slightly** because its tree branch caches the compiled
  formula and the plan executor's per-node dispatch costs more than an AST walk
  at these sizes.

Allocation counts do not scale with bar count (they are per-call, not per-bar).
The same probe also reports coverage and Tree/Plan consistency, and flags any
case where both backends agree while computing nothing.

### Follow-up: Plan dispatch and scratch reuse (2026-10-28)

The release workload probe was rerun over the same 60 cases before and after
three Plan-path changes: canonical kernel inputs are moved into temporary
ndarrays and restored instead of copied; canonical outputs transfer their
allocation into the Plan slot; and `FormulaEngine` retains its bounded scratch
arena across Plan evaluations while clearing per-run streaming state. The
common kernel argument table (up to eight inputs) now stays on the stack.

| Corpus | Bars | Before warm Plan/Tree | After warm Plan/Tree (two runs) | Allocations, Tree → Plan after |
|---|---:|---:|---:|---:|
| domestic | 5000 | 1.247x | 1.115x / 1.136x | 42.8 → 50.8 |
| Pine | 5000 | 0.471x | 0.469x / 0.472x | 904.0 → 82.6 |

The domestic Plan/Tree time ratio improved by about 9–11% across the two final
runs, and Plan allocations fell from 59.9 to 50.8 per execution (about 15%).
All 35 domestic and 25 Pine cases remained numerically consistent in each
probe. The follow-up below further reduces allocations on the primary-result
path.

### Follow-up: Primary-only result extraction (2026-10-28)

`eval_plan` and `eval_plan_with_dialect` return only the primary series, but
previously built all detailed channel and variable arrays, copied those arrays
into `FormulaContext`, then dropped the detail result. The primary-only path now
moves unique retained variable buffers directly into the context; only the
primary result and repeated aliases need copies. The public
`eval_plan_channels` path still returns the full details.

| Corpus | Bars | Previous Plan/Tree warm | New Plan/Tree warm (two runs) | Allocations, Tree → Plan now |
|---|---:|---:|---:|---:|
| domestic | 5000 | 1.136x | 1.065x / 1.057x | 42.8 → 42.5 |
| Pine | 5000 | 0.481x | 0.441x / 0.437x | 904.0 → 62.4 |

The input-slot vector was also removed from the per-run path: compiled bindings
are ordered by slot, so execution can build the input references directly
instead of allocating an `Option` slot vector and then a second input vector.
Across two more release runs this brought 5000-bar domestic Plan/Tree warm to
1.047x / 1.048x, with 41.5 allocations (Tree: 42.8). Pine measured 0.422x /
0.426x warm with 61.4 allocations (Tree: 904.0). Domestic cold measured
1.221x / 1.235x, so Plan remains slower cold and the overall formula-efficiency
goal is still open. All 60 cases remained non-degenerate and numerically
consistent. The latest captured warm hotspots were `dialect_ohlc_aliases`
(about 2x) and `cross_period_refdate` (about 1.5x).

The binary dispatcher now selects the operator once before entering the
element loop, so arithmetic, comparison, and logical kernels no longer match
the operator for every element. A release probe after this change kept all 60
cases consistent. Two 5000-bar release runs measured domestic warm Plan/Tree at
1.020x / 1.032x with 38.8 Plan allocations (Tree: 42.8); Pine measured
0.413x / 0.404x with 56.2 Plan allocations (Tree: 904.0). Domestic cold
remained slower at 1.218x / 1.221x, and Pine cold measured 1.411x / 1.420x.
The domestic warm result is near parity but not yet a
repeatable Plan advantage; the overall formula-efficiency goal remains open.

After the candlestick kernel work, the same 60-case scaled release probe still
reported 60/60 non-degenerate Plan/Tree matches. Domestic 5000-bar warm
Plan/Tree was 1.027x with 38.8 allocations (Tree: 42.8); Pine was 0.407x with
56.2 allocations (Tree: 904.0). Domestic cold was 1.224x and Pine cold was
1.409x. This confirms that the indicator changes did not regress the formula
workload; domestic warm and cold Plan execution still need work.

The final 5000-bar rerun on the full candle patch remained 60/60 consistent:
domestic warm Plan/Tree measured 1.056x with 38.8 allocations (Tree: 42.8),
and Pine warm measured 0.408x with 56.2 allocations (Tree: 904.0). Domestic
cold remained 1.222x and Pine cold 1.412x. This keeps the domestic Plan path
near parity but does not establish a repeatable Plan lead.

### Follow-up: Cache warm Pine ASTs (2026-10-28)

`FormulaEngine::eval_with_dialect` and `eval_multi_with_dialect` now reuse a
bounded per-engine Pine AST cache keyed by normalized formula source. Cache
entries are cleared with the formula caches and when custom formula
registration changes. The 5000-bar release workload remained 60/60
non-degenerate and consistent. Pine Tree warm allocations fell from **904.0**
to **101.3** per evaluation; the Plan path used **56.2**. Plan/Tree warm moved
from **0.408x** to **0.934x** because the Tree baseline no longer reparses every
call. Domestic warm Plan/Tree was **1.028x** with 38.8 vs 42.8 allocations;
domestic and Pine cold Plan/Tree remained **1.222x** and **1.415x**. Probe log:
`dist/bench/formula-mode-workload-pine-ast-cache-20261028.txt`.

The next run also borrowed already-clean source text in dialect-aware calls,
allocating normalized text only when the input has a BOM or carriage returns.
All 60 cases still matched. Domestic warm Plan/Tree was **1.032x**; average
allocations fell **42.8 → 40.8** for Tree and **38.8 → 36.8** for Plan. Pine
warm Plan/Tree was **0.936x** with unchanged 101.3/56.2 allocation counts;
cold Plan/Tree was **1.221x** domestic and **1.421x** Pine. Probe log:
`dist/bench/formula-mode-workload-pine-cache-cow-20261028.txt`.

## Test Environment

- **CPU**: Intel Core i7-12700K (12 cores, 20 threads)
- **RAM**: 32 GB DDR4-3200
- **OS**: Windows 11 / Linux 5.15
- **Rust Version**: 1.87+
- **Data Size**: 1000 / 10000 / 100000 bars (OHLCV)
- **Iterations**: 1000 per test

## Formula vs Native Indicators

This section compares the performance of executing indicators through the formula engine versus calling the native Rust implementation directly.

### 1000 data points

| Method | SMA(20) | EMA(12) | RSI(14) | MACD(12,26,9) | BOLL(20,2) |
|--------|---------|---------|---------|---------------|------------|
| Native | 5.63µs | 1.96µs | 5.02µs | 6.68µs | 23.05µs |
| Formula (direct) | 3.38µs | 3.47µs | 4.20µs | 11.62µs | 14.86µs |
| Formula (builtin RSI/MACD/BOLL) | - | - | 6.41µs | 6.04µs | 7.02µs |

### 10000 data points

| Method | SMA(20) | EMA(12) | RSI(14) | MACD(12,26,9) | BOLL(20,2) |
|--------|---------|---------|---------|---------------|------------|
| Native | 58.73µs | 20.46µs | 50.95µs | 68.50µs | 187.69µs |
| Formula (direct) | 25.58µs | 33.12µs | 40.42µs | 118.25µs | 151.52µs |
| Formula (builtin) | - | - | 40.46µs | 53.06µs | 70.24µs |

### 100000 data points

| Method | SMA(20) | EMA(12) | RSI(14) | MACD(12,26,9) | BOLL(20,2) |
|--------|---------|---------|---------|---------------|------------|
| Native | 587.99µs | 207.23µs | 553.91µs | 830.72µs | 2074.0µs |
| Formula (direct) | 465.11µs | 526.71µs | 595.93µs | 2324.2µs | 2880.0µs |
| Formula (builtin) | - | - | 611.82µs | 1060.8µs | 1359.8µs |

### Key Findings

- Formula engine's built-in function mapping (RSI/MACD/BOLL builtin) achieves near-native performance
- For SMA, the formula engine is actually faster than the naive native implementation due to optimized Rust code
- For complex multi-step indicators (MACD via formula composition), there's overhead from intermediate array allocation
- Using builtin function mapping (MACD() function directly) brings performance close to native

## Bytecode vs AST Interpretation

This section provides a detailed comparison between bytecode compilation and AST interpretation.

### Execution Pipeline Comparison

**AST Interpretation:**
```
Source -> Parser -> AST -> Evaluator -> Result
```
- Each evaluation traverses the AST tree
- Dynamic type checking at each node
- No compilation overhead
- Best for single execution

**Bytecode Compilation:**
```
Source -> Parser -> AST -> Bytecode Compiler -> VM -> Result
```
- One-time compilation to bytecode
- VM executes bytecode with minimal overhead
- Type checking done at compile time
- Best for repeated execution

### Execution Modes (1000 data points, MACD formula)

| Mode | Time |
|------|------|
| AST interpreter | 68.85µs |
| Bytecode VM | 107.21µs |
| Optimized AST | 77.74µs |
| JIT optimized | 76.91µs |

> **Note**: For this benchmark, AST interpretation is actually faster than bytecode because the formula is simple enough. Bytecode excels with repeated execution where compilation cost is amortized.

### Compilation Overhead

| Formula Type | Parse (us) | Compile (us) | Total Overhead |
|-------------|------------|--------------|----------------|
| Simple | 45 | 120 | 165 |
| Medium | 120 | 280 | 400 |
| Complex | 180 | 420 | 600 |
| Very Complex | 320 | 650 | 970 |

**Break-even Point**: Bytecode becomes faster after ~5-10 executions (depending on formula complexity).

## SIMD Optimization Results

| Operation | SIMD Time | Scalar Time | Speedup |
|-----------|-----------|-------------|---------|
| SimdOps_add | 41.35µs | 48.17µs | 1.17x faster |
| SimdOps_mul | 42.16µs | 50.32µs | 1.19x faster |
| SimdOps_sma | 202.83µs | 287.66µs | 1.42x faster |

## Data Size Scaling

| Data Points | MA(CLOSE,20) Time |
|-------------|-------------------|
| 100 | 1.30µs |
| 500 | 3.04µs |
| 1000 | 4.40µs |
| 5000 | 19.79µs |
| 10000 | 32.24µs |
| 50000 | 257.23µs |
| 100000 | 545.09µs |

Scaling is linear O(n) with data size.

## Individual Function Performance

| Function | Time (1000 bars) |
|----------|-----------------|
| MA(20) | 35.07µs |
| EMA(12) | 39.17µs |
| SMA(14) | 75.14µs |
| DEMA(20) | 70.99µs |
| TEMA(20) | 77.56µs |
| KAMA(10) | 50.95µs |
| T3(5) | 307.44µs |
| RSI(14) | 48.49µs |
| STOCH(14,3,3) | 165.42µs |
| ADX(14) | 228.38µs |
| DMI(14) | 223.35µs |
| CCI(14) | 266.81µs |
| WILLR(14) | 279.90µs |
| CMO(14) | 407.64µs |
| STD(20) | 47.52µs |
| VAR(20) | 36.75µs |
| CORREL(20) | 2470.9µs |
| BETA(20) | 2074.9µs |
| HISTVOL(20) | 128.71µs |
| JMA(7) | 44.28µs |
| JMA(14) | 44.56µs |

## Optimization Impact

The optimizer applies the following passes:

1. **Constant Folding** - Pre-computes constant expressions
2. **Dead Code Elimination** - Removes unused variable assignments
3. **Common Subexpression Elimination** - Caches repeated calculations
4. **Loop Invariant Code Motion** - Moves loop-invariant calculations outside loops

### Optimization Results by Formula

#### MACD Formula
```
Original: EMA(CLOSE,12) - EMA(CLOSE,26), EMA(DIF,9)
After optimization: Cached EMA(12), EMA(26), shared intermediate results
```

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| AST Nodes | 28 | 18 | 36% reduction |
| Bytecode Instructions | 45 | 32 | 29% reduction |
| Execution Time (ms) | 0.072 | 0.055 | 24% faster |

#### KDJ Formula
```
Original: RSV calculation, SMA for K, SMA for D, J derivation
After optimization: Cached RSV, optimized SMA chains
```

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| AST Nodes | 45 | 30 | 33% reduction |
| Bytecode Instructions | 72 | 48 | 33% reduction |
| Execution Time (ms) | 0.098 | 0.068 | 31% faster |

#### Custom Multi-Indicator Formula
```
Original: MA5+MA10+MA20+RSI+MACD+VOL analysis
After optimization: Shared data, eliminated redundant calculations
```

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| AST Nodes | 120 | 65 | 46% reduction |
| Bytecode Instructions | 180 | 95 | 47% reduction |
| Execution Time (ms) | 0.145 | 0.078 | 46% faster |

## Memory Usage

### Peak Memory by Execution Mode

| Execution Mode | Peak Memory (KB) | Notes |
|---------------|------------------|-------|
| AST Interpretation | 125 | Tree structure in memory |
| Bytecode Compilation | 95 | Compact bytecode + VM state |
| Optimized Execution | 85 | Reduced AST after optimization |

### Memory per 1000 bars

| Data Structure | Memory (KB) |
|---------------|-------------|
| OHLCV Input | 40 (5 arrays x 8 bytes x 1000) |
| Intermediate Variables | 8-64 (depending on formula) |
| Bytecode Program | 2-12 (compact instruction format) |
| VM State | 4-16 (stack + registers) |

## Scaling Analysis

### Performance vs Data Size

| Data Points | AST (ms) | Bytecode (ms) | Optimized (ms) |
|-------------|----------|---------------|----------------|
| 100 | 0.012 | 0.005 | 0.004 |
| 500 | 0.055 | 0.022 | 0.018 |
| 1000 | 0.112 | 0.045 | 0.035 |
| 5000 | 0.558 | 0.225 | 0.175 |
| 10000 | 1.120 | 0.450 | 0.350 |

**Scaling**: All modes scale linearly O(n) with data size.

### Performance vs Formula Complexity

| AST Nodes | AST (ms) | Bytecode (ms) | Optimized (ms) |
|-----------|----------|---------------|----------------|
| 10 | 0.025 | 0.010 | 0.008 |
| 25 | 0.068 | 0.028 | 0.022 |
| 50 | 0.145 | 0.058 | 0.045 |
| 100 | 0.320 | 0.125 | 0.095 |
| 200 | 0.680 | 0.265 | 0.198 |

## API Performance Comparison

### Python Binding

| Function | Overhead vs Native Rust | Notes |
|----------|------------------------|-------|
| `formula_eval()` | +0.15ms | PyO3 conversion overhead |
| `formula_eval_bytecode()` | +0.15ms | Same overhead, faster execution |
| `formula_eval_optimized()` | +0.15ms | Same rewrite as `eval`, no cache reuse |
| `formula_eval_debug()` | +0.25ms | Additional debug info collection |

### Node.js Binding

| Function | Overhead vs Native Rust | Notes |
|----------|------------------------|-------|
| `formulaEval()` | +0.08ms | N-API conversion overhead |
| `formulaEvalBytecode()` | +0.08ms | Same overhead, faster execution |
| `formulaEvalOptimized()` | +0.08ms | Same rewrite as `eval`, no cache reuse |
| `formulaEvalDebug()` | +0.15ms | Additional debug info collection |

### Java Binding

| Function | Overhead vs Native Rust | Notes |
|----------|------------------------|-------|
| `formulaEval()` | +0.12ms | JNI conversion overhead |
| `formulaEvalBytecode()` | +0.12ms | Same overhead, faster execution |
| `formulaEvalOptimized()` | +0.12ms | Same rewrite as `eval`, no cache reuse |
| `formulaEvalDebug()` | +0.20ms | Additional debug info collection |

### Go Binding

| Function | Overhead vs Native Rust | Notes |
|----------|------------------------|-------|
| `FormulaEval()` | +0.10ms | CGO conversion overhead |
| `FormulaEvalBytecode()` | +0.10ms | Same overhead, faster execution |
| `FormulaEvalOptimized()` | +0.10ms | Same rewrite as `eval`, no cache reuse |
| `FormulaEvalDebug()` | +0.18ms | Additional debug info collection |

### .NET Binding

| Function | Overhead vs Native Rust | Notes |
|----------|------------------------|-------|
| `FormulaEval()` | +0.09ms | P/Invoke conversion overhead |
| `FormulaEvalBytecode()` | +0.09ms | Same overhead, faster execution |
| `FormulaEvalOptimized()` | +0.09ms | Same rewrite as `eval`, no cache reuse |
| `FormulaEvalDebug()` | +0.16ms | Additional debug info collection |

### C/C++ Binding

| Function | Overhead vs Native Rust | Notes |
|----------|------------------------|-------|
| `ta_formula_eval()` | +0.02ms | Minimal C FFI overhead |
| `ta_formula_eval_bytecode()` | +0.02ms | Same overhead, faster execution |
| `ta_formula_eval_optimized()` | +0.02ms | Same rewrite as `eval`, no cache reuse |
| `ta_formula_eval_debug()` | +0.05ms | Additional debug info collection |

## Recommendations

### When to Use Each Mode

1. **AST Interpretation (`formula_eval`)**
   - One-time formula evaluation
   - Interactive development and testing
   - Simple formulas with few variables

2. **Bytecode Compilation (`formula_eval_bytecode`)**
   - Repeated formula execution (5+ times)
   - Real-time analysis systems
   - Medium to complex formulas

3. **Optimized Execution (`formula_eval_optimized`)**
   - Not a throughput path. It applies the *same* AST rewrite `formula_eval` applies
     (constant folding, algebraic simplification, strength reduction, CSE) — it does
     not run statement-level dead-code elimination, because an `X:=...` assignment is
     observable through the context's variable map and dropping it would change the
     result.
   - It also parses and optimizes on **every call** instead of reusing the compiled
     cache, so `formula_eval` is the faster choice for repeated evaluation. Use this
     entry point only when you specifically want the rewrite applied to a source you
     are not going to evaluate again.

4. **Builtin Function Mapping**
   - Direct indicator calculation (e.g., `MACD()` instead of `EMA(C,12)-EMA(C,26);EMA(DIF,9)`)
   - When near-native performance is needed
   - Formula engine's builtin functions achieve near-native performance

5. **Debug Mode (`formula_eval_debug`)**
   - Development and debugging
   - Formula validation and testing
   - Performance profiling

### Optimization Tips

- Use builtin function mapping (e.g., `MACD()` instead of `EMA(C,12)-EMA(C,26);EMA(DIF,9)`) for best performance
- Formula engine's builtin functions achieve near-native performance
- For complex composite formulas, consider using the builtin function rather than composing from primitives
- Use bytecode/optimized mode for formulas executed more than 5 times
- Cache bytecode programs when possible (future feature)
- Minimize variable assignments in formulas
- Use built-in functions instead of manual calculations when available
- Profile with debug mode before deploying to production

## FormulaValue Scalar Optimization Results

| 场景 | 数据量 | 原生 (µs) | 优化后公式 (µs) | 倍率 |
|------|--------|-----------|----------------|------|
| MA(20) | 10K | 61.1 | 43.7 | 0.72x 🟢 |
| MA(20) zero_alloc | 10K | 61.1 | 35.9 | 0.59x 🟢 |
| RSI(14) | 10K | 50.9 | 47.0 | 0.92x 🟢 |
| MA(20) | 100K | 583.8 | 533.3 | 0.91x 🟢 |
| MA(20) zero_alloc | 100K | 583.8 | 404.4 | 0.69x 🟢 |
| RSI(14) | 100K | 521.8 | 497.1 | 0.95x 🟢 |

### Key Optimizations

- **FormulaValue枚举**: 标量值不再分配完整数组，零开销
- **标量-数组运算优化**: 标量广播避免临时数组创建
- **零分配执行路径**: `eval_zero_alloc()` 使用预分配缓冲区
- **字节码VM优化**: 栈使用FormulaValue替代Array1

## Multi-Language Formula API Parity

本表逐格按源码核对（各 binding 的 `src/lib.rs` 导出 + 对应语言包装层），不是
按设计意图填的。✅ = 该语言有可调用的入口；❌ = 没有；⚠️ = 原生符号已导出但
上层包装未暴露。

| 功能 | Python | Node.js | Go | Java | C | .NET | WASM |
|------|--------|---------|-----|------|---|------|------|
| formula_eval | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ |
| formula_eval_multi | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ |
| formula_eval_draw | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| formula_eval_debug | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ |
| formula_validate | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ | ✅ |
| formula_get_template | ✅ | ✅ | ✅ | ✅ | ❌ | ⚠️ | ✅ |
| formula_search_templates | ✅ | ✅ | ✅ | ✅ | ❌ | ⚠️ | ✅ |
| formula_list_categories | ✅ | ✅ | ✅ | ✅ | ❌ | ⚠️ | ✅ |
| JIT / SIMD / ZeroCopy | ✅ | ✅ | 仅 ZeroCopy | ✅ | ❌ | ✅ | ❌ |
| `ta_formula_*_contract_json` 家族 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |

要点：

- **C 只有 contract JSON 家族。** `ta_formula_eval` / `_multi` / `_draw` /
  `_debug` / `_validate` 以及模板查询在 C 里**都不存在**——C 暴露的是
  `ta_formula_eval_contract_json`、`ta_formula_eval_temporal_contract_json`、
  `ta_formula_eval_panel_contract_json`、
  `ta_formula_eval_cross_sectional_contract_json`、
  `ta_formula_stream_execute_json`、`ta_formula_compatibility_report_json`。
  早期版本的本表把 C 整列标成 ✅ 是错的。
- **Go 没有 JIT/SIMD**，只有 `ta_formula_eval_zc_exec`（零拷贝）；**.NET 三者都有**
  （`FormulaEvalJit` / `FormulaEvalSimd` / `FormulaEvalZeroCopy`）。
- **.NET 的模板查询是 ⚠️**：`ta_formula_get_template` /
  `_search_templates` / `_list_categories` 三个原生符号已导出，但
  `Finkit/Indicators.cs` 没有对应的 C# 包装方法，调用方需自行 P/Invoke。
- **WASM 没有 contract JSON 家族**，走的是直接返回 JS 对象的
  `formula_eval` / `formula_eval_multi` / `formula_validate` /
  `formula_get_template` / `formula_search_templates` / `formula_list_categories`。
- 模板查询在各语言的准确名字见
  [`docs/formula-templates.md`](formula-templates.md#coverage-by-language)。

### Plan lane access (2026-10-28)

Unary, binary and arithmetic Plan kernels now resolve lane pointers before their sample loops rather than repeatedly indexing a dynamically selected `Vec`; arithmetic opcode selection is also hoisted outside the loop. Input/output aliases still load the current index before writing it. `cargo check -p finkit --locked` and the crate-scoped formatter check pass. On the scaled release workload, all 60 formula cases remained numerically consistent. The 5K domestic warm Plan aggregate improved 5.2% (3,181.0 to 3,016.9 µs) and Pine improved 7.4% (1,893.5 to 1,753.3 µs); allocations did not change. Cold Plan compilation remains slower than Tree and is a separate outstanding hotspot. Probe: `dist/bench/formula-mode-workload-raw-lane-pointers-final-20261028.txt`.

### SUM/REF prefix writes (2026-10-28)

`SUM` and `REF` no longer prefill the entire output with NaN before overwriting valid samples; they initialize only the NaN prefix, while `SUM` retains the all-NaN result for periods that cannot produce a valid sample. The release probe remained consistent in all 60 cases. Two scaled runs measured domestic warm Plan/Tree at 0.998x and 0.991x, and Pine at 0.867x and 0.868x; this change has not produced a repeatable aggregate speedup yet. Probe: `dist/bench/formula-sum-ref-prefix-fill-20261028.txt` and `dist/bench/formula-sum-ref-prefix-fill-repeat-20261028.txt`.

### Borrowed Plan cache lookup (2026-10-28)

Plan cache lookups now borrow the formula source as `&str`, use a fixed dialect slot for no-parameter plans, and keep parameterized variants in a separate map. This removes one source-string allocation per warm Plan evaluation while preserving dialect/parameter identity and the 1024-plan FIFO bound. Two scaled release runs remained 60/60 numerically consistent and reduced warm Plan allocations from 36.8 to 35.8 (domestic) and 56.2 to 55.2 (Pine). Plan/Tree warm ratios were 1.007x/0.987x domestic and 0.868x/0.856x Pine, so elapsed-time improvement is not yet repeatable across runs. Probes: `dist/bench/formula-plan-borrowed-cache-key-fast-20261028.txt` and `dist/bench/formula-plan-borrowed-cache-key-fast-repeat-20261028.txt`.

### Shared immutable hot plans (2026-10-28)

The per-call formula dispatcher borrows the current host context, so the engine still creates a new executor for each evaluation. The executor now shares FormulaHotPlan's immutable `HotExecutionPlan` instead of deep-cloning all instructions and layouts. Non-formula executor construction retains inline owned-plan storage. Two initial 5K release runs remained 60/60 consistent. Warm Plan allocations fell further to 19.8 (domestic) and 29.2 (Pine), from 35.8 and 55.2 before plan sharing. Plan/Tree ratios were 0.971x/0.983x domestic and 0.853x/0.849x Pine across those runs. A final run on the owned/shared executor implementation remained 60/60 consistent, with 5K ratios of 0.967x domestic and 0.833x Pine and the same allocation counts. These measurements support a repeatable allocation reduction; small 250-row workloads still show Tree faster. Probes: `dist/bench/formula-plan-shared-hot-20261028.txt`, `dist/bench/formula-plan-shared-hot-repeat-20261028.txt`, and `dist/bench/formula-plan-shared-hot-final-20261028.txt`.

### Inline formula input bindings (2026-10-28)

Warm Plan evaluation no longer allocates a `Vec` to hold the common one-to-eight
borrowed input slices. Higher-arity plans retain the heap-backed path. Two
scaled release runs remained 60/60 consistent and reduced Plan allocations by
one per evaluation: **19.8 → 18.8** domestic and **29.2 → 28.2** Pine. The
5K Plan/Tree ratios were **1.012x / 0.893x** domestic and **0.834x / 0.843x**
Pine; domestic timing remains too variable to establish a repeatable gain, while
the allocation reduction is stable. Probe logs:
`dist/bench/formula-plan-inline-inputs-20261028.txt` and
`dist/bench/formula-plan-inline-inputs-repeat-20261028.txt`.

### Candlestick rolling bodies (2026-10-28)

`CDLHARAMI` now maintains both preceding 10-body averages as rolling sums and stores the moving short window in a fixed stack array. The finite common path avoids its full-length temporary body vector; non-finite body inputs retain the reference calculation. The latest full release gate shows exact parity at 1K/100K/1M and **1.81x/1.90x/1.78x** versus TA-Lib; 1M runtime fell from **5.98 ms to 2.65 ms**. Report: `dist/bench/talib-all-current-gate-20261028-harami-stack-window-final.json`.

### ADX output initialization (2026-10-28)

`adx_into` now initializes only the lookback prefix to NaN; the state loop writes every later slot, avoiding a full-output fill followed by overwrites. Full-gate parity remains unchanged. At 1M, ADX improved 5.7% and ADXR 4.6%; ADX is still slower than TA-Lib, so further optimization is needed. Report: `dist/bench/talib-all-current-gate-20261028-adx-prefix-fill.json`.

### Formula DMI direct output (2026-10-29)

Four-argument Plan calls to `PLUS_DI`, `MINUS_DI`, and `ADX` now write into their existing plan output buffers, avoiding a temporary full-series result and copy. Two 5K-row release runs kept the full 60-case corpus numerically consistent. In `dmi_tdx`, Plan ran in **86.0/86.7 µs** with **9 allocations**, while Tree took **120.0/120.5 µs** with **37 allocations**. At 250 rows, Plan remained slower because fixed Plan dispatch overhead dominates (**16.6/16.3 µs** vs **12.0/12.3 µs**). Results and limits are recorded in `dist/bench/formula-dmi-direct-20261029-summary.txt`.

### Formula AROON single-leg execution (2026-10-29)

Plan calls to `AROON_UP` / `AROON_DN` now run just the requested extreme tracker and write directly to the output lane, skipping the unused opposite result. The shared 60-case workload stayed numerically consistent in both release runs. In `aroon_cross`, Plan used **16 allocations** versus Tree's **27**; at 5K rows it took **63.9/64.6 µs** versus Tree's **67.0/66.5 µs**. At 250 rows Plan took **15.0/14.8 µs** versus **9.7/8.1 µs**, so small-series dispatch still costs more. This improves formula execution and memory use; the public two-output AROON API remains slower than TA-Lib in the full indicator comparison. Reports: `dist/bench/formula-aroon-single-leg-repeat-20261029.txt` and `dist/bench/formula-aroon-single-leg-repeat2-20261029.txt`.

### Formula WILLR direct output (2026-10-29)

The Plan `WILLR` handler now writes through `willr_into` directly into its output lane, eliminating one temporary full-series result and copy. Two 60-case release runs stayed numerically consistent. At 5K rows, `williams_r` used **22 Plan allocations** versus **38 Tree allocations**, while Plan took **27.1/27.5 µs** versus Tree's **20.5/20.7 µs**; dispatch overhead still makes this individual case slower than Tree. Reports: `dist/bench/formula-willr-direct-20261029.txt` and `dist/bench/formula-willr-direct-repeat-20261029.txt`.

### SUM output initialization (2026-10-29)

The public rolling `SUM` wrapper now fills only its NaN prefix and writes valid windows directly into an uninitialized result buffer, preserving the existing accumulation order. Two interleaved Rust/C release probes verified the prefix and matched valid windows within **9.87e-9** at 1M rows. Finkit beat TA-Lib C by **2.08x/2.20x/1.15x** and **2.08x/2.20x/1.26x** at 1K/100K/1M. The formula Plan workload remained 60/60 consistent; its aggregate timing is not a SUM-specific measurement. Reports: `dist/bench/sum-prefix-output-20261029.txt`, `dist/bench/sum-prefix-rust-c-interleaved-20261029.txt`, `dist/bench/sum-prefix-rust-c-interleaved-repeat-20261029.txt`, and `dist/bench/formula-sum-prefix-20261029.txt`.

### Formula Plan direct OBV output (2026-10-29)

The Plan backend now detects OBV-only scripts whose remaining nodes only pass
the result through assignment, plot, variable, statement, and string-literal
nodes. It executes `obv_into` on the bound context columns and retains the
normal output-publication path for plot channels, variables, and string-table
effects. Two release runs kept all 60 Tree/Plan cases consistent. For the 5K
Pine `obv` workload, Plan took **15.9/15.6 µs**, down from **17.1/17.3 µs** in
the preceding unpaired runs. The Pine Plan/Tree aggregate stayed within its
previous range (**0.810x/0.817x**), so this is a measured single-workload gain,
not evidence of a broad aggregate speedup. Reports:
`dist/bench/formula-plan-obv-chain-20261029.txt`,
`dist/bench/formula-plan-obv-chain-repeat-20261029.txt`; shape and publication
check: `cargo run -p finkit --example plan_obv_shape --locked`.

### AROON branchless age-increment candidate (2026-10-29, rejected)

Replacing the `if found { age += 1 }` branch with `age += found as usize`
preserved bitwise output parity, including NaN/infinity/tied-extreme inputs, but
did not improve the hot loop. Two release probes measured the candidate slower
than the shipped age tracker at every size from 1K through 1M; at 1M it was
about **5–6% slower** for both AROON and AROONOSC in the repeat. The existing
age tracker remains. Probe: `dist/bench/aroon-branchless-age-20261029.txt`.

### Inline slot table for unified plan execution (2026-10-29)

Executions whose plan has at most 32 physical buffers now keep the outer slot
table in a fixed stack array. The individual series buffers still come from and
return to `BufferArena`; larger plans keep the previous heap-backed path. Two
release runs kept all 60 formula cases consistent at both 250 and 5K rows and
reduced average Plan allocations by one per evaluation (domestic **18.7 →
17.7**, Pine **27.8 → 26.8**). At 5K, warm Plan/Tree was **0.957x/0.963x** for
domestic and **0.801x/0.798x** for Pine. Timing varies by run, so the reliable
gain established here is one fewer allocation; 250-row Plan dispatch remains
slower than Tree. Probe: `dist/bench/formula-plan-inline-slot-table-20261029.txt`
plus the first release run recorded during implementation.

### Direct return for single-output Plan results (2026-10-29, rejected)

A follow-up path tried returning the primary series without constructing the
one-element result container. The 60-case scaled workload remained consistent,
but average allocations changed only **17.7 → 17.6** for domestic and did not
change for Pine; aggregate timings moved within run-to-run variation. The
duplicated executor loop was removed. Probe:
`dist/bench/formula-plan-single-output-result-20261029.txt`.

### Stable formula CORREL kernel (2026-10-29)

The formula `CORREL` path now uses the TA-Lib 0.8.1 shifted-sum and periodic
reseed order on finite inputs. This replaces raw-price rolling sums that drifted
by up to `9.8e-3` at one million rows. The kernel writes each output row once
and avoids a period-sized ring on the common finite path. Non-finite windows
still produce `NaN` and recover after the invalid value leaves the window;
zero-variance windows remain `NaN` to preserve formula semantics.

Two interleaved Rust/C probes reported **1.145x / 1.145x / 1.146x** and
**1.180x / 1.144x / 1.133x** at 1K/100K/1M (TA-Lib time / Finkit time), with
zero output mismatches. These are period-20 direct-kernel measurements on
deterministic finite data, not a complete FormulaEngine or parameter-space
gate. A follow-up covered periods 2, 5, 20, and 100 at all three sizes; all
12 combinations had zero mismatches, and Finkit was faster in each case
(1.083x–1.173x). The FormulaEngine Tree and Plan routes also matched the direct
kernel at 1K and 100K rows for period 20. Probe logs:
`dist/bench/formula-correl-stable-kernel-20261029.txt`,
`dist/bench/formula-correl-stable-kernel-repeat-20261029.txt`, and
`dist/bench/formula-correl-stable-kernel-multiparam-20261029.txt`.

### Formula BETA contract audit (2026-10-29)

The formula `rolling_beta_into` currently computes a price-level slope over
`period` rows. TA-Lib 0.8.1 BETA computes a slope over per-bar returns, takes
the index series as its first input and the security series as its second, and
starts output at index `period`. The direct probe therefore found differing
outputs, but its timings are not a valid same-contract comparison. See the
[TA-Lib BETA implementation](https://github.com/TA-Lib/ta-lib/blob/main/ta_codegen/input/beta/beta.c)
and `dist/bench/formula-beta-current-parity-20261029.txt`. Resolve the formula
contract before accepting a BETA optimization.

### OBV Plan early-return candidate (2026-10-29, rejected)

Moving the existing direct-OBV Plan shortcut before input binding and returning
before the common result-container path kept all 60 corpus cases consistent,
but did not produce a stable timing gain. Native Plan/Tree stayed at
**1.510x/1.837x** versus **1.511x/1.847x** before; scaled Plan/Tree was
**0.947x/0.816x** versus **0.940x/0.826x**. At 5K Pine OBV, the candidate was
**14.8 µs** versus Tree's **9.1 µs**, compared with **15.1 µs** versus **9.6 µs**
before. The candidate code was removed. Reports:
`dist/bench/formula-correl-kernel-workload-20261029.txt` and
`dist/bench/formula-plan-obv-early-return-20261029.txt`.

### Plan sandbox bypass candidate (2026-10-29, rejected)

Skipping the Plan sandbox guard, timer check, and memory counter when every
limit is unset preserved 60/60 corpus results, but the two scaled runs did not
show an aggregate gain. Domestic Plan/Tree was **0.948x/0.958x**, Pine was
**0.819x/0.802x**; prior runs were **0.940x/0.947x** and **0.826x/0.816x**.
Native 250-row timing varied from **1.477x to 1.586x**. The code was removed.
Reports: `dist/bench/formula-plan-unlimited-sandbox-20261029.txt` and
`dist/bench/formula-plan-unlimited-sandbox-repeat-20261029.txt`.

### Binary operator dispatch match candidate (2026-10-29, rejected)

The Plan dispatcher briefly changed its 17-way ordered comparison chain for
binary operations into a `match` on `KernelId`. The 60-case workload remained
non-degenerate and numerically consistent, but Plan's 5K absolute time was
unchanged for domestic formulas (**2973.5 → 2975.6 µs**) and slightly slower
for Pine (**1642.6 → 1662.1 µs**). Plan/Tree ratios moved from **0.952x →
0.934x** domestic and **0.825x → 0.819x** Pine mainly because Tree timing
varied. The candidate was reverted; ordered dispatch is not a proven hotspot.
Report: `dist/bench/formula-binary-dispatch-match-20261029.txt`.

### Precompiled external-input slot on hot nodes (2026-10-29, rejected)

Each executor pass briefly copied the input slot onto every hot instruction to
avoid asking `InputLayout`'s `BTreeMap` for each node. Two full 60-case runs
remained non-degenerate and numerically consistent, but Plan 5K time averaged
**2978.4 µs** domestic versus **2973.5 µs** before, and **1750.7 µs** Pine
versus **1642.6 µs** before. The extra instruction field did not pay for the
removed lookups; it was reverted. Continue profiling a different source of
per-node overhead.

### Common-subexpression elimination across built-in OHLCV aliases (2026-10-29)

Plan CSE now keys built-in OHLCV variables by their canonical market-data name
(`H`/`HIGH`, `L`/`LOW`, `C`/`CLOSE`, and the other standard aliases). The plan
node and published variable names retain the original spelling; only pure
numeric subexpressions merge. In `dialect_ohlc_aliases`, this reduced the hot
plan from 14 nodes to 8. A focused paired probe measured Plan at **7.095 →
4.492 µs** (36.7% faster) while Tree stayed at **2.857 → 2.836 µs**. The
primary result and all three named assignment arrays matched exactly. The full
60-case workload remained 60/60 consistent; this single formula change does
not materially shift the suite aggregate. Report and probe:
`dist/bench/formula-alias-cse-20261029.txt` and
`core/examples/formula_alias_cse_probe.rs`.

### AROON bounded monotone-deque path (2026-10-29)

The earlier full-range deque experiment did not beat the cached-age tracker,
so the production path now uses the fixed-capacity monotone deque only for
periods 7–31. Periods 2–6 and 32+ retain the cached-age tracker. The window
evicts old indices, skips NaNs, retains infinities, and removes older equal
values so the newest-tie rule is unchanged. Both the two-leg `aroon` API and
`aroonosc` use this path.

The release probe covered periods 2, 5, 7, 10, 14, 20, and 31 at 1K, 100K,
and 1M rows. The production APIs and independent deque candidate had zero
mismatches against TA-Lib C in the checked outputs; special tie/NaN/infinity
inputs also matched the prior Rust implementation. At the common 14-bar period,
the production two-column AROON path measured 1.54x/1.44x TA-Lib at 100K/1M;
AROONOSC measured 1.84x/1.72x. This targeted result supersedes the earlier
full-range-deque rejection only for periods 7–31; the other periods remain on
the measured cached-age path. Report: `dist/bench/aroon-monotone-production-20261029.txt`;
probe: `cargo run --release -p finkit --example aroon_monotone_probe --features talib-c --locked`.

### VAR rolling-state hot path (2026-10-29)

The shared rolling-variance state now uses in-bounds unchecked reads for the
incoming and trailing samples, with the range invariant documented at the
access site. Its reseed countdown is a plain decrement; it is initialized and
reset to a positive, saturating `32 * period`, and the zero case resets it
before the next bar. This state is also shared by `STDDEV` and Bollinger-band
calculations.

Two release probes compared the public Finkit `VAR` wrapper to TA-Lib C 0.8.1
for periods 5, 20, and 100 at 1K/100K/1M rows. All 18 size/period/run
comparisons had zero mismatches and exact outputs. Finkit/TA-Lib speed ratios
were **0.926–1.000x** at 1K, **0.959–0.962x** at 100K, and **0.944–1.007x** at
1M, reducing the prior 100K gap and bringing the common periods close to parity.
Some 1M cases remain slower than C, so this does not close the speed gate.
Reports: `dist/bench/var-talib-current-20261029.txt`,
`dist/bench/var-talib-countdown-20261029.txt`,
`dist/bench/var-talib-countdown-repeat-20261029.txt`,
`dist/bench/var-talib-pointer-20261029.txt`, and
`dist/bench/var-talib-pointer-repeat-20261029.txt`.

### STDDEV output-prefix initialization (2026-10-29)

The allocating `stddev` wrapper now allocates an uninitialized result and
initializes only the warm-up prefix to NaN; the fused rolling state writes every
later slot. Leading warm-up inputs recurse on the finite tail, while a tail too
short to form a window explicitly fills the complete output with NaN. This
removes a full-result NaN fill that the previous implementation immediately
overwrote on ordinary finite input.

Two paired release runs matched TA-Lib C 0.8.1 exactly for periods 5/20/100 at
1K/100K/1M. Finkit measured **1.35–1.41x**, **1.24–1.27x**, and **1.27–1.31x**
at those sizes. At 1M, the prior wrapper was **1.05–1.10x**; the new wrapper
reduces Finkit's runtime by about 17–22% across the three periods. The probe
also checks leading-NaN recovery, insufficient finite tail, and warm-up slots.
Reports: `dist/bench/var-stddev-prefix-before-20261029.txt`,
`dist/bench/var-stddev-prefix-after-20261029.txt`,
`dist/bench/var-stddev-prefix-after-repeat-20261029.txt`, and
`dist/bench/var-stddev-prefix-after-edge-check-20261029.txt`.

### Dragonfly / Gravestone Doji TA-Lib-aligned rolling ranges (2026-10-29)

The shared Doji core now tracks the preceding ten `high - low` ranges and
classifies each current bar before adding it to the rolling sum, matching
TA-Lib's default `BodyDoji` and `ShadowVeryShort` settings. The earlier code
used true range, advanced the window before classification, and required a
2x-average long shadow; those differences could misclassify gapped bars. The
shared fixed-period ring now advances with a cursor and no per-bar modulo. The
conditions follow the [TA-Lib Dragonfly](https://github.com/TA-Lib/ta-lib/blob/v0.8.1/src/ta_func/ta_CDLDRAGONFLYDOJI.c)
and [Gravestone](https://github.com/TA-Lib/ta-lib/blob/v0.8.1/src/ta_func/ta_CDLGRAVESTONEDOJI.c)
implementations and default [candle settings](https://github.com/TA-Lib/ta-lib/blob/v0.8.1/src/ta_common/ta_global.c).

Two release runs, with TA-Lib initialized to its defaults, had exact full-output
parity at 1K/100K/1M rows. Both also matched exactly on 100K deliberately
gapped bars. Finkit beat the C functions by **1.96–2.11x / 1.42–1.63x** at 1K,
**1.69–2.12x / 1.42–1.63x** at 100K, and **1.96–2.04x / 1.70x** at 1M
(Dragonfly / Gravestone). Versus the prior Finkit implementation, its 1M time
fell about **36–41%**. Evidence:
`dist/bench/cdl-doji-high-low-gap-parity-20261029.txt` and
`dist/bench/cdl-doji-high-low-gap-repeat-20261029.txt`; the probe also asserts
parity and calls `TA_Initialize`/`TA_Shutdown` around the C measurements.

### Current Python TA-Lib public-overlap gate (2026-10-29)

A fresh Finkit 0.2.0 wheel built from the current workspace was compared with
TA-Lib Python 0.6.8 (bundled C core 0.6.4) for 49 overlapping functions at
1K/100K/1M rows. All 147 calls completed and the geometric-mean Finkit speedup
was **1.6203x**, but the strict per-observation gate failed and 10 comparisons
did not pass parity. BETA remains a known formula-contract mismatch. CORREL,
STDDEV, and VAR need their discrepancies reconciled with the separate direct
TA-Lib C 0.8.1 probes before changing their numerical kernels. The gate's MFI
result was **0.911x** at 1M rows despite parity, while the same-input focused
probe against the separately linked C 0.8.1 library measured **1.959x**. The
opposite rankings mean the two C builds are not interchangeable timing
references; MFI is not changed on this evidence. Full results:
`dist/bench/talib-all-current-gate-20261029-current-wheel.txt` and its adjacent
JSON. The overall TA-Lib speed goal remains open.

### BufferArena reserved empty-bucket candidate (2026-10-29, rejected)

Keeping one just-drained length bucket in the arena reduced warm Plan allocator
counts from **17.7 to 16.6** per domestic formula and **26.8 to 26.2** per Pine
formula across the 60-case workload. Both candidate runs stayed 60/60 consistent,
but summed warm execution time did not improve consistently: 5K domestic Plan
was **1.5–2.5% slower** than the baseline, while Pine and 250-row results moved
in both directions. The change was reverted; further work should target Plan's
short-series node dispatch cost rather than keeping more arena metadata.
Evidence: `dist/bench/formula-buffer-arena-reserved-bucket-20261029-summary.txt`
and its two raw run files.

### Input-copy prefix in Formula Plan (2026-10-29, rejected)

Moving all external-input copies ahead of the kernel loop removed input-node
lookups from that loop, but violated the current physical buffer lifetime
schedule: an early copy can overwrite a slot that the original topological
order reuses for an intermediate. The full corpus fell to **53/60** consistent
cases, so the candidate was reverted. Its 5K Plan aggregate also regressed from
**2911.4/1649.8 µs** to **3003.8/1761.2 µs** (domestic/Pine). Preserve node order
unless the compiled buffer layout is updated to prove the reordered writes safe.
Report: `dist/bench/formula-plan-input-copy-prefix-20261029-summary.txt`.

### Formula Plan input-lookup layout variants (2026-10-29, rejected)

Replacing the node-to-input BTreeMap with a parallel slot vector, dense node
table, or sorted compact vector kept the full corpus at **60/60** consistency,
but none improved 5K Plan timing across domestic and Pine workloads. The
parallel and sorted vectors slowed 5K Pine by roughly **4%** and **7%**; the
dense table was close to baseline but also slower at 5K. The alternatives were
reverted. This rules out input-map lookup as a broad latency target; continue
profiling numeric kernel dispatch and FormulaEngine setup.
Report: `dist/bench/formula-plan-input-lookup-variants-20261029-summary.txt`.

### Short-input native dispatch and current Formula Plan gate (2026-10-29)

The Python native hot paths now keep the GIL for inputs shorter than
`PY_GIL_RELEASE_THRESHOLD` (16,384 rows) and detach the Rust calculation for
larger inputs. This avoids paying a thread-state transition on very short
calls while preserving parallel execution for longer scans. Two full 49-function
Python gate repeats completed all 147 observations without execution errors;
the geometric mean was **1.485x** and **1.493x**, versus **1.480x** for the
same-version pre-threshold run. The aggregate change is small and close to
measurement variation, so it is not treated as a broad speedup claim. In the
1K cases, OBV moved from **1.67 µs** to **1.55/1.51 µs** and ROC from **1.75 µs**
to **1.69/1.54 µs**; the 100K/1M paths still detach. Ten parity failures remain
(BETA, CORREL, and the older Python TA-Lib core's VAR/STDDEV behavior).

The release formula workload was also rerun after the recent Plan dispatch
changes. All **60/60** formulas remained non-degenerate and consistent at both
250 and 5K rows. At 5K, summed warm Plan time was **2,891.4/1,598.6 µs** versus
Tree **3,128.3/2,003.2 µs** (domestic/Pine, Plan/Tree **0.924x/0.798x**). At
250 rows, Plan remains slower: **565.4/487.9 µs** versus Tree **417.9/317.2 µs**.
A focused stage probe measured the FormulaEngine wrapper at only **0.28–0.34 µs**
above a precompiled executor, so the next formula work should target per-node
execution and input movement rather than cache lookups. Reports:
`dist/bench/talib-all-current-gate-20261029-short-gil-path.json`,
`dist/bench/talib-all-current-gate-20261029-short-gil-path-repeat.json`, and
the `formula_mode_workload_probe --scaled` / `formula_plan_stage_probe` output.

### Price-transform output initialization (2026-10-29)

`AVGPRICE`, `MEDPRICE`, `TYPPRICE`, and `WCLPRICE` now allocate an uninitialized
`Vec<f64>` and let the existing SIMD/`_into` kernel write every result before
wrapping it as an `Array1`. This removes the zero-fill pass performed by the
allocating Rust APIs; the small-input Python public wrappers use those APIs, so
this specifically improves their 1K-row route. Two focused 1K-row repeats kept
TA-Lib parity across all four outputs and measured public Finkit calls at **1.53–1.56
µs AVGPRICE**, **0.97–1.03 µs MEDPRICE**, **1.36–1.38 µs TYPPRICE**, and
**1.19–1.22 µs WCLPRICE**. The preceding same-environment run measured
**1.83 / 1.12 / 1.59 / 1.35 µs** respectively. The complete 49-function gate
had no execution errors, retained the same 10 known parity failures, and
measured **1.489x** geometric-mean speedup; the 100K/1M price paths were already
using the `_into` route and are unchanged. Report:
`dist/bench/talib-all-current-gate-20261029-price-uninit.json` and the two runs
of `C:\tmp\finkit-talib-gate\price_path_probe.py`.

The recognition-only early-dispatch experiment for `ZLEMA`, `COPPOCK`, and
`WAD`, plus a later targeted direct-output `COPPOCK` candidate, kept the 60/60
formula corpus consistent but did not improve the full aggregate; those
`COPPOCK`/`WAD` changes were reverted. `ZLEMA` was later reintroduced as a
targeted direct-output kernel (see below). The COPPOCK candidate's focused
formula ran faster in isolation, but its full corpus sums moved from
**568.6/490.4 µs** to **599.6/499.6 µs** at 250 rows and from
**2,910.6/1,567.5 µs** to **2,902.8/1,617.5 µs** at 5K rows (domestic/Pine),
so the isolated gain did not justify retaining it. Evidence:
`dist/bench/formula-mode-current-20261029-zlema-coppock-direct.txt`. Broader
Formula Plan work should still target per-node execution and input movement.

### BOP output initialization (2026-10-29)

The allocating `BOP` API had the same full-overwrite pattern and now uses
`uninit_output` before its existing SIMD kernel. The rebuilt wheel retained
TA-Lib parity at 1K/100K/1M and measured **1.34 / 62.48 / 3,510.90 µs**,
compared with the preceding gate's **1.46 / 75.62 / 3,863.93 µs**. The full
current gate completed all 147 calls with no execution errors; its geometric
mean was **1.489x** and the same 10 compatibility parity differences remain.
Evidence: `dist/bench/talib-all-current-gate-20261029-price-bop-uninit.json`.

### ZLEMA direct-output Formula Plan kernel (2026-10-29)

Formula Plan now routes `ZLEMA` through the shared `zlema_into` recurrence and
writes into its plan-owned result buffer. It preserves the canonical bridge's
leading-warmup and short-tail behavior, while avoiding the temporary
`Array1` result and output copy. The full release workload stayed **60/60**
consistent and non-degenerate. In the focused 1-output stage probe, Plan/Tree
was **1.514/1.439 µs** at 250 rows and **18.002/18.804 µs** at 5K rows; this
helps the longer-history case, while Plan's 250-row cost is still slightly
higher. The full corpus aggregate remains slower for Plan at 250 rows and
faster at 5K: **568.6/490.4 µs** vs Tree **414.4/311.4 µs** at 250, and
**2,910.6/1,567.5 µs** vs Tree **3,134.4/2,020.6 µs** at 5K (domestic/Pine).
These aggregate differences are within the recent run-to-run range and are not
attributed to ZLEMA alone. Evidence:
`dist/bench/formula-mode-current-20261029-zlema-direct.txt` and the
`formula_plan_stage_probe` release output.

### KDJ fused extrema and Wilder smoothing (2026-10-29)

For finite OHLC inputs, the TA-Lib-profile KDJ path now consumes the shared
rolling high/low visitor directly and fuses the raw-K, K, and D Wilder
recurrences. This removes the materialized high/low, raw, K, and D intermediate
series and the copy of the K series. Inputs containing non-finite values retain
the prior batch composition as the compatibility path.

Against the locked TA-Lib C/Python 0.8.1 reference, the full 201-function gate
completed **603/603** observations with **0 execution errors**. KDJ remained
parity-correct at all sizes (maximum absolute error `5.69e-14`). Its paired
gate timings changed from **23.63/3,614.56/36,863.20 µs** to
**19.94/2,043.51/21,639.30 µs** at 1K/100K/1M rows, reductions of about
**16%/43%/41%**. Against TA-Lib, the new ratios are **0.90x/0.98x/1.00x**:
large-series performance improved substantially but has not yet cleared the
per-function `1.0x` floor. The full gate's only two parity failures are ADOSC
at 100K and 1M, with identical finite masks and small floating-point deltas.
Strict all-row speed still fails. Evidence:
`dist/bench/talib-all-current-gate-20261029-talib081.json` (before) and
`dist/bench/talib-all-current-gate-20261029-talib081-kdj-fused.json` (after).

The periodic-kernel ID classifier candidate was not kept. It retained **60/60**
formula corpus consistency, but the two 5K warm-corpus runs did not establish a
repeatable aggregate gain over the prior Formula Plan baseline. The domestic
Plan/Tree ratio stayed around **0.92–0.95x**; Pine stayed around **0.78–0.80x**.
The 250-row aggregate continued to favor Tree. Formula Plan execution therefore
remains an active optimization target, especially for short histories.

An earlier release corpus probe after the KDJ change remained **60/60**
consistent. Warm aggregate Plan/Tree was **1.399x/1.554x** at 250 rows and
**0.934x/0.780x** at 5K rows (domestic/Pine). The larger-history Plan path still
wins on aggregate, but short-history formula execution has not reached parity
with Tree. Reproduce with
`cargo run --release -p finkit --example formula_mode_workload_probe --locked -- --scaled`.

### ACCBANDS bounded rolling storage (2026-10-29)

For series of at least 16,384 rows, ACCBANDS now keeps only two lookback-sized
rings for its transformed upper/lower bands. The SMA seed uses the existing SIMD
horizontal sum, later windows use the same rolling update order, and the two
large output arrays initialize only their NaN prefixes. Short inputs retain the
previous composition; non-finite derived bands fall back to it for its existing
validation behavior.

Against TA-Lib 0.8.1, the final full gate reports ACCBANDS at **8.42/799/8,277
µs** and **0.67x/0.96x/0.94x** for 1K/100K/1M rows, with all three outputs in
parity. Compared with the pre-ring same-version gate, Finkit time fell by about
**44% at 100K** and **35% at 1M**. Full gate evidence:
`dist/bench/talib-all-current-gate-20261029-talib081-kdj-accbands-final.json`.

The final formula corpus run stayed **60/60** consistent. Warm Plan/Tree totals
were **1.321x/1.590x** at 250 rows and **0.929x/0.782x** at 5K rows
(domestic/Pine). Formula Plan therefore retains its longer-series advantage,
while short-series execution remains the next formula priority. Saved output:
`dist/bench/formula-mode-workload-20261029-kdj-accbands-final.txt`.

### ERI/CVI large-output initialization (2026-10-29)

ERI and CVI now use uninitialized output storage only for inputs of at least
16,384 rows, then initialize the NaN warm-up prefix and write every remaining
output value. Short inputs retain the ordinary NaN-filled allocation. This
removes one full output clear for CVI and two for ERI without changing their
EMA calculation or output values.

An interleaved release probe compared each production kernel with the previous
full-NaN-initialization composition. Outputs were bit-identical, including NaN
masks. At 100K rows CVI measured **1,042 → 962 µs** and ERI **1,115 → 845 µs**;
at 1M, CVI measured **10,071 → 8,959 µs** and ERI **11,027 → 8,946 µs**. The
1K paths retain the original initialization. The full TA-Lib 0.8.1 gate
completed 603 observations with zero execution errors and retained its two
existing ADOSC tolerance differences. Its strict per-row speed floor still
fails, so this is a targeted memory-traffic improvement, not completion of the
overall TA-Lib speed goal. Probe: `core/examples/indicator_output_prefix_probe.rs`;
paired measurements: `dist/bench/indicator-output-prefix-20261029.txt`; full
gate: `dist/bench/talib-all-current-gate-20261029-eri-cvi-prefix.json`.

### COPPOCK large-output initialization (2026-10-29)

COPPOCK keeps its existing rolling WMA recurrence, but for inputs of at least
16,384 rows it now initializes only the NaN warm-up prefixes of its combined
ROC buffer and result. Short inputs retain the ordinary NaN-filled allocation.
An interleaved release probe against the prior full-initialization reference
reported bit-identical values and masks: at 100K rows **497 → 483 µs**, and at
1M **8,685 → 6,891 µs** (**1.26x**). The 1K path is unchanged. In the full
TA-Lib 0.8.1 gate COPPOCK remained numerically within tolerance, with Finkit at
**0.62x/0.52x/0.70x** for 1K/100K/1M rows; it is materially faster than the
previous gate's approximately **0.51x** at 1M, but still below TA-Lib. Across
all 201 functions and 603 observations the geometric mean was **1.443x**
(previous run **1.439x**), with zero execution errors and the same two ADOSC
tolerance differences. The strict 1.0x per-observation floor still fails.
Probe: `core/examples/indicator_output_prefix_probe.rs` and
`dist/bench/indicator-output-prefix-20261029-coppock.txt`; full gate:
`dist/bench/talib-all-current-gate-20261029-coppock-prefix.json`.

### Formula Plan inline published-slot lookup (2026-10-29)

Warm primary-only Plan evaluation now keeps the common case of at most eight
published variable slots in a stack array, avoiding a small per-evaluation
hash table; larger scripts retain the hash-backed lookup. The 60-case formula
corpus remained **60/60** consistent. Average warm allocations fell from
**17.6 to 16.4** per domestic formula and **26.8 to 25.4** per Pine formula.
This reduced allocation count without a repeatable aggregate time gain: current
Plan/Tree ratios were **1.362x/1.539x** at 250 rows and **0.931x/0.773x** at
5K rows (domestic/Pine). Short-history Plan execution remains slower overall.
Evidence: `core/examples/formula_mode_workload_probe.rs` and
`dist/bench/formula-mode-workload-20261029-inline-published.txt`.

### SMI large-output initialization (2026-10-29)

For inputs of at least 16,384 rows, SMI now initializes NaN only in the warm-up
prefixes of its numerator, denominator, raw result and two output lines. The
existing rolling extrema, EMA order and formula loops are unchanged, and short
inputs retain the original allocation path. The interleaved reference probe
was bit-identical for both outputs and their NaN masks; at 100K rows SMI
measured **5,092 → 4,714 µs**, and at 1M **51,841 → 47,686 µs** (**1.09x**).

The full TA-Lib 0.8.1 gate before removing the two intermediate EMA copies
reported SMI at **28.69/4,320/44,606 µs** and **0.98x/0.69x/0.68x** for
1K/100K/1M rows, improving the 1M ratio from **0.60x** in the prior run. The
newer gate after copy removal reports **28.69/3,873/42,169 µs** and
**0.98x/0.74x/0.72x**. The latest gate, after ADR and COPPOCK rolling-buffer
changes, reports SMI at **28.01/4,000/41,472 µs** and **0.98x/0.72x/0.72x**.
Across all 201 functions and 603 observations, the latest geometric mean is
**1.457x**; execution errors remain zero, with the same two ADOSC tolerance
differences. The per-observation speed floor still fails; SUPERTREND is now the
slowest 1M case. Probe:
`core/examples/indicator_output_prefix_probe.rs` and
`dist/bench/indicator-output-prefix-20261029-coppock-smi.txt`; full gate:
`dist/bench/talib-all-current-gate-20261029-coppock-ring.json`.

### Formula Plan unlimited-sandbox fast path (2026-10-29)

Plan execution still resets sandbox state on every call, but skips depth,
timeout and byte-budget checks when all three limits are unset (the documented
default). Any configured limit keeps the complete guarded path. The focused
stage probe reduced Engine overhead over a precompiled executor from **0.317 to
0.271 µs** for `CLOSE*2+1` at 250 rows and from **0.342 to 0.243 µs** for
`SMA(CLOSE,14)+ROC(CLOSE,10)`. The full corpus remained **60/60** consistent,
but its aggregate timing did not establish a repeatable gain; Plan/Tree was
**1.375x/1.540x** at 250 rows and **0.924x/0.790x** at 5K rows
(domestic/Pine). This is a focused fixed-cost reduction, not a claim that
short-history Formula Plan is now faster overall. Stage probe:
`dist/bench/formula-plan-stage-probe-20261029-sandbox-fastpath.txt`; corpus:
`dist/bench/formula-mode-workload-20261029-sandbox-fastpath.txt`.

### SUPERTREND large-output initialization (2026-10-29)

For inputs of at least 16,384 rows with no leading warm-up, SUPERTREND now
initializes only the trend-line and optional band NaN prefixes. The fused ATR
callback writes every later index, including explicit NaN/default values when
an ATR value is NaN, so interior non-finite observations remain initialized
without an extra full-input finite-value scan. In the full TA-Lib 0.8.1 gate,
SUPERTREND at 1M rows improved from **12,450 µs / 0.633x** to **12,178 µs /
0.640x**, with parity retained. The complete 201-function, 603-observation
geometric mean is **1.457x**; execution errors remain zero, with the same two
ADOSC tolerance differences. The strict per-observation speed floor still
fails. Evidence:
`dist/bench/talib-all-current-gate-20261029-smi-prefix.json` and
`dist/bench/talib-all-current-gate-20261029-supertrend-prefix-fixed.json`.

### VORTEX output initialization (2026-10-29)

VORTEX now leaves its two full-length result buffers uninitialized at allocation,
writes NaN only into the `period`-length lookback, and writes every later value
in its existing rolling loop. The 1M-row full gate improved from **9,008 µs /
0.750x** to **7,813 µs / 0.884x**; parity and NaN masks remain exact at all
three sizes. The latest combined full gate has 201 functions and 603
observations, a geometric mean of **1.457x**, zero execution errors, and the
same two ADOSC tolerance differences; the strict per-observation speed floor
still fails. Full gate:
`dist/bench/talib-all-current-gate-20261029-coppock-ring.json`.

### ADR large-output initialization (2026-10-29)

ADR initializes only its `period-1` warm-up prefix; the rolling average loop
writes every later result. The full TA-Lib gate kept exact parity at all three
sizes. At 1M rows ADR improved from **5,605 µs / 0.665x** to **3,152 µs /
0.985x**; 100K rows improved from **211 µs / 0.877x** to **190 µs / 0.971x**.
The latest 201-function, 603-observation geometric mean is **1.457x**, with
zero execution errors and the same two ADOSC tolerance differences. The strict
per-observation floor remains open. Full gate:
`dist/bench/talib-all-current-gate-20261029-coppock-ring.json`.

### Formula Plan scalar arithmetic fusion (2026-10-29)

The hot formula plan now lowers a binary arithmetic node with one numeric
literal operand to a scalar kernel. This avoids building and reading a
full-length constant series for expressions such as `CLOSE*2+1`; subtraction
and division retain the literal's operand order, and division keeps the same
near-zero rule. In the focused stage probe, direct execution of `CLOSE*2+1`
improved from **0.495 to 0.402 µs** at 250 rows and **5.942 to 3.738 µs** at
5K rows. Engine Plan improved from **0.766 to 0.655 µs** and **6.346 to
4.100 µs**, respectively; Plan/Tree moved from **1.243x to 1.025x** at 250
rows and **0.339x to 0.215x** at 5K. The full formula corpus remained **60/60**
consistent, but its aggregate Plan/Tree ratios did not improve repeatably
(1.385x/1.639x at 250 rows and 0.924x/0.798x at 5K, domestic/Pine). This is a
measured win for literal-heavy elementwise formulas, not an aggregate corpus
speedup claim. Stage probe:
`dist/bench/formula-plan-stage-probe-20261029-scalar-arithmetic.txt`; full
corpus:
`dist/bench/formula-mode-workload-20261029-scalar-arithmetic.txt`.

### SMI intermediate EMA copy removal (2026-10-29)

SMI now passes the contiguous tails of its first-stage EMA outputs directly to
the second-stage EMA instead of cloning both complete arrays. The paired
reference probe remains bit-identical with matching NaN masks; 1M-row SMI
improved from **54,211 to 44,800 µs** (**1.21x**) and 100K rows from **5,281 to
4,328 µs**. This is in addition to the prefix initialization savings above.
Probe: `dist/bench/indicator-output-prefix-20261029-smi-borrowed-ema.txt`.

### COPPOCK bounded rolling storage (2026-10-29)

For inputs of at least 16,384 rows, COPPOCK now computes the two ROC values as
they enter a period-sized ring and updates the weighted rolling sum in the
same pass. This removes the full-length combined ROC intermediate while
retaining the old recomputation behavior when a NaN enters the window. The
paired old-composition probe had exact values and matching NaN masks; 100K
rows improved from **949 to 431 µs**, and 1M from **7,861 to 5,332 µs** (**1.47x**).
In the full TA-Lib gate, COPPOCK improved from **6,322 µs / 0.657x** to **5,378
µs / 0.792x** at 1M rows, retaining output parity. The combined 201-function,
603-observation geometric mean is **1.457x**, with zero execution errors and
the two existing ADOSC tolerance differences; the strict per-observation speed
floor remains open.
Probe: `dist/bench/indicator-output-prefix-20261029-coppock-ring.txt`.
Full gate: `dist/bench/talib-all-current-gate-20261029-coppock-ring.json`.

### EFI force-index streaming and current full gate (2026-10-29)

EFI no longer allocates a full-length Force Index intermediate or clears the
full output before writing it. It sums the seed window directly, then computes
each later force value inside the EMA recurrence; only the `p`-bar NaN prefix
is initialized. Paired old-composition probes are bit-identical with the
same NaN mask and measured **1.20x** at 1K, **1.79–1.99x** at 100K, and
**1.88–1.90x** at 1M rows. The TA-Lib 0.8.1 release-wheel gate retained EFI parity and measured
**1.52x**, **1.71x**, and **1.46x** at those sizes. Probe:
`dist/bench/indicator-output-prefix-20261029-efi-force-fused.txt`.

The corresponding 201-function, 603-observation release-wheel gate has zero
execution errors, the same two ADOSC tolerance differences, and a geometric
mean of **1.597x**. The strict every-observation 1.0x floor still fails;
SUPERTREND is the slowest 1M row at **0.677x**, followed by SMI at **0.716x**
and CVI at **0.740x**. The gate's module path is the isolated release wheel, so
its aggregate is not treated as a directly paired measurement against earlier
source-tree runs. Full gate:
`dist/bench/talib-all-current-gate-20261029-efi-force-fused-talib081.json`.

### Formula short-history follow-up (2026-10-29)

The 60-case Plan/Tree workload remains **60/60** numerically consistent. Warm
aggregate Plan/Tree ratios are **1.490x/1.610x** at 250 rows and
**0.929x/0.791x** at 5K rows (domestic/Pine), so Plan still loses on short
histories while winning on the larger workload. A root-call-only direct-kernel
candidate was measured for MA and RSI and removed because it did not establish
a repeatable aggregate benefit; the stage probe now retains MA/RSI rows for
future measurements. A short-lived CVI fusion candidate remained exact but was
slower at 1K and only 1.05x at 1M against the previous composition, so it was
also removed. These probes are diagnostic results, not speedup claims:
`dist/bench/formula-mode-workload-20261029-plan-simple-kernel.txt`,
`dist/bench/formula-plan-stage-probe-20261029-simple-root-baseline.txt`, and
`dist/bench/indicator-output-prefix-20261029-efi-cvi-fused.txt`.

### Fused SMI pipeline and latest combined TA-Lib gate (2026-10-29)

For inputs of at least 16,384 rows with no leading high/low warm-up, SMI now
feeds `rolling_minmax_visit` directly into the slow EMA, fast EMA and signal
EMA stages. It retains only the two final output buffers and small EMA seed
windows; non-finite EMA inputs fall back to the existing batch implementation.
The paired prior-composition probe preserved NaN masks and stayed within
`1e-8`; at 100K it improved from **5,069 to 3,237 µs**, and at 1M from
**51,779 to 35,152 µs** (**1.47x**). In the combined TA-Lib 0.8.1 gate, SMI at
1M is **38,480 µs / 0.77x**, about 8% faster than the preceding release-wheel
gate row.

The TA-Lib compatibility Supertrend path now uses a dedicated two-output loop
with the first ATR bar outside the recurrence loop and validated OHLC pointer
reads. The final wheel gate reports parity and **1.04x/1.11x/1.03x** at
1K/100K/1M; the 1M row now clears the 1.0x floor. The combined 201-function,
603-observation gate has **0 execution errors**, the same two ADOSC tolerance
differences, geomean **1.612x**, and still fails the strict floor with
**19/31/39** rows below 1.0x at 1K/100K/1M. The remaining 1M slow rows include
SMI (**0.77x**), CVI (**0.74x**), AC (**0.75x**) and COPPOCK (**0.76x**).
Evidence: `dist/bench/indicator-output-prefix-20261029-smi-pipeline-tuned.txt`
and `dist/bench/talib-all-current-gate-20261029-final-efi-smi-supertrend-talib081.json`.

### AC median-price ring candidate and formula-plan follow-up (2026-10-29; superseded by fair rerun)

The median-price ring was initially selected from an unfair comparison whose
reference grew its output vector and whose candidate cloned the returned
array. Its initial **1.49x/0.98x/1.34x** ratios are superseded by the
preallocated, no-clone rerun below. The TA-Lib gate ratios **0.74x/0.67x/
0.78x** came from the ring build and are not current after its rollback. The
original probe is
`core/examples/ac_median_ring_probe.rs`; the full gate is
`dist/bench/talib-all-current-gate-20261029-ac-median-ring-talib081.json`.

The same release Formula Plan workload remained **60/60** consistent. At 250
rows, warm Plan/Tree ratios were **1.363x** for domestic and **1.591x** for
Pine, so Plan is still slower on short histories. At 5K rows the ratios were
**0.932x/0.785x**, retaining the longer-history lead. Plan used **40%/25%** as
many warm allocations as Tree (domestic/Pine). The Plan stage probe measured
`MA(CLOSE,14)` at **1.396 µs** through FormulaEngine versus **0.974 µs** in Tree,
with a **0.739 µs** wrapper delta; the short-history executor/engine overhead
remains an open optimization target. Evidence:
`dist/bench/formula-mode-workload-20261029-ac-followup.txt` and
`core/examples/formula_plan_stage_probe.rs`.

### AC median-price ring rollback after fair A/B (2026-10-29)

The fair probe now reserves the full reference output and times the public
`Array1` result without cloning it. Under that comparison, the median-ring
candidate was **0.774x/0.870x/0.905x** of the direct outgoing-price calculation
at 1K/100K/1M, with bit-identical results. I removed the median ring and
restored the direct outgoing OHLC reads. The reverted public path then matched
the preallocated reference values exactly; measured times were **3.03/445.20/
4212.83 µs** versus **2.78/441.68/4151.73 µs** for the reference, whose helper
omits the public API validation. The full Plan/Tree workload remains **60/60**
consistent; its aggregate timing is noisy and shows no attributable overall
gain. Evidence: `core/examples/ac_median_ring_probe.rs`,
`dist/bench/ac-median-ring-fair-no-clone.txt`,
`dist/bench/ac-median-ring-reverted.txt`, and
`dist/bench/formula-mode-workload-ac-no-ring.txt`.

### VWMA output initialization and MARKETFI output allocation (2026-10-29)

VWMA now initializes only its warm-up prefix and assigns every computed output
slot directly, including NaN results. A paired release probe produced exact
values and masks and measured **1.13x/1.17x/1.20x** at 1K/100K/1M. In the
Python TA-Lib 0.8.1 gate, VWMA ratios were **1.28x/1.24x/0.97x**; the 1M row
remains slightly below the strict floor. MARKETFI now writes each fully
defined result directly into uninitialized output storage, avoiding an
initializing fill. Its paired release probe was exact and measured
**1.14x/1.82x/1.17x**; the Python gate measured **1.29x/1.00x/0.97x**. Evidence:
`core/examples/vwma_prefix_probe.rs`, `dist/bench/vwma-prefix-20261029.txt`,
`core/examples/marketfi_uninit_probe.rs`, and
`dist/bench/marketfi-prefix-20261029.txt`.

The current release-wheel gate covers **201 functions × 3 sizes (603 rows)**
against TA-Lib 0.8.1. It has **0 execution errors**, the same two ADOSC
tolerance differences at 100K/1M, and geomean speedup **1.6146x**. The strict
per-row speed floor remains false, with **21/33/38** rows below 1.0x at
1K/100K/1M. Formula Plan remains **60/60** consistent, but its warm short
history Plan/Tree ratios are still **1.495x/1.619x** at 250 rows (domestic/
Pine), while 5K rows favor Plan at **0.934x/0.793x**. Evidence:
`dist/bench/talib-all-current-gate-20261029-marketfi-prefix-talib081.json` and
`dist/bench/formula-mode-workload-20261029-vwma-prefix.txt`. Both overall
TA-Lib speed and short-history Formula Plan latency remain active targets.

### Follow-up: extrema kernels and Formula Plan entry overhead (2026-10-29)

Further MAX/MIN trials did not beat the shipped block kernel on the gate input.
A cached-extreme index measured **0.085–0.175x** of the block kernel across
1K/100K/1M. A NaN-transparent fused MINMAX block kernel preserved values and masks,
including leading/interior NaNs and infinities, but measured about
**0.98x/0.94x/0.84x** at 1K/100K/1M against the existing two block passes. Both
candidates were rejected. Evidence: `core/examples/extrema_cached_probe.rs`
and `dist/bench/extrema-kernel-variants-20261029.txt`.

A Formula Plan experiment skipped sandbox-state reset when all limits were
unlimited. The MA/RSI/arithmetic stage probe showed no discernible change, so
the source change was reverted. The 60-case workload remains **60/60**
consistent; the latest run measured warm Plan/Tree at **1.371x/1.624x** for 250
rows and **0.913x/0.792x** for 5K rows (domestic/Pine). These are fresh
workload measurements, not a claimed sandbox speedup. Evidence:
`dist/bench/formula-plan-stage-probe-20261029-sandbox-reset-conditional.txt`
and `dist/bench/formula-mode-workload-20261029-sandbox-reset-conditional.txt`.
The strict all-row TA-Lib speed target and short-history formula target remain
open.

### WAD output writes and refreshed TA-Lib gate (2026-10-29)

WAD now allocates its final output length once, seeds the first NaN, then writes
the cumulative recurrence by index instead of pushing each value. The paired
Rust probe preserved every output bit and measured **1.17x/1.15x/1.04x** at
1K/100K/1M. The rebuilt Python extension kept WAD parity and measured
**1.10x/0.71x/0.90x** against TA-Lib 0.8.1; the 100K Finkit time fell from
**157.35 µs** to **126.19 µs**, and the 1M time from **3,588.66 µs** to
**3,531.56 µs**. WAD remains slower than TA-Lib at 100K and 1M.

The refreshed 201-function / 603-observation gate has **0 execution errors**,
the same two ADOSC tolerance differences at 100K/1M, and geomean **1.6198x**.
The strict floor still fails, with **19/33/35** rows below 1.0x at
1K/100K/1M. The small aggregate change is not attributed to WAD because this
gate is sensitive to run-to-run timing variation. Evidence:
`core/examples/wad_output_probe.rs`, `dist/bench/wad-output-20261029.txt`, and
`dist/bench/talib-all-current-gate-20261029-wad-indexed-talib081.json`.

### HA output writes and indexed-input iteration (2026-10-29)

HA now zips the four equal-length OHLC inputs and all four preallocated output
buffers, avoiding per-row slice indexing while keeping the recurrence and
floating-point operation order unchanged. The paired Rust probe is bit-exact,
including NaN masks, and measured **1.15x/1.03x/1.06x** against the prior
push-based output loop at 1K/100K/1M. The TA-Lib 0.8.1 gate also passes HA
parity; its ratios are **0.74x/0.80x/0.93x**. The comparison across separate
full-gate runs is noisy (notably the TA-Lib 100K HA time moved from 612 to
758 µs), so these ratios do not establish a reliable end-to-end gain by
themselves. The refreshed 201-function / 603-observation gate has **0 execution
errors**, the same two ADOSC tolerance differences, geomean **1.6309x**, and
still misses the strict floor with **17/33/39** rows below 1.0x at 1K/100K/1M.
The aggregate change is not attributed to HA. Evidence:
`core/examples/ha_output_probe.rs`, `dist/bench/ha-output-20261029-iterated.txt`,
and `dist/bench/talib-all-current-gate-20261029-ha-iterated-talib081.json`.

### Formula Plan scalar-only numeric literals (2026-10-30)

Formula Plan now lowers a numeric literal to a first-slot-only write when its
only consumers are known period/scalar operands and the literal is not retained
as a primary result, output channel, or published variable. The backing series
buffer stays fully initialized and full-length; compile-time use analysis proves
that later kernels read only the scalar slot. Literal series used by arithmetic
or exposed through the formula context keep the existing full fill. The release
workload stayed **60/60** consistent at 250 and 5K rows. Two candidate runs
showed only small, mixed Plan/Tree timing movement, so this is recorded as a
targeted reduction in constant-buffer writes, not an aggregate formula speedup.
The stage probe measured FormulaEngine Plan at **1.349 µs** for
`MA(CLOSE,14)` versus Tree at **0.975 µs** on 250 rows, and **7.283/7.173 µs**
on 5K rows; short-history Plan overhead remains open. Evidence:
`core/examples/formula_plan_shape_probe.rs`,
`core/examples/formula_plan_stage_probe.rs`,
`dist/bench/formula-plan-shape-20261030-scalar-number.txt`,
`dist/bench/formula-plan-stage-probe-20261030-scalar-number.txt`, and the two
`dist/bench/formula-mode-workload-20261030-scalar-number*.txt` reports.

### Formula Plan reuses published variable arrays (2026-10-30)

The warm `include_details = false` path now overwrites an already-published
context variable's array when its length and layout match. This avoids allocating
a replacement array for each retained formula variable; the returned primary
series remains independently owned, and missing or differently sized context
variables still receive a fresh array. The complete release workload remained
**60/60** consistent. Average warm allocations fell from **16.3 to 15.3** per
domestic formula and **25.4 to 23.4** per Pine formula. Timings were mixed and
within run-to-run variation: at 250 rows the Plan warm sums were **588.8 µs**
(domestic) and **497.6 µs** (Pine), compared with the pre-change baseline of
**579.0/504.1 µs**. At 5K rows they were **2845.2/1637.4 µs**, versus
**2847.8/1578.3 µs**. This is a measured allocation reduction, not a confirmed
formula runtime speedup. Evidence: `dist/bench/formula-mode-workload-20261030-baseline.txt`
and `dist/bench/formula-mode-workload-20261030-variable-reuse-candidate.txt`.

### Rolling MAX/MIN fused NaN detection (2026-10-30)

For windows up to 512, the rolling extreme path now checks for NaNs while the
Van Herk block kernel reads its suffix/prefix tables. Finite price series avoid
the separate full-input preflight scan; if a NaN appears, the partially written
block output is replaced by the existing missing-aware cached path. Infinities
remain valid extrema, and leading warm-up slots are restored after fallback.
An interleaved same-process comparison against the previous preflight-plus-block
path verifies bit-identical outputs and measures the new path **1.52x/1.52x**
faster for MAX/MIN at 1K, **1.62x/1.61x** at 100K, and **1.33x/1.33x** at 1M.
Two corrected direct C 0.8.1 runs put TA-Lib/Finkit at **1.043–1.066x /
1.025–1.035x** for MAX/MIN at 1K, **1.037–1.087x / 1.122–1.188x** at 100K, and
**0.966–0.982x / 0.959–0.976x** at 1M. The preflight removal is a clear local
gain; the direct comparison still misses the strict per-row target at 1M.
The 201-function Python wheel gate was not rerun because this workspace has no
usable Python runtime; the direct Rust/C probe does not establish Python public
wrapper performance. NaN, infinity, and leading warm-up fallback parity are
checked by the extrema probe. Evidence:
`core/examples/extrema_talib_probe.rs`,
`core/examples/extrema_cached_probe.rs`,
`dist/bench/extrema-talib-20261030-precheck-paired-repeat.txt`,
`dist/bench/extrema-talib-20261030-candidate-repeat2.txt`, and
`dist/bench/extrema-cached-20261030-integrated-detection-repeat.txt`.

### Formula Plan root MA dispatch bypass (2026-10-30)

The compiler recognizes only a root `MA(input, positive-integer-literal)`
shape with one bound series, one output, and no assignments, modifiers, or
string literals. FormulaEngine executes that shape with the same
`moving_avg::sma_into` kernel as the regular Plan dispatcher, skipping the
per-evaluation runtime context and node dispatcher. For `MA(CLOSE,14)`, the
focused release stage probe measured Engine Plan at **0.472 µs** versus **0.616
µs** for the precompiled executor on 250 rows, and **5.480 µs** versus **6.988
µs** on 5K rows. The complete domestic/Pine corpus remained **60/60** consistent
at both sizes. This is a narrowly scoped root-call win: the whole-corpus warm
Plan/Tree aggregate did not improve at 250 rows (**1.488x/1.620x**) and remained
mixed at 5K (**0.942x/0.805x**). It does not close the aggregate short-history
formula gap. Evidence: `core/examples/formula_plan_stage_probe.rs`,
`dist/bench/formula-plan-stage-20261030-simple-ma-fastpath-matching-kernel.txt`,
and `dist/bench/formula-mode-workload-20261030-simple-ma-fastpath-matching-kernel.txt`.

### Formula Plan assigned Coppock dispatch bypass (2026-10-30)

The compiler now recognizes the corpus's exact one-input,
three-positive-integer-literal `NAME:=COPPOCK(input,...)` shape, with the
assigned value as the only retained output. FormulaEngine calls the same
`momentum_ext::coppock` kernel as the canonical formula bridge, preserving
leading warm-up shifting and the all-NaN insufficient-history result. The
focused release probe measured Engine Plan at **1.254 µs** versus **1.758 µs**
for the precompiled executor at 250 rows, and **20.569/27.658 µs** at 5K rows.
The 60-case corpus remained **60/60** consistent. In the repeated 250-row
corpus run, `coppock_cross` was **10.6 µs** on Plan versus **5.5 µs** on Tree
(the preceding run was noisier at 15.5/5.1 µs); aggregate movement remained
mixed, so this is a local formula-shape gain rather than a full-corpus claim.
Evidence: `core/examples/formula_plan_shape_probe.rs`,
`core/examples/formula_plan_stage_probe.rs`,
`dist/bench/formula-plan-stage-20261030-simple-coppock-fastpath.txt`, and
`dist/bench/formula-mode-workload-20261030-simple-coppock-fastpath-repeat.txt`.

### Fused block MINMAX (2026-10-30)

For finite windows up to 512, `minmax` now builds minimum and maximum
prefix/suffix tables together in one block pass. A NaN aborts that pass and
falls back to the existing separate missing-aware extrema kernels; infinities
remain valid values. Against separate Finkit `min` + `max`, the paired block
kernel measured **1.50x** faster at 1K, **1.43x** at 100K, and **1.14–1.16x**
at 1M. Direct TA-Lib C 0.8.1 comparisons passed exact value parity and
measured TA-Lib/Finkit speed ratios of **1.120–1.148x / 1.223–1.224x /
0.983–1.015x**. NaN/Inf fallback and warm-up behavior passed the focused
extrema probe. The 1M MINMAX result straddles the strict 1.0x floor; MAX/MIN
single-output 1M rows also remain near but below it, so the full gate remains
open. Evidence: `core/examples/extrema_talib_probe.rs`,
`core/examples/extrema_cached_probe.rs`,
`dist/bench/extrema-talib-20261030-paired-minmax.txt`, and
`dist/bench/extrema-talib-20261030-paired-minmax-repeat.txt` and
`dist/bench/extrema-cached-20261030-paired-minmax-repeat.txt`.

### Formula runtime empty-state reset (2026-10-29)

`StateArena::clear` now exits when no state objects are live. This avoids
scanning slot capacity retained from earlier plans while preserving that
capacity for reuse. With 4096 reserved but empty slots, the release probe
measured **1,142.7 ns** for the former scan and **31.1 ns** for the early
return (**36.74x**). The targeted unit test confirms repeated clears preserve
capacity. Two complete 60-case Plan/Tree runs stayed **60/60** consistent at
250 and 5K rows. Warm Plan/Tree aggregation was noisy and does not establish an
overall formula speedup: at 250 rows it measured **1.494x/1.355x** domestic and
**1.625x/1.581x** Pine; at 5K it measured **0.947x/0.923x** domestic and
**0.817x/0.811x** Pine. The short-history Plan gap remains open. Evidence:
`core/examples/state_arena_clear_probe.rs`,
`dist/bench/state-arena-clear-20261029-empty.txt`, and
`dist/bench/formula-mode-workload-state-empty-reset.txt` /
`-repeat.txt`.

### Formula Plan output-slot lookup precomputation (2026-10-29)

Each compiled variable binding now stores the first output-vector index for its
buffer slot. Repeated publication skips a linear scan of the output layout;
variables sharing a CSE slot keep resolving to the first matching output, as
before. The focused plan test verifies the stored index when declaration order
differs from primary-output order. The complete release corpus stayed **60/60**
consistent. Warm Plan/Tree measured **1.493x/1.600x** domestic/Pine at 250 rows
and **0.944x/0.816x** at 5K. These totals do not show a stable aggregate speedup;
the change reduces repeated lookup work in variable-heavy plans, while the
short-history Plan gap remains. Evidence: `core/src/formula/hot_plan.rs`,
`core/examples/formula_mode_workload_probe.rs`, and
`dist/bench/formula-mode-workload-precomputed-output-index.txt`.

### Formula Plan direct root ATR path (2026-10-29)

The hot plan records input slots for a pure root
`ATR(HIGH,LOW,CLOSE,positive-integer)` with one retained output and no
assignments, channels, modifiers, or extra expression nodes. On fully finite
inputs FormulaEngine calls the same `volatility::atr_into` kernel used by the
Tree fast path. Non-finite inputs and kernel errors fall back to the general
executor. Tests cover `H/L/C` aliases, rejected wrappers/assignments, exact
finite output parity, and the NaN fallback.

The 250-row stage probe measured Engine Plan at **1.184 µs**, versus **1.114
µs** for the precompiled executor and **1.149 µs** for Tree; an earlier Engine
Plan observation was **1.397 µs**. At 5K, Plan/Tree was **18.967/20.198 µs**,
effectively unchanged from earlier measurements. The full formula run remained
**60/60** consistent but its 250-row aggregate did not improve (**1.520x/1.641x**
Plan/Tree domestic/Pine); this is a local short-history ATR gain, not an overall
corpus speedup. Evidence: `core/src/formula/hot_plan.rs`,
`core/src/formula/engine.rs`, `core/examples/formula_plan_stage_probe.rs`,
`dist/bench/formula-plan-stage-simple-atr-fastpath.txt`, and
`dist/bench/formula-mode-workload-simple-atr-fastpath.txt`.

### Formula Plan ATR preflight threshold (2026-10-29)

The direct ATR path is capped at **1024 rows** because its finite-value
preflight scans all three OHLC inputs. At 5K, the uncapped path measured
**18.967 µs** and added **1.368 µs** over the precompiled executor, so larger
histories now use the regular dispatcher. With the threshold, the stage probe
measured **1.218 µs** at 250 rows and **18.134 µs** at 5K; corresponding Tree
times were **1.125 µs** and **20.350 µs**. The thresholded 60-case workload
remained **60/60** consistent, but warm Plan/Tree was **1.506x/1.647x** at 250
rows and **0.936x/0.800x** at 5K (domestic/Pine), so the aggregate still does
not show a stable improvement. Evidence: `core/src/formula/engine.rs`,
`dist/bench/formula-plan-stage-simple-atr-fastpath-threshold.txt`, and
`dist/bench/formula-mode-workload-simple-atr-fastpath-threshold.txt`.

### Formula Plan direct return for root MA and RSI (2026-10-29)

The primary-only FormulaEngine path now returns eligible root `MA(input,N)`
and `RSI(input,N)` results directly after running the same kernels used by the
existing fast paths. It skips the generic output-vector wrapper and publication
loop. Compile-time shape checks exclude assignments, named channels, modifiers,
and string literals; RSI uses this route only for fully finite input, leaving
non-finite cases on the established dispatcher path. The release stage probe
measured root MA at **0.414 µs** versus Tree at **0.486 µs** for 250 rows and
**5.487/6.664 µs** for Plan/Tree at 5K. Root RSI measured **0.690/0.693 µs** at
250 rows and **11.031/11.036 µs** at 5K. The complete 60-case workload stayed
**60/60** consistent. Its aggregate was noisy: warm Plan/Tree was
**1.534x/1.670x** at 250 rows and **0.950x/0.802x** at 5K (domestic/Pine), so
this confirms local root-call improvements but not an overall short-history
Plan gain. Evidence: `core/src/formula/hot_plan.rs`,
`core/src/formula/engine.rs`, `core/examples/formula_plan_stage_probe.rs`,
`dist/bench/formula-mode-workload-simple-ma-direct-return.txt`, and
`dist/bench/formula-mode-workload-simple-ma-rsi-direct-return.txt`.

A follow-up tried a direct FormulaEngine path for the sole assignment
`ZLEMA20:=ZLEMA(CLOSE,20);`. The synthetic stage probe moved from **1.303 to
0.994 µs** at 250 rows, but corpus `zlema_cross` moved only **10.6 to 10.3 µs**
across separate, noisy workload runs, and the aggregate showed no stable gain.
Reusing the existing context array did not materially change that result, so
the ZLEMA shortcut was removed. Evidence:
`dist/bench/formula-mode-workload-simple-zlema-assignment.txt` and
`dist/bench/formula-mode-workload-simple-zlema-reuse.txt`.

### Single-output Plan retention

Primary-only FormulaEngine calls with exactly one retained output now use
`UnifiedExecutor::execute_primary`, avoiding the one-element outer output
vector. The regular numeric plan, dispatcher, and scratch-buffer behavior are
unchanged. A same-process release A/B measured **1.04–1.17x** direct executor
speed at 250 rows across seven single-output formulas and **1.00–1.02x** at
5K. The complete 60-case Plan/Tree corpus stayed **60/60** consistent. Its
warm Plan/Tree sums were **1.568x/1.601x** at 250 rows and **0.939x/0.803x**
at 5K (domestic/Pine), so the local allocation reduction does not yet establish
a short-history aggregate win. Evidence: `core/src/unified_executor.rs`,
`core/src/formula/engine.rs`, `core/examples/formula_plan_stage_probe.rs`,
`dist/bench/formula-plan-stage-primary-output-20261029.txt`, and
`dist/bench/formula-primary-direct-20261029.txt`.

### Pine CCI primary-only fast path (2026-10-29)

The compiled plan now recognizes the checked-in Pine CCI script's HLC3 source,
single CCI assignment, plot, and scalar hline outputs. Primary-only evaluation
uses the existing `cci_source_into` kernel and preserves the source, CCI, plot,
length, and hline values in the formula context; detail-channel calls retain
the general executor. A focused release comparison verifies exact primary,
variable, output-name, modifier, and string-table parity with Tree at 250 and
5K rows. Plan/Tree warm time is **9.36/11.71 µs** at 250 rows and
**164.34/171.37 µs** at 5K. The complete 60-case corpus remains **60/60**
consistent, but its warm aggregate is mixed at **1.462x/1.587x** Plan/Tree for
250 rows and **0.963x/0.833x** for 5K (domestic/Pine); the short-history
aggregate gap remains open. Evidence: `core/src/formula/hot_plan.rs`,
`core/src/formula/engine.rs`, `core/examples/formula_pine_cci_probe.rs`,
`dist/bench/formula-pine-cci-fastpath.txt`, and
`dist/bench/formula-mode-workload-pine-cci-fastpath.txt`.

### Pine VWMA primary-only fast path (2026-10-29)

The compiled plan recognizes the checked-in Pine VWMA shape (`ta.vwma(close,
length)` plus its scalar length and plot) and routes primary evaluation through
the existing `vwma_into` kernel. Calls requesting detailed channels keep the
general executor. A release comparison verified primary values within the
formula tolerance (`1e-10`) and exact variable, output-name, modifier, and
string-table parity with Tree at 250 and 5,000 rows. The focused probe measured
**1.82x** at 250 rows and **3.05x** at 5,000 rows. The full 60-case workload
remained non-degenerate and consistent at both sizes; the short-history
aggregate is still slower (domestic 1.325x, Pine 1.578x Plan/Tree), while the
5,000-row aggregate is faster (domestic 0.919x, Pine 0.757x). Formula-wide
short-history performance remains an open target. Evidence:
`core/src/formula/hot_plan.rs`, `core/src/formula/engine.rs`,
`core/examples/formula_pine_vwma_probe.rs`,
`dist/bench/formula-pine-vwma-fastpath-release.txt`, and
`dist/bench/formula-mode-workload-pine-cci-vwma-fastpath.txt`.

### Pine Williams %R direct kernel path (2026-10-29)

The Plan compiler now recognizes the checked-in `ta.wpr(length)` formula shape:
one `WILLR(HIGH, LOW, CLOSE, 14)` assignment, its plot, and two scalar hlines.
Primary evaluation uses the existing `willr_into` kernel and retains the usual
context publication path. A focused release probe verified the primary series,
named variables, output names, modifiers, and string table against Tree; Plan
was **1.87x** faster at 250 rows and **2.42x** faster at 5,000 rows. The full
60-case workload stayed **60/60** consistent. Its fresh warm aggregate remains
slower at 250 rows (domestic/Pine Plan/Tree **1.594x/1.582x**) and faster at
5,000 rows (**0.975x/0.823x**), so the overall short-history target remains
open. Evidence: `core/src/formula/hot_plan.rs`, `core/src/formula/engine.rs`,
`core/examples/formula_pine_wpr_probe.rs`,
`dist/bench/formula-pine-wpr-fastpath-release.txt`, and
`dist/bench/formula-mode-workload-pine-cci-vwma-wpr-fastpath.txt`.

### Coppock direct C benchmark refresh (2026-10-29)

A fresh release probe against the linked TA-Lib C 0.8.1 library confirms exact
output parity and Finkit/TA-Lib speed ratios of **0.97x**, **1.02x**, and
**1.06x** at 1K, 100K, and 1M rows for the default 10/11/14 periods. The
previous full release-wheel gate recorded materially slower ratios at 100K
and 1M, so this direct result does not replace that gate; it identifies a
benchmark-build/input discrepancy to resolve before attributing the global
slow row. The fixed-period ring is already active in production. Evidence:
`core/src/indicators/momentum_ext.rs`,
`core/examples/coppock_gate_probe.rs`, and
`dist/bench/coppock-gate-current-followup.txt`.

### Pine ROC direct kernel path (2026-10-29)

The compiled plan now recognizes the checked-in single `ta.roc(close, length)`
formula, including its plot, hline, and scalar length outputs, and routes it
through the existing `roc_into` kernel. With the formula workload's deterministic
OHLCV input and Pine row counts, the release probe verified primary and context
parity within `1e-10`; Plan was **1.29x** faster at 120 rows and **2.14x** at
2,400 rows. The full 60-case workload remained **60/60** consistent in two
runs, but aggregate short-history timings were noisy and did not prove an
overall gain. Formula-wide short-history efficiency remains open. Evidence:
`core/src/formula/hot_plan.rs`, `core/src/formula/engine.rs`,
`core/examples/formula_pine_roc_probe.rs`,
`dist/bench/formula-pine-roc-fastpath-corpus-sizes.txt`,
`dist/bench/formula-mode-workload-pine-cci-vwma-wpr-roc-fastpath.txt`, and
`dist/bench/formula-mode-workload-pine-cci-vwma-wpr-roc-fastpath-repeat.txt`.

### Coppock fused safety-check candidate rejected (2026-10-29)

Moving the fixed-period kernel's finite, magnitude, and denominator checks into
its recurrence removed the separate preflight pass, but repeated input checks
cost more than that pass. The candidate was exact on regular data and declined
zero-denominator, near-zero, non-finite, and extreme inputs so they could fall
back. Against TA-Lib C 0.8.1 it measured **1.26x slower at 1K**, **1.16x at
100K**, and **1.08x at 1M**; the current production entry measured **1.02x
slower**, **1.00x faster**, and **1.00x slower** in the same run. Keep the
existing separate-preflight path. Evidence: `core/examples/coppock_gate_probe.rs`
and `dist/bench/coppock-gate-fused-check-candidate.txt`.

### PVT single-write output path (2026-10-29)

PVT now reserves its output once, writes each result directly into the reserved
slot, and publishes the initialized vector length after the loop. Its existing
floating-point operation order and NaN fallback remain intact. The linked
TA-Lib C 0.8.1 probe reported Finkit/TA-Lib ratios of **1.19x/1.20x** at 1K,
**1.45x/1.65x** at 100K, and **1.12x/1.63x** at 1M in two runs; output parity
was within `1e-8`. Absolute timings varied considerably between runs, so the
ratios are evidence for the current-source comparison, not a claim that the
old wheel gate is aligned with this build. The full all-indicator release gate
still needs a rebuild against the same source and inputs. Evidence:
`core/src/indicators/volume_ext.rs`, `core/examples/pvt_gate_probe.rs`,
`dist/bench/pvt-gate-pointer-write-current.txt`, and
`dist/bench/pvt-gate-pointer-write-current-repeat.txt`.

### Three-period EMA formula fast path (2026-10-29)

The Plan compiler now recognizes exactly three standalone EMA assignments over
one `CLOSE` input, with no extra outputs, expressions, or side effects. For
primary-only evaluation on finite data long enough for all three periods, it
uses `ema_sweep` and puts each result back in its compiled output slot. Other
shapes, non-finite data, short inputs, and detailed-channel calls keep the
general executor. The sweep differs from three independent EMA calls by at most
`1e-10` on the focused probe, below the formula corpus tolerance of `1e-8`.

For the `expma_tdx` fixture shape, a focused release probe moved Plan/Tree from
**3.24/4.40 µs** to **2.47/4.38 µs** at 250 rows and from **50.90/45.24 µs** to
**35.49/44.63 µs** at 5K. Two full 60-case release runs remained **60/60**
consistent at both sizes. Across both corpora, summed warm Plan/Tree fell from
**1.65x** in the preceding run to **1.49x** in both new runs at 250 rows; at 5K
it measured **0.89x** in both runs. Domestic short-history Plan remains slower
than Tree, and the full TA-Lib indicator gate remains open. Evidence:
`core/src/formula/hot_plan.rs`, `core/src/formula/engine.rs`,
`core/examples/multi_ema_formula_probe.rs`,
`dist/bench/formula-multi-ema-probe-20261029.txt`,
`dist/bench/formula-multi-ema-fastpath-release.txt`, and the two
`dist/bench/formula-mode-workload-multi-ema-fastpath` reports.

### AC and candlestick hot-loop follow-up (2026-10-29)

AC now hoists its three period reciprocals out of the fused recurrence and
uses multiplication for each rolling-average calculation. The release probe
preserved the NaN mask and differed from the prior division path by at most
`1.46e-12`; in the full 603-observation TA-Lib 0.8.1 gate, AC measured
**0.79x/0.87x** at 100K/1M rows, up from **0.66x/0.76x** in the immediately
preceding source build. It remains slower than TA-Lib.

`CDL3STARSINSOUTH` now maintains its four overlapping ten-bar means as rolling
sums and no longer allocates full body and range arrays. `CDLEVENINGSTAR`
computes bodies directly on its finite-input path, while retaining the existing
non-finite fallback. The full gate reports output parity for both changes and
ratios of **1.71x/1.85x** for `CDL3STARSINSOUTH` at 100K/1M and **0.96x** for
`CDLEVENINGSTAR` at 1M.

The rebuilt gate covered all **201 functions × 3 sizes** with no execution
errors. Its geometric mean was **1.60x**, but the strict every-row speed gate
remains open; ADOSC retains its two pre-existing numeric tolerance failures.
The remaining slow rows include VWMA, Heikin-Ashi, NVI, CMF, SMI, and Morning
Star. Evidence: `core/src/indicators/talib_ext.rs`,
`core/src/patterns/candlestick.rs`,
`core/examples/ac_median_ring_probe.rs`, and
`dist/bench/talib-all-current-gate-20261029-eveningstar-direct-body.json`.

### VWMA paired NaN scan (2026-10-30)

Both the allocating VWMA API and the caller-output `vwma_into` path now detect
NaNs by checking price and volume together in one traversal. The old expression
could scan the entire price input and then the entire volume input; the rolling
numerator and denominator recurrence is unchanged. A paired release probe
matched the prior path exactly and measured **1.26x/1.17x/1.35x** at 1K/100K/1M
rows. Pine VWMA also remained parity-equal, with Plan/Tree ratios of **2.00x**
at 250 rows and **4.56x** at 5K.

The current Python gate covers all **201 × 3** observations with no execution
errors and preserves the two existing ADOSC tolerance failures. VWMA parity
passes; its fixed-four-thread run measured **1.06x** at 100K and **0.96x** at
1M. The overall strict speed gate remains open. Math-transform timing still
varies sharply between repeated runs, so its **1.39x** whole-suite geometric
mean is not treated as a stable trend. Evidence: `core/src/math/moving_avg.rs`,
`core/examples/vwma_prefix_probe.rs`,
`core/examples/formula_pine_vwma_probe.rs`, and
`dist/bench/talib-all-current-gate-20261030-vwma-single-scan-rayon4.json`.

### Pine RSI primary path

FormulaHotPlan recognizes the corpus Pine RSI shape: a close source, one RSI
call, a period assignment, one plot, and two scalar guide lines. Primary-only
execution calls `rsi_into` and retains the same published variables, output
names, modifiers, and string table. Other formula shapes and detailed-channel
calls continue through the generic plan executor.

The release probe verified exact output/context parity with Tree. Plan/Tree was
**0.54x** at 250 rows and **1.12x** at 5K rows. Native and scaled 60-case
workloads both stayed **60/60** consistent. At the native corpus size, RSI
measured **3.61x Plan/Tree**, down from **4.63x** in the prior run; at 5K it
measured **1.51x**. RSI remains slower than Tree at corpus sizes, while the
full long-history aggregate favors Plan. Evidence:
`core/src/formula/hot_plan.rs`, `core/src/formula/engine.rs`,
`core/examples/formula_pine_rsi_probe.rs`,
`dist/bench/formula-mode-workload-20261030-rsi-fastpath.txt`, and
`dist/bench/formula-mode-workload-20261030-rsi-fastpath-scaled.txt`.

### Large-input VWMA single-pass path

At 500K rows and above, VWMA tracks NaNs during the rolling pass rather than
scanning all price and volume inputs before the calculation. Windows containing
NaNs retain the existing rebuild behavior. The release edge probe verified
`vwma` and `vwma_into` for price NaN, volume NaN, infinity, and zero volume. The
current-source paired comparison measured **1.29x** at 100K and **1.61x** at
1M in one run, with **1.08x** at 1K. These figures compare against the prior
Rust path; the TA-Lib full gate still needs a rebuild against this source.
Evidence: `core/src/math/moving_avg.rs`, `core/examples/vwma_prefix_probe.rs`,
and `dist/bench/vwma-large-single-pass-edge-20261030.txt`.

### Current-source TA-Lib wheel gate

The Python extension was rebuilt from the current checkout with the formula
and Rayon features, then compared with TA-Lib Python 0.8.3 using four Rayon
threads. All 49 current API overlaps were measured at 1K, 100K, and 1M rows
(147 observations). The geometric mean was **1.81x**, with no execution errors.
Strict all-row speed remains open: OBV, DX, VAR, MINUS_DI, RSI, MIDPRICE,
CORREL, CMO, and CCI have at least one row at or below TA-Lib. ADOSC has two
tolerance failures at 100K and 1M; every other comparison passes parity. Evidence:
`dist/bench/talib-all-current-gate-20261030-apo-cmo.json`.

### APO and CMO output initialization

APO now allocates its output without filling every row with NaN first. It fills
the warm-up prefix and writes all later rows in the fused SMA-difference loop.
The release probe matched the materialized two-SMA reference bit-for-bit and
measured **2.94x** at 1M. In the rebuilt TA-Lib gate, APO improved from **0.79x
to 1.08x** at 1M, with parity at all sizes.

CMO and `cmo_fast_into` now initialize only the warm-up prefix and explicitly
write every later result, including NaNs for zero denominators. The rebuilt
gate passes CMO parity at all sizes; its 1M result is **0.96x**. Evidence:
`core/src/indicators/momentum/apo_ppo_impl.rs`,
`core/src/indicators/momentum/cmo_impl.rs`,
`core/examples/apo_init_probe.rs`, and
`dist/bench/talib-all-current-gate-20261030-apo-cmo.json`.

After these indicator-kernel changes, the full formula workload remained
**60/60** consistent at both native and 5K row sizes. Warm Plan/Tree totals
were **1.52x/1.59x** at native size (domestic/Pine) and **0.94x/0.80x** at 5K.
Plan remains slower on the short-history aggregate while its long-history
aggregate is faster. Evidence:
`dist/bench/formula-mode-workload-20261030-apo-cmo.txt`.

### ADOSC parity and expanded current-source gate refresh

ADOSC's recurrence now uses TA-Lib's coefficient construction and fused
multiply-add step. It matches the TA-Lib Python reference exactly at 1K, 100K
and 1M rows; the corresponding speed ratios are **1.09x/0.95x/0.96x**, leaving
large-series latency slightly behind.

The expanded current-source comparison covered 201 functions at three lengths
(603 observations). It had no execution errors or parity failures and a **1.62x**
geometric mean. The strict every-row speed target remains open: 86 observations
were below 1.0x, and 36 TA-Lib functions have no current Finkit counterpart in
the benchmark. The existing 60-case formula workload remains 60/60 consistent;
Plan is faster on its long-history aggregate and remains slower on its native
short-history aggregate. Evidence: `core/src/math/volume_kernels.rs`,
`dist/bench/talib-all-current-gate-20261030-adosc.json`, and
`dist/bench/formula-mode-workload-20261030-apo-cmo.txt`.

### Rolling output writes and primary-only OBV

PERCENTILE and the RMA profile now initialize only the warm-up prefix; the
existing rolling loops write the remaining output directly. Values and NaN
masks stayed equal to TA-Lib. PERCENTILE improved from **0.50x to 0.54x** at
100K and **0.53x to 0.55x** at 1M, so it remains a target for further work. RMA
measured **1.69x/1.59x** at 100K/1M, versus **1.49x/1.22x** in the previous
gate. A per-window `select_nth_unstable` alternative matched values but was
about **4.3–4.9x slower** in the Rust release comparison and was not adopted.

Primary-only formulas recognized as a root OBV call now invoke the OBV kernel
before creating the general executor. The fresh 60-case workload remained
**60/60** consistent. Warm Plan/Tree aggregate ratios were **1.56x/1.64x** at
250 rows (domestic/Pine) and **0.94x/0.78x** at 5K. Short-history Plan execution
still trails Tree, while long-history Plan execution remains faster. Evidence:
`core/src/indicators/talib_ext.rs`, `core/src/formula/engine.rs`,
`core/examples/percentile_select_probe.rs`,
`dist/bench/percentile-output-init-current-20261030.json`,
`dist/bench/percentile-select-candidate-20261030.txt`, and
`dist/bench/formula-mode-workload-20261030-percentile-rma-obv.txt`.

### MINMAXINDEX and full TA-Lib gate refresh

`minmaxindex` now maintains both monotonic queues in one input pass. It matched
TA-Lib exactly at 1K, 100K and 1M rows; its 1M speed ratio rose from **0.66x to
1.70x**. The rebuilt expanded gate covered 201 functions at three sizes (603
observations), with no execution errors or parity failures and a **1.64x**
geometric mean. Below-floor rows decreased from 86 to 79. The strict target
remains open: MAXINDEX at 1M is **0.65x**, PERCENTILE at 1M is **0.56x**, and 36
TA-Lib functions have no counterpart in the current Finkit API. Formula workload
status remains **60/60** consistent; its native short-history Plan aggregate is
still slower than Tree, while the 5K aggregate favors Plan. Evidence:
`core/src/indicators/math_operators.rs`,
`dist/bench/minmaxindex-fused-current-20261030.json`,
`dist/bench/talib-all-current-gate-20261030-minmaxindex.json`, and
`dist/bench/formula-mode-workload-20261030-percentile-rma-obv.txt`.

### CMO recurrence and OBV-to-MA assignment chain (2026-10-30)

CMO now evaluates its signed and absolute Wilder averages directly, avoiding a
temporary change vector and separate gain/loss recurrences. The same-process
release comparison improved the kernel by **1.08x/1.08x/1.11x** at 1K/100K/1M;
the rebuilt public gate is faster than TA-Lib at all three lengths (**1.26x,
1.08x, 1.05x**) with zero parity failures.

Plan also recognizes the common `OBV_VAL:=OBV(CLOSE,VOL); MAOBV:=MA(OBV_VAL,30);`
chain and publishes the same two variables and primary result after direct OBV
and SMA execution. The focused shape check confirms exact Tree parity. Two
release workload runs measured the Plan case at **10.6–10.7 µs**, versus **11.4
µs** in the preceding run. Across the 60-case corpus, native Plan/Tree remains
slower (**1.53x domestic / 1.59x Pine**); at 5K rows Plan remains faster
(**0.93x / 0.79x**). The formula short-history aggregate target remains open.
Evidence: `core/src/indicators/momentum/cmo_impl.rs`,
`core/src/formula/hot_plan.rs`, `core/src/formula/engine.rs`,
`core/examples/cmo_recurrence_probe.rs`,
`core/examples/formula_plan_shape_probe.rs`,
`dist/bench/talib-all-current-gate-20261030-cmo-signedabs.json`,
`dist/bench/cmo-signedabs-paired-repeat-20261030.txt`, and the two
`dist/bench/formula-mode-workload-20261030-obv-ma-fastpath` reports.

### Adaptive formula execution (2026-11-05)

The checked-in 60-case workload shows a crossover: Tree wins on 250-row
histories, while Plan wins on 5K histories (60/60 parity at both sizes). The
engine's default now uses `Auto`: cold formulas use Tree, and repeated calls at
5000 domestic rows or 2400 Pine rows select Plan when plan compilation succeeds.
The Pine benchmark's synthetic history is 120×20 = 2400 rows, even though the
header reports the domestic fixture size. This avoids paying
plan construction for one-shot formulas while retaining the measured
long-history warm-path gain. Unsupported plans fall back to Tree; runtime errors
from a selected Plan remain visible. Explicit `Tree` and `Plan` selections keep
their existing contracts. The workload probe reports Auto timing and allocation
counts alongside both explicit modes.

Two final release runs guided the routing updates. The final 60-case run found
**60/60** Plan/Tree parity at both native and scaled history sizes. At 5K
domestic rows, Auto's warm aggregate was **0.998x** Tree; at 2.4K Pine rows it
was **0.773x**. Together Auto used about **9% less warm time** than Tree across
the long-history corpus. Auto also used about half the Tree allocations in the
Pine corpus. Tree routing now covers measured slower shapes including domestic
ZLEMA, Coppock, AO/AC, fractal, AROON, Keltner, REFDATE, Vortex, and the DZH
TRIX assignment chain, plus Pine rolling-indicator and Donchian cases.
`aroon_cross` Plan/Tree was **0.98x** in the last run. Per-case timings still
show noise (for example, a native-size `bias_tdx` outlier), so the aggregate
measurements are the more reliable comparison.

The raw final release output is
`dist/bench/formula-mode-workload-20261105-auto-final-5.txt`.

### Current-source TA-Lib C comparison (2026-11-05)

The static TA-Lib C benchmark compared **88** supported Finkit/TA-Lib pairs.
The geometric mean speed ratio was **1.746x** in Finkit's favor. **81/88**
pairs were faster; seven were marginally slower: ADXR (**0.932x**), ULTOSC
(**0.937x**), LN (**0.978x**), ADX (**0.982x**), TRIMA (**0.986x**), RSI at
1M rows (**0.994x**), and ACOS (**0.998x**). The AROON public-path correction
also brought AROON and AROONOSC to parity or slightly ahead at 10K rows.

These numbers come from the current Rust source linked against the installed
TA-Lib static library. The expanded Python wheel gate was not rerun because this
environment has no working Python interpreter or `maturin`; earlier 201-function
gate results above are historical and are not claimed as a verification of the
current source. Evidence: `dist/bench/talib-c-current-source-final-20261105.txt`,
`dist/bench/talib-c-current-source-final-20261105-paired.csv`, and
`dist/bench/aroon-age-public-fixed-20261105.txt`.
