#!/usr/bin/env bash
# =============================================================================
# AlphaTA — one-click QUICK multi-language package builder.
#
# Design goal: build all 7 usage packages (python/node/java/go/c/dotnet/wasm)
# as FAST as possible, with one command.
#
# Why this is faster than `build-usage-packages.sh`:
#   * The full builder builds + verifies + bundles each language SEQUENTIALLY,
#     and every per-language script runs its own `cargo build --release`
#     (so the shared core crate is "touched" 7 times and packaging never
#     overlaps with compilation).
#   * This script instead does ONE combined `cargo build --release` of every
#     native cdylib up front (the core is compiled exactly once, and cargo
#     parallelises the cdylib crates inside a single process), THEN fans out
#     the 7 per-language *packaging* scripts in parallel. Their internal
#     `cargo build` calls are no-ops (everything is already up to date), so
#     the heavy packaging work (maturin / npm pack / cmake / dotnet pack /
#     mvn / wasm-pack) runs concurrently.
#
# Defaults are SPEED-oriented: verify and bundle are OFF. Opt in with flags.
#
# Usage:
#   ./build-quick.sh                  # all 7 languages, no verify, no bundle
#   ./build-quick.sh c go node        # subset
#   ./build-quick.sh --verify         # also install + smoke-test each artifact
#   ./build-quick.sh --bundle         # also zip dist/ into a usage-bundle
#   ./build-quick.sh --clean          # wipe dist/ first
#   ./build-quick.sh --help           # this text
#
# Exit codes: 0 = all requested languages packaged OK; 1 = at least one failed.
# =============================================================================

set -uo pipefail   # do NOT use -e: we want to collect per-language failures

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="${SCRIPT_DIR}"   # this script sits at the repo root, next to build-usage.sh
DIST="${ROOT}/dist"
LOGDIR="${DIST}/.build-quick"
# Windows-native tools (python3/cmake/powershell) can't resolve the POSIX
# paths Git Bash uses (/p/...); convert to a Windows path where needed.
DIST_WIN="$( cygpath -w "$DIST" 2>/dev/null || echo "$DIST" )"
VERSION="$( grep -E '^version' "${ROOT}/Cargo.toml" | head -1 | sed -E 's/.*"([^"]+)".*/\1/' )"

# ---------------------------------------------------------------- platform ---
case "$( uname -s )" in
  MINGW*|MSYS*|CYGWIN*) PLATFORM="windows-x64" ;;
  Darwin)
    case "$( uname -m )" in
      arm64)  PLATFORM="macos-arm64" ;;
      x86_64) PLATFORM="macos-x64"   ;;
    esac ;;
  Linux)
    case "$( uname -m )" in
      aarch64) PLATFORM="linux-arm64" ;;
      x86_64)  PLATFORM="linux-x64"   ;;
    esac ;;
  *) echo "unsupported platform: $( uname -s )" >&2; exit 1 ;;
esac

# ------------------------------------------------------------------- CLI ------
LANGS=( "python" "node" "java" "go" "c" "dotnet" "wasm" )
ALL_LANGS=( "${LANGS[@]}" )
WANT=()
DO_VERIFY=0
DO_BUNDLE=0
DO_CLEAN=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    python|node|java|go|c|dotnet|wasm) WANT+=( "$1" ); shift ;;
    --verify)  DO_VERIFY=1; shift ;;
    --bundle)  DO_BUNDLE=1; shift ;;
    --clean)   DO_CLEAN=1;  shift ;;
    -h|--help)
      sed -n '2,46p' "$0" | sed 's/^# \{0,1\}//'
      exit 0 ;;
    *) echo "unknown arg: $1 (try --help)" >&2; exit 2 ;;
  esac
done
[[ ${#WANT[@]} -eq 0 ]] && WANT=( "${ALL_LANGS[@]}" )

# ----------------------------------------------------------------- helpers ----
ok()   { echo -e "  \033[32m[OK]\033[0m   $*"; }
err()  { echo -e "  \033[31m[FAIL]\033[0m $*" >&2; }
info() { echo -e "  \033[36m[INFO]\033[0m $*"; }
hdr()  { echo -e "\n\033[1;36m=== $* ===\033[0m"; }

if ! command -v cargo >/dev/null 2>&1; then
  echo "[build-quick] FATAL: cargo not found on PATH" >&2
  exit 1
fi

# On native Windows Git Bash the per-language packaging steps shell out to
# Windows-native cmake/maven/dotnet with POSIX paths they cannot resolve
# (cargo itself is fine). The repo's supported Windows one-click is Docker
# (make docker-build && make docker-run); go/node usually still work natively.
case "$( uname -s )" in
  MINGW*|MSYS*|CYGWIN*)
    echo "[build-quick] NOTE: native Windows Git Bash detected."
    echo "            cargo builds work, but cmake/maven/dotnet packaging steps pass"
    echo "            POSIX paths that Windows tools reject. For a clean all-language"
    echo "            one-click on Windows use Docker: make docker-build && make docker-run"
    echo "            (go/node typically build fine natively; c/java/dotnet may fail here)"
    ;;
esac

# ------------------------------------------------------------------- clean ----
if [[ "${DO_CLEAN}" -eq 1 ]]; then
  hdr "clean"
  rm -rf "${DIST}"
  ok "removed ${DIST}"
fi

mkdir -p "${DIST}" "${LOGDIR}"

# =============================================================== STEP 1 ======
# Combined parallel cargo pre-build of every native cdylib.
# python needs --features abi3 (its own feature set, can't be merged), so it
# is built in a separate cargo invocation. wasm is driven by wasm-pack (its
# own target), so it is left to its per-language script.
hdr "step 1/3 — combined cargo pre-build (core compiled once)"
native_crates=()
want_python=0
want_wasm=0
for lang in "${WANT[@]}"; do
  case "${lang}" in
    python) want_python=1 ;;
    wasm)   want_wasm=1 ;;
    node)    native_crates+=( "alpha-ta-node" ) ;;
    go)      native_crates+=( "alpha-ta-go" ) ;;
    c)       native_crates+=( "alpha-ta-ffi" ) ;;
    dotnet)  native_crates+=( "alpha-ta-dotnet" ) ;;
    java)    native_crates+=( "alpha-ta-java" ) ;;
  esac
done

t0=$( date +%s )
if [[ ${#native_crates[@]} -gt 0 ]]; then
  info "cargo build --release $( printf -- '-p %s ' "${native_crates[@]}" )"
  # Best-effort: a failure here is non-fatal; the per-language script will
  # rebuild the offending crate and report it.
  cargo build --release $( printf -- '-p %s ' "${native_crates[@]}" ) \
    || echo "[build-quick] WARN: native pre-build failed; per-language scripts will rebuild"
fi
if [[ "${want_python}" -eq 1 ]]; then
  info "cargo build --release -p alpha-ta-python --features abi3"
  cargo build --release -p alpha-ta-python --features abi3 \
    || echo "[build-quick] WARN: python pre-build failed; build-usage-python.sh will rebuild"
fi
if [[ "${want_wasm}" -eq 1 ]]; then
  info "wasm: ensuring wasm32-unknown-unknown target"
  rustup target add wasm32-unknown-unknown 2>/dev/null \
    || echo "[build-quick] WARN: could not add wasm32 target (wasm-pack may still manage it)"
fi
t1=$( date +%s )
info "pre-build took $(( t1 - t0 ))s"

# =============================================================== STEP 2 ======
# Fan out the per-language *packaging* scripts in parallel. Their internal
# cargo builds are no-ops (already compiled above), so this is pure packaging
# running concurrently.
hdr "step 2/3 — parallel per-language packaging"
pids=()
for lang in "${WANT[@]}"; do
  info "launching build-usage-${lang}.sh  (log: ${LOGDIR}/${lang}.log)"
  bash "${SCRIPT_DIR}/scripts/build-usage-${lang}.sh" >"${LOGDIR}/${lang}.log" 2>&1 &
  pids+=( $! )
done

fail_count=0
ok_count=0
for i in "${!WANT[@]}"; do
  if wait "${pids[$i]}"; then
    ok "${WANT[$i]} packaged"
    ok_count=$(( ok_count + 1 ))
  else
    err "${WANT[$i]} packaging FAILED — see ${LOGDIR}/${WANT[$i]}.log"
    fail_count=$(( fail_count + 1 ))
  fi
done

# =============================================================== STEP 3 ======
# Optional verify / bundle / manifest.
hdr "step 3/3 — verify / bundle / manifest"

if [[ "${DO_VERIFY}" -eq 1 ]]; then
  info "verify requested — delegating to build-usage-packages.sh --verify"
  bash "${SCRIPT_DIR}/scripts/build-usage-packages.sh" "${WANT[@]}" --verify --no-bundle \
    || err "verify step reported failures"
fi

# ---- manifest (sha256 + size of every produced artifact) -------------------
info "writing dist/manifest.json"
LANGS_CSV="$( IFS=,; echo "${WANT[*]}" )"
python3 - <<PY
import hashlib, json, os, pathlib
root = pathlib.Path(r"${DIST_WIN}")
langs = "${LANGS_CSV}".split(",")
components = []
for lang in langs:
    base = root / lang / "${PLATFORM}"
    if not base.exists():
        continue
    for f in base.rglob("*"):
        if not f.is_file():
            continue
        if any(part.startswith('.') for part in f.parts):
            continue
        if f.suffix == ".pyc":
            continue
        components.append({
            "language": lang,
            "platform": "${PLATFORM}",
            "path": str(f.relative_to(root)).replace(os.sep, "/"),
            "size_bytes": f.stat().st_size,
            "sha256": hashlib.sha256(f.read_bytes()).hexdigest(),
        })
manifest = {
    "name": "AlphaTA",
    "version": "${VERSION}",
    "platform": "${PLATFORM}",
    "quick_build": True,
    "components": components,
}
out = root / "manifest.json"
out.write_text(json.dumps(manifest, indent=2))
print(f"wrote {out} ({len(components)} components)")
PY

# ---- bundle (optional) ------------------------------------------------------
if [[ "${DO_BUNDLE}" -eq 1 && "${fail_count}" -eq 0 ]]; then
  BUNDLE="${DIST}/alpha-ta-${VERSION}-${PLATFORM}-quick-bundle.zip"
  info "bundling into ${BUNDLE}"
  # Collect the per-language source directories into an array.
  bundle_srcs=()
  for lang in "${WANT[@]}"; do
    bundle_srcs+=( "${DIST}/${lang}/${PLATFORM}" )
  done
  if command -v zip >/dev/null 2>&1; then
    ( cd "${DIST}" && zip -qr "${BUNDLE}" "${bundle_srcs[@]/#${DIST}/}" manifest.json ) \
      && ok "bundle: ${BUNDLE}" \
      || err "zip failed"
  elif command -v powershell >/dev/null 2>&1; then
    # Build the comma-separated -Path list for Compress-Archive (mirrors the
    # existing build-usage-packages.sh fallback: POSIX root + backslash rest).
    srcs=""
    for lang in "${WANT[@]}"; do
      srcs="${srcs},'${DIST_WIN}\\${lang}\\${PLATFORM}'"
    done
    srcs="${srcs#,},'${DIST_WIN}\\manifest.json'"
    powershell -NoProfile -Command "Compress-Archive -Path ${srcs} -DestinationPath '${BUNDLE}' -Force" \
      && ok "bundle (ps1): ${BUNDLE}" \
      || err "Compress-Archive failed"
  else
    err "no zip or powershell found, skipping bundle"
  fi
fi

# ----------------------------------------------------------------- summary ----
hdr "summary"
echo "  platform : ${PLATFORM}"
echo "  version  : ${VERSION}"
echo "  packaged : ${ok_count}"
echo "  failed   : ${fail_count}"
echo "  verify   : ${DO_VERIFY}"
echo "  bundle   : ${DO_BUNDLE}"
echo "  logs     : ${LOGDIR}/<lang>.log"
[[ "${fail_count}" -eq 0 ]]
