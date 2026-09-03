# Finkit Formula Engine Guide

Finkit's formula engine provides a terminal-style financial expression language on top of the Rust computation core. It is intended for technical indicators, derived series, conditions, reusable compiled formulas, and compatibility workflows; it is not a broker, strategy-execution engine, or full TradingView runtime.

For the exact current function list, use [generated/formula-functions.md](generated/formula-functions.md). Do not rely on hard-coded counts in prose documentation.

## 1. Core data variables

The common market-series variables are:

| Variable | Meaning |
| --- | --- |
| `OPEN` | open price |
| `HIGH` | high price |
| `LOW` | low price |
| `CLOSE` | close price |
| `VOLUME` | volume |

Inputs are aligned time series ordered oldest -> newest.

## 2. Basic expressions

Examples:

```text
CLOSE + 1
HIGH - LOW
CLOSE > MA(CLOSE, 20)
VOLUME > REF(VOLUME, 1)
```

Common operator classes include arithmetic, comparison, logical expressions, and conditional expressions supported by the active grammar.

For grammar-level details, see [formula/grammar.md](formula/grammar.md).

## 3. Variables and assignments

Terminal-style formulas can define intermediate variables before producing a result.

Example:

```text
MA5 := MA(CLOSE, 5);
MA20 := MA(CLOSE, 20);
CROSS(MA5, MA20)
```

Named outputs and assignment semantics depend on the parsed formula form. When using the Python `CompiledFormula` API, the returned result dictionary may contain named series plus the final `__result__` series.

## 4. Common function families

Finkit includes formula functions for categories such as:

- moving averages and overlap studies;
- momentum and oscillators;
- volatility;
- volume;
- statistics;
- historical references;
- logical and signal functions;
- mathematical transforms;
- terminal compatibility helpers;
- selected drawing/visualization directives at the formula/compatibility layer.

Because the registry is generated from the source of truth, use:

- [generated/formula-functions.md](generated/formula-functions.md) for formula functions;
- [generated/indicators.md](generated/indicators.md) for indicator metadata;
- [generated/pine-compatibility.md](generated/pine-compatibility.md) for Pine compatibility.

## 5. Historical references

A common historical-reference pattern is:

```text
PREV_CLOSE := REF(CLOSE, 1);
```

Lookback-dependent expressions preserve series alignment. Leading output values may be `NaN` until enough historical data exists.

## 6. Cross and signal conditions

Example:

```text
MA5 := MA(CLOSE, 5);
MA20 := MA(CLOSE, 20);
BUY := CROSS(MA5, MA20);
BUY
```

When combining multiple rolling signals, do not treat warm-up values as valid conditions. Downstream applications should wait until every required series is finite.

## 7. Python formula execution

### One reusable compiled plan

```python
import numpy as np
import finkit as ta

n = 1000
open_ = np.arange(n, dtype=np.float64)
high = open_ + 1.0
low = open_ - 1.0
close = open_ + 0.5
volume = np.full(n, 1000.0, dtype=np.float64)

plan = ta.CompiledFormula("MA(CLOSE, 20)")
result = plan.eval(open_, high, low, close, volume)
ma20 = result["__result__"]
```

Compile once and reuse the plan for repeated calculations rather than reparsing the same formula on every request.

### Zero-copy synchronous evaluation

```python
open_ = np.ascontiguousarray(open_, dtype=np.float64)
high = np.ascontiguousarray(high, dtype=np.float64)
low = np.ascontiguousarray(low, dtype=np.float64)
close = np.ascontiguousarray(close, dtype=np.float64)
volume = np.ascontiguousarray(volume, dtype=np.float64)

out = plan.eval_zero_copy(open_, high, low, close, volume)
```

The OHLCV arrays are borrowed during the synchronous call. Do not resize or mutate them concurrently while evaluation is running.

### Range evaluation

```python
out = plan.eval_range(open_, high, low, close, volume, 900, 1000)
```

The range is half-open: `[start, end)`. The runtime uses plan dependency/lookback information to evaluate the required prefix/window conservatively.

### Latest value and incremental append

```python
plan.eval(open_, high, low, close, volume)
plan.reserve_bars(10_000)
plan.append_bar(126.0, 128.0, 125.0, 127.5, 1_600_000.0)
latest = plan.eval_last()
```

Use `reset()` when you want to discard the retained market context while keeping the compiled formula/runtime caches.

See [formula-runtime.md](formula-runtime.md) and [formula-runtime-contract.md](formula-runtime-contract.md) for the detailed ownership and reuse contract.

## 8. CLI formula execution

```bash
./target/release/finkit-cli formula "MA(CLOSE, 5)" --input ohlcv.csv
./target/release/finkit-cli formula --expr "MA(CLOSE,5) + 2*STDDEV(CLOSE,5)" --input ohlcv.csv --format json
```

See [cli.md](cli.md) for input formats and command behavior.

## 9. Dialects and compatibility

Finkit contains compatibility layers for terminal-style formulas and a deliberately limited Pine Script v5 subset.

Compatibility must be interpreted at several levels:

1. syntax can parse;
2. built-ins are mapped;
3. runtime numerical semantics match;
4. historical/lookahead/repaint behavior matches;
5. chart/strategy side effects match.

Passing level 1 does not imply levels 3-5.

For Pine:

- [formula/pine-grammar.md](formula/pine-grammar.md) describes the supported grammar subset;
- [generated/pine-compatibility.md](generated/pine-compatibility.md) is the generated compatibility matrix;
- [migration/pine-to-finkit.md](migration/pine-to-finkit.md) explains migration boundaries.

Do not publish fixed compatibility percentages in prose unless they are generated from an explicit test corpus and refreshed automatically.

## 10. Pine migration boundaries

Pine concepts such as `strategy()`, alerts, external libraries, unrestricted chart objects, complete repaint semantics, and broker/order execution are not equivalent to formula evaluation.

Move unsupported strategy and application behavior to the host application instead of forcing it into the formula runtime.

## 11. Warm-up and NaN policy

Rolling formula functions generally preserve input length and emit leading `NaN` values while lookback is incomplete.

Correct downstream handling:

```python
ready = np.isfinite(series_a) & np.isfinite(series_b)
signal = np.zeros(len(series_a), dtype=bool)
signal[ready] = series_a[ready] > series_b[ready]
```

Do not independently drop warm-up rows from multiple series because that can destroy bar alignment.

## 12. Performance model

For repeated workloads:

- reuse compiled formula plans;
- use `eval_zero_copy()` when the synchronous borrowing contract fits;
- use `eval_range()` or `eval_last()` when full-history recomputation is unnecessary;
- reserve capacity before many `append_bar()` calls;
- benchmark the real workload and target CPU/compiler/runtime.

Finkit has benchmark, allocation, and relative-performance gates, but measured repository results are not universal latency guarantees.

See [formula-performance.md](formula-performance.md).

## 13. Debugging

Formula-debug support is **binding-specific**, not one universally named cross-language API.

The current Go/CGO source exposes:

```go
debugJSON, err := ta.FormulaEvalDebugJSON(source, open, high, low, close, volume)
```

That wrapper is backed by the Go native binding's `ta_formula_eval_debug` symbol, which calls the core formula engine's `eval_with_debug` path and serializes debugger events as JSON.

Do not infer that Python, Node, Java, .NET, or the C/C++ SDK exposes the same method name or payload without checking that binding's public wrapper. The removed legacy debugger document was incorrect because it generalized debugger calls across every language without verifying those wrappers.

For every binding, the common diagnostic workflow is still:

1. validate the grammar/dialect;
2. reduce the expression to the smallest failing subexpression;
3. confirm every function exists in the generated catalog;
4. verify OHLCV lengths, ordering, and binding-specific dtype/shape requirements;
5. inspect warm-up/lookback alignment;
6. compare the minimal expression against a fixed golden dataset;
7. add assignments and nested functions back one at a time;
8. distinguish syntax support from semantic parity with the source terminal.

See [troubleshooting.md](troubleshooting.md) for formula, Go debug, Python, CLI, native-binding, runtime, Android, iOS, .NET, and WASM diagnosis.

## 14. Templates

Reusable formula patterns are documented in [formula-templates.md](formula-templates.md). Treat templates as examples, not as a substitute for the generated function registry or exact runtime contract.

## 15. Implementation references

The formula implementation lives under `core/src/formula/`, including parser/compiler/runtime, bytecode, optimizer, JIT/SIMD support, Pine compatibility, and related execution infrastructure.

Architecture references:

- [architecture/formula-engine.md](architecture/formula-engine.md)
- [architecture/dataflow.md](architecture/dataflow.md)
- [core-contracts.md](core-contracts.md)
