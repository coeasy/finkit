# Finkit Python Binding

This directory contains the Python package and PyO3 native extension for Finkit `v0.1.3`.

## Current release status

The GitHub `v0.1.3` Release contains verified CPython ABI3 wheels for:

- Linux x86_64;
- Windows x86_64;
- macOS x86_64;
- macOS arm64.

The wheels use `cp38-abi3` and are validated for GIL-enabled CPython 3.8-3.14 on the supported matrix.

Install a matching wheel from:

`https://github.com/coeasy/finkit/releases/tag/v0.1.3`

```bash
python -m pip install ./finkit-0.1.3-<matching-platform>.whl
```

Do not assume a PyPI package is available unless the exact registry entry has been independently published and verified.

## Source build

```bash
git clone https://github.com/coeasy/finkit.git
cd finkit
git checkout v0.1.3

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
