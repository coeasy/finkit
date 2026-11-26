# qx-finkit v0.6.0 Release Checklist

This release adds generic paper-style operator compatibility to the caller-owned
panel expression path. It does not add or expand the built-in WorldQuant factor
catalog, because redistribution rights for the formula content remain
unconfirmed.

## Release scope

- [x] Map `SIGNEDPOWER` / `SIGNED_POWER` to the shared signed-power operation.
- [x] Interpret scalar-window `MIN` / `MAX` as rolling extrema in panel
      expressions while retaining element-wise behavior for panel-valued
      arguments.
- [x] Map panel `STDDEV` to the sample rolling deviation already used by the
      existing transcription.
- [x] Parse dotted identifiers and permit dotted sector, industry, and
      subindustry labels in CLI panel CSV input.
- [x] Parse paper-style `condition ? yes : no` expressions.
- [x] Document caller-owned expression use without adding factor formula text.

## Local build checks

- [x] `cargo fmt --all -- --check`.
- [x] `cargo check -p qx-finkit -p finkit-cli --locked`.

## Same-commit release gates

All required gates evaluated commit `58e1d7692a8e19c616f7f5b500622ed47f5552bf`.
The [release-readiness aggregation](https://github.com/coeasy/finkit/actions/runs/45872159063) passed.

- [x] CI — [run 45867722312](https://github.com/coeasy/finkit/actions/runs/45867722312).
- [x] Docs Check — [run 45867722034](https://github.com/coeasy/finkit/actions/runs/45867722034).
- [x] Python wheels — [run 45867722045](https://github.com/coeasy/finkit/actions/runs/45867722045).
- [x] TA-Lib installed-wheel release gate — [run 45867820093](https://github.com/coeasy/finkit/actions/runs/45867820093).
- [x] Multi-language cross-platform — [run 45867824262](https://github.com/coeasy/finkit/actions/runs/45867824262).
- [x] Multi-language release — [run 45867828147](https://github.com/coeasy/finkit/actions/runs/45867828147).
- [x] Competitive benchmark — [run 45867757035](https://github.com/coeasy/finkit/actions/runs/45867757035).

## Tag workflows

- [x] crates.io publication — [run 45872213343](https://github.com/coeasy/finkit/actions/runs/45872213343).
- [x] Python wheel validation and PyPI publication — [run 45872213361](https://github.com/coeasy/finkit/actions/runs/45872213361).
- [x] Multi-language release assets — [run 45872213297](https://github.com/coeasy/finkit/actions/runs/45872213297).
- [x] Native installers and platform packages — [run 45872213351](https://github.com/coeasy/finkit/actions/runs/45872213351). The upload job was rerun after the GitHub Release was created and then succeeded.
- [x] Tag-triggered cross-platform validation — [run 45872213266](https://github.com/coeasy/finkit/actions/runs/45872213266).

## Published artifacts

- [x] [GitHub Release v0.6.0](https://github.com/coeasy/finkit/releases/tag/v0.6.0) is public and contains 28 assets, including four Python ABI3 wheels, the Rust crate, CLI, native installers, multi-language packages, and `SHA256SUMS`, `SHA256SUMS.installers`, and `SHA256SUMS.multilang`.
- [x] [crates.io `qx-finkit 0.6.0`](https://crates.io/crates/qx-finkit/0.6.0).
- [x] [PyPI `qx-finkit 0.6.0`](https://pypi.org/project/qx-finkit/0.6.0/).
- [x] Formula redistribution status remains unconfirmed; this release contains no new WorldQuant formula content.
