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
- [x] Same-commit pre-release workflows passed for CI, docs, Python wheels,
      TA-Lib wheel gate, multilang builds, and competitive benchmarks.
- [x] Tag-driven asset workflows completed successfully; installer publishing
      required one rerun and then succeeded.
- [x] `scripts/check_docs_links.py` and the version contract passed after the
      release-current docs update.

## Publication status

- Workspace version: `0.4.0`.
- Previous published version: `0.3.2`.
- GitHub tag and Release: published at `v0.4.0`; release commit:
  `dde6b19d839623529b1acf7be76d04fa24f2d7b6`.
- crates.io and PyPI: `qx-finkit 0.4.0` verified; four ABI3 wheels are present
  on PyPI.
- GitHub Release assets: 28 verified artifacts, including wheels, crate, CLI,
  language packages, installers, and checksums.
- `PUBLISHED_VERSION` and current-version docs point to `0.4.0`.
