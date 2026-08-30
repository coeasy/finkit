#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# AlphaTA — toolchain preflight check.
#
# Verifies that the host has the toolchain needed for a one-click build.
# Used by:
#   * `make preflight` — fast dry-run before invoking the full builder
#   * CI checks       — fails fast if a required tool is missing
#
# Exit code:
#   0  all required tools present (warnings still allowed)
#   1  one or more hard requirements missing
# ----------------------------------------------------------------------------
set -uo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$( cd "${SCRIPT_DIR}/../.." && pwd )"

ok()    { printf "  \033[32m[OK]\033[0m   %s\n" "$*"; }
warn()  { printf "  \033[33m[WARN]\033[0m %s\n" "$*"; }
err()   { printf "  \033[31m[MISS]\033[0m %s\n" "$*"; }
hdr()   { printf "\n\033[1;36m=== %s ===\033[0m\n" "$*"; }
note()  { printf "        %s\n" "$*"; }

hard_missing=0

check_tool() {
  local name="$1"; local why="$2"; local install="$3"; local required="${4:-soft}"
  if command -v "$name" >/dev/null 2>&1; then
    local ver
    ver="$("$name" --version 2>/dev/null | head -1 | tr -d '\n' || true)"
    if [[ -z "${ver}" ]]; then
      ver="$( "$name" -version 2>&1 | head -1 | tr -d '\n' || true)"
    fi
    ok "${name} ${ver:-found} — ${why}"
  else
    if [[ "${required}" == "hard" ]]; then
      err "${name} MISSING — ${why}"
      note "install: ${install}"
      hard_missing=$(( hard_missing + 1 ))
    else
      warn "${name} missing — ${why}"
      note "install (optional): ${install}"
    fi
  fi
}

# ---- hard requirements (build-usage requires these) ----------------------
hdr "hard requirements"

check_tool "cargo"   "Rust compiler & build tool"            "curl https://sh.rustup.rs -sSf | sh"                 "hard"
check_tool "python3" "Python interpreter + pip"              "apt install python3 python3-pip / brew install python@3.12" "hard"
check_tool "pip3"    "Python package manager"                "python3 -m ensurepip --upgrade"                      "hard"

# ---- soft requirements (per-language) -----------------------------------
hdr "per-language toolchains"

# Python wheel build
check_tool "maturin" "Python wheel builder"                  "pip3 install maturin"                                "hard"
check_tool "node"    "Node.js 20+ (for tgz build)"           "brew install node@20 / nvm install 20"               "hard"
check_tool "npm"     "Node package manager"                  "(bundled with node)"                                 "hard"
check_tool "mvn"     "Maven (Java build)"                    "apt install maven / brew install maven"              "hard"
check_tool "javac"   "Java compiler (JDK 17+)"               "apt install openjdk-17-jdk / brew install openjdk@17" "hard"
check_tool "go"      "Go 1.22+ (Go package build)"           "brew install go@1.22 / apt install golang-go"         "hard"
check_tool "cmake"   "CMake (C/C++ FFI install)"             "apt install cmake / brew install cmake"              "hard"
check_tool "gcc"     "C compiler (C FFI build)"              "apt install gcc g++ / xcode-select --install"         "hard"
check_tool "g++"     "C++ compiler (C++ FFI build)"          "apt install g++"                                     "hard"
check_tool "pkg-config" "pkg-config (C lib discovery)"       "apt install pkg-config / brew install pkg-config"    "hard"
check_tool "dotnet"  ".NET SDK 8.0+ (NuGet build)"           "brew install dotnet@8 / dotnet-install.ps1"          "hard"
check_tool "wasm-pack" "WASM packager"                       "cargo install wasm-pack"                             "soft"

# Optional / nice-to-have
check_tool "ta-lib"  "TA-Lib C library (only for --bench-talib)" "apt install libta-lib0-dev / brew install ta-lib"  "soft"
check_tool "talib"   "TA-Lib PyPI (only for precision step)" "pip install TA-Lib"                                  "soft"
check_tool "zip"     "zip archiver (--bundle step)"          "apt install zip"                                     "soft"
check_tool "tar"     "tar archiver"                          "(always present on Linux/macOS)"                     "soft"
check_tool "git"     "git"                                    "(almost always present)"                             "soft"

# ---- platform info --------------------------------------------------------
hdr "platform"
case "$( uname -s )" in
  MINGW*|MSYS*|CYGWIN*) PLATFORM="windows-x64" ;;
  Darwin)
    case "$( uname -m )" in
      arm64)  PLATFORM="macos-arm64" ;;
      x86_64) PLATFORM="macos-x64" ;;
      *)      PLATFORM="macos-unknown" ;;
    esac ;;
  Linux)
    case "$( uname -m )" in
      aarch64) PLATFORM="linux-arm64" ;;
      x86_64)  PLATFORM="linux-x64" ;;
      *)       PLATFORM="linux-unknown" ;;
    esac ;;
  *) PLATFORM="unknown" ;;
esac
echo "  uname : $(uname -srm)"
echo "  plat  : ${PLATFORM}"

# ---- summary --------------------------------------------------------------
hdr "summary"
if [[ ${hard_missing} -eq 0 ]]; then
  echo -e "  \033[32m✓ ready to run ./build-usage.sh\033[0m"
  exit 0
else
  echo -e "  \033[31m✗ ${hard_missing} hard requirement(s) missing\033[0m"
  echo "    Install the tools marked [MISS] above, then re-run."
  exit 1
fi
