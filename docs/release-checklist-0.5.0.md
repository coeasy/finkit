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
- [ ] Required same-commit release CI and readiness aggregation.
- [ ] Tag-driven release artifact workflows.
- [ ] Verify published crates.io, PyPI, GitHub Release assets, and checksums.

## Publication status

- Workspace version: `0.5.0`.
- Previous published version: `0.4.0`.
- GitHub tag, crates.io, PyPI, and release assets: pending.
- Move `PUBLISHED_VERSION` to `0.5.0` and update current-release prose only
  after independently verifying publication.
