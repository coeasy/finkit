#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# Finkit Python usage-package builder.
#
# Produces wheels for Python 3.10..3.14 + an abi3 stable wheel, in
# dist/python/<platform>/. Fixes the `return` bug from the old
# build-all-packages.sh and rejects `--interpreter` together with
# `--features abi3` (maturin treats them as mutually exclusive).
# ----------------------------------------------------------------------------

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$( cd "${SCRIPT_DIR}/.." && pwd )"
BINDING_DIR="${ROOT}/ffi/python-binding"

VERSION="$( grep -E '^version' "${ROOT}/Cargo.toml" | head -1 | sed -E 's/.*"([^"]+)".*/\1/' )"
PLATFORM="$( uname -s | tr '[:upper:]' '[:lower:]' )-$( uname -m )"
OUT_DIR="${ROOT}/dist/python/${PLATFORM}"
mkdir -p "${OUT_DIR}"

# --- platform normalization (Windows / PowerShell parity) ------------------
case "$( uname -s )" in
  MINGW*|MSYS*|CYGWIN*) PLATFORM="windows-x64" ;;
  Darwin)
    case "$( uname -m )" in
      arm64)   PLATFORM="macos-arm64" ;;
      x86_64)  PLATFORM="macos-x64" ;;
    esac ;;
  Linux)
    case "$( uname -m )" in
      aarch64) PLATFORM="linux-arm64" ;;
      x86_64)  PLATFORM="linux-x64" ;;
    esac ;;
esac
OUT_DIR="${ROOT}/dist/python/${PLATFORM}"
mkdir -p "${OUT_DIR}"

# --- helpers ----------------------------------------------------------------
PYTHON_VERSIONS=( "3.10" "3.11" "3.12" "3.13" "3.14" )

has_tool() { command -v "$1" >/dev/null 2>&1; }

# Find the pythonX.Y interpreter for a given X.Y version. Echoes the path
# (or empty). Fixes the `return` bug in build-all-packages.sh.
python_for_version() {
  local v="$1"
  if has_tool "python${v}"; then
    echo "python${v}"; return 0
  fi
  if has_tool "python3"; then
    local cur
    cur="$( python3 --version 2>&1 | awk '{print $2}' )"
    if [[ "${cur}" == "${v}"* ]]; then
      echo "python3"; return 0
    fi
  fi
  return 1
}

ensure_maturin() {
  if has_tool maturin; then return 0; fi
  echo "[build-usage-python] maturin not found, installing..."
  python3 -m pip install --quiet maturin
}

# --- 1) abi3 stable wheel (Python 3.8+) ------------------------------------
build_abi3() {
  echo "[build-usage-python] building abi3 stable wheel"
  (
    cd "${BINDING_DIR}"
    # maturin refuses `--interpreter` together with `--features abi3`, so
    # we build the cdylib here and let maturin pick the abi3 layout.
    cargo build --release --features abi3 >/dev/null 2>&1 || true
    maturin build --release --out "${OUT_DIR}" --features abi3 --strip
  )
  local n
  n=$( find "${OUT_DIR}" -name 'finkit-*-abi3-*.whl' | wc -l )
  echo "[build-usage-python] abi3 wheels: ${n}"
}

# --- 2) version-specific wheels --------------------------------------------
build_specific() {
  local v="$1"
  local py
  if ! py="$( python_for_version "${v}" )"; then
    echo "[build-usage-python] Python ${v} not found, skipping"
    return 0
  fi

  echo "[build-usage-python] building wheel for Python ${v} (${py})"
  (
    cd "${BINDING_DIR}"
    maturin build --release --out "${OUT_DIR}" --interpreter "${py}" --strip
  )
}

# --- main -------------------------------------------------------------------
ensure_maturin
build_abi3
for v in "${PYTHON_VERSIONS[@]}"; do
  build_specific "${v}"
done

# Sanity: drop any wheel that still carries the legacy `rust_ta_lib` name.
LEGACY=$( find "${OUT_DIR}" -name 'rust_ta_lib-*.whl' 2>/dev/null || true )
if [[ -n "${LEGACY}" ]]; then
  echo "[build-usage-python] removing legacy rust_ta_lib wheels:"
  echo "${LEGACY}"
  rm -f ${LEGACY}
fi

echo
echo "[build-usage-python] done. Wheels in ${OUT_DIR}:"
ls -lh "${OUT_DIR}"/*.whl 2>/dev/null || echo "(no wheels produced)"
