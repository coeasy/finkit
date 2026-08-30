#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# AlphaTA Java usage-package builder.
#
# Produces `alpha-ta-1.0.0.jar` and the native `.dll`/`.so`/`.dylib` for
# the current platform, in dist/java/<platform>/.  The native lib is
# staged under `natives/` inside the JAR so consumers do not need to set
# `java.library.path`.
# ----------------------------------------------------------------------------

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$( cd "${SCRIPT_DIR}/.." && pwd )"
BINDING_DIR="${ROOT}/ffi/java-binding"

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

OUT_DIR="${ROOT}/dist/java/${PLATFORM}"
mkdir -p "${OUT_DIR}/natives"

VERSION="$( grep -E '^version' "${ROOT}/Cargo.toml" | head -1 | sed -E 's/.*"([^"]+)".*/\1/' )"
JAR_NAME="alpha-ta-${VERSION}.jar"

has_tool() { command -v "$1" >/dev/null 2>&1; }

# 1. Build the native cdylib ----------------------------------------------
echo "[build-usage-java] cargo build --release -p finkit-java"
( cd "${ROOT}" && cargo build --release -p finkit-java )

# 2. Stage the native lib ------------------------------------------------
case "${PLATFORM}" in
  windows-x64)  NATIVE="alpha_ta_java.dll"          ;;
  macos-*)      NATIVE="libalpha_ta_java.dylib"     ;;
  linux-*)      NATIVE="libalpha_ta_java.so"        ;;
esac

if [[ ! -f "${ROOT}/target/release/${NATIVE}" ]]; then
  echo "[build-usage-java] ERROR: native lib not found: ${NATIVE}" >&2
  exit 1
fi
cp "${ROOT}/target/release/${NATIVE}" "${OUT_DIR}/natives/"
echo "[build-usage-java] staged natives/${NATIVE}"

# 3. Maven package --------------------------------------------------------
if has_tool mvn; then
  echo "[build-usage-java] mvn package -DskipTests"
  ( cd "${BINDING_DIR}" && mvn -B -q package -DskipTests -Dmaven.javadoc.skip=true )

  # Pick the primary jar (skip sources/javadoc).
  for jar in "${BINDING_DIR}/target/"*.jar; do
    base="$( basename "${jar}" )"
    if [[ "${base}" == *sources* || "${base}" == *javadoc* ]]; then
      continue
    fi
    cp "${jar}" "${OUT_DIR}/${JAR_NAME}"
    echo "[build-usage-java] staged ${JAR_NAME}"
    break
  done
else
  echo "[build-usage-java] mvn not found, skipping jar packaging"
fi

# 4. Merge the native lib *inside* the jar -------------------------------
if [[ -f "${OUT_DIR}/${JAR_NAME}" ]]; then
  echo "[build-usage-java] embedding native lib into ${JAR_NAME}"
  ( cd "${OUT_DIR}" && jar uf "${JAR_NAME}" -C natives "${NATIVE}" )
fi

# 5. Drop the staging natives/ directory now that it is inside the jar ----
rm -rf "${OUT_DIR}/natives"

echo
echo "[build-usage-java] done. Artifacts in ${OUT_DIR}:"
ls -lh "${OUT_DIR}/" | sed -n '2,$p'
