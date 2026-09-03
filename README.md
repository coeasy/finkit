# Finkit

[![CI](https://github.com/coeasy/finkit/actions/workflows/ci.yml/badge.svg)](https://github.com/coeasy/finkit/actions/workflows/ci.yml)
[![Docs Check](https://github.com/coeasy/finkit/actions/workflows/docs-check.yml/badge.svg)](https://github.com/coeasy/finkit/actions/workflows/docs-check.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE)

Finkit is a high-performance financial technical-analysis and factor-computation library written in Rust. It focuses on reusable indicator kernels, streaming updates, formula execution, factor/runtime infrastructure, and native-language bindings rather than trading execution or a research platform.

Current release: **v0.1.3**.

## What is ready in v0.1.3

- Rust core with batch indicators, streaming indicators, formula engine, factor engine, runtime contracts, transforms, patterns, risk helpers, and optional Polars integration.
- Python ABI3 wheels for CPython 3.8-3.14 on Linux x86_64, Windows x86_64, macOS x86_64, and macOS arm64.
- Linux x86_64 CLI binary in the GitHub Release.
- Rust `.crate` source package in the GitHub Release.
- Node.js, Java/JNI, and C/C++ packaging paths validated by CI from source.
- Canonical indicator/function metadata and generated documentation guarded by CI.

### Distribution status

The GitHub `v0.1.3` Release is the authoritative binary distribution for this version. The release currently contains:

- 4 Python `cp38-abi3` wheels;
- `finkit-0.1.3.crate`;
- `finkit-cli-linux-x86_64`;
- `SHA256SUMS`.

Package-manager publication is intentionally treated separately from repository packaging. **Do not assume PyPI, crates.io, npm, Maven Central, NuGet, or a public Go module is available unless a package is visible in that registry.** For Node.js, Java, C/C++, Go, .NET, Android, iOS, and WASM, use the source-build instructions until a registry or Release asset is explicitly published.

## Documentation

Start with these documents:

| Document | Purpose |
| --- | --- |
| [Documentation index](docs/README.md) | Canonical documentation map and support status |
| [Installation guide](docs/installation.md) | Release assets, source builds, prerequisites, verification |
| [Complete usage guide](docs/usage.md) | Python, Rust, CLI, formula, streaming, factor/runtime, and binding workflows |
| [Python guide](docs/python.md) | ABI3 wheels, NumPy conventions, `CompiledFormula`, troubleshooting |
| [Indicators](docs/indicators.md) | Indicator catalog and parameters |
| [Formula engine](docs/formula.md) | Formula syntax, terminal compatibility, evaluation |
| [Core contracts](docs/core-contracts.md) | Compute plans, FactorPlan, MarketFrame, warm-up/NaN policy |
| [API reference](docs/api-reference.md) | Public API overview |
| [Development guide](docs/development.md) | Build, test, benchmark, package, and CI workflow |

Machine-generated SSOT files under `docs/generated/` and `docs/indicator_registry.json` are part of the validation contract and should not be edited manually unless the generator/source of truth changes.

## Quick start: Python

Download the wheel matching your platform from the [v0.1.3 GitHub Release](https://github.com/coeasy/finkit/releases/tag/v0.1.3), then install it locally:

```bash
python -m pip install --upgrade pip
python -m pip install ./finkit-0.1.3-<platform>.whl
```

Example:

```python
import numpy as np
import finkit as ta

close = np.arange(1.0, 101.0, dtype=np.float64)

sma = ta.sma(close, timeperiod=14)
rsi = ta.rsi(close, timeperiod=14)
macd, signal, hist = ta.macd(
    close,
    fastperiod=12,
    slowperiod=26,
    signalperiod=9,
)

print(sma[-1], rsi[-1], macd[-1])
```

Finkit returns NumPy arrays at the Python package boundary. Rolling indicators intentionally contain leading warm-up `NaN` values until enough input is available.

## Reusable formula execution

For repeated formula evaluation, compile once and reuse the plan:

```python
import numpy as np
import finkit as ta

n = 1000
open_ = np.arange(n, dtype=np.float64)
high = open_ + 1.0
low = open_ - 1.0
close = open_ + 0.5
volume = np.full(n, 1000.0, dtype=np.float64)

plan = ta.CompiledFormula("MA(CLOSE, 20)")
result = plan.eval(open_, high, low, close, volume)
ma20 = result["__result__"]
```

The reusable plan also exposes `eval_zero_copy`, `eval_range`, `eval_last`, `append_bar`, `reserve_bars`, and `reset`. See [docs/usage.md](docs/usage.md) and [docs/formula-runtime.md](docs/formula-runtime.md) for ownership and incremental-execution details.

## Quick start: Rust

The Rust package exists as a workspace crate and as the `finkit-0.1.3.crate` Release asset. Until a registry publication is explicitly available, use the Git tag or a local path.

Git dependency:

```toml
[dependencies]
finkit = { git = "https://github.com/coeasy/finkit", tag = "v0.1.3" }
```

Local checkout:

```toml
[dependencies]
finkit = { path = "../finkit/core" }
```

Example:

```rust
use finkit::indicators;
use finkit::math::moving_avg;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let close = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let sma = moving_avg::sma(&close, 3)?;
    let rsi = indicators::rsi(&close, 3)?;
    println!("SMA last = {:?}, RSI last = {:?}", sma.last(), rsi.last());
    Ok(())
}
```

## Quick start: CLI

Linux x86_64 can use the release binary directly:

```bash
curl -L -o finkit-cli \
  https://github.com/coeasy/finkit/releases/download/v0.1.3/finkit-cli-linux-x86_64
chmod +x finkit-cli
./finkit-cli --help
```

Or build it from source on any supported Rust host:

```bash
git clone https://github.com/coeasy/finkit.git
cd finkit
cargo build -p finkit-cli --release --locked
./target/release/finkit-cli --help
```

For close-only commands, input may be a newline-delimited numeric file or stdin. OHLCV commands accept CSV headers matched case-insensitively to `open`, `high`, `low`, `close`, and `volume`/`vol`; `close` is required.

Examples:

```bash
./target/release/finkit-cli sma --input close.txt --period 20
./target/release/finkit-cli rsi --input close.txt --period 14 --format json
./target/release/finkit-cli atr --input ohlcv.csv --period 14 --format json
./target/release/finkit-cli formula "MA(CLOSE, 5)" --input ohlcv.csv
./target/release/finkit-cli streaming ema --input ohlcv.csv --period 20
./target/release/finkit-cli transform log_return --input close.txt
./target/release/finkit-cli calc MACD --input ohlcv.csv --fast 12 --slow 26 --signal 9
```

See [docs/usage.md](docs/usage.md#cli-usage) for the full command families and CSV rules.

## Source-build status for other bindings

| Binding | v0.1.3 status | Recommended use today |
| --- | --- | --- |
| Python | Release wheels verified | Install a GitHub Release wheel |
| Rust | Core + `.crate` Release asset verified | Git tag/local path; publish to registry only when registry entry exists |
| CLI | Linux x86_64 Release binary verified | Release binary or source build |
| Node.js | Native build/test/npm-pack path verified in CI | Build from `ffi/node-binding`; do not assume npm publication |
| Java | JNI build, Maven package/Javadoc, embedded-native loader smoke verified in CI | Build from `ffi/java-binding`; do not assume Maven Central publication |
| C/C++ | CMake build/test/install packaging verified in CI | Build/install from `ffi/c-binding` |
| Go | Source binding exists; module/distribution layout is not a public v0.1.3 release contract | Source development only |
| .NET | Source binding exists; no verified v0.1.3 NuGet release | Source development only |
| Android/iOS/WASM | Source integration exists but is not part of the v0.1.3 binary release matrix | Development/experimental |

## Core capabilities

Finkit includes:

- batch technical indicators across overlap, momentum, volume, volatility, cycle, statistics, price transforms, market-specific extensions, and patterns;
- incremental streaming indicators with O(1)-style per-bar update paths where supported;
- candlestick and chart-pattern detection;
- formula parsing, optimization, bytecode execution, reusable compiled plans, range/last evaluation, and terminal-compatibility layers;
- dependency-aware factors and factor transforms;
- aligned `MarketFrame` runtime contracts and explicit warm-up/NaN behavior;
- transforms, feature engineering, risk helpers, selectors, and optional Polars integration;
- C ABI plus higher-level language bindings;
- benchmark, memory-regression, dependency-audit, formatting, clippy, docs, and version-consistency CI gates.

The generated registry is the source of truth for exact indicator/function counts; avoid relying on a hard-coded count in downstream integrations.

## Build and verify the workspace

```bash
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p finkit --locked
cargo test --workspace --doc --locked
python scripts/check_versions.py
python scripts/gen_ssot_docs.py --check
```

Additional binding-specific commands are documented in [docs/development.md](docs/development.md).

## Performance

The repository contains real benchmark harnesses and CI performance gates. Results are workload-, CPU-, compiler-, and feature-dependent; use the checked-in benchmark reports as measured snapshots rather than universal performance guarantees.

- [Benchmark summary](docs/benchmark-results.md)
- [TA-Lib comparison methodology](docs/BENCHMARK_VS_TALIB.md)
- [Generated benchmark report](docs/BENCHMARK_REPORT.md)

## License

Finkit is dual-licensed under MIT OR Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
