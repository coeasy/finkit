#!/usr/bin/env bash
# Build the Finkit iOS XCFramework from the Rust static library.
#
# Rust's iOS targets used here are:
#   * aarch64-apple-ios      - physical arm64 iPhone/iPad
#   * aarch64-apple-ios-sim  - Apple Silicon simulator
#   * x86_64-apple-ios       - Intel simulator
#
# The two simulator libraries are combined with lipo into one universal
# simulator slice before xcodebuild creates the final XCFramework.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
OUT="${ROOT}/dist/ios"
WORK="${OUT}/_work"
HEADERS="${WORK}/headers"

DEVICE_TARGET="aarch64-apple-ios"
SIM_ARM_TARGET="aarch64-apple-ios-sim"
SIM_X64_TARGET="x86_64-apple-ios"

rm -rf "${WORK}" "${OUT}/Finkit.xcframework"
mkdir -p "${OUT}" "${WORK}" "${HEADERS}"

cp "${ROOT}/ffi/ios-binding/include/finkit.h" "${HEADERS}/"
cp "${ROOT}/ffi/ios-binding/include/module.modulemap" "${HEADERS}/"
cp "${ROOT}/ffi/ios-binding/include/Finkit.swift" "${HEADERS}/"

build_target() {
  local target="$1"
  echo "[build-ios] cargo build --release --locked --target ${target} -p finkit-ios"
  (cd "${ROOT}" && cargo build --release --locked --target "${target}" -p finkit-ios)
}

build_target "${DEVICE_TARGET}"
build_target "${SIM_ARM_TARGET}"
build_target "${SIM_X64_TARGET}"

DEVICE_LIB="${ROOT}/target/${DEVICE_TARGET}/release/libfinkit_ios.a"
SIM_ARM_LIB="${ROOT}/target/${SIM_ARM_TARGET}/release/libfinkit_ios.a"
SIM_X64_LIB="${ROOT}/target/${SIM_X64_TARGET}/release/libfinkit_ios.a"
SIM_UNIVERSAL_LIB="${WORK}/libfinkit_ios_simulator.a"

for lib in "${DEVICE_LIB}" "${SIM_ARM_LIB}" "${SIM_X64_LIB}"; do
  test -f "${lib}"
done

lipo -create "${SIM_ARM_LIB}" "${SIM_X64_LIB}" -output "${SIM_UNIVERSAL_LIB}"
lipo -info "${SIM_UNIVERSAL_LIB}"

xcodebuild -create-xcframework \
  -library "${DEVICE_LIB}" -headers "${HEADERS}" \
  -library "${SIM_UNIVERSAL_LIB}" -headers "${HEADERS}" \
  -output "${OUT}/Finkit.xcframework"

echo "[build-ios] OK: ${OUT}/Finkit.xcframework"
find "${OUT}/Finkit.xcframework" -maxdepth 3 -type f -print
