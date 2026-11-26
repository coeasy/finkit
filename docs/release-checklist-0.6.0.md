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

- [x] `cargo fmt --all`.
- [x] `cargo check -p qx-finkit -p finkit-cli --locked`.
- [ ] Same-commit release CI and release-readiness aggregation.
- [ ] Tag-driven crate, wheel, multi-language, and installer workflows.
- [ ] Verify release assets and published package versions.

## Publication status

- Workspace version: `0.6.0` (candidate).
- Previous published version: `0.5.0`.
- GitHub tag and Release: pending.
- crates.io / PyPI packages: pending.
- Formula redistribution status: still unconfirmed; no formula catalog is added
  by this release.
