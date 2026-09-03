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
LANGS := $(notdir $(wildcard $(ROOT)/scripts/build-usage-*.sh $(ROOT)/scripts/build-usage-*.ps1))
LANGS := $(LANGS:build-usage-%.sh=%)
LANGS := $(LANGS:build-usage-%.ps1=%)
LANGS := $(sort $(LANGS))

# ---- phony targets ---------------------------------------------------------
.PHONY: all help clean dist
.PHONY: $(LANGS)
.PHONY: bench-vs-talib bench-talib
.PHONY: install-and-test
.PHONY: docker-build docker-run docker-bench
.PHONY: preflight lint
.PHONY: gen-c-header verify-ffi gen-c-binding verify-bindings verify-all-bindings

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

# ---- codegen: regenerate the C Rust wrappers from the indicator registry ---
# `gen-c-binding` rewrites ffi/c-binding/src/{lib.rs -> include! generated.rs}
# from docs/indicator_registry.json. `verify-bindings` fails CI if the
# committed generated.rs has drifted from the registry. Python/Node emitters
# can be generated on demand with scripts/gen_binding.py.
gen-c-binding:
	python3 $(ROOT)/scripts/gen_binding.py --lang c --rewrite-cbinding

verify-bindings:
	python3 $(ROOT)/scripts/gen_binding.py --lang c --check

# ---- codegen: registry-driven drift check for all tracked FFI bindings -----
# `verify-all-bindings` runs scripts/sync_bindings.py --check across the
# registry-backed language bindings and fails if committed wrappers drift from
# docs/indicator_registry.json.
verify-all-bindings:
	python3 $(ROOT)/scripts/sync_bindings.py --check

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
	@echo "  make preflight        Toolchain pre-check (no build)"
	@echo "  make clean            Wipe dist/"
	@echo "  make gen-c-header     Regenerate ffi/c-binding/include/finkit.h from registry"
	@echo "  make verify-ffi       Fail if the C header has drifted from the registry"
	@echo "  make gen-c-binding    Regenerate ffi/c-binding/src/{lib.rs,generated.rs} from registry"
	@echo "  make verify-bindings  Fail if the C wrappers drifted from the registry"
	@echo "  make verify-all-bindings  Fail if tracked bindings drift from the registry"
	@echo ""
	@echo "Underlying scripts (read these for full control):"
	@echo "  build-usage.{sh,ps1}                  Root entry point"
	@echo "  scripts/build-usage-packages.{sh,ps1} Unified builder+verifier"
	@echo "  scripts/bench-vs-talib.{sh,ps1}       TA-Lib head-to-head"
	@echo "  scripts/install-and-test.{sh,ps1}     Local install + smoke"
	@echo ""
