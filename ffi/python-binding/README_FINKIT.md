# Finkit Python v0.1.0

Finkit is a high-performance financial indicator, factor, and formula computation engine backed by Rust.

## Install

After the v0.1.0 wheel is published or downloaded from the GitHub Release assets:

```bash
pip install finkit
```

For a local wheel:

```bash
pip install ./finkit-0.1.0-*.whl
```

## Quick start

```python
import numpy as np
import finkit

close = np.arange(1.0, 101.0, dtype=np.float64)

sma = finkit.sma(close, 20)
rsi = finkit.rsi(close, 14)
macd, signal, histogram = finkit.macd(close, 12, 26, 9)

print(finkit.__version__)  # 0.1.0
```

## v0.1.0 scope

The first Finkit package focuses on a stable installable foundation:

- Rust high-performance computation core
- technical indicator library
- formula engine integration
- NumPy-compatible Python binding
- Linux/macOS/Windows wheel build pipeline
- version-aligned `0.1.0` public package metadata

The historical `alpha-ta-*` Rust crate names and native `alpha_ta` module are retained as internal compatibility details during the v0.1.x migration. Python users should use the public `finkit` package API.

## Project

Repository: `coeasy/finkit`
