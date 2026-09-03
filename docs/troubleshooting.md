# Finkit Troubleshooting Guide

This guide separates the published **v0.1.3** distribution contract from the multi-language source/build paths being validated for the next release. Do not infer that a CI artifact is already available from a public package registry.

If you are new to Finkit, start with [Getting started](getting-started.md), then use this guide when installation, data alignment, formulas, CLI input, native bindings, or builds behave unexpectedly.

## 1. Identify the layer that is failing

Before changing code, determine which layer is actually broken:

| Layer | Typical symptom | First check |
| --- | --- | --- |
| Release asset | wheel/binary cannot be installed or started | OS, CPU architecture, Python implementation/version, checksum |
| Python binding | import or NumPy call fails | active interpreter, installed wheel, array dtype/shape/contiguity |
| Indicator | unexpected `NaN` or length | lookback/warm-up and input alignment |
| Formula | parse/compile/evaluation error | grammar, function catalog, OHLCV lengths, minimal expression |
| Runtime/incremental | latest/range result differs | retained context, append order, reset/reuse contract |
| CLI | input rejected or wrong columns | command help, delimiter/header/column requirements |
| Native binding | loader/linker error | target OS/arch/runtime ABI and native library location |
| Source build | Cargo/CMake/Maven/npm/Go/.NET/Gradle failure | required toolchain and documented build directory |
| Docs/metadata | version/catalog mismatch | version, generated SSOT, local-link checks |

Do not treat a packaging or loader failure as an indicator-algorithm failure. Narrowing the layer first usually saves the most time.

## 2. Confirm the version and platform

The authoritative binary/source distribution for v0.1.3 is the GitHub Release. It contains four Python `cp38-abi3` wheels, the Rust crate archive, a Linux x86_64 CLI binary, and `SHA256SUMS`.

Check the local runtime before selecting an asset:

```bash
python -VV
python -c "import platform; print(platform.system(), platform.machine())"
rustc --version
cargo --version
```

A Python ABI3 wheel can support multiple CPython minor versions, but a wheel cannot cross operating systems or CPU architectures. For example, the Windows `win_amd64` wheel is not installable on Linux, and the macOS arm64 wheel is not the same artifact as the macOS x86_64 wheel.

For exact release/build instructions, see [Installation](installation.md).

## 3. Python installation and import problems

### `pip` says the wheel is unsupported

Confirm all three dimensions:

1. CPython is being used rather than a different Python implementation;
2. the wheel OS tag matches the running OS;
3. the wheel architecture matches the Python process architecture.

Then make sure the same interpreter is used for installation and execution:

```bash
python -m pip --version
python -m pip install ./finkit-0.1.3-<platform>.whl
python -c "import finkit; print(finkit)"
```

Prefer `python -m pip` over a bare `pip` command when multiple Python installations are present.

### `ModuleNotFoundError: No module named 'finkit'`

Check which interpreter is active:

```bash
python -c "import sys; print(sys.executable)"
python -m pip show finkit
```

If `pip show` reports nothing, install the release wheel into that interpreter or build the binding from source as described in [python.md](python.md).

### NumPy input is rejected or copied unexpectedly

For the lowest-overhead Python path, use one-dimensional contiguous `numpy.float64` arrays:

```python
import numpy as np

close = np.ascontiguousarray(close, dtype=np.float64)
```

For OHLCV formulas, normalize all inputs consistently:

```python
open_ = np.ascontiguousarray(open_, dtype=np.float64)
high = np.ascontiguousarray(high, dtype=np.float64)
low = np.ascontiguousarray(low, dtype=np.float64)
close = np.ascontiguousarray(close, dtype=np.float64)
volume = np.ascontiguousarray(volume, dtype=np.float64)
```

Do not resize or mutate borrowed arrays concurrently while a synchronous zero-copy evaluation is running.

## 4. Unexpected `NaN` values are often warm-up, not corruption

Rolling indicators preserve time-series alignment. A 20-period calculation normally cannot produce a fully initialized value on the first few bars, so leading values may be `NaN`.

Do not independently drop `NaN` rows from related outputs because that can destroy bar alignment. Use one joint validity mask instead:

```python
import numpy as np

ready = np.isfinite(fast) & np.isfinite(slow)
signal = np.zeros(len(fast), dtype=bool)
signal[ready] = fast[ready] > slow[ready]
```

When results differ from another library or terminal, compare only bars where both implementations have completed their warm-up periods, then check parameter conventions and initialization semantics.

## 5. OHLCV alignment problems

Formula and multi-input indicator calculations assume related arrays refer to the same ordered bars.

Check these invariants before calling Finkit:

```python
assert len(open_) == len(high) == len(low) == len(close) == len(volume)
assert len(close) > 0
```

Bars must be ordered **oldest -> newest**. Do not reverse only one series, and do not independently filter missing values from individual OHLCV columns.

If source data contains gaps, align the market frame in the host application first and apply one consistent missing-data policy before calculation.

## 6. Formula parse, compile, evaluation, and debug failures

Use this sequence:

1. reduce the formula to the smallest expression that still fails;
2. confirm the function exists in [generated/formula-functions.md](generated/formula-functions.md);
3. check grammar in [formula/grammar.md](formula/grammar.md) or the Pine subset in [formula/pine-grammar.md](formula/pine-grammar.md);
4. validate OHLCV lengths and order;
5. restore assignments and nested functions one at a time;
6. compare against a small fixed input dataset before testing large histories.

A minimal Python reproduction looks like this:

```python
import numpy as np
import finkit as ta

n = 64
open_ = np.arange(n, dtype=np.float64)
high = open_ + 1.0
low = open_ - 1.0
close = open_ + 0.5
volume = np.full(n, 1000.0, dtype=np.float64)

plan = ta.CompiledFormula("MA(CLOSE, 5)")
out = plan.eval(open_, high, low, close, volume)
print(out["__result__"][-5:])
```

### Debugger coverage is binding-specific

Do **not** assume one debugger method name exists identically across every language binding. The current Go/CGO source explicitly exposes `FormulaEvalDebugJSON`, backed by the native Go-binding symbol `ta_formula_eval_debug` and the core formula engine's debug event stream.

Example Go shape:

```go
debugJSON, err := ta.FormulaEvalDebugJSON(source, open, high, low, close, volume)
if err != nil {
    panic(err)
}
fmt.Println(debugJSON)
```

Other bindings must be checked against their own public wrapper/API before documenting a matching debug call. The deleted legacy debugger document incorrectly generalized a single conceptual API across Python, Node, Java, Go, .NET, and C/C++.

For portable diagnosis, parser/compiler errors, minimal formulas, generated function catalogs, fixed golden datasets, and the runtime contract remain the common workflow.

## 7. Pine compatibility problems

Finkit's Pine support is a compatibility subset, not a complete TradingView strategy runtime.

Distinguish these questions:

1. can the syntax be parsed?
2. is the built-in mapped?
3. do the numerical semantics match?
4. do historical/repaint semantics match?
5. does the source rely on chart objects, alerts, strategies, orders, or external libraries?

A formula passing parsing does not guarantee complete TradingView behavior. Move broker/order/alert/application behavior into the host application.

Use:

- [generated/pine-compatibility.md](generated/pine-compatibility.md) for the generated compatibility matrix;
- [migration/pine-to-finkit.md](migration/pine-to-finkit.md) for migration boundaries.

## 8. `CompiledFormula`, range, latest-value, and append problems

For repeated workloads, compile once and reuse the plan:

```python
plan = ta.CompiledFormula("MA(CLOSE, 20)")
full = plan.eval(open_, high, low, close, volume)
```

For range evaluation, remember that `[start, end)` is half-open:

```python
part = plan.eval_range(open_, high, low, close, volume, 900, 1000)
```

For incremental use, initialize the retained context before relying on the latest value:

```python
plan.eval(open_, high, low, close, volume)
plan.reserve_bars(10_000)
plan.append_bar(126.0, 128.0, 125.0, 127.5, 1_600_000.0)
latest = plan.eval_last()
```

If an incremental result appears stale or belongs to the wrong market sequence, call `reset()` and rebuild the retained context. Do not mix bars from different instruments or timeframes inside one retained plan state.

See [formula-runtime.md](formula-runtime.md) and [formula-runtime-contract.md](formula-runtime-contract.md) for the exact reuse/ownership contract.

## 9. CLI failures

Start from the command's own help rather than guessing flags:

```bash
./target/release/finkit-cli --help
./target/release/finkit-cli formula --help
./target/release/finkit-cli streaming --help
```

Common failure sources are:

- using a close-only input for a command that requires OHLCV;
- unexpected CSV headers or column order;
- wrong delimiter;
- malformed numeric values;
- a formula function not present in the current generated catalog;
- invoking an old binary while reading documentation for a newer checkout.

Use [cli.md](cli.md) for supported input formats and examples.

## 10. Rust source-build failures

Check the documented Rust toolchain first:

```bash
rustc --version
cargo --version
cargo check --workspace --locked
```

Then isolate the failing package:

```bash
cargo check -p finkit --locked
cargo test -p finkit --locked
cargo build -p finkit-cli --release --locked
```

If the workspace builds without `--locked` but fails with it, do not silently regenerate dependencies in CI. Investigate whether `Cargo.lock` and manifests are out of sync.

For warnings treated as errors, reproduce the repository check:

```bash
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

## 11. Node.js native loading problems

The Node binding is a source-build/CI-verified path in v0.1.3; the documentation does not claim a public npm registry release.

From the repository:

```bash
cd ffi/node-binding
npm install
npm run build
npm test
npm pack
```

If the JavaScript loader cannot find the native module, verify that the platform-specific package/artifact expected by the root loader actually contains `finkit.node` for the current OS and architecture.

## 12. Java/JNI loading problems

The Java binding can load a native library from an explicit `finkit.native.path`, packaged `/natives/<os>-<arch>/...` resources, or the normal system library path fallback.

If a JAR builds but fails at runtime:

1. confirm the native Rust library was built for the same OS/architecture as the JVM;
2. confirm the resource directory name matches the loader's platform mapping;
3. use an absolute explicit native path during diagnosis;
4. only after that diagnose JNI symbol/API issues.

See [installation.md](installation.md) and [language-bindings.md](language-bindings.md).

## 13. C/C++ consumer problems

Prefer the installed CMake package contract instead of hard-coding a guessed shared-library path.

Build/install the SDK, then point `CMAKE_PREFIX_PATH` at the install prefix and consume it with:

```cmake
find_package(finkit CONFIG REQUIRED)
```

For ownership, lifetime, buffer, and error-code rules, use the FFI documentation under `docs/ffi/` rather than inferring them from one example program.

## 14. Go/CGO problems

The Go module lives at `ffi/go-binding/go` and its package import path is:

```text
github.com/coeasy/finkit/ffi/go-binding/go/ta
```

Build the Rust native library before running Go tests:

```bash
cargo build -p finkit-go --release --locked
cd ffi/go-binding/go
LD_LIBRARY_PATH="../../../target/release:${LD_LIBRARY_PATH:-}" go test ./...
```

On macOS use `DYLD_LIBRARY_PATH`; on Windows make the matching DLL discoverable through `PATH` or the executable directory.

If compilation fails before runtime loading, check:

```bash
go env CGO_ENABLED
go env GOOS GOARCH
```

CGO must be enabled and the C/Rust native library architecture must match the Go process. A successful `go build` does not prove the dynamic loader can find `libfinkit_go` at runtime.

## 15. .NET P/Invoke/package problems

The .NET binding targets .NET 6 and .NET 8 and calls the Rust `finkit_dotnet` library through P/Invoke.

For Linux source validation:

```bash
cargo build -p finkit-dotnet --release --locked
LD_LIBRARY_PATH="$PWD/target/release:${LD_LIBRARY_PATH:-}" \
  dotnet test ffi/dotnet-binding/src/Finkit.Tests/Finkit.Tests.csproj -c Release --framework net8.0
```

When creating a NuGet candidate, native libraries must be staged into standard RID paths such as:

```text
runtimes/linux-x64/native/libfinkit_dotnet.so
runtimes/win-x64/native/finkit_dotnet.dll
runtimes/osx-x64/native/libfinkit_dotnet.dylib
runtimes/osx-arm64/native/libfinkit_dotnet.dylib
```

A local `dotnet pack` succeeding is not proof that all RIDs are present. Inspect the `.nupkg` contents and test restore/run from a clean external project before public NuGet publication.

## 16. Android AAR problems

Android uses a Rust JNI library plus a standard Gradle Android Library project.

The expected build order is:

1. build ABI-specific `libfinkit_android.so` files with `cargo-ndk`;
2. stage them below `ffi/android-binding/android/src/main/jniLibs/<abi>/`;
3. run `gradle assembleRelease` from `ffi/android-binding/android`;
4. inspect the AAR for every advertised ABI.

If `System.loadLibrary("finkit_android")` fails, first inspect the AAR/APK rather than changing JNI code. Confirm the device ABI has a matching `jni/<abi>/libfinkit_android.so` entry.

The Java `Finkit` class loads the library automatically; there is no separate `init()` method.

## 17. iOS XCFramework problems

The iOS build uses three Rust targets:

```text
aarch64-apple-ios
aarch64-apple-ios-sim
x86_64-apple-ios
```

The latter two form one universal simulator slice. There is no separate `x86_64-apple-ios-sim` Rust target in the supported build flow.

Build with:

```bash
bash ffi/ios-binding/build-xcframework.sh
```

If `lipo` or `xcodebuild -create-xcframework` fails, verify all three target libraries exist under `target/<rust-target>/release/` and that Xcode command-line tools point at a complete Xcode installation.

The underlying C symbols retain a historical `alpha_ta_*` prefix for ABI compatibility, while the Swift-facing API is `Finkit`.

## 18. WebAssembly problems

A host workspace build does not prove the WASM target works. Validate the real target:

```bash
rustup target add wasm32-unknown-unknown
cargo build -p finkit-wasm --target wasm32-unknown-unknown --release --locked
```

The raw module is `target/wasm32-unknown-unknown/release/finkit_wasm.wasm`. JavaScript glue/package generation is a separate step and must match the intended `web`, `bundler`, or Node environment.

## 19. Registry commands fail

For v0.1.3, GitHub Release assets are authoritative. Source-build or CI packaging paths do **not** imply publication to PyPI, crates.io, npm, Maven Central, NuGet, a public Go module, CocoaPods/SPM, or an Android registry.

Therefore a registry command failing to find Finkit is not automatically a local environment problem. Check [Installation](installation.md), [Language bindings](language-bindings.md), and the actual release assets first.

## 20. Performance is slower than expected

First ensure the comparison measures the same work:

- same input length and dtype;
- same warm-up and output semantics;
- same number of formula parses/compiles;
- same compiler/profile/CPU;
- no accidental language-level list/array conversion inside the timed loop.

For repeated Python formula workloads:

- reuse `CompiledFormula`;
- use contiguous `float64` inputs;
- consider `eval_zero_copy()` when its borrowing contract fits;
- use `eval_range()` / `eval_last()` when full-history recomputation is unnecessary;
- call `reserve_bars()` before large append workloads.

Repository benchmark reports are measured snapshots, not universal latency guarantees. See [formula-performance.md](formula-performance.md) and [benchmark-results.md](benchmark-results.md).

## 21. Documentation or generated metadata is inconsistent

Run the same contract checks used by the repository:

```bash
python scripts/check_versions.py
python scripts/gen_ssot_docs.py --check
python scripts/check_docs_links.py
```

Generated documents and registries should be regenerated from their source of truth rather than hand-edited.

## 22. What to include in a useful bug report

Provide enough information to reproduce the failing layer:

- Finkit version/tag/commit;
- operating system and CPU architecture;
- Python/Rust/Node/Go/.NET/JDK/Gradle/Xcode/CMake version as relevant;
- installation method or exact source-build command;
- smallest input that reproduces the issue;
- exact indicator/formula/CLI call;
- expected result and actual result;
- complete error message/backtrace;
- whether the problem also occurs on a clean checkout/build directory.

For numerical mismatches, include the smallest fixed dataset and identify the reference implementation plus its exact parameter/warm-up semantics. This is much more actionable than a screenshot of the last output value alone.

## Related documentation

- [Getting started](getting-started.md)
- [Installation](installation.md)
- [Complete usage guide](usage.md)
- [Python guide](python.md)
- [CLI guide](cli.md)
- [Formula engine](formula.md)
- [Runtime and factors](runtime-and-factors.md)
- [Language bindings](language-bindings.md)
- [Development](development.md)
