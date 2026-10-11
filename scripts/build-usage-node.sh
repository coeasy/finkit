#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# Finkit Node.js usage-package builder.
#
# Produces a versioned `finkit-<workspace-version>.tgz` (npm packed from the ffi/node-binding
# manifest) plus the per-triple `.node` files, in
# dist/node/<platform>/.  The .tgz name is **finkit** so
# consumers can install the matching workspace version.
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
PKG_NAME="finkit-${VERSION}.tgz"

has_tool() { command -v "$1" >/dev/null 2>&1; }

# Build the native .node for the current triple ---------------------------
echo "[build-usage-node] cargo build --release -p finkit-node"
( cd "${ROOT}" && cargo build --release -p finkit-node )

# Copy the .node binary into the package output ---------------------------
# N-API produces platform-specific names like: finkit.win32-x64-msvc.node
for cand in "${ROOT}/target/release/finkit".*.node; do
  if [[ -f "${cand}" ]]; then
    cp "${cand}" "${OUT_DIR}/"
    echo "[build-usage-node] staged $( basename "${cand}" )"
  fi
done

# Stage the binding manifest so `npm install` can find it ----------------
cp -r "${BINDING_DIR}/." "${OUT_DIR}/" 2>/dev/null || true
# Drop the heavyweight node_modules from the stage
rm -rf "${OUT_DIR}/node_modules"

# The copy above takes the *whole* binding directory, and a working directory
# collects gitignored build output. This one still held
# `alpha_ta.win32-x64-msvc.node`, a binary from the product's previous name that
# no `index.js` resolves.
#
# It would not reach a consumer through the npm tarball -- that is JS-only by
# `package.json`'s `files` field, and the native binary ships in the per-platform
# `finkit-<triple>` optional dependencies -- but it does reach everything built
# from this directory: the loose `dist/node/<platform>/` tree that the manifest
# records, and the usage-bundle zip assembled from it. A dead native module in a
# release asset is a support question waiting to happen, and its presence made
# the manifest's coverage look broader than the products actually behind it.
#
# The rule is "the package's own binary", not "the files copied above": the
# `target/release/finkit.*.node` glob two blocks up does not match on this
# tree at all (cargo emits `finkit_node.dll`; the `.node` is produced by the
# binding's own napi step), so a prune driven by what that loop staged would
# delete the real binary as well as the stale one. napi names the binary after
# the package, so read the package name and keep exactly `<name>.*.node`.
PKG_BASENAME="$( grep -m1 '"name"' "${BINDING_DIR}/package.json" 2>/dev/null \
  | sed -E 's/.*"name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/' \
  | awk -F/ '{print $NF}' )"
PKG_BASENAME="${PKG_BASENAME:-finkit}"
for present in "${OUT_DIR}"/*.node; do
  [[ -e "${present}" ]] || continue
  case "$( basename "${present}" )" in
    "${PKG_BASENAME}."*.node) echo "[build-usage-node] kept $( basename "${present}" )" ;;
    *) echo "[build-usage-node] dropped stale $( basename "${present}" )"
       rm -f "${present}" ;;
  esac
done

# Build the .tgz via `npm pack` (so name/version are derived from package.json)
echo "[build-usage-node] npm pack -> ${PKG_NAME}"
( cd "${OUT_DIR}" && npm pack --silent )

# Cleanup staging copy (we kept only the .tgz + .node + manifest files in OUT_DIR)
# (no-op; OUT_DIR was filled by the cp above; we now keep everything in place)

echo
echo "[build-usage-node] done. Artifacts in ${OUT_DIR}:"
ls -lh "${OUT_DIR}/" | sed -n '2,$p'
