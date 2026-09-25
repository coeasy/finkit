# Language Bindings and Distribution Status

Finkit distinguishes three different claims:

1. **source exists** — a binding is present in the repository;
2. **CI validated / package candidate** — the binding is built and exercised on a real target and an artifact can be produced;
3. **published distribution** — the artifact/package is attached to a release or public registry and clean-install smoke tests pass.

These states are intentionally not treated as equivalent.

## Active language tier (SSOT)

The project's *active* language set is deliberately narrow, and the drift gate
is scoped to match it. `scripts/sync_bindings.py` is the single source of truth
for the tier.

| Tier | Languages | Drift-checked? |
| --- | --- | --- |
| **Active** | Rust (this repository's `core` crate), Python, Node | yes — a tier-1 language with no stored bodies fails the run |
| **Deferred** | C, Go, Java, .NET, iOS, Android | no — reported as `status=DEFERRED`, does not fail the run |

The Rust core needs no binding: it *is* the runtime, and `cargo test -p finkit`
is its contract. Python and Node are the first-tier bindings.

A deferred binding still lives in `ffi/` and still compiles under
`cargo check --workspace`; what it does **not** have is stored wrapper bodies in
`docs/ffi_registry.json`, so `--check` cannot detect hand edits to it. That gap
is recorded explicitly rather than papered over, for two reasons: a gate that is
permanently red gets ignored, and a language reported as `drift=none` with zero
stored bodies would be a vacuous pass that hides the real gap.

**Promoting a language** to the active tier is a two-step change: move it from
`DEFERRED_LANGS` to `TIER1_LANGS` in `scripts/sync_bindings.py`, then store its
bodies with `python scripts/sync_bindings.py --discover --lang <lang>` and
confirm `--check` is green.

```
python scripts/sync_bindings.py --check        # active tier (Python, Node)
python scripts/sync_bindings.py --check --all  # + report the deferred languages
```

`--check` exits non-zero on drift, or on an active-tier language that has no
stored bodies. It is wired into CI (`.github/workflows/ci.yml`, job
`binding-ssot`, with no `--allow-unchecked` escape hatch) and into the
`make verify-bindings-tier` / `make verify-all-bindings` targets.

### Registry round-trip invariants

`docs/indicator_registry.json` (235 indicators, rich per-indicator metadata) is
a **superset** of `docs/ffi_registry.json` (78 indicators carrying binding
bodies) — the remaining 157 indicators have no binding at all. `sync_bindings.py`
must therefore never rebuild the core registry *from* the FFI one. Two invariants
are load-bearing:

1. **`--discover` is superset-preserving.** It keeps every entry already in the
   core registry, in its existing order, and only appends names that carry core
   metadata. A name that appears in the FFI SSOT with nothing but `{name, ffi}`
   (for example `DARVAS_BOX`, `RENKO` — dispatched through a `match` and
   deliberately without a core entry) must not spring into existence. Without
   this, running `--discover` on a clean checkout — where the transient Python
   overlay is absent and the FFI SSOT is the fallback — deleted all 157
   binding-less indicators.
2. **Both registries are written with `\n` line endings.** The default
   translation to `os.linesep` made every Windows run rewrite both files as CRLF
   and produce a whole-file diff.

CI asserts the core registry is untouched by the binding job
(`git diff --exit-code -- docs/indicator_registry.json`).

## v0.1.15 distribution contract

The `v0.1.15` tag and release workflow are the authoritative version contract:

| Target | v0.1.15 release path | Installation path |
| --- | --- | --- |
| Python | ABI3 wheels | GitHub Release wheel |
| Rust | `.crate` asset | Release asset or git tag/source |
| CLI | Linux x86_64 binary | GitHub Release or source build |
| Node.js | platform `.tgz` candidates | CI artifact / source |
| Java/JNI | versioned JAR | CI artifact / source |
| C/C++ | versioned SDK archive | CI artifact / source |
| Go | versioned module archive | CI artifact / source |
| .NET | versioned NuGet package | CI artifact / source |
| Android | versioned AAR | CI artifact / source |
| iOS | versioned XCFramework | CI artifact / source |
| WASM | web/node/bundler bundles | CI artifact / source |

An artifact is not a public registry package until its target registry has been
published to and a clean consumer install has passed.

## Screening API across bindings

The market-neutral screening formulas are exposed through the Rust core and
formula runtime. Bindings that embed the formula evaluator can call the same
names without introducing language-specific indicator semantics. See
[screening-formulas.md](screening-formulas.md) for signatures, aliases, warm-up
rules, and A-share/HK/US/crypto usage notes.

## Next-release multi-language target

The next-release validation layer is split across `Multilang release` and `Multilang cross-platform`.

| Target | CI-validated target | Candidate artifact |
| --- | --- | --- |
| Node.js | Linux x86_64 GNU, Windows x64 MSVC, macOS arm64; native runtime tests and platform package inspection | platform `.tgz` packages |
| Java/JNI | Linux Rust JNI build, JAR resource check, runtime SMA smoke | JAR |
| C/C++ | Linux CMake build/test/install | SDK `.tar.gz` |
| Go/CGO | Linux Rust native build, `go test`, external-module example and packaged consumer smoke | Go module source + `libfinkit_go.so` |
| .NET | Linux x64, Windows x64 and macOS arm64; .NET 8 tests plus NuGet RID inspection | `.nupkg` candidates |
| WASM | real `wasm32-unknown-unknown` build | raw `.wasm` module |
| Android | four NDK ABI builds + Gradle AAR assembly + archive inspection | `.aar` candidate |
| iOS | arm64 device + arm64/x86_64 simulator build + XCFramework packaging | `.xcframework.zip` candidate |
| Rust/CLI | crate packaging + Linux CLI release build | `.crate` + CLI |

These are **validated candidate artifacts**, not public registry packages. A target is only listed as CI-validated after a real hosted runner completed its build, language-level smoke/test path and package-content checks.

The current Node package manifest declares additional optional packages such as macOS x64, Linux arm64, musl variants and Windows arm64. Those targets remain outside the proven candidate matrix until they receive equivalent real-runner or cross-build package validation. The same rule applies to the declared `.NET` `osx-x64` RID.

## Python

Python remains the most complete binary-distribution path in v0.1.15. Four `cp38-abi3` wheels are built for Linux x86_64, Windows x86_64, macOS x86_64, and macOS arm64.

Use [installation.md](installation.md) and [python.md](python.md) for exact wheel selection, NumPy input requirements, `CompiledFormula`, and troubleshooting.

## Rust

Use the release tag when a crates.io package is not independently verified:

```toml
[dependencies]
finkit = { git = "https://github.com/coeasy/finkit", tag = "v0.1.15" }
```

Or use a local checkout:

```toml
[dependencies]
finkit = { path = "../finkit/core" }
```

Validation:

```bash
cargo build -p finkit --release --locked
cargo test -p finkit --locked
cargo package -p finkit --locked --no-verify
```

## Node.js

The Node binding is under `ffi/node-binding` and uses NAPI-RS.

```bash
cd ffi/node-binding
npm ci
npm run build
npm test
npm pack
```

The next-release gates have now proven these native package candidates on real GitHub-hosted runners:

- `linux-x64-gnu`;
- `win32-x64-msvc`;
- `darwin-arm64`.

Each validated path builds the native module, runs the Node smoke tests, stages the native payload into the matching npm platform package, runs `npm pack`, and uploads the resulting package candidate.

The root npm package still declares additional optional platform packages. Do not publish or advertise the root package as universally installable until every platform in the advertised support matrix is built and package-tested.

## Java/JNI

The Java binding is under `ffi/java-binding`. The validated Linux packaging path performs:

1. Rust JNI native build;
2. native library staging into `natives/linux-x86_64/`;
3. Maven package/Javadoc build;
4. JAR resource inspection;
5. a real JVM loader + SMA smoke test.

Example:

```bash
cargo build -p finkit-java --release --locked
mkdir -p ffi/java-binding/natives/linux-x86_64
cp target/release/libfinkit_java.so ffi/java-binding/natives/linux-x86_64/
mvn -B -f ffi/java-binding/pom.xml -DskipTests package
```

Maven Central publication remains a separate milestone.

## C and C++

The C/C++ SDK is under `ffi/c-binding` and links to the Rust C FFI library.

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

Installed consumers should prefer:

```cmake
find_package(finkit CONFIG REQUIRED)
```

For ownership and lifetime rules, read [ffi/memory-contract.md](ffi/memory-contract.md) and [ffi/error-codes.md](ffi/error-codes.md).

### ABI surface and its gate

`ffi/c-binding/include/*.h` and the Rust `#[no_mangle] extern "C"` functions are
two halves of one ABI. The Rust side is spread over three files that all export
symbols, and all three are part of the contract:

| Source | Symbols | What they are |
| --- | --- | --- |
| `ffi/c-binding/src/generated.rs` | 78 | one `ta_*` per indicator in `docs/ffi_registry.json` |
| `ffi/c-binding/src/lib.rs` | 18 | fixed-template entry points (`ta_version`, `ta_last_error`, `ta_factor_execute_json`, `finkit_free_string`, ...) |
| `ffi/c-binding/src/research.rs` | 3 | research surface (`finkit_factor_study_json`, `finkit_quant_evaluation_json`, `finkit_factor_study_free_string`) |

That is **99 shipped exports**, matched one-for-one by 99 declarations across
`ffi/c-binding/include/finkit.h` (96) and `finkit_research.h` (3).

`python scripts/gen_c_header.py --check ffi/c-binding/include/finkit.h` asserts
that match in both directions and is wired into CI (`.github/workflows/ci.yml`,
job `binding-ssot`). It scans *all* of `ffi/c-binding/src/*.rs`: an earlier
version only looked at `generated.rs`, which left 21 real exports unverified.
`#[cfg(test)]`-gated exports (for example `ta_ffi_panic_test`) are correctly
ignored, because they are absent from a release build.

`generated.rs` no longer has an in-tree generator — `scripts/gen_binding.py` used
to emit it from an `ffi` block in `docs/indicator_registry.json`, but that
metadata now lives in `docs/ffi_registry.json` and the emitter refuses to run
rather than write an empty binding. `make verify-ffi` is what keeps the frozen
artifact honest.

## Go/CGO

The canonical nested Go module is:

```text
github.com/coeasy/finkit/ffi/go-binding/go
```

and the package import path is:

```text
github.com/coeasy/finkit/ffi/go-binding/go/ta
```

Source validation:

```bash
cargo build -p finkit-go --release --locked
cd ffi/go-binding/go
LD_LIBRARY_PATH="../../../target/release:${LD_LIBRARY_PATH:-}" go test ./...
```

The release gate also builds an **external temporary module** using a local `replace` directive and runs the repository example. The packaged candidate is then unpacked into another clean temporary consumer and executed again against the staged native library. This catches module-path and delivery mistakes that same-module tests can miss.

The Go binding includes indicator, streaming, formula, and template APIs. In particular, its formula debug wrapper is `FormulaEvalDebugJSON`; debugger method names are not assumed to be identical in other languages.

A public `go get` path must not be advertised until the nested-module tag convention and native-library delivery strategy are published and install-tested.

See [../ffi/go-binding/README.md](../ffi/go-binding/README.md).

## .NET

The .NET binding uses P/Invoke and targets .NET 6 and .NET 8.

The project defines native package paths for:

- `win-x64`;
- `linux-x64`;
- `osx-x64`;
- `osx-arm64`.

The next-release validation currently proves three of those RIDs:

| RID | Validation |
| --- | --- |
| `linux-x64` | native Rust build, .NET 8 tests, NuGet pack, `runtimes/linux-x64/native/libfinkit_dotnet.so` inspection |
| `win-x64` | native Rust build, 18 .NET 8 tests, NuGet pack, `runtimes/win-x64/native/finkit_dotnet.dll` inspection |
| `osx-arm64` | native Rust build on a real arm64 macOS runner, 18 .NET 8 tests, NuGet pack, `runtimes/osx-arm64/native/libfinkit_dotnet.dylib` inspection |

`osx-x64` is still a declared package RID, not a proven candidate target. It must receive its own build/package verification before being listed as validated.

Do not document `dotnet add package Finkit` as a public feed install until an actual NuGet publication and clean consumer install test exist.

See [../ffi/dotnet-binding/README.md](../ffi/dotnet-binding/README.md).

## Android

Android consists of the Rust `finkit-android` JNI crate plus a standard Gradle Android Library project under `ffi/android-binding/android`.

The validated release gate builds these ABIs with `cargo-ndk`:

- `arm64-v8a`;
- `armeabi-v7a`;
- `x86_64`;
- `x86`.

It stages `libfinkit_android.so` below `src/main/jniLibs/<abi>/`, runs `gradle assembleRelease`, then inspects the AAR for all four native payloads.

The Java API is `com.finkit.indicators.Finkit` and loads the native library automatically. There is no separate `init()` method.

See [../ffi/android-binding/README.md](../ffi/android-binding/README.md).

## iOS / Swift

The iOS crate is packaged as `Finkit.xcframework` from:

- `aarch64-apple-ios` for physical arm64 devices;
- `aarch64-apple-ios-sim` for Apple Silicon simulators;
- `x86_64-apple-ios` for Intel simulators.

The two simulator static libraries are combined into one universal simulator slice before `xcodebuild -create-xcframework` runs.

New Swift code uses `Finkit` and `FinkitError`. The historical `AlphaTA`/`AlphaTAError` names are retained only as deprecated aliases, while the underlying `alpha_ta_*` C ABI symbol prefix remains temporarily for compatibility.

An XCFramework CI artifact is not yet a Swift Package Manager/CocoaPods publication. See [../ffi/ios-binding/README.md](../ffi/ios-binding/README.md).

## WebAssembly

The `finkit-wasm` crate is validated against the actual WebAssembly target rather than host-only compilation:

```bash
rustup target add wasm32-unknown-unknown
cargo build -p finkit-wasm --target wasm32-unknown-unknown --release --locked
```

The result is a raw `finkit_wasm.wasm` candidate artifact. JavaScript/TypeScript glue and an npm/browser package are separate packaging steps and should use a wasm-bindgen toolchain compatible with the locked crate dependency.

See [../wasm/README.md](../wasm/README.md).

## Publication policy

The following claims must only appear after real publication **and** a clean consumer smoke test:

- `pip install finkit` from PyPI;
- `cargo add finkit` from crates.io;
- `npm install finkit` from npm;
- Maven Central coordinates;
- NuGet package IDs;
- a plain public `go get` path;
- Swift Package Manager/CocoaPods dependency coordinates;
- Android Maven repository coordinates.

GitHub Actions artifacts, GitHub Release assets, source builds, and public registries are distinct milestones.

## Validation entry points

Repository-wide version/docs contracts:

```bash
python scripts/check_versions.py
python scripts/gen_ssot_docs.py --check
python scripts/check_docs_links.py
```

Binding SSOT contracts (all wired into `ci.yml`, job `binding-ssot`):

```bash
python scripts/sync_bindings.py --check --all
python scripts/optimize_python_bindings.py --check ffi/python-binding/src/*.rs
python scripts/check_python_stub.py                     # static half
python scripts/check_streaming_registry_contract.py
python scripts/gen_c_header.py --check ffi/c-binding/include/finkit.h
```

The strong half of the stub check needs a built wheel, so it runs from
`python-wheels.yml` after the wheel is installed:

```bash
python scripts/check_python_stub.py \
  --require-extension --expect-prefix "$pythonLocation"
```

`--require-extension` makes "finkit not importable" a failure instead of a skip,
and `--expect-prefix` pins the build under test so a stale or shadowing install
cannot make the check pass vacuously. The check is opt-in for exactly that
reason: importing whatever `finkit` happens to be on `sys.path` would fail
against a stale site-packages copy, or silently validate the wrong build.

Multi-language packaging is defined by `.github/workflows/multilang-release.yml` and `.github/workflows/multilang-cross-platform.yml`. See [development.md](development.md) and [troubleshooting.md](troubleshooting.md) for diagnosis details.
