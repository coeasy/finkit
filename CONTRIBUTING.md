# Contributing to Finkit

Thank you for contributing to Finkit. Changes should preserve the repository's current contracts: Rust is the core implementation, generated metadata is checked in CI, native bindings must be validated with real smoke tests, and documentation must distinguish source availability from published packages.

## Development setup

```bash
git clone https://github.com/coeasy/finkit.git
cd finkit
rustup component add rustfmt clippy
```

The current workspace MSRV is Rust 1.85+.

## Create a branch

Use a short branch tied to one coherent change:

```bash
git checkout -b feat/my-change
```

Do not mix release metadata, unrelated refactors, generated-document changes, and feature work in the same PR unless they are part of one required contract change.

## Required Rust checks

Before opening a PR:

```bash
cargo fmt --all -- --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test -p finkit --locked
cargo test --workspace --doc --locked
```

CI also contains dependency/security, benchmark-compilation, zero-allocation, and relative-performance gates. Do not weaken a gate merely to make a PR green; investigate the measured regression or contract difference.

## Version and generated-document checks

Run:

```bash
python scripts/check_versions.py
python scripts/gen_ssot_docs.py --check
```

When a source-of-truth registry changes, run the appropriate generator and commit the resulting generated files. Do not hand-edit generated indicator/function/version docs to hide drift.

Key generated/SSOT files include:

- `docs/indicator_registry.json`;
- `docs/generated/indicators.md`;
- `docs/generated/streaming-indicators.md`;
- `docs/generated/formula-functions.md`;
- `docs/generated/features.md`;
- `docs/generated/error-codes.md`;
- `docs/generated/pine-compatibility.md`;
- `docs/generated/version-matrix.md`.

## Adding or changing indicators

When adding an indicator:

1. implement it in the appropriate Rust core module;
2. preserve the documented warm-up/NaN and validation semantics;
3. add normal-input, invalid-parameter, insufficient-data, and edge-case tests;
4. update streaming support separately if an incremental implementation exists;
5. update registry/schema metadata so generated docs remain correct;
6. propagate the API through each binding that claims support;
7. add a real binding smoke test when exposing the function in a new language;
8. regenerate SSOT docs and run the full checks above.

Do not advertise a hard-coded indicator count when the generated registry can be linked instead.

## Formula-engine changes

Formula changes can affect parsing, optimization, bytecode, JIT, range evaluation, terminal compatibility, and bindings simultaneously. Add regression tests for:

- parser/semantic behavior;
- warm-up/lookback behavior;
- repeated compiled-plan execution;
- `eval_range` / `eval_last` where affected;
- side effects and common-subexpression optimization safety;
- terminal-compatibility golden vectors where affected.

Python changes to `CompiledFormula` should also validate contiguous `float64` NumPy requirements and retained-context behavior where relevant.

## FFI and language bindings

### Python

```bash
python3 -m venv .venv
source .venv/bin/activate
python -m pip install "maturin>=1.5,<2.0" "numpy>=1.24" pytest
cd ffi/python-binding
maturin develop --release
cd ../..
python -m pytest ffi/python-binding/tests -q
```

### Node.js

```bash
cd ffi/node-binding
npm install
npm run build
npm test
npm pack
```

The native smoke test must load the actual `.node` module. Packaging validation should verify that the host platform package contains `finkit.node` before the root package is considered releasable.

### Java/JNI

```bash
cargo build -p finkit-java --release --locked
mvn -B -f ffi/java-binding/pom.xml -DskipTests package
```

For release validation, also stage the native library into the proper `natives/<platform>/` JAR resource path and run a Java program that calls a real indicator such as `Indicators.sma(...)`.

### C/C++

```bash
cargo build -p finkit-ffi --release --locked
cmake -S ffi/c-binding -B build/cpp \
  -DFINKIT_AUTO_BUILD_RS=OFF \
  -DFINKIT_BUILD_TESTS=ON \
  -DFINKIT_BUILD_EXAMPLES=ON \
  -DCMAKE_BUILD_TYPE=Release
cmake --build build/cpp --parallel 2
ctest --test-dir build/cpp --output-on-failure
```

Release-quality C/C++ validation should also install the SDK and compile an external consumer using the installed CMake package config.

### Go/.NET/mobile/WASM

These source integrations are not part of the current `v0.1.3` binary Release contract. If changing them, add platform-appropriate native build and external-consumer tests before upgrading their documented maturity.

## Documentation rules

User documentation lives under `docs/` with [docs/README.md](docs/README.md) as the canonical index.

When behavior or distribution changes:

- update `README.md`, `docs/README.md`, and the relevant install/usage guide together;
- do not claim a registry package until the exact package/version is publicly visible and install-tested;
- distinguish “builds from source”, “CI validated”, “GitHub Release asset”, and “registry published”;
- keep completed plans and temporary progress snapshots out of current user docs; Git history/PRs are the historical record;
- use current API names from code, not remembered names from older releases.

## Pull requests

A PR description should state:

- the problem/contract being changed;
- the implementation summary;
- tests run locally or by CI;
- any platform or language limitations;
- whether generated files changed and why;
- whether the change affects release artifacts or only source support.

Merge only after the required workflows for the final PR head are green.

## Release process

A version bump and a Git tag do **not** imply every language registry is published.

For the current release architecture:

1. align the canonical workspace/package versions and run `scripts/check_versions.py`;
2. run Rust CI, Docs Check, Python Wheels, and multi-language validation as applicable;
3. ensure generated SSOT docs are clean;
4. build and install-test release artifacts on their target platforms;
5. create/update the GitHub Release only after release gates pass;
6. verify the tag points to the intended main commit;
7. verify uploaded assets and `SHA256SUMS`;
8. publish to PyPI/crates.io/npm/Maven Central/NuGet/etc. only through an explicitly configured registry workflow or trusted-publishing path;
9. after any registry publication, test a clean install from that registry before documenting it as supported.

The `v0.1.3` GitHub Release currently proves Python wheel, Rust `.crate`, Linux CLI, and checksum assets. Node/Java/C++ are source/CI packaging paths; other bindings have narrower source/development status. Future releases should update documentation only after those facts change.

## Issues and security

Use GitHub issues for reproducible bugs and feature requests. Include the Finkit commit/tag, OS/architecture, language binding, minimal input, expected result, actual result, and relevant logs. Avoid posting secrets, tokens, or private market data in public issues.
