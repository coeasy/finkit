# Finkit Python Binding

This directory contains the Python package and PyO3 native extension for Finkit. The `0.4.0` PyPI distribution name is `qx-finkit`; the Python import name remains `finkit`.

## Current release status

The v0.4.0 release adds a panel-factor API and publishes verified CPython ABI3 wheels for:

- Linux x86_64;
- Windows x86_64;
- macOS x86_64;
- macOS arm64.

The wheels use `cp38-abi3` and are validated for GIL-enabled CPython 3.8-3.14 on the supported matrix.

Install a matching wheel from:

`https://github.com/coeasy/finkit/releases/tag/v0.4.0`

```bash
python -m pip install qx-finkit==0.4.0
```

The package exposes the Qlib-compatible `alpha158` and `alpha360` feature sets through `finkit.feature_set(...)`. It also evaluates aligned `time × symbols` panels through built-in libraries and custom formulas. The full WorldQuant 101 formula corpus remains out of the release because redistribution rights have not been confirmed.

```python
import numpy as np
import finkit as ta

timestamps = np.arange(3, dtype=np.int64)
symbols = ["AAA", "BBB"]
close = np.ascontiguousarray([[1., 10.], [2., 20.], [3., 30.]])
values = ta.evaluate_panel_formula(
    "MA(CS_RANK(CLOSE), 2)", timestamps, symbols, {"close": close}
)
assert values.shape == (3, 2)  # time × symbols
```

## Source build

```bash
git clone https://github.com/coeasy/finkit.git
cd finkit
git checkout v0.4.0

python3 -m venv .venv
source .venv/bin/activate  # PowerShell: .\.venv\Scripts\Activate.ps1
python -m pip install --upgrade pip
python -m pip install "maturin>=1.5,<2.0" "numpy>=1.24" pytest

cd ffi/python-binding
maturin develop --release
cd ../..
python -m pytest ffi/python-binding/tests -q
```

## Basic usage

```python
import numpy as np
import finkit as ta

close = np.arange(1.0, 101.0, dtype=np.float64)

sma20 = ta.sma(close, timeperiod=20)
rsi14 = ta.rsi(close, timeperiod=14)
macd, signal, hist = ta.macd(close, 12, 26, 9)

# The same versioned metadata is available to every official binding.
catalog_json = ta.operation_catalog_json()
compatibility_json = ta.formula_compatibility_report_json(
    "MA(CLOSE, 20)", "tdx"
)
```

Rolling indicators normally contain leading warm-up `NaN` values.

## Reusable formulas

```python
open_ = close - 0.25
high = close + 1.0
low = close - 1.0
volume = np.full(close.size, 1_000_000.0)

plan = ta.CompiledFormula("MA(CLOSE, 20)")
result = plan.eval(open_, high, low, close, volume)
ma20 = result["__result__"]
```

Register reusable parameterized combinations once and compile them through the
same optimized runtime:

```python
registry = ta.FormulaRegistry()
registry.register("ZMA", ["X", "N"], "MA(X, N) + EMA(X, N)")
plan = registry.compile("ZMA(CLOSE, 20)")
ma_combo = plan.eval(open_, high, low, close, volume)["__result__"]
```

Available reusable-plan operations include:

- `eval(...)` — owned context, suitable before incremental append;
- `eval_zero_copy(...)` — synchronous borrowed contiguous `float64` NumPy inputs;
- `eval_range(..., start, end, ...)` — half-open range evaluation;
- `eval_last(...)` — latest value, with arrays or retained context;
- `append_bar(open, high, low, close, volume)` — append one bar to retained context;
- `reserve_bars(additional)` — reserve capacity before repeated append;
- `reset()` — discard retained context while keeping the compiled formula.

For zero-copy calls, every OHLCV input must be a one-dimensional contiguous `numpy.float64` array of equal nonzero length.

## Pandas

Pandas is optional:

```bash
python -m pip install pandas
```

The package exports `TaAccessor` and `register_accessor()`. Explicit NumPy conversion remains a predictable integration path:

```python
close = df["close"].to_numpy(dtype=np.float64, copy=False)
df["rsi14"] = ta.rsi(close, timeperiod=14)
```

## Exceptions

The package exports stable wrapper exceptions:

- `FinkitError`;
- `InsufficientDataError`;
- `InvalidParameterError`;
- `IndicatorNotFoundError`.

## Documentation

- [Complete usage guide](../../docs/usage.md)
- [Python guide](../../docs/python.md)
- [Installation guide](../../docs/installation.md)
- [Indicator catalog](../../docs/indicators.md)
- [Formula engine](../../docs/formula.md)
- [Formula runtime contract](../../docs/formula-runtime-contract.md)
