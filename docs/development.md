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

`docs/README.md` is the canonical documentation index. The single current execution baseline is [`FINKIT_ARCHITECTURE_AND_REFACTOR_PLAN_V5.md`](FINKIT_ARCHITECTURE_AND_REFACTOR_PLAN_V5.md). Completed plans, dated audits and temporary implementation snapshots stay in Git history instead of returning as current user documentation: `docs/FINKIT_ARCHITECTURE_AND_OPTIMIZATION_PLAN_V4.md`, `docs/refactor-plan-2026-09-21.md`, `docs/runtime-carrier-adoption-plan-2026-09-20.md`, the `docs/competitive-analysis/` set and several dated roadmaps were deleted from the working tree on 2026-10-10 and are recoverable with `git log --diff-filter=D -- <path>`.

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
# NOT `--all-features`: `finkit` declares `std` and `no_std` as mutually
# exclusive (enforced by a `compile_error!` in `core/src/lib.rs`), so
# `--all-features` guarantees a failure -- 88 errors, including the one it is
# supposed to produce. And not `-D warnings`: the tree carries a *tracked
# warning budget* (`scripts/check_warning_budget.py`) rather than a zero-warning
# policy. This mirrors the CI `clippy` job, including the one lint it denies.
RUSTFLAGS="-D unfulfilled_lint_expectations" cargo clippy --workspace --all-targets --locked
# The supported feature subsets (and the recorded-broken ones) are pinned by
# `make check-feature-matrix`, which is minutes long; see its own section below.
cargo test -p finkit --locked
cargo test --workspace --doc --locked
# The CI package set, plus the binding crates CI only compiles. `--no-fail-fast`
# matters: without it the first failing binary hides every later one.
cargo test -p finkit -p finkit-factor-analysis -p finkit-cli \
  -p finkit-visualization -p finkit-ffi-common -p finkit-ffi \
  --locked --no-fail-fast
cargo test -p finkit-go -p finkit-dotnet -p finkit-java \
  -p finkit-android -p finkit-ios -p finkit-wasm --locked --no-fail-fast
python scripts/check_versions.py
python scripts/gen_ssot_docs.py --check
python scripts/check_docs_links.py
bash scripts/check_rustdoc.sh
python scripts/check_orphan_scripts.py
python scripts/check_workflow_liveness.py
python scripts/check_orphan_docs.py
python scripts/check_script_references.py
python scripts/check_dead_code_allows.py
python scripts/check_ios_header_contract.py
python scripts/check_talib_ffi_contract.py
python scripts/check_feature_matrix.py     # slow: one `cargo check` per subset
```

Do not remove `--locked` from CI-equivalent commands. `Cargo.lock` is part of the reproducibility contract.

The last eight are the repository-hygiene gates, also available as
`make check-rustdoc`, `make check-orphans` (scripts, workflows, documents),
`make check-script-refs`, `make check-dead-code`, `make check-ios-header` and
`make check-talib-ffi`:

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
- `scripts/check_orphan_docs.py` fails when a tracked Markdown document is
  reachable from no other document, by link or by backticked
  repository-relative path. A document nothing links to is unreachable from the
  index and indistinguishable from a deleted one; it found two current
  documents in that state.
- `scripts/check_script_references.py` fails when a caller names a `scripts/`
  path that is not in the tree — the inverse of the orphan check. A workflow
  step, Makefile recipe or document that invokes a script which does not exist
  has a name and a place in the release checklist, and no implementation.
  Paths that are legitimately absent (planned, or written at run time by the
  caller itself) must be recorded in `RECORDED_MISSING` with a reason.
- `scripts/check_ios_header_contract.py` compares
  `ffi/ios-binding/include/finkit.h` with the iOS binding's shipped
  `#[no_mangle] extern "C"` exports, in both directions, ignoring
  `#[cfg(test)]`-gated symbols. It is the iOS counterpart to
  `gen_c_header.py --check`; before it existed the header had drifted by three
  undeclared entry points.
- `scripts/check_talib_ffi_contract.py` diffs the hand-written TA-Lib
  transcription in `core/src/talib_ffi.rs` against the pinned TA-Lib catalog in
  `tests/contracts/talib_coverage_matrix_v1.json`. The transcription is compiled
  only under the `talib-c` feature, and an unused `extern` declaration never
  reaches the linker — so it advertised `TA_SKEWNESS`/`TA_KURTOSIS`, which TA-Lib
  does not export at any version, while the benchmark stayed green. The same
  check verifies the header's stated indicator total and each section's
  `[declared=N]` count, because three hand-written numbers in that file
  disagreed with each other and with the declarations.
- `scripts/check_dead_code_allows.py` fails on any `#[allow(dead_code)]` (or
  `#![allow(dead_code)]`, or the `expect` form) that does not carry a `// why`
  comment on the line, inside the attribute block above it, or on the line
  directly above that block. That attribute disables the only compiler check
  that can see orphan logic, so an item behind it is invisible to every build
  and every test. Most suppressions here are legitimate — a `#[cfg]`-gated
  fallback the current build does not select, or a legacy spelling kept for one
  release — but the reason has to be written down, because a bare suppression
  and a function that was never wired up look identical. Five of them turned
  out to be the latter and were deleted.
- `scripts/check_warning_contracts.py` also protects the Java JNI panic boundary:
  the six generated multi-output functions that return `()` must use
  `ffi_catch_void`. The synchronizer recognizes that shape explicitly; otherwise
  an arrow-only parser silently leaves those exports able to unwind into Java.

Every hygiene check that enumerates tracked files uses `git ls-files -z`. The
`-z` is required: without it git octal-escapes paths containing non-ASCII
bytes, so a CJK-named document silently drops out of the scan — nine tracked
files were invisible that way. The tree currently has no non-ASCII path, but
re-adding one would re-open the hole, so the flag stays.

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

### Cargo feature contract

`finkit` publishes a feature list, and a feature list is a claim about which
subsets compile. Only four subsets are supported and verified:

| Subset | Command |
| --- | --- |
| default | `cargo check -p finkit --locked` |
| bare | `cargo check -p finkit --no-default-features --locked` |
| `no_std` | `cargo check -p finkit --no-default-features --features no_std --locked` |
| `tracing` only | `cargo check -p finkit --no-default-features --features tracing --locked` |

`scripts/check_feature_matrix.py` (CI `regression-gates`, `make
check-feature-matrix`) builds those four and asserts that the recorded-broken
ones still fail, so both directions are enforced.

Two things are deliberately **not** supported, and stating them here is cheaper
than rediscovering them:

- **`--all-features` always fails.** `std` and `no_std` are mutually exclusive
  by construction (`compile_error!` in `core/src/lib.rs`).
- **The indicator tree-shaking scaffold does not yet yield a build.** Turning
  off `indicators-all`, or enabling a single category, fails: `operation.rs`,
  `composite.rs` and `factors/builtin.rs` use `crate::formula` and
  `crate::factor_graph` without being gated on `formula`, and
  `indicators/mod.rs`'s `impl_slice_output!` adapters reach into
  `indicators::momentum` from the category-neutral part of the file. Build with
  the default set. Making the scaffold real is a separate refactor whose
  regression surface is every category combination.

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
