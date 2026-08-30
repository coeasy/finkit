#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# AlphaTA .NET usage-package builder.
#
# Produces `AlphaTA.<version>.nupkg` and stages the native lib under
# `runtimes/<rid>/native/`. The .nupkg is a *real* NuGet package that
# can be `dotnet add package`'d from a local feed (see packaging/usage/
# dotnet/README.md).
# ----------------------------------------------------------------------------

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$( cd "${SCRIPT_DIR}/.." && pwd )"
BINDING_DIR="${ROOT}/ffi/dotnet-binding"

case "$( uname -s )" in
  MINGW*|MSYS*|CYGWIN*)
    PLATFORM="windows-x64"
    RID="win-x64"
    NATIVE="alpha_ta_dotnet.dll" ;;
  Darwin)
    case "$( uname -m )" in
      arm64)
        PLATFORM="macos-arm64"; RID="osx-arm64"
        NATIVE="libalpha_ta_dotnet.dylib" ;;
      x86_64)
        PLATFORM="macos-x64"; RID="osx-x64"
        NATIVE="libalpha_ta_dotnet.dylib" ;;
    esac ;;
  Linux)
    case "$( uname -m )" in
      aarch64) PLATFORM="linux-arm64"; RID="linux-arm64"
               NATIVE="libalpha_ta_dotnet.so" ;;
      x86_64)  PLATFORM="linux-x64"; RID="linux-x64"
               NATIVE="libalpha_ta_dotnet.so" ;;
    esac ;;
  *) echo "unsupported platform: $( uname -s )" >&2; exit 1 ;;
esac

# Translate a POSIX path to a Windows path when running on a native Windows
# shell (MINGW/MSYS/CYGWIN). Windows-native tools (dotnet, PowerShell) do not
# understand POSIX-style paths such as /p/llm_code/..., so we convert before
# passing them as command-line arguments.
to_win() {
  case "$( uname -s )" in
    MINGW*|MSYS*|CYGWIN*) cygpath -w "$1" 2>/dev/null || echo "$1" ;;
    *) echo "$1" ;;
  esac
}

OUT_DIR="${ROOT}/dist/dotnet/${PLATFORM}"
RUNTIMES_DIR="${OUT_DIR}/runtimes/${RID}/native"
mkdir -p "${RUNTIMES_DIR}"

# 1. Build the native cdylib ---------------------------------------------
echo "[build-usage-dotnet] cargo build --release -p alpha-ta-dotnet"
( cd "${ROOT}" && cargo build --release -p alpha-ta-dotnet )

if [[ ! -f "${ROOT}/target/release/${NATIVE}" ]]; then
  echo "[build-usage-dotnet] ERROR: native lib not found: ${NATIVE}" >&2
  exit 1
fi
cp "${ROOT}/target/release/${NATIVE}" "${RUNTIMES_DIR}/"
echo "[build-usage-dotnet] staged runtimes/${RID}/native/${NATIVE}"

# 2. dotnet pack ---------------------------------------------------------
if ! command -v dotnet >/dev/null 2>&1; then
  echo "[build-usage-dotnet] dotnet CLI not found, skipping pack"
  exit 0
fi

# Stage the runtimes/ tree where the csproj <Content Include="..\..\native\**\*">
# expects to find it.  On POSIX we can use a symlink; on Windows we copy the
# tree (symlinks need elevated privileges).
LINK_TARGET="${BINDING_DIR}/native"
if [[ -L "${LINK_TARGET}" || -d "${LINK_TARGET}" || -f "${LINK_TARGET}" ]]; then
    rm -rf "${LINK_TARGET}"
fi
mkdir -p "${LINK_TARGET}"
if command -v cp >/dev/null 2>&1; then
    cp -r "${OUT_DIR}/runtimes/." "${LINK_TARGET}/"
else
    # PowerShell fallback
    powershell -NoProfile -Command "Copy-Item -Recurse -Force '$(to_win "${OUT_DIR}")/runtimes/*' '$(to_win "${LINK_TARGET}")/'"
fi

echo "[build-usage-dotnet] dotnet pack -c Release"
( cd "${BINDING_DIR}/src/AlphaTA" && dotnet pack -c Release -o "$(to_win "${OUT_DIR}")" -p:Version=1.0.0 )

# Drop the staging tree
rm -rf "${LINK_TARGET}"

echo
echo "[build-usage-dotnet] done. Artifacts in ${OUT_DIR}:"
find "${OUT_DIR}" -maxdepth 4 -type f | sed "s|^|  |"
