# ----------------------------------------------------------------------------
# AlphaTA iOS .xcframework builder.
#
# Produces `AlphaTA.xcframework` containing the AlphaTA static library
# compiled for the four iOS targets Apple supports as of 2026:
#   * aarch64-apple-ios          (physical iPhone / iPad)
#   * aarch64-apple-ios-sim     (Apple Silicon simulator)
#   * x86_64-apple-ios          (legacy Intel device, kept for completeness)
#   * x86_64-apple-ios-sim      (Intel Mac simulator)
#
# Required toolchains:
#   * Xcode 15+ (provides `xcodebuild`, `lipo`, `lldb`)
#   * `rustup target add aarch64-apple-ios aarch64-apple-ios-sim \
#                      x86_64-apple-ios x86_64-apple-ios-sim`
#   * `cargo install --locked cargo-lipo` (only when producing a universal
#     static lib; the script below builds per-target lipo manually so
#     cargo-lipo is optional).
# ----------------------------------------------------------------------------

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$( cd "${SCRIPT_DIR}/../.." && pwd )"
OUT="${ROOT}/dist/ios"
mkdir -p "${OUT}"

VERSION="$( grep -E '^version' "${ROOT}/Cargo.toml" | head -1 | sed -E 's/.*"([^"]+)".*/\1/' )"

TARGETS=(
  "aarch64-apple-ios"
  "aarch64-apple-ios-sim"
  "x86_64-apple-ios"
  "x86_64-apple-ios-sim"
)

build_target() {
  local target="$1"
  echo "[build-ios] cargo build --release --target ${target} -p finkit-ios"
  ( cd "${ROOT}" && cargo build --release --target "${target}" -p finkit-ios )
}

# 1. Compile for every target
for t in "${TARGETS[@]}"; do
  build_target "${t}"
done

# 2. Lay out the .xcframework skeleton
SLICES=(
  "ios-arm64:aarch64-apple-ios"
  "ios-arm64-simulator:aarch64-apple-ios-sim"
  "ios:x86_64-apple-ios"
  "ios-x86_64-simulator:x86_64-apple-ios-sim"
)

WORK="${OUT}/_work"
rm -rf "${WORK}"
mkdir -p "${WORK}"

for slice in "${SLICES[@]}"; do
  name="${slice%%:*}"
  target="${slice##*:}"
  mkdir -p "${WORK}/${name}/Headers"
  cp "${ROOT}/target/${target}/release/libfinkit_ios.a" "${WORK}/${name}/"
  cp "${ROOT}/ffi/ios-binding/include/finkit.h"  "${WORK}/${name}/Headers/"
  cp "${ROOT}/ffi/ios-binding/include/Finkit.swift" "${WORK}/${name}/Headers/"
done

# 3. Generate the .xcframework
rm -rf "${OUT}/Finkit.xcframework"
xcodebuild -create-xcframework \
  -library "${WORK}/ios-arm64/libfinkit_ios.a"            -headers "${WORK}/ios-arm64/Headers" \
  -library "${WORK}/ios-arm64-simulator/libfinkit_ios.a"  -headers "${WORK}/ios-arm64-simulator/Headers" \
  -library "${WORK}/ios/libfinkit_ios.a"                  -headers "${WORK}/ios/Headers" \
  -library "${WORK}/ios-x86_64-simulator/libfinkit_ios.a" -headers "${WORK}/ios-x86_64-simulator/Headers" \
  -output "${OUT}/Finkit.xcframework"

echo
echo "[build-ios] OK: ${OUT}/Finkit.xcframework"
ls -la "${OUT}/Finkit.xcframework"