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

# Build the N-API module for the current triple. `cargo build -p finkit-node`
# only emits the intermediate DLL; `napi build` applies the N-API filename and
# platform packaging contract required by the JavaScript loader.
echo "[build-usage-node] npm run build (N-API native module)"
( cd "${BINDING_DIR}" && npm run build )

PKG_BASENAME="$( grep -m1 '"name"' "${BINDING_DIR}/package.json" \
  | sed -E 's/.*"name"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/' \
  | awk -F/ '{print $NF}' )"
PKG_BASENAME="${PKG_BASENAME:-finkit}"

# Copy the .node binary into the package output ---------------------------
# N-API produces platform-specific names like: finkit.win32-x64-msvc.node
for cand in "${BINDING_DIR}/${PKG_BASENAME}."*.node; do
  if [[ -f "${cand}" ]]; then
    cp "${cand}" "${OUT_DIR}/"
    echo "[build-usage-node] staged $( basename "${cand}" )"
  fi
done

# Stage only the files the root npm package can publish. Copying the complete
# binding checkout also copied Cargo sources, tests, npm platform templates,
# lockfiles and build outputs into the release bundle even though package.json
# exposes only these runtime entry points.
for file in package.json index.js index.mjs index.d.ts README.md; do
  cp "${BINDING_DIR}/${file}" "${OUT_DIR}/"
done

# napi can leave ignored native binaries from an earlier product name in the
# binding checkout. Keep only the binary whose basename matches this package.
# The root tarball is JavaScript-only; the current native binary is packed into
# the matching optional platform package below.
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

# The root package intentionally delegates native loading to an optional
# platform package. A root tarball without that companion is not installable,
# so emit the current target's package alongside it and verify both together.
case "${PLATFORM}" in
  windows-x64) PLATFORM_PACKAGE="finkit-win32-x64-msvc" ;;
  linux-x64) PLATFORM_PACKAGE="finkit-linux-x64-gnu" ;;
  linux-arm64) PLATFORM_PACKAGE="finkit-linux-arm64-gnu" ;;
  macos-x64) PLATFORM_PACKAGE="finkit-darwin-x64" ;;
  macos-arm64) PLATFORM_PACKAGE="finkit-darwin-arm64" ;;
  *) echo "[build-usage-node] unsupported native npm platform: ${PLATFORM}" >&2; exit 1 ;;
esac
PLATFORM_DIR="${OUT_DIR}/${PLATFORM_PACKAGE}"
rm -rf "${PLATFORM_DIR}"
mkdir -p "${PLATFORM_DIR}"
cp "${OUT_DIR}/finkit."*.node "${PLATFORM_DIR}/"
NATIVE_BASENAME="$( basename "${PLATFORM_DIR}/"*.node )"
cat > "${PLATFORM_DIR}/package.json" <<EOF
{
  "name": "${PLATFORM_PACKAGE}",
  "version": "${VERSION}",
  "main": "./${NATIVE_BASENAME}",
  "os": ["$( case "${PLATFORM}" in windows-*) echo win32 ;; macos-*) echo darwin ;; *) echo linux ;; esac )"],
  "cpu": ["$( case "${PLATFORM}" in *arm64) echo arm64 ;; *) echo x64 ;; esac )"]
}
EOF
( cd "${PLATFORM_DIR}" && npm pack --silent --pack-destination "${OUT_DIR}" )

# The binary is already included in the platform tarball. Drop the staging copy
# from the loose release tree so the bundle contains only installable packages.
rm -f "${OUT_DIR}/"*.node

# Keep the two npm tarballs and package metadata in the output tree.

echo
echo "[build-usage-node] done. Artifacts in ${OUT_DIR}:"
ls -lh "${OUT_DIR}/" | sed -n '2,$p'
