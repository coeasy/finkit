# Python Guide

Finkit Python uses PyO3 + maturin and ships `v0.1.3` as platform-specific CPython stable-ABI wheels (`cp38-abi3`). The same wheel for a platform is validated across GIL-enabled CPython 3.8-3.14 on the supported matrix.

## Release status

`v0.1.3` is published on GitHub Releases. The verified wheel families are:

- Linux x86_64 — manylinux 2.17 / manylinux2014;
- Windows x86_64 — `win_amd64`;
- macOS x86_64;
- macOS arm64.

Not in the v0.1.3 wheel matrix: Linux arm64, musllinux, 32-bit Windows, PyPy, and free-threaded CPython.

The GitHub Release is the documented installation source for v0.1.3. Registry publication is separate; do not assume PyPI contains this exact package/version unless independently verified.

## Install a Release wheel

Download the matching `finkit-0.1.3-cp38-abi3-*.whl` from:

`https://github.com/coeasy/finkit/releases/tag/v0.1.3`

Then install:

```bash
python -m pip install --upgrade pip
python -m pip install ./finkit-0.1.3-<matching-platform>.whl
```

Verify outside the source tree:

```bash
cd /tmp  # use another clean directory on Windows
python - <<'PY'
import numpy as np
import finkit as ta

close = np.arange(1.0, 101.0, dtype=np.float64)
rsi = ta.rsi(close, timeperiod=14)
assert len(rsi) == 100
assert np.isfinite(rsi[-1])
print("Finkit OK", rsi[-1])
PY
```

## Build from source

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

Source builds need Rust 1.85+ and the platform's native compiler/linker.

## Input contract

For best performance and predictable behavior:

```python
import numpy as np

close = np.ascontiguousarray(close, dtype=np.float64)
```

Use one-dimensional arrays. OHLCV arrays passed to one computation must have the same length. Rolling outputs preserve input length and normally start with warm-up `NaN` values.

## Indicator examples

```python
import numpy as np
import finkit as ta

close = np.arange(1.0, 101.0, dtype=np.float64)
high = close + 1.0
low = close - 1.0
open_ = close - 0.25
volume = np.full(close.size, 1_000_000.0)

sma20 = ta.sma(close, timeperiod=20)
ema20 = ta.ema(close, timeperiod=20)
rsi14 = ta.rsi(close, timeperiod=14)
macd, signal, hist = ta.macd(close, 12, 26, 9)
atr14 = ta.atr(high, low, close, timeperiod=14)
obv = ta.obv(close, volume)
```

The package wrapper converts public numeric native list/tuple results to NumPy arrays recursively.

## Handling warm-up values

```python
ready = np.isfinite(sma20) & np.isfinite(rsi14)
strategy_signal = np.zeros(close.size, dtype=bool)
strategy_signal[ready] = (close[ready] > sma20[ready]) & (rsi14[ready] > 50)
```

Do not treat leading `NaN` values as a calculation failure; they represent insufficient lookback history for many rolling indicators.

## `CompiledFormula`

Use `CompiledFormula` when the same formula is executed repeatedly. The object keeps the parsed/optimized plan and formula-engine caches alive across calls.

```python
plan = ta.CompiledFormula("MA(CLOSE, 20)")
result = plan.eval(open_, high, low, close, volume)
ma20 = result["__result__"]
```

### `eval()`

`eval()` copies the input arrays into an owned formula context. This is the correct starting point when you plan to call `append_bar()` later.

```python
plan.eval(open_, high, low, close, volume)
plan.reserve_bars(10000)
plan.append_bar(101.0, 103.0, 100.0, 102.5, 1_200_000.0)
latest = plan.eval_last()
```

### `eval_zero_copy()`

`eval_zero_copy()` borrows contiguous `float64` NumPy OHLCV buffers for the synchronous evaluation:

```python
out = plan.eval_zero_copy(open_, high, low, close, volume)
value = out["__result__"]
```

Rules:

- every required input must be a contiguous, one-dimensional `float64` NumPy array;
- all arrays must have equal length and be non-empty;
- keep the arrays alive and do not concurrently resize/mutate them while the call is executing;
- direct fast-path formulas can avoid input materialization, while complex formulas may allocate intermediate arrays;
- `eval_zero_copy()` does not establish the retained streaming context used by `append_bar()`.

### `eval_range()`

Evaluate `[start, end)`:

```python
out = plan.eval_range(
    open_, high, low, close, volume,
    900, 1000,
)
```

The runtime uses dependency/lookback information to include the required prefix conservatively.

### `eval_last()`

With arrays:

```python
latest = plan.eval_last(open_, high, low, close, volume)
```

Or reuse a context created by `eval()` / `eval_range()`:

```python
plan.eval(open_, high, low, close, volume)
latest = plan.eval_last()
```

### `append_bar()`, `reserve_bars()`, `reset()`

```python
plan.eval(open_, high, low, close, volume)
plan.reserve_bars(5000)
plan.append_bar(102.0, 104.0, 101.0, 103.5, 900_000.0)
latest = plan.eval_last()
plan.reset()
```

`reset()` removes the retained market context but does not discard the compiled formula itself.

## Formula result dictionaries

`eval()` can return named formula variables plus `__result__`. Internal common-subexpression variables are filtered from the Python-facing dictionary.

```python
out = plan.eval(open_, high, low, close, volume)
print(out.keys())
print(out["__result__"][-1])
```

See [formula-runtime.md](formula-runtime.md) and [formula-runtime-contract.md](formula-runtime-contract.md) for the detailed contract.

## Pandas

Pandas is optional. Explicit NumPy conversion is the simplest integration:

```python
import pandas as pd
import numpy as np
import finkit as ta

frame = pd.DataFrame({"close": np.arange(1.0, 101.0)})
close = frame["close"].to_numpy(dtype=np.float64, copy=False)
frame["rsi14"] = ta.rsi(close, timeperiod=14)
```

The Python package also contains an optional `TaAccessor`. Install pandas when using/testing it:

```bash
python -m pip install pandas
python -m pytest ffi/python-binding/tests/test_accessor.py -q
```

## Stable exceptions

The Python wrapper exposes:

- `FinkitError`;
- `InsufficientDataError`;
- `InvalidParameterError`;
- `IndicatorNotFoundError`.

Common native validation failures are translated at the package boundary. Invalid MACD periods, invalid period arguments, insufficient data, and empty inputs should be handled explicitly in application code.

## Patterns

```python
doji = ta.cdl_doji(open_, high, low, close)
hammer = ta.cdl_hammer(open_, high, low, close)
engulfing = ta.cdl_engulfing(open_, high, low, close)

heads = ta.detect_head_shoulders(high)
double_tops = ta.detect_double_top(high)
```

Pattern algorithms have lookbacks; an initial region with no pattern signal is expected.

## Build a wheel locally

```bash
cd ffi/python-binding
maturin build --release --locked --out dist --compatibility pypi --interpreter python
```

Install the generated wheel from a clean directory and run tests against the installed package. Avoid verifying from a working directory that can shadow the installed `finkit` package.

## CI release behavior

The Python Wheels workflow:

1. builds the four v0.1.3 platform wheels;
2. installs/tests each platform wheel outside the source tree;
3. reuses the Linux ABI3 wheel across CPython 3.8-3.14 compatibility jobs;
4. validates package version, wheel metadata, and platform coverage;
5. on the explicit release path, builds the `.crate`, Linux CLI, checksum file, and creates/updates the GitHub Release.

A normal pull request does not publish Release assets.

## Troubleshooting

### `is not a supported wheel on this platform`

```bash
python -VV
python -c "import platform; print(platform.system(), platform.machine())"
```

Match OS and CPU architecture. ABI3 spans supported CPython minor versions; it does not span OS/architecture boundaries.

### `ModuleNotFoundError: No module named 'finkit'`

```bash
python -m pip show finkit
python -c "import sys; print(sys.executable)"
```

Confirm that pip and Python use the same environment, and test outside the repository/source package directory.

### NumPy import/ABI failure

```bash
python -m pip install --upgrade pip numpy
python -m pip install --force-reinstall ./finkit-0.1.3-<matching-platform>.whl
```

### `eval_zero_copy()` rejects an array

Normalize it:

```python
arr = np.ascontiguousarray(arr, dtype=np.float64)
```

Also ensure every OHLCV array is one-dimensional and the lengths match.

## Related documentation

- [Complete usage guide](usage.md)
- [Installation](installation.md)
- [Formula engine](formula.md)
- [Formula runtime](formula-runtime.md)
- [Indicators](indicators.md)
- [Python binding source README](../ffi/python-binding/README.md)
