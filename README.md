# Finkit

[![CI](https://github.com/coeasy/finkit/actions/workflows/ci.yml/badge.svg)](https://github.com/coeasy/finkit/actions/workflows/ci.yml)
[![Docs Check](https://github.com/coeasy/finkit/actions/workflows/docs-check.yml/badge.svg)](https://github.com/coeasy/finkit/actions/workflows/docs-check.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE)

Finkit is a high-performance financial technical-analysis, formula, factor, and streaming-computation library written in Rust. It focuses on reusable calculation infrastructure and native-language bindings rather than trading execution, brokerage, or a full research platform.

Current published release: **v0.1.3**.

## What is ready in v0.1.3

- Rust core: batch indicators, streaming indicators, formula engine, factor/runtime infrastructure, transforms, patterns, risk helpers, and optional Polars integration.
- Python: verified ABI3 wheels for CPython 3.8-3.14 on Linux x86_64, Windows x86_64, macOS x86_64, and macOS arm64.
- CLI: verified Linux x86_64 binary in the GitHub Release; source build supported on Rust hosts.
- Rust package: `finkit-0.1.3.crate` Release asset.
- Node.js: native build/test/`npm pack` path verified by CI from source.
- Java/JNI: Maven package/Javadoc, embedded native loader, and runtime smoke test verified by CI from source.
- C/C++: CMake build/test/install SDK path verified by CI from source.
- Generated indicator/function/version metadata guarded by CI.

## Distribution status

The GitHub `v0.1.3` Release is the authoritative distribution for that version. It contains:

- four Python `cp38-abi3` wheels;
- `finkit-0.1.3.crate`;
- `finkit-cli-linux-x86_64`;
- `SHA256SUMS`.

Public package registries are a separate contract. Do **not** assume PyPI, crates.io, npm, Maven Central, NuGet, a public Go module, Android Maven coordinates, or Swift package coordinates are available unless that exact package/version has actually been published and verified.

## Next-release multi-language expansion

The next-release branch expands the permanent multi-language gate beyond Node/Java/C++/Rust. The `Multilang release` workflow now requires target-specific validation for:

| Target | Next-release gate |
| --- | --- |
| Go/CGO | Rust native build, `go test`, external-module example |
| .NET | Rust native build, .NET 8 tests, NuGet RID inspection |
| WebAssembly | real `wasm32-unknown-unknown` release build |
| Android | four NDK ABIs, Gradle AAR build, AAR native payload inspection |
| iOS | arm64 device + universal arm64/x86_64 simulator XCFramework |
| Node.js | native tests + platform/root npm package candidates |
| Java/JNI | packaged native JAR + JVM runtime smoke |
| C/C++ | CMake build/test/install SDK candidate |
| Rust/CLI | crate + Linux CLI packaging |

These are **validation targets**, not retroactive v0.1.3 Release assets. A language moves from “source exists” to “CI validated” only when its final-head job is green, and it becomes a published distribution only after the resulting artifact is released and a clean consumer install succeeds.

## Documentation

Start here:

| Document | Use it for |
| --- | --- |
| [Getting started](docs/getting-started.md) | First successful installation and calculation |
| [Documentation index](docs/README.md) | Full documentation map and support policy |
| [Installation](docs/installation.md) | Release assets, source builds, prerequisites and verification |
| [Complete usage guide](docs/usage.md) | Python/Rust usage, formulas, streaming and runtime conventions |
| [CLI guide](docs/cli.md) | Input formats, commands and CLI troubleshooting |
| [Language bindings](docs/language-bindings.md) | Python/Rust/Node/Java/C/C++/Go/.NET/Android/iOS/WASM support matrix |
| [Runtime and factors](docs/runtime-and-factors.md) | MarketFrame, factor plans, dependency validation and reuse |
| [Troubleshooting](docs/troubleshooting.md) | Installation, data alignment, formula/runtime and cross-language build diagnosis |
| [Python guide](docs/python.md) | NumPy, ABI3 wheels, `CompiledFormula`, pandas and troubleshooting |
| [Indicators](docs/indicators.md) | Indicator reference |
| [Formula engine](docs/formula.md) | Formula syntax, execution and binding-specific debug guidance |
| [API reference](docs/api-reference.md) | Public API overview |
| [Development](docs/development.md) | Build, test, benchmark, package and CI workflow |

Generated files under `docs/generated/`, `docs/indicator_registry.json`, and benchmark baselines are machine-readable/CI contracts and should not be deleted as stale prose.

## Quick start: Python

Download the wheel matching your platform from the `v0.1.3` GitHub Release, then install it locally:

```bash
python -m pip install --upgrade pip
python -m pip install ./finkit-0.1.3-<platform>.whl
```

Example:

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

print("SMA20", sma20[-1])
print("RSI14", rsi14[-1])
print("MACD", macd[-1], signal[-1], hist[-1])
```

Time-series outputs preserve input alignment. Rolling indicators normally contain leading warm-up `NaN` values until enough bars are available.

## Reusable formula execution

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

Reusable plans also support `eval_zero_copy`, `eval_range`, `eval_last`, `append_bar`, `reserve_bars`, and `reset`. See [formula runtime](docs/formula-runtime.md) and [runtime contract](docs/formula-runtime-contract.md).

## Quick start: Rust

Until a public registry package is independently verified, use the release tag or a local path:

```toml
[dependencies]
finkit = { git = "https://github.com/coeasy/finkit", tag = "v0.1.3" }
```

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

## Quick start: CLI

Build from source:

```bash
git clone https://github.com/coeasy/finkit.git
cd finkit
cargo build -p finkit-cli --release --locked
./target/release/finkit-cli --help
```

Examples:

```bash
./target/release/finkit-cli sma --input close.txt --period 20
./target/release/finkit-cli rsi --input close.txt --period 14 --format json
./target/release/finkit-cli atr --input ohlcv.csv --period 14
./target/release/finkit-cli formula "MA(CLOSE, 5)" --input ohlcv.csv
./target/release/finkit-cli streaming ema --input ohlcv.csv --period 20
```

See [docs/cli.md](docs/cli.md) for file formats and command families.

## Core data conventions

Across language bindings:

- bars are ordered oldest -> newest;
- related OHLCV arrays must remain aligned and have compatible lengths;
- rolling calculations preserve alignment with leading warm-up `NaN` values;
- combine multiple indicator outputs with a joint finite-value mask rather than dropping rows independently;
- Python's lowest-overhead path uses contiguous one-dimensional `numpy.float64` arrays;
- zero-copy borrowed inputs must not be resized or mutated concurrently while evaluation is running.

## Multi-language source guides

Binding-specific instructions:

- [Go/CGO](ffi/go-binding/README.md)
- [.NET](ffi/dotnet-binding/README.md)
- [Android](ffi/android-binding/README.md)
- [iOS](ffi/ios-binding/README.md)
- [WebAssembly](wasm/README.md)

See [docs/language-bindings.md](docs/language-bindings.md) for exact support and publication semantics.

## Build and verify the workspace

```bash
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p finkit --locked
cargo test --workspace --doc --locked
python scripts/check_versions.py
python scripts/gen_ssot_docs.py --check
python scripts/check_docs_links.py
```

Multi-language package/target validation is defined in `.github/workflows/multilang-release.yml`.

## Performance

Finkit includes benchmark, zero-allocation, and relative-performance gates. Results are workload-, CPU-, compiler-, and feature-dependent, so checked-in benchmark reports should be treated as measured snapshots rather than universal guarantees.

- [Benchmark summary](docs/benchmark-results.md)
- [TA-Lib comparison methodology](docs/BENCHMARK_VS_TALIB.md)
- [Generated benchmark report](docs/BENCHMARK_REPORT.md)

## License

Finkit is dual-licensed under MIT OR Apache-2.0. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
