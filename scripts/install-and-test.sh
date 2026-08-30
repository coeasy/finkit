#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# AlphaTA — install + smoke test every built artifact, one language at a time.
#
# For each language in {python, node, java, go, c, dotnet, wasm} this:
#   1. Invokes scripts/build-usage-packages.sh <lang> --no-bundle --no-verify
#      (or skips if dist/<lang>/<plat> already has fresh artifacts).
#   2. Invokes the same with --no-bundle (verify is on by default) to actually
#      install the artifact and run packaging/usage/<lang>/verify_install.*.
#   3. Captures stdout/stderr in .test_venv/logs/<lang>.log and prints a
#      pass/fail summary at the end.
#
# Exit code: number of failed languages (0 = all OK).
# ----------------------------------------------------------------------------
set -uo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$( cd "${SCRIPT_DIR}/.." && pwd )"
LOG_DIR="${ROOT}/.test_venv/logs"
mkdir -p "${LOG_DIR}"

LANGS=( "python" "node" "java" "go" "c" "dotnet" "wasm" )
UNIFIED="${ROOT}/scripts/build-usage-packages.sh"

# ---- platform (must match build-usage-packages.sh) -----------------------
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

ok=0
fail=0
skipped=0
declare -a FAILED=()

hdr() { printf "\n\033[1;36m=== %s ===\033[0m\n" "$*"; }
okp() { printf "  \033[32m[OK]\033[0m   %s\n" "$*"; }
errp(){ printf "  \033[31m[FAIL]\033[0m %s\n" "$*"; }
warnp(){ printf "  \033[33m[SKIP]\033[0m %s\n" "$*"; }

# Auto-build only the languages whose dist/<lang>/<plat>/ tree is missing.
needs_build=()
for lang in "${LANGS[@]}"; do
  if [[ -d "${ROOT}/dist/${lang}/${PLATFORM}" ]] && \
     find "${ROOT}/dist/${lang}/${PLATFORM}" -mindepth 1 -maxdepth 1 -print -quit 2>/dev/null | grep -q .; then
    :
  else
    needs_build+=( "${lang}" )
  fi
done

if [[ ${#needs_build[@]} -gt 0 ]]; then
  hdr "pre-build: ${needs_build[*]}"
  "${UNIFIED}" "${needs_build[@]}" --no-bundle --no-verify || true
fi

# Verify one language at a time so logs are isolated and the failure mode
# is unambiguous.
for lang in "${LANGS[@]}"; do
  hdr "[${lang}] install + smoke"
  log="${LOG_DIR}/${lang}.log"
  if "${UNIFIED}" "${lang}" --no-bundle 2>&1 | tee "${log}" | grep -qE '\[OK\]|\[OK\] *|verify OK|build OK'; then
    # Look for a "build OK" or "[OK]" tail in the log as the success signal.
    if grep -qE '\[OK\] *(python|node|java|go|c|dotnet|wasm) (build|verify)|verify OK|build OK' "${log}"; then
      okp "${lang}"
      ok=$(( ok + 1 ))
    else
      # Fallback: only count it OK if exit status from the underlying script was 0
      errp "${lang} (no OK marker in log)"
      FAILED+=( "${lang}" )
      fail=$(( fail + 1 ))
    fi
  else
    errp "${lang} (see ${log})"
    FAILED+=( "${lang}" )
    fail=$(( fail + 1 ))
  fi
done

hdr "summary"
echo "  ok      : ${ok}"
echo "  failed  : ${fail}"
echo "  skipped : ${skipped}"
if [[ ${fail} -gt 0 ]]; then
  echo "  failed languages: ${FAILED[*]}"
  echo "  inspect logs:    ls -1 ${LOG_DIR}/"
fi
exit "${fail}"
