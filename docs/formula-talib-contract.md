# Formula / TA-Lib compatibility contract

Finkit keeps the formula runtime and the TA-Lib compatibility catalog as two
separate layers:

1. The catalog contains the 161 public TA-Lib Python function names used by
   the differential comparison matrix.
2. The runtime-registration bit is derived from the actual formula function
   map. A catalog entry is therefore not an implicit promise that the function
   can already execute.
3. `inspect_formula_compatibility` reports `exact`, `near`, `approximate`,
   `host_required` and `unsupported` per called function. It also reports the
   catalog revision and global runtime coverage.

The current release keeps 161 catalog entries and reports formula-runtime
registration separately from binding-level execution. The Python batch binding
also exposes an explicit `talib_compat=True` adapter. It normalizes TA-Lib
lookback/NaN conventions, absolute index outputs, directional-movement
smoothing, AROON output order and PPO moving-average type without changing the
default native finkit behavior.

The differential matrix must be read as two independent gates: callable
coverage (`161/161` in the maintained Python environment) and numerical parity
(reported per function; the current release matrix is `161/161` across the
maintained long-input seed/length checks). A catalog entry is not promoted to
exact parity until its
parameter, warm-up, NaN, output-count and differential golden contract is
complete.

Each catalog entry exposes:

- category and expected output count;
- dynamic input/lookback markers where TA-Lib parameters differ by function;
- the warm-up and NaN policy that still requires golden-vector verification;
- `runtime_registered`, which is the only field that claims an executable
  formula adapter exists.

The minimum acceptance matrix for promoting an entry from catalog-only to
runtime-compatible is:

- scalar and multi-output arity checks;
- default and non-default parameter checks;
- warm-up mask and NaN-position equality;
- finite-value tolerance comparison against the pinned TA-Lib Python version;
- range/eval-last/append equivalence when a streaming implementation exists;
- Python, Node and WASM metadata/compatibility serialization checks.

The repository benchmark is intentionally allowed to report precision failures
and unavailable adapters. `--strict` remains a release gate for a claimed full
numeric compatibility release; a normal run is a coverage report and must not
be interpreted as a full-pass result.

## Python batch compatibility mode

```python
results = finkit.compute_indicators(
    close=close,
    high=high,
    low=low,
    volume=volume,
    requests=[("maxindex", [30]), ("ppo", [12, 26, 0])],
    talib_compat=True,
)
```

The flag is deliberately opt-in. Native formulas and charts retain finkit's
existing warm-up and index semantics; migration code can request the TA-Lib
contract at the boundary.
