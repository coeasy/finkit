# Development Guide

This guide describes the current development and validation workflow for Finkit `v0.1.3` and the main branch. It is intended for contributors changing the Rust core, formula/runtime logic, generated metadata, native bindings, release packaging, or documentation.

## 1. Repository layout

Important top-level areas:

| Path | Purpose |
| --- | --- |
| `core/` | Rust calculation engine, formula/runtime/factors/streaming/features |
| `cli/` | `finkit-cli` command-line package and `finkit-schema` tooling |
| `ffi/python-binding/` | PyO3/maturin Python binding |
| `ffi/node-binding/` | NAPI-RS Node.js binding |
| `ffi/java-binding/` | JNI + Maven Java binding |
| `ffi/c-binding/` | C/C++ SDK wrapper and CMake package |
| `ffi/go-binding/` | Go/CGO source integration |
| `ffi/dotnet-binding/` | .NET/P-Invoke source integration |
| `ffi/android-binding/`, `ffi/ios-binding/`, `ffi/wasm-binding/` | mobile/WASM source integrations |
| `docs/` | canonical user/API/architecture/generated documentation |
| `scripts/` | version, SSOT generation, benchmark, release/helper scripts |
| `.github/workflows/` | CI, docs, wheel, and multi-language validation |

`docs/README.md` is the canonical documentation index. Completed plans and temporary implementation snapshots should not be reintroduced as current user documentation.

## 2. Toolchains

### Rust

The current workspace MSRV is Rust 1.85+.

```bash
rustc --version
cargo --version
rustup component add rustfmt clippy
```

### Python

For Python binding development:

```bash
python3 -m venv .venv
source .venv/bin/activate  # PowerShell: .\.venv\Scripts\Activate.ps1
python -m pip install --upgrade pip
python -m pip install "maturin>=1.5,<2.0" "numpy>=1.24" pytest
```

### Other bindings

Install the language toolchain only when working on that binding:

- Node.js 16+ and npm for NAPI-RS;
- a JDK and Maven for Java/JNI;
- CMake plus a C/C++ compiler for the C/C++ SDK;
- Go 1.21+ with CGO for the Go source binding;
- a compatible .NET SDK for the .NET source binding.

All native bindings also require a host compiler/linker compatible with the Rust target.

## 3. Baseline validation

Run these commands before opening a PR that changes Rust/core behavior:

```bash
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p finkit --locked
cargo test --workspace --doc --locked
python scripts/check_versions.py
python scripts/gen_ssot_docs.py --check
```

Do not remove `--locked` from CI-equivalent commands. `Cargo.lock` is part of the reproducibility contract.

## 4. Version contract

`scripts/check_versions.py` enforces alignment across release-bearing metadata. When bumping a release version, update the canonical version and every package file the checker covers rather than editing one ecosystem in isolation.

Run:

```bash
python scripts/check_versions.py
```

Use the checker's supported fix mode only when intentionally performing a coordinated version update, then review the diff before committing it.

Protocol/schema versions should change only when their contracts change; do not mechanically couple independent schema versions to the package release number.

## 5. Generated SSOT documentation

Generated metadata is part of the repository contract. Typical generated/checked outputs include:

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

If a registry or source-of-truth change legitimately changes generated output, run the generator in its write mode as implemented by the script, commit the generated diff, then run `--check` again.

Do not hand-edit generated Markdown/JSON to silence CI drift.

## 6. Rust core development

### Build core

```bash
cargo build -p finkit --release --locked
```

### Test core

```bash
cargo test -p finkit --locked
```

### Compile benchmarks

```bash
cargo bench -p finkit --no-run
```

A change that improves one synthetic benchmark but breaks warm-up semantics, numerical correctness, allocation contracts, or another supported path is not an acceptable performance optimization.

### Warm-up and alignment contract

When adding/changing rolling calculations:

- preserve output alignment unless the API explicitly states otherwise;
- allow documented leading warm-up `NaN` values;
- test that finite output begins at the expected lookback;
- after valid output begins, do not silently introduce unexplained non-finite values;
- keep equal-length OHLCV validation explicit.

## 7. Formula-engine development

Formula changes can affect parser, optimizer, bytecode, JIT/SIMD execution, reusable plans, compatibility dialects, Python wrappers, and benchmarks.

At minimum, cover affected behavior with tests for:

- parse/validation failures;
- execution semantics;
- aliases/terminal compatibility where relevant;
- common-subexpression safety;
- repeated compiled-plan execution;
- `eval_range` / `eval_last` if dependency/lookback behavior changes;
- append/reset/reserve behavior when retained context changes;
- warm-up and NaN alignment;
- side-effecting/mutable variables if optimizer rules are touched.

Python `CompiledFormula` additionally requires validation of contiguous one-dimensional `float64` arrays on its borrowed zero-copy path.

Relevant documentation:

- `docs/formula.md`;
- `docs/formula-runtime.md`;
- `docs/formula-runtime-contract.md`;
- `docs/formula/grammar.md`.

## 8. Factor/runtime development

The Factor/Runtime layer should maintain these invariants:

- identifiers/dependencies are validated;
- dependency cycles are rejected;
- `MarketFrame` columns remain aligned;
- plan execution does not silently reindex mismatched input;
- warm-up/missing-value semantics remain explicit;
- aliases/registries do not partially mutate on a failed registration.

Use `docs/core-contracts.md` as the public contract and keep tests close to behavior changes.

## 9. Python binding

Build an editable native package:

```bash
cd ffi/python-binding
maturin develop --release
cd ../..
python -m pytest ffi/python-binding/tests -q
```

Build a wheel:

```bash
cd ffi/python-binding
maturin build --release --locked --out dist --compatibility pypi --interpreter python
```

Release-quality validation must install the wheel in a clean directory/environment so the repository source tree cannot shadow the installed package.

The `v0.1.3` Python workflow validates four ABI3 wheel platforms and Linux CPython 3.8-3.14 compatibility. Adding a new advertised platform requires a real build + clean install + test path, not only metadata in `pyproject.toml`.

## 10. Node.js binding

```bash
cd ffi/node-binding
npm install
npm run build
npm test
npm pack
```

The smoke test must load the actual native module. Release packaging also needs to stage the generated native file into the correct platform package as `finkit.node` before packing the root package.

Do not advertise every `optionalDependencies` platform as published merely because it is declared in `package.json`.

## 11. Java/JNI binding

Linux example matching the permanent CI contract:

```bash
cargo build -p finkit-java --release --locked
mkdir -p ffi/java-binding/natives/linux-x86_64
cp target/release/libfinkit_java.so ffi/java-binding/natives/linux-x86_64/
mvn -B -f ffi/java-binding/pom.xml -DskipTests package
jar tf ffi/java-binding/target/*.jar | grep natives
```

A release-quality test must then run Java code that loads the packaged/external native library and calls a real method such as `Indicators.sma(...)`.

For another OS/architecture, stage the correctly named native library under the matching `natives/<platform>/` directory expected by `NativeLoader`.

## 12. C/C++ binding

Reference validation path:

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

After install, validate an external consumer using `find_package(finkit CONFIG REQUIRED)` with the install prefix in `CMAKE_PREFIX_PATH`. This catches package-config/install-layout failures that an in-tree build cannot detect.

Keep the C ABI memory/error contract synchronized with:

- `docs/ffi/memory-contract.md`;
- `docs/ffi/error-codes.md`.

## 13. Go/.NET/mobile/WASM

These are source integrations in the current v0.1.3 release line. Their existence in `ffi/` is not a guarantee of package-manager publication or prebuilt binary coverage.

When promoting one of these bindings to a public release tier, add:

1. a clean external-consumer install/build test;
2. the advertised OS/architecture matrix;
3. native asset packaging/linking rules;
4. version consistency checks;
5. a verified registry or GitHub Release distribution path;
6. user documentation that is updated only after those checks pass.

## 14. Benchmarks and performance gates

Useful repository references:

- `docs/benchmark-results.md`;
- `docs/BENCHMARK_VS_TALIB.md`;
- `docs/BENCHMARK_REPORT.md`;
- benchmark sources under `core/benches/`.

CI includes benchmark compilation plus dedicated allocation/performance gates. Treat benchmark data as measured snapshots tied to CPU/compiler/feature/data conditions.

When modifying a hot path:

- preserve correctness first;
- add or update a focused benchmark;
- run the relevant regression test;
- compare allocations and throughput against the established baseline;
- document meaningful methodology changes rather than silently replacing a baseline.

## 15. Documentation changes

Current user docs intentionally use one canonical tree under `docs/` instead of the previous duplicate mdBook placeholder tree.

Rules:

- update `README.md`, `docs/README.md`, and the relevant install/usage/binding README together when a public contract changes;
- do not reintroduce completed internal plans or PRD snapshots as current docs;
- distinguish source support, CI validation, GitHub Release assets, and registry publication;
- do not claim a package-manager command until the exact package/version can be installed from that registry;
- do not hard-code registry counts that are generated from SSOT;
- verify links after deleting or moving docs.

## 16. Pull-request workflow

Before opening a PR:

1. keep the branch focused;
2. run the relevant local checks;
3. commit generated output when source-of-truth changes require it;
4. explain platform limitations and release impact in the PR body.

After opening the PR, judge only the checks for the **current PR head SHA**. If a new commit supersedes an old run, do not use the old run as release evidence.

Expected workflows vary by changed paths but can include:

- CI;
- Docs Check;
- Python Wheels;
- Multilang release validation.

Read the failing job's current logs before changing code. A runner/preflight failure with zero executed steps is not the same as a code-test failure.

Merge only when required checks for the final head are green.

## 17. Release workflow

A GitHub Release and an ecosystem registry publication are separate events.

For a release:

1. align version metadata;
2. pass version/generated-doc checks;
3. pass core CI and binding/package gates;
4. build and clean-install/smoke-test the intended artifacts;
5. create/update the GitHub Release from the intended main commit;
6. verify the tag target;
7. verify every Release asset and checksum;
8. publish external registries only through an explicitly configured release/trusted-publishing mechanism;
9. test a clean install from each registry before adding that installation command to user docs.

For `v0.1.3`, the verified GitHub Release assets are Python ABI3 wheels, the Rust `.crate`, Linux x86_64 CLI, and `SHA256SUMS`. Node/Java/C++ have CI-validated source/package paths but are not part of the current Release asset set.
