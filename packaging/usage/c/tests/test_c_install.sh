#!/usr/bin/env bash
set -euo pipefail
ROOT="$( cd "$( dirname "${BASH_SOURCE[0]}" )/../../../.." && pwd )"
PLATFORM="$( uname -s | tr '[:upper:]' '[:lower:]' )-$( uname -m )"
case "$( uname -s )" in
  MINGW*|MSYS*|CYGWIN*) PLATFORM="windows-x64" ;;
  Darwin) PLATFORM="macos-$( [[ "$( uname -m )" == arm64 ]] && echo arm64 || echo x64 )" ;;
  Linux) PLATFORM="linux-$( [[ "$( uname -m )" == aarch64 ]] && echo arm64 || echo x64 )" ;;
esac
PREFIX="${ROOT}/dist/c/${PLATFORM}/installed"
BUILD="${ROOT}/.test_venv/_usage_c"
rm -rf "${BUILD}"
cmake_args=(-DCMAKE_PREFIX_PATH="${PREFIX}")
if [[ "${PLATFORM}" != windows-* ]]; then
  cmake_args+=(-DCMAKE_BUILD_TYPE=Release)
fi
cmake -S "${ROOT}/packaging/usage/c/tests" -B "${BUILD}" "${cmake_args[@]}"
cmake --build "${BUILD}" --config Release --parallel
if [[ "${PLATFORM}" == windows-* ]]; then
  PATH="${PREFIX}/bin:${PATH}" "${BUILD}/Release/finkit_installed_smoke.exe"
else
  "${BUILD}/finkit_installed_smoke"
fi
