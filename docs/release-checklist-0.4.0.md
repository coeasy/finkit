# qx-finkit v0.4.0 Release Checklist

This record tracks the panel-factor API release. The intended scope is the
general panel executor and its Rust, Python, CLI, and JSON interfaces. It adds
no WorldQuant formula text; the built-in WorldQuant catalog remains partial.

## Release scope

- [x] Add aligned row-major `time × symbols` panel input and output.
- [x] Evaluate temporal and cross-sectional operations in a single expression.
- [x] Expose the panel path through Rust, Python, CLI, and `factor.panel.v1` JSON.
- [x] Keep the 30 numbers absent from the checked-in WorldQuant transcription
      documented as catalog gaps, not as numbers reserved by the paper.
- [x] Keep the redistribution-rights gate for the full WorldQuant formula
      corpus; add no new formula text to this release.

## Local validation

- [x] Panel Rust, CLI, FFI JSON, and Python tests passed.
- [x] Selected Rust crate suites passed: 242 passed, 6 ignored, 0 failed.
- [x] `scripts/check_versions.py` passed for workspace version 0.4.0.
- [ ] Run release CI and same-commit readiness checks before creating the tag.
- [ ] Verify the published crate, Python package, GitHub assets, and checksums.

## Publication status

- Workspace version: `0.4.0`.
- Previous published version: `0.3.2`.
- GitHub tag, crates.io, PyPI, and release assets: pending.
- Update this checklist and `PUBLISHED_VERSION` only after independently
  verifying publication.
