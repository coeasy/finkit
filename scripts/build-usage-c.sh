#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# AlphaTA C/C++ usage-package builder.
#
# Runs the upstream CMake install() rules and copies the staged tree
# (libAlphaTA_ffi.so + headers + cmake config) to dist/c/<platform>/.
# ----------------------------------------------------------------------------

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$( cd "${SCRIPT_DIR}/.." && pwd )"
BINDING_DIR="${ROOT}/ffi/c-binding"

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

# Translate a POSIX path to a Windows path when running on a native Windows
# shell (MINGW/MSYS/CYGWIN). Windows-native tools (cmake, dotnet, ...) do not
# understand POSIX-style paths such as /p/llm_code/..., so we convert before
# passing them as command-line arguments.
to_win() {
  case "$( uname -s )" in
    MINGW*|MSYS*|CYGWIN*) cygpath -w "$1" 2>/dev/null || echo "$1" ;;
    *) echo "$1" ;;
  esac
}

OUT_DIR="${ROOT}/dist/c/${PLATFORM}"
PREFIX="${OUT_DIR}/installed"
rm -rf "${PREFIX}"
mkdir -p "${PREFIX}"

# 1. cargo build the cdylib ----------------------------------------------
echo "[build-usage-c] cargo build --release -p alpha-ta-ffi"
( cd "${ROOT}" && cargo build --release -p alpha-ta-ffi )

# 2. cmake install ------------------------------------------------------
echo "[build-usage-c] cmake --install into ${PREFIX}"
BUILD_DIR="${BINDING_DIR}/build-usage"
rm -rf "${BUILD_DIR}"

# Pick the generator that matches the host toolchain. We prefer:
#   * Ninja          (Linux/macOS, fastest)
#   * Visual Studio  (Windows, ships with Build Tools)
#   * default        (whatever CMake picks for the platform)
if [[ "${PLATFORM}" == windows-* ]]; then
    CMAKE_GENERATOR="Visual Studio 17 2022"
    CMAKE_ARCH="-A x64"
else
    CMAKE_GENERATOR="Ninja"
    CMAKE_ARCH=""
fi

cmake -S "$(to_win "${BINDING_DIR}")" -B "$(to_win "${BUILD_DIR}")" \
      -G "${CMAKE_GENERATOR}" ${CMAKE_ARCH} \
      -DCMAKE_BUILD_TYPE=Release \
      -DALPHA_TA_AUTO_BUILD_RS=OFF \
      -DALPHA_TA_BUILD_TESTS=OFF \
      -DALPHA_TA_BUILD_EXAMPLES=OFF
cmake --build "$(to_win "${BUILD_DIR}")" --config Release --parallel
cmake --install "$(to_win "${BUILD_DIR}")" --config Release --prefix "$(to_win "${PREFIX}")"

# 3. quick inventory ----------------------------------------------------
echo
echo "[build-usage-c] done. Installed tree at ${PREFIX}:"
find "${PREFIX}" -maxdepth 4 -type f | sed "s|^|  |"
