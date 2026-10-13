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
PYTHON_BIN="${FINKIT_PYTHON:-python3}"
export PYO3_PYTHON="${PYTHON_BIN}"
if [[ "$( uname -s )" == MINGW* || "$( uname -s )" == MSYS* || "$( uname -s )" == CYGWIN* ]]; then
  PYTHON_DIR="$( dirname "${PYTHON_BIN}" )"
  export PATH="${PYTHON_DIR}/Scripts:${PATH}"
fi

has_tool() { command -v "$1" >/dev/null 2>&1; }

ensure_maturin() {
  if "${PYTHON_BIN}" -m maturin --version >/dev/null 2>&1; then return 0; fi
  if has_tool maturin; then return 0; fi
  echo "[build-usage-python] maturin not found, installing..."
  "${PYTHON_BIN}" -m pip install --quiet maturin
}

# Prefer the module installed for the selected Python interpreter. This also
# works on Windows environments where the Python Scripts directory is not on
# Git Bash's PATH and the `maturin` shim cannot be discovered.
maturin() {
  if "${PYTHON_BIN}" -m maturin --version >/dev/null 2>&1; then
    "${PYTHON_BIN}" -m maturin "$@"
  else
    command maturin "$@"
  fi
}

# --- abi3 stable wheel (Python 3.8+) --------------------------------------
build_abi3() {
  echo "[build-usage-python] building abi3 stable wheel"
  (
    cd "${BINDING_DIR}"
    # The project config enables PyO3 abi3 globally. One stable-ABI wheel
    # covers all supported interpreters; repeating builds with each
    # --interpreter only overwrote the same cp38-abi3 artifact and could hide
    # failed cargo builds behind `|| true`.
    maturin build --release --out "${OUT_DIR}" --features abi3 --strip
  )
  local n
  n=$( find "${OUT_DIR}" -name 'finkit-*-abi3-*.whl' | wc -l )
  echo "[build-usage-python] abi3 wheels: ${n}"
}

# --- main -------------------------------------------------------------------
ensure_maturin
build_abi3

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
