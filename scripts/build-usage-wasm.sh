#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# AlphaTA WASM usage-package builder.
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
  ( cd "${WASM_CRATE}" && wasm-pack build --release --target "${target}" --out-dir "${out}" )
}

build_target web       "${OUT_DIR}/web"
build_target nodejs    "${OUT_DIR}/nodejs"
build_target bundler   "${OUT_DIR}/bundler"

# Drop the published package.json that wasm-pack writes — the dist/wasm tree
# is the single source of truth and we don't want two competing manifests.
echo
echo "[build-usage-wasm] done. Artifacts in ${OUT_DIR}:"
find "${OUT_DIR}" -maxdepth 3 -type f | sed "s|^|  |"
