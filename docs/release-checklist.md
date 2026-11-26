# qx-finkit v0.3.2 Release Checklist

This checklist records the v0.3.2 release and its verification. It separates
the published v0.3.2 packages/assets from future release candidates.

## Before creating the tag

- [x] Confirm the intended release commit contains the v0.3.2 workspace version.
- [x] Run the version/docs contracts:
      `python scripts/check_versions.py`,
      `python scripts/check_changelog.py CHANGELOG.md`,
      `python scripts/gen_ssot_docs.py --check`,
      `python scripts/check_docs_links.py`,
      `python scripts/check_orphan_docs.py`.
- [x] Run the binding SSOT contracts (CI job `binding-ssot`):
      `python scripts/sync_bindings.py --check --all`,
      `python scripts/optimize_python_bindings.py --check ffi/python-binding/src/*.rs`,
      `python scripts/check_python_stub.py`,
      `python scripts/check_streaming_registry_contract.py`,
      `python scripts/gen_c_header.py --check ffi/c-binding/include/finkit.h`,
      `python scripts/check_ios_header_contract.py`.
      The last two are the same contract for two different headers: every
      shipped `#[no_mangle] extern "C"` symbol must be declared in the header
      that documents it. The iOS header is separate from the C binding's and had
      drifted by three undeclared entry points before this gate existed.
      `check_python_stub.py` covers both directions: a name declared in the stub
      must exist, and a documented Python call must be declared.
- [x] Run the per-crate test suites (CI jobs `test`, `research-test`,
      `runtime-integration-tests`, `binding-unit-tests`). `--no-fail-fast` is
      required — without it the first failing binary hides every later one, and
      the aggregate "ok" is meaningless:
      `cargo test -p qx-finkit -p finkit-factor-analysis -p finkit-cli \
        -p finkit-visualization -p finkit-ffi-common -p finkit-ffi \
        --locked --no-fail-fast`,
      `cargo test -p finkit-go -p finkit-dotnet -p finkit-java \
        -p finkit-android -p finkit-ios -p finkit-wasm --locked --no-fail-fast`.
      The second set exists because those crates were only ever *compiled* by
      the multi-language workflows; their Rust-side FFI tests never ran.
- [x] Run the TA-Lib transcription gate (`make check-talib-ffi`):
      `python scripts/check_talib_ffi_contract.py`.
      `core/src/talib_ffi.rs` transcribes TA-Lib's `ta_func.h` for the
      head-to-head benchmark and is compiled only under `talib-c`. An unused
      `extern` declaration never reaches the linker, so the benchmark stayed
      green while the file declared `TA_SKEWNESS`/`TA_KURTOSIS` (absent from
      every TA-Lib release), cited an older TA-Lib release beside
      `TA_PERCENTRANK` (which that release does not export), and carried counts
      that agreed with neither the file nor each other. This gate diffs the
      declarations against the committed catalog, offline.
- [x] Run the repository hygiene gates, which fail on orphan automation
      (`make check-orphans`):
      `python scripts/check_orphan_scripts.py`,
      `python scripts/check_workflow_liveness.py`,
      `python scripts/check_orphan_docs.py`.
      A `scripts/` file with no consumer, a workflow whose `on:` block can never
      match an existing branch, or a Markdown document no other document links
      to, reads as something that exists while being unreachable.
- [x] Run the dead-code reason gate (`make check-dead-code`):
      `python scripts/check_dead_code_allows.py`.
      `#[allow(dead_code)]` turns off the only compiler check that can see
      orphan logic, so any suppression without a stated reason is
      indistinguishable from a function that was never wired up.
- [x] Run the rustdoc policy gate (`make check-rustdoc`), which is the
      enforcement point for ADR 0011:
      `bash scripts/check_rustdoc.sh`.
      The wrapper could not locate Windows Cargo through WSL Bash, so the
      equivalent Rust documentation build ran directly with `RUSTDOCFLAGS` and
      `-D warnings`. Note that `RUSTFLAGS` does not affect rustdoc.
- [x] Run the remaining registry/research gates:
      `python scripts/check_orphan_modules.py`,
      `python scripts/check_warning_contracts.py`,
      `python scripts/check_research_ssot.py`,
      `python scripts/gen_dialect_coverage.py --check`,
      `python scripts/gen_talib_numeric_contract.py --check`.
- [x] Run `cargo fmt --all -- --check` and `cargo check --workspace --locked`.
- [x] Run the feature-subset gate (`make check-feature-matrix`):
      `python scripts/check_feature_matrix.py`.
      A feature list is a claim about which subsets compile, and `cargo build`
      only exercises the default one, so the claim has to be executed. It was
      not: on 2026-10-11 the documented `no_std` subset was 49 errors, and a
      `core/Cargo.toml` comment asserted a tree-shaking workflow that produced
      24–155 errors. The gate builds the four supported subsets and asserts the
      recorded-broken ones still fail, so the list can only shrink by a
      deliberate edit. Do not add `--all-features` anywhere: `std` and `no_std`
      are mutually exclusive on purpose.
- [x] Run the Rust test and package gates required by CI:
      `cargo test -p qx-finkit --locked` and `cargo package -p qx-finkit --locked`.
- [x] Build the Python wheel, install it into a throwaway venv, then run
      `python scripts/check_python_stub.py --require-extension --expect-prefix <venv>`
      and the binding test suite from outside the source tree. `--expect-prefix`
      is required: without it the check can silently validate a stale install.
- [ ] Build the per-language usage packages: `bash scripts/build-usage-packages.sh`
      (add `--no-bundle` to skip the zip, drop `--no-verify` to install and run
      each artifact rather than only producing it). Toolchain prerequisites, one
      per language: `maturin` and NumPy for Python, Node plus npm, Maven and a
      JDK for Java, Go/CGO plus a C compiler, the .NET SDK, and the
      `wasm-bindgen-cli` version pinned by `Cargo.lock` for WASM. Node builds
      both the root package and its matching native platform package; Java
      embeds the native library under the exact resource path used by its loader.
      Each language's `dist/<lang>/<platform>` tree is
      **cleared before its build**, so the run's output is the only thing the
      manifest can see; a failing language leaves its tree empty and is reported
      as uncovered instead of silently contributing last month's files.
- [ ] Repack the native C/C++ archive from the freshly built
      `finkit_ffi.dll`/`.lib` plus `ffi/c-binding/include/*` and `LICENSE`:
      `cargo build -p finkit-ffi --release --locked` then
      `python scripts/build_native_archive.py`. Refresh the
      `size_bytes`/`sha256` records in `dist/manifest.json` with
      `python scripts/refresh_release_manifests.py` (`make
      refresh-release-manifests`) — the linker output is not reproducible, so
      the digest changes on every rebuild and a stale record is
      indistinguishable from a fresh one by inspection. `--check` (or `make
      check-release-manifests`) is the verify-only form.
      `dist/manifest.json` has three writers — `scripts/build-usage-packages.sh`
      plus `build-quick.sh` and `build-quick.ps1` at the repository root — and
      all three must agree on the schema; the `name`/`version` fields are
      checked against `core/Cargo.toml` partly so they cannot drift apart again.
      It records the `languages` the run covered, so a subset build is visibly a
      subset. `dist/` is gitignored: `--check` reports an absent manifest as
      skipped rather than failed, and reports a manifest whose name/version
      disagree with `Cargo.toml` as a stale product identity rather than as one
      "artifact missing" line per entry.
- [x] Verify the tag will be exactly `v0.3.2`; `publish.yml` rejects a mismatch.
- [x] Confirm the GitHub Actions secret `CARGO_REGISTRY_TOKEN` is available to
      the repository and has permission to publish `qx-finkit`.
- [x] Confirm the PyPI pending publisher is configured for owner `coeasy`,
      repository `finkit`, workflow `python-wheels.yml`, and environment `pypi`.
- [x] Confirm the release notes cover ordered Alpha158 and Alpha360 feature
      matrices, direct Python/Rust/CLI use, and state that the WorldQuant101
      formula corpus is unchanged pending redistribution-rights confirmation.
- [x] Validate the docs.rs feature allowlist. `std` and `no_std` are mutually
      exclusive, so `--all-features` cannot represent the supported public API;
      metadata and the publish workflow must use the same explicit default
      feature set recorded in `core/Cargo.toml`.

## Tag and crates.io publication

1. Create and push the `v0.3.2` tag from the intended release commit.
2. Let the tag workflows run their version/changelog checks, docs.rs build,
   `cargo publish --dry-run` gate, wheel validation, and PyPI Trusted Publishing.
3. The registry package names are `qx-finkit` on crates.io and PyPI. The Rust
   library target and Python import package remain `finkit`.
4. If the dry run or docs.rs build fails, fix the release commit and create a
   new release candidate tag. Do not bypass the gate or publish manually from a
   dirty checkout.

## After publication

- [x] Confirm `qx-finkit 0.3.2` is visible on crates.io and PyPI; official
      registry metadata reports version 0.3.2.
- [x] Publish workflow passed the crate dry-run/publish gates and the Python
      wheel install tests passed for CPython 3.8–3.14. The PyPI Windows wheel
      SHA-256 matches the corresponding GitHub Release asset.
- [x] The docs.rs feature configuration and crate publication build passed.
- [ ] Confirm the hosted docs.rs page for `qx-finkit 0.3.2` is available; it
      had not appeared when this release record was updated.
- [x] Confirm the GitHub Release contains the four Python ABI3 wheels, the Rust
      crate, CLI binaries, multi-language assets, native installers, and all
      three checksum manifests.
- [x] Update `PUBLISHED_VERSION` to `0.3.2` and make v0.3.2 install commands
      the default in user documentation.
- [x] Keep unverified npm, Maven Central, NuGet, public `go get`, and
      Android/iOS registry coordinates out of the installation instructions.
- [ ] Verify downloaded release files against their checksum manifests in a
      clean consumer environment.
- [ ] Open the optional trial feedback issue when the maintainer requests it.

PyPI serves the correct v0.3.2 package files, but its long description was
generated from the tag's pre-release binding README and still contains v0.3.1
installation guidance. The repository README and this guide now describe
v0.3.2; refresh the PyPI description on the next permitted package publication.

## Current status

- Workspace and latest public release: `0.3.2` / `v0.3.2`.
- crates.io and PyPI: `qx-finkit 0.3.2` published.
- Same-SHA release readiness passed for tag commit `3c11bac7`; publish,
  multi-language, Python wheels, and installer assets completed successfully.
- Release installer upload was retried after the GitHub Release creation race;
  the retry attached all installer files and `SHA256SUMS.installers`.
