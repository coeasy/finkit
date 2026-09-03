# Complete Usage Guide

This guide is the practical entry point for using Finkit `v0.1.3`. It focuses on public, currently implemented behavior and distinguishes release artifacts from source-only bindings.

## 1. Data conventions

Finkit computations are time-series operations. Unless a function documents otherwise:

- input arrays are ordered from oldest bar to newest bar;
- related OHLCV arrays must have identical lengths;
- Python numeric inputs should be one-dimensional `numpy.float64` arrays for the lowest-overhead path;
- rolling indicators preserve input length and use leading `NaN` values during their warm-up/lookback region;
- a warm-up `NaN` is not an error; non-finite values appearing after the valid output region begins may indicate invalid input or an algorithm-specific condition;
- volume is a floating-point series at the core/FFI boundary;
- do not silently drop warm-up rows before combining several indicators unless you deliberately realign every series.

Example OHLCV data:

```text
open,high,low,close,volume
100.0,103.0,99.0,102.0,1200000
102.0,104.5,101.0,103.8,1350000
103.8,105.0,102.5,104.1,980000
```

## 2. Python: basic indicators

Install a matching wheel from the GitHub `v0.1.3` Release first; see [installation.md](installation.md).

```python
import numpy as np
import finkit as ta

close = np.asarray(
    [100, 101, 102, 101, 103, 105, 104, 106, 108, 107,
     109, 110, 111, 109, 112, 113, 114, 113, 115, 116,
     118, 117, 119, 120, 121, 123, 122, 124, 125, 126],
    dtype=np.float64,
)

sma = ta.sma(close, timeperiod=5)
ema = ta.ema(close, timeperiod=5)
rsi = ta.rsi(close, timeperiod=14)
macd, signal, hist = ta.macd(
    close,
    fastperiod=12,
    slowperiod=26,
    signalperiod=9,
)

print("SMA", sma[-1])
print("EMA", ema[-1])
print("RSI", rsi[-1])
print("MACD", macd[-1], signal[-1], hist[-1])
```

The Python package wrapper converts native numeric list/tuple outputs to NumPy arrays recursively, so public numeric results are consistent for normal indicator calls.

### OHLCV indicators

```python
open_ = close - 0.3
high = close + 1.0
low = close - 1.0
volume = np.linspace(1_000_000, 1_500_000, close.size, dtype=np.float64)

atr = ta.atr(high, low, close, timeperiod=14)
adx = ta.adx(high, low, close, timeperiod=14)
cci = ta.cci(high, low, close, timeperiod=14)
willr = ta.willr(high, low, close, timeperiod=14)
obv = ta.obv(close, volume)
```

### Multi-output indicators

```python
upper, middle, lower = ta.bollinger_bands(
    close,
    timeperiod=20,
    nbdevup=2.0,
    nbdevdn=2.0,
)

k, d = ta.stoch(
    high,
    low,
    close,
    fastk_period=14,
    slowk_period=3,
    slowd_period=3,
)
```

Check [indicators.md](indicators.md) and the generated [indicator catalog](generated/indicators.md) for the current registry rather than depending on a hard-coded total indicator count.

## 3. Python: warm-up and missing values

Many indicators cannot produce a finite value until enough bars exist.

```python
rsi = ta.rsi(close, timeperiod=14)
valid = np.isfinite(rsi)
first_valid = int(np.argmax(valid)) if valid.any() else None
print("first finite RSI index:", first_valid)
```

When combining signals:

```python
sma20 = ta.sma(close, timeperiod=20)
rsi14 = ta.rsi(close, timeperiod=14)

ready = np.isfinite(sma20) & np.isfinite(rsi14)
signal = np.zeros(close.size, dtype=bool)
signal[ready] = (close[ready] > sma20[ready]) & (rsi14[ready] > 50)
```

This preserves bar alignment and avoids treating the warm-up region as real data.

## 4. Python: formula engine

The formula engine accepts terminal-style expressions such as moving averages and standard-deviation bands.

A reusable plan is preferred when a formula is evaluated repeatedly:

```python
plan = ta.CompiledFormula("MA(CLOSE, 5)")
result = plan.eval(open_, high, low, close, volume)
ma5 = result["__result__"]
```

`eval()` copies input values into an owned formula context so that the same context can later be extended with `append_bar()`.

### Named variables

When the formula creates named variables, the returned dictionary can contain those variables plus the final `__result__` series. The exact grammar is documented in [formula.md](formula.md) and [formula/grammar.md](formula/grammar.md).

### Zero-copy synchronous evaluation

For a synchronous calculation that does not need to retain a streaming context, use `eval_zero_copy()` with contiguous one-dimensional `float64` NumPy arrays:

```python
open_ = np.ascontiguousarray(open_, dtype=np.float64)
high = np.ascontiguousarray(high, dtype=np.float64)
low = np.ascontiguousarray(low, dtype=np.float64)
close = np.ascontiguousarray(close, dtype=np.float64)
volume = np.ascontiguousarray(volume, dtype=np.float64)

plan = ta.CompiledFormula("MA(CLOSE, 20)")
out = plan.eval_zero_copy(open_, high, low, close, volume)
ma20 = out["__result__"]
```

The OHLCV buffers are borrowed for the complete synchronous evaluation. Direct fast-path formulas can avoid input materialization; complex formulas may still allocate intermediate arrays required by built-in function execution. The returned result is owned by Python.

Do not resize or mutate borrowed input arrays from another thread while the call is running.

### Evaluate only a range

```python
plan = ta.CompiledFormula("MA(CLOSE, 20)")
out = plan.eval_range(
    open_, high, low, close, volume,
    900, 1000,
)
tail = out["__result__"]
```

The range is half-open: `[start, end)`. The runtime uses the compiled plan's dependency/lookback information to materialize the required prefix/window conservatively.

### Evaluate only the latest value

With arrays:

```python
latest = plan.eval_last(open_, high, low, close, volume)
```

After `eval()` or `eval_range()` has established a retained context, `eval_last()` can reuse it with no arrays:

```python
plan.eval(open_, high, low, close, volume)
latest = plan.eval_last()
```

### Incremental append

```python
plan = ta.CompiledFormula("MA(CLOSE, 20)")
plan.eval(open_, high, low, close, volume)
plan.reserve_bars(10_000)

plan.append_bar(
    126.0,   # open
    128.0,   # high
    125.0,   # low
    127.5,   # close
    1_600_000.0,  # volume
)
latest = plan.eval_last()
```

`reserve_bars()` is useful when many bars will be appended. `reset()` discards the retained market context while keeping the compiled formula and runtime caches.

See [formula-runtime.md](formula-runtime.md) and [formula-runtime-contract.md](formula-runtime-contract.md) for detailed execution semantics.

## 5. Python: pandas integration

Pandas is optional. The basic and predictable integration is to expose contiguous NumPy views/copies explicitly:

```python
import pandas as pd
import numpy as np
import finkit as ta

frame = pd.DataFrame({
    "close": np.arange(1.0, 101.0),
})

close_np = frame["close"].to_numpy(dtype=np.float64, copy=False)
frame["rsi14"] = ta.rsi(close_np, timeperiod=14)
frame["sma20"] = ta.sma(close_np, timeperiod=20)
```

The package also includes an optional accessor implementation. Install pandas when using or testing that layer:

```bash
python -m pip install pandas
python -m pytest ffi/python-binding/tests/test_accessor.py -q
```

## 6. Python: patterns

Candlestick functions operate on OHLC arrays and conventionally return `100`, `-100`, or `0` pattern markers where applicable.

```python
doji = ta.cdl_doji(open_, high, low, close)
hammer = ta.cdl_hammer(open_, high, low, close)
engulfing = ta.cdl_engulfing(open_, high, low, close)
```

Chart-pattern APIs return their documented detection output, for example:

```python
head_shoulders = ta.detect_head_shoulders(high)
double_top = ta.detect_double_top(high)
double_bottom = ta.detect_double_bottom(low)
```

Pattern detection typically has a lookback region. Do not require a nonzero signal on the first bar.

## 7. Rust usage

Until a crates.io entry is independently published, use the Git tag or a local path.

```toml
[dependencies]
finkit = { git = "https://github.com/coeasy/finkit", tag = "v0.1.3" }
```

### Batch indicators

```rust
use finkit::indicators;
use finkit::math::moving_avg;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let close: Vec<f64> = (1..=100).map(|x| x as f64).collect();

    let sma20 = moving_avg::sma(&close, 20)?;
    let rsi14 = indicators::rsi(&close, 14)?;

    println!("SMA20 last: {:?}", sma20.last());
    println!("RSI14 last: {:?}", rsi14.last());
    Ok(())
}
```

### Public modules

The core crate exposes modules for:

- `indicators` — batch technical indicators;
- `streaming` — incremental indicators;
- `formula` — parser/compiler/runtime;
- `compute` — unified compute/factor plans and execution policies;
- `factors` — dependency-aware factor evaluation;
- `runtime` — aligned market-frame contracts;
- `registry` / `schema` — canonical metadata and machine-readable API schema;
- `transforms` — transformations and pipelines;
- `features` — feature engineering when default indicator/formula features are enabled;
- `patterns` — candlestick/chart patterns;
- `risk`, `selectors`, `sector`, and other finance helpers;
- `polars_ext` when the `finkit-polars` feature is enabled.

Use [core-contracts.md](core-contracts.md) for the stable contracts around FactorPlan and MarketFrame instead of relying on internal implementation details.

### Features

Default features include the standard library, formula support, serde, observability scaffolding, formula JIT/SIMD, and all indicator categories. Optional features include `rayon`, `finkit-polars`, `talib-c`, `nightly-avx512`, `precision-f32`, and profiling/observability options defined in `core/Cargo.toml`.

If minimizing a build, disable defaults only after checking transitive indicator dependencies:

```toml
[dependencies]
finkit = {
  git = "https://github.com/coeasy/finkit",
  tag = "v0.1.3",
  default-features = false,
  features = ["std", "indicators-overlap"]
}
```

## 8. Streaming usage

Streaming indicators are intended for one-bar-at-a-time updates without recalculating the entire history. The exact constructor/update API varies by indicator implementation; use the `StreamingIndicator` trait and the generated [streaming indicator list](generated/streaming-indicators.md) as the current registry.

Key rules:

- initialize with a valid period/configuration;
- push bars in chronological order;
- expect warm-up before a finite value exists;
- do not mix histories from different instruments in one stateful instance unless you reset/recreate it;
- use checkpoint/restore only where the concrete indicator supports the documented serialization/state contract.

For command-line streaming, see the CLI section below.

## 9. Factor engine and runtime

Finkit includes a dependency-aware factor layer and aligned market runtime. The intended flow is:

1. construct/obtain a `MarketFrame` containing aligned market series;
2. register or resolve factor definitions;
3. compile/validate a dependency-aware factor plan;
4. execute the plan against the frame;
5. consume aligned outputs while respecting warm-up/NaN contracts.

The runtime rejects inconsistent lengths/invalid plans rather than silently realigning inputs. Factor dependencies are validated and cycles are rejected.

Because these APIs are architecture-level and evolve more slowly than examples, use [core-contracts.md](core-contracts.md) as the canonical contract reference.

## 10. CLI usage

Build:

```bash
cargo build -p finkit-cli --release --locked
CLI=./target/release/finkit-cli
$CLI --help
```

### Close-only input

Commands such as `sma`, `ema`, `wma`, and `rsi` use the close-input reader. A file is newline-delimited numeric values:

```text
100.0
101.5
102.2
101.8
```

Examples:

```bash
$CLI sma --input close.txt --period 20
$CLI ema --input close.txt --period 20 --format json
$CLI rsi --input close.txt --period 14 --output rsi.csv --format csv
```

When the command's `--input` is optional, stdin can be used:

```bash
printf '100\n101\n102\n103\n104\n' | $CLI sma --period 3
```

### OHLCV CSV

The CSV parser matches headers case-insensitively. `close` is required. `open`, `high`, and `low` are optional at the parser layer and become `NaN` if absent; `volume` or `vol` becomes `0.0` if absent. An indicator that semantically requires OHLCV will still need meaningful corresponding columns.

```text
open,high,low,close,volume
100,103,99,102,1200000
102,105,101,104,1300000
```

```bash
$CLI atr --input ohlcv.csv --period 14 --format json
$CLI adx --input ohlcv.csv --period 14
$CLI cci --input ohlcv.csv --period 20
$CLI obv --input ohlcv.csv
$CLI willr --input ohlcv.csv --period 14
$CLI bbands --input ohlcv.csv --period 20 --stddev 2
$CLI stoch --input ohlcv.csv --fastk-period 14 --slowk-period 3 --slowd-period 3
```

### Formula command

```bash
$CLI formula "MA(CLOSE, 5)" --input ohlcv.csv
$CLI formula --expr "MA(CLOSE,5) + 2*STDDEV(CLOSE,5)" --input ohlcv.csv --format json
```

The CLI exposes a `--dialect` option. The default remains the project's terminal-compatible dialect (`alpha_ta` in the current code); Pine parsing is available through the supported compatibility path. Consult [formula.md](formula.md) and generated compatibility docs before assuming complete source-terminal parity.

### Streaming command

```bash
$CLI streaming sma --input ohlcv.csv --period 20
$CLI streaming ema --input ohlcv.csv --period 20
$CLI streaming macd --input ohlcv.csv --fast-period 12 --slow-period 26 --signal-period 9
```

### Transforms

```bash
$CLI transform log_return --input close.txt
$CLI transform pct_change --input close.txt
$CLI transform zscore --input close.txt --period 20
```

### Feature pack

```bash
$CLI features alpha_pack --input ohlcv.csv --period 14 --format csv
```

### Parameter sweep

```bash
$CLI sweep sma \
  --input close.txt \
  --period-min 5 \
  --period-max 50 \
  --period-step 5 \
  --metric last \
  --format csv
```

Supported metrics in the current CLI include `mean`, `std`, `min`, `max`, `last`, and `slope` where implemented.

### Generic calculator

```bash
$CLI calc SMA --input ohlcv.csv --period 20
$CLI calc MACD --input ohlcv.csv --fast 12 --slow 26 --signal 9 --format json
```

### Patterns

```bash
$CLI pattern --input ohlcv.csv --kind candlestick --name doji --format json
$CLI pattern --input ohlcv.csv --kind chart --name head_shoulders --format json
```

### Formula templates

```bash
$CLI template list
$CLI template search macd
$CLI template info <template-name>
$CLI template render <template-name> --input ohlcv.csv
```

### Chart output

```bash
$CLI chart --input ohlcv.csv --chart-format svg --output chart.svg
$CLI chart --input ohlcv.csv --chart-format html --output chart.html
```

## 11. Node.js source usage

The Node package source is under `ffi/node-binding` and uses NAPI-RS.

```bash
cd ffi/node-binding
npm install
npm run build
npm test
```

The smoke test loads the real native module and checks a calculation. For local use, keep the generated/staged `finkit.node` where the JS loader expects it for the host platform.

Example API shape:

```javascript
const ta = require('./index.js')
const close = [1, 2, 3, 4, 5]
console.log(ta.sma(close, 3))
```

Do not substitute `npm install finkit` in production instructions until npm and all declared native platform packages have actually been published.

## 12. Java source usage

Build the JNI library and JAR as described in [installation.md](installation.md). The Java surface uses `com.finkit.Indicators`.

```java
import com.finkit.Indicators;

public class Example {
    public static void main(String[] args) {
        double[] close = {1, 2, 3, 4, 5};
        double[] sma = Indicators.sma(close, 3);
        System.out.println(sma[sma.length - 1]);
    }
}
```

Native loading supports the explicit `finkit.native.path` mechanism and packaged native resources. A JAR alone is not enough if the required platform native library is missing.

## 13. C/C++ source usage

Build and install the C/C++ SDK through CMake rather than hard-coding a build-tree library path.

```bash
cargo build -p finkit-ffi --release --locked
cmake -S ffi/c-binding -B build/cpp \
  -DFINKIT_AUTO_BUILD_RS=OFF \
  -DFINKIT_BUILD_TESTS=ON \
  -DFINKIT_BUILD_EXAMPLES=ON \
  -DCMAKE_BUILD_TYPE=Release
cmake --build build/cpp --parallel 2
ctest --test-dir build/cpp --output-on-failure
cmake --install build/cpp --prefix "$PWD/dist/cpp"
```

A downstream CMake project should point `CMAKE_PREFIX_PATH` to that install prefix and use the installed Finkit package config. Ownership and buffer rules are documented in [ffi/memory-contract.md](ffi/memory-contract.md); error codes are in [ffi/error-codes.md](ffi/error-codes.md).

## 14. Go/.NET/mobile/WASM

These source bindings are useful for development, but the current v0.1.3 release does not promise package-manager/binary distribution for them. Treat their in-repository READMEs as developer integration notes, verify their native dependencies locally, and do not use registry installation snippets until those packages are actually published.

## 15. Function and indicator discovery

For tooling, generated SDKs, or UI integration, prefer machine-readable metadata instead of scraping Markdown:

- `docs/indicator_registry.json` — indicator registry snapshot;
- `finkit-schema` CLI — function schema export path;
- `docs/generated/formula-functions.md` — generated formula function view;
- `docs/generated/version-matrix.md` — version metadata view.

Build the schema CLI:

```bash
cargo build -p finkit-cli --bin finkit-schema --release --locked
./target/release/finkit-schema --help
```

## 16. Production integration checklist

Before shipping a Finkit integration:

1. Pin a version/tag or checksum instead of tracking an unpinned branch.
2. Verify Release asset checksums using `SHA256SUMS`.
3. Confirm the target OS/architecture is in the verified matrix.
4. Preserve bar ordering and equal OHLCV lengths.
5. Handle warm-up `NaN` values explicitly.
6. Reuse compiled formula plans in hot loops.
7. Use `eval_zero_copy()` only with contiguous `float64` NumPy arrays and synchronous ownership discipline.
8. Use streaming state per independent instrument/time series.
9. Run an integration smoke test using a known input/output before deployment.
10. Benchmark on the target CPU/compiler if performance is part of an SLA.

## 17. Validation commands for contributors

```bash
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p finkit --locked
cargo test --workspace --doc --locked
python scripts/check_versions.py
python scripts/gen_ssot_docs.py --check
```

Python binding:

```bash
python -m pytest ffi/python-binding/tests -q
```

Node binding:

```bash
cd ffi/node-binding
npm install
npm run build
npm test
```

C/C++ and Java build commands are documented in [installation.md](installation.md) and are also exercised by `.github/workflows/multilang-release.yml`.
