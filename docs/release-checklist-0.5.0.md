# qx-finkit v0.5.0 Release Checklist

This release adds caller-owned factor expression libraries and CLI JSON
manifests. It does not add WorldQuant formula text or change the partial
built-in WorldQuant catalog.

## Release scope

- [x] Add a Rust constructor for a named, caller-owned expression library.
- [x] Expose Python `factor_library_from_expressions` and panel single/all
      evaluation.
- [x] Add CLI `--manifest` support for caller-supplied factor expressions.
- [x] Document that expressions are caller-provided and are not bundled by
      Finkit.
- [x] Preserve the full WorldQuant redistribution-rights gate.

## Local build checks

- [x] `cargo fmt --all`.
- [x] `cargo check -p qx-finkit --features formula`.
- [x] `cargo check -p finkit-cli`.
- [x] `cargo check --manifest-path ffi/python-binding/Cargo.toml --features formula`.
- [x] Required same-commit release CI and readiness aggregation on
      `4a5dd6397cdb1b98229efdfb29e5f5d5840493c4`; all seven required workflows
      passed.
- [x] Tag-driven release artifact workflows passed: crates.io publish, Python
      wheels, multi-language builds, cross-platform contract, and installers.
- [x] Verify published crates.io, PyPI, GitHub Release assets, and checksums.

## Publication status

- Workspace version: `0.5.0`.
- Previous published version: `0.4.0`.
- GitHub tag and Release: [`v0.5.0`](https://github.com/coeasy/finkit/releases/tag/v0.5.0),
  commit `4a5dd6397cdb1b98229efdfb29e5f5d5840493c4`, 28 assets.
- `qx-finkit 0.5.0` verified on crates.io and PyPI; PyPI has four ABI3 wheels
  (Linux x86_64, Windows x86_64, macOS x86_64, macOS arm64).
- `SHA256SUMS`, `SHA256SUMS.multilang`, and `SHA256SUMS.installers` verified:
  all 25 listed assets match their SHA-256 digests.
- `PUBLISHED_VERSION` and current-version documentation updated to `0.5.0`.
