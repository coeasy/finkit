# Changelog

## [0.2.0] - 2026-09-23

Trial release. This cycle closes the gap between "the engine can compute it"
and "a user can find out that it can": the formula surface is now described by
a machine-checkable contract, and the factor libraries are defined once and
reused.

### Fixed - 2026-10-03

A pre-release audit that stopped looking for missing features and started
looking for **green that lies**: gates blind to files they never scan, modules
with no caller, and loops whose termination was asserted by a comment rather
than by construction. Seven defects, all fixed, none allowlisted.

- **The orphan gates could not see untracked files.** `check_orphan_docs.py`
  and `check_orphan_scripts.py` read `git ls-files`, which is exactly right in
  CI and exactly wrong locally: a new document no one had `git add`ed was
  invisible, so the gates stayed green and CI went red on the first push.
  Running them with the tree staged surfaced **8 unreachable documents**
  (`docs/new.md`, the plan itself, and six archived AlphaTA-era reports). The
  misnamed `docs/new.md` is now
  `docs/archive/finkit-architecture-v3.1-implementation-plan.md`, and the
  archive index explains that the *deleted* V4 architecture documents are not
  the same thing as the current V4 plan.
- **`workflow_run` was a completely ungated dead-link risk.**
  `release-readiness.yml` — the release licence — waits on eight workflows *by
  display name*; one wrong letter and the trigger never fires, while
  `check_workflow_liveness.py` still reported "every workflow has a trigger that
  can fire" because the file kept its `workflow_dispatch`. The same required
  list is maintained a second time, by file name, in
  `release_readiness_aggregate.py`, with nothing comparing the two.
  `check_workflow_liveness.py` now resolves every `workflow_run` target against
  the declared `name:`s and checks the two lists in both directions. It also
  stops mis-reporting `workflow_run` as a non-automatic trigger — a workflow
  relying on `workflow_run` alone would have been failed as unreachable.
- **7.3 MB of build output was committed, and the no-build-artifacts gate said
  there was none.** `ci.yml` generates `gpu_large_chart.html` (7 MB) by running
  `cargo run --example gpu_large_chart` and consumes it in the next step; five
  sibling files come from `--example improved_chart`. All six were tracked, and
  none matched the gate's `target/`/`dist/`/binary-extension rules, so it
  reported "none look like build output" over 7.3 MB of it. The files are
  untracked (working copies kept), `.gitignore` covers them, and the gate grew a
  repository-root rule for renderable output.
- **`RuntimeContext::ArtifactCache` had no production caller.** Its own
  documentation says it is "the one cache the runtime context owns", replacing
  the per-layer caches that made "is this already compiled?" have a different
  answer in every layer — and nothing ever wrote to it. The plan states
  `content_hash` exists so that "formulas and factors computing the same series
  can share a cache"; both halves existed and were never connected. New
  `UnifiedRuntime::compile_semantic_graph_cached` stores a
  `CompiledPlanArtifact` (plan + CSE report + dependency shape — plain data) in a
  caller-supplied `RuntimeContext`, so a scan compiles one declaration instead of
  one per symbol. The executor is deliberately **not** cached: it owns an arena
  and persistent kernel state, and sharing it would let the second symbol
  overwrite the first one's working set.
- **`execute_into` could advance persistent kernel state on a rejected call.**
  Its documentation promised a "clean rejection rather than a half-updated
  result", but the length check ran *after* `execute()`: a stateful kernel had
  already advanced its arena slot, so a retry with a corrected destination
  computed the *second* value and returned it as the first. The logical length is
  known before the run, so both the count and the length are now validated up
  front and a rejected call has no side effects at all.
- **Cohen–Sutherland clipping terminated only by comment.**
  `visualization::geometry::ClipRect::clip_line` argued that "each pass either
  returns or clips away one outside region, and there are only four" — true in
  exact arithmetic, false in floating point, where rounding can leave the new
  coordinate a hair outside the boundary so the outcode bit survives and an
  unbounded `loop` re-clips the same endpoint forever. It is now `for _ in 0..4`,
  with a residual treated as unclippable: refusing to draw is strictly safer than
  emitting a point that is still outside the rect.
- **`parse_pine` could be made to overflow the stack by its input.** Pine source
  is user input (CLI, Python binding, HTTP), and the entry point had no size or
  depth limit in front of a recursive-descent pest grammar plus a recursive pair
  walk — twenty thousand `(` would abort the process instead of returning a
  `PineError`. A budget check now runs first: iterative and lexical (it must not
  recurse, or it would carry the very failure it prevents), 128 levels of bracket
  nesting and 1 MiB of source, skipping string literals and `//` comments.

Every fix ships with a gate that can fail, and the new gates were verified by
injection rather than by observing green — a gate that cannot go red is worse
than no gate at all.

A fourth pass re-ran the same three questions over the *result* of the first
three, on the theory that a refactor which deletes twenty thousand lines is
itself a fresh source of orphans. It found three more defects — and one of them
had been one command away from being committed as a "fix". The fourth item
below is not a defect but the guard that keeps the first one from returning.

- **`indicators::volume::adosc_into` was deleted by the kernel-unification
  pass, and the gate that noticed was about to hide it.** Deduplicating the
  fused ADOSC implementation onto `math::volume_kernels::adosc_into` was
  correct; forgetting that the public name now had nothing to resolve to was
  not. Nothing failed: `cargo check` cannot see a *removed* `pub fn` that no
  in-tree caller uses, and the unit test named
  `test_adosc_into_matches_allocating_path` had been rewritten to call the
  kernel directly, so the test that looked like it covered the public entry
  point was covering its implementation instead. The only artifact that
  complained was `gen_ssot_docs.py --check`, whose snapshot no longer listed
  `adosc_into` (389 → 388 public functions) — and the documented remedy is
  "run `--generate`", which would have recorded the breakage as the new
  baseline. The function is restored as a validating wrapper over the canonical
  kernel, which also gives it the kernel's stricter pre-conditions (a zero
  period is now rejected, and a too-short series reports
  `TaError::InsufficientData` instead of reading past warm-up). The unit test
  calls the public wrapper again.
- **The class, not just the instance.** New target
  `core/tests/indicator_api_surface.rs` pins all 53 public
  `indicators::<module>::<name>_into` entry points plus the root re-export seam
  in `core/src/indicators/mod.rs`, where an explicit
  `pub use math::volume_kernels::{ad, adosc, obv}` shadows same-named glob
  members from `pub use volume::*` — the exact place `adosc_into` fell through.
  Removing or renaming any pinned name now fails **compilation**, verified by
  injection (`E0425`), so the decision becomes deliberate and reviewable. The
  guard is one-directional on purpose: it constrains removals, not additions,
  so it cannot rot into a list nobody wants to maintain.
- **Two `#[allow(clippy::uninit_vec)]` attributes outlived their reason.**
  `indicators::volume::ad` and `obv` still carried the allow after the same
  refactor replaced their `set_len` blocks with `Array1::zeros` /
  `vec![0.0; len]`, leaving a safety-relevant waiver attached to code that no
  longer does the thing being waived — and inviting the next reader to copy the
  pattern as a licence to skip initialisation. Scanned all six `uninit_vec`
  sites in the workspace; the other four genuinely still call `set_len`.
  **Those four are now `#[expect(clippy::uninit_vec)]` instead of `#[allow]`,
  and the clippy job denies `unfulfilled_lint_expectations`**, so a suppression
  whose justification disappears fails the build rather than silently outliving
  it — which `#[allow]` cannot express at all. Both directions were verified:
  the four legitimate sites are fulfilled under clippy and produce nothing under
  plain `cargo check` with `-D warnings` (tool lints are not evaluated without
  the tool, so the other jobs are untouched), while an expectation planted on
  `volume::obv` — which no longer calls `set_len` — reports *"this lint
  expectation is unfulfilled"*.
- **`gen_ssot_docs.py` rewrote its output with CRLF on every run.**
  `Path.write_text` translates `\n` to the platform separator, while
  `.gitattributes` pins `*.md` to `eol=lf`, so each `--generate` left seven
  files whose *diff* was empty and whose `git status` entry was not. A gate
  directory that is permanently dirty trains people to stop reading
  `git status`. Writes now pass `newline="\n"`.
- **`docs/generated/indicators.md` is therefore unchanged by this pass**, which
  is the point: the restored surface makes the snapshot match again. The
  regenerated file that this pass initially produced has been discarded.

### Fixed - 2026-09-29

This maintenance pass connects the release pipelines end-to-end and removes the
last broken links, orphan logic and silent-pass gates so the project reaches a
publishable state.

- **GitHub Release now auto-builds installers.** New workflow
  `.github/workflows/release-installers.yml` listens to the `release:
  published` event (the "publish a Release from the UI / `gh release create`"
  path that every other tag-driven workflow silently missed) and builds
  native + MSI (Windows), native + deb + rpm (Linux) and native + pkg + dmg
  (macOS), then attaches them to the Release with `--clobber` so re-runs
  repair in place. A `report` job opens an issue if any installer is missing.
- **Installer build scripts are now real.** `scripts/build-installer.sh`
  supports multiple `--target` flags (previously repeated flags silently
  overwrote each other, dropping installers), fails loudly on unknown targets
  and missing tools instead of returning exit 0, and builds a self-contained
  `native` bundle on every host. The committed `packaging/wix/Product.wxs`
  (previously referenced but never committed, making the MSI target dead) now
  defines the Windows installer. `scripts/build-installer-msi.cmd` is reduced to
  a thin wrapper over the bash script.
- **Native SDK archive is cross-platform.** `scripts/build_native_archive.py`
  now builds `linux-x64`, `linux-aarch64`, `macos-x64` and `macos-arm64`
  (`.tar.gz`) in addition to `windows-x64` (`.zip`), which is why it was an
  orphan in CI. `scripts/check_installer_contract.py` asserts the MSI payload
  and the native archive agree on their member list.
- **Gates no longer lie.** `scripts/check_versions.py` ignored three real
  version mismatches (it now passes). `scripts/check_workflow_liveness.py` no
  longer short-circuits to "healthy" on `workflow_dispatch`, so the workflow
  pinned to a deleted `main` branch is now correctly flagged.
- **Release manifest gate is live.** `scripts/refresh_release_manifests.py`
  read a non-existent `artifacts` field (always empty → always passed); it now
  validates the real `components` entries and refreshes digests in place.
- **Makefile ghosts removed.** `make packages` (always failed), `make
  docker-bench` (no recipe) and the missing `installer` entry point are fixed.
- **Node binding type contract fixed.** Two Rust structs both named
  `MacdResult` (vector in `lib.rs`, scalar in `streaming.rs`) produced two
  `MacdResult` interfaces in `ffi/node-binding/index.d.ts`, a duplicate-identifier
  error for TypeScript consumers; the streaming struct is now
  `StreamingMacdResult`.
- **API reference corrected.** Go (function names, returns, module path),
  .NET (method names, `MacdResult`/`BbandsResult` fields, example), Python
  (exception class names), Rust (removed non-existent `bollinger_bands` /
  `head_shoulders`) and Node.js (camelCase names, required params, `cdlDoji` /
  `detectDoubleTop` moved out of `Indicators`) now match the actual exports.
  Java `macd`/`bbands`/`stoch` are correctly documented as `void` methods that
  write into a pre-allocated result object; pattern functions live in `Patterns`
  / `ChartPatterns`.

### Added

- `scripts/check_coverage.py` is wired into CI so it is no longer an orphan.
- `scripts/check_installer_contract.py` is wired into CI.

- The 31 TA-Lib 0.7/0.8 indicator functions that were numeric-green but
  unreachable from the formula engine: `AC`, `ACCBANDS`, `ADR`, `AO`, `AROON`,
  `CMOU`, `COPPOCK`, `CVI`, `EFI`, `ER`, `ERI`, `FOSC`, `FRACTAL`, `HA`, `KC`,
  `MAMA`, `MARKETFI`, `MASSI`, `NVI`, `PERCENTRANK`, `PVI`, `PVO`, `PVT`,
  `QSTICK`, `RVI`, `RVOL`, `SMI`, `VHF`, `VORTEX`, `WAD`, `ZLEMA`, together with
  their multi-output registrations.
- `tests/contracts/formula_dialect_coverage_v1.json`, a per-function coverage
  contract for 通达信 / 同花顺 / 大智慧 / TradingView Pine. Each row carries
  three independent facts: `status` (usable out of the box?), `registered` (a
  machine-derived fact about the live function table) and `provenance`
  (substantiated by a named in-repo file, or merely attributed to a vendor
  list). Each terminal also records which rows a checked-in corpus actually
  calls, recomputed by the gate from the corpus files.
- 大智慧 as the sixth `FormulaTerminal` (`dzh` / `dazhihui` / `大智慧`),
  routing the 同花顺 common subset.
- Alpha158 (158 factors) and WorldQuant101 (17 computable, the rest annotated
  with the reason they are not) as declarative factor graphs, plus a one-line
  `finkit.factor_library("alpha158")` entry point in Rust and Python.
- Pine user-defined function blocks, tuple destructuring, and plot visual
  metadata.
- `scripts/build_native_archive.py`, a builder for the native C/C++ SDK archive
  (`dist/native/<platform>/finkit-<version>-native-<platform>.zip`). The archive
  used to be hand-zipped, so its member list and timestamps drifted from release
  to release. It now packs a fixed member set with a fixed 1980-01-01 timestamp
  (so identical inputs give a byte-identical archive), warns when the target
  directory it did *not* choose holds a differing `finkit_ffi.dll` (two build
  trees can coexist and linker output is not reproducible), and offers
  `--verify` to confirm the archive on disk still matches the current build.
  Wired into `make build-native-archive` / `make verify-native-archive`.
- `scripts/check_orphan_scripts.py`, a gate that fails when a file under
  `scripts/` has no consumer anywhere in the tree. A dead script reads as
  documentation of a workflow that no longer exists, and a second, unused
  implementation of a gate silently lowers the bar for the one that runs.
- `scripts/check_workflow_liveness.py`, a gate that fails when a GitHub Actions
  workflow can never be triggered — every trigger is branch-filtered, no
  filtered branch exists, and there is no `workflow_dispatch`/`schedule`/
  `release`/`workflow_call`. It also reports *dormant* branch filters (filters
  matching no existing branch) on workflows that are still reachable, so a
  stale `on:` block is visible without failing the build.
- An enforcement point for ADR 0011: `scripts/check_rustdoc.sh` now runs from
  `make check-rustdoc` and from the CI `doc` job, so a local run and CI cannot
  disagree about what "the public surface is documented" means.
- `scripts/check_script_references.py`, the inverse of the orphan check: it
  fails when a workflow step, Makefile recipe or document invokes a `scripts/`
  path that is not in the tree. A reference like that has a name, a place in
  the release checklist, and often a paragraph explaining what it enforces —
  and no implementation, so nothing compiles it and nothing tests it. Paths
  that are legitimately absent (planned work, or a helper the caller writes
  itself before running it) must be recorded in `RECORDED_MISSING` with a
  reason, and the list is checked in both directions. Wired into the CI
  `binding-ssot` job and `make check-script-refs`.
- `scripts/refresh_release_manifests.py`, which recomputes the `size_bytes` /
  `sha256` records in `dist/manifest.json` and
  `dist/python/windows-x64/manifest.json` from the artefacts on disk and then
  re-reads what it wrote to confirm the match. Those numbers were refreshed by
  hand, which is the same failure mode the native archive already had: linker
  output is not reproducible, so every rebuild changes the digests and a stale
  record is indistinguishable from a correct one by inspection. `--check` is
  the verify-only form. Wired into `make refresh-release-manifests` /
  `make check-release-manifests`.
- `scripts/check_dead_code_allows.py`, which fails on any
  `#[allow(dead_code)]`, `#![allow(dead_code)]` or `expect(dead_code)` that
  does not carry a `// why` comment on the line, inside the attribute block
  above it, or on the line directly above that block. That attribute disables
  the only compiler check that can see orphan logic, so an item behind it is
  invisible to every build and every test, and a bare suppression is
  indistinguishable from a function that was never wired up. Wired into the CI
  `binding-ssot` job and `make check-dead-code`.
- `scripts/check_orphan_docs.py`, which fails when a tracked Markdown document is
  reachable from no other document, by link or by backticked repository-relative
  path. Part of `make check-orphans`.
- `scripts/check_ios_header_contract.py`, which compares
  `ffi/ios-binding/include/finkit.h` against the iOS binding's shipped
  `#[no_mangle] extern "C"` exports in both directions, ignoring
  `#[cfg(test)]`-gated symbols. It is the iOS counterpart to
  `gen_c_header.py --check` for the C binding.

### Changed

- `FormulaExecutionMode` is now a real execution switch. The default is still
  the tree interpreter; the compiled plan path is selected explicitly and is
  covered by a differential gate against the interpreter.
- The Pine canonical-name mapping is derived from the engine (the builtin
  table plus the `ast_mapper` special cases plus the `TA_<NAME>` fallback) and
  recorded in the coverage contract, instead of being mirrored by hand in the
  generator.
- The release version gate now covers `docs/getting-started.md`,
  `docs/cli.md`, `docs/language-bindings.md` and `docs/development.md`, and it
  matches any `MAJOR.MINOR.PATCH` series rather than only `0.1.x`. Those four
  documents had drifted to `v0.1.5` while the workspace was at `0.1.15`, and
  the old pattern could not have caught it.
- Removed the unreferenced `formula::pine::runtime` module (`PineRuntime`,
  `PineRuntimeError`, `SeriesValue` and its plot/security/barstate support
  types). It was introduced in the initial commit and never had a caller
  anywhere in the workspace, including the language bindings and the test
  suite. It was a second, parallel Pine evaluator for semantics the mapper plus
  the AlphaTA engine already own, so it could only ever drift from them.

### Fixed

- **45 float-ordering comparisons panicked on a NaN input.** Each
  `partial_cmp(..).unwrap()` aborts the whole evaluation as soon as a window
  holds a `NaN`, because `f64::partial_cmp` returns `None` for `NaN` and the
  `unwrap` then panics. That is the opposite of this codebase's
  `null_policy: "nan"` convention, where a gap value should propagate as `NaN`
  rather than crash. 34 of the sites were in `patterns/chart.rs` (every chart
  detector: `double_top` / `double_bottom`, `head_and_shoulders_top` /
  `_bottom`, `triple_top` / `_bottom`, the ascending/descending/symmetrical
  triangles, the rising/falling wedges, `pennant`, `flag` and `rectangle`), and
  the rest in `indicators/talib_ext.rs` (`percentile`),
  `indicators/volume_profile.rs`, `indicators/classic_patterns.rs`,
  `patterns/astock_kline.rs`, `patterns/classic_ext.rs` and
  `patterns/streaming.rs`. All now use `f64::total_cmp` — a total order that
  never returns `None`, and already the house style in `math/quantile.rs`,
  `math/rank.rs`, `math/regression.rs` and `factors.rs` — so a `NaN` propagates
  instead of aborting the process. Guarded by a new
  `scripts/check_nan_unsafe_ordering.py` (`make check-nan-safety`) that fails on
  any `partial_cmp(..).unwrap()` in the scanned Rust sources, and by regression
  tests that feed `NaN` to `percentile` and to the chart detectors.

- **Eleven functions were unusable from the compiled-plan path, and 28 names
  were affected.** `BARSLAST`, `BREAKDOWN`, `BREAKOUT`, `COUNT`, `DEAD_CROSS`,
  `GAP_SIGNAL`, `GOLDEN_CROSS`, `MA_ALIGN`, `RELATIVE_STRENGTH`,
  `TREND_BREAKOUT` and `VOLUME_SURGE` were declared pure, so the planner
  lowered them to `CALL:<NAME>` kernels — but no such kernel existed, and every
  formula using one failed with `unsupported kernel` (code 1). Because the
  planner resolves registry aliases to their canonical name, another seventeen
  spellings broke with them: `CROSSUP` / `BULLISH_CROSS`, `CROSSDOWN` /
  `BEARISH_CROSS`, `BREAKOUT_UP` / `PRICE_BREAKOUT`, `BREAKOUT_DOWN` /
  `PRICE_BREAKDOWN`, `BREAKOUT_SCREEN` / `TREND_SCREEN`, `VOLSURGE` /
  `VOLUME_EXPANSION`, `MA_ALIGNMENT` / `TREND_ALIGN`, `RELSTRENGTH` /
  `RS_EXCESS_RETURN` and `GAP`. No corpus case exercised any of them, so the
  differential gate stayed green throughout. Each now has a kernel delegating
  to the exact `canonical_*` (or `fn_count` / `fn_barslast`) implementation the
  tree, bytecode and JIT paths already used, so the plan path agrees by
  construction. Covered by
  `domestic_screening_formulas_match_the_ast_through_the_plan_path`, and by a
  new invariant `every_alias_reaches_a_kernel_through_its_canonical_name`,
  which is what actually protects an alias — checking `CALL:<alias>` directly
  would have measured something no caller can reach and stayed green through
  the whole outage.

- **The registry told callers the wrong input shape for seventeen functions.**
  `ICHIMOKU_TENKAN`, `ICHIMOKU_KIJUN`, `FISHER`, `FISHER_SIGNAL`, the five
  `DONCHIAN*` projections and `AROON_UP` / `AROON_DN` were declared
  `InputKind::Hlc` — high, low, *close*, plus a period — while their
  implementations take high, low, period. A caller who followed the metadata
  wrote `ICHIMOKU_TENKAN(HIGH, LOW, CLOSE, 9)` and got a tenkan-sen computed
  over a period of `CLOSE[0]` (about 100, not 9) on the tree path, and a hard
  arity error on the plan path. The enum had no way to say "two series", which
  is why `Hlc` was used as an approximation; it now has `InputKind::Hl`.
  A new gate, `declared_signatures_run_on_both_execution_paths`, rebuilds the
  call the metadata describes from the registry itself and runs it through both
  paths, so this class of drift cannot recur silently.

  The same investigation found that the *parameter* half of the declaration is
  a superset for five functions — `BBANDS` / `BOLLUP` / `BOLLDN` publish
  `nbdevup`, `nbdevdn` and `matype`, and `STDDEV` / `VAR` publish `nb_dev`.
  Those were **not** removed, because the operation and FFI surfaces really do
  accept them (`finkit.bbands(real, timeperiod, nbdevup, nbdevdn, matype)`,
  `finkit.stddev(close, timeperiod, nbdev)`); it is the formula surface that
  takes a shorter list, since a formula evaluates to a single series. The five
  are now recorded in the gate's `FORMULA_PARAM_SUBSET` list with the reason,
  and an entry that stops being needed fails the test.

- **`tests/formula_corpus/README.md` still said the plan path had no kernels.**
  A current (not dated-snapshot) document asserted that the 31 TA-Lib 0.7/0.8
  functions were available "only on the tree / bytecode / JIT paths", that
  `CALL:<NAME>` answered `code 1`, and that all 17 corpus cases were therefore
  registered in `DOMESTIC_UNSUPPORTED`. None of that has been true since
  2026-09-24: `unified_dispatch.rs` routes all 44 `CALL:<NAME>` plan kernels
  through `dispatch_modern_call`, which delegates each to the same
  `canonical_*` implementation the other paths use, so the plan path emits
  byte-identical output and the allowlist is empty. The prose now states the
  closed state and keeps the allowlist documented as the backlog mechanism it
  is. The corpus table itself was already bidirectionally gated; only the
  surrounding prose could drift.

- **`docs/api-reference.md` omitted seven shipped Python API groups.** The
  reference documented the classic TA surface and `Streaming*`, but said
  nothing about the Chanlun entry points (`chan_analyze` and its three
  multi-timeframe variants), `compute_composite`, `factor_library`, the formula
  template catalog (`formula_get_template` / `formula_search_templates` /
  `formula_list_categories` / `FormulaRegistry`), `CompiledFormula`, the market
  calendar (`resolve_market_session`) or `KlineChart` — all 18 of which are
  exported by the wheel. They are now documented, with the return shapes
  verified against a built wheel rather than inferred.
- **`factor_library` was documented nowhere and missing from the type stub.**
  It ships as a 0.2.0 entry point but appeared in neither `docs/python.md` nor
  `docs/api-reference.md`, and `.pyi` did not declare it, so a `mypy`/
  `pyright` user got a missing-name error on a public API. Both the docs and
  the stub now cover it. Its `FactorLibrary` return is documented as
  supporting `len()` and `in` but **not** iteration and **not** subscripting —
  it exposes neither `__iter__` nor `__getitem__`, so enumerate it with
  `names()`.
- **`docs/python.md` told users to iterate a `FactorLibrary`.** A first draft
  of the new section said to inspect it "with `len()` / iteration"; it is not
  iterable, so the example was corrected to `names()` before it shipped.

- **Six binding crates had `#[test]` functions that CI never ran.** `ci.yml`
  tested `finkit`, `finkit-factor-analysis`, `finkit-cli`,
  `finkit-visualization` and `finkit-ffi-common`; `multilang-release.yml` built
  `finkit-go`, `finkit-dotnet`, `finkit-java`, `finkit-android`, `finkit-ios`
  and `finkit-wasm` but only *compiled* them, and it drove the C/C++ `ctest`
  suite without ever running `cargo test -p finkit-ffi`. That left 55 Rust unit
  tests unexecuted — 31 in the C binding alone, including the
  `dispatch_ta`-unknown-name test that guards a fix from the previous round.
  `cargo test -p finkit-ffi` now runs in `ci.yml` and in the `v*`-tag job, and
  a new `binding-unit-tests` job covers the other six crates on every push and
  pull request.
- **The shipped Python type stub did not cover the documented surface.** Its
  own docstring promises it "covers the documented surface rather than all 400+
  exported names", but 16 documented, registered functions were missing from
  it: `formula_get_template`, `formula_search_templates`,
  `formula_list_categories`, `bbands`, `compute_indicators`, `cdl_doji`,
  `cdl_engulfing`, `cdl_hammer`, `detect_double_top`, `detect_double_bottom`,
  `detect_head_shoulders`, the four `chan_analyze*` entry points and the
  `FormulaRegistry` class. Each was callable at runtime and shown in `docs/`,
  and each was a type error for a `mypy`/`pyright` user. All 16 are now
  declared, and `check_python_stub.py` gained the reverse check that would have
  caught them — it previously only verified that a name *declared* in the stub
  exists, never that a documented name *is* declared.
- **`docs/generated/version-matrix.md` recorded a number that could not be
  correct in CI.** The "Criterion JSON benchmarks indexed: N" line counted files
  under a hard-coded `target/criterion/`, but this repository builds to
  `.cargo-target` (or `$CARGO_TARGET_DIR`), so the directory never existed and
  the line always read `0`. Pointing the generator at `$CARGO_TARGET_DIR`
  instead would have been worse: the count is a property of one developer's
  disk, so the committed document would disagree with CI and the
  `gen_ssot_docs.py --check` gate would flap. The count and the ~80 lines of
  helper code that computed it are removed; the section now points at
  `scripts/gen_benchmark_report.py`, which is the real local report path. The
  gate is now independent of the environment, verified both with and without
  `CARGO_TARGET_DIR` set against a populated `.cargo-target/criterion`.
- `docs/installation.md` §14a listed six members for the native C/C++ archive
  when `scripts/build_native_archive.py` packs eight — both
  `finkit_research.h` and `finkit_research.hpp` were missing from the table, so
  a reader following it would have shipped a bundle without the research
  headers. The section also omitted `$CARGO_TARGET_DIR` from the documented
  target-directory resolution order, which is the one step that matters in a
  checkout that redirects cargo.
- **`docs/archive/` was in `.gitignore` while 11 archived plans were already
  tracked.** Any newly archived document therefore became untracked *and*
  invisible to `git status`, so moving a file into the audit trail silently
  deleted it from the repository. The consequence was live, not theoretical:
  `docs/archive/README.md` — the archive index that `docs/README.md` links to
  three times — had never been committed, so `scripts/check_docs_links.py`
  reported success locally (the file was on the author's disk) and failed on a
  clean checkout. The rule is removed, the index is committed, and
  `check_docs_links.py` now fails when a link target exists on disk but is not
  tracked by git.
- **`docs/ffi/memory-contract.md` documented an API that does not exist.** It
  described `alphata_free_string` and an eleven-function `alphata_kline_chart_*`
  / `alphata_kline_data_*` chart surface, none of which is exported by any
  binding (the chart surface exists for the Java binding and the CLI only), and
  it omitted the entire JSON-contract surface plus the research pair — 18 real
  exports with no documented ownership. The document is the ownership contract a
  C consumer follows to avoid undefined behaviour, so a fictional free function
  in it is a memory-safety trap. All 99 shipped exports are now documented and
  the invented names are gone; `docs/ffi/error-codes.md` had the same
  `alphata_free_string` reference and additionally truncated the formula error
  tier at `55`–`59`, leaving `60` and `61` undocumented.
- `FormulaError::BackendUnsupported` — the variant that makes tree-only entry
  points *refuse* to run under `FormulaExecutionMode::Plan` instead of silently
  walking the tree — was mapped to C error code `61` but documented nowhere.
  `docs/formula-runtime-contract.md` now states the backend contract, lists the
  13 tree-only entry points and the plan-capable ones, and
  `docs/ffi/error-codes.md` spells out the full `50`–`61` formula tier.
- **`scripts/check_orphan_docs.py`** (new gate): `docs/formula-talib-contract.md`
  and `docs/quant-evaluation.md` were both current, substantive documents with
  zero inbound references anywhere in the repository, so the documentation index
  could not reach them. Both are indexed now, the dated competitive analyses and
  roadmaps are indexed under an explicitly non-authoritative section, and the
  gate fails if a document ever becomes unreachable again.
- **`scripts/check_ios_header_contract.py`** (new gate):
  `ffi/ios-binding/include/finkit.h` was missing declarations for three shipped
  exports (`finkit_ios_factor_study_json`, `finkit_ios_factor_study_free_string`,
  `finkit_ios_quant_evaluation_json`). The Swift wrappers reach them through
  `@_silgen_name`, which is exactly why nothing noticed — but a plain C consumer
  of the iOS static library could not see that the research and quant-evaluation
  entry points existed. The header is complete and the gate keeps it that way.
- **The scheduled TA-Lib head-to-head guardrail installed TA-Lib C 0.7.1 while
  the numeric contract pins 0.8.1.** The guardrail therefore measured Finkit
  against a superseded upstream and could pass more easily than its threshold
  implies. `competitive-benchmark.yml` now reads `talib_core_version` from
  `tests/contracts/talib_coverage_matrix_v1.json` at run time and refuses to
  record evidence when the installed library disagrees, so the two can no longer
  drift.
- `scripts/check_versions.py` covered 15 release-facing documents and missed 10
  more that state Finkit release versions — including `docs/usage.md`,
  `docs/troubleshooting.md`, `CONTRIBUTING.md` and `core/README.md`, which had
  rotted to `v0.1.5` and `v0.1.3` while the published release was `v0.1.15`.
  The list is extended and the stale values corrected.
- `docs/formula-templates.md` claimed 317 templates but printed a per-language
  export table with rows for C and iOS template lookup functions that are not
  exported by those bindings; `docs/formula-performance.md`'s multi-language
  parity table marked C as supporting `formula_eval`, `_multi`, `_draw`,
  `_debug` and `_validate`, none of which the C ABI has. Both tables are
  rebuilt from the actual exports.
- `docs/indicators.md` listed two indicators that do not exist
  (`KAMA_VOLATILITY`, `MEDIPRICE`) and documented the indicator error type as
  `Result<Array<TaError>>` with three variants (`InvalidPeriod`,
  `InvalidParameters`, `InvalidInput`) that no error enum defines.
  `docs/features.md` documented `MultiPeriodFeature::fast_periods/medium_periods/
  slow_periods` (the methods are `fast`/`medium`/`slow`), wrong default period
  sets, an unsupported indicator list, and `to_csv`/`to_json_lines`/
  `to_arrow_ipc` as returning strings when they write to a path.
  `docs/api-reference.md` gave `StreamingIndicator::next` a non-`Option` return
  and a `&dyn Ohlcv` input for `StreamingAtr`, which takes a `(high, low, close)`
  tuple.
- Java JNI multi-output exports (`MAMA`, `BBANDS`, `SAR`, `MACD`, `STOCH`, and
  `AROON`) could unwind a Rust panic across the JNI boundary because the binding
  synchronizer only wrapped functions with an explicit `->` return type. They
  return `()` and populate a result object, so the old parser skipped them.
  `sync_bindings.py` now recognizes Java void bodies and wraps them with
  `ffi_catch_void`; the generated six exports are guarded, and the structural
  warning contract prevents the hole from returning. The same pass wrapped 21
  unguarded manual Java exports (including chart and research bridges) with the
  return-type-appropriate guard.
- Five private functions were unreachable from every build and every test, and
  nothing reported it: each carried a bare `#[allow(dead_code)]`, which is
  precisely the attribute that hides an item from the compiler's dead-code
  check. Removing the suppressions and letting `cargo check` adjudicate found
  them, and they are gone — `push_sliding_max`, `push_sliding_min`,
  `aroonosc_scan_inner` and `aroonosc_deque_inner` in
  `core/src/indicators/momentum.rs` (two deque helpers with no call sites, and
  two alternative AROONOSC kernels superseded by the inlined deque loop in
  `aroonosc` itself, 178 lines), and `null_pointer` in
  `ffi/c-binding/src/lib.rs`, a second implementation of the
  `FfiError::NullPointer` arm already inside `map_ta_error`.
- Six `#[allow(dead_code)]` suppressions were stale: the item they guarded is
  genuinely used, so the attribute only served to make a future regression
  invisible. Removed from `bop_scalar`, `avgprice_scalar`,
  `avx2_fma_available`, `ta_t3`, `approx_eq` and the `Case::fixture_path`
  field (also deleted: `approx_eq` in the `top_bottom` test module, which the
  test build proved unused).
- The repository-hygiene gates were blind to any tracked file whose path
  contains a non-ASCII byte. `git ls-files` octal-escapes such paths
  (`"docs/...finkit-\350\220\275..."`), so `Path.is_file()` failed and nine
  tracked files — every one of them a document that mentions `scripts/` paths —
  silently dropped out of the scan. Both checks now enumerate with
  `git ls-files -z`.
- `scripts/check_orphan_scripts.py` could be fooled in two ways, both of which
  made a dead script look alive. A cluster of scripts that only mention each
  other satisfied the one-hop "is it mentioned anywhere" test for every member,
  and a bare stem matched as a substring, so a script named `_probe_a.py`
  counted as referenced by
  `fn the_probe_actually_exercises_the_degenerate_guard()`. References are now
  peeled to a fixed point and stem hits must fall on identifier boundaries.
- Four one-shot migration-bot workflows were still in the tree after their work
  landed: `apply-architecture-v3-round2.yml` and `apply-extrema-round6.yml`
  pushed to `perf/outperform-talib-v3-20260904`, and `apply-perf-plan.yml` and
  `apply-talib-performance-plan.yml` pushed to `fix/*` branches that no longer
  exist. They carried `contents: write`, and the first embedded a 27 KB base64
  payload that rewrote seven source files and force-pushed, so its own command
  line named a script that was never in the tree. Removed.
- Six completed TA-Lib migration codemods that nothing invoked:
  `apply_core_hotpath_fixes.py`, `apply_formula_output_fixes.py`,
  `apply_formula_runtime_performance_fixes.py`,
  `apply_talib_existing_contract_fixes.py`, `apply_talib_semantic_fixes.py`
  and `normalize_migration_parsers.py`. They rewrite Rust sources, so running
  one against today's tree would have applied a stale transformation; their
  effect is preserved by the commits that landed it.
- `make verify-native-archive` was red in any checkout that redirects cargo
  with `CARGO_TARGET_DIR`. The archive is packed from the tree cargo actually
  wrote, but `--verify` looked in `target/release` first and compared against a
  `finkit_ffi.dll` built in a different tree — and linker output differs
  between trees, so the comparison could never succeed and the message gave no
  clue why. The script now locates the release directory the way cargo does
  (`--target-dir`, then `$CARGO_TARGET_DIR/release`, then `target/release`),
  and when a member does differ it names the tree the archive *was* packed
  from and the flag to pass.
- `docs/refactor-plan-2026-09-21.md`, the declared sole execution baseline,
  pointed at `docs/improvement-plan-2026-09-20.md` and
  `docs/architecture-gap-assessment-2026-09-20.md` at their pre-archive
  locations, in the same sentence that says historical plans have been
  archived. Both paths now carry the `docs/archive/` prefix.
- Pine `na(x)` was silently mis-parsed. `na` is a grammar keyword, so it could
  not reach the call rule, and `na(close)` parsed *without error* as an `na`
  literal followed by a discarded `(close)` expression statement — so
  `is_missing = na(close)` evaluated to `NaN` instead of `ISNA(CLOSE)`. The
  grammar now has an explicit `na_call` rule.
- The Pine coverage figure was a false green. The generator's hand-written
  canonical-name mirror disagreed with the engine on 7 names, left 48 spellings
  unmapped, and credited 43 unusable names as out-of-the-box, reporting 97.8%
  coverage. Deriving the mapping from the engine drops it to the real 81.7%
  (263 rows, 47 `unsupported`).
- Formula and plan kernel correctness for `IF`, `STOCHF`, `SAR`, `BOLLUP` /
  `BOLLMID` / `BOLLDN`, `DEA`, `WILLR`, `PLUS_DI` / `MINUS_DI` / `ADX`, `TRIX`,
  `STDDEV`, `CROSS` / `FIXNAN` / `VAR`, and the money-flow / `TR` host kernels.
- External variables bound through `HostContext` are now visible on the
  bytecode and JIT paths, which previously resolved only their own private
  tables.
- `MACDFIX` used MACD's `2/(period+1)` EMA constants instead of TA-Lib's fixed
  `0.15` / `0.075`, so `MACDFIX(CLOSE, 9)` on the formula path disagreed with
  `indicators::macdfix` on the very same input. Every differential path agreed
  with every other path — on the wrong constants — so only an absolute
  comparison against the reference implementation could see it. The formula
  function now delegates to the parity implementation.
- The MACD family (`MACDFIX`, `MACDEXT`, `DIFF`, `DEA`) treated a leading
  warm-up run as bad data instead of skipping it, so a composed input such as
  `MACDFIX(MA(CLOSE, 5), 9)` collapsed to an all-NaN series. The plan path's
  `MACD` and `DEA` kernels carried the same gap independently of the tree path,
  which the corpus never exercised.
- `MACDEXT`'s three `matype` arguments were read with the period extractor,
  which rejects `0` outright. The canonical TA-Lib spelling
  `MACDEXT(close, 12, 0, 26, 0, 9, 0)` therefore failed with
  "period must be > 0", and the `0 => SMA` match arm behind it was unreachable:
  only the `EMA` selector ever worked.
- Pine `barstate.*` mapped to a variable name that no execution path resolved
  (`barstate_islast` and friends), so any script reading `barstate` parsed,
  mapped, and then failed with "Unknown variable". The fields are now
  translated in the mapper into `BARPOS` / `BARSCOUNT` comparisons, which keeps
  the decision inside the frontend and out of the five numeric backends.
- Pine's `hl2`, `hlc3`, `ohlc4` and `time` had the same defect: they were
  renamed to `HL2` / `HLC3` / `OHLC4` / `DATE` and handed to the engine as
  variable references, and none of those names resolves. The derived price
  sources are now expanded into OPEN/HIGH/LOW/CLOSE arithmetic (identical to
  `streaming::PriceSource`) and `time` is emitted as a call to the
  zero-argument `DATE()` builtin. The Pine corpus runner had been pre-binding
  `HL2` / `HLC3` / `OHLC4` into the context, which is exactly why a green
  corpus gate never noticed; that workaround is removed.
- SIMD documentation no longer claims acceleration that does not exist. The
  Hilbert section in `indicators::cycle` said the pipeline was AVX2-accelerated
  by the `simd_ht_*` kernels and that `ht_sine`'s terminal stage batched through
  `simd_sin_cos`; in fact the production Hilbert chain is a scalar TA-Lib-faithful
  state machine and `simd_sin_cos` is only reached from a unit test. Likewise
  `simd_ht_dcphase` documented an AVX2 path and radian output that its body does
  not provide (it is a scalar, degree-returning approximation), and
  `simd_aroon` / `simd_kama` / `simd_ema_next` / `simd_mama_hilbert` /
  `simd_sar_step` / `simd_atr` documented AVX2 batching that their scalar bodies
  do not perform. The module docs for `math::simd_ops` and `math::simd_ops_avx512`
  now state the real dispatch policy (AVX-512 falls back to scalar for every
  kernel except `simd512_sma`) and the `avx512` coverage table no longer lists
  non-existent `simd512_macd` / `simd512_bbands` / `simd512_atr` / `simd512_adx`
  names (they are `*_seed`). No numeric behaviour changed — these are
  documentation↔reality corrections.

- Streaming `OBV` diverged from the batch `obv` / `simd_obv` contract on flat
  bars. It accumulated `diff.signum() * volume`, and `f64::signum(0.0)` is
  `+1.0`, so a bar with an unchanged close wrongly added `volume` instead of
  leaving OBV unchanged (the comment even claimed the opposite). The streaming
  step is now an explicit three-way `> prev` / `< prev` / flat, matching the AVX2
  and scalar batch kernels. A regression gate pins the two implementations
  together on shared bars including a flat one.

- The formula streaming path (`FormulaStatefulStream`) was a differential
  blind spot: `check_all_paths` never instantiates it, so two of its stateful
  indicators silently disagreed with the batch formula table.
  1. `SMA(X, N[, M])` was implemented as a simple moving average (warm-up `NaN`
     prefix), while the batch `fn_sma` is recursive smoothing seeded at the
     first value (`value = (M*cur + (N - M)*prev) / N`). The streaming path now
     has its own `StreamingSmaSmoothed` and routes `SMA` (distinct from `MA`) to
     it in both `compile_state` and `compile_expression_stateful_function`, with
     the optional third weight argument honoured.
  2. `MACD(...)` streaming returned the histogram (`macd - signal`) while the
     batch `MACD` formula returns the DIF line (`fast_ema - slow_ema`); the two
     are different series. The streaming `MACD` now returns `value.macd`.
  A new gate `streaming_stateful_matches_batch` compares MA / SMA / EMA / WMA /
  RSI / HHV / LLV / SUM / REF / STD / VAR / CROSS / CROSSBELOW / ATR / MACD
  (direct and composed) against the batch (AST) path on oscillating data,
  closing the blind spot.

- The streaming formula path poisoned its rolling accumulators on a leading
  `NaN` run. A composed source such as `MA(MA(CLOSE,5),9)` (or raw input that
  opens with a warm-up `NaN` prefix) handed the outer indicator a leading `NaN`
  run; the batch math layer skips it via `math::leading_warmup`, but the
  streaming accumulators absorbed the `NaN` and returned an all-`NaN` series.
  Both the direct state kernels and the composed expression kernels now hold
  the indicator until the first finite input arrives, so its window starts on
  the first finite value exactly like the batch path. `RSI` is deliberately
  exempt: its batch kernel computes `change.max(0.0)` (and
  `NaN.max(0.0) == 0.0`), so it treats a leading `NaN` delta as a zero change
  and keeps its warm-up at `period`; feeding the `NaN` through reproduces
  `rsi_scalar` exactly, whereas skipping would shift the warm-up by the run. A
  second gate `streaming_stateful_survives_leading_nan` locks this on a context
  whose columns open with a `NaN` run.

- The `%` operator had four implementations that disagreed with the other eight.
  The dialect contract is the **floor-based** remainder (`-7 % 3` is `2`), which
  the tree scalar and array kernels, the bytecode VM, the plan's `BINARY:Mod`
  kernel and the streaming path all implement; but `FormulaOptimizer`'s
  constant folder, the JIT's bytecode folder, the JIT's *runtime* `OpCode::Mod`
  and `compute_ir`'s `const_eval` used Rust's truncating `%` (`-7 % 3` is `-1`).
  A folder that changes the value of a formula is the worst form of this: with
  literals the optimizer folded `-7 % 3` to `-1`, while the equivalent
  `CLOSE % 3` on the same bar produced the floor form. All eleven sites now
  call `math::floor_remainder`. `MOD(A, B)` the *function* stays truncating by
  design; a new gate pins the two apart, and `%` gained a negative-dividend case
  in the four-way harness (the harness data is strictly positive, so
  `CLOSE % 3` alone is satisfied by either definition).

- The logical operators had two truthiness conventions. `truth.rs` records that
  `And` / `Or` / `Xor` / `Not` treat a value as true only when it is **strictly
  positive** (deliberately unlike `IF`'s branch selection, which is
  `!= 0.0`), and eight implementations follow it — the scalar kernels, the
  bytecode VM, the JIT, the plan's `BINARY:And/Or/Xor` and `UNARY:Not` kernels,
  the streaming path, the optimizer's folder and `fn_not`. `SimdOps::logical_and`
  / `logical_or` / `logical_xor` / `logical_not` compared against zero
  (`!= 0.0`), and those are what the *array* legs call, so the same expression
  shape answered two ways depending on whether an operand happened to be a
  scalar: `CLOSE AND 0` was `0.0` while `CLOSE AND (0 - CLOSE)` was `1.0`. All
  sites now call `truth::is_logical_true` / `truth::logical_bool`, and
  `SimdOps::select` now calls `truth::is_true` instead of restating it.

- `==` / `!=` compared with a tolerance everywhere except the array-array leg.
  Every kernel uses `|lhs - rhs| < 1e-10`, but `SimdOps::simd_eq_arrays` /
  `simd_neq_arrays` compared exactly (`_CMP_EQ_OQ` in the AVX2 kernel), so
  `CLOSE == (CLOSE + 1e-12)` answered `0.0` while the scalar leg answered
  `1.0`. The JIT had the same split *within one path*, choosing the exact SIMD
  kernel from 16 elements up and the tolerant scalar loop below — the
  length-dependent anomaly that `truth.rs` exists to prevent. The AVX2, AVX-512
  and NEON kernels now compute `|a - b| < eps` and `|a - b| >= eps` (ordered, so
  a `NaN` operand makes both comparisons false, exactly as before), and every
  site calls `math::nearly_equal` / `nearly_not_equal`.

- The second-moment family used the one-pass form `sum(x^2) - sum(x)^2 / n`,
  which subtracts two large nearly equal totals and loses every significant
  digit when a series' mean dwarfs its spread. A series and its exact affine
  image `2x + 3` are perfectly correlated, yet `correlation` — exposed to Python
  as `correlation` — returned `0.667` at a `1e9` baseline, `covariance` returned
  `520.1` where the exact answer is `693.3`, `batch_zscore` returned `-1.607`
  where the offset-invariant answer is `-1.705`, and `rolling_std_simd` returned
  `0.0` for every window of `1e12 + i` with `window = 3` where the true sample
  deviation is exactly `1.0`. Every spelling now goes through the new
  `math::centred_moments`, which subtracts the mean first so each addend stays
  at the scale of the spread. `math::statistics::{correlation, covariance}`,
  `features::{correlation_simd, batch_zscore_simd, rolling_mean_simd,
  rolling_std_simd}`, `features::combinations::rolling_correlation`,
  `features::rolling_stats::{rolling_zscore, pearson_ic}` are routed through it.
  The rolling spellings delegate to the canonical `math::kernels` entries
  instead of carrying their own sliding-window recurrences.

- `statistics::rolling_std_dev` took the square root of an *unclamped* variance,
  so a window whose true variance is zero but whose removable-Welford residue
  came out slightly negative produced `NaN` — while the canonical kernel
  `rolling_sample_stddev_into` and `features::rolling_std_simd` clamped and
  returned `0.0`. Two spellings of "rolling sample standard deviation" must not
  answer differently on the same window: `rolling_std_dev` now delegates to
  `rolling_sample_stddev_into`, and `rolling_sample_variance_into` clamps its
  output at the source so the variance is never negative and the deviation is
  exactly its square root.

- `features::simd_opt` carried doc claims its bodies did not implement:
  `rolling_std_simd` claimed to "match `statistics::rolling_std_dev`" (it did
  not — see above), `batch_minmax_simd` claimed to use
  `SimdOps::min_elementwise` / `max_elementwise` "for the reduction" (those are
  *element-wise* kernels and cannot reduce; the body runs a scalar loop), and
  `sum_and_sum_sq_simd` claimed an "AVX2-accelerated mul + horizontal reduce"
  (the reduce is scalar `iter().sum()`). The claims now match the bodies.

- The streaming registry and each indicator's own `IndicatorMeta` are two
  independent sources for the same metadata, and nothing tied them together, so
  they had drifted. Eight types answered `IndicatorMeta::category()` with the
  slug `"statistic"`, which is not a member of `VALID_CATEGORIES` — the declared
  vocabulary — so a consumer grouping by that value would invent a category the
  contract does not define; five of them (`BETA`, `CORREL`, `STDDEV`, `TSF`,
  `VAR`) additionally contradicted their own registry entry, which says
  `"statistics"`. `StreamingSuperTrend` reported `"volatility"` while the
  registry entry it is published under says `"overlap"`. The slugs are aligned,
  `"smc"` (the category of the two smart-money-concepts detectors) is added to
  `VALID_CATEGORIES`, `impl_indicator_meta!` now `debug_assert!`s that its slug
  is declared, and a new gate
  `scripts/check_streaming_registry_contract.py` (wired into the Docs Check
  workflow) fails on any undeclared slug or any category that disagrees with the
  registry entry.

- Three contract tests had gone red in crates the release gate never ran.
  `cargo test -p finkit` was the only test command used to certify the workspace
  locally, and it does not build `finkit-ffi-common` or `finkit-ffi`, even
  though `.github/workflows/ci.yml` does. Two regressions had accumulated behind
  that blind spot:
  1. `factor_catalog::tests::catalog_is_stable_and_describes_dependencies`
     asserted a hard-coded factor count of `9`. The built-in library grew to the
     demo factors plus Alpha158 plus WorldQuant101, so the FFI catalog
     legitimately publishes 184 and the assertion could only ever rot. It now
     derives the expectation from `builtin_factor_registry()`, which is the same
     registry the catalog projects.
  2. `profile_only_entries_publish_executable_parameter_contracts` (ffi-common)
     and `operation_catalog_json_exposes_talib_parameter_contract` (c-binding)
     both asserted that the *flat* `params` field of `STOCH` carries the TA-Lib
     five-parameter contract. That was true while `STOCH` was a profile-only
     name; it is now a registry-backed formula bridge, and the flat field
     describes the default `core_registry` profile instead — publishing the
     TA-Lib names there would hand `core_registry` callers parameters the core
     kernel silently ignores (`STOCH` is positional, defaults to `fastk = 14`,
     and has no `matype` arguments). Both tests now assert the executable TA-Lib
     contract where it actually lives, in
      `profile_output_contracts["talib_0_8_0"].params`, and a new loop in the
      ffi-common test pins every declared TA-Lib parameter spec to that contract.

- The streaming discovery API was documented at a path that does not resolve.
  `docs/api-reference.md` told users to
  `use finkit::streaming::{all_indicators, by_id, by_category, registry_document, VALID_CATEGORIES}`,
  but `streaming/mod.rs` re-exported only `all_indicators`, leaving its natural
  companions reachable only through `streaming::registry::`. The whole family is
  now re-exported together, so every path the interface reference advertises
  resolves.
- `by_id` and `by_category` had no caller and no test anywhere in the workspace.
  They are an index-cache implementation of the same lookup `all_indicators()`
  provides by linear scan, so nothing proved the two agreed. A new test asserts
  they return pointer-identical entries from the shared cached slice, that the
  per-category buckets partition the registry exactly (a category typo can no
  longer silently drop an indicator from discovery), and that unknown keys fail
  closed.
- The interface reference had drifted from the code it documents.
  `all_indicators()` was written as returning `Vec<IndicatorInfo>` (it returns
  `&'static [IndicatorInfo]`), `RegistryDocument::indicators` likewise,
  `by_id` / `by_category` / `VALID_CATEGORIES` were missing from the listing
  entirely, `IndicatorInfo` was missing its `streaming` field, and the "valid
  category slugs" line listed 5 of the 15 slugs. All corrected, with a full
  category table and prose for the `convergence` / `streaming` semantics.
- The Chinese API reference documented no registry-discovery surface at all —
  the whole `Indicator Registry API` section existed only in English. It now has
  a matching section, and its table of contents lists the sections in document
  order (`跨市场信号公式` had been listed last while sitting second).
- The Windows MSI script staged the wrong native library names. It copied
  `AlphaTA_ffi.dll` / `.dll.lib` / `.lib`, but the crate is `finkit-ffi` with no
  explicit `[lib]` name, so the artifacts are `finkit_ffi.*`; `copy` does not
  fail a batch script, so the run would have produced an MSI whose `bin\` was
  empty. It also never created the `bin\` directory it copied into, used a
  component-group name that disagreed with `build-installer.sh`, and wrote a
  third spelling of the output filename. All corrected, and the script now
  preflights its inputs.
- No OS-level installer target has ever been buildable from this repository.
  `scripts/build-installer.sh` and `scripts/build-installer-msi.cmd` both hand a
  staged payload to WiX, but the `packaging/wix/Product.wxs` they read has never
  existed in any commit (checked across all branches), and no workflow installs
  the WiX toolset. `scripts/build-usage-packages.sh` is in the same position:
  its `packaging/usage/<lang>/` verification tree is likewise absent, as is the
  `packaging/build-installer.sh` its own RPM changelog refers to. Both installer
  scripts now fail up front with a message naming the missing file, instead of
  dying inside `candle`/`heat` with a tool error. The working release artifact is
  the Python ABI3 wheel, which is what `python-wheels.yml` actually ships.
- `docs/installation.md` §14 told readers to certify a change with
  `cargo test -p finkit --locked` — the same command that left the three
  contract tests in `finkit-ffi-common` / `finkit-ffi` unrun (see above) — and
  listed three of the ten gate scripts. It now gives the full CI-aligned
  per-crate test command, warns about `--no-fail-fast`, and separates the
  docs-check gates from the ci.yml gates. `scripts/check_coverage.py` was
  dropped from that list: it is a coverage *report*, not a CI gate, and is not
  wired into any workflow.
- `MONEY_FLOW` was advertised as streaming-capable in
  `streaming::registry` with no implementation behind it. Added
  `StreamingMoneyFlow` (`streaming::volume::money_flow`), which mirrors
  `indicators::astock::money_flow` bar-for-bar, including its NaN behaviour, and
  is re-exported from `finkit::streaming`.
- The streaming-registry contract gate silently skipped every `IndicatorMeta`
  impl written as `impl crate::streaming::IndicatorMeta for T` (three types:
  `StreamingDema`, `StreamingObv`, `StreamingWclPrice`), so a qualified-path
  impl could drift without ever being checked. The extractor now accepts
  qualified paths.
- The same gate's self-check was tautological: `count_declared_impls()` reused
  the extractor's own regexes, so a form the extractor could not read was also
  uncountable and the two counts agreed on the wrong number. The self-check now
  uses an independent, deliberately looser census.
- `streaming::registry` listed Elder Ray twice — `ELDERRAY` with a 14-period
  default and `Elder Ray` with the correct 13 — and `test_unique_names` compared
  raw strings, so both passed. Consolidated onto the canonical `ELDERRAY` entry
  with the 13-period default (`StreamingElderRay::new(13)`).
- `docs/generated/streaming-indicators.md` was stale after the registry
  corrections; regenerated via `gen_ssot_docs.py --generate`.
- `scripts/optimize_python_bindings.py` rewrote **integer** results to NumPy
  arrays, contradicting `sync_bindings.transform_python_numpy_body`, whose
  documented rule is that integer candlestick results keep the `Vec<T>` ->
  Python list boundary. The mismatch made `sync_bindings.py --generate` a
  non-idempotent operation (it would have introduced 13 wrappers the committed
  bindings never contained) and left `optimize_python_bindings.py --check`
  permanently red. The optimizer now targets floating-point series only, which
  restores `--generate` as a fixed point and turns the check green.
- The same rewriter wrote with `Path.write_text`, which translates every newline
  to `os.linesep`; run on Windows it silently rewrote a whole binding to CRLF.
  It now preserves the file's existing newline convention.
- The 13 hand-written `Vec<f64>` pyfunctions in `ffi/python-binding/src/lib.rs`
  (`dx`, `minus_di`, `minus_dm`, `plus_di`, `plus_dm`, `var`, `ichimoku`,
  `vwap`, `anchored_vwap`, `vwap_bands`, `elder_ray`, `donchian`,
  `pivot_points`) still returned Python lists, while the 39 generated numeric
  functions in the same module returned NumPy arrays and
  `finkit/__init__.pyi` declares `dx`/`var`/`plus_di`/… as `Array1D`. They now
  use the same NumPy-direct wrapper shape, so the native surface is uniform and
  `optimize_python_bindings.py --check` covers `lib.rs` as well as
  `generated.rs`.
- The NumPy-direct binding check never actually ran on a live branch: the only
  workflow that invoked it was `apply-talib-performance-plan.yml`, whose `on:`
  covers `fix/talib-performance-plan-20260904`. Because the check was dead, two
  binding modules kept their list-returning wrappers — `features.rs` (40
  numeric pyfunctions) and `transforms.rs` (5) — so `finkit.features.acf(...)`
  and friends returned Python lists while every top-level numeric function
  returned a NumPy array. Both modules are now NumPy-direct, and the check runs
  in `ci.yml` over **every** binding source, not two hand-listed files.
- The shipped type stub `ffi/python-binding/finkit/__init__.pyi` described an
  API that does not exist. It declared a `StreamingIndicator` base class with a
  `next()` method (the real classes have no base class and the method is
  `update`), declared `cdlhomingsoldier` (the real name is `cdlhomingpigeon`),
  omitted `StreamingMACD` entirely, and froze an `__all__` of 170 names while
  the runtime exported 449. `__all__` is now declared as an annotation so it
  cannot drift again, the streaming section documents the six documented
  classes with their real signatures (`restore_state` is a static method), and
  `scripts/check_python_stub.py` fails the build if the stub declares a name
  the extension does not have.
- Nine streaming classes were registered under their internal Rust struct name,
  so Python users had to reach `finkit.PyStreamingAnchoredVwap`,
  `finkit.PyStreamingPlusDi`, `finkit.PyStreamingStochRsi` and six others while
  the other seventy followed the `Streaming*` convention. They now register as
  `StreamingAnchoredVwap`, `StreamingPlusDi`, `StreamingStochRsi`, … .
- The streaming section of `docs/api-reference-zh.md` documented an API that
  cannot run: `StreamingMACD(fast=…, slow=…)` (the parameters are
  `fast_period`/`slow_period`/`signal_period`), `save()` and
  `from_state()` (the real methods are `save_state()` and the static
  `restore_state(bytes)`), and `macd, signal, hist = update(price)` where
  `update` returns a non-iterable `MACDResult`. `docs/api-reference.md` had no
  Python streaming section at all; both documents now describe the common
  interface and point at the generated catalog.
- Five tests in `ffi/python-binding/tests/` failed against a real wheel, and the
  live `python-wheels.yml` workflow runs them — so the Python release gate was
  red. The compatibility layer that would have fixed them exists only in
  `scripts/apply_perf_plan.py`, whose workflow triggers on
  `fix/perf-contract-talib-parity-20260904`. The layer is now applied for real:
  * `available_factor_libraries()` returned `array(['alpha158',
    'worldquant101'])`. The blanket `_as_numpy_result` adapter used "this list
    contains no containers" as its numeric predicate, which also matches a list
    of *strings*, so `== [...]` raised "truth value of an array is ambiguous".
    It now converts only lists of real numbers (`bool` excluded).
  * `bollinger_bands(matype=1)` and `stoch(slowk_matype=1)` raised a bare
    `ValueError`, so `except finkit.InvalidParameterError` did not catch them.
  * `sar` discarded the acceleration-factor series the engine computes with it,
    leaving no way to reach it from Python; `sar_with_af` now exposes it.
  * `CompiledFormula` had no explicit name for the owned/context-retaining
    evaluation mode; `eval_owned` is now an alias for `eval`.
- `InsufficientDataError` and `InvalidParameterError` inherited only from
  `FinkitError`, and `IndicatorNotFoundError` from `FinkitError` alone, so a
  caller writing `except ValueError` or `except KeyError` missed them. They now
  also inherit from the matching built-in, which is what the documented
  semantic names were always meant to specialise.
- The C-binding export gate was checking roughly one third of the ABI. It
  compared the committed headers against `ffi/c-binding/src/generated.rs` only,
  so the 18 fixed-template entry points in `lib.rs` (`ta_version`,
  `ta_last_error`, `ta_factor_execute_json`, `finkit_free_string`, ...) and the
  3 research functions in `research.rs` were never verified — 21 of the 99
  shipped exports could have disappeared from the Rust side while the header
  kept promising them. `scripts/gen_c_header.py --check` now scans every
  `ffi/c-binding/src/*.rs` and every `ffi/c-binding/include/*.h` and compares in
  both directions, correctly ignoring `#[cfg(test)]`-gated exports such as
  `ta_ffi_panic_test` (deliberately absent from a release build). Cross-checked
  against the shipped DLL's PE export table: 95 `ta_*` plus 4 `finkit_*`
  exports, matching the headers one-for-one.
- That gate had never run in CI either — it was reachable only through
  `make verify-ffi`, which no workflow invokes. It is now a step in `ci.yml`'s
  `binding-ssot` job, alongside the Python binding contracts.
- `scripts/check_python_stub.py` ran its dynamic half whenever *any* `finkit`
  was importable, so on a machine with an older wheel in `site-packages` it
  failed against a build the developer never touched — and, worse, it could
  report success after validating the wrong build. The dynamic check is now
  opt-in (`--require-extension`, or `--expect-prefix` to name the build), and
  `python-wheels.yml` pins the freshly installed wheel with
  `--expect-prefix "$pythonLocation"`.
- Documentation counts that had drifted by one: `docs/indicator_registry.json`
  holds 235 indicators, not 236, so 157 of them have no FFI binding rather than
  158. Corrected in `docs/language-bindings.md` and in the matching comments in
  `scripts/check_coverage.py` and `scripts/sync_bindings.py`.
- The generated API reference had 137 rustdoc diagnostics, all of them silent.
  `cargo doc` exits 0 because rustdoc lints are warn-by-default, and the CI
  `doc` job set `RUSTFLAGS` — but rustdoc reads `RUSTDOCFLAGS`, so nothing was
  denied. The bulk were broken intra-doc links: 62 were bare array indexing in
  prose (`a[i] / b[i]`) that rustdoc read as links to an item named `i`, and
  the module indexes in `math`/`patterns`/`features` used bare sibling names,
  which do not resolve because rustdoc resolves a module's own `//!` links
  against the parent scope. Fixed by escaping the indexing, adding crate-rooted
  paths, and unlinking the targets that are private (`HilbertState`,
  `TaVarianceState`, `ema_into`) or cross-crate (`FfiError`). Two of the
  references were simply stale: `crate::factor_graph::GraphPlan` no longer
  exists (`node_index` lives on `FactorGraphPlan`), and `math::simd_ops`
  documented an AVX2 kernel name that is not in the tree. Eight
  `invalid_html_tags` (`Arc<str>`, `Array1<i32>`) are now code spans. The build
  is clean under `RUSTDOCFLAGS="-D warnings"`.
- `.github/workflows/release-v014-orchestrator.yml` could never run and would
  have failed if it had: its only trigger was a push to `release/v0.1.4`, a
  branch that no longer exists, and its body asserted `VERSION = 0.1.4`, a
  hard-coded commit SHA and a hard-coded workflow run id while the workspace is
  at 0.2.0. Removed rather than left as a release gate that silently never fires.
- Fifteen dead files under `scripts/`, found by the new orphan gate and each
  removed only after establishing that it was superseded, broken, or falsely
  documented: `check-versions.sh`/`.ps1` (the live gate is
  `scripts/check_versions.py`, which the release checklist already names),
  `with-path.sh`, `audit_talib_numeric_contract.mjs`,
  `verify_python_binding.py`, `benchmark_full_coverage.py`,
  `full_accuracy_diagnose.py`, `bench_alpha_vs_talib_python.py`,
  `bench_gate.sh`, `bench_regression_check.sh`, `gen_compat_matrix.py`,
  `gen_competitor_comparison.py`, `gen_performance_report.py`, and
  `generated_samples/{node,python}_indicators.rs`.

## [0.1.15] - 2026-09-11

### Added

- Forming-bar repaint rollback for streaming MACDEXT.
- Scalar WMA, DEMA, TEMA, KAMA, T3, TRIMA, HMA, ALMA, and VIDYA variants for
  streaming MACDEXT fast, slow, and signal lines.
- Configurable streaming MACDEXT exposed through the Python binding.

### Changed

- Close-stream parsing is now injectable, which makes CLI tests deterministic.

## [0.1.14] - 2026-09-11

- Add registered, parameterized custom formula components with nested
  expansion, built-in shadowing protection and bounded expansion limits.
- Expose the same custom formula registry through the Python binding.
- Add explicit custom-edge PSI calculation with finite-value filtering and
  stable outlier handling.
- Add a shared crosshair data-window snapshot and visible-range-aware cursor
  mapping for native/WASM chart frontends.

## [0.1.13] - 2026-09-11

- Correct the Rust CDLDOJI public wrapper to use TA-Lib's default 0.1
  BodyDoji factor, matching the Python binding and compatibility contract.

## [0.1.12] - 2026-09-11

- Complete the maintained TA-Lib 0.6.x compatibility matrix at 161/161 on
  multiple long-input lengths and seeds, including SAR/SAREXT bootstrap
  semantics, T3 warm-up/coefficient behavior and extended candlestick rules.
- Add shared CandleSettings-based thresholds for candlestick compatibility,
  while preserving native full-length and short-input API behavior.
- Correct streaming T3 coefficients and align cycle/trendline compatibility
  paths with the batch implementation.

## [0.1.11] - 2026-09-11

- Add exact TA-Lib compatibility implementations for STOCHRSI, MACDFIX and
  BETA, including their multi-output, lookback and secondary-input contracts.
- Correct the shared Hilbert recursive warm-up and radian/degree period
  conversion; HT_DCPERIOD, HT_DCPHASE, HT_PHASOR, HT_SINE, MAMA and the
  trend-mode path now match TA-Lib on the maintained comparison matrix.
- Add the TA-Lib dominant-cycle phase projection and raw-price trendline
  calculation, plus regression coverage for the extended compatibility paths.
- Align CDLDOJI with TA-Lib's preceding real-body average candle setting.

## [0.1.10] - 2026-09-11

- Correct TA-Lib MACDEXT lookback selection for its `slowperiod` and
  `signalperiod` parameter positions in the compatibility adapter.

## [0.1.9] - 2026-09-11

- Add opt-in Python `compute_indicators(..., talib_compat=True)` semantics for
  TA-Lib lookback/NaN conventions, absolute MAX/MININDEX results and compatible
  multi-output ordering without changing native formula behavior.
- Align ADXR, AROONOSC, SAR, PLUS_DM/MINUS_DM and PPO compatibility paths with
  TA-Lib's window, smoothing, output and moving-average-type contracts.
- Correct the full Python comparison harness for TA-Lib 0.6.x multi-output
  finite-ratio accounting and explicit parameter signatures; the maintained
  smoke matrix now calls all 161 public functions and reports 124 exact rows.
- Add Python regression coverage for native-vs-TA-Lib compatibility semantics.

## [0.1.8] - 2026-09-11

- Add a shared formula result metadata contract for dtype, output names,
  NaN/null policy, lookback, warm-up and valid-start semantics across Rust,
  Python, Node and WASM.
- Add a complete 161-function public TA-Lib catalog with explicit runtime
  coverage, category, output-count and compatibility-report fields; unsupported
  and host-required functions are no longer ambiguous.
- Add O(1) append/eval-last formula paths for direct MA, RSI and formula-SMA
  ATR, with exact fallback on mutation or discontinuity.
- Add binding APIs for formula metadata and TA-Lib catalog discovery.
- Preserve separate Wilder/RMA streaming ATR semantics from the formula-layer
  rolling-SMA true-range contract, with differential regression coverage.

## [0.1.7] - 2026-09-11

- Add formula static analysis for dependencies, lookback, future-data risks,
  side effects, stateful nodes and streaming suitability.
- Add terminal semantic profiles and function-level compatibility reports for
  Finkit, TongDaXin, TongHuaShun, EastMoney and Pine.
- Add borrowed range evaluation and Python `eval_range_zero_copy` for chart
  window refreshes without copying complete OHLCV history.
- Add conservative O(1) EMA updates for continuous append/eval-last streams,
  with exact fallback when continuity cannot be proven.
- Expose formula analysis and compatibility APIs through Python, Node and WASM.

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.5] - 2026-09-09

### Added

- Cross-market screening indicators for crossover, breakout, volume expansion,
  moving-average alignment, relative strength, gaps, and trend confirmation.
- Formula DSL routes and aliases for the new screening indicators.
- Versioned multi-language release workflow documentation and package build
  contracts.

### Performance and validation

- Expanded the TA-Lib release-wheel gate to 155 compatible functions and 310
  observations at 100K/1M values.
- v0.1.5 local validation: 307/310 observations faster, geometric mean 2.03x,
  zero runtime errors; the two existing `HT_TRENDMODE` mask exceptions remain
  explicitly documented.

### Documentation

- Added the cross-market screening API and selection recipes.
- Updated generated indicator/formula/version catalogs and the multi-language
  installation/release guides.
- Replaced stale six-indicator benchmark claims with the current full-matrix
  report and per-observation caveats.

## [0.1.2] - Pending release (source baseline 2026-09-02)

- Added cross-language version checks for .NET and Java binding metadata, with SSOT version-matrix coverage.
- Connected the remaining Android, Go, and .NET generated indicator/chart entrypoints and hardened pre-epoch formula timestamp conversion with regression coverage.

> The `0.1.2` source metadata is ready on the repair branch in PR #13; the GitHub Release and downloadable assets are still pending until CI verification and merge.

### Changed
- 将公式运行时升级为可复用的零拷贝/增量执行模型：公共子表达式合并、`eval_range`/`eval_last`、容量增长式 `append_bar`、持久化 Bytecode/JIT 与缓冲池。
- Python ABI3 wheel 发布矩阵与版本元数据统一为 `0.1.2`，覆盖 CPython 3.8–3.14 的 Linux x86_64、macOS x86_64/arm64 和 Windows x86_64。

### Fixed
- 加固 GitHub Actions 的权限与并发控制，移除旧版 `v0.1.0` 强制回写逻辑，统一使用版本 tag 发布 wheel。

### Added
- Comprehensive CI/CD pipeline with fmt, clippy, and security audits
- Multi-language binding tests (Python, Node.js, Go, .NET, Java, WASM, CLI)
- Complete documentation suite (indicators, installation, API reference, development)
- Cross-platform compilation support (Linux, macOS, Windows)
- Dependency review for pull requests
- **`core/tests/golden_regression.rs`**：30 个黄金 CSV 回归断言（容差 1e-9），覆盖 SMA/EMA/RSI/MACD/ATR/STOCH/ADX/BBANDS/MOM/ROC 等核心指标
- **`core/tests/property_smoke.rs`**：3 个 proptest 属性测试 — SMA 线性、Bollinger 上下界包络、RSI 范围 [0,100]
- **`core/benches/zero_alloc_bench.rs`**：SIMD 路径 `n=10_000` 零分配验证（`fma_avx2` vs `scalar_fma`）
- **`streaming::registry` 额外缓存**：`by_category()` / `by_id()` 改 `OnceLock<HashMap<...>>` 缓存，与 `all_indicators` 保持一致
- **`core/benches/watchlist_self_bench.rs`**：9 个 watchlist 指标（AROON/WILLR/WMA/KAMA/MFI/STOCHF/AD/ADOSC/OBV）的自基准测试，3 档数据规模（1K/10K/100K）
- **`watchlist_self_bench` 注册到 `Cargo.toml`**：纳入 cargo bench 体系

### Changed
- **Crate-wide `#[allow(missing_docs)]`**：在 `core` / `visualization` / `cli` / 5 个 FFI binding / `wasm` 的 `lib.rs` 顶部集中抑制 internal helper 的 missing_docs warning（净减 1700+ pedantic warnings，cargo check 0 warnings）
- **`StreamingPpo` / `StreamingTsi` / `StreamingApo` / `StreamingCoppock` / `StreamingNatr`**：移出 `wasm_streaming_f64!` 宏，补独立 wrapper（PPO/TSI/APO 2 参构造、Coppock 3 参构造、NATR `&dyn Ohlcv` 入参）
- **`StreamingBoll` / `StreamingStoch` 构造签名**：wasm 入口补第 3 参数（`nb_dev_dn` / `k_slow`），与 core 对齐
- **`aroon` 阈值分流**：`period ≤ 16` 走 `aroon_scan`（线性扫描，cache 友好），`period > 16` 走 `aroon_with_deques`（单调双端队列，O(1) amortized）。消除原 O(period) rescan 慢路径
- **`willr` 单调双端队列**：完全重写为 max-deque + min-deque，热路径 O(1) amortized，删除 rescan 内层循环
- **`stochf` 单调双端队列**：max-deque 替代 rescan；保留 ring-buffered %D 累加器，输出流水线不变
- **`docs/BENCHMARK_REPORT.md` watchlist 章节**：补充 9 指标吞吐量表 + 算法形态分类 + 下一步 SIMD 优化建议

### Fixed
- **dotnet-binding & java-binding 内存管理 README 文档契约** 已在 `ffi/dotnet-binding/README.md` 与 `ffi/java-binding/README.md` 完善
- **`simd_kernels.rs` 两处死赋值**：`highest = *high_ptr.add(ws);` 与 `lowest = *low_ptr.add(ws);` 立即被覆盖，删除
- **`pca.rs::cov_idx` 死方法**：未引用，删除
- **dotnet-binding & java-binding CString UB**：为两者增加 `ta_free_cstring` / `freeJString` 配对释放函数；引入 `serde_json` 替代 36+ 处手写 JSON 序列化（DrawCommand / DebugEvent / FormulaTemplate 等核心类型现在 derive `serde::Serialize`）
- **FormulaCache 真 LRU**：从 O(n) 最小 counter 扫描改为 `lru` crate 的 O(1) LruCache，公开 API 完全兼容
- **JIT 路径 `load_variable` 零分配**：从 `name.to_uppercase().as_str()` 链式匹配改为 `bytes.eq_ignore_ascii_case` 字节级零分配匹配
- **SIMD feature detection 缓存**：用 `std::sync::OnceLock<SimdLevel>` 缓存 CPUID 结果，18 个 public SIMD 入口改为 `match simd_level()`，100 万次 add 仅触发 1 次 CPUID
- **BufferPool 双池清理**：删除未使用的 legacy `VecDeque<Vec<f64>>` 字段与 8 个 API（`acquire/release/shrink_to/...`），构造 BufferPool 不再预分配 64KB `Vec<f64>`
- **`resolve_variable_zero_copy` 6 段重复代码抽取**：Close/High/Low/Open/Volume/Amount 6 段近似重复逻辑抽取为单一 `copy_view_to_pool` helper
- **Parser 别名表零分配**：`parse_variable` 中 16 个 C1/O1/CLOSE1 等别名从 `to_uppercase().as_str()` 改为 `bytes.eq_ignore_ascii_case` 字节级零分配匹配
- **streaming `all_indicators` OnceLock 缓存**：避免重复调用时重复构造
- **FormulaCache 单次查找**：`get_cloned` 与 `insert` 中删除 `contains_key + get/get_mut` 双查找
- **workspace 构建配置精细化**：`[profile.release]` 改为 `lto="thin", strip="debuginfo"`；`[profile.release.package."alpha-ta-..."] codegen-units = 16`；`[profile.dev] opt-level = 1, debug = 1`；新增 workspace lints（`unsafe_op_in_unsafe_fn = "warn"`, `missing_debug_implementations = "warn"`, `clippy::pedantic`）
- **Go 绑定版本硬编码 bug**：`ta_version()` 从硬编码 `0.1.0` 改为 `env!("CARGO_PKG_VERSION")`
- **Java 绑定 panic 保护**：为公式评估函数添加 `catch_unwind` 包裹和 `RuntimeException` 错误传播

## [1.0.0] - 2026-06-24

### Added
- Hilbert Transform cycle indicators (HT_DCPERIOD, HT_DCPHASE, HT_PHASOR, HT_SINE, HT_TRENDMODE)
- Statistics indicators (STDDEV, VAR, LINEARREG, ZSCORE, CORREL)
- Price transform functions (AVGPRICE, MEDPRICE, TYPPRICE, WCLPRICE)
- Volume indicators (AD, ADOSC, CMF)
- Pattern recognition for 60+ candlestick patterns
- Chart pattern detection (Double Top/Bottom, Head & Shoulders)
- Go binding with CGO support
- .NET binding with P/Invoke
- Java binding with JNI support
- CLI tool for command-line analysis
- WebAssembly module for browser usage
- Visualization module
- Formula engine with AST parsing, bytecode compilation, JIT optimization, and SIMD acceleration
- Streaming (incremental) indicator framework with 150+ indicators
- Feature engineering module for ML label generation

### Changed
- Improved performance of SMA and EMA calculations
- Enhanced error handling with detailed error messages
- Updated Python binding to use latest PyO3 features
- Optimized memory allocation in core library

### Fixed
- Fixed RSI calculation for edge cases with constant values
- Fixed MACD signal line initialization
- Fixed pattern detection boundary conditions

## [0.3.0] - 2024-XX-XX

### Added
- Python binding with PyO3
- Node.js binding with NAPI-RS
- Support for all major overlap indicators (SMA, EMA, WMA, DEMA, TEMA, KAMA, BBANDS, SAR)
- Momentum indicators (RSI, MACD, STOCH, ADX, AROON, CCI, WILLR)
- Volatility indicators (ATR, NATR, TRANGE)
- Basic candlestick pattern recognition

### Changed
- Refactored moving average calculations for better performance
- Improved test coverage for all indicators

### Fixed
- Fixed EMA calculation precision issues
- Fixed Bollinger Bands upper/lower band calculation

## [0.2.0] - 2024-XX-XX

### Added
- Initial release of FTA core library
- Basic technical analysis framework
- Moving average implementations (SMA, EMA)
- Core mathematical functions
- Rust API design
- Multi-platform support (Linux, macOS, Windows)
- CI/CD pipeline setup
- Comprehensive test suite

