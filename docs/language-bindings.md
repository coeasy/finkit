# Language Bindings and Distribution Status

Finkit distinguishes three different claims:

1. **source exists** — a binding is present in the repository;
2. **CI validated / package candidate** — the binding is built and exercised on a real target and an artifact can be produced;
3. **published distribution** — the artifact/package is attached to a release or public registry and clean-install smoke tests pass.

These states are intentionally not treated as equivalent.

## Current v0.1.3 distribution

The published `v0.1.3` GitHub Release remains the authoritative distribution contract for that version:

| Target | v0.1.3 published status | Installation path |
| --- | --- | --- |
| Python | four ABI3 wheels | GitHub Release wheel |
| Rust | `.crate` Release asset | Release asset or git tag/source |
| CLI | Linux x86_64 binary | GitHub Release or source build |
| Node.js | source build/npm-pack CI path | repository source; no npm claim |
| Java/JNI | source build/JAR CI path | repository source; no Maven Central claim |
| C/C++ | source build/CMake install CI path | repository source; no binary SDK Release claim |
| Go | source only in v0.1.3 | repository checkout |
| .NET | source only in v0.1.3 | repository checkout |
| Android | source only in v0.1.3 | repository checkout |
| iOS | source only in v0.1.3 | repository checkout |
| WASM | source only in v0.1.3 | repository checkout |

Do not retroactively describe the v0.1.3 Release as containing Go, NuGet, AAR, XCFramework, or WASM artifacts.

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

Python remains the most complete binary-distribution path in v0.1.3. Four `cp38-abi3` wheels are published for Linux x86_64, Windows x86_64, macOS x86_64, and macOS arm64.

Use [installation.md](installation.md) and [python.md](python.md) for exact wheel selection, NumPy input requirements, `CompiledFormula`, and troubleshooting.

## Rust

Use the release tag when a crates.io package is not independently verified:

```toml
[dependencies]
finkit = { git = "https://github.com/coeasy/finkit", tag = "v0.1.3" }
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

Multi-language packaging is defined by `.github/workflows/multilang-release.yml` and `.github/workflows/multilang-cross-platform.yml`. See [development.md](development.md) and [troubleshooting.md](troubleshooting.md) for diagnosis details.
