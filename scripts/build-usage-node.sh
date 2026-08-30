#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# AlphaTA Node.js usage-package builder.
#
# Produces `alpha_ta-1.0.0.tgz` (npm packed from the ffi/node-binding
# manifest) plus the per-triple `.node` files, in
# dist/node/<platform>/.  The .tgz name is **alpha_ta** (not alpha-ta-node) so
# consumers can `npm install alpha_ta@1.0.0`.
# ----------------------------------------------------------------------------

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$( cd "${SCRIPT_DIR}/.." && pwd )"
BINDING_DIR="${ROOT}/ffi/node-binding"

# platform normalize ---------------------------------------------------------
case "$( uname -s )" in
  MINGW*|MSYS*|CYGWIN*) PLATFORM="windows-x64" ;;
  Darwin)
    case "$( uname -m )" in
      arm64)  PLATFORM="macos-arm64" ;;
      x86_64) PLATFORM="macos-x64" ;;
    esac ;;
  Linux)
    case "$( uname -m )" in
      aarch64) PLATFORM="linux-arm64" ;;
      x86_64)  PLATFORM="linux-x64" ;;
    esac ;;
  *) echo "unsupported platform: $( uname -s )" >&2; exit 1 ;;
esac

OUT_DIR="${ROOT}/dist/node/${PLATFORM}"
mkdir -p "${OUT_DIR}"

VERSION="$( grep -E '^version' "${ROOT}/Cargo.toml" | head -1 | sed -E 's/.*"([^"]+)".*/\1/' )"
PKG_NAME="alpha_ta-${VERSION}.tgz"

has_tool() { command -v "$1" >/dev/null 2>&1; }

# Build the native .node for the current triple ---------------------------
echo "[build-usage-node] cargo build --release -p alpha-ta-node"
( cd "${ROOT}" && cargo build --release -p alpha-ta-node )

# Copy the .node binary into the package output ---------------------------
# N-API produces platform-specific names like: alpha_ta.win32-x64-msvc.node
for cand in "${ROOT}/target/release/alpha_ta."*.node; do
  if [[ -f "${cand}" ]]; then
    cp "${cand}" "${OUT_DIR}/"
    echo "[build-usage-node] staged $( basename "${cand}" )"
  fi
done

# Stage the binding manifest so `npm install` can find it ----------------
cp -r "${BINDING_DIR}/." "${OUT_DIR}/" 2>/dev/null || true
# Drop the heavyweight node_modules from the stage
rm -rf "${OUT_DIR}/node_modules"

# Build the .tgz via `npm pack` (so name/version are derived from package.json)
echo "[build-usage-node] npm pack -> ${PKG_NAME}"
( cd "${OUT_DIR}" && npm pack --silent )

# Cleanup staging copy (we kept only the .tgz + .node + manifest files in OUT_DIR)
# (no-op; OUT_DIR was filled by the cp above; we now keep everything in place)

echo
echo "[build-usage-node] done. Artifacts in ${OUT_DIR}:"
ls -lh "${OUT_DIR}/" | sed -n '2,$p'
