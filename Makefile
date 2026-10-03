# =============================================================================
# Finkit — one-click build for multi-language usage packages + TA-Lib compare
# =============================================================================
#
# Quick start:
#   make help              # show all targets
#   make                   # build + verify all discovered language packages
#   make python            # build + verify Python package only
#   make bench-vs-talib    # run Finkit vs TA-Lib C head-to-head
#   make docker-build      # build the one-click Docker image
#   make docker-run        # run the build inside the image, mount ./dist
#   make install-and-test  # install built artifacts locally + run smoke tests
#   make clean             # wipe dist/
#
# This Makefile is a thin wrapper — the actual logic lives in:
#   * build-usage.sh / build-usage.ps1   (root entry points)
#   * scripts/build-usage-packages.{sh,ps1}  (unified builder+verifier)
#   * scripts/bench-vs-talib.{sh,ps1}    (TA-Lib head-to-head)
#   * scripts/install-and-test.{sh,ps1}  (install + smoke)
# =============================================================================

SHELL := /usr/bin/env bash
ROOT  := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))

# On Windows, prefer pwsh over bash. GNU make + Git-Bash can still call
# build-usage.sh directly, so this only matters for `make help` text.
ifeq ($(OS),Windows_NT)
  ENTRY   := $(ROOT)/build-usage.ps1
  PREFLIGHT := powershell -NoProfile -ExecutionPolicy Bypass -File $(ROOT)/scripts/lib/preflight.ps1
else
  ENTRY   := $(ROOT)/build-usage.sh
  PREFLIGHT := bash $(ROOT)/scripts/lib/preflight.sh
endif

# ---- discover buildable languages from scripts/ ----------------------------
# The glob also matches `build-usage-packages.sh`, whose suffix looks exactly
# like a language name. Without the filter below, `make packages` is advertised
# as a language target, forwards `packages` to the entry script, and fails with
# "unknown language: packages" every single time.
LANGS := $(notdir $(wildcard $(ROOT)/scripts/build-usage-*.sh $(ROOT)/scripts/build-usage-*.ps1))
LANGS := $(LANGS:build-usage-%.sh=%)
LANGS := $(LANGS:build-usage-%.ps1=%)
LANGS := $(sort $(LANGS))
LANGS := $(filter-out packages,$(LANGS))

# ---- phony targets ---------------------------------------------------------
.PHONY: all help clean dist
.PHONY: $(LANGS)
.PHONY: bench-vs-talib bench-talib
.PHONY: install-and-test
.PHONY: docker-build docker-run docker-bench docker-compose-up
.PHONY: preflight lint
.PHONY: gen-c-header verify-ffi gen-c-binding verify-bindings verify-bindings-tier verify-all-bindings
.PHONY: build-native-archive verify-native-archive
.PHONY: installer installer-all check-installer
.PHONY: $(INSTALLER_TARGETS:%=installer-%)
.PHONY: check-rustdoc check-orphans check-script-refs check-dead-code check-nan-safety check-ios-header
.PHONY: check-ios-header check-talib-ffi refresh-release-manifests check-release-manifests

# ---- default ----------------------------------------------------------------
all: preflight
	$(ENTRY)

# ---- per-language shortcuts -----------------------------------------------
# Each per-language target forwards to the unified entry so behavior is
# identical to `make all` minus the other languages.
$(LANGS):
	$(ENTRY) $(@)

# ---- benchmarks ------------------------------------------------------------
bench-vs-talib: bench-talib
bench-talib:
	bash $(ROOT)/scripts/bench-vs-talib.sh

# ---- install + smoke -------------------------------------------------------
install-and-test:
	bash $(ROOT)/scripts/install-and-test.sh

# ---- docker ----------------------------------------------------------------
docker-build:
	docker build -t finkit/builder:latest $(ROOT)
docker-run:
	docker run --rm -v $(ROOT)/dist:/work/dist finkit/builder:latest --no-bundle
	docker run --rm -v $(ROOT)/dist:/work/dist finkit/builder:latest --bench-talib

docker-compose-up:
	docker compose -f $(ROOT)/docker-compose.yml up --abort-on-container-exit

# Previously advertised in `make help` and declared .PHONY, but never given a
# recipe -- `make docker-bench` failed with "No rule to make target". The image
# mounts ./dist so results survive the container.
docker-bench:
	docker build -t finkit/builder:latest $(ROOT)
	docker run --rm -v $(ROOT)/dist:/work/dist finkit/builder:latest --bench-talib

# ---- preflight ------------------------------------------------------------
preflight:
	@echo "[make] preflight toolchain check"
	@$(PREFLIGHT)

# ---- lint: convenient local Rust formatting/clippy pre-check --------------
# First-time setup: `rustup component add clippy`. The permanent CI workflow
# remains the source of truth for the complete locked/all-feature gate matrix.
lint:
	cargo fmt --all -- --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo clippy -p finkit --no-default-features --features no_std -- -D warnings

# ---- housekeeping ---------------------------------------------------------
dist:
	mkdir -p $(ROOT)/dist

clean:
	rm -rf $(ROOT)/dist
	@echo "[make] removed $(ROOT)/dist"

# ---- codegen: regenerate the C FFI header from the indicator registry -----
# `verify-ffi` fails CI if the committed header drifts from the registry
# (docs/indicator_registry.json), keeping the single source of truth honest.
gen-c-header:
	python3 $(ROOT)/scripts/gen_c_header.py --generate $(ROOT)/ffi/c-binding/include/finkit.h

verify-ffi:
	python3 $(ROOT)/scripts/gen_c_header.py --check $(ROOT)/ffi/c-binding/include/finkit.h

# ---- codegen: C Rust wrappers ---------------------------------------------
# `ffi/c-binding/src/{lib.rs -> include! generated.rs}` was emitted by
# scripts/gen_binding.py from an `ffi` block in docs/indicator_registry.json.
# That metadata moved to docs/ffi_registry.json, and the emitter now refuses to
# run rather than write an empty binding, so there is no in-tree generator for
# the C wrappers any more. `verify-ffi` (gen_c_header.py) is what keeps the
# artifact honest: it fails when the symbols exported by `ffi/c-binding/src/*.rs`
# (the registry-generated set in `generated.rs`, the fixed-template entry points
# in `lib.rs`, and the research surface in `research.rs`) stop matching the
# declarations in `ffi/c-binding/include/*.h`. `#[cfg(test)]`-gated exports are
# ignored, since they are intentionally absent from a release build.
gen-c-binding:
	@echo "no in-tree generator for ffi/c-binding/src/generated.rs;"
	@echo "the FFI SSOT is docs/ffi_registry.json -- run 'make verify-ffi' to check it."

# ---- packaging: the native C/C++ SDK archive --------------------------------
# The archive is the C/C++ half of the release. It used to be hand-zipped, which
# let its member list and timestamps drift; the script now packs a fixed member
# set with fixed timestamps, so identical inputs give a byte-identical archive.
# The linker output itself is not reproducible, so `--verify` compares content
# rather than digests.
build-native-archive:
	python3 $(ROOT)/scripts/build_native_archive.py

verify-native-archive:
	python3 $(ROOT)/scripts/build_native_archive.py --verify

# ---- documentation gate: rustdoc must be warning-free (ADR 0011) -----------
# `cargo doc` *is* the generated interface reference, so a broken intra-doc link
# is a broken link in the interface documentation. The script is the single
# implementation of this gate; the `doc` job in .github/workflows/ci.yml runs
# the same file, so a local run and CI cannot disagree.
check-rustdoc:
	bash $(ROOT)/scripts/check_rustdoc.sh

# ---- repository hygiene: no orphan scripts, workflows or documents ---------
# A `scripts/` file with no consumer reads as documentation of a workflow that
# does not exist; a workflow whose `on:` block can never match reads as a live
# gate that never runs; a Markdown document no other document links to is
# unreachable from the index and indistinguishable from a deleted one. All three
# are checked, and all three must stay clean.
check-orphans:
	python3 $(ROOT)/scripts/check_orphan_scripts.py
	python3 $(ROOT)/scripts/check_workflow_liveness.py
	python3 $(ROOT)/scripts/check_orphan_docs.py

# ---- repository hygiene: the inverse direction of `check-orphans` ----------
# `check_orphan_scripts.py` catches a script with no consumer. This catches the
# opposite break: a workflow, Makefile target or document that *calls* a script
# which is not in the tree. That reads as an automated gate while being a link
# to nothing, and no compiler or test ever visits it.
check-script-refs:
	python3 $(ROOT)/scripts/check_script_references.py

# ---- repository hygiene: no unexplained dead_code suppressions -------------
# `#[allow(dead_code)]` turns off the only compiler check that can see orphan
# logic. Most suppressions here are legitimate -- a `#[cfg]`-gated fallback the
# current build does not select, or a legacy spelling kept for one release --
# but a bare one is indistinguishable from a function that was never wired up.
# Five of them turned out to be exactly that. Require a `// why` next to each.
check-dead-code:
	python3 $(ROOT)/scripts/check_dead_code_allows.py

# ---- repository hygiene: float ordering must be NaN-safe --------------------
# `f64::partial_cmp` returns `None` on NaN, so `partial_cmp(..).unwrap()` panics
# on a gap value instead of propagating NaN -- the opposite of this codebase's
# `null_policy: "nan"` convention. 45 such sites existed in the pattern and
# percentile paths (34 in `patterns/chart.rs` alone). `total_cmp` is the
# NaN-safe house style; this gate keeps the outliers from returning.
check-nan-safety:
	python3 $(ROOT)/scripts/check_nan_unsafe_ordering.py

# ---- repository hygiene: the iOS header matches the iOS exports -------------
# `gen_c_header.py --check` guards the C binding's header. The iOS binding has
# a header of its own and no gate, so it drifted: three shipped entry points
# (`finkit_ios_factor_study_json`, its free function, and
# `finkit_ios_quant_evaluation_json`) were undeclared, invisible to any plain C
# consumer because the Swift wrappers bypass the header via `@_silgen_name`.
check-ios-header:
	python3 $(ROOT)/scripts/check_ios_header_contract.py

# ---- repository hygiene: the TA-Lib transcription matches TA-Lib ------------
# `core/src/talib_ffi.rs` is a hand-written transcription of TA-Lib's
# `ta_func.h`, compiled only under `talib-c` and used by the head-to-head
# benchmark. Nothing compared it against the library it mirrors, so it drifted
# three ways at once: it declared `TA_SKEWNESS`/`TA_KURTOSIS` (neither exists in
# TA-Lib at any version), it cited "TA-Lib 0.6.4" while binding
# `TA_PERCENTRANK` (which 0.6.4 does not have), and its hand-written counts
# disagreed with each other and with the file. An unused `extern` never reaches
# the linker, which is why the benchmark stayed green throughout.
check-talib-ffi:
	python3 $(ROOT)/scripts/check_talib_ffi_contract.py

# Default underneath MSYS/Git Bash so the installer script runs even if the
# caller's `make` was invoked from PowerShell.
BASH := bash

# The `--target` values scripts/build-installer.sh understands.
INSTALLER_TARGETS := native msi deb rpm pkg dmg

# Build the installers this host can actually produce. Not every host has the
# tooling for every format, and that is why the list is not hardcoded: on
# Linux it yields deb/rpm, on macOS pkg/dmg, on Windows msi. `native` is always
# included by the script itself.
installer: installer-native

# `$*` is the part matched by `%`, so `make installer-deb` becomes
# `--target deb`. A pattern rule rather than six copies of the same recipe.
installer-%:
	$(BASH) $(ROOT)/scripts/build-installer.sh --target $*

# Every installer this host supports. Pass STRICT=1 to turn a missing tool from
# a skip into a failure -- use it in release builds, where silently shipping a
# Release without its Windows installer is worse than a red build.
installer-all:
	$(BASH) $(ROOT)/scripts/build-installer.sh --all $(if $(STRICT),--strict,)

.PHONY: installer $(INSTALLER_TARGETS:%=installer-%) installer-all

# The MSI payload and the portable archive must describe the same set of files;
# this is the gate that keeps them from drifting apart.
check-installer:
	python3 $(ROOT)/scripts/check_installer_contract.py

.PHONY: check-installer

# ---- release records: keep the shipped digests in step with the artefacts ---
# `dist/**/manifest.json` records a `size_bytes`/`sha256` pair per shipped
# artefact. The linker output is not reproducible, so every rebuild changes
# those numbers and a stale record is indistinguishable from a fresh one by
# inspection. Refresh after any rebuild; `--check` is the verify-only form.
refresh-release-manifests:
	python3 $(ROOT)/scripts/refresh_release_manifests.py

check-release-manifests:
	python3 $(ROOT)/scripts/refresh_release_manifests.py --check

verify-bindings: verify-all-bindings

# ---- codegen: registry-driven drift check for the active FFI binding tier --
# `verify-bindings-tier` drift-checks the **active tier** (Python, Node) against
# docs/ffi_registry.json -- the Rust core needs no binding, so this is the whole
# first tier. `verify-all-bindings` additionally *reports* the deferred
# languages (c/go/java/dotnet/ios/android): they stay in-tree and keep
# compiling, but their bodies are not stored, so they are not drift-checked.
# Neither target accepts `--allow-unchecked`: a tier-1 language without stored
# bodies is a hard failure, not something to wave through.
verify-bindings-tier:
	python3 $(ROOT)/scripts/sync_bindings.py --check

verify-all-bindings:
	python3 $(ROOT)/scripts/sync_bindings.py --check --all

# ---- help ------------------------------------------------------------------
help:
	@echo ""
	@echo "Finkit one-click targets"
	@echo "========================"
	@echo "  make                  Build + verify all discovered language packages (default)"
	@echo "  make <lang>           Build + verify a single language"
	@echo "                          languages: $(LANGS)"
	@echo "  make bench-vs-talib   Finkit vs TA-Lib C head-to-head"
	@echo "  make install-and-test Install built artifacts + run smoke tests"
	@echo "  make docker-build     Build the one-click Docker image"
	@echo "  make docker-run       Run the build inside Docker (mounts ./dist)"
	@echo "  make docker-bench     Run only --bench-talib inside Docker"
	@echo "  make installer        Build this host's native SDK installer package"
	@echo "  make installer-all    Build every installer this host supports (STRICT=1 to fail on missing tools)"
	@echo "  make check-installer  Fail if the MSI payload and the native archive disagree"
	@echo "  make preflight        Toolchain pre-check (no build)"
	@echo "  make clean            Wipe dist/"
	@echo "  make gen-c-header     Regenerate ffi/c-binding/include/finkit.h from registry"
	@echo "  make verify-ffi       Fail if the C header has drifted from the registry"
	@echo "  make gen-c-binding    Regenerate ffi/c-binding/src/{lib.rs,generated.rs} from registry"
	@echo "  make verify-bindings  Fail if the C wrappers drifted from the registry"
	@echo "  make verify-bindings-tier  Drift-check the active tier (Python, Node)"
	@echo "  make verify-all-bindings  Same, plus report the deferred languages"
	@echo "  make check-rustdoc    Fail if rustdoc emits any diagnostic (ADR 0011)"
	@echo "  make check-orphans    Fail on orphan scripts, unreachable workflows or unreachable docs"
	@echo "  make check-script-refs  Fail if a caller references a script that is missing"
	@echo "  make check-dead-code    Fail on an unexplained `#[allow(dead_code)]`"
	@echo "  make check-nan-safety   Fail on `partial_cmp(..).unwrap()` (panics on NaN)"
	@echo "  make check-ios-header   Fail if ffi/ios-binding/include/finkit.h drifts from the iOS exports"
	@echo "  make refresh-release-manifests  Recompute dist/**/manifest.json digests"
	@echo "  make check-release-manifests    Fail if those digests are stale"
	@echo ""
	@echo "Underlying scripts (read these for full control):"
	@echo "  build-usage.{sh,ps1}                  Root entry point"
	@echo "  scripts/build-usage-packages.{sh,ps1} Unified builder+verifier"
	@echo "  scripts/bench-vs-talib.{sh,ps1}       TA-Lib head-to-head"
	@echo "  scripts/install-and-test.{sh,ps1}     Local install + smoke"
	@echo ""
