# Getting Started with Finkit v0.1.3

This guide is the shortest path from a fresh machine to a verified Finkit calculation. For the full documentation map, see [README.md](README.md).

## 1. What Finkit is

Finkit is a high-performance financial indicator, formula, factor, and streaming-computation library. It is not a broker, order-routing system, or full research platform.

The current public release is **v0.1.3**.

The GitHub Release currently publishes:

- Python ABI3 wheels for Linux x86_64, Windows x86_64, macOS x86_64, and macOS arm64;
- `finkit-0.1.3.crate`;
- `finkit-cli-linux-x86_64`;
- `SHA256SUMS`.

Node.js, Java/JNI, and C/C++ are validated from source by CI but are not claimed as public registry packages in v0.1.3.

## 2. Core data rules

Before using any API, keep these rules consistent across languages:

1. Time-series arrays are ordered **oldest bar -> newest bar**.
2. Related OHLCV arrays must have the same length.
3. Rolling indicators preserve alignment and normally return leading `NaN` values during warm-up.
4. Do not interpret warm-up `NaN` values as trading signals or calculation failures.
5. When combining indicators, build a validity mask instead of dropping rows independently.
6. For the fastest Python path, use contiguous one-dimensional `numpy.float64` arrays.

## 3. Fastest start: Python

Download the wheel matching your platform from the GitHub `v0.1.3` Release, then install it locally:

```bash
python -m pip install --upgrade pip
python -m pip install ./finkit-0.1.3-<platform>.whl
```

Verify the installation:

```python
import numpy as np
import finkit as ta

close = np.arange(1.0, 101.0, dtype=np.float64)

sma20 = ta.sma(close, timeperiod=20)
rsi14 = ta.rsi(close, timeperiod=14)
macd, signal, hist = ta.macd(
    close,
    fastperiod=12,
    slowperiod=26,
    signalperiod=9,
)

assert len(sma20) == len(close)
assert len(rsi14) == len(close)
assert np.isfinite(sma20[-1])
print("SMA20:", sma20[-1])
print("RSI14:", rsi14[-1])
print("MACD:", macd[-1], signal[-1], hist[-1])
```

For OHLCV indicators:

```python
open_ = close - 0.2
high = close + 1.0
low = close - 1.0
volume = np.full(close.size, 1_000_000.0, dtype=np.float64)

atr14 = ta.atr(high, low, close, timeperiod=14)
adx14 = ta.adx(high, low, close, timeperiod=14)
obv = ta.obv(close, volume)
```

Next: [Python guide](python.md) and [indicator reference](indicators.md).

## 4. Reusable formula execution

Compile formulas once when they will be evaluated repeatedly:

```python
plan = ta.CompiledFormula("MA(CLOSE, 20)")
result = plan.eval(open_, high, low, close, volume)
ma20 = result["__result__"]
```

For synchronous low-overhead evaluation, prepare contiguous arrays and use `eval_zero_copy()`:

```python
open_ = np.ascontiguousarray(open_, dtype=np.float64)
high = np.ascontiguousarray(high, dtype=np.float64)
low = np.ascontiguousarray(low, dtype=np.float64)
close = np.ascontiguousarray(close, dtype=np.float64)
volume = np.ascontiguousarray(volume, dtype=np.float64)

plan = ta.CompiledFormula("MA(CLOSE, 20)")
out = plan.eval_zero_copy(open_, high, low, close, volume)
```

For incremental workloads:

```python
plan.eval(open_, high, low, close, volume)
plan.reserve_bars(10_000)
plan.append_bar(126.0, 128.0, 125.0, 127.5, 1_600_000.0)
latest = plan.eval_last()
```

Next: [formula guide](formula.md), [formula runtime](formula-runtime.md), and [runtime contract](formula-runtime-contract.md).

## 5. Rust

Until a registry package is independently verified, use the release tag or a local path:

```toml
[dependencies]
finkit = { git = "https://github.com/coeasy/finkit", tag = "v0.1.3" }
```

Example:

```rust
use finkit::indicators;
use finkit::math::moving_avg;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let close: Vec<f64> = (1..=100).map(|v| v as f64).collect();
    let sma20 = moving_avg::sma(&close, 20)?;
    let rsi14 = indicators::rsi(&close, 14)?;

    println!("SMA20 = {:?}", sma20.last());
    println!("RSI14 = {:?}", rsi14.last());
    Ok(())
}
```

## 6. CLI

Linux x86_64 can use the release binary directly. Other platforms can build the CLI from source:

```bash
cargo build -p finkit-cli --release --locked
./target/release/finkit-cli --help
```

Examples:

```bash
./target/release/finkit-cli sma --input close.txt --period 20
./target/release/finkit-cli rsi --input close.txt --period 14 --format json
./target/release/finkit-cli atr --input ohlcv.csv --period 14 --format json
./target/release/finkit-cli formula "MA(CLOSE, 5)" --input ohlcv.csv
./target/release/finkit-cli streaming ema --input ohlcv.csv --period 20
```

Next: [CLI guide](cli.md).

## 7. Factor and runtime workflow

The factor/runtime layer is designed around aligned market data and validated dependencies:

1. create or obtain an aligned `MarketFrame`;
2. define/register factors;
3. build a dependency-aware plan;
4. validate the plan;
5. execute against the frame;
6. consume aligned outputs while preserving warm-up semantics.

Cycles, invalid dependencies, and inconsistent input lengths are rejected rather than silently repaired.

Next: [runtime and factor guide](runtime-and-factors.md) and [core contracts](core-contracts.md).

## 8. Other language bindings

Use [language-bindings.md](language-bindings.md) for Node.js, Java/JNI, C/C++, Go, .NET, Android, iOS, and WASM status and build paths.

A source binding being present in the repository does **not** mean its package is already published to npm, Maven Central, NuGet, crates.io, PyPI, or another registry.

## 9. Production checklist

Before integrating Finkit into a production service:

- pin the release/tag/commit;
- verify downloaded Release assets with `SHA256SUMS`;
- preserve input alignment and warm-up semantics;
- benchmark on the target CPU/compiler/runtime;
- use reusable formula plans for repeated calculations;
- do not mutate arrays concurrently while a zero-copy evaluation is borrowing them;
- run the repository validation commands if building from source.

For source-build validation:

```bash
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p finkit --locked
cargo test --workspace --doc --locked
python scripts/check_versions.py
python scripts/gen_ssot_docs.py --check
```

## 10. Where to go next

- [Installation](installation.md)
- [Complete usage patterns](usage.md)
- [Python](python.md)
- [CLI](cli.md)
- [Language bindings](language-bindings.md)
- [Indicators](indicators.md)
- [Formula engine](formula.md)
- [Runtime and factors](runtime-and-factors.md)
- [API reference](api-reference.md)
- [Development](development.md)
