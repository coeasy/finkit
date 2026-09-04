# Installation Guide

This guide distinguishes the installation paths actually published in Finkit **v0.1.4** from the additional multi-language package candidates being validated for the next release.

## 1. Choose an installation path

| Target | Recommended path today | Published v0.1.4 status | Next-release validation |
| --- | --- | --- | --- |
| Python | Install a wheel from the GitHub `v0.1.4` Release | Verified Release wheels | existing wheel matrix retained |
| Rust library | Git tag/local path, or unpack the `.crate` asset | Verified Release artifact | crate packaging gate |
| CLI | Download Linux x86_64 binary or build from source | Verified Release binary | Linux CLI packaging gate |
| Node.js | Build from `ffi/node-binding` | CI build/test/npm-pack path | native/root package-candidate gate |
| Java/JNI | Build JNI + Maven JAR | CI package/Javadoc/loader smoke | JAR package-candidate gate |
| C/C++ | Build/install from `ffi/c-binding` | CI CMake build/test/install | SDK package-candidate gate |
| Go/CGO | Build from `ffi/go-binding` | source only in v0.1.4 | Linux `go test` + external module gate |
| .NET | Build from `ffi/dotnet-binding` | source only in v0.1.4 | Linux tests + NuGet candidate inspection |
| Android | Build Rust JNI + AAR | source only in v0.1.4 | four-ABI AAR gate |
| iOS | Build XCFramework from source | source only in v0.1.4 | device/simulator XCFramework gate |
| WASM | Build `finkit-wasm` for wasm32 | source only in v0.1.4 | real wasm32 target gate |

The published `v0.1.4` GitHub Release is:

`https://github.com/coeasy/finkit/releases/tag/v0.1.4`

Its assets are four Python ABI3 wheels, `finkit-0.1.4.crate`, `finkit-cli-linux-x86_64`, and `SHA256SUMS`.

A next-release CI artifact is not automatically a v0.1.4 asset, a final future Release asset, or a public registry package.

## 2. Common source-build prerequisites

For source builds, install:

- Git;
- Rust stable compatible with the workspace MSRV (currently Rust 1.85+);
- a native compiler/linker for the host platform;
- the language-specific toolchain for the binding you are building.

Clone the repository:

```bash
git clone https://github.com/coeasy/finkit.git
cd finkit
```

To reproduce the published v0.1.4 source contract:

```bash
git checkout v0.1.4
```

For next-release multi-language work, use the current development branch/main once the changes are merged.

Verify the Rust workspace before building a binding:

```bash
cargo check --workspace --locked
cargo test -p finkit --locked
```

## 3. Python

### Supported v0.1.4 wheel matrix

The `v0.1.4` Release uses CPython stable ABI (`cp38-abi3`). One wheel per platform is reused by supported GIL-enabled CPython versions.

| Platform | Release wheel family | CI compatibility |
| --- | --- | --- |
| Linux x86_64 | `finkit-0.1.4-cp38-abi3-manylinux_2_17_x86_64...whl` | CPython 3.8-3.14 |
| Windows x86_64 | `finkit-0.1.4-cp38-abi3-win_amd64.whl` | build/install smoke verified |
| macOS x86_64 | `finkit-0.1.4-cp38-abi3-macosx_*_x86_64.whl` | build/install smoke verified |
| macOS arm64 | `finkit-0.1.4-cp38-abi3-macosx_*_arm64.whl` | build/install smoke verified |

Not part of the v0.1.4 wheel matrix: Linux arm64, musllinux, 32-bit Windows, PyPy, and free-threaded CPython.

### Install a Release wheel

```bash
python -m pip install --upgrade pip
python -m pip install ./finkit-0.1.4-<matching-platform>.whl
```

Verify:

```bash
python - <<'PY'
import numpy as np
import finkit as ta

close = np.arange(1.0, 101.0, dtype=np.float64)
rsi = ta.rsi(close, timeperiod=14)
assert len(rsi) == len(close)
assert np.isfinite(rsi[-1])
print("Finkit Python OK", rsi[-1])
PY
```

### Build Python from source

Linux/macOS:

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade pip
python -m pip install "maturin>=1.5,<2.0" "numpy>=1.24" pytest
cd ffi/python-binding
maturin develop --release
cd ../..
python -m pytest ffi/python-binding/tests -q
```

Windows PowerShell:

```powershell
py -3.11 -m venv .venv
.\.venv\Scripts\Activate.ps1
python -m pip install --upgrade pip
python -m pip install "maturin>=1.5,<2.0" "numpy>=1.24" pytest
Set-Location ffi/python-binding
maturin develop --release
Set-Location ../..
python -m pytest ffi/python-binding/tests -q
```

See [python.md](python.md) for `CompiledFormula`, NumPy contracts, pandas integration, and troubleshooting.

## 4. Rust library

The workspace package is named `finkit`. The repository produces `finkit-0.1.4.crate` as a Release asset, but that does not imply crates.io publication.

Git tag dependency:

```toml
[dependencies]
finkit = { git = "https://github.com/coeasy/finkit", tag = "v0.1.4" }
```

Local path dependency:

```toml
[dependencies]
finkit = { path = "../finkit/core" }
```

Build/package:

```bash
cargo build -p finkit --release --locked
cargo test -p finkit --locked
cargo package -p finkit --locked --no-verify
```

## 5. CLI

Download the v0.1.4 Linux x86_64 binary:

```bash
curl -L -o finkit-cli \
  https://github.com/coeasy/finkit/releases/download/v0.1.4/finkit-cli-linux-x86_64
chmod +x finkit-cli
./finkit-cli --help
```

Verify it with `SHA256SUMS` from the same Release.

Build from source:

```bash
cargo build -p finkit-cli --release --locked
./target/release/finkit-cli --help
```

## 6. Node.js

The Node binding uses NAPI-RS.

Requirements: Node.js 16+, npm, Rust, and native build tools.

```bash
cd ffi/node-binding
npm ci
npm run build
npm test
npm pack
```

The root JavaScript loader expects a platform-specific native `.node` package. For release work, the platform package must contain `finkit.node` before the root package is treated as distributable.

Do not assume an npm registry release exists until it is published and clean-install tested.

## 7. Java/JNI

Linux source/package example:

```bash
cargo build -p finkit-java --release --locked
mkdir -p ffi/java-binding/natives/linux-x86_64
cp target/release/libfinkit_java.so ffi/java-binding/natives/linux-x86_64/
mvn -B -f ffi/java-binding/pom.xml -DskipTests package
```

The Java loader supports an explicit `finkit.native.path`, packaged `/natives/<os>-<arch>/...` resources, and `System.loadLibrary("finkit_java")` fallback.

Do not assume `com.finkit:finkit:0.1.4` is downloadable from Maven Central until publication is independently verified.

## 8. C/C++ SDK

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

Installed consumers should prefer the exported CMake package and `find_package(finkit CONFIG REQUIRED)` instead of hard-coding a shared-library path.

## 9. Go/CGO

The corrected nested module path for the next-release source is:

```text
github.com/coeasy/finkit/ffi/go-binding/go
```

Package import:

```go
import "github.com/coeasy/finkit/ffi/go-binding/go/ta"
```

Build/test from a checkout:

```bash
cargo build -p finkit-go --release --locked
cd ffi/go-binding/go
LD_LIBRARY_PATH="../../../target/release:${LD_LIBRARY_PATH:-}" go test ./...
```

On macOS use `DYLD_LIBRARY_PATH`; on Windows make the DLL visible through `PATH` or the executable directory.

The next-release CI additionally creates a temporary external Go module and runs the repository example so module identity is validated outside the nested module itself.

Do **not** advertise a plain public `go get` flow until a matching nested-module tag and native-library distribution strategy are actually published and smoke-tested.

See [../ffi/go-binding/README.md](../ffi/go-binding/README.md).

## 10. .NET

Build the native library:

```bash
cargo build -p finkit-dotnet --release --locked
```

Linux managed/native tests:

```bash
LD_LIBRARY_PATH="$PWD/target/release:${LD_LIBRARY_PATH:-}" \
  dotnet test ffi/dotnet-binding/src/Finkit.Tests/Finkit.Tests.csproj \
  -c Release --framework net8.0
```

For a Linux NuGet candidate, stage the native library into its RID source directory and pack:

```bash
mkdir -p ffi/dotnet-binding/native/linux-x64/native
cp target/release/libfinkit_dotnet.so \
  ffi/dotnet-binding/native/linux-x64/native/
dotnet pack ffi/dotnet-binding/src/Finkit/Finkit.csproj \
  -c Release -o dist/dotnet
```

The package project also defines Windows x64, macOS x64, and macOS arm64 native RID paths, but those platforms are only release-supported after native-runner build/package tests pass.

Do not advertise a public NuGet install command until the package exists in a feed and an external project can restore/run it.

See [../ffi/dotnet-binding/README.md](../ffi/dotnet-binding/README.md).

## 11. Android

Android uses the Rust `finkit-android` JNI crate plus a standard Gradle Android Library project.

Install target/tooling support:

```bash
cargo install cargo-ndk --locked
rustup target add \
  aarch64-linux-android \
  armv7-linux-androideabi \
  x86_64-linux-android \
  i686-linux-android
```

Build native ABI libraries:

```bash
cargo ndk \
  --platform 24 \
  -t arm64-v8a \
  -t armeabi-v7a \
  -t x86_64 \
  -t x86 \
  -o ffi/android-binding/android/src/main/jniLibs \
  build --release --locked -p finkit-android
```

Assemble the AAR:

```bash
cd ffi/android-binding/android
gradle assembleRelease
```

The next-release gate inspects the AAR to ensure all four advertised ABIs contain `libfinkit_android.so`.

See [../ffi/android-binding/README.md](../ffi/android-binding/README.md).

## 12. iOS / Swift

Install Rust targets on macOS:

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

Output:

```text
dist/ios/Finkit.xcframework
```

The build uses one physical arm64 slice and one universal simulator slice made from Apple Silicon + Intel simulator libraries. New Swift source uses `Finkit`; legacy `AlphaTA` is only a deprecated compatibility alias.

See [../ffi/ios-binding/README.md](../ffi/ios-binding/README.md).

## 13. WebAssembly

```bash
rustup target add wasm32-unknown-unknown
cargo build -p finkit-wasm \
  --target wasm32-unknown-unknown \
  --release --locked
```

Raw output:

```text
target/wasm32-unknown-unknown/release/finkit_wasm.wasm
```

This verifies the real WASM target. JavaScript/TypeScript glue and an npm/browser package remain separate packaging work and must be tested for the chosen `web`, `bundler`, or Node runtime.

See [../wasm/README.md](../wasm/README.md).

## 14. Verify versions and generated contracts

```bash
python scripts/check_versions.py
python scripts/gen_ssot_docs.py --check
python scripts/check_docs_links.py
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p finkit --locked
cargo test --workspace --doc --locked
```

The full next-release target/package matrix is defined in `.github/workflows/multilang-release.yml`.

## 15. Troubleshooting

### Python wheel is rejected as unsupported

Check interpreter, OS, and architecture:

```bash
python -VV
python -c "import platform; print(platform.system(), platform.machine())"
```

### Native binding cannot load

Confirm that the native library matches the language process OS/architecture and that the runtime loader can find it. A successful compile does not guarantee runtime native-library discovery.

### Source build fails before compilation

```bash
rustc --version
cargo --version
```

On Windows use an MSVC-compatible Rust toolchain and Visual Studio C++ Build Tools. On macOS install a complete Xcode/Xcode Command Line Tools setup. On Linux install compiler/linker build essentials plus any binding-specific SDK.

### Registry command cannot find Finkit

Registry publication is not the same as a GitHub Actions artifact or GitHub Release asset. Use the verified Release/source instructions above until the exact package/version is visible and clean-install tested in the target registry.

For deeper diagnosis, see [troubleshooting.md](troubleshooting.md).
