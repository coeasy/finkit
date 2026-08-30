#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# AlphaTA native system installer builder.
#
# Produces *true* OS-level installers that integrate with the platform
# package manager / installer framework (NOT a plain tarball):
#
#   * Windows : .msi via WiX Toolset 3.x
#   * Debian  : .deb (dpkg-deb)
#   * Fedora  : .rpm  (rpmbuild)
#   * macOS   : .pkg  (pkgbuild) and .dmg via hdiutil
#
# Pre-requisites (CI installs these before invoking the script):
#   * WiX 3.x           (Windows)
#   * dpkg-dev / fakeroot
#   * rpm-build
#   * Xcode Command Line Tools (pkgbuild + hdiutil)
#
# Usage:
#   ./scripts/build-installer.sh             # autodetect host
#   ./scripts/build-installer.sh --target msi
#   ./scripts/build-installer.sh --target deb
#   ./scripts/build-installer.sh --target rpm
#   ./scripts/build-installer.sh --target pkg
#   ./scripts/build-installer.sh --target dmg
#   ./scripts/build-installer.sh --all       # produce every installer that
#                                             # the host can build
# ----------------------------------------------------------------------------

set -euo pipefail

# --- paths ------------------------------------------------------------------
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$( cd "${SCRIPT_DIR}/.." && pwd )"
DIST="${ROOT}/dist/installer"
mkdir -p "${DIST}"

VERSION="$( grep -E '^version' "${ROOT}/Cargo.toml" | head -1 | sed -E 's/.*"([^"]+)".*/\1/' )"
ARCH="$( uname -m )"
case "${ARCH}" in
  x86_64)  RUST_TRIPLE="x86_64"  ;;
  aarch64|arm64) RUST_TRIPLE="aarch64" ;;
  *) echo "Unsupported arch: ${ARCH}" >&2; exit 1 ;;
esac

# --- CLI --------------------------------------------------------------------
TARGET=""
ALL=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --target) TARGET="$2"; shift 2 ;;
    --all)    ALL=1; shift ;;
    -h|--help) sed -n '2,30p' "$0"; exit 0 ;;
    *) echo "Unknown arg: $1" >&2; exit 1 ;;
  esac
done

detect_host_target() {
  case "$(uname -s)" in
    Linux*)   echo "deb rpm" ;;
    Darwin*)  echo "pkg dmg" ;;
    MINGW*|CYGWIN*|MSYS*) echo "msi" ;;
    *) echo "" ;;
  esac
}

if [[ -z "${TARGET}" ]]; then
  if [[ "${ALL}" -eq 1 ]]; then
    TARGETS=( $(detect_host_target) )
  else
    mapfile -t TARGETS < <(detect_host_target | head -1 | tr ' ' '\n')
  fi
else
  TARGETS=( "${TARGET}" )
fi

# --- pre-build: ensure the native library exists ----------------------------
build_native() {
  echo "[build-installer] cargo build --release -p finkit-ffi"
  ( cd "${ROOT}" && cargo build --release -p finkit-ffi )
}

stage_libs() {
  # Stage the platform-native shared library + headers under
  # ${DIST}/_stage/lib and ${DIST}/_stage/include, regardless of which
  # installer format is being produced.
  local stage="${DIST}/_stage"
  rm -rf "${stage}"
  mkdir -p "${stage}/lib" "${stage}/include" "${stage}/bin" "${stage}/share/finkit"

  cp "${ROOT}"/ffi/c-binding/include/*.h   "${stage}/include/"
  cp "${ROOT}"/ffi/c-binding/include/*.hpp "${stage}/include/"

  case "$(uname -s)" in
    MINGW*|CYGWIN*|MSYS*)
      cp "${ROOT}/target/release/finkit_ffi.dll"      "${stage}/bin/"
      cp "${ROOT}/target/release/finkit_ffi.dll.lib"  "${stage}/lib/"
      ;;
    Darwin*)
      cp "${ROOT}/target/release/libfinkit_ffi.dylib" "${stage}/lib/"
      install_name_tool -id "@rpath/libfinkit_ffi.dylib" "${stage}/lib/libfinkit_ffi.dylib" 2>/dev/null || true
      ;;
    Linux*)
      cp "${ROOT}/target/release/libfinkit_ffi.so"    "${stage}/lib/"
      ;;
  esac

  # License files for the system installer payload.
  if [[ -f "${ROOT}/LICENSE" ]]; then
    cp "${ROOT}/LICENSE" "${stage}/share/finkit/LICENSE"
  fi
}

# --- per-target installers --------------------------------------------------
build_msi() {
  echo "[build-installer] Building .msi (WiX)"
  command -v candle >/dev/null || { echo "WiX Toolset not in PATH (candle.exe)" >&2; return 1; }
  command -v light  >/dev/null || { echo "WiX Toolset not in PATH (light.exe)"  >&2; return 1; }

  local wix="${ROOT}/packaging/wix"
  mkdir -p "${wix}/obj"
  local wobj="${wix}/obj"

  # Harvest the staged payload into WiX fragments
  "${WIX:-wix}/bin/heat.exe" dir "${DIST}/_stage" \
      -cg FinkitComponentGroup \
      -dr INSTALLDIR \
      -srd -scom -sreg -sfrag -sb \
      -out "${wobj}/harvested.wxs"

  candle.exe -ext WixUIExtension -out "${wobj}\\" \
      "${wix}/Product.wxs" "${wobj}/harvested.wxs"

  light.exe -ext WixUIExtension -out "${DIST}/finkit-${VERSION}-${RUST_TRIPLE}-pc-windows-msvc.msi" \
      "${wobj}/Product.wixobj" "${wobj}/harvested.wixobj"
}

build_deb() {
  echo "[build-installer] Building .deb (dpkg-deb)"
  command -v dpkg-deb >/dev/null || { echo "dpkg-deb not installed" >&2; return 1; }
  command -v fakeroot >/dev/null || { echo "fakeroot not installed" >&2; return 1; }

  local pkgroot="${DIST}/_deb/Finkit_${VERSION}"
  rm -rf "${pkgroot}"
  mkdir -p "${pkgroot}/DEBIAN" \
           "${pkgroot}/usr/lib" \
           "${pkgroot}/usr/include/finkit" \
           "${pkgroot}/usr/share/doc/finkit"

  cp -r "${DIST}/_stage/lib/."   "${pkgroot}/usr/lib/"
  cp    "${DIST}/_stage/include/."  "${pkgroot}/usr/include/finkit/"
  cp    "${DIST}/_stage/share/finkit/LICENSE" \
        "${pkgroot}/usr/share/doc/finkit/copyright"

  cat > "${pkgroot}/DEBIAN/control" <<EOF
Package: finkit
Version: ${VERSION}
Section: libs
Priority: optional
Architecture: $(dpkg --print-architecture)
Maintainer: Finkit Contributors <[email protected]>
Depends: libc6
Description: AlphaTA — high-performance financial technical analysis library
 Provides 40+ technical indicators (SMA, EMA, RSI, MACD, BBANDS, STOCH,
 ADX, ATR, Hilbert Transform, ...) with C and C++ bindings, backed by a
 Rust core.
License: MIT OR Apache-2.0
EOF

  fakeroot dpkg-deb --build "${pkgroot}" \
      "${DIST}/finkit_${VERSION}_$(dpkg --print-architecture).deb"
}

build_rpm() {
  echo "[build-installer] Building .rpm (rpmbuild)"
  command -v rpmbuild >/dev/null || { echo "rpmbuild not installed" >&2; return 1; }

  local topdir="${DIST}/_rpmbuild"
  rm -rf "${topdir}"
  mkdir -p "${topdir}"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

  # Pack the staged payload into a source tarball for rpmbuild.
  local src_tar="${topdir}/SOURCES/finkit-${VERSION}.tar.gz"
  tar -C "${DIST}/_stage" -czf "${src_tar}" .

  cat > "${topdir}/SPECS/finkit.spec" <<EOF
Name:           finkit
Version:        ${VERSION}
Release:        1%{?dist}
Summary:        AlphaTA — high-performance financial technical analysis library
License:        MIT OR Apache-2.0
URL:            https://github.com/coeasy/finkit
Source0:        finkit-${VERSION}.tar.gz
BuildArch:      $(uname -m)
%description
Provides 40+ technical indicators (SMA, EMA, RSI, MACD, BBANDS, STOCH, ADX,
ATR, Hilbert Transform, ...) with C and C++ bindings, backed by a Rust core.
%global debug_package %{nil}
%prep
%setup -q -c
%install
mkdir -p %{buildroot}/usr/lib
mkdir -p %{buildroot}/usr/include/finkit
mkdir -p %{buildroot}/usr/share/doc/finkit
cp -r lib/.  %{buildroot}/usr/lib/
cp -r include/. %{buildroot}/usr/include/finkit/
cp share/finkit/LICENSE %{buildroot}/usr/share/doc/finkit/
%files
/usr/lib/libfinkit_ffi.so
%dir /usr/include/finkit
/usr/include/finkit/*.h
/usr/include/finkit/*.hpp
/usr/share/doc/finkit/LICENSE
%changelog
* $(date '+%a %b %d %Y') Finkit Contributors - ${VERSION}-1
- Automated build via packaging/build-installer.sh
EOF

  rpmbuild --define "_topdir ${topdir}" -ba "${topdir}/SPECS/finkit.spec"
  find "${topdir}/RPMS" -name "*.rpm" -exec cp {} "${DIST}/" \;
}

build_pkg() {
  echo "[build-installer] Building .pkg (pkgbuild)"
  command -v pkgbuild >/dev/null || { echo "pkgbuild not installed" >&2; return 1; }

  local pkgroot="${DIST}/_pkgroot"
  rm -rf "${pkgroot}"
  mkdir -p "${pkgroot}/usr/local/lib" \
           "${pkgroot}/usr/local/include/finkit" \
           "${pkgroot}/usr/local/share/finkit"

  cp -r "${DIST}/_stage/lib/."   "${pkgroot}/usr/local/lib/"
  cp    "${DIST}/_stage/include/."  "${pkgroot}/usr/local/include/finkit/"
  cp    "${DIST}/_stage/share/finkit/LICENSE" \
        "${pkgroot}/usr/local/share/finkit/LICENSE"

  local out="${DIST}/finkit-${VERSION}-${RUST_TRIPLE}-apple-darwin.pkg"
  pkgbuild \
    --root "${pkgroot}" \
    --identifier "com.finkit.lib" \
    --version "${VERSION}" \
    --install-location "/" \
    --ownership recommended \
    "${out}"
}

build_dmg() {
  echo "[build-installer] Building .dmg (hdiutil)"
  command -v hdiutil >/dev/null || { echo "hdiutil not installed" >&2; return 1; }

  # First make sure the .pkg is available so we can wrap it in a DMG.
  build_pkg

  local staging="${DIST}/_dmg"
  rm -rf "${staging}"
  mkdir -p "${staging}"
  cp "${DIST}"/finkit-*-apple-darwin.pkg "${staging}/"

  local rw="${staging}/finkit-rw.dmg"
  hdiutil create -ov -fs HFS+ -srcfolder "${staging}" -volname "AlphaTA ${VERSION}" \
      "${rw}"
  hdiutil convert "${rw}" -format UDZO -o "${DIST}/finkit-${VERSION}-${RUST_TRIPLE}-apple-darwin.dmg"
  rm -f "${rw}"
}

# --- driver -----------------------------------------------------------------
build_native
stage_libs

for t in "${TARGETS[@]}"; do
  case "${t}" in
    msi) build_msi ;;
    deb) build_deb ;;
    rpm) build_rpm ;;
    pkg) build_pkg ;;
    dmg) build_dmg ;;
    *)   echo "Unknown target: ${t}" >&2 ;;
  esac
done

echo
echo "[build-installer] Done. Artifacts in ${DIST}:"
ls -lh "${DIST}"/*.msi "${DIST}"/*.deb "${DIST}"/*.rpm "${DIST}"/*.pkg "${DIST}"/*.dmg 2>/dev/null || true
