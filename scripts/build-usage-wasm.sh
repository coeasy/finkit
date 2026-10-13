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
if [[ "${OUT_DIR}" != "${ROOT}/dist/wasm" || "${ROOT}" == "/" ]]; then
  echo "[build-usage-wasm] refusing unexpected output directory: ${OUT_DIR}" >&2
  exit 1
fi
rm -rf -- "${OUT_DIR}"
mkdir -p "${OUT_DIR}"

WASM_BINDGEN_VERSION="$( awk '/name = "wasm-bindgen"/{f=1;next} f && /version =/{gsub(/"/,"",$3);print $3;exit}' "${ROOT}/Cargo.lock" )"
if ! command -v wasm-bindgen >/dev/null 2>&1 || [[ "$(wasm-bindgen --version | awk '{print $2}')" != "${WASM_BINDGEN_VERSION}" ]]; then
  echo "[build-usage-wasm] installing wasm-bindgen-cli ${WASM_BINDGEN_VERSION} from Cargo.lock"
  cargo install --locked wasm-bindgen-cli --version "${WASM_BINDGEN_VERSION}"
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
  mkdir -p "${out}"
  echo "[build-usage-wasm] wasm-bindgen --target ${target} -> ${out}"
  wasm-bindgen "${ROOT}/target/wasm32-unknown-unknown/release/finkit_wasm.wasm" \
    --target "${target}" --out-name finkit_wasm --out-dir "${out}"
}

# Build through Cargo directly. wasm-pack 0.15 forwards --out-dir to Cargo,
# which rejects it on the pinned Cargo 1.98 toolchain. Running the locked
# build and the exactly matching wasm-bindgen CLI as separate steps keeps the
# native build flags valid and aligns generated JS glue with the locked crate.
( cd "${ROOT}" && cargo build -p finkit-wasm --target wasm32-unknown-unknown \
    --release --locked --features console_error_panic_hook )

build_target web       "${OUT_DIR}/web"
build_target nodejs    "${OUT_DIR}/nodejs"
build_target bundler   "${OUT_DIR}/bundler"

# Drop the published package.json that wasm-pack writes — the dist/wasm tree
# is the single source of truth and we don't want two competing manifests.
echo
echo "[build-usage-wasm] done. Artifacts in ${OUT_DIR}:"
find "${OUT_DIR}" -maxdepth 3 -type f | sed "s|^|  |"
