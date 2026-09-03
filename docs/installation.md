# Installation Guide

This guide describes the installation paths that are actually supported by the current Finkit `v0.1.3` repository and Release.

## 1. Choose an installation path

| Target | Recommended path | v0.1.3 status |
| --- | --- | --- |
| Python | Install a wheel from the GitHub `v0.1.3` Release | Verified release path |
| Rust library | Git tag/local path, or unpack the `.crate` Release asset | Verified package artifact; registry publication is separate |
| CLI | Download Linux x86_64 Release binary or build from source | Linux Release binary verified |
| Node.js | Build from `ffi/node-binding` | CI build/test/npm-pack path verified; no registry assumption |
| Java | Build JNI + Maven JAR from `ffi/java-binding` | CI package/Javadoc/loader smoke verified; no Maven Central assumption |
| C/C++ | Build/install from `ffi/c-binding` | CMake build/test/install path verified in CI |
| Go | Build from repository source | Source/development integration; not a public v0.1.3 module contract |
| .NET | Build from repository source | Source/development integration; no verified NuGet release |
| Android/iOS/WASM | Build from repository source | Development/experimental for v0.1.3 |

The GitHub Release is at:

`https://github.com/coeasy/finkit/releases/tag/v0.1.3`

The published `v0.1.3` assets are four Python ABI3 wheels, `finkit-0.1.3.crate`, `finkit-cli-linux-x86_64`, and `SHA256SUMS`.

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
git checkout v0.1.3
```

For development on the latest main branch, omit the tag checkout.

Verify the Rust workspace before building a binding:

```bash
cargo check --workspace --locked
cargo test -p finkit --locked
```

## 3. Python

### Supported wheel matrix

The `v0.1.3` Release uses CPython stable ABI (`cp38-abi3`). One wheel per platform is reused by supported GIL-enabled CPython versions.

| Platform | Release wheel family | CI compatibility |
| --- | --- | --- |
| Linux x86_64 | `finkit-0.1.3-cp38-abi3-manylinux_2_17_x86_64...whl` | CPython 3.8-3.14 |
| Windows x86_64 | `finkit-0.1.3-cp38-abi3-win_amd64.whl` | Build/install smoke verified |
| macOS x86_64 | `finkit-0.1.3-cp38-abi3-macosx_*_x86_64.whl` | Build/install smoke verified |
| macOS arm64 | `finkit-0.1.3-cp38-abi3-macosx_*_arm64.whl` | Build/install smoke verified |

Not part of the v0.1.3 wheel matrix: Linux arm64, musllinux, 32-bit Windows, PyPy, and free-threaded CPython.

### Install a Release wheel

Download the wheel matching the operating system and CPU architecture, then run:

```bash
python -m pip install --upgrade pip
python -m pip install ./finkit-0.1.3-<matching-platform>.whl
```

NumPy is a required runtime dependency and is declared by the package metadata.

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

See [python.md](python.md) for wheel selection, `CompiledFormula`, pandas integration, and troubleshooting.

## 4. Rust library

The workspace package is named `finkit`. The repository produces `finkit-0.1.3.crate` as a Release asset, but the presence of that artifact does not imply crates.io publication.

### Git tag dependency

```toml
[dependencies]
finkit = { git = "https://github.com/coeasy/finkit", tag = "v0.1.3" }
```

### Local path dependency

```toml
[dependencies]
finkit = { path = "../finkit/core" }
```

### Build the package directly

```bash
cargo build -p finkit --release --locked
cargo test -p finkit --locked
cargo package -p finkit --locked --no-verify
```

Common optional features include `rayon`, `finkit-polars`, `talib-c`, `nightly-avx512`, and `precision-f32`; the default feature set already enables the standard library, formula support, serialization, observability scaffolding, formula JIT/SIMD, and the full indicator categories. Inspect `core/Cargo.toml` before disabling defaults because indicator categories have transitive dependencies.

## 5. CLI

### Linux x86_64 Release binary

```bash
curl -L -o finkit-cli \
  https://github.com/coeasy/finkit/releases/download/v0.1.3/finkit-cli-linux-x86_64
chmod +x finkit-cli
./finkit-cli --help
```

Verify the downloaded file with `SHA256SUMS` from the same Release before production use.

### Build the CLI from source

```bash
cargo build -p finkit-cli --release --locked
./target/release/finkit-cli --help
```

The package is `finkit-cli`; its Clap application name is `finkit`. Documentation examples use the produced `finkit-cli` executable path to avoid ambiguity.

## 6. Node.js

The Node binding uses NAPI-RS and is configured as package `finkit` version `0.1.3`. The repository validates native build/test/package logic, but v0.1.3 documentation does not claim an npm registry release.

Prerequisites:

- Node.js 16+;
- npm;
- Rust;
- platform native build tools.

Build and test:

```bash
cd ffi/node-binding
npm install
npm run build
npm test
npm pack
```

The root JS loader expects a platform-specific native `.node` package/artifact. For distribution work, ensure the optional platform package declared in `package.json` is actually built and staged; do not publish a root npm package whose declared platform packages are missing.

## 7. Java/JNI

The Java binding source is under `ffi/java-binding`. The CI path builds the Rust JNI library, embeds the native resource into the JAR, runs Maven package/Javadoc, and executes a JNI loader smoke test.

Prerequisites:

- Rust 1.85+;
- JDK compatible with the project source/target level;
- Maven;
- native compiler/linker.

Linux source build example:

```bash
cargo build -p finkit-java --release --locked
mkdir -p ffi/java-binding/natives/linux-x86_64
cp target/release/libfinkit_java.so ffi/java-binding/natives/linux-x86_64/
mvn -B -f ffi/java-binding/pom.xml -DskipTests package
```

The Java `NativeLoader` first supports an explicit `finkit.native.path`, then packaged `/natives/<os>-<arch>/...` resources, then a normal `System.loadLibrary("finkit_java")` fallback. Match the native resource directory and filename to the target OS/architecture when packaging another platform.

Do not assume `com.finkit:finkit:0.1.3` is downloadable from Maven Central until the registry publication is independently verified.

## 8. C/C++ SDK

The C/C++ wrapper is under `ffi/c-binding` and links to the Rust C FFI library.

Linux example:

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

The installed SDK exports CMake package metadata so downstream consumers can use `find_package(finkit CONFIG REQUIRED)` after setting `CMAKE_PREFIX_PATH` to the installation prefix.

Do not point CMake users directly at a guessed `libfinkit_ffi.so` path when an installed package can be used; the installed CMake config is the preferred consumer contract.

## 9. Go

The Go source currently lives below `ffi/go-binding/go/` and its nested `go.mod` declares `module github.com/coeasy/finkit`. This layout is useful for repository development but is not a clean public versioned Go-module release contract for v0.1.3.

Use it only from a source checkout while the module path/native CGO distribution is being finalized. Do **not** document `go get github.com/coeasy/finkit/go/ta` as a supported public v0.1.3 install command.

## 10. .NET, Android, iOS, WASM

These directories contain source integrations, but they are not part of the verified `v0.1.3` GitHub Release asset matrix. Build them from the repository only when developing or validating those bindings. Registry/binary installation commands should be added to documentation only after a real artifact exists and an install smoke test succeeds.

## 11. Verify versions and generated contracts

After changing package or release metadata, run:

```bash
python scripts/check_versions.py
python scripts/gen_ssot_docs.py --check
```

For a complete repository validation:

```bash
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p finkit --locked
cargo test --workspace --doc --locked
```

## 12. Troubleshooting

### Python wheel is rejected as unsupported

Check interpreter, OS, and architecture:

```bash
python -VV
python -c "import platform; print(platform.system(), platform.machine())"
```

A `cp38-abi3` wheel can span supported CPython minor versions but cannot cross operating systems or CPU architectures.

### Native binding cannot load

Confirm that the native library was built for the same OS, architecture, and runtime ABI as the language process. For Java, check packaged `natives/<platform>/` resources or set `-Dfinkit.native.path=/absolute/path/to/library` where supported. For Node, verify the expected platform package contains `finkit.node`.

### Source build fails before compilation

Check the toolchain first:

```bash
rustc --version
cargo --version
```

On Windows use an MSVC-compatible Rust toolchain and Visual Studio C++ Build Tools. On macOS install Xcode Command Line Tools. On Linux install the distribution's compiler/linker build essentials.

### Registry command cannot find Finkit

That is not evidence that the source package is broken. Registry publication is not the same as GitHub Release packaging. Use the verified Release/source instructions above and only switch to a registry command once that registry contains the exact package/version.
