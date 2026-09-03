# Language Bindings and Distribution Status

This document describes the **actual v0.1.3 support contract** for each language binding. A binding existing in source is not the same thing as a package being published to a public registry.

## Support matrix

| Target | v0.1.3 status | Recommended path |
| --- | --- | --- |
| Python | Release wheels verified | Install a wheel from GitHub Release |
| Rust | Core crate and `.crate` Release asset verified | Git tag/local path; registry only when independently published |
| CLI | Linux x86_64 Release binary verified | Release binary or source build |
| Node.js | Native build/test/npm-pack verified in CI | Build from `ffi/node-binding` |
| Java/JNI | JNI build, Maven package/Javadoc, embedded-native loader smoke verified in CI | Build from `ffi/java-binding` |
| C/C++ | CMake build/test/install verified in CI | Build/install from `ffi/c-binding` |
| Go | Source integration exists | Development/source checkout only |
| .NET | Source integration exists | Development/source checkout only |
| Android/iOS/WASM | Source integration exists | Experimental/development for v0.1.3 |

## Python

Python is the most complete binary-distribution path in v0.1.3. Four `cp38-abi3` wheels are published for Linux x86_64, Windows x86_64, macOS x86_64, and macOS arm64.

See [installation.md](installation.md) and [python.md](python.md).

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

Build and test:

```bash
cargo build -p finkit --release --locked
cargo test -p finkit --locked
cargo package -p finkit --locked --no-verify
```

## Node.js

The Node binding is under `ffi/node-binding` and uses NAPI-RS. The repository validates native build, runtime smoke tests, and `npm pack` behavior. This does not imply npm registry publication.

Prerequisites:

- Node.js 16+;
- npm;
- Rust;
- host native build tools.

Build:

```bash
cd ffi/node-binding
npm install
npm run build
npm test
npm pack
```

The JS loader expects the correct platform-specific native `.node` payload. When preparing a real registry release, every platform package declared as an optional dependency must exist and contain the expected `finkit.node` file, or the declared support matrix must be narrowed first.

## Java/JNI

The Java binding is under `ffi/java-binding`. The verified CI path performs:

1. Rust JNI native build;
2. native library staging into `natives/<os>-<arch>/`;
3. Maven package and Javadoc generation;
4. JAR resource validation;
5. a runtime loader + SMA smoke test.

Linux example:

```bash
cargo build -p finkit-java --release --locked
mkdir -p ffi/java-binding/natives/linux-x86_64
cp target/release/libfinkit_java.so ffi/java-binding/natives/linux-x86_64/
mvn -B -f ffi/java-binding/pom.xml -DskipTests package
```

`NativeLoader` supports an explicit `finkit.native.path`, packaged native resources, and a normal `System.loadLibrary("finkit_java")` fallback.

Do not document Maven Central coordinates as installable until the registry publication has actually been verified.

## C and C++

The C/C++ SDK is under `ffi/c-binding` and links to the Rust C FFI library.

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

Installed consumers should prefer the generated CMake package contract:

```cmake
find_package(finkit CONFIG REQUIRED)
```

Set `CMAKE_PREFIX_PATH` to the SDK installation prefix when required.

For ownership and lifetime rules, read [ffi/memory-contract.md](ffi/memory-contract.md) and [ffi/error-codes.md](ffi/error-codes.md).

## Go

The Go source currently lives under `ffi/go-binding/go/`. Its module/distribution layout and CGO native-library delivery are not a stable public v0.1.3 installation contract.

Use it from a source checkout for development and validation. Do not advertise a `go get` command until the module path, tags, and native distribution strategy are finalized and verified.

## .NET

The .NET source binding exists, but v0.1.3 does not publish a verified NuGet package. Build and test it from the repository when working on the binding. Add registry instructions only after a real package and install smoke test exist.

## Android, iOS, and WASM

These are source integrations rather than v0.1.3 binary distribution targets. Treat them as development/experimental surfaces and pin the repository commit when evaluating them.

## Registry publication policy

The following claims must only appear in documentation after real publication and an installation smoke test:

- `pip install finkit` from PyPI;
- `cargo add finkit` from crates.io;
- `npm install finkit` from npm;
- Maven Central coordinates;
- NuGet package IDs;
- public `go get` paths.

GitHub Release packaging, source build success, and public registry publication are separate milestones.

## Release verification

For GitHub Release assets, verify checksums using the `SHA256SUMS` asset. For source builds, run the binding's actual build/test path and the repository-wide version check:

```bash
python scripts/check_versions.py
```

See [development.md](development.md) for the complete validation matrix.
