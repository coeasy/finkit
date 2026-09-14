# Formula runtime: zero-copy, ranges, and streaming

The reusable Python plan is finkit.CompiledFormula. Construct it once so parsing,
optimization, bytecode/JIT caches, and scratch buffers are reused:

~~~python
import finkit
plan = finkit.CompiledFormula("MA(CLOSE, 20)")
~~~

## NumPy zero-copy input

Use plan.eval_zero_copy(...) or the top-level
finkit.formula_eval_numpy_zero_copy(...) with contiguous float64, one-dimensional
NumPy arrays:

~~~python
result = plan.eval_zero_copy(open_, high, low, close, volume)["__result__"]
~~~

The zero-copy path borrows all contiguous OHLCV input buffers for the complete
synchronous evaluation; it does not clone the input history into an owned context.
The direct MA/EMA/RSI/BOLLMID kernels also avoid materialising input argument arrays.
The result array is newly allocated, as it must be independent of the input array.
Non-contiguous views are rejected instead of being silently copied. Use
np.ascontiguousarray explicitly when a copy is acceptable.

Complex formulas still allocate owned intermediate argument/result arrays because
the existing built-in function ABI is array-based. This does not copy the original
NumPy OHLCV input into a second history buffer, and the borrowed context cannot
escape the synchronous call.

Before evaluation, `CompiledFormula.analyze()` exposes input dependencies,
lookback, future-data warnings, stateful nodes and streaming suitability.
`CompiledFormula.compatibility_report(terminal)` exposes terminal semantic
policies and per-function exact/near/approximate/host-required status.
`CompiledFormula.metadata(data_len)` exposes the shared result contract:
output names, `float64` dtype, NaN null policy, conservative warm-up and the
first potentially valid row. Compatibility reports also expose the complete
TA-Lib public catalog revision and registered-runtime coverage.

## Range and last-bar evaluation

eval_range(open, high, low, close, volume, start, end) uses a half-open range
[start, end). The engine computes only the dependency window needed by formulas
with a finite lookback; recursive or unknown functions conservatively retain the
full prefix so historical results remain exact.

~~~python
part = plan.eval_range(open_, high, low, close, volume, 1000, 1100)["__result__"]
last = plan.eval_last(open_, high, low, close, volume)
~~~

After a normal eval, the retained stream context can be used without passing
arrays again:

~~~python
plan.reserve_bars(4096)
plan.append_bar(o, h, l, c, v)
last = plan.eval_last()
~~~

For repeated chart windows without retaining a stream context, use
`eval_range_zero_copy(...)`; it borrows contiguous NumPy input and returns only
the requested range.

## Execution reuse

The plan keeps one FormulaEngine alive. Its pooled executor buffers, compiled
bytecode cache, persistent Bytecode VM scratch state, and optimized JIT programs
are reused between calls. append_bar uses capacity-growing Vec storage, so a
sequence of appends is amortized O(1) per bar instead of repeatedly concatenating
the complete history.

## Semantics and limits

- eval_range uses end as an exclusive index and returns a new NumPy array.
- eval_last() without arrays requires a previous eval, eval_range, or appended
  stream context.
- append_bar appends OHLCV; the core `FormulaContext.append_bar_with_amount()`
  keeps an optional amount series aligned and uses NaN when amount is missing.
- CSE only merges pure expression subtrees. Drawing, alert, selection, and other
  side-effecting nodes are not merged.
- Direct `eval_last` append updates for `MA`, `RSI`, and formula-semantic
  `ATR(HIGH, LOW, CLOSE, N)` use dedicated O(1) state; the general streaming
  `ATR` indicator uses Wilder/RMA semantics and is intentionally not reused for
  the formula SMA-TR contract.
