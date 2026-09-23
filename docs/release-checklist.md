# Finkit v0.2.0 Release Checklist

This checklist is the release boundary for the current workspace version. It
keeps the published v0.1.15 assets separate from v0.2.0 candidates and does not
claim registry installation commands before a clean consumer test exists.

## Before creating the tag

- [ ] Confirm the intended release commit contains the v0.2.0 workspace version.
- [ ] Run `python scripts/check_versions.py`.
- [ ] Run `python scripts/check_changelog.py CHANGELOG.md`.
- [ ] Run `python scripts/gen_ssot_docs.py --check` and
      `python scripts/check_docs_links.py`.
- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run the Rust test and package gates required by CI:
      `cargo test -p finkit --locked` and `cargo package -p finkit --locked`.
- [ ] Verify the tag will be exactly `v0.2.0`; `publish.yml` rejects a mismatch.
- [ ] Confirm the GitHub Actions secret `CARGO_REGISTRY_TOKEN` is available to
      the repository and has permission to publish `finkit`.
- [ ] Confirm the release notes cover the 31 TA-Lib functions, dialect coverage
      contract, Alpha158/WorldQuant101 libraries, and Pine mapping repair.

## Tag and crates.io publication

1. Create and push the `v0.2.0` tag from the intended release commit.
2. Let the tag workflow run its version/changelog checks, docs.rs build, and
   `cargo publish --dry-run` gate.
3. The workflow publishes only the `finkit` crate. PyPI publication is
   intentionally deferred because the `finkit` distribution name is occupied;
   the Python wheel remains a GitHub Release asset until a distribution-name
   decision is made.
4. If the dry run or docs.rs build fails, fix the release commit and create a
   new release candidate tag. Do not bypass the gate or publish manually from a
   dirty checkout.

## After publication

- [ ] Open the crates.io page and confirm version `0.2.0` is visible.
- [ ] Run a clean consumer smoke test in a temporary project using
      `finkit = "0.2.0"` and a minimal indicator call.
- [ ] Confirm docs.rs generated documentation for `finkit 0.2.0`.
- [ ] Confirm the GitHub Release contains the four Python ABI3 wheels, the Rust
      crate asset, the Linux x86_64 CLI, and `SHA256SUMS`.
- [ ] Verify each downloaded asset against `SHA256SUMS`.
- [ ] Only after the registry smoke test passes, add `cargo add finkit` to user
      installation docs and move `PUBLISHED_VERSION` in
      `scripts/check_versions.py` to `0.2.0`.
- [ ] Do not add `pip install finkit`, `npm install finkit`, Maven Central,
      NuGet, public `go get`, or Android/iOS registry coordinates until their
      own publication and clean-consumer tests pass.
- [ ] File the trial feedback issue using
      `.github/ISSUE_TEMPLATE/trial-feedback.md` and link it from the release.

## Current status

- Workspace target: `0.2.0`.
- Latest published release: `v0.1.15`.
- crates.io workflow: implemented in `.github/workflows/publish.yml`, pending a
  real tag run with repository credentials.
- PyPI: deliberately deferred; no `pip install finkit` claim is valid yet.
