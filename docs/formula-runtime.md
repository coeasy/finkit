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

The direct MA/EMA/RSI/BOLLMID kernels borrow the NumPy buffers and do not create
input Vec/ndarray copies. The result array is newly allocated, as it must be
independent of the input array. Non-contiguous views are rejected instead of being
silently copied. Use np.ascontiguousarray explicitly when a copy is acceptable.

For formulas that contain arbitrary nested function calls, the public formula
function ABI still needs owned argument arrays. The zero-copy API keeps the input
boundary borrowed and falls back to the regular ABI for those intermediate arrays.

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
- append_bar currently appends OHLCV; append amount through a new full-context
  evaluation when an amount series is required.
- CSE only merges pure expression subtrees. Drawing, alert, selection, and other
  side-effecting nodes are not merged.
