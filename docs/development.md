# Development Guide

This guide describes the development and validation workflow for the Finkit Rust core, formula/runtime layers, generated metadata, native bindings, release packaging, and documentation. The current workspace targets v0.2.0; its candidate artifacts and multi-language validation targets are deliberately described separately from the published v0.1.15 assets.

## 1. Repository layout

| Path | Purpose |
| --- | --- |
| `core/` | Rust calculation engine, indicators, formulas, runtime/factors, streaming |
| `cli/` | `finkit-cli` and schema tooling |
| `ffi/python-binding/` | PyO3/maturin Python binding |
| `ffi/node-binding/` | NAPI-RS Node.js binding |
| `ffi/java-binding/` | Java/JNI + Maven binding |
| `ffi/c-binding/` | C/C++ SDK and CMake package |
| `ffi/go-binding/` | Go/CGO binding and nested Go module |
| `ffi/dotnet-binding/` | .NET/P-Invoke binding and tests |
| `ffi/android-binding/` | Android JNI crate + Gradle AAR project |
| `ffi/ios-binding/` | iOS static library, C module, Swift wrapper, XCFramework build |
| `wasm/` | `wasm32-unknown-unknown` WebAssembly binding |
| `visualization/` | visualization support crate |
| `docs/` | canonical user/API/architecture/generated documentation |
| `scripts/` | version, SSOT generation, benchmark, release/helper scripts |
| `.github/workflows/` | core CI, docs, wheels, multi-language packaging validation |

`docs/README.md` is the canonical documentation index. Completed plans and temporary implementation snapshots should remain in Git history instead of returning as current user documentation.

## 2. Toolchains

### Rust

The workspace MSRV is Rust 1.85+.

```bash
rustc --version
cargo --version
rustup component add rustfmt clippy
```

### Python

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install "maturin>=1.5,<2.0" "numpy>=1.24" pytest
```

### Multi-language tools

Install only what the affected binding requires:

- Node.js 16+ and npm for NAPI-RS;
- JDK 17 + Maven for Java/JNI;
- CMake + C/C++ compiler for the C/C++ SDK;
- Go 1.21+ with CGO for Go;
- .NET 8 SDK for the permanent .NET test gate;
- Android SDK/NDK, Java 17, Gradle 8.7+, `cargo-ndk` for Android;
- macOS/Xcode plus Rust Apple targets for iOS;
- Rust `wasm32-unknown-unknown` target for WASM.

Native bindings also require compiler/linker/runtime architecture compatibility with the process that loads them.

## 3. Baseline repository validation

Before opening a PR that changes Rust/core behavior:

```bash
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p finkit --locked
cargo test --workspace --doc --locked
python scripts/check_versions.py
python scripts/gen_ssot_docs.py --check
python scripts/check_docs_links.py
bash scripts/check_rustdoc.sh
python scripts/check_orphan_scripts.py
python scripts/check_workflow_liveness.py
python scripts/check_script_references.py
```

Do not remove `--locked` from CI-equivalent commands. `Cargo.lock` is part of the reproducibility contract.

The last four are the repository-hygiene gates, also available as
`make check-rustdoc`, `make check-orphans` and `make check-script-refs`:

- `scripts/check_rustdoc.sh` is the enforcement point for ADR 0011 (see the
  policy note at the top of `core/src/lib.rs`). `cargo doc` on its own is *not*
  a gate: rustdoc lints are warn-by-default, so a crate whose API reference is
  full of broken intra-doc links still exits 0. `RUSTFLAGS` does not change
  that — rustdoc reads `RUSTDOCFLAGS`.
- `scripts/check_orphan_scripts.py` fails when a file under `scripts/` has no
  consumer anywhere in the tree. A dead script reads as documentation of a
  workflow that no longer exists, and a second, unused implementation of a gate
  silently lowers the bar for the one that runs. References are resolved to a
  fixed point and at identifier boundaries, so a cluster of scripts that only
  mention each other, or an accidental substring hit, cannot excuse a dead one.
- `scripts/check_workflow_liveness.py` fails when a workflow can never be
  triggered (every trigger branch-filtered, no filtered branch exists, and no
  `workflow_dispatch`/`schedule`/`release`/`workflow_call`). It also reports
  dormant branch filters on workflows that are still reachable, so a stale
  `on:` block is visible without failing the build.
- `scripts/check_script_references.py` fails when a caller names a `scripts/`
  path that is not in the tree — the inverse of the orphan check. A workflow
  step, Makefile recipe or document that invokes a script which does not exist
  has a name and a place in the release checklist, and no implementation.
  Paths that are legitimately absent (planned, or written at run time by the
  caller itself) must be recorded in `RECORDED_MISSING` with a reason.

All three hygiene checks enumerate tracked files with `git ls-files -z`. The
`-z` is required: without it git octal-escapes paths containing non-ASCII
bytes, so `docs/competitive-analysis/finkit-<cjk>.md` silently drops out of the
scan — nine tracked files were invisible that way.

## 4. Version contract

`[workspace.package].version` in the root `Cargo.toml` is the canonical release version. `scripts/check_versions.py` checks release-bearing metadata across Rust, Cargo.lock, Python, Node, Java, .NET, CMake, generated version docs, and release-facing documentation.

```bash
python scripts/check_versions.py
```

The multi-language release workflow also reads the workspace version dynamically and uses it to inspect/package ecosystem artifacts. Do not reintroduce hard-coded package versions into the workflow when a value can come from the canonical workspace version.

Protocol/schema versions should change only when those contracts change; do not mechanically tie unrelated schema versions to the package number.

## 5. Generated SSOT documentation and binding code

Generated metadata includes:

- `docs/indicator_registry.json`;
- `docs/generated/indicators.md`;
- `docs/generated/streaming-indicators.md`;
- `docs/generated/formula-functions.md`;
- `docs/generated/features.md`;
- `docs/generated/error-codes.md`;
- `docs/generated/pine-compatibility.md`;
- `docs/generated/version-matrix.md`.

Validate:

```bash
python scripts/gen_ssot_docs.py --check
```

Some binding wrappers are also generated from registry metadata. Regenerate them through the repository scripts instead of editing generated files by hand.

## 6. Rust core and formulas

Core build/test:

```bash
cargo build -p finkit --release --locked
cargo test -p finkit --locked
cargo bench -p finkit --no-run
```

Rolling calculations must preserve documented alignment, leading warm-up `NaN` behavior, parameter validation, and related OHLCV length invariants.

Formula changes can affect parser, optimizer, bytecode, JIT/SIMD execution, reusable plans, compatibility dialects, bindings, and benchmarks. Cover affected behavior for:

- parse/validation failures;
- execution semantics;
- aliases/terminal compatibility;
- common-subexpression safety;
- repeated compiled-plan execution;
- `eval_range` / `eval_last`;
- append/reset/reserve retained context;
- warm-up/NaN alignment;
- optimizer handling of mutable/side-effecting expressions.

Formula debug coverage is binding-specific. The Go binding currently exposes `FormulaEvalDebugJSON`; do not invent the same wrapper name in another language unless it is actually implemented and tested.

## 7. Factor/runtime development

Maintain these invariants:

- identifiers/dependencies are validated;
- dependency cycles are rejected;
- `MarketFrame` columns remain aligned;
- plans do not silently reindex mismatched input;
- warm-up/missing-value semantics remain explicit;
- failed registry/alias operations do not leave partial mutations.

Use `docs/core-contracts.md` and `docs/runtime-and-factors.md` as public contracts.

## 8. Python binding

Editable native package:

```bash
cd ffi/python-binding
maturin develop --release
cd ../..
python -m pytest ffi/python-binding/tests -q
```

Wheel:

```bash
cd ffi/python-binding
maturin build --release --locked --out dist --compatibility pypi --interpreter python
```

Release-quality validation must install the built wheel into a clean environment so the source checkout cannot shadow it.

## 9. Node.js binding

```bash
cd ffi/node-binding
npm ci
npm run build
npm test
npm pack
```

The release gate must stage the generated native module into the matching platform package as `finkit.node` and pack that package before the root package is treated as a candidate.

## 10. Java/JNI binding

Linux CI-equivalent path:

```bash
cargo build -p finkit-java --release --locked
mkdir -p ffi/java-binding/natives/linux-x86_64
cp target/release/libfinkit_java.so ffi/java-binding/natives/linux-x86_64/
mvn -B -f ffi/java-binding/pom.xml -DskipTests package
```

A release-quality test must inspect the JAR for the native resource and run a real JVM call such as `Indicators.sma(...)` through the packaged loader.

## 11. C/C++ binding

```bash
cargo build -p finkit-ffi --release --locked
cmake -S ffi/c-binding -B build/cpp \
  -DFINKIT_AUTO_BUILD_RS=OFF \
  -DFINKIT_BUILD_TESTS=ON \
  -DFINKIT_BUILD_EXAMPLES=ON \
  -DCMAKE_BUILD_TYPE=Release
cmake --build build/cpp --config Release --parallel 2
ctest --test-dir build/cpp -C Release --output-on-failure
cmake --install build/cpp --config Release --prefix dist/cpp
```

After installation, an external consumer should resolve the SDK with `find_package(finkit CONFIG REQUIRED)`.

## 12. Go/CGO binding

The nested module is:

```text
github.com/coeasy/finkit/ffi/go-binding/go
```

Build/test:

```bash
cargo build -p finkit-go --release --locked
cd ffi/go-binding/go
LD_LIBRARY_PATH="../../../target/release:${LD_LIBRARY_PATH:-}" go test ./...
```

The release gate also creates a temporary external module with a local `replace` and runs `ffi/go-binding/examples/example.go`. This catches module/import errors hidden by same-module tests.

When changing Go APIs, verify CGO compile **and** runtime native-library loading. Do not claim a public `go get` path until a compatible nested-module tag and native delivery strategy are real.

## 13. .NET binding

Build native library:

```bash
cargo build -p finkit-dotnet --release --locked
```

Linux managed/native tests:

```bash
LD_LIBRARY_PATH="$PWD/target/release:${LD_LIBRARY_PATH:-}" \
  dotnet test ffi/dotnet-binding/src/Finkit.Tests/Finkit.Tests.csproj \
  -c Release --framework net8.0
```

For a NuGet candidate, stage native assets into `ffi/dotnet-binding/native/<rid>/native/` and pack. Inspect the resulting `.nupkg` for the correct standard `runtimes/<rid>/native/` entry; a successful `dotnet pack` alone is insufficient.

Linux validation does not prove Windows/macOS RID support. Add native-runner jobs before promoting those RIDs to verified status.

## 14. Android binding

The Android build is intentionally two-stage: Rust NDK native libraries first, Gradle AAR second.

```bash
cargo install cargo-ndk --locked
rustup target add \
  aarch64-linux-android \
  armv7-linux-androideabi \
  x86_64-linux-android \
  i686-linux-android

cargo ndk \
  --platform 24 \
  -t arm64-v8a \
  -t armeabi-v7a \
  -t x86_64 \
  -t x86 \
  -o ffi/android-binding/android/src/main/jniLibs \
  build --release --locked -p finkit-android

cd ffi/android-binding/android
gradle assembleRelease
```

Release validation must inspect the AAR and confirm every advertised ABI contains `libfinkit_android.so`. The Java `Finkit` wrapper auto-loads the native library; there is no `init()` API.

## 15. iOS / Swift binding

Use these Rust targets:

```bash
rustup target add \
  aarch64-apple-ios \
  aarch64-apple-ios-sim \
  x86_64-apple-ios
```

Build:

```bash
bash ffi/ios-binding/build-xcframework.sh
```

The script builds one arm64 device library and two simulator libraries, combines the simulator architectures with `lipo`, and creates `dist/ios/Finkit.xcframework`.

The C module is named `FinkitC`; the Swift wrapper imports it and exposes `Finkit`/`FinkitError`. Historical `alpha_ta_*` C symbols and deprecated `AlphaTA` Swift aliases are preserved for compatibility during this transition.

A package gate should also type-check `Finkit.swift` against the packaged module map, not only verify that `xcodebuild -create-xcframework` succeeds.

## 16. WebAssembly

Host workspace compilation is not proof of WASM support. Build the actual target:

```bash
rustup target add wasm32-unknown-unknown
cargo build -p finkit-wasm --target wasm32-unknown-unknown --release --locked
```

The raw result is `target/wasm32-unknown-unknown/release/finkit_wasm.wasm`. JavaScript glue and browser/npm packaging are additional release stages.

## 17. Benchmarks and performance gates

Useful references:

- `docs/benchmark-results.md`;
- `docs/BENCHMARK_VS_TALIB.md`;
- `docs/BENCHMARK_REPORT.md`;
- benchmark sources under `core/benches/`.

Performance changes must preserve correctness and allocation contracts. Treat checked-in benchmark data as measured snapshots tied to CPU/compiler/features/data rather than universal latency guarantees.

## 18. Documentation rules

When a public contract changes:

- update root `README.md`, `docs/README.md`, installation/language guides, and binding-local README together;
- distinguish **source exists**, **CI validated**, **package candidate**, **GitHub Release asset**, and **public registry package**;
- do not claim a package-manager command until the exact package/version installs from that registry;
- do not generalize one binding's API (for example a debug wrapper) across all languages;
- regenerate SSOT outputs through the generator;
- run `scripts/check_docs_links.py` after moving/deleting docs.

## 19. Pull-request workflow

Before opening/updating a PR:

1. keep the branch focused;
2. run relevant local checks;
3. commit generated output only when source-of-truth changes require it;
4. explain platform limitations and release impact.

Judge checks only for the **final PR head SHA**. Stale runs are not release evidence. The multi-language workflow uses PR-scoped concurrency so obsolete runs are cancelled when a newer head arrives.

Expected workflows can include:

- CI;
- Docs Check;
- Python Wheels;
- Multilang release.

A runner/preflight failure with zero executed steps is not a code-test failure. Read the final head's real job logs before changing code.

## 20. Release workflow

A GitHub Release and an ecosystem registry publication are separate events.

For a release:

1. align version metadata;
2. pass version/generated-doc checks;
3. pass core CI and every advertised language/package gate;
4. build and clean-install/smoke-test intended artifacts;
5. create/update the GitHub Release from the intended main commit;
6. verify tag target, assets, and checksums;
7. publish external registries only through an explicit release/trusted-publishing mechanism;
8. test a clean consumer install from each registry before adding that command to user docs.

For the published v0.1.15 release, the verified base assets are the Python ABI3 wheels, Rust `.crate`, Linux x86_64 CLI, and `SHA256SUMS`. The v0.2.0 workspace produces release candidates; Go/.NET/Android/iOS/WASM candidates only become published support after their target gates and release distribution are proven.
