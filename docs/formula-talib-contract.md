# Formula / TA-Lib compatibility contract

Finkit keeps the formula runtime and the TA-Lib compatibility catalog as two
separate layers:

1. The profile-only catalog contains 138 public TA-Lib-profile names that are not
   necessarily present in the Core registry. The complete versioned
   dispatcher surface is the union of those names and Core-overlap names,
   currently 201 operations.
2. The runtime-registration bit is derived from the actual formula function
   map. A catalog entry is therefore not an implicit promise that the function
   can already execute.
3. `inspect_formula_compatibility` reports `exact`, `near`, `approximate`,
   `host_required` and `unsupported` per called function. It also reports the
   catalog revision and global runtime coverage.

The current release keeps 138 profile-only catalog entries and reports the
complete 201-name dispatcher surface separately from formula-runtime
registration and binding-level execution. The Python batch binding
also exposes an explicit `talib_compat=True` adapter. It normalizes TA-Lib
lookback/NaN conventions, absolute index outputs, directional-movement
smoothing, AROON output order and PPO moving-average type without changing the
default native finkit behavior.

The differential matrix must be read as three independent gates. The shared
Rust JSON dispatcher currently executes all `201/201` dispatcher names in its
checked-in smoke contract; that is callable/shape coverage only. Numerical parity is
currently pinned by the checked-in 201-indicator golden corpus, not by a claimed
201-function pass. The versioned state is recorded in
`tests/contracts/talib_coverage_matrix_v1.json`. An entry is not promoted to exact parity until its
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
