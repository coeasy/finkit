#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# Finkit WASM usage-package builder.
#
# Produces three bundles:
#   * dist/wasm/web/         — `<script type="module">` consumer
#   * dist/wasm/nodejs/      — CommonJS/ESM Node.js consumer
#   * dist/wasm/bundler/     — Webpack/Vite/rollup consumer
# ----------------------------------------------------------------------------

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$( cd "${SCRIPT_DIR}/.." && pwd )"

OUT_DIR="${ROOT}/dist/wasm"
mkdir -p "${OUT_DIR}"

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "[build-usage-wasm] installing wasm-pack"
  cargo install --locked wasm-pack
fi

# A wasm-pack that cannot drive the pinned cargo fails *inside* cargo, where the
# message is about a flag nobody in this repository wrote:
#
#   error: unexpected argument '--out-dir' found
#     tip: a similar argument exists: '--artifact-dir'
#   Usage: cargo.exe build [OPTIONS]
#
# That reads as a broken Cargo.toml. It is not: wasm-pack 0.15.0 forwards
# `--out-dir` to `cargo build`, and the pinned toolchain no longer accepts it.
# The version pair is reported up front, and a failure gets the diagnosis
# attached rather than leaving the reader to find it.
#
# Worth knowing while reading this file: CI does **not** use wasm-pack for
# releases -- `.github/workflows/multilang-release.yml` builds the artifact with
# `cargo build --target wasm32-unknown-unknown --release` and copies the `.wasm`
# -- so this script's three JS-glue bundles have no CI counterpart, and the two
# paths can drift without anything noticing.
echo "[build-usage-wasm] $( wasm-pack --version 2>/dev/null || echo 'wasm-pack unknown' ) with $( cargo --version 2>/dev/null || echo 'cargo unknown' )"

wasm_pack_hint() {
  echo "[build-usage-wasm] ERROR: wasm-pack could not complete the build." >&2
  echo "  Known mismatch: wasm-pack 0.15.0 with cargo >= 1.98, where wasm-pack" >&2
  echo "  passes \`--out-dir\` to \`cargo build\` and cargo rejects it." >&2
  echo "  Install a wasm-pack matching the pinned toolchain, or build the artifact" >&2
  echo "  the way CI does:" >&2
  echo "    cargo build -p finkit-wasm --target wasm32-unknown-unknown --release --locked" >&2
}

if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
  echo "[build-usage-wasm] installing wasm32-unknown-unknown target"
  rustup target add wasm32-unknown-unknown
fi

WASM_CRATE="${ROOT}/wasm"
if [[ ! -d "${WASM_CRATE}" ]]; then
  echo "[build-usage-wasm] ERROR: ${WASM_CRATE} does not exist" >&2
  exit 1
fi

build_target() {
  local target="$1"
  local out="$2"
  echo "[build-usage-wasm] wasm-pack build --target ${target} --out-dir ${out}"
  # `console_error_panic_hook` is what turns a Rust panic into a readable
  # console message; without it the module traps with a bare `unreachable` and
  # no cause. The feature is opt-in because its dependency is declared only for
  # `cfg(target_arch = "wasm32")`, so every wasm build must ask for it
  # explicitly — nothing else in the tree does.
  ( cd "${WASM_CRATE}" && wasm-pack build --release --target "${target}" \
      --features console_error_panic_hook --out-dir "${out}" ) || {
    wasm_pack_hint
    exit 1
  }
}

build_target web       "${OUT_DIR}/web"
build_target nodejs    "${OUT_DIR}/nodejs"
build_target bundler   "${OUT_DIR}/bundler"

# Drop the published package.json that wasm-pack writes — the dist/wasm tree
# is the single source of truth and we don't want two competing manifests.
echo
echo "[build-usage-wasm] done. Artifacts in ${OUT_DIR}:"
find "${OUT_DIR}" -maxdepth 3 -type f | sed "s|^|  |"
