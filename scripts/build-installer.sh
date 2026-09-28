#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# Finkit native system installer builder.
#
# Produces OS-level installers that integrate with the platform package
# manager / installer framework, plus one portable archive that every platform
# can consume:
#
#   * native : portable archive (tar.gz / zip) built by build_native_archive.py
#   * Windows: .msi via WiX Toolset 3.x
#   * Debian : .deb (dpkg-deb)
#   * Fedora : .rpm (rpmbuild)
#   * macOS  : .pkg (pkgbuild), and .dmg wrapping that .pkg via hdiutil
#
# Every run ends by printing a summary table and *verifying that a requested
# target produced a file*. A target whose tooling is absent is reported as
# SKIPPED with the reason; a target that ran but produced nothing is a FAILURE.
# That distinction is the whole point -- the previous version of this script
# exited 0 even when every requested target had actually produced nothing.
#
# Pre-requisites per target (CI installs these before invoking the script):
#   * native : nothing beyond cargo
#   * msi    : WiX Toolset 3.x (candle.exe + light.exe on PATH)
#   * deb    : dpkg-deb, dpkg-architecture, fakeroot
#   * rpm    : rpmbuild
#   * pkg    : pkgbuild (macOS)
#   * dmg    : hdiutil (macOS)
#
# Usage:
#   ./scripts/build-installer.sh                   # autodetect host, first target
#   ./scripts/build-installer.sh --target native
#   ./scripts/build-installer.sh --target deb
#   ./scripts/build-installer.sh --target rpm
#   ./scripts/build-installer.sh --target pkg
#   ./scripts/build-installer.sh --target dmg
#   ./scripts/build-installer.sh --target msi
#   ./scripts/build-installer.sh --all             # every target the host can build
#   ./scripts/build-installer.sh --all --strict    # treat SKIPPED as failure
# ----------------------------------------------------------------------------

set -euo pipefail

# --- paths ------------------------------------------------------------------
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$( cd "${SCRIPT_DIR}/.." && pwd )"
DIST="${ROOT}/dist/installer"
mkdir -p "${DIST}"

VERSION="$( grep -E '^version' "${ROOT}/Cargo.toml" | head -1 | sed -E 's/.*"([^"]+)".*/\1/' )"
if [[ -z "${VERSION}" ]]; then
  echo "[build-installer] could not read the workspace version from Cargo.toml" >&2
  exit 1
fi

ARCH="$( uname -m )"
case "${ARCH}" in
  x86_64|amd64)  RUST_TRIPLE="x86_64" ;;
  aarch64|arm64) RUST_TRIPLE="aarch64" ;;
  *) echo "[build-installer] Unsupported arch: ${ARCH}" >&2; exit 1 ;;
esac

# Cargo honours CARGO_TARGET_DIR; staging must read from the same tree it wrote,
# otherwise a redirected checkout packs from target/release and stages from
# .cargo-target/release (or produces nothing at all).
TARGET_DIR="${CARGO_TARGET_DIR:-${ROOT}/target}"
if [[ ! -d "${TARGET_DIR}" ]]; then
  TARGET_DIR="${ROOT}/target"
fi
RELEASE_DIR="${TARGET_DIR}/release"

# --- result bookkeeping -----------------------------------------------------
declare -a RESULTS=()
STATUS_OK="ok"
STATUS_SKIP="skip"
STATUS_FAIL="fail"
record() { RESULTS+=("${1}|${2}"); }   # target|status

have() { command -v "$1" >/dev/null 2>&1; }

# --- CLI --------------------------------------------------------------------
# `--target` may be passed repeatedly: the CI matrix expands one flag per
# requested format, so it accumulates instead of overwriting. Assigning to a
# scalar here silently dropped every earlier --target and built only the last
# one, which is invisible when the summary still reports success.
declare -a TARGET_LIST=()
ALL=0
STRICT=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      [[ $# -ge 2 ]] || { echo "[build-installer] --target needs an argument" >&2; exit 2; }
      TARGET_LIST+=( "$2" ); shift 2 ;;
    --all)    ALL=1; shift ;;
    --strict) STRICT=1; shift ;;
    -h|--help) sed -n '2,44p' "$0"; exit 0 ;;
    *) echo "[build-installer] Unknown arg: $1" >&2; exit 2 ;;
  esac
done

detect_host_targets() {
  case "$(uname -s)" in
    Linux*)   echo "native deb rpm" ;;
    Darwin*)  echo "native pkg dmg" ;;
    MINGW*|CYGWIN*|MSYS*) echo "native msi" ;;
    *) echo "native" ;;
  esac
}

if [[ "${ALL}" -eq 1 ]]; then
  mapfile -t TARGETS < <(detect_host_targets | tr ' ' '\n')
elif [[ ${#TARGET_LIST[@]} -gt 0 ]]; then
  # Preserve the order the caller asked for, de-duplicated: repeating a target
  # would otherwise rebuild the same installer twice into the same path.
  declare -a TARGETS=()
  for t in "${TARGET_LIST[@]}"; do
    already=0
    for seen in ${TARGETS[@]+"${TARGETS[@]}"}; do
      [[ "${seen}" == "${t}" ]] && already=1
    done
    [[ "${already}" -eq 0 ]] && TARGETS+=( "${t}" )
  done
else
  TARGETS=( "$(detect_host_targets | tr ' ' '\n' | head -1)" )
fi

# Every requested target must be one this script knows how to build. Failing
# here beats falling through to a case-statement default that prints "Unknown
# target" and then reports success.
KNOWN_TARGETS=" native msi deb rpm pkg dmg "
for t in "${TARGETS[@]}"; do
  [[ "${KNOWN_TARGETS}" == *" ${t} "* ]] || {
    echo "[build-installer] unknown target: ${t} (known: native msi deb rpm pkg dmg)" >&2
    exit 2
  }
done

# --- pre-build: ensure the native library exists ----------------------------
build_native() {
  echo "[build-installer] cargo build --release -p finkit-ffi"
  ( cd "${ROOT}" && cargo build --release -p finkit-ffi --locked )
}

platform_slug() {
  case "$(uname -s)" in
    Darwin*)  [[ "${RUST_TRIPLE}" == "aarch64" ]] && echo "macos-arm64" || echo "macos-x64" ;;
    Linux*)   [[ "${RUST_TRIPLE}" == "aarch64" ]] && echo "linux-aarch64" || echo "linux-x64" ;;
    *)        echo "windows-x64" ;;
  esac
}

stage_libs() {
  local stage="${DIST}/_stage"
  rm -rf "${stage}"
  mkdir -p "${stage}/lib" "${stage}/include" "${stage}/bin" "${stage}/share/finkit"

  cp "${ROOT}"/ffi/c-binding/include/*.h   "${stage}/include/"
  cp "${ROOT}"/ffi/c-binding/include/*.hpp "${stage}/include/"

  case "$(uname -s)" in
    MINGW*|CYGWIN*|MSYS*)
      cp "${RELEASE_DIR}/finkit_ffi.dll"      "${stage}/bin/"
      cp "${RELEASE_DIR}/finkit_ffi.dll.lib"  "${stage}/lib/"
      cp "${RELEASE_DIR}/finkit_ffi.lib"      "${stage}/lib/"
      ;;
    Darwin*)
      cp "${RELEASE_DIR}/libfinkit_ffi.dylib" "${stage}/lib/"
      cp "${RELEASE_DIR}/libfinkit_ffi.a"     "${stage}/lib/"
      # Without rewriting the install name the dylib records its build path, so
      # a consumer that copies it anywhere else gets a dyld load error rather
      # than a working library. This step must not be allowed to fail quietly.
      install_name_tool -id "@rpath/libfinkit_ffi.dylib" \
          "${stage}/lib/libfinkit_ffi.dylib" || {
            echo "[build-installer] install_name_tool failed on the staged dylib" >&2
            return 1
          }
      ;;
    Linux*)
      cp "${RELEASE_DIR}/libfinkit_ffi.so"    "${stage}/lib/"
      cp "${RELEASE_DIR}/libfinkit_ffi.a"     "${stage}/lib/"
      ;;
  esac

  if [[ -f "${ROOT}/LICENSE" ]]; then
    cp "${ROOT}/LICENSE" "${stage}/share/finkit/LICENSE"
  fi
}

# --- native: portable archive (every platform) ------------------------------
build_native_archive() {
  echo "[build-installer] Building native archive (tar.gz/zip)"
  local slug out
  slug="$(platform_slug)"
  out="${DIST}/finkit-${VERSION}-native-${slug}"
  case "${slug}" in
    windows-x64) out="${out}.zip" ;;
    *)           out="${out}.tar.gz" ;;
  esac

  # Delegate rather than duplicate: build_native_archive.py owns the member
  # list, the reproducible timestamps and the target-directory resolution. A
  # second implementation here is how the archive and the MSI payload drifted
  # apart in the first place.
  python "${ROOT}/scripts/build_native_archive.py" \
      --platform "${slug}" \
      --target-dir "${RELEASE_DIR}" \
      --out "${out}" || return 1

  [[ -s "${out}" ]] || { echo "[build-installer] ${out} was not produced" >&2; return 1; }
  echo "[build-installer]   -> ${out}"
}

# --- msi --------------------------------------------------------------------
build_msi() {
  echo "[build-installer] Building .msi (WiX)"
  if ! have candle || ! have light; then
    echo "[build-installer] SKIP: WiX Toolset not on PATH (need candle.exe + light.exe)"
    return 3
  fi

  local wix="${ROOT}/packaging/wix"
  if [[ ! -f "${wix}/Product.wxs" ]]; then
    echo "[build-installer] FAIL: missing WiX product definition ${wix}/Product.wxs"
    return 1
  fi

  # The Windows Installer UI requires a license in RTF, not plain text. It is
  # generated from LICENSE so there is exactly one license in the repository.
  local rtf="${DIST}/_stage/meta/license.rtf"
  mkdir -p "$(dirname "${rtf}")"
  python - "${ROOT}/LICENSE" "${rtf}" <<'PY' || return 1
import sys
src, dst = sys.argv[1], sys.argv[2]
text = open(src, encoding="utf-8").read()
out = ["\\rtf1\\ansi\\ansicpg1252\\deff0\\nouicompat",
       "{\\fonttbl{\\f0\\fnil\\fcharset0 Courier New;}}",
       "\\viewkind4\\uc1\\pard\\f0\\fs18 "]
for ch in text:
    if ch in "\\{}":
        out.append("\\" + ch)
    elif ch == "\n":
        out.append("\\par\n")
    elif ord(ch) > 127:
        out.append("\\u%d?" % ord(ch))
    else:
        out.append(ch)
out.append("\\par\n}\n")
open(dst, "w", encoding="ascii", newline="").write("{" + "".join(out))
PY

  local wobj="${wix}/obj"
  mkdir -p "${wobj}"

  # NOTE: no heat.exe harvesting. The product definition names every member
  # explicitly; harvesting would let a stray file in the staging tree silently
  # become part of the shipped installer.
  candle.exe -dVersion="${VERSION}" -dPayloadDir="${DIST}/_stage" \
      -out "${wobj}/" "${wix}/Product.wxs" || return 1

  light.exe -ext WixUIExtension \
      -out "${DIST}/finkit-${VERSION}-${RUST_TRIPLE}-pc-windows-msvc.msi" \
      "${wobj}/Product.wixobj" || return 1

  local msi="${DIST}/finkit-${VERSION}-${RUST_TRIPLE}-pc-windows-msvc.msi"
  [[ -s "${msi}" ]] || { echo "[build-installer] ${msi} was not produced" >&2; return 1; }
  echo "[build-installer]   -> ${msi}"
}

# --- deb --------------------------------------------------------------------
build_deb() {
  echo "[build-installer] Building .deb (dpkg-deb)"
  for tool in dpkg-deb dpkg-architecture fakeroot; do
    if ! have "${tool}"; then
      echo "[build-installer] SKIP: ${tool} not installed"
      return 3
    fi
  done

  # Debian multiarch: libraries belong under /usr/lib/<gnu-triplet> so that a
  # foreign-architecture package can be co-installed instead of overwriting.
  local triplet deb_arch
  triplet="$(dpkg-architecture -qDEB_HOST_MULTIARCH)"
  deb_arch="$(dpkg-architecture -qDEB_HOST_ARCH)"

  local pkgroot="${DIST}/_deb/finkit_${VERSION}_${deb_arch}"
  rm -rf "${pkgroot}"
  mkdir -p "${pkgroot}/DEBIAN" \
           "${pkgroot}/usr/lib/${triplet}" \
           "${pkgroot}/usr/include/finkit" \
           "${pkgroot}/usr/share/doc/finkit"

  cp -r "${DIST}/_stage/lib/."       "${pkgroot}/usr/lib/${triplet}/"
  cp -r "${DIST}/_stage/include/."   "${pkgroot}/usr/include/finkit/"
  cp    "${DIST}/_stage/share/finkit/LICENSE" \
        "${pkgroot}/usr/share/doc/finkit/copyright"

  cat > "${pkgroot}/DEBIAN/control" <<EOF
Package: finkit
Version: ${VERSION}
Section: libs
Priority: optional
Architecture: ${deb_arch}
Maintainer: Finkit Contributors
Homepage: https://github.com/coeasy/finkit
Depends: libc6
Description: Finkit - high-performance quantitative finance engine
 Provides 40+ technical indicators (SMA, EMA, RSI, MACD, BBANDS, STOCH,
 ADX, ATR, Hilbert Transform, ...) with C and C++ bindings, backed by a
 Rust core.
License: MIT OR Apache-2.0
EOF

  fakeroot dpkg-deb --build "${pkgroot}" \
      "${DIST}/finkit_${VERSION}_${deb_arch}.deb" || return 1

  local deb="${DIST}/finkit_${VERSION}_${deb_arch}.deb"
  [[ -s "${deb}" ]] || { echo "[build-installer] ${deb} was not produced" >&2; return 1; }
  echo "[build-installer]   -> ${deb}"
}

# --- rpm --------------------------------------------------------------------
build_rpm() {
  echo "[build-installer] Building .rpm (rpmbuild)"
  if ! have rpmbuild; then
    echo "[build-installer] SKIP: rpmbuild not installed"
    return 3
  fi

  # rpm reports %{_libdir} instead of hardcoding it: it is /usr/lib64 on the
  # 64-bit Red Hat family and /usr/lib elsewhere, and getting it wrong
  # produces a package whose files land outside any configured library path.
  local topdir="${DIST}/_rpmbuild"
  rm -rf "${topdir}"
  mkdir -p "${topdir}"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

  tar -C "${DIST}/_stage" -czf "${topdir}/SOURCES/finkit-${VERSION}.tar.gz" .

  cat > "${topdir}/SPECS/finkit.spec" <<EOF
Name:           finkit
Version:        ${VERSION}
Release:        1%{?dist}
Summary:        Finkit - high-performance quantitative finance engine
License:        MIT OR Apache-2.0
URL:            https://github.com/coeasy/finkit
Source0:        finkit-${VERSION}.tar.gz
%description
Provides 40+ technical indicators (SMA, EMA, RSI, MACD, BBANDS, STOCH, ADX,
ATR, Hilbert Transform, ...) with C and C++ bindings, backed by a Rust core.
%global debug_package %{nil}
%prep
%setup -q -c
%install
mkdir -p %{buildroot}%{_libdir}
mkdir -p %{buildroot}%{_includedir}/finkit
mkdir -p %{buildroot}%{_docdir}/finkit
cp -r lib/.  %{buildroot}%{_libdir}/
cp -r include/. %{buildroot}%{_includedir}/finkit/
cp share/finkit/LICENSE %{buildroot}%{_docdir}/finkit/
%files
%{_libdir}/libfinkit_ffi.so
%{_libdir}/libfinkit_ffi.a
%dir %{_includedir}/finkit
%{_includedir}/finkit/*.h
%{_includedir}/finkit/*.hpp
%{_docdir}/finkit/LICENSE
%changelog
* $(date '+%a %b %d %Y') Finkit Contributors - ${VERSION}-1
- Automated build via scripts/build-installer.sh
EOF

  rpmbuild --define "_topdir ${topdir}" -ba "${topdir}/SPECS/finkit.spec" || return 1

  local found=0
  while IFS= read -r rpm; do
    cp "${rpm}" "${DIST}/"
    found=1
    echo "[build-installer]   -> ${DIST}/$(basename "${rpm}")"
  done < <(find "${topdir}/RPMS" -name "*.rpm")
  [[ "${found}" -eq 1 ]] || {
    echo "[build-installer] rpmbuild produced no .rpm" >&2
    return 1
  }
}

# --- macOS pkg / dmg --------------------------------------------------------
build_pkg() {
  echo "[build-installer] Building .pkg (pkgbuild)"
  if ! have pkgbuild; then
    echo "[build-installer] SKIP: pkgbuild not installed (macOS only)"
    return 3
  fi

  local pkgroot="${DIST}/_pkgroot"
  rm -rf "${pkgroot}"
  mkdir -p "${pkgroot}/usr/local/lib" \
           "${pkgroot}/usr/local/include/finkit" \
           "${pkgroot}/usr/local/share/finkit"

  cp -r "${DIST}/_stage/lib/."     "${pkgroot}/usr/local/lib/"
  cp -r "${DIST}/_stage/include/." "${pkgroot}/usr/local/include/finkit/"
  cp    "${DIST}/_stage/share/finkit/LICENSE" \
        "${pkgroot}/usr/local/share/finkit/LICENSE"

  local out="${DIST}/finkit-${VERSION}-${RUST_TRIPLE}-apple-darwin.pkg"
  pkgbuild \
    --root "${pkgroot}" \
    --identifier "com.finkit.lib" \
    --version "${VERSION}" \
    --install-location "/" \
    --ownership recommended \
    "${out}" || return 1

  [[ -s "${out}" ]] || { echo "[build-installer] ${out} was not produced" >&2; return 1; }
  echo "[build-installer]   -> ${out}"
}

build_dmg() {
  echo "[build-installer] Building .dmg (hdiutil)"
  if ! have hdiutil; then
    echo "[build-installer] SKIP: hdiutil not installed (macOS only)"
    return 3
  fi

  local pkg="${DIST}/finkit-${VERSION}-${RUST_TRIPLE}-apple-darwin.pkg"
  if [[ ! -s "${pkg}" ]]; then
    # Reuse the .pkg if this run already built one; build it only if it is
    # missing. Unconditionally re-running build_pkg rebuilt the whole payload
    # whenever both targets were requested.
    build_pkg || return 1
  fi

  local staging="${DIST}/_dmg"
  rm -rf "${staging}"
  mkdir -p "${staging}"
  cp "${pkg}" "${staging}/"

  local rw="${staging}/finkit-rw.dmg"
  hdiutil create -ov -fs HFS+ -srcfolder "${staging}" -volname "Finkit ${VERSION}" \
      "${rw}" || return 1
  hdiutil convert "${rw}" -format UDZO \
      -o "${DIST}/finkit-${VERSION}-${RUST_TRIPLE}-apple-darwin.dmg" || return 1
  rm -f "${rw}"

  local dmg="${DIST}/finkit-${VERSION}-${RUST_TRIPLE}-apple-darwin.dmg"
  [[ -s "${dmg}" ]] || { echo "[build-installer] ${dmg} was not produced" >&2; return 1; }
  echo "[build-installer]   -> ${dmg}"
}

# --- driver -----------------------------------------------------------------
build_native

# staging is only needed by the OS-level installers; the native archive packs
# straight out of the target tree.
needs_stage=0
for t in "${TARGETS[@]}"; do
  [[ "${t}" == "native" ]] || needs_stage=1
done
[[ "${needs_stage}" -eq 1 ]] && stage_libs

declare -a PRODUCED=()
declare -a MISSING=()

for t in "${TARGETS[@]}"; do
  rc=0
  case "${t}" in
    native) build_native_archive || rc=$? ;;
    msi)    build_msi            || rc=$? ;;
    deb)    build_deb            || rc=$? ;;
    rpm)    build_rpm            || rc=$? ;;
    pkg)    build_pkg            || rc=$? ;;
    dmg)    build_dmg            || rc=$? ;;
  esac

  case "${rc}" in
    0) record "${t}" "${STATUS_OK}" ;;
    3) record "${t}" "${STATUS_SKIP}" ;;
    *) record "${t}" "${STATUS_FAIL}"; MISSING+=("${t}") ;;
  esac
done

# --- summary ----------------------------------------------------------------
echo
echo "[build-installer] Summary"
printf '  %-8s %s\n' "TARGET" "RESULT"
for entry in "${RESULTS[@]}"; do
  printf '  %-8s %s\n' "${entry%%|*}" "${entry##*|}"
done

skip_count=0
for entry in "${RESULTS[@]}"; do
  [[ "${entry##*|}" == "${STATUS_SKIP}" ]] && skip_count=$(( skip_count + 1 ))
done

# Nothing was built at all is never a success: it means a release would ship
# without installers and nobody would notice until a user tried to download
# one.
if [[ ${#RESULTS[@]} -gt 0 && "${skip_count}" -eq "${#RESULTS[@]}" ]]; then
  echo
  echo "[build-installer] every requested target was skipped; nothing to publish" >&2
  exit 1
fi

if [[ "${STRICT}" -eq 1 && "${skip_count}" -gt 0 ]]; then
  echo
  echo "[build-installer] --strict: skipped targets are failures" >&2
  exit 1
fi

if [[ ${#MISSING[@]} -gt 0 ]]; then
  echo
  echo "[build-installer] failed target(s): ${MISSING[*]}" >&2
  exit 1
fi

echo
echo "[build-installer] Artifacts in ${DIST}:"
printed=0
for pattern in '*.msi' '*.deb' '*.rpm' '*.pkg' '*.dmg' '*.tar.gz' '*.zip'; do
  # No `|| true` here. Exercising the "nothing matched" path silently is how
  # this script previously reported success for a build that produced nothing.
  compgen -G "${DIST}/${pattern}" >/dev/null && {
    ls -lh "${DIST}"/${pattern}
    printed=1
  } || true
done
[[ "${printed}" -eq 1 ]] || echo "  (none)"
